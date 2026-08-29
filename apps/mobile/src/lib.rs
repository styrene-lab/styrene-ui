use dioxus::prelude::*;
use styrene_ui_app::{BackNavigation, MobileShell};
use styrene_ui_state::{MobileFixture, MobileMinimumCorpus, TargetClass};

mod platform;

const FIXTURES: &str = include_str!("../../../tests/fixtures/mobile-minimum-v1/states.json");
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
    platform::use_back_navigation();

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

    const ANDROID_MANIFEST: &str = include_str!("../AndroidManifest.xml");

    #[test]
    fn bootstrap_is_visibly_fixture_only() {
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
    fn android_manifest_preserves_webview_across_supported_configuration_changes() {
        assert!(ANDROID_MANIFEST.contains("Theme.AppCompat.DayNight.NoActionBar"));
        assert!(ANDROID_MANIFEST.contains("fontScale"));
        assert!(ANDROID_MANIFEST.contains("uiMode"));
        assert!(ANDROID_MANIFEST.contains("density"));
        assert!(ANDROID_MANIFEST.contains("smallestScreenSize"));
        assert!(ANDROID_MANIFEST.contains("android:stateNotNeeded=\"true\""));
    }
}
