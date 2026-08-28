use dioxus::prelude::*;
use styrene_ui_app::MobileShell;
use styrene_ui_state::{MobileMinimumCorpus, RuntimeBoundary, TargetClass};

const FIXTURES: &str = include_str!("../../../tests/fixtures/mobile-minimum-v1/states.json");

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
