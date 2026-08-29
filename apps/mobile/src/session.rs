use std::path::PathBuf;
use std::thread;
use std::time::Duration;

use async_channel::{Receiver, Sender};
use styrene_ipc::types::{MessageInfo, MessageLifecycleState};
use styrene_ui_state::{
    Bearer, BearerKind, BearerState, Conversation, DeliveryEvidence, DeliveryMethod,
    ExpectedProjection, Message, MessageLifecycle, MobileAction, MobileActionKind, MobileFixture,
    Peer, PeerSource, PersistenceState, Profile, Propagation, PropagationCandidate,
    PropagationEvidence, PropagationPolicy, PropagationProgress, PropagationUpdate, Session,
    SessionPhase, SyncState, TransportEvidence, TypedFailure,
};
use styrened::mobile::{
    IdentityBackend, MobileBearerKind, MobileBearerReason, MobileBearerState, MobileConfig,
    MobileConnectionPhase, MobileDeliveryMethod, MobileInterfaceConfig, MobileNode,
    MobilePeerAspect, MobilePropagationSnapshot, MobilePropagationSyncState, MobileSendRequest,
    persist_mobile_tcp_endpoint,
};

const DEFAULT_ENDPOINT: &str = "rns.styrene.io:4242";
const ACTION_CAPACITY: usize = 64;

#[derive(Clone)]
pub struct MobileSession {
    actions: Sender<MobileAction>,
    updates: Receiver<SessionUpdate>,
}

#[derive(Clone, Debug)]
pub struct SessionUpdate {
    pub fixture: MobileFixture,
    pub propagation: PropagationUpdate,
}

impl MobileSession {
    pub fn start() -> Self {
        let (actions, action_receiver) = async_channel::bounded(ACTION_CAPACITY);
        let (update_sender, updates) = async_channel::bounded(1);
        let startup_failures = update_sender.clone();
        if let Err(error) = thread::Builder::new()
            .name("styrene-mobile-session".into())
            .spawn(move || run_owner(action_receiver, update_sender))
        {
            let _ = startup_failures.force_send(failed_update(
                1,
                "session_thread_start_failed",
                error.to_string(),
            ));
        }
        Self { actions, updates }
    }

    pub fn dispatch(&self, action: MobileAction) {
        let _ = self.actions.try_send(action);
    }

    pub async fn next_update(&self) -> Option<SessionUpdate> {
        self.updates.recv().await.ok()
    }

    pub fn starting_update() -> SessionUpdate {
        let mut update = failed_update(1, "starting", String::new());
        update.fixture.id = "embedded-live-starting".into();
        update.fixture.session.phase = SessionPhase::Starting;
        update.fixture.session.failure = None;
        update
    }
}

fn run_owner(actions: Receiver<MobileAction>, updates: Sender<SessionUpdate>) {
    let runtime = match tokio::runtime::Builder::new_multi_thread().enable_all().build() {
        Ok(runtime) => runtime,
        Err(error) => {
            let _ = updates.force_send(failed_update(1, "runtime_start_failed", error.to_string()));
            return;
        }
    };
    runtime.block_on(owner_loop(actions, updates));
}

async fn owner_loop(actions: Receiver<MobileAction>, updates: Sender<SessionUpdate>) {
    let mut generation = 1;
    let mut config = match mobile_config(DEFAULT_ENDPOINT) {
        Ok(config) => config,
        Err(error) => {
            let _ =
                updates.force_send(failed_update(generation, "platform_paths_unavailable", error));
            return;
        }
    };
    let mut node = match MobileNode::boot(config.clone()).await {
        Ok(node) => node,
        Err(error) => {
            let _ = updates.force_send(failed_update(
                generation,
                "embedded_start_failed",
                error.to_string(),
            ));
            return;
        }
    };
    publish_snapshot(&node, generation, &updates).await;

    let mut refresh = tokio::time::interval(Duration::from_secs(2));
    refresh.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    loop {
        tokio::select! {
            _ = refresh.tick() => publish_snapshot(&node, generation, &updates).await,
            action = actions.recv() => {
                let Ok(action) = action else {
                    let _ = node.shutdown().await;
                    return;
                };
                if action.generation != generation {
                    continue;
                }
                if let MobileActionKind::ApplyEndpoint { endpoint } = &action.kind {
                    if let Err(error) = persist_mobile_tcp_endpoint(&config.config_dir, endpoint) {
                        publish_snapshot_with_failure(
                            &node,
                            generation,
                            &updates,
                            TypedFailure {
                                code: format!("{:?}", error.code()).to_ascii_lowercase(),
                                retryable: error.retryable(),
                            },
                        )
                        .await;
                        continue;
                    }
                    let _ = node.shutdown().await;
                    generation = generation.saturating_add(1);
                    config.interfaces = vec![MobileInterfaceConfig::TcpClient {
                        remote_address: endpoint.clone(),
                    }];
                    match MobileNode::boot(config.clone()).await {
                        Ok(replacement) => node = replacement,
                        Err(error) => {
                            let _ = updates.force_send(failed_update(
                                generation,
                                "embedded_restart_failed",
                                error.to_string(),
                            ));
                            return;
                        }
                    }
                } else if let Err(failure) = execute_action(&node, action.kind).await {
                    publish_snapshot_with_failure(&node, generation, &updates, failure).await;
                    continue;
                }
                publish_snapshot(&node, generation, &updates).await;
            }
        }
    }
}

async fn execute_action(node: &MobileNode, action: MobileActionKind) -> Result<(), TypedFailure> {
    match action {
        MobileActionKind::ApplyEndpoint { .. } => {}
        MobileActionKind::SetActiveConversation { peer_hash } => {
            node.set_active_conversation(peer_hash.as_deref()).await.map_err(|error| {
                messaging_failure("active_conversation_failed", error.retryable)
            })?;
        }
        MobileActionKind::SaveDraft { peer_hash, content, .. } => {
            node.set_draft(&peer_hash, &content)
                .await
                .map_err(|error| messaging_failure("draft_save_failed", error.retryable))?;
        }
        MobileActionKind::SendMessage { peer_hash, content, requested_method, draft_revision } => {
            let requested_method = match requested_method {
                DeliveryMethod::Direct => MobileDeliveryMethod::Direct,
                DeliveryMethod::Opportunistic => MobileDeliveryMethod::Opportunistic,
                DeliveryMethod::Propagated => MobileDeliveryMethod::Propagated,
                DeliveryMethod::Unknown => {
                    return Err(messaging_failure("unsupported_delivery_method", false));
                }
            };
            node.send_text(MobileSendRequest {
                destination_hash: peer_hash,
                content,
                requested_method,
                draft_revision: Some(draft_revision),
            })
            .await
            .map_err(|error| messaging_failure("send_failed", error.retryable))?;
        }
        MobileActionKind::RetryMessage { message_id } => {
            node.retry_text(&message_id)
                .await
                .map_err(|error| messaging_failure("retry_failed", error.retryable))?;
        }
        MobileActionKind::SelectPropagationNode { destination_hash } => {
            if let Some(destination_hash) = destination_hash {
                node.select_propagation_destination(&destination_hash).await.map_err(|error| {
                    messaging_failure("propagation_selection_failed", error.retryable)
                })?;
            } else {
                node.clear_propagation_destination().await.map_err(|error| {
                    messaging_failure("propagation_clear_failed", error.retryable)
                })?;
            }
        }
        MobileActionKind::SyncPropagation => {
            node.sync_propagation_once(Duration::from_secs(32))
                .await
                .map_err(|error| messaging_failure("propagation_sync_failed", error.retryable))?;
        }
    }
    Ok(())
}

fn messaging_failure(code: &str, retryable: bool) -> TypedFailure {
    TypedFailure { code: code.into(), retryable }
}

async fn publish_snapshot(node: &MobileNode, generation: u64, updates: &Sender<SessionUpdate>) {
    if let Ok(update) = project(node, generation).await {
        let _ = updates.force_send(update);
    }
}

async fn publish_snapshot_with_failure(
    node: &MobileNode,
    generation: u64,
    updates: &Sender<SessionUpdate>,
    failure: TypedFailure,
) {
    if let Ok(mut update) = project(node, generation).await {
        update.fixture.session.failure = Some(failure);
        let _ = updates.force_send(update);
    }
}

async fn project(node: &MobileNode, generation: u64) -> Result<SessionUpdate, String> {
    let session = node.session_snapshot().await;
    let peers = node.peer_snapshot().await.map_err(|error| error.to_string())?;
    let propagation = node.propagation_snapshot().await.map_err(|error| error.to_string())?;
    let summaries = node.list_conversations().await?;
    let mut conversations = Vec::with_capacity(summaries.len());
    let mut messages = Vec::new();
    for summary in summaries {
        let draft = node.draft(&summary.peer_hash).await.map_err(|error| error.to_string())?;
        conversations.push(Conversation {
            peer_hash: summary.peer_hash.clone(),
            unread_count: summary.unread_count,
            draft: draft.as_ref().map_or_else(String::new, |draft| draft.content.clone()),
            draft_revision: draft.map_or(0, |draft| draft.revision),
        });
        messages.extend(
            node.get_messages(&summary.peer_hash, 200).await?.into_iter().map(project_message),
        );
    }

    let propagation_update = project_propagation(generation, &propagation);
    Ok(SessionUpdate {
        fixture: MobileFixture {
            id: "embedded-live".into(),
            profile: Profile::Live,
            generation,
            session: Session {
                phase: project_phase(session.phase),
                identity_hash: node.delivery_hash().unwrap_or_default(),
                endpoint: session.endpoint,
                failure: session.failure.map(|failure| TypedFailure {
                    code: format!("{:?}", failure.code).to_ascii_lowercase(),
                    retryable: failure.retryable,
                }),
            },
            bearers: session.bearers.iter().map(project_bearer).collect(),
            peers: peers
                .peers
                .into_iter()
                .map(|peer| Peer {
                    destination_hash: peer.destination_hash,
                    aspect: match peer.aspect {
                        MobilePeerAspect::LxmfDelivery => "lxmf.delivery",
                        MobilePeerAspect::LxmfPropagation => "lxmf.propagation",
                        MobilePeerAspect::NomadNetworkNode => "nomadnetwork.node",
                    }
                    .into(),
                    display_name: peer.display_name,
                    observed_at: peer.observed_at,
                    age_secs: peer.age_secs,
                    source: PeerSource::CanonicalAnnounce,
                    announce_count: peer.announce_count,
                })
                .collect(),
            conversations,
            messages,
            propagation: Propagation {
                selected_destination: propagation_update.selected_destination.clone(),
                ready: propagation_update.ready,
                sync_state: propagation_update.sync_state,
                new_messages: propagation_update.new_messages,
                failure: propagation_update.failure.clone(),
            },
            event: None,
            expected: ExpectedProjection {
                fixture_banner: false,
                live_network_enabled: true,
                peer_count: 0,
                conversation_count: 0,
                message_count: 0,
                accessibility_ids: Vec::new(),
            },
        },
        propagation: propagation_update,
    })
}

fn project_message(message: MessageInfo) -> Message {
    let lifecycle = project_lifecycle(message.lifecycle_state);
    let delivered = lifecycle == MessageLifecycle::Delivered;
    let failed = matches!(
        lifecycle,
        MessageLifecycle::Failed | MessageLifecycle::Expired | MessageLifecycle::Rejected
    );
    Message {
        id: message.id,
        peer_hash: if message.is_outgoing { message.destination_hash } else { message.source_hash },
        content: message.content,
        requested_method: project_method(message.requested_delivery_method.as_deref()),
        actual_method: project_method(message.actual_delivery_method.as_deref()),
        persistence: PersistenceState::Durable,
        transport: TransportEvidence::None,
        propagation: PropagationEvidence::None,
        delivery: if delivered { DeliveryEvidence::Delivered } else { DeliveryEvidence::Pending },
        correlation_id: message.correlation_id.unwrap_or_default(),
        failure: failed
            .then_some(TypedFailure { code: "terminal_message".into(), retryable: true }),
        lifecycle: Some(lifecycle),
    }
}

fn project_method(method: Option<&str>) -> DeliveryMethod {
    match method {
        Some("direct") => DeliveryMethod::Direct,
        Some("propagated") => DeliveryMethod::Propagated,
        Some("opportunistic") => DeliveryMethod::Opportunistic,
        _ => DeliveryMethod::Unknown,
    }
}

fn project_lifecycle(lifecycle: MessageLifecycleState) -> MessageLifecycle {
    match lifecycle {
        MessageLifecycleState::Queued => MessageLifecycle::Queued,
        MessageLifecycleState::Sending => MessageLifecycle::Sending,
        MessageLifecycleState::Sent => MessageLifecycle::Sent,
        MessageLifecycleState::Delivered => MessageLifecycle::Delivered,
        MessageLifecycleState::Failed => MessageLifecycle::Failed,
        MessageLifecycleState::Cancelled => MessageLifecycle::Cancelled,
        MessageLifecycleState::Expired => MessageLifecycle::Expired,
        MessageLifecycleState::Rejected => MessageLifecycle::Rejected,
        _ => MessageLifecycle::Unknown,
    }
}

fn project_phase(phase: MobileConnectionPhase) -> SessionPhase {
    match phase {
        MobileConnectionPhase::Connected => SessionPhase::Connected,
        MobileConnectionPhase::Failed => SessionPhase::Failed,
        MobileConnectionPhase::Stopped
        | MobileConnectionPhase::Starting
        | MobileConnectionPhase::Connecting
        | MobileConnectionPhase::Reconnecting
        | MobileConnectionPhase::Degraded => SessionPhase::Reconnecting,
    }
}

fn project_bearer(bearer: &styrened::mobile::MobileBearerObservation) -> Bearer {
    Bearer {
        kind: match bearer.kind {
            MobileBearerKind::Tcp => BearerKind::Tcp,
            MobileBearerKind::BluetoothRnode => BearerKind::BluetoothRnode,
            MobileBearerKind::AndroidUsb => BearerKind::AndroidUsb,
        },
        state: match bearer.state {
            MobileBearerState::Connected => BearerState::Connected,
            MobileBearerState::Connecting | MobileBearerState::Reconnecting => {
                BearerState::Reconnecting
            }
            MobileBearerState::Disconnected => BearerState::Disconnected,
            MobileBearerState::Unavailable => BearerState::Unavailable,
            MobileBearerState::Unverified => BearerState::Unverified,
        },
        reason: bearer.reason.map(|reason| {
            match reason {
                MobileBearerReason::NotConfigured => "not_configured",
                MobileBearerReason::PermissionDenied => "permission_denied",
                MobileBearerReason::ConnectionInterrupted => "connection_interrupted",
                MobileBearerReason::PhysicalEvidenceAbsent => "physical_evidence_absent",
            }
            .into()
        }),
    }
}

fn project_propagation(generation: u64, snapshot: &MobilePropagationSnapshot) -> PropagationUpdate {
    PropagationUpdate {
        generation,
        selected_destination: snapshot.selected_destination.clone(),
        ready: snapshot.ready,
        sync_state: match snapshot.sync_state {
            MobilePropagationSyncState::Idle => SyncState::Idle,
            MobilePropagationSyncState::InProgress => SyncState::InProgress,
            MobilePropagationSyncState::Complete => SyncState::Complete,
            MobilePropagationSyncState::Failed => SyncState::Failed,
        },
        new_messages: snapshot.new_messages,
        failure: snapshot.failure.as_ref().map(|failure| TypedFailure {
            code: format!("{:?}", failure.code).to_ascii_lowercase(),
            retryable: failure.retryable,
        }),
        automatic_sync_enabled: snapshot.automatic_sync_enabled,
        automatic_sync_cooldown_secs: snapshot.automatic_sync_cooldown_secs,
        sync_deadline_secs: snapshot.sync_deadline_secs,
        progress: snapshot.in_flight.as_ref().map(|progress| PropagationProgress {
            attempt_id: progress.attempt_id.clone(),
            received_count: progress.received_count,
            received_bytes: progress.received_bytes,
        }),
        candidates: snapshot
            .candidates
            .iter()
            .map(|candidate| PropagationCandidate {
                destination_hash: candidate.destination_hash.clone(),
                active: candidate.active,
                observed_at: candidate.observed_at,
                age_secs: candidate.age_secs,
                policy: candidate.policy.as_ref().map(|policy| PropagationPolicy {
                    transfer_limit_kb: policy.transfer_limit_kb,
                    sync_limit_kb: policy.sync_limit_kb,
                    stamp_cost: policy.stamp_cost,
                    stamp_flexibility: policy.stamp_flexibility,
                }),
            })
            .collect(),
        selected_policy: snapshot.selected_policy.as_ref().map(|policy| PropagationPolicy {
            transfer_limit_kb: policy.transfer_limit_kb,
            sync_limit_kb: policy.sync_limit_kb,
            stamp_cost: policy.stamp_cost,
            stamp_flexibility: policy.stamp_flexibility,
        }),
    }
}

fn mobile_config(endpoint: &str) -> Result<MobileConfig, String> {
    let root = mobile_data_root()?;
    let identity_backend = if cfg!(target_os = "android") {
        IdentityBackend::AndroidKeystore
    } else if cfg!(target_abi = "sim") {
        IdentityBackend::PlaintextFile
    } else {
        IdentityBackend::Keychain
    };
    Ok(MobileConfig {
        config_dir: root.join("config"),
        data_dir: root.join("data"),
        hub_address: None,
        hub_delivery_hash: None,
        display_name: None,
        identity_backend,
        interfaces: vec![MobileInterfaceConfig::TcpClient { remote_address: endpoint.into() }],
        enable_rnode_channel: false,
    })
}

#[cfg(target_os = "android")]
fn mobile_data_root() -> Result<PathBuf, String> {
    manganis::android::with_activity(|env, activity| {
        let files =
            env.call_method(activity, "getFilesDir", "()Ljava/io/File;", &[]).ok()?.l().ok()?;
        let path = env
            .call_method(&files, "getAbsolutePath", "()Ljava/lang/String;", &[])
            .ok()?
            .l()
            .ok()?;
        let path = manganis::jni::objects::JString::from(path);
        let path = env.get_string(&path).ok()?;
        Some(PathBuf::from(path.to_string_lossy().into_owned()).join("Styrene").join("Mobile"))
    })
    .ok_or_else(|| "Android application files directory is unavailable".into())
}

#[cfg(not(target_os = "android"))]
fn mobile_data_root() -> Result<PathBuf, String> {
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| "HOME is unavailable in the application container".to_string())?;
    Ok(home.join("Library").join("Application Support").join("Styrene").join("Mobile"))
}

fn failed_update(generation: u64, code: &str, _message: String) -> SessionUpdate {
    let fixture = MobileFixture {
        id: "embedded-live-failed".into(),
        profile: Profile::Live,
        generation,
        session: Session {
            phase: SessionPhase::Failed,
            identity_hash: String::new(),
            endpoint: Some(DEFAULT_ENDPOINT.into()),
            failure: Some(TypedFailure { code: code.into(), retryable: true }),
        },
        bearers: Vec::new(),
        peers: Vec::new(),
        conversations: Vec::new(),
        messages: Vec::new(),
        propagation: Propagation {
            selected_destination: None,
            ready: false,
            sync_state: SyncState::Idle,
            new_messages: 0,
            failure: None,
        },
        event: None,
        expected: ExpectedProjection {
            fixture_banner: false,
            live_network_enabled: true,
            peer_count: 0,
            conversation_count: 0,
            message_count: 0,
            accessibility_ids: Vec::new(),
        },
    };
    SessionUpdate { propagation: PropagationUpdate::from_fixture(&fixture), fixture }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initial_projection_is_live_starting_state() {
        let update = MobileSession::starting_update();

        assert_eq!(update.fixture.profile, Profile::Live);
        assert_eq!(update.fixture.session.phase, SessionPhase::Starting);
        assert!(update.fixture.session.failure.is_none());
    }

    #[test]
    fn unknown_backend_delivery_method_is_not_reported_as_direct() {
        assert_eq!(project_method(None), DeliveryMethod::Unknown);
        assert_eq!(project_method(Some("future-method")), DeliveryMethod::Unknown);
        assert_eq!(project_method(Some("direct")), DeliveryMethod::Direct);
    }
}
