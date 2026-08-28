use std::collections::HashSet;

use styrene_ui_state::{
    ApplyResult, BearerKind, BearerState, DeliveryEvidence, DeliveryMethod, EndpointUpdate,
    MobileFixture, MobileMinimumCorpus, MobileStore, Profile, PropagationEvidence, SessionPhase,
    SyncState, TargetClass, TypedFailure,
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
