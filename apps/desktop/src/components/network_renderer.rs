use dioxus::prelude::*;

use crate::state::{GraphEdge, GraphEdgeKind, GraphNode, GraphNodeType};

use super::network_interaction::Interaction;

pub(crate) type GraphTooltip = (f64, f64, String, String, String, String);

#[component]
pub(crate) fn GraphRenderer(
    nodes: Signal<Vec<GraphNode>>,
    edges: Signal<Vec<GraphEdge>>,
    visible: Vec<bool>,
    mut selected: Signal<Option<usize>>,
    selected_neighbors: Vec<usize>,
    search_matches: Vec<usize>,
    has_search: bool,
    zoom: f64,
    viewbox: String,
    interaction: Signal<Interaction>,
    mut tooltip: Signal<Option<GraphTooltip>>,
) -> Element {
    let mut route_sources = std::collections::HashMap::<usize, usize>::new();
    for edge in edges.read().iter().filter(|edge| edge.kind == GraphEdgeKind::Route) {
        *route_sources.entry(edge.source).or_default() += 1;
    }
    let route_sources: std::collections::HashSet<_> = route_sources
        .into_iter()
        .filter_map(|(index, count)| (count >= 8).then_some(index))
        .collect();
    let show_all_labels = zoom > 1.5;
    let show_named_labels = zoom > 0.6;
    rsx! {
        svg {
            view_box: "{viewbox}",
            xmlns: "http://www.w3.org/2000/svg",
            role: "group",
            "aria-label": "Interactive mesh topology. Tab to a node and press Enter or Space to inspect it.",
            for edge in edges.read().iter() {
                {
                    let source_visible = edge.source < visible.len() && visible[edge.source];
                    let target_visible = edge.target < visible.len() && visible[edge.target];
                    if !source_visible || !target_visible {
                        rsx! {}
                    } else {
                        let selected_index = *selected.read();
                        let nodes = nodes.read();
                        let structural = edge.source < nodes.len() && edge.target < nodes.len() && {
                            let source = &nodes[edge.source].node_type;
                            let target = &nodes[edge.target].node_type;
                            matches!(source, GraphNodeType::Local | GraphNodeType::Interface { .. } | GraphNodeType::Hub { .. })
                                || matches!(target, GraphNodeType::Local | GraphNodeType::Interface { .. } | GraphNodeType::Hub { .. })
                        };
                        let show = structural || route_sources.contains(&edge.source) || match selected_index {
                            Some(index) => edge.source == index || edge.target == index,
                            None => zoom > 0.8,
                        };
                        if show {
                            let source = &nodes[edge.source];
                            let target = &nodes[edge.target];
                            let kind_class = match edge.kind {
                                GraphEdgeKind::Interface => "interface",
                                GraphEdgeKind::Route => "route",
                                GraphEdgeKind::Link => "link",
                                GraphEdgeKind::Association => "association",
                            };
                            let class = format!("graph-edge {kind_class}");
                            rsx! {
                                line {
                                    class: "{class}",
                                    vector_effect: "non-scaling-stroke",

                                    x1: "{source.x}",
                                    y1: "{source.y}",
                                    x2: "{target.x}",
                                    y2: "{target.y}",
                                }
                                if edge.kind == GraphEdgeKind::Link {
                                    line { class: "graph-link-core", vector_effect: "non-scaling-stroke",
                                        x1: "{source.x}", y1: "{source.y}", x2: "{target.x}", y2: "{target.y}" }
                                }
                            }
                        } else {
                            rsx! {}
                        }
                    }
                }
            }
            {
                let nodes = nodes.read();
                let selected_index = *selected.read();
                rsx! {
                    for (index, node) in nodes.iter().enumerate() {
                        if index < visible.len() && visible[index] {
                            {
                                let color = node.color();
                                let border = node.border_color();
                                let radius = node.radius();
                                let x = node.x;
                                let y = node.y;
                                let label = node.label.lines().next().unwrap_or_default().to_string();
                                let summary_detail = node.label.lines().nth(1).unwrap_or_default().to_string();
                                let local = node.node_type == GraphNodeType::Local;
                                let anchor = local || matches!(node.node_type, GraphNodeType::Interface { .. } | GraphNodeType::Summary | GraphNodeType::Endpoint) || route_sources.contains(&index);
                                let label_offset = radius + if nodes.len() <= 100 || anchor { 14.0 / zoom.max(0.01) } else { 14.0 };
                                let label_style = if nodes.len() <= 100 || anchor { format!("font-size: {}px; paint-order: stroke; stroke: #0e1319; stroke-width: {}px;", 12.0 / zoom.max(0.01), 3.0 / zoom.max(0.01)) } else { String::new() };
                                let custom_path = node.shape_path(x, y);
                                let tooltip_label = label.clone();
                                let tooltip_hash = node.id.clone();
                                let tooltip_type = node.type_label().to_string();
                                let tooltip_capabilities = node.capabilities.join(", ");
                                let status_label = node.status_label();
                                let accessible_label = format!("{}; {}; {}", node.label, node.type_label(), status_label);
                                let icon = node.role_badge();
                                let unknown = node.node_type == GraphNodeType::Unknown;
                                let is_selected = selected_index == Some(index);
                                let neighbor = selected_index.is_some() && selected_neighbors.contains(&index);
                                let search_match = has_search && search_matches.contains(&index);
                                let dragging = matches!(*interaction.read(), Interaction::DraggingNode { idx, .. } if idx == index);
                                let opacity = if dragging {
                                    "0.85"
                                } else if has_search && !search_match {
                                    "0.15"
                                } else if selected_index.is_some() && !is_selected && !neighbor && !local {
                                    "0.25"
                                } else {
                                    "1"
                                };
                                let show_label = nodes.len() <= 100 || anchor || is_selected
                                    || neighbor
                                    || search_match
                                    || show_all_labels
                                    || (show_named_labels && is_notable(node))
                                    || (show_named_labels && node.name_is_set());
                                rsx! {
                                    g {
                                        class: "graph-node",
                                        opacity: "{opacity}",
                                        role: "button",
                                        tabindex: "0",
                                        "aria-label": "{accessible_label}",
                                        "aria-pressed": if is_selected { "true" } else { "false" },
                                        onkeydown: move |event: KeyboardEvent| {
                                            let key = event.key();
                                            if key == Key::Enter
                                                || matches!(key, Key::Character(value) if value == " ")
                                            {
                                                selected.set(Some(index));
                                            }
                                        },
                                        onmouseenter: move |event: MouseEvent| {
                                            if *interaction.read() == Interaction::None {
                                                let coordinates = event.client_coordinates();
                                                tooltip.set(Some((
                                                    coordinates.x,
                                                    coordinates.y,
                                                    tooltip_label.clone(),
                                                    tooltip_hash.clone(),
                                                    tooltip_type.clone(),
                                                    tooltip_capabilities.clone(),
                                                )));
                                            }
                                        },
                                        onmouseleave: move |_| {
                                            if *interaction.read() == Interaction::None {
                                                tooltip.set(None);
                                            }
                                        },
                                        if is_selected {
                                            circle {
                                                cx: "{x}", cy: "{y}", r: "{radius + 6.0}", fill: "none",
                                                stroke: "#f0f6fc",
                                                stroke_opacity: "1",
                                                stroke_width: "2",
                                                vector_effect: "non-scaling-stroke",
                                            }
                                        }
                                        if let Some(ref path) = custom_path {
                                            path {
                                                d: "{path}", fill: "{color}",
                                                stroke: "{border}",
                                                stroke_dasharray: if unknown { "3 3" } else { "none" },
                                                stroke_width: "2",
                                                vector_effect: "non-scaling-stroke",
                                            }
                                        } else {
                                            circle {
                                                cx: "{x}", cy: "{y}", r: "{radius}", fill: "{color}",
                                                stroke: "{border}",
                                                stroke_dasharray: if unknown { "3 3" } else { "none" },
                                                stroke_width: "2",
                                                vector_effect: "non-scaling-stroke",
                                            }
                                        }
                                        if let Some(glyph) = icon {
                                            rect { x: "{x + 6.0}", y: "{y - 17.0}", width: "14", height: "14", rx: "3",
                                                fill: "#171c22", stroke: "#9aa8b8", stroke_width: "1", vector_effect: "non-scaling-stroke" }
                                            text {
                                                x: "{x + 13.0}", y: "{y - 7.0}", text_anchor: "middle",
                                                fill: "white", font_size: "10", font_weight: "bold",
                                                pointer_events: "none", "{glyph}"
                                            }
                                        }
                                        if show_label {
                                            text { class: "graph-label", style:"{label_style}", x: "{x}", y: "{y + label_offset}", "{label}" }
                                            if !summary_detail.is_empty() {
                                                text { class: "graph-label", style:"{label_style}", x: "{x}", y: "{y + label_offset + 18.0 / zoom.max(0.01)}", "{summary_detail}" }
                                            }
                                            if summary_detail.is_empty() {
                                                text { class: "graph-label graph-status-label", style:"{label_style}", x: "{x}", y: "{y + label_offset + 18.0 / zoom.max(0.01)}", "{status_label}" }
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
}

fn is_notable(node: &GraphNode) -> bool {
    matches!(
        node.node_type,
        GraphNodeType::Local
            | GraphNodeType::Interface { .. }
            | GraphNodeType::Hub { .. }
            | GraphNodeType::PageHost { .. }
    )
}

#[component]
pub(crate) fn MapLegend() -> Element {
    let samples = [
        ("This device", GraphNodeType::Local),
        ("Interface", GraphNodeType::Interface { online: false }),
        ("TCP endpoint", GraphNodeType::Endpoint),
        ("Destination", GraphNodeType::Unknown),
        ("Group", GraphNodeType::Summary),
    ];
    rsx! {
        details { class: "network-symbol-key",
            summary { "Map key · symbols and states" }
            div { class: "network-key-row",
                for (label, kind) in samples {
                    {
                        let node = GraphNode { id: String::new(), label: String::new(), node_type: kind, capabilities: Vec::new(), x: 0.0, y: 0.0, vx: 0.0, vy: 0.0 };
                        rsx! { span { class: "network-key-item",
                            svg { view_box: "-18 -16 36 32", "aria-hidden": "true",
                                if let Some(path) = node.shape_path(0.0, 0.0) {
                                    path { d: "{path}", fill: "#171c22", stroke: "#9aa8b8", stroke_width: "2" }
                                } else { circle { r: "11", fill: "#171c22", stroke: "#9aa8b8", stroke_width: "2" } }
                            }
                            "{label}"
                        } }
                    }
                }
            }
            div { class: "network-key-row",
                for (class, label) in [("interface", "Attachment"), ("route", "Observed route"), ("link", "Active link"), ("association", "Group membership")] {
                    span { class: "network-key-item", span { class: "network-key-line {class}" } "{label}" }
                }
            }
            p { "Green outline: active / reported online. Gray: offline, unavailable, or no status applicable. Dashed destination outline: unknown status. White outer ring: selected." }
            p { "Role badges: H Hub · P Page host · S Styrene · R RNS. Lines describe relationships; they do not indicate packet flow." }
        }
    }
}
