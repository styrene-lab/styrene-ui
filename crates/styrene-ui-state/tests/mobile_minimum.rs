use std::collections::HashSet;

use styrene_ui_state::{
    ApplyResult, Bearer, BearerEvent, BearerKind, BearerState, Conversation, DeliveryEvidence,
    DeliveryMethod, DraftClearDisposition, EndpointUpdate, IdentityCustody,
    IdentityCustodyAuthentication, IdentityCustodyAvailability, IdentityCustodyBackend,
    IdentityCustodyDowngrade, IdentityCustodyProtection, LocalAnnounceOutcome, MessageEvent,
    MessageSnapshot, MobileAction, MobileActionKind, MobileFixture, MobileMinimumCorpus,
    MobileStore, PeerEvent, PeerSnapshot, Profile, PropagationEvidence, PropagationProgress,
    PropagationUpdate, SendOutcome, SessionPhase, SyncState, TargetClass, TypedFailure,
};

const FIXTURES: &str = include_str!("../../../tests/fixtures/mobile-minimum-v1/states.json");

fn fixture(id: &str) -> MobileFixture {
    let corpus: MobileMinimumCorpus =
        serde_json::from_str(FIXTURES).expect("mobile minimum fixture must deserialize");
    corpus
        .fixtures
        .into_iter()
        .find(|fixture| fixture.id == id)
        .unwrap_or_else(|| panic!("missing fixture {id}"))
}

#[test]
fn shared_mobile_minimum_fixture_deserializes_strictly() {
    let corpus: MobileMinimumCorpus =
        serde_json::from_str(FIXTURES).expect("mobile minimum fixture must deserialize");

    assert_eq!(corpus.schema_version, 1);
    assert_eq!(corpus.corpus, "styrene-mobile-minimum-v1");
    assert_eq!(corpus.target_classes, [TargetClass::Ios, TargetClass::Android]);
    assert_eq!(corpus.fixtures.len(), 8);
    assert_eq!(
        corpus.fixtures.iter().map(|fixture| fixture.id.as_str()).collect::<HashSet<_>>().len(),
        corpus.fixtures.len()
    );
    assert!(corpus.fixtures.iter().all(|fixture| fixture.session.custody.is_none()));
    let serialized = serde_json::to_value(&corpus).expect("mobile minimum fixture must serialize");
    assert!(
        serialized["fixtures"]
            .as_array()
            .expect("fixtures must be an array")
            .iter()
            .all(|fixture| fixture["session"].get("custody").is_none())
    );
}

#[test]
fn runtime_and_extended_session_phase_round_trip_without_collapsing() {
    let mut fixture = fixture("live-empty-connected");
    fixture.session.runtime = styrene_ui_state::SessionRuntime::Failed;
    fixture.session.phase = SessionPhase::Degraded;

    let encoded = serde_json::to_vec(&fixture).expect("session state must serialize");
    let restored: MobileFixture =
        serde_json::from_slice(&encoded).expect("session state must deserialize");

    assert_eq!(restored.session.runtime, styrene_ui_state::SessionRuntime::Failed);
    assert_eq!(restored.session.phase, SessionPhase::Degraded);
    assert_eq!(restored.session.runtime.as_str(), "failed");
    assert_eq!(restored.session.phase.as_str(), "degraded");
}

#[test]
fn authoritative_message_details_round_trip_without_optional_value_collapse() {
    let mut fixture = fixture("direct-message-queued");
    let message = &mut fixture.messages[0];
    message.details.projection_complete = true;
    message.details.source_hash = "source".into();
    message.details.destination_hash = "destination".into();
    message.details.lxmf_timestamp = Some(42.25);
    message.details.correlation_id = None;
    message.details.requested_delivery_method = Some("future-method".into());
    message.details.retry_eligible = None;
    message.details.attempts.push(styrene_ui_state::MessageAttempt {
        message_id: message.id.clone(),
        number: 1,
        bearer: Some("tcp".into()),
        route: styrene_ui_state::MessageRouteObservation {
            outcome: styrene_ui_state::MessageRouteOutcome::Observed,
            connection_generation: Some(fixture.generation),
            ..Default::default()
        },
        ..Default::default()
    });

    let encoded = serde_json::to_vec(&fixture).expect("message details must serialize");
    let restored: MobileFixture =
        serde_json::from_slice(&encoded).expect("message details must deserialize");
    let details = &restored.messages[0].details;

    assert!(details.projection_complete);
    assert_eq!(details.lxmf_timestamp, Some(42.25));
    assert_eq!(details.correlation_id, None);
    assert_eq!(details.requested_delivery_method.as_deref(), Some("future-method"));
    assert_eq!(details.retry_eligible, None);
    assert_eq!(details.attempts[0].route.outcome, styrene_ui_state::MessageRouteOutcome::Observed);
}

#[test]
fn custody_projection_round_trips_in_renderer_neutral_state() {
    let mut fixture = fixture("live-empty-connected");
    fixture.session.custody = Some(IdentityCustody {
        requested_backend: IdentityCustodyBackend::Keychain,
        active_backend: Some(IdentityCustodyBackend::Keychain),
        protection: Some(IdentityCustodyProtection::PlatformProtected),
        authentication: IdentityCustodyAuthentication::DeviceAuthentication,
        availability: IdentityCustodyAvailability::Available,
        downgrade: IdentityCustodyDowngrade::None,
        failure: None,
    });

    let bytes = serde_json::to_vec(&fixture).expect("custody fixture must serialize");
    let restored: MobileFixture =
        serde_json::from_slice(&bytes).expect("custody fixture must deserialize");

    assert_eq!(restored.session.custody, fixture.session.custody);
}

#[test]
fn mobile_actions_preserve_originating_generation_and_command_facts() {
    let send = MobileAction::new(
        12,
        MobileActionKind::SendMessage {
            peer_hash: "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".into(),
            content: "retained draft".into(),
            requested_method: DeliveryMethod::Propagated,
            draft_revision: 7,
        },
    );

    assert_eq!(send.generation, 12);
    assert_eq!(
        send.kind,
        MobileActionKind::SendMessage {
            peer_hash: "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".into(),
            content: "retained draft".into(),
            requested_method: DeliveryMethod::Propagated,
            draft_revision: 7,
        }
    );
}

#[test]
fn start_conversation_action_preserves_generation_and_canonical_destination() {
    let action = MobileAction::new(
        11,
        MobileActionKind::StartConversation {
            peer_hash: "e01b09b22ccc4e2755d29eead962677b".into(),
        },
    );

    assert_eq!(action.generation, 11);
    assert_eq!(
        action.kind,
        MobileActionKind::StartConversation {
            peer_hash: "e01b09b22ccc4e2755d29eead962677b".into(),
        }
    );
}

#[test]
fn identity_display_name_action_preserves_generation_and_public_value() {
    let action = MobileAction::new(
        12,
        MobileActionKind::SetIdentityDisplayName { display_name: "Field Node".into() },
    );

    assert_eq!(action.generation, 12);
    assert_eq!(
        action.kind,
        MobileActionKind::SetIdentityDisplayName { display_name: "Field Node".into() }
    );
}

#[test]
fn new_message_entry_admits_only_bounded_backend_candidates() {
    use styrene_ui_state::{
        DestinationEntryConstraint, LXMF_DESTINATION_INPUT_MAX_BYTES, bounded_destination_input,
        destination_entry_constraint, start_conversation_action,
    };

    assert_eq!(destination_entry_constraint("  "), DestinationEntryConstraint::Empty);
    assert_eq!(destination_entry_constraint("abc"), DestinationEntryConstraint::Incomplete);
    assert!(start_conversation_action(4, "abc").is_none());

    let canonical = "e01b09b22ccc4e2755d29eead962677b";
    assert_eq!(canonical.len(), LXMF_DESTINATION_INPUT_MAX_BYTES);
    assert_eq!(destination_entry_constraint(canonical), DestinationEntryConstraint::Ready);
    assert_eq!(
        start_conversation_action(4, canonical),
        Some(MobileAction::new(
            4,
            MobileActionKind::StartConversation { peer_hash: canonical.into() },
        ))
    );

    let malformed_but_bounded = "z".repeat(LXMF_DESTINATION_INPUT_MAX_BYTES);
    assert!(start_conversation_action(4, &malformed_but_bounded).is_some());

    let oversized = "a".repeat(LXMF_DESTINATION_INPUT_MAX_BYTES + 10_000);
    let retained = bounded_destination_input(&oversized);
    assert_eq!(retained.len(), LXMF_DESTINATION_INPUT_MAX_BYTES + 1);
    assert_eq!(destination_entry_constraint(&retained), DestinationEntryConstraint::Oversized);
    assert!(start_conversation_action(4, &retained).is_none());
}

#[test]
fn new_message_peer_search_is_empty_safe_case_insensitive_and_bounded() {
    use styrene_ui_state::{
        PEER_SEARCH_INPUT_MAX_BYTES, bounded_peer_search_input, peer_matches_search,
    };

    let peer = fixture("canonical-peer-discovery").peers.remove(0);
    assert!(peer_matches_search(&peer, ""));
    assert!(peer_matches_search(&peer, "skywave"));
    assert!(peer_matches_search(&peer, "E01B09"));
    assert!(peer_matches_search(&peer, "LXMF.DELIVERY"));
    assert!(!peer_matches_search(&peer, "missing"));

    let oversized = "q".repeat(PEER_SEARCH_INPUT_MAX_BYTES + 1024);
    assert_eq!(bounded_peer_search_input(&oversized).len(), PEER_SEARCH_INPUT_MAX_BYTES + 1);
}

#[test]
fn endpoint_action_is_an_intent_not_a_backend_outcome() {
    let action = MobileAction::new(
        4,
        MobileActionKind::ApplyEndpoint { endpoint: "rns.styrene.io:4242".into() },
    );

    assert_eq!(action.generation, 4);
    assert_eq!(
        action.kind,
        MobileActionKind::ApplyEndpoint { endpoint: "rns.styrene.io:4242".into() }
    );
}

#[test]
fn reducer_contract_keeps_upload_distinct_from_delivery() {
    let corpus: MobileMinimumCorpus = serde_json::from_str(FIXTURES).unwrap();
    let fixture = corpus
        .fixtures
        .iter()
        .find(|fixture| fixture.id == "propagation-uploaded-not-delivered")
        .unwrap();
    let message = fixture.messages.first().unwrap();

    assert_eq!(fixture.profile, Profile::Fixture);
    assert_eq!(fixture.session.phase, SessionPhase::Connected);
    assert_eq!(message.requested_method, DeliveryMethod::Propagated);
    assert_eq!(message.propagation, PropagationEvidence::Uploaded);
    assert_eq!(message.delivery, DeliveryEvidence::Pending);
    assert_eq!(fixture.propagation.sync_state, SyncState::Idle);
}

#[test]
fn propagation_selection_survives_reconnect_but_readiness_does_not() {
    let fixture = fixture("canonical-peer-discovery");
    let selected = fixture.propagation.selected_destination.clone();
    let mut store = MobileStore::new(fixture);

    store.begin_reconnect(4, "socket_closed");

    assert_eq!(store.propagation().selected_destination, selected);
    assert!(!store.propagation().ready);
    assert_eq!(store.propagation().sync_state, SyncState::Idle);
}

#[test]
fn propagation_reducer_rejects_stale_generation_and_projects_stale_metadata() {
    let fixture = fixture("canonical-peer-discovery");
    let generation = fixture.generation;
    let mut store = MobileStore::new(fixture);
    let mut stale_metadata = store.propagation().clone();
    stale_metadata.ready = false;

    assert_eq!(store.apply_propagation_update(stale_metadata.clone()), ApplyResult::Applied);
    assert!(!store.propagation().ready);
    stale_metadata.generation = generation - 1;
    stale_metadata.ready = true;
    assert_eq!(store.apply_propagation_update(stale_metadata), ApplyResult::IgnoredStale);
    assert!(!store.propagation().ready);
}

#[test]
fn manual_sync_progress_and_repeat_completion_do_not_duplicate_messages() {
    let fixture = fixture("propagation-sync-complete");
    let message_id = fixture.messages[0].id.clone();
    let mut store = MobileStore::new(fixture);
    let mut in_progress = store.propagation().clone();
    in_progress.sync_state = SyncState::InProgress;
    in_progress.new_messages = 0;
    in_progress.progress = Some(PropagationProgress {
        attempt_id: "attempt-sync-2".into(),
        received_count: 1,
        received_bytes: 128,
    });

    assert_eq!(store.apply_propagation_update(in_progress), ApplyResult::Applied);
    assert_eq!(store.propagation().sync_state, SyncState::InProgress);
    assert_eq!(store.propagation().progress.as_ref().unwrap().received_count, 1);

    let mut repeated = store.propagation().clone();
    repeated.sync_state = SyncState::Complete;
    repeated.new_messages = 0;
    repeated.progress = None;
    assert_eq!(store.apply_propagation_update(repeated), ApplyResult::Applied);
    assert_eq!(store.propagation().new_messages, 0);
    assert_eq!(store.snapshot().messages.len(), 1);
    assert_eq!(store.snapshot().messages[0].id, message_id);
}

#[test]
fn propagation_policy_and_recoverable_failure_are_backend_owned_projections() {
    let fixture = fixture("recoverable-session-failure");
    let mut store = MobileStore::new(fixture);
    let update = PropagationUpdate {
        generation: store.snapshot().generation,
        selected_destination: Some("780e7aa7b2f175c88f28c7ba8ab1b714".into()),
        readiness: styrene_ui_state::PropagationReadiness::Unavailable,
        ready: false,
        sync_state: SyncState::Failed,
        new_messages: 0,
        failure: Some(TypedFailure { code: "transport_unavailable".into(), retryable: true }),
        automatic_sync_enabled: true,
        automatic_sync_cooldown_secs: 30,
        sync_deadline_secs: 32,
        progress: None,
        candidates: Vec::new(),
        selected_policy: None,
        trigger_capabilities: Vec::new(),
        active_trigger: None,
        active_sync_started_at: None,
        last_synchronization: None,
        cooldown_remaining_secs: 0,
    };

    assert_eq!(store.apply_propagation_update(update), ApplyResult::Applied);
    assert!(store.propagation().automatic_sync_enabled);
    assert_eq!(store.propagation().automatic_sync_cooldown_secs, 30);
    assert_eq!(store.propagation().sync_deadline_secs, 32);
    assert!(store.propagation().failure.as_ref().is_some_and(|failure| failure.retryable));
}

#[test]
fn reducer_contract_rejects_stale_generation_event() {
    let corpus: MobileMinimumCorpus = serde_json::from_str(FIXTURES).unwrap();
    let fixture =
        corpus.fixtures.iter().find(|fixture| fixture.id == "stale-generation-rejected").unwrap();
    let event = fixture.event.as_ref().unwrap();

    assert!(event.generation < fixture.generation);
    assert!(!event.expected_applied);
    assert!(fixture.accepts_generation(fixture.generation));
    assert!(!fixture.accepts_generation(event.generation));
    assert_eq!(fixture.bearer(BearerKind::Tcp).unwrap().state.to_string(), "connected");
}

#[test]
fn cold_restore_retains_durable_messaging_state_but_reconnects_transport() {
    let mut persisted = fixture("direct-message-queued");
    persisted.conversations[0].draft = "survives process death".into();

    let bytes = serde_json::to_vec(&persisted).expect("fixture must serialize");
    let persisted = serde_json::from_slice(&bytes).expect("fixture must restore");
    let store = MobileStore::cold_restore(persisted, 10);
    let restored = store.snapshot();

    assert_eq!(restored.generation, 10);
    assert_eq!(restored.session.phase, SessionPhase::Reconnecting);
    assert_eq!(
        restored.bearer(BearerKind::Tcp).expect("TCP bearer").state,
        BearerState::Reconnecting
    );
    assert_eq!(
        restored.bearer(BearerKind::BluetoothRnode).expect("Bluetooth RNode bearer").state,
        BearerState::Unavailable
    );
    assert_eq!(restored.messages[0].id, "message-direct-1");
    assert_eq!(restored.conversations[0].draft, "survives process death");
    assert_eq!(restored.session.identity_hash, "44444444444444444444444444444444");
}

#[test]
fn reconnect_keeps_messages_visible_until_current_generation_completes() {
    let initial = fixture("direct-message-queued");
    let mut store = MobileStore::new(initial.clone());

    store.begin_reconnect(5, "socket_closed");

    assert_eq!(store.snapshot().generation, 5);
    assert_eq!(store.snapshot().session.phase, SessionPhase::Reconnecting);
    assert_eq!(store.snapshot().messages, initial.messages);
    assert_eq!(store.snapshot().conversations, initial.conversations);

    let mut connected = initial;
    connected.generation = 5;
    connected.session.phase = SessionPhase::Connected;
    connected.bearers[0].state = BearerState::Connected;
    connected.bearers[0].reason = None;

    assert_eq!(store.apply_snapshot(5, connected), ApplyResult::Applied);
    assert_eq!(store.snapshot().session.phase, SessionPhase::Connected);
    assert_eq!(store.snapshot().messages[0].id, "message-direct-1");
}

#[test]
fn stale_completion_cannot_replace_current_generation_state() {
    let mut store = MobileStore::new(fixture("stale-generation-rejected"));
    let before = store.snapshot().clone();

    let mut stale_completion = fixture("recoverable-session-failure");
    stale_completion.generation = 7;

    assert_eq!(store.apply_snapshot(7, stale_completion), ApplyResult::IgnoredStale);
    assert_eq!(store.snapshot(), &before);
    assert_eq!(store.snapshot().generation, 8);
}

#[test]
fn tcp_enables_messaging_when_rnode_is_unavailable() {
    let store = MobileStore::new(fixture("direct-message-queued"));

    assert_eq!(
        store.snapshot().bearer(BearerKind::Tcp).expect("TCP bearer").state,
        BearerState::Connected
    );
    assert_eq!(
        store.snapshot().bearer(BearerKind::BluetoothRnode).expect("Bluetooth RNode bearer").state,
        BearerState::Unavailable
    );
    assert!(store.messaging_available());
}

#[test]
fn operational_summary_uses_only_authoritative_loaded_facts() {
    let mut state = fixture("direct-message-queued");
    state.conversations[0].unread_count = u32::MAX;
    state.conversations.push(styrene_ui_state::Conversation {
        peer_hash: "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb".into(),
        unread_count: 7,
        draft: String::new(),
        draft_revision: 0,
    });
    state.messages[0].details.attempts.extend([
        styrene_ui_state::MessageAttempt {
            route: styrene_ui_state::MessageRouteObservation {
                outcome: styrene_ui_state::MessageRouteOutcome::Observed,
                ..Default::default()
            },
            ..Default::default()
        },
        styrene_ui_state::MessageAttempt::default(),
    ]);
    let summary = MobileStore::new(state).operational_summary();

    assert_eq!(summary.runtime, styrene_ui_state::SessionRuntime::Ready);
    assert_eq!(summary.phase, SessionPhase::Connected);
    assert_eq!(summary.connected_bearers, 1);
    assert_eq!(summary.bearer_count, 3);
    assert_eq!(summary.peer_count, 0);
    assert_eq!(summary.unread_count, u32::MAX);
    assert_eq!(summary.loaded_route_observed, 1);
    assert_eq!(summary.loaded_route_unknown, 1);
    assert!(!summary.propagation_selected);
    assert!(!summary.propagation_ready);
    assert_eq!(summary.propagation_sync_state, SyncState::Idle);
}

#[test]
fn operational_summary_preserves_empty_ready_reconnecting_degraded_failed_and_unknown_states() {
    let empty = MobileStore::new(fixture("live-empty-connected")).operational_summary();
    assert_eq!(empty.peer_count, 0);
    assert_eq!(empty.unread_count, 0);
    assert_eq!(empty.loaded_route_observed, 0);
    assert_eq!(empty.loaded_route_unknown, 0);

    let ready = MobileStore::new(fixture("canonical-peer-discovery")).operational_summary();
    assert_eq!(ready.phase, SessionPhase::Connected);
    assert_eq!(ready.peer_count, 1);
    assert!(ready.propagation_selected);
    assert!(ready.propagation_ready);

    let reconnecting =
        MobileStore::new(fixture("tcp-reconnecting-rnode-unavailable")).operational_summary();
    assert_eq!(reconnecting.phase, SessionPhase::Reconnecting);
    assert_eq!(reconnecting.connected_bearers, 0);
    assert_eq!(reconnecting.bearer_count, 3);

    let mut degraded_state = fixture("live-empty-connected");
    degraded_state.session.phase = SessionPhase::Degraded;
    let degraded = MobileStore::new(degraded_state).operational_summary();
    assert_eq!(degraded.phase, SessionPhase::Degraded);

    let failed = MobileStore::new(fixture("recoverable-session-failure")).operational_summary();
    assert_eq!(failed.phase, SessionPhase::Failed);
    assert!(!failed.propagation_ready);

    let unknown_route = MobileStore::new(fixture("direct-message-queued")).operational_summary();
    assert_eq!(unknown_route.loaded_route_observed, 0);
    assert_eq!(unknown_route.loaded_route_unknown, 0);
}

#[test]
fn platform_bearer_events_remain_independent_from_connected_tcp() {
    let fixture = fixture("direct-message-queued");
    let generation = fixture.generation;
    let mut store = MobileStore::new(fixture);

    for bearer in [
        Bearer {
            kind: BearerKind::BluetoothRnode,
            state: BearerState::Unavailable,
            reason: Some("permission_denied".into()),
        },
        Bearer {
            kind: BearerKind::BluetoothRnode,
            state: BearerState::Disconnected,
            reason: Some("connection_interrupted".into()),
        },
        Bearer {
            kind: BearerKind::AndroidUsb,
            state: BearerState::Unverified,
            reason: Some("physical_evidence_absent".into()),
        },
    ] {
        assert_eq!(
            store.apply_bearer_event(BearerEvent { generation, bearer: bearer.clone() }),
            ApplyResult::Applied
        );
        assert_eq!(store.snapshot().session.phase, SessionPhase::Connected);
        assert_eq!(
            store.snapshot().bearer(BearerKind::Tcp).expect("TCP bearer").state,
            BearerState::Connected
        );
        assert_eq!(store.snapshot().bearer(bearer.kind), Some(&bearer));
        assert!(store.messaging_available());
    }

    let current = store.snapshot().bearer(BearerKind::AndroidUsb).cloned();
    assert_eq!(
        store.apply_bearer_event(BearerEvent {
            generation: generation - 1,
            bearer: Bearer {
                kind: BearerKind::AndroidUsb,
                state: BearerState::Connected,
                reason: None,
            },
        }),
        ApplyResult::IgnoredStale
    );
    assert_eq!(store.snapshot().bearer(BearerKind::AndroidUsb), current.as_ref());
}

#[test]
fn endpoint_edit_starts_a_new_reconnect_generation_without_losing_content() {
    let initial = fixture("direct-message-queued");
    let mut store = MobileStore::new(initial.clone());

    let update = EndpointUpdate {
        endpoint: "rns.styrene.io:4242".into(),
        generation: initial.generation + 1,
    };
    store.apply_endpoint_update(update.clone());

    assert_eq!(update.endpoint, "rns.styrene.io:4242");
    assert_eq!(update.generation, initial.generation + 1);
    assert_eq!(store.snapshot().session.endpoint.as_deref(), Some(update.endpoint.as_str()));
    assert_eq!(store.snapshot().session.phase, SessionPhase::Reconnecting);
    assert_eq!(store.snapshot().messages, initial.messages);
    assert_eq!(store.snapshot().conversations, initial.conversations);
}

#[test]
fn malformed_endpoint_edit_is_recoverable_and_does_not_replace_current_state() {
    let mut store = MobileStore::new(fixture("direct-message-queued"));
    let endpoint = store.snapshot().session.endpoint.clone();
    let generation = store.snapshot().generation;
    let messages = store.snapshot().messages.clone();

    store.apply_endpoint_failure(TypedFailure {
        code: "invalid_tcp_endpoint".into(),
        retryable: true,
    });
    let error = store.snapshot().session.failure.as_ref().expect("typed endpoint failure");

    assert_eq!(error.code, "invalid_tcp_endpoint");
    assert!(error.retryable);
    assert_eq!(store.snapshot().session.endpoint, endpoint);
    assert_eq!(store.snapshot().generation, generation);
    assert_eq!(store.snapshot().messages, messages);
}

#[test]
fn repeated_peer_events_upsert_one_newer_destination_observation() {
    let initial = fixture("canonical-peer-discovery");
    let mut store = MobileStore::new(initial.clone());
    let mut peer = initial.peers[0].clone();
    peer.display_name = Some("Current Name".into());
    peer.observed_at += 10;
    peer.age_secs = 0;
    peer.announce_count += 1;

    assert_eq!(
        store.apply_peer_event(PeerEvent { generation: initial.generation, peer }),
        ApplyResult::Applied
    );
    assert_eq!(store.snapshot().peers.len(), 1);
    assert_eq!(store.snapshot().peers[0].display_name.as_deref(), Some("Current Name"));
    assert_eq!(store.snapshot().peers[0].announce_count, 2);

    assert_eq!(
        store.apply_peer_snapshot(PeerSnapshot {
            generation: initial.generation,
            peers: initial.peers.clone(),
        }),
        ApplyResult::Applied
    );
    assert_eq!(store.snapshot().peers[0].display_name.as_deref(), Some("Current Name"));
    assert_eq!(store.snapshot().peers[0].announce_count, 2);
}

#[test]
fn stale_peer_event_and_snapshot_cannot_replace_current_directory() {
    let initial = fixture("canonical-peer-discovery");
    let mut store = MobileStore::new(initial.clone());
    let mut stale_peer = initial.peers[0].clone();
    stale_peer.display_name = Some("Stale Name".into());

    assert_eq!(
        store.apply_peer_event(PeerEvent { generation: initial.generation - 1, peer: stale_peer }),
        ApplyResult::IgnoredStale
    );
    assert_eq!(
        store.apply_peer_snapshot(PeerSnapshot {
            generation: initial.generation - 1,
            peers: Vec::new(),
        }),
        ApplyResult::IgnoredStale
    );
    assert_eq!(store.snapshot().peers, initial.peers);
}

#[test]
fn empty_current_generation_snapshot_preserves_a_genuine_empty_live_directory() {
    let live = fixture("live-empty-connected");
    let mut store = MobileStore::new(live.clone());

    assert_eq!(
        store.apply_peer_snapshot(PeerSnapshot { generation: live.generation, peers: Vec::new() }),
        ApplyResult::Applied
    );
    assert!(store.snapshot().peers.is_empty());
}

#[test]
fn local_announce_outcome_never_implies_remote_reception() {
    let fixture = fixture("canonical-peer-discovery");
    let mut store = MobileStore::new(fixture.clone());
    let outcome = LocalAnnounceOutcome {
        generation: fixture.generation,
        accepted_at: 1_787_927_100,
        local_dispatch_accepted: true,
        remote_reception_confirmed: false,
        failure: None,
    };

    assert_eq!(store.apply_local_announce_outcome(outcome.clone()), ApplyResult::Applied);
    assert_eq!(store.local_announce_outcome(), Some(&outcome));
    assert!(!store.local_announce_outcome().unwrap().remote_reception_confirmed);
}

#[test]
fn newer_draft_revision_survives_an_older_send_clear_outcome() {
    let initial = fixture("direct-message-queued");
    let mut store = MobileStore::new(initial.clone());
    let peer_hash = initial.conversations[0].peer_hash.clone();
    assert_eq!(
        store.apply_draft(Conversation {
            peer_hash: peer_hash.clone(),
            unread_count: 0,
            draft: "submitted".into(),
            draft_revision: 1,
        }),
        ApplyResult::Applied
    );
    assert_eq!(
        store.apply_draft(Conversation {
            peer_hash: peer_hash.clone(),
            unread_count: 0,
            draft: "newer edit".into(),
            draft_revision: 2,
        }),
        ApplyResult::Applied
    );

    assert_eq!(
        store.apply_send_outcome(SendOutcome {
            generation: initial.generation,
            message: initial.messages[0].clone(),
            submitted_draft_revision: Some(1),
            draft_clear: DraftClearDisposition::Cleared,
        }),
        ApplyResult::Applied
    );
    assert_eq!(store.snapshot().conversations[0].draft, "newer edit");
    assert_eq!(store.snapshot().conversations[0].draft_revision, 2);
}

#[test]
fn message_events_are_canonical_id_upserts_and_stale_generations_are_ignored() {
    let initial = fixture("direct-message-queued");
    let mut store = MobileStore::new(initial.clone());
    let mut updated = initial.messages[0].clone();
    updated.delivery = DeliveryEvidence::Delivered;

    assert_eq!(
        store.apply_message_event(MessageEvent {
            generation: initial.generation,
            message: updated.clone(),
        }),
        ApplyResult::Applied
    );
    assert_eq!(
        store
            .apply_message_event(MessageEvent { generation: initial.generation, message: updated }),
        ApplyResult::Applied
    );
    assert_eq!(store.snapshot().messages.len(), 1);
    assert_eq!(store.snapshot().messages[0].delivery, DeliveryEvidence::Delivered);

    let mut stale = initial.messages[0].clone();
    stale.content = "stale".into();
    assert_eq!(
        store.apply_message_event(MessageEvent {
            generation: initial.generation - 1,
            message: stale,
        }),
        ApplyResult::IgnoredStale
    );
    assert_ne!(store.snapshot().messages[0].content, "stale");
}

#[test]
fn backend_message_snapshot_controls_unread_and_deduplicates_canonical_ids() {
    let initial = fixture("direct-message-queued");
    let mut store = MobileStore::new(initial.clone());
    let mut conversation = initial.conversations[0].clone();
    conversation.unread_count = 3;
    let message = initial.messages[0].clone();

    assert_eq!(
        store.apply_message_snapshot(MessageSnapshot {
            generation: initial.generation,
            conversations: vec![conversation],
            messages: vec![message.clone(), message],
        }),
        ApplyResult::Applied
    );
    assert_eq!(store.snapshot().conversations[0].unread_count, 3);
    assert_eq!(store.snapshot().messages.len(), 1);
}
