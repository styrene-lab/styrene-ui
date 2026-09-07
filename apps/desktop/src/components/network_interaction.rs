use crate::state::GraphNode;

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Interaction {
    None,
    PressingNode { idx: usize, start_x: f64, start_y: f64 },
    Panning { last_x: f64, last_y: f64 },
    DraggingNode { idx: usize, last_x: f64, last_y: f64 },
}

impl Interaction {
    /// A small amount of pointer jitter still counts as a click.
    pub(crate) fn clicked_node(&self, x: f64, y: f64) -> Option<usize> {
        match *self {
            Self::PressingNode { idx, start_x, start_y }
                if (x - start_x).hypot(y - start_y) < 5.0 =>
            {
                Some(idx)
            }
            _ => None,
        }
    }

    pub(crate) fn move_node(&mut self, x: f64, y: f64) -> Option<(usize, f64, f64)> {
        let (idx, last_x, last_y) = match *self {
            Self::PressingNode { idx, start_x, start_y }
                if (x - start_x).hypot(y - start_y) >= 5.0 =>
            {
                (idx, start_x, start_y)
            }
            Self::DraggingNode { idx, last_x, last_y } => (idx, last_x, last_y),
            _ => return None,
        };
        *self = Self::DraggingNode { idx, last_x: x, last_y: y };
        Some((idx, x - last_x, y - last_y))
    }
}

/// Place a measured hover card in canvas coordinates, flipping at each edge.
pub(crate) fn tooltip_position(
    pointer: (f64, f64),
    canvas: (f64, f64),
    card: (f64, f64),
) -> (f64, f64) {
    let place = |pointer: f64, extent: f64, length: f64| {
        let after = pointer + 12.0;
        let preferred =
            if after + length <= extent - 8.0 { after } else { pointer - length - 12.0 };
        preferred.clamp(8.0, (extent - length - 8.0).max(8.0))
    };
    (place(pointer.0, canvas.0, card.0), place(pointer.1, canvas.1, card.1))
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
    fn hover_card_flips_at_bottom_right_and_stays_inside_all_canvas_edges() {
        let canvas = (640.0, 360.0);
        let card = (240.0, 120.0);
        assert_eq!(tooltip_position((630.0, 350.0), canvas, card), (378.0, 218.0));
        for pointer in [(-20.0, -20.0), (0.0, 0.0), (640.0, 0.0), (0.0, 360.0), (640.0, 360.0)] {
            let (x, y) = tooltip_position(pointer, canvas, card);
            assert!(x >= 8.0 && x + card.0 <= canvas.0 - 8.0);
            assert!(y >= 8.0 && y + card.1 <= canvas.1 - 8.0);
        }
    }

    #[test]
    fn node_click_tolerates_jitter_but_drag_never_selects_even_after_returning() {
        let mut gesture = Interaction::PressingNode { idx: 2, start_x: 100.0, start_y: 100.0 };
        assert_eq!(gesture.move_node(102.0, 101.0), None);
        assert_eq!(gesture.clicked_node(102.0, 101.0), Some(2));
        assert_eq!(gesture.move_node(106.0, 100.0), Some((2, 6.0, 0.0)));
        assert_eq!(gesture.move_node(100.0, 100.0), Some((2, -6.0, 0.0)));
        assert_eq!(gesture.clicked_node(100.0, 100.0), None);
    }

    #[test]
    fn release_far_from_press_without_a_move_event_does_not_select() {
        let gesture = Interaction::PressingNode { idx: 2, start_x: 100.0, start_y: 100.0 };
        assert_eq!(gesture.clicked_node(120.0, 100.0), None);
        assert_eq!(Interaction::None.clicked_node(100.0, 100.0), None);
    }

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
