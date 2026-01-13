//! Longest path ranking algorithm
//!
//! A simple O(V+E) algorithm that assigns ranks based on the longest path
//! from any source node. This is used as an initial ranking before network
//! simplex optimization, and can be used standalone for simple cases.

use crate::layout::dagre::graph::DagreGraph;
use std::collections::HashSet;

/// Assign ranks using the longest path algorithm
pub fn run(g: &mut DagreGraph) {
    let mut visited = HashSet::new();

    // Process nodes in topological order (sources first)
    fn dfs(g: &mut DagreGraph, v: &str, visited: &mut HashSet<String>) {
        if visited.contains(v) {
            return;
        }

        // Visit all predecessors first
        let preds: Vec<String> = g.predecessors(v).into_iter().cloned().collect();
        for pred in &preds {
            dfs(g, pred, visited);
        }

        visited.insert(v.to_string());

        // Calculate rank as max predecessor rank + minlen
        let mut rank = 0;
        for edge_key in g.in_edges(v) {
            if let Some(pred_label) = g.node(&edge_key.v) {
                if let Some(pred_rank) = pred_label.rank {
                    let minlen = g.edge_by_key(edge_key).map(|e| e.minlen).unwrap_or(1);
                    rank = rank.max(pred_rank + minlen);
                }
            }
        }

        if let Some(label) = g.node_mut(v) {
            label.rank = Some(rank);
        }
    }

    // Get all node keys first
    let nodes: Vec<String> = g.nodes().into_iter().cloned().collect();

    // Initialize source nodes with rank 0
    for v in &nodes {
        if g.in_edges(v).is_empty() {
            if let Some(label) = g.node_mut(v) {
                label.rank = Some(0);
            }
            visited.insert(v.clone());
        }
    }

    // Process remaining nodes
    for v in &nodes {
        dfs(g, v, &mut visited);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::dagre::graph::{EdgeLabel, NodeLabel};

    #[test]
    fn test_single_node() {
        let mut g = DagreGraph::new();
        g.set_node("a", NodeLabel::default());

        run(&mut g);

        assert_eq!(g.node("a").unwrap().rank, Some(0));
    }

    #[test]
    fn test_chain() {
        let mut g = DagreGraph::new();
        g.set_path(&["a", "b", "c"]);

        run(&mut g);

        assert_eq!(g.node("a").unwrap().rank, Some(0));
        assert_eq!(g.node("b").unwrap().rank, Some(1));
        assert_eq!(g.node("c").unwrap().rank, Some(2));
    }

    #[test]
    fn test_diamond() {
        let mut g = DagreGraph::new();
        g.set_path(&["a", "b", "d"]);
        g.set_path(&["a", "c", "d"]);

        run(&mut g);

        assert_eq!(g.node("a").unwrap().rank, Some(0));
        assert_eq!(g.node("b").unwrap().rank, Some(1));
        assert_eq!(g.node("c").unwrap().rank, Some(1));
        assert_eq!(g.node("d").unwrap().rank, Some(2));
    }

    #[test]
    fn test_respects_minlen() {
        let mut g = DagreGraph::new();
        g.set_edge(
            "a",
            "b",
            EdgeLabel {
                minlen: 3,
                ..Default::default()
            },
        );

        run(&mut g);

        assert_eq!(g.node("a").unwrap().rank, Some(0));
        assert_eq!(g.node("b").unwrap().rank, Some(3));
    }
}
