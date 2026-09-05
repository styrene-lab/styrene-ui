use dioxus::document;
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
    #[default]
    Discovery,
    Routes,
    Links,
    Interfaces,
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
    connected: bool,
    peers: Vec<PeerEntry>,
    paths: Vec<PathEntry>,
    status: MeshStatusInfo,
    local_hash: String,
    local_name: Option<String>,
    on_select_peer: EventHandler<String>,
    on_browse_page: EventHandler<String>,
    links: Vec<LinkInfo>,
    interfaces: Vec<InterfaceInfo>,
    interface_inventory: Option<styrene_ipc::types::InterfaceInventory>,
    interface_unavailable: Option<String>,
    interface_error: Option<String>,
    interface_result_serial: u64,
    interface_action_notice: Option<String>,
    interface_focus: Option<String>,
    interface_edit_unavailable: Option<String>,
    operations: Vec<NetworkOperationInfo>,
    requests: Vec<RequestObservationInfo>,
    resources: Vec<styrene_ipc::types::ResourceTransferInfo>,
    safety: Memo<SafetyContext>,
    on_network_command: EventHandler<DaemonCommand>,
) -> Element {
    let safety_policy = safety;
    let mut mode = use_signal(|| match std::env::var("STYRENE_DX_ROUTE").as_deref() {
        Ok("network/interfaces") => NetworkMode::Interfaces,
        Ok("network/routes") => NetworkMode::Routes,
        Ok("network/links") => NetworkMode::Links,
        Ok("network/map") => NetworkMode::Combined,
        _ => NetworkMode::default(),
    });
    let mut selected = use_signal(|| None::<String>);
    let mut actions_open = use_signal(|| false);
    let mut filters_open = use_signal(|| false);
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
        selected.set(None);
        actions_open.set(false);
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
    let mut map_interfaces = interfaces.clone();
    let interface_names: Vec<_> = interface_inventory
        .as_ref()
        .map(|inventory| {
            inventory
                .entries
                .iter()
                .map(|entry| {
                    let hash = entry
                        .runtime_hash
                        .clone()
                        .unwrap_or_else(|| format!("configured:{}", entry.settings.id));
                    if !map_interfaces.iter().any(|i| i.hash == hash) {
                        let mut interface = InterfaceInfo::default();
                        interface.hash = hash.clone();
                        interface.name = entry.settings.name.clone();
                        interface.kind = entry.settings.kind.clone();
                        interface.enabled = entry.settings.enabled;
                        interface.status = entry.state.clone();
                        map_interfaces.push(interface);
                    }
                    crate::state::MapInterface {
                        hash,
                        name: entry.settings.name.clone(),
                        remote_target: (entry.settings.kind == "tcp_client"
                            && !entry.settings.host.trim().is_empty())
                        .then(|| crate::state::remote_target_key(&entry.settings.host)),
                    }
                })
                .collect()
        })
        .unwrap_or_default();
    let interface_count = interface_inventory
        .as_ref()
        .map(|inventory| inventory.entries.len())
        .unwrap_or(interfaces.len());
    let mut all_rows = inventory_rows(&peers, &paths, &links, &interfaces, now);
    all_rows.push(InventoryRow {
        key: format!("local:{local_hash}"),
        mode: NetworkMode::Combined,
        name: local_name.clone().unwrap_or_else(|| "This node".into()),
        kind: "Local node".into(),
        cells: Vec::new(),
        details: vec![
            ("Destination".into(), local_hash.clone()),
            (
                "Transport".into(),
                if status.transport_active { "Active" } else { "Inactive" }.into(),
            ),
            ("Interfaces".into(), interface_count.to_string()),
        ],
        diagnostics: "Local daemon observation".into(),
        destination: String::new(),
        link_id: String::new(),
        can_browse: false,
    });
    let visible_rows: Vec<_> = all_rows
        .iter()
        .filter(|row| {
            row.mode == *mode.read()
                && row.matches(&query_value)
                && (row.mode != NetworkMode::Discovery
                    || filtered_peers.iter().any(|peer| peer.hash == row.destination))
        })
        .cloned()
        .collect();
    let mut selected_for_map = all_rows.clone();
    if let Some(inventory) = &interface_inventory {
        let mut entities = std::collections::BTreeMap::<String, Vec<_>>::new();
        for entry in &inventory.entries {
            if entry.settings.kind == "tcp_client" && !entry.settings.host.trim().is_empty() {
                entities
                    .entry(crate::state::remote_target_key(&entry.settings.host))
                    .or_default()
                    .push(entry);
            }
        }
        for (target, mut connections) in entities {
            connections.sort_by(|a, b| a.runtime_hash.cmp(&b.runtime_hash));
            let details = connections
                .iter()
                .map(|entry| {
                    (
                        format!("{} · {}", entry.settings.name, entry.settings.kind),
                        format!(
                            "{}:{} · {}\nResolved endpoint: {}",
                            entry.settings.host,
                            entry.settings.port,
                            entry.state,
                            entry.remote_endpoint.as_deref().unwrap_or("Not connected")
                        ),
                    )
                })
                .collect();
            selected_for_map.push(InventoryRow {
                key: format!("entity:{target}"),
                mode: NetworkMode::Interfaces,
                name: connections[0].settings.name.clone(),
                kind: format!("Remote entity · {} connections", connections.len()),
                cells: Vec::new(),
                details,
                diagnostics: format!(
                    "Configured target: {target}\nRemote mesh identity: not established"
                ),
                destination: String::new(),
                link_id: String::new(),
                can_browse: false,
            });
        }
    }
    let selected_row =
        selected_for_map.iter().find(|row| Some(&row.key) == selected.read().as_ref()).cloned();
    let mut select_row = move |row: InventoryRow| {
        target.set(row.destination.clone());
        link_control_id.set(row.link_id.clone());
        request_link_id.set(row.link_id.clone());
        actions_open.set(!matches!(row.mode, NetworkMode::Interfaces | NetworkMode::Combined));
        selected.set(Some(row.key));
    };
    let local_only = !interfaces.is_empty()
        && interfaces.iter().all(|interface| {
            interface.local_endpoint.as_deref().is_some_and(|endpoint| {
                endpoint.starts_with("127.") || endpoint.starts_with("[::1]:")
            }) && interface.remote_endpoint.is_none()
        });

    rsx! {
        div { class: if *mode.read() == NetworkMode::Combined { "network-page network-map-page" } else { "network-page" },
            header { class: "network-heading",
                div {
                    h2 { "Network" }
                    p {
                        if connected {
                            "Interfaces: {interface_count} · Observed peers: {peers.len()} · Routes: {paths.len()} · Link records: {links.len()}"
                        } else {
                            "Disconnected · Network observations unavailable"
                        }
                    }
                }
                button {
                    class: "network-action-menu",
                    hidden: *mode.read() == NetworkMode::Interfaces,
                    aria_expanded: "{actions_open}",
                    onclick: move |_| {
                        selected.set(None);
                        target.set(String::new());
                        link_control_id.set(String::new());
                        request_link_id.set(String::new());
                        actions_open.toggle();
                    },
                    "Actions"
                }
            }
            div { class: "network-mode-bar",
                for (value, label) in [
                    (NetworkMode::Discovery, "Peers"),
                    (NetworkMode::Interfaces, "Interfaces"),
                    (NetworkMode::Routes, "Routes"),
                    (NetworkMode::Links, "Links"),
                    (NetworkMode::Combined, "Map"),
                ]
                {
                    button {
                        class:
                        if *mode.read() == value {
                            "network-mode active"
                        } else {
                            "network-mode"
                        },
                        onclick: move |_| {
                            mode.set(value);
                            selected.set(None);
                            actions_open.set(false);
                            target.set(String::new());
                            link_control_id.set(String::new());
                            request_link_id.set(String::new());
                        },
                        "{label}"
                    }
                }
            }
            if *mode.read() == NetworkMode::Interfaces {
                super::interface_manager::InterfaceManager {
                    inventory: interface_inventory,
                    load_error: interface_unavailable,
                    mutation_error: interface_error,
                    result_serial: interface_result_serial,
                    action_notice: interface_action_notice,
                    focus: interface_focus,
                    unavailable: interface_edit_unavailable,
                    connected,
                    generation: safety.read().generation_key(),
                    runtime: interfaces.clone(),
                    on_command: on_network_command,
                }
            } else {
            div { class: "network-filter-bar",
                input {
                    aria_label: "Search current network inventory",
                    placeholder: if *mode.read() == NetworkMode::Combined { "Find a destination…" } else { "Search this view…" },
                    value: "{query}",
                    oninput: move |event| query.set(event.value()),
                }
                if *mode.read() == NetworkMode::Discovery {
                    button {
                        class: "network-mode",
                        aria_expanded: "{filters_open}",
                        onclick: move |_| filters_open.toggle(),
                        "Filters"
                    }
                    if *filters_open.read() {
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
                    }
                }
                span { class: "network-filter-count",
                    if *mode.read() != NetworkMode::Combined {
                        if visible_rows.len() == 1 {
                            "1 result"
                        } else {
                            "{visible_rows.len()} results"
                        }
                    }
                }
            }

            div { class:
                if connected && (selected_row.is_some() || *actions_open.read()) {
                    "network-workspace inspecting"
                } else {
                    "network-workspace"
                },
                div { class: "network-inventory",
                    if !connected {
                        div { class: "network-empty", role: "status",
                            h3 { "Network unavailable" }
                            p {
                                "Connect to a daemon to inspect its network. Previous observations are not current status."
                            }
                        }
                    } else if *mode.read() == NetworkMode::Combined {
                        NetworkGraph {
                            peers: peers.clone(),
                            paths: paths.clone(),
                            status: status.clone(),
                            local_hash: local_hash.clone(),
                            local_name: local_name.clone(),
                            on_select_peer,
                            on_browse_page,
                            links: links.clone(),
                            interfaces: map_interfaces,
                            interface_names: interface_names.clone(),
                            inspection: Some(
                                EventHandler::new(move |id: String| {
                                    if let Some(row) = selected_for_map
                                        .iter()
                                        .find(|row| {
                                            row.key == id || row.key == format!("interface:{}", id.trim_start_matches("iface:"))
                                                || row.key == format!("peer:{id}")
                                                || row.key == format!("route:{id}")
                                                    || row.key == format!("local:{id}")
                                                    || (row.mode == NetworkMode::Links && row.destination == id)
                                        })
                                    {
                                        select_row(row.clone());
                                    }
                                }),
                            ),
                            search: query_value.clone(),
                        }
                    } else if visible_rows.is_empty() {
                        div { class: "network-empty",
                            h3 {
                                if !query_value.is_empty() || (*mode.read() == NetworkMode::Discovery && !peers.is_empty()) {
                                    "No matches"
                                } else {
                                    {empty_title(*mode.read())}
                                }
                            }
                            p {
                                if !query_value.is_empty() || (*mode.read() == NetworkMode::Discovery && !peers.is_empty()) {
                                    "Try another search or clear the filters."
                                } else if *mode.read() == NetworkMode::Discovery && local_only {
                                    "Your interfaces accept local connections only. No external connection is configured."
                                } else if *mode.read() == NetworkMode::Discovery {
                                    "No peer announcements have been observed. Review your interfaces and their connection status."
                                } else {
                                    "This session has not supplied any records for this view."
                                }
                            }
                            if *mode.read() == NetworkMode::Discovery {
                                button {
                                    class: "network-mode",
                                    onclick: move |_| {
                                        mode.set(NetworkMode::Interfaces);
                                        query.set(String::new());
                                    },
                                    "View interfaces"
                                }
                            }
                        }
                    } else {
                        InventoryTable {
                            rows: visible_rows,
                            mode: *mode.read(),
                            selected: selected.read().clone(),
                            on_select: select_row,
                        }
                    }
                }
                if connected && (selected_row.is_some() || *actions_open.read()) {
                    aside {
                        class: "network-detail",
                        aria_label: "Network selection",
                        button {
                            class: "network-detail-close",
                            onclick: move |_| {
                                selected.set(None);
                                actions_open.set(false);
                            },
                            "Close details"
                        }
                        if let Some(row) = selected_row.clone() {
                            h3 { "{row.name}" }
                            p { class: "network-detail-kind", "{row.kind}" }
                            CopyIdentifier { value: row.key.split_once(':').map_or(row.key.as_str(), |(_, value)| value).to_string() }
                            dl {
                                for (label, value) in row.details.iter() {
                                    dt { "{label}" }
                                    dd { "{value}" }
                                }
                            }
                            if !row.destination.is_empty() {
                                button {
                                    class: "network-mode",
                                    onclick: {
                                        let destination = row.destination.clone();
                                        move |_| on_select_peer.call(destination.clone())
                                    },
                                    "Message"
                                }
                            }
                            if row.can_browse {
                                button {
                                    class: "network-mode",
                                    onclick: {
                                        let destination = row.destination.clone();
                                        move |_| on_browse_page.call(destination.clone())
                                    },
                                    "Browse content"
                                }
                            }
                            details { class: "network-advanced",
                                summary { "Diagnostics" }
                                code { "{row.diagnostics}" }
                            }
                        }
                        if *actions_open.read() {
                            section { class: "network-operations contextual-operations",
                                div { class: "network-operation-heading",
                                    div {
                                        h2 { "Actions" }
                                    }
                                    match network_availability {
                                        NetworkAvailability::Available => rsx! {
                                            span { class: "network-operation-status available", "Available" }
                                        },
                                        NetworkAvailability::PartiallyAvailable => rsx! {
                                            span { class: "network-operation-status partial", "Partially available" }
                                        },
                                        NetworkAvailability::ReadOnly => rsx! {
                                            span {
                                                id: "network-operation-disabled",
                                                class: "network-operation-status unavailable",
                                                "Read-only"
                                            }
                                        },
                                    }
                                }
                                details { class: "network-advanced",
                                    summary { "Advanced targeting" }
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

                                    }
                                }
                                div { class: "network-operation-actions",
                                    for (kind, label) in DISPLAYED_NETWORK_OPERATIONS
                                        .into_iter()
                                        .filter(|(kind, _)| {
                                            match kind {
                                                NetworkOperationKind::Announce => selected.read().is_none(),
                                                NetworkOperationKind::PathRequest | NetworkOperationKind::LinkOpen => {
                                                    !target.read().is_empty()
                                                }
                                                NetworkOperationKind::Probe | NetworkOperationKind::LinkClose => {
                                                    !link_control_id.read().is_empty()
                                                }
                                                _ => false,
                                            }
                                        })
                                    {
                                        {
                                            let destructive = kind == NetworkOperationKind::LinkClose;
                                            let action = SafetyAction::operate(
                                                network_operation_capability(kind),
                                                destructive,
                                            );
                                            let destination = target.read().trim().to_string();
                                            let selected_link = link_control_id.read().trim().to_string();
                                            let input_missing = operation_input_missing(kind, &destination, &selected_link);
                                            let authorization = safety_policy
                                                .read()
                                                .authorization(ControlPlane::Operate, &action);
                                            let disabled = authorization.is_err() || input_missing;
                                            let input_hint = if matches!(
                                                kind,
                                                NetworkOperationKind::Probe | NetworkOperationKind::LinkClose
                                            ) {
                                                "Requires an active link ID"
                                            } else if kind == NetworkOperationKind::Announce {
                                                "No destination required"
                                            } else {
                                                "Requires a destination hash"
                                            };
                                            let guidance = authorization
                                                .as_ref()
                                                .err()
                                                .map_or(input_hint, |denial| denial.operator_message());
                                            rsx! {
                                                div { class: "network-action-tile",
                                                    button {
                                                        class:
                                                        if destructive {
                                                            "network-operation-button danger"
                                                        } else {
                                                            "network-operation-button"
                                                        },
                                                        disabled,
                                                        title:
                                                        if input_missing {
                                                            input_hint
                                                        } else if let Err(denial) = &authorization {
                                                            denial.operator_message()
                                                        } else {
                                                            "Submit typed daemon operation"
                                                        },
                                                        aria_describedby: disabled.then_some("network-action-guidance"),
                                                        aria_label: format!(
                                                            "{label} using {}",
                                                            if matches!(
                                                                kind,
                                                                NetworkOperationKind::Probe | NetworkOperationKind::LinkClose
                                                            ) {
                                                                selected_link.as_str()
                                                            } else {
                                                                destination.as_str()
                                                            },
                                                        ),
                                                        onclick: move |_| {
                                                            let mut request = StartNetworkOperationInfo::default();
                                                            request.kind = kind;
                                                            request.timeout_ms = 15_000;
                                                            if matches!(
                                                                kind,
                                                                NetworkOperationKind::PathRequest | NetworkOperationKind::LinkOpen
                                                            ) && !destination.is_empty()
                                                            {
                                                                request.destination_hash = Some(destination.clone());
                                                            }
                                                            if matches!(kind, NetworkOperationKind::Probe | NetworkOperationKind::LinkClose)
                                                                && !selected_link.is_empty()
                                                            {
                                                                request.link_id = Some(selected_link.clone());
                                                            }
                                                            let command = DaemonCommand::StartNetworkOperation(request);
                                                            if action.is_destructive() {
                                                                if let Some(pending) = network_confirmation(
                                                                    &safety_policy.read(),
                                                                    label,
                                                                    if selected_link.is_empty() {
                                                                        destination.clone()
                                                                    } else {
                                                                        selected_link.clone()
                                                                    },
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
                                    if !request_link_id.read().is_empty() {
                                        {
                                            let request_action = SafetyAction::operate("network.request", false);
                                            let request_authorization = safety_policy
                                                .read()
                                                .authorization(ControlPlane::Operate, &request_action);
                                            let request_input_missing = request_input_missing(
                                                &request_link_id.read(),
                                                &request_path.read(),
                                            );
                                            let request_disabled = request_authorization.is_err() || request_input_missing;
                                            let request_guidance = request_authorization
                                                .as_ref()
                                                .err()
                                                .map_or(
                                                    "Requires a link ID and request path",
                                                    |denial| denial.operator_message(),
                                                );
                                            rsx! {
                                                details { class: "network-advanced",
                                                    summary { "Native request" }
                                                    div { class: "network-operation-inputs",
                                        label { class: "network-operation-field",
                                            span { "Request link ID" }
                                            input {
                                                placeholder: "Link for native request",
                                                value: "{request_link_id}",
                                                readonly: true,
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

                                                div { class: "network-action-tile",
                                                    button {
                                                        class: "network-operation-button",
                                                        disabled: request_disabled,
                                                        title:
                                                        if request_link_id.read().trim().is_empty() {
                                                            "Requires a request link ID"
                                                        } else if request_path.read().trim().is_empty() {
                                                            "Requires a request path"
                                                        } else if let Err(denial) = &request_authorization {
                                                            denial.operator_message()
                                                        } else {
                                                            "Send native request"
                                                        },
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
                                    }
                                }
                                p {
                                    id: "network-action-guidance",
                                    class: "network-action-guidance",
                                    "Unavailable actions need required input or permission from the active session."
                                }
                            }
                        }
                    }
                }
            }
            }
            details { class: "network-activity",
                summary {
                    "Activity · {operations.len()} operations · {requests.len()} requests · {resources.len()} transfers"
                }
                if operations.is_empty() && requests.is_empty() && resources.is_empty() {
                    p { "No operations recorded in this session." }
                }
                div { class: "network-observation-grid",
                    if !operations.is_empty() {
                        div { class: "network-observation-panel",
                            h3 { "Operation observations" }
                            if operations.is_empty() {
                                p { class: "network-observation-empty",
                                    "None recorded in this session."
                                }
                            }
                            for operation in operations.iter().rev() {
                                article { class: "network-observation",
                                    strong { "{operation.kind.as_str()}" }
                                    code { "{operation.operation_id}" }
                                    span {
                                        {
                                            operation
                                                .outcome
                                                .map(|value| value.as_str())
                                                .unwrap_or(operation.progress.as_str())
                                        }
                                    }
                                    p { class: "network-observation-relations",
                                        span {
                                            {relationship_label("Link", operation.link_id.as_deref())}
                                        }
                                        span {
                                            {
                                                relationship_label(
                                                    "Correlation",
                                                    operation.observation.correlation_id.as_deref(),
                                                )
                                            }
                                        }
                                    }
                                    if let Some(detail) = &operation.detail {
                                        p { "{detail}" }
                                    }
                                    if operation.cancellable && operation.outcome.is_none() {
                                        button {
                                            disabled: safety_policy
                                                .read()
                                                .authorization(ControlPlane::Operate, &SafetyAction::operate_session(true))
                                                .is_err(),
                                            title: safety_policy
                                                .read()
                                                .authorization(ControlPlane::Operate, &SafetyAction::operate_session(true))
                                                .err()
                                                .map_or("Confirm cancellation", |denial| denial.operator_message()),
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
                                                        DaemonCommand::CancelNetworkOperation {
                                                            operation_id: id.clone(),
                                                        },
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
                    }
                    if !requests.is_empty() {
                        div { class: "network-observation-panel",
                            h3 { "Native requests" }
                            if requests.is_empty() {
                                p { class: "network-observation-empty",
                                    "None recorded in this session."
                                }
                            }
                            for request in requests.iter().rev() {
                                article { class: "network-observation",
                                    strong { "{request.path_hash}" }
                                    code { "{request.request_id}" }
                                    span {
                                        {format!("{:?} · {:.0}%", request.state, request.progress * 100.0)}
                                    }
                                    p { class: "network-observation-relations",
                                        span { {relationship_label("Link", nonempty(&request.link_id))} }
                                        span {
                                            {relationship_label("Request resource", request.request_resource_hash.as_deref())}
                                        }
                                        span {
                                            {relationship_label("Response resource", request.resource_hash.as_deref())}
                                        }
                                        span {
                                            {relationship_label("Correlation", request.observation.correlation_id.as_deref())}
                                        }
                                    }
                                    if !request.state.is_terminal() {
                                        button {
                                            disabled: safety_policy
                                                .read()
                                                .authorization(
                                                    ControlPlane::Operate,
                                                    &SafetyAction::operate("network.request_cancel", true),
                                                )
                                                .is_err(),
                                            title: safety_policy
                                                .read()
                                                .authorization(
                                                    ControlPlane::Operate,
                                                    &SafetyAction::operate("network.request_cancel", true),
                                                )
                                                .err()
                                                .map_or("Confirm request cancellation", |denial| denial.operator_message()),
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
                                                        DaemonCommand::CancelRequest {
                                                            request_id: id.clone(),
                                                        },
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
                    }
                    if !resources.is_empty() {
                        div { class: "network-observation-panel",
                            h3 { "Resource transfers" }
                            if resources.is_empty() {
                                p { class: "network-observation-empty",
                                    "None recorded in this session."
                                }
                            }
                            for resource in resources.iter().rev() {
                                article { class: "network-observation",
                                    strong { {format!("{:?}", resource.direction)} }
                                    code { "{resource.resource_hash}" }
                                    span {
                                        {format!("{:?} · {:.0}%", resource.state, resource.progress * 100.0)}
                                    }
                                    span { "{resource.received_bytes}/{resource.total_bytes} bytes" }
                                    p { class: "network-observation-relations",
                                        span { {relationship_label("Link", nonempty(&resource.link_id))} }
                                        span {
                                            {
                                                relationship_label("Correlation", resource.observation.correlation_id.as_deref())
                                            }
                                        }
                                    }
                                    if resource.cancellable && !resource.state.is_terminal() {
                                        button {
                                            disabled: safety_policy
                                                .read()
                                                .authorization(
                                                    ControlPlane::Operate,
                                                    &SafetyAction::operate("network.resource_cancel", true),
                                                )
                                                .is_err(),
                                            title: safety_policy
                                                .read()
                                                .authorization(
                                                    ControlPlane::Operate,
                                                    &SafetyAction::operate("network.resource_cancel", true),
                                                )
                                                .err()
                                                .map_or("Confirm resource cancellation", |denial| denial.operator_message()),
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
                                                        DaemonCommand::CancelResource {
                                                            resource_hash: hash.clone(),
                                                        },
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
            }
            if let Some(pending) = confirmation.read().clone() {
                div { class: "confirmation-backdrop",
                    div {
                        class: "confirmation-dialog",
                        role: "dialog",
                        aria_modal: "true",
                        aria_labelledby: "network-confirmation-title",
                        onkeydown: move |event: KeyboardEvent| {
                            if event.key() == Key::Escape {
                                confirmation.set(None);
                            }
                        },
                        h2 { id: "network-confirmation-title", "Confirm {pending.label}" }
                        p { "Target: {pending.target}" }
                        p { "Parameters: {pending.parameters}" }
                        p { "Consequence: {pending.consequence}" }
                        div { class: "confirmation-actions",
                            button {
                                autofocus: true,
                                onclick: move |_| confirmation.set(None),
                                "Cancel"
                            }
                            button {
                                class: "danger",
                                disabled: safety_policy
                                    .read()
                                    .confirm_authorization(ControlPlane::Operate, &pending.action, pending.token)
                                    .is_err(),
                                onclick: move |_| {
                                    match safety_policy
                                        .read()
                                        .confirm(ControlPlane::Operate, &pending.action, pending.token)
                                    {
                                        Ok(()) => on_network_command.call(pending.command.clone()),
                                        Err(error) => {
                                            tracing::warn!(
                                                target : "dx::safety", % error, "network confirmation rejected"
                                            )
                                        }
                                    }
                                    confirmation.set(None);
                                },
                                "Confirm"
                            }
                        }
                        if let Err(denial) = safety_policy
                            .read()
                            .confirm_authorization(ControlPlane::Operate, &pending.action, pending.token)
                        {
                            p { class: "control-disabled-reason",
                                "Confirmation unavailable: {denial.operator_message()}"
                            }
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
fn CopyIdentifier(value: String) -> Element {
    let mut feedback = use_signal(String::new);
    let short: String = value.chars().take(16).collect();
    rsx! {
        div { class: "network-identifier",
            code { title: "{value}", "{short}…" }
            button {
                class: "network-mode",
                onclick: move |_| {
                    let Ok(encoded) = serde_json::to_string(&value) else {
                        return;
                    };
                    spawn(async move {
                        let script = format!(
                            r#"
                                        const value = {encoded};
                                        try {{ await navigator.clipboard.writeText(value); return true; }} catch (_) {{}}
                                        const field = document.createElement('textarea');
                                        field.value = value; document.body.appendChild(field); field.select();
                                        const copied = document.execCommand('copy'); field.remove(); return copied;
                                    "#,
                        );
                        let copied = document::eval(&script).join::<bool>().await.unwrap_or(false);
                        feedback
                            .set(
                                if copied {
                                    "Copied"
                                } else {
                                    "Copy unavailable; select the identifier in Details."
                                }
                                    .into(),
                            );
                    });
                },
                "Copy ID"
            }
            span { role: "status", "{feedback}" }
        }
    }
}

#[derive(Clone, PartialEq)]
struct InventoryRow {
    key: String,
    mode: NetworkMode,
    name: String,
    kind: String,
    cells: Vec<String>,
    details: Vec<(String, String)>,
    diagnostics: String,
    destination: String,
    link_id: String,
    can_browse: bool,
}

impl InventoryRow {
    fn matches(&self, query: &str) -> bool {
        query.is_empty()
            || self.name.to_ascii_lowercase().contains(query)
            || self.key.to_ascii_lowercase().contains(query)
            || self.cells.iter().any(|cell| cell.to_ascii_lowercase().contains(query))
    }
}

fn inventory_rows(
    peers: &[PeerEntry],
    paths: &[PathEntry],
    links: &[LinkInfo],
    interfaces: &[InterfaceInfo],
    now: i64,
) -> Vec<InventoryRow> {
    let mut rows = Vec::new();
    for peer in peers {
        let route =
            paths.iter().find(|path| path.destination_hash == peer.hash && !path.observation.stale);
        let active_link = links.iter().find(|link| {
            link.peer_hash == peer.hash && link.status == "active" && !link.observation.stale
        });
        let reachability = if active_link.is_some() {
            "Link active"
        } else if route.is_some() {
            "Route known"
        } else {
            "Not established"
        };
        let heard = match peer.last_announce.filter(|time| *time > 0) {
            Some(time) => {
                let age = now.saturating_sub(time);
                if age < 60 {
                    "Just now".into()
                } else if age < 3600 {
                    format!("{} min ago", age / 60)
                } else {
                    format!("{} h ago", age / 3600)
                }
            }
            None => "Unknown".into(),
        };
        rows.push(InventoryRow {
            key: format!("peer:{}", peer.hash),
            mode: NetworkMode::Discovery,
            name: peer
                .name
                .clone()
                .filter(|name| !name.is_empty())
                .unwrap_or_else(|| peer_label(peers, &peer.hash)),
            kind: role_label(&peer.node_role).into(),
            cells: vec![
                role_label(&peer.node_role).into(),
                reachability.into(),
                heard.clone(),
                route.map_or_else(|| "Unknown".into(), |route| route.interface.clone()),
            ],
            details: vec![
                ("Destination".into(), peer.hash.clone()),
                ("Reachability".into(), reachability.into()),
                ("Last heard".into(), heard),
                ("Capabilities".into(), peer.capabilities.join(", ")),
            ],
            diagnostics: format!(
                "Announces: {}\nReported status: {}",
                peer.announce_count, peer.status
            ),
            destination: peer.hash.clone(),
            link_id: String::new(),
            can_browse: matches!(peer.node_role, PeerRole::PageHost | PeerRole::Hub),
        });
    }
    for path in paths {
        rows.push(InventoryRow {
            key: format!("route:{}", path.destination_hash),
            mode: NetworkMode::Routes,
            name: peer_label(peers, &path.destination_hash),
            kind: "Route".into(),
            cells: vec![
                peer_label(peers, &path.next_hop),
                path.hops.to_string(),
                path.interface.clone(),
                if path.observation.stale { "Stale" } else { "Observed" }.into(),
            ],
            details: vec![
                ("Destination".into(), path.destination_hash.clone()),
                ("Next hop".into(), path.next_hop.clone()),
                ("Interface".into(), path.interface.clone()),
            ],
            diagnostics: format!("{:?}", path.observation),
            destination: path.destination_hash.clone(),
            link_id: String::new(),
            can_browse: false,
        });
    }
    for link in links {
        rows.push(InventoryRow {
            key: format!("link:{}", link.link_id),
            mode: NetworkMode::Links,
            name: peer_label(peers, &link.peer_hash),
            kind: "Link".into(),
            cells: vec![
                if link.status.is_empty() { "Unknown".into() } else { link.status.clone() },
                link.rtt_ms.map_or_else(|| "Unknown".into(), |rtt| format!("{rtt:.0} ms")),
                if link.observation.stale { "Stale" } else { "Observed" }.into(),
                link.link_id.chars().take(12).collect(),
            ],
            details: vec![
                ("Peer".into(), link.peer_hash.clone()),
                ("Link ID".into(), link.link_id.clone()),
                ("State".into(), link.status.clone()),
            ],
            diagnostics: format!("{:?}\n{:?}", link.activity, link.observation),
            destination: link.peer_hash.clone(),
            link_id: link.link_id.clone(),
            can_browse: false,
        });
    }
    for interface in interfaces {
        rows.push(InventoryRow {
            key: format!("interface:{}", interface.hash),
            mode: NetworkMode::Interfaces,
            name: interface.name.clone(),
            kind: interface.kind.clone(),
            cells: vec![
                interface.kind.clone(),
                if interface.status.is_empty() {
                    "Unknown".into()
                } else {
                    interface.status.clone()
                },
                format_bytes(interface.tx_bytes),
                format_bytes(interface.rx_bytes),
            ],
            details: vec![
                (
                    "Local endpoint".into(),
                    interface.local_endpoint.clone().unwrap_or_else(|| "Unknown".into()),
                ),
                (
                    "Remote endpoint".into(),
                    interface.remote_endpoint.clone().unwrap_or_else(|| "Not reported".into()),
                ),
                ("Mode".into(), interface.mode.clone()),
                ("Enabled".into(), interface.enabled.to_string()),
                ("Connected peers".into(), interface.peers_connected.to_string()),
            ],
            diagnostics: format!("{}\n{:?}", interface.hash, interface.observation),
            destination: String::new(),
            link_id: String::new(),
            can_browse: false,
        });
    }
    rows.sort_by(|left, right| {
        left.name
            .to_ascii_lowercase()
            .cmp(&right.name.to_ascii_lowercase())
            .then_with(|| left.key.cmp(&right.key))
    });
    rows
}

fn empty_title(mode: NetworkMode) -> &'static str {
    match mode {
        NetworkMode::Discovery => "No peers observed yet",
        NetworkMode::Interfaces => "No interface telemetry",
        NetworkMode::Routes => "No routes observed",
        NetworkMode::Links => "No links observed",
        NetworkMode::Combined => "No topology observations",
    }
}

#[component]
fn InventoryTable(
    rows: Vec<InventoryRow>,
    mode: NetworkMode,
    selected: Option<String>,
    on_select: EventHandler<InventoryRow>,
) -> Element {
    let headings = match mode {
        NetworkMode::Discovery => ["Peer", "Role", "Reachability", "Last heard", "Interface"],
        NetworkMode::Routes => ["Destination", "Next hop", "Hops", "Interface", "Observation"],
        NetworkMode::Links => ["Peer", "State", "RTT", "Observation", "Link"],
        NetworkMode::Interfaces => ["Interface", "Type", "State", "Sent", "Received"],
        NetworkMode::Combined => ["Node", "Role", "Reachability", "Last heard", "Interface"],
    };
    rsx! {
        table { class: "network-table",
            caption { class: "sr-only", "Network inventory. Select a name to inspect its details." }
            thead {
                tr {
                    for heading in headings {
                        th { scope: "col", "{heading}" }
                    }
                }
            }
            tbody {
                for row in rows {
                    tr {
                        key: "{row.key}",
                        class:
                        if selected.as_deref() == Some(row.key.as_str()) {
                            "selected"
                        } else {
                            ""
                        },
                        td {
                            button {
                                class: "network-row-select",
                                onclick: {
                                    let row = row.clone();
                                    move |_| on_select.call(row.clone())
                                },
                                "{row.name}"
                            }
                        }
                        for cell in &row.cells {
                            td { "{cell}" }
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
    let online = peer.status == "online";
    match filter {
        StatusFilter::All => true,
        StatusFilter::Online => online,
        StatusFilter::Offline => peer.status == "offline",
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
    fn unknown_peer_status_is_not_offline() {
        let value = peer(PeerRole::Rns, "", None);
        assert!(!status_matches(&value, StatusFilter::Online));
        assert!(!status_matches(&value, StatusFilter::Offline));
        let rows = inventory_rows(&[value], &[], &[], &[], 10_000);
        assert_eq!(rows[0].cells[1], "Not established");
        assert_eq!(rows[0].cells[2], "Unknown");
    }

    #[test]
    fn inventories_preserve_routes_and_links_without_discovery() {
        let path = PathEntry {
            destination_hash: "undiscovered".into(),
            hops: 2,
            next_hop: "relay".into(),
            interface: "Radio".into(),
            expires: None,
            observation: Default::default(),
        };
        let link = LinkInfo {
            link_id: "link-1".into(),
            peer_hash: "undiscovered".into(),
            status: "active".into(),
            activity: styrene_ipc::types::LinkActivity::Active,
            rtt_ms: None,
            timestamp: 1,
            observation: Default::default(),
        };
        let rows = inventory_rows(&[], &[path], &[link], &[], 10_000);
        assert_eq!(rows.len(), 2);
        assert!(rows.iter().any(|row| row.mode == NetworkMode::Routes && row.matches("radio")));
        let link = rows.iter().find(|row| row.mode == NetworkMode::Links).expect("link inventory");
        assert_eq!(link.link_id, "link-1");
        assert_eq!(link.destination, "undiscovered");
        assert_eq!(link.cells[1], "Unknown");
        assert!(!link.matches("unrelated"));
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
