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

    // Initialize source nodes with rank 0
    for v in &nodes {
        if g.in_edges(v).is_empty() {
            if let Some(label) = g.node_mut(v) {
                label.rank = Some(0);
            }
            visited.insert(v.clone());
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
