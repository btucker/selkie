//! Network simplex algorithm for optimal rank assignment
//!
//! This implements the network simplex algorithm as described in:
//! Gansner et al. "A Technique for Drawing Directed Graphs" (1993)
//!
//! The algorithm finds an optimal rank assignment that minimizes the
//! weighted sum of edge lengths while respecting minimum length constraints.

use crate::layout::dagre::graph::{DagreGraph, EdgeKey};
use std::collections::{HashMap, HashSet};

/// Type alias for tracking edge exchanges during network simplex iteration
type EdgeExchange = ((String, String), (String, String));

/// A spanning tree used for network simplex
#[derive(Debug, Clone)]
pub struct SpanningTree {
    /// Edges in the tree (undirected)
    edges: HashMap<(String, String), TreeEdge>,
    /// Node data
    nodes: HashMap<String, TreeNode>,
    /// Root of the tree
    root: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct TreeEdge {
    pub cutvalue: Option<i32>,
}

#[derive(Debug, Clone, Default)]
pub struct TreeNode {
    /// Low value for DFS numbering
    pub low: Option<i32>,
    /// Limit value for DFS numbering
    pub lim: Option<i32>,
    /// Parent in the tree
    pub parent: Option<String>,
}

impl Default for SpanningTree {
    fn default() -> Self {
        Self::new()
    }
}

impl SpanningTree {
    pub fn new() -> Self {
        Self {
            edges: HashMap::new(),
            nodes: HashMap::new(),
            root: None,
        }
    }

    pub fn set_edge(&mut self, v: &str, w: &str, edge: TreeEdge) {
        self.edges
            .insert((v.to_string(), w.to_string()), edge.clone());
        self.edges.insert((w.to_string(), v.to_string()), edge);
    }

    pub fn remove_edge(&mut self, v: &str, w: &str) {
        self.edges.remove(&(v.to_string(), w.to_string()));
        self.edges.remove(&(w.to_string(), v.to_string()));
    }

    pub fn has_edge(&self, v: &str, w: &str) -> bool {
        self.edges.contains_key(&(v.to_string(), w.to_string()))
    }

    pub fn edge(&self, v: &str, w: &str) -> Option<&TreeEdge> {
        self.edges.get(&(v.to_string(), w.to_string()))
    }

    pub fn edge_mut(&mut self, v: &str, w: &str) -> Option<&mut TreeEdge> {
        self.edges.get_mut(&(v.to_string(), w.to_string()))
    }

    pub fn set_node(&mut self, v: &str, node: TreeNode) {
        self.nodes.insert(v.to_string(), node);
    }

    pub fn node(&self, v: &str) -> Option<&TreeNode> {
        self.nodes.get(v)
    }

    pub fn node_mut(&mut self, v: &str) -> Option<&mut TreeNode> {
        self.nodes.get_mut(v)
    }

    pub fn nodes(&self) -> Vec<&String> {
        let mut nodes: Vec<&String> = self.nodes.keys().collect();
        nodes.sort();
        nodes
    }

    pub fn edges_list(&self) -> Vec<(&str, &str)> {
        let mut seen = HashSet::new();
        let mut result = Vec::new();
        // Sort edge keys for deterministic iteration
        let mut edge_keys: Vec<&(String, String)> = self.edges.keys().collect();
        edge_keys.sort();
        for (v, w) in edge_keys {
            if !seen.contains(&(w.as_str(), v.as_str())) {
                result.push((v.as_str(), w.as_str()));
                seen.insert((v.as_str(), w.as_str()));
            }
        }
        result
    }

    /// Get neighbors of a node in the tree
    pub fn neighbors(&self, v: &str) -> Vec<String> {
        let mut neighbors: Vec<String> = self
            .edges
            .keys()
            .filter(|(a, _)| a == v)
            .map(|(_, b)| b.clone())
            .collect();
        neighbors.sort();
        neighbors
    }
}

/// Run the network simplex algorithm
pub fn run(g: &mut DagreGraph) {
    // Collect compound nodes (nodes with children) - these are excluded from ranking
    // because their position is determined by their children, not by edges
    let compound_nodes: HashSet<String> = g
        .nodes()
        .iter()
        .filter(|v| !g.children(v).is_empty())
        .map(|v| (*v).clone())
        .collect();

    // Step 1: Initialize with longest path ranking
    super::longest_path::run(g);

    // Build feasible spanning tree (excluding compound nodes)
    let mut tree = feasible_tree(g, &compound_nodes);

    // Initialize low/lim values for tree traversal
    let root = tree.root.clone();
    init_low_lim_values(&mut tree, root);

    // Initialize cut values
    init_cut_values(&mut tree, g);

    // Iterate: find leaving edge, find entering edge, exchange
    let mut iterations = 0;
    let max_iterations = g.node_count() * g.edge_count() + 1; // dagre.js uses this formula

    // Track exchanges for cycle detection - count how many times each exchange occurs
    let mut exchange_counts: HashMap<EdgeExchange, usize> = HashMap::new();

    while let Some(leave) = leave_edge(&tree) {
        if iterations >= max_iterations {
            break; // Prevent infinite loops
        }
        iterations += 1;

        if let Some(enter) = enter_edge(&tree, g, &leave) {
            // Create normalized exchange record for cycle detection
            let exchange = (
                if leave.0 < leave.1 {
                    leave.clone()
                } else {
                    (leave.1.clone(), leave.0.clone())
                },
                if enter.v < enter.w {
                    (enter.v.clone(), enter.w.clone())
                } else {
                    (enter.w.clone(), enter.v.clone())
                },
            );

            // Check for repeated exchanges (cycle detection)
            let count = exchange_counts.entry(exchange.clone()).or_insert(0);
            *count += 1;
            if *count > 2 {
                // Seen this exchange more than twice = definitely cycling
                break;
            }

            exchange_edges(&mut tree, g, &leave, &enter);
        } else {
            break;
        }
    }
}

/// Build an initial feasible spanning tree using tight edges
/// compound_nodes are excluded from the tree
fn feasible_tree(g: &mut DagreGraph, compound_nodes: &HashSet<String>) -> SpanningTree {
    let mut tree = SpanningTree::new();

    // Add all non-compound nodes to tree
    for v in g.nodes() {
        if !compound_nodes.contains(v.as_str()) {
            tree.set_node(v, TreeNode::default());
        }
    }

    let target_count = tree.nodes.len();
    if target_count == 0 {
        return tree;
    }

    let mut in_tree: HashSet<String> = HashSet::new();

    if let Some(first) = g.nodes().iter().find(|v| !compound_nodes.contains(**v)) {
        in_tree.insert((*first).clone());
        tree.root = Some((*first).clone());
    }

    while in_tree.len() < target_count {
        expand_tight_tree(&mut tree, g, compound_nodes, &mut in_tree);

        if in_tree.len() == target_count {
            break;
        }

        let Some(edge_key) = find_min_slack_edge(g, compound_nodes, &in_tree) else {
            break;
        };

        let slack = super::util::slack(g, &edge_key.v, &edge_key.w).unwrap_or(0);
        let delta = if in_tree.contains(&edge_key.v) {
            slack
        } else {
            -slack
        };

        if delta != 0 {
            for v in &in_tree {
                if let Some(node) = g.node_mut(v) {
                    if let Some(rank) = node.rank {
                        node.rank = Some(rank + delta);
                    }
                }
            }
        }
    }

    tree
}

fn expand_tight_tree(
    tree: &mut SpanningTree,
    g: &DagreGraph,
    compound_nodes: &HashSet<String>,
    in_tree: &mut HashSet<String>,
) {
    let mut stack: Vec<String> = in_tree.iter().cloned().collect();
    while let Some(v) = stack.pop() {
        for edge_key in g.in_edges(&v).iter().chain(g.out_edges(&v).iter()) {
            let w = if edge_key.v == v {
                &edge_key.w
            } else {
                &edge_key.v
            };

            if compound_nodes.contains(w) {
                continue;
            }

            if in_tree.contains(w) {
                continue;
            }

            let slack = super::util::slack(g, &edge_key.v, &edge_key.w).unwrap_or(i32::MAX);
            if slack == 0 {
                tree.set_edge(&edge_key.v, &edge_key.w, TreeEdge::default());
                in_tree.insert(w.clone());
                stack.push(w.clone());
            }
        }
    }
}

fn find_min_slack_edge(
    g: &DagreGraph,
    compound_nodes: &HashSet<String>,
    in_tree: &HashSet<String>,
) -> Option<EdgeKey> {
    let mut best_edge: Option<EdgeKey> = None;
    let mut best_slack = i32::MAX;

    for edge_key in g.edges() {
        if compound_nodes.contains(&edge_key.v) || compound_nodes.contains(&edge_key.w) {
            continue;
        }

        let v_in = in_tree.contains(&edge_key.v);
        let w_in = in_tree.contains(&edge_key.w);
        if v_in == w_in {
            continue;
        }

        if let Some(slack) = super::util::slack(g, &edge_key.v, &edge_key.w) {
            if slack < best_slack {
                best_slack = slack;
                best_edge = Some(edge_key.clone());
            }
        }
    }

    best_edge
}

/// Initialize low and lim values for tree traversal
pub fn init_low_lim_values(tree: &mut SpanningTree, root: Option<String>) {
    let root = root.or_else(|| tree.nodes().first().cloned().cloned());

    if let Some(root) = root {
        let mut counter = 0;
        dfs_assign(tree, &root, None, &mut counter);
    }
}

fn dfs_assign(tree: &mut SpanningTree, v: &str, parent: Option<&str>, counter: &mut i32) {
    // Check recursion depth to detect infinite recursion
    if *counter > 1000 {
        eprintln!(
            "[dfs_assign] OVERFLOW: counter={}, v={}, parent={:?}",
            counter, v, parent
        );
        panic!("dfs_assign recursion overflow");
    }

    *counter += 1;
    let low = *counter;

    // Set parent
    if let Some(node) = tree.node_mut(v) {
        node.parent = parent.map(|s| s.to_string());
    }

    // Visit children
    let neighbors: Vec<String> = tree.neighbors(v);
    for w in neighbors {
        if parent.is_none_or(|p| p != w) {
            dfs_assign(tree, &w, Some(v), counter);
        }
    }

    *counter += 1;
    let lim = *counter;

    // Update low and lim
    if let Some(node) = tree.node_mut(v) {
        node.low = Some(low);
        node.lim = Some(lim);
    }
}

/// Initialize cut values for all tree edges
pub fn init_cut_values(tree: &mut SpanningTree, g: &DagreGraph) {
    let root = tree
        .root
        .clone()
        .or_else(|| tree.nodes().first().cloned().cloned());

    let Some(root) = root else {
        return;
    };

    let mut order = Vec::new();
    dfs_postorder(tree, &root, None, &mut order);

    for v in order.into_iter().filter(|v| v != &root) {
        let Some(parent) = tree.node(&v).and_then(|n| n.parent.as_ref()).cloned() else {
            continue;
        };
        let cutvalue = calc_cut_value(tree, g, &v, &parent);
        if let Some(edge) = tree.edge_mut(&v, &parent) {
            edge.cutvalue = Some(cutvalue);
        }
        if let Some(edge) = tree.edge_mut(&parent, &v) {
            edge.cutvalue = Some(cutvalue);
        }
    }
}

/// Calculate the cut value for a tree edge
pub fn calc_cut_value(tree: &SpanningTree, g: &DagreGraph, v: &str, w: &str) -> i32 {
    let child = if tree
        .node(v)
        .and_then(|n| n.parent.as_ref())
        .is_some_and(|p| p == w)
    {
        v
    } else {
        w
    };

    let parent = if child == v { w } else { v };

    let mut child_is_tail = true;
    let mut graph_edge = g.edge(child, parent);
    if graph_edge.is_none() {
        child_is_tail = false;
        graph_edge = g.edge(parent, child);
    }

    let mut cutvalue = graph_edge.map(|e| e.weight).unwrap_or(0);

    let mut incident_edges: Vec<&EdgeKey> = g
        .in_edges(child)
        .iter()
        .chain(g.out_edges(child).iter())
        .copied()
        .collect();
    incident_edges.sort();
    incident_edges.dedup();

    for edge_key in incident_edges {
        let is_out_edge = edge_key.v == child;
        let other = if is_out_edge {
            &edge_key.w
        } else {
            &edge_key.v
        };
        if other == parent {
            continue;
        }

        let points_to_head = is_out_edge == child_is_tail;
        let other_weight = g.edge_by_key(edge_key).map(|e| e.weight).unwrap_or(1);
        cutvalue += if points_to_head {
            other_weight
        } else {
            -other_weight
        };

        if tree.has_edge(child, other) {
            let other_cut = tree
                .edge(child, other)
                .and_then(|e| e.cutvalue)
                .unwrap_or(0);
            cutvalue += if points_to_head {
                -other_cut
            } else {
                other_cut
            };
        }
    }

    cutvalue
}

/// Check if node v is a descendant of node u in the tree
fn is_descendant(tree: &SpanningTree, v: &str, u: &str) -> bool {
    let v_node = match tree.node(v) {
        Some(n) => n,
        None => return false,
    };
    let u_node = match tree.node(u) {
        Some(n) => n,
        None => return false,
    };

    let (v_low, v_lim) = match (v_node.low, v_node.lim) {
        (Some(l), Some(m)) => (l, m),
        _ => return false,
    };
    let (u_low, u_lim) = match (u_node.low, u_node.lim) {
        (Some(l), Some(m)) => (l, m),
        _ => return false,
    };

    u_low <= v_low && v_lim <= u_lim
}

/// Find an edge to leave the tree (with negative cut value)
pub fn leave_edge(tree: &SpanningTree) -> Option<(String, String)> {
    for (v, w) in tree.edges_list() {
        if let Some(edge) = tree.edge(v, w) {
            if edge.cutvalue.is_some_and(|cv| cv < 0) {
                return Some((v.to_string(), w.to_string()));
            }
        }
    }
    None
}

/// Find an edge to enter the tree
pub fn enter_edge(
    tree: &SpanningTree,
    g: &DagreGraph,
    leave: &(String, String),
) -> Option<EdgeKey> {
    enter_edge_with_exclusion(tree, g, leave, None)
}

/// Find an edge to enter the tree, optionally excluding a specific edge
pub fn enter_edge_with_exclusion(
    tree: &SpanningTree,
    g: &DagreGraph,
    leave: &(String, String),
    exclude: Option<&(String, String)>,
) -> Option<EdgeKey> {
    let mut v = leave.0.clone();
    let mut w = leave.1.clone();

    if g.edge(&v, &w).is_none() {
        std::mem::swap(&mut v, &mut w);
    }

    let v_label = tree.node(&v)?;
    let w_label = tree.node(&w)?;
    let mut tail = &v;
    let mut flip = false;

    if v_label.lim.unwrap_or(0) > w_label.lim.unwrap_or(0) {
        tail = &w;
        flip = true;
    }

    // Find the non-tree edge with minimum slack that crosses the cut
    let mut best_edge: Option<EdgeKey> = None;
    let mut best_slack = i32::MAX;

    for edge_key in g.edges() {
        // Skip tree edges
        if tree.has_edge(&edge_key.v, &edge_key.w) {
            continue;
        }

        // Skip excluded edge (anti-cycling)
        if let Some(ex) = exclude {
            if (edge_key.v == ex.0 && edge_key.w == ex.1)
                || (edge_key.v == ex.1 && edge_key.w == ex.0)
            {
                continue;
            }
        }

        let v_in_tail = is_descendant(tree, &edge_key.v, tail);
        let w_in_tail = is_descendant(tree, &edge_key.w, tail);

        // Edge must cross the cut
        if flip == v_in_tail && flip != w_in_tail {
            if let Some(slack) = super::util::slack(g, &edge_key.v, &edge_key.w) {
                if slack < best_slack {
                    best_slack = slack;
                    best_edge = Some(edge_key.clone());
                }
            }
        }
    }

    best_edge
}

/// Exchange edges in the tree and update the graph
pub fn exchange_edges(
    tree: &mut SpanningTree,
    g: &mut DagreGraph,
    leave: &(String, String),
    enter: &EdgeKey,
) {
    // Remove leaving edge from tree
    tree.remove_edge(&leave.0, &leave.1);

    // Add entering edge to tree
    tree.set_edge(&enter.v, &enter.w, TreeEdge::default());

    // Recompute low/lim values
    init_low_lim_values(tree, tree.root.clone());

    // Recompute cut values
    init_cut_values(tree, g);

    // Update ranks based on the new tree structure
    update_ranks(tree, g);
}

fn update_ranks(tree: &SpanningTree, g: &mut DagreGraph) {
    let root = tree
        .nodes()
        .into_iter()
        .find(|v| g.parent(v.as_str()).is_none())
        .cloned()
        .or_else(|| tree.root.clone())
        .or_else(|| tree.nodes().first().cloned().cloned());

    let Some(root) = root else {
        return;
    };

    if let Some(node) = g.node_mut(&root) {
        if node.rank.is_none() {
            node.rank = Some(0);
        }
    }

    let mut ordered = Vec::new();
    dfs_preorder(tree, &root, None, &mut ordered);

    for v in ordered.into_iter().skip(1) {
        let Some(parent) = tree.node(&v).and_then(|n| n.parent.as_ref()).cloned() else {
            continue;
        };

        let (edge, flipped) = if let Some(edge) = g.edge(&v, &parent) {
            (edge, false)
        } else if let Some(edge) = g.edge(&parent, &v) {
            (edge, true)
        } else {
            continue;
        };

        let parent_rank = g.node(&parent).and_then(|n| n.rank).unwrap_or(0);
        let delta = if flipped { edge.minlen } else { -edge.minlen };

        if let Some(node) = g.node_mut(&v) {
            node.rank = Some(parent_rank + delta);
        }
    }
}

fn dfs_preorder(tree: &SpanningTree, v: &str, parent: Option<&str>, out: &mut Vec<String>) {
    out.push(v.to_string());
    for w in tree.neighbors(v) {
        if parent.is_some_and(|p| p == w.as_str()) {
            continue;
        }
        dfs_preorder(tree, &w, Some(v), out);
    }
}

fn dfs_postorder(tree: &SpanningTree, v: &str, parent: Option<&str>, out: &mut Vec<String>) {
    for w in tree.neighbors(v) {
        if parent.is_some_and(|p| p == w.as_str()) {
            continue;
        }
        dfs_postorder(tree, &w, Some(v), out);
    }
    out.push(v.to_string());
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
    fn test_two_connected_nodes() {
        let mut g = DagreGraph::new();
        g.set_edge("a", "b", EdgeLabel::default());

        run(&mut g);

        assert_eq!(g.node("a").unwrap().rank, Some(0));
        assert_eq!(g.node("b").unwrap().rank, Some(1));
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
    fn test_leave_edge_returns_none_for_positive_cutvalues() {
        let mut tree = SpanningTree::new();
        tree.set_node("a", TreeNode::default());
        tree.set_node("b", TreeNode::default());
        tree.set_node("c", TreeNode::default());
        tree.set_edge("a", "b", TreeEdge { cutvalue: Some(1) });
        tree.set_edge("b", "c", TreeEdge { cutvalue: Some(1) });

        assert!(leave_edge(&tree).is_none());
    }

    #[test]
    fn test_leave_edge_returns_negative_cutvalue_edge() {
        let mut tree = SpanningTree::new();
        tree.set_node("a", TreeNode::default());
        tree.set_node("b", TreeNode::default());
        tree.set_node("c", TreeNode::default());
        tree.set_edge("a", "b", TreeEdge { cutvalue: Some(1) });
        tree.set_edge("b", "c", TreeEdge { cutvalue: Some(-1) });

        let result = leave_edge(&tree);
        assert!(result.is_some());
        let edge = result.unwrap();
        assert!((edge.0 == "b" && edge.1 == "c") || (edge.0 == "c" && edge.1 == "b"));
    }
}
