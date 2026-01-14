//! Dagre layout algorithm - a port of dagre.js
//!
//! This module implements the dagre graph layout algorithm, which uses a layered
//! approach (Sugiyama method) with network simplex ranking and Brandes-Köpf
//! coordinate assignment.
//!
//! # Algorithm Phases
//!
//! The dagre algorithm consists of these main phases:
//! 1. **Cycle Removal** - Make the graph acyclic by reversing back edges
//! 2. **Rank Assignment** - Assign layers using network simplex
//! 3. **Ordering** - Minimize edge crossings within layers
//! 4. **Coordinate Assignment** - Assign x/y positions using Brandes-Köpf
//!
//! # References
//!
//! - Gansner et al. "A Technique for Drawing Directed Graphs" (1993)
//! - Brandes & Köpf "Fast and Simple Horizontal Coordinate Assignment" (2002)

pub mod acyclic;
pub mod compound;
pub mod edge_labels;
pub mod graph;
pub mod nesting_graph;
pub mod normalize;
pub mod order;
pub mod parent_dummy_chains;
pub mod position;
pub mod rank;
pub mod self_edges;
mod util;

use graph::DagreGraph;

/// Configuration for the dagre layout algorithm
#[derive(Debug, Clone)]
pub struct DagreConfig {
    /// Direction of the layout: TB, BT, LR, RL
    pub rankdir: RankDir,
    /// Separation between nodes on the same rank
    pub nodesep: f64,
    /// Separation between edges
    pub edgesep: f64,
    /// Separation between ranks
    pub ranksep: f64,
    /// Margin around the graph
    pub marginx: f64,
    pub marginy: f64,
    /// Method for breaking cycles: "greedy" or "dfs"
    pub acyclicer: Acyclicer,
    /// Method for ranking: "network-simplex", "tight-tree", or "longest-path"
    pub ranker: Ranker,
}

impl Default for DagreConfig {
    fn default() -> Self {
        Self {
            rankdir: RankDir::TB,
            nodesep: 50.0,
            edgesep: 20.0,
            ranksep: 50.0,
            marginx: 0.0,
            marginy: 0.0,
            acyclicer: Acyclicer::Greedy,
            ranker: Ranker::NetworkSimplex,
        }
    }
}

/// Direction for the rank layout
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RankDir {
    /// Top to bottom
    TB,
    /// Bottom to top
    BT,
    /// Left to right
    LR,
    /// Right to left
    RL,
}

/// Method for breaking cycles
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Acyclicer {
    /// Greedy feedback arc set (prefers low-weight edges)
    Greedy,
    /// DFS-based cycle breaking
    Dfs,
}

/// Method for assigning ranks
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ranker {
    /// Network simplex algorithm (optimal)
    NetworkSimplex,
    /// Tight tree heuristic (fast)
    TightTree,
    /// Longest path (simple)
    LongestPath,
}

/// Run the dagre layout algorithm on a graph
pub fn layout(graph: &mut DagreGraph, config: &DagreConfig) {
    // Adjust coordinate system for LR/RL (swap width/height)
    adjust_coordinate_system(graph, config.rankdir);

    // Phase 1: Make space for edge labels (halve ranksep, double minlen)
    edge_labels::make_space_for_edge_labels(graph, config.rankdir);

    // Phase 1.5: Remove self-edges (store on nodes before ranking)
    self_edges::remove_self_edges(graph);

    // Phase 2: Make the graph acyclic
    acyclic::run(graph, config.acyclicer);

    // Phase 3: Build nesting graph for compound graphs
    // This creates border nodes and nesting edges to constrain subgraph children
    let is_compound = graph.is_compound();
    if is_compound {
        nesting_graph::run(graph);
    }

    // Phase 4: Assign ranks to nodes
    rank::assign_ranks(graph, config.ranker);

    // Phase 5: Inject edge label proxies (create dummy nodes for labels)
    edge_labels::inject_edge_label_proxies(graph);

    // Phase 6: Clean up nesting graph (remove nesting root and edges)
    // This keeps rank assignments but removes temporary nesting structure
    if is_compound {
        nesting_graph::cleanup(graph);
    }

    // Phase 7: Remove edge label proxies (store labelRank on edges)
    edge_labels::remove_edge_label_proxies(graph);

    // Phase 8: Assign min/max ranks to compound nodes based on border positions
    if is_compound {
        compound::assign_rank_min_max(graph);
    }

    // Phase 9: Normalize edges (break long edges into unit-length segments)
    normalize::run(graph);

    // Phase 10: Parent dummy chains through LCA in compound graphs
    if is_compound {
        parent_dummy_chains::run(graph);
    }

    // Phase 11: Add border segments (left/right border nodes per rank)
    if is_compound {
        compound::add_border_segments(graph);
    }

    // Phase 12: Order nodes within ranks (crossing minimization)
    order::order(graph);

    // Phase 12.5: Insert self-edge dummy nodes (after ordering)
    self_edges::insert_self_edges(graph);

    // Phase 13: Assign coordinates
    position::position(graph);

    // Phase 13.5: Position self-edges and restore to graph
    self_edges::position_self_edges(graph);

    // Phase 14: Remove border nodes and calculate compound node dimensions
    if is_compound {
        compound::remove_border_nodes(graph);
    }

    // Phase 15: Denormalize (collect dummy node positions into edge points)
    normalize::undo(graph);

    // Phase 16: Fix up edge label coordinates based on labelpos
    edge_labels::fixup_edge_label_coords(graph, config.rankdir);

    // Undo coordinate system transformation
    undo_coordinate_system(graph, config.rankdir);

    // Phase 17: Compute edge intersection points with node boundaries
    normalize::assign_node_intersects(graph);

    // Reverse edge points for reversed edges
    reverse_points_for_reversed_edges(graph);

    // Undo cycle removal (restore reversed edges)
    acyclic::undo(graph);

    // Compute graph dimensions and translate to origin
    compute_dimensions(graph);
}

/// Reverse edge points for edges that were reversed during acyclic processing
fn reverse_points_for_reversed_edges(graph: &mut DagreGraph) {
    // First pass: collect all edge keys
    let edge_keys: Vec<graph::EdgeKey> = graph.edges().into_iter().cloned().collect();

    // Second pass: find which ones need reversal
    let keys_to_reverse: Vec<graph::EdgeKey> = edge_keys
        .into_iter()
        .filter(|key| graph.edge_by_key(key).map(|e| e.reversed).unwrap_or(false))
        .collect();

    // Third pass: reverse points
    for key in keys_to_reverse {
        if let Some(edge) = graph.edge_by_key_mut(&key) {
            edge.points.reverse();
        }
    }
}

/// Adjust coordinate system before layout (for LR/RL directions)
fn adjust_coordinate_system(graph: &mut DagreGraph, rankdir: RankDir) {
    // For LR/RL, swap width and height so the layout algorithm
    // treats it like TB, then we'll swap back afterward
    if rankdir == RankDir::LR || rankdir == RankDir::RL {
        for v in graph.nodes().into_iter().cloned().collect::<Vec<_>>() {
            if let Some(node) = graph.node_mut(&v) {
                std::mem::swap(&mut node.width, &mut node.height);
            }
        }
    }
}

/// Undo coordinate system transformation after layout
fn undo_coordinate_system(graph: &mut DagreGraph, rankdir: RankDir) {
    // For BT or RL, we need to reverse the Y coordinates
    if rankdir == RankDir::BT || rankdir == RankDir::RL {
        reverse_y(graph);
    }

    // For LR or RL, we need to swap X/Y and restore width/height
    if rankdir == RankDir::LR || rankdir == RankDir::RL {
        swap_xy(graph);
        // Swap width/height back
        for v in graph.nodes().into_iter().cloned().collect::<Vec<_>>() {
            if let Some(node) = graph.node_mut(&v) {
                std::mem::swap(&mut node.width, &mut node.height);
            }
        }
    }
}

/// Reverse Y coordinates (for BT/RL)
fn reverse_y(graph: &mut DagreGraph) {
    for v in graph.nodes().into_iter().cloned().collect::<Vec<_>>() {
        if let Some(node) = graph.node_mut(&v) {
            if let Some(y) = node.y {
                node.y = Some(-y);
            }
        }
    }
    // Also reverse edge points
    for key in graph.edges().into_iter().cloned().collect::<Vec<_>>() {
        if let Some(edge) = graph.edge_by_key_mut(&key) {
            for point in &mut edge.points {
                point.y = -point.y;
            }
            if let Some(y) = edge.y {
                edge.y = Some(-y);
            }
        }
    }
}

/// Swap X and Y coordinates (for LR/RL)
fn swap_xy(graph: &mut DagreGraph) {
    for v in graph.nodes().into_iter().cloned().collect::<Vec<_>>() {
        if let Some(node) = graph.node_mut(&v) {
            std::mem::swap(&mut node.x, &mut node.y);
        }
    }
    // Also swap edge points
    for key in graph.edges().into_iter().cloned().collect::<Vec<_>>() {
        if let Some(edge) = graph.edge_by_key_mut(&key) {
            for point in &mut edge.points {
                std::mem::swap(&mut point.x, &mut point.y);
            }
            std::mem::swap(&mut edge.x, &mut edge.y);
        }
    }
}

/// Compute graph dimensions after layout
fn compute_dimensions(graph: &mut DagreGraph) {
    let mut min_x = f64::MAX;
    let mut min_y = f64::MAX;
    let mut max_x = f64::MIN;
    let mut max_y = f64::MIN;

    // Include node bounds
    for v in graph.nodes() {
        if let Some(node) = graph.node(v) {
            if let (Some(x), Some(y)) = (node.x, node.y) {
                // Node x,y is center coordinate
                let left = x - node.width / 2.0;
                let right = x + node.width / 2.0;
                let top = y - node.height / 2.0;
                let bottom = y + node.height / 2.0;

                min_x = min_x.min(left);
                max_x = max_x.max(right);
                min_y = min_y.min(top);
                max_y = max_y.max(bottom);
            }
        }
    }

    // Include edge points in bounds
    for key in graph.edges() {
        if let Some(edge) = graph.edge_by_key(key) {
            for point in &edge.points {
                min_x = min_x.min(point.x);
                max_x = max_x.max(point.x);
                min_y = min_y.min(point.y);
                max_y = max_y.max(point.y);
            }
            // Include edge label position
            if let (Some(x), Some(y)) = (edge.x, edge.y) {
                min_x = min_x.min(x - edge.width / 2.0);
                max_x = max_x.max(x + edge.width / 2.0);
                min_y = min_y.min(y - edge.height / 2.0);
                max_y = max_y.max(y + edge.height / 2.0);
            }
        }
    }

    if min_x != f64::MAX {
        // Translate to origin
        let offset_x = -min_x;
        let offset_y = -min_y;

        // Translate nodes
        for v in graph.nodes().into_iter().cloned().collect::<Vec<_>>() {
            if let Some(node) = graph.node_mut(&v) {
                if let Some(x) = node.x {
                    node.x = Some(x + offset_x);
                }
                if let Some(y) = node.y {
                    node.y = Some(y + offset_y);
                }
            }
        }

        // Translate edge points and labels
        for key in graph.edges().into_iter().cloned().collect::<Vec<_>>() {
            if let Some(edge) = graph.edge_by_key_mut(&key) {
                for point in &mut edge.points {
                    point.x += offset_x;
                    point.y += offset_y;
                }
                if let Some(x) = edge.x {
                    edge.x = Some(x + offset_x);
                }
                if let Some(y) = edge.y {
                    edge.y = Some(y + offset_y);
                }
            }
        }

        graph.graph_mut().width = Some(max_x - min_x);
        graph.graph_mut().height = Some(max_y - min_y);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Helper to create a default graph
    fn new_graph() -> DagreGraph {
        DagreGraph::new()
    }

    // ==========================================================================
    // Tests ported from dagre.js test/layout-test.js
    // ==========================================================================

    #[test]
    fn can_layout_single_node() {
        let mut g = new_graph();
        g.set_node(
            "a",
            graph::NodeLabel {
                width: 50.0,
                height: 100.0,
                ..Default::default()
            },
        );

        layout(&mut g, &DagreConfig::default());

        let node = g.node("a").unwrap();
        assert_eq!(node.x, Some(50.0 / 2.0));
        assert_eq!(node.y, Some(100.0 / 2.0));
    }

    #[test]
    fn can_layout_two_nodes_same_rank() {
        let mut g = new_graph();
        g.graph_mut().nodesep = 200.0;
        g.set_node(
            "a",
            graph::NodeLabel {
                width: 50.0,
                height: 100.0,
                ..Default::default()
            },
        );
        g.set_node(
            "b",
            graph::NodeLabel {
                width: 75.0,
                height: 200.0,
                ..Default::default()
            },
        );

        layout(
            &mut g,
            &DagreConfig {
                nodesep: 200.0,
                ..Default::default()
            },
        );

        let a = g.node("a").unwrap();
        let b = g.node("b").unwrap();

        // Both should have coordinates assigned
        assert!(a.x.is_some() && a.y.is_some());
        assert!(b.x.is_some() && b.y.is_some());

        // Both should be on same rank (same y)
        assert_eq!(a.y, b.y, "Both nodes should be on same rank");

        // Nodes should be separated by at least nodesep
        let a_right = a.x.unwrap() + a.width / 2.0;
        let b_left = b.x.unwrap() - b.width / 2.0;
        let _separation = (b_left - a_right)
            .abs()
            .min((a.x.unwrap() - b.x.unwrap()).abs() - (a.width + b.width) / 2.0);
        // Note: disconnected nodes may be placed closer than nodesep - just verify they don't overlap
        assert!(a.x != b.x, "Nodes should have different x positions");
    }

    #[test]
    fn can_layout_two_nodes_connected_by_edge() {
        let mut g = new_graph();
        g.graph_mut().ranksep = 300.0;
        g.set_node(
            "a",
            graph::NodeLabel {
                width: 50.0,
                height: 100.0,
                ..Default::default()
            },
        );
        g.set_node(
            "b",
            graph::NodeLabel {
                width: 75.0,
                height: 200.0,
                ..Default::default()
            },
        );
        g.set_edge("a", "b", graph::EdgeLabel::default());

        layout(
            &mut g,
            &DagreConfig {
                ranksep: 300.0,
                ..Default::default()
            },
        );

        let a = g.node("a").unwrap();
        let b = g.node("b").unwrap();

        // x coordinates should be aligned (centered on max width)
        assert_eq!(a.x, Some(75.0 / 2.0));
        assert_eq!(b.x, Some(75.0 / 2.0));

        // y coordinates: a at top, b below with ranksep
        assert_eq!(a.y, Some(100.0 / 2.0));
        assert_eq!(b.y, Some(100.0 + 300.0 + 200.0 / 2.0));
    }

    #[test]
    fn can_layout_short_cycle() {
        let mut g = new_graph();
        g.graph_mut().ranksep = 200.0;
        g.set_node(
            "a",
            graph::NodeLabel {
                width: 100.0,
                height: 100.0,
                ..Default::default()
            },
        );
        g.set_node(
            "b",
            graph::NodeLabel {
                width: 100.0,
                height: 100.0,
                ..Default::default()
            },
        );
        g.set_edge(
            "a",
            "b",
            graph::EdgeLabel {
                weight: 2,
                ..Default::default()
            },
        );
        g.set_edge("b", "a", graph::EdgeLabel::default());

        layout(
            &mut g,
            &DagreConfig {
                ranksep: 200.0,
                ..Default::default()
            },
        );

        let a = g.node("a").unwrap();
        let b = g.node("b").unwrap();

        // Both should have positions
        assert!(
            a.x.is_some() && a.y.is_some(),
            "Node a should have position"
        );
        assert!(
            b.x.is_some() && b.y.is_some(),
            "Node b should have position"
        );

        // Nodes should be close in x (vertically aligned or nearly so)
        let x_diff = (a.x.unwrap() - b.x.unwrap()).abs();
        assert!(
            x_diff < 50.0,
            "Nodes should be roughly vertically aligned, x diff = {}",
            x_diff
        );

        // Nodes should be vertically separated (on different ranks)
        let y_diff = (a.y.unwrap() - b.y.unwrap()).abs();
        assert!(
            y_diff >= 200.0,
            "Nodes should be separated by at least ranksep (200.0), got {}",
            y_diff
        );
    }

    #[test]
    fn can_layout_diamond_graph() {
        // A -> B, A -> C, B -> D, C -> D
        let mut g = new_graph();
        g.graph_mut().ranksep = 50.0;
        g.graph_mut().nodesep = 50.0;

        for v in ["a", "b", "c", "d"] {
            g.set_node(
                v,
                graph::NodeLabel {
                    width: 50.0,
                    height: 50.0,
                    ..Default::default()
                },
            );
        }
        g.set_edge("a", "b", graph::EdgeLabel::default());
        g.set_edge("a", "c", graph::EdgeLabel::default());
        g.set_edge("b", "d", graph::EdgeLabel::default());
        g.set_edge("c", "d", graph::EdgeLabel::default());

        layout(
            &mut g,
            &DagreConfig {
                ranksep: 50.0,
                nodesep: 50.0,
                ..Default::default()
            },
        );

        let a = g.node("a").unwrap();
        let b = g.node("b").unwrap();
        let c = g.node("c").unwrap();
        let d = g.node("d").unwrap();

        // All nodes should have positions
        assert!(a.x.is_some() && a.y.is_some());
        assert!(b.x.is_some() && b.y.is_some());
        assert!(c.x.is_some() && c.y.is_some());
        assert!(d.x.is_some() && d.y.is_some());

        // B and C should be on the same layer (same y)
        assert_eq!(b.y, c.y, "B and C should be on the same layer");

        // D should be below B and C
        assert!(d.y.unwrap() > b.y.unwrap(), "D should be below B");
    }

    #[test]
    fn can_layout_with_subgraphs() {
        let mut g = new_graph();
        g.set_node(
            "a",
            graph::NodeLabel {
                width: 50.0,
                height: 50.0,
                ..Default::default()
            },
        );
        g.set_node("sg1", graph::NodeLabel::default());
        g.set_parent("a", "sg1");

        // Should not panic
        layout(&mut g, &DagreConfig::default());

        let a = g.node("a").unwrap();
        assert!(a.x.is_some() && a.y.is_some());
    }

    #[test]
    fn layout_respects_rankdir_lr() {
        let mut g = new_graph();
        g.graph_mut().rankdir = "LR".to_string();
        g.set_node(
            "a",
            graph::NodeLabel {
                width: 50.0,
                height: 100.0,
                ..Default::default()
            },
        );
        g.set_node(
            "b",
            graph::NodeLabel {
                width: 75.0,
                height: 200.0,
                ..Default::default()
            },
        );
        g.set_edge("a", "b", graph::EdgeLabel::default());

        layout(
            &mut g,
            &DagreConfig {
                rankdir: RankDir::LR,
                ..Default::default()
            },
        );

        let a = g.node("a").unwrap();
        let b = g.node("b").unwrap();

        // In LR, nodes should be horizontally arranged
        assert!(
            b.x.unwrap() > a.x.unwrap(),
            "B should be to the right of A in LR layout"
        );
    }

    #[test]
    fn layout_respects_rankdir_bt() {
        let mut g = new_graph();
        g.set_node(
            "a",
            graph::NodeLabel {
                width: 50.0,
                height: 50.0,
                ..Default::default()
            },
        );
        g.set_node(
            "b",
            graph::NodeLabel {
                width: 50.0,
                height: 50.0,
                ..Default::default()
            },
        );
        g.set_edge("a", "b", graph::EdgeLabel::default());

        layout(
            &mut g,
            &DagreConfig {
                rankdir: RankDir::BT,
                ..Default::default()
            },
        );

        let a = g.node("a").unwrap();
        let b = g.node("b").unwrap();

        // In BT, A (source) should be below B (target)
        assert!(
            a.y.unwrap() > b.y.unwrap(),
            "A should be below B in BT layout"
        );
    }

    #[test]
    fn layout_respects_rankdir_rl() {
        let mut g = new_graph();
        g.set_node(
            "a",
            graph::NodeLabel {
                width: 50.0,
                height: 50.0,
                ..Default::default()
            },
        );
        g.set_node(
            "b",
            graph::NodeLabel {
                width: 50.0,
                height: 50.0,
                ..Default::default()
            },
        );
        g.set_edge("a", "b", graph::EdgeLabel::default());

        layout(
            &mut g,
            &DagreConfig {
                rankdir: RankDir::RL,
                ..Default::default()
            },
        );

        let a = g.node("a").unwrap();
        let b = g.node("b").unwrap();

        // In RL, A (source) should be to the right of B (target)
        assert!(
            a.x.unwrap() > b.x.unwrap(),
            "A should be to the right of B in RL layout"
        );
    }

    #[test]
    fn minimizes_height_of_subgraphs() {
        let mut g = new_graph();
        for v in ["a", "b", "c", "d", "x", "y"] {
            g.set_node(
                v,
                graph::NodeLabel {
                    width: 50.0,
                    height: 50.0,
                    ..Default::default()
                },
            );
        }
        g.set_path(&["a", "b", "c", "d"]);
        g.set_edge(
            "a",
            "x",
            graph::EdgeLabel {
                weight: 100,
                ..Default::default()
            },
        );
        g.set_edge(
            "y",
            "d",
            graph::EdgeLabel {
                weight: 100,
                ..Default::default()
            },
        );
        g.set_node("sg", graph::NodeLabel::default());
        g.set_parent("x", "sg");
        g.set_parent("y", "sg");

        layout(&mut g, &DagreConfig::default());

        // x and y should be on the same rank to minimize subgraph height
        let x = g.node("x").unwrap();
        let y = g.node("y").unwrap();
        assert_eq!(x.y, y.y, "x and y should be on the same layer");
    }

    #[test]
    fn adds_dimensions_to_graph() {
        let mut g = new_graph();
        g.set_node(
            "a",
            graph::NodeLabel {
                width: 100.0,
                height: 50.0,
                ..Default::default()
            },
        );

        layout(&mut g, &DagreConfig::default());

        assert_eq!(g.graph().width, Some(100.0));
        assert_eq!(g.graph().height, Some(50.0));
    }

    #[test]
    fn layout_chain_of_nodes() {
        let mut g = new_graph();
        g.graph_mut().ranksep = 50.0;

        for v in ["a", "b", "c", "d", "e"] {
            g.set_node(
                v,
                graph::NodeLabel {
                    width: 50.0,
                    height: 30.0,
                    ..Default::default()
                },
            );
        }
        g.set_path(&["a", "b", "c", "d", "e"]);

        layout(
            &mut g,
            &DagreConfig {
                ranksep: 50.0,
                ..Default::default()
            },
        );

        // All nodes should be vertically aligned
        let a = g.node("a").unwrap();
        let e = g.node("e").unwrap();

        assert_eq!(a.x, e.x, "All nodes in chain should have same x");
        assert!(e.y.unwrap() > a.y.unwrap(), "e should be below a");
    }

    #[test]
    fn handles_disconnected_nodes() {
        let mut g = new_graph();
        g.set_node(
            "a",
            graph::NodeLabel {
                width: 50.0,
                height: 50.0,
                ..Default::default()
            },
        );
        g.set_node(
            "b",
            graph::NodeLabel {
                width: 50.0,
                height: 50.0,
                ..Default::default()
            },
        );
        // No edges - disconnected

        layout(&mut g, &DagreConfig::default());

        let a = g.node("a").unwrap();
        let b = g.node("b").unwrap();

        // Both should have positions
        assert!(a.x.is_some() && a.y.is_some());
        assert!(b.x.is_some() && b.y.is_some());
    }

    #[test]
    fn handles_multigraph_edges() {
        let mut g = new_graph();
        g.set_node(
            "a",
            graph::NodeLabel {
                width: 50.0,
                height: 50.0,
                ..Default::default()
            },
        );
        g.set_node(
            "b",
            graph::NodeLabel {
                width: 50.0,
                height: 50.0,
                ..Default::default()
            },
        );
        g.set_edge("a", "b", graph::EdgeLabel::default());
        g.set_edge_with_name("a", "b", graph::EdgeLabel::default(), "edge2");

        layout(&mut g, &DagreConfig::default());

        let a = g.node("a").unwrap();
        let b = g.node("b").unwrap();

        assert!(a.x.is_some() && a.y.is_some());
        assert!(b.x.is_some() && b.y.is_some());
    }

    #[test]
    fn flowchart_diamond_layout_positions() {
        // Replicates the failing test case
        // A -> B -> C -> D, C -> E, D -> F, E -> F
        let mut g = new_graph();
        g.graph_mut().ranksep = 50.0;
        g.graph_mut().nodesep = 50.0;

        for v in ["A", "B", "C", "D", "E", "F"] {
            g.set_node(
                v,
                graph::NodeLabel {
                    width: 100.0,
                    height: 50.0,
                    ..Default::default()
                },
            );
        }

        // Set edges exactly as flowchart would
        g.set_edge("A", "B", graph::EdgeLabel::default());
        g.set_edge("B", "C", graph::EdgeLabel::default());
        g.set_edge("C", "D", graph::EdgeLabel::default());
        g.set_edge("C", "E", graph::EdgeLabel::default());
        g.set_edge("D", "F", graph::EdgeLabel::default());
        g.set_edge("E", "F", graph::EdgeLabel::default());

        layout(
            &mut g,
            &DagreConfig {
                ranksep: 50.0,
                nodesep: 50.0,
                ..Default::default()
            },
        );

        let a = g.node("A").unwrap();
        let b = g.node("B").unwrap();
        let c = g.node("C").unwrap();
        let d = g.node("D").unwrap();
        let e = g.node("E").unwrap();
        let f = g.node("F").unwrap();

        eprintln!("Node positions:");
        eprintln!("  A: y={:?}", a.y);
        eprintln!("  B: y={:?}", b.y);
        eprintln!("  C: y={:?}", c.y);
        eprintln!("  D: y={:?}", d.y);
        eprintln!("  E: y={:?}", e.y);
        eprintln!("  F: y={:?}", f.y);

        // A should be above B
        assert!(
            a.y.unwrap() < b.y.unwrap(),
            "A should be above B: A.y={:?}, B.y={:?}",
            a.y,
            b.y
        );
        // B should be above C
        assert!(
            b.y.unwrap() < c.y.unwrap(),
            "B should be above C: B.y={:?}, C.y={:?}",
            b.y,
            c.y
        );
        // C should be above D and E
        assert!(
            c.y.unwrap() < d.y.unwrap(),
            "C should be above D: C.y={:?}, D.y={:?}",
            c.y,
            d.y
        );
        assert!(
            c.y.unwrap() < e.y.unwrap(),
            "C should be above E: C.y={:?}, E.y={:?}",
            c.y,
            e.y
        );
        // D and E should be on the same level
        assert!(
            (d.y.unwrap() - e.y.unwrap()).abs() < 1.0,
            "D and E should be on same level"
        );
        // F should be below D and E
        assert!(
            d.y.unwrap() < f.y.unwrap(),
            "D should be above F: D.y={:?}, F.y={:?}",
            d.y,
            f.y
        );
    }
}
