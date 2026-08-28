//! Shared Dioxus application components.

use dioxus::prelude::*;
use styrene_ui_state::{MobileFixture, TargetClass};

#[component]
pub fn MobileShell(target: TargetClass, fixture: MobileFixture) -> Element {
    rsx! {
        main {
            "data-target": target.as_str(),
            "data-fixture-id": fixture.id,
            for accessibility_id in fixture.expected.accessibility_ids {
                section { id: accessibility_id }
            }
        }
    }
}
