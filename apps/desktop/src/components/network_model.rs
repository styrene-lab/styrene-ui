use std::collections::HashMap;

use crate::state::{
    GraphEdge, GraphEdgeKind, GraphNode, GraphNodeType, InterfaceInfo, LinkInfo, PathEntry,
    PeerEntry, PeerRole,
};

use super::network_layout;

pub(crate) fn build_graph(
    local_hash: &str,
    local_name: Option<&str>,
    peers: &[PeerEntry],
    paths: &[PathEntry],
    interfaces: &[InterfaceInfo],
    links: &[LinkInfo],
    associations: &[(String, String)],
) -> (Vec<GraphNode>, Vec<GraphEdge>) {
    let mut nodes = Vec::with_capacity(1 + interfaces.len() + peers.len());
    let mut edges = Vec::new();
    let mut node_indices = HashMap::new();
    let mut interface_indices = HashMap::new();

    nodes.push(GraphNode {
        id: local_hash.to_string(),
        label: local_name.unwrap_or("Local Node").to_string(),
        node_type: GraphNodeType::Local,
        capabilities: Vec::new(),
        x: -250.0,
        y: 0.0,
        vx: 0.0,
        vy: 0.0,
    });
    node_indices.insert(local_hash.to_string(), 0);

    for (index, interface) in interfaces.iter().enumerate() {
        let node_index = nodes.len();
        nodes.push(GraphNode {
            id: format!("iface:{}", interface.hash),
            label: interface.name.clone(),
            node_type: GraphNodeType::Interface {
                online: matches!(
                    interface.status.as_str(),
                    "online" | "active" | "listening" | "connected"
                ),
            },
            capabilities: Vec::new(),
            x: -150.0,
            y: index as f64 * 70.0 - (interfaces.len().saturating_sub(1) as f64 * 35.0),
            vx: 0.0,
            vy: 0.0,
        });
        interface_indices.insert(interface.name.clone(), node_index);
        edges.push(GraphEdge {
            source: 0,
            target: node_index,
            hops: 0,
            kind: GraphEdgeKind::Interface,
        });
    }

    let peer_count = peers.len().max(1);
    for (index, peer) in peers.iter().enumerate() {
        let fraction = index as f64 / peer_count as f64;
        let angle = (fraction - 0.5) * std::f64::consts::PI * 1.6;
        let online = peer.status != "offline" && !peer.status.is_empty();
        let (x, y) = initial_peer_position(peer, angle);
        let node_type = if peer.status.is_empty() || peer.status == "unknown" {
            GraphNodeType::Unknown
        } else {
            match &peer.node_role {
                PeerRole::Hub => GraphNodeType::Hub { online },
                PeerRole::PageHost => GraphNodeType::PageHost { online },
                PeerRole::Styrene => GraphNodeType::Styrene { online },
                PeerRole::Rns => GraphNodeType::Rns { online },
            }
        };
        let node_index = nodes.len();
        nodes.push(GraphNode {
            id: peer.hash.clone(),
            label: peer
                .name
                .clone()
                .unwrap_or_else(|| peer.hash[..8.min(peer.hash.len())].to_string()),
            node_type,
            capabilities: peer.capabilities.clone(),
            x,
            y,
            vx: 0.0,
            vy: 0.0,
        });
        node_indices.insert(peer.hash.clone(), node_index);
    }

    // Routes and links are independent observations; discovery is not required.
    for hash in paths
        .iter()
        .flat_map(|path| [&path.destination_hash, &path.next_hop])
        .chain(links.iter().map(|link| &link.peer_hash))
    {
        if hash.is_empty() || node_indices.contains_key(hash) {
            continue;
        }
        let index = nodes.len();
        let slot = index.saturating_sub(1 + interfaces.len() + peers.len()) as f64;
        let angle = slot * 2.399963229728653;
        let radius = 80.0 * (slot + 1.0).sqrt();
        nodes.push(GraphNode {
            id: hash.clone(),
            label: hash.chars().take(8).collect(),
            node_type: GraphNodeType::Unknown,
            capabilities: Vec::new(),
            x: 450.0 + radius * angle.cos(),
            y: radius * angle.sin(),
            vx: 0.0,
            vy: 0.0,
        });
        node_indices.insert(hash.clone(), index);
    }

    for path in paths.iter().filter(|path| !path.observation.stale) {
        let Some(&target) = node_indices.get(&path.destination_hash) else {
            continue;
        };
        let source = if path.next_hop != path.destination_hash {
            node_indices.get(&path.next_hop).copied()
        } else {
            interface_indices
                .get(&path.interface)
                .copied()
                .or_else(|| {
                    (interfaces.len() == 1)
                        .then(|| interface_indices.values().next().copied())
                        .flatten()
                })
                .or(Some(0))
        };
        if let Some(source) = source.filter(|source| *source != target) {
            edges.push(GraphEdge { source, target, hops: path.hops, kind: GraphEdgeKind::Route });
        }
    }

    for link in links.iter().filter(|link| {
        !link.observation.stale && link.activity == styrene_ipc::types::LinkActivity::Active
    }) {
        if let Some(&target) = node_indices.get(&link.peer_hash) {
            edges.push(GraphEdge { source: 0, target, hops: 0, kind: GraphEdgeKind::Link });
        }
    }
    for (source_hash, target_hash) in associations {
        if let (Some(&source), Some(&target)) =
            (node_indices.get(source_hash), node_indices.get(target_hash))
            && source != target
        {
            edges.push(GraphEdge { source, target, hops: 0, kind: GraphEdgeKind::Association });
        }
    }

    let pinned: Vec<usize> =
        std::iter::once(0).chain(interface_indices.values().copied()).collect();
    if nodes.len() > 200 {
        // Large inventories need bounded two-dimensional geometry, not a column
        // whose height grows linearly with the number of route-only endpoints.
        network_layout::compact_layout(&mut nodes, &pinned);
    } else {
        network_layout::settle_layout(&mut nodes, &edges, &pinned);
    }
    (nodes, edges)
}

pub(crate) fn preserve_positions(previous: &[GraphNode], next: &mut [GraphNode]) {
    let positions: HashMap<&str, (f64, f64, f64, f64)> = previous
        .iter()
        .map(|node| (node.id.as_str(), (node.x, node.y, node.vx, node.vy)))
        .collect();
    for node in next {
        if let Some((x, y, velocity_x, velocity_y)) = positions.get(node.id.as_str()) {
            node.x = *x;
            node.y = *y;
            node.vx = *velocity_x;
            node.vy = *velocity_y;
        }
    }
}

fn initial_peer_position(peer: &PeerEntry, angle: f64) -> (f64, f64) {
    match &peer.node_role {
        PeerRole::Hub => (0.0, 0.0),
        PeerRole::PageHost => (100.0 * angle.cos() + 50.0, 100.0 * angle.sin()),
        PeerRole::Styrene => (140.0 * angle.cos() + 60.0, 140.0 * angle.sin()),
        PeerRole::Rns if peer.name.is_some() => (180.0 * angle.cos() + 70.0, 180.0 * angle.sin()),
        PeerRole::Rns => (250.0 * angle.cos() + 80.0, 250.0 * angle.sin()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};

    const HIGH_CARDINALITY_BUILD_BUDGET: Duration = Duration::from_secs(2);

    fn peer(hash: &str) -> PeerEntry {
        PeerEntry {
            hash: hash.into(),
            identity_hash: None,
            name: Some("Observed peer".into()),
            status: "online".into(),
            node_role: PeerRole::Rns,
            capabilities: Vec::new(),
            version: None,
            last_announce: None,
            announce_count: 1,
        }
    }

    #[test]
    fn undiscovered_destination_remains_visible_with_unknown_status() {
        let path = PathEntry {
            destination_hash: "unheard".into(),
            hops: 1,
            next_hop: "unheard".into(),
            interface: "".into(),
            expires: None,
            observation: Default::default(),
        };
        let (nodes, edges) = build_graph("local", None, &[], &[path], &[], &[], &[]);
        let unknown = nodes.iter().find(|node| node.id == "unheard").expect("observed destination");
        assert_eq!(unknown.node_type, GraphNodeType::Unknown);
        assert_eq!(unknown.type_label(), "Destination (status unknown)");
        assert_eq!(edges.iter().filter(|edge| edge.kind == GraphEdgeKind::Route).count(), 1);
    }

    #[test]
    fn discovery_does_not_imply_a_route_or_link() {
        let (_, edges) = build_graph("local", None, &[peer("peer")], &[], &[], &[], &[]);
        assert!(edges.is_empty());
    }

    #[test]
    fn graph_edges_preserve_observation_semantics() {
        let mut interface = InterfaceInfo::default();
        interface.name = "Fixture TCP".into();
        interface.hash = "iface".into();
        interface.status = "online".into();
        let path = PathEntry {
            destination_hash: "peer".into(),
            hops: 1,
            next_hop: "peer".into(),
            interface: "Fixture TCP".into(),
            expires: None,
            observation: Default::default(),
        };
        let link = LinkInfo {
            link_id: "link".into(),
            peer_hash: "peer".into(),
            status: "active".into(),
            activity: styrene_ipc::types::LinkActivity::Active,
            rtt_ms: Some(12.0),
            timestamp: 1,
            observation: Default::default(),
        };
        let (_, edges) = build_graph(
            "local",
            None,
            &[peer("peer"), peer("associated")],
            &[path],
            &[interface],
            &[link],
            &[("peer".into(), "associated".into())],
        );
        assert!(edges.iter().any(|edge| edge.kind == GraphEdgeKind::Interface));
        assert!(edges.iter().any(|edge| edge.kind == GraphEdgeKind::Route));
        assert!(edges.iter().any(|edge| edge.kind == GraphEdgeKind::Link));
        assert!(edges.iter().any(|edge| edge.kind == GraphEdgeKind::Association));
    }

    #[test]
    fn reconciliation_preserves_positions_for_existing_nodes() {
        let (mut previous, _) = build_graph("local", None, &[peer("peer")], &[], &[], &[], &[]);
        previous[1].x = 321.0;
        previous[1].y = 123.0;
        let mut updated_peer = peer("peer");
        updated_peer.status = "offline".into();
        let (mut next, _) = build_graph("local", None, &[updated_peer], &[], &[], &[], &[]);
        preserve_positions(&previous, &mut next);
        assert_eq!((next[1].x, next[1].y), (321.0, 123.0));
        assert!(matches!(next[1].node_type, GraphNodeType::Rns { online: false }));
    }

    #[test]
    fn large_route_inventory_is_compact_and_preserves_all_destinations() {
        let paths: Vec<_> = (0..1000)
            .map(|index| {
                let hash = format!("{index:032x}");
                PathEntry {
                    destination_hash: hash.clone(),
                    next_hop: hash,
                    hops: 1,
                    interface: String::new(),
                    expires: None,
                    observation: Default::default(),
                }
            })
            .collect();
        let (nodes, edges) = build_graph("local", None, &[], &paths, &[], &[], &[]);
        assert_eq!(nodes.len(), 1001);
        assert_eq!(edges.len(), 1000);
        let width = nodes.iter().map(|n| n.x).fold(f64::NEG_INFINITY, f64::max)
            - nodes.iter().map(|n| n.x).fold(f64::INFINITY, f64::min);
        let height = nodes.iter().map(|n| n.y).fold(f64::NEG_INFINITY, f64::max)
            - nodes.iter().map(|n| n.y).fold(f64::INFINITY, f64::min);
        assert!(width < 4000.0 && height < 4000.0);
        assert!(width / height > 0.5 && width / height < 2.0);
    }

    #[test]
    fn high_cardinality_model_has_bounded_shape() {
        let peers: Vec<_> = (0..500).map(|index| peer(&format!("{index:032x}"))).collect();
        let started = Instant::now();
        let (nodes, edges) = build_graph("local", None, &peers, &[], &[], &[], &[]);
        let elapsed = started.elapsed();
        assert_eq!(nodes.len(), 501);
        assert!(edges.is_empty());
        assert!(
            elapsed <= HIGH_CARDINALITY_BUILD_BUDGET,
            "500-peer topology build took {elapsed:?}, budget is {HIGH_CARDINALITY_BUILD_BUDGET:?}"
        );
    }
}
