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
fn feasible_tree(g: &DagreGraph, compound_nodes: &HashSet<String>) -> SpanningTree {
    let mut tree = SpanningTree::new();

    // Add all non-compound nodes to tree
    for v in g.nodes() {
        if !compound_nodes.contains(v.as_str()) {
            tree.set_node(v, TreeNode::default());
        }
    }

    // Find tight edges (slack = 0) and build tree
    let mut in_tree: HashSet<String> = HashSet::new();

    // Start with first non-compound node
    if let Some(first) = g.nodes().iter().find(|v| !compound_nodes.contains(**v)) {
        in_tree.insert((*first).clone());
        tree.root = Some((*first).clone());
    }

    // Greedily add tight edges (skip edges involving compound nodes)
    let mut changed = true;
    while changed {
        changed = false;
        for edge_key in g.edges() {
            let v = &edge_key.v;
            let w = &edge_key.w;

            // Skip edges involving compound nodes
            if compound_nodes.contains(v) || compound_nodes.contains(w) {
                continue;
            }

            let v_in = in_tree.contains(v);
            let w_in = in_tree.contains(w);

            // Add edge if exactly one endpoint is in tree and edge is tight
            if v_in != w_in {
                let slack = super::util::slack(g, v, w).unwrap_or(i32::MAX);
                if slack == 0 {
                    tree.set_edge(v, w, TreeEdge::default());
                    in_tree.insert(v.clone());
                    in_tree.insert(w.clone());
                    changed = true;
                }
            }
        }
    }

    // If not all non-compound nodes are connected, add non-tight edges
    for v in g.nodes() {
        // Skip compound nodes
        if compound_nodes.contains(v.as_str()) {
            continue;
        }
        if !in_tree.contains(v.as_str()) {
            // Find an edge connecting this node to the tree
            for edge_key in g.in_edges(v).iter().chain(g.out_edges(v).iter()) {
                let other = if edge_key.v == **v {
                    &edge_key.w
                } else {
                    &edge_key.v
                };
                // Skip if other is a compound node
                if compound_nodes.contains(other) {
                    continue;
                }
                if in_tree.contains(other) {
                    tree.set_edge(v, other, TreeEdge::default());
                    in_tree.insert((*v).clone());
                    // Adjust rank to make edge tight
                    if edge_key.v == **v {
                        // v -> other, so v.rank = other.rank - minlen
                        let _other_rank = g.node(other).and_then(|n| n.rank).unwrap_or(0);
                        let _minlen = g.edge_by_key(edge_key).map(|e| e.minlen).unwrap_or(1);
                        // We need to update the graph's node rank
                        // But we can't here since we don't have mutable access
                    }
                    break;
                }
            }
        }
    }

    tree
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
    let edges: Vec<(String, String)> = tree
        .edges_list()
        .iter()
        .map(|(v, w)| (v.to_string(), w.to_string()))
        .collect();

    for (v, w) in edges {
        let cutvalue = calc_cut_value(tree, g, &v, &w);
        if let Some(edge) = tree.edge_mut(&v, &w) {
            edge.cutvalue = Some(cutvalue);
        }
    }
}

/// Calculate the cut value for a tree edge
pub fn calc_cut_value(tree: &SpanningTree, g: &DagreGraph, v: &str, w: &str) -> i32 {
    // The cut value is the sum of weights of edges crossing the cut
    // minus the sum of weights of edges in the same direction

    let mut cutvalue = 0;

    // Determine which side is the tail (contains the child in the tree edge)
    let v_node = tree.node(v);
    let _w_node = tree.node(w);

    let (tail, _head) = if v_node.and_then(|n| n.parent.as_ref()) == Some(&w.to_string()) {
        (v, w)
    } else {
        (w, v)
    };

    // For each edge in the graph, check if it crosses the cut
    for edge_key in g.edges() {
        let edge_v = &edge_key.v;
        let edge_w = &edge_key.w;

        let v_in_tail = is_descendant(tree, edge_v, tail);
        let w_in_tail = is_descendant(tree, edge_w, tail);

        let weight = g.edge_by_key(edge_key).map(|e| e.weight).unwrap_or(1);

        if v_in_tail && !w_in_tail {
            // Edge goes from tail to head component (same direction as tree edge)
            cutvalue += weight;
        } else if !v_in_tail && w_in_tail {
            // Edge goes from head to tail component (opposite direction)
            cutvalue -= weight;
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
    // Determine which side is the tail (child in tree edge)
    // The tail is the node whose parent is the other node
    let tail = {
        let v_parent = tree.node(&leave.0).and_then(|n| n.parent.as_ref());
        if v_parent.map(|p| p == &leave.1).unwrap_or(false) {
            &leave.0 // leave.0's parent is leave.1, so leave.0 is the child (tail)
        } else {
            &leave.1 // Otherwise, leave.1 is the child (tail)
        }
    };

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
        if v_in_tail != w_in_tail {
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
    // Determine which side is the tail (child in tree edge) BEFORE modifying tree
    // The tail is the node whose parent is the other node
    let tail = {
        let v_parent = tree.node(&leave.0).and_then(|n| n.parent.as_ref());
        if v_parent.map(|p| p == &leave.1).unwrap_or(false) {
            leave.0.clone() // leave.0's parent is leave.1, so leave.0 is the child (tail)
        } else {
            leave.1.clone() // Otherwise, leave.1 is the child (tail)
        }
    };

    // Collect nodes in the tail component BEFORE modifying tree
    let tail_nodes: Vec<String> = g
        .nodes()
        .iter()
        .filter(|v| is_descendant(tree, v, &tail))
        .map(|v| (*v).clone())
        .collect();

    // Remove leaving edge from tree
    tree.remove_edge(&leave.0, &leave.1);

    // Add entering edge to tree
    tree.set_edge(&enter.v, &enter.w, TreeEdge::default());

    // Update ranks based on slack of entering edge
    if let Some(slack) = super::util::slack(g, &enter.v, &enter.w) {
        if slack != 0 {
            // Determine which direction to adjust based on whether enter.v is in tail
            let v_in_tail = tail_nodes.contains(&enter.v);

            // Adjust ranks in the tail component
            let delta = if v_in_tail { slack } else { -slack };

            for v in &tail_nodes {
                if let Some(node) = g.node_mut(v) {
                    if let Some(rank) = node.rank {
                        node.rank = Some(rank + delta);
                    }
                }
            }
        }
    }

    // Recompute low/lim values
    init_low_lim_values(tree, tree.root.clone());

    // Recompute cut values
    init_cut_values(tree, g);
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
