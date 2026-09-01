use std::path::PathBuf;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use async_channel::{Receiver, Sender};
use styrene_ipc::DaemonIdentity;
use styrene_ipc::types::{
    IdentityBackupImport, IdentityCustodyAuthentication as BackendCustodyAuthentication,
    IdentityCustodyAvailability as BackendCustodyAvailability,
    IdentityCustodyBackend as BackendCustodyBackend,
    IdentityCustodyDowngrade as BackendCustodyDowngrade,
    IdentityCustodyFailureCode as BackendCustodyFailureCode, IdentityCustodyInfo,
    IdentityCustodyProtection as BackendCustodyProtection, MessageInfo, MessageLifecycleState,
    MessageRetryIneligibilityReason as BackendRetryIneligibilityReason,
};
#[cfg(target_os = "ios")]
use styrene_ui_platform::DeviceAuthenticationOutcome;
use styrene_ui_platform::{AndroidUsbAttachment, OpaqueDocument};
use styrene_ui_state::{
    Bearer, BearerKind, BearerState, Conversation, DeliveryEvidence, DeliveryMethod,
    ExpectedProjection, IdentityBackupProtection, IdentityCustody, IdentityCustodyAuthentication,
    IdentityCustodyAvailability, IdentityCustodyBackend, IdentityCustodyDowngrade,
    IdentityCustodyProtection, IdentityRecoveryFailure, Message, MessageAttachment,
    MessageAttachmentTransfer, MessageAttempt, MessageAuthentication, MessageDeliveryKind,
    MessageDeliveryObservation, MessageDeliveryState, MessageDetails, MessageInterfaceObservation,
    MessageLifecycle, MessagePropagationCorrelation, MessageRetryIneligibilityReason,
    MessageRouteObservation, MessageRouteOutcome, MessageStampState, MobileAction,
    MobileActionKind, MobileFixture, Peer, PeerSource, PersistenceState, Profile, Propagation,
    PropagationCandidate, PropagationEvidence, PropagationPolicy, PropagationProgress,
    PropagationReadiness, PropagationSynchronization, PropagationTerminalOutcome,
    PropagationTriggerSource, PropagationUpdate, Session, SessionPhase, SessionRuntime, SyncState,
    TransportEvidence, TypedFailure,
};
use styrened::mobile::{
    IdentityBackend, MobileBearerKind, MobileBearerReason, MobileBearerState, MobileConfig,
    MobileConnectionPhase, MobileDeliveryMethod, MobileIdentityPresence,
    MobileIdentityRecoveryError, MobileInterfaceConfig, MobileNode, MobilePeerAspect,
    MobilePropagationReadiness, MobilePropagationSnapshot, MobilePropagationSyncState,
    MobilePropagationTerminalOutcome, MobilePropagationTriggerSource, MobileRuntimeState,
    MobileSendRequest, MobileStateEvent, MobileStateSubscription, MobileStateSubscriptionError,
    MobileUsbFallbackDisposition, persist_mobile_tcp_endpoint,
};
#[cfg(target_os = "android")]
use styrened::mobile::{
    MobileRNodeAttempt, MobileRNodeBearer, MobileRNodeWriteBatch, RNodeBearerInfo, RNodeBearerKind,
};

const DEFAULT_ENDPOINT: &str = "rns.styrene.io:4242";
const ACTION_CAPACITY: usize = 64;
const SUBSCRIPTION_RETRY_DELAY: Duration = Duration::from_millis(250);

#[derive(Clone)]
pub struct MobileSession {
    actions: Sender<MobileAction>,
    updates: Receiver<SessionView>,
    bootstrap_requests: Sender<BootstrapRequest>,
    usb_fallbacks: Sender<UsbFallbackRequest>,
    usb_connections: Sender<UsbConnectionRequest>,
    usb_probes: Sender<UsbProbeRequest>,
    backup_exports: Sender<BackupExportRequest>,
    #[cfg(target_os = "ios")]
    backend_nodes: Receiver<Arc<MobileNode>>,
}

#[derive(Clone, Debug)]
pub struct SessionUpdate {
    pub fixture: MobileFixture,
    pub propagation: PropagationUpdate,
}

#[derive(Clone, Debug)]
pub enum SessionView {
    Inspecting,
    Bootstrap { generation: u64 },
    Running(SessionUpdate),
    Failed(SessionUpdate),
}

impl SessionView {
    pub const fn generation(&self) -> Option<u64> {
        match self {
            Self::Inspecting => None,
            Self::Bootstrap { generation } => Some(*generation),
            Self::Running(update) | Self::Failed(update) => Some(update.fixture.generation),
        }
    }
}

struct SessionOwner {
    generation: u64,
    backend_generation: u64,
    config: MobileConfig,
    node: Arc<MobileNode>,
    state_events: MobileStateSubscription,
    updates: Sender<SessionView>,
    usb_worker: Option<UsbWorker>,
}

struct UsbFallbackRequest {
    response: Sender<Result<(), String>>,
}

struct UsbConnectionRequest {
    kind: UsbConnectionRequestKind,
    response: Sender<Result<(), String>>,
}

struct UsbProbeRequest {
    response: Sender<Result<AndroidUsbProbeOutcome, String>>,
}

struct BackupExportRequest {
    generation: u64,
    protection: Vec<u8>,
    response: Sender<Result<OpaqueDocument, IdentityRecoveryFailure>>,
}

enum BootstrapRequestKind {
    Create,
    Restore { document: OpaqueDocument, protection: Vec<u8> },
}

struct BootstrapRequest {
    generation: u64,
    kind: BootstrapRequestKind,
    response: Sender<Result<(), IdentityRecoveryFailure>>,
}

impl Drop for BootstrapRequestKind {
    fn drop(&mut self) {
        if let Self::Restore { protection, .. } = self {
            protection.fill(0);
        }
    }
}

impl Drop for BackupExportRequest {
    fn drop(&mut self) {
        self.protection.fill(0);
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AndroidUsbProbeOutcome {
    pub frame_bytes: usize,
}

enum UsbConnectionRequestKind {
    Connect(AndroidUsbAttachment),
    PermissionDenied,
}

struct UsbWorker {
    cancel: Sender<()>,
    probes: Sender<UsbProbeRequest>,
    task: tokio::task::JoinHandle<()>,
    #[cfg(target_os = "android")]
    attempt: Arc<tokio::sync::Mutex<Option<MobileRNodeAttempt>>>,
}

impl MobileSession {
    pub fn start() -> Self {
        let (actions, action_receiver) = async_channel::bounded(ACTION_CAPACITY);
        let (update_sender, updates) = async_channel::bounded(1);
        let (bootstrap_requests, bootstrap_request_receiver) = async_channel::bounded(1);
        let (usb_fallbacks, usb_fallback_receiver) = async_channel::bounded(1);
        let (usb_connections, usb_connection_receiver) = async_channel::bounded(1);
        let (usb_probes, usb_probe_receiver) = async_channel::bounded(1);
        let (backup_exports, backup_export_receiver) = async_channel::bounded(1);
        #[cfg(target_os = "ios")]
        let (backend_node_sender, backend_nodes) = async_channel::bounded(1);
        let startup_failures = update_sender.clone();
        if let Err(error) =
            thread::Builder::new().name("styrene-mobile-session".into()).spawn(move || {
                run_owner(
                    action_receiver,
                    bootstrap_request_receiver,
                    usb_fallback_receiver,
                    usb_connection_receiver,
                    usb_probe_receiver,
                    backup_export_receiver,
                    #[cfg(target_os = "ios")]
                    backend_node_sender,
                    update_sender,
                );
            })
        {
            let _ = startup_failures.force_send(SessionView::Failed(failed_update(
                1,
                "session_thread_start_failed",
                error.to_string(),
            )));
        }
        Self {
            actions,
            updates,
            bootstrap_requests,
            usb_fallbacks,
            usb_connections,
            usb_probes,
            backup_exports,
            #[cfg(target_os = "ios")]
            backend_nodes,
        }
    }

    pub fn dispatch(&self, action: MobileAction) {
        let _ = self.actions.try_send(action);
    }

    pub async fn next_update(&self) -> Option<SessionView> {
        self.updates.recv().await.ok()
    }

    pub async fn confirm_identity_creation(
        &self,
        generation: u64,
    ) -> Result<(), IdentityRecoveryFailure> {
        self.request_bootstrap(generation, BootstrapRequestKind::Create).await
    }

    pub async fn restore_identity(
        &self,
        generation: u64,
        document: OpaqueDocument,
        protection: IdentityBackupProtection,
    ) -> Result<(), IdentityRecoveryFailure> {
        self.request_bootstrap(
            generation,
            BootstrapRequestKind::Restore { document, protection: protection.into_bytes() },
        )
        .await
    }

    async fn request_bootstrap(
        &self,
        generation: u64,
        kind: BootstrapRequestKind,
    ) -> Result<(), IdentityRecoveryFailure> {
        let (response, result) = async_channel::bounded(1);
        self.bootstrap_requests
            .send(BootstrapRequest { generation, kind, response })
            .await
            .map_err(|_| IdentityRecoveryFailure::SessionUnavailable)?;
        result.recv().await.map_err(|_| IdentityRecoveryFailure::SessionUnavailable)?
    }

    pub async fn request_android_usb_fallback(&self) -> Result<(), String> {
        let (response, result) = async_channel::bounded(1);
        self.usb_fallbacks
            .send(UsbFallbackRequest { response })
            .await
            .map_err(|_| "mobile session is unavailable".to_string())?;
        result
            .recv()
            .await
            .map_err(|_| "mobile session closed the USB fallback request".to_string())?
    }

    pub async fn connect_android_usb(
        &self,
        attachment: AndroidUsbAttachment,
    ) -> Result<(), String> {
        let (response, result) = async_channel::bounded(1);
        self.usb_connections
            .send(UsbConnectionRequest {
                kind: UsbConnectionRequestKind::Connect(attachment),
                response,
            })
            .await
            .map_err(|_| "mobile session is unavailable".to_string())?;
        result
            .recv()
            .await
            .map_err(|_| "mobile session closed the USB connection request".to_string())?
    }

    pub async fn report_android_usb_permission_denied(&self) -> Result<(), String> {
        let (response, result) = async_channel::bounded(1);
        self.usb_connections
            .send(UsbConnectionRequest {
                kind: UsbConnectionRequestKind::PermissionDenied,
                response,
            })
            .await
            .map_err(|_| "mobile session is unavailable".to_string())?;
        result
            .recv()
            .await
            .map_err(|_| "mobile session closed the USB denial report".to_string())?
    }

    pub async fn probe_android_usb(&self) -> Result<AndroidUsbProbeOutcome, String> {
        let (response, result) = async_channel::bounded(1);
        self.usb_probes
            .send(UsbProbeRequest { response })
            .await
            .map_err(|_| "mobile session is unavailable".to_string())?;
        result
            .recv()
            .await
            .map_err(|_| "mobile session closed the USB probe request".to_string())?
    }

    pub async fn export_portable_identity_backup(
        &self,
        generation: u64,
        protection: IdentityBackupProtection,
    ) -> Result<OpaqueDocument, IdentityRecoveryFailure> {
        let (response, result) = async_channel::bounded(1);
        self.backup_exports
            .send(BackupExportRequest { generation, protection: protection.into_bytes(), response })
            .await
            .map_err(|_| IdentityRecoveryFailure::SessionUnavailable)?;
        result.recv().await.map_err(|_| IdentityRecoveryFailure::SessionUnavailable)?
    }

    #[cfg(target_os = "ios")]
    pub async fn backend_node(&self) -> Result<Arc<MobileNode>, String> {
        self.backend_nodes
            .recv()
            .await
            .map_err(|_| "mobile session closed before publishing its backend node".into())
    }

    pub const fn starting_view() -> SessionView {
        SessionView::Inspecting
    }
}

fn run_owner(
    actions: Receiver<MobileAction>,
    bootstrap_requests: Receiver<BootstrapRequest>,
    usb_fallbacks: Receiver<UsbFallbackRequest>,
    usb_connections: Receiver<UsbConnectionRequest>,
    usb_probes: Receiver<UsbProbeRequest>,
    backup_exports: Receiver<BackupExportRequest>,
    #[cfg(target_os = "ios")] backend_nodes: Sender<Arc<MobileNode>>,
    updates: Sender<SessionView>,
) {
    #[cfg(target_os = "ios")]
    {
        if styrene_ui_apple_bridge::app_lock_requires_authentication() {
            let _ = updates.force_send(authenticating_update());
            let outcome = styrene_ui_apple_bridge::authenticate_device_owner(
                "unlock your private mesh communications",
            );
            if outcome != DeviceAuthenticationOutcome::Authenticated {
                let code = match outcome {
                    DeviceAuthenticationOutcome::Cancelled => "app_unlock_cancelled",
                    DeviceAuthenticationOutcome::Unavailable => "app_unlock_unavailable",
                    DeviceAuthenticationOutcome::Failed => "app_unlock_failed",
                    DeviceAuthenticationOutcome::Authenticated => unreachable!(),
                };
                let _ =
                    updates.force_send(SessionView::Failed(failed_update(1, code, String::new())));
                return;
            }
            styrene_ui_apple_bridge::record_app_lock_authentication();
        }
    }
    let runtime = match tokio::runtime::Builder::new_multi_thread().enable_all().build() {
        Ok(runtime) => runtime,
        Err(error) => {
            let _ = updates.force_send(SessionView::Failed(failed_update(
                1,
                "runtime_start_failed",
                error.to_string(),
            )));
            return;
        }
    };
    runtime.block_on(owner_loop(
        actions,
        bootstrap_requests,
        usb_fallbacks,
        usb_connections,
        usb_probes,
        backup_exports,
        #[cfg(target_os = "ios")]
        backend_nodes,
        updates,
    ));
}

#[cfg(target_os = "ios")]
fn authenticating_update() -> SessionView {
    let mut update = failed_update(1, "authenticating", String::new());
    update.fixture.id = "embedded-live-authenticating".into();
    update.fixture.session.runtime = SessionRuntime::Ready;
    update.fixture.session.phase = SessionPhase::Starting;
    update.fixture.session.failure = None;
    SessionView::Running(update)
}

async fn owner_loop(
    actions: Receiver<MobileAction>,
    bootstrap_requests: Receiver<BootstrapRequest>,
    usb_fallbacks: Receiver<UsbFallbackRequest>,
    usb_connections: Receiver<UsbConnectionRequest>,
    usb_probes: Receiver<UsbProbeRequest>,
    backup_exports: Receiver<BackupExportRequest>,
    #[cfg(target_os = "ios")] backend_nodes: Sender<Arc<MobileNode>>,
    updates: Sender<SessionView>,
) {
    let generation = 1;
    let Some(mut owner) = prepare_owner(
        generation,
        bootstrap_requests,
        #[cfg(target_os = "ios")]
        backend_nodes,
        updates,
    )
    .await
    else {
        return;
    };
    publish_snapshot(&owner.node, owner.generation, owner.backend_generation, &owner.updates).await;

    loop {
        tokio::select! {
            event = owner.state_events.recv() => owner.handle_event(event).await,
            request = usb_fallbacks.recv(), if !usb_fallbacks.is_closed() => {
                let Ok(request) = request else {
                    continue;
                };
                let result = match owner.node.platform_service().request_android_usb_fallback().await {
                    MobileUsbFallbackDisposition::Accepted => Ok(()),
                    MobileUsbFallbackDisposition::BluetoothActive => {
                        Err("approved Bluetooth is active; USB fallback was not requested".into())
                    }
                };
                let _ = request.response.try_send(result);
            }
            request = usb_connections.recv(), if !usb_connections.is_closed() => {
                let Ok(request) = request else {
                    continue;
                };
                let result = match request.kind {
                    UsbConnectionRequestKind::Connect(attachment) => {
                        owner.start_usb_worker(attachment).await
                    }
                    UsbConnectionRequestKind::PermissionDenied => owner.report_usb_denied().await,
                };
                let _ = request.response.try_send(result);
            }
            request = usb_probes.recv(), if !usb_probes.is_closed() => {
                let Ok(request) = request else {
                    continue;
                };
                if let Some(worker) = &owner.usb_worker {
                    if let Err(error) = worker.probes.try_send(request) {
                        let request = error.into_inner();
                        let _ = request.response.try_send(Err("Android USB probe is already pending".into()));
                    }
                } else {
                    let _ = request.response.try_send(Err("Android USB is not active".into()));
                }
            }
            request = backup_exports.recv(), if !backup_exports.is_closed() => {
                let Ok(request) = request else {
                    continue;
                };
                let result = owner.export_identity_backup(request.generation, &request.protection).await;
                let _ = request.response.try_send(result);
            }
            action = actions.recv() => {
                let Ok(action) = action else {
                    owner.stop_usb_worker().await;
                    let _ = owner.node.shutdown().await;
                    return;
                };
                if !owner.handle_action(action).await {
                    return;
                }
            }
        }
    }
}

async fn prepare_owner(
    generation: u64,
    bootstrap_requests: Receiver<BootstrapRequest>,
    #[cfg(target_os = "ios")] backend_nodes: Sender<Arc<MobileNode>>,
    updates: Sender<SessionView>,
) -> Option<SessionOwner> {
    let config = match mobile_config(DEFAULT_ENDPOINT) {
        Ok(config) => config,
        Err(error) => {
            let _ = updates.force_send(SessionView::Failed(failed_update(
                generation,
                "platform_paths_unavailable",
                error,
            )));
            return None;
        }
    };
    let presence = match MobileNode::identity_presence(&config).await {
        Ok(presence) => presence,
        Err(error) => {
            let _ = updates.force_send(SessionView::Failed(failed_update(
                generation,
                "identity_presence_failed",
                error.to_string(),
            )));
            return None;
        }
    };
    let pending_response = if presence == MobileIdentityPresence::Absent {
        let _ = updates.force_send(SessionView::Bootstrap { generation });
        loop {
            let Ok(request) = bootstrap_requests.recv().await else {
                return None;
            };
            if request.generation != generation {
                let _ = request.response.try_send(Err(IdentityRecoveryFailure::SessionUnavailable));
                continue;
            }
            if let BootstrapRequestKind::Restore { document, protection } = &request.kind {
                let mut import = IdentityBackupImport::default();
                import.encrypted_bytes = document.clone().into_bytes();
                if let Err(error) =
                    MobileNode::restore_identity_before_boot(&config, import, protection).await
                {
                    let _ = request.response.try_send(Err(identity_recovery_failure(error)));
                    continue;
                }
                if MobileNode::identity_presence(&config).await
                    != Ok(MobileIdentityPresence::Present)
                {
                    let _ =
                        request.response.try_send(Err(IdentityRecoveryFailure::CustodyUnavailable));
                    let _ = updates.force_send(SessionView::Failed(failed_update(
                        generation,
                        "identity_restore_verification_failed",
                        String::new(),
                    )));
                    return None;
                }
            }
            break Some(request.response.clone());
        }
    } else {
        None
    };
    let node = match MobileNode::boot(config.clone()).await {
        Ok(node) => Arc::new(node),
        Err(error) => {
            if let Some(response) = pending_response {
                let _ = response.try_send(Err(IdentityRecoveryFailure::SessionUnavailable));
            }
            let _ = updates.force_send(SessionView::Failed(failed_update(
                generation,
                "embedded_start_failed",
                error.to_string(),
            )));
            return None;
        }
    };
    #[cfg(target_os = "ios")]
    styrene_ui_apple_bridge::record_app_lock_setup_complete();
    if let Some(response) = pending_response {
        let _ = response.try_send(Ok(()));
    }
    let state_events = node.subscribe_state_events();
    #[cfg(target_os = "ios")]
    let _ = backend_nodes.try_send(Arc::clone(&node));
    let backend_generation = node.session_snapshot().await.generation;
    Some(SessionOwner {
        generation,
        backend_generation,
        config,
        node,
        state_events,
        updates,
        usb_worker: None,
    })
}

const fn identity_recovery_failure(error: MobileIdentityRecoveryError) -> IdentityRecoveryFailure {
    match error {
        MobileIdentityRecoveryError::ProtectionRequired => {
            IdentityRecoveryFailure::ProtectionRequired
        }
        MobileIdentityRecoveryError::ProtectionTooLarge => {
            IdentityRecoveryFailure::ProtectionTooLarge
        }
        MobileIdentityRecoveryError::ArtifactTooLarge => IdentityRecoveryFailure::ArtifactTooLarge,
        MobileIdentityRecoveryError::InvalidBackup => IdentityRecoveryFailure::InvalidBackup,
        MobileIdentityRecoveryError::AuthenticationFailed => {
            IdentityRecoveryFailure::AuthenticationFailed
        }
        MobileIdentityRecoveryError::CustodyUnavailable => {
            IdentityRecoveryFailure::CustodyUnavailable
        }
        MobileIdentityRecoveryError::IdentityConflict => IdentityRecoveryFailure::IdentityConflict,
        MobileIdentityRecoveryError::UnsupportedBackend => {
            IdentityRecoveryFailure::UnsupportedBackend
        }
    }
}

impl SessionOwner {
    async fn export_identity_backup(
        &self,
        generation: u64,
        protection: &[u8],
    ) -> Result<OpaqueDocument, IdentityRecoveryFailure> {
        if generation == self.generation {
            self.node
                .export_portable_identity_backup(protection)
                .await
                .map_err(identity_recovery_failure)
                .and_then(|export| {
                    OpaqueDocument::new(export.encrypted_bytes)
                        .map_err(|_| IdentityRecoveryFailure::ArtifactTooLarge)
                })
        } else {
            Err(IdentityRecoveryFailure::SessionUnavailable)
        }
    }

    async fn report_usb_denied(&mut self) -> Result<(), String> {
        self.stop_usb_worker().await;
        self.node
            .platform_service()
            .report(styrened::mobile::MobileBearerObservation {
                kind: MobileBearerKind::AndroidUsb,
                state: MobileBearerState::Disconnected,
                reason: Some(MobileBearerReason::PermissionDenied),
            })
            .await
            .map_err(str::to_owned)
    }

    async fn start_usb_worker(&mut self, attachment: AndroidUsbAttachment) -> Result<(), String> {
        self.stop_usb_worker().await;
        #[cfg(target_os = "android")]
        {
            let (cancel, cancelled) = async_channel::bounded(1);
            let (probes, probe_requests) = async_channel::bounded(1);
            let node = Arc::clone(&self.node);
            let attempt = Arc::new(tokio::sync::Mutex::new(None));
            let task = tokio::spawn(run_android_usb(
                node,
                attachment,
                cancelled,
                probe_requests,
                Arc::clone(&attempt),
            ));
            self.usb_worker = Some(UsbWorker { cancel, probes, task, attempt });
            Ok(())
        }
        #[cfg(not(target_os = "android"))]
        {
            let _ = attachment;
            Err("Android USB is unavailable on this platform".into())
        }
    }

    async fn stop_usb_worker(&mut self) {
        if let Some(mut worker) = self.usb_worker.take() {
            let _ = worker.cancel.try_send(());
            if tokio::time::timeout(Duration::from_secs(2), &mut worker.task).await.is_err() {
                worker.task.abort();
                let _ = worker.task.await;
            }
            #[cfg(target_os = "android")]
            if let Some(attempt) = worker.attempt.lock().await.take() {
                let _ = self
                    .node
                    .stop_rnode_bytes(attempt, MobileBearerReason::ConnectionInterrupted)
                    .await;
            }
        }
    }

    async fn handle_event(
        &mut self,
        event: Result<MobileStateEvent, MobileStateSubscriptionError>,
    ) {
        match event {
            Ok(event) => {
                if advance_generation_if_changed(
                    &mut self.backend_generation,
                    event.generation,
                    &mut self.generation,
                ) {
                    self.state_events = self.node.subscribe_state_events();
                }
                publish_snapshot(
                    &self.node,
                    self.generation,
                    self.backend_generation,
                    &self.updates,
                )
                .await;
            }
            Err(MobileStateSubscriptionError::Lagged(_)) => {
                self.synchronize_generation().await;
                self.state_events = self.node.subscribe_state_events();
                publish_snapshot(
                    &self.node,
                    self.generation,
                    self.backend_generation,
                    &self.updates,
                )
                .await;
            }
            Err(MobileStateSubscriptionError::Closed) => {
                tokio::time::sleep(SUBSCRIPTION_RETRY_DELAY).await;
                self.state_events = self.node.subscribe_state_events();
                self.synchronize_generation().await;
                publish_snapshot_with_failure(
                    &self.node,
                    self.generation,
                    self.backend_generation,
                    &self.updates,
                    messaging_failure("state_subscription_closed", true),
                )
                .await;
            }
        }
    }

    async fn handle_action(&mut self, action: MobileAction) -> bool {
        if action.generation != self.generation {
            return true;
        }
        if self.synchronize_generation().await {
            self.state_events = self.node.subscribe_state_events();
            publish_snapshot(&self.node, self.generation, self.backend_generation, &self.updates)
                .await;
            return true;
        }
        if let MobileActionKind::ApplyEndpoint { endpoint } = &action.kind {
            if let Err(error) = persist_mobile_tcp_endpoint(&self.config.config_dir, endpoint) {
                publish_snapshot_with_failure(
                    &self.node,
                    self.generation,
                    self.backend_generation,
                    &self.updates,
                    TypedFailure {
                        code: format!("{:?}", error.code()).to_ascii_lowercase(),
                        retryable: error.retryable(),
                    },
                )
                .await;
                return true;
            }
            self.stop_usb_worker().await;
            let _ = self.node.shutdown().await;
            self.generation = self.generation.saturating_add(1);
            self.config.interfaces =
                vec![MobileInterfaceConfig::TcpClient { remote_address: endpoint.clone() }];
            let replacement = match MobileNode::boot(self.config.clone()).await {
                Ok(replacement) => replacement,
                Err(error) => {
                    let _ = self.updates.force_send(SessionView::Failed(failed_update(
                        self.generation,
                        "embedded_restart_failed",
                        error.to_string(),
                    )));
                    return false;
                }
            };
            self.node = Arc::new(replacement);
            self.state_events = self.node.subscribe_state_events();
            self.backend_generation = self.node.session_snapshot().await.generation;
        } else if let Err(failure) = execute_action(&self.node, action.kind).await {
            publish_snapshot_with_failure(
                &self.node,
                self.generation,
                self.backend_generation,
                &self.updates,
                failure,
            )
            .await;
            return true;
        }
        publish_snapshot(&self.node, self.generation, self.backend_generation, &self.updates).await;
        true
    }

    async fn synchronize_generation(&mut self) -> bool {
        synchronize_backend_generation(
            &self.node,
            &mut self.backend_generation,
            &mut self.generation,
        )
        .await
    }
}

#[cfg(target_os = "android")]
async fn run_android_usb(
    node: Arc<MobileNode>,
    attachment: AndroidUsbAttachment,
    cancelled: Receiver<()>,
    probe_requests: Receiver<UsbProbeRequest>,
    active_attempt: Arc<tokio::sync::Mutex<Option<MobileRNodeAttempt>>>,
) {
    use styrened::mobile::{MobileBearerObservation, MobileBearerReason};

    let Ok(mut link) = crate::android_usb::AndroidUsbLink::open(attachment.clone()).await else {
        let _ = node
            .platform_service()
            .report(MobileBearerObservation {
                kind: MobileBearerKind::AndroidUsb,
                state: MobileBearerState::Unavailable,
                reason: Some(MobileBearerReason::ConnectionInterrupted),
            })
            .await;
        return;
    };
    let Ok(start) = node
        .start_rnode_bytes(
            MobileRNodeBearer::AndroidUsb,
            RNodeBearerInfo {
                kind: RNodeBearerKind::AndroidUsb,
                negotiated_mtu: None,
                max_write_size: Some(crate::android_usb::AndroidUsbLink::MAX_WRITE_SIZE),
            },
        )
        .await
    else {
        link.close();
        return;
    };
    let attempt = start.attempt;
    *active_attempt.lock().await = Some(attempt);
    let result = async {
        for frame in start.writes {
            link.write(frame).await.map_err(|error| error.code)?;
        }
        let startup_deadline = tokio::time::Instant::now() + Duration::from_secs(8);
        let mut attachment_check = tokio::time::Instant::now();
        loop {
            if cancelled.try_recv().is_ok() {
                return Ok::<(), String>(());
            }
            if attachment_check.elapsed() >= Duration::from_secs(1) {
                let attached = crate::platform::native_android_usb_attachments()
                    .await
                    .map_err(|error| error.code)?
                    .contains(&attachment);
                if !attached {
                    return Err("android_usb_detached".into());
                }
                attachment_check = tokio::time::Instant::now();
            }
            if let Ok(request) = probe_requests.try_recv() {
                let result = probe_android_usb(&node, attempt, &mut link).await;
                let _ = request.response.try_send(result);
            }
            if let Some(bytes) = link.read().await.map_err(|error| error.code)? {
                for response in node.submit_rnode_bytes(attempt, &bytes).await? {
                    link.write(response).await.map_err(|error| error.code)?;
                }
            }
            if let Some(batch) = node.poll_rnode_bytes(attempt).await? {
                write_android_usb_handoff(&node, attempt, &link, batch).await?;
            }
            let connected = node
                .session_snapshot()
                .await
                .bearer(MobileBearerKind::AndroidUsb)
                .is_some_and(|bearer| bearer.state == MobileBearerState::Connected);
            if !connected && tokio::time::Instant::now() >= startup_deadline {
                return Err("android_usb_rnode_startup_timeout".into());
            }
        }
    }
    .await;
    if let Ok(shutdown) =
        node.stop_rnode_bytes(attempt, MobileBearerReason::ConnectionInterrupted).await
    {
        for write in shutdown {
            let _ = link.write(write).await;
        }
    }
    *active_attempt.lock().await = None;
    link.close();
    let _ = result;
}

#[cfg(target_os = "android")]
async fn probe_android_usb(
    node: &MobileNode,
    attempt: MobileRNodeAttempt,
    link: &mut crate::android_usb::AndroidUsbLink,
) -> Result<AndroidUsbProbeOutcome, String> {
    let connected = node
        .session_snapshot()
        .await
        .bearer(MobileBearerKind::AndroidUsb)
        .is_some_and(|bearer| bearer.state == MobileBearerState::Connected);
    if !connected {
        return Err("Android USB RNode is not connected".into());
    }
    while let Some(batch) = node.poll_rnode_bytes(attempt).await? {
        write_android_usb_handoff(node, attempt, link, batch).await?;
    }
    let outcome = node.announce_outcome().await.map_err(|error| error.to_string())?;
    if !outcome.local_dispatch_accepted {
        return Err("local transport rejected the announce".into());
    }

    let deadline = tokio::time::Instant::now() + Duration::from_secs(2);
    loop {
        if let Some(batch) = node.poll_rnode_bytes(attempt).await? {
            let frame_bytes = write_android_usb_handoff(node, attempt, link, batch).await?;
            return Ok(AndroidUsbProbeOutcome { frame_bytes });
        }
        if tokio::time::Instant::now() >= deadline {
            return Err("RNode packet handoff timed out".into());
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}

#[cfg(target_os = "android")]
async fn write_android_usb_handoff(
    node: &MobileNode,
    attempt: MobileRNodeAttempt,
    link: &crate::android_usb::AndroidUsbLink,
    batch: MobileRNodeWriteBatch,
) -> Result<usize, String> {
    let frame_bytes = batch.writes.iter().map(Vec::len).sum();
    for write in batch.writes {
        if let Err(error) = link.write(write).await {
            let _ = node.fail_rnode_write(attempt, batch.handoff).await;
            return Err(error.code);
        }
    }
    if !node.complete_rnode_write(attempt, batch.handoff).await? {
        return Err("RNode write handoff became stale before completion".into());
    }
    Ok(frame_bytes)
}

async fn synchronize_backend_generation(
    node: &MobileNode,
    backend_generation: &mut u64,
    generation: &mut u64,
) -> bool {
    let observed = node.session_snapshot().await.generation;
    advance_generation_if_changed(backend_generation, observed, generation)
}

fn advance_generation_if_changed(
    backend_generation: &mut u64,
    observed: u64,
    generation: &mut u64,
) -> bool {
    if observed <= *backend_generation {
        return false;
    }
    *backend_generation = observed;
    *generation = generation.saturating_add(1);
    true
}

async fn execute_action(node: &MobileNode, action: MobileActionKind) -> Result<(), TypedFailure> {
    match action {
        MobileActionKind::ApplyEndpoint { .. } => {}
        MobileActionKind::SetActiveConversation { peer_hash } => {
            node.set_active_conversation(peer_hash.as_deref()).await.map_err(|error| {
                messaging_failure("active_conversation_failed", error.retryable)
            })?;
        }
        MobileActionKind::StartConversation { peer_hash } => {
            node.start_conversation(&peer_hash)
                .await
                .map_err(|_| messaging_failure("conversation_start_failed", false))?;
            node.set_active_conversation(Some(&peer_hash)).await.map_err(|error| {
                messaging_failure("active_conversation_failed", error.retryable)
            })?;
        }
        MobileActionKind::SetIdentityDisplayName { display_name } => {
            node.facade.as_ref().set_identity(Some(&display_name), None, None).await.map_err(
                |error| messaging_failure("identity_display_name_failed", error.is_retryable()),
            )?;
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

async fn publish_snapshot(
    node: &MobileNode,
    generation: u64,
    backend_generation: u64,
    updates: &Sender<SessionView>,
) {
    if let Ok(update) = project(node, generation, backend_generation).await {
        let _ = updates.force_send(SessionView::Running(update));
    }
}

async fn publish_snapshot_with_failure(
    node: &MobileNode,
    generation: u64,
    backend_generation: u64,
    updates: &Sender<SessionView>,
    failure: TypedFailure,
) {
    if let Ok(mut update) = project(node, generation, backend_generation).await {
        update.fixture.session.failure = Some(failure);
        let _ = updates.force_send(SessionView::Running(update));
    }
}

async fn project(
    node: &MobileNode,
    generation: u64,
    backend_generation: u64,
) -> Result<SessionUpdate, String> {
    let session = node.session_snapshot().await;
    if session.generation != backend_generation {
        return Err("backend generation changed before projection".into());
    }
    let identity =
        node.facade.as_ref().query_identity().await.map_err(|error| error.to_string())?;
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
    if node.session_snapshot().await.generation != backend_generation {
        return Err("backend generation changed during projection".into());
    }

    let propagation_update = project_propagation(generation, &propagation);
    Ok(SessionUpdate {
        fixture: MobileFixture {
            id: "embedded-live".into(),
            profile: Profile::Live,
            generation,
            session: Session {
                runtime: project_runtime(session.runtime),
                phase: project_phase(session.phase),
                identity_hash: if identity.lxmf_destination_hash.is_empty() {
                    node.delivery_hash().unwrap_or_default()
                } else {
                    identity.lxmf_destination_hash
                },
                display_name: identity.display_name,
                endpoint: session.endpoint,
                failure: session.failure.map(|failure| TypedFailure {
                    code: format!("{:?}", failure.code).to_ascii_lowercase(),
                    retryable: failure.retryable,
                }),
                custody: identity.custody.as_ref().map(project_identity_custody),
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
    let details = project_message_details(&message);
    let delivered = details
        .delivery_evidence
        .iter()
        .any(|evidence| evidence.state == MessageDeliveryState::Completed);
    Message {
        id: message.id,
        peer_hash: if message.is_outgoing { message.destination_hash } else { message.source_hash },
        content: message.content,
        requested_method: project_method(message.requested_delivery_method.as_deref()),
        actual_method: project_method(message.actual_delivery_method.as_deref()),
        persistence: PersistenceState::Unknown,
        transport: TransportEvidence::None,
        propagation: PropagationEvidence::None,
        delivery: if delivered { DeliveryEvidence::Delivered } else { DeliveryEvidence::Pending },
        correlation_id: message.correlation_id.unwrap_or_default(),
        failure: None,
        lifecycle: Some(lifecycle),
        details,
    }
}

fn project_message_details(message: &MessageInfo) -> MessageDetails {
    MessageDetails {
        projection_complete: message.projection_complete,
        source_hash: message.source_hash.clone(),
        destination_hash: message.destination_hash.clone(),
        timestamp: message.timestamp,
        lxmf_timestamp: message.lxmf_timestamp,
        title: message.title.clone(),
        status: message.status.clone(),
        terminal_detail: message.terminal_detail.clone(),
        is_outgoing: message.is_outgoing,
        delivery_method: message.delivery_method.clone(),
        requested_delivery_method: message.requested_delivery_method.clone(),
        actual_delivery_method: message.actual_delivery_method.clone(),
        fallback_reason: message.fallback_reason.clone(),
        correlation_id: message.correlation_id.clone(),
        attempts: message.attempts.iter().map(project_message_attempt).collect(),
        propagation_correlations: message
            .propagation_correlations
            .iter()
            .map(|correlation| MessagePropagationCorrelation {
                relation: correlation.relation.clone(),
                transient_id: correlation.transient_id.clone(),
                attempt_id: correlation.attempt_id.clone(),
                peer_hash: correlation.peer_hash.clone(),
                state: correlation.state.clone(),
                created_at: correlation.created_at,
                updated_at: correlation.updated_at,
            })
            .collect(),
        read: message.read,
        attachment_info: message.attachment_info.as_ref().map(project_attachment),
        attachments: message.attachments.iter().map(project_attachment).collect(),
        authentication: match message.authentication_state {
            styrene_ipc::types::MessageAuthenticationState::Verified => {
                MessageAuthentication::Verified
            }
            styrene_ipc::types::MessageAuthenticationState::Invalid => {
                MessageAuthentication::Invalid
            }
            styrene_ipc::types::MessageAuthenticationState::UnknownIdentity => {
                MessageAuthentication::UnknownIdentity
            }
            styrene_ipc::types::MessageAuthenticationState::NotApplicable => {
                MessageAuthentication::NotApplicable
            }
            _ => MessageAuthentication::Unknown,
        },
        stamp_state: match message.stamp_state {
            styrene_ipc::types::MessageStampState::Verified => MessageStampState::Verified,
            styrene_ipc::types::MessageStampState::Invalid => MessageStampState::Invalid,
            styrene_ipc::types::MessageStampState::NotApplicable => {
                MessageStampState::NotApplicable
            }
            _ => MessageStampState::Unknown,
        },
        stamp_value: message.stamp_value,
        stamp_cost: message.stamp_cost,
        delivery_evidence: message
            .delivery_evidence
            .iter()
            .map(|evidence| MessageDeliveryObservation {
                kind: match evidence.kind {
                    styrene_ipc::types::MessageDeliveryEvidenceKind::PacketReceipt => {
                        MessageDeliveryKind::PacketReceipt
                    }
                    styrene_ipc::types::MessageDeliveryEvidenceKind::ResourceCompletion => {
                        MessageDeliveryKind::ResourceCompletion
                    }
                    _ => MessageDeliveryKind::Unknown,
                },
                hash: evidence.hash.clone(),
                representation: evidence.representation.clone(),
                state: match evidence.state {
                    styrene_ipc::types::MessageDeliveryEvidenceState::Tracked => {
                        MessageDeliveryState::Tracked
                    }
                    styrene_ipc::types::MessageDeliveryEvidenceState::Completed => {
                        MessageDeliveryState::Completed
                    }
                    styrene_ipc::types::MessageDeliveryEvidenceState::Failed => {
                        MessageDeliveryState::Failed
                    }
                    styrene_ipc::types::MessageDeliveryEvidenceState::Cancelled => {
                        MessageDeliveryState::Cancelled
                    }
                    _ => MessageDeliveryState::Unknown,
                },
                outcome: evidence.outcome.clone(),
                attempt: evidence.attempt,
                correlation_id: evidence.correlation_id.clone(),
                observed_at: evidence.observed_at,
                terminal_at: evidence.terminal_at,
                transferred_bytes: evidence.transferred_bytes,
                total_bytes: evidence.total_bytes,
                progress: evidence.progress,
            })
            .collect(),
        retry_eligible: message.retry_eligible,
        retry_ineligibility_reason: message.retry_ineligibility_reason.map(project_retry_reason),
    }
}

fn project_retry_reason(
    reason: BackendRetryIneligibilityReason,
) -> MessageRetryIneligibilityReason {
    match reason {
        BackendRetryIneligibilityReason::Inbound => MessageRetryIneligibilityReason::Inbound,
        BackendRetryIneligibilityReason::MissingOutboundRoute => {
            MessageRetryIneligibilityReason::MissingOutboundRoute
        }
        BackendRetryIneligibilityReason::LifecycleState => {
            MessageRetryIneligibilityReason::LifecycleState
        }
        BackendRetryIneligibilityReason::CanonicalWireUnavailable => {
            MessageRetryIneligibilityReason::CanonicalWireUnavailable
        }
        BackendRetryIneligibilityReason::AttemptLimitReached => {
            MessageRetryIneligibilityReason::AttemptLimitReached
        }
        _ => MessageRetryIneligibilityReason::Unknown,
    }
}

fn project_message_attempt(attempt: &styrene_ipc::types::MessageAttemptInfo) -> MessageAttempt {
    MessageAttempt {
        message_id: attempt.message_id.clone(),
        number: attempt.number,
        started_unix_ms: attempt.started_unix_ms,
        deadline_unix_ms: attempt.deadline_unix_ms,
        state: attempt.state.clone(),
        bearer: attempt.bearer.clone(),
        route: MessageRouteObservation {
            outcome: match attempt.route.outcome {
                styrene_ipc::types::MessageAttemptRouteOutcome::Observed => {
                    MessageRouteOutcome::Observed
                }
                _ => MessageRouteOutcome::Unknown,
            },
            connection_generation: attempt.route.connection_generation,
            observed_at: attempt.route.observed_at,
            next_hop: attempt.route.next_hop.clone(),
            hops: attempt.route.hops,
            stale: attempt.route.stale,
            interface: attempt.route.interface.as_ref().map(|interface| {
                MessageInterfaceObservation {
                    id: interface.id.clone(),
                    kind: interface.kind.clone(),
                    generation: interface.generation,
                }
            }),
        },
    }
}

fn project_attachment(attachment: &styrene_ipc::types::AttachmentInfo) -> MessageAttachment {
    MessageAttachment {
        ordinal: attachment.ordinal,
        id: attachment.id.clone(),
        name: attachment.name.clone(),
        content_type: attachment.content_type.clone(),
        size: attachment.size,
        checksum: attachment.checksum.clone(),
        availability: attachment.availability.clone(),
        integrity: attachment.integrity.clone(),
        transfer: attachment.transfer.as_ref().map(|transfer| MessageAttachmentTransfer {
            message_id: transfer.message_id.clone(),
            transfer_id: transfer.transfer_id.clone(),
            resource_hash: transfer.resource_hash.clone(),
            representation: transfer.representation.clone(),
            direction: transfer.direction.clone(),
            state: transfer.state.clone(),
            transferred: transfer.transferred,
            total: transfer.total,
            checksum_verified: transfer.checksum_verified,
            cancellable: transfer.cancellable,
            error: transfer.error.clone(),
        }),
    }
}

fn project_identity_custody(custody: &IdentityCustodyInfo) -> IdentityCustody {
    IdentityCustody {
        requested_backend: match custody.requested_backend {
            BackendCustodyBackend::Keychain => IdentityCustodyBackend::Keychain,
            BackendCustodyBackend::AndroidKeystore => IdentityCustodyBackend::AndroidKeystore,
            BackendCustodyBackend::EncryptedFile => IdentityCustodyBackend::EncryptedFile,
            BackendCustodyBackend::PlaintextFile => IdentityCustodyBackend::PlaintextFile,
        },
        active_backend: custody.active_backend.map(|backend| match backend {
            BackendCustodyBackend::Keychain => IdentityCustodyBackend::Keychain,
            BackendCustodyBackend::AndroidKeystore => IdentityCustodyBackend::AndroidKeystore,
            BackendCustodyBackend::EncryptedFile => IdentityCustodyBackend::EncryptedFile,
            BackendCustodyBackend::PlaintextFile => IdentityCustodyBackend::PlaintextFile,
        }),
        protection: custody.protection.map(|protection| match protection {
            BackendCustodyProtection::PlatformProtected => {
                IdentityCustodyProtection::PlatformProtected
            }
            BackendCustodyProtection::EncryptedAtRest => IdentityCustodyProtection::EncryptedAtRest,
            BackendCustodyProtection::DevelopmentPlaintext => {
                IdentityCustodyProtection::DevelopmentPlaintext
            }
        }),
        authentication: match custody.authentication {
            BackendCustodyAuthentication::DeviceAuthentication => {
                IdentityCustodyAuthentication::DeviceAuthentication
            }
            BackendCustodyAuthentication::HostKeyMaterial => {
                IdentityCustodyAuthentication::HostKeyMaterial
            }
            BackendCustodyAuthentication::None => IdentityCustodyAuthentication::None,
        },
        availability: match custody.availability {
            BackendCustodyAvailability::Available => IdentityCustodyAvailability::Available,
            BackendCustodyAvailability::Unavailable => IdentityCustodyAvailability::Unavailable,
        },
        downgrade: match custody.downgrade {
            BackendCustodyDowngrade::None => IdentityCustodyDowngrade::None,
            BackendCustodyDowngrade::ActiveBackendMismatch => {
                IdentityCustodyDowngrade::ActiveBackendMismatch
            }
        },
        failure: custody.failure.as_ref().map(|failure| TypedFailure {
            code: match failure.code {
                BackendCustodyFailureCode::UnsupportedTarget => "unsupported_target",
                BackendCustodyFailureCode::FeatureDisabled => "feature_disabled",
                BackendCustodyFailureCode::AuthenticationRequired => "authentication_required",
                BackendCustodyFailureCode::KeyMaterialRequired => "key_material_required",
                BackendCustodyFailureCode::BackendFailure => "backend_failure",
            }
            .into(),
            retryable: failure.retryable,
        }),
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
        MobileConnectionPhase::Stopped => SessionPhase::Stopped,
        MobileConnectionPhase::Starting => SessionPhase::Starting,
        MobileConnectionPhase::Connecting => SessionPhase::Connecting,
        MobileConnectionPhase::Connected => SessionPhase::Connected,
        MobileConnectionPhase::Offline => SessionPhase::Offline,
        MobileConnectionPhase::Reconnecting => SessionPhase::Reconnecting,
        MobileConnectionPhase::Degraded => SessionPhase::Degraded,
        MobileConnectionPhase::Failed => SessionPhase::Failed,
    }
}

fn project_runtime(runtime: MobileRuntimeState) -> SessionRuntime {
    match runtime {
        MobileRuntimeState::Ready => SessionRuntime::Ready,
        MobileRuntimeState::Failed => SessionRuntime::Failed,
        MobileRuntimeState::Stopped => SessionRuntime::Stopped,
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
            MobileBearerState::Connecting => BearerState::Connecting,
            MobileBearerState::Reconnecting => BearerState::Reconnecting,
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
        readiness: match snapshot.readiness {
            MobilePropagationReadiness::Unselected => PropagationReadiness::Unselected,
            MobilePropagationReadiness::Ready => PropagationReadiness::Ready,
            MobilePropagationReadiness::Unavailable => PropagationReadiness::Unavailable,
            MobilePropagationReadiness::Inactive => PropagationReadiness::Inactive,
            MobilePropagationReadiness::InvalidMetadata => PropagationReadiness::InvalidMetadata,
        },
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
        trigger_capabilities: snapshot
            .trigger_capabilities
            .iter()
            .copied()
            .map(project_propagation_trigger)
            .collect(),
        active_trigger: snapshot.active_trigger.map(project_propagation_trigger),
        active_sync_started_at: snapshot.active_sync_started_at,
        last_synchronization: snapshot.last_synchronization.as_ref().map(|synchronization| {
            PropagationSynchronization {
                trigger: project_propagation_trigger(synchronization.trigger),
                started_at: synchronization.started_at,
                finished_at: synchronization.finished_at,
                outcome: match synchronization.outcome {
                    MobilePropagationTerminalOutcome::Succeeded => {
                        PropagationTerminalOutcome::Succeeded
                    }
                    MobilePropagationTerminalOutcome::Failed => PropagationTerminalOutcome::Failed,
                    MobilePropagationTerminalOutcome::TimedOut => {
                        PropagationTerminalOutcome::TimedOut
                    }
                    MobilePropagationTerminalOutcome::Cancelled => {
                        PropagationTerminalOutcome::Cancelled
                    }
                },
                new_messages: synchronization.new_messages,
            }
        }),
        cooldown_remaining_secs: snapshot.cooldown_remaining_secs,
    }
}

fn project_propagation_trigger(
    trigger: MobilePropagationTriggerSource,
) -> PropagationTriggerSource {
    match trigger {
        MobilePropagationTriggerSource::InitialConnection => {
            PropagationTriggerSource::InitialConnection
        }
        MobilePropagationTriggerSource::Reconnect => PropagationTriggerSource::Reconnect,
        MobilePropagationTriggerSource::ForegroundOpportunity => {
            PropagationTriggerSource::ForegroundOpportunity
        }
        MobilePropagationTriggerSource::GrantedBackgroundOpportunity => {
            PropagationTriggerSource::GrantedBackgroundOpportunity
        }
        MobilePropagationTriggerSource::Manual => PropagationTriggerSource::Manual,
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
        enable_rnode_channel: cfg!(any(target_os = "android", target_os = "ios")),
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
            runtime: SessionRuntime::Failed,
            phase: SessionPhase::Failed,
            identity_hash: String::new(),
            display_name: String::new(),
            endpoint: Some(DEFAULT_ENDPOINT.into()),
            failure: Some(TypedFailure { code: code.into(), retryable: true }),
            custody: None,
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

    const PRODUCT_HANDOFF: &str =
        include_str!("../../../tests/fixtures/mobile-product-handoff-v1/message.json");
    const DESTINATION_CONVERGENCE: &str =
        include_str!("../../../tests/fixtures/mobile-destination-convergence-v1/corpus.json");

    struct ScratchRoot(PathBuf);

    impl ScratchRoot {
        fn create(label: &str) -> Self {
            let unique = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|elapsed| elapsed.as_nanos())
                .unwrap_or_default();
            let root = std::env::temp_dir()
                .join("styrene-mobile-tests")
                .join(format!("{label}-{}-{unique}", std::process::id()));
            std::fs::create_dir_all(&root).expect("create scratch root");
            Self(root)
        }

        fn config(&self) -> MobileConfig {
            MobileConfig {
                config_dir: self.0.join("config"),
                data_dir: self.0.join("data"),
                hub_address: None,
                hub_delivery_hash: None,
                display_name: None,
                identity_backend: IdentityBackend::PlaintextFile,
                interfaces: Vec::new(),
                enable_rnode_channel: false,
            }
        }
    }

    impl Drop for ScratchRoot {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn corpus_candidate_action(ingress: &str, raw: &str) -> Option<MobileActionKind> {
        use styrene_ui_state::{bounded_destination_input, start_conversation_action};

        if ingress == "discovered" {
            return Some(MobileActionKind::StartConversation { peer_hash: raw.to_owned() });
        }
        start_conversation_action(1, &bounded_destination_input(raw)).map(|action| action.kind)
    }

    async fn assert_single_shell(node: &MobileNode, canonical: &str, context: &str) {
        let page = node.conversation_page(16, None).await.expect("conversation page");
        assert_eq!(page.conversations.len(), 1, "{context} changed the conversation count");
        assert_eq!(page.conversations[0].peer_hash, canonical, "{context}");
        assert_eq!(page.conversations[0].message_count, 0, "{context} fabricated messages");
        assert!(
            node.list_contacts().await.expect("contacts").is_empty(),
            "{context} created a contact"
        );
    }

    #[test]
    fn destination_convergence_reaches_one_backend_conversation_from_every_ingress_path() {
        let corpus: serde_json::Value =
            serde_json::from_str(DESTINATION_CONVERGENCE).expect("corpus must deserialize");
        assert_eq!(corpus["schema_version"], 1);
        assert_eq!(corpus["corpus"], "styrene-mobile-destination-convergence-v1");
        assert_eq!(
            corpus["authority"]["repository"],
            "https://github.com/styrene-lab/styrene-rs.git"
        );
        let canonical = corpus["canonical_peer_hash"].as_str().expect("canonical peer hash");
        let converging = corpus["converging_candidates"].as_array().expect("converging");
        let rejected = corpus["rejected_candidates"].as_array().expect("rejected");
        assert_eq!(converging.len(), 4);

        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("tokio runtime");
        let scratch = ScratchRoot::create("destination-convergence");
        let config = scratch.config();

        runtime.block_on(async {
            let node = MobileNode::boot(config.clone()).await.expect("boot embedded backend");
            assert!(node.conversation_page(16, None).await.unwrap().conversations.is_empty());

            for candidate in converging {
                let id = candidate["id"].as_str().unwrap();
                let ingress = candidate["ingress"].as_str().unwrap();
                let raw = candidate["raw"].as_str().unwrap();
                let submitted = candidate["submitted"].as_str().unwrap();
                let action = corpus_candidate_action(ingress, raw)
                    .unwrap_or_else(|| panic!("{id} was blocked before dispatch"));
                assert_eq!(
                    action,
                    MobileActionKind::StartConversation { peer_hash: submitted.to_owned() },
                    "{id}"
                );
                execute_action(&node, action)
                    .await
                    .unwrap_or_else(|failure| panic!("{id} failed: {failure:?}"));
                assert_single_shell(&node, canonical, id).await;
            }

            for candidate in rejected {
                let id = candidate["id"].as_str().unwrap();
                let ingress = candidate["ingress"].as_str().unwrap();
                let raw = candidate["raw"].as_str().unwrap();
                let dispatch = candidate["ui_dispatch"].as_str().unwrap();
                match corpus_candidate_action(ingress, raw) {
                    Some(action) => {
                        assert!(
                            matches!(dispatch, "forwarded" | "trimmed_before_dispatch"),
                            "{id} was dispatched despite {dispatch}"
                        );
                        let result = execute_action(&node, action).await;
                        if dispatch == "forwarded" {
                            let failure = result.expect_err("backend must reject forwarded input");
                            assert_eq!(failure.code, "conversation_start_failed", "{id}");
                            assert!(!failure.retryable, "{id} was reported retryable");
                        } else {
                            result.unwrap_or_else(|failure| panic!("{id} failed: {failure:?}"));
                        }
                    }
                    None => assert!(
                        dispatch.starts_with("blocked_"),
                        "{id} was blocked despite {dispatch}"
                    ),
                }
                assert!(
                    node.start_conversation(raw).await.is_err(),
                    "{id} raw value was accepted by the backend"
                );
                assert_single_shell(&node, canonical, id).await;
            }
            node.shutdown().await.expect("shutdown");

            let reopened = MobileNode::boot(config).await.expect("reboot embedded backend");
            assert_single_shell(&reopened, canonical, "restart").await;
            reopened.shutdown().await.expect("shutdown");
        });
    }

    fn validated_handoff_message(value: &serde_json::Value) -> Result<MessageInfo, String> {
        if value["schema_version"] != 1
            || value["corpus"] != "styrene-mobile-product-handoff-v1"
            || value["authority"]["repository"] != "https://github.com/styrene-lab/styrene-rs.git"
            || value["authority"]["source_revision"] != "0d9dc04cab795f23f791278b019fe4c28430ab6f"
        {
            return Err("invalid immutable backend authority".into());
        }
        let message = value.get("message").ok_or("message is required")?;
        let authoritative_fields = value["authoritative_fields"]
            .as_array()
            .ok_or("authoritative_fields must be an array")?;
        for pointer in authoritative_fields {
            let pointer = pointer.as_str().ok_or("authoritative field must be a string")?;
            if message.pointer(pointer).is_none() {
                return Err(format!("missing authoritative field {pointer}"));
            }
        }
        let message: MessageInfo = serde_json::from_value(message.clone())
            .map_err(|error| format!("invalid typed backend message: {error}"))?;
        if message.retry_eligible != Some(false)
            || message.requested_delivery_method.as_deref() != Some("propagated")
            || message.actual_delivery_method.as_deref() != Some("direct")
        {
            return Err("authoritative delivery facts were altered or synthesized".into());
        }
        Ok(message)
    }

    #[test]
    fn initial_view_inspects_custody_without_publishing_a_fixture() {
        let view = MobileSession::starting_view();

        assert!(matches!(view, SessionView::Inspecting));
        assert_eq!(view.generation(), None);
    }

    #[test]
    fn bootstrap_generation_is_explicit_and_stale_views_do_not_match() {
        let view = SessionView::Bootstrap { generation: 7 };

        assert_eq!(view.generation(), Some(7));
        assert_ne!(view.generation(), Some(8));
    }

    #[test]
    fn backend_restore_failures_remain_typed_for_bootstrap_ui() {
        assert_eq!(
            identity_recovery_failure(MobileIdentityRecoveryError::AuthenticationFailed),
            IdentityRecoveryFailure::AuthenticationFailed
        );
        assert_eq!(
            identity_recovery_failure(MobileIdentityRecoveryError::InvalidBackup),
            IdentityRecoveryFailure::InvalidBackup
        );
        assert_eq!(
            identity_recovery_failure(MobileIdentityRecoveryError::IdentityConflict),
            IdentityRecoveryFailure::IdentityConflict
        );
    }

    #[test]
    fn unknown_backend_delivery_method_is_not_reported_as_direct() {
        assert_eq!(project_method(None), DeliveryMethod::Unknown);
        assert_eq!(project_method(Some("future-method")), DeliveryMethod::Unknown);
        assert_eq!(project_method(Some("direct")), DeliveryMethod::Direct);
    }

    #[test]
    fn backend_message_projection_preserves_authoritative_evidence_without_retry_inference() {
        let mut route = styrene_ipc::types::MessageAttemptRouteObservation::default();
        route.outcome = styrene_ipc::types::MessageAttemptRouteOutcome::Observed;
        route.connection_generation = Some(7);
        route.observed_at = Some(1_700_000_010);
        route.next_hop = Some("next-hop".into());
        route.hops = Some(2);
        let mut interface = styrene_ipc::types::MessageAttemptInterfaceObservation::default();
        interface.id = "interface-id".into();
        interface.kind = "tcp-client".into();
        interface.generation = 7;
        route.interface = Some(interface);
        let mut attempt = styrene_ipc::types::MessageAttemptInfo::default();
        attempt.message_id = "message-1".into();
        attempt.number = 2;
        attempt.started_unix_ms = 1_700_000_000_000;
        attempt.deadline_unix_ms = 1_700_000_030_000;
        attempt.state = "failed".into();
        attempt.bearer = Some("tcp".into());
        attempt.route = route;

        let mut propagation = styrene_ipc::types::MessagePropagationCorrelationInfo::default();
        propagation.relation = "upload".into();
        propagation.transient_id = "transient-1".into();
        propagation.attempt_id = Some("attempt-2".into());
        propagation.peer_hash = Some("propagation-node".into());
        propagation.state = "accepted".into();
        propagation.created_at = 1_700_000_001;
        propagation.updated_at = 1_700_000_002;

        let mut evidence = styrene_ipc::types::MessageDeliveryEvidenceInfo::default();
        evidence.kind = styrene_ipc::types::MessageDeliveryEvidenceKind::PacketReceipt;
        evidence.hash = "receipt-hash".into();
        evidence.representation = "packet".into();
        evidence.state = styrene_ipc::types::MessageDeliveryEvidenceState::Completed;
        evidence.outcome = Some("delivered".into());
        evidence.attempt = Some(2);
        evidence.correlation_id = Some("correlation-1".into());
        evidence.observed_at = 1_700_000_020;
        evidence.terminal_at = Some(1_700_000_021);

        let mut message = MessageInfo::default();
        message.projection_complete = true;
        message.id = "message-1".into();
        message.source_hash = "local-source".into();
        message.destination_hash = "remote-destination".into();
        message.timestamp = 1_700_000_000;
        message.lxmf_timestamp = Some(1_700_000_000.25);
        message.content = "payload".into();
        message.title = Some("title".into());
        message.status = "failed".into();
        message.lifecycle_state = MessageLifecycleState::Failed;
        message.terminal_detail = Some("non-retryable policy rejection".into());
        message.is_outgoing = true;
        message.delivery_method = Some("propagated".into());
        message.requested_delivery_method = Some("propagated".into());
        message.actual_delivery_method = Some("direct".into());
        message.fallback_reason = Some("node unavailable".into());
        message.correlation_id = Some("correlation-1".into());
        message.attempts = vec![attempt];
        message.propagation_correlations = vec![propagation];
        message.read = true;
        message.authentication_state = styrene_ipc::types::MessageAuthenticationState::Verified;
        message.stamp_state = styrene_ipc::types::MessageStampState::Verified;
        message.stamp_value = Some(18);
        message.stamp_cost = Some(16);
        message.delivery_evidence = vec![evidence];

        let projected = project_message(message);
        let details = &projected.details;

        assert!(details.projection_complete);
        assert_eq!(details.source_hash, "local-source");
        assert_eq!(details.destination_hash, "remote-destination");
        assert_eq!(details.lxmf_timestamp, Some(1_700_000_000.25));
        assert_eq!(details.terminal_detail.as_deref(), Some("non-retryable policy rejection"));
        assert_eq!(details.requested_delivery_method.as_deref(), Some("propagated"));
        assert_eq!(details.actual_delivery_method.as_deref(), Some("direct"));
        assert_eq!(details.attempts[0].bearer.as_deref(), Some("tcp"));
        assert_eq!(details.attempts[0].route.outcome, MessageRouteOutcome::Observed);
        assert_eq!(
            details.attempts[0].route.interface.as_ref().map(|interface| interface.generation),
            Some(7)
        );
        assert_eq!(details.propagation_correlations[0].state, "accepted");
        assert_eq!(details.delivery_evidence[0].state, MessageDeliveryState::Completed);
        assert_eq!(details.authentication, MessageAuthentication::Verified);
        assert_eq!(details.stamp_state, MessageStampState::Verified);
        assert_eq!(details.retry_eligible, None);
        assert_eq!(projected.persistence, PersistenceState::Unknown);
        assert_eq!(projected.delivery, DeliveryEvidence::Delivered);
        assert!(projected.failure.is_none());
    }

    #[test]
    fn immutable_backend_handoff_projects_without_dropping_or_synthesizing_evidence() {
        let value: serde_json::Value =
            serde_json::from_str(PRODUCT_HANDOFF).expect("handoff fixture must be JSON");
        let projected = project_message(
            validated_handoff_message(&value).expect("handoff fixture must preserve authority"),
        );

        assert!(projected.details.projection_complete);
        assert_eq!(projected.details.requested_delivery_method.as_deref(), Some("propagated"));
        assert_eq!(projected.details.actual_delivery_method.as_deref(), Some("direct"));
        assert_eq!(projected.details.retry_eligible, Some(false));
        assert_eq!(
            projected.details.retry_ineligibility_reason,
            Some(MessageRetryIneligibilityReason::AttemptLimitReached)
        );
        assert_eq!(projected.details.attempts[0].bearer.as_deref(), Some("tcp"));
        assert_eq!(projected.details.attempts[0].route.outcome, MessageRouteOutcome::Observed);
        assert_eq!(projected.details.propagation_correlations[0].state, "accepted");
        assert_eq!(projected.details.delivery_evidence[0].state, MessageDeliveryState::Completed);
        assert_eq!(projected.delivery, DeliveryEvidence::Delivered);
    }

    #[test]
    fn immutable_backend_handoff_rejects_authority_mutations() {
        let canonical: serde_json::Value =
            serde_json::from_str(PRODUCT_HANDOFF).expect("handoff fixture must be JSON");
        let mut dropped = canonical.clone();
        dropped["message"].as_object_mut().unwrap().remove("actual_delivery_method");
        assert!(
            validated_handoff_message(&dropped)
                .unwrap_err()
                .contains("missing authoritative field")
        );

        let mut synthesized = canonical;
        synthesized["message"]["retry_eligible"] = serde_json::Value::Bool(true);
        assert!(
            validated_handoff_message(&synthesized).unwrap_err().contains("altered or synthesized")
        );
    }

    #[test]
    fn backend_offline_ready_state_remains_distinct_from_reconnecting() {
        assert_eq!(project_phase(MobileConnectionPhase::Offline), SessionPhase::Offline);
        assert_eq!(SessionPhase::Offline.as_str(), "offline");
    }

    #[test]
    fn backend_runtime_and_connection_phases_remain_distinct() {
        for (backend, projected) in [
            (MobileConnectionPhase::Stopped, SessionPhase::Stopped),
            (MobileConnectionPhase::Offline, SessionPhase::Offline),
            (MobileConnectionPhase::Starting, SessionPhase::Starting),
            (MobileConnectionPhase::Connecting, SessionPhase::Connecting),
            (MobileConnectionPhase::Connected, SessionPhase::Connected),
            (MobileConnectionPhase::Reconnecting, SessionPhase::Reconnecting),
            (MobileConnectionPhase::Degraded, SessionPhase::Degraded),
            (MobileConnectionPhase::Failed, SessionPhase::Failed),
        ] {
            assert_eq!(project_phase(backend), projected);
        }

        for (backend, projected) in [
            (MobileRuntimeState::Ready, SessionRuntime::Ready),
            (MobileRuntimeState::Failed, SessionRuntime::Failed),
            (MobileRuntimeState::Stopped, SessionRuntime::Stopped),
        ] {
            assert_eq!(project_runtime(backend), projected);
        }
    }

    #[test]
    fn propagation_trigger_sources_remain_typed_and_background_is_explicitly_granted() {
        for (backend, projected) in [
            (
                MobilePropagationTriggerSource::InitialConnection,
                PropagationTriggerSource::InitialConnection,
            ),
            (MobilePropagationTriggerSource::Reconnect, PropagationTriggerSource::Reconnect),
            (
                MobilePropagationTriggerSource::ForegroundOpportunity,
                PropagationTriggerSource::ForegroundOpportunity,
            ),
            (
                MobilePropagationTriggerSource::GrantedBackgroundOpportunity,
                PropagationTriggerSource::GrantedBackgroundOpportunity,
            ),
            (MobilePropagationTriggerSource::Manual, PropagationTriggerSource::Manual),
        ] {
            assert_eq!(project_propagation_trigger(backend), projected);
        }
    }

    #[test]
    fn backend_custody_projection_is_typed_and_secret_free() {
        let custody = IdentityCustodyInfo {
            requested_backend: BackendCustodyBackend::AndroidKeystore,
            active_backend: Some(BackendCustodyBackend::AndroidKeystore),
            protection: Some(BackendCustodyProtection::PlatformProtected),
            authentication: BackendCustodyAuthentication::DeviceAuthentication,
            availability: BackendCustodyAvailability::Unavailable,
            downgrade: BackendCustodyDowngrade::None,
            failure: Some(styrene_ipc::types::IdentityCustodyFailure {
                code: BackendCustodyFailureCode::AuthenticationRequired,
                retryable: true,
            }),
        };

        let projected = project_identity_custody(&custody);

        assert_eq!(projected.requested_backend, IdentityCustodyBackend::AndroidKeystore);
        assert_eq!(projected.active_backend, Some(IdentityCustodyBackend::AndroidKeystore));
        assert_eq!(projected.protection, Some(IdentityCustodyProtection::PlatformProtected));
        assert_eq!(projected.authentication, IdentityCustodyAuthentication::DeviceAuthentication);
        assert_eq!(projected.availability, IdentityCustodyAvailability::Unavailable);
        assert_eq!(projected.downgrade, IdentityCustodyDowngrade::None);
        assert_eq!(
            projected.failure,
            Some(TypedFailure { code: "authentication_required".into(), retryable: true })
        );
    }

    #[test]
    fn stale_backend_generations_cannot_replace_current_state() {
        let mut backend_generation = 3;
        let mut generation = 9;

        assert!(!advance_generation_if_changed(&mut backend_generation, 3, &mut generation));
        assert!(!advance_generation_if_changed(&mut backend_generation, 2, &mut generation));
        assert_eq!(backend_generation, 3);
        assert_eq!(generation, 9);
        assert!(advance_generation_if_changed(&mut backend_generation, 4, &mut generation));
        assert_eq!(backend_generation, 4);
        assert_eq!(generation, 10);
    }
}
