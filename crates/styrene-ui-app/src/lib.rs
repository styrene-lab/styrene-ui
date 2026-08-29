//! Shared Dioxus application components.

use std::collections::HashMap;

use dioxus::prelude::*;
use styrene_ui_state::{
    Conversation, DeliveryEvidence, LocalAnnounceOutcome, Message, MobileFixture, MobileStore,
    Peer, PropagationEvidence, PropagationUpdate, RuntimeBoundary, SyncState, TargetClass,
    TransportEvidence,
};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct BackNavigation {
    web_history: bool,
}

impl BackNavigation {
    #[must_use]
    pub const fn web_history() -> Self {
        Self { web_history: true }
    }

    fn open_thread(self) {
        if self.web_history {
            document::eval(
                r##"
                if (matchMedia("(max-width: 51.999rem)").matches
                    && history.state?.styrenePane !== "thread") {
                    history.pushState({ styrenePane: "thread" }, "", "#thread");
                }
                "##,
            );
        }
    }

    fn close_thread(self) {
        if self.web_history {
            document::eval(
                r#"
                if (history.state?.styrenePane === "thread") {
                    history.back();
                }
                "#,
            );
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum MobileDestination {
    Messages,
    People,
    Network,
    More,
}

impl MobileDestination {
    fn label(self) -> &'static str {
        match self {
            Self::Messages => "Messages",
            Self::People => "People",
            Self::Network => "Network",
            Self::More => "More",
        }
    }

    fn id(self) -> &'static str {
        match self {
            Self::Messages => "messages",
            Self::People => "people",
            Self::Network => "network",
            Self::More => "more",
        }
    }

    fn mark(self) -> &'static str {
        match self {
            Self::Messages => "M",
            Self::People => "P",
            Self::Network => "N",
            Self::More => "+",
        }
    }
}

fn short_hash(hash: &str) -> String {
    hash.chars().take(8).collect()
}

fn hash_glyph(hash: &str) -> String {
    hash.chars().take(2).flat_map(char::to_uppercase).collect()
}

fn peer_name(hash: &str, peers: &[Peer]) -> String {
    peers
        .iter()
        .find(|peer| peer.destination_hash == hash)
        .and_then(|peer| peer.display_name.clone())
        .unwrap_or_else(|| format!("Peer {}", short_hash(hash)))
}

#[component]
pub fn MobileShell(
    target: TargetClass,
    fixture: MobileFixture,
    #[props(default)] back_navigation: BackNavigation,
) -> Element {
    let boundary = RuntimeBoundary::from(fixture.profile);
    let messaging_available = MobileStore::new(fixture.clone()).messaging_available();
    let live_actions_enabled = boundary.live_network_allowed();
    let mut destination = use_signal(|| MobileDestination::Messages);
    let mut selected_peer = use_signal(|| None::<String>);
    let mut compact_thread_open = use_signal(|| false);

    let active_destination = *destination.read();
    let selected_hash = selected_peer
        .read()
        .clone()
        .filter(|peer_hash| {
            fixture.conversations.iter().any(|conversation| &conversation.peer_hash == peer_hash)
                || fixture.peers.iter().any(|peer| &peer.destination_hash == peer_hash)
        })
        .or_else(|| {
            fixture.conversations.first().map(|conversation| conversation.peer_hash.clone())
        });
    let selected_conversation = selected_hash.as_ref().and_then(|peer_hash| {
        fixture
            .conversations
            .iter()
            .find(|conversation| &conversation.peer_hash == peer_hash)
            .cloned()
    });
    let selected_messages = selected_hash.as_ref().map_or_else(Vec::new, |peer_hash| {
        fixture.messages.iter().filter(|message| &message.peer_hash == peer_hash).cloned().collect()
    });
    let selected_name = selected_hash
        .as_deref()
        .map_or_else(|| "Conversation".into(), |hash| peer_name(hash, &fixture.peers));
    let selected_short_hash = selected_hash.as_deref().map(short_hash).unwrap_or_default();
    let composer_enabled =
        selected_conversation.is_some() && messaging_available && live_actions_enabled;
    let compact_pane = if *compact_thread_open.read() { "thread" } else { "list" };
    let compact_thread_is_open = *compact_thread_open.read();
    let conversation_count = fixture.conversations.len().to_string();
    let peer_count = fixture.peers.len().to_string();

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
            if back_navigation.web_history {
                button {
                    id: "mobile.platform-back",
                    hidden: true,
                    r#type: "button",
                    tabindex: "-1",
                    onclick: move |_| compact_thread_open.set(false),
                    "Platform Back"
                }
            }
            header {
                class: "app-header",
                div {
                    p { class: "app-kicker", "Styrene" }
                    h1 { id: "mobile.app-title", {active_destination.label()} }
                }
                div {
                    id: "mobile.session-state",
                    class: "session-status",
                    role: "status",
                    "aria-live": "polite",
                    "aria-atomic": "true",
                    "data-phase": fixture.session.phase.as_str(),
                    {fixture.session.phase.as_str()}
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
            section {
                id: "mobile.messages",
                class: "app-surface messages-section",
                "aria-labelledby": "mobile.messages-heading",
                hidden: active_destination != MobileDestination::Messages,
                div {
                    class: "message-workspace",
                    "data-compact-pane": compact_pane,
                    div {
                        class: "conversation-pane",
                        div {
                            class: "section-heading",
                            div {
                                p { class: "section-kicker", "Inbox" }
                                h2 { id: "mobile.messages-heading", "Conversations" }
                            }
                            span {
                                class: "count-badge",
                                {conversation_count.clone()}
                            }
                        }
                        ConversationList {
                            conversations: fixture.conversations.clone(),
                            peers: fixture.peers.clone(),
                            selected_peer: selected_hash.clone(),
                            on_select: move |peer_hash| {
                                selected_peer.set(Some(peer_hash));
                                if !compact_thread_is_open {
                                    back_navigation.open_thread();
                                }
                                compact_thread_open.set(true);
                            },
                        }
                    }
                    div {
                        class: "thread-pane",
                        header {
                            class: "thread-header",
                            button {
                                class: "thread-back",
                                r#type: "button",
                                "aria-label": "Back to conversations",
                                onclick: move |_| {
                                    compact_thread_open.set(false);
                                    back_navigation.close_thread();
                                },
                                "Back"
                            }
                            div {
                                h2 { {selected_name.clone()} }
                                if !selected_short_hash.is_empty() {
                                    p { class: "technical-value", {selected_short_hash.clone()} }
                                }
                            }
                        }
                        MessageHistory {
                            messages: selected_messages,
                            has_selection: selected_hash.is_some(),
                            actions_enabled: live_actions_enabled,
                        }
                        Composer {
                            conversation: selected_conversation,
                            enabled: composer_enabled,
                        }
                    }
                }
            }
            section {
                id: "mobile.people",
                class: "app-surface directory-surface",
                "aria-labelledby": "mobile.people-heading",
                hidden: active_destination != MobileDestination::People,
                div {
                    class: "section-heading",
                    div {
                        p { class: "section-kicker", "Directory" }
                        h2 { id: "mobile.people-heading", "People" }
                    }
                    span { class: "count-badge", {peer_count.clone()} }
                }
                if fixture.peers.is_empty() {
                    div {
                        class: "empty-state",
                        h3 { "No peers discovered" }
                        p { "Announced peers will appear here." }
                    }
                }
                for peer in &fixture.peers {
                    button {
                        id: format!("mobile.peer.{}", peer.destination_hash),
                        class: "peer-card",
                        r#type: "button",
                        "data-aspect": peer.aspect.clone(),
                        "data-source": "canonical_announce",
                        onclick: {
                            let peer_hash = peer.destination_hash.clone();
                            move |_| {
                                selected_peer.set(Some(peer_hash.clone()));
                                if !compact_thread_is_open {
                                    back_navigation.open_thread();
                                }
                                compact_thread_open.set(true);
                                destination.set(MobileDestination::Messages);
                            }
                        },
                        span { class: "hash-glyph", {hash_glyph(&peer.destination_hash)} }
                        span {
                            class: "directory-copy",
                            strong {
                                {peer.display_name.clone().unwrap_or_else(|| format!("Peer {}", short_hash(&peer.destination_hash)))}
                            }
                            span { class: "technical-value", {short_hash(&peer.destination_hash)} }
                        }
                        span { class: "row-action", "Open" }
                    }
                }
            }
            section {
                id: "mobile.network",
                class: "app-surface network-surface",
                "aria-labelledby": "mobile.network-heading",
                hidden: active_destination != MobileDestination::Network,
                div {
                    class: "section-heading",
                    div {
                        p { class: "section-kicker", "Connectivity" }
                        h2 { id: "mobile.network-heading", "Network" }
                    }
                }
                div {
                    class: "settings-card",
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
                        "Host and port, for example rns.styrene.io:4242."
                    }
                    button {
                        id: "mobile.tcp-endpoint-apply",
                        disabled: !live_actions_enabled,
                        "Apply endpoint"
                    }
                }
                h3 { class: "group-heading", "Bearers" }
                for bearer in &fixture.bearers {
                    article {
                        id: format!("mobile.bearer.{}", bearer.kind.as_str()),
                        class: "bearer-card",
                        "data-state": bearer.state.to_string(),
                        "data-reason": bearer.reason.clone().unwrap_or_default(),
                        div {
                            h3 { {bearer.kind.as_str()} }
                            if let Some(reason) = &bearer.reason {
                                p { class: "field-hint", {reason.clone()} }
                            }
                        }
                        span { class: "state-chip", {bearer.state.to_string()} }
                    }
                }
                PropagationPanel {
                    propagation: PropagationUpdate::from_fixture(&fixture),
                    actions_enabled: live_actions_enabled,
                }
            }
            section {
                id: "mobile.more",
                class: "app-surface more-surface",
                "aria-labelledby": "mobile.more-heading",
                hidden: active_destination != MobileDestination::More,
                div {
                    class: "section-heading",
                    div {
                        p { class: "section-kicker", "This device" }
                        h2 { id: "mobile.more-heading", "More" }
                    }
                }
                article {
                    class: "settings-card identity-card",
                    h3 { "Node identity" }
                    p {
                        id: "mobile.identity",
                        class: "identity",
                        "aria-label": format!("Local identity {}", fixture.session.identity_hash),
                        {fixture.session.identity_hash.clone()}
                    }
                }
                article {
                    class: "settings-card",
                    h3 { "About this build" }
                    p { "Rust-owned Dioxus mobile shell" }
                    p { class: "technical-value", "Generation {fixture.generation}" }
                }
            }
            nav {
                class: "destination-bar",
                "aria-label": "Primary",
                for item in [
                    MobileDestination::Messages,
                    MobileDestination::People,
                    MobileDestination::Network,
                    MobileDestination::More,
                ] {
                    button {
                        id: format!("mobile.destination.{}", item.id()),
                        class: if active_destination == item { "destination-item is-active" } else { "destination-item" },
                        r#type: "button",
                        "aria-current": if active_destination == item { "page" } else { "false" },
                        onclick: move |_| {
                            if item != MobileDestination::Messages && compact_thread_is_open {
                                compact_thread_open.set(false);
                                back_navigation.close_thread();
                            }
                            destination.set(item);
                        },
                        span { class: "destination-mark", "aria-hidden": "true", {item.mark()} }
                        span { {item.label()} }
                    }
                }
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
pub fn ConversationList(
    conversations: Vec<Conversation>,
    peers: Vec<Peer>,
    selected_peer: Option<String>,
    on_select: EventHandler<String>,
) -> Element {
    rsx! {
        nav {
            id: "mobile.conversations",
            class: "conversation-list",
            "aria-label": "Conversations",
            if conversations.is_empty() {
                div {
                    id: "mobile.messages-empty",
                    class: "empty-state",
                    h3 { "No conversations yet" }
                    p { "Discover a peer to begin a private conversation." }
                }
            }
            for conversation in conversations {
                {
                    let is_selected = selected_peer.as_deref() == Some(conversation.peer_hash.as_str());
                    let name = peer_name(&conversation.peer_hash, &peers);
                    let peer_short_hash = short_hash(&conversation.peer_hash);
                    let hash_glyph = hash_glyph(&conversation.peer_hash);
                    let selected_hash = conversation.peer_hash.clone();
                    rsx! {
                button {
                    id: format!("mobile.conversation.{}", conversation.peer_hash),
                    class: if is_selected { "conversation-row is-selected" } else { "conversation-row" },
                    r#type: "button",
                    "data-peer": conversation.peer_hash.clone(),
                    "aria-current": if is_selected { "true" } else { "false" },
                    onclick: move |_| on_select.call(selected_hash.clone()),
                    span { class: "hash-glyph", {hash_glyph} }
                    span {
                        class: "conversation-copy",
                        strong { {name} }
                        span { class: "technical-value", {peer_short_hash} }
                    }
                    if conversation.unread_count > 0 {
                        span {
                            id: format!("mobile.conversation-unread.{}", conversation.peer_hash),
                            class: "unread-badge",
                            "aria-label": format!("{} unread messages", conversation.unread_count),
                            "{conversation.unread_count}"
                        }
                    }
                }
                    }
                }
            }
        }
    }
}

#[component]
pub fn MessageHistory(
    messages: Vec<Message>,
    has_selection: bool,
    actions_enabled: bool,
) -> Element {
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
            if !has_selection {
                div {
                    class: "empty-state thread-empty",
                    h3 { "Choose a conversation" }
                    p { "Messages and delivery evidence will appear here." }
                }
            } else if messages.is_empty() {
                div {
                    class: "empty-state thread-empty",
                    h3 { "No messages with this peer" }
                    p { "Write a message below to start the conversation." }
                }
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
pub fn Composer(conversation: Option<Conversation>, enabled: bool) -> Element {
    let mut draft_buffers = use_signal(HashMap::<String, (u64, String)>::new);
    let draft_id = conversation.as_ref().map_or_else(
        || "mobile.draft".to_string(),
        |conversation| format!("mobile.draft.{}", conversation.peer_hash),
    );
    let peer_hash = conversation.as_ref().map(|conversation| conversation.peer_hash.clone());
    let draft_revision =
        conversation.as_ref().map_or(0, |conversation| conversation.draft_revision);
    let draft = conversation.as_ref().map_or_else(String::new, |conversation| {
        draft_buffers
            .read()
            .get(&conversation.peer_hash)
            .filter(|(revision, _)| *revision == conversation.draft_revision)
            .map_or_else(|| conversation.draft.clone(), |(_, draft)| draft.clone())
    });
    let enabled = enabled && conversation.is_some();
    rsx! {
        form {
            key: "{draft_id}",
            id: "mobile.composer",
            class: "composer",
            "data-peer": peer_hash.clone(),
            div {
                class: "composer-row",
                label { class: "visually-hidden", r#for: draft_id.clone(), "Message" }
                textarea {
                    id: draft_id,
                    name: "message",
                    rows: "2",
                    placeholder: "Message",
                    "data-revision": draft_revision,
                    value: draft,
                    oninput: {
                        let peer_hash = peer_hash.clone();
                        move |event| {
                            if let Some(peer_hash) = &peer_hash {
                                draft_buffers.write().insert(
                                    peer_hash.clone(),
                                    (draft_revision, event.value()),
                                );
                            }
                        }
                    },
                }
                button {
                    id: "mobile.send",
                    r#type: "button",
                    "data-enabled": enabled.to_string(),
                    disabled: !enabled,
                    "Send"
                }
            }
            div {
                class: "delivery-method-row",
                label { r#for: "mobile.delivery-method", "Delivery" }
                select {
                    id: "mobile.delivery-method",
                    name: "delivery-method",
                    option { value: "direct", "Direct" }
                    option { value: "propagated", "Propagated" }
                }
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
