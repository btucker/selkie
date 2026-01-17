//! Entity Relationship diagram types

use std::collections::HashMap;

pub use crate::diagrams::direction::Direction;

/// Cardinality types for ER relationships
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Cardinality {
    #[default]
    ZeroOrOne,
    ZeroOrMore,
    OneOrMore,
    OnlyOne,
    MdParent,
}

impl Cardinality {
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Self {
        match s.to_uppercase().as_str() {
            "ZERO_OR_ONE" | "ZERO OR ONE" | "|o" | "o|" => Self::ZeroOrOne,
            "ZERO_OR_MORE" | "ZERO OR MORE" | "}o" | "o{" => Self::ZeroOrMore,
            "ONE_OR_MORE" | "ONE OR MORE" | "}|" | "|{" => Self::OneOrMore,
            "ONLY_ONE" | "ONLY ONE" | "||" => Self::OnlyOne,
            "MD_PARENT" => Self::MdParent,
            _ => Self::ZeroOrOne,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::ZeroOrOne => "ZERO_OR_ONE",
            Self::ZeroOrMore => "ZERO_OR_MORE",
            Self::OneOrMore => "ONE_OR_MORE",
            Self::OnlyOne => "ONLY_ONE",
            Self::MdParent => "MD_PARENT",
        }
    }
}

/// Identification type for relationships (solid vs dashed line)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Identification {
    #[default]
    NonIdentifying,
    Identifying,
}

impl Identification {
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Self {
        match s.to_uppercase().as_str() {
            "IDENTIFYING" | "--" => Self::Identifying,
            _ => Self::NonIdentifying,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Identifying => "IDENTIFYING",
            Self::NonIdentifying => "NON_IDENTIFYING",
        }
    }
}

/// Attribute key types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttributeKey {
    PrimaryKey,
    ForeignKey,
    UniqueKey,
}

impl AttributeKey {
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_uppercase().as_str() {
            "PK" => Some(Self::PrimaryKey),
            "FK" => Some(Self::ForeignKey),
            "UK" => Some(Self::UniqueKey),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::PrimaryKey => "PK",
            Self::ForeignKey => "FK",
            Self::UniqueKey => "UK",
        }
    }
}

/// An attribute of an entity
#[derive(Debug, Clone)]
pub struct Attribute {
    pub attr_type: String,
    pub name: String,
    pub keys: Vec<AttributeKey>,
    pub comment: String,
}

impl Attribute {
    pub fn new(attr_type: String, name: String) -> Self {
        Self {
            attr_type,
            name,
            keys: Vec::new(),
            comment: String::new(),
        }
    }

    pub fn with_keys(mut self, keys: Vec<AttributeKey>) -> Self {
        self.keys = keys;
        self
    }

    pub fn with_comment(mut self, comment: String) -> Self {
        self.comment = comment;
        self
    }
}

/// Relationship specification (cardinality and identification)
#[derive(Debug, Clone)]
pub struct RelSpec {
    pub card_a: Cardinality,
    pub card_b: Cardinality,
    pub rel_type: Identification,
}

impl RelSpec {
    pub fn new(card_a: Cardinality, card_b: Cardinality, rel_type: Identification) -> Self {
        Self {
            card_a,
            card_b,
            rel_type,
        }
    }
}

/// An entity in the ER diagram
#[derive(Debug, Clone)]
pub struct Entity {
    pub id: String,
    pub label: String,
    pub attributes: Vec<Attribute>,
    pub alias: String,
    pub css_classes: String,
    pub css_styles: Vec<String>,
}

impl Entity {
    pub fn new(name: String, entity_count: usize) -> Self {
        Self {
            id: format!("entity-{}-{}", name, entity_count),
            label: name,
            attributes: Vec::new(),
            alias: String::new(),
            css_classes: "default".to_string(),
            css_styles: Vec::new(),
        }
    }
}

/// A relationship between entities
#[derive(Debug, Clone)]
pub struct Relationship {
    pub entity_a: String,
    pub role_a: String,
    pub entity_b: String,
    pub rel_spec: RelSpec,
}

impl Relationship {
    pub fn new(entity_a: String, role_a: String, entity_b: String, rel_spec: RelSpec) -> Self {
        Self {
            entity_a,
            role_a,
            entity_b,
            rel_spec,
        }
    }
}

/// Style class definition
#[derive(Debug, Clone)]
pub struct EntityClass {
    pub id: String,
    pub styles: Vec<String>,
    pub text_styles: Vec<String>,
}

impl EntityClass {
    pub fn new(id: String) -> Self {
        Self {
            id,
            styles: Vec::new(),
            text_styles: Vec::new(),
        }
    }
}

/// The ER diagram database
#[derive(Debug, Clone)]
pub struct ErDb {
    /// Entities in the diagram
    entities: HashMap<String, Entity>,
    /// Relationships between entities
    relationships: Vec<Relationship>,
    /// Style class definitions
    classes: HashMap<String, EntityClass>,
    /// Diagram direction
    direction: Direction,
    /// Accessibility title
    pub acc_title: String,
    /// Accessibility description
    pub acc_descr: String,
    /// Diagram title
    pub diagram_title: String,
}

impl Default for ErDb {
    fn default() -> Self {
        Self::new()
    }
}

impl ErDb {
    pub fn new() -> Self {
        Self {
            entities: HashMap::new(),
            relationships: Vec::new(),
            classes: HashMap::new(),
            direction: Direction::TopToBottom,
            acc_title: String::new(),
            acc_descr: String::new(),
            diagram_title: String::new(),
        }
    }

    pub fn clear(&mut self) {
        self.entities.clear();
        self.relationships.clear();
        self.classes.clear();
        self.direction = Direction::TopToBottom;
        self.acc_title.clear();
        self.acc_descr.clear();
        self.diagram_title.clear();
    }

    /// Add an entity to the diagram
    pub fn add_entity(&mut self, name: &str, alias: Option<&str>) -> &mut Entity {
        if !self.entities.contains_key(name) {
            let entity_count = self.entities.len();
            let mut entity = Entity::new(name.to_string(), entity_count);
            if let Some(a) = alias {
                entity.alias = a.to_string();
            }
            self.entities.insert(name.to_string(), entity);
        } else if let (Some(a), Some(entity)) = (alias, self.entities.get_mut(name)) {
            // Update alias if not already set
            if entity.alias.is_empty() {
                entity.alias = a.to_string();
            }
        }
        self.entities.get_mut(name).unwrap()
    }

    /// Get an entity by name
    pub fn get_entity(&self, name: &str) -> Option<&Entity> {
        self.entities.get(name)
    }

    /// Get a mutable entity by name
    pub fn get_entity_mut(&mut self, name: &str) -> Option<&mut Entity> {
        self.entities.get_mut(name)
    }

    /// Get all entities
    pub fn get_entities(&self) -> &HashMap<String, Entity> {
        &self.entities
    }

    /// Add attributes to an entity
    pub fn add_attributes(&mut self, entity_name: &str, attributes: Vec<Attribute>) {
        self.add_entity(entity_name, None);
        if let Some(entity) = self.entities.get_mut(entity_name) {
            entity.attributes.extend(attributes);
        }
    }

    /// Add a relationship between entities
    pub fn add_relationship(
        &mut self,
        entity_a: &str,
        role_a: &str,
        entity_b: &str,
        rel_spec: RelSpec,
    ) {
        // Only add relationship if both entities exist
        let entity_a_id = self.entities.get(entity_a).map(|e| e.id.clone());
        let entity_b_id = self.entities.get(entity_b).map(|e| e.id.clone());

        if let (Some(id_a), Some(id_b)) = (entity_a_id, entity_b_id) {
            let relationship = Relationship::new(id_a, role_a.to_string(), id_b, rel_spec);
            self.relationships.push(relationship);
        }
    }

    /// Get all relationships
    pub fn get_relationships(&self) -> &[Relationship] {
        &self.relationships
    }

    /// Set diagram direction
    pub fn set_direction(&mut self, dir: Direction) {
        self.direction = dir;
    }

    /// Get diagram direction
    pub fn get_direction(&self) -> Direction {
        self.direction
    }

    /// Add CSS styles to entities
    pub fn add_css_styles(&mut self, ids: &[&str], styles: &[&str]) {
        for id in ids {
            if let Some(entity) = self.entities.get_mut(*id) {
                for style in styles {
                    entity.css_styles.push(style.to_string());
                }
            }
        }
    }

    /// Add a style class definition
    pub fn add_class(&mut self, ids: &[&str], styles: &[&str]) {
        for id in ids {
            let class_node = self
                .classes
                .entry(id.to_string())
                .or_insert_with(|| EntityClass::new(id.to_string()));

            for style in styles {
                // If style contains "color", also add to textStyles with "fill" -> "bgFill"
                if style.contains("color") {
                    let new_style = style.replace("fill", "bgFill");
                    class_node.text_styles.push(new_style);
                }
                class_node.styles.push(style.to_string());
            }
        }
    }

    /// Apply a class to entities
    pub fn set_class(&mut self, ids: &[&str], class_names: &[&str]) {
        for id in ids {
            if let Some(entity) = self.entities.get_mut(*id) {
                for class_name in class_names {
                    entity.css_classes.push(' ');
                    entity.css_classes.push_str(class_name);
                }
            }
        }
    }

    /// Get all style classes
    pub fn get_classes(&self) -> &HashMap<String, EntityClass> {
        &self.classes
    }

    /// Get compiled styles for a set of class names
    pub fn get_compiled_styles(&self, class_defs: &[&str]) -> Vec<String> {
        let mut compiled_styles = Vec::new();
        for class_name in class_defs {
            if let Some(css_class) = self.classes.get(*class_name) {
                for style in &css_class.styles {
                    compiled_styles.push(style.trim().to_string());
                }
                for style in &css_class.text_styles {
                    compiled_styles.push(style.trim().to_string());
                }
            }
        }
        compiled_styles
    }
}
