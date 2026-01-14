//! Hierarchical subgraph sorting for crossing minimization
//!
//! This module implements the dagre.js approach to sorting nodes within subgraphs,
//! which recursively sorts children of each subgraph while maintaining constraints
//! between sibling subgraphs.

use super::barycenter::{barycenter, barycenter_down, BarycenterEntry};
use super::resolve_conflicts::{resolve_conflicts, ConstraintGraph, ResolvedEntry};
use super::sort::{sort, SortResult};
use crate::layout::dagre::graph::DagreGraph;
use std::collections::HashMap;

/// Sort a subgraph and its nested subgraphs hierarchically
///
/// This function recursively sorts the children of a subgraph, merging barycenters
/// from nested subgraphs and respecting constraints from the constraint graph.
///
/// # Arguments
/// * `g` - The graph (must have order attributes set on fixed layer)
/// * `parent` - The parent subgraph to sort children of (None for root)
/// * `cg` - Constraint graph for sibling subgraph ordering
/// * `bias_right` - Tie-breaking direction
/// * `use_predecessors` - If true, use in-edges (predecessors); if false, use out-edges (successors)
pub fn sort_subgraph(
    g: &DagreGraph,
    parent: Option<&str>,
    cg: &ConstraintGraph,
    bias_right: bool,
    use_predecessors: bool,
) -> SortResult {
    // Get children of this parent
    let children: Vec<String> = match parent {
        Some(p) => g.children(p).into_iter().map(|s| s.to_string()).collect(),
        None => g
            .root_children()
            .into_iter()
            .map(|s| s.to_string())
            .collect(),
    };

    if children.is_empty() {
        return SortResult {
            vs: Vec::new(),
            barycenter: None,
            weight: 0.0,
        };
    }

    // Get border nodes for this subgraph (if it is a subgraph)
    let (border_left, border_right) = if let Some(p) = parent {
        if let Some(node) = g.node(p) {
            // For subgraphs, border_left/right are indexed by rank offset from min_rank
            // We need to find which rank we're operating on
            // For now, get the first non-None border nodes
            let bl = node.border_left.iter().find_map(|opt| opt.clone());
            let br = node.border_right.iter().find_map(|opt| opt.clone());
            (bl, br)
        } else {
            (None, None)
        }
    } else {
        (None, None)
    };

    // Filter out border nodes from movable children
    let movable: Vec<String> = children
        .iter()
        .filter(|v| {
            Some(v.as_str()) != border_left.as_deref()
                && Some(v.as_str()) != border_right.as_deref()
        })
        .cloned()
        .collect();

    // Calculate barycenters for movable children
    let mut barycenters: Vec<BarycenterEntry> = if use_predecessors {
        barycenter(g, &movable)
    } else {
        barycenter_down(g, &movable)
    };

    // Store results from nested subgraph sorts
    let mut subgraph_results: HashMap<String, SortResult> = HashMap::new();

    // For each child that is a subgraph, recursively sort and merge barycenters
    for entry in &mut barycenters {
        let child_children = g.children(&entry.v);
        if !child_children.is_empty() {
            // This child is itself a subgraph - recursively sort it
            let subgraph_result =
                sort_subgraph(g, Some(&entry.v), cg, bias_right, use_predecessors);

            // Merge barycenters
            if let Some(sub_bc) = subgraph_result.barycenter {
                if let Some(entry_bc) = entry.barycenter {
                    // Weighted average
                    entry.barycenter = Some(
                        (entry_bc * entry.weight + sub_bc * subgraph_result.weight)
                            / (entry.weight + subgraph_result.weight),
                    );
                    entry.weight += subgraph_result.weight;
                } else {
                    entry.barycenter = Some(sub_bc);
                    entry.weight = subgraph_result.weight;
                }
            }

            subgraph_results.insert(entry.v.clone(), subgraph_result);
        }
    }

    // Resolve conflicts with constraint graph
    let resolved = resolve_conflicts(barycenters, cg);

    // Expand subgraphs in resolved entries
    let expanded = expand_subgraphs(resolved, &subgraph_results);

    // Sort the expanded entries
    let mut sorted = sort(expanded, bias_right);

    // Add border nodes at edges
    if let Some(bl) = &border_left {
        if children.contains(bl) {
            sorted.vs.insert(0, bl.clone());
        }
    }
    if let Some(br) = &border_right {
        if children.contains(br) {
            sorted.vs.push(br.clone());
        }
    }

    // Update barycenter based on border node predecessors
    if border_left.is_some() {
        if let Some(bl) = &border_left {
            let bl_preds: Vec<_> = if use_predecessors {
                g.in_edges(bl).iter().map(|e| e.v.clone()).collect()
            } else {
                g.out_edges(bl).iter().map(|e| e.w.clone()).collect()
            };

            if let Some(br) = &border_right {
                let br_preds: Vec<_> = if use_predecessors {
                    g.in_edges(br).iter().map(|e| e.v.clone()).collect()
                } else {
                    g.out_edges(br).iter().map(|e| e.w.clone()).collect()
                };

                if !bl_preds.is_empty() && !br_preds.is_empty() {
                    let bl_pred_order = bl_preds
                        .first()
                        .and_then(|p| g.node(p))
                        .and_then(|n| n.order)
                        .unwrap_or(0) as f64;
                    let br_pred_order = br_preds
                        .first()
                        .and_then(|p| g.node(p))
                        .and_then(|n| n.order)
                        .unwrap_or(0) as f64;

                    let bc = sorted.barycenter.unwrap_or(0.0);
                    let w = sorted.weight;

                    sorted.barycenter = Some((bc * w + bl_pred_order + br_pred_order) / (w + 2.0));
                    sorted.weight = w + 2.0;
                }
            }
        }
    }

    sorted
}

/// Expand resolved entries by replacing subgraph nodes with their sorted children
fn expand_subgraphs(
    entries: Vec<ResolvedEntry>,
    subgraphs: &HashMap<String, SortResult>,
) -> Vec<BarycenterEntry> {
    entries
        .into_iter()
        .flat_map(|entry| {
            let expanded_vs: Vec<String> = entry
                .vs
                .into_iter()
                .flat_map(|v| {
                    if let Some(result) = subgraphs.get(&v) {
                        result.vs.clone()
                    } else {
                        vec![v]
                    }
                })
                .collect();

            // Convert back to BarycenterEntry for sorting
            // All nodes in an entry share the same barycenter and original index
            expanded_vs
                .into_iter()
                .map(|v| BarycenterEntry {
                    v,
                    barycenter: entry.barycenter,
                    weight: entry.weight,
                    i: entry.i,
                })
                .collect::<Vec<_>>()
        })
        .collect()
}

/// Add constraints between sibling subgraphs based on the current ordering
///
/// After sorting a layer, this function walks through the sorted nodes and adds
/// constraint edges between consecutive sibling subgraphs to preserve their
/// relative ordering in subsequent sweeps.
pub fn add_subgraph_constraints(g: &DagreGraph, cg: &mut ConstraintGraph, vs: &[String]) {
    // Track the previous child for each parent
    let mut prev: HashMap<Option<String>, String> = HashMap::new();
    let mut root_prev: Option<String> = None;

    for v in vs {
        let mut child = g.parent(v).map(|s| s.to_string());
        let mut parent: Option<String>;

        while child.is_some() || root_prev.is_none() {
            parent = child
                .as_ref()
                .and_then(|c| g.parent(c).map(|s| s.to_string()));

            let prev_child = if parent.is_some() {
                prev.get(&parent).cloned()
            } else {
                root_prev.clone()
            };

            if parent.is_some() {
                prev.insert(parent.clone(), child.clone().unwrap_or_default());
            } else {
                root_prev = child.clone();
            }

            if let Some(pc) = prev_child {
                if let Some(ref c) = child {
                    if pc != *c {
                        cg.set_edge(&pc, c);
                        return; // Return after adding first constraint
                    }
                }
            }

            child = parent;
            if child.is_none() {
                break;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::dagre::graph::NodeLabel;

    #[test]
    fn test_sort_subgraph_simple() {
        let mut g = DagreGraph::new();

        // Create a simple graph: parent with two children
        g.set_node(
            "a",
            NodeLabel {
                order: Some(0),
                rank: Some(0),
                ..Default::default()
            },
        );
        g.set_node(
            "b",
            NodeLabel {
                order: Some(1),
                rank: Some(0),
                ..Default::default()
            },
        );

        let cg = ConstraintGraph::new();
        let result = sort_subgraph(&g, None, &cg, false, true);

        assert_eq!(result.vs.len(), 2);
    }

    #[test]
    fn test_add_subgraph_constraints() {
        let mut g = DagreGraph::new();

        // Create parent and children
        g.set_node("parent", NodeLabel::default());
        g.set_node("child1", NodeLabel::default());
        g.set_node("child2", NodeLabel::default());
        g.set_parent("child1", "parent");
        g.set_parent("child2", "parent");

        let mut cg = ConstraintGraph::new();
        add_subgraph_constraints(&g, &mut cg, &["child1".to_string(), "child2".to_string()]);

        // Should not add constraints between children of the same parent in this case
        // (they're not subgraphs themselves)
    }
}
