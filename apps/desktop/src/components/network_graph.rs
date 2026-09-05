//! Combined topology coordinator. Model, layout, rendering, interaction, filters,
//! and inspection live behind separate component/module boundaries.

use dioxus::prelude::*;

use crate::state::{GraphEdge, GraphNode, GraphNodeType, MeshStatusInfo, PathEntry, PeerEntry};

use super::network_inspector::NetworkInspector;
use super::network_interaction::{self, Interaction};
use super::network_renderer::{GraphRenderer, GraphTooltip};
use super::{network_layout, network_model};

#[component]
pub fn NetworkGraph(
    peers: Vec<PeerEntry>,
    inspection: Option<EventHandler<String>>,
    search: String,
    paths: Vec<PathEntry>,
    status: MeshStatusInfo,
    local_hash: String,
    local_name: Option<String>,
    on_select_peer: EventHandler<String>,
    on_browse_page: EventHandler<String>,
    links: Vec<crate::state::LinkInfo>,
    interfaces: Vec<crate::state::InterfaceInfo>,
    interface_names: Vec<(String, String)>,
) -> Element {
    let mut nodes = use_signal(Vec::<GraphNode>::new);
    let mut edges = use_signal(Vec::<GraphEdge>::new);
    let mut selected = use_signal(|| None::<usize>);
    let mut model_key = use_signal(String::new);
    let mut auto_fit = use_signal(|| true);
    let current_key = format!(
        "{local_hash}|{local_name:?}|{interface_names:?}|{}|{:?}|{:?}|{:?}",
        peer_key(&peers),
        paths
            .iter()
            .map(|p| (&p.destination_hash, &p.next_hop, &p.interface, p.hops, p.observation.stale))
            .collect::<Vec<_>>(),
        interfaces
            .iter()
            .map(|i| (&i.hash, &i.name, &i.status, &i.parent_hash))
            .collect::<Vec<_>>(),
        links.iter().map(|l| (&l.peer_hash, &l.activity, l.observation.stale)).collect::<Vec<_>>()
    );
    if *model_key.read() != current_key {
        let (mut next_nodes, next_edges) = network_model::build_graph(
            &local_hash,
            local_name.as_deref(),
            &peers,
            &paths,
            &interfaces,
            &links,
            &[],
        );
        for node in &mut next_nodes {
            if let Some(hash) = node.id.strip_prefix("iface:")
                && let Some((_, name)) = interface_names.iter().find(|(id, _)| id == hash)
            {
                node.label = name.clone();
            }
        }
        let selected_id =
            selected.read().and_then(|index| nodes.read().get(index).map(|node| node.id.clone()));
        if (nodes.read().len() > 200) == (next_nodes.len() > 200) {
            let mut degree = vec![0_usize; next_nodes.len()];
            for edge in &next_edges {
                if edge.kind == crate::state::GraphEdgeKind::Route {
                    degree[edge.source] += 1;
                }
            }
            let anchors: Vec<_> = next_nodes
                .iter()
                .enumerate()
                .filter(|(index, node)| {
                    matches!(node.node_type, GraphNodeType::Local | GraphNodeType::Interface { .. })
                        || degree[*index] >= 8
                })
                .map(|(index, node)| (index, node.x, node.y))
                .collect();
            network_model::preserve_positions(&nodes.read(), &mut next_nodes);
            // While following discovery, reserve label space for anchors that
            // arrive after the initial peer snapshot. Manual layouts stay put.
            if *auto_fit.read() && next_nodes.len() > 200 {
                for (index, x, y) in anchors {
                    next_nodes[index].x = x;
                    next_nodes[index].y = y;
                }
            }
        }
        selected.set(selected_id.and_then(|id| next_nodes.iter().position(|node| node.id == id)));
        nodes.set(next_nodes);
        edges.set(next_edges);
        model_key.set(current_key);
    }

    let initial_camera = fit_camera(&nodes.read(), (800.0, 600.0));
    let mut camera_x = use_signal(move || initial_camera.0);
    let mut camera_y = use_signal(move || initial_camera.1);
    let mut zoom = use_signal(move || initial_camera.2);
    let mut interaction = use_signal(|| Interaction::None);
    let mut container_size = use_signal(|| (800.0_f64, 600.0_f64));
    use_effect(move || {
        let _ = model_key();
        let size = container_size();
        if auto_fit() {
            let (x, y, scale) = fit_camera(&nodes.peek(), size);
            camera_x.set(x);
            camera_y.set(y);
            zoom.set(scale);
        }
    });
    let mut tooltip = use_signal(|| None::<GraphTooltip>);
    let mut physics_on = use_signal(initial_physics_enabled);
    let search_query = use_signal(String::new);
    let show_all = use_signal(|| true);

    let visible: Vec<bool> =
        nodes.read().iter().map(|node| *show_all.read() || is_interesting(node)).collect();
    let hidden_count = visible.iter().filter(|visible| !**visible).count();

    let _physics = use_coroutine(move |_rx: UnboundedReceiver<()>| async move {
        loop {
            tokio::time::sleep(tokio::time::Duration::from_millis(33)).await;
            if !*physics_on.read() {
                continue;
            }
            let mut pinned = vec![0];
            if let Interaction::DraggingNode { idx, .. } = *interaction.read()
                && !pinned.contains(&idx)
            {
                pinned.push(idx);
            }
            let edge_snapshot = edges.read().clone();
            let mut node_snapshot = nodes.read().clone();
            if node_snapshot.is_empty() {
                continue;
            }
            network_layout::force_tick(&mut node_snapshot, &edge_snapshot, &pinned);
            if network_layout::kinetic_energy(&node_snapshot) < 0.01
                && *interaction.read() == Interaction::None
            {
                continue;
            }
            nodes.set(node_snapshot);
        }
    });

    let zoom_value = *zoom.read();
    let width = container_size.read().0 / zoom_value;
    let height = container_size.read().1 / zoom_value;
    let viewbox = format!(
        "{} {} {width} {height}",
        *camera_x.read() - width / 2.0,
        *camera_y.read() - height / 2.0
    );
    let pixel_to_world = {
        let (container_width, container_height) = *container_size.read();
        (width / container_width.max(1.0), height / container_height.max(1.0))
    };

    let selected_neighbors: Vec<usize> = selected
        .read()
        .and_then(|selected| {
            Some(
                edges
                    .read()
                    .iter()
                    .filter_map(|edge| {
                        if edge.source == selected {
                            Some(edge.target)
                        } else if edge.target == selected {
                            Some(edge.source)
                        } else {
                            None
                        }
                    })
                    .collect(),
            )
        })
        .unwrap_or_default();
    let query = if inspection.is_some() {
        search.to_ascii_lowercase()
    } else {
        search_query.read().to_ascii_lowercase()
    };
    let search_matches: Vec<usize> = if query.is_empty() {
        Vec::new()
    } else {
        nodes
            .read()
            .iter()
            .enumerate()
            .filter(|(_, node)| {
                node.label.to_ascii_lowercase().contains(&query)
                    || node.id.to_ascii_lowercase().contains(&query)
            })
            .map(|(index, _)| index)
            .collect()
    };
    let focus_query = query.clone();
    let selected_detail = selected.read().and_then(|index| nodes.read().get(index).cloned());

    use_effect(move || {
        if let Some(callback) = inspection
            && let Some(index) = *selected.read()
            && let Some(node) = nodes.read().get(index)
        {
            callback.call(node.id.clone());
        }
    });

    rsx! {
        div { class: "network-view",
            div { class: "network-map-tools",
                button {
                    onclick: move |_| {
                        auto_fit.set(true);
                        let (x, y, scale) = fit_camera(&nodes.read(), *container_size.read());
                        camera_x.set(x);
                        camera_y.set(y);
                        zoom.set(scale);
                    },
                    "Fit to view"
                }
                if !search_matches.is_empty() {
                    button { onclick: move |_| {
                        auto_fit.set(false);
                        let matching: Vec<_> = nodes.read().iter().filter(|n| n.label.to_ascii_lowercase().contains(&focus_query) || n.id.to_ascii_lowercase().contains(&focus_query)).cloned().collect();
                        let (x,y,scale) = fit_camera(&matching, *container_size.read());
                        camera_x.set(x);camera_y.set(y);zoom.set(scale);
                    }, "Focus matches" }
                }
                span { class:"network-map-summary", "{nodes.read().len()} nodes · {edges.read().len()} connections" }
                button {
                    aria_label: "Zoom in",
                    onclick: move |_| {
                        auto_fit.set(false);
                        let value = *zoom.read();
                        zoom.set((value * 1.25).min(5.0));
                    },
                    "+"
                }
                button {
                    aria_label: "Zoom out",
                    onclick: move |_| {
                        auto_fit.set(false);
                        let value = *zoom.read();
                        zoom.set((value / 1.25).max(0.001));
                    },
                    "−"
                }
                details {
                    summary { "Map settings" }
                    button {
                        aria_pressed: "{physics_on}",
                        onclick: move |_| physics_on.toggle(),
                        if *physics_on.read() {
                            "Pause animation"
                        } else {
                            "Animate layout"
                        }
                    }
                }
                span { class: "network-map-legend", "Interface —   Route ···   Link ═" }
            }
            div {
                class: "graph-container",
                onmounted: move |event| {
                    let data = event.data();
                    spawn(async move {
                        if let Ok(rect) = data.get_client_rect().await {
                            container_size.set((rect.size.width, rect.size.height));
                            let (x, y, scale) = fit_camera(&nodes.read(), (rect.size.width, rect.size.height));
                            camera_x.set(x); camera_y.set(y); zoom.set(scale);
                        }
                    });
                },
                onmousedown: move |event: MouseEvent| {
                    auto_fit.set(false);
                    let client = event.client_coordinates();
                    let element = event.element_coordinates();
                    let (container_width, container_height) = *container_size.read();
                    let zoom_value = *zoom.read();
                    let width = container_size.read().0 / zoom_value;
                    let height = container_size.read().1 / zoom_value;
                    let world_x = (*camera_x.read() - width / 2.0)
                        + (element.x / container_width.max(1.0)) * width;
                    let world_y = (*camera_y.read() - height / 2.0)
                        + (element.y / container_height.max(1.0)) * height;
                    if let Some(index) = network_interaction::hit_test(
                        &nodes.read(),
                        world_x,
                        world_y,
                    ) {
                        selected.set(Some(index));
                        interaction
                            .set(Interaction::DraggingNode {
                                idx: index,
                                last_x: client.x,
                                last_y: client.y,
                            });
                    } else {
                        selected.set(None);
                        interaction
                            .set(Interaction::Panning {
                                last_x: client.x,
                                last_y: client.y,
                            });
                    }
                    tooltip.set(None);
                },
                onmousemove: move |event: MouseEvent| {
                    let client = event.client_coordinates();
                    let current_interaction = interaction.read().clone();
                    match current_interaction {
                        Interaction::Panning { last_x, last_y } => {
                            camera_x -= (client.x - last_x) * pixel_to_world.0;
                            camera_y -= (client.y - last_y) * pixel_to_world.1;
                            interaction
                                .set(Interaction::Panning {
                                    last_x: client.x,
                                    last_y: client.y,
                                });
                        }
                        Interaction::DraggingNode { idx, last_x, last_y } => {
                            if let Some(node) = nodes.write().get_mut(idx) {
                                node.x += (client.x - last_x) * pixel_to_world.0;
                                node.y += (client.y - last_y) * pixel_to_world.1;
                                node.vx = 0.0;
                                node.vy = 0.0;
                            }
                            interaction
                                .set(Interaction::DraggingNode {
                                    idx,
                                    last_x: client.x,
                                    last_y: client.y,
                                });
                        }
                        Interaction::None => {}
                    }
                },
                onmouseup: move |_| interaction.set(Interaction::None),
                onmouseleave: move |_| {
                    interaction.set(Interaction::None);
                    tooltip.set(None);
                },
                onwheel: move |event: WheelEvent| {
                    auto_fit.set(false);
                    let factor = if event.delta().strip_units().y < 0.0 { 1.1 } else { 1.0 / 1.1 };
                    let current_zoom = *zoom.read();
                    zoom.set((current_zoom * factor).clamp(0.001, 5.0));
                },
                GraphRenderer {
                    nodes,
                    edges,
                    visible,
                    selected,
                    selected_neighbors,
                    search_matches: search_matches.clone(),
                    has_search: !query.is_empty(),
                    zoom: zoom_value,
                    viewbox,
                    interaction,
                    tooltip,
                }
                if let Some((x, y, label, hash, status, capabilities)) = tooltip.read().clone() {
                    div {
                        class: "graph-tooltip",
                        style: "left: {x + 12.0}px; top: {y - 10.0}px;",
                        div { class: "tt-label", "{label}" }
                        div { class: "tt-hash", "{hash}" }
                        div { class: "tt-status", "{status}" }
                        if !capabilities.is_empty() {
                            div { class: "tt-caps", "{capabilities}" }
                        }
                    }
                }
            }
            if inspection.is_none() {
                NetworkInspector {
                    selected: selected_detail,
                    peers,
                    links,
                    status,
                    search_query,
                    search_count: search_matches.len(),
                    hidden_count,
                    show_all,
                    physics_on,
                    nodes,
                    on_select_peer,
                    on_browse_page,
                }
            }
        }
    }
}

fn fit_camera(nodes: &[GraphNode], size: (f64, f64)) -> (f64, f64, f64) {
    if nodes.is_empty() {
        return (0.0, 0.0, 1.0);
    }
    let (mut left, mut right, mut top, mut bottom) =
        (f64::INFINITY, f64::NEG_INFINITY, f64::INFINITY, f64::NEG_INFINITY);
    for node in nodes {
        left = left.min(node.x);
        right = right.max(node.x);
        top = top.min(node.y);
        bottom = bottom.max(node.y);
    }
    let zoom =
        (size.0 / (right - left + 180.0)).min(size.1 / (bottom - top + 160.0)).clamp(0.000001, 2.0);
    ((left + right) / 2.0, (top + bottom) / 2.0, zoom)
}

const fn initial_physics_enabled() -> bool {
    // Start static so platform reduced-motion preferences are honored without script detection.
    false
}

fn peer_key(peers: &[PeerEntry]) -> String {
    let mut hashes: Vec<String> = peers
        .iter()
        .map(|peer| {
            format!(
                "{}:{:?}:{}:{:?}:{:?}",
                peer.hash, peer.name, peer.status, peer.node_role, peer.capabilities
            )
        })
        .collect();
    hashes.sort_unstable();
    hashes.join(",")
}

fn is_interesting(node: &GraphNode) -> bool {
    match &node.node_type {
        GraphNodeType::Unknown
        | GraphNodeType::Local
        | GraphNodeType::Interface { .. }
        | GraphNodeType::Hub { .. } => true,
        GraphNodeType::PageHost { .. } | GraphNodeType::Styrene { .. } => true,
        GraphNodeType::Rns { .. } => node.name_is_set(),
    }
}

#[cfg(test)]
mod accessibility_tests {
    use super::*;

    #[test]
    fn fit_camera_encloses_even_extreme_route_extents() {
        let node = |x, y| GraphNode {
            id: "node".into(),
            label: String::new(),
            node_type: GraphNodeType::Unknown,
            capabilities: Vec::new(),
            x,
            y,
            vx: 0.0,
            vy: 0.0,
        };
        let nodes = vec![node(-250.0, 0.0), node(100.0, 50000.0)];
        let (x, y, zoom) = fit_camera(&nodes, (1200.0, 700.0));
        for node in nodes {
            assert!((node.x - x).abs() < 1200.0 / zoom / 2.0);
            assert!((node.y - y).abs() < 700.0 / zoom / 2.0);
        }
    }

    #[test]
    fn force_layout_requires_explicit_operator_opt_in() {
        assert!(!initial_physics_enabled());
    }
}
