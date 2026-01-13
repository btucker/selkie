//! Edge label positioning phases for dagre layout
//!
//! This module implements the edge-label-specific phases from dagre.js that
//! create proper spacing and positioning for edge labels.
//!
//! The phases are:
//! 1. make_space_for_edge_labels - Create vertical space by halving ranksep
//! 2. inject_edge_label_proxies - Create dummy nodes to lock label ranks
//! 3. remove_edge_label_proxies - Store computed labelRank, remove proxies
//! 4. fixup_edge_label_coords - Final coordinate adjustment based on labelpos

use super::graph::{DagreGraph, EdgeKey, NodeLabel};
use super::RankDir;

/// Create space for edge labels by adjusting ranksep and edge minlen
///
/// This phase runs early in the pipeline (before acyclic) and:
/// - Halves the graph's ranksep to create space for labels between ranks
/// - Doubles each edge's minlen to span two logical ranks
/// - Adds labeloffset padding to edge dimensions for non-center labels
pub fn make_space_for_edge_labels(g: &mut DagreGraph, rankdir: RankDir) {
    // Halve ranksep to create space for label rows
    let graph = g.graph_mut();
    graph.ranksep /= 2.0;

    // Collect edge info we need before mutating
    let edge_updates: Vec<(EdgeKey, i32, String, f64)> = g
        .edges()
        .iter()
        .filter_map(|key| {
            let edge = g.edge_by_key(key)?;
            Some((
                (*key).clone(),
                edge.minlen,
                edge.labelpos.to_lowercase(),
                edge.labeloffset,
            ))
        })
        .collect();

    // Now apply updates
    for (key, minlen, labelpos, labeloffset) in edge_updates {
        if let Some(edge) = g.edge_by_key_mut(&key) {
            // Double the minimum length to span label space
            edge.minlen = minlen * 2;

            // Add labeloffset to edge dimensions for non-center labels
            if labelpos != "c" {
                // Adjust dimensions based on rankdir
                if rankdir == RankDir::TB || rankdir == RankDir::BT {
                    edge.width += labeloffset;
                } else {
                    edge.height += labeloffset;
                }
            }
        }
    }
}

/// Inject dummy nodes for edge labels to lock their rank positions
///
/// Creates "edge-proxy" dummy nodes at the midpoint rank for edges
/// that have label dimensions. This ensures labels get positioned
/// at a specific rank during the layout process.
pub fn inject_edge_label_proxies(g: &mut DagreGraph) {
    // Collect edge info for labeled edges before mutating
    let proxies_to_create: Vec<(String, String, Option<String>, i32)> = g
        .edges()
        .iter()
        .filter_map(|key| {
            let edge = g.edge_by_key(key)?;

            // Only create proxy for edges with actual label dimensions
            if edge.width <= 0.0 || edge.height <= 0.0 {
                return None;
            }

            let v_rank = g.node(&key.v).and_then(|n| n.rank).unwrap_or(0);
            let w_rank = g.node(&key.w).and_then(|n| n.rank).unwrap_or(0);

            // Calculate midpoint rank for label
            let label_rank = (w_rank - v_rank) / 2 + v_rank;

            Some((key.v.clone(), key.w.clone(), key.name.clone(), label_rank))
        })
        .collect();

    // Now create the proxy nodes
    for (v, w, name, label_rank) in proxies_to_create {
        let proxy_id = g.unique_id("_ep");
        g.set_node(
            &proxy_id,
            NodeLabel {
                dummy: Some("edge-proxy".to_string()),
                rank: Some(label_rank),
                edge_obj: Some((v, w, name)),
                ..Default::default()
            },
        );
    }
}

/// Remove edge label proxy nodes and store the computed rank on edges
///
/// This phase runs after ranking to clean up proxy nodes while
/// preserving the computed label rank for use in normalization.
pub fn remove_edge_label_proxies(g: &mut DagreGraph) {
    // Find all edge-proxy nodes
    let proxy_nodes: Vec<(String, i32, String, String, Option<String>)> = g
        .nodes()
        .iter()
        .filter_map(|v| {
            let node = g.node(v)?;
            if node.dummy.as_deref() == Some("edge-proxy") {
                let rank = node.rank?;
                let (ev, ew, ename) = node.edge_obj.clone()?;
                Some(((*v).clone(), rank, ev, ew, ename))
            } else {
                None
            }
        })
        .collect();

    // Store labelRank on edges and remove proxy nodes
    for (node_id, rank, ev, ew, ename) in proxy_nodes {
        // Store the computed rank on the edge (using edge_by_key for multigraph support)
        let key = EdgeKey {
            v: ev,
            w: ew,
            name: ename,
        };
        if let Some(edge) = g.edge_by_key_mut(&key) {
            edge.label_rank = Some(rank);
        }

        // Remove the proxy node
        g.remove_node(&node_id);
    }
}

/// Adjust final edge label coordinates based on labelpos
///
/// This phase runs at the end of layout to:
/// - Remove the padding added by make_space_for_edge_labels
/// - Shift label coordinates based on labelpos (l/c/r)
pub fn fixup_edge_label_coords(g: &mut DagreGraph, rankdir: RankDir) {
    // Collect edge info we need before mutating
    let edge_updates: Vec<(EdgeKey, Option<f64>, String, f64, f64)> = g
        .edges()
        .iter()
        .filter_map(|key| {
            let edge = g.edge_by_key(key)?;
            Some((
                (*key).clone(),
                edge.x,
                edge.labelpos.to_lowercase(),
                edge.labeloffset,
                edge.width,
            ))
        })
        .collect();

    for (key, x, labelpos, labeloffset, width) in edge_updates {
        if let Some(edge) = g.edge_by_key_mut(&key) {
            // Skip edges without computed coordinates
            if x.is_none() {
                continue;
            }

            // Remove the padding we added in make_space_for_edge_labels
            if labelpos == "l" || labelpos == "r" {
                if rankdir == RankDir::TB || rankdir == RankDir::BT {
                    edge.width -= labeloffset;
                } else {
                    edge.height -= labeloffset;
                }
            }

            // Adjust x coordinate based on labelpos
            if let Some(x_val) = x {
                match labelpos.as_str() {
                    "l" => {
                        edge.x = Some(x_val - width / 2.0 - labeloffset);
                    }
                    "r" => {
                        edge.x = Some(x_val + width / 2.0 + labeloffset);
                    }
                    _ => {} // "c" - no adjustment
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::dagre::graph::EdgeLabel;

    #[test]
    fn test_make_space_doubles_minlen() {
        let mut g = DagreGraph::new();
        g.set_node("a", NodeLabel::default());
        g.set_node("b", NodeLabel::default());
        g.set_edge(
            "a",
            "b",
            EdgeLabel {
                minlen: 1,
                ..Default::default()
            },
        );

        make_space_for_edge_labels(&mut g, RankDir::TB);

        let edge = g.edge("a", "b").unwrap();
        assert_eq!(edge.minlen, 2);
    }

    #[test]
    fn test_make_space_halves_ranksep() {
        let mut g = DagreGraph::new();
        g.graph_mut().ranksep = 100.0;

        make_space_for_edge_labels(&mut g, RankDir::TB);

        assert_eq!(g.graph().ranksep, 50.0);
    }

    #[test]
    fn test_inject_creates_proxy_for_labeled_edge() {
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
                rank: Some(4),
                ..Default::default()
            },
        );
        g.set_edge(
            "a",
            "b",
            EdgeLabel {
                width: 50.0,
                height: 20.0,
                ..Default::default()
            },
        );

        let node_count_before = g.node_count();
        inject_edge_label_proxies(&mut g);

        // Should have added a proxy node
        assert_eq!(g.node_count(), node_count_before + 1);

        // Find the proxy
        let nodes = g.nodes();
        let proxy = nodes
            .iter()
            .find(|v| {
                g.node(v)
                    .map(|n| n.dummy.as_deref() == Some("edge-proxy"))
                    .unwrap_or(false)
            })
            .unwrap();

        let proxy_node = g.node(proxy).unwrap();
        assert_eq!(proxy_node.rank, Some(2)); // Midpoint of 0 and 4
    }

    #[test]
    fn test_remove_stores_label_rank() {
        let mut g = DagreGraph::new();
        g.set_node("a", NodeLabel::default());
        g.set_node("b", NodeLabel::default());
        g.set_edge(
            "a",
            "b",
            EdgeLabel {
                width: 50.0,
                height: 20.0,
                ..Default::default()
            },
        );
        g.set_node(
            "_ep_1",
            NodeLabel {
                dummy: Some("edge-proxy".to_string()),
                rank: Some(5),
                edge_obj: Some(("a".to_string(), "b".to_string(), None)),
                ..Default::default()
            },
        );

        remove_edge_label_proxies(&mut g);

        // Proxy should be removed
        assert!(g.node("_ep_1").is_none());

        // Edge should have label_rank set
        let edge = g.edge("a", "b").unwrap();
        assert_eq!(edge.label_rank, Some(5));
    }
}
