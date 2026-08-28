//! Shared Dioxus application components.

use dioxus::prelude::*;
use styrene_ui_state::{
    LocalAnnounceOutcome, MobileFixture, MobileStore, RuntimeBoundary, TargetClass,
};

#[component]
pub fn MobileShell(target: TargetClass, fixture: MobileFixture) -> Element {
    let boundary = RuntimeBoundary::from(fixture.profile);
    let messaging_available = MobileStore::new(fixture.clone()).messaging_available();

    rsx! {
        main {
            "data-target": target.as_str(),
            "data-fixture-id": fixture.id.clone(),
            "data-generation": fixture.generation.to_string(),
            "data-live-network-enabled": boundary.live_network_allowed().to_string(),
            if boundary.fixture_marker_visible() {
                aside { id: "mobile.fixture-banner", "Fixture data" }
            }
            section {
                id: "mobile.session-state",
                "data-phase": fixture.session.phase.as_str(),
            }
            section { id: "mobile.identity", {fixture.session.identity_hash.clone()} }
            section {
                id: "mobile.messages",
                for message in &fixture.messages {
                    article {
                        id: format!("mobile.message.{}", message.id),
                        {message.content.clone()}
                    }
                }
                for conversation in &fixture.conversations {
                    p {
                        id: format!("mobile.draft.{}", conversation.peer_hash),
                        {conversation.draft.clone()}
                    }
                }
                button {
                    id: "mobile.send",
                    "data-enabled": messaging_available.to_string(),
                    "Send"
                }
            }
            section {
                id: "mobile.people",
                for peer in &fixture.peers {
                    article {
                        id: format!("mobile.peer.{}", peer.destination_hash),
                        "data-aspect": peer.aspect.clone(),
                        "data-source": "canonical_announce",
                        {peer.display_name.clone().unwrap_or_else(|| peer.destination_hash.clone())}
                    }
                }
            }
            section {
                id: "mobile.network",
                input {
                    id: "mobile.tcp-endpoint",
                    value: fixture.session.endpoint.clone().unwrap_or_default(),
                }
                button { id: "mobile.tcp-endpoint-apply", "Apply endpoint" }
                for bearer in &fixture.bearers {
                    div {
                        id: format!("mobile.bearer.{}", bearer.kind.as_str()),
                        "data-state": bearer.state.to_string(),
                    }
                }
            }
            section { id: "mobile.propagation" }
        }
    }
}

#[component]
pub fn LocalAnnounceStatus(outcome: LocalAnnounceOutcome) -> Element {
    rsx! {
        output {
            id: "mobile.local-announce-outcome",
            "data-generation": outcome.generation.to_string(),
            if let Some(failure) = outcome.failure {
                span { "Local announce failed: {failure.code}" }
            } else if outcome.local_dispatch_accepted {
                span { "Accepted by local transport" }
                if !outcome.remote_reception_confirmed {
                    span { "Remote reception unconfirmed" }
                }
            }
        }
    }
}
