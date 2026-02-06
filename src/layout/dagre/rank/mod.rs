//! Rank assignment algorithms for dagre layout
//!
//! Assigns each node to a layer (rank) in the graph. The goal is to minimize
//! the total edge length while respecting minimum length constraints.

mod longest_path;
mod network_simplex;
mod util;

use super::graph::DagreGraph;
use super::Ranker;

pub use network_simplex::{
    calc_cut_value, enter_edge, exchange_edges, init_cut_values, init_low_lim_values, leave_edge,
};

/// Assign ranks to all nodes in the graph
pub fn assign_ranks(g: &mut DagreGraph, method: Ranker) {
    match method {
        Ranker::LongestPath => {
            longest_path::run(g);
            // Pull source-only nodes closer to their targets to minimize edge length.
            // Longest-path assigns all source nodes to rank 0, but nodes like
            // test_entity3 (which only connects to test_req5 at rank 3) should be
            // pulled down to rank 2 to match dagre/mermaid's network-simplex behavior.
            pull_sources_toward_targets(g);
        }
        Ranker::TightTree => {
            // Tight tree uses longest path as initial assignment, then tightens
            longest_path::run(g);
            // TODO: implement tight tree refinement
        }
        Ranker::NetworkSimplex => network_simplex::run(g),
    }

    // Normalize ranks to start at 0
    util::normalize_ranks(g);
}

/// After longest-path ranking, pull source-only nodes down toward their targets.
///
/// Longest-path assigns all source nodes (no predecessors) to rank 0.
/// But for nodes that connect to targets at deeper ranks, this creates
/// unnecessarily long edges. This post-processing step moves such nodes
/// as close to their targets as possible, matching network-simplex behavior.
fn pull_sources_toward_targets(g: &mut DagreGraph) {
    let nodes: Vec<String> = g.nodes().iter().map(|s| (*s).clone()).collect();

    for v in &nodes {
        // Only process source nodes (no incoming edges)
        if !g.in_edges(v).is_empty() {
            continue;
        }

        let out_edges = g.out_edges(v);
        if out_edges.is_empty() {
            continue; // Disconnected node, leave at rank 0
        }

        // Find the minimum rank we can assign and the minimum minlen.
        // The minlen matters because make_space_for_edge_labels doubles it,
        // which inflates apparent rank spans.
        let mut max_rank = i32::MAX;
        let mut min_minlen = i32::MAX;
        for edge_key in &out_edges {
            if let Some(target_label) = g.node(&edge_key.w) {
                if let Some(target_rank) = target_label.rank {
                    let minlen = g.edge_by_key(edge_key).map(|e| e.minlen).unwrap_or(1);
                    max_rank = max_rank.min(target_rank - minlen);
                    min_minlen = min_minlen.min(minlen);
                }
            }
        }

        // Only pull when the rank gap exceeds one "real" layer (minlen).
        // After make_space_for_edge_labels doubles minlen, a 2-layer span
        // becomes a 4-rank gap. Using minlen as threshold ensures we only
        // pull when there are genuinely multiple layers of slack.
        if max_rank != i32::MAX && min_minlen != i32::MAX {
            if let Some(label) = g.node_mut(v) {
                if let Some(current_rank) = label.rank {
                    if max_rank - current_rank > min_minlen {
                        label.rank = Some(max_rank);
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::dagre::graph::{DagreGraph, EdgeLabel, NodeLabel};

    #[test]
    fn test_assign_single_node() {
        let mut g = DagreGraph::new();
        g.set_node("a", NodeLabel::default());

        assign_ranks(&mut g, Ranker::NetworkSimplex);

        assert_eq!(g.node("a").unwrap().rank, Some(0));
    }

    #[test]
    fn test_assign_two_connected_nodes() {
        let mut g = DagreGraph::new();
        g.set_edge("a", "b", EdgeLabel::default());

        assign_ranks(&mut g, Ranker::NetworkSimplex);

        assert_eq!(g.node("a").unwrap().rank, Some(0));
        assert_eq!(g.node("b").unwrap().rank, Some(1));
    }

    #[test]
    fn test_assign_diamond() {
        let mut g = DagreGraph::new();
        g.set_path(&["a", "b", "d"]);
        g.set_path(&["a", "c", "d"]);

        assign_ranks(&mut g, Ranker::NetworkSimplex);

        assert_eq!(g.node("a").unwrap().rank, Some(0));
        assert_eq!(g.node("b").unwrap().rank, Some(1));
        assert_eq!(g.node("c").unwrap().rank, Some(1));
        assert_eq!(g.node("d").unwrap().rank, Some(2));
    }

    #[test]
    fn test_respects_minlen() {
        let mut g = DagreGraph::new();
        g.set_path(&["a", "b", "d"]);
        g.set_edge("a", "c", EdgeLabel::default());
        g.set_edge(
            "c",
            "d",
            EdgeLabel {
                minlen: 2,
                ..Default::default()
            },
        );

        assign_ranks(&mut g, Ranker::NetworkSimplex);

        let a_rank = g.node("a").unwrap().rank.unwrap();
        let c_rank = g.node("c").unwrap().rank.unwrap();
        let d_rank = g.node("d").unwrap().rank.unwrap();

        // c -> d should be at least 2 ranks apart
        assert!(d_rank - c_rank >= 2);
        assert!(c_rank >= a_rank);
    }

    #[test]
    fn test_gansner_graph() {
        // The classic example from the paper (Gansner et al. 1993)
        let mut g = DagreGraph::new();
        g.set_path(&["a", "b", "c", "d", "h"]);
        g.set_path(&["a", "e", "g", "h"]);
        g.set_path(&["a", "f", "g"]);

        assign_ranks(&mut g, Ranker::NetworkSimplex);

        assert_eq!(g.node("a").unwrap().rank, Some(0));
        assert_eq!(g.node("b").unwrap().rank, Some(1));
        assert_eq!(g.node("c").unwrap().rank, Some(2));
        assert_eq!(g.node("d").unwrap().rank, Some(3));
        assert_eq!(g.node("h").unwrap().rank, Some(4));
        assert_eq!(g.node("e").unwrap().rank, Some(1));
        assert_eq!(g.node("f").unwrap().rank, Some(1));
        assert_eq!(g.node("g").unwrap().rank, Some(2));
    }

    #[test]
    fn test_flowchart_diamond_structure() {
        // This replicates the failing case from flowchart rendering:
        // A -> B -> C -> D, C -> E, D -> F, E -> F
        // Expected: A(0) -> B(1) -> C(2) -> D,E(3) -> F(4)
        let mut g = DagreGraph::new();
        g.set_path(&["A", "B", "C", "D", "F"]);
        g.set_edge("C", "E", EdgeLabel::default());
        g.set_edge("E", "F", EdgeLabel::default());

        assign_ranks(&mut g, Ranker::NetworkSimplex);

        assert_eq!(g.node("A").unwrap().rank, Some(0), "A should be rank 0");
        assert_eq!(g.node("B").unwrap().rank, Some(1), "B should be rank 1");
        assert_eq!(g.node("C").unwrap().rank, Some(2), "C should be rank 2");
        assert_eq!(g.node("D").unwrap().rank, Some(3), "D should be rank 3");
        assert_eq!(g.node("E").unwrap().rank, Some(3), "E should be rank 3");
        assert_eq!(g.node("F").unwrap().rank, Some(4), "F should be rank 4");
    }

    #[test]
    fn test_pull_sources_long_edge() {
        // Mimics requirement_complex: test_entity3 -> test_req5 with a long edge.
        // The source should be pulled down when gap > minlen.
        let mut g = DagreGraph::new();
        g.set_path(&["a", "b", "c", "d", "e"]); // a(0) -> b(1) -> c(2) -> d(3) -> e(4)
        g.set_edge("src", "d", EdgeLabel::default()); // src -> d, 3-layer gap

        assign_ranks(&mut g, Ranker::LongestPath);

        // src should be pulled from rank 0 to rank 2 (d is at rank 3, minlen=1, gap=2 > 1)
        assert_eq!(g.node("a").unwrap().rank, Some(0));
        assert_eq!(
            g.node("src").unwrap().rank,
            Some(2),
            "source should be pulled toward target"
        );
        assert_eq!(g.node("d").unwrap().rank, Some(3));
    }

    #[test]
    fn test_pull_sources_short_edge_no_pull() {
        // Source with only 1-layer gap should NOT be pulled
        let mut g = DagreGraph::new();
        g.set_path(&["a", "b"]); // a(0) -> b(1)
        g.set_edge("src", "b", EdgeLabel::default()); // src -> b, gap of 1

        assign_ranks(&mut g, Ranker::LongestPath);

        // src should stay at rank 0 (gap of 1 is not > minlen of 1)
        assert_eq!(
            g.node("src").unwrap().rank,
            Some(0),
            "source should not be pulled for small gap"
        );
        assert_eq!(g.node("b").unwrap().rank, Some(1));
    }

    #[test]
    fn test_pull_sources_multi_outgoing() {
        // Source with multiple outgoing edges should be pulled to the
        // tightest constraint (minimum of target_rank - minlen).
        let mut g = DagreGraph::new();
        g.set_path(&["a", "b", "c", "d", "e"]); // a(0)->b(1)->c(2)->d(3)->e(4)
        g.set_edge("src", "d", EdgeLabel::default()); // src -> d (rank 3)
        g.set_edge("src", "e", EdgeLabel::default()); // src -> e (rank 4)

        assign_ranks(&mut g, Ranker::LongestPath);

        // src should be pulled to rank 2 = min(3-1, 4-1) = min(2, 3) = 2
        // The tighter constraint (d at rank 3) limits how far src can move.
        assert_eq!(
            g.node("src").unwrap().rank,
            Some(2),
            "source with multiple targets should respect the tightest constraint"
        );
    }

    #[test]
    fn test_pull_sources_disconnected_node() {
        // Disconnected nodes (no edges) should stay at rank 0
        let mut g = DagreGraph::new();
        g.set_path(&["a", "b"]);
        g.set_node("orphan", NodeLabel::default());

        assign_ranks(&mut g, Ranker::LongestPath);

        assert_eq!(
            g.node("orphan").unwrap().rank,
            Some(0),
            "disconnected node stays at rank 0"
        );
    }
}
