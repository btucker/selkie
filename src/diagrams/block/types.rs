//! Block diagram types
//!
//! Block diagrams show blocks/nodes and their connections in a grid layout.

use std::collections::HashMap;

/// Block shape types
#[derive(Debug, Clone, PartialEq, Default)]
pub enum BlockType {
    #[default]
    Square,
    Round,
    Circle,
    Diamond,
    Hexagon,
    Stadium,
    Subroutine,
    Cylinder,
    DoubleCircle,
    LeanRight,
    LeanLeft,
    Trapezoid,
    InvTrapezoid,
    BlockArrow,
    Space,
    Composite,
    Edge,
}

/// A block in the diagram
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Block {
    pub id: String,
    pub label: Option<String>,
    pub block_type: BlockType,
    pub children: Vec<Block>,
    pub columns: Option<usize>,
    pub width_in_columns: Option<usize>,
    pub classes: Vec<String>,
    pub styles: Vec<String>,
    pub parent_id: Option<String>,
}

impl Block {
    pub fn new(id: String) -> Self {
        Self {
            id,
            label: None,
            block_type: BlockType::Square,
            children: Vec::new(),
            columns: None,
            width_in_columns: None,
            classes: Vec::new(),
            styles: Vec::new(),
            parent_id: None,
        }
    }

    pub fn with_label(mut self, label: &str) -> Self {
        self.label = Some(label.to_string());
        self
    }

    pub fn with_type(mut self, block_type: BlockType) -> Self {
        self.block_type = block_type;
        self
    }
}

/// An edge between blocks
#[derive(Debug, Clone, PartialEq)]
pub struct Edge {
    pub id: String,
    pub start: String,
    pub end: String,
    pub label: Option<String>,
    pub arrow_type_start: Option<String>,
    pub arrow_type_end: Option<String>,
}

impl Edge {
    pub fn new(start: String, end: String) -> Self {
        Self {
            id: format!("{}-{}", start, end),
            start,
            end,
            label: None,
            arrow_type_start: None,
            arrow_type_end: Some("arrow_point".to_string()),
        }
    }
}

/// A class definition
#[derive(Debug, Clone, PartialEq)]
pub struct ClassDef {
    pub id: String,
    pub styles: Vec<String>,
    pub text_styles: Vec<String>,
}

/// The Block diagram database
#[derive(Debug, Clone, Default)]
pub struct BlockDb {
    /// All blocks by ID
    blocks: HashMap<String, Block>,
    /// Block IDs in insertion order (for maintaining layout order)
    block_order: Vec<String>,
    /// Root block (composite containing all top-level blocks)
    root_block: Block,
    /// All edges
    edges: Vec<Edge>,
    /// Class definitions
    classes: HashMap<String, ClassDef>,
    /// Counter for generating unique space IDs
    space_counter: usize,
    /// Counter for generating unique composite IDs
    composite_counter: usize,
}

impl BlockDb {
    /// Create a new empty BlockDb
    pub fn new() -> Self {
        Self {
            root_block: Block {
                id: "root".to_string(),
                label: None,
                block_type: BlockType::Composite,
                children: Vec::new(),
                columns: None,
                width_in_columns: None,
                classes: Vec::new(),
                styles: Vec::new(),
                parent_id: None,
            },
            ..Default::default()
        }
    }

    /// Clear all data
    pub fn clear(&mut self) {
        *self = Self::new();
    }

    /// Add a block with optional parent
    pub fn add_block_with_parent(
        &mut self,
        id: &str,
        label: Option<&str>,
        block_type: BlockType,
        parent_id: Option<&str>,
    ) {
        // Protect against prototype pollution
        if id == "__proto__" || id == "constructor" {
            return;
        }

        let mut block = Block::new(id.to_string());
        block.label = label.map(|s| s.to_string());
        block.block_type = block_type;
        block.parent_id = parent_id.map(|s| s.to_string());

        // Track insertion order
        if !self.blocks.contains_key(id) {
            self.block_order.push(id.to_string());
        }
        self.blocks.insert(id.to_string(), block.clone());
        // Only add to root children if no parent (top-level block)
        if parent_id.is_none() {
            self.root_block.children.push(block);
        }
    }

    /// Add a block (legacy method, adds to root)
    pub fn add_block(&mut self, id: &str, label: Option<&str>, block_type: BlockType) {
        self.add_block_with_parent(id, label, block_type, None);
    }

    /// Add an edge
    pub fn add_edge(&mut self, start: &str, end: &str, label: Option<&str>) {
        let mut edge = Edge::new(start.to_string(), end.to_string());
        edge.label = label.map(|s| s.to_string());
        self.edges.push(edge);
    }

    /// Define a class
    pub fn define_class(&mut self, id: &str, styles: &[&str]) {
        let class_def = ClassDef {
            id: id.to_string(),
            styles: styles.iter().map(|s| s.to_string()).collect(),
            text_styles: Vec::new(),
        };
        self.classes.insert(id.to_string(), class_def);
    }

    /// Apply a class to a block
    pub fn apply_class(&mut self, block_id: &str, class_name: &str) {
        if let Some(block) = self.blocks.get_mut(block_id) {
            block.classes.push(class_name.to_string());
        }
    }

    /// Set columns for a block
    pub fn set_columns(&mut self, block_id: &str, columns: usize) {
        if block_id == "root" {
            self.root_block.columns = Some(columns);
        } else if let Some(block) = self.blocks.get_mut(block_id) {
            block.columns = Some(columns);
        }
    }

    /// Set width in columns for a block
    pub fn set_width(&mut self, block_id: &str, width: usize) {
        if let Some(block) = self.blocks.get_mut(block_id) {
            block.width_in_columns = Some(width);
        }
    }

    /// Apply styles to a block
    pub fn apply_styles(&mut self, block_id: &str, styles: &[String]) {
        if let Some(block) = self.blocks.get_mut(block_id) {
            block.styles.extend(styles.iter().cloned());
        }
    }

    /// Generate a unique space block ID
    pub fn generate_space_id(&mut self) -> String {
        self.space_counter += 1;
        format!("space_{}", self.space_counter)
    }

    /// Generate a unique composite block ID
    pub fn generate_composite_id(&mut self) -> String {
        self.composite_counter += 1;
        format!("composite_{}", self.composite_counter)
    }

    /// Get all blocks
    pub fn get_blocks(&self) -> &HashMap<String, Block> {
        &self.blocks
    }

    /// Get blocks as flat vec (insertion order preserved)
    pub fn get_blocks_flat(&self) -> Vec<&Block> {
        self.block_order
            .iter()
            .filter_map(|id| self.blocks.get(id))
            .collect()
    }

    /// Get block IDs in insertion order
    pub fn get_block_order(&self) -> &[String] {
        &self.block_order
    }

    /// Get all edges
    pub fn get_edges(&self) -> &[Edge] {
        &self.edges
    }

    /// Get class definitions
    pub fn get_classes(&self) -> &HashMap<String, ClassDef> {
        &self.classes
    }

    /// Get root columns setting
    pub fn get_columns(&self) -> Option<usize> {
        self.root_block.columns
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_block() {
        let mut db = BlockDb::new();
        db.add_block("a", Some("Block A"), BlockType::Square);

        let blocks = db.get_blocks();
        assert!(blocks.contains_key("a"));
        assert_eq!(blocks.get("a").unwrap().label, Some("Block A".to_string()));
    }

    #[test]
    fn test_add_block_with_type() {
        let mut db = BlockDb::new();
        db.add_block("b", Some("Block B"), BlockType::Round);

        let blocks = db.get_blocks();
        assert_eq!(blocks.get("b").unwrap().block_type, BlockType::Round);
    }

    #[test]
    fn test_add_edge() {
        let mut db = BlockDb::new();
        db.add_block("a", None, BlockType::Square);
        db.add_block("b", None, BlockType::Square);
        db.add_edge("a", "b", Some("connects"));

        let edges = db.get_edges();
        assert_eq!(edges.len(), 1);
        assert_eq!(edges[0].start, "a");
        assert_eq!(edges[0].end, "b");
        assert_eq!(edges[0].label, Some("connects".to_string()));
    }

    #[test]
    fn test_define_class() {
        let mut db = BlockDb::new();
        db.define_class("highlight", &["fill: yellow"]);

        let classes = db.get_classes();
        assert!(classes.contains_key("highlight"));
        assert_eq!(
            classes.get("highlight").unwrap().styles,
            vec!["fill: yellow"]
        );
    }

    #[test]
    fn test_apply_class() {
        let mut db = BlockDb::new();
        db.add_block("a", None, BlockType::Square);
        db.define_class("highlight", &["fill: yellow"]);
        db.apply_class("a", "highlight");

        let blocks = db.get_blocks();
        assert!(blocks
            .get("a")
            .unwrap()
            .classes
            .contains(&"highlight".to_string()));
    }

    #[test]
    fn test_proto_protection() {
        let mut db = BlockDb::new();
        db.add_block("__proto__", None, BlockType::Square);
        db.add_block("constructor", None, BlockType::Square);

        // These should be ignored
        let blocks = db.get_blocks();
        assert!(!blocks.contains_key("__proto__"));
        assert!(!blocks.contains_key("constructor"));
    }

    #[test]
    fn test_set_columns() {
        let mut db = BlockDb::new();
        db.add_block("a", None, BlockType::Composite);
        db.set_columns("a", 3);

        let blocks = db.get_blocks();
        assert_eq!(blocks.get("a").unwrap().columns, Some(3));
    }

    #[test]
    fn test_clear() {
        let mut db = BlockDb::new();
        db.add_block("a", None, BlockType::Square);
        db.clear();

        assert!(db.get_blocks().is_empty());
    }
}
