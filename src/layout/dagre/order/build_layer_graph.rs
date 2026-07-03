//! Layer graph construction for crossing minimization
//!
//! Port of dagre's lib/order/build-layer-graph.js.
//!
//! Constructs a graph that can be used to sort a layer of nodes. The graph
//! contains all base and subgraph nodes from the requested layer in their
//! original hierarchy and any edges that are incident on these nodes and are
//! of the type requested by the "relationship" parameter.
//!
//! Nodes from the requested rank that do not have parents are assigned the
//! synthetic root node of the output graph, which makes it easy to walk the
//! hierarchy of movable nodes during ordering.

use crate::layout::dagre::graph::{DagreGraph, EdgeLabel, NodeLabel};

/// Which edges incident on movable nodes are copied into the layer graph
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Relationship {
    /// Copy in-edges (used for down sweeps)
    InEdges,
    /// Copy out-edges (used for up sweeps)
    OutEdges,
}

/// Build a layer graph for the given rank.
///
/// Returns the layer graph and the id of its synthetic root node.
///
/// Pre-conditions (as in dagre):
/// 1. Input graph is a DAG
/// 2. Base nodes in the input graph have a rank attribute
/// 3. Subgraph nodes in the input graph have minRank and maxRank attributes
/// 4. Edges have an assigned weight
///
/// Post-conditions:
/// 1. Output graph has all nodes in the movable rank with preserved hierarchy
/// 2. Root nodes in the movable layer are made children of the synthetic root
/// 3. Non-movable nodes incident on movable nodes, selected by the
///    relationship parameter, are included in the graph (without hierarchy)
/// 4. Edges incident on movable nodes, selected by the relationship
///    parameter, are added to the output graph (`u -> v` with v movable)
/// 5. The weights for copied edges are aggregated as needed, since the
///    output graph is not a multi-graph
pub fn build_layer_graph(
    g: &DagreGraph,
    rank: i32,
    relationship: Relationship,
) -> (DagreGraph, String) {
    let mut result = DagreGraph::new();
    let root = create_root_node(g, &mut result);
    result.set_node(root.clone(), NodeLabel::default());

    for v in g.nodes() {
        let node = match g.node(v) {
            Some(n) => n,
            None => continue,
        };

        // Subgraph nodes span minRank..=maxRank; base nodes sit at their rank.
        //
        // dagre never ranks compound nodes (ranking runs on the non-compound
        // graph), so its `node.rank === rank` test only ever matches base
        // nodes. Selkie's ranking leaves a spurious rank on compound nodes,
        // so explicitly restrict the rank test to base (childless) nodes.
        let is_subgraph_span = matches!(
            (node.min_rank, node.max_rank),
            (Some(min_r), Some(max_r)) if min_r <= rank && rank <= max_r
        );
        let is_base_at_rank = node.rank == Some(rank) && g.children(v).is_empty();

        if is_base_at_rank || is_subgraph_span {
            // Copy the node label from the input graph (dagre shares labels via
            // the default node label function; we copy the current state).
            set_node_label(&mut result, v, node.clone());
            let parent = g.parent(v).cloned().unwrap_or_else(|| root.clone());
            result.set_parent(v.clone(), parent);

            // This assumes we have only short edges!
            let edges = match relationship {
                Relationship::InEdges => g.in_edges(v),
                Relationship::OutEdges => g.out_edges(v),
            };
            for e in edges {
                let u = if &e.v == v { e.w.clone() } else { e.v.clone() };
                let existing_weight = result.edge(&u, v).map(|l| l.weight).unwrap_or(0);
                let weight = g.edge_by_key(e).map(|l| l.weight).unwrap_or(0);

                // Ensure the fixed-layer endpoint carries its label (its
                // `order` is read by the barycenter heuristic).
                if !result.has_node(&u) {
                    let u_label = g.node(&u).cloned().unwrap_or_default();
                    result.set_node(u.clone(), u_label);
                }

                result.set_edge(
                    u,
                    v.clone(),
                    EdgeLabel {
                        weight: weight + existing_weight,
                        ..Default::default()
                    },
                );
            }

            // Subgraph nodes get a minimal label holding only this rank's
            // border nodes (dagre replaces the label with
            // `{ borderLeft: node.borderLeft[rank], borderRight: node.borderRight[rank] }`).
            if let Some(min_r) = node.min_rank {
                let idx = (rank - min_r) as usize;
                let border_label = NodeLabel {
                    border_left: vec![node.border_left.get(idx).cloned().flatten()],
                    border_right: vec![node.border_right.get(idx).cloned().flatten()],
                    ..Default::default()
                };
                set_node_label(&mut result, v, border_label);
            }
        }
    }

    (result, root)
}

/// Overwrite (or create) a node's label
fn set_node_label(g: &mut DagreGraph, v: &str, label: NodeLabel) {
    if g.has_node(v) {
        *g.node_mut(v).expect("node exists") = label;
    } else {
        g.set_node(v.to_string(), label);
    }
}

/// Create a root node id that does not collide with any node in `g`
fn create_root_node(g: &DagreGraph, result: &mut DagreGraph) -> String {
    loop {
        let candidate = result.unique_id("_root");
        if !g.has_node(&candidate) {
            return candidate;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rank_node(rank: i32) -> NodeLabel {
        NodeLabel {
            rank: Some(rank),
            ..Default::default()
        }
    }

    #[test]
    fn test_places_movable_nodes_with_no_parents_under_root() {
        let mut g = DagreGraph::new();
        g.set_node("a", rank_node(1));
        g.set_node("b", rank_node(1));
        g.set_node("c", rank_node(2));
        g.set_node("d", rank_node(3));

        let (lg, root) = build_layer_graph(&g, 1, Relationship::InEdges);
        assert!(lg.has_node(&root));
        let children: Vec<&str> = lg.children(&root).into_iter().map(|s| s.as_str()).collect();
        assert_eq!(children, vec!["a", "b"]);
    }

    #[test]
    fn test_copies_flat_nodes_from_layer() {
        let mut g = DagreGraph::new();
        g.set_node("a", rank_node(1));
        g.set_node("b", rank_node(1));
        g.set_node("c", rank_node(2));
        g.set_node("d", rank_node(3));

        let (lg1, _) = build_layer_graph(&g, 1, Relationship::InEdges);
        assert!(lg1.has_node("a"));
        assert!(lg1.has_node("b"));
        let (lg2, _) = build_layer_graph(&g, 2, Relationship::InEdges);
        assert!(lg2.has_node("c"));
        let (lg3, _) = build_layer_graph(&g, 3, Relationship::InEdges);
        assert!(lg3.has_node("d"));
    }

    #[test]
    fn test_copies_in_edges_incident_on_rank_nodes() {
        let mut g = DagreGraph::new();
        g.set_node("a", rank_node(1));
        g.set_node("b", rank_node(1));
        g.set_node("c", rank_node(2));
        g.set_node("d", rank_node(3));
        g.set_edge(
            "a",
            "c",
            EdgeLabel {
                weight: 2,
                ..Default::default()
            },
        );
        g.set_edge(
            "b",
            "c",
            EdgeLabel {
                weight: 3,
                ..Default::default()
            },
        );
        g.set_edge(
            "c",
            "d",
            EdgeLabel {
                weight: 4,
                ..Default::default()
            },
        );

        let (lg1, _) = build_layer_graph(&g, 1, Relationship::InEdges);
        assert_eq!(lg1.edge_count(), 0);
        let (lg2, _) = build_layer_graph(&g, 2, Relationship::InEdges);
        assert_eq!(lg2.edge_count(), 2);
        assert_eq!(lg2.edge("a", "c").unwrap().weight, 2);
        assert_eq!(lg2.edge("b", "c").unwrap().weight, 3);
        let (lg3, _) = build_layer_graph(&g, 3, Relationship::InEdges);
        assert_eq!(lg3.edge_count(), 1);
        assert_eq!(lg3.edge("c", "d").unwrap().weight, 4);
    }

    #[test]
    fn test_copies_out_edges_incident_on_rank_nodes() {
        let mut g = DagreGraph::new();
        g.set_node("a", rank_node(1));
        g.set_node("b", rank_node(1));
        g.set_node("c", rank_node(2));
        g.set_node("d", rank_node(3));
        g.set_edge(
            "a",
            "c",
            EdgeLabel {
                weight: 2,
                ..Default::default()
            },
        );
        g.set_edge(
            "b",
            "c",
            EdgeLabel {
                weight: 3,
                ..Default::default()
            },
        );
        g.set_edge(
            "c",
            "d",
            EdgeLabel {
                weight: 4,
                ..Default::default()
            },
        );

        // Out-edges are reversed in the layer graph: fixed node -> movable node
        let (lg1, _) = build_layer_graph(&g, 1, Relationship::OutEdges);
        assert_eq!(lg1.edge_count(), 2);
        assert_eq!(lg1.edge("c", "a").unwrap().weight, 2);
        assert_eq!(lg1.edge("c", "b").unwrap().weight, 3);
        let (lg2, _) = build_layer_graph(&g, 2, Relationship::OutEdges);
        assert_eq!(lg2.edge_count(), 1);
        assert_eq!(lg2.edge("d", "c").unwrap().weight, 4);
        let (lg3, _) = build_layer_graph(&g, 3, Relationship::OutEdges);
        assert_eq!(lg3.edge_count(), 0);
    }

    #[test]
    fn test_collapses_multi_edges() {
        let mut g = DagreGraph::new();
        g.set_node("a", rank_node(1));
        g.set_node("b", rank_node(2));
        g.set_edge(
            "a",
            "b",
            EdgeLabel {
                weight: 2,
                ..Default::default()
            },
        );
        g.set_edge_with_name(
            "a",
            "b",
            EdgeLabel {
                weight: 3,
                ..Default::default()
            },
            "multi",
        );

        let (lg, _) = build_layer_graph(&g, 2, Relationship::InEdges);
        assert_eq!(lg.edge("a", "b").unwrap().weight, 5);
    }

    #[test]
    fn test_compound_nodes_only_included_via_min_max_span() {
        // dagre never assigns `rank` to compound nodes (ranking runs on
        // asNonCompoundGraph), so buildLayerGraph only picks them up via
        // minRank..maxRank. Selkie's ranking DOES leave a rank (0) on
        // compound nodes; that spurious rank must not pull a subgraph into
        // an unrelated layer graph.
        let mut g = DagreGraph::new();
        g.set_node("a", rank_node(1));
        g.set_node("bl", rank_node(1));
        g.set_node("br", rank_node(1));
        g.set_node("outside", rank_node(0));
        g.set_node(
            "sg",
            NodeLabel {
                // Spurious rank from selkie's longest-path ranking
                rank: Some(0),
                min_rank: Some(1),
                max_rank: Some(1),
                border_left: vec![Some("bl".to_string())],
                border_right: vec![Some("br".to_string())],
                ..Default::default()
            },
        );
        g.set_parent("a", "sg");
        g.set_parent("bl", "sg");
        g.set_parent("br", "sg");

        // Rank 0 layer graph must NOT contain the subgraph node
        let (lg0, root0) = build_layer_graph(&g, 0, Relationship::InEdges);
        assert!(!lg0.has_node("sg"), "sg must not be movable at rank 0");
        let children0: Vec<&str> = lg0
            .children(&root0)
            .into_iter()
            .map(|s| s.as_str())
            .collect();
        assert_eq!(children0, vec!["outside"]);

        // Rank 1 layer graph includes it via its span, with border label
        let (lg1, _) = build_layer_graph(&g, 1, Relationship::InEdges);
        assert!(lg1.has_node("sg"));
        let sg = lg1.node("sg").unwrap();
        assert_eq!(sg.border_left, vec![Some("bl".to_string())]);
        assert_eq!(sg.border_right, vec![Some("br".to_string())]);
    }

    #[test]
    fn test_preserves_hierarchy_for_movable_layer() {
        let mut g = DagreGraph::new();
        g.set_node("a", rank_node(0));
        g.set_node("b", rank_node(0));
        g.set_node("c", rank_node(0));
        g.set_node(
            "sg",
            NodeLabel {
                min_rank: Some(0),
                max_rank: Some(0),
                border_left: vec![Some("bl".to_string())],
                border_right: vec![Some("br".to_string())],
                ..Default::default()
            },
        );
        g.set_parent("a", "sg");
        g.set_parent("b", "sg");

        let (lg, root) = build_layer_graph(&g, 0, Relationship::InEdges);
        let mut root_children: Vec<&str> =
            lg.children(&root).into_iter().map(|s| s.as_str()).collect();
        root_children.sort();
        assert_eq!(root_children, vec!["c", "sg"]);
        assert_eq!(lg.parent("a"), Some(&"sg".to_string()));
        assert_eq!(lg.parent("b"), Some(&"sg".to_string()));
        // Subgraph label reduced to this rank's border nodes
        let sg = lg.node("sg").unwrap();
        assert_eq!(sg.border_left, vec![Some("bl".to_string())]);
        assert_eq!(sg.border_right, vec![Some("br".to_string())]);
    }
}
