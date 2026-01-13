//! Architecture diagram types
//!
//! Architecture diagrams show software architecture with services, groups, and connections.

use std::collections::HashMap;

/// Direction for edge connections
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ArchitectureDirection {
    Left,
    Right,
    Top,
    Bottom,
}

impl ArchitectureDirection {
    /// Parse a direction from a single character
    pub fn from_char(c: char) -> Option<Self> {
        match c.to_ascii_uppercase() {
            'L' => Some(Self::Left),
            'R' => Some(Self::Right),
            'T' => Some(Self::Top),
            'B' => Some(Self::Bottom),
            _ => None,
        }
    }

    /// Get the opposite direction
    pub fn opposite(&self) -> Self {
        match self {
            Self::Left => Self::Right,
            Self::Right => Self::Left,
            Self::Top => Self::Bottom,
            Self::Bottom => Self::Top,
        }
    }

    /// Check if this is an X-axis direction (Left or Right)
    pub fn is_x(&self) -> bool {
        matches!(self, Self::Left | Self::Right)
    }

    /// Check if this is a Y-axis direction (Top or Bottom)
    pub fn is_y(&self) -> bool {
        matches!(self, Self::Top | Self::Bottom)
    }

    /// Get short name (L, R, T, B)
    pub fn short_name(&self) -> char {
        match self {
            Self::Left => 'L',
            Self::Right => 'R',
            Self::Top => 'T',
            Self::Bottom => 'B',
        }
    }
}

/// Alignment between connected groups
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ArchitectureAlignment {
    Vertical,
    Horizontal,
    Bend,
}

/// Get alignment between two directions
pub fn get_direction_alignment(
    a: ArchitectureDirection,
    b: ArchitectureDirection,
) -> ArchitectureAlignment {
    if (a.is_x() && b.is_y()) || (a.is_y() && b.is_x()) {
        ArchitectureAlignment::Bend
    } else if a.is_x() {
        ArchitectureAlignment::Horizontal
    } else {
        ArchitectureAlignment::Vertical
    }
}

/// A direction pair for edges (source direction + target direction)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DirectionPair {
    pub source: ArchitectureDirection,
    pub target: ArchitectureDirection,
}

impl DirectionPair {
    /// Create a new direction pair, validating that they're not the same
    pub fn new(source: ArchitectureDirection, target: ArchitectureDirection) -> Option<Self> {
        // Invalid pairs: LL, RR, TT, BB
        if source == target {
            None
        } else {
            Some(Self { source, target })
        }
    }

    /// Get a string key for this pair (e.g., "LR", "TB")
    pub fn key(&self) -> String {
        format!("{}{}", self.source.short_name(), self.target.short_name())
    }

    /// Check if this is an XY pair (one X direction, one Y direction)
    pub fn is_xy(&self) -> bool {
        (self.source.is_x() && self.target.is_y()) || (self.source.is_y() && self.target.is_x())
    }

    /// Shift a position based on this direction pair
    pub fn shift_position(&self, x: i32, y: i32) -> (i32, i32) {
        if self.source.is_x() {
            let dx = if self.source == ArchitectureDirection::Left {
                -1
            } else {
                1
            };
            if self.target.is_y() {
                let dy = if self.target == ArchitectureDirection::Top {
                    1
                } else {
                    -1
                };
                (x + dx, y + dy)
            } else {
                (x + dx, y)
            }
        } else {
            let dy = if self.source == ArchitectureDirection::Top {
                1
            } else {
                -1
            };
            if self.target.is_x() {
                let dx = if self.target == ArchitectureDirection::Left {
                    1
                } else {
                    -1
                };
                (x + dx, y + dy)
            } else {
                (x, y + dy)
            }
        }
    }
}

/// An architecture service node
#[derive(Debug, Clone, PartialEq)]
pub struct ArchitectureService {
    pub id: String,
    pub icon: Option<String>,
    pub icon_text: Option<String>,
    pub title: Option<String>,
    /// Parent group ID
    pub parent: Option<String>,
}

impl ArchitectureService {
    pub fn new(id: String) -> Self {
        Self {
            id,
            icon: None,
            icon_text: None,
            title: None,
            parent: None,
        }
    }

    pub fn with_title(mut self, title: &str) -> Self {
        self.title = Some(title.to_string());
        self
    }

    pub fn with_icon(mut self, icon: &str) -> Self {
        self.icon = Some(icon.to_string());
        self
    }

    pub fn with_parent(mut self, parent: &str) -> Self {
        self.parent = Some(parent.to_string());
        self
    }
}

/// A junction node (for routing edges)
#[derive(Debug, Clone, PartialEq)]
pub struct ArchitectureJunction {
    pub id: String,
    /// Parent group ID
    pub parent: Option<String>,
}

impl ArchitectureJunction {
    pub fn new(id: String) -> Self {
        Self { id, parent: None }
    }

    pub fn with_parent(mut self, parent: &str) -> Self {
        self.parent = Some(parent.to_string());
        self
    }
}

/// A node can be either a service or a junction
#[derive(Debug, Clone, PartialEq)]
pub enum ArchitectureNode {
    Service(ArchitectureService),
    Junction(ArchitectureJunction),
}

impl ArchitectureNode {
    pub fn id(&self) -> &str {
        match self {
            Self::Service(s) => &s.id,
            Self::Junction(j) => &j.id,
        }
    }

    pub fn parent(&self) -> Option<&str> {
        match self {
            Self::Service(s) => s.parent.as_deref(),
            Self::Junction(j) => j.parent.as_deref(),
        }
    }

    pub fn is_service(&self) -> bool {
        matches!(self, Self::Service(_))
    }

    pub fn is_junction(&self) -> bool {
        matches!(self, Self::Junction(_))
    }
}

/// A group containing services or other groups
#[derive(Debug, Clone, PartialEq)]
pub struct ArchitectureGroup {
    pub id: String,
    pub icon: Option<String>,
    pub title: Option<String>,
    /// Parent group ID
    pub parent: Option<String>,
}

impl ArchitectureGroup {
    pub fn new(id: String) -> Self {
        Self {
            id,
            icon: None,
            title: None,
            parent: None,
        }
    }

    pub fn with_title(mut self, title: &str) -> Self {
        self.title = Some(title.to_string());
        self
    }

    pub fn with_icon(mut self, icon: &str) -> Self {
        self.icon = Some(icon.to_string());
        self
    }

    pub fn with_parent(mut self, parent: &str) -> Self {
        self.parent = Some(parent.to_string());
        self
    }
}

/// An edge connecting two nodes
#[derive(Debug, Clone, PartialEq)]
pub struct ArchitectureEdge {
    /// Left-hand side node ID
    pub lhs_id: String,
    /// Direction from LHS node
    pub lhs_dir: ArchitectureDirection,
    /// Arrow pointing into LHS
    pub lhs_into: bool,
    /// Edge traverses LHS group boundary
    pub lhs_group: bool,
    /// Right-hand side node ID
    pub rhs_id: String,
    /// Direction from RHS node
    pub rhs_dir: ArchitectureDirection,
    /// Arrow pointing into RHS
    pub rhs_into: bool,
    /// Edge traverses RHS group boundary
    pub rhs_group: bool,
    /// Edge label
    pub title: Option<String>,
}

impl ArchitectureEdge {
    pub fn new(
        lhs_id: String,
        lhs_dir: ArchitectureDirection,
        rhs_id: String,
        rhs_dir: ArchitectureDirection,
    ) -> Self {
        Self {
            lhs_id,
            lhs_dir,
            lhs_into: false,
            lhs_group: false,
            rhs_id,
            rhs_dir,
            rhs_into: false,
            rhs_group: false,
            title: None,
        }
    }

    pub fn with_title(mut self, title: &str) -> Self {
        self.title = Some(title.to_string());
        self
    }

    pub fn with_lhs_into(mut self) -> Self {
        self.lhs_into = true;
        self
    }

    pub fn with_rhs_into(mut self) -> Self {
        self.rhs_into = true;
        self
    }
}

/// Error types for architecture diagram operations
#[derive(Debug, Clone, PartialEq)]
pub enum ArchitectureError {
    /// ID already in use
    DuplicateId { id: String, used_by: String },
    /// Cannot place element within itself
    SelfReference { id: String },
    /// Parent does not exist
    ParentNotFound { id: String, parent: String },
    /// Parent is not a group
    ParentNotGroup { id: String, parent: String },
    /// Node not found for edge
    NodeNotFound { id: String },
    /// Invalid direction
    InvalidDirection {
        lhs_id: String,
        rhs_id: String,
        dir: String,
    },
    /// Group boundary traversal error
    InvalidGroupBoundary { id: String },
}

impl std::fmt::Display for ArchitectureError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ArchitectureError::DuplicateId { id, used_by } => {
                write!(
                    f,
                    "The id [{}] is already in use by another {}",
                    id, used_by
                )
            }
            ArchitectureError::SelfReference { id } => {
                write!(f, "The element [{}] cannot be placed within itself", id)
            }
            ArchitectureError::ParentNotFound { id, parent } => {
                write!(
                    f,
                    "The element [{}]'s parent [{}] does not exist",
                    id, parent
                )
            }
            ArchitectureError::ParentNotGroup { id, parent } => {
                write!(
                    f,
                    "The element [{}]'s parent [{}] is not a group",
                    id, parent
                )
            }
            ArchitectureError::NodeNotFound { id } => {
                write!(f, "The node [{}] does not exist", id)
            }
            ArchitectureError::InvalidDirection {
                lhs_id,
                rhs_id,
                dir,
            } => {
                write!(
                    f,
                    "Invalid direction [{}] for edge {}--{}",
                    dir, lhs_id, rhs_id
                )
            }
            ArchitectureError::InvalidGroupBoundary { id } => {
                write!(f, "The id [{}] has invalid group boundary modifier", id)
            }
        }
    }
}

impl std::error::Error for ArchitectureError {}

/// Registry entry type
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RegistryEntry {
    Node,
    Group,
}

/// The Architecture diagram database
#[derive(Debug, Clone, Default)]
pub struct ArchitectureDb {
    /// Diagram title
    title: String,
    /// Accessibility title
    acc_title: String,
    /// Accessibility description
    acc_description: String,
    /// All nodes by ID
    nodes: HashMap<String, ArchitectureNode>,
    /// All groups by ID
    groups: HashMap<String, ArchitectureGroup>,
    /// All edges
    edges: Vec<ArchitectureEdge>,
    /// Registry of all IDs (tracks if node or group)
    registry: HashMap<String, RegistryEntry>,
}

impl ArchitectureDb {
    /// Create a new empty ArchitectureDb
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

    /// Set accessibility title
    pub fn set_acc_title(&mut self, title: &str) {
        self.acc_title = title.to_string();
    }

    /// Get accessibility title
    pub fn get_acc_title(&self) -> &str {
        &self.acc_title
    }

    /// Set accessibility description
    pub fn set_acc_description(&mut self, desc: &str) {
        self.acc_description = desc.to_string();
    }

    /// Get accessibility description
    pub fn get_acc_description(&self) -> &str {
        &self.acc_description
    }

    /// Add a service
    pub fn add_service(&mut self, service: ArchitectureService) -> Result<(), ArchitectureError> {
        let id = service.id.clone();

        // Check for duplicate ID
        if let Some(entry) = self.registry.get(&id) {
            return Err(ArchitectureError::DuplicateId {
                id,
                used_by: match entry {
                    RegistryEntry::Node => "node".to_string(),
                    RegistryEntry::Group => "group".to_string(),
                },
            });
        }

        // Check parent constraints
        if let Some(ref parent) = service.parent {
            if &id == parent {
                return Err(ArchitectureError::SelfReference { id });
            }
            match self.registry.get(parent) {
                None => {
                    return Err(ArchitectureError::ParentNotFound {
                        id,
                        parent: parent.clone(),
                    })
                }
                Some(RegistryEntry::Node) => {
                    return Err(ArchitectureError::ParentNotGroup {
                        id,
                        parent: parent.clone(),
                    })
                }
                Some(RegistryEntry::Group) => {}
            }
        }

        self.registry.insert(id.clone(), RegistryEntry::Node);
        self.nodes.insert(id, ArchitectureNode::Service(service));
        Ok(())
    }

    /// Add a junction
    pub fn add_junction(
        &mut self,
        junction: ArchitectureJunction,
    ) -> Result<(), ArchitectureError> {
        let id = junction.id.clone();

        // Check for duplicate ID
        if let Some(entry) = self.registry.get(&id) {
            return Err(ArchitectureError::DuplicateId {
                id,
                used_by: match entry {
                    RegistryEntry::Node => "node".to_string(),
                    RegistryEntry::Group => "group".to_string(),
                },
            });
        }

        // Check parent constraints
        if let Some(ref parent) = junction.parent {
            if &id == parent {
                return Err(ArchitectureError::SelfReference { id });
            }
            match self.registry.get(parent) {
                None => {
                    return Err(ArchitectureError::ParentNotFound {
                        id,
                        parent: parent.clone(),
                    })
                }
                Some(RegistryEntry::Node) => {
                    return Err(ArchitectureError::ParentNotGroup {
                        id,
                        parent: parent.clone(),
                    })
                }
                Some(RegistryEntry::Group) => {}
            }
        }

        self.registry.insert(id.clone(), RegistryEntry::Node);
        self.nodes.insert(id, ArchitectureNode::Junction(junction));
        Ok(())
    }

    /// Add a group
    pub fn add_group(&mut self, group: ArchitectureGroup) -> Result<(), ArchitectureError> {
        let id = group.id.clone();

        // Check for duplicate ID
        if let Some(entry) = self.registry.get(&id) {
            return Err(ArchitectureError::DuplicateId {
                id,
                used_by: match entry {
                    RegistryEntry::Node => "node".to_string(),
                    RegistryEntry::Group => "group".to_string(),
                },
            });
        }

        // Check parent constraints
        if let Some(ref parent) = group.parent {
            if &id == parent {
                return Err(ArchitectureError::SelfReference { id });
            }
            match self.registry.get(parent) {
                None => {
                    return Err(ArchitectureError::ParentNotFound {
                        id,
                        parent: parent.clone(),
                    })
                }
                Some(RegistryEntry::Node) => {
                    return Err(ArchitectureError::ParentNotGroup {
                        id,
                        parent: parent.clone(),
                    })
                }
                Some(RegistryEntry::Group) => {}
            }
        }

        self.registry.insert(id.clone(), RegistryEntry::Group);
        self.groups.insert(id, group);
        Ok(())
    }

    /// Add an edge
    pub fn add_edge(&mut self, edge: ArchitectureEdge) -> Result<(), ArchitectureError> {
        // Validate that both nodes exist
        if !self.nodes.contains_key(&edge.lhs_id) && !self.groups.contains_key(&edge.lhs_id) {
            return Err(ArchitectureError::NodeNotFound {
                id: edge.lhs_id.clone(),
            });
        }
        if !self.nodes.contains_key(&edge.rhs_id) && !self.groups.contains_key(&edge.rhs_id) {
            return Err(ArchitectureError::NodeNotFound {
                id: edge.rhs_id.clone(),
            });
        }

        // Validate group boundary constraints
        let lhs_group_id = self.nodes.get(&edge.lhs_id).and_then(|n| n.parent());
        let rhs_group_id = self.nodes.get(&edge.rhs_id).and_then(|n| n.parent());

        if edge.lhs_group {
            if let (Some(lhs_g), Some(rhs_g)) = (lhs_group_id, rhs_group_id) {
                if lhs_g == rhs_g {
                    return Err(ArchitectureError::InvalidGroupBoundary {
                        id: edge.lhs_id.clone(),
                    });
                }
            }
        }

        if edge.rhs_group {
            if let (Some(lhs_g), Some(rhs_g)) = (lhs_group_id, rhs_group_id) {
                if lhs_g == rhs_g {
                    return Err(ArchitectureError::InvalidGroupBoundary {
                        id: edge.rhs_id.clone(),
                    });
                }
            }
        }

        self.edges.push(edge);
        Ok(())
    }

    /// Get all services
    pub fn get_services(&self) -> Vec<&ArchitectureService> {
        self.nodes
            .values()
            .filter_map(|n| match n {
                ArchitectureNode::Service(s) => Some(s),
                _ => None,
            })
            .collect()
    }

    /// Get all junctions
    pub fn get_junctions(&self) -> Vec<&ArchitectureJunction> {
        self.nodes
            .values()
            .filter_map(|n| match n {
                ArchitectureNode::Junction(j) => Some(j),
                _ => None,
            })
            .collect()
    }

    /// Get all nodes
    pub fn get_nodes(&self) -> Vec<&ArchitectureNode> {
        self.nodes.values().collect()
    }

    /// Get a node by ID
    pub fn get_node(&self, id: &str) -> Option<&ArchitectureNode> {
        self.nodes.get(id)
    }

    /// Get all groups
    pub fn get_groups(&self) -> Vec<&ArchitectureGroup> {
        self.groups.values().collect()
    }

    /// Get all edges
    pub fn get_edges(&self) -> &[ArchitectureEdge] {
        &self.edges
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_db() {
        let db = ArchitectureDb::new();
        assert!(db.get_nodes().is_empty());
        assert!(db.get_groups().is_empty());
        assert!(db.get_edges().is_empty());
    }

    #[test]
    fn test_set_title() {
        let mut db = ArchitectureDb::new();
        db.set_title("Simple Architecture Diagram");
        assert_eq!(db.get_title(), "Simple Architecture Diagram");
    }

    #[test]
    fn test_set_acc_title() {
        let mut db = ArchitectureDb::new();
        db.set_acc_title("Accessibility Title");
        assert_eq!(db.get_acc_title(), "Accessibility Title");
    }

    #[test]
    fn test_set_acc_description() {
        let mut db = ArchitectureDb::new();
        db.set_acc_description("Accessibility Description");
        assert_eq!(db.get_acc_description(), "Accessibility Description");
    }

    #[test]
    fn test_add_service() {
        let mut db = ArchitectureDb::new();
        let service = ArchitectureService::new("db".to_string()).with_title("Database");
        db.add_service(service).unwrap();

        let services = db.get_services();
        assert_eq!(services.len(), 1);
        assert_eq!(services[0].id, "db");
        assert_eq!(services[0].title, Some("Database".to_string()));
    }

    #[test]
    fn test_add_junction() {
        let mut db = ArchitectureDb::new();
        let junction = ArchitectureJunction::new("junc1".to_string());
        db.add_junction(junction).unwrap();

        let junctions = db.get_junctions();
        assert_eq!(junctions.len(), 1);
        assert_eq!(junctions[0].id, "junc1");
    }

    #[test]
    fn test_add_group() {
        let mut db = ArchitectureDb::new();
        let group = ArchitectureGroup::new("cloud".to_string()).with_title("Cloud Services");
        db.add_group(group).unwrap();

        let groups = db.get_groups();
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].id, "cloud");
        assert_eq!(groups[0].title, Some("Cloud Services".to_string()));
    }

    #[test]
    fn test_service_in_group() {
        let mut db = ArchitectureDb::new();

        let group = ArchitectureGroup::new("cloud".to_string());
        db.add_group(group).unwrap();

        let service = ArchitectureService::new("db".to_string()).with_parent("cloud");
        db.add_service(service).unwrap();

        let services = db.get_services();
        assert_eq!(services[0].parent, Some("cloud".to_string()));
    }

    #[test]
    fn test_duplicate_id_error() {
        let mut db = ArchitectureDb::new();

        let service = ArchitectureService::new("db".to_string());
        db.add_service(service).unwrap();

        let duplicate = ArchitectureService::new("db".to_string());
        let result = db.add_service(duplicate);

        assert!(matches!(result, Err(ArchitectureError::DuplicateId { .. })));
    }

    #[test]
    fn test_self_reference_error() {
        let mut db = ArchitectureDb::new();

        let group = ArchitectureGroup::new("cloud".to_string()).with_parent("cloud");
        let result = db.add_group(group);

        assert!(matches!(
            result,
            Err(ArchitectureError::SelfReference { .. })
        ));
    }

    #[test]
    fn test_parent_not_found_error() {
        let mut db = ArchitectureDb::new();

        let service = ArchitectureService::new("db".to_string()).with_parent("nonexistent");
        let result = db.add_service(service);

        assert!(matches!(
            result,
            Err(ArchitectureError::ParentNotFound { .. })
        ));
    }

    #[test]
    fn test_parent_not_group_error() {
        let mut db = ArchitectureDb::new();

        let service1 = ArchitectureService::new("db".to_string());
        db.add_service(service1).unwrap();

        let service2 = ArchitectureService::new("api".to_string()).with_parent("db");
        let result = db.add_service(service2);

        assert!(matches!(
            result,
            Err(ArchitectureError::ParentNotGroup { .. })
        ));
    }

    #[test]
    fn test_add_edge() {
        let mut db = ArchitectureDb::new();

        let s1 = ArchitectureService::new("db".to_string());
        let s2 = ArchitectureService::new("api".to_string());
        db.add_service(s1).unwrap();
        db.add_service(s2).unwrap();

        let edge = ArchitectureEdge::new(
            "db".to_string(),
            ArchitectureDirection::Right,
            "api".to_string(),
            ArchitectureDirection::Left,
        );
        db.add_edge(edge).unwrap();

        let edges = db.get_edges();
        assert_eq!(edges.len(), 1);
        assert_eq!(edges[0].lhs_id, "db");
        assert_eq!(edges[0].rhs_id, "api");
    }

    #[test]
    fn test_edge_node_not_found() {
        let mut db = ArchitectureDb::new();

        let s1 = ArchitectureService::new("db".to_string());
        db.add_service(s1).unwrap();

        let edge = ArchitectureEdge::new(
            "db".to_string(),
            ArchitectureDirection::Right,
            "nonexistent".to_string(),
            ArchitectureDirection::Left,
        );
        let result = db.add_edge(edge);

        assert!(matches!(
            result,
            Err(ArchitectureError::NodeNotFound { .. })
        ));
    }

    #[test]
    fn test_direction_from_char() {
        assert_eq!(
            ArchitectureDirection::from_char('L'),
            Some(ArchitectureDirection::Left)
        );
        assert_eq!(
            ArchitectureDirection::from_char('r'),
            Some(ArchitectureDirection::Right)
        );
        assert_eq!(
            ArchitectureDirection::from_char('T'),
            Some(ArchitectureDirection::Top)
        );
        assert_eq!(
            ArchitectureDirection::from_char('b'),
            Some(ArchitectureDirection::Bottom)
        );
        assert_eq!(ArchitectureDirection::from_char('X'), None);
    }

    #[test]
    fn test_direction_opposite() {
        assert_eq!(
            ArchitectureDirection::Left.opposite(),
            ArchitectureDirection::Right
        );
        assert_eq!(
            ArchitectureDirection::Right.opposite(),
            ArchitectureDirection::Left
        );
        assert_eq!(
            ArchitectureDirection::Top.opposite(),
            ArchitectureDirection::Bottom
        );
        assert_eq!(
            ArchitectureDirection::Bottom.opposite(),
            ArchitectureDirection::Top
        );
    }

    #[test]
    fn test_direction_pair() {
        let pair = DirectionPair::new(ArchitectureDirection::Left, ArchitectureDirection::Right);
        assert!(pair.is_some());
        assert_eq!(pair.unwrap().key(), "LR");

        // Same direction is invalid
        let invalid = DirectionPair::new(ArchitectureDirection::Left, ArchitectureDirection::Left);
        assert!(invalid.is_none());
    }

    #[test]
    fn test_direction_pair_is_xy() {
        let xy =
            DirectionPair::new(ArchitectureDirection::Left, ArchitectureDirection::Top).unwrap();
        assert!(xy.is_xy());

        let not_xy =
            DirectionPair::new(ArchitectureDirection::Left, ArchitectureDirection::Right).unwrap();
        assert!(!not_xy.is_xy());
    }

    #[test]
    fn test_get_direction_alignment() {
        assert_eq!(
            get_direction_alignment(ArchitectureDirection::Left, ArchitectureDirection::Right),
            ArchitectureAlignment::Horizontal
        );
        assert_eq!(
            get_direction_alignment(ArchitectureDirection::Top, ArchitectureDirection::Bottom),
            ArchitectureAlignment::Vertical
        );
        assert_eq!(
            get_direction_alignment(ArchitectureDirection::Left, ArchitectureDirection::Top),
            ArchitectureAlignment::Bend
        );
    }

    #[test]
    fn test_clear() {
        let mut db = ArchitectureDb::new();
        db.set_title("Test");
        let service = ArchitectureService::new("db".to_string());
        db.add_service(service).unwrap();

        db.clear();

        assert_eq!(db.get_title(), "");
        assert!(db.get_nodes().is_empty());
    }
}
