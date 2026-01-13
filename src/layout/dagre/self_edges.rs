//! Self-edge handling for dagre layout
//!
//! Self-edges (edges where source == target) require special handling because:
//! - They would create cycles that confuse the ranking algorithm
//! - They need dummy nodes to reserve space for the self-edge label
//! - They need special arc positioning after layout
//!
//! The three phases are:
//! 1. remove_self_edges - Store self-edges on nodes and remove from graph (before ranking)
//! 2. insert_self_edges - Create dummy nodes for self-edges (after ordering)
//! 3. position_self_edges - Calculate arc positions and restore edges (after positioning)

use super::graph::{DagreGraph, EdgeKey, NodeLabel, SelfEdgeInfo};
use super::util;

/// Remove self-edges from the graph and store them on their nodes.
///
/// This should be called early in the layout pipeline, before acyclic processing.
/// Self-edges are stored in node.self_edges and removed from the graph.
pub fn remove_self_edges(g: &mut DagreGraph) {
    // Collect self-edges first (can't modify while iterating)
    let self_edge_keys: Vec<EdgeKey> = g
        .edges()
        .iter()
        .filter(|e| e.v == e.w)
        .map(|e| (*e).clone())
        .collect();

    // Process each self-edge
    for key in self_edge_keys {
        // Get the edge label before removing
        if let Some(label) = g.edge_by_key(&key).cloned() {
            // Store on the node
            if let Some(node) = g.node_mut(&key.v) {
                node.self_edges.push(SelfEdgeInfo {
                    edge_key: key.clone(),
                    label,
                });
            }

            // Remove from graph
            g.remove_edge_by_key(&key);
        }
    }
}

/// Insert dummy nodes for self-edges after ordering.
///
/// This creates a dummy node next to each node that has self-edges.
/// The dummy node has the same rank as the original node and order = node.order + 1.
pub fn insert_self_edges(g: &mut DagreGraph) {
    let layers = util::build_layer_matrix(g);

    for layer in &layers {
        let mut order_shift = 0;

        for (i, v) in layer.iter().enumerate() {
            if let Some(node) = g.node_mut(v) {
                // Update node order with accumulated shift
                node.order = Some(i + order_shift);

                // Get self-edges (take ownership to avoid borrow issues)
                let self_edges = std::mem::take(&mut node.self_edges);
                let rank = node.rank;

                // Insert dummy nodes for each self-edge
                for self_edge in self_edges {
                    order_shift += 1;

                    // Create dummy node for self-edge
                    let dummy_name = format!("_se_{}_{}", v, order_shift);
                    let dummy = NodeLabel {
                        width: self_edge.label.width,
                        height: self_edge.label.height,
                        rank,
                        order: Some(i + order_shift),
                        dummy: Some("selfedge".to_string()),
                        // Store original edge info for positionSelfEdges
                        self_edge_info: Some(self_edge),
                        ..Default::default()
                    };

                    g.set_node(&dummy_name, dummy);
                }
            }
        }
    }
}

/// Position self-edges and restore them to the graph after positioning.
///
/// This calculates the arc curve for self-edges based on the node's final position.
pub fn position_self_edges(g: &mut DagreGraph) {
    // Collect all self-edge dummy nodes
    let dummy_nodes: Vec<String> = g
        .nodes()
        .into_iter()
        .filter(|v| {
            g.node(v)
                .map(|n| n.dummy.as_deref() == Some("selfedge"))
                .unwrap_or(false)
        })
        .cloned()
        .collect();

    for dummy_name in dummy_nodes {
        // Get dummy node info
        let (self_edge_info, dummy_x, _dummy_y) = {
            let dummy = match g.node(&dummy_name) {
                Some(n) => n,
                None => continue,
            };
            (dummy.self_edge_info.clone(), dummy.x, dummy.y)
        };

        // Get the self-edge info
        let self_edge = match self_edge_info {
            Some(info) => info,
            None => continue,
        };

        // Get the original node's position
        let (self_node_x, self_node_y, self_node_width, self_node_height) = {
            let self_node = match g.node(&self_edge.edge_key.v) {
                Some(n) => n,
                None => continue,
            };
            (
                self_node.x.unwrap_or(0.0),
                self_node.y.unwrap_or(0.0),
                self_node.width,
                self_node.height,
            )
        };

        // Calculate arc points
        // The arc goes from the right side of the node, curves up/down, and returns
        let x = self_node_x + self_node_width / 2.0;
        let y = self_node_y;
        let dx = dummy_x.unwrap_or(x) - x;
        let dy = self_node_height / 2.0;

        // Create the arc path points
        let points = vec![
            super::graph::Point {
                x: x + dx,
                y: y - dy,
            },
            super::graph::Point {
                x: x + dx,
                y: y + dy,
            },
        ];

        // Restore the edge with calculated points
        let mut label = self_edge.label.clone();
        label.points = points;
        label.x = Some(x + dx);
        label.y = Some(y);

        // Restore the edge using set_edge_with_name
        if let Some(name) = &self_edge.edge_key.name {
            g.set_edge_with_name(&self_edge.edge_key.v, &self_edge.edge_key.w, label, name);
        } else {
            g.set_edge(&self_edge.edge_key.v, &self_edge.edge_key.w, label);
        }

        // Remove the dummy node
        g.remove_node(&dummy_name);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::dagre::graph::EdgeLabel;

    fn create_test_graph() -> DagreGraph {
        let mut g = DagreGraph::new();
        g.set_node(
            "a",
            NodeLabel {
                width: 100.0,
                height: 50.0,
                rank: Some(0),
                order: Some(0),
                ..Default::default()
            },
        );
        g.set_node(
            "b",
            NodeLabel {
                width: 100.0,
                height: 50.0,
                rank: Some(0),
                order: Some(1),
                ..Default::default()
            },
        );
        g
    }

    #[test]
    fn test_remove_self_edges_removes_from_graph() {
        let mut g = create_test_graph();

        // Add a self-edge
        g.set_edge(
            "a",
            "a",
            EdgeLabel {
                width: 20.0,
                height: 10.0,
                ..Default::default()
            },
        );

        assert!(
            g.has_edge("a", "a"),
            "Self-edge should exist before removal"
        );

        remove_self_edges(&mut g);

        assert!(
            !g.has_edge("a", "a"),
            "Self-edge should be removed from graph"
        );
    }

    #[test]
    fn test_remove_self_edges_stores_on_node() {
        let mut g = create_test_graph();

        // Add a self-edge with dimensions
        g.set_edge(
            "a",
            "a",
            EdgeLabel {
                width: 20.0,
                height: 10.0,
                ..Default::default()
            },
        );

        remove_self_edges(&mut g);

        // Check self-edge was stored on node
        let node = g.node("a").expect("Node should exist");
        assert_eq!(node.self_edges.len(), 1, "Node should have one self-edge");
        assert_eq!(node.self_edges[0].label.width, 20.0);
        assert_eq!(node.self_edges[0].label.height, 10.0);
    }

    #[test]
    fn test_remove_self_edges_preserves_regular_edges() {
        let mut g = create_test_graph();

        // Add regular edge and self-edge
        g.set_edge("a", "b", EdgeLabel::default());
        g.set_edge(
            "a",
            "a",
            EdgeLabel {
                width: 20.0,
                ..Default::default()
            },
        );

        remove_self_edges(&mut g);

        assert!(g.has_edge("a", "b"), "Regular edge should be preserved");
        assert!(!g.has_edge("a", "a"), "Self-edge should be removed");
    }

    #[test]
    fn test_insert_self_edges_creates_dummy_nodes() {
        let mut g = create_test_graph();

        // Store self-edge on node
        if let Some(node) = g.node_mut("a") {
            node.self_edges.push(SelfEdgeInfo {
                edge_key: EdgeKey {
                    v: "a".to_string(),
                    w: "a".to_string(),
                    name: None,
                },
                label: EdgeLabel {
                    width: 20.0,
                    height: 10.0,
                    ..Default::default()
                },
            });
        }

        let initial_node_count = g.node_count();

        insert_self_edges(&mut g);

        assert_eq!(
            g.node_count(),
            initial_node_count + 1,
            "Should have created one dummy node"
        );

        // Find the dummy node
        let nodes_list: Vec<String> = g.nodes().into_iter().cloned().collect();
        let dummy_node = nodes_list
            .iter()
            .find(|v| g.node(v).map(|n| n.dummy.is_some()).unwrap_or(false));

        assert!(dummy_node.is_some(), "Should have a dummy node");

        let dummy = g.node(dummy_node.unwrap()).unwrap();
        assert_eq!(dummy.dummy, Some("selfedge".to_string()));
        assert_eq!(dummy.width, 20.0);
        assert_eq!(dummy.height, 10.0);
    }

    #[test]
    fn test_insert_self_edges_shifts_order() {
        let mut g = create_test_graph();

        // Store self-edge on node "a"
        if let Some(node) = g.node_mut("a") {
            node.self_edges.push(SelfEdgeInfo {
                edge_key: EdgeKey {
                    v: "a".to_string(),
                    w: "a".to_string(),
                    name: None,
                },
                label: EdgeLabel::default(),
            });
        }

        insert_self_edges(&mut g);

        // Node "a" should still have order 0
        assert_eq!(g.node("a").unwrap().order, Some(0));

        // The dummy node should have order 1
        let nodes_list: Vec<String> = g.nodes().into_iter().cloned().collect();
        let dummy_node = nodes_list
            .iter()
            .find(|v| g.node(v).map(|n| n.dummy.is_some()).unwrap_or(false))
            .unwrap();
        assert_eq!(g.node(dummy_node).unwrap().order, Some(1));
    }

    #[test]
    fn test_position_self_edges_restores_edge() {
        let mut g = DagreGraph::new();

        // Set up positioned node
        g.set_node(
            "a",
            NodeLabel {
                width: 100.0,
                height: 50.0,
                x: Some(50.0),
                y: Some(25.0),
                ..Default::default()
            },
        );

        // Set up self-edge dummy node
        g.set_node(
            "_se_a_1",
            NodeLabel {
                width: 20.0,
                height: 10.0,
                x: Some(150.0),
                y: Some(25.0),
                dummy: Some("selfedge".to_string()),
                self_edge_info: Some(SelfEdgeInfo {
                    edge_key: EdgeKey {
                        v: "a".to_string(),
                        w: "a".to_string(),
                        name: None,
                    },
                    label: EdgeLabel {
                        width: 20.0,
                        height: 10.0,
                        ..Default::default()
                    },
                }),
                ..Default::default()
            },
        );

        position_self_edges(&mut g);

        // Dummy node should be removed
        assert!(g.node("_se_a_1").is_none(), "Dummy node should be removed");

        // Self-edge should be restored
        assert!(g.has_edge("a", "a"), "Self-edge should be restored");

        // Edge should have points
        let edge = g.edge("a", "a").unwrap();
        assert!(!edge.points.is_empty(), "Edge should have points for arc");
    }
}
