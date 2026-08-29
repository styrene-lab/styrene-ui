//! Shared Dioxus application components.

use dioxus::prelude::*;
use styrene_ui_state::{
    Conversation, DeliveryEvidence, LocalAnnounceOutcome, Message, MobileFixture, MobileStore,
    PropagationEvidence, PropagationUpdate, RuntimeBoundary, SyncState, TargetClass,
    TransportEvidence,
};

#[component]
pub fn MobileShell(target: TargetClass, fixture: MobileFixture) -> Element {
    let boundary = RuntimeBoundary::from(fixture.profile);
    let messaging_available = MobileStore::new(fixture.clone()).messaging_available();
    let live_actions_enabled = boundary.live_network_allowed();

    rsx! {
        document::Title { "Styrene Messages" }
        document::Stylesheet { href: asset!("/assets/mobile.css") }
        main {
            class: "mobile-shell",
            "aria-labelledby": "mobile.app-title",
            "data-target": target.as_str(),
            "data-fixture-id": fixture.id.clone(),
            "data-generation": fixture.generation.to_string(),
            "data-live-network-enabled": boundary.live_network_allowed().to_string(),
            header {
                class: "app-header",
                p { class: "app-kicker", "Styrene mesh communicator" }
                h1 { id: "mobile.app-title", "Messages" }
                p {
                    id: "mobile.identity",
                    class: "identity",
                    "aria-label": format!("Local identity {}", fixture.session.identity_hash),
                    {fixture.session.identity_hash.clone()}
                }
            }
            if boundary.fixture_marker_visible() {
                aside {
                    id: "mobile.fixture-banner",
                    class: "fixture-banner",
                    role: "note",
                    "Fixture data. Network actions are disabled."
                }
            }
            div {
                id: "mobile.session-state",
                class: "session-status",
                role: "status",
                "aria-live": "polite",
                "aria-atomic": "true",
                "data-phase": fixture.session.phase.as_str(),
                "Session {fixture.session.phase.as_str()}"
            }
            section {
                id: "mobile.messages",
                class: "product-section messages-section",
                "aria-labelledby": "mobile.messages-heading",
                h2 { id: "mobile.messages-heading", "Messages" }
                div {
                    class: "message-workspace",
                    ConversationList { conversations: fixture.conversations.clone() }
                    div {
                        class: "thread-pane",
                        MessageHistory {
                            messages: fixture.messages.clone(),
                            actions_enabled: live_actions_enabled,
                        }
                        Composer {
                            conversations: fixture.conversations.clone(),
                            enabled: messaging_available && live_actions_enabled,
                        }
                    }
                }
            }
            section {
                id: "mobile.people",
                class: "product-section",
                "aria-labelledby": "mobile.people-heading",
                h2 { id: "mobile.people-heading", "People" }
                for peer in &fixture.peers {
                    article {
                        id: format!("mobile.peer.{}", peer.destination_hash),
                        class: "peer-card",
                        "data-aspect": peer.aspect.clone(),
                        "data-source": "canonical_announce",
                        h3 {
                            {peer.display_name.clone().unwrap_or_else(|| peer.destination_hash.clone())}
                        }
                        p { class: "technical-value", {peer.destination_hash.clone()} }
                    }
                }
            }
            section {
                id: "mobile.network",
                class: "product-section",
                "aria-labelledby": "mobile.network-heading",
                h2 { id: "mobile.network-heading", "Network" }
                label { r#for: "mobile.tcp-endpoint", "TCP endpoint" }
                input {
                    id: "mobile.tcp-endpoint",
                    name: "tcp-endpoint",
                    r#type: "text",
                    inputmode: "url",
                    "aria-describedby": "mobile.tcp-endpoint-hint",
                    value: fixture.session.endpoint.clone().unwrap_or_default(),
                }
                p {
                    id: "mobile.tcp-endpoint-hint",
                    class: "field-hint",
                    "Use a host and port, for example rns.styrene.io:4242."
                }
                button {
                    id: "mobile.tcp-endpoint-apply",
                    disabled: !live_actions_enabled,
                    "Apply endpoint"
                }
                for bearer in &fixture.bearers {
                    article {
                        id: format!("mobile.bearer.{}", bearer.kind.as_str()),
                        class: "bearer-card",
                        "data-state": bearer.state.to_string(),
                        "data-reason": bearer.reason.clone().unwrap_or_default(),
                        h3 { {bearer.kind.as_str()} }
                        p { "State: {bearer.state}" }
                        if let Some(reason) = &bearer.reason {
                            p { {reason.clone()} }
                        }
                    }
                }
            }
            PropagationPanel {
                propagation: PropagationUpdate::from_fixture(&fixture),
                actions_enabled: live_actions_enabled,
            }
        }
    }
}

#[component]
pub fn PropagationPanel(propagation: PropagationUpdate, actions_enabled: bool) -> Element {
    let selected = propagation.selected_destination.as_deref().unwrap_or("No node selected");
    rsx! {
        section {
            id: "mobile.propagation",
            class: "product-section",
            "aria-labelledby": "mobile.propagation-heading",
            "data-ready": propagation.ready.to_string(),
            "data-sync-state": propagation.sync_state.as_str(),
            h2 { id: "mobile.propagation-heading", "Propagation" }
            p {
                id: "mobile.propagation-selected",
                "aria-label": "Selected propagation node",
                {selected}
            }
            label {
                r#for: "mobile.propagation-node",
                "Propagation node"
            }
            select {
                id: "mobile.propagation-node",
                option { value: "", "No node selected" }
                for candidate in &propagation.candidates {
                    option {
                        value: candidate.destination_hash.clone(),
                        selected: propagation.selected_destination.as_deref()
                            == Some(candidate.destination_hash.as_str()),
                        disabled: !candidate.active || candidate.policy.is_none(),
                        "data-active": candidate.active.to_string(),
                        "data-age-secs": candidate.age_secs.to_string(),
                        {candidate.destination_hash.clone()}
                    }
                }
            }
            if let Some(policy) = &propagation.selected_policy {
                p {
                    id: "mobile.propagation-policy",
                    "data-transfer-limit-kb": policy.transfer_limit_kb.to_string(),
                    "data-sync-limit-kb": policy.sync_limit_kb.to_string(),
                    "data-stamp-cost": policy.stamp_cost.to_string(),
                    "data-stamp-flexibility": policy.stamp_flexibility.to_string(),
                    "Backend-enforced propagation policy"
                }
            }
            p {
                id: "mobile.propagation-automatic-policy",
                "data-enabled": propagation.automatic_sync_enabled.to_string(),
                "data-cooldown-secs": propagation.automatic_sync_cooldown_secs.to_string(),
                "data-deadline-secs": propagation.sync_deadline_secs.to_string(),
                if propagation.automatic_sync_enabled {
                    "Automatic synchronization enabled"
                } else {
                    "Automatic synchronization disabled"
                }
            }
            button {
                id: "mobile.propagation-sync",
                disabled: !actions_enabled
                    || !propagation.ready
                    || propagation.sync_state == SyncState::InProgress,
                "Sync now"
            }
            div {
                id: "mobile.propagation-status",
                role: "status",
                "aria-live": "polite",
                "aria-atomic": "true",
                if let Some(failure) = &propagation.failure {
                    span {
                        id: "mobile.propagation-failure",
                        "data-code": failure.code.clone(),
                        "data-retryable": failure.retryable.to_string(),
                        "Synchronization failed"
                    }
                } else if let Some(progress) = &propagation.progress {
                    span {
                        id: "mobile.propagation-progress",
                        "data-attempt-id": progress.attempt_id.clone(),
                        "data-received-count": progress.received_count.to_string(),
                        "data-received-bytes": progress.received_bytes.to_string(),
                        "Synchronizing"
                    }
                } else if propagation.sync_state == SyncState::Complete {
                    span {
                        id: "mobile.propagation-result",
                        "{propagation.new_messages} new messages"
                    }
                }
            }
        }
    }
}

#[component]
pub fn ConversationList(conversations: Vec<Conversation>) -> Element {
    rsx! {
        nav {
            id: "mobile.conversations",
            class: "conversation-list",
            "aria-label": "Conversations",
            for conversation in conversations {
                button {
                    id: format!("mobile.conversation.{}", conversation.peer_hash),
                    class: "conversation-row",
                    r#type: "button",
                    "data-peer": conversation.peer_hash.clone(),
                    span { class: "technical-value", {conversation.peer_hash.clone()} }
                    span {
                        id: format!("mobile.conversation-unread.{}", conversation.peer_hash),
                        "aria-label": format!("{} unread messages", conversation.unread_count),
                        "{conversation.unread_count} unread"
                    }
                }
            }
        }
    }
}

#[component]
pub fn MessageHistory(messages: Vec<Message>, actions_enabled: bool) -> Element {
    rsx! {
        section {
            id: "mobile.message-history",
            class: "message-history",
            "aria-labelledby": "mobile.message-history-heading",
            h3 {
                id: "mobile.message-history-heading",
                class: "visually-hidden",
                "Message history"
            }
            if messages.is_empty() {
                p { id: "mobile.messages-empty", "No conversations yet" }
            }
            ol {
                class: "message-list",
                for message in messages {
                    li {
                        article {
                            id: format!("mobile.message.{}", message.id),
                            class: "message-card",
                            "aria-label": format!("Message with {}", message.peer_hash),
                            p { {message.content.clone()} }
                            DeliveryDetail {
                                message: message.clone(),
                                actions_enabled,
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
pub fn DeliveryDetail(message: Message, actions_enabled: bool) -> Element {
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
            class: "delivery-detail",
            p {
                id: format!("mobile.message-state.{}", message.id),
                {state}
            }
            if message.failure.as_ref().is_some_and(|failure| failure.retryable) {
                button {
                    id: format!("mobile.retry.{}", message.id),
                    disabled: !actions_enabled,
                    "Retry"
                }
            }
        }
    }
}

#[component]
pub fn Composer(conversations: Vec<Conversation>, enabled: bool) -> Element {
    let draft = conversations.first();
    let draft_id = draft.map_or_else(
        || "mobile.draft".to_string(),
        |conversation| format!("mobile.draft.{}", conversation.peer_hash),
    );
    rsx! {
        form {
            id: "mobile.composer",
            class: "composer",
            label { r#for: draft_id.clone(), "Message" }
            textarea {
                id: draft_id,
                name: "message",
                rows: "3",
                "data-revision": draft.map_or(0, |conversation| conversation.draft_revision),
                value: draft.map_or("", |conversation| conversation.draft.as_str()),
            }
            label { r#for: "mobile.delivery-method", "Delivery method" }
            select {
                id: "mobile.delivery-method",
                name: "delivery-method",
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
        div {
            id: "mobile.local-announce-outcome",
            role: "status",
            "aria-live": "polite",
            "aria-atomic": "true",
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
