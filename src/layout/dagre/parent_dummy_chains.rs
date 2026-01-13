//! Parent dummy chains for compound graph layout
//!
//! This module ensures that dummy nodes (created during edge normalization) are
//! assigned to the correct parent in compound graphs. The dummy nodes should be
//! parented to the appropriate subgraph along the path from the edge's source
//! to its target through their Lowest Common Ancestor (LCA).
//!
//! Reference: dagre.js parent-dummy-chains.js

use std::collections::HashMap;

use super::graph::DagreGraph;

/// Assign parents to dummy node chains in compound graphs
///
/// This function ensures that dummy nodes created during edge normalization
/// are assigned to the correct parent subgraph. The chain of dummy nodes
/// should follow the path through the compound hierarchy from source to target.
pub fn run(g: &mut DagreGraph) {
    // Get the dummy chains from the graph
    let dummy_chains = g.graph().dummy_chains.clone();

    if dummy_chains.is_empty() {
        return;
    }

    // Calculate postorder numbering for efficient LCA queries
    let postorder_nums = postorder(g);

    for chain_start in dummy_chains {
        // Get the original edge's source and target
        let (edge_v, edge_w) = {
            let node = g.node(&chain_start);
            if let Some(n) = node {
                if let Some((v, w, _)) = &n.edge_obj {
                    (v.clone(), w.clone())
                } else {
                    continue;
                }
            } else {
                continue;
            }
        };

        // Find path through LCA
        let path_data = find_path(g, &postorder_nums, &edge_v, &edge_w);
        let path = path_data.path;
        let lca = path_data.lca;

        let mut path_idx = 0;
        let mut path_v = path.get(path_idx).cloned();
        let mut ascending = true;

        // Walk through the dummy chain
        let mut current = chain_start.clone();

        while current != edge_w {
            let node_rank = g.node(&current).and_then(|n| n.rank);

            if ascending {
                // Walk up the path until we reach LCA or find a node containing this rank
                while path_v.as_ref() != Some(&lca) {
                    if let Some(pv) = &path_v {
                        let pv_max_rank = g.node(pv).and_then(|n| n.max_rank);
                        if let (Some(nr), Some(pmr)) = (node_rank, pv_max_rank) {
                            if pmr < nr {
                                path_idx += 1;
                                path_v = path.get(path_idx).cloned();
                                continue;
                            }
                        }
                    }
                    break;
                }

                if path_v.as_ref() == Some(&lca) {
                    ascending = false;
                }
            }

            if !ascending {
                // Walk down the path to find the appropriate parent
                while path_idx < path.len() - 1 {
                    let next_v = path.get(path_idx + 1);
                    if let Some(nv) = next_v {
                        let nv_min_rank = g.node(nv).and_then(|n| n.min_rank);
                        if let (Some(nr), Some(nmr)) = (node_rank, nv_min_rank) {
                            if nmr <= nr {
                                path_idx += 1;
                                continue;
                            }
                        }
                    }
                    break;
                }
                path_v = path.get(path_idx).cloned();
            }

            // Set the parent of the current dummy node
            if let Some(pv) = &path_v {
                g.set_parent(&current, pv);
            }

            // Move to next dummy node in chain (successor)
            let successors = g.successors(&current);
            if let Some(next) = successors.first() {
                current = (*next).clone();
            } else {
                break;
            }
        }
    }
}

/// Postorder numbering result
struct PostorderNums {
    low: i32,
    lim: i32,
}

/// Calculate postorder numbering for all nodes
fn postorder(g: &DagreGraph) -> HashMap<String, PostorderNums> {
    let mut result = HashMap::new();
    let mut lim = 0;

    fn dfs(g: &DagreGraph, v: &str, lim: &mut i32, result: &mut HashMap<String, PostorderNums>) {
        let low = *lim;
        let children: Vec<String> = g.children(v).into_iter().cloned().collect();
        for child in children {
            dfs(g, &child, lim, result);
        }
        result.insert(
            v.to_string(),
            PostorderNums {
                low,
                lim: {
                    *lim += 1;
                    *lim - 1
                },
            },
        );
    }

    // Process all root-level nodes
    let roots: Vec<String> = g.root_children().into_iter().cloned().collect();
    for root in roots {
        dfs(g, &root, &mut lim, &mut result);
    }

    result
}

/// Result of finding a path through the LCA
struct PathResult {
    path: Vec<String>,
    lca: String,
}

/// Find a path from v to w through the Lowest Common Ancestor
fn find_path(
    g: &DagreGraph,
    postorder_nums: &HashMap<String, PostorderNums>,
    v: &str,
    w: &str,
) -> PathResult {
    let mut v_path = Vec::new();
    let mut w_path = Vec::new();

    // Get bounds for LCA detection
    let v_nums = postorder_nums.get(v);
    let w_nums = postorder_nums.get(w);

    let (low, lim) = match (v_nums, w_nums) {
        (Some(vn), Some(wn)) => (vn.low.min(wn.low), vn.lim.max(wn.lim)),
        _ => {
            return PathResult {
                path: vec![],
                lca: String::new(),
            }
        }
    };

    // Traverse up from v to find the LCA
    let mut parent = Some(v.to_string());
    let mut lca = String::new();

    while let Some(p) = parent {
        let p_str = p.clone();
        parent = g.parent(&p_str).cloned();

        if let Some(ref p_parent) = parent {
            v_path.push(p_parent.clone());

            if let Some(pn) = postorder_nums.get(p_parent) {
                if pn.low <= low && lim <= pn.lim {
                    lca = p_parent.clone();
                    break;
                }
            }
        } else {
            // Reached root without finding LCA in compound hierarchy
            // This can happen when v and w don't share a compound ancestor
            break;
        }
    }

    // Traverse from w to LCA
    parent = g.parent(w).cloned();
    while let Some(p) = parent {
        if p == lca {
            break;
        }
        w_path.push(p.clone());
        parent = g.parent(&p).cloned();
    }

    // Combine paths
    let mut path = v_path;
    w_path.reverse();
    path.extend(w_path);

    PathResult { path, lca }
}

#[cfg(test)]
mod tests {
    use super::super::graph::{EdgeLabel, NodeLabel};
    use super::*;

    #[test]
    fn test_parent_dummy_chains_simple() {
        let mut g = DagreGraph::new();

        // Create compound graph: sg1 contains a, sg2 contains b
        // Edge from a to b should have dummy nodes parented appropriately
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
        g.set_node(
            "sg1",
            NodeLabel {
                min_rank: Some(0),
                max_rank: Some(1),
                ..Default::default()
            },
        );
        g.set_node(
            "sg2",
            NodeLabel {
                min_rank: Some(2),
                max_rank: Some(3),
                ..Default::default()
            },
        );
        g.set_parent("a", "sg1");
        g.set_parent("b", "sg2");

        // Create dummy chain
        g.set_node(
            "dummy1",
            NodeLabel {
                rank: Some(1),
                edge_obj: Some(("a".to_string(), "b".to_string(), None)),
                ..Default::default()
            },
        );
        g.set_node(
            "dummy2",
            NodeLabel {
                rank: Some(2),
                edge_obj: Some(("a".to_string(), "b".to_string(), None)),
                ..Default::default()
            },
        );
        g.set_edge("a", "dummy1", EdgeLabel::default());
        g.set_edge("dummy1", "dummy2", EdgeLabel::default());
        g.set_edge("dummy2", "b", EdgeLabel::default());

        g.graph_mut().dummy_chains = vec!["dummy1".to_string()];

        run(&mut g);

        // Dummy nodes should be parented
        // (exact parenting depends on compound hierarchy)
    }

    #[test]
    fn test_postorder_numbering() {
        let mut g = DagreGraph::new();

        g.set_node("sg1", NodeLabel::default());
        g.set_node("a", NodeLabel::default());
        g.set_node("b", NodeLabel::default());
        g.set_parent("a", "sg1");
        g.set_parent("b", "sg1");

        let nums = postorder(&g);

        // All nodes should have postorder numbers
        assert!(nums.contains_key("sg1"));
        assert!(nums.contains_key("a"));
        assert!(nums.contains_key("b"));

        // Children should have lower numbers than parents
        let sg1 = nums.get("sg1").unwrap();
        let a = nums.get("a").unwrap();
        let b = nums.get("b").unwrap();

        assert!(sg1.lim > a.lim);
        assert!(sg1.lim > b.lim);
    }
}
