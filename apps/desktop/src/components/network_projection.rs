//! Bounded presentation of observations. Group membership is not physical topology.
use std::collections::{BTreeMap, BTreeSet, HashMap};

use super::network_model;
use crate::state::{
    GraphEdge, GraphEdgeKind, GraphNode, GraphNodeType, InterfaceInfo, LinkInfo, PathEntry,
    PeerEntry,
};

pub(crate) const PAGE_SIZE: usize = 16;
const NODE_LIMIT: usize = 100;
const EDGE_LIMIT: usize = 200;

#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct DestinationGroup {
    pub id: String,
    pub label: String,
    pub count: usize,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct Projection {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
    pub groups: Vec<DestinationGroup>,
    pub destinations: usize,
    pub stale_routes: usize,
    pub inactive_links: usize,
    pub offline_peers: usize,
    pub matches: usize,
    pub shown: usize,
    pub page: usize,
    pub pages: usize,
    pub omitted_interfaces: usize,
    pub omitted_links: usize,
}

pub(crate) struct View<'a> {
    pub group: &'a str,
    pub query: &'a str,
    pub page: usize,
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn project(
    local_hash: &str,
    local_name: Option<&str>,
    peers: &[PeerEntry],
    paths: &[PathEntry],
    interfaces: &[InterfaceInfo],
    links: &[LinkInfo],
    names: &[(String, String)],
    view: View<'_>,
) -> Projection {
    let mut result = Projection {
        stale_routes: paths.iter().filter(|p| p.observation.stale).count(),
        inactive_links: links.iter().filter(|l| !active(l)).count(),
        offline_peers: peers.iter().filter(|p| p.status == "offline").count(),
        ..Default::default()
    };
    let mut sorted_interfaces: Vec<_> = interfaces.iter().collect();
    sorted_interfaces.sort_by(|a, b| a.hash.cmp(&b.hash));
    let interface_ids: HashMap<_, _> = interfaces
        .iter()
        .flat_map(|i| {
            [
                (i.name.as_str(), format!("iface:{}", i.hash)),
                (i.hash.as_str(), format!("iface:{}", i.hash)),
            ]
        })
        .collect();
    let interface_labels: HashMap<_, _> = interfaces
        .iter()
        .map(|i| {
            let label =
                names.iter().find(|(hash, _)| hash == &i.hash).map_or(&i.name, |(_, name)| name);
            (format!("iface:{}", i.hash), label.clone())
        })
        .collect();
    let peers_by_id: HashMap<_, _> = peers.iter().map(|p| (p.hash.as_str(), p)).collect();
    let mut groups = BTreeMap::<String, BTreeSet<String>>::new();
    let mut current_paths = BTreeMap::new();
    for path in paths.iter().filter(|p| !p.observation.stale && !p.destination_hash.is_empty()) {
        let key = interface_ids
            .get(path.interface.as_str())
            .cloned()
            .unwrap_or_else(|| "unattributed".into());
        groups.entry(key).or_default().insert(path.destination_hash.clone());
        current_paths.entry(path.destination_hash.as_str()).or_insert(path);
    }
    for peer in peers.iter().filter(|p| p.status != "offline" && p.hash != local_hash) {
        if !current_paths.contains_key(peer.hash.as_str()) {
            groups.entry("discovery".into()).or_default().insert(peer.hash.clone());
        }
    }
    let all: BTreeSet<_> = groups.values().flatten().cloned().collect();
    result.destinations = all.len();
    result.groups = groups
        .iter()
        .map(|(key, members)| DestinationGroup {
            id: key.clone(),
            label: interface_labels.get(key).map_or_else(
                || match key.as_str() {
                    "discovery" => "Discovered · no current route".into(),
                    _ => "Routes · interface unknown".into(),
                },
                |name| format!("Via {name}"),
            ),
            count: members.len(),
        })
        .collect();
    let query = view.query.trim().to_ascii_lowercase();
    let candidates: Vec<_> = if !query.is_empty() {
        all.iter()
            .filter(|id| {
                id.to_ascii_lowercase().contains(&query)
                    || peers_by_id
                        .get(id.as_str())
                        .and_then(|p| p.name.as_ref())
                        .is_some_and(|name| name.to_ascii_lowercase().contains(&query))
            })
            .cloned()
            .collect()
    } else {
        groups.get(view.group).map(|g| g.iter().cloned().collect()).unwrap_or_default()
    };
    result.matches = candidates.len();
    result.pages = candidates.len().div_ceil(PAGE_SIZE);
    result.page = view.page.min(result.pages.saturating_sub(1));
    let revealed: BTreeSet<_> =
        candidates.into_iter().skip(result.page * PAGE_SIZE).take(PAGE_SIZE).collect();
    result.shown = revealed.len();
    let mut wanted = revealed.clone();
    for id in &revealed {
        if let Some(path) = current_paths.get(id.as_str())
            && !path.next_hop.is_empty()
        {
            wanted.insert(path.next_hop.clone());
        }
    }
    let mut active_links: Vec<_> = links.iter().filter(|l| active(l)).collect();
    active_links.sort_by(|a, b| a.link_id.cmp(&b.link_id));
    result.omitted_links = active_links.len().saturating_sub(8);
    active_links.truncate(8);
    wanted.extend(active_links.iter().map(|l| l.peer_hash.clone()));
    let selected_peers: Vec<_> = wanted
        .iter()
        .map(|id| {
            peers_by_id.get(id.as_str()).map(|p| (*p).clone()).unwrap_or_else(|| PeerEntry {
                hash: id.clone(),
                identity_hash: None,
                name: None,
                status: String::new(),
                node_role: crate::state::PeerRole::Rns,
                capabilities: Vec::new(),
                version: None,
                last_announce: None,
                announce_count: 0,
            })
        })
        .collect();
    // The existing peer projection only sees a bounded neighborhood.
    (result.nodes, _) =
        network_model::build_graph(local_hash, local_name, &selected_peers, &[], &[], &[], &[]);
    let mut indices: HashMap<_, _> =
        result.nodes.iter().enumerate().map(|(i, n)| (n.id.clone(), i)).collect();
    result.omitted_interfaces = sorted_interfaces.len().saturating_sub(16);
    for (row, interface) in sorted_interfaces.into_iter().take(16).enumerate() {
        let id = format!("iface:{}", interface.hash);
        let online =
            matches!(interface.status.as_str(), "online" | "active" | "connected" | "listening");
        let index = add_node(
            &mut result,
            &mut indices,
            id.clone(),
            interface_labels[&id].clone(),
            GraphNodeType::Interface { online },
            0.0,
            row as f64 * 140.0,
        );
        connect(&mut result, Some(0), index, GraphEdgeKind::Interface, 0);
        if online
            && let Some(endpoint) = interface.remote_endpoint.as_ref().filter(|s| !s.is_empty())
        {
            let endpoint_index = add_node(
                &mut result,
                &mut indices,
                format!("endpoint:{}", interface.hash),
                endpoint.clone(),
                GraphNodeType::Endpoint,
                250.0,
                row as f64 * 140.0,
            );
            connect(&mut result, index, endpoint_index, GraphEdgeKind::Interface, 0);
        }
    }
    // Summaries are bounded too. Every group remains accessible through the selector.
    let summaries: Vec<_> = result.groups.iter().take(16).cloned().collect();
    for (row, group) in summaries.iter().enumerate() {
        let index = add_node(
            &mut result,
            &mut indices,
            format!("group:{}", group.id),
            format!("{} destinations\n{}", group.count, group.label),
            GraphNodeType::Summary,
            510.0,
            row as f64 * 140.0,
        );
        let source = group
            .id
            .strip_prefix("iface:")
            .and_then(|hash| indices.get(&format!("endpoint:{hash}")))
            .or_else(|| indices.get(&group.id))
            .copied();
        connect(&mut result, source, index, GraphEdgeKind::Association, 0);
    }
    for id in &revealed {
        if let Some(path) = current_paths.get(id.as_str()) {
            let target = indices.get(id).copied();
            let iface =
                interface_ids.get(path.interface.as_str()).and_then(|id| indices.get(id)).copied();
            if path.next_hop != path.destination_hash && !path.next_hop.is_empty() {
                let relay = indices.get(&path.next_hop).copied();
                connect(&mut result, iface, relay, GraphEdgeKind::Route, 0);
                connect(&mut result, relay, target, GraphEdgeKind::Route, path.hops);
            } else {
                connect(&mut result, iface, target, GraphEdgeKind::Route, path.hops);
            }
        }
    }
    for link in active_links {
        let target = indices.get(&link.peer_hash).copied();
        connect(&mut result, Some(0), target, GraphEdgeKind::Link, 0);
    }
    let mut row = 0;
    for node in &mut result.nodes {
        if !matches!(
            node.node_type,
            GraphNodeType::Local
                | GraphNodeType::Interface { .. }
                | GraphNodeType::Endpoint
                | GraphNodeType::Summary
        ) {
            node.x = 820.0 + (row % 2) as f64 * 250.0;
            node.y = (row / 2) as f64 * 85.0;
            row += 1;
        }
        node.vx = 0.0;
        node.vy = 0.0;
    }
    result
}

fn active(link: &LinkInfo) -> bool {
    !link.observation.stale && link.activity == styrene_ipc::types::LinkActivity::Active
}

#[allow(clippy::too_many_arguments)]
fn add_node(
    result: &mut Projection,
    indices: &mut HashMap<String, usize>,
    id: String,
    label: String,
    node_type: GraphNodeType,
    x: f64,
    y: f64,
) -> Option<usize> {
    if let Some(index) = indices.get(&id) {
        return Some(*index);
    }
    if result.nodes.len() >= NODE_LIMIT {
        return None;
    }
    let index = result.nodes.len();
    indices.insert(id.clone(), index);
    result.nodes.push(GraphNode {
        id,
        label,
        node_type,
        capabilities: Vec::new(),
        x,
        y,
        vx: 0.0,
        vy: 0.0,
    });
    Some(index)
}

fn connect(
    result: &mut Projection,
    source: Option<usize>,
    target: Option<usize>,
    kind: GraphEdgeKind,
    hops: u8,
) {
    if let (Some(source), Some(target)) = (source, target)
        && source != target
        && result.edges.len() < EDGE_LIMIT
        && !result.edges.iter().any(|e| e.source == source && e.target == target && e.kind == kind)
    {
        result.edges.push(GraphEdge { source, target, kind, hops });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn paths(count: usize) -> Vec<PathEntry> {
        (0..count)
            .map(|i| PathEntry {
                destination_hash: format!("destination-{i:05}"),
                next_hop: "relay".into(),
                interface: "tcp".into(),
                hops: 2,
                expires: None,
                observation: Default::default(),
            })
            .collect()
    }

    fn interface() -> InterfaceInfo {
        let mut value = InterfaceInfo::default();
        value.hash = "tcp-id".into();
        value.name = "tcp".into();
        value.status = "connected".into();
        value.remote_endpoint = Some("rns.styrene.io:4242".into());
        value
    }

    fn view<'a>(group: &'a str, query: &'a str, page: usize) -> View<'a> {
        View { group, query, page }
    }

    #[test]
    fn thousand_destinations_collapse_before_layout() {
        let p =
            project("local", None, &[], &paths(1500), &[interface()], &[], &[], view("", "", 0));
        assert_eq!(p.destinations, 1500);
        assert_eq!(p.nodes.len(), 4); // local, interface, endpoint, summary
        assert_eq!(p.groups[0].count, 1500);
        assert!(!p.nodes.iter().any(|n| n.id == "relay"));
        assert!(!p.edges.iter().any(|e| e.kind == GraphEdgeKind::Route));
    }

    #[test]
    fn expansion_pages_routes_with_next_hop_context_and_stable_order() {
        let paths = paths(1500);
        let p = project(
            "local",
            None,
            &[],
            &paths,
            &[interface()],
            &[],
            &[],
            view("iface:tcp-id", "", 1),
        );
        assert_eq!(p.shown, PAGE_SIZE);
        assert_eq!(p.pages, 94);
        assert!(p.nodes.iter().any(|n| n.id == "destination-00016"));
        assert!(!p.nodes.iter().any(|n| n.id == "destination-00000"));
        let relay = p.nodes.iter().position(|n| n.id == "relay").expect("next hop context");
        assert_eq!(
            p.edges.iter().filter(|e| e.source == relay && e.kind == GraphEdgeKind::Route).count(),
            PAGE_SIZE
        );
        let reversed: Vec<_> = paths.into_iter().rev().collect();
        assert_eq!(
            p,
            project(
                "local",
                None,
                &[],
                &reversed,
                &[interface()],
                &[],
                &[],
                view("iface:tcp-id", "", 1)
            )
        );
    }

    #[test]
    fn search_reveals_collapsed_destination_without_fabricating_local_route() {
        let p = project(
            "local",
            None,
            &[],
            &paths(1500),
            &[],
            &[],
            &[],
            view("", "destination-01499", 0),
        );
        assert_eq!(p.matches, 1);
        assert!(p.nodes.iter().any(|n| n.id == "destination-01499"));
        assert!(!p.edges.iter().any(|e| e.source == 0));
        assert_eq!(p.groups[0].id, "unattributed");
    }

    #[test]
    fn stale_routes_and_offline_discovery_are_counted_but_not_rendered() {
        let mut paths = paths(3);
        paths[0].observation.stale = true;
        let peer = PeerEntry {
            hash: "offline".into(),
            identity_hash: None,
            name: None,
            status: "offline".into(),
            node_role: crate::state::PeerRole::Rns,
            capabilities: Vec::new(),
            version: None,
            last_announce: None,
            announce_count: 0,
        };
        let p = project("local", None, &[peer], &paths, &[], &[], &[], view("", "", 0));
        assert_eq!(p.stale_routes, 1);
        assert_eq!(p.offline_peers, 1);
        assert_eq!(p.destinations, 2);
        assert!(!p.nodes.iter().any(|n| n.id == "offline"));
    }

    #[test]
    fn render_budget_covers_large_interface_and_link_inventories() {
        let interfaces: Vec<_> = (0..200)
            .map(|i| {
                let mut value = interface();
                value.hash = format!("iface-{i}");
                value.name = format!("iface-{i}");
                value
            })
            .collect();
        let links: Vec<_> = (0..200)
            .map(|i| LinkInfo {
                link_id: format!("link-{i}"),
                peer_hash: format!("peer-{i}"),
                status: "active".into(),
                activity: styrene_ipc::types::LinkActivity::Active,
                rtt_ms: None,
                timestamp: 0,
                observation: Default::default(),
            })
            .collect();
        let mut paths = paths(1500);
        for (i, path) in paths.iter_mut().enumerate() {
            path.interface = format!("iface-{}", i % 200);
            path.next_hop = format!("relay-{i}");
        }
        let p = project(
            "local",
            None,
            &[],
            &paths,
            &interfaces,
            &links,
            &[],
            view("", "destination", 0),
        );
        assert!(p.nodes.len() <= NODE_LIMIT);
        assert!(p.edges.len() <= EDGE_LIMIT);
        assert_eq!(p.omitted_interfaces, 184);
        assert_eq!(p.omitted_links, 192);
        assert_eq!(p.groups.len(), 200);
        assert_eq!(p.destinations, 1500);
        assert!(p.edges.iter().all(|e| e.source < p.nodes.len() && e.target < p.nodes.len()));
    }
}
