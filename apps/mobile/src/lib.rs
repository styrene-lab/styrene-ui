use dioxus::prelude::*;
use styrene_ui_app::MobileShell;
use styrene_ui_state::{MobileFixture, MobileMinimumCorpus, TargetClass};

const FIXTURES: &str = include_str!("../../../tests/fixtures/mobile-minimum-v1/states.json");
const BOOTSTRAP_FIXTURE: &str = "tcp-reconnecting-rnode-unavailable";

fn target_class() -> TargetClass {
    if cfg!(target_os = "android") {
        TargetClass::Android
    } else {
        TargetClass::Ios
    }
}

fn bootstrap_fixture() -> MobileFixture {
    let corpus: MobileMinimumCorpus =
        serde_json::from_str(FIXTURES).expect("embedded mobile fixture corpus must deserialize");
    corpus
        .fixtures
        .into_iter()
        .find(|fixture| fixture.id == BOOTSTRAP_FIXTURE)
        .expect("embedded mobile bootstrap fixture must exist")
}

#[component]
pub fn App() -> Element {
    rsx! {
        MobileShell {
            target: target_class(),
            fixture: bootstrap_fixture(),
        }
    }
}

#[cfg(test)]
mod tests {
    use styrene_ui_state::{Profile, RuntimeBoundary};

    use super::*;

    #[test]
    fn bootstrap_is_visibly_fixture_only() {
        let fixture = bootstrap_fixture();

        assert_eq!(fixture.id, BOOTSTRAP_FIXTURE);
        assert_eq!(fixture.profile, Profile::Fixture);
        assert!(RuntimeBoundary::from(fixture.profile).fixture_marker_visible());
        assert!(!RuntimeBoundary::from(fixture.profile).live_network_allowed());
    }
}
