//! Layout graph structure and traversal methods

use std::collections::{HashMap, HashSet};

use super::types::{LayoutEdge, LayoutNode, LayoutOptions};

/// A graph ready for layout computation
#[derive(Debug, Clone)]
pub struct LayoutGraph {
    /// Graph identifier
    pub id: String,
    /// All nodes in the graph (flat list, children are nested in nodes)
    pub nodes: Vec<LayoutNode>,
    /// All edges in the graph
    pub edges: Vec<LayoutEdge>,
    /// Layout options
    pub options: LayoutOptions,
    /// Computed graph width (set after layout)
    pub width: Option<f64>,
    /// Computed graph height (set after layout)
    pub height: Option<f64>,
}

impl LayoutGraph {
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            nodes: Vec::new(),
            edges: Vec::new(),
            options: LayoutOptions::default(),
            width: None,
            height: None,
        }
    }

    pub fn with_options(mut self, options: LayoutOptions) -> Self {
        self.options = options;
        self
    }

    /// Add a node to the graph
    pub fn add_node(&mut self, node: LayoutNode) {
        self.nodes.push(node);
    }

    /// Add an edge to the graph
    pub fn add_edge(&mut self, edge: LayoutEdge) {
        self.edges.push(edge);
    }

    /// Get a node by ID (searches recursively through children)
    pub fn get_node(&self, id: &str) -> Option<&LayoutNode> {
        Self::find_node_recursive(&self.nodes, id)
    }

    /// Get a mutable node by ID
    pub fn get_node_mut(&mut self, id: &str) -> Option<&mut LayoutNode> {
        Self::find_node_mut_recursive(&mut self.nodes, id)
    }

    fn find_node_recursive<'a>(nodes: &'a [LayoutNode], id: &str) -> Option<&'a LayoutNode> {
        for node in nodes {
            if node.id == id {
                return Some(node);
            }
            if let Some(found) = Self::find_node_recursive(&node.children, id) {
                return Some(found);
            }
        }
        None
    }

    fn find_node_mut_recursive<'a>(
        nodes: &'a mut [LayoutNode],
        id: &str,
    ) -> Option<&'a mut LayoutNode> {
        for node in nodes {
            if node.id == id {
                return Some(node);
            }
            if let Some(found) = Self::find_node_mut_recursive(&mut node.children, id) {
                return Some(found);
            }
        }
        None
    }

    /// Get all edges where the given node is the source
    pub fn out_edges(&self, node_id: &str) -> Vec<&LayoutEdge> {
        self.edges
            .iter()
            .filter(|e| e.sources.iter().any(|s| s == node_id))
            .collect()
    }

    /// Get all edges where the given node is the target
    pub fn in_edges(&self, node_id: &str) -> Vec<&LayoutEdge> {
        self.edges
            .iter()
            .filter(|e| e.targets.iter().any(|t| t == node_id))
            .collect()
    }

    /// Get successor node IDs (nodes this node points to)
    pub fn successors(&self, node_id: &str) -> Vec<&str> {
        self.out_edges(node_id)
            .iter()
            .flat_map(|e| e.targets.iter().map(|s| s.as_str()))
            .collect()
    }

    /// Get predecessor node IDs (nodes that point to this node)
    pub fn predecessors(&self, node_id: &str) -> Vec<&str> {
        self.in_edges(node_id)
            .iter()
            .flat_map(|e| e.sources.iter().map(|s| s.as_str()))
            .collect()
    }

    /// Get all node IDs (recursively including children)
    pub fn all_node_ids(&self) -> Vec<&str> {
        let mut ids = Vec::new();
        self.collect_node_ids(&self.nodes, &mut ids);
        ids
    }

    fn collect_node_ids<'a>(&self, nodes: &'a [LayoutNode], ids: &mut Vec<&'a str>) {
        for node in nodes {
            ids.push(&node.id);
            self.collect_node_ids(&node.children, ids);
        }
    }

    /// Build an adjacency list representation of the graph
    pub fn adjacency_list(&self) -> HashMap<&str, Vec<&str>> {
        let mut adj: HashMap<&str, Vec<&str>> = HashMap::new();

        // Initialize all nodes
        for id in self.all_node_ids() {
            adj.entry(id).or_default();
        }

        // Add edges
        for edge in &self.edges {
            for source in &edge.sources {
                for target in &edge.targets {
                    adj.entry(source.as_str())
                        .or_default()
                        .push(target.as_str());
                }
            }
        }

        adj
    }

    /// Get all root-level (non-dummy) nodes
    pub fn root_nodes(&self) -> impl Iterator<Item = &LayoutNode> {
        self.nodes.iter().filter(|n| !n.is_dummy)
    }

    /// Compute bounding box after layout
    pub fn compute_bounds(&mut self) {
        let mut min_x = f64::MAX;
        let mut min_y = f64::MAX;
        let mut max_x = f64::MIN;
        let mut max_y = f64::MIN;

        self.traverse_nodes(|node| {
            if let (Some(x), Some(y)) = (node.x, node.y) {
                min_x = min_x.min(x);
                min_y = min_y.min(y);
                max_x = max_x.max(x + node.width);
                max_y = max_y.max(y + node.height);
            }
        });

        if min_x != f64::MAX {
            self.width =
                Some(max_x - min_x + self.options.padding.left + self.options.padding.right);
            self.height =
                Some(max_y - min_y + self.options.padding.top + self.options.padding.bottom);
        }
    }

    /// Traverse all nodes with a callback
    pub fn traverse_nodes<F>(&self, mut f: F)
    where
        F: FnMut(&LayoutNode),
    {
        self.traverse_nodes_recursive(&self.nodes, &mut f);
    }

    fn traverse_nodes_recursive<F>(&self, nodes: &[LayoutNode], f: &mut F)
    where
        F: FnMut(&LayoutNode),
    {
        for node in nodes {
            f(node);
            self.traverse_nodes_recursive(&node.children, f);
        }
    }

    /// Traverse all nodes mutably
    pub fn traverse_nodes_mut<F>(&mut self, mut f: F)
    where
        F: FnMut(&mut LayoutNode),
    {
        Self::traverse_nodes_mut_recursive(&mut self.nodes, &mut f);
    }

    fn traverse_nodes_mut_recursive<F>(nodes: &mut [LayoutNode], f: &mut F)
    where
        F: FnMut(&mut LayoutNode),
    {
        for node in nodes {
            f(node);
            Self::traverse_nodes_mut_recursive(&mut node.children, f);
        }
    }

    /// Check if the graph has cycles using DFS
    pub fn has_cycles(&self) -> bool {
        let adj = self.adjacency_list();
        let mut visited = HashSet::new();
        let mut rec_stack = HashSet::new();

        for node_id in self.all_node_ids() {
            if self.has_cycle_dfs(node_id, &adj, &mut visited, &mut rec_stack) {
                return true;
            }
        }
        false
    }

    fn has_cycle_dfs<'a>(
        &self,
        node_id: &'a str,
        adj: &HashMap<&'a str, Vec<&'a str>>,
        visited: &mut HashSet<&'a str>,
        rec_stack: &mut HashSet<&'a str>,
    ) -> bool {
        if rec_stack.contains(node_id) {
            return true;
        }
        if visited.contains(node_id) {
            return false;
        }

        visited.insert(node_id);
        rec_stack.insert(node_id);

        if let Some(neighbors) = adj.get(node_id) {
            for &neighbor in neighbors {
                if self.has_cycle_dfs(neighbor, adj, visited, rec_stack) {
                    return true;
                }
            }
        }

        rec_stack.remove(node_id);
        false
    }

    /// Get nodes organized by layer (after layering phase)
    pub fn nodes_by_layer(&self) -> Vec<Vec<&LayoutNode>> {
        let mut max_layer = 0;
        self.traverse_nodes(|n| {
            if let Some(l) = n.layer {
                max_layer = max_layer.max(l);
            }
        });

        let mut layers: Vec<Vec<&LayoutNode>> = vec![Vec::new(); max_layer + 1];

        // Collect nodes by layer
        fn collect_by_layer<'a>(nodes: &'a [LayoutNode], layers: &mut Vec<Vec<&'a LayoutNode>>) {
            for node in nodes {
                if let Some(l) = node.layer {
                    if l < layers.len() {
                        layers[l].push(node);
                    }
                }
                collect_by_layer(&node.children, layers);
            }
        }

        collect_by_layer(&self.nodes, &mut layers);

        // Sort each layer by order
        for layer in &mut layers {
            layer.sort_by_key(|n| n.order.unwrap_or(0));
        }

        layers
    }
}

impl Default for LayoutGraph {
    fn default() -> Self {
        Self::new("graph")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_graph() {
        let mut graph = LayoutGraph::new("test");
        graph.add_node(LayoutNode::new("A", 50.0, 30.0));
        graph.add_node(LayoutNode::new("B", 50.0, 30.0));
        graph.add_edge(LayoutEdge::new("e1", "A", "B"));

        assert_eq!(graph.nodes.len(), 2);
        assert_eq!(graph.edges.len(), 1);
        assert_eq!(graph.successors("A"), vec!["B"]);
        assert_eq!(graph.predecessors("B"), vec!["A"]);
    }

    #[test]
    fn test_cycle_detection() {
        let mut graph = LayoutGraph::new("test");
        graph.add_node(LayoutNode::new("A", 50.0, 30.0));
        graph.add_node(LayoutNode::new("B", 50.0, 30.0));
        graph.add_node(LayoutNode::new("C", 50.0, 30.0));
        graph.add_edge(LayoutEdge::new("e1", "A", "B"));
        graph.add_edge(LayoutEdge::new("e2", "B", "C"));

        assert!(!graph.has_cycles());

        // Add cycle
        graph.add_edge(LayoutEdge::new("e3", "C", "A"));
        assert!(graph.has_cycles());
    }
}
