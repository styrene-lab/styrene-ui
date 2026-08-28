//! Shared Dioxus application components.

use dioxus::prelude::*;
use styrene_ui_state::{MobileFixture, RuntimeBoundary, TargetClass};

#[component]
pub fn MobileShell(target: TargetClass, fixture: MobileFixture) -> Element {
    let boundary = RuntimeBoundary::from(fixture.profile);

    rsx! {
        main {
            "data-target": target.as_str(),
            "data-fixture-id": fixture.id,
            "data-live-network-enabled": boundary.live_network_allowed().to_string(),
            if boundary.fixture_marker_visible() {
                aside { id: "mobile.fixture-banner", "Fixture data" }
            }
            for accessibility_id in fixture.expected.accessibility_ids {
                section { id: accessibility_id }
            }
        }
    }
}
