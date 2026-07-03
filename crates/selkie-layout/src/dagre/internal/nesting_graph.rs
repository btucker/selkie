//! Nesting graph for compound graph layout
//!
//! This module implements the nesting graph algorithm from dagre.js, which creates
//! dummy nodes and edges to ensure that compound graph children are positioned
//! between their parent's borders.
//!
//! The algorithm works by:
//! 1. Creating a root node connected to all top-level nodes
//! 2. Creating border top and bottom dummy nodes for each compound node
//! 3. Adding weighted "nesting edges" that constrain children between parent borders
//! 4. After ranking, removing the nesting infrastructure but keeping the rank assignments
//!
//! Reference: Sander, "Layout of Compound Directed Graphs"

use std::collections::HashMap;

use super::graph::{DagreGraph, EdgeLabel, NodeLabel};

/// Run the nesting graph algorithm
///
/// This creates dummy nodes for the tops and bottoms of subgraphs,
/// adds appropriate edges to ensure that all cluster nodes are placed between
/// these boundaries, and ensures that the graph is connected.
pub fn run(g: &mut DagreGraph) {
    // Only run if this is a compound graph
    if !g.is_compound() {
        return;
    }

    // Create root node
    let root = add_dummy_node(g, "root", NodeLabel::default(), "_root");
    g.graph_mut().nesting_root = Some(root.clone());

    // Calculate tree depths
    let depths = tree_depths(g);

    // Calculate height (maximum depth minus 1, matching dagre.js)
    // For flat subgraphs at depth 1, height = 0
    let height = depths
        .values()
        .copied()
        .max()
        .unwrap_or(1)
        .saturating_sub(1);

    // nodeSep = 2 * height + 1, ensures space for border nodes
    let node_sep = if height > 0 { 2 * height + 1 } else { 1 };

    // Multiply all edge minlen by nodeSep to align nodes on non-border ranks
    for key in g.edges().into_iter().cloned().collect::<Vec<_>>() {
        if let Some(edge) = g.edge_by_key_mut(&key) {
            edge.minlen *= node_sep;
        }
    }

    // Calculate total weight for vertical compaction
    let weight = sum_weights(g) + 1;

    // Process all root-level children
    let root_children: Vec<String> = g.root_children().into_iter().cloned().collect();
    for child in root_children {
        dfs(g, &root, node_sep, weight, height, &depths, &child);
    }

    // Save the multiplier for later removal of empty border layers
    g.graph_mut().node_rank_factor = Some(node_sep);
}

/// DFS to create border nodes and nesting edges
fn dfs(
    g: &mut DagreGraph,
    root: &str,
    node_sep: i32,
    weight: i32,
    height: i32,
    depths: &HashMap<String, i32>,
    v: &str,
) {
    // Filter out nodes that were added during nesting (border nodes)
    // to prevent infinite recursion
    let children: Vec<String> = g
        .children(v)
        .into_iter()
        .filter(|c| {
            g.node(c)
                .map(|n| n.dummy.as_deref() != Some("border"))
                .unwrap_or(true)
        })
        .cloned()
        .collect();

    if children.is_empty() {
        // Leaf node - connect to root
        if v != root {
            g.set_edge(
                root,
                v,
                EdgeLabel {
                    weight: 0,
                    minlen: node_sep,
                    nesting_edge: true,
                    ..Default::default()
                },
            );
        }
        return;
    }

    // Compound node - create border top and bottom nodes
    let top = add_border_node(g, "_bt");
    let bottom = add_border_node(g, "_bb");

    // Set parent for border nodes
    g.set_parent(&top, v);
    g.set_parent(&bottom, v);

    // Store border node references on the parent
    if let Some(node) = g.node_mut(v) {
        node.border_top = Some(top.clone());
        node.border_bottom = Some(bottom.clone());
    }

    // Process children
    for child in &children {
        dfs(g, root, node_sep, weight, height, depths, child);

        // Get child's border nodes (or the child itself if it's a leaf)
        let (child_top, child_bottom) = {
            let child_node = g.node(child).cloned().unwrap_or_default();
            let ct = child_node
                .border_top
                .clone()
                .unwrap_or_else(|| child.clone());
            let cb = child_node
                .border_bottom
                .clone()
                .unwrap_or_else(|| child.clone());
            (ct, cb)
        };

        // Determine edge weight and minlen
        let child_has_border = g
            .node(child)
            .map(|n| n.border_top.is_some())
            .unwrap_or(false);
        let this_weight = if child_has_border { weight } else { 2 * weight };

        let v_depth = depths.get(v).copied().unwrap_or(0);
        let minlen = if child_top != child_bottom {
            1
        } else {
            height - v_depth + 1
        };

        // Add nesting edge from parent's top border to child's top
        g.set_edge(
            &top,
            &child_top,
            EdgeLabel {
                weight: this_weight,
                minlen,
                nesting_edge: true,
                ..Default::default()
            },
        );

        // Add nesting edge from child's bottom to parent's bottom border
        g.set_edge(
            &child_bottom,
            &bottom,
            EdgeLabel {
                weight: this_weight,
                minlen,
                nesting_edge: true,
                ..Default::default()
            },
        );
    }

    // If this is a root-level compound node, connect root to its top border
    if g.parent(v).is_none() {
        let v_depth = depths.get(v).copied().unwrap_or(0);
        g.set_edge(
            root,
            &top,
            EdgeLabel {
                weight: 0,
                minlen: height + v_depth,
                nesting_edge: true,
                ..Default::default()
            },
        );
    }
}

/// Calculate depth of each node in the compound tree
fn tree_depths(g: &DagreGraph) -> HashMap<String, i32> {
    let mut depths = HashMap::new();

    fn dfs_depth(g: &DagreGraph, v: &str, depth: i32, depths: &mut HashMap<String, i32>) {
        let children: Vec<String> = g.children(v).into_iter().cloned().collect();
        if !children.is_empty() {
            for child in children {
                dfs_depth(g, &child, depth + 1, depths);
            }
        }
        depths.insert(v.to_string(), depth);
    }

    // Process all root-level nodes
    let roots: Vec<String> = g.root_children().into_iter().cloned().collect();
    for root in roots {
        dfs_depth(g, &root, 1, &mut depths);
    }

    depths
}

/// Sum all edge weights
fn sum_weights(g: &DagreGraph) -> i32 {
    g.edges()
        .iter()
        .filter_map(|key| g.edge_by_key(key))
        .map(|e| e.weight)
        .sum()
}

/// Clean up the nesting graph after ranking
///
/// This removes the nesting root node and all nesting edges,
/// leaving just the original graph with ranks assigned.
pub fn cleanup(g: &mut DagreGraph) {
    // Remove the nesting root node
    if let Some(root) = g.graph().nesting_root.clone() {
        g.remove_node(&root);
        g.graph_mut().nesting_root = None;
    }

    // Remove all nesting edges
    let nesting_edges: Vec<_> = g
        .edges()
        .into_iter()
        .filter(|&key| g.edge_by_key(key).map(|e| e.nesting_edge).unwrap_or(false))
        .cloned()
        .collect();

    for key in nesting_edges {
        g.remove_edge_by_key(&key);
    }
}

/// Add a dummy node with a generated unique id
fn add_dummy_node(g: &mut DagreGraph, dummy_type: &str, label: NodeLabel, prefix: &str) -> String {
    let id = g.unique_id(prefix);
    let mut node_label = label;
    node_label.dummy = Some(dummy_type.to_string());
    g.set_node(&id, node_label);
    id
}

/// Add a border dummy node
fn add_border_node(g: &mut DagreGraph, prefix: &str) -> String {
    add_dummy_node(
        g,
        "border",
        NodeLabel {
            width: 0.0,
            height: 0.0,
            ..Default::default()
        },
        prefix,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_compound_graph() {
        let mut g = DagreGraph::new();

        // Create a simple compound graph: sg1 contains a and b
        g.set_node("a", NodeLabel::default());
        g.set_node("b", NodeLabel::default());
        g.set_node("sg1", NodeLabel::default());
        g.set_parent("a", "sg1");
        g.set_parent("b", "sg1");
        g.set_edge("a", "b", EdgeLabel::default());

        run(&mut g);

        // Should have created nesting root
        assert!(g.graph().nesting_root.is_some());

        // sg1 should have border top and bottom nodes
        let sg1 = g.node("sg1").unwrap();
        assert!(sg1.border_top.is_some());
        assert!(sg1.border_bottom.is_some());

        // There should be nesting edges
        let nesting_edge_count = g
            .edges()
            .iter()
            .filter(|key| g.edge_by_key(key).map(|e| e.nesting_edge).unwrap_or(false))
            .count();
        assert!(nesting_edge_count > 0);
    }

    #[test]
    fn test_cleanup_removes_nesting_structure() {
        let mut g = DagreGraph::new();

        g.set_node("a", NodeLabel::default());
        g.set_node("sg1", NodeLabel::default());
        g.set_parent("a", "sg1");

        run(&mut g);
        assert!(g.graph().nesting_root.is_some());

        cleanup(&mut g);

        // Nesting root should be removed
        assert!(g.graph().nesting_root.is_none());

        // No nesting edges should remain
        let nesting_edge_count = g
            .edges()
            .iter()
            .filter(|key| g.edge_by_key(key).map(|e| e.nesting_edge).unwrap_or(false))
            .count();
        assert_eq!(nesting_edge_count, 0);
    }

    #[test]
    fn test_non_compound_graph_unchanged() {
        let mut g = DagreGraph::new();
        g.set_node("a", NodeLabel::default());
        g.set_node("b", NodeLabel::default());
        g.set_edge("a", "b", EdgeLabel::default());

        let edge_count_before = g.edge_count();
        let node_count_before = g.node_count();

        run(&mut g);

        // Non-compound graph should be unchanged
        assert_eq!(g.edge_count(), edge_count_before);
        assert_eq!(g.node_count(), node_count_before);
        assert!(g.graph().nesting_root.is_none());
    }

    #[test]
    fn test_nested_compound_graph() {
        let mut g = DagreGraph::new();

        // Create nested compound graph: sg1 contains sg2, sg2 contains a
        g.set_node("a", NodeLabel::default());
        g.set_node("sg2", NodeLabel::default());
        g.set_node("sg1", NodeLabel::default());
        g.set_parent("a", "sg2");
        g.set_parent("sg2", "sg1");

        run(&mut g);

        // Both sg1 and sg2 should have border nodes
        let sg1 = g.node("sg1").unwrap();
        let sg2 = g.node("sg2").unwrap();
        assert!(sg1.border_top.is_some());
        assert!(sg1.border_bottom.is_some());
        assert!(sg2.border_top.is_some());
        assert!(sg2.border_bottom.is_some());
    }
}
