//! Desktop-only navigation projections. No service or custody ownership.
use dioxus::prelude::*;

use crate::state::{ActivityEntry, AppRoute, PeerRole};
use crate::stores::{DataState, DomainStores, FleetJobState};

#[derive(Clone, Debug, PartialEq)]
pub struct Destination {
    pub route: AppRoute,
    pub label: &'static str,
    pub summary: String,
    pub tools: bool,
    pub icon: &'static str,
}

fn observed(state: &DataState, summary: String) -> String {
    match state {
        DataState::Loading => "Loading…".into(),
        DataState::Error { .. } => "Unavailable".into(),
        DataState::Degraded { .. } => "Needs attention".into(),
        DataState::Ready | DataState::Empty => summary,
    }
}

pub fn destinations(view: &DomainStores) -> Vec<Destination> {
    let network = if let Some(inventory) = &view.network.interface_inventory {
        let connected = inventory.entries.iter().filter(|i| i.state == "connected").count();
        format!("{connected} connected · {} saved", inventory.entries.len())
    } else {
        "Interface state unavailable".into()
    };
    let unread: u32 = view.messages.conversations.iter().map(|c| c.unread_count).sum();
    let rows = [
        (
            AppRoute::Overview,
            "Overview",
            observed(&view.runtime.state, "Session ready".into()),
            false,
            "M3 3h7v7H3z M14 3h7v7h-7z M3 14h7v7H3z M14 14h7v7h-7z",
        ),
        (
            AppRoute::Messages,
            "Messages",
            observed(
                &view.messages.state,
                if view.messages.conversation_cursor.is_some() {
                    format!("{unread} unread in loaded list")
                } else {
                    format!("{unread} unread")
                },
            ),
            false,
            "M4 4h16v12H9l-5 4z",
        ),
        (
            AppRoute::Browse,
            "Browse",
            observed(
                &view.network.state,
                format!(
                    "{} advertised page hosts",
                    view.network.peers.iter().filter(|p| p.node_role == PeerRole::PageHost).count()
                ),
            ),
            false,
            "M3 5h18v15H3z M3 9h18 M7 5v4",
        ),
        (
            AppRoute::Network,
            "Network",
            observed(&view.network.state, network),
            false,
            "M4 5h5v5H4z M15 14h5v5h-5z M9 7h8v7 M6 10v7h9",
        ),
        (
            AppRoute::Fleet,
            "Fleet",
            observed(
                &view.fleet.state,
                format!(
                    "{} active jobs",
                    view.fleet
                        .jobs
                        .iter()
                        .filter(|job| job.state == FleetJobState::Running)
                        .count()
                ),
            ),
            false,
            "M3 4h7v6H3z M14 4h7v6h-7z M8 16h8v5H8z M6 10v3h12v-3 M12 13v3",
        ),
        (
            AppRoute::Lab,
            "Lab",
            observed(
                &view.scenario.state,
                if view.scenario.run.is_some() { "Run available" } else { "No active run" }.into(),
            ),
            true,
            "M8 3h8 M10 3v7L4 20h16l-6-10V3 M7 15h10",
        ),
        (
            AppRoute::System,
            "System",
            observed(&view.runtime.state, "Runtime and storage".into()),
            true,
            "M4 4h16v6H4z M4 14h16v6H4z M7 7h1 M7 17h1",
        ),
    ];
    rows.into_iter()
        .map(|(route, label, summary, tools, icon)| Destination {
            route,
            label,
            summary,
            tools,
            icon,
        })
        .collect()
}

#[component]
pub fn WorkspaceNav(
    items: Vec<Destination>,
    active: AppRoute,
    on_select: EventHandler<AppRoute>,
) -> Element {
    let mut open = use_signal(|| false);
    rsx! {
        div { class: "workspace-navigation",
            onkeydown: move |event| if event.key() == Key::Escape { open.set(false); },
            button { class: "workspace-menu", aria_expanded: *open.read(), aria_controls: "workspace-nav",
                onclick: move |_| open.toggle(), "Navigation" }
            nav { id: "workspace-nav", class: if *open.read() { "workspace-rail expanded" } else { "workspace-rail" }, aria_label: "Primary navigation",
                for item in items {
                    div { key: "{item.label}",
                    if item.tools && item.route == AppRoute::Lab { div { class: "workspace-tools-label", "Tools" } }
                    button {
                        class: if active.primary() == item.route { "workspace-destination selected" } else { "workspace-destination" },
                        aria_current: (active.primary() == item.route).then_some("page"),
                        title: "{item.label}: {item.summary}",
                        onclick: move |_| { open.set(false); on_select.call(item.route.clone()); },
                        svg { width: "20", height: "20", view_box: "0 0 24 24", fill: "none", stroke: "currentColor", stroke_width: "1.5", "aria-hidden": "true",
                            path { d: item.icon }
                        }
                        span { class: "workspace-labels",
                            strong { "{item.label}" }
                            span { "{item.summary}" }
                        }
                    }
                    }
                }
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ConnectionSummary {
    pub id: String,
    pub name: String,
    pub state: String,
    pub detail: String,
    pub error: Option<String>,
}

pub fn connections(view: &DomainStores) -> Vec<ConnectionSummary> {
    view.network
        .interface_inventory
        .as_ref()
        .map(|inventory| {
            inventory
                .entries
                .iter()
                .map(|i| ConnectionSummary {
                    id: i.settings.id.clone(),
                    name: i.settings.name.clone(),
                    state: i.state.clone(),
                    detail: format!(
                        "{} · TX queued {} / RX {}",
                        match i.settings.kind.as_str() {
                            "tcp_client" => "TCP client",
                            "tcp_server" => "TCP listener",
                            other => other,
                        },
                        byte_count(i.tx_bytes),
                        byte_count(i.rx_bytes)
                    ),
                    error: i.error.clone(),
                })
                .collect()
        })
        .unwrap_or_default()
}

fn byte_count(bytes: u64) -> String {
    if bytes >= 1_048_576 {
        format!("{:.1} MiB", bytes as f64 / 1_048_576.0)
    } else if bytes >= 1024 {
        format!("{:.1} KiB", bytes as f64 / 1024.0)
    } else {
        format!("{bytes} B")
    }
}

pub fn recent_activity(view: &DomainStores) -> Vec<ActivityEntry> {
    view.activity
        .entries
        .iter()
        .filter(|entry| {
            entry.severity != crate::state::ActivitySeverity::Info
                || !matches!(entry.kind, "discovery" | "routes" | "status" | "session")
        })
        .rev()
        .take(5)
        .cloned()
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect()
}

#[component]
pub fn OverviewPage(
    session_summary: String,
    profile: String,
    connections: Vec<ConnectionSummary>,
    inventory_known: bool,
    activity: Vec<ActivityEntry>,
    on_select: EventHandler<AppRoute>,
    on_connection: EventHandler<String>,
    on_session: EventHandler<()>,
    on_activity: EventHandler<()>,
) -> Element {
    let failures: Vec<_> = connections.iter().filter(|c| c.error.is_some()).cloned().collect();
    rsx! {
        section { class: "overview-page",
            header { class: "workspace-heading",
                p { class: "eyebrow", "Your mesh workspace" }
                h1 { "Overview" }
                p { "{profile} · {session_summary}" }
                button { class: "workspace-link", onclick: move |_| on_session.call(()), "Connection settings" }
            }
            if !failures.is_empty() {
                section { class: "overview-section attention",
                    h2 { "Needs attention" }
                    for item in failures {
                        button { class: "overview-row", onclick: move |_| on_connection.call(item.id.clone()),
                            span { strong { "{item.name}" } span { "{item.error.as_deref().unwrap_or_default()}" } }
                            span { "Inspect →" }
                        }
                    }
                }
            }
            section { class: "overview-section",
                div { class: "overview-section-heading", h2 { "Saved connections" }
                    button { class: "workspace-link", onclick: move |_| on_select.call(AppRoute::Network), "Open Network →" }
                }
                if !inventory_known {
                    p { class: "overview-empty", "Saved interface inventory is not available for this session. Open Network for reported runtime interfaces." }
                } else if connections.is_empty() {
                    p { class: "overview-empty", "No saved connections. Add an interface in Network to connect this node." }
                }
                for item in connections.iter().take(6) {
                    {
                        let id = item.id.clone();
                        rsx! { button { class: "overview-row", onclick: move |_| on_connection.call(id.clone()),
                            span { strong { "{item.name}" } span { "{item.detail}" } }
                            span { "{item.state} →" }
                        } }
                    }
                }
                if connections.len() > 6 { p { class: "overview-empty", "Showing 6 of {connections.len()} saved connections. Open Network for all interfaces." } }
            }
            section { class: "overview-section",
                div { class: "overview-section-heading", h2 { "Recent activity" }
                    button { class: "workspace-link", onclick: move |_| on_activity.call(()), "All activity →" }
                }
                if activity.is_empty() { p { class: "overview-empty", "No recent operator activity. Open Activity for runtime diagnostics." } }
                for entry in activity.iter().rev().take(5) {
                    div { class: "overview-event",
                        span { class: "eyebrow", "{entry.kind}" }
                        p { "{entry.summary}" }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workspace_routes_preserve_legacy_hints_and_delivery_parent() {
        assert_eq!(AppRoute::from_hint("command"), AppRoute::Overview);
        assert_eq!(AppRoute::from_hint("content"), AppRoute::Browse);
        assert_eq!(AppRoute::from_hint("propagation"), AppRoute::Delivery);
        assert_eq!(AppRoute::from_hint("messages/delivery").primary(), AppRoute::Messages);
        assert_eq!(AppRoute::from_hint("network/interfaces"), AppRoute::Network);
    }

    #[test]
    fn workspace_navigation_is_stable_and_unknown_is_not_zero() {
        let view = DomainStores::default();
        let items = destinations(&view);
        assert_eq!(
            items.iter().map(|i| i.label).collect::<Vec<_>>(),
            ["Overview", "Messages", "Browse", "Network", "Fleet", "Lab", "System"]
        );
        assert!(items.iter().all(|i| i.summary == "Loading…"));
        assert_eq!(items.iter().filter(|i| i.tools).count(), 2);
    }

    #[test]
    fn workspace_fleet_does_not_mistake_discovery_for_management() {
        let mut view = DomainStores::default();
        view.fleet.state = DataState::Ready;
        view.fleet.managed_peers = vec!["discovered-node".into(); 245];
        assert_eq!(destinations(&view)[4].summary, "0 active jobs");
    }

    #[test]
    fn workspace_recent_activity_is_bounded_and_keeps_failures() {
        let mut view = DomainStores::default();
        for timestamp in 0..10 {
            view.activity.entries.push(ActivityEntry {
                timestamp,
                severity: crate::state::ActivitySeverity::Info,
                kind: "interface",
                summary: "Interface saved".into(),
                entity: None,
                correlation_id: None,
                provenance: "fixture".into(),
            });
        }
        let mut noise = view.activity.entries[0].clone();
        noise.kind = "routes";
        view.activity.entries.push(noise.clone());
        assert_eq!(recent_activity(&view).len(), 5);
        assert_eq!(recent_activity(&view)[0].timestamp, 5);
        noise.severity = crate::state::ActivitySeverity::Error;
        view.activity.entries.push(noise);
        assert_eq!(recent_activity(&view).last().map(|e| e.kind), Some("routes"));
    }

    #[test]
    fn workspace_inventory_preserves_connection_failures_and_saved_count() {
        let mut view = DomainStores::default();
        view.network.state = DataState::Ready;
        let mut inventory = styrene_ipc::types::InterfaceInventory::default();
        let mut entry = styrene_ipc::types::ManagedInterface::default();
        entry.settings.id = "hub".into();
        entry.settings.name = "Community Hub".into();
        entry.state = "failed".into();
        entry.error = Some("Connection refused".into());
        inventory.entries.push(entry);
        view.network.interface_inventory = Some(inventory);
        assert_eq!(connections(&view)[0].error.as_deref(), Some("Connection refused"));
        assert_eq!(destinations(&view)[3].summary, "0 connected · 1 saved");
    }
}
