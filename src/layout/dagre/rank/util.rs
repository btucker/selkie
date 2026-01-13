//! Utility functions for ranking algorithms

use crate::layout::dagre::graph::DagreGraph;

/// Normalize ranks so that the minimum rank is 0
pub fn normalize_ranks(g: &mut DagreGraph) {
    let min_rank = g
        .nodes()
        .iter()
        .filter_map(|v| g.node(v).and_then(|n| n.rank))
        .min()
        .unwrap_or(0);

    if min_rank != 0 {
        let nodes: Vec<String> = g.nodes().into_iter().cloned().collect();
        for v in nodes {
            if let Some(label) = g.node_mut(&v) {
                if let Some(rank) = label.rank {
                    label.rank = Some(rank - min_rank);
                }
            }
        }
    }
}

/// Calculate the slack of an edge: actual rank difference minus minlen
pub fn slack(g: &DagreGraph, v: &str, w: &str) -> Option<i32> {
    let v_rank = g.node(v)?.rank?;
    let w_rank = g.node(w)?.rank?;
    let minlen = g.edge(v, w).map(|e| e.minlen).unwrap_or(1);

    Some(w_rank - v_rank - minlen)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::dagre::graph::{EdgeLabel, NodeLabel};

    #[test]
    fn test_normalize_ranks() {
        let mut g = DagreGraph::new();
        g.set_node(
            "a",
            NodeLabel {
                rank: Some(2),
                ..Default::default()
            },
        );
        g.set_node(
            "b",
            NodeLabel {
                rank: Some(4),
                ..Default::default()
            },
        );

        normalize_ranks(&mut g);

        assert_eq!(g.node("a").unwrap().rank, Some(0));
        assert_eq!(g.node("b").unwrap().rank, Some(2));
    }

    #[test]
    fn test_slack() {
        let mut g = DagreGraph::new();
        g.set_node(
            "a",
            NodeLabel {
                rank: Some(0),
                ..Default::default()
            },
        );
        g.set_node(
            "b",
            NodeLabel {
                rank: Some(3),
                ..Default::default()
            },
        );
        g.set_edge(
            "a",
            "b",
            EdgeLabel {
                minlen: 1,
                ..Default::default()
            },
        );

        assert_eq!(slack(&g, "a", "b"), Some(2)); // 3 - 0 - 1 = 2
    }
}
