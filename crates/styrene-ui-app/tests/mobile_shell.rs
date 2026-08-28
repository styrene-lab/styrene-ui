use dioxus::prelude::*;
use styrene_ui_app::MobileShell;
use styrene_ui_state::{MobileMinimumCorpus, TargetClass};

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
