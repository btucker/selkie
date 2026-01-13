//! Sankey diagram types
//!
//! Sankey diagrams show flow/movement between nodes with weighted connections.

use std::collections::HashMap;

/// A node in the sankey diagram
#[derive(Debug, Clone, PartialEq)]
pub struct SankeyNode {
    pub id: String,
}

impl SankeyNode {
    pub fn new(id: String) -> Self {
        Self { id }
    }
}

/// A link between two nodes
#[derive(Debug, Clone, PartialEq)]
pub struct SankeyLink {
    pub source: String,
    pub target: String,
    pub value: f64,
}

impl SankeyLink {
    pub fn new(source: String, target: String, value: f64) -> Self {
        Self {
            source,
            target,
            value,
        }
    }
}

/// The Sankey database
#[derive(Debug, Clone, Default)]
pub struct SankeyDb {
    /// Nodes map for uniqueness
    nodes_map: HashMap<String, SankeyNode>,
    /// Nodes in order
    nodes: Vec<SankeyNode>,
    /// Links
    links: Vec<SankeyLink>,
}

impl SankeyDb {
    /// Create a new empty SankeyDb
    pub fn new() -> Self {
        Self::default()
    }

    /// Clear all data
    pub fn clear(&mut self) {
        *self = Self::default();
    }

    /// Find or create a node
    pub fn find_or_create_node(&mut self, id: &str) -> &SankeyNode {
        // Protect against prototype pollution
        if id == "__proto__" {
            panic!("Illegal node ID: __proto__");
        }

        if !self.nodes_map.contains_key(id) {
            let node = SankeyNode::new(id.to_string());
            self.nodes.push(node.clone());
            self.nodes_map.insert(id.to_string(), node);
        }
        self.nodes_map.get(id).unwrap()
    }

    /// Add a link
    pub fn add_link(&mut self, source: &str, target: &str, value: f64) {
        // Ensure nodes exist
        self.find_or_create_node(source);
        self.find_or_create_node(target);

        let link = SankeyLink::new(source.to_string(), target.to_string(), value);
        self.links.push(link);
    }

    /// Get all nodes
    pub fn get_nodes(&self) -> &[SankeyNode] {
        &self.nodes
    }

    /// Get all links
    pub fn get_links(&self) -> &[SankeyLink] {
        &self.links
    }

    /// Get the graph structure for rendering
    pub fn get_graph(&self) -> SankeyGraph {
        SankeyGraph {
            nodes: self
                .nodes
                .iter()
                .map(|n| GraphNode { id: n.id.clone() })
                .collect(),
            links: self
                .links
                .iter()
                .map(|l| GraphLink {
                    source: l.source.clone(),
                    target: l.target.clone(),
                    value: l.value,
                })
                .collect(),
        }
    }
}

/// Graph node for rendering
#[derive(Debug, Clone, PartialEq)]
pub struct GraphNode {
    pub id: String,
}

/// Graph link for rendering
#[derive(Debug, Clone, PartialEq)]
pub struct GraphLink {
    pub source: String,
    pub target: String,
    pub value: f64,
}

/// Graph structure for D3 rendering
#[derive(Debug, Clone, PartialEq)]
pub struct SankeyGraph {
    pub nodes: Vec<GraphNode>,
    pub links: Vec<GraphLink>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_link_creates_nodes() {
        let mut db = SankeyDb::new();
        db.add_link("Alice", "Bob", 23.0);

        let nodes = db.get_nodes();
        assert_eq!(nodes.len(), 2);
        assert_eq!(nodes[0].id, "Alice");
        assert_eq!(nodes[1].id, "Bob");
    }

    #[test]
    fn test_add_multiple_links() {
        let mut db = SankeyDb::new();
        db.add_link("Alice", "Bob", 23.0);
        db.add_link("Bob", "Carol", 43.0);

        let nodes = db.get_nodes();
        assert_eq!(nodes.len(), 3);

        let links = db.get_links();
        assert_eq!(links.len(), 2);
        assert_eq!(links[0].value, 23.0);
        assert_eq!(links[1].value, 43.0);
    }

    #[test]
    fn test_get_graph() {
        let mut db = SankeyDb::new();
        db.add_link("Alice", "Bob", 23.0);
        db.add_link("Bob", "Carol", 43.0);

        let graph = db.get_graph();

        assert_eq!(graph.nodes.len(), 3);
        assert_eq!(graph.nodes[0].id, "Alice");
        assert_eq!(graph.nodes[1].id, "Bob");
        assert_eq!(graph.nodes[2].id, "Carol");

        assert_eq!(graph.links.len(), 2);
        assert_eq!(graph.links[0].source, "Alice");
        assert_eq!(graph.links[0].target, "Bob");
        assert_eq!(graph.links[0].value, 23.0);
    }

    #[test]
    #[should_panic(expected = "Illegal node ID: __proto__")]
    fn test_proto_protection() {
        let mut db = SankeyDb::new();
        db.find_or_create_node("__proto__");
    }

    #[test]
    fn test_node_deduplication() {
        let mut db = SankeyDb::new();
        db.add_link("Alice", "Bob", 10.0);
        db.add_link("Alice", "Carol", 20.0);

        let nodes = db.get_nodes();
        assert_eq!(nodes.len(), 3); // Alice, Bob, Carol (not 4)
    }

    #[test]
    fn test_clear() {
        let mut db = SankeyDb::new();
        db.add_link("Alice", "Bob", 23.0);
        db.clear();

        assert!(db.get_nodes().is_empty());
        assert!(db.get_links().is_empty());
    }
}
