use dioxus::prelude::*;
use styrene_ui_app::{BackNavigation, MobileShell};
use styrene_ui_state::TargetClass;
#[cfg(any(test, not(any(target_os = "android", target_os = "ios", target_os = "macos"))))]
use styrene_ui_state::{MobileFixture, MobileMinimumCorpus};

mod platform;
#[cfg(any(target_os = "android", target_os = "ios", target_os = "macos"))]
mod session;

#[cfg(any(test, not(any(target_os = "android", target_os = "ios", target_os = "macos"))))]
const FIXTURES: &str = include_str!("../../../tests/fixtures/mobile-minimum-v1/states.json");
#[cfg(any(test, not(any(target_os = "android", target_os = "ios", target_os = "macos"))))]
const BOOTSTRAP_FIXTURE: &str = "propagation-sync-complete";
pub const MOBILE_INDEX: &str = r#"<!DOCTYPE html>
<html lang="en">
    <head>
        <meta charset="utf-8">
        <meta name="viewport" content="width=device-width, initial-scale=1, viewport-fit=cover, interactive-widget=resizes-content">
        <meta name="color-scheme" content="light dark">
    </head>
    <body>
        <div id="main"></div>
    </body>
</html>"#;

fn target_class() -> TargetClass {
    if cfg!(target_os = "android") { TargetClass::Android } else { TargetClass::Ios }
}

#[cfg(any(test, not(any(target_os = "android", target_os = "ios", target_os = "macos"))))]
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
    platform::use_back_navigation();

    #[cfg(any(target_os = "android", target_os = "ios", target_os = "macos"))]
    {
        let session = use_signal(session::MobileSession::start);
        let mut update = use_signal(session::MobileSession::starting_update);
        let update_receiver = session.read().clone();
        use_future(move || {
            let update_receiver = update_receiver.clone();
            async move {
                while let Some(next) = update_receiver.next_update().await {
                    update.set(next);
                }
            }
        });
        let current = update.read().clone();

        return rsx! {
            MobileShell {
                target: target_class(),
                fixture: current.fixture,
                propagation: current.propagation,
                back_navigation: BackNavigation::web_history(),
                action_sink: move |action| session.read().dispatch(action),
            }
        };
    }

    #[cfg(not(any(target_os = "android", target_os = "ios", target_os = "macos")))]
    rsx! {
        MobileShell {
            target: target_class(),
            fixture: bootstrap_fixture(),
            back_navigation: BackNavigation::web_history(),
        }
    }
}

#[cfg(test)]
mod tests {
    use styrene_ui_state::{Profile, RuntimeBoundary};

    use super::*;

    const BACKEND_REVISION: &str = "2b9e1aeeff71733a8fc11d8a541cc417fb9450f0";
    const WORKSPACE_MANIFEST: &str = include_str!("../../../Cargo.toml");
    const MOBILE_MANIFEST: &str = include_str!("../Cargo.toml");
    const FIXTURE_PROVENANCE: &str =
        include_str!("../../../tests/fixtures/mobile-minimum-v1/README.md");
    const PARITY_CONTRACT: &str = include_str!("../../../docs/parity-corpus.md");

    #[test]
    fn embedded_fixture_corpus_remains_visibly_fixture_only() {
        let fixture = bootstrap_fixture();

        assert_eq!(fixture.id, BOOTSTRAP_FIXTURE);
        assert_eq!(fixture.profile, Profile::Fixture);
        assert!(RuntimeBoundary::from(fixture.profile).fixture_marker_visible());
        assert!(!RuntimeBoundary::from(fixture.profile).live_network_allowed());
    }

    #[test]
    fn mobile_index_preserves_zoom_and_covers_safe_areas() {
        assert!(MOBILE_INDEX.contains("<html lang=\"en\">"));
        assert!(MOBILE_INDEX.contains("width=device-width"));
        assert!(MOBILE_INDEX.contains("viewport-fit=cover"));
        assert!(MOBILE_INDEX.contains("name=\"color-scheme\" content=\"light dark\""));
        assert!(!MOBILE_INDEX.contains("user-scalable=no"));
        assert!(!MOBILE_INDEX.contains("maximum-scale"));
    }

    #[test]
    fn workspace_and_corpora_share_the_backend_contract() {
        assert!(WORKSPACE_MANIFEST.contains("resolver = \"3\""));
        assert!(WORKSPACE_MANIFEST.contains("edition = \"2024\""));
        assert_eq!(MOBILE_MANIFEST.matches(BACKEND_REVISION).count(), 4);
        assert!(FIXTURE_PROVENANCE.contains(BACKEND_REVISION));
        assert!(PARITY_CONTRACT.contains(BACKEND_REVISION));
        assert!(PARITY_CONTRACT.contains("styrene-mobile-integration-v1"));
        assert!(PARITY_CONTRACT.contains("Not consumed by UI"));
    }
}
