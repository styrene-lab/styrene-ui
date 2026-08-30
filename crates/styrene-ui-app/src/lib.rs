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
    BearerKind, BearerState, Conversation, DeliveryEvidence, DeliveryMethod,
    IdentityCustodyAuthentication, IdentityCustodyAvailability, IdentityCustodyBackend,
    IdentityCustodyDowngrade, IdentityCustodyProtection, LocalAnnounceOutcome, Message,
    MobileAction, MobileActionKind, MobileFixture, MobileStore, Peer, PropagationEvidence,
    PropagationUpdate, RuntimeBoundary, SessionPhase, SyncState, TargetClass, TransportEvidence,
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
            Self::More => "...",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum StatusTone {
    Positive,
    Caution,
    Negative,
    Neutral,
}

impl StatusTone {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Positive => "positive",
            Self::Caution => "caution",
            Self::Negative => "negative",
            Self::Neutral => "neutral",
        }
    }

    const fn for_session(phase: SessionPhase) -> Self {
        match phase {
            SessionPhase::Connected => Self::Positive,
            SessionPhase::Starting | SessionPhase::Reconnecting => Self::Caution,
            SessionPhase::Failed => Self::Negative,
            SessionPhase::Offline => Self::Neutral,
        }
    }

    const fn for_bearer(state: BearerState) -> Self {
        match state {
            BearerState::Connected => Self::Positive,
            BearerState::Reconnecting | BearerState::Unverified => Self::Caution,
            BearerState::Disconnected => Self::Negative,
            BearerState::Unavailable => Self::Neutral,
        }
    }

    const fn for_ble(phase: BleControlPhase) -> Self {
        match phase {
            BleControlPhase::Connected => Self::Positive,
            BleControlPhase::Scanning
            | BleControlPhase::Connecting
            | BleControlPhase::Reconnecting => Self::Caution,
            BleControlPhase::Idle => Self::Neutral,
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

const fn bearer_label(kind: BearerKind) -> &'static str {
    match kind {
        BearerKind::Tcp => "TCP",
        BearerKind::BluetoothRnode => "Bluetooth RNode",
        BearerKind::AndroidUsb => "Android USB",
    }
}

fn bearer_reason_label(reason: &str) -> &'static str {
    match reason {
        "approved_device_absent" => "The approved device is not currently available.",
        "connection_interrupted" | "socket_closed" => "The connection was interrupted.",
        "invalid_tcp_endpoint" => "The configured TCP endpoint is invalid.",
        "not_configured" => "This bearer is not configured.",
        "not_selected" => "No device is selected.",
        "permission_denied" => "Permission was denied.",
        "physical_evidence_absent" => "A physical-device connection has not been verified.",
        _ => "Additional diagnostic details are available for this bearer.",
    }
}

const fn custody_backend_label(backend: IdentityCustodyBackend) -> &'static str {
    match backend {
        IdentityCustodyBackend::Keychain => "Apple Keychain",
        IdentityCustodyBackend::AndroidKeystore => "Android Keystore",
        IdentityCustodyBackend::EncryptedFile => "Encrypted file",
        IdentityCustodyBackend::PlaintextFile => "Development plaintext file",
    }
}

const fn custody_protection_label(protection: IdentityCustodyProtection) -> &'static str {
    match protection {
        IdentityCustodyProtection::PlatformProtected => "Platform protected",
        IdentityCustodyProtection::EncryptedAtRest => "Encrypted at rest",
        IdentityCustodyProtection::DevelopmentPlaintext => "Development plaintext",
    }
}

const fn custody_authentication_label(
    authentication: IdentityCustodyAuthentication,
) -> &'static str {
    match authentication {
        IdentityCustodyAuthentication::DeviceAuthentication => "Device authentication",
        IdentityCustodyAuthentication::HostKeyMaterial => "Host key material",
        IdentityCustodyAuthentication::None => "None",
    }
}

const fn custody_availability_label(availability: IdentityCustodyAvailability) -> &'static str {
    match availability {
        IdentityCustodyAvailability::Available => "Available",
        IdentityCustodyAvailability::Unavailable => "Unavailable",
    }
}

const fn custody_downgrade_label(downgrade: IdentityCustodyDowngrade) -> &'static str {
    match downgrade {
        IdentityCustodyDowngrade::None => "None",
        IdentityCustodyDowngrade::ActiveBackendMismatch => "Active backend mismatch",
    }
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
        BleControlDisabledReason::AlreadyConnected => {
            "The approved Bluetooth RNode is already connected"
        }
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
    let connected = state.phase == BleControlPhase::Connected;
    let scan_label = if state.permission == AuthorizationState::NotDetermined {
        "Allow Bluetooth and scan"
    } else if state.phase == BleControlPhase::Scanning {
        "Scanning for RNodes"
    } else {
        "Scan for RNodes"
    };
    let forget_label = if matches!(
        state.phase,
        BleControlPhase::Connecting | BleControlPhase::Connected | BleControlPhase::Reconnecting
    ) {
        "Disconnect and forget RNode"
    } else {
        "Forget RNode"
    };
    let status = if !actions_enabled {
        "Fixture data. Bluetooth actions are disabled."
    } else if state.phase == BleControlPhase::Scanning {
        "Scanning for compatible RNodes. This takes up to 10 seconds."
    } else if state.phase == BleControlPhase::Connecting {
        "Connecting to the approved RNode."
    } else if state.phase == BleControlPhase::Connected {
        "The approved RNode is connected. Disconnect and forget it before choosing another."
    } else if state.phase == BleControlPhase::Reconnecting {
        "Reconnecting to the approved RNode."
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
            class: "surface-card settings-card bluetooth-card",
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
                span {
                    id: "mobile.bluetooth-phase",
                    class: "state-chip",
                    "data-tone": StatusTone::for_ble(state.phase).as_str(),
                    "aria-label": format!("Bluetooth RNode {}", ble_phase(state.phase)),
                    {ble_phase(state.phase)}
                }
            }
            if !connected {
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
            }
            if state.phase == BleControlPhase::Scanning {
                div {
                    class: "scan-progress",
                    role: "progressbar",
                    "aria-label": "Bluetooth RNode scan progress",
                    "aria-valuetext": "Scanning for compatible RNodes",
                    span {}
                }
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
                        {forget_label}
                    }
                }
            }
            if !connected
                && state.candidates.is_empty()
                && state.approved.is_none()
                && state.permission == AuthorizationState::Granted
                && state.adapter == BleAdapterState::Ready
                && state.phase == BleControlPhase::Idle
            {
                p { class: "field-hint", "No scan results." }
            } else if !connected && !state.candidates.is_empty() {
                ul { class: "peripheral-list", "aria-label": "Discovered Bluetooth RNodes",
                    for candidate in state.candidates.clone() {
                        {
                            let candidate_id = candidate.id.clone();
                            let display_name = candidate
                                .display_name
                                .clone()
                                .unwrap_or_else(|| "Unnamed RNode".into());
                            let already_approved = state
                                .approved
                                .as_ref()
                                .is_some_and(|approved| approved.id == candidate.id);
                            let action_name = if already_approved {
                                format!("{display_name} is already approved")
                            } else {
                                format!("Approve and connect {display_name}")
                            };
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
                                        id: format!(
                                            "mobile.bluetooth-candidate.{}",
                                            candidate.id.as_str()
                                        ),
                                        r#type: "button",
                                        class: "secondary-action",
                                        "data-disposition": if already_approved { "approved" } else { "available" },
                                        disabled: selection_disabled || already_approved,
                                        "aria-label": action_name,
                                        "aria-describedby": if selection_reason.is_some() { "mobile.bluetooth-selection-disabled" } else { "mobile.bluetooth-status" },
                                        onclick: move |_| {
                                            if let Some(handler) = select {
                                                handler.call(candidate_id.clone());
                                            }
                                        },
                                        if already_approved { "Approved" } else { "Use RNode" }
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
                if let Some(code) = &state.diagnostic_code {
                    p { class: "technical-value", "Diagnostic: {code}" }
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
    #[props(default)] android_usb_probe_ready: bool,
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
        document::Title { "Styrene {active_destination.label()}" }
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
                    "data-tone": StatusTone::for_session(fixture.session.phase).as_str(),
                    "aria-label": format!("Session {}", fixture.session.phase.as_str()),
                    {format!("Session {}", fixture.session.phase.as_str())}
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
                    if fixture.session.phase == SessionPhase::Failed {
                        p { "Session unavailable. Open Network to review connection settings." }
                    } else {
                        p { "The last operation failed. Current session state is unchanged." }
                    }
                    p { class: "technical-value", "Diagnostic: {failure.code}" }
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
                    {
                        let has_conversation = fixture.conversations.iter().any(|conversation| {
                            conversation.peer_hash == peer.destination_hash
                        });
                        let display_name = peer.display_name.clone().unwrap_or_else(|| {
                            format!("Peer {}", short_hash(&peer.destination_hash))
                        });
                        if has_conversation {
                            let peer_hash = peer.destination_hash.clone();
                            rsx! {
                                button {
                                    id: format!("mobile.peer.{}", peer.destination_hash),
                                    class: "peer-card",
                                    r#type: "button",
                                    "data-aspect": peer.aspect.clone(),
                                    "data-source": "canonical_announce",
                                    "data-action": "open-conversation",
                                    onclick: move |_| {
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
                                    },
                                    span { class: "hash-glyph", {hash_glyph(&peer.destination_hash)} }
                                    span {
                                        class: "directory-copy",
                                        strong { {display_name} }
                                        span { class: "technical-value", {short_hash(&peer.destination_hash)} }
                                    }
                                    span { class: "row-action", "Open conversation" }
                                }
                            }
                        } else {
                            rsx! {
                                article {
                                    id: format!("mobile.peer.{}", peer.destination_hash),
                                    class: "peer-card",
                                    "data-aspect": peer.aspect.clone(),
                                    "data-source": "canonical_announce",
                                    "data-action": "unavailable",
                                    span { class: "hash-glyph", {hash_glyph(&peer.destination_hash)} }
                                    span {
                                        class: "directory-copy",
                                        strong { {display_name} }
                                        span { class: "technical-value", {short_hash(&peer.destination_hash)} }
                                    }
                                    span { class: "row-action", "No conversation yet" }
                                }
                            }
                        }
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
                        class: "surface-card settings-card",
                        "aria-labelledby": "mobile.android-usb-heading",
                        div {
                            h3 { id: "mobile.android-usb-heading", "Android USB fallback" }
                            p { class: "field-hint", "Explicitly choose an attached USB device. Bluetooth remains preferred." }
                        }
                        button {
                            r#type: "button",
                            class: "secondary-action",
                            disabled: !live_actions_enabled
                                || android_usb_busy
                                || android_usb_refresh.is_none(),
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
                                class: "surface-card bearer-card",
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
                                    disabled: !live_actions_enabled
                                        || android_usb_busy
                                        || android_usb_select.is_none(),
                                    "aria-label": format!("Use USB device {}", attachment.device_id),
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
                                disabled: !live_actions_enabled
                                    || android_usb_busy
                                    || !android_usb_connected
                                    || !android_usb_probe_ready,
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
                        class: "surface-card bearer-card",
                        "data-state": bearer.state.to_string(),
                        "data-reason": bearer.reason.clone().unwrap_or_default(),
                        div {
                            h3 { {bearer_label(bearer.kind)} }
                            if let Some(reason) = &bearer.reason {
                                p { class: "field-hint", {bearer_reason_label(reason)} }
                            }
                        }
                        span {
                            id: format!("mobile.bearer.{}.state", bearer.kind.as_str()),
                            class: "state-chip",
                            "data-tone": StatusTone::for_bearer(bearer.state).as_str(),
                            "aria-label": format!(
                                "{} bearer {}",
                                bearer_label(bearer.kind),
                                bearer.state
                            ),
                            {bearer.state.to_string()}
                        }
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
                    class: "surface-card settings-card identity-card",
                    h3 { "Node identity" }
                    p {
                        id: "mobile.identity",
                        class: "identity",
                        "aria-label": format!("Local identity {}", fixture.session.identity_hash),
                        {fixture.session.identity_hash.clone()}
                    }
                    if let Some(custody) = &fixture.session.custody {
                        section {
                            id: "mobile.identity-custody",
                            class: "identity-custody",
                            "aria-labelledby": "mobile.identity-custody-heading",
                            "data-availability": custody.availability.as_str(),
                            "data-downgrade": custody.downgrade.as_str(),
                            h4 { id: "mobile.identity-custody-heading", "Identity custody" }
                            dl {
                                dt { "Requested storage" }
                                dd {
                                    id: "mobile.identity-custody-requested",
                                    "data-backend": custody.requested_backend.as_str(),
                                    {custody_backend_label(custody.requested_backend)}
                                }
                                dt { "Active storage" }
                                dd {
                                    id: "mobile.identity-custody-active",
                                    "data-backend": custody.active_backend.map(IdentityCustodyBackend::as_str),
                                    {custody.active_backend.map_or("Unavailable", custody_backend_label)}
                                }
                                dt { "Protection" }
                                dd {
                                    id: "mobile.identity-custody-protection",
                                    "data-protection": custody.protection.map(IdentityCustodyProtection::as_str),
                                    {custody.protection.map_or("Unavailable", custody_protection_label)}
                                }
                                dt { "Authentication" }
                                dd {
                                    id: "mobile.identity-custody-authentication",
                                    "data-authentication": custody.authentication.as_str(),
                                    {custody_authentication_label(custody.authentication)}
                                }
                                dt { "Availability" }
                                dd {
                                    id: "mobile.identity-custody-availability",
                                    {custody_availability_label(custody.availability)}
                                }
                                dt { "Downgrade" }
                                dd {
                                    id: "mobile.identity-custody-downgrade",
                                    {custody_downgrade_label(custody.downgrade)}
                                }
                            }
                            if let Some(failure) = &custody.failure {
                                p {
                                    id: "mobile.identity-custody-failure",
                                    role: "status",
                                    "data-code": failure.code.clone(),
                                    "data-retryable": failure.retryable.to_string(),
                                    "Identity custody is unavailable: {failure.code}"
                                }
                            }
                        }
                    }
                }
                article {
                    class: "surface-card settings-card",
                    h3 { "About this build" }
                    p { "Styrene mobile application" }
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
    let configured_endpoint = endpoint.clone();
    let mut endpoint_buffer = use_signal(|| endpoint);
    let editing_enabled = enabled && action_sink.is_some();
    let endpoint_value = endpoint_buffer.read().trim().to_owned();
    let can_apply = editing_enabled
        && !endpoint_value.is_empty()
        && endpoint_value != configured_endpoint.trim();
    rsx! {
        div {
            class: "surface-card settings-card",
            label { r#for: "mobile.tcp-endpoint", "TCP endpoint" }
            input {
                id: "mobile.tcp-endpoint",
                name: "tcp-endpoint",
                r#type: "text",
                inputmode: "url",
                "aria-describedby": "mobile.tcp-endpoint-hint",
                disabled: !editing_enabled,
                value: endpoint_buffer,
                oninput: move |event| endpoint_buffer.set(event.value()),
            }
            p {
                id: "mobile.tcp-endpoint-hint",
                class: "field-hint",
                if !editing_enabled {
                    "Endpoint editing is unavailable in this view."
                } else if endpoint_value.is_empty() {
                    "Enter a host and port, for example rns.styrene.io:4242."
                } else if endpoint_value == configured_endpoint.trim() {
                    "Enter a different host and port to change the endpoint."
                } else {
                    "Apply this host and port to reconnect the TCP session."
                }
            }
            button {
                id: "mobile.tcp-endpoint-apply",
                class: "primary-action",
                r#type: "button",
                disabled: !can_apply,
                onclick: move |_| {
                    if can_apply && let Some(action_sink) = action_sink {
                        action_sink.call(MobileAction::new(
                            generation,
                            MobileActionKind::ApplyEndpoint {
                                endpoint: endpoint_buffer.read().trim().to_owned(),
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
    let controls_enabled = actions_enabled && action_sink.is_some();
    let sync_in_progress = propagation.sync_state == SyncState::InProgress;
    let retry_allowed = propagation.failure.as_ref().is_none_or(|failure| failure.retryable);
    let sync_enabled = controls_enabled && propagation.ready && !sync_in_progress && retry_allowed;
    let selected_candidate_missing =
        propagation.selected_destination.as_ref().is_some_and(|selected| {
            !propagation.candidates.iter().any(|candidate| candidate.destination_hash == *selected)
        });
    let sync_label = match propagation.sync_state {
        SyncState::Idle => "Sync now",
        SyncState::InProgress => "Synchronizing",
        SyncState::Complete => "Sync again",
        SyncState::Failed => "Retry sync",
    };
    rsx! {
        section {
            id: "mobile.propagation",
            class: "surface-card product-section",
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
                disabled: !controls_enabled,
                onchange: move |event| {
                    if controls_enabled && let Some(action_sink) = action_sink {
                        let destination_hash = (!event.value().is_empty()).then(|| event.value());
                        action_sink.call(MobileAction::new(
                            propagation.generation,
                            MobileActionKind::SelectPropagationNode { destination_hash },
                        ));
                    }
                },
                option { value: "", "No node selected" }
                if selected_candidate_missing {
                    option {
                        value: propagation.selected_destination.clone().unwrap_or_default(),
                        selected: true,
                        disabled: true,
                        "Selected node is currently unavailable"
                    }
                }
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
                class: "primary-action",
                disabled: !sync_enabled,
                "aria-describedby": "mobile.propagation-status",
                onclick: move |_| {
                    if sync_enabled && let Some(action_sink) = action_sink {
                        action_sink.call(MobileAction::new(
                            propagation.generation,
                            MobileActionKind::SyncPropagation,
                        ));
                    }
                },
                {sync_label}
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
                        if failure.retryable {
                            "Synchronization failed. Retry is available."
                        } else {
                            "Synchronization failed."
                        }
                    }
                } else if let Some(progress) = &propagation.progress {
                    span {
                        id: "mobile.propagation-progress",
                        "data-attempt-id": progress.attempt_id.clone(),
                        "data-received-count": progress.received_count.to_string(),
                        "data-received-bytes": progress.received_bytes.to_string(),
                        "Synchronizing: {progress.received_count} messages and {progress.received_bytes} bytes received."
                    }
                } else if propagation.sync_state == SyncState::Complete {
                    span {
                        id: "mobile.propagation-result",
                        if propagation.new_messages == 1 {
                            "1 new message"
                        } else {
                            "{propagation.new_messages} new messages"
                        }
                    }
                } else if propagation.selected_destination.is_none() {
                    span { "Select an active propagation node to synchronize." }
                } else if !propagation.ready {
                    span { "The selected propagation node is not ready." }
                } else if propagation.sync_state == SyncState::Failed {
                    span { "Synchronization failed. Retry is available." }
                } else {
                    span { "Ready to synchronize." }
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
                    p { "Open People to review discovered peers and existing conversations." }
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
    let retry_enabled = actions_enabled && action_sink.is_some();
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
                    disabled: !retry_enabled,
                    "aria-label": format!("Retry message {}", message.id),
                    onclick: {
                        let message_id = message.id.clone();
                        move |_| {
                            if retry_enabled && let Some(action_sink) = action_sink {
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
    let mut delivery_methods = use_signal(HashMap::<String, DeliveryMethod>::new);
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
    let has_conversation = conversation.is_some();
    let editing_enabled = has_conversation && action_sink.is_some();
    let send_enabled = enabled && editing_enabled && !draft.trim().is_empty();
    let delivery_method = peer_hash
        .as_ref()
        .and_then(|peer_hash| delivery_methods.read().get(peer_hash).copied())
        .unwrap_or(DeliveryMethod::Direct);
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
                    if send_enabled
                        && let (Some(action_sink), Some(peer_hash)) = (action_sink, &peer_hash)
                    {
                        action_sink.call(MobileAction::new(
                            generation,
                            MobileActionKind::SendMessage {
                                peer_hash: peer_hash.clone(),
                                content: draft.clone(),
                                requested_method: delivery_method,
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
                    disabled: !editing_enabled,
                    "aria-describedby": "mobile.composer-status",
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
                    class: "primary-action",
                    r#type: "submit",
                    "data-enabled": send_enabled.to_string(),
                    disabled: !send_enabled,
                    "Send"
                }
            }
            div {
                class: "delivery-method-row",
                label { r#for: "mobile.delivery-method", "Delivery" }
                select {
                    id: "mobile.delivery-method",
                    name: "delivery-method",
                    disabled: !editing_enabled,
                    value: match delivery_method {
                        DeliveryMethod::Direct => "direct",
                        DeliveryMethod::Opportunistic => "opportunistic",
                        DeliveryMethod::Propagated => "propagated",
                        DeliveryMethod::Unknown => "unknown",
                    },
                    onchange: {
                        let peer_hash = peer_hash.clone();
                        move |event| {
                            if let Some(peer_hash) = &peer_hash {
                                let method = if event.value() == "propagated" {
                                    DeliveryMethod::Propagated
                                } else {
                                    DeliveryMethod::Direct
                                };
                                delivery_methods.write().insert(peer_hash.clone(), method);
                            }
                        }
                    },
                    option { value: "direct", "Direct" }
                    option { value: "propagated", "Propagated" }
                }
            }
            p {
                id: "mobile.composer-status",
                class: "field-hint",
                role: "status",
                if !has_conversation {
                    "Choose a conversation before writing a message."
                } else if !editing_enabled {
                    "Draft editing is unavailable in this view."
                } else if !enabled {
                    "No bearer is connected. You can continue editing this saved draft."
                } else if draft.trim().is_empty() {
                    "Write a message to enable Send."
                } else {
                    "Ready to send."
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
