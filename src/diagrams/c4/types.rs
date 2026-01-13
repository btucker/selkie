//! C4 diagram types
//!
//! C4 diagrams show software architecture using the C4 model
//! (Context, Containers, Components, Code).

/// C4 element shape type
#[derive(Debug, Clone, PartialEq)]
pub enum C4ShapeType {
    Person,
    PersonExt,
    System,
    SystemDb,
    SystemQueue,
    SystemExt,
    SystemDbExt,
    SystemQueueExt,
    Container,
    ContainerDb,
    ContainerQueue,
    ContainerExt,
    ContainerDbExt,
    ContainerQueueExt,
    Component,
    ComponentDb,
    ComponentQueue,
    ComponentExt,
    ComponentDbExt,
    ComponentQueueExt,
}

/// A C4 element (Person, System, Container, Component)
#[derive(Debug, Clone, PartialEq)]
pub struct C4Element {
    pub alias: String,
    pub label: String,
    pub description: String,
    pub shape_type: C4ShapeType,
    pub technology: String,
    pub sprite: Option<String>,
    pub tags: Option<String>,
    pub link: Option<String>,
    pub parent_boundary: String,
}

impl C4Element {
    pub fn new_person(alias: String, label: String, description: String) -> Self {
        Self {
            alias,
            label,
            description,
            shape_type: C4ShapeType::Person,
            technology: String::new(),
            sprite: None,
            tags: None,
            link: None,
            parent_boundary: String::new(),
        }
    }

    pub fn new_system(alias: String, label: String, description: String) -> Self {
        Self {
            alias,
            label,
            description,
            shape_type: C4ShapeType::System,
            technology: String::new(),
            sprite: None,
            tags: None,
            link: None,
            parent_boundary: String::new(),
        }
    }

    pub fn new_container(
        alias: String,
        label: String,
        technology: String,
        description: String,
    ) -> Self {
        Self {
            alias,
            label,
            description,
            shape_type: C4ShapeType::Container,
            technology,
            sprite: None,
            tags: None,
            link: None,
            parent_boundary: String::new(),
        }
    }

    pub fn new_component(
        alias: String,
        label: String,
        technology: String,
        description: String,
    ) -> Self {
        Self {
            alias,
            label,
            description,
            shape_type: C4ShapeType::Component,
            technology,
            sprite: None,
            tags: None,
            link: None,
            parent_boundary: String::new(),
        }
    }
}

/// A C4 boundary (System or Container boundary)
#[derive(Debug, Clone, PartialEq)]
pub struct C4Boundary {
    pub alias: String,
    pub label: String,
    pub boundary_type: String,
    pub tags: Option<String>,
    pub link: Option<String>,
    pub parent_boundary: String,
}

/// A C4 relationship between elements
#[derive(Debug, Clone, PartialEq)]
pub struct C4Relationship {
    pub rel_type: String,
    pub from: String,
    pub to: String,
    pub label: String,
    pub technology: String,
    pub description: String,
    pub sprite: Option<String>,
    pub tags: Option<String>,
    pub link: Option<String>,
}

/// The C4 diagram database
#[derive(Debug, Clone, Default)]
pub struct C4Db {
    /// All elements (persons, systems, containers, components)
    elements: Vec<C4Element>,
    /// All boundaries
    boundaries: Vec<C4Boundary>,
    /// All relationships
    relationships: Vec<C4Relationship>,
    /// Current boundary stack for nesting
    boundary_stack: Vec<String>,
}

impl C4Db {
    /// Create a new empty C4Db
    pub fn new() -> Self {
        Self::default()
    }

    /// Clear all data
    pub fn clear(&mut self) {
        *self = Self::default();
    }

    /// Add a person element
    pub fn add_person(&mut self, alias: &str, label: &str, description: &str) {
        let mut elem = C4Element::new_person(
            alias.to_string(),
            label.to_string(),
            description.to_string(),
        );
        elem.parent_boundary = self.current_boundary();
        self.elements.push(elem);
    }

    /// Add a person element with specific shape type
    pub fn add_person_with_type(
        &mut self,
        alias: &str,
        label: &str,
        description: &str,
        shape_type: C4ShapeType,
    ) {
        let mut elem = C4Element::new_person(
            alias.to_string(),
            label.to_string(),
            description.to_string(),
        );
        elem.shape_type = shape_type;
        elem.parent_boundary = self.current_boundary();
        self.elements.push(elem);
    }

    /// Add a system element
    pub fn add_system(&mut self, alias: &str, label: &str, description: &str) {
        let mut elem = C4Element::new_system(
            alias.to_string(),
            label.to_string(),
            description.to_string(),
        );
        elem.parent_boundary = self.current_boundary();
        self.elements.push(elem);
    }

    /// Add a system element with specific shape type
    pub fn add_system_with_type(
        &mut self,
        alias: &str,
        label: &str,
        description: &str,
        shape_type: C4ShapeType,
    ) {
        let mut elem = C4Element::new_system(
            alias.to_string(),
            label.to_string(),
            description.to_string(),
        );
        elem.shape_type = shape_type;
        elem.parent_boundary = self.current_boundary();
        self.elements.push(elem);
    }

    /// Add a container element
    pub fn add_container(&mut self, alias: &str, label: &str, technology: &str, description: &str) {
        let mut elem = C4Element::new_container(
            alias.to_string(),
            label.to_string(),
            technology.to_string(),
            description.to_string(),
        );
        elem.parent_boundary = self.current_boundary();
        self.elements.push(elem);
    }

    /// Add a container element with specific shape type
    pub fn add_container_with_type(
        &mut self,
        alias: &str,
        label: &str,
        technology: &str,
        description: &str,
        shape_type: C4ShapeType,
    ) {
        let mut elem = C4Element::new_container(
            alias.to_string(),
            label.to_string(),
            technology.to_string(),
            description.to_string(),
        );
        elem.shape_type = shape_type;
        elem.parent_boundary = self.current_boundary();
        self.elements.push(elem);
    }

    /// Add a component element
    pub fn add_component(&mut self, alias: &str, label: &str, technology: &str, description: &str) {
        let mut elem = C4Element::new_component(
            alias.to_string(),
            label.to_string(),
            technology.to_string(),
            description.to_string(),
        );
        elem.parent_boundary = self.current_boundary();
        self.elements.push(elem);
    }

    /// Add a component element with specific shape type
    pub fn add_component_with_type(
        &mut self,
        alias: &str,
        label: &str,
        technology: &str,
        description: &str,
        shape_type: C4ShapeType,
    ) {
        let mut elem = C4Element::new_component(
            alias.to_string(),
            label.to_string(),
            technology.to_string(),
            description.to_string(),
        );
        elem.shape_type = shape_type;
        elem.parent_boundary = self.current_boundary();
        self.elements.push(elem);
    }

    /// Start a boundary
    pub fn start_boundary(&mut self, alias: &str, label: &str, boundary_type: &str) {
        let boundary = C4Boundary {
            alias: alias.to_string(),
            label: label.to_string(),
            boundary_type: boundary_type.to_string(),
            tags: None,
            link: None,
            parent_boundary: self.current_boundary(),
        };
        self.boundaries.push(boundary);
        self.boundary_stack.push(alias.to_string());
    }

    /// End current boundary
    pub fn end_boundary(&mut self) {
        self.boundary_stack.pop();
    }

    /// Get current boundary alias
    fn current_boundary(&self) -> String {
        self.boundary_stack.last().cloned().unwrap_or_default()
    }

    /// Add a relationship
    pub fn add_relationship(&mut self, from: &str, to: &str, label: &str, technology: &str) {
        let rel = C4Relationship {
            rel_type: "Rel".to_string(),
            from: from.to_string(),
            to: to.to_string(),
            label: label.to_string(),
            technology: technology.to_string(),
            description: String::new(),
            sprite: None,
            tags: None,
            link: None,
        };
        self.relationships.push(rel);
    }

    /// Get all elements
    pub fn get_elements(&self) -> &[C4Element] {
        &self.elements
    }

    /// Get all boundaries
    pub fn get_boundaries(&self) -> &[C4Boundary] {
        &self.boundaries
    }

    /// Get all relationships
    pub fn get_relationships(&self) -> &[C4Relationship] {
        &self.relationships
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_person() {
        let mut db = C4Db::new();
        db.add_person("user", "User", "A user of the system");

        let elements = db.get_elements();
        assert_eq!(elements.len(), 1);
        assert_eq!(elements[0].alias, "user");
        assert_eq!(elements[0].label, "User");
        assert_eq!(elements[0].shape_type, C4ShapeType::Person);
    }

    #[test]
    fn test_add_system() {
        let mut db = C4Db::new();
        db.add_system("sys", "System", "The main system");

        let elements = db.get_elements();
        assert_eq!(elements.len(), 1);
        assert_eq!(elements[0].alias, "sys");
        assert_eq!(elements[0].shape_type, C4ShapeType::System);
    }

    #[test]
    fn test_add_container() {
        let mut db = C4Db::new();
        db.add_container("api", "API", "Node.js", "REST API");

        let elements = db.get_elements();
        assert_eq!(elements.len(), 1);
        assert_eq!(elements[0].alias, "api");
        assert_eq!(elements[0].technology, "Node.js");
        assert_eq!(elements[0].shape_type, C4ShapeType::Container);
    }

    #[test]
    fn test_add_component() {
        let mut db = C4Db::new();
        db.add_component("auth", "Auth Service", "Python", "Handles authentication");

        let elements = db.get_elements();
        assert_eq!(elements.len(), 1);
        assert_eq!(elements[0].alias, "auth");
        assert_eq!(elements[0].shape_type, C4ShapeType::Component);
    }

    #[test]
    fn test_boundary_nesting() {
        let mut db = C4Db::new();
        db.start_boundary("system", "My System", "system");
        db.add_container("api", "API", "Node.js", "");
        db.end_boundary();

        let elements = db.get_elements();
        assert_eq!(elements[0].parent_boundary, "system");
    }

    #[test]
    fn test_add_relationship() {
        let mut db = C4Db::new();
        db.add_person("user", "User", "");
        db.add_system("sys", "System", "");
        db.add_relationship("user", "sys", "Uses", "HTTPS");

        let rels = db.get_relationships();
        assert_eq!(rels.len(), 1);
        assert_eq!(rels[0].from, "user");
        assert_eq!(rels[0].to, "sys");
        assert_eq!(rels[0].label, "Uses");
    }

    #[test]
    fn test_clear() {
        let mut db = C4Db::new();
        db.add_person("user", "User", "");
        db.clear();

        assert!(db.get_elements().is_empty());
    }
}
