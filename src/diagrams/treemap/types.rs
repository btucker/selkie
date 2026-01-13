//! Treemap diagram types
//!
//! Treemap diagrams show hierarchical data as nested rectangles.
//! Each leaf node has a value that determines its area.

use std::collections::HashMap;

/// A node in the treemap (either section or leaf)
#[derive(Debug, Clone, PartialEq)]
pub struct TreemapNode {
    pub name: String,
    pub value: Option<f64>,
    pub class_selector: Option<String>,
    pub children: Vec<TreemapNode>,
}

impl TreemapNode {
    /// Create a new section node (no value, can have children)
    pub fn section(name: &str) -> Self {
        Self {
            name: name.to_string(),
            value: None,
            class_selector: None,
            children: Vec::new(),
        }
    }

    /// Create a new leaf node (has value, no children)
    pub fn leaf(name: &str, value: f64) -> Self {
        Self {
            name: name.to_string(),
            value: Some(value),
            class_selector: None,
            children: Vec::new(),
        }
    }

    /// Add a class selector
    pub fn with_class(mut self, class: &str) -> Self {
        self.class_selector = Some(class.to_string());
        self
    }

    /// Check if this is a leaf node
    pub fn is_leaf(&self) -> bool {
        self.value.is_some()
    }

    /// Add a child node
    pub fn add_child(&mut self, child: TreemapNode) {
        self.children.push(child);
    }
}

/// Style class definition
#[derive(Debug, Clone, Default)]
pub struct StyleClass {
    pub id: String,
    pub styles: Vec<String>,
    pub text_styles: Vec<String>,
}

impl StyleClass {
    pub fn new(id: &str) -> Self {
        Self {
            id: id.to_string(),
            styles: Vec::new(),
            text_styles: Vec::new(),
        }
    }

    pub fn add_style(&mut self, style: &str) {
        // Check if it's a label/text style
        let is_text =
            style.starts_with("color:") || style.starts_with("font-") || style.starts_with("text-");
        if is_text {
            self.text_styles.push(style.to_string());
        }
        self.styles.push(style.to_string());
    }
}

/// The Treemap database
#[derive(Debug, Clone, Default)]
pub struct TreemapDb {
    title: String,
    acc_title: String,
    acc_description: String,
    root_nodes: Vec<TreemapNode>,
    classes: HashMap<String, StyleClass>,
}

impl TreemapDb {
    /// Create a new empty TreemapDb
    pub fn new() -> Self {
        Self::default()
    }

    /// Clear all data
    pub fn clear(&mut self) {
        *self = Self::default();
    }

    /// Set the diagram title
    pub fn set_title(&mut self, title: &str) {
        self.title = title.to_string();
    }

    /// Get the diagram title
    pub fn get_title(&self) -> &str {
        &self.title
    }

    /// Set the accessibility title
    pub fn set_acc_title(&mut self, title: &str) {
        self.acc_title = title.to_string();
    }

    /// Get the accessibility title
    pub fn get_acc_title(&self) -> &str {
        &self.acc_title
    }

    /// Set the accessibility description
    pub fn set_acc_description(&mut self, description: &str) {
        self.acc_description = description.to_string();
    }

    /// Get the accessibility description
    pub fn get_acc_description(&self) -> &str {
        &self.acc_description
    }

    /// Add a root node
    pub fn add_root_node(&mut self, node: TreemapNode) {
        self.root_nodes.push(node);
    }

    /// Get the root node (wrapper containing all top-level nodes)
    pub fn get_root(&self) -> TreemapNode {
        TreemapNode {
            name: String::new(),
            value: None,
            class_selector: None,
            children: self.root_nodes.clone(),
        }
    }

    /// Get all root-level nodes
    pub fn get_root_nodes(&self) -> &[TreemapNode] {
        &self.root_nodes
    }

    /// Add a class definition
    pub fn add_class(&mut self, class_name: &str, style_text: &str) {
        let mut style_class = self
            .classes
            .get(class_name)
            .cloned()
            .unwrap_or_else(|| StyleClass::new(class_name));

        // Parse styles (comma or semicolon separated)
        let normalized = style_text
            .replace("\\,", "\x00")
            .replace(',', ";")
            .replace('\x00', ",");

        let styles: Vec<&str> = normalized
            .split(';')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .collect();

        for style in styles {
            style_class.add_style(style);
        }

        self.classes.insert(class_name.to_string(), style_class);
    }

    /// Get classes
    pub fn get_classes(&self) -> &HashMap<String, StyleClass> {
        &self.classes
    }

    /// Get styles for a class
    pub fn get_styles_for_class(&self, class_selector: &str) -> Vec<String> {
        self.classes
            .get(class_selector)
            .map(|c| c.styles.clone())
            .unwrap_or_default()
    }

    /// Count total nodes in the tree
    pub fn count_nodes(&self) -> usize {
        fn count_recursive(nodes: &[TreemapNode]) -> usize {
            nodes.iter().map(|n| 1 + count_recursive(&n.children)).sum()
        }
        count_recursive(&self.root_nodes)
    }
}

/// Build hierarchy from flat items with indentation levels
pub fn build_hierarchy(items: Vec<(usize, TreemapNode)>) -> Vec<TreemapNode> {
    if items.is_empty() {
        return Vec::new();
    }

    let mut root: Vec<TreemapNode> = Vec::new();
    let mut stack: Vec<(usize, usize)> = Vec::new(); // (level, index in root/parent.children)

    for (level, node) in items {
        // Find the right parent for this node
        while !stack.is_empty() && stack.last().unwrap().0 >= level {
            stack.pop();
        }

        if stack.is_empty() {
            // This is a root node
            root.push(node);
            if !root.last().unwrap().is_leaf() {
                stack.push((level, root.len() - 1));
            }
        } else {
            // Find the parent node and add this as a child
            let parent = get_node_mut(&mut root, &stack);
            parent.add_child(node);
            if !parent.children.last().unwrap().is_leaf() {
                stack.push((level, parent.children.len() - 1));
            }
        }
    }

    root
}

/// Helper to get mutable reference to a node using the stack path
fn get_node_mut<'a>(root: &'a mut [TreemapNode], stack: &[(usize, usize)]) -> &'a mut TreemapNode {
    let mut current = &mut root[stack[0].1];
    for &(_, idx) in stack.iter().skip(1) {
        current = &mut current.children[idx];
    }
    current
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_section_node() {
        let node = TreemapNode::section("Category A");
        assert_eq!(node.name, "Category A");
        assert!(node.value.is_none());
        assert!(!node.is_leaf());
    }

    #[test]
    fn test_leaf_node() {
        let node = TreemapNode::leaf("Item A1", 10.0);
        assert_eq!(node.name, "Item A1");
        assert_eq!(node.value, Some(10.0));
        assert!(node.is_leaf());
    }

    #[test]
    fn test_node_with_class() {
        let node = TreemapNode::section("Test").with_class("myClass");
        assert_eq!(node.class_selector, Some("myClass".to_string()));
    }

    #[test]
    fn test_add_child() {
        let mut parent = TreemapNode::section("Parent");
        parent.add_child(TreemapNode::leaf("Child", 5.0));
        assert_eq!(parent.children.len(), 1);
    }

    #[test]
    fn test_build_hierarchy_simple() {
        let items = vec![
            (0, TreemapNode::section("Root")),
            (4, TreemapNode::leaf("Leaf 1", 10.0)),
            (4, TreemapNode::leaf("Leaf 2", 20.0)),
        ];

        let result = build_hierarchy(items);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name, "Root");
        assert_eq!(result[0].children.len(), 2);
    }

    #[test]
    fn test_build_hierarchy_nested() {
        let items = vec![
            (0, TreemapNode::section("Root")),
            (4, TreemapNode::section("Branch")),
            (8, TreemapNode::leaf("Leaf", 10.0)),
        ];

        let result = build_hierarchy(items);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].children.len(), 1);
        assert_eq!(result[0].children[0].children.len(), 1);
        assert_eq!(result[0].children[0].children[0].name, "Leaf");
    }

    #[test]
    fn test_build_hierarchy_multiple_roots() {
        let items = vec![
            (0, TreemapNode::section("Root 1")),
            (4, TreemapNode::leaf("Leaf 1", 10.0)),
            (0, TreemapNode::section("Root 2")),
            (4, TreemapNode::leaf("Leaf 2", 20.0)),
        ];

        let result = build_hierarchy(items);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].name, "Root 1");
        assert_eq!(result[1].name, "Root 2");
    }

    #[test]
    fn test_add_class() {
        let mut db = TreemapDb::new();
        db.add_class("class1", "fill:red,stroke:blue");

        let class = db.get_classes().get("class1").unwrap();
        assert_eq!(class.styles.len(), 2);
    }

    #[test]
    fn test_count_nodes() {
        let mut db = TreemapDb::new();
        let mut root = TreemapNode::section("Root");
        root.add_child(TreemapNode::leaf("Leaf 1", 10.0));
        root.add_child(TreemapNode::leaf("Leaf 2", 20.0));
        db.add_root_node(root);

        assert_eq!(db.count_nodes(), 3);
    }

    #[test]
    fn test_get_root() {
        let mut db = TreemapDb::new();
        db.add_root_node(TreemapNode::section("Root 1"));
        db.add_root_node(TreemapNode::section("Root 2"));

        let root = db.get_root();
        assert_eq!(root.children.len(), 2);
    }

    #[test]
    fn test_clear() {
        let mut db = TreemapDb::new();
        db.set_title("Test");
        db.add_root_node(TreemapNode::section("Root"));
        db.add_class("class1", "fill:red");

        db.clear();

        assert!(db.get_title().is_empty());
        assert!(db.get_root_nodes().is_empty());
        assert!(db.get_classes().is_empty());
    }
}
