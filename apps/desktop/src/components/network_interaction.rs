use crate::state::GraphNode;

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Interaction {
    None,
    Panning { last_x: f64, last_y: f64 },
    DraggingNode { idx: usize, last_x: f64, last_y: f64 },
}

/// Scale by gesture distance, not by the number of trackpad events.
pub(crate) fn wheel_zoom(current: f64, pixels: f64) -> f64 {
    if !pixels.is_finite() || pixels == 0.0 {
        return current;
    }
    (current * (-pixels.clamp(-80.0, 80.0) * 0.0015).exp()).clamp(0.001, 5.0)
}

pub(crate) fn hit_test(nodes: &[GraphNode], world_x: f64, world_y: f64) -> Option<usize> {
    for index in (0..nodes.len()).rev() {
        let node = &nodes[index];
        let dx = world_x - node.x;
        let dy = world_y - node.y;
        let radius = node.radius() + 8.0;
        if dx * dx + dy * dy <= radius * radius {
            return Some(index);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::GraphNodeType;

    #[test]
    fn trackpad_zoom_depends_on_distance_and_ignores_zero_motion() {
        let small_events = (0..40).fold(1.0, |zoom, _| wheel_zoom(zoom, 1.0));
        assert!((small_events - wheel_zoom(1.0, 40.0)).abs() < 1e-12);
        assert_eq!(wheel_zoom(1.0, 0.0), 1.0);
        assert_eq!(wheel_zoom(1.0, f64::NAN), 1.0);
        assert!((wheel_zoom(1.0, 1.0) - 1.0).abs() < 0.002);
        assert!(wheel_zoom(1.0, -10000.0) < 1.13);
    }

    #[test]
    fn hit_testing_prefers_topmost_node_and_respects_radius() {
        let node = |id: &str| GraphNode {
            id: id.into(),
            label: id.into(),
            node_type: GraphNodeType::Rns { online: true },
            capabilities: Vec::new(),
            x: 10.0,
            y: 10.0,
            vx: 0.0,
            vy: 0.0,
        };
        let nodes = vec![node("lower"), node("upper")];
        assert_eq!(hit_test(&nodes, 10.0, 10.0), Some(1));
        assert_eq!(hit_test(&nodes, 100.0, 100.0), None);
    }
}
