use std::collections::HashSet;

use dioxus::prelude::*;

use crate::daemon_bridge::DaemonCommand;
use crate::safety::{ConfirmationToken, ControlPlane, SafetyAction, SafetyContext};
use crate::state::{InterfaceInfo, LinkInfo, MeshStatusInfo, PathEntry, PeerEntry, PeerRole};
use styrene_ipc::types::{
    NetworkOperationInfo, NetworkOperationKind, RequestObservationInfo, StartNetworkOperationInfo,
    StartRequestInfo,
};

use super::NetworkGraph;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum NetworkMode {
    Discovery,
    Routes,
    Links,
    Interfaces,
    #[default]
    Combined,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum RoleFilter {
    #[default]
    All,
    Styrene,
    Hub,
    PageHost,
    Rns,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum StatusFilter {
    #[default]
    All,
    Online,
    Offline,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum FreshnessFilter {
    #[default]
    All,
    Recent,
    Stale,
    Unknown,
}

#[derive(Clone)]
struct NetworkConfirmation {
    label: String,
    target: String,
    parameters: String,
    consequence: String,
    action: SafetyAction,
    command: DaemonCommand,
    token: ConfirmationToken,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum NetworkAvailability {
    Available,
    PartiallyAvailable,
    ReadOnly,
}

const DISPLAYED_NETWORK_OPERATIONS: [(NetworkOperationKind, &str); 5] = [
    (NetworkOperationKind::Announce, "Announce"),
    (NetworkOperationKind::PathRequest, "Request path"),
    (NetworkOperationKind::Probe, "Probe"),
    (NetworkOperationKind::LinkOpen, "Open link"),
    (NetworkOperationKind::LinkClose, "Close link"),
];

#[component]
pub fn NetworkPage(
    peers: Vec<PeerEntry>,
    paths: Vec<PathEntry>,
    status: MeshStatusInfo,
    local_hash: String,
    local_name: Option<String>,
    on_select_peer: EventHandler<String>,
    on_browse_page: EventHandler<String>,
    links: Vec<LinkInfo>,
    interfaces: Vec<InterfaceInfo>,
    operations: Vec<NetworkOperationInfo>,
    requests: Vec<RequestObservationInfo>,
    resources: Vec<styrene_ipc::types::ResourceTransferInfo>,
    safety: Memo<SafetyContext>,
    on_network_command: EventHandler<DaemonCommand>,
) -> Element {
    let safety_policy = safety;
    let mut mode = use_signal(NetworkMode::default);
    let mut query = use_signal(String::new);
    let mut capability = use_signal(String::new);
    let mut role = use_signal(RoleFilter::default);
    let mut peer_status = use_signal(StatusFilter::default);
    let mut freshness = use_signal(FreshnessFilter::default);
    let mut target = use_signal(String::new);
    let mut link_control_id = use_signal(String::new);
    let mut request_link_id = use_signal(String::new);
    let mut request_path = use_signal(|| "/status".to_string());
    let mut request_data = use_signal(String::new);
    let mut confirmation = use_signal(|| None::<NetworkConfirmation>);
    use_effect(move || {
        let _generation = safety_policy.read().generation_key();
        target.set(String::new());
        link_control_id.set(String::new());
        request_link_id.set(String::new());
        request_path.set("/status".into());
        request_data.set(String::new());
        confirmation.set(None);
    });
    let network_availability = {
        let safety = safety_policy.read();
        let mut authorized = DISPLAYED_NETWORK_OPERATIONS
            .iter()
            .map(|(kind, _)| {
                safety
                    .authorization(
                        ControlPlane::Operate,
                        &SafetyAction::operate(
                            network_operation_capability(*kind),
                            *kind == NetworkOperationKind::LinkClose,
                        ),
                    )
                    .is_ok()
            })
            .collect::<Vec<_>>();
        authorized.push(
            safety
                .authorization(
                    ControlPlane::Operate,
                    &SafetyAction::operate("network.request", false),
                )
                .is_ok(),
        );
        authorized.extend(
            operations
                .iter()
                .filter(|operation| operation.cancellable && operation.outcome.is_none())
                .map(|_| {
                    safety
                        .authorization(ControlPlane::Operate, &SafetyAction::operate_session(true))
                        .is_ok()
                }),
        );
        authorized.extend(requests.iter().filter(|request| !request.state.is_terminal()).map(
            |_| {
                safety
                    .authorization(
                        ControlPlane::Operate,
                        &SafetyAction::operate("network.request_cancel", true),
                    )
                    .is_ok()
            },
        ));
        authorized.extend(
            resources
                .iter()
                .filter(|resource| resource.cancellable && !resource.state.is_terminal())
                .map(|_| {
                    safety
                        .authorization(
                            ControlPlane::Operate,
                            &SafetyAction::operate("network.resource_cancel", true),
                        )
                        .is_ok()
                }),
        );
        aggregate_availability(authorized)
    };

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or(0);
    let query_value = query.read().trim().to_ascii_lowercase();
    let capability_value = capability.read().trim().to_ascii_lowercase();
    let filtered_peers: Vec<PeerEntry> = peers
        .iter()
        .filter(|peer| {
            (query_value.is_empty()
                || peer.hash.to_ascii_lowercase().contains(&query_value)
                || peer
                    .name
                    .as_deref()
                    .unwrap_or_default()
                    .to_ascii_lowercase()
                    .contains(&query_value))
                && (capability_value.is_empty()
                    || peer
                        .capabilities
                        .iter()
                        .any(|item| item.to_ascii_lowercase().contains(&capability_value)))
                && role_matches(peer, *role.read())
                && status_matches(peer, *peer_status.read())
                && freshness_matches(peer, *freshness.read(), now)
        })
        .cloned()
        .collect();
    let visible_hashes: HashSet<&str> =
        filtered_peers.iter().map(|peer| peer.hash.as_str()).collect();
    let filtered_paths: Vec<PathEntry> = paths
        .iter()
        .filter(|path| visible_hashes.contains(path.destination_hash.as_str()))
        .cloned()
        .collect();
    let filtered_links: Vec<LinkInfo> = links
        .iter()
        .filter(|link| visible_hashes.contains(link.peer_hash.as_str()))
        .cloned()
        .collect();

    rsx! {
        div { class: "network-page",
            section { class: "network-operations",
                div { class: "network-operation-heading",
                    div {
                        span { class: "network-operation-kicker", "Network control" }
                        h2 { "Network operations" }
                        p { "Start a bounded daemon operation. Results remain separate from submitted commands." }
                    }
                    match network_availability {
                        NetworkAvailability::Available => rsx! {
                            span { class: "network-operation-status available", "Available" }
                        },
                        NetworkAvailability::PartiallyAvailable => rsx! {
                            span { class: "network-operation-status partial", "Partially available" }
                        },
                        NetworkAvailability::ReadOnly => rsx! {
                            span { id: "network-operation-disabled", class: "network-operation-status unavailable", "Read-only" }
                        },
                    }
                }
                div { class: "network-operation-inputs",
                    label { class: "network-operation-field",
                        span { "Destination hash" }
                        input {
                            placeholder: "Enter a destination hash",
                            value: "{target}",
                            oninput: move |event| target.set(event.value()),
                        }
                    }
                    label { class: "network-operation-field",
                        span { "Link control ID" }
                        input {
                            placeholder: "Link to probe or close",
                            value: "{link_control_id}",
                            oninput: move |event| link_control_id.set(event.value()),
                        }
                    }
                    label { class: "network-operation-field",
                        span { "Request link ID" }
                        input {
                            placeholder: "Link for native request",
                            value: "{request_link_id}",
                            oninput: move |event| request_link_id.set(event.value()),
                        }
                    }
                    label { class: "network-operation-field",
                        span { "Request path" }
                        input {
                            placeholder: "/request/path",
                            value: "{request_path}",
                            oninput: move |event| request_path.set(event.value()),
                        }
                    }
                    label { class: "network-operation-field",
                        span { "Request data" }
                        input {
                            placeholder: "Optional request payload",
                            value: "{request_data}",
                            oninput: move |event| request_data.set(event.value()),
                        }
                    }
                }
                div { class: "network-operation-actions",
                    for (kind, label) in DISPLAYED_NETWORK_OPERATIONS {
                        {
                            let destructive = kind == NetworkOperationKind::LinkClose;
                            let action = SafetyAction::operate(network_operation_capability(kind), destructive);
                            let destination = target.read().trim().to_string();
                            let selected_link = link_control_id.read().trim().to_string();
                            let input_missing = operation_input_missing(kind, &destination, &selected_link);
                            let authorization = safety_policy.read().authorization(ControlPlane::Operate, &action);
                            let disabled = authorization.is_err() || input_missing;
                            let input_hint = if matches!(kind, NetworkOperationKind::Probe | NetworkOperationKind::LinkClose) { "Requires an active link ID" } else if kind == NetworkOperationKind::Announce { "No destination required" } else { "Requires a destination hash" };
                            let guidance = authorization.as_ref().err().map_or(input_hint, |denial| denial.operator_message());
                            rsx! {
                                div { class: "network-action-tile",
                                    button {
                                        class: if destructive { "network-operation-button danger" } else { "network-operation-button" },
                                        disabled,
                                        title: if input_missing { input_hint } else if let Err(denial) = &authorization { denial.operator_message() } else { "Submit typed daemon operation" },
                                        aria_describedby: disabled.then_some("network-action-guidance"),
                                        aria_label: format!("{label} using {}", if matches!(kind, NetworkOperationKind::Probe | NetworkOperationKind::LinkClose) { selected_link.as_str() } else { destination.as_str() }),
                                        onclick: move |_| {
                                            let mut request = StartNetworkOperationInfo::default();
                                            request.kind = kind;
                                            request.timeout_ms = 15_000;
                                            if matches!(kind, NetworkOperationKind::PathRequest | NetworkOperationKind::LinkOpen) && !destination.is_empty() {
                                                request.destination_hash = Some(destination.clone());
                                            }
                                            if matches!(kind, NetworkOperationKind::Probe | NetworkOperationKind::LinkClose) && !selected_link.is_empty() {
                                                request.link_id = Some(selected_link.clone());
                                            }
                                            let command = DaemonCommand::StartNetworkOperation(request);
                                            if action.is_destructive() {
                                                if let Some(pending) = network_confirmation(
                                                    &safety_policy.read(),
                                                    label,
                                                    if selected_link.is_empty() { destination.clone() } else { selected_link.clone() },
                                                    "timeout=15000 ms",
                                                    "Closes the active link and interrupts traffic using it.",
                                                    action.clone(),
                                                    command,
                                                ) {
                                                    confirmation.set(Some(pending));
                                                }
                                            } else {
                                                on_network_command.call(command);
                                            }
                                        },
                                        "{label}"
                                    }
                                    small { "{guidance}" }
                                }
                            }
                        }
                    }
                    {
                        let request_action = SafetyAction::operate("network.request", false);
                        let request_authorization = safety_policy.read().authorization(ControlPlane::Operate, &request_action);
                        let request_input_missing = request_input_missing(&request_link_id.read(), &request_path.read());
                        let request_disabled = request_authorization.is_err() || request_input_missing;
                        let request_guidance = request_authorization.as_ref().err().map_or("Requires a link ID and request path", |denial| denial.operator_message());
                        rsx! {
                            div { class: "network-action-tile",
                                button {
                                    class: "network-operation-button",
                                    disabled: request_disabled,
                                    title: if request_link_id.read().trim().is_empty() { "Requires a request link ID" } else if request_path.read().trim().is_empty() { "Requires a request path" } else if let Err(denial) = &request_authorization { denial.operator_message() } else { "Send native request" },
                                    aria_describedby: request_disabled.then_some("network-action-guidance"),
                                    onclick: move |_| {
                                        let mut request = StartRequestInfo::default();
                                        request.link_id = request_link_id.read().trim().to_string();
                                        request.path = request_path.read().trim().to_string();
                                        request.data = request_data.read().as_bytes().to_vec();
                                        request.timeout_ms = 15_000;
                                        request.max_response_size = 4 * 1024 * 1024;
                                        on_network_command.call(DaemonCommand::StartRequest(request));
                                    },
                                    "Send request"
                                }
                                small { "{request_guidance}" }
                            }
                        }
                    }
                }
                p { id: "network-action-guidance", class: "network-action-guidance", "Unavailable actions need required input or permission from the active session." }
                div { class: "network-observation-grid",
                    div { class: "network-observation-panel",
                        h3 { "Operation observations" }
                        if operations.is_empty() { p { class: "network-observation-empty", "None recorded in this session." } }
                        for operation in operations.iter().rev() {
                            article { class: "network-observation",
                                strong { "{operation.kind.as_str()}" }
                                code { "{operation.operation_id}" }
                                span { {operation.outcome.map(|value| value.as_str()).unwrap_or(operation.progress.as_str())} }
                                p { class: "network-observation-relations",
                                    span { {relationship_label("Link", operation.link_id.as_deref())} }
                                    span { {relationship_label("Correlation", operation.observation.correlation_id.as_deref())} }
                                }
                                if let Some(detail) = &operation.detail { p { "{detail}" } }
                                if operation.cancellable && operation.outcome.is_none() {
                                    button {
                                        disabled: safety_policy.read().authorization(ControlPlane::Operate, &SafetyAction::operate_session(true)).is_err(),
                                        title: safety_policy.read().authorization(ControlPlane::Operate, &SafetyAction::operate_session(true)).err().map_or("Confirm cancellation", |denial| denial.operator_message()),
                                        aria_describedby: "network-action-guidance",
                                        onclick: {
                                            let id = operation.operation_id.clone();
                                            let target = operation.operation_id.clone();
                                            move |_| {
                                                let action = SafetyAction::operate_session(true);
                                                if let Some(pending) = network_confirmation(
                                                    &safety_policy.read(),
                                                    "Cancel operation",
                                                    target.clone(),
                                                    "cancel the selected operation ID",
                                                    "Stops the in-flight network operation before it reaches a terminal outcome.",
                                                    action,
                                                    DaemonCommand::CancelNetworkOperation { operation_id: id.clone() },
                                                ) {
                                                    confirmation.set(Some(pending));
                                                }
                                            }
                                        },
                                        aria_label: format!("Cancel {} operation {}", operation.kind.as_str(), operation.operation_id),
                                        "Cancel"
                                    }
                                }
                            }
                        }
                    }
                    div { class: "network-observation-panel",
                        h3 { "Native requests" }
                        if requests.is_empty() { p { class: "network-observation-empty", "None recorded in this session." } }
                        for request in requests.iter().rev() {
                            article { class: "network-observation",
                                strong { "{request.path_hash}" }
                                code { "{request.request_id}" }
                                span { {format!("{:?} · {:.0}%", request.state, request.progress * 100.0)} }
                                p { class: "network-observation-relations",
                                    span { {relationship_label("Link", nonempty(&request.link_id))} }
                                    span { {relationship_label("Request resource", request.request_resource_hash.as_deref())} }
                                    span { {relationship_label("Response resource", request.resource_hash.as_deref())} }
                                    span { {relationship_label("Correlation", request.observation.correlation_id.as_deref())} }
                                }
                                if !request.state.is_terminal() {
                                    button {
                                        disabled: safety_policy.read().authorization(ControlPlane::Operate, &SafetyAction::operate("network.request_cancel", true)).is_err(),
                                        title: safety_policy.read().authorization(ControlPlane::Operate, &SafetyAction::operate("network.request_cancel", true)).err().map_or("Confirm request cancellation", |denial| denial.operator_message()),
                                        aria_describedby: "network-action-guidance",
                                        onclick: {
                                            let id = request.request_id.clone();
                                            let target = request.request_id.clone();
                                            move |_| {
                                                let action = SafetyAction::operate("network.request_cancel", true);
                                                if let Some(pending) = network_confirmation(
                                                    &safety_policy.read(),
                                                    "Cancel request",
                                                    target.clone(),
                                                    "cancel the selected request ID",
                                                    "Stops the native request and discards any incomplete response.",
                                                    action,
                                                    DaemonCommand::CancelRequest { request_id: id.clone() },
                                                ) {
                                                    confirmation.set(Some(pending));
                                                }
                                            }
                                        },
                                        aria_label: format!("Cancel request {}", request.request_id),
                                        "Cancel"
                                    }
                                }
                            }
                        }
                    }
                    div { class: "network-observation-panel",
                        h3 { "Resource transfers" }
                        if resources.is_empty() { p { class: "network-observation-empty", "None recorded in this session." } }
                        for resource in resources.iter().rev() {
                            article { class: "network-observation",
                                strong { {format!("{:?}", resource.direction)} }
                                code { "{resource.resource_hash}" }
                                span { {format!("{:?} · {:.0}%", resource.state, resource.progress * 100.0)} }
                                span { "{resource.received_bytes}/{resource.total_bytes} bytes" }
                                p { class: "network-observation-relations",
                                    span { {relationship_label("Link", nonempty(&resource.link_id))} }
                                    span { {relationship_label("Correlation", resource.observation.correlation_id.as_deref())} }
                                }
                                if resource.cancellable && !resource.state.is_terminal() {
                                    button {
                                        disabled: safety_policy.read().authorization(ControlPlane::Operate, &SafetyAction::operate("network.resource_cancel", true)).is_err(),
                                        title: safety_policy.read().authorization(ControlPlane::Operate, &SafetyAction::operate("network.resource_cancel", true)).err().map_or("Confirm resource cancellation", |denial| denial.operator_message()),
                                        aria_describedby: "network-action-guidance",
                                        onclick: {
                                            let hash = resource.resource_hash.clone();
                                            let target = resource.resource_hash.clone();
                                            move |_| {
                                                let action = SafetyAction::operate("network.resource_cancel", true);
                                                if let Some(pending) = network_confirmation(
                                                    &safety_policy.read(),
                                                    "Cancel resource",
                                                    target.clone(),
                                                    "cancel the selected resource hash",
                                                    "Stops this transfer and leaves the resource incomplete.",
                                                    action,
                                                    DaemonCommand::CancelResource { resource_hash: hash.clone() },
                                                ) {
                                                    confirmation.set(Some(pending));
                                                }
                                            }
                                        },
                                        aria_label: format!("Cancel resource {}", resource.resource_hash),
                                        "Cancel"
                                    }
                                }
                            }
                        }
                    }
                }
            }
            div { class: "network-mode-bar",
                for (value, label) in [
                    (NetworkMode::Discovery, "Discovery"),
                    (NetworkMode::Routes, "Routes"),
                    (NetworkMode::Links, "Links"),
                    (NetworkMode::Interfaces, "Interfaces"),
                    (NetworkMode::Combined, "Combined"),
                ] {
                    button {
                        class: if *mode.read() == value { "network-mode active" } else { "network-mode" },
                        onclick: move |_| mode.set(value),
                        "{label}"
                    }
                }
            }
            div { class: "network-filter-bar",
                input {
                    aria_label: "Search peers",
                    placeholder: "Search name or hash",
                    value: "{query}",
                    oninput: move |event| query.set(event.value()),
                }
                select {
                    aria_label: "Filter by role",
                    onchange: move |event| role.set(parse_role(&event.value())),
                    option { value: "all", "All roles" }
                    option { value: "styrene", "Styrene" }
                    option { value: "hub", "Hub" }
                    option { value: "pages", "Page host" }
                    option { value: "rns", "RNS" }
                }
                select {
                    aria_label: "Filter by status",
                    onchange: move |event| peer_status.set(parse_status(&event.value())),
                    option { value: "all", "All status" }
                    option { value: "online", "Online" }
                    option { value: "offline", "Offline" }
                }
                select {
                    aria_label: "Filter by freshness",
                    onchange: move |event| freshness.set(parse_freshness(&event.value())),
                    option { value: "all", "All freshness" }
                    option { value: "recent", "Recent <1h" }
                    option { value: "stale", "Stale ≥1h" }
                    option { value: "unknown", "Unknown" }
                }
                input {
                    aria_label: "Filter by capability",
                    placeholder: "Capability",
                    value: "{capability}",
                    oninput: move |event| capability.set(event.value()),
                }
                span { class: "network-filter-count", "{filtered_peers.len()}/{peers.len()} peers" }
            }

            match *mode.read() {
                NetworkMode::Discovery => rsx! {
                    DiscoveryView { peers: filtered_peers }
                },
                NetworkMode::Routes => rsx! {
                    RoutesView { paths: filtered_paths, peers: peers.clone() }
                },
                NetworkMode::Links => rsx! {
                    LinksView {
                        links: filtered_links,
                        peers: peers.clone(),
                        on_use_for_control: move |id| link_control_id.set(id),
                        on_use_for_request: move |id| request_link_id.set(id),
                    }
                },
                NetworkMode::Interfaces => rsx! {
                    InterfacesView { interfaces: interfaces.clone() }
                },
                NetworkMode::Combined => rsx! {
                    NetworkGraph {
                        peers: filtered_peers,
                        paths: filtered_paths.clone(),
                        status: status.clone(),
                        local_hash: local_hash.clone(),
                        local_name: local_name.clone(),
                        on_select_peer,
                        on_browse_page,
                        links: filtered_links,
                        interfaces: interfaces.clone(),
                    }
                },
            }
            if let Some(pending) = confirmation.read().clone() {
                div { class: "confirmation-backdrop",
                    div {
                        class: "confirmation-dialog",
                        role: "dialog",
                        aria_modal: "true",
                        aria_labelledby: "network-confirmation-title",
                        onkeydown: move |event: KeyboardEvent| if event.key() == Key::Escape {
                            confirmation.set(None);
                        },
                        h2 { id: "network-confirmation-title", "Confirm {pending.label}" }
                        p { "Target: {pending.target}" }
                        p { "Parameters: {pending.parameters}" }
                        p { "Consequence: {pending.consequence}" }
                        div { class: "confirmation-actions",
                            button { autofocus: true, onclick: move |_| confirmation.set(None), "Cancel" }
                            button {
                                class: "danger",
                                disabled: safety_policy.read().confirm_authorization(ControlPlane::Operate, &pending.action, pending.token).is_err(),
                                onclick: move |_| {
                                    match safety_policy.read().confirm(ControlPlane::Operate, &pending.action, pending.token) {
                                        Ok(()) => on_network_command.call(pending.command.clone()),
                                        Err(error) => tracing::warn!(target: "dx::safety", %error, "network confirmation rejected"),
                                    }
                                    confirmation.set(None);
                                },
                                "Confirm"
                            }
                        }
                        if let Err(denial) = safety_policy.read().confirm_authorization(ControlPlane::Operate, &pending.action, pending.token) {
                            p { class: "control-disabled-reason", "Confirmation unavailable: {denial.operator_message()}" }
                        }
                    }
                }
            }
        }
    }
}

fn network_confirmation(
    safety: &SafetyContext,
    label: impl Into<String>,
    target: String,
    parameters: impl Into<String>,
    consequence: impl Into<String>,
    action: SafetyAction,
    command: DaemonCommand,
) -> Option<NetworkConfirmation> {
    match safety.begin_confirmation(ControlPlane::Operate, &action) {
        Ok(token) => Some(NetworkConfirmation {
            label: label.into(),
            target,
            parameters: parameters.into(),
            consequence: consequence.into(),
            action,
            command,
            token,
        }),
        Err(error) => {
            tracing::warn!(target: "dx::safety", %error, "network action rejected");
            None
        }
    }
}

fn network_operation_capability(kind: NetworkOperationKind) -> &'static str {
    match kind {
        NetworkOperationKind::Announce => "network.announce",
        NetworkOperationKind::PathRequest => "network.path_request",
        NetworkOperationKind::Probe => "network.probe",
        NetworkOperationKind::LinkOpen => "network.link_open",
        NetworkOperationKind::LinkClose => "network.link_close",
        _ => "network.unknown",
    }
}

fn aggregate_availability(authorization: impl IntoIterator<Item = bool>) -> NetworkAvailability {
    let mut total = 0_usize;
    let mut available = 0_usize;
    for authorized in authorization {
        total += 1;
        available += usize::from(authorized);
    }
    if total > 0 && available == total {
        NetworkAvailability::Available
    } else if available > 0 {
        NetworkAvailability::PartiallyAvailable
    } else {
        NetworkAvailability::ReadOnly
    }
}

fn operation_input_missing(kind: NetworkOperationKind, destination: &str, link_id: &str) -> bool {
    (matches!(kind, NetworkOperationKind::PathRequest | NetworkOperationKind::LinkOpen)
        && destination.trim().is_empty())
        || (matches!(kind, NetworkOperationKind::Probe | NetworkOperationKind::LinkClose)
            && link_id.trim().is_empty())
}

fn request_input_missing(link_id: &str, path: &str) -> bool {
    link_id.trim().is_empty() || path.trim().is_empty()
}

fn nonempty(value: &str) -> Option<&str> {
    (!value.trim().is_empty()).then_some(value)
}

fn relationship_label(label: &str, value: Option<&str>) -> String {
    format!("{label}: {}", value.filter(|value| !value.trim().is_empty()).unwrap_or("unknown"))
}

#[component]
fn DiscoveryView(peers: Vec<PeerEntry>) -> Element {
    rsx! {
        div { class: "network-mode-content",
            h2 { "Discovery Observations" }
            p { class: "network-mode-description", "Accepted announces. Discovery does not imply a route or active link." }
            if peers.is_empty() {
                div { class: "network-mode-empty", "No peer observations match the current filters." }
            } else {
                div { class: "network-record-list",
                    for peer in peers {
                        article { class: "network-record",
                            div { class: "network-record-primary",
                                strong { {peer.name.as_deref().unwrap_or("Unnamed peer")} }
                                code { "{peer.hash}" }
                            }
                            span { class: "network-record-kind", "{role_label(&peer.node_role)}" }
                            span { "{peer.status}" }
                            span { {peer.last_announce.map(format_epoch).unwrap_or_else(|| "unknown".into())} }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn RoutesView(paths: Vec<PathEntry>, peers: Vec<PeerEntry>) -> Element {
    rsx! {
        div { class: "network-mode-content",
            h2 { "Route Table" }
            p { class: "network-mode-description", "Daemon-authoritative next hops and hop counts." }
            if paths.is_empty() {
                div { class: "network-mode-empty", "No routes match the current filters." }
            } else {
                div { class: "network-record-list",
                    for path in paths {
                        article { class: "network-record",
                            div { class: "network-record-primary",
                                strong { {peer_label(&peers, &path.destination_hash)} }
                                code { "{path.destination_hash}" }
                            }
                            span { class: "network-record-kind", "{path.hops} hops" }
                            span { "via {peer_label(&peers, &path.next_hop)}" }
                            span { "{path.interface}" }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn LinksView(
    links: Vec<LinkInfo>,
    peers: Vec<PeerEntry>,
    on_use_for_control: EventHandler<String>,
    on_use_for_request: EventHandler<String>,
) -> Element {
    rsx! {
        div { class: "network-mode-content",
            h2 { "Link Lifecycle" }
            p { class: "network-mode-description", "Observed RNS link state and round-trip telemetry." }
            if links.is_empty() {
                div { class: "network-mode-empty", "No links match the current filters." }
            } else {
                div { class: "network-record-list",
                    for link in links {
                        article { class: "network-record",
                            div { class: "network-record-primary",
                                strong { {peer_label(&peers, &link.peer_hash)} }
                                code { "{link.peer_hash}" }
                            }
                            span { class: "network-record-kind", "{link.status}" }
                            span { {format!("{:?}", link.activity)} }
                            code { "{link.link_id}" }
                            span { {link.rtt_ms.map(|value| format!("{value:.0} ms")).unwrap_or_else(|| "RTT unknown".into())} }
                            span { {if link.observation.stale { "stale".to_string() } else { format_epoch(link.timestamp) }} }
                            span { {relationship_label("Correlation", link.observation.correlation_id.as_deref())} }
                            div { class: "network-record-actions",
                                button {
                                    onclick: {
                                        let id = link.link_id.clone();
                                        move |_| on_use_for_control.call(id.clone())
                                    },
                                    "Use for controls"
                                }
                                button {
                                    onclick: {
                                        let id = link.link_id.clone();
                                        move |_| on_use_for_request.call(id.clone())
                                    },
                                    "Use for request"
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
fn InterfacesView(interfaces: Vec<InterfaceInfo>) -> Element {
    rsx! {
        div { class: "network-mode-content",
            h2 { "Transport Interfaces" }
            p { class: "network-mode-description", "Configured transport interfaces and observed byte counters." }
            if interfaces.is_empty() {
                div { class: "network-mode-empty", "No interface telemetry is available." }
            } else {
                div { class: "network-record-list",
                    for interface in interfaces {
                        article { class: "network-record",
                            div { class: "network-record-primary",
                                strong { "{interface.name}" }
                                code { "{interface.hash}" }
                            }
                            span { class: "network-record-kind", "{interface.status}" }
                            span { "type {interface.kind} · mode {interface.mode} · enabled {interface.enabled}" }
                            span { {format!("local {}", interface.local_endpoint.as_deref().unwrap_or("unknown"))} }
                            span { {format!("remote {}", interface.remote_endpoint.as_deref().unwrap_or("unknown"))} }
                            span { {format!("host {} · port {}", interface.host.as_deref().unwrap_or("unknown"), interface.port.map(|value| value.to_string()).unwrap_or_else(|| "unknown".into()))} }
                            span { {format!("parent {} · peers {}", interface.parent_hash.as_deref().unwrap_or("none"), interface.peers_connected)} }
                            span { "TX {format_bytes(interface.tx_bytes)}" }
                            span { "RX {format_bytes(interface.rx_bytes)}" }
                            span { {format!(
                                "source {:?} · observed {:?} · age {:?} · threshold {:?} · stale {} · generation {:?} · correlation {}",
                                interface.observation.source,
                                interface.observation.observed_at,
                                interface.observation.age_secs,
                                interface.observation.freshness_threshold_secs,
                                interface.observation.stale,
                                interface.observation.connection_generation,
                                interface.observation.correlation_id.as_deref().unwrap_or("none")
                            )} }
                        }
                    }
                }
            }
        }
    }
}

fn role_matches(peer: &PeerEntry, filter: RoleFilter) -> bool {
    match filter {
        RoleFilter::All => true,
        RoleFilter::Styrene => peer.node_role == PeerRole::Styrene,
        RoleFilter::Hub => peer.node_role == PeerRole::Hub,
        RoleFilter::PageHost => peer.node_role == PeerRole::PageHost,
        RoleFilter::Rns => peer.node_role == PeerRole::Rns,
    }
}

fn status_matches(peer: &PeerEntry, filter: StatusFilter) -> bool {
    let online = peer.status != "offline" && !peer.status.is_empty();
    match filter {
        StatusFilter::All => true,
        StatusFilter::Online => online,
        StatusFilter::Offline => !online,
    }
}

fn freshness_matches(peer: &PeerEntry, filter: FreshnessFilter, now: i64) -> bool {
    match (filter, peer.last_announce) {
        (FreshnessFilter::All, _) => true,
        (FreshnessFilter::Unknown, None) => true,
        (FreshnessFilter::Recent, Some(timestamp)) => now.saturating_sub(timestamp) < 3600,
        (FreshnessFilter::Stale, Some(timestamp)) => now.saturating_sub(timestamp) >= 3600,
        _ => false,
    }
}

fn parse_role(value: &str) -> RoleFilter {
    match value {
        "styrene" => RoleFilter::Styrene,
        "hub" => RoleFilter::Hub,
        "pages" => RoleFilter::PageHost,
        "rns" => RoleFilter::Rns,
        _ => RoleFilter::All,
    }
}

fn parse_status(value: &str) -> StatusFilter {
    match value {
        "online" => StatusFilter::Online,
        "offline" => StatusFilter::Offline,
        _ => StatusFilter::All,
    }
}

fn parse_freshness(value: &str) -> FreshnessFilter {
    match value {
        "recent" => FreshnessFilter::Recent,
        "stale" => FreshnessFilter::Stale,
        "unknown" => FreshnessFilter::Unknown,
        _ => FreshnessFilter::All,
    }
}

fn role_label(role: &PeerRole) -> &'static str {
    match role {
        PeerRole::Styrene => "Styrene",
        PeerRole::Hub => "Hub",
        PeerRole::PageHost => "Page host",
        PeerRole::Rns => "RNS",
    }
}

fn peer_label(peers: &[PeerEntry], hash: &str) -> String {
    peers
        .iter()
        .find(|peer| peer.hash == hash)
        .and_then(|peer| peer.name.clone())
        .unwrap_or_else(|| hash[..8.min(hash.len())].to_string())
}

fn format_epoch(timestamp: i64) -> String {
    if timestamp <= 0 { "unknown".into() } else { format!("epoch {timestamp}") }
}

fn format_bytes(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{bytes} B")
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn peer(role: PeerRole, status: &str, last_announce: Option<i64>) -> PeerEntry {
        PeerEntry {
            hash: "peer".into(),
            identity_hash: None,
            name: Some("Test peer".into()),
            status: status.into(),
            node_role: role,
            capabilities: vec!["pages".into()],
            version: None,
            last_announce,
            announce_count: 1,
        }
    }

    #[test]
    fn filters_keep_role_status_and_freshness_semantics_independent() {
        let value = peer(PeerRole::Hub, "online", Some(9_500));
        assert!(role_matches(&value, RoleFilter::Hub));
        assert!(status_matches(&value, StatusFilter::Online));
        assert!(freshness_matches(&value, FreshnessFilter::Recent, 10_000));
        assert!(!freshness_matches(&value, FreshnessFilter::Stale, 10_000));
    }

    #[test]
    fn unknown_freshness_is_not_treated_as_recent() {
        let value = peer(PeerRole::Rns, "offline", None);
        assert!(freshness_matches(&value, FreshnessFilter::Unknown, 10_000));
        assert!(!freshness_matches(&value, FreshnessFilter::Recent, 10_000));
    }

    #[test]
    fn aggregate_availability_uses_authorization_only() {
        assert_eq!(aggregate_availability([true, true, true]), NetworkAvailability::Available);
        assert_eq!(
            aggregate_availability([true, false, false]),
            NetworkAvailability::PartiallyAvailable
        );
        assert_eq!(aggregate_availability([false, false]), NetworkAvailability::ReadOnly);
        assert_eq!(aggregate_availability([]), NetworkAvailability::ReadOnly);
    }

    #[test]
    fn network_operation_capabilities_are_explicit() {
        assert_eq!(
            network_operation_capability(NetworkOperationKind::Announce),
            "network.announce"
        );
        assert_eq!(
            network_operation_capability(NetworkOperationKind::PathRequest),
            "network.path_request"
        );
        assert_eq!(
            network_operation_capability(NetworkOperationKind::LinkClose),
            "network.link_close"
        );
    }

    #[test]
    fn operation_validation_uses_only_its_required_input() {
        assert!(!operation_input_missing(NetworkOperationKind::Announce, "", ""));
        assert!(!operation_input_missing(NetworkOperationKind::LinkOpen, "peer", ""));
        assert!(operation_input_missing(NetworkOperationKind::LinkOpen, "", "link"));
        assert!(!operation_input_missing(NetworkOperationKind::Probe, "", "link"));
        assert!(operation_input_missing(NetworkOperationKind::Probe, "peer", ""));
    }

    #[test]
    fn request_validation_requires_its_own_link_and_path() {
        assert!(request_input_missing("", "/status"));
        assert!(request_input_missing("request-link", ""));
        assert!(!request_input_missing("request-link", "/status"));
    }

    #[test]
    fn relationships_distinguish_authoritative_values_from_unknown() {
        assert_eq!(relationship_label("Link", Some("link-7")), "Link: link-7");
        assert_eq!(relationship_label("Link", None), "Link: unknown");
        assert_eq!(relationship_label("Link", Some("  ")), "Link: unknown");
    }
}
