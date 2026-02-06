//! Longest path ranking algorithm
//!
//! A simple O(V+E) algorithm that assigns ranks based on the longest path
//! from any source node. This is used as an initial ranking before network
//! simplex optimization, and can be used standalone for simple cases.

use crate::layout::dagre::graph::DagreGraph;
use std::collections::HashSet;

/// State for iterative DFS
enum DfsState {
    /// First visit - need to process predecessors
    Explore(String),
    /// Returning from predecessors - compute rank
    Compute(String),
}

/// Assign ranks using the longest path algorithm
/// Uses iterative DFS to avoid stack overflow on deep graphs
pub fn run(g: &mut DagreGraph) {
    let mut visited = HashSet::new();
    let mut in_progress = HashSet::new(); // Track nodes currently on the stack
    let mut stack: Vec<DfsState> = Vec::new();

    // Get all node keys first
    let nodes: Vec<String> = g.nodes().into_iter().cloned().collect();

    // Collect source nodes (no predecessors)
    let mut sources = Vec::new();
    for v in &nodes {
        if g.in_edges(v).is_empty() {
            if let Some(label) = g.node_mut(v) {
                label.rank = Some(0);
            }
            visited.insert(v.clone());
            sources.push(v.clone());
        }
    }

    // Process remaining nodes using iterative DFS
    for start in &nodes {
        if visited.contains(start) {
            continue;
        }

        stack.push(DfsState::Explore(start.clone()));
        in_progress.insert(start.clone());

        while let Some(state) = stack.pop() {
            match state {
                DfsState::Explore(v) => {
                    if visited.contains(&v) {
                        in_progress.remove(&v);
                        continue;
                    }

                    // Push compute state to be processed after predecessors
                    stack.push(DfsState::Compute(v.clone()));

                    // Push predecessors to explore first (skip visited and in-progress to handle cycles)
                    let preds: Vec<String> = g.predecessors(&v).into_iter().cloned().collect();
                    for pred in preds {
                        if !visited.contains(&pred) && !in_progress.contains(&pred) {
                            stack.push(DfsState::Explore(pred.clone()));
                            in_progress.insert(pred);
                        }
                    }
                }
                DfsState::Compute(v) => {
                    in_progress.remove(&v);

                    if visited.contains(&v) {
                        continue;
                    }

                    visited.insert(v.clone());

                    // Calculate rank as max predecessor rank + minlen
                    let mut rank = 0;
                    for edge_key in g.in_edges(&v) {
                        if let Some(pred_label) = g.node(&edge_key.v) {
                            if let Some(pred_rank) = pred_label.rank {
                                let minlen = g.edge_by_key(edge_key).map(|e| e.minlen).unwrap_or(1);
                                rank = rank.max(pred_rank + minlen);
                            }
                        }
                    }

                    if let Some(label) = g.node_mut(&v) {
                        label.rank = Some(rank);
                    }
                }
            }
        }
    }

    // Tighten sources: push source nodes with out-edges closer to their
    // successors. Without this, disconnected sources (e.g., test_entity3 ->
    // test_req5) sit at rank 0, adding width to that rank. By moving them
    // to min(successor_rank) - minlen, we reduce the number of nodes at
    // rank 0 and produce narrower layouts.
    for v in &sources {
        let out_edges = g.out_edges(v);
        if out_edges.is_empty() {
            continue; // Isolated nodes stay at rank 0
        }
        let mut min_successor_rank = i32::MAX;
        for edge_key in &out_edges {
            if let Some(succ_label) = g.node(&edge_key.w) {
                if let Some(succ_rank) = succ_label.rank {
                    let minlen = g.edge_by_key(edge_key).map(|e| e.minlen).unwrap_or(1);
                    min_successor_rank = min_successor_rank.min(succ_rank - minlen);
                }
            }
        }
        if min_successor_rank > 0 {
            if let Some(label) = g.node_mut(v) {
                label.rank = Some(min_successor_rank);
            }
        }
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

    #[test]
    fn test_tighten_sources_pushes_disconnected_source_down() {
        // Models the requirement diagram scenario:
        // test_entity -> test_req2 (rank 1)
        // test_req -> test_req2 (rank 1), test_req -> test_req3 -> test_req4 -> test_req5
        // test_entity3 -> test_req5 (rank 4)
        //
        // Without tightening: test_entity, test_req, test_entity3 all at rank 0 (3 sources)
        // With tightening: test_entity3 should move to rank 3 (one before test_req5)
        let mut g = DagreGraph::new();

        // Main chain: a -> b -> c -> d -> e (ranks 0-4)
        g.set_path(&["a", "b", "c", "d", "e"]);
        // Another source connecting to b: f -> b
        g.set_edge("f", "b", EdgeLabel::default());
        // Disconnected source connecting deep: x -> e
        g.set_edge("x", "e", EdgeLabel::default());

        run(&mut g);

        // a and f must stay at rank 0 (they connect to b at rank 1)
        assert_eq!(g.node("a").unwrap().rank, Some(0), "a stays at rank 0");
        assert_eq!(g.node("f").unwrap().rank, Some(0), "f stays at rank 0");
        // x should be tightened: its only successor e is at rank 4,
        // so x should be at rank 3 (= 4 - minlen(1))
        assert_eq!(
            g.node("x").unwrap().rank,
            Some(3),
            "x should be tightened to rank 3 (one before e at rank 4)"
        );
    }
}
