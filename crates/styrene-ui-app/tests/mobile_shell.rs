use dioxus::prelude::*;
use styrene_ui_app::MobileShell;
use styrene_ui_state::{
    ApplyResult, BearerState, MobileFixture, MobileMinimumCorpus, MobileStore, RuntimeBoundary,
    SessionPhase, TargetClass,
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

fn render(fixture: MobileFixture) -> String {
    dioxus_ssr::render_element(rsx! {
        MobileShell { target: TargetClass::Ios, fixture }
    })
}

#[test]
fn every_fixture_renders_the_shared_accessibility_contract_for_both_targets() {
    let corpus: MobileMinimumCorpus =
        serde_json::from_str(FIXTURES).expect("mobile minimum fixture must deserialize");

    for target in [TargetClass::Ios, TargetClass::Android] {
        for fixture in &corpus.fixtures {
            let markup = dioxus_ssr::render_element(rsx! {
                MobileShell { target, fixture: fixture.clone() }
            });

            assert!(
                markup.contains(&format!("data-fixture-id=\"{}\"", fixture.id)),
                "{} must identify its fixture on {target:?}",
                fixture.id
            );
            assert!(
                markup.contains(&format!("data-target=\"{}\"", target.as_str())),
                "{} must identify the target class",
                fixture.id
            );
            for accessibility_id in &corpus.required_accessibility_ids {
                assert!(
                    markup.contains(&format!("id=\"{accessibility_id}\"")),
                    "{} must render {accessibility_id} on {target:?}",
                    fixture.id
                );
            }
        }
    }
}

#[test]
fn runtime_profiles_keep_live_and_fixture_data_paths_isolated() {
    let corpus: MobileMinimumCorpus =
        serde_json::from_str(FIXTURES).expect("mobile minimum fixture must deserialize");

    for fixture in &corpus.fixtures {
        let boundary = RuntimeBoundary::from(fixture.profile);
        let markup = dioxus_ssr::render_element(rsx! {
            MobileShell { target: TargetClass::Ios, fixture: fixture.clone() }
        });

        assert_eq!(boundary.live_network_allowed(), fixture.expected.live_network_enabled);
        assert_eq!(boundary.fixture_marker_visible(), fixture.expected.fixture_banner);
        assert_eq!(
            markup.contains("id=\"mobile.fixture-banner\""),
            fixture.expected.fixture_banner
        );
        assert!(markup.contains(&format!(
            "data-live-network-enabled=\"{}\"",
            fixture.expected.live_network_enabled
        )));

        if fixture.expected.live_network_enabled {
            assert!(fixture.peers.is_empty(), "Live must not substitute fixture peers");
            assert!(fixture.messages.is_empty(), "Live must not substitute fixture messages");
        }
    }
}

#[test]
fn cold_restoration_renders_retained_message_and_draft_while_reconnecting() {
    let mut persisted = fixture("direct-message-queued");
    persisted.conversations[0].draft = "survives process death".into();

    let bytes = serde_json::to_vec(&persisted).expect("fixture must serialize");
    let persisted = serde_json::from_slice(&bytes).expect("fixture must restore");
    let store = MobileStore::cold_restore(persisted, 10);
    let markup = render(store.snapshot().clone());

    assert!(markup.contains("data-generation=\"10\""));
    assert!(markup.contains("data-phase=\"reconnecting\""));
    assert!(markup.contains("id=\"mobile.message.message-direct-1\""));
    assert!(markup.contains("Direct message awaiting evidence"));
    assert!(markup.contains("id=\"mobile.draft.aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\""));
    assert!(markup.contains("survives process death"));
}

#[test]
fn reconnect_component_keeps_content_and_reports_tcp_transition() {
    let initial = fixture("direct-message-queued");
    let mut store = MobileStore::new(initial.clone());
    store.begin_reconnect(5, "socket_closed");

    let reconnecting = render(store.snapshot().clone());
    assert!(reconnecting.contains("data-phase=\"reconnecting\""));
    assert!(reconnecting.contains("id=\"mobile.bearer.tcp\""));
    assert!(reconnecting.contains("data-state=\"reconnecting\""));
    assert!(reconnecting.contains("Direct message awaiting evidence"));

    let mut connected = initial;
    connected.generation = 5;
    connected.session.phase = SessionPhase::Connected;
    connected.bearers[0].state = BearerState::Connected;
    connected.bearers[0].reason = None;
    assert_eq!(store.apply_snapshot(5, connected), ApplyResult::Applied);

    let connected = render(store.snapshot().clone());
    assert!(connected.contains("data-phase=\"connected\""));
    assert!(connected.contains("data-state=\"connected\""));
}

#[test]
fn stale_completion_never_appears_in_rendered_state() {
    let mut store = MobileStore::new(fixture("stale-generation-rejected"));
    let mut stale = fixture("recoverable-session-failure");
    stale.generation = 7;

    assert_eq!(store.apply_snapshot(7, stale), ApplyResult::IgnoredStale);
    let markup = render(store.snapshot().clone());

    assert!(markup.contains("data-generation=\"8\""));
    assert!(markup.contains("data-phase=\"connected\""));
    assert!(markup.contains("data-state=\"connected\""));
    assert!(!markup.contains("invalid_tcp_endpoint"));
    assert!(!markup.contains("data-phase=\"failed\""));
}

#[test]
fn tcp_only_state_renders_messaging_as_enabled_without_rnode() {
    let store = MobileStore::new(fixture("direct-message-queued"));
    assert!(store.messaging_available());

    let markup = render(store.snapshot().clone());

    assert!(markup.contains("id=\"mobile.bearer.tcp\""));
    assert!(markup.contains("data-state=\"connected\""));
    assert!(markup.contains("id=\"mobile.bearer.bluetooth-rnode\""));
    assert!(markup.contains("data-state=\"unavailable\""));
    assert!(markup.contains("id=\"mobile.send\""));
    assert!(markup.contains("data-enabled=\"true\""));
}
