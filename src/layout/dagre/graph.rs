//! Graph data structure for dagre layout
//!
//! This provides a graph implementation similar to graphlib.js, supporting:
//! - Multigraph (multiple edges between nodes)
//! - Compound graphs (nodes with parent/child relationships)
//! - Node and edge labels with attributes

use std::collections::{HashMap, HashSet};

use crate::layout::NodeShape;

/// A multigraph with compound node support for dagre layout
#[derive(Debug, Clone)]
pub struct DagreGraph {
    /// Graph-level attributes
    graph_label: GraphLabel,
    /// Node labels indexed by node id
    pub(crate) nodes: HashMap<String, NodeLabel>,
    /// Edges indexed by (v, w, name)
    pub(crate) edges: HashMap<EdgeKey, EdgeLabel>,
    /// Outgoing edges from each node
    pub(crate) out_edges: HashMap<String, Vec<EdgeKey>>,
    /// Incoming edges to each node
    pub(crate) in_edges: HashMap<String, Vec<EdgeKey>>,
    /// Parent relationships for compound graphs
    parent: HashMap<String, String>,
    /// Children for each parent node
    children: HashMap<String, HashSet<String>>,
    /// Counter for generating unique edge names
    edge_counter: usize,
}

/// Key for identifying edges (supports multigraph)
#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct EdgeKey {
    pub v: String,
    pub w: String,
    pub name: Option<String>,
}

impl EdgeKey {
    pub fn new(v: impl Into<String>, w: impl Into<String>) -> Self {
        Self {
            v: v.into(),
            w: w.into(),
            name: None,
        }
    }

    pub fn with_name(v: impl Into<String>, w: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            v: v.into(),
            w: w.into(),
            name: Some(name.into()),
        }
    }
}

/// Graph-level label/attributes
#[derive(Debug, Clone, Default)]
pub struct GraphLabel {
    pub nodesep: f64,
    pub edgesep: f64,
    pub ranksep: f64,
    pub rankdir: String,
    pub marginx: f64,
    pub marginy: f64,
    pub acyclicer: String,
    pub ranker: String,
    /// Computed graph width
    pub width: Option<f64>,
    /// Computed graph height
    pub height: Option<f64>,
    /// Tracks first dummy node in each edge chain (for normalize undo)
    pub dummy_chains: Vec<String>,
    /// Root node for nesting graph (compound graph support)
    pub nesting_root: Option<String>,
    /// Multiplier for node ranks to create space for border nodes
    pub node_rank_factor: Option<i32>,
    /// Maximum rank in the graph
    pub max_rank: Option<i32>,
}

/// Stored self-edge information on a node
#[derive(Debug, Clone)]
pub struct SelfEdgeInfo {
    /// The edge key (v, w, name)
    pub edge_key: EdgeKey,
    /// The edge label (dimensions, etc.)
    pub label: EdgeLabel,
}

/// Label/attributes for a node
#[derive(Debug, Clone, Default)]
pub struct NodeLabel {
    pub width: f64,
    pub height: f64,
    /// Node shape for intersection calculations
    pub shape: NodeShape,
    /// Computed x coordinate (center)
    pub x: Option<f64>,
    /// Computed y coordinate (center)
    pub y: Option<f64>,
    /// Assigned rank (layer)
    pub rank: Option<i32>,
    /// Order within rank
    pub order: Option<usize>,
    /// For dummy nodes during normalization
    pub dummy: Option<String>,
    /// Edge label position (for edge label nodes)
    pub labelpos: Option<String>,
    /// For network simplex
    pub low: Option<i32>,
    pub lim: Option<i32>,
    pub parent: Option<String>,
    /// Original edge label (for dummy nodes in normalize)
    pub edge_label: Option<Box<EdgeLabel>>,
    /// Original edge object (v, w, name) for dummy nodes
    pub edge_obj: Option<(String, String, Option<String>)>,
    /// Border top node id (for compound nodes)
    pub border_top: Option<String>,
    /// Border bottom node id (for compound nodes)
    pub border_bottom: Option<String>,
    /// Border left nodes by rank index (for compound nodes)
    pub border_left: Vec<Option<String>>,
    /// Border right nodes by rank index (for compound nodes)
    pub border_right: Vec<Option<String>>,
    /// Minimum rank of this compound node's contents
    pub min_rank: Option<i32>,
    /// Maximum rank of this compound node's contents
    pub max_rank: Option<i32>,
    /// Type of border node ("borderLeft" or "borderRight")
    pub border_type: Option<String>,
    /// Self-edges stored during layout (removed before ranking, restored after positioning)
    pub self_edges: Vec<SelfEdgeInfo>,
    /// For self-edge dummy nodes: stores the original edge info
    pub self_edge_info: Option<SelfEdgeInfo>,
}

/// Label/attributes for an edge
#[derive(Debug, Clone)]
pub struct EdgeLabel {
    /// Minimum length (number of ranks)
    pub minlen: i32,
    /// Edge weight (for ranking optimization)
    pub weight: i32,
    /// Label width
    pub width: f64,
    /// Label height
    pub height: f64,
    /// Computed x coordinate for label
    pub x: Option<f64>,
    /// Computed y coordinate for label
    pub y: Option<f64>,
    /// Control points for edge routing
    pub points: Vec<Point>,
    /// Label position: "l", "c", "r"
    pub labelpos: String,
    /// Label offset
    pub labeloffset: f64,
    /// Whether edge was reversed for acyclic processing
    pub reversed: bool,
    /// Original edge name before reversal
    pub forward_name: Option<String>,
    /// For network simplex cut values
    pub cutvalue: Option<i32>,
    /// Rank where label should be placed (for normalization)
    pub label_rank: Option<i32>,
    /// Whether this is a nesting edge (compound graph support)
    pub nesting_edge: bool,
    /// Original source node before edge redirection (for compound graphs)
    pub original_source: Option<String>,
    /// Original target node before edge redirection (for compound graphs)
    pub original_target: Option<String>,
}

impl Default for EdgeLabel {
    fn default() -> Self {
        Self {
            minlen: 1, // dagre.js defaults to 1
            weight: 1, // dagre.js defaults to 1
            width: 0.0,
            height: 0.0,
            x: None,
            y: None,
            points: Vec::new(),
            labelpos: "r".to_string(),
            labeloffset: 10.0,
            reversed: false,
            forward_name: None,
            cutvalue: None,
            label_rank: None,
            nesting_edge: false,
            original_source: None,
            original_target: None,
        }
    }
}

/// A 2D point
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

impl DagreGraph {
    /// Create a new empty graph
    pub fn new() -> Self {
        Self {
            graph_label: GraphLabel {
                nodesep: 50.0,
                edgesep: 20.0,
                ranksep: 50.0,
                rankdir: "TB".to_string(),
                marginx: 0.0,
                marginy: 0.0,
                acyclicer: "greedy".to_string(),
                ranker: "network-simplex".to_string(),
                width: None,
                height: None,
                dummy_chains: Vec::new(),
                nesting_root: None,
                node_rank_factor: None,
                max_rank: None,
            },
            nodes: HashMap::new(),
            edges: HashMap::new(),
            out_edges: HashMap::new(),
            in_edges: HashMap::new(),
            parent: HashMap::new(),
            children: HashMap::new(),
            edge_counter: 0,
        }
    }

    /// Get graph-level label
    pub fn graph(&self) -> &GraphLabel {
        &self.graph_label
    }

    /// Get mutable graph-level label
    pub fn graph_mut(&mut self) -> &mut GraphLabel {
        &mut self.graph_label
    }

    /// Set graph-level label
    pub fn set_graph(&mut self, label: GraphLabel) {
        self.graph_label = label;
    }

    // --- Node operations ---

    /// Get all node ids (sorted for deterministic iteration order)
    pub fn nodes(&self) -> Vec<&String> {
        let mut nodes: Vec<&String> = self.nodes.keys().collect();
        nodes.sort();
        nodes
    }

    /// Get number of nodes
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Check if node exists
    pub fn has_node(&self, v: &str) -> bool {
        self.nodes.contains_key(v)
    }

    /// Get node label
    pub fn node(&self, v: &str) -> Option<&NodeLabel> {
        self.nodes.get(v)
    }

    /// Get mutable node label
    pub fn node_mut(&mut self, v: &str) -> Option<&mut NodeLabel> {
        self.nodes.get_mut(v)
    }

    /// Set a node with the given label
    pub fn set_node(&mut self, v: impl Into<String>, label: NodeLabel) {
        let v = v.into();
        if !self.nodes.contains_key(&v) {
            self.out_edges.insert(v.clone(), Vec::new());
            self.in_edges.insert(v.clone(), Vec::new());
        }
        self.nodes.insert(v, label);
    }

    /// Set multiple nodes
    pub fn set_nodes(&mut self, nodes: &[&str]) {
        for v in nodes {
            self.set_node(*v, NodeLabel::default());
        }
    }

    /// Remove a node and its edges
    pub fn remove_node(&mut self, v: &str) {
        if self.nodes.remove(v).is_some() {
            // Remove all edges connected to this node
            if let Some(out) = self.out_edges.remove(v) {
                for key in out {
                    self.edges.remove(&key);
                    if let Some(in_list) = self.in_edges.get_mut(&key.w) {
                        in_list.retain(|k| k != &key);
                    }
                }
            }
            if let Some(in_list) = self.in_edges.remove(v) {
                for key in in_list {
                    self.edges.remove(&key);
                    if let Some(out_list) = self.out_edges.get_mut(&key.v) {
                        out_list.retain(|k| k != &key);
                    }
                }
            }
            // Clean up parent/child relationships
            if let Some(parent) = self.parent.remove(v) {
                if let Some(children) = self.children.get_mut(&parent) {
                    children.remove(v);
                }
            }
            self.children.remove(v);
        }
    }

    // --- Edge operations ---

    /// Get all edges (sorted for deterministic iteration order)
    /// Get all edge keys in insertion order
    ///
    /// Preserves the order edges were added to maintain proper edge ordering
    /// through normalization and other phases.
    pub fn edges(&self) -> Vec<&EdgeKey> {
        // Note: We no longer sort alphabetically here because it breaks edge
        // definition order when normalize creates dummy edges. The edge order
        // is important for fork/join layout where the first defined edge target
        // should appear on the left.
        self.edges.keys().collect()
    }

    /// Get number of edges
    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }

    /// Check if edge exists
    pub fn has_edge(&self, v: &str, w: &str) -> bool {
        self.edges.keys().any(|k| k.v == v && k.w == w)
    }

    /// Check if edge with specific name exists
    pub fn has_edge_with_name(&self, v: &str, w: &str, name: &str) -> bool {
        let key = EdgeKey::with_name(v, w, name);
        self.edges.contains_key(&key)
    }

    /// Get edge label
    pub fn edge(&self, v: &str, w: &str) -> Option<&EdgeLabel> {
        self.edges
            .iter()
            .find(|(k, _)| k.v == v && k.w == w)
            .map(|(_, label)| label)
    }

    /// Get mutable edge label
    pub fn edge_mut(&mut self, v: &str, w: &str) -> Option<&mut EdgeLabel> {
        self.edges
            .iter_mut()
            .find(|(k, _)| k.v == v && k.w == w)
            .map(|(_, label)| label)
    }

    /// Get edge label by key
    pub fn edge_by_key(&self, key: &EdgeKey) -> Option<&EdgeLabel> {
        self.edges.get(key)
    }

    /// Get mutable edge label by key
    pub fn edge_by_key_mut(&mut self, key: &EdgeKey) -> Option<&mut EdgeLabel> {
        self.edges.get_mut(key)
    }

    /// Set an edge with the given label
    pub fn set_edge(&mut self, v: impl Into<String>, w: impl Into<String>, label: EdgeLabel) {
        let v = v.into();
        let w = w.into();
        let key = EdgeKey::new(v.clone(), w.clone());

        // Ensure nodes exist
        if !self.nodes.contains_key(&v) {
            self.set_node(v.clone(), NodeLabel::default());
        }
        if !self.nodes.contains_key(&w) {
            self.set_node(w.clone(), NodeLabel::default());
        }

        // Add edge
        self.edges.insert(key.clone(), label);
        self.out_edges.get_mut(&v).unwrap().push(key.clone());
        self.in_edges.get_mut(&w).unwrap().push(key);
    }

    /// Set an edge with a specific name (for multigraph support)
    pub fn set_edge_with_name(
        &mut self,
        v: impl Into<String>,
        w: impl Into<String>,
        label: EdgeLabel,
        name: impl Into<String>,
    ) {
        let v = v.into();
        let w = w.into();
        let key = EdgeKey::with_name(v.clone(), w.clone(), name);

        // Ensure nodes exist
        if !self.nodes.contains_key(&v) {
            self.set_node(v.clone(), NodeLabel::default());
        }
        if !self.nodes.contains_key(&w) {
            self.set_node(w.clone(), NodeLabel::default());
        }

        // Add edge
        self.edges.insert(key.clone(), label);
        self.out_edges.get_mut(&v).unwrap().push(key.clone());
        self.in_edges.get_mut(&w).unwrap().push(key);
    }

    /// Remove an edge
    pub fn remove_edge(&mut self, v: &str, w: &str) {
        let keys: Vec<_> = self
            .edges
            .keys()
            .filter(|k| k.v == v && k.w == w)
            .cloned()
            .collect();

        for key in keys {
            self.remove_edge_by_key(&key);
        }
    }

    /// Remove edge by key
    pub fn remove_edge_by_key(&mut self, key: &EdgeKey) {
        if self.edges.remove(key).is_some() {
            if let Some(out_list) = self.out_edges.get_mut(&key.v) {
                out_list.retain(|k| k != key);
            }
            if let Some(in_list) = self.in_edges.get_mut(&key.w) {
                in_list.retain(|k| k != key);
            }
        }
    }

    /// Get outgoing edges from a node in insertion order
    ///
    /// Preserves the order edges were added to maintain proper fork/join ordering
    /// where the first defined edge should appear first in layout.
    pub fn out_edges(&self, v: &str) -> Vec<&EdgeKey> {
        self.out_edges
            .get(v)
            .map(|edges| edges.iter().collect())
            .unwrap_or_default()
    }

    /// Get outgoing edges from v to w specifically in insertion order
    pub fn out_edges_to(&self, v: &str, w: &str) -> Vec<&EdgeKey> {
        self.out_edges
            .get(v)
            .map(|edges| edges.iter().filter(|e| e.w == w).collect())
            .unwrap_or_default()
    }

    /// Get incoming edges to a node in insertion order
    ///
    /// Preserves the order edges were added to maintain consistency with out_edges.
    pub fn in_edges(&self, w: &str) -> Vec<&EdgeKey> {
        self.in_edges
            .get(w)
            .map(|edges| edges.iter().collect())
            .unwrap_or_default()
    }

    /// Get predecessor nodes in edge insertion order
    ///
    /// Preserves the order edges were added for deterministic behavior.
    pub fn predecessors(&self, v: &str) -> Vec<&String> {
        self.in_edges
            .get(v)
            .map(|edges| edges.iter().map(|e| &e.v).collect())
            .unwrap_or_default()
    }

    /// Get successor nodes in edge insertion order
    ///
    /// Preserves the order edges were added to maintain proper fork/join ordering
    /// where the first defined edge target should appear on the left in TB layout.
    pub fn successors(&self, v: &str) -> Vec<&String> {
        self.out_edges
            .get(v)
            .map(|edges| edges.iter().map(|e| &e.w).collect())
            .unwrap_or_default()
    }

    /// Get neighbor nodes (predecessors + successors, sorted for deterministic iteration)
    pub fn neighbors(&self, v: &str) -> Vec<&String> {
        let mut result: Vec<&String> = Vec::new();
        result.extend(self.predecessors(v));
        result.extend(self.successors(v));
        result.sort();
        result
    }

    // --- Path operations ---

    /// Set a path of edges through the given nodes
    pub fn set_path(&mut self, nodes: &[&str]) {
        for i in 0..nodes.len() - 1 {
            self.set_edge(nodes[i], nodes[i + 1], EdgeLabel::default());
        }
    }

    // --- Compound graph operations ---

    /// Set parent of a node
    pub fn set_parent(&mut self, v: impl Into<String>, parent: impl Into<String>) {
        let v = v.into();
        let parent = parent.into();

        // Ensure nodes exist
        if !self.nodes.contains_key(&v) {
            self.set_node(v.clone(), NodeLabel::default());
        }
        if !self.nodes.contains_key(&parent) {
            self.set_node(parent.clone(), NodeLabel::default());
        }

        // Remove from old parent if any
        if let Some(old_parent) = self.parent.get(&v).cloned() {
            if let Some(children) = self.children.get_mut(&old_parent) {
                children.remove(&v);
            }
        }

        // Set new parent
        self.parent.insert(v.clone(), parent.clone());
        self.children.entry(parent).or_default().insert(v);
    }

    /// Get parent of a node
    pub fn parent(&self, v: &str) -> Option<&String> {
        self.parent.get(v)
    }

    /// Get children of a node (sorted for deterministic iteration)
    pub fn children(&self, v: &str) -> Vec<&String> {
        let mut children: Vec<&String> = self
            .children
            .get(v)
            .map(|c| c.iter().collect())
            .unwrap_or_default();
        children.sort();
        children
    }

    /// Get root-level nodes (nodes without parents, sorted for deterministic iteration)
    pub fn root_children(&self) -> Vec<&String> {
        let mut roots: Vec<&String> = self
            .nodes
            .keys()
            .filter(|v| !self.parent.contains_key(*v))
            .collect();
        roots.sort();
        roots
    }

    /// Check if this is a compound graph (any node has children)
    pub fn is_compound(&self) -> bool {
        self.children.values().any(|c| !c.is_empty())
    }

    // --- Utility ---

    /// Generate a unique id for internal use
    pub fn unique_id(&mut self, prefix: &str) -> String {
        self.edge_counter += 1;
        format!("{}_{}", prefix, self.edge_counter)
    }
}

impl Default for DagreGraph {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_graph() {
        let g = DagreGraph::new();
        assert_eq!(g.node_count(), 0);
        assert_eq!(g.edge_count(), 0);
    }

    #[test]
    fn test_set_and_get_node() {
        let mut g = DagreGraph::new();
        g.set_node(
            "a",
            NodeLabel {
                width: 100.0,
                height: 50.0,
                ..Default::default()
            },
        );

        assert!(g.has_node("a"));
        assert!(!g.has_node("b"));

        let node = g.node("a").unwrap();
        assert_eq!(node.width, 100.0);
        assert_eq!(node.height, 50.0);
    }

    #[test]
    fn test_set_and_get_edge() {
        let mut g = DagreGraph::new();
        g.set_edge(
            "a",
            "b",
            EdgeLabel {
                minlen: 2,
                weight: 3,
                ..Default::default()
            },
        );

        assert!(g.has_edge("a", "b"));
        assert!(!g.has_edge("b", "a"));

        let edge = g.edge("a", "b").unwrap();
        assert_eq!(edge.minlen, 2);
        assert_eq!(edge.weight, 3);
    }

    #[test]
    fn test_multigraph() {
        let mut g = DagreGraph::new();
        g.set_edge("a", "b", EdgeLabel::default());
        g.set_edge_with_name(
            "a",
            "b",
            EdgeLabel {
                weight: 5,
                ..Default::default()
            },
            "edge2",
        );

        // Should have 2 edges from a to b
        let out = g.out_edges("a");
        assert_eq!(out.len(), 2);
    }

    #[test]
    fn test_set_path() {
        let mut g = DagreGraph::new();
        g.set_path(&["a", "b", "c", "d"]);

        assert!(g.has_edge("a", "b"));
        assert!(g.has_edge("b", "c"));
        assert!(g.has_edge("c", "d"));
        assert!(!g.has_edge("a", "c"));
        assert_eq!(g.edge_count(), 3);
    }

    #[test]
    fn test_predecessors_and_successors() {
        let mut g = DagreGraph::new();
        g.set_path(&["a", "b", "c"]);
        g.set_edge("a", "c", EdgeLabel::default());

        assert_eq!(g.predecessors("b"), vec![&"a".to_string()]);
        assert_eq!(g.successors("a").len(), 2); // b and c
        assert!(g.successors("a").contains(&&"b".to_string()));
        assert!(g.successors("a").contains(&&"c".to_string()));
    }

    #[test]
    fn test_compound_graph() {
        let mut g = DagreGraph::new();
        g.set_node("a", NodeLabel::default());
        g.set_node("sg1", NodeLabel::default());
        g.set_parent("a", "sg1");

        assert_eq!(g.parent("a"), Some(&"sg1".to_string()));
        assert!(g.children("sg1").contains(&&"a".to_string()));
    }

    #[test]
    fn test_remove_node() {
        let mut g = DagreGraph::new();
        g.set_path(&["a", "b", "c"]);
        g.remove_node("b");

        assert!(!g.has_node("b"));
        assert!(!g.has_edge("a", "b"));
        assert!(!g.has_edge("b", "c"));
        assert!(g.has_node("a"));
        assert!(g.has_node("c"));
    }

    #[test]
    fn test_remove_edge() {
        let mut g = DagreGraph::new();
        g.set_edge("a", "b", EdgeLabel::default());
        g.remove_edge("a", "b");

        assert!(!g.has_edge("a", "b"));
        assert!(g.has_node("a"));
        assert!(g.has_node("b"));
    }
}
