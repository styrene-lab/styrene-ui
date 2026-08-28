use std::collections::HashSet;

use styrene_ui_state::{
    ApplyResult, BearerKind, BearerState, Conversation, DeliveryEvidence, DeliveryMethod,
    DraftClearDisposition, EndpointUpdate, LocalAnnounceOutcome, MessageEvent, MessageSnapshot,
    MobileFixture, MobileMinimumCorpus, MobileStore, PeerEvent, PeerSnapshot, Profile,
    PropagationEvidence, SendOutcome, SessionPhase, SyncState, TargetClass, TypedFailure,
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
