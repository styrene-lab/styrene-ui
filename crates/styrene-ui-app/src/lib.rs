//! Shared Dioxus application components.

use dioxus::prelude::*;
use styrene_ui_state::{
    Conversation, DeliveryEvidence, LocalAnnounceOutcome, Message, MobileFixture, MobileStore,
    PropagationEvidence, RuntimeBoundary, TargetClass, TransportEvidence,
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
                ConversationList { conversations: fixture.conversations.clone() }
                MessageHistory { messages: fixture.messages.clone() }
                Composer {
                    conversations: fixture.conversations.clone(),
                    enabled: messaging_available,
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
pub fn ConversationList(conversations: Vec<Conversation>) -> Element {
    rsx! {
        nav {
            id: "mobile.conversations",
            for conversation in conversations {
                button {
                    id: format!("mobile.conversation.{}", conversation.peer_hash),
                    "data-peer": conversation.peer_hash.clone(),
                    span { {conversation.peer_hash.clone()} }
                    output {
                        id: format!("mobile.conversation-unread.{}", conversation.peer_hash),
                        {conversation.unread_count.to_string()}
                    }
                }
            }
        }
    }
}

#[component]
pub fn MessageHistory(messages: Vec<Message>) -> Element {
    rsx! {
        section {
            id: "mobile.message-history",
            if messages.is_empty() {
                p { id: "mobile.messages-empty", "No conversations yet" }
            }
            for message in messages {
                article {
                    id: format!("mobile.message.{}", message.id),
                    p { {message.content.clone()} }
                    DeliveryDetail { message: message.clone() }
                }
            }
        }
    }
}

#[component]
pub fn DeliveryDetail(message: Message) -> Element {
    let state = if message.delivery == DeliveryEvidence::Delivered {
        "Delivered"
    } else if message.propagation == PropagationEvidence::Uploaded {
        "Uploaded to propagation node; recipient delivery pending"
    } else if message.transport == TransportEvidence::Accepted {
        "Accepted by local transport; recipient delivery pending"
    } else {
        "Queued"
    };
    rsx! {
        div {
            id: format!("mobile.delivery-detail.{}", message.id),
            output {
                id: format!("mobile.message-state.{}", message.id),
                {state}
            }
            if message.failure.as_ref().is_some_and(|failure| failure.retryable) {
                button { id: format!("mobile.retry.{}", message.id), "Retry" }
            }
        }
    }
}

#[component]
pub fn Composer(conversations: Vec<Conversation>, enabled: bool) -> Element {
    let draft = conversations.first();
    rsx! {
        form {
            id: "mobile.composer",
            textarea {
                id: draft.map_or_else(
                    || "mobile.draft".to_string(),
                    |conversation| format!("mobile.draft.{}", conversation.peer_hash),
                ),
                "data-revision": draft.map_or(0, |conversation| conversation.draft_revision),
                value: draft.map_or("", |conversation| conversation.draft.as_str()),
            }
            select {
                id: "mobile.delivery-method",
                option { value: "direct", "Direct" }
                option { value: "propagated", "Propagated" }
            }
            button {
                id: "mobile.send",
                r#type: "button",
                "data-enabled": enabled.to_string(),
                disabled: !enabled,
                "Send"
            }
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
