//! Node ordering (crossing minimization) for dagre layout
//!
//! Port of dagre's lib/order/index.js. Applies heuristics to minimize edge
//! crossings in the graph and sets the best order solution as an order
//! attribute on each node.
//!
//! Each sweep builds a layer graph per rank (containing the movable layer
//! with its full subgraph hierarchy under a synthetic root, plus the fixed
//! neighbor layer and the edges between them) and sorts it recursively with
//! `sort_subgraph`, threading a single constraint graph across the layers of
//! the sweep to keep sibling subgraphs in a consistent relative order.

mod barycenter;
mod build_layer_graph;
mod cross_count;
mod init_order;
mod resolve_conflicts;
mod sort;
mod sort_subgraph;

use crate::layout::dagre::graph::DagreGraph;

pub use barycenter::{barycenter, BarycenterEntry};
pub use build_layer_graph::{build_layer_graph, Relationship};
pub use cross_count::cross_count;
pub use init_order::{assign_order, init_order};
pub use resolve_conflicts::{resolve_conflicts, ConstraintGraph, ResolvedEntry};
pub use sort::{sort, SortResult};
pub use sort_subgraph::{add_subgraph_constraints, sort_subgraph};

/// Assign order to nodes to minimize edge crossings
pub fn order(g: &mut DagreGraph) {
    // Get initial layering from DFS
    let layering = init_order(g);
    assign_order(g, &layering);

    // Find max rank for iteration (compound nodes have no rank)
    let max_rank = g
        .nodes()
        .iter()
        .filter_map(|v| g.node(v).and_then(|n| n.rank))
        .max()
        .unwrap_or(0) as usize;

    if max_rank == 0 {
        return; // Only one layer, no crossings possible
    }

    // dagre: downLayerGraphs use ranks 1..=maxRank with in-edges,
    // upLayerGraphs use ranks maxRank-1..=0 with out-edges.
    let down_ranks: Vec<i32> = (1..=max_rank as i32).collect();
    let up_ranks: Vec<i32> = (0..max_rank as i32).rev().collect();

    // Track best solution
    let mut best_cc = i32::MAX;
    let mut best_layering = layering.clone();

    // Iterate: alternate between up and down sweeps.
    // dagre: `for (let i = 0, lastBest = 0; lastBest < 4; ++i, ++lastBest)` —
    // note lastBest is ALSO incremented right after an improving iteration
    // (the body sets it to 0, the loop update bumps it to 1).
    let mut i = 0usize;
    let mut last_best = 0usize;
    while last_best < 4 {
        let bias_right = (i % 4) >= 2;

        // dagre: `sweepLayerGraphs(i % 2 ? downLayerGraphs : upLayerGraphs, ...)`
        if i % 2 == 1 {
            sweep_layer_graphs(g, &down_ranks, Relationship::InEdges, bias_right);
        } else {
            sweep_layer_graphs(g, &up_ranks, Relationship::OutEdges, bias_right);
        }

        // Build layering from current order
        let current_layering = build_layer_matrix(g, max_rank);
        let cc = cross_count(g, &current_layering);

        // dagre-d3-es (the dagre vendored by mermaid) only keeps a layering
        // that STRICTLY improves the crossing count; ties keep the earlier
        // layering. (Newer standalone dagre keeps the latest layering on
        // ties, but mermaid parity requires the dagre-d3-es behavior.)
        if cc < best_cc {
            best_cc = cc;
            best_layering = current_layering;
            last_best = 0;
        }

        i += 1;
        last_best += 1;
    }

    // Apply best ordering
    assign_order(g, &best_layering);
}

/// One sweep across the given ranks (dagre's sweepLayerGraphs)
///
/// A single constraint graph is threaded across all layer graphs of the
/// sweep; each sorted layer extends it with constraints between sibling
/// subgraphs so later layers keep them in a consistent relative order.
///
/// dagre builds all layer graphs up front and mutates shared node labels; we
/// build each rank's layer graph on demand so it reflects the orders
/// assigned to the previous layer.
fn sweep_layer_graphs(
    g: &mut DagreGraph,
    ranks: &[i32],
    relationship: Relationship,
    bias_right: bool,
) {
    let mut cg = ConstraintGraph::new();

    for &rank in ranks {
        let (lg, root) = build_layer_graph(g, rank, relationship);
        let sorted = sort_subgraph(&lg, &root, &cg, bias_right);

        for (i, v) in sorted.vs.iter().enumerate() {
            if let Some(node) = g.node_mut(v) {
                node.order = Some(i);
            }
        }

        add_subgraph_constraints(&lg, &mut cg, &sorted.vs);
    }
}

/// Build layer matrix from current order assignments
fn build_layer_matrix(g: &DagreGraph, max_rank: usize) -> Vec<Vec<String>> {
    let mut layers: Vec<Vec<(String, usize)>> = (0..=max_rank).map(|_| Vec::new()).collect();

    for v in g.nodes() {
        if let Some(node) = g.node(v) {
            if let (Some(rank), Some(order)) = (node.rank, node.order) {
                if rank >= 0 && (rank as usize) <= max_rank {
                    layers[rank as usize].push((v.clone(), order));
                }
            }
        }
    }

    // Sort each layer by order
    for layer in &mut layers {
        layer.sort_by_key(|(_, order)| *order);
    }

    layers
        .into_iter()
        .map(|layer| layer.into_iter().map(|(v, _)| v).collect())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::dagre::graph::EdgeLabel;
    use crate::layout::dagre::rank;
    use crate::layout::dagre::Ranker;

    #[test]
    fn test_order_single_node() {
        let mut g = DagreGraph::new();
        g.set_node("a", Default::default());
        rank::assign_ranks(&mut g, Ranker::LongestPath);

        order(&mut g);

        assert_eq!(g.node("a").unwrap().order, Some(0));
    }

    #[test]
    fn test_order_chain() {
        let mut g = DagreGraph::new();
        g.set_path(&["a", "b", "c"]);
        rank::assign_ranks(&mut g, Ranker::LongestPath);

        order(&mut g);

        assert_eq!(g.node("a").unwrap().order, Some(0));
        assert_eq!(g.node("b").unwrap().order, Some(0));
        assert_eq!(g.node("c").unwrap().order, Some(0));
    }

    #[test]
    fn test_order_diamond() {
        let mut g = DagreGraph::new();
        g.set_path(&["a", "b", "d"]);
        g.set_path(&["a", "c", "d"]);
        rank::assign_ranks(&mut g, Ranker::LongestPath);

        order(&mut g);

        // a and d should be at order 0 (only nodes in their layers)
        assert_eq!(g.node("a").unwrap().order, Some(0));
        assert_eq!(g.node("d").unwrap().order, Some(0));

        // b and c should have different orders
        let b_order = g.node("b").unwrap().order;
        let c_order = g.node("c").unwrap().order;
        assert!(b_order.is_some());
        assert!(c_order.is_some());
        assert_ne!(b_order, c_order);
    }

    #[test]
    fn test_order_tree_has_no_crossings() {
        // dagre order-test.js "does not add crossings to a tree structure"
        let mut g = DagreGraph::new();
        g.set_path(&["a", "b", "c"]);
        g.set_edge("b", "d", EdgeLabel::default());
        g.set_path(&["a", "e", "f"]);
        rank::assign_ranks(&mut g, Ranker::LongestPath);

        order(&mut g);

        let max_rank = g
            .nodes()
            .iter()
            .filter_map(|v| g.node(v).and_then(|n| n.rank))
            .max()
            .unwrap() as usize;
        let layering = build_layer_matrix(&g, max_rank);
        assert_eq!(cross_count(&g, &layering), 0);
    }

    #[test]
    fn test_order_minimizes_crossings() {
        // Create a graph where initial order has crossings:
        // a   b
        //  \ /
        //   X
        //  / \
        // c   d
        // If a->d, b->c, there's a crossing
        // Optimal order: either swap a,b or swap c,d

        let mut g = DagreGraph::new();
        g.set_edge("a", "d", EdgeLabel::default());
        g.set_edge("b", "c", EdgeLabel::default());
        rank::assign_ranks(&mut g, Ranker::LongestPath);

        // Force initial order with crossing
        if let Some(node) = g.node_mut("a") {
            node.order = Some(0);
        }
        if let Some(node) = g.node_mut("b") {
            node.order = Some(1);
        }
        if let Some(node) = g.node_mut("c") {
            node.order = Some(0);
        }
        if let Some(node) = g.node_mut("d") {
            node.order = Some(1);
        }

        let initial_layering = vec![
            vec!["a".to_string(), "b".to_string()],
            vec!["c".to_string(), "d".to_string()],
        ];
        let initial_crossings = cross_count(&g, &initial_layering);

        order(&mut g);

        let final_layering = build_layer_matrix(&g, 1);
        let final_crossings = cross_count(&g, &final_layering);

        // After ordering, crossings should be reduced (ideally to 0)
        assert!(final_crossings <= initial_crossings);
    }

    #[test]
    fn test_order_decision_branches_match_reference() {
        // Simulates the flowchart:
        //   B -->|Yes| C[Action 1]
        //   B -->|No| D[Action 2]
        //
        // dagre-d3-es (mermaid's dagre) keeps the FIRST layering with the
        // best crossing count, so the initial [C, D] order survives the
        // later bias-right sweeps: C order 0, D order 1 (first edge target
        // on the left, as mermaid renders it).
        let mut g = DagreGraph::new();

        // Add edges in specific order - C first, then D
        g.set_edge("B", "C", EdgeLabel::default()); // "Yes" branch
        g.set_edge("B", "D", EdgeLabel::default()); // "No" branch

        rank::assign_ranks(&mut g, Ranker::LongestPath);
        order(&mut g);

        assert_eq!(
            g.node("C").unwrap().order,
            Some(0),
            "C (first edge target) must keep order 0 like dagre-d3-es"
        );
        assert_eq!(
            g.node("D").unwrap().order,
            Some(1),
            "D (second edge target) must keep order 1 like dagre-d3-es"
        );
    }

    #[test]
    fn test_order_fork_pattern_matches_reference() {
        // Fork pattern like state diagrams:
        // start -> fork -> first_target
        //              \-> second_target
        // -> join
        //
        // dagre-d3-es (mermaid's dagre) keeps the FIRST layering with the
        // best crossing count, so the initial edge-definition order wins:
        // first_target order 0, second_target order 1.
        let mut g = DagreGraph::new();

        g.set_edge("start", "fork", EdgeLabel::default());
        g.set_edge("fork", "first_target", EdgeLabel::default()); // First fork edge
        g.set_edge("fork", "second_target", EdgeLabel::default()); // Second fork edge
        g.set_edge("first_target", "join", EdgeLabel::default());
        g.set_edge("second_target", "join", EdgeLabel::default());

        rank::assign_ranks(&mut g, Ranker::LongestPath);
        order(&mut g);

        assert_eq!(
            g.node("first_target").unwrap().order,
            Some(0),
            "first_target (first edge) must keep order 0 like dagre-d3-es"
        );
        assert_eq!(
            g.node("second_target").unwrap().order,
            Some(1),
            "second_target (second edge) must keep order 1 like dagre-d3-es"
        );
    }
}
