//! Coordinate assignment for dagre layout
//!
//! Assigns x and y coordinates to nodes after rank and order assignment.
//!
//! Y-coordinates are based on rank with ranksep separation.
//! X-coordinates use a simplified Brandes-Köpf inspired algorithm.

mod bk;

use crate::layout::dagre::graph::DagreGraph;

/// Assign x and y coordinates to all nodes
pub fn position(g: &mut DagreGraph) {
    position_y(g);
    let xs = bk::position_x(g);

    // Apply x coordinates
    for (v, x) in xs {
        if let Some(node) = g.node_mut(&v) {
            node.x = Some(x);
        }
    }
}

/// Assign y coordinates based on rank
fn position_y(g: &mut DagreGraph) {
    let ranksep = g.graph().ranksep;

    // Build layer matrix
    let max_rank = g
        .nodes()
        .iter()
        .filter_map(|v| g.node(v).and_then(|n| n.rank))
        .max()
        .unwrap_or(0) as usize;

    let mut layers: Vec<Vec<String>> = (0..=max_rank).map(|_| Vec::new()).collect();

    for v in g.nodes() {
        if let Some(node) = g.node(v) {
            if let Some(rank) = node.rank {
                if rank >= 0 && (rank as usize) <= max_rank {
                    layers[rank as usize].push(v.clone());
                }
            }
        }
    }

    // Assign y coordinates
    let mut prev_y = 0.0;

    for layer in &layers {
        // Find max height in this layer
        let max_height = layer
            .iter()
            .filter_map(|v| g.node(v).map(|n| n.height))
            .fold(0.0_f64, f64::max);

        // Assign y to center of the row
        let y = prev_y + max_height / 2.0;

        for v in layer {
            if let Some(node) = g.node_mut(v) {
                node.y = Some(y);
            }
        }

        prev_y += max_height + ranksep;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::dagre::graph::{EdgeLabel, NodeLabel};
    use crate::layout::dagre::order;
    use crate::layout::dagre::rank;
    use crate::layout::dagre::Ranker;

    #[test]
    fn test_position_single_node() {
        let mut g = DagreGraph::new();
        g.set_node(
            "a",
            NodeLabel {
                width: 50.0,
                height: 100.0,
                ..Default::default()
            },
        );
        rank::assign_ranks(&mut g, Ranker::LongestPath);
        order::order(&mut g);

        position(&mut g);

        let node = g.node("a").unwrap();
        assert!(node.x.is_some());
        assert!(node.y.is_some());
        assert_eq!(node.y, Some(50.0)); // height/2 = 100/2 = 50
    }

    #[test]
    fn test_position_chain() {
        let mut g = DagreGraph::new();
        g.graph_mut().ranksep = 100.0;
        g.set_node(
            "a",
            NodeLabel {
                width: 50.0,
                height: 40.0,
                ..Default::default()
            },
        );
        g.set_node(
            "b",
            NodeLabel {
                width: 50.0,
                height: 60.0,
                ..Default::default()
            },
        );
        g.set_edge("a", "b", EdgeLabel::default());
        rank::assign_ranks(&mut g, Ranker::LongestPath);
        order::order(&mut g);

        position(&mut g);

        let a = g.node("a").unwrap();
        let b = g.node("b").unwrap();

        // a is at y = 40/2 = 20
        assert_eq!(a.y, Some(20.0));
        // b is at y = 40 + 100 + 60/2 = 170
        assert_eq!(b.y, Some(170.0));
    }

    #[test]
    fn test_position_parallel_nodes() {
        let mut g = DagreGraph::new();
        g.set_node(
            "a",
            NodeLabel {
                width: 50.0,
                height: 40.0,
                ..Default::default()
            },
        );
        g.set_node(
            "b",
            NodeLabel {
                width: 50.0,
                height: 40.0,
                ..Default::default()
            },
        );
        g.set_node(
            "c",
            NodeLabel {
                width: 50.0,
                height: 40.0,
                ..Default::default()
            },
        );
        g.set_edge("a", "b", EdgeLabel::default());
        g.set_edge("a", "c", EdgeLabel::default());
        rank::assign_ranks(&mut g, Ranker::LongestPath);
        order::order(&mut g);

        position(&mut g);

        let a = g.node("a").unwrap();
        let b = g.node("b").unwrap();
        let c = g.node("c").unwrap();

        // All should have coordinates
        assert!(a.x.is_some());
        assert!(a.y.is_some());
        assert!(b.x.is_some());
        assert!(b.y.is_some());
        assert!(c.x.is_some());
        assert!(c.y.is_some());

        // b and c should be at same y (same rank)
        assert_eq!(b.y, c.y);

        // b and c should have different x
        assert_ne!(b.x, c.x);
    }
}
