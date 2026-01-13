//! Layout engine for positioning diagram elements
//!
//! This module provides a graph layout engine using the dagre algorithm
//! (a port of dagre.js) for visual parity with mermaid.js.

mod adapter;
mod graph;
mod size;
mod types;

pub mod dagre;

pub use adapter::{NodeSizeConfig, SizeEstimator, ToLayoutGraph};
pub use graph::LayoutGraph;
pub use size::CharacterSizeEstimator;
pub use types::{
    LayoutDirection, LayoutEdge, LayoutNode, LayoutOptions, NodeShape, Padding, Point,
};

use crate::error::Result;
use dagre::graph::{DagreGraph, EdgeLabel, NodeLabel};
use dagre::{DagreConfig, RankDir, Ranker};

/// Perform layout on a graph using dagre algorithm
pub fn layout(mut graph: LayoutGraph) -> Result<LayoutGraph> {
    // Convert LayoutGraph to DagreGraph
    let mut dagre_graph = to_dagre_graph(&graph);

    // Configure dagre based on LayoutOptions
    let config = to_dagre_config(&graph.options);

    // Run dagre layout
    dagre::layout(&mut dagre_graph, &config);

    // Copy results back to LayoutGraph
    apply_dagre_results(&mut graph, &dagre_graph);

    // Compute graph bounds
    graph.compute_bounds();

    Ok(graph)
}

/// Convert LayoutGraph to DagreGraph for dagre processing
fn to_dagre_graph(graph: &LayoutGraph) -> DagreGraph {
    let mut dg = DagreGraph::new();

    // Set graph-level options
    dg.graph_mut().nodesep = graph.options.node_spacing;
    dg.graph_mut().ranksep = graph.options.layer_spacing;
    dg.graph_mut().rankdir = match graph.options.direction {
        LayoutDirection::TopToBottom => "TB".to_string(),
        LayoutDirection::BottomToTop => "BT".to_string(),
        LayoutDirection::LeftToRight => "LR".to_string(),
        LayoutDirection::RightToLeft => "RL".to_string(),
    };

    // Add nodes (flatten the tree, handling children separately)
    add_nodes_recursive(&mut dg, &graph.nodes, None);

    // Add edges
    for edge in &graph.edges {
        if let (Some(source), Some(target)) = (edge.source(), edge.target()) {
            // Estimate label size if present (roughly 8px per char, 16px height)
            let (label_width, label_height) = if let Some(label) = &edge.label {
                let width = (label.len() as f64) * 8.0 + 16.0; // padding
                let height = 20.0;
                (width, height)
            } else {
                (0.0, 0.0)
            };

            let label = EdgeLabel {
                weight: edge.weight as i32,
                width: label_width,
                height: label_height,
                ..Default::default()
            };
            dg.set_edge(source, target, label);
        }
    }

    dg
}

/// Recursively add nodes to DagreGraph, handling compound nodes
fn add_nodes_recursive(dg: &mut DagreGraph, nodes: &[LayoutNode], parent: Option<&str>) {
    for node in nodes {
        let label = NodeLabel {
            width: node.width,
            height: node.height,
            shape: node.shape,
            ..Default::default()
        };
        dg.set_node(&node.id, label);

        // Set parent relationship for compound graphs
        if let Some(parent_id) = parent {
            dg.set_parent(&node.id, parent_id);
        }

        // Recursively add children
        if !node.children.is_empty() {
            add_nodes_recursive(dg, &node.children, Some(&node.id));
        }
    }
}

/// Convert LayoutOptions to DagreConfig
fn to_dagre_config(options: &LayoutOptions) -> DagreConfig {
    DagreConfig {
        rankdir: match options.direction {
            LayoutDirection::TopToBottom => RankDir::TB,
            LayoutDirection::BottomToTop => RankDir::BT,
            LayoutDirection::LeftToRight => RankDir::LR,
            LayoutDirection::RightToLeft => RankDir::RL,
        },
        nodesep: options.node_spacing,
        ranksep: options.layer_spacing,
        // Use LongestPath as workaround for network simplex bug (mermaid-rs-3yi)
        ranker: Ranker::LongestPath,
        ..Default::default()
    }
}

/// Copy position results from DagreGraph back to LayoutGraph
fn apply_dagre_results(graph: &mut LayoutGraph, dg: &DagreGraph) {
    apply_results_recursive(&mut graph.nodes, dg);

    // Copy edge bend points
    for edge in &mut graph.edges {
        if let (Some(source), Some(target)) = (edge.source(), edge.target()) {
            if let Some(edge_label) = dg.edge(source, target) {
                // Convert dagre points to layout points
                edge.bend_points = edge_label
                    .points
                    .iter()
                    .map(|p| Point::new(p.x, p.y))
                    .collect();

                // Set label position if present from dagre (for long edges with dummy nodes)
                if let (Some(x), Some(y)) = (edge_label.x, edge_label.y) {
                    edge.label_position = Some(Point::new(x, y));
                }
                // For edges without label position from dagre (short edges), compute from bend points
                else if edge.label.is_some() && !edge.bend_points.is_empty() {
                    // Use the midpoint of the edge path as label position
                    let mid_idx = edge.bend_points.len() / 2;
                    if mid_idx > 0 {
                        // Average the two middle points if even number of points
                        let p1 = &edge.bend_points[mid_idx - 1];
                        let p2 = &edge.bend_points[mid_idx];
                        edge.label_position =
                            Some(Point::new((p1.x + p2.x) / 2.0, (p1.y + p2.y) / 2.0));
                    } else {
                        // Use the middle point if odd number
                        edge.label_position = Some(edge.bend_points[mid_idx]);
                    }
                }
            }
        }
    }
}

/// Recursively apply results to nodes
fn apply_results_recursive(nodes: &mut [LayoutNode], dg: &DagreGraph) {
    for node in nodes {
        if let Some(dagre_node) = dg.node(&node.id) {
            // Dagre returns center coordinates, convert to top-left
            if let (Some(cx), Some(cy)) = (dagre_node.x, dagre_node.y) {
                node.x = Some(cx - node.width / 2.0);
                node.y = Some(cy - node.height / 2.0);
            }

            // Copy layer/order info
            if let Some(rank) = dagre_node.rank {
                node.layer = Some(rank as usize);
            }
            if let Some(order) = dagre_node.order {
                node.order = Some(order);
            }
        }

        // Recursively apply to children
        if !node.children.is_empty() {
            apply_results_recursive(&mut node.children, dg);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_layout_simple_graph() {
        let mut graph = LayoutGraph::new("test");
        graph.add_node(LayoutNode::new("A", 50.0, 30.0));
        graph.add_node(LayoutNode::new("B", 50.0, 30.0));
        graph.add_edge(LayoutEdge::new("e1", "A", "B"));

        let result = layout(graph).unwrap();

        // Both nodes should have positions assigned
        let a = result.get_node("A").unwrap();
        let b = result.get_node("B").unwrap();

        assert!(a.x.is_some(), "Node A should have x position");
        assert!(a.y.is_some(), "Node A should have y position");
        assert!(b.x.is_some(), "Node B should have x position");
        assert!(b.y.is_some(), "Node B should have y position");

        // B should be below A (in TB layout)
        assert!(
            b.y.unwrap() > a.y.unwrap(),
            "B should be below A in top-to-bottom layout"
        );
    }

    #[test]
    fn test_layout_diamond() {
        // A -> B, A -> C, B -> D, C -> D
        let mut graph = LayoutGraph::new("diamond");
        graph.add_node(LayoutNode::new("A", 50.0, 30.0));
        graph.add_node(LayoutNode::new("B", 50.0, 30.0));
        graph.add_node(LayoutNode::new("C", 50.0, 30.0));
        graph.add_node(LayoutNode::new("D", 50.0, 30.0));
        graph.add_edge(LayoutEdge::new("e1", "A", "B"));
        graph.add_edge(LayoutEdge::new("e2", "A", "C"));
        graph.add_edge(LayoutEdge::new("e3", "B", "D"));
        graph.add_edge(LayoutEdge::new("e4", "C", "D"));

        let result = layout(graph).unwrap();

        let a = result.get_node("A").unwrap();
        let b = result.get_node("B").unwrap();
        let c = result.get_node("C").unwrap();
        let d = result.get_node("D").unwrap();

        // All nodes should have positions
        assert!(a.x.is_some() && a.y.is_some());
        assert!(b.x.is_some() && b.y.is_some());
        assert!(c.x.is_some() && c.y.is_some());
        assert!(d.x.is_some() && d.y.is_some());

        // B and C should be on the same layer (same y)
        assert!(
            (b.y.unwrap() - c.y.unwrap()).abs() < 1.0,
            "B and C should be on the same layer"
        );

        // D should be below B and C
        assert!(d.y.unwrap() > b.y.unwrap());
    }

    #[test]
    fn test_edge_points_generated() {
        let mut graph = LayoutGraph::new("test");
        graph.add_node(LayoutNode::new("A", 50.0, 30.0));
        graph.add_node(LayoutNode::new("B", 50.0, 30.0));
        graph.add_edge(LayoutEdge::new("e1", "A", "B"));

        let result = layout(graph).unwrap();

        // Edge should have bend points after layout
        let edge = result.edges.first().expect("Should have an edge");
        assert!(
            !edge.bend_points.is_empty(),
            "Edge should have bend points after layout, got {} points",
            edge.bend_points.len()
        );

        // Should have at least 2 points (start and end)
        assert!(
            edge.bend_points.len() >= 2,
            "Edge should have at least start and end points, got {} points",
            edge.bend_points.len()
        );
    }

    #[test]
    fn test_edge_points_lr_direction() {
        // Test LR (left-to-right) layout which flowcharts use
        let mut graph = LayoutGraph::new("test_lr");
        graph.options.direction = LayoutDirection::LeftToRight;
        graph.add_node(LayoutNode::new("A", 50.0, 30.0));
        graph.add_node(LayoutNode::new("B", 50.0, 30.0));
        graph.add_edge(LayoutEdge::new("L-A-B-0", "A", "B"));

        let result = layout(graph).unwrap();

        // Check that edge points exist for LR layout
        let edge = result.edges.first().expect("Should have an edge");
        eprintln!(
            "LR Edge {} has {} bend points:",
            edge.id,
            edge.bend_points.len()
        );
        for (i, p) in edge.bend_points.iter().enumerate() {
            eprintln!("  Point {}: ({:.1}, {:.1})", i, p.x, p.y);
        }

        assert!(
            !edge.bend_points.is_empty(),
            "LR Edge should have bend points, got {} points",
            edge.bend_points.len()
        );
    }

    #[test]
    fn test_layout_left_to_right() {
        let mut graph = LayoutGraph::new("test");
        graph.options.direction = LayoutDirection::LeftToRight;
        graph.add_node(LayoutNode::new("A", 50.0, 30.0));
        graph.add_node(LayoutNode::new("B", 50.0, 30.0));
        graph.add_edge(LayoutEdge::new("e1", "A", "B"));

        let result = layout(graph).unwrap();

        let a = result.get_node("A").unwrap();
        let b = result.get_node("B").unwrap();

        // B should be to the right of A (in LR layout)
        assert!(
            b.x.unwrap() > a.x.unwrap(),
            "B should be to the right of A in left-to-right layout"
        );
    }

    #[test]
    fn test_edge_label_gets_position() {
        let mut graph = LayoutGraph::new("test_label");
        graph.options.direction = LayoutDirection::LeftToRight;
        graph.add_node(LayoutNode::new("A", 50.0, 30.0));
        graph.add_node(LayoutNode::new("B", 50.0, 30.0));

        // Add edge with label
        let edge = LayoutEdge::new("e1", "A", "B").with_label("Yes");
        graph.add_edge(edge);

        let result = layout(graph).unwrap();

        // Edge label should have a position
        let edge = result.edges.first().expect("Should have an edge");
        assert!(
            edge.label_position.is_some(),
            "Edge with label should have label_position set. Label: {:?}, Position: {:?}",
            edge.label,
            edge.label_position
        );

        // Label position should be between the nodes
        let a = result.get_node("A").unwrap();
        let b = result.get_node("B").unwrap();
        let label_pos = edge.label_position.unwrap();

        // For LR layout, label x should be between A and B
        let a_right = a.x.unwrap() + a.width;
        let b_left = b.x.unwrap();
        assert!(
            label_pos.x > a_right && label_pos.x < b_left,
            "Label x ({}) should be between A right edge ({}) and B left edge ({})",
            label_pos.x,
            a_right,
            b_left
        );
    }
}
