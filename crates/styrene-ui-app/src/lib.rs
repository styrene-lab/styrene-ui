//! Shared Dioxus application components.

use std::collections::HashMap;

use dioxus::prelude::*;
use qrcode::{Color, QrCode};
use styrene_ui_platform::{
    AndroidUsbAttachment, Appearance, ApplicationLifecycle, AuthorizationState, BleAdapterState,
    BleControlDisabledReason, BleControlFailure, BleControlPhase, BleControlState, BlePeripheralId,
    Contrast, KeyboardGeometry, MotionPreference, PermissionKind, PlatformInsets, PlatformSnapshot,
    TextScale, WindowClass,
};
use styrene_ui_state::{
    BearerKind, BearerState, Conversation, DeliveryEvidence, DeliveryMethod,
    DestinationEntryConstraint, IdentityCustodyAuthentication, IdentityCustodyAvailability,
    IdentityCustodyBackend, IdentityCustodyDowngrade, IdentityCustodyProtection,
    LXMF_DESTINATION_INPUT_MAX_BYTES, LocalAnnounceOutcome, Message, MobileAction,
    MobileActionKind, MobileFixture, MobileStore, OperationalSummary, PEER_SEARCH_INPUT_MAX_BYTES,
    Peer, PropagationEvidence, PropagationUpdate, RuntimeBoundary, SessionPhase, SyncState,
    TargetClass, TransportEvidence, TypedFailure, bounded_destination_input,
    bounded_peer_search_input, destination_entry_constraint, peer_matches_search,
    start_conversation_action,
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
            SessionPhase::Starting
            | SessionPhase::Connecting
            | SessionPhase::Reconnecting
            | SessionPhase::Degraded => Self::Caution,
            SessionPhase::Failed => Self::Negative,
            SessionPhase::Stopped | SessionPhase::Offline => Self::Neutral,
        }
    }

    const fn for_bearer(state: BearerState) -> Self {
        match state {
            BearerState::Connected => Self::Positive,
            BearerState::Connecting | BearerState::Reconnecting | BearerState::Unverified => {
                Self::Caution
            }
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
                                        if already_approved { "Approved" } else { "Approve and connect" }
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
    #[props(default)] clipboard_candidate: Option<String>,
    #[props(default)] clipboard_failure: Option<String>,
    #[props(default)] clipboard_busy: bool,
    #[props(default)] clipboard_read: Option<EventHandler<()>>,
    #[props(default)] identity_copy_busy: bool,
    #[props(default)] identity_copy_succeeded: bool,
    #[props(default)] identity_copy_failure: Option<String>,
    #[props(default)] identity_copy: Option<EventHandler<String>>,
    #[props(default)] application_settings_busy: bool,
    #[props(default)] application_settings_failure: Option<String>,
    #[props(default)] open_application_settings: Option<EventHandler<()>>,
) -> Element {
    let boundary = RuntimeBoundary::from(fixture.profile);
    let store = MobileStore::new(fixture.clone());
    let messaging_available = store.messaging_available();
    let propagation = propagation.unwrap_or_else(|| PropagationUpdate::from_fixture(&fixture));
    let mut operational_summary = store.operational_summary();
    operational_summary.propagation_selected = propagation.selected_destination.is_some();
    operational_summary.propagation_ready = propagation.ready;
    operational_summary.propagation_sync_state = propagation.sync_state;
    let live_actions_enabled = boundary.live_network_allowed();
    let action_sink = live_actions_enabled.then_some(action_sink).flatten();
    let mut destination = use_signal(|| MobileDestination::Messages);
    let mut selected_peer = use_signal(|| None::<String>);
    let mut compact_thread_open = use_signal(|| false);
    let mut new_message_open = use_signal(|| false);
    let mut new_message_trigger = use_signal(|| None::<Event<MountedData>>);
    let mut identity_qr_open = use_signal(|| false);
    let new_message_key =
        format!("{}:{}", fixture.generation, clipboard_candidate.as_deref().unwrap_or_default());
    let public_destination = fixture.session.identity_hash.clone();
    let public_destination_available = !public_destination.is_empty();
    let identity_copy_enabled =
        public_destination_available && identity_copy.is_some() && !identity_copy_busy;
    let identity_qr_visible = public_destination_available && identity_qr_open();

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
    let camera_authorization = platform_snapshot
        .as_ref()
        .and_then(|snapshot| {
            snapshot.permissions.iter().find(|permission| permission.kind == PermissionKind::Camera)
        })
        .map_or(AuthorizationState::Unavailable, |permission| permission.state);
    let notification_authorization = platform_snapshot
        .as_ref()
        .map_or(AuthorizationState::Unavailable, |snapshot| snapshot.notification_authorization);
    let bluetooth_authorization = ble_controls.permission;
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
                    "data-runtime": fixture.session.runtime.as_str(),
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
            OperationalSummaryPanel { summary: operational_summary }
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
                            button {
                                id: "mobile.new-message",
                                class: "secondary-action",
                                r#type: "button",
                                disabled: action_sink.is_none(),
                                "aria-expanded": new_message_open().to_string(),
                                "aria-controls": "mobile.new-message-form",
                                "aria-describedby": action_sink
                                    .is_none()
                                    .then_some("mobile.new-message-disabled"),
                                onmounted: move |event| new_message_trigger.set(Some(event)),
                                onclick: move |_| new_message_open.set(true),
                                "New Message"
                            }
                        }
                        if action_sink.is_none() {
                            p {
                                id: "mobile.new-message-disabled",
                                class: "field-hint",
                                "New Message is unavailable in this view."
                            }
                        }
                        if new_message_open() {
                            NewMessageForm {
                                key: "{new_message_key}",
                                peers: fixture.peers.clone(),
                                generation: fixture.generation,
                                enabled: action_sink.is_some(),
                                initial_destination: clipboard_candidate.clone().unwrap_or_default(),
                                paste_busy: clipboard_busy,
                                paste_failure: clipboard_failure.clone(),
                                on_paste: clipboard_read,
                                failure: fixture.session.failure.clone(),
                                action_sink,
                                on_cancel: move |()| {
                                    new_message_open.set(false);
                                    if let Some(trigger) = new_message_trigger.read().clone() {
                                        spawn(async move {
                                            let _ = trigger.data().set_focus(true).await;
                                        });
                                    }
                                },
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
                            propagation: propagation.clone(),
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
                if !fixture.peers.is_empty() && action_sink.is_none() {
                    p {
                        id: "mobile.people-actions-disabled",
                        class: "field-hint",
                        "Starting a conversation is unavailable in this view."
                    }
                }
                if !fixture.peers.is_empty() {
                    ul {
                        class: "people-list",
                        "aria-label": "Discovered people",
                    for peer in &fixture.peers {
                    {
                        let has_conversation = fixture.conversations.iter().any(|conversation| {
                            conversation.peer_hash == peer.destination_hash
                        });
                        let display_name = peer.display_name.clone().unwrap_or_else(|| {
                            format!("Peer {}", short_hash(&peer.destination_hash))
                        });
                        let announce_label = if peer.announce_count == 1 { "announce" } else { "announces" };
                        if has_conversation {
                            let peer_hash = peer.destination_hash.clone();
                            rsx! {
                                li { button {
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
                                        span {
                                            class: "field-hint",
                                            "{peer.aspect} · Canonical announce · observed {peer.age_secs}s ago · {peer.announce_count} {announce_label}"
                                        }
                                    }
                                    span { class: "row-action", "Open conversation" }
                                } }
                            }
                        } else {
                            let peer_hash = peer.destination_hash.clone();
                            rsx! {
                                li { button {
                                    id: format!("mobile.peer.{}", peer.destination_hash),
                                    class: "peer-card",
                                    r#type: "button",
                                    "data-aspect": peer.aspect.clone(),
                                    "data-source": "canonical_announce",
                                    "data-action": "start-conversation",
                                    disabled: action_sink.is_none(),
                                    "aria-describedby": action_sink
                                        .is_none()
                                        .then_some("mobile.people-actions-disabled"),
                                    onclick: move |_| {
                                        selected_peer.set(Some(peer_hash.clone()));
                                        if let Some(action_sink) = action_sink {
                                            action_sink.call(MobileAction::new(
                                                fixture.generation,
                                                MobileActionKind::StartConversation {
                                                    peer_hash: peer_hash.clone(),
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
                                        span {
                                            class: "field-hint",
                                            "{peer.aspect} · Canonical announce · observed {peer.age_secs}s ago · {peer.announce_count} {announce_label}"
                                        }
                                    }
                                    span { class: "row-action", "Start conversation" }
                                } }
                            }
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
                    propagation,
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
                    IdentityDisplayNameEditor {
                        key: fixture.session.display_name.clone(),
                        display_name: fixture.session.display_name.clone(),
                        generation: fixture.generation,
                        enabled: live_actions_enabled,
                        failure: fixture.session.failure.clone(),
                        action_sink,
                    }
                    dl {
                        dt { "Public LXMF destination" }
                        dd {
                            id: "mobile.identity",
                            class: "identity",
                            "aria-label": format!("Public LXMF destination {}", fixture.session.identity_hash),
                            {fixture.session.identity_hash.clone()}
                        }
                    }
                    div {
                        class: "identity-actions",
                        button {
                            id: "mobile.identity-copy",
                            class: "secondary-action",
                            r#type: "button",
                            disabled: !identity_copy_enabled,
                            "aria-describedby": "mobile.identity-copy-status",
                            onclick: {
                                let public_destination = public_destination.clone();
                                move |_| {
                                    if let Some(handler) = identity_copy {
                                        handler.call(public_destination.clone());
                                    }
                                }
                            },
                            if identity_copy_busy { "Copying" } else { "Copy" }
                        }
                        button {
                            id: "mobile.identity-show-qr",
                            class: "secondary-action",
                            r#type: "button",
                            "aria-expanded": identity_qr_visible.to_string(),
                            "aria-controls": "mobile.identity-qr",
                            disabled: !public_destination_available,
                            onclick: move |_| {
                                if public_destination_available {
                                    identity_qr_open.toggle();
                                }
                            },
                            if identity_qr_visible { "Hide QR" } else { "Show QR" }
                        }
                    }
                    p {
                        id: "mobile.identity-copy-status",
                        class: if identity_copy_failure.is_some() { "field-error" } else { "field-hint" },
                        role: "status",
                        if let Some(failure) = &identity_copy_failure {
                            "Public destination was not copied ({failure})."
                        } else if identity_copy_succeeded {
                            "Public destination copied."
                        } else if !public_destination_available {
                            "Public destination is not available yet."
                        } else {
                            "Copy shares only the public LXMF destination."
                        }
                    }
                    if identity_qr_visible {
                        IdentityQrCode { value: public_destination.clone() }
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
                    id: "mobile.permissions",
                    class: "surface-card settings-card",
                    "aria-labelledby": "mobile.permissions-heading",
                    h3 { id: "mobile.permissions-heading", "Permissions and device access" }
                    dl {
                        dt { "Camera" }
                        dd {
                            id: "mobile.permission.camera",
                            "data-state": authorization_state(camera_authorization),
                            {authorization_state(camera_authorization)}
                        }
                        dt { "Bluetooth" }
                        dd {
                            id: "mobile.permission.bluetooth",
                            "data-state": authorization_state(bluetooth_authorization),
                            {authorization_state(bluetooth_authorization)}
                        }
                        dt { "Notifications" }
                        dd {
                            id: "mobile.permission.notifications",
                            "data-state": authorization_state(notification_authorization),
                            {authorization_state(notification_authorization)}
                        }
                        dt { "Secure identity storage" }
                        dd {
                            id: "mobile.permission.secure-storage",
                            "data-state": fixture.session.custody.as_ref().map_or("unavailable", |custody| custody.availability.as_str()),
                            {fixture.session.custody.as_ref().map_or("unavailable", |custody| custody.availability.as_str())}
                        }
                        dt { "Location" }
                        dd {
                            id: "mobile.permission.location",
                            "data-state": "not-requested",
                            "Not requested by Styrene"
                        }
                    }
                    if [camera_authorization, bluetooth_authorization, notification_authorization]
                        .iter()
                        .any(|state| matches!(state, AuthorizationState::Denied | AuthorizationState::Restricted))
                    {
                        p {
                            id: "mobile.permissions-recovery",
                            class: "field-hint",
                            "Denied or restricted access can be reviewed in this app's system Settings."
                        }
                        button {
                            id: "mobile.open-application-settings",
                            class: "secondary-action",
                            r#type: "button",
                            disabled: application_settings_busy || open_application_settings.is_none(),
                            "aria-describedby": "mobile.permissions-recovery",
                            onclick: move |_| {
                                if let Some(open_application_settings) = open_application_settings {
                                    open_application_settings.call(());
                                }
                            },
                            if application_settings_busy { "Opening Settings" } else { "Open system Settings" }
                        }
                        if let Some(failure) = &application_settings_failure {
                            p {
                                id: "mobile.application-settings-failure",
                                class: "field-error",
                                role: "status",
                                "System Settings could not be opened ({failure})."
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
pub fn IdentityQrCode(value: String) -> Element {
    let Ok(code) = QrCode::new(value.as_bytes()) else {
        return rsx! {
            p { id: "mobile.identity-qr", class: "field-error", role: "status", "QR unavailable." }
        };
    };
    let width = code.width();
    let view_size = width + 8;
    let dark_modules = (0..width)
        .flat_map(|y| (0..width).map(move |x| (x, y)))
        .filter(|&(x, y)| code[(x, y)] == Color::Dark)
        .collect::<Vec<_>>();

    rsx! {
        figure {
            id: "mobile.identity-qr",
            class: "identity-qr",
            "data-payload": value.clone(),
            svg {
                role: "img",
                "aria-label": format!("QR code for public LXMF destination {value}"),
                view_box: format!("-4 -4 {view_size} {view_size}"),
                rect { x: "-4", y: "-4", width: view_size, height: view_size, fill: "white" }
                for (x, y) in dark_modules {
                    rect { key: "{x}-{y}", x, y, width: "1", height: "1", fill: "black" }
                }
            }
            figcaption { "Public LXMF destination" }
        }
    }
}

#[component]
fn OperationalSummaryPanel(summary: OperationalSummary) -> Element {
    let loaded_routes = summary.loaded_route_observed.saturating_add(summary.loaded_route_unknown);
    rsx! {
        details {
            id: "mobile.operational-summary",
            class: "surface-card operational-summary",
            summary { "Operational summary" }
            dl {
                dt { "Runtime" }
                dd {
                    id: "mobile.summary.runtime",
                    "data-runtime": summary.runtime.as_str(),
                    "data-phase": summary.phase.as_str(),
                    "{summary.runtime.as_str()} · {summary.phase.as_str()}"
                }
                dt { "Bearers" }
                dd {
                    id: "mobile.summary.bearers",
                    "{summary.connected_bearers} of {summary.bearer_count} connected"
                }
                dt { "Peers" }
                dd { id: "mobile.summary.peers", "{summary.peer_count} canonical observations" }
                dt { "Unread" }
                dd { id: "mobile.summary.unread", "{summary.unread_count}" }
                dt { "Loaded route evidence" }
                dd {
                    id: "mobile.summary.routes",
                    if loaded_routes == 0 {
                        "Unknown; no loaded attempt evidence"
                    } else {
                        "{summary.loaded_route_observed} observed · {summary.loaded_route_unknown} unknown"
                    }
                }
                dt { "Propagation" }
                dd {
                    id: "mobile.summary.propagation",
                    "data-selected": summary.propagation_selected.to_string(),
                    "data-ready": summary.propagation_ready.to_string(),
                    if !summary.propagation_selected {
                        "No node selected"
                    } else if summary.propagation_ready {
                        "Selected node ready · {summary.propagation_sync_state.as_str()}"
                    } else {
                        "Selected node not ready · {summary.propagation_sync_state.as_str()}"
                    }
                }
            }
            p {
                class: "field-hint",
                "Route counts describe loaded attempts only. No relay, path, mail, or reachability state is inferred."
            }
        }
    }
}

const IDENTITY_DISPLAY_NAME_MAX_CHARS: usize = 64;

#[component]
fn IdentityDisplayNameEditor(
    display_name: String,
    generation: u64,
    enabled: bool,
    failure: Option<TypedFailure>,
    action_sink: Option<EventHandler<MobileAction>>,
) -> Element {
    let configured_name = display_name.clone();
    let mut name = use_signal(|| display_name);
    let value = name.read().trim().to_owned();
    let character_count = value.chars().count();
    let editing_enabled = enabled && action_sink.is_some();
    let has_name_failure =
        failure.as_ref().is_some_and(|failure| failure.code == "identity_display_name_failed");
    let has_name_error = has_name_failure || character_count > IDENTITY_DISPLAY_NAME_MAX_CHARS;
    let can_save = editing_enabled
        && !value.is_empty()
        && character_count <= IDENTITY_DISPLAY_NAME_MAX_CHARS
        && value != configured_name.trim();

    rsx! {
        form {
            class: "identity-name-form",
            onsubmit: move |event| {
                event.prevent_default();
                if can_save && let Some(action_sink) = action_sink {
                    action_sink.call(MobileAction::new(
                        generation,
                        MobileActionKind::SetIdentityDisplayName {
                            display_name: name.read().trim().to_owned(),
                        },
                    ));
                }
            },
            label { r#for: "mobile.identity-display-name", "Display name" }
            input {
                id: "mobile.identity-display-name",
                name: "identity-display-name",
                r#type: "text",
                maxlength: (IDENTITY_DISPLAY_NAME_MAX_CHARS + 1).to_string(),
                disabled: !editing_enabled,
                "aria-invalid": has_name_error.to_string(),
                "aria-describedby": "mobile.identity-display-name-status",
                "aria-errormessage": has_name_error.then_some("mobile.identity-display-name-status"),
                value: name,
                oninput: move |event| {
                    name.set(event.value().chars().take(IDENTITY_DISPLAY_NAME_MAX_CHARS + 1).collect());
                },
            }
            p {
                id: "mobile.identity-display-name-status",
                class: if has_name_error { "field-error" } else { "field-hint" },
                role: "status",
                "aria-live": "polite",
                if has_name_failure {
                    "The backend rejected the display name. Check it and try again."
                } else if !editing_enabled {
                    "Display-name editing is unavailable in this view."
                } else if value.is_empty() {
                    "Enter a display name."
                } else if character_count > IDENTITY_DISPLAY_NAME_MAX_CHARS {
                    "The display name exceeds the 64-character limit."
                } else if value == configured_name.trim() {
                    "This is the current public display name."
                } else {
                    "The backend will validate and persist this public display name."
                }
            }
            button {
                id: "mobile.identity-display-name-save",
                class: "primary-action",
                r#type: "submit",
                disabled: !can_save,
                "aria-describedby": "mobile.identity-display-name-status",
                "Save display name"
            }
        }
    }
}

#[component]
pub fn NewMessageForm(
    peers: Vec<Peer>,
    generation: u64,
    enabled: bool,
    #[props(default)] initial_search: String,
    #[props(default)] initial_destination: String,
    #[props(default)] failure: Option<TypedFailure>,
    #[props(default)] paste_busy: bool,
    #[props(default)] paste_failure: Option<String>,
    #[props(default)] on_paste: Option<EventHandler<()>>,
    #[props(default)] action_sink: Option<EventHandler<MobileAction>>,
    #[props(default)] on_cancel: Option<EventHandler<()>>,
) -> Element {
    let mut search = use_signal(|| bounded_peer_search_input(&initial_search));
    let mut destination = use_signal(|| bounded_destination_input(&initial_destination));
    let search_value = search.read().clone();
    let destination_value = destination.read().clone();
    let constraint = destination_entry_constraint(&destination_value);
    let controls_enabled = enabled && action_sink.is_some();
    let can_submit = controls_enabled && constraint.permits_submission();
    let matching_peers = peers
        .iter()
        .filter(|peer| peer_matches_search(peer, &search_value))
        .cloned()
        .collect::<Vec<_>>();
    let submitted_destination = destination_value.clone();
    let destination_error_id = "mobile.direct-destination-error";
    let destination_status_id = "mobile.direct-destination-status";
    let destination_failure_id = "mobile.new-message-failure";
    let has_validation_error = matches!(constraint, DestinationEntryConstraint::Oversized);
    let has_start_failure =
        failure.as_ref().is_some_and(|failure| failure.code == "conversation_start_failed");
    let destination_described_by = if has_validation_error && has_start_failure {
        format!("{destination_status_id} {destination_error_id} {destination_failure_id}")
    } else if has_validation_error {
        format!("{destination_status_id} {destination_error_id}")
    } else if has_start_failure {
        format!("{destination_status_id} {destination_failure_id}")
    } else {
        destination_status_id.to_owned()
    };
    let validation_message = match constraint {
        _ if !controls_enabled => "Starting a conversation is unavailable in this view.".into(),
        DestinationEntryConstraint::Empty => {
            format!("Enter a {LXMF_DESTINATION_INPUT_MAX_BYTES}-character LXMF destination.")
        }
        DestinationEntryConstraint::Incomplete => format!(
            "Destination must contain {LXMF_DESTINATION_INPUT_MAX_BYTES} characters before backend validation."
        ),
        DestinationEntryConstraint::Ready => {
            "The backend will validate this destination before creating a conversation.".into()
        }
        DestinationEntryConstraint::Oversized => {
            format!("Destination exceeds the {LXMF_DESTINATION_INPUT_MAX_BYTES}-byte input limit.")
        }
    };

    rsx! {
        section {
            id: "mobile.new-message-form",
            class: "new-message surface-card",
            "aria-labelledby": "mobile.new-message-heading",
            h3 { id: "mobile.new-message-heading", "New Message" }
            label { r#for: "mobile.peer-search", "Search discovered peers" }
            input {
                id: "mobile.peer-search",
                name: "peer-search",
                r#type: "search",
                maxlength: PEER_SEARCH_INPUT_MAX_BYTES.to_string(),
                disabled: !controls_enabled,
                autofocus: !has_validation_error && !has_start_failure,
                "aria-describedby": (!controls_enabled).then_some(destination_status_id),
                value: search_value,
                oninput: move |event| search.set(bounded_peer_search_input(&event.value())),
            }
            if matching_peers.is_empty() {
                p { id: "mobile.peer-search-empty", class: "field-hint", "No discovered peers match this search." }
            } else {
                ul { class: "new-message-peer-list", "aria-label": "Matching discovered peers",
                    for peer in matching_peers {
                        {
                            let peer_hash = peer.destination_hash.clone();
                            let display_name = peer.display_name.clone().unwrap_or_else(|| {
                                format!("Peer {}", short_hash(&peer.destination_hash))
                            });
                            rsx! {
                                li {
                                    button {
                                        id: format!("mobile.new-message-peer.{}", peer.destination_hash),
                                        class: "peer-card",
                                        r#type: "button",
                                        disabled: !controls_enabled,
                                        "aria-describedby": (!controls_enabled).then_some(destination_status_id),
                                        onclick: move |_| {
                                            if let Some(action_sink) = action_sink
                                                && let Some(action) = start_conversation_action(generation, &peer_hash)
                                            {
                                                action_sink.call(action);
                                            }
                                        },
                                        span { class: "hash-glyph", {hash_glyph(&peer.destination_hash)} }
                                        span { class: "directory-copy",
                                            strong { {display_name} }
                                            span { class: "technical-value", {short_hash(&peer.destination_hash)} }
                                        }
                                        span { class: "row-action", "Choose" }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            form {
                class: "direct-destination-form",
                onsubmit: move |event| {
                    event.prevent_default();
                    if can_submit
                        && let Some(action_sink) = action_sink
                        && let Some(action) = start_conversation_action(generation, &submitted_destination)
                    {
                        action_sink.call(action);
                    }
                },
                label { r#for: "mobile.direct-destination", "LXMF destination" }
                button {
                    id: "mobile.paste-destination",
                    class: "secondary-action",
                    r#type: "button",
                    disabled: !controls_enabled || paste_busy || on_paste.is_none(),
                    "aria-describedby": "mobile.paste-destination-status",
                    onclick: move |_| {
                        if let Some(on_paste) = on_paste {
                            on_paste.call(());
                        }
                    },
                    if paste_busy { "Reading clipboard" } else { "Paste destination" }
                }
                p {
                    id: "mobile.paste-destination-status",
                    class: if paste_failure.is_some() { "field-error" } else { "field-hint" },
                    role: "status",
                    "aria-live": "polite",
                    if let Some(failure) = &paste_failure {
                        "Clipboard text was not added ({failure})."
                    } else if !controls_enabled {
                        "Clipboard access is unavailable in this view."
                    } else {
                        "Clipboard text is treated as an unvalidated destination candidate."
                    }
                }
                input {
                    id: "mobile.direct-destination",
                    name: "direct-destination",
                    r#type: "text",
                    inputmode: "text",
                    autocomplete: "off",
                    autocapitalize: "none",
                    spellcheck: "false",
                    maxlength: LXMF_DESTINATION_INPUT_MAX_BYTES.to_string(),
                    disabled: !controls_enabled,
                    autofocus: has_validation_error || has_start_failure,
                    "aria-invalid": (has_validation_error || has_start_failure).to_string(),
                    "aria-describedby": destination_described_by.clone(),
                    "aria-errormessage": if has_validation_error {
                        Some(destination_error_id)
                    } else if has_start_failure {
                        Some(destination_failure_id)
                    } else {
                        None
                    },
                    value: destination_value,
                    oninput: move |event| {
                        destination.set(bounded_destination_input(&event.value()));
                    },
                }
                p {
                    id: destination_status_id,
                    class: "field-hint",
                    role: "status",
                    "aria-live": "polite",
                    {validation_message}
                }
                if has_validation_error {
                    p {
                        id: destination_error_id,
                        class: "field-error",
                        role: "alert",
                        {format!(
                            "Destination exceeds the {LXMF_DESTINATION_INPUT_MAX_BYTES}-byte input limit."
                        )}
                    }
                }
                if has_start_failure {
                    p {
                        id: destination_failure_id,
                        class: "field-error",
                        role: "alert",
                        "The backend rejected the destination. Check it and try again."
                    }
                }
                div { class: "new-message-actions",
                    if let Some(on_cancel) = on_cancel {
                        button {
                            class: "secondary-action",
                            r#type: "button",
                            onclick: move |_| on_cancel.call(()),
                            "Cancel"
                        }
                    }
                    button {
                        id: "mobile.start-conversation",
                        class: "primary-action",
                        r#type: "submit",
                        disabled: !can_submit,
                        "aria-describedby": destination_described_by,
                        "Start conversation"
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
    let trigger_capabilities = propagation
        .trigger_capabilities
        .iter()
        .map(|trigger| trigger.as_str())
        .collect::<Vec<_>>()
        .join(", ");
    let readiness_reason = match propagation.readiness {
        styrene_ui_state::PropagationReadiness::Unselected => "No propagation node is selected.",
        styrene_ui_state::PropagationReadiness::Ready => "The selected node is ready.",
        styrene_ui_state::PropagationReadiness::Unavailable => "The selected node is unavailable.",
        styrene_ui_state::PropagationReadiness::Inactive => {
            "The selected node announcement is inactive."
        }
        styrene_ui_state::PropagationReadiness::InvalidMetadata => {
            "The selected node metadata is invalid."
        }
    };
    let sync_disabled_reason = if !controls_enabled {
        Some("Propagation actions are unavailable in this view.")
    } else if propagation.selected_destination.is_none() {
        Some("Select an active propagation node to synchronize.")
    } else if !propagation.ready {
        Some("The selected propagation node is not ready.")
    } else if sync_in_progress {
        Some("Synchronization is already in progress.")
    } else if !retry_allowed {
        Some("The last synchronization failure cannot be retried from this action.")
    } else {
        None
    };
    rsx! {
        section {
            id: "mobile.propagation",
            class: "surface-card product-section",
            "aria-labelledby": "mobile.propagation-heading",
            "data-ready": propagation.ready.to_string(),
            "data-readiness": propagation.readiness.as_str(),
            "data-sync-state": propagation.sync_state.as_str(),
            h2 { id: "mobile.propagation-heading", "Propagation" }
            p {
                id: "mobile.propagation-selected",
                strong { "Selected propagation node: " }
                {selected}
            }
            label {
                r#for: "mobile.propagation-node",
                "Propagation node"
            }
            select {
                id: "mobile.propagation-node",
                disabled: !controls_enabled,
                "aria-describedby": (!controls_enabled)
                    .then_some("mobile.propagation-node-disabled"),
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
            if !controls_enabled {
                p {
                    id: "mobile.propagation-node-disabled",
                    class: "field-hint",
                    "Propagation-node selection is unavailable in this view."
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
                    "Automatic synchronization is enabled for connection, reconnection, and allowed foreground opportunities. Background collection is best effort and follows system scheduling."
                } else {
                    "Automatic synchronization is disabled. Use manual synchronization when a selected node is ready."
                }
            }
            p {
                id: "mobile.propagation-readiness",
                "data-readiness": propagation.readiness.as_str(),
                {readiness_reason}
            }
            p {
                id: "mobile.propagation-trigger-capabilities",
                class: "field-hint",
                if trigger_capabilities.is_empty() {
                    "Automatic trigger capabilities are unavailable."
                } else {
                    "Available trigger sources: {trigger_capabilities}. Background opportunity means explicitly granted, not guaranteed."
                }
            }
            if let Some(trigger) = propagation.active_trigger {
                p {
                    id: "mobile.propagation-active-trigger",
                    "data-trigger": trigger.as_str(),
                    "data-started-at": propagation.active_sync_started_at.map(|value| value.to_string()),
                    "Active synchronization trigger: {trigger.as_str()}"
                }
            }
            if let Some(last) = &propagation.last_synchronization {
                p {
                    id: "mobile.propagation-last-sync",
                    "data-trigger": last.trigger.as_str(),
                    "data-outcome": last.outcome.as_str(),
                    "data-started-at": last.started_at.to_string(),
                    "data-finished-at": last.finished_at.to_string(),
                    "Last synchronization: {last.trigger.as_str()} · {last.outcome.as_str()} · {last.new_messages} new messages"
                }
            } else {
                p {
                    id: "mobile.propagation-last-sync",
                    "data-trigger": "unknown",
                    "No completed synchronization evidence."
                }
            }
            p {
                id: "mobile.propagation-cooldown",
                class: "field-hint",
                "data-remaining-secs": propagation.cooldown_remaining_secs.to_string(),
                "Cooldown remaining: {propagation.cooldown_remaining_secs} seconds"
            }
            p {
                id: "mobile.propagation-airtime-policy",
                class: "field-hint",
                "Manual synchronization contacts the selected propagation node and may consume network airtime."
            }
            button {
                id: "mobile.propagation-sync",
                class: "primary-action",
                disabled: !sync_enabled,
                autofocus: propagation.sync_state == SyncState::Failed,
                "aria-describedby": if sync_disabled_reason.is_some() {
                    "mobile.propagation-status mobile.propagation-airtime-policy mobile.propagation-sync-disabled"
                } else {
                    "mobile.propagation-status mobile.propagation-airtime-policy"
                },
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
            if let Some(reason) = sync_disabled_reason {
                p {
                    id: "mobile.propagation-sync-disabled",
                    class: "field-hint",
                    {reason}
                }
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
            if !messages.is_empty() {
                ol {
                    class: "message-list",
                for message in messages {
                    {
                        let direction = if message.details.source_hash.is_empty()
                            && message.details.destination_hash.is_empty()
                        {
                            "unknown"
                        } else if message.details.is_outgoing {
                            "outgoing"
                        } else {
                            "incoming"
                        };
                        let direction_label = match direction {
                            "outgoing" => "Sent",
                            "incoming" => "Received",
                            _ => "Message",
                        };
                        rsx! {
                    li {
                        article {
                            id: format!("mobile.message.{}", message.id),
                            class: "message-card",
                            "data-direction": direction,
                            "data-timestamp": message.details.timestamp.to_string(),
                            "aria-labelledby": format!("mobile.message-heading.{}", message.id),
                            header {
                                class: "message-context",
                                h4 {
                                    id: format!("mobile.message-heading.{}", message.id),
                                    {direction_label}
                                }
                                if message.details.timestamp != 0 {
                                    time {
                                        class: "technical-value",
                                        "data-unix-seconds": message.details.timestamp.to_string(),
                                        "Unix {message.details.timestamp}"
                                    }
                                }
                            }
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
    let retry_eligible = message
        .details
        .retry_eligible
        .or_else(|| message.failure.as_ref().map(|failure| failure.retryable));
    let retry_enabled = actions_enabled && action_sink.is_some() && retry_eligible == Some(true);
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
            if let Some(method) = &message.details.requested_delivery_method {
                p {
                    class: "field-hint",
                    "Requested method: {method}"
                    if let Some(actual) = &message.details.actual_delivery_method {
                        if actual != method {
                            span { " · Actual method: {actual}" }
                        }
                    }
                }
            }
            if let Some(reason) = &message.details.fallback_reason {
                p { class: "field-hint", "Fallback: {reason}" }
            }
            if let Some(detail) = &message.details.terminal_detail {
                p {
                    id: format!("mobile.message-terminal.{}", message.id),
                    class: "field-error",
                    "Terminal outcome: {detail}"
                }
            }
            if !message.details.attempts.is_empty() {
                details {
                    class: "message-evidence",
                    summary { "Attempt and route evidence" }
                    ol {
                        for attempt in &message.details.attempts {
                            li {
                                "Attempt {attempt.number}: {attempt.state}"
                                if let Some(bearer) = &attempt.bearer {
                                    span { " · Bearer: {bearer}" }
                                }
                                if attempt.route.outcome == styrene_ui_state::MessageRouteOutcome::Observed {
                                    span {
                                        " · Route observed"
                                        if let Some(hops) = attempt.route.hops {
                                            span { " · {hops} hops" }
                                        }
                                        if attempt.route.stale {
                                            span { " · stale" }
                                        }
                                    }
                                } else {
                                    span { " · Route unknown" }
                                }
                            }
                        }
                    }
                }
            }
            if !message.details.propagation_correlations.is_empty() {
                details {
                    class: "message-evidence",
                    summary { "Propagation evidence" }
                    ul {
                        for correlation in &message.details.propagation_correlations {
                            li {
                                "{correlation.relation}: {correlation.state}"
                                if let Some(peer) = &correlation.peer_hash {
                                    span { " · Node {short_hash(peer)}" }
                                }
                            }
                        }
                    }
                }
            }
            if !message.details.delivery_evidence.is_empty() {
                details {
                    class: "message-evidence",
                    summary { "Recipient delivery evidence" }
                    ul {
                        for evidence in &message.details.delivery_evidence {
                            li {
                                {format!("{:?}: {:?}", evidence.kind, evidence.state)}
                                if let Some(outcome) = &evidence.outcome {
                                    span { " · {outcome}" }
                                }
                                if let Some(progress) = evidence.progress {
                                    span { " · {progress}%" }
                                }
                            }
                        }
                    }
                }
            }
            if retry_eligible == Some(true) {
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
            } else if retry_eligible == Some(false) {
                p { class: "field-hint", "Retry unavailable for this terminal outcome." }
            }
        }
    }
}

#[component]
pub fn Composer(
    conversation: Option<Conversation>,
    enabled: bool,
    propagation: PropagationUpdate,
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
    let delivery_method = peer_hash
        .as_ref()
        .and_then(|peer_hash| delivery_methods.read().get(peer_hash).copied())
        .unwrap_or(DeliveryMethod::Direct);
    let propagated_ready = propagation.selected_destination.is_some() && propagation.ready;
    let propagation_unavailable_reason = if propagation.selected_destination.is_none() {
        Some("Select a propagation node in Network before using Propagated delivery.")
    } else if !propagation.ready {
        Some("The selected propagation node is not ready for Propagated delivery.")
    } else {
        None
    };
    let selected_method_ready = delivery_method != DeliveryMethod::Propagated || propagated_ready;
    let send_enabled =
        enabled && editing_enabled && selected_method_ready && !draft.trim().is_empty();
    let send_disabled_reason = if !has_conversation {
        Some("Choose a conversation before sending a message.")
    } else if !editing_enabled {
        Some("Draft editing is unavailable in this view.")
    } else if !enabled {
        Some("No bearer is connected. You can continue editing this saved draft.")
    } else if !selected_method_ready {
        propagation_unavailable_reason
    } else if draft.trim().is_empty() {
        Some("Write a message to enable Send.")
    } else {
        None
    };
    rsx! {
        form {
            key: "{draft_id}",
            id: "mobile.composer",
            class: "composer",
            "data-peer": peer_hash.clone(),
            "data-delivery-method": match delivery_method {
                DeliveryMethod::Direct => "direct",
                DeliveryMethod::Opportunistic => "opportunistic",
                DeliveryMethod::Propagated => "propagated",
                DeliveryMethod::Unknown => "unknown",
            },
            "data-selected-method-ready": selected_method_ready.to_string(),
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
                    "aria-describedby": if send_disabled_reason.is_some() {
                        "mobile.composer-status mobile.send-disabled-reason"
                    } else {
                        "mobile.composer-status"
                    },
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
                    "aria-describedby": "mobile.delivery-method-status mobile.composer-status",
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
                    option {
                        value: "propagated",
                        disabled: !propagated_ready,
                        "Propagated"
                    }
                }
            }
            if let Some(reason) = propagation_unavailable_reason {
                p {
                    id: "mobile.delivery-method-status",
                    class: "field-hint",
                    "Propagated unavailable: {reason}"
                }
            } else {
                p {
                    id: "mobile.delivery-method-status",
                    class: "field-hint",
                    "Propagated delivery is available through the selected node."
                }
            }
            div {
                class: "composer-status-stack",
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
                    } else if !selected_method_ready {
                        {propagation_unavailable_reason.unwrap_or(
                            "Propagated delivery is currently unavailable."
                        )}
                    } else if draft.trim().is_empty() {
                        "Write a message to enable Send."
                    } else {
                        "Ready to send."
                    }
                }
                if let Some(reason) = send_disabled_reason {
                    p {
                        id: "mobile.send-disabled-reason",
                        class: "field-hint",
                        {reason}
                    }
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
