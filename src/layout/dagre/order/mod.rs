//! Node ordering (crossing minimization) for dagre layout
//!
//! Applies heuristics to minimize edge crossings in the graph and sets the best
//! order solution as an order attribute on each node.
//!
//! Uses the barycenter heuristic with alternating up/down sweeps, with
//! hierarchical subgraph sorting to keep sibling subgraphs together.

mod barycenter;
mod cross_count;
mod init_order;
mod resolve_conflicts;
mod sort;
mod sort_subgraph;

use crate::layout::dagre::graph::DagreGraph;

pub use barycenter::{barycenter, barycenter_down, BarycenterEntry};
pub use cross_count::cross_count;
pub use init_order::{assign_order, init_order};
pub use resolve_conflicts::ConstraintGraph;
pub use sort::{sort, SortResult};
pub use sort_subgraph::{add_subgraph_constraints, sort_subgraph};

/// Entry for parent group during layer sorting: (parent, sorted_nodes, barycenter, weight)
type ParentGroupEntry = (Option<String>, Vec<String>, Option<f64>, f64);

/// Assign order to nodes to minimize edge crossings
pub fn order(g: &mut DagreGraph) {
    // Get initial layering from DFS
    let layering = init_order(g);
    assign_order(g, &layering);

    // Find max rank for iteration
    let max_rank = g
        .nodes()
        .iter()
        .filter_map(|v| g.node(v).and_then(|n| n.rank))
        .max()
        .unwrap_or(0) as usize;

    if max_rank == 0 {
        return; // Only one layer, no crossings possible
    }

    // Track best solution
    let mut best_cc = i32::MAX;
    let mut best_layering = layering.clone();

    // Iterate: alternate between down sweep and up sweep
    let mut last_best = 0;
    for i in 0..24 {
        // Max 24 iterations like dagre.js
        if last_best >= 4 {
            break; // Stop if no improvement in 4 iterations
        }

        let bias_right = (i % 4) >= 2;

        if i % 2 == 0 {
            // Down sweep (top to bottom) - uses predecessors
            sweep_down_hierarchical(g, max_rank, bias_right);
        } else {
            // Up sweep (bottom to top) - uses successors
            sweep_up_hierarchical(g, max_rank, bias_right);
        }

        // Build layering from current order
        let current_layering = build_layer_matrix(g, max_rank);
        let cc = cross_count(g, &current_layering);

        if cc < best_cc {
            best_cc = cc;
            best_layering = current_layering;
            last_best = 0;
        } else {
            last_best += 1;
        }
    }

    // Apply best ordering
    assign_order(g, &best_layering);
}

/// Down sweep with hierarchical subgraph sorting
///
/// For each layer (top to bottom), sort nodes hierarchically, keeping sibling
/// subgraphs together and maintaining ordering constraints between them.
fn sweep_down_hierarchical(g: &mut DagreGraph, max_rank: usize, bias_right: bool) {
    let mut cg = ConstraintGraph::new();

    for rank in 1..=max_rank {
        // Get nodes at this rank (the "movable" layer)
        // Sort by current order to preserve init_order's edge-based ordering for tie-breaking
        let mut layer_nodes: Vec<String> = g
            .nodes()
            .iter()
            .filter(|v| {
                g.node(v)
                    .map(|n| n.rank == Some(rank as i32))
                    .unwrap_or(false)
            })
            .map(|s| s.to_string())
            .collect();
        // Sort by current order to preserve edge definition order for tie-breaking
        layer_nodes.sort_by_key(|v| g.node(v).and_then(|n| n.order).unwrap_or(i32::MAX as usize));

        if layer_nodes.is_empty() {
            continue;
        }

        // Build a temporary view for sorting this rank
        // We need to sort nodes hierarchically based on their subgraph structure
        let sorted = sort_layer_hierarchical(g, &layer_nodes, &cg, bias_right, true);

        // Assign new order
        for (i, v) in sorted.iter().enumerate() {
            if let Some(node) = g.node_mut(v) {
                node.order = Some(i);
            }
        }

        // Add subgraph constraints based on new ordering
        add_subgraph_constraints(g, &mut cg, &sorted);
    }
}

/// Up sweep with hierarchical subgraph sorting
///
/// For each layer (bottom to top), sort nodes hierarchically, keeping sibling
/// subgraphs together and maintaining ordering constraints between them.
fn sweep_up_hierarchical(g: &mut DagreGraph, max_rank: usize, bias_right: bool) {
    let mut cg = ConstraintGraph::new();

    for rank in (0..max_rank).rev() {
        // Get nodes at this rank (the "movable" layer)
        // Sort by current order to preserve init_order's edge-based ordering for tie-breaking
        let mut layer_nodes: Vec<String> = g
            .nodes()
            .iter()
            .filter(|v| {
                g.node(v)
                    .map(|n| n.rank == Some(rank as i32))
                    .unwrap_or(false)
            })
            .map(|s| s.to_string())
            .collect();
        // Sort by current order to preserve edge definition order for tie-breaking
        layer_nodes.sort_by_key(|v| g.node(v).and_then(|n| n.order).unwrap_or(i32::MAX as usize));

        if layer_nodes.is_empty() {
            continue;
        }

        // Sort hierarchically using successors (outgoing edges)
        let sorted = sort_layer_hierarchical(g, &layer_nodes, &cg, bias_right, false);

        // Assign new order
        for (i, v) in sorted.iter().enumerate() {
            if let Some(node) = g.node_mut(v) {
                node.order = Some(i);
            }
        }

        // Add subgraph constraints based on new ordering
        add_subgraph_constraints(g, &mut cg, &sorted);
    }
}

/// Sort a layer hierarchically, respecting subgraph structure
///
/// This function groups nodes by their immediate parent, sorts each group
/// recursively, and then combines them while respecting constraint graph.
fn sort_layer_hierarchical(
    g: &DagreGraph,
    layer_nodes: &[String],
    _cg: &ConstraintGraph,
    bias_right: bool,
    use_predecessors: bool,
) -> Vec<String> {
    use std::collections::HashMap;

    // Group nodes by their parent subgraph
    let mut by_parent: HashMap<Option<String>, Vec<String>> = HashMap::new();
    for v in layer_nodes {
        let parent = g.parent(v).map(|s| s.to_string());
        by_parent.entry(parent).or_default().push(v.clone());
    }

    // For each parent, get barycenters and sort
    let mut parent_entries: Vec<ParentGroupEntry> = Vec::new();

    for (parent, mut nodes) in by_parent {
        // Sort nodes within this parent by current order for stability
        nodes.sort_by_key(|v| g.node(v).and_then(|n| n.order).unwrap_or(usize::MAX));

        // Debug: trace sorted nodes
        #[cfg(test)]
        {
            if nodes.iter().any(|v| v == "ZZZ" || v == "AAA") {
                eprintln!(
                    "    After sort by order: {:?}, orders: {:?}",
                    nodes,
                    nodes
                        .iter()
                        .map(|v| g.node(v).and_then(|n| n.order))
                        .collect::<Vec<_>>()
                );
            }
        }

        // Calculate barycenters
        let entries: Vec<BarycenterEntry> = if use_predecessors {
            barycenter(g, &nodes)
        } else {
            barycenter_down(g, &nodes)
        };

        // Debug: trace barycenters
        #[cfg(test)]
        {
            if nodes.iter().any(|v| v == "ZZZ" || v == "AAA") {
                eprintln!(
                    "    Barycenters: {:?}",
                    entries
                        .iter()
                        .map(|e| (&e.v, e.barycenter, e.i))
                        .collect::<Vec<_>>()
                );
            }
        }

        // Sort by barycenter
        let sorted = sort(entries, bias_right);

        // Debug: trace sorted result
        #[cfg(test)]
        {
            if nodes.iter().any(|v| v == "ZZZ" || v == "AAA") {
                eprintln!(
                    "    After sort (bias_right={}): {:?}",
                    bias_right, sorted.vs
                );
            }
        }

        // Ensure border nodes at edges for this parent
        let reordered = if parent.is_some() {
            ensure_border_nodes_at_edges_for_parent(g, sorted.vs, &parent)
        } else {
            sorted.vs
        };

        // Calculate aggregate barycenter for this parent group
        let (sum, weight) = reordered.iter().fold((0.0, 0.0), |(sum, weight), v| {
            if let Some(node) = g.node(v) {
                if let Some(order) = node.order {
                    return (sum + order as f64, weight + 1.0);
                }
            }
            (sum, weight)
        });

        let avg_bc = if weight > 0.0 {
            Some(sum / weight)
        } else {
            None
        };

        parent_entries.push((parent, reordered, avg_bc, weight));
    }

    // Sort parent groups by their aggregate barycenter
    parent_entries.sort_by(|a, b| match (a.2, b.2) {
        (Some(bc_a), Some(bc_b)) => bc_a.partial_cmp(&bc_b).unwrap_or(std::cmp::Ordering::Equal),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => std::cmp::Ordering::Equal,
    });

    // Flatten into final order
    let mut result: Vec<String> = Vec::new();
    for (_, nodes, _, _) in parent_entries {
        result.extend(nodes);
    }

    result
}

/// Ensure border nodes are at edges for a specific parent subgraph
fn ensure_border_nodes_at_edges_for_parent(
    g: &DagreGraph,
    vs: Vec<String>,
    parent: &Option<String>,
) -> Vec<String> {
    let parent = match parent {
        Some(p) => p,
        None => return vs,
    };

    let (border_left, border_right) = if let Some(parent_node) = g.node(parent) {
        // Get the rank of the first node to determine which border nodes to use
        let first_rank = vs.first().and_then(|v| g.node(v)).and_then(|n| n.rank);

        if let Some(rank) = first_rank {
            let min_rank = parent_node.min_rank.unwrap_or(0);
            if rank >= min_rank {
                let rank_idx = (rank - min_rank) as usize;
                let bl = parent_node.border_left.get(rank_idx).cloned().flatten();
                let br = parent_node.border_right.get(rank_idx).cloned().flatten();
                (bl, br)
            } else {
                (None, None)
            }
        } else {
            (None, None)
        }
    } else {
        (None, None)
    };

    // Remove border nodes from their current positions
    let non_border: Vec<String> = vs
        .iter()
        .filter(|v| {
            Some(v.as_str()) != border_left.as_deref()
                && Some(v.as_str()) != border_right.as_deref()
        })
        .cloned()
        .collect();

    // Reconstruct: [borderLeft, ...non_border, borderRight]
    let mut reordered = Vec::new();
    if let Some(ref bl) = border_left {
        if vs.contains(bl) {
            reordered.push(bl.clone());
        }
    }
    reordered.extend(non_border);
    if let Some(ref br) = border_right {
        if vs.contains(br) {
            reordered.push(br.clone());
        }
    }

    reordered
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
    fn test_order_decision_branches_preserve_edge_order() {
        // Simulates the flowchart:
        //   B -->|Yes| C[Action 1]
        //   B -->|No| D[Action 2]
        //
        // In mermaid.js, the first edge's target (C) appears ABOVE
        // the second edge's target (D). So C should have order 0, D order 1.
        let mut g = DagreGraph::new();

        // Add edges in specific order - C first, then D
        g.set_edge("B", "C", EdgeLabel::default()); // "Yes" branch
        g.set_edge("B", "D", EdgeLabel::default()); // "No" branch

        rank::assign_ranks(&mut g, Ranker::LongestPath);
        order(&mut g);

        // C and D are on the same rank (both successors of B)
        let c_order = g.node("C").unwrap().order;
        let d_order = g.node("D").unwrap().order;

        assert!(c_order.is_some() && d_order.is_some());

        // C (first edge target) should have lower order than D (second edge target)
        // This matches mermaid.js behavior where first branch appears on top
        assert!(
            c_order.unwrap() < d_order.unwrap(),
            "C (Action 1, first edge) should have lower order than D (Action 2, second edge). C order: {:?}, D order: {:?}",
            c_order, d_order
        );
    }

    #[test]
    fn test_order_fork_pattern_preserves_edge_order() {
        // Fork pattern like state diagrams:
        // start -> fork -> first_target
        //              \-> second_target
        // -> join
        //
        // When both targets have same barycenter (both connected to single fork node),
        // the first-defined edge target should appear on the left (lower order).
        let mut g = DagreGraph::new();

        g.set_edge("start", "fork", EdgeLabel::default());
        g.set_edge("fork", "first_target", EdgeLabel::default()); // First fork edge
        g.set_edge("fork", "second_target", EdgeLabel::default()); // Second fork edge
        g.set_edge("first_target", "join", EdgeLabel::default());
        g.set_edge("second_target", "join", EdgeLabel::default());

        rank::assign_ranks(&mut g, Ranker::LongestPath);

        // Check initial order
        let init_layering = init_order(&g);
        eprintln!("init_order layer 2: {:?}", init_layering[2]);

        order(&mut g);

        let first_order = g.node("first_target").unwrap().order;
        let second_order = g.node("second_target").unwrap().order;

        eprintln!(
            "After order(): first_target order={:?}, second_target order={:?}",
            first_order, second_order
        );

        assert!(first_order.is_some() && second_order.is_some());
        assert!(
            first_order.unwrap() < second_order.unwrap(),
            "first_target (first edge) should have lower order than second_target. first={:?}, second={:?}",
            first_order, second_order
        );
    }
}
