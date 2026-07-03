//! Hierarchical subgraph sorting for crossing minimization
//!
//! Port of dagre's lib/order/sort-subgraph.js. Operates on a layer graph
//! built by `build_layer_graph`: barycenters are computed from the layer
//! graph's in-edges (fixed layer -> movable layer), nested subgraphs are
//! sorted recursively with merged barycenters, and border nodes are placed
//! at the extremes of their subgraph.

use super::barycenter::{barycenter, BarycenterEntry};
use super::resolve_conflicts::{resolve_conflicts, ConstraintGraph, ResolvedEntry};
use super::sort::{sort, SortResult};
use crate::layout::dagre::graph::DagreGraph;
use std::collections::HashMap;

/// Sort the children of `v` in the layer graph `g`, recursing into nested
/// subgraphs and respecting constraints from the constraint graph.
pub fn sort_subgraph(
    g: &DagreGraph,
    v: &str,
    cg: &ConstraintGraph,
    bias_right: bool,
) -> SortResult {
    let mut movable: Vec<String> = g.children(v).into_iter().cloned().collect();
    let node = g.node(v);
    let bl = node.and_then(|n| n.border_left.first().cloned().flatten());
    let br = node.and_then(|n| n.border_right.first().cloned().flatten());
    let mut subgraphs: HashMap<String, SortResult> = HashMap::new();

    if bl.is_some() {
        movable.retain(|w| Some(w.as_str()) != bl.as_deref() && Some(w.as_str()) != br.as_deref());
    }

    let mut barycenters: Vec<BarycenterEntry> = barycenter(g, &movable);
    for entry in barycenters.iter_mut() {
        if !g.children(&entry.v).is_empty() {
            let subgraph_result = sort_subgraph(g, &entry.v, cg, bias_right);
            if subgraph_result.barycenter.is_some() {
                merge_barycenters(entry, &subgraph_result);
            }
            subgraphs.insert(entry.v.clone(), subgraph_result);
        }
    }

    let mut entries = resolve_conflicts(barycenters, cg);
    expand_subgraphs(&mut entries, &subgraphs);

    let mut result = sort(entries, bias_right);

    if let (Some(bl), Some(br)) = (bl, br) {
        let mut vs = Vec::with_capacity(result.vs.len() + 2);
        vs.push(bl.clone());
        vs.append(&mut result.vs);
        vs.push(br.clone());
        result.vs = vs;

        let bl_preds = g.predecessors(&bl);
        if !bl_preds.is_empty() {
            let bl_pred_order = g.node(bl_preds[0]).and_then(|n| n.order).unwrap_or(0) as f64;
            let br_pred_order = g
                .predecessors(&br)
                .first()
                .and_then(|p| g.node(p))
                .and_then(|n| n.order)
                .unwrap_or(0) as f64;

            let bc = result.barycenter.unwrap_or(0.0);
            let weight = result.weight;
            result.barycenter =
                Some((bc * weight + bl_pred_order + br_pred_order) / (weight + 2.0));
            result.weight = weight + 2.0;
        }
    }

    result
}

/// Replace subgraph ids in resolved entries with their sorted children
fn expand_subgraphs(entries: &mut [ResolvedEntry], subgraphs: &HashMap<String, SortResult>) {
    for entry in entries.iter_mut() {
        let mut expanded: Vec<String> = Vec::with_capacity(entry.vs.len());
        for v in entry.vs.drain(..) {
            if let Some(result) = subgraphs.get(&v) {
                expanded.extend(result.vs.iter().cloned());
            } else {
                expanded.push(v);
            }
        }
        entry.vs = expanded;
    }
}

/// Merge a nested subgraph's barycenter into its entry (weighted average)
fn merge_barycenters(target: &mut BarycenterEntry, other: &SortResult) {
    let other_bc = other.barycenter.expect("caller checked barycenter");
    if let Some(target_bc) = target.barycenter {
        target.barycenter = Some(
            (target_bc * target.weight + other_bc * other.weight) / (target.weight + other.weight),
        );
        target.weight += other.weight;
    } else {
        target.barycenter = Some(other_bc);
        target.weight = other.weight;
    }
}

/// Add constraints between sibling subgraphs based on the current ordering
///
/// Port of dagre's lib/order/add-subgraph-constraints.js. After sorting a
/// layer, walks each sorted node's parent chain and adds a constraint edge
/// the first time the previous sibling under a shared ancestor differs.
pub fn add_subgraph_constraints(g: &DagreGraph, cg: &mut ConstraintGraph, vs: &[String]) {
    let mut prev: HashMap<String, String> = HashMap::new();
    let mut root_prev: Option<String> = None;

    for v in vs {
        let mut child = g.parent(v).cloned();
        while let Some(c) = child {
            let parent = g.parent(&c).cloned();
            let prev_child = if let Some(p) = &parent {
                let pc = prev.get(p).cloned();
                prev.insert(p.clone(), c.clone());
                pc
            } else {
                let pc = root_prev.clone();
                root_prev = Some(c.clone());
                pc
            };
            if let Some(pc) = prev_child {
                if pc != c {
                    cg.set_edge(&pc, &c);
                    break; // continue with the next v (JS `return` in forEach)
                }
            }
            child = parent;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::dagre::graph::{EdgeLabel, NodeLabel};

    /// Build the base graph used by dagre's sort-subgraph tests: fixed-layer
    /// nodes "0".."4" with order 0..4.
    fn base_graph() -> DagreGraph {
        let mut g = DagreGraph::new();
        for i in 0..5 {
            g.set_node(
                i.to_string(),
                NodeLabel {
                    order: Some(i),
                    ..Default::default()
                },
            );
        }
        g
    }

    fn weighted(weight: i32) -> EdgeLabel {
        EdgeLabel {
            weight,
            ..Default::default()
        }
    }

    #[test]
    fn test_sorts_flat_subgraph_by_barycenter() {
        let mut g = base_graph();
        g.set_edge("3", "x", weighted(1));
        g.set_edge("1", "y", weighted(2));
        g.set_edge("4", "y", weighted(1));
        g.set_parent("x", "movable");
        g.set_parent("y", "movable");

        let cg = ConstraintGraph::new();
        let result = sort_subgraph(&g, "movable", &cg, false);
        assert_eq!(result.vs, vec!["y", "x"]);
    }

    #[test]
    fn test_preserves_pos_of_node_without_neighbors() {
        let mut g = base_graph();
        g.set_edge("3", "x", weighted(1));
        g.set_node("y", NodeLabel::default());
        g.set_edge("1", "z", weighted(2));
        g.set_edge("4", "z", weighted(1));
        g.set_parent("x", "movable");
        g.set_parent("y", "movable");
        g.set_parent("z", "movable");

        let cg = ConstraintGraph::new();
        let result = sort_subgraph(&g, "movable", &cg, false);
        assert_eq!(result.vs, vec!["z", "y", "x"]);
    }

    #[test]
    fn test_biases_left_without_reverse_bias() {
        let mut g = base_graph();
        g.set_edge("1", "x", weighted(1));
        g.set_edge("1", "y", weighted(1));
        g.set_parent("x", "movable");
        g.set_parent("y", "movable");

        let cg = ConstraintGraph::new();
        let result = sort_subgraph(&g, "movable", &cg, false);
        assert_eq!(result.vs, vec!["x", "y"]);
    }

    #[test]
    fn test_biases_right_with_reverse_bias() {
        let mut g = base_graph();
        g.set_edge("1", "x", weighted(1));
        g.set_edge("1", "y", weighted(1));
        g.set_parent("x", "movable");
        g.set_parent("y", "movable");

        let cg = ConstraintGraph::new();
        let result = sort_subgraph(&g, "movable", &cg, true);
        assert_eq!(result.vs, vec!["y", "x"]);
    }

    #[test]
    fn test_aggregates_stats_about_subgraph() {
        let mut g = base_graph();
        g.set_edge("3", "x", weighted(1));
        g.set_edge("1", "y", weighted(2));
        g.set_edge("4", "y", weighted(1));
        g.set_parent("x", "movable");
        g.set_parent("y", "movable");

        let cg = ConstraintGraph::new();
        let result = sort_subgraph(&g, "movable", &cg, false);
        assert_eq!(result.barycenter, Some(2.25));
        assert_eq!(result.weight, 4.0);
    }

    #[test]
    fn test_sorts_nested_subgraph_with_no_barycenter() {
        let mut g = base_graph();
        for v in ["a", "b", "c"] {
            g.set_node(v, NodeLabel::default());
            g.set_parent(v, "y");
        }
        g.set_edge("0", "x", weighted(1));
        g.set_edge("1", "z", weighted(1));
        g.set_edge("2", "y", weighted(1));
        g.set_parent("x", "movable");
        g.set_parent("y", "movable");
        g.set_parent("z", "movable");

        let cg = ConstraintGraph::new();
        let result = sort_subgraph(&g, "movable", &cg, false);
        assert_eq!(result.vs, vec!["x", "z", "a", "b", "c"]);
    }

    #[test]
    fn test_sorts_nested_subgraph_with_barycenter() {
        let mut g = base_graph();
        for v in ["a", "b", "c"] {
            g.set_node(v, NodeLabel::default());
            g.set_parent(v, "y");
        }
        g.set_edge("0", "a", weighted(3));
        g.set_edge("0", "x", weighted(1));
        g.set_edge("1", "z", weighted(1));
        g.set_edge("2", "y", weighted(1));
        g.set_parent("x", "movable");
        g.set_parent("y", "movable");
        g.set_parent("z", "movable");

        let cg = ConstraintGraph::new();
        let result = sort_subgraph(&g, "movable", &cg, false);
        assert_eq!(result.vs, vec!["x", "a", "b", "c", "z"]);
    }

    #[test]
    fn test_sorts_nested_subgraph_with_no_in_edges() {
        let mut g = base_graph();
        for v in ["a", "b", "c"] {
            g.set_node(v, NodeLabel::default());
            g.set_parent(v, "y");
        }
        g.set_edge("0", "a", weighted(1));
        g.set_edge("1", "b", weighted(1));
        g.set_edge("0", "x", weighted(1));
        g.set_edge("1", "z", weighted(1));
        g.set_parent("x", "movable");
        g.set_parent("y", "movable");
        g.set_parent("z", "movable");

        let cg = ConstraintGraph::new();
        let result = sort_subgraph(&g, "movable", &cg, false);
        assert_eq!(result.vs, vec!["x", "a", "b", "c", "z"]);
    }

    #[test]
    fn test_sorts_border_nodes_to_extremes_of_subgraph() {
        let mut g = base_graph();
        g.set_edge("0", "x", weighted(1));
        g.set_edge("1", "y", weighted(1));
        g.set_edge("2", "z", weighted(1));
        g.set_node(
            "sg1",
            NodeLabel {
                border_left: vec![Some("bl".to_string())],
                border_right: vec![Some("br".to_string())],
                ..Default::default()
            },
        );
        for v in ["x", "y", "z", "bl", "br"] {
            g.set_parent(v, "sg1");
        }

        let cg = ConstraintGraph::new();
        let result = sort_subgraph(&g, "sg1", &cg, false);
        assert_eq!(result.vs, vec!["bl", "x", "y", "z", "br"]);
    }

    #[test]
    fn test_assigns_barycenter_from_previous_border_nodes() {
        let mut g = base_graph();
        g.set_node(
            "bl1",
            NodeLabel {
                order: Some(0),
                ..Default::default()
            },
        );
        g.set_node(
            "br1",
            NodeLabel {
                order: Some(1),
                ..Default::default()
            },
        );
        g.set_edge("bl1", "bl2", weighted(1));
        g.set_edge("br1", "br2", weighted(1));
        g.set_parent("bl2", "sg");
        g.set_parent("br2", "sg");
        g.set_node(
            "sg",
            NodeLabel {
                border_left: vec![Some("bl2".to_string())],
                border_right: vec![Some("br2".to_string())],
                ..Default::default()
            },
        );

        let cg = ConstraintGraph::new();
        let result = sort_subgraph(&g, "sg", &cg, false);
        assert_eq!(result.barycenter, Some(0.5));
        assert_eq!(result.weight, 2.0);
        assert_eq!(result.vs, vec!["bl2", "br2"]);
    }

    #[test]
    fn test_add_subgraph_constraints_flat_nodes_no_change() {
        let mut g = DagreGraph::new();
        let vs: Vec<String> = ["a", "b", "c", "d"].iter().map(|s| s.to_string()).collect();
        for v in &vs {
            g.set_node(v.clone(), NodeLabel::default());
        }
        let mut cg = ConstraintGraph::new();
        add_subgraph_constraints(&g, &mut cg, &vs);
        assert_eq!(cg.edges().count(), 0);
    }

    #[test]
    fn test_add_subgraph_constraints_contiguous_same_parent_no_change() {
        let mut g = DagreGraph::new();
        let vs: Vec<String> = ["a", "b", "c"].iter().map(|s| s.to_string()).collect();
        for v in &vs {
            g.set_parent(v.clone(), "sg");
        }
        let mut cg = ConstraintGraph::new();
        add_subgraph_constraints(&g, &mut cg, &vs);
        assert_eq!(cg.edges().count(), 0);
    }

    #[test]
    fn test_add_subgraph_constraints_different_parents() {
        let mut g = DagreGraph::new();
        g.set_parent("a", "sg1");
        g.set_parent("b", "sg2");
        let vs: Vec<String> = vec!["a".to_string(), "b".to_string()];
        let mut cg = ConstraintGraph::new();
        add_subgraph_constraints(&g, &mut cg, &vs);
        let edges: Vec<(String, String)> = cg
            .edges()
            .map(|(v, w)| (v.to_string(), w.to_string()))
            .collect();
        assert_eq!(edges, vec![("sg1".to_string(), "sg2".to_string())]);
    }

    #[test]
    fn test_add_subgraph_constraints_multiple_levels() {
        // Port of dagre add-subgraph-constraints-test.js "works for multiple
        // levels". The JS `return` inside vs.forEach continues with the next
        // v, so BOTH constraints must be added.
        let mut g = DagreGraph::new();
        for v in ["a", "b", "c", "d", "e", "f", "g", "h"] {
            g.set_node(v, NodeLabel::default());
        }
        g.set_parent("b", "sg2");
        g.set_parent("sg2", "sg1");
        g.set_parent("c", "sg1");
        g.set_parent("d", "sg3");
        g.set_parent("sg3", "sg1");
        g.set_parent("f", "sg4");
        g.set_parent("g", "sg5");
        g.set_parent("sg5", "sg4");

        let mut cg = ConstraintGraph::new();
        let vs: Vec<String> = ["a", "b", "c", "d", "e", "f", "g", "h"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        add_subgraph_constraints(&g, &mut cg, &vs);

        let mut edges: Vec<(String, String)> = cg
            .edges()
            .map(|(v, w)| (v.to_string(), w.to_string()))
            .collect();
        edges.sort();
        assert_eq!(
            edges,
            vec![
                ("sg1".to_string(), "sg4".to_string()),
                ("sg2".to_string(), "sg3".to_string())
            ]
        );
    }
}
