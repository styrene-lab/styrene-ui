use std::collections::HashSet;

use styrene_ui_state::{
    BearerKind, DeliveryEvidence, DeliveryMethod, MobileMinimumCorpus, Profile,
    PropagationEvidence, SessionPhase, SyncState, TargetClass,
};

const FIXTURES: &str = include_str!("../../../tests/fixtures/mobile-minimum-v1/states.json");

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
