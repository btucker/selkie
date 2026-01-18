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
    geometric_midpoint, LayoutDirection, LayoutEdge, LayoutNode, LayoutOptions, LayoutRanker,
    NodeShape, Padding, Point,
};

use crate::error::Result;
use dagre::graph::{DagreGraph, EdgeLabel, NodeLabel};
use dagre::{DagreConfig, RankDir, Ranker};
use std::collections::HashMap;

/// Stored layout result for a subgraph that was laid out with its own direction
#[derive(Debug, Clone)]
struct SubgraphLayoutResult {
    /// Relative positions of child nodes (relative to subgraph origin)
    child_positions: HashMap<String, (f64, f64)>, // (x, y) relative to subgraph top-left
    /// Computed width of the subgraph content
    width: f64,
    /// Computed height of the subgraph content
    height: f64,
}

/// Perform layout on a graph using dagre algorithm
pub fn layout(mut graph: LayoutGraph) -> Result<LayoutGraph> {
    // Phase 1: Identify subgraphs with their own directions and lay them out first
    let subgraph_layouts = layout_subgraphs_with_directions(&graph)?;

    // Phase 2: Update subgraph node dimensions based on their internal layouts
    for (subgraph_id, layout_result) in &subgraph_layouts {
        if let Some(node) = graph.get_node_mut(subgraph_id) {
            // Set subgraph dimensions from internal layout + padding for the cluster
            let padding = 20.0; // Padding around subgraph content
            node.width = layout_result.width + padding * 2.0;
            node.height = layout_result.height + padding * 2.0;
        }
    }

    // Phase 3: Run main layout on the parent graph
    let mut dagre_graph = to_dagre_graph(&graph);
    let config = to_dagre_config(&graph.options);
    dagre::layout(&mut dagre_graph, &config);

    // Phase 4: Copy results back to LayoutGraph
    apply_dagre_results(&mut graph, &dagre_graph);

    // Phase 5: Apply pre-computed child positions for subgraphs with custom directions
    apply_subgraph_child_positions(&mut graph, &subgraph_layouts);

    // Compute graph bounds
    graph.compute_bounds();

    Ok(graph)
}

/// Parse direction from string
fn parse_direction(dir: &str) -> LayoutDirection {
    match dir.to_uppercase().as_str() {
        "LR" => LayoutDirection::LeftToRight,
        "RL" => LayoutDirection::RightToLeft,
        "BT" => LayoutDirection::BottomToTop,
        _ => LayoutDirection::TopToBottom, // Default TB
    }
}

/// Identify and layout subgraphs that have their own direction
fn layout_subgraphs_with_directions(
    graph: &LayoutGraph,
) -> Result<HashMap<String, SubgraphLayoutResult>> {
    let mut results = HashMap::new();

    // Find subgraphs with custom directions
    for node in &graph.nodes {
        if node.metadata.get("is_group") == Some(&"true".to_string()) {
            if let Some(dir_str) = node.metadata.get("dir") {
                let subgraph_dir = parse_direction(dir_str);

                // Only process if direction differs from parent
                if subgraph_dir != graph.options.direction {
                    // Find all child nodes belonging to this subgraph
                    let child_ids: Vec<&str> = graph
                        .nodes
                        .iter()
                        .filter(|n| n.parent_id.as_deref() == Some(&node.id))
                        .map(|n| n.id.as_str())
                        .collect();

                    if child_ids.is_empty() {
                        continue;
                    }

                    // Create a sub-graph with just these nodes
                    let mut sub_graph = LayoutGraph::new(format!("{}_internal", node.id));
                    sub_graph.options.direction = subgraph_dir;
                    sub_graph.options.node_spacing = graph.options.node_spacing;
                    sub_graph.options.layer_spacing = graph.options.layer_spacing;

                    // Add child nodes (without parent relationship for internal layout)
                    for child_id in &child_ids {
                        if let Some(child_node) = graph.get_node(child_id) {
                            let mut cloned = child_node.clone();
                            cloned.parent_id = None; // Remove parent for internal layout
                            sub_graph.add_node(cloned);
                        }
                    }

                    // Add edges between children
                    for edge in &graph.edges {
                        if let (Some(source), Some(target)) = (edge.source(), edge.target()) {
                            if child_ids.contains(&source) && child_ids.contains(&target) {
                                sub_graph.add_edge(edge.clone());
                            }
                        }
                    }

                    // Layout the sub-graph
                    let mut dagre_graph = to_dagre_graph(&sub_graph);
                    let config = to_dagre_config(&sub_graph.options);
                    dagre::layout(&mut dagre_graph, &config);
                    apply_dagre_results(&mut sub_graph, &dagre_graph);
                    sub_graph.compute_bounds();

                    // Store relative positions
                    let mut child_positions = HashMap::new();
                    let min_x = sub_graph
                        .nodes
                        .iter()
                        .filter_map(|n| n.x)
                        .fold(f64::MAX, f64::min);
                    let min_y = sub_graph
                        .nodes
                        .iter()
                        .filter_map(|n| n.y)
                        .fold(f64::MAX, f64::min);

                    for child in &sub_graph.nodes {
                        if let (Some(x), Some(y)) = (child.x, child.y) {
                            // Store position relative to subgraph origin
                            child_positions.insert(child.id.clone(), (x - min_x, y - min_y));
                        }
                    }

                    results.insert(
                        node.id.clone(),
                        SubgraphLayoutResult {
                            child_positions,
                            width: sub_graph.width.unwrap_or(0.0),
                            height: sub_graph.height.unwrap_or(0.0),
                        },
                    );
                }
            }
        }
    }

    Ok(results)
}

/// Apply pre-computed child positions for subgraphs with custom directions
fn apply_subgraph_child_positions(
    graph: &mut LayoutGraph,
    subgraph_layouts: &HashMap<String, SubgraphLayoutResult>,
) {
    for (subgraph_id, layout_result) in subgraph_layouts {
        // Get the subgraph's final position
        if let Some(subgraph_node) = graph.get_node(subgraph_id).cloned() {
            if let (Some(sg_x), Some(sg_y)) = (subgraph_node.x, subgraph_node.y) {
                // Calculate padding offset (center the content in the subgraph)
                let padding = 20.0;
                let content_offset_x = padding;
                let content_offset_y = padding;

                // Apply relative positions to children
                for (child_id, (rel_x, rel_y)) in &layout_result.child_positions {
                    if let Some(child) = graph.get_node_mut(child_id) {
                        child.x = Some(sg_x + content_offset_x + rel_x);
                        child.y = Some(sg_y + content_offset_y + rel_y);
                    }
                }
            }
        }
    }
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
        // Priority: explicit parent_id field, then parent parameter (from nested children)
        if let Some(ref parent_id) = node.parent_id {
            dg.set_parent(&node.id, parent_id);
        } else if let Some(parent_id) = parent {
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
    use types::LayoutRanker;

    DagreConfig {
        rankdir: match options.direction {
            LayoutDirection::TopToBottom => RankDir::TB,
            LayoutDirection::BottomToTop => RankDir::BT,
            LayoutDirection::LeftToRight => RankDir::LR,
            LayoutDirection::RightToLeft => RankDir::RL,
        },
        nodesep: options.node_spacing,
        ranksep: options.layer_spacing,
        ranker: match options.ranker {
            LayoutRanker::NetworkSimplex => Ranker::NetworkSimplex,
            LayoutRanker::LongestPath => Ranker::LongestPath,
        },
        // Use DFS-based cycle detection instead of greedy
        // Greedy can incorrectly reverse forward edges in graphs with back edges
        acyclicer: dagre::Acyclicer::Dfs,
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

                // Compute label position from the actual edge path (bend_points)
                // Use geometric midpoint (point at half the total path length) for accurate
                // positioning, matching mermaid's traverseEdge approach
                if edge.label.is_some() && !edge.bend_points.is_empty() {
                    edge.label_position = types::geometric_midpoint(&edge.bend_points);
                }
                // Fallback to dagre's position if no bend points (shouldn't happen)
                else if edge.label.is_some() {
                    if let (Some(x), Some(y)) = (edge_label.x, edge_label.y) {
                        edge.label_position = Some(Point::new(x, y));
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
            // For compound nodes (subgraphs), dagre calculates width/height from border positions
            // Copy these calculated dimensions back to the LayoutNode
            if dagre_node.width > 0.0 && node.width == 0.0 {
                node.width = dagre_node.width;
            }
            if dagre_node.height > 0.0 && node.height == 0.0 {
                node.height = dagre_node.height;
            }

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
    fn test_subgraph_with_different_direction() {
        // Test that a subgraph with its own direction lays out nodes differently
        // Main graph is TB (top-to-bottom), subgraph is LR (left-to-right)
        //
        // In TB layout: nodes stack vertically (y increases)
        // In LR layout: nodes stack horizontally (x increases)
        //
        // So nodes inside the LR subgraph should be side-by-side (same y, different x)
        // while the subgraph itself is positioned vertically relative to other top-level nodes

        let mut graph = LayoutGraph::new("test_subgraph_dir");
        graph.options.direction = LayoutDirection::TopToBottom;

        // Add a subgraph node with LR direction
        let mut subgraph = LayoutNode::new("sub1", 0.0, 0.0);
        subgraph
            .metadata
            .insert("is_group".to_string(), "true".to_string());
        subgraph
            .metadata
            .insert("dir".to_string(), "LR".to_string());
        graph.add_node(subgraph);

        // Add child nodes belonging to the subgraph
        graph.add_node(LayoutNode::new("A", 50.0, 30.0).with_parent("sub1"));
        graph.add_node(LayoutNode::new("B", 50.0, 30.0).with_parent("sub1"));

        // Add edge within subgraph
        graph.add_edge(LayoutEdge::new("e1", "A", "B"));

        // Add a node outside the subgraph
        graph.add_node(LayoutNode::new("C", 50.0, 30.0));

        // Add edge from subgraph to external node
        graph.add_edge(LayoutEdge::new("e2", "B", "C"));

        let result = layout(graph).unwrap();

        let a = result.get_node("A").unwrap();
        let b = result.get_node("B").unwrap();
        let c = result.get_node("C").unwrap();

        eprintln!("Node A: x={:?}, y={:?}", a.x, a.y);
        eprintln!("Node B: x={:?}, y={:?}", b.x, b.y);
        eprintln!("Node C: x={:?}, y={:?}", c.x, c.y);

        // Within the LR subgraph, A and B should be side-by-side (B to the right of A)
        // They should have similar y-coordinates
        let a_center_y = a.y.unwrap() + a.height / 2.0;
        let b_center_y = b.y.unwrap() + b.height / 2.0;

        assert!(
            (a_center_y - b_center_y).abs() < 10.0,
            "A and B in LR subgraph should have similar y. A.y={:.1}, B.y={:.1}",
            a_center_y,
            b_center_y
        );

        assert!(
            b.x.unwrap() > a.x.unwrap(),
            "B should be to the right of A in LR subgraph. A.x={:.1}, B.x={:.1}",
            a.x.unwrap(),
            b.x.unwrap()
        );
    }

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

    #[test]
    fn test_edge_label_y_position_diagonal_edge() {
        // Test that edge labels are positioned at the midpoint for diagonal edges
        // This reproduces the bug where LR flowchart edge labels were positioned
        // at the source y-coordinate instead of the edge midpoint
        let mut graph = LayoutGraph::new("test_diagonal_label");
        graph.options.direction = LayoutDirection::LeftToRight;

        // Create a "decision" pattern: B has edges going to C (above) and D (below)
        graph.add_node(LayoutNode::new("A", 50.0, 30.0));
        graph.add_node(LayoutNode::new("B", 80.0, 80.0)); // Larger node like a diamond
        graph.add_node(LayoutNode::new("C", 50.0, 30.0));
        graph.add_node(LayoutNode::new("D", 50.0, 30.0));

        // A -> B (no label)
        graph.add_edge(LayoutEdge::new("e_ab", "A", "B"));
        // B -> C with "Yes" label
        graph.add_edge(LayoutEdge::new("e_bc", "B", "C").with_label("Yes"));
        // B -> D with "No" label
        graph.add_edge(LayoutEdge::new("e_bd", "B", "D").with_label("No"));

        let result = layout(graph).unwrap();

        // Get node positions
        let b = result.get_node("B").unwrap();
        let c = result.get_node("C").unwrap();
        let d = result.get_node("D").unwrap();

        let b_center_y = b.y.unwrap() + b.height / 2.0;
        let c_center_y = c.y.unwrap() + c.height / 2.0;
        let d_center_y = d.y.unwrap() + d.height / 2.0;

        eprintln!("Node B center y: {}", b_center_y);
        eprintln!("Node C center y: {}", c_center_y);
        eprintln!("Node D center y: {}", d_center_y);

        // Find the B->C edge
        let edge_bc = result
            .edges
            .iter()
            .find(|e| e.id == "e_bc")
            .expect("Should have edge B->C");
        let label_pos_bc = edge_bc
            .label_position
            .expect("Edge B->C should have label position");

        eprintln!("Edge B->C label y: {}", label_pos_bc.y);
        eprintln!("Edge B->C bend points: {:?}", edge_bc.bend_points);

        // The label y should be between B and C's y-coordinates, not at B's y
        // For a diagonal edge going from B to C, label should be near midpoint
        let min_y = b_center_y.min(c_center_y);
        let max_y = b_center_y.max(c_center_y);
        let midpoint_y = (b_center_y + c_center_y) / 2.0;

        // Allow some tolerance - label should be within the range, closer to midpoint
        // The bug was that labels were at source y, not midpoint
        assert!(
            label_pos_bc.y >= min_y - 10.0 && label_pos_bc.y <= max_y + 10.0,
            "Label y ({}) should be between B ({}) and C ({}) y-coordinates (with tolerance)",
            label_pos_bc.y,
            b_center_y,
            c_center_y
        );

        // More strict check: label should be reasonably close to the midpoint
        // If B and C are at the same y (range=0), this is valid - label at their y is correct
        let distance_from_midpoint = (label_pos_bc.y - midpoint_y).abs();
        let total_range = (max_y - min_y).abs();
        if total_range > 0.0 {
            assert!(
                distance_from_midpoint < total_range * 0.6,
                "Label y ({}) should be close to midpoint ({}), not at an extreme. Distance: {}, Range: {}",
                label_pos_bc.y,
                midpoint_y,
                distance_from_midpoint,
                total_range
            );
        } else {
            // When range is 0 (B and C at same y), label at that y is correct
            assert!(
                distance_from_midpoint < 5.0,
                "Label y ({}) should be at midpoint ({}) when nodes are at same y",
                label_pos_bc.y,
                midpoint_y
            );
        }
    }

    #[test]
    fn test_simple_chain_tb_alignment() {
        // Simple chain without back edges should be perfectly vertically aligned
        let mut graph = LayoutGraph::new("simple_chain");
        graph.options.direction = LayoutDirection::TopToBottom;
        graph.options.node_spacing = 50.0;
        graph.options.layer_spacing = 60.0;

        graph.add_node(LayoutNode::new("A", 80.0, 40.0));
        graph.add_node(LayoutNode::new("B", 80.0, 40.0));
        graph.add_node(LayoutNode::new("C", 80.0, 40.0));
        graph.add_node(LayoutNode::new("D", 80.0, 40.0));

        graph.add_edge(LayoutEdge::new("e1", "A", "B"));
        graph.add_edge(LayoutEdge::new("e2", "B", "C"));
        graph.add_edge(LayoutEdge::new("e3", "C", "D"));

        let result = layout(graph).unwrap();

        eprintln!("Simple chain layout:");
        for node in &result.nodes {
            eprintln!(
                "  {}: x={:.1}, y={:.1}",
                node.id,
                node.x.unwrap_or(0.0),
                node.y.unwrap_or(0.0)
            );
        }

        // All nodes should have the same x (within 1 pixel)
        let a_x = result.get_node("A").unwrap().x.unwrap();
        let b_x = result.get_node("B").unwrap().x.unwrap();
        let c_x = result.get_node("C").unwrap().x.unwrap();
        let d_x = result.get_node("D").unwrap().x.unwrap();

        assert!(
            (a_x - b_x).abs() < 1.0,
            "A ({:.1}) and B ({:.1}) should have same x",
            a_x,
            b_x
        );
        assert!(
            (b_x - c_x).abs() < 1.0,
            "B ({:.1}) and C ({:.1}) should have same x",
            b_x,
            c_x
        );
        assert!(
            (c_x - d_x).abs() < 1.0,
            "C ({:.1}) and D ({:.1}) should have same x",
            c_x,
            d_x
        );
    }

    #[test]
    fn test_state_diagram_pattern_tb_alignment() {
        // This test mimics the state diagram pattern:
        // Start -> Idle -> Running -> Error -> End
        // With back edges: Running -> Idle, Error -> Idle
        //
        // In TB layout, all nodes should be roughly vertically aligned
        // Back edges create dummy nodes but shouldn't significantly spread the layout
        let mut graph = LayoutGraph::new("state_pattern");
        graph.options.direction = LayoutDirection::TopToBottom;
        graph.options.node_spacing = 50.0;
        graph.options.layer_spacing = 60.0;

        // Add nodes (small circles for start/end, rectangles for states)
        graph.add_node(LayoutNode::new("Start", 24.0, 24.0).with_shape(NodeShape::Circle));
        graph.add_node(LayoutNode::new("Idle", 80.0, 40.0).with_shape(NodeShape::RoundedRect));
        graph.add_node(LayoutNode::new("Running", 80.0, 40.0).with_shape(NodeShape::RoundedRect));
        graph.add_node(LayoutNode::new("Error", 80.0, 40.0).with_shape(NodeShape::RoundedRect));
        graph.add_node(LayoutNode::new("End", 24.0, 24.0).with_shape(NodeShape::DoubleCircle));

        // Forward edges (main flow)
        graph.add_edge(LayoutEdge::new("e1", "Start", "Idle"));
        graph.add_edge(LayoutEdge::new("e2", "Idle", "Running").with_label("start"));
        graph.add_edge(LayoutEdge::new("e3", "Running", "Error").with_label("error"));
        graph.add_edge(LayoutEdge::new("e4", "Error", "End"));

        // Back edges (cycles)
        graph.add_edge(LayoutEdge::new("e5", "Running", "Idle").with_label("stop"));
        graph.add_edge(LayoutEdge::new("e6", "Error", "Idle").with_label("reset"));

        let result = layout(graph).unwrap();

        // Get x coordinates for main states (excluding start/end circles)
        let idle_x = result.get_node("Idle").unwrap().x.unwrap();
        let running_x = result.get_node("Running").unwrap().x.unwrap();
        let error_x = result.get_node("Error").unwrap().x.unwrap();

        // In TB layout with this structure, all states should be roughly aligned
        // Back edges create dummy nodes which can cause some horizontal offset
        // Allow up to 50 pixels tolerance (less than a full node width)
        let mean_x = (idle_x + running_x + error_x) / 3.0;
        let max_deviation = 50.0;

        assert!(
            (idle_x - mean_x).abs() < max_deviation,
            "Idle x ({:.1}) should be near mean ({:.1}). States should be vertically aligned in TB layout.",
            idle_x, mean_x
        );
        assert!(
            (running_x - mean_x).abs() < max_deviation,
            "Running x ({:.1}) should be near mean ({:.1}). States should be vertically aligned in TB layout.",
            running_x, mean_x
        );
        assert!(
            (error_x - mean_x).abs() < max_deviation,
            "Error x ({:.1}) should be near mean ({:.1}). States should be vertically aligned in TB layout.",
            error_x, mean_x
        );
    }

    #[test]
    fn test_bidirectional_edges_both_have_points() {
        // Test that edges A→B and B→A both get bend points after layout
        // This is important for state diagrams with transitions in both directions
        let mut graph = LayoutGraph::new("bidirectional");
        graph.options.direction = LayoutDirection::TopToBottom;
        graph.options.node_spacing = 50.0;
        graph.options.layer_spacing = 60.0;

        // Create nodes
        graph.add_node(LayoutNode::new("Idle", 60.0, 40.0));
        graph.add_node(LayoutNode::new("Running", 80.0, 40.0));

        // Create bidirectional edges
        graph.add_edge(LayoutEdge::new("forward", "Idle", "Running").with_label("start"));
        graph.add_edge(LayoutEdge::new("backward", "Running", "Idle").with_label("stop"));

        let result = layout(graph).unwrap();

        // Find both edges
        let forward_edge = result
            .edges
            .iter()
            .find(|e| e.id == "forward")
            .expect("Should have forward edge");
        let backward_edge = result
            .edges
            .iter()
            .find(|e| e.id == "backward")
            .expect("Should have backward edge");

        eprintln!(
            "Forward edge (Idle→Running) has {} bend points",
            forward_edge.bend_points.len()
        );
        eprintln!(
            "Backward edge (Running→Idle) has {} bend points",
            backward_edge.bend_points.len()
        );

        // Both edges should have at least 2 points (start and end)
        assert!(
            forward_edge.bend_points.len() >= 2,
            "Forward edge should have at least 2 bend points, got {}",
            forward_edge.bend_points.len()
        );
        assert!(
            backward_edge.bend_points.len() >= 2,
            "Backward edge should have at least 2 bend points, got {}",
            backward_edge.bend_points.len()
        );
    }

    #[test]
    fn test_dagre_graph_preserves_edge_order() {
        // Test that edge order is preserved when converting LayoutGraph to DagreGraph.
        // This is critical for fork/join ordering.
        let mut graph = LayoutGraph::new("test_edge_order");
        graph.options.direction = LayoutDirection::TopToBottom;

        // Create fork pattern
        graph.add_node(LayoutNode::new("fork", 70.0, 10.0)); // Fork bar
        graph.add_node(LayoutNode::new("first_target", 100.0, 40.0));
        graph.add_node(LayoutNode::new("second_target", 100.0, 40.0));

        // Add edges in specific order
        graph.add_edge(LayoutEdge::new("e1", "fork", "first_target")); // First
        graph.add_edge(LayoutEdge::new("e2", "fork", "second_target")); // Second

        // Convert to DagreGraph
        let dg = to_dagre_graph(&graph);

        // Check successors order
        let successors = dg.successors("fork");
        eprintln!("DagreGraph successors of fork: {:?}", successors);

        assert_eq!(successors.len(), 2, "Should have 2 successors");
        assert_eq!(
            successors[0], "first_target",
            "First successor should be first_target"
        );
        assert_eq!(
            successors[1], "second_target",
            "Second successor should be second_target"
        );
    }

    #[test]
    fn test_fork_layout_position_order() {
        // Test that fork targets are positioned in edge definition order.
        // First defined target should be on the LEFT (smaller x).
        let mut graph = LayoutGraph::new("test_fork_positions");
        graph.options.direction = LayoutDirection::TopToBottom;

        // Create fork pattern
        graph.add_node(LayoutNode::new("start", 50.0, 30.0));
        graph.add_node(LayoutNode::new("fork", 70.0, 10.0)); // Fork bar
        graph.add_node(LayoutNode::new("first_target", 100.0, 40.0));
        graph.add_node(LayoutNode::new("second_target", 100.0, 40.0));
        graph.add_node(LayoutNode::new("join", 70.0, 10.0)); // Join bar

        // Add edges in specific order
        graph.add_edge(LayoutEdge::new("e0", "start", "fork"));
        graph.add_edge(LayoutEdge::new("e1", "fork", "first_target")); // First fork edge
        graph.add_edge(LayoutEdge::new("e2", "fork", "second_target")); // Second fork edge
        graph.add_edge(LayoutEdge::new("e3", "first_target", "join"));
        graph.add_edge(LayoutEdge::new("e4", "second_target", "join"));

        // Run layout
        let result = layout(graph).expect("Layout should succeed");

        let first = result
            .get_node("first_target")
            .expect("Should have first_target");
        let second = result
            .get_node("second_target")
            .expect("Should have second_target");

        let first_x = first.x.expect("first_target should have x position");
        let second_x = second.x.expect("second_target should have x position");

        eprintln!(
            "Fork layout: first_target.x={}, second_target.x={}",
            first_x, second_x
        );

        // First defined edge target should be on the left (smaller x)
        assert!(
            first_x < second_x,
            "first_target (first edge) should be LEFT of second_target. \
             first_target.x={}, second_target.x={}",
            first_x,
            second_x
        );
    }

    #[test]
    fn test_fork_layout_alphabetical_order_reversed() {
        // Test fork layout when alphabetical order is OPPOSITE to edge definition order.
        // This matches the state diagram case where:
        // - Edge 1: fork_state -> Validation (first edge)
        // - Edge 2: fork_state -> ResourceAlloc (second edge)
        // Alphabetically: "ResourceAlloc" < "Validation" (R < V)
        // So if alphabetical sorting happens, ResourceAlloc would be placed first.
        //
        // We want edge definition order, so Validation should be on the LEFT.
        let mut graph = LayoutGraph::new("test_alphabetical");
        graph.options.direction = LayoutDirection::TopToBottom;

        // Use names where alphabetical order is opposite to edge order
        // ZZZ should be FIRST (edge order) but comes LAST alphabetically
        // AAA should be SECOND (edge order) but comes FIRST alphabetically
        graph.add_node(LayoutNode::new("start", 50.0, 30.0));
        graph.add_node(LayoutNode::new("fork", 70.0, 10.0));
        graph.add_node(LayoutNode::new("ZZZ", 100.0, 40.0)); // First edge target
        graph.add_node(LayoutNode::new("AAA", 100.0, 40.0)); // Second edge target
        graph.add_node(LayoutNode::new("join", 70.0, 10.0));

        // Add edges in specific order - ZZZ first, AAA second
        graph.add_edge(LayoutEdge::new("e0", "start", "fork"));
        graph.add_edge(LayoutEdge::new("e1", "fork", "ZZZ")); // First fork edge
        graph.add_edge(LayoutEdge::new("e2", "fork", "AAA")); // Second fork edge
        graph.add_edge(LayoutEdge::new("e3", "ZZZ", "join"));
        graph.add_edge(LayoutEdge::new("e4", "AAA", "join"));

        // Convert to DagreGraph and check intermediate state
        let dg = to_dagre_graph(&graph);
        eprintln!("DagreGraph successors of fork: {:?}", dg.successors("fork"));

        // Check init_order
        use crate::layout::dagre::order::{assign_order, init_order};
        use crate::layout::dagre::rank;
        let mut dg = to_dagre_graph(&graph);
        let config = to_dagre_config(&graph.options);
        rank::assign_ranks(&mut dg, config.ranker);

        let layering = init_order(&dg);
        eprintln!("init_order layer 2: {:?}", layering.get(2));

        assign_order(&mut dg, &layering);
        eprintln!(
            "After assign_order: ZZZ.order={:?}, AAA.order={:?}",
            dg.node("ZZZ").and_then(|n| n.order),
            dg.node("AAA").and_then(|n| n.order)
        );

        // Run layout step by step to trace where order gets lost
        let mut dg2 = to_dagre_graph(&graph);
        let config2 = to_dagre_config(&graph.options);
        crate::layout::dagre::layout(&mut dg2, &config2);

        eprintln!(
            "After dagre::layout: ZZZ.order={:?}, AAA.order={:?}",
            dg2.node("ZZZ").and_then(|n| n.order),
            dg2.node("AAA").and_then(|n| n.order)
        );
        eprintln!(
            "After dagre::layout: ZZZ.x={:?}, AAA.x={:?}",
            dg2.node("ZZZ").and_then(|n| n.x),
            dg2.node("AAA").and_then(|n| n.x)
        );

        // Run full layout
        let result = layout(graph).expect("Layout should succeed");

        let zzz = result.get_node("ZZZ").expect("Should have ZZZ");
        let aaa = result.get_node("AAA").expect("Should have AAA");

        let zzz_x = zzz.x.expect("ZZZ should have x position");
        let aaa_x = aaa.x.expect("AAA should have x position");

        eprintln!(
            "Fork layout (reversed alpha): ZZZ.x={}, AAA.x={}",
            zzz_x, aaa_x
        );

        // ZZZ (first defined edge target) should be on the LEFT (smaller x)
        // even though "AAA" < "ZZZ" alphabetically
        assert!(
            zzz_x < aaa_x,
            "ZZZ (first edge) should be LEFT of AAA even though A < Z alphabetically. \
             ZZZ.x={}, AAA.x={}",
            zzz_x,
            aaa_x
        );
    }
}
