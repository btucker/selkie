//! Initial order assignment using DFS
//!
//! Assigns an initial order value for each node by performing a DFS search
//! starting from nodes in the first rank. Nodes are assigned an order in their
//! rank as they are first visited.
//!
//! This approach comes from Gansner, et al., "A Technique for Drawing Directed Graphs."

use crate::layout::dagre::graph::DagreGraph;
use std::collections::HashSet;

/// Build initial layering by DFS traversal
///
/// Returns a layering matrix with an array per layer and each layer sorted by
/// the order of its nodes.
pub fn init_order(g: &DagreGraph) -> Vec<Vec<String>> {
    let mut visited = HashSet::new();

    // Find max rank
    let max_rank = g
        .nodes()
        .iter()
        .filter_map(|v| g.node(v).and_then(|n| n.rank))
        .max()
        .unwrap_or(0);

    // Create empty layers
    let mut layers: Vec<Vec<String>> = (0..=max_rank).map(|_| Vec::new()).collect();

    // DFS function to visit nodes
    fn dfs(g: &DagreGraph, v: &str, visited: &mut HashSet<String>, layers: &mut Vec<Vec<String>>) {
        if visited.contains(v) {
            return;
        }
        visited.insert(v.to_string());

        if let Some(node) = g.node(v) {
            if let Some(rank) = node.rank {
                if rank >= 0 && (rank as usize) < layers.len() {
                    layers[rank as usize].push(v.to_string());
                }
            }
        }

        // Visit successors in edge definition order
        for succ in g.successors(v) {
            dfs(g, succ, visited, layers);
        }
    }

    // Get all nodes sorted by rank
    let mut nodes: Vec<&String> = g.nodes();
    nodes.sort_by_key(|v| g.node(v).and_then(|n| n.rank).unwrap_or(i32::MAX));

    // Perform DFS from each node
    for v in nodes {
        dfs(g, v, &mut visited, &mut layers);
    }

    layers
}

/// Assign order values based on layering
pub fn assign_order(g: &mut DagreGraph, layering: &[Vec<String>]) {
    for layer in layering {
        for (i, v) in layer.iter().enumerate() {
            if let Some(node) = g.node_mut(v) {
                node.order = Some(i);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::dagre::graph::NodeLabel;
    use crate::layout::dagre::rank;
    use crate::layout::dagre::Ranker;

    #[test]
    fn test_init_order_single_node() {
        let mut g = DagreGraph::new();
        g.set_node(
            "a",
            NodeLabel {
                rank: Some(0),
                ..Default::default()
            },
        );

        let layers = init_order(&g);

        assert_eq!(layers.len(), 1);
        assert_eq!(layers[0], vec!["a"]);
    }

    #[test]
    fn test_init_order_chain() {
        let mut g = DagreGraph::new();
        g.set_path(&["a", "b", "c"]);
        rank::assign_ranks(&mut g, Ranker::LongestPath);

        let layers = init_order(&g);

        assert_eq!(layers.len(), 3);
        assert_eq!(layers[0], vec!["a"]);
        assert_eq!(layers[1], vec!["b"]);
        assert_eq!(layers[2], vec!["c"]);
    }

    #[test]
    fn test_init_order_diamond() {
        let mut g = DagreGraph::new();
        g.set_path(&["a", "b", "d"]);
        g.set_path(&["a", "c", "d"]);
        rank::assign_ranks(&mut g, Ranker::LongestPath);

        let layers = init_order(&g);

        assert_eq!(layers.len(), 3);
        assert_eq!(layers[0], vec!["a"]);
        assert!(layers[1].contains(&"b".to_string()));
        assert!(layers[1].contains(&"c".to_string()));
        assert_eq!(layers[2], vec!["d"]);
    }

    #[test]
    fn test_assign_order() {
        let mut g = DagreGraph::new();
        g.set_path(&["a", "b", "d"]);
        g.set_path(&["a", "c", "d"]);
        rank::assign_ranks(&mut g, Ranker::LongestPath);

        let layers = init_order(&g);
        assign_order(&mut g, &layers);

        assert_eq!(g.node("a").unwrap().order, Some(0));
        assert!(g.node("b").unwrap().order.is_some());
        assert!(g.node("c").unwrap().order.is_some());
        assert_eq!(g.node("d").unwrap().order, Some(0));
    }

    #[test]
    fn test_init_order_fork_preserves_edge_order() {
        // Tests that when a node has multiple outgoing edges (like a fork),
        // the successors are visited in edge definition order.
        // First edge target should get lower order (appear on LEFT in TB layout).
        use crate::layout::dagre::graph::EdgeLabel;

        let mut g = DagreGraph::new();
        // Create fork pattern: start -> fork, fork -> first_target, fork -> second_target
        g.set_edge("start", "fork", EdgeLabel::default());
        g.set_edge("fork", "first_target", EdgeLabel::default()); // First edge
        g.set_edge("fork", "second_target", EdgeLabel::default()); // Second edge
        rank::assign_ranks(&mut g, Ranker::LongestPath);

        let layers = init_order(&g);

        // All nodes should be in layers
        assert_eq!(layers[0], vec!["start"]);
        assert_eq!(layers[1], vec!["fork"]);

        // Layer 2 should have first_target BEFORE second_target
        // because edges were defined in that order
        assert_eq!(layers[2].len(), 2);
        let first_idx = layers[2].iter().position(|x| x == "first_target");
        let second_idx = layers[2].iter().position(|x| x == "second_target");

        assert!(
            first_idx.unwrap() < second_idx.unwrap(),
            "first_target should come before second_target in layer. Layer: {:?}",
            layers[2]
        );
    }
}
