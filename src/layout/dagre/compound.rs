//! Compound graph support functions for dagre layout
//!
//! This module contains helper functions for handling compound graphs (graphs with
//! subgraphs/clusters) in the dagre layout algorithm.

use super::graph::{DagreGraph, EdgeLabel, NodeLabel};

/// Assign minRank and maxRank to compound nodes based on their border node ranks
///
/// This must be called after ranking but before normalization.
/// It uses the border top/bottom nodes (created by nesting_graph) to determine
/// the vertical extent of each compound node.
pub fn assign_rank_min_max(g: &mut DagreGraph) {
    let mut max_rank = 0i32;

    // Collect compound nodes (nodes with children)
    let compound_nodes: Vec<String> = g
        .nodes()
        .iter()
        .filter(|v| !g.children(v).is_empty())
        .map(|v| (*v).clone())
        .collect();

    for v in compound_nodes {
        // Get border_top and border_bottom from the node
        let (border_top, border_bottom) = {
            let node = g.node(&v).unwrap();
            (node.border_top.clone(), node.border_bottom.clone())
        };

        if let (Some(bt), Some(bb)) = (border_top, border_bottom) {
            // Get ranks from border nodes
            let min_rank = g.node(&bt).and_then(|n| n.rank);
            let max_rank_node = g.node(&bb).and_then(|n| n.rank);

            if let (Some(min_r), Some(max_r)) = (min_rank, max_rank_node) {
                // Update the compound node
                if let Some(node) = g.node_mut(&v) {
                    node.min_rank = Some(min_r);
                    node.max_rank = Some(max_r);
                }
                max_rank = max_rank.max(max_r);
            }
        }
    }

    g.graph_mut().max_rank = Some(max_rank);
}

/// Add border segments (left and right border nodes) for each rank within a compound node
///
/// This creates border nodes at each rank from minRank to maxRank for each compound node,
/// and links consecutive border nodes with edges to ensure they stay aligned.
pub fn add_border_segments(g: &mut DagreGraph) {
    // Process compound nodes in a DFS manner
    fn dfs(g: &mut DagreGraph, v: &str) {
        // First process children
        let children: Vec<String> = g.children(v).into_iter().cloned().collect();
        for child in children {
            dfs(g, &child);
        }

        // Then add border segments for this node if it's compound
        let (min_rank, max_rank) = {
            let node = g.node(v);
            node.map(|n| (n.min_rank, n.max_rank))
                .unwrap_or((None, None))
        };

        if let (Some(min_r), Some(max_r)) = (min_rank, max_rank) {
            // Create border left and right arrays
            let capacity = (max_r - min_r + 1) as usize;

            // Initialize border arrays with None values
            if let Some(node) = g.node_mut(v) {
                node.border_left = vec![None; capacity + 1]; // +1 for 0-indexing
                node.border_right = vec![None; capacity + 1];
            }

            // Create border nodes for each rank
            for rank in min_r..=max_r {
                add_border_node(g, "borderLeft", "_bl", v, rank);
                add_border_node(g, "borderRight", "_br", v, rank);
            }
        }
    }

    // Process all root-level nodes
    let roots: Vec<String> = g.root_children().into_iter().cloned().collect();
    for root in roots {
        dfs(g, &root);
    }
}

/// Add a single border node at a specific rank
fn add_border_node(g: &mut DagreGraph, prop: &str, prefix: &str, sg: &str, rank: i32) {
    // Create the border node
    // Border nodes have a small non-zero width to create separation between
    // adjacent subgraphs during x-coordinate assignment. This matches how
    // dagre.js creates visual separation between sibling subgraph boxes.
    const BORDER_NODE_WIDTH: f64 = 10.0;

    let id = g.unique_id(prefix);
    let label = NodeLabel {
        width: BORDER_NODE_WIDTH,
        height: 0.0,
        rank: Some(rank),
        border_type: Some(prop.to_string()),
        dummy: Some("border".to_string()),
        ..Default::default()
    };
    g.set_node(&id, label);
    g.set_parent(&id, sg);

    // Get previous border node at rank-1 (if any)
    let min_rank = g.node(sg).and_then(|n| n.min_rank).unwrap_or(0);
    let idx = (rank - min_rank) as usize;

    // Get the previous border node id if it exists
    let prev_id = if idx > 0 {
        match prop {
            "borderLeft" => g
                .node(sg)
                .and_then(|n| n.border_left.get(idx - 1).cloned().flatten()),
            "borderRight" => g
                .node(sg)
                .and_then(|n| n.border_right.get(idx - 1).cloned().flatten()),
            _ => None,
        }
    } else {
        None
    };

    // Store the current border node id
    if let Some(node) = g.node_mut(sg) {
        match prop {
            "borderLeft" => {
                if idx < node.border_left.len() {
                    node.border_left[idx] = Some(id.clone());
                }
            }
            "borderRight" => {
                if idx < node.border_right.len() {
                    node.border_right[idx] = Some(id.clone());
                }
            }
            _ => {}
        }
    }

    // Connect to previous border node
    if let Some(prev) = prev_id {
        g.set_edge(
            &prev,
            &id,
            EdgeLabel {
                weight: 1,
                ..Default::default()
            },
        );
    }
}

/// Remove border nodes and calculate final compound node dimensions
///
/// This is called after positioning to:
/// 1. Calculate the width and height of each compound node from its border positions
/// 2. Remove all border dummy nodes from the graph
pub fn remove_border_nodes(g: &mut DagreGraph) {
    // First pass: calculate dimensions from border positions
    let compound_nodes: Vec<String> = g
        .nodes()
        .iter()
        .filter(|v| !g.children(v).is_empty())
        .map(|v| (*v).clone())
        .collect();

    for v in &compound_nodes {
        // Get border node references
        let (border_top, border_bottom, border_left, border_right) = {
            let node = g.node(v).cloned().unwrap_or_default();
            (
                node.border_top,
                node.border_bottom,
                node.border_left.clone(),
                node.border_right.clone(),
            )
        };

        // Get positions from border nodes
        let top_y = border_top
            .as_ref()
            .and_then(|id| g.node(id))
            .and_then(|n| n.y);
        let bottom_y = border_bottom
            .as_ref()
            .and_then(|id| g.node(id))
            .and_then(|n| n.y);

        // Get leftmost and rightmost positions from border arrays
        // The LAST element in each array corresponds to the outermost border (like dagre.js)
        let left_x = border_left
            .iter()
            .rev() // Reverse to get last element first
            .filter_map(|opt| opt.as_ref())
            .filter_map(|id| g.node(id))
            .filter_map(|n| n.x)
            .next(); // First non-None from reversed = last element

        let right_x = border_right
            .iter()
            .rev() // Reverse to get last element first
            .filter_map(|opt| opt.as_ref())
            .filter_map(|id| g.node(id))
            .filter_map(|n| n.x)
            .next(); // First non-None from reversed = last element

        // Calculate and set dimensions
        if let (Some(ty), Some(by), Some(lx), Some(rx)) = (top_y, bottom_y, left_x, right_x) {
            let width = (rx - lx).abs();
            let height = (by - ty).abs();
            let x = lx + width / 2.0;
            let y = ty + height / 2.0;

            if let Some(node) = g.node_mut(v) {
                node.width = width;
                node.height = height;
                node.x = Some(x);
                node.y = Some(y);
            }
        }
    }

    // Second pass: remove all border dummy nodes
    let border_nodes: Vec<String> = g
        .nodes()
        .iter()
        .filter(|v| {
            g.node(v)
                .map(|n| n.dummy.as_deref() == Some("border"))
                .unwrap_or(false)
        })
        .map(|v| (*v).clone())
        .collect();

    for v in border_nodes {
        g.remove_node(&v);
    }
}

/// Redirect edges to/from compound nodes to their border nodes
///
/// In state diagrams, edges can go directly TO or FROM composite states.
/// For ranking to work correctly, these edges must be redirected:
/// - Edge TO compound → redirect to compound's border_top
/// - Edge FROM compound → redirect from compound's border_bottom
///
/// This should be called AFTER nesting_graph::run (which creates border nodes)
/// but BEFORE rank assignment.
pub fn redirect_edges_to_border_nodes(g: &mut DagreGraph) {
    // Collect compound nodes (nodes with children and border nodes)
    let compound_nodes: Vec<(String, Option<String>, Option<String>)> = g
        .nodes()
        .iter()
        .filter(|v| !g.children(v).is_empty())
        .map(|v| {
            let node = g.node(v).cloned().unwrap_or_default();
            ((*v).clone(), node.border_top, node.border_bottom)
        })
        .collect();

    if compound_nodes.is_empty() {
        return;
    }

    // Build a map for quick lookup
    let border_top_map: std::collections::HashMap<String, String> = compound_nodes
        .iter()
        .filter_map(|(v, bt, _)| bt.as_ref().map(|b| (v.clone(), b.clone())))
        .collect();

    let border_bottom_map: std::collections::HashMap<String, String> = compound_nodes
        .iter()
        .filter_map(|(v, _, bb)| bb.as_ref().map(|b| (v.clone(), b.clone())))
        .collect();

    // Collect edges that need redirection
    let edges_to_redirect: Vec<(
        super::graph::EdgeKey,
        super::graph::EdgeLabel,
        String,
        String,
    )> = g
        .edges()
        .iter()
        .filter_map(|key| {
            let edge = g.edge_by_key(key)?.clone();

            // Skip nesting edges - they're already correctly configured
            if edge.nesting_edge {
                return None;
            }

            let new_source = border_bottom_map.get(&key.v).cloned();
            let new_target = border_top_map.get(&key.w).cloned();

            // Only redirect if source or target is a compound node
            if new_source.is_some() || new_target.is_some() {
                let final_source = new_source.unwrap_or_else(|| key.v.clone());
                let final_target = new_target.unwrap_or_else(|| key.w.clone());
                Some(((*key).clone(), edge, final_source, final_target))
            } else {
                None
            }
        })
        .collect();

    // Remove old edges and add redirected ones
    for (old_key, mut label, new_source, new_target) in edges_to_redirect {
        g.remove_edge_by_key(&old_key);

        // Store original source/target for edge routing later
        if label.original_source.is_none() {
            label.original_source = Some(old_key.v.clone());
        }
        if label.original_target.is_none() {
            label.original_target = Some(old_key.w.clone());
        }

        g.set_edge(&new_source, &new_target, label);
    }
}

/// Restore edges to their original source/target after layout
///
/// This undoes the redirect_edges_to_border_nodes transformation.
/// Should be called after layout is complete.
pub fn restore_redirected_edges(g: &mut DagreGraph) {
    // Collect edges with original_source or original_target set
    let edges_to_restore: Vec<(super::graph::EdgeKey, super::graph::EdgeLabel)> = g
        .edges()
        .iter()
        .filter_map(|key| {
            let edge = g.edge_by_key(key)?;
            if edge.original_source.is_some() || edge.original_target.is_some() {
                Some(((*key).clone(), edge.clone()))
            } else {
                None
            }
        })
        .collect();

    // Remove redirected edges and add restored ones
    for (old_key, mut label) in edges_to_restore {
        g.remove_edge_by_key(&old_key);

        let orig_source = label.original_source.take().unwrap_or(old_key.v);
        let orig_target = label.original_target.take().unwrap_or(old_key.w);

        g.set_edge(&orig_source, &orig_target, label);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_assign_rank_min_max() {
        let mut g = DagreGraph::new();

        // Create compound node with border nodes
        g.set_node(
            "sg1",
            NodeLabel {
                border_top: Some("bt".to_string()),
                border_bottom: Some("bb".to_string()),
                ..Default::default()
            },
        );
        g.set_node(
            "bt",
            NodeLabel {
                rank: Some(1),
                ..Default::default()
            },
        );
        g.set_node(
            "bb",
            NodeLabel {
                rank: Some(3),
                ..Default::default()
            },
        );
        g.set_node("a", NodeLabel::default());
        g.set_parent("a", "sg1");
        g.set_parent("bt", "sg1");
        g.set_parent("bb", "sg1");

        assign_rank_min_max(&mut g);

        let sg1 = g.node("sg1").unwrap();
        assert_eq!(sg1.min_rank, Some(1));
        assert_eq!(sg1.max_rank, Some(3));
        assert_eq!(g.graph().max_rank, Some(3));
    }

    #[test]
    fn test_add_border_segments() {
        let mut g = DagreGraph::new();

        // Create compound node with minRank and maxRank
        g.set_node(
            "sg1",
            NodeLabel {
                min_rank: Some(0),
                max_rank: Some(2),
                ..Default::default()
            },
        );
        g.set_node("a", NodeLabel::default());
        g.set_parent("a", "sg1");

        add_border_segments(&mut g);

        // Should have created border nodes
        let sg1 = g.node("sg1").unwrap();
        assert!(!sg1.border_left.is_empty());
        assert!(!sg1.border_right.is_empty());

        // Border nodes should exist and have correct ranks
        let border_left_ids: Vec<_> = sg1.border_left.iter().filter_map(|o| o.clone()).collect();
        assert!(!border_left_ids.is_empty());

        for id in &border_left_ids {
            let node = g.node(id).unwrap();
            assert_eq!(node.dummy.as_deref(), Some("border"));
            assert_eq!(node.border_type.as_deref(), Some("borderLeft"));
        }
    }

    #[test]
    fn test_remove_border_nodes() {
        let mut g = DagreGraph::new();

        // Create compound node with border nodes that have positions
        g.set_node(
            "sg1",
            NodeLabel {
                border_top: Some("bt".to_string()),
                border_bottom: Some("bb".to_string()),
                border_left: vec![Some("bl".to_string())],
                border_right: vec![Some("br".to_string())],
                ..Default::default()
            },
        );
        g.set_node(
            "bt",
            NodeLabel {
                x: Some(50.0),
                y: Some(0.0),
                dummy: Some("border".to_string()),
                ..Default::default()
            },
        );
        g.set_node(
            "bb",
            NodeLabel {
                x: Some(50.0),
                y: Some(100.0),
                dummy: Some("border".to_string()),
                ..Default::default()
            },
        );
        g.set_node(
            "bl",
            NodeLabel {
                x: Some(0.0),
                y: Some(50.0),
                dummy: Some("border".to_string()),
                ..Default::default()
            },
        );
        g.set_node(
            "br",
            NodeLabel {
                x: Some(100.0),
                y: Some(50.0),
                dummy: Some("border".to_string()),
                ..Default::default()
            },
        );
        g.set_node("a", NodeLabel::default());
        g.set_parent("a", "sg1");
        g.set_parent("bt", "sg1");
        g.set_parent("bb", "sg1");
        g.set_parent("bl", "sg1");
        g.set_parent("br", "sg1");

        remove_border_nodes(&mut g);

        // Border nodes should be removed
        assert!(!g.has_node("bt"));
        assert!(!g.has_node("bb"));
        assert!(!g.has_node("bl"));
        assert!(!g.has_node("br"));

        // sg1 should have dimensions set
        let sg1 = g.node("sg1").unwrap();
        assert_eq!(sg1.width, 100.0);
        assert_eq!(sg1.height, 100.0);
        assert_eq!(sg1.x, Some(50.0));
        assert_eq!(sg1.y, Some(50.0));
    }
}
