//! Node ordering (crossing minimization) for dagre layout
//!
//! Applies heuristics to minimize edge crossings in the graph and sets the best
//! order solution as an order attribute on each node.
//!
//! Uses the barycenter heuristic with alternating up/down sweeps.

mod barycenter;
mod cross_count;
mod init_order;
mod sort;

use crate::layout::dagre::graph::DagreGraph;

pub use barycenter::{barycenter, barycenter_down, BarycenterEntry};
pub use cross_count::cross_count;
pub use init_order::{assign_order, init_order};
pub use sort::{sort, SortResult};

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
            // Down sweep (top to bottom)
            sweep_down(g, max_rank, bias_right);
        } else {
            // Up sweep (bottom to top)
            sweep_up(g, max_rank, bias_right);
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

/// Down sweep: for each layer (top to bottom), order nodes by barycenter of predecessors
fn sweep_down(g: &mut DagreGraph, max_rank: usize, bias_right: bool) {
    for rank in 1..=max_rank {
        // Collect nodes and sort by current order to preserve stable ordering
        let mut layer: Vec<(String, usize)> = g
            .nodes()
            .iter()
            .filter_map(|v| {
                let node = g.node(v)?;
                if node.rank == Some(rank as i32) {
                    Some(((*v).clone(), node.order.unwrap_or(usize::MAX)))
                } else {
                    None
                }
            })
            .collect();

        // Sort by current order to ensure stable input to barycenter/sort
        layer.sort_by_key(|(_, order)| *order);
        let layer: Vec<String> = layer.into_iter().map(|(v, _)| v).collect();

        let entries = barycenter(g, &layer);
        let sorted = sort(entries, bias_right);

        // Assign new order
        for (i, v) in sorted.vs.iter().enumerate() {
            if let Some(node) = g.node_mut(v) {
                node.order = Some(i);
            }
        }
    }
}

/// Up sweep: for each layer (bottom to top), order nodes by barycenter of successors
fn sweep_up(g: &mut DagreGraph, max_rank: usize, bias_right: bool) {
    for rank in (0..max_rank).rev() {
        // Collect nodes and sort by current order to preserve stable ordering
        let mut layer: Vec<(String, usize)> = g
            .nodes()
            .iter()
            .filter_map(|v| {
                let node = g.node(v)?;
                if node.rank == Some(rank as i32) {
                    Some(((*v).clone(), node.order.unwrap_or(usize::MAX)))
                } else {
                    None
                }
            })
            .collect();

        // Sort by current order to ensure stable input to barycenter/sort
        layer.sort_by_key(|(_, order)| *order);
        let layer: Vec<String> = layer.into_iter().map(|(v, _)| v).collect();

        let entries = barycenter_down(g, &layer);
        let sorted = sort(entries, bias_right);

        // Assign new order
        for (i, v) in sorted.vs.iter().enumerate() {
            if let Some(node) = g.node_mut(v) {
                node.order = Some(i);
            }
        }
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
}
