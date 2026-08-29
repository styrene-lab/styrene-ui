//! Shared Dioxus application components.

use std::collections::HashMap;

use dioxus::prelude::*;
use styrene_ui_platform::{
    AndroidUsbAttachment, Appearance, ApplicationLifecycle, AuthorizationState, BleAdapterState,
    BleControlDisabledReason, BleControlFailure, BleControlPhase, BleControlState, BlePeripheralId,
    Contrast, KeyboardGeometry, MotionPreference, PlatformInsets, PlatformSnapshot, TextScale,
    WindowClass,
};
use styrene_ui_state::{
    BearerKind, BearerState, Conversation, DeliveryEvidence, DeliveryMethod, LocalAnnounceOutcome,
    Message, MobileAction, MobileActionKind, MobileFixture, MobileStore, Peer, PropagationEvidence,
    PropagationUpdate, RuntimeBoundary, SyncState, TargetClass, TransportEvidence,
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

const fn authorization_state(state: AuthorizationState) -> &'static str {
    match state {
        AuthorizationState::NotDetermined => "not determined",
        AuthorizationState::Granted => "granted",
        AuthorizationState::Denied => "denied",
        AuthorizationState::Restricted => "restricted",
        AuthorizationState::Unavailable => "unavailable",
    }
}

const fn ble_adapter_state(state: BleAdapterState) -> &'static str {
    match state {
        BleAdapterState::Ready => "ready",
        BleAdapterState::PoweredOff => "powered off",
        BleAdapterState::Resetting => "resetting",
        BleAdapterState::Unsupported => "unsupported",
        BleAdapterState::Unavailable => "unavailable",
    }
}

const fn ble_phase(phase: BleControlPhase) -> &'static str {
    match phase {
        BleControlPhase::Idle => "idle",
        BleControlPhase::Scanning => "scanning",
        BleControlPhase::Connecting => "connecting",
        BleControlPhase::Connected => "connected",
        BleControlPhase::Reconnecting => "reconnecting",
    }
}

const fn ble_disabled_reason(reason: BleControlDisabledReason) -> &'static str {
    match reason {
        BleControlDisabledReason::PermissionRequired => "Bluetooth permission is required",
        BleControlDisabledReason::PermissionDenied => "Bluetooth permission is denied",
        BleControlDisabledReason::PermissionRestricted => "Bluetooth permission is restricted",
        BleControlDisabledReason::PermissionUnavailable => "Bluetooth permission is unavailable",
        BleControlDisabledReason::AdapterUnavailable(BleAdapterState::PoweredOff) => {
            "Bluetooth is powered off"
        }
        BleControlDisabledReason::AdapterUnavailable(BleAdapterState::Resetting) => {
            "Bluetooth is resetting"
        }
        BleControlDisabledReason::AdapterUnavailable(BleAdapterState::Unsupported) => {
            "Bluetooth is unsupported"
        }
        BleControlDisabledReason::AdapterUnavailable(_) => "Bluetooth adapter is unavailable",
        BleControlDisabledReason::OperationInProgress => "Bluetooth operation is in progress",
        BleControlDisabledReason::NoApprovedPeripheral => "No Bluetooth RNode is approved",
        BleControlDisabledReason::NoRetryableFailure => "There is no retryable Bluetooth failure",
    }
}

const fn ble_failure(failure: &BleControlFailure) -> &'static str {
    match failure {
        BleControlFailure::ScanFailed => "Bluetooth scan failed.",
        BleControlFailure::ConnectionInterrupted => "Bluetooth RNode connection was interrupted.",
        BleControlFailure::ConnectionFailed => "Bluetooth RNode connection failed.",
        BleControlFailure::IncompatiblePeripheral => {
            "The selected device does not provide the required Nordic UART service."
        }
        BleControlFailure::PlatformUnavailable => {
            "Bluetooth RNode integration is unavailable in this build."
        }
    }
}

#[component]
pub fn BleRNodeControls(
    state: BleControlState,
    actions_enabled: bool,
    #[props(default)] scan: Option<EventHandler<()>>,
    #[props(default)] select: Option<EventHandler<BlePeripheralId>>,
    #[props(default)] retry: Option<EventHandler<()>>,
    #[props(default)] forget: Option<EventHandler<()>>,
) -> Element {
    let scan_reason = state.scan_disabled_reason();
    let selection_reason = state.selection_disabled_reason();
    let retry_reason = state.retry_disabled_reason();
    let forget_reason = state.forget_disabled_reason();
    let scan_disabled = !actions_enabled || scan.is_none() || scan_reason.is_some();
    let selection_disabled = !actions_enabled || select.is_none() || selection_reason.is_some();
    let retry_disabled = !actions_enabled || retry.is_none() || retry_reason.is_some();
    let forget_disabled = !actions_enabled || forget.is_none() || forget_reason.is_some();
    let scan_label = if state.permission == AuthorizationState::NotDetermined {
        "Allow Bluetooth and scan"
    } else if state.phase == BleControlPhase::Scanning {
        "Scanning for RNodes"
    } else {
        "Scan for RNodes"
    };
    let status = if !actions_enabled {
        "Fixture data. Bluetooth actions are disabled."
    } else if let Some(reason) = scan_reason {
        ble_disabled_reason(reason)
    } else if scan.is_none() {
        "Bluetooth adapter actions are unavailable."
    } else {
        "Bluetooth is ready to scan."
    };

    rsx! {
        article {
            id: "mobile.bluetooth-rnode",
            class: "settings-card bluetooth-card",
            "aria-labelledby": "mobile.bluetooth-rnode-heading",
            "data-phase": ble_phase(state.phase),
            "data-permission": authorization_state(state.permission),
            "data-adapter": ble_adapter_state(state.adapter),
            div {
                class: "settings-card-heading",
                div {
                    h3 { id: "mobile.bluetooth-rnode-heading", "Bluetooth RNode" }
                    p { class: "field-hint", "Scan only when your RNode is in its pairing window. Selection is required before connection." }
                }
                span { class: "state-chip", {ble_phase(state.phase)} }
            }
            button {
                id: "mobile.bluetooth-scan",
                r#type: "button",
                class: "secondary-action",
                disabled: scan_disabled,
                "aria-describedby": "mobile.bluetooth-status",
                onclick: move |_| {
                    if let Some(handler) = scan {
                        handler.call(());
                    }
                },
                {scan_label}
            }
            p {
                id: "mobile.bluetooth-status",
                class: "field-hint",
                role: "status",
                "aria-live": "polite",
                "aria-atomic": "true",
                {status}
            }
            if let Some(approved) = &state.approved {
                div {
                    class: "approved-peripheral",
                    div {
                        strong { "Approved RNode" }
                        p { class: "technical-value", {approved.id.as_str()} }
                    }
                    button {
                        id: "mobile.bluetooth-forget",
                        r#type: "button",
                        class: "secondary-action danger-action",
                        disabled: forget_disabled,
                        "aria-describedby": "mobile.bluetooth-status",
                        onclick: move |_| {
                            if let Some(handler) = forget {
                                handler.call(());
                            }
                        },
                        "Forget"
                    }
                }
            }
            if state.candidates.is_empty() {
                p { class: "field-hint", "No compatible RNodes discovered." }
            } else {
                ul { class: "peripheral-list", "aria-label": "Discovered Bluetooth RNodes",
                    for candidate in state.candidates.clone() {
                        {
                            let candidate_id = candidate.id.clone();
                            let display_name = candidate
                                .display_name
                                .clone()
                                .unwrap_or_else(|| "Unnamed RNode".into());
                            let action_name = format!("Approve and connect {display_name}");
                            rsx! {
                                li {
                                    class: "peripheral-row",
                                    "data-peripheral-id": candidate.id.as_str(),
                                    div {
                                        strong { {display_name} }
                                        p { class: "technical-value", {candidate.id.as_str()} }
                                        if let Some(rssi) = candidate.rssi_dbm {
                                            p { class: "field-hint", "Signal {rssi} dBm" }
                                        }
                                    }
                                    button {
                                        r#type: "button",
                                        class: "secondary-action",
                                        disabled: selection_disabled,
                                        "aria-label": action_name,
                                        "aria-describedby": if selection_reason.is_some() { "mobile.bluetooth-selection-disabled" } else { "mobile.bluetooth-status" },
                                        onclick: move |_| {
                                            if let Some(handler) = select {
                                                handler.call(candidate_id.clone());
                                            }
                                        },
                                        "Use RNode"
                                    }
                                }
                            }
                        }
                    }
                }
                if let Some(reason) = selection_reason {
                    p {
                        id: "mobile.bluetooth-selection-disabled",
                        class: "field-hint",
                        {ble_disabled_reason(reason)}
                    }
                }
            }
            if let Some(failure) = &state.failure {
                p {
                    id: "mobile.bluetooth-failure",
                    class: "field-error",
                    role: if failure == &BleControlFailure::PlatformUnavailable { "status" } else { "alert" },
                    "aria-live": if failure == &BleControlFailure::PlatformUnavailable { "polite" } else { "assertive" },
                    "data-retryable": failure.is_retryable().to_string(),
                    {ble_failure(failure)}
                }
                button {
                    id: "mobile.bluetooth-retry",
                    r#type: "button",
                    class: "secondary-action",
                    disabled: retry_disabled,
                    "aria-describedby": if retry_reason.is_some() { "mobile.bluetooth-failure mobile.bluetooth-retry-disabled" } else { "mobile.bluetooth-failure" },
                    onclick: move |_| {
                        if let Some(handler) = retry {
                            handler.call(());
                        }
                    },
                    "Retry Bluetooth connection"
                }
                if let Some(reason) = retry_reason {
                    p {
                        id: "mobile.bluetooth-retry-disabled",
                        class: "field-hint",
                        {ble_disabled_reason(reason)}
                    }
                }
            }
        }
    }
}

#[component]
pub fn MobileShell(
    target: TargetClass,
    fixture: MobileFixture,
    #[props(default)] back_navigation: BackNavigation,
    #[props(default)] action_sink: Option<EventHandler<MobileAction>>,
    #[props(default)] propagation: Option<PropagationUpdate>,
    #[props(default)] platform_snapshot: Option<PlatformSnapshot>,
    #[props(default)] android_usb_attachments: Vec<AndroidUsbAttachment>,
    #[props(default)] android_usb_authorization: Option<AuthorizationState>,
    #[props(default)] android_usb_failure: Option<String>,
    #[props(default)] android_usb_busy: bool,
    #[props(default)] android_usb_refresh: Option<EventHandler<()>>,
    #[props(default)] android_usb_select: Option<EventHandler<AndroidUsbAttachment>>,
    #[props(default)] android_usb_probe: Option<EventHandler<()>>,
    #[props(default)] android_usb_probe_status: Option<String>,
    #[props(default)] ble_controls: BleControlState,
    #[props(default)] ble_scan: Option<EventHandler<()>>,
    #[props(default)] ble_select: Option<EventHandler<BlePeripheralId>>,
    #[props(default)] ble_retry: Option<EventHandler<()>>,
    #[props(default)] ble_forget: Option<EventHandler<()>>,
) -> Element {
    let boundary = RuntimeBoundary::from(fixture.profile);
    let messaging_available = MobileStore::new(fixture.clone()).messaging_available();
    let live_actions_enabled = boundary.live_network_allowed();
    let action_sink = live_actions_enabled.then_some(action_sink).flatten();
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
    let android_usb_connected = fixture.bearers.iter().any(|bearer| {
        bearer.kind == BearerKind::AndroidUsb && bearer.state == BearerState::Connected
    });
    let compact_pane = if *compact_thread_open.read() { "thread" } else { "list" };
    let compact_thread_is_open = *compact_thread_open.read();
    let conversation_count = fixture.conversations.len().to_string();
    let peer_count = fixture.peers.len().to_string();
    let window_class = platform_snapshot.as_ref().map(|snapshot| match snapshot.window.class {
        WindowClass::Compact => "compact",
        WindowClass::Wide => "wide",
    });
    let appearance =
        platform_snapshot.as_ref().map(|snapshot| match snapshot.accessibility.appearance {
            Appearance::Light => "light",
            Appearance::Dark => "dark",
        });
    let contrast =
        platform_snapshot.as_ref().map(|snapshot| match snapshot.accessibility.contrast {
            Contrast::Standard => "standard",
            Contrast::Increased => "increased",
        });
    let motion = platform_snapshot.as_ref().map(|snapshot| match snapshot.accessibility.motion {
        MotionPreference::Full => "full",
        MotionPreference::Reduced => "reduced",
    });
    let text_scale =
        platform_snapshot.as_ref().map(|snapshot| match snapshot.accessibility.text_scale {
            TextScale::Percent(_) => "percent",
            TextScale::Category(_) => "category",
            TextScale::Unavailable => "unavailable",
        });
    let text_scale_percent =
        platform_snapshot.as_ref().and_then(|snapshot| match snapshot.accessibility.text_scale {
            TextScale::Percent(percent) => Some(percent.to_string()),
            TextScale::Category(_) | TextScale::Unavailable => None,
        });
    let text_scale_category =
        platform_snapshot.as_ref().and_then(|snapshot| match snapshot.accessibility.text_scale {
            TextScale::Category(category) => Some(category.as_str()),
            TextScale::Percent(_) | TextScale::Unavailable => None,
        });
    let lifecycle = platform_snapshot.as_ref().map(|snapshot| match snapshot.lifecycle {
        ApplicationLifecycle::Active => "active",
        ApplicationLifecycle::Inactive => "inactive",
        ApplicationLifecycle::Background => "background",
    });
    let keyboard_visible =
        platform_snapshot.as_ref().map(|snapshot| match snapshot.geometry.keyboard {
            KeyboardGeometry::WebViewManaged { visible } => visible.to_string(),
            KeyboardGeometry::NativeBridge { occluded_height_css_px } => {
                (occluded_height_css_px > 0).to_string()
            }
        });
    let insets = platform_snapshot.as_ref().map(|snapshot| match snapshot.geometry.insets {
        PlatformInsets::CssEnvironment => "css-environment",
        PlatformInsets::NativeBridge(_) => "native-bridge",
    });

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
            "data-window-class": window_class,
            "data-appearance": appearance,
            "data-contrast": contrast,
            "data-motion": motion,
            "data-text-scale": text_scale,
            "data-text-scale-percent": text_scale_percent,
            "data-text-scale-category": text_scale_category,
            "data-lifecycle": lifecycle,
            "data-keyboard-visible": keyboard_visible,
            "data-insets": insets,
            if back_navigation.web_history {
                button {
                    id: "mobile.platform-back",
                    hidden: true,
                    r#type: "button",
                    tabindex: "-1",
                    onclick: move |_| {
                        compact_thread_open.set(false);
                        if let Some(action_sink) = action_sink {
                            action_sink.call(MobileAction::new(
                                fixture.generation,
                                MobileActionKind::SetActiveConversation { peer_hash: None },
                            ));
                        }
                    },
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
            if let Some(failure) = &fixture.session.failure {
                div {
                    id: "mobile.session-failure",
                    class: "failure-banner",
                    role: "status",
                    "aria-live": "polite",
                    "data-code": failure.code.clone(),
                    "data-retryable": failure.retryable.to_string(),
                    "Session unavailable ({failure.code})"
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
                            on_select: move |peer_hash: String| {
                                selected_peer.set(Some(peer_hash.clone()));
                                if let Some(action_sink) = action_sink {
                                    action_sink.call(MobileAction::new(
                                        fixture.generation,
                                        MobileActionKind::SetActiveConversation {
                                            peer_hash: Some(peer_hash),
                                        },
                                    ));
                                }
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
                                    if let Some(action_sink) = action_sink {
                                        action_sink.call(MobileAction::new(
                                            fixture.generation,
                                            MobileActionKind::SetActiveConversation {
                                                peer_hash: None,
                                            },
                                        ));
                                    }
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
                            generation: fixture.generation,
                            action_sink,
                        }
                        Composer {
                            conversation: selected_conversation,
                            enabled: composer_enabled,
                            generation: fixture.generation,
                            action_sink,
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
                                if let Some(action_sink) = action_sink {
                                    action_sink.call(MobileAction::new(
                                        fixture.generation,
                                        MobileActionKind::SetActiveConversation {
                                            peer_hash: Some(peer_hash.clone()),
                                        },
                                    ));
                                }
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
                EndpointEditor {
                    key: "{fixture.generation}",
                    endpoint: fixture.session.endpoint.clone().unwrap_or_default(),
                    generation: fixture.generation,
                    enabled: live_actions_enabled,
                    action_sink,
                }
                BleRNodeControls {
                    state: ble_controls,
                    actions_enabled: live_actions_enabled,
                    scan: ble_scan,
                    select: ble_select,
                    retry: ble_retry,
                    forget: ble_forget,
                }
                if target == TargetClass::Android
                    && (android_usb_refresh.is_some() || !android_usb_attachments.is_empty())
                {
                    article {
                        id: "mobile.android-usb",
                        class: "settings-card",
                        "aria-labelledby": "mobile.android-usb-heading",
                        div {
                            h3 { id: "mobile.android-usb-heading", "Android USB fallback" }
                            p { class: "field-hint", "Explicitly choose an attached USB device. Bluetooth remains preferred." }
                        }
                        button {
                            r#type: "button",
                            class: "secondary-action",
                            disabled: android_usb_busy,
                            onclick: move |_| {
                                if let Some(handler) = android_usb_refresh {
                                    handler.call(());
                                }
                            },
                            if android_usb_busy { "USB request in progress" } else { "Refresh USB devices" }
                        }
                        if android_usb_attachments.is_empty() {
                            p { class: "field-hint", "No attached USB host devices." }
                        }
                        for attachment in android_usb_attachments.clone() {
                            article {
                                class: "bearer-card",
                                "data-device-id": attachment.device_id.to_string(),
                                "data-vendor-id": attachment.vendor_id.to_string(),
                                "data-product-id": attachment.product_id.to_string(),
                                div {
                                    h3 { "USB device {attachment.device_id}" }
                                    p { class: "technical-value", {format!("{:04x}:{:04x}", attachment.vendor_id, attachment.product_id)} }
                                }
                                button {
                                    r#type: "button",
                                    class: "secondary-action",
                                    disabled: android_usb_busy,
                                    onclick: move |_| {
                                        if let Some(handler) = android_usb_select {
                                            handler.call(attachment.clone());
                                        }
                                    },
                                    "Use USB"
                                }
                            }
                        }
                        if let Some(authorization) = android_usb_authorization {
                            p {
                                class: "field-hint",
                                role: "status",
                                "USB authorization: {authorization_state(authorization)}"
                            }
                        }
                        if android_usb_probe.is_some() {
                            button {
                                id: "mobile.android-usb-probe",
                                r#type: "button",
                                class: "secondary-action",
                                disabled: android_usb_busy || !android_usb_connected,
                                onclick: move |_| {
                                    if let Some(handler) = android_usb_probe {
                                        handler.call(());
                                    }
                                },
                                "Test RNode packet"
                            }
                        }
                        if let Some(status) = &android_usb_probe_status {
                            p {
                                id: "mobile.android-usb-probe-status",
                                class: "field-hint",
                                role: "status",
                                "aria-live": "polite",
                                "aria-atomic": "true",
                                {status.clone()}
                            }
                        }
                        if let Some(failure) = &android_usb_failure {
                            p { class: "field-error", role: "alert", {failure.clone()} }
                        }
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
                    propagation: propagation
                        .unwrap_or_else(|| PropagationUpdate::from_fixture(&fixture)),
                    actions_enabled: live_actions_enabled,
                    action_sink,
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
                                if let Some(action_sink) = action_sink {
                                    action_sink.call(MobileAction::new(
                                        fixture.generation,
                                        MobileActionKind::SetActiveConversation {
                                            peer_hash: None,
                                        },
                                    ));
                                }
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
fn EndpointEditor(
    endpoint: String,
    generation: u64,
    enabled: bool,
    action_sink: Option<EventHandler<MobileAction>>,
) -> Element {
    let mut endpoint_buffer = use_signal(|| endpoint);
    rsx! {
        div {
            class: "settings-card",
            label { r#for: "mobile.tcp-endpoint", "TCP endpoint" }
            input {
                id: "mobile.tcp-endpoint",
                name: "tcp-endpoint",
                r#type: "text",
                inputmode: "url",
                "aria-describedby": "mobile.tcp-endpoint-hint",
                value: endpoint_buffer,
                oninput: move |event| endpoint_buffer.set(event.value()),
            }
            p {
                id: "mobile.tcp-endpoint-hint",
                class: "field-hint",
                "Host and port, for example rns.styrene.io:4242."
            }
            button {
                id: "mobile.tcp-endpoint-apply",
                r#type: "button",
                disabled: !enabled,
                onclick: move |_| {
                    if enabled && let Some(action_sink) = action_sink {
                        action_sink.call(MobileAction::new(
                            generation,
                            MobileActionKind::ApplyEndpoint {
                                endpoint: endpoint_buffer.read().clone(),
                            },
                        ));
                    }
                },
                "Apply endpoint"
            }
        }
    }
}

#[component]
pub fn PropagationPanel(
    propagation: PropagationUpdate,
    actions_enabled: bool,
    #[props(default)] action_sink: Option<EventHandler<MobileAction>>,
) -> Element {
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
                disabled: !actions_enabled,
                onchange: move |event| {
                    if actions_enabled && let Some(action_sink) = action_sink {
                        let destination_hash = (!event.value().is_empty()).then(|| event.value());
                        action_sink.call(MobileAction::new(
                            propagation.generation,
                            MobileActionKind::SelectPropagationNode { destination_hash },
                        ));
                    }
                },
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
                onclick: move |_| {
                    if actions_enabled
                        && propagation.ready
                        && propagation.sync_state != SyncState::InProgress
                        && let Some(action_sink) = action_sink
                    {
                        action_sink.call(MobileAction::new(
                            propagation.generation,
                            MobileActionKind::SyncPropagation,
                        ));
                    }
                },
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
    generation: u64,
    #[props(default)] action_sink: Option<EventHandler<MobileAction>>,
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
                                generation,
                                action_sink,
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
pub fn DeliveryDetail(
    message: Message,
    actions_enabled: bool,
    generation: u64,
    #[props(default)] action_sink: Option<EventHandler<MobileAction>>,
) -> Element {
    let state = if message.delivery == DeliveryEvidence::Delivered {
        "Delivered"
    } else if let Some(lifecycle) = message.lifecycle {
        match lifecycle {
            styrene_ui_state::MessageLifecycle::Queued => "Queued",
            styrene_ui_state::MessageLifecycle::Sending => "Sending",
            styrene_ui_state::MessageLifecycle::Sent => "Sent; recipient delivery pending",
            styrene_ui_state::MessageLifecycle::Delivered => "Delivered",
            styrene_ui_state::MessageLifecycle::Failed => "Failed",
            styrene_ui_state::MessageLifecycle::Cancelled => "Cancelled",
            styrene_ui_state::MessageLifecycle::Expired => "Expired",
            styrene_ui_state::MessageLifecycle::Rejected => "Rejected",
            styrene_ui_state::MessageLifecycle::Unknown => "Delivery state unavailable",
        }
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
                    onclick: {
                        let message_id = message.id.clone();
                        move |_| {
                            if actions_enabled && let Some(action_sink) = action_sink {
                                action_sink.call(MobileAction::new(
                                    generation,
                                    MobileActionKind::RetryMessage {
                                        message_id: message_id.clone(),
                                    },
                                ));
                            }
                        }
                    },
                    "Retry"
                }
            }
        }
    }
}

#[component]
pub fn Composer(
    conversation: Option<Conversation>,
    enabled: bool,
    #[props(default)] generation: u64,
    #[props(default)] action_sink: Option<EventHandler<MobileAction>>,
) -> Element {
    let mut draft_buffers = use_signal(HashMap::<String, (u64, String)>::new);
    let mut delivery_method = use_signal(|| DeliveryMethod::Direct);
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
    let enabled = enabled && conversation.is_some() && !draft.trim().is_empty();
    rsx! {
        form {
            key: "{draft_id}",
            id: "mobile.composer",
            class: "composer",
            "data-peer": peer_hash.clone(),
            onsubmit: {
                let peer_hash = peer_hash.clone();
                let draft = draft.clone();
                move |event| {
                    event.prevent_default();
                    if enabled
                        && let (Some(action_sink), Some(peer_hash)) = (action_sink, &peer_hash)
                    {
                        action_sink.call(MobileAction::new(
                            generation,
                            MobileActionKind::SendMessage {
                                peer_hash: peer_hash.clone(),
                                content: draft.clone(),
                                requested_method: *delivery_method.read(),
                                draft_revision,
                            },
                        ));
                    }
                }
            },
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
                                let content = event.value();
                                draft_buffers.write().insert(
                                    peer_hash.clone(),
                                    (draft_revision, content.clone()),
                                );
                                if let Some(action_sink) = action_sink {
                                    action_sink.call(MobileAction::new(
                                        generation,
                                        MobileActionKind::SaveDraft {
                                            peer_hash: peer_hash.clone(),
                                            content,
                                            base_revision: draft_revision,
                                        },
                                    ));
                                }
                            }
                        }
                    },
                }
                button {
                    id: "mobile.send",
                    r#type: "submit",
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
                    value: match *delivery_method.read() {
                        DeliveryMethod::Direct => "direct",
                        DeliveryMethod::Opportunistic => "opportunistic",
                        DeliveryMethod::Propagated => "propagated",
                        DeliveryMethod::Unknown => "unknown",
                    },
                    onchange: move |event| {
                        delivery_method.set(if event.value() == "propagated" {
                            DeliveryMethod::Propagated
                        } else {
                            DeliveryMethod::Direct
                        });
                    },
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
