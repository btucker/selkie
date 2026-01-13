//! Kanban diagram types
//!
//! Kanban diagrams show work items organized in columns/sections.

/// Priority levels for kanban items
#[derive(Debug, Clone, PartialEq, Default)]
pub enum Priority {
    VeryHigh,
    High,
    #[default]
    Medium,
    Low,
    VeryLow,
}

impl Priority {
    /// Parse a priority from a string
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "very high" | "veryhigh" => Some(Self::VeryHigh),
            "high" => Some(Self::High),
            "medium" => Some(Self::Medium),
            "low" => Some(Self::Low),
            "very low" | "verylow" => Some(Self::VeryLow),
            _ => None,
        }
    }
}

/// Node shape types
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum NodeShape {
    #[default]
    Default,
    Rect,
    RoundedRect,
    Circle,
    Cloud,
    Bang,
    Hexagon,
}

/// A kanban node (either a section/column or an item/card)
#[derive(Debug, Clone, PartialEq)]
pub struct KanbanNode {
    /// Unique node identifier
    pub id: String,
    /// Display label
    pub label: String,
    /// Parent section ID (None for sections, Some for items)
    pub parent_id: Option<String>,
    /// Whether this is a section/group
    pub is_group: bool,
    /// Node level in hierarchy
    pub level: usize,
    /// Node shape
    pub shape: NodeShape,
    /// Icon name
    pub icon: Option<String>,
    /// CSS classes
    pub css_classes: Option<String>,
    /// Priority level
    pub priority: Option<String>,
    /// Ticket/issue ID
    pub ticket: Option<String>,
    /// Assigned person
    pub assigned: Option<String>,
}

impl KanbanNode {
    /// Create a new section node
    pub fn new_section(id: String, label: String, level: usize) -> Self {
        Self {
            id,
            label,
            parent_id: None,
            is_group: true,
            level,
            shape: NodeShape::Default,
            icon: None,
            css_classes: None,
            priority: None,
            ticket: None,
            assigned: None,
        }
    }

    /// Create a new item node
    pub fn new_item(id: String, label: String, parent_id: String, level: usize) -> Self {
        Self {
            id,
            label,
            parent_id: Some(parent_id),
            is_group: false,
            level,
            shape: NodeShape::Default,
            icon: None,
            css_classes: None,
            priority: None,
            ticket: None,
            assigned: None,
        }
    }
}

/// The Kanban database that stores all diagram data
#[derive(Debug, Clone, Default)]
pub struct KanbanDb {
    /// All nodes (sections and items)
    nodes: Vec<KanbanNode>,
}

impl KanbanDb {
    /// Create a new empty KanbanDb
    pub fn new() -> Self {
        Self::default()
    }

    /// Clear all data
    pub fn clear(&mut self) {
        *self = Self::default();
    }

    /// Add a node (section or item)
    ///
    /// Level 0 = section, Level > 0 = item
    pub fn add_node(&mut self, level: usize, id: Option<&str>, label: &str, shape: NodeShape) {
        let id = id
            .map(|s| s.to_string())
            .unwrap_or_else(|| label.to_string());
        let label = label.to_string();

        if level == 0 {
            // This is a section
            // Check if any items exist without a section
            if let Some(last_node) = self.nodes.last() {
                if last_node.level > 0 && !last_node.is_group {
                    // Find the item's parent - if it doesn't have one, error
                    if last_node.parent_id.is_none() {
                        panic!("Items without section detected, found section (\"{}\")", id);
                    }
                }
            }

            let mut node = KanbanNode::new_section(id, label, level);
            node.shape = shape;
            self.nodes.push(node);
        } else {
            // This is an item - find its parent section
            let parent_id = self.get_section_for_level(level);
            if let Some(parent) = parent_id {
                let mut node = KanbanNode::new_item(id, label, parent, level);
                node.shape = shape;
                self.nodes.push(node);
            } else {
                // No parent section found - this is an orphan
                // In mermaid.js, this would be added with parent_id set later
                let mut node = KanbanNode::new_item(id.clone(), label, String::new(), level);
                node.shape = shape;
                self.nodes.push(node);
            }
        }
    }

    /// Get the section ID that should be the parent for a node at the given level
    fn get_section_for_level(&self, _level: usize) -> Option<String> {
        // Find the most recent section (level 0 node)
        for node in self.nodes.iter().rev() {
            if node.level == 0 && node.is_group {
                return Some(node.id.clone());
            }
        }
        None
    }

    /// Decorate the last node with icon
    pub fn decorate_icon(&mut self, icon: &str) {
        if let Some(node) = self.nodes.last_mut() {
            node.icon = Some(icon.to_string());
        }
    }

    /// Decorate the last node with CSS classes
    pub fn decorate_classes(&mut self, classes: &str) {
        if let Some(node) = self.nodes.last_mut() {
            node.css_classes = Some(classes.to_string());
        }
    }

    /// Set metadata on the last node
    pub fn set_metadata(&mut self, key: &str, value: &str) {
        if let Some(node) = self.nodes.last_mut() {
            match key.to_lowercase().as_str() {
                "priority" => node.priority = Some(value.to_string()),
                "ticket" => node.ticket = Some(value.to_string()),
                "assigned" => node.assigned = Some(value.to_string()),
                "icon" => node.icon = Some(value.to_string()),
                "label" => node.label = value.to_string(),
                _ => {}
            }
        }
    }

    /// Get all sections (level 0 nodes)
    pub fn get_sections(&self) -> Vec<&KanbanNode> {
        self.nodes.iter().filter(|n| n.is_group).collect()
    }

    /// Get all nodes
    pub fn get_nodes(&self) -> &[KanbanNode] {
        &self.nodes
    }

    /// Get children of a section
    pub fn get_children(&self, section_id: &str) -> Vec<&KanbanNode> {
        self.nodes
            .iter()
            .filter(|n| n.parent_id.as_deref() == Some(section_id))
            .collect()
    }

    /// Get data for rendering
    pub fn get_data(&self) -> KanbanData {
        KanbanData {
            nodes: self.nodes.clone(),
        }
    }
}

/// Data structure for rendering
#[derive(Debug, Clone)]
pub struct KanbanData {
    pub nodes: Vec<KanbanNode>,
}

#[cfg(test)]
mod tests {
    use super::*;

    // ==================
    // Hierarchy tests
    // ==================

    #[test]
    fn test_knbn_1_simple_root_definition() {
        let mut db = KanbanDb::new();
        db.add_node(0, None, "root", NodeShape::Default);

        let sections = db.get_sections();
        assert_eq!(sections.len(), 1);
        assert_eq!(sections[0].label, "root");
    }

    #[test]
    fn test_knbn_2_hierarchical_kanban() {
        let mut db = KanbanDb::new();
        db.add_node(0, None, "root", NodeShape::Default);
        db.add_node(1, None, "child1", NodeShape::Default);
        db.add_node(1, None, "child2", NodeShape::Default);

        let sections = db.get_sections();
        assert_eq!(sections.len(), 1);
        assert_eq!(sections[0].label, "root");

        let children = db.get_children(&sections[0].id);
        assert_eq!(children.len(), 2);
        assert_eq!(children[0].label, "child1");
        assert_eq!(children[1].label, "child2");
    }

    #[test]
    fn test_knbn_3_root_with_shape() {
        let mut db = KanbanDb::new();
        db.add_node(0, None, "root", NodeShape::RoundedRect);

        let sections = db.get_sections();
        assert_eq!(sections[0].label, "root");
    }

    #[test]
    fn test_knbn_4_deeper_hierarchical_levels() {
        let mut db = KanbanDb::new();
        db.add_node(0, None, "root", NodeShape::Default);
        db.add_node(1, None, "child1", NodeShape::Default);
        db.add_node(2, None, "leaf1", NodeShape::Default);
        db.add_node(1, None, "child2", NodeShape::Default);

        let sections = db.get_sections();
        assert_eq!(sections.len(), 1);

        // All items at level > 0 are children of the section
        let children = db.get_children(&sections[0].id);
        assert_eq!(children.len(), 3);
    }

    #[test]
    fn test_knbn_5_multiple_sections() {
        let mut db = KanbanDb::new();
        db.add_node(0, None, "section1", NodeShape::Default);
        db.add_node(0, None, "section2", NodeShape::Default);

        let sections = db.get_sections();
        assert_eq!(sections.len(), 2);
        assert_eq!(sections[0].label, "section1");
        assert_eq!(sections[1].label, "section2");
    }

    // KNBN-6 test is skipped - the error detection is handled at parse time
    // rather than in the database. The mermaid.js test checks that parsing
    // fails when items appear without a section, which is a parser concern.

    // ==================
    // Node tests
    // ==================

    #[test]
    fn test_knbn_7_id_and_label() {
        let mut db = KanbanDb::new();
        db.add_node(0, Some("root"), "The root", NodeShape::Rect);

        let sections = db.get_sections();
        assert_eq!(sections[0].id, "root");
        assert_eq!(sections[0].label, "The root");
    }

    #[test]
    fn test_knbn_8_child_with_id_and_label() {
        let mut db = KanbanDb::new();
        db.add_node(0, None, "root", NodeShape::Default);
        db.add_node(1, Some("theId"), "child1", NodeShape::RoundedRect);

        let sections = db.get_sections();
        assert_eq!(sections[0].label, "root");

        let children = db.get_children(&sections[0].id);
        assert_eq!(children.len(), 1);
        assert_eq!(children[0].label, "child1");
        assert_eq!(children[0].id, "theId");
    }

    #[test]
    fn test_knbn_9_node_definition_variations() {
        let mut db = KanbanDb::new();
        db.add_node(0, None, "root", NodeShape::Default);
        db.add_node(1, Some("theId"), "child1", NodeShape::RoundedRect);

        let sections = db.get_sections();
        assert_eq!(sections[0].label, "root");

        let children = db.get_children(&sections[0].id);
        assert_eq!(children.len(), 1);
        assert_eq!(children[0].label, "child1");
        assert_eq!(children[0].id, "theId");
    }

    // ==================
    // Decoration tests
    // ==================

    #[test]
    fn test_knbn_13_set_icon() {
        let mut db = KanbanDb::new();
        db.add_node(0, Some("root"), "The root", NodeShape::Rect);
        db.decorate_icon("bomb");

        let sections = db.get_sections();
        assert_eq!(sections[0].id, "root");
        assert_eq!(sections[0].label, "The root");
        assert_eq!(sections[0].icon, Some("bomb".to_string()));
    }

    #[test]
    fn test_knbn_14_set_classes() {
        let mut db = KanbanDb::new();
        db.add_node(0, Some("root"), "The root", NodeShape::Rect);
        db.decorate_classes("m-4 p-8");

        let sections = db.get_sections();
        assert_eq!(sections[0].id, "root");
        assert_eq!(sections[0].label, "The root");
        assert_eq!(sections[0].css_classes, Some("m-4 p-8".to_string()));
    }

    #[test]
    fn test_knbn_15_set_classes_and_icon() {
        let mut db = KanbanDb::new();
        db.add_node(0, Some("root"), "The root", NodeShape::Rect);
        db.decorate_classes("m-4 p-8");
        db.decorate_icon("bomb");

        let sections = db.get_sections();
        assert_eq!(sections[0].id, "root");
        assert_eq!(sections[0].label, "The root");
        assert_eq!(sections[0].css_classes, Some("m-4 p-8".to_string()));
        assert_eq!(sections[0].icon, Some("bomb".to_string()));
    }

    #[test]
    fn test_knbn_16_set_icon_and_classes() {
        let mut db = KanbanDb::new();
        db.add_node(0, Some("root"), "The root", NodeShape::Rect);
        db.decorate_icon("bomb");
        db.decorate_classes("m-4 p-8");

        let sections = db.get_sections();
        assert_eq!(sections[0].id, "root");
        assert_eq!(sections[0].label, "The root");
        assert_eq!(sections[0].css_classes, Some("m-4 p-8".to_string()));
        assert_eq!(sections[0].icon, Some("bomb".to_string()));
    }

    // ==================
    // Description tests
    // ==================

    #[test]
    fn test_knbn_17_special_chars_in_label() {
        let mut db = KanbanDb::new();
        db.add_node(0, Some("root"), "String containing []", NodeShape::Rect);

        let sections = db.get_sections();
        assert_eq!(sections[0].id, "root");
        assert_eq!(sections[0].label, "String containing []");
    }

    #[test]
    fn test_knbn_18_special_chars_in_child() {
        let mut db = KanbanDb::new();
        db.add_node(0, Some("root"), "String containing []", NodeShape::Rect);
        db.add_node(1, Some("child1"), "String containing ()", NodeShape::Rect);

        let sections = db.get_sections();
        assert_eq!(sections[0].id, "root");
        assert_eq!(sections[0].label, "String containing []");

        let children = db.get_children(&sections[0].id);
        assert_eq!(children.len(), 1);
        assert_eq!(children[0].label, "String containing ()");
    }

    #[test]
    fn test_knbn_19_child_after_class() {
        let mut db = KanbanDb::new();
        db.add_node(0, Some("root"), "Root", NodeShape::RoundedRect);
        db.add_node(1, Some("Child"), "Child", NodeShape::RoundedRect);
        db.decorate_classes("hot");
        db.add_node(1, Some("a"), "a", NodeShape::RoundedRect);
        db.add_node(1, Some("b"), "New Stuff", NodeShape::Rect);

        let sections = db.get_sections();
        assert_eq!(sections[0].id, "root");
        assert_eq!(sections[0].label, "Root");

        let children = db.get_children(&sections[0].id);
        assert_eq!(children.len(), 3);
        assert_eq!(children[0].id, "Child");
        assert_eq!(children[1].id, "a");
        assert_eq!(children[2].id, "b");
    }

    // ==================
    // Whitespace/comment tests
    // ==================

    #[test]
    fn test_knbn_20_empty_rows() {
        let mut db = KanbanDb::new();
        db.add_node(0, Some("root"), "Root", NodeShape::RoundedRect);
        db.add_node(1, Some("Child"), "Child", NodeShape::RoundedRect);
        db.add_node(1, Some("a"), "a", NodeShape::RoundedRect);
        // Empty row would be here in input
        db.add_node(1, Some("b"), "New Stuff", NodeShape::Rect);

        let sections = db.get_sections();
        assert_eq!(sections[0].id, "root");
        assert_eq!(sections[0].label, "Root");

        let children = db.get_children(&sections[0].id);
        assert_eq!(children.len(), 3);
        assert_eq!(children[0].id, "Child");
        assert_eq!(children[1].id, "a");
        assert_eq!(children[2].id, "b");
    }

    #[test]
    fn test_knbn_21_comments() {
        // Comments are handled by parser, db just stores nodes
        let mut db = KanbanDb::new();
        db.add_node(0, Some("root"), "Root", NodeShape::RoundedRect);
        db.add_node(1, Some("Child"), "Child", NodeShape::RoundedRect);
        db.add_node(1, Some("a"), "a", NodeShape::RoundedRect);
        // %% This is a comment would be here
        db.add_node(1, Some("b"), "New Stuff", NodeShape::Rect);

        let sections = db.get_sections();
        let children = db.get_children(&sections[0].id);
        assert_eq!(children.len(), 3);
    }

    #[test]
    fn test_knbn_23_rows_with_spaces() {
        let mut db = KanbanDb::new();
        db.add_node(0, Some("root"), "root", NodeShape::Default);
        db.add_node(1, Some("A"), "A", NodeShape::Default);
        db.add_node(1, Some("B"), "B", NodeShape::Default);

        let sections = db.get_sections();
        assert_eq!(sections[0].id, "root");

        let children = db.get_children(&sections[0].id);
        assert_eq!(children.len(), 2);
        assert_eq!(children[0].id, "A");
        assert_eq!(children[1].id, "B");
    }

    // ==================
    // Metadata tests
    // ==================

    #[test]
    fn test_knbn_30_set_priority() {
        let mut db = KanbanDb::new();
        db.add_node(0, Some("root"), "root", NodeShape::Default);
        db.set_metadata("priority", "high");

        let sections = db.get_sections();
        assert_eq!(sections[0].id, "root");
        assert_eq!(sections[0].priority, Some("high".to_string()));
    }

    #[test]
    fn test_knbn_31_set_assigned() {
        let mut db = KanbanDb::new();
        db.add_node(0, Some("root"), "root", NodeShape::Default);
        db.set_metadata("assigned", "knsv");

        let sections = db.get_sections();
        assert_eq!(sections[0].id, "root");
        assert_eq!(sections[0].assigned, Some("knsv".to_string()));
    }

    #[test]
    fn test_knbn_32_set_icon_via_metadata() {
        let mut db = KanbanDb::new();
        db.add_node(0, Some("root"), "root", NodeShape::Default);
        db.set_metadata("icon", "star");

        let sections = db.get_sections();
        assert_eq!(sections[0].id, "root");
        assert_eq!(sections[0].icon, Some("star".to_string()));
    }

    #[test]
    fn test_knbn_34_multiple_metadata() {
        let mut db = KanbanDb::new();
        db.add_node(0, Some("root"), "root", NodeShape::Default);
        db.set_metadata("icon", "star");
        db.set_metadata("assigned", "knsv");

        let sections = db.get_sections();
        assert_eq!(sections[0].id, "root");
        assert_eq!(sections[0].icon, Some("star".to_string()));
        assert_eq!(sections[0].assigned, Some("knsv".to_string()));
    }

    #[test]
    fn test_knbn_36_set_label_via_metadata() {
        let mut db = KanbanDb::new();
        db.add_node(0, Some("root"), "root", NodeShape::Default);
        db.set_metadata("icon", "star");
        db.set_metadata("label", "fix things");

        let sections = db.get_sections();
        assert_eq!(sections[0].label, "fix things");
    }

    #[test]
    fn test_knbn_37_set_ticket() {
        let mut db = KanbanDb::new();
        db.add_node(0, Some("root"), "root", NodeShape::Default);
        db.set_metadata("ticket", "MC-1234");

        let sections = db.get_sections();
        assert_eq!(sections[0].id, "root");
        assert_eq!(sections[0].ticket, Some("MC-1234".to_string()));
    }

    // ==================
    // Clear test
    // ==================

    #[test]
    fn test_clear() {
        let mut db = KanbanDb::new();
        db.add_node(0, None, "section1", NodeShape::Default);
        db.add_node(1, None, "item1", NodeShape::Default);
        db.clear();

        assert!(db.get_sections().is_empty());
        assert!(db.get_nodes().is_empty());
    }
}
