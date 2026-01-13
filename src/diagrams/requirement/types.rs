//! Requirement diagram types
//!
//! Requirement diagrams show requirements, elements, and their relationships.

use std::collections::HashMap;

/// Requirement type
#[derive(Debug, Clone, PartialEq, Default)]
pub enum RequirementType {
    #[default]
    Requirement,
    FunctionalRequirement,
    InterfaceRequirement,
    PerformanceRequirement,
    PhysicalRequirement,
    DesignConstraint,
}

/// Risk level
#[derive(Debug, Clone, PartialEq, Default)]
pub enum RiskLevel {
    Low,
    #[default]
    Medium,
    High,
}

/// Verification method
#[derive(Debug, Clone, PartialEq, Default)]
pub enum VerifyType {
    Analysis,
    Demonstration,
    Inspection,
    #[default]
    Test,
}

/// Relationship type between requirements/elements
#[derive(Debug, Clone, PartialEq)]
pub enum RelationshipType {
    Contains,
    Copies,
    Derives,
    Satisfies,
    Verifies,
    Refines,
    Traces,
}

/// A requirement
#[derive(Debug, Clone, PartialEq)]
pub struct Requirement {
    pub name: String,
    pub req_type: RequirementType,
    pub requirement_id: String,
    pub text: String,
    pub risk: RiskLevel,
    pub verify_method: VerifyType,
    pub css_styles: Vec<String>,
    pub classes: Vec<String>,
}

impl Requirement {
    pub fn new(name: String, req_type: RequirementType) -> Self {
        Self {
            name,
            req_type,
            requirement_id: String::new(),
            text: String::new(),
            risk: RiskLevel::default(),
            verify_method: VerifyType::default(),
            css_styles: Vec::new(),
            classes: vec!["default".to_string()],
        }
    }
}

/// An element (design element that satisfies requirements)
#[derive(Debug, Clone, PartialEq)]
pub struct Element {
    pub name: String,
    pub element_type: String,
    pub doc_ref: String,
    pub css_styles: Vec<String>,
    pub classes: Vec<String>,
}

impl Element {
    pub fn new(name: String) -> Self {
        Self {
            name,
            element_type: String::new(),
            doc_ref: String::new(),
            css_styles: Vec::new(),
            classes: vec!["default".to_string()],
        }
    }
}

/// A relationship between requirements/elements
#[derive(Debug, Clone, PartialEq)]
pub struct Relation {
    pub rel_type: RelationshipType,
    pub src: String,
    pub dst: String,
}

/// A class definition for styling
#[derive(Debug, Clone, PartialEq)]
pub struct RequirementClass {
    pub id: String,
    pub styles: Vec<String>,
    pub text_styles: Vec<String>,
}

/// The Requirement database
#[derive(Debug, Clone, Default)]
pub struct RequirementDb {
    /// Requirements by name
    requirements: HashMap<String, Requirement>,
    /// Elements by name
    elements: HashMap<String, Element>,
    /// Relationships
    relations: Vec<Relation>,
    /// Class definitions
    classes: HashMap<String, RequirementClass>,
    /// Layout direction
    direction: String,
}

impl RequirementDb {
    /// Create a new empty RequirementDb
    pub fn new() -> Self {
        Self::default()
    }

    /// Clear all data
    pub fn clear(&mut self) {
        *self = Self::default();
    }

    /// Add a requirement
    pub fn add_requirement(&mut self, name: &str, req_type: &str) {
        let rtype = match req_type {
            "Functional Requirement" => RequirementType::FunctionalRequirement,
            "Interface Requirement" => RequirementType::InterfaceRequirement,
            "Performance Requirement" => RequirementType::PerformanceRequirement,
            "Physical Requirement" => RequirementType::PhysicalRequirement,
            "Design Constraint" => RequirementType::DesignConstraint,
            _ => RequirementType::Requirement,
        };
        let req = Requirement::new(name.to_string(), rtype);
        self.requirements.insert(name.to_string(), req);
    }

    /// Get all requirements
    pub fn get_requirements(&self) -> &HashMap<String, Requirement> {
        &self.requirements
    }

    /// Add an element
    pub fn add_element(&mut self, name: &str) {
        let elem = Element::new(name.to_string());
        self.elements.insert(name.to_string(), elem);
    }

    /// Get all elements
    pub fn get_elements(&self) -> &HashMap<String, Element> {
        &self.elements
    }

    /// Add a relationship
    pub fn add_relationship(&mut self, rel_type: &str, src: &str, dst: &str) {
        let rtype = match rel_type {
            "contains" => RelationshipType::Contains,
            "copies" => RelationshipType::Copies,
            "derives" => RelationshipType::Derives,
            "satisfies" => RelationshipType::Satisfies,
            "verifies" => RelationshipType::Verifies,
            "refines" => RelationshipType::Refines,
            "traces" => RelationshipType::Traces,
            _ => RelationshipType::Contains,
        };
        self.relations.push(Relation {
            rel_type: rtype,
            src: src.to_string(),
            dst: dst.to_string(),
        });
    }

    /// Get all relationships
    pub fn get_relationships(&self) -> &[Relation] {
        &self.relations
    }

    /// Define a class
    pub fn define_class(&mut self, ids: &[&str], styles: &[&str]) {
        for id in ids {
            let class = RequirementClass {
                id: id.to_string(),
                styles: styles.iter().map(|s| s.to_string()).collect(),
                text_styles: Vec::new(),
            };
            self.classes.insert(id.to_string(), class);
        }
    }

    /// Get all classes
    pub fn get_classes(&self) -> &HashMap<String, RequirementClass> {
        &self.classes
    }

    /// Set the direction
    pub fn set_direction(&mut self, direction: &str) {
        self.direction = direction.to_string();
    }

    /// Get the direction
    pub fn get_direction(&self) -> &str {
        &self.direction
    }

    /// Set requirement ID (on the most recently added requirement)
    pub fn set_req_id(&mut self, name: &str, id: &str) {
        if let Some(req) = self.requirements.get_mut(name) {
            req.requirement_id = id.to_string();
        }
    }

    /// Set requirement text (on the most recently added requirement)
    pub fn set_req_text(&mut self, name: &str, text: &str) {
        if let Some(req) = self.requirements.get_mut(name) {
            req.text = text.to_string();
        }
    }

    /// Set requirement risk level
    pub fn set_req_risk(&mut self, name: &str, risk: &str) {
        if let Some(req) = self.requirements.get_mut(name) {
            req.risk = match risk.to_lowercase().as_str() {
                "low" => RiskLevel::Low,
                "medium" => RiskLevel::Medium,
                "high" => RiskLevel::High,
                _ => RiskLevel::Medium,
            };
        }
    }

    /// Set requirement verify method
    pub fn set_req_verify_method(&mut self, name: &str, method: &str) {
        if let Some(req) = self.requirements.get_mut(name) {
            req.verify_method = match method.to_lowercase().as_str() {
                "analysis" => VerifyType::Analysis,
                "demonstration" => VerifyType::Demonstration,
                "inspection" => VerifyType::Inspection,
                "test" => VerifyType::Test,
                _ => VerifyType::Test,
            };
        }
    }

    /// Set element type
    pub fn set_element_type(&mut self, name: &str, elem_type: &str) {
        if let Some(elem) = self.elements.get_mut(name) {
            elem.element_type = elem_type.to_string();
        }
    }

    /// Set element doc reference
    pub fn set_element_docref(&mut self, name: &str, docref: &str) {
        if let Some(elem) = self.elements.get_mut(name) {
            elem.doc_ref = docref.to_string();
        }
    }

    /// Set CSS styles on requirements/elements
    pub fn set_css_style(&mut self, ids: &[&str], styles: &[&str]) {
        for id in ids {
            if let Some(req) = self.requirements.get_mut(*id) {
                req.css_styles = styles.iter().map(|s| s.to_string()).collect();
            }
            if let Some(elem) = self.elements.get_mut(*id) {
                elem.css_styles = styles.iter().map(|s| s.to_string()).collect();
            }
        }
    }

    /// Set classes on requirements/elements
    pub fn set_class(&mut self, ids: &[&str], class_names: &[&str]) {
        for id in ids {
            if let Some(req) = self.requirements.get_mut(*id) {
                for class_name in class_names {
                    if !req.classes.contains(&class_name.to_string()) {
                        req.classes.push(class_name.to_string());
                    }
                    // Apply class styles
                    if let Some(class_def) = self.classes.get(*class_name) {
                        req.css_styles = class_def.styles.clone();
                    }
                }
            }
            if let Some(elem) = self.elements.get_mut(*id) {
                for class_name in class_names {
                    if !elem.classes.contains(&class_name.to_string()) {
                        elem.classes.push(class_name.to_string());
                    }
                    // Apply class styles
                    if let Some(class_def) = self.classes.get(*class_name) {
                        elem.css_styles = class_def.styles.clone();
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_requirement() {
        let mut db = RequirementDb::new();
        db.add_requirement("requirement", "Requirement");
        let requirements = db.get_requirements();
        assert!(requirements.contains_key("requirement"));
    }

    #[test]
    fn test_add_element() {
        let mut db = RequirementDb::new();
        db.add_element("element");
        let elements = db.get_elements();
        assert!(elements.contains_key("element"));
    }

    #[test]
    fn test_add_relationship() {
        let mut db = RequirementDb::new();
        db.add_relationship("contains", "src", "dst");
        let relationships = db.get_relationships();
        let relationship = relationships.iter().find(|r| {
            matches!(r.rel_type, RelationshipType::Contains) && r.src == "src" && r.dst == "dst"
        });
        assert!(relationship.is_some());
    }

    #[test]
    fn test_define_single_class() {
        let mut db = RequirementDb::new();
        db.define_class(&["a"], &["stroke-width: 8px"]);
        let classes = db.get_classes();

        assert!(classes.contains_key("a"));
        assert_eq!(classes.get("a").unwrap().styles, vec!["stroke-width: 8px"]);
    }

    #[test]
    fn test_define_many_classes() {
        let mut db = RequirementDb::new();
        db.define_class(&["a", "b"], &["stroke-width: 8px"]);
        let classes = db.get_classes();

        assert!(classes.contains_key("a"));
        assert!(classes.contains_key("b"));
        assert_eq!(classes.get("a").unwrap().styles, vec!["stroke-width: 8px"]);
        assert_eq!(classes.get("b").unwrap().styles, vec!["stroke-width: 8px"]);
    }

    #[test]
    fn test_set_direction() {
        let mut db = RequirementDb::new();
        db.set_direction("TB");
        let direction = db.get_direction();

        assert_eq!(direction, "TB");
    }

    #[test]
    fn test_add_styles_to_requirement_and_element() {
        let mut db = RequirementDb::new();
        db.add_requirement("requirement", "Requirement");
        db.set_css_style(&["requirement"], &["color:red"]);
        db.add_element("element");
        db.set_css_style(&["element"], &["stroke-width:4px", "stroke: yellow"]);

        let requirement = db.get_requirements().get("requirement").unwrap();
        let element = db.get_elements().get("element").unwrap();

        assert_eq!(requirement.css_styles, vec!["color:red"]);
        assert_eq!(
            element.css_styles,
            vec!["stroke-width:4px", "stroke: yellow"]
        );
    }

    #[test]
    fn test_add_classes_to_requirement_and_element() {
        let mut db = RequirementDb::new();
        db.add_requirement("requirement", "Requirement");
        db.add_element("element");
        db.set_class(&["requirement", "element"], &["myClass"]);

        let requirement = db.get_requirements().get("requirement").unwrap();
        let element = db.get_elements().get("element").unwrap();

        assert_eq!(requirement.classes, vec!["default", "myClass"]);
        assert_eq!(element.classes, vec!["default", "myClass"]);
    }

    #[test]
    fn test_styles_inherited_from_class() {
        let mut db = RequirementDb::new();
        db.add_requirement("requirement", "Requirement");
        db.add_element("element");
        db.define_class(&["myClass"], &["color:red"]);
        db.define_class(&["myClass2"], &["stroke-width:4px", "stroke: yellow"]);
        db.set_class(&["requirement"], &["myClass"]);
        db.set_class(&["element"], &["myClass2"]);

        let requirement = db.get_requirements().get("requirement").unwrap();
        let element = db.get_elements().get("element").unwrap();

        assert_eq!(requirement.css_styles, vec!["color:red"]);
        assert_eq!(
            element.css_styles,
            vec!["stroke-width:4px", "stroke: yellow"]
        );
    }

    #[test]
    fn test_clear() {
        let mut db = RequirementDb::new();
        db.add_requirement("req", "Requirement");
        db.add_element("elem");
        db.clear();

        assert!(db.get_requirements().is_empty());
        assert!(db.get_elements().is_empty());
    }
}
