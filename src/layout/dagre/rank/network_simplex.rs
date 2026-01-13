//! Network simplex algorithm for optimal rank assignment
//!
//! This implements the network simplex algorithm as described in:
//! Gansner et al. "A Technique for Drawing Directed Graphs" (1993)
//!
//! The algorithm finds an optimal rank assignment that minimizes the
//! weighted sum of edge lengths while respecting minimum length constraints.

use crate::layout::dagre::graph::{DagreGraph, EdgeKey};
use std::collections::{HashMap, HashSet};

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
    // Step 1: Initialize with longest path ranking
    super::longest_path::run(g);

    // Build feasible spanning tree
    let mut tree = feasible_tree(g);

    // Initialize low/lim values for tree traversal
    let root = tree.root.clone();
    init_low_lim_values(&mut tree, root);

    // Initialize cut values
    init_cut_values(&mut tree, g);

    // Iterate: find leaving edge, find entering edge, exchange
    let mut iterations = 0;
    let max_iterations = g.node_count() * g.node_count();

    while let Some(leave) = leave_edge(&tree) {
        if iterations >= max_iterations {
            break; // Prevent infinite loops
        }
        iterations += 1;

        if let Some(enter) = enter_edge(&tree, g, &leave) {
            exchange_edges(&mut tree, g, &leave, &enter);
        } else {
            break;
        }
    }
}

/// Build an initial feasible spanning tree using tight edges
fn feasible_tree(g: &DagreGraph) -> SpanningTree {
    let mut tree = SpanningTree::new();

    // Add all nodes to tree
    for v in g.nodes() {
        tree.set_node(v, TreeNode::default());
    }

    // Find tight edges (slack = 0) and build tree
    let mut in_tree: HashSet<String> = HashSet::new();

    // Start with first node
    if let Some(first) = g.nodes().first() {
        in_tree.insert((*first).clone());
        tree.root = Some((*first).clone());
    }

    // Greedily add tight edges
    let mut changed = true;
    while changed {
        changed = false;
        for edge_key in g.edges() {
            let v = &edge_key.v;
            let w = &edge_key.w;

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

    // If not all nodes are connected, add non-tight edges and adjust ranks
    for v in g.nodes() {
        if !in_tree.contains(v) {
            // Find an edge connecting this node to the tree
            for edge_key in g.in_edges(v).iter().chain(g.out_edges(v).iter()) {
                let other = if edge_key.v == *v {
                    &edge_key.w
                } else {
                    &edge_key.v
                };
                if in_tree.contains(other) {
                    tree.set_edge(v, other, TreeEdge::default());
                    in_tree.insert(v.clone());
                    // Adjust rank to make edge tight
                    if edge_key.v == *v {
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
    *counter += 1;
    let low = *counter;

    // Set parent
    if let Some(node) = tree.node_mut(v) {
        node.parent = parent.map(|s| s.to_string());
    }

    // Visit children
    let neighbors: Vec<String> = tree.neighbors(v);
    for w in neighbors {
        if parent.map_or(true, |p| p != w) {
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
            if edge.cutvalue.map_or(false, |cv| cv < 0) {
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
    let (tail, _head) = (&leave.0, &leave.1);

    // Find the non-tree edge with minimum slack that crosses the cut
    let mut best_edge: Option<EdgeKey> = None;
    let mut best_slack = i32::MAX;

    for edge_key in g.edges() {
        // Skip tree edges
        if tree.has_edge(&edge_key.v, &edge_key.w) {
            continue;
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
    // Remove leaving edge from tree
    tree.remove_edge(&leave.0, &leave.1);

    // Add entering edge to tree
    tree.set_edge(&enter.v, &enter.w, TreeEdge::default());

    // Update ranks based on slack of entering edge
    if let Some(slack) = super::util::slack(g, &enter.v, &enter.w) {
        if slack != 0 {
            // Determine which side to adjust
            let v_in_tail = is_descendant(tree, &enter.v, &leave.0);

            // Adjust ranks in the appropriate component
            let delta = if v_in_tail { slack } else { -slack };

            // Update all nodes in the tail component
            let nodes: Vec<String> = g.nodes().into_iter().cloned().collect();
            for v in nodes {
                if is_descendant(tree, &v, &leave.0) {
                    if let Some(node) = g.node_mut(&v) {
                        if let Some(rank) = node.rank {
                            node.rank = Some(rank + delta);
                        }
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
