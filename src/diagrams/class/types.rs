//! Class diagram types

use std::collections::HashMap;

use crate::common::parse_generic_types;

/// CSS style for static members (underlined)
pub const STATIC_CSS_STYLE: &str = "text-decoration:underline;";
/// CSS style for abstract members (italic)
pub const ABSTRACT_CSS_STYLE: &str = "font-style:italic;";

/// Visibility modifiers for class members
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Visibility {
    #[default]
    None,
    Public,    // +
    Private,   // -
    Protected, // #
    Internal,  // ~
}

impl Visibility {
    /// Parse visibility from the first character
    pub fn from_char(c: char) -> Option<Self> {
        match c {
            '+' => Some(Self::Public),
            '-' => Some(Self::Private),
            '#' => Some(Self::Protected),
            '~' => Some(Self::Internal),
            _ => None,
        }
    }

    /// Get the display character for this visibility
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::None => "",
            Self::Public => "+",
            Self::Private => "-",
            Self::Protected => "#",
            Self::Internal => "~",
        }
    }
}

/// Classifier for static or abstract members
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Classifier {
    #[default]
    None,
    Static,   // $
    Abstract, // *
}

impl Classifier {
    /// Parse classifier from a character
    pub fn from_char(c: char) -> Option<Self> {
        match c {
            '$' => Some(Self::Static),
            '*' => Some(Self::Abstract),
            _ => None,
        }
    }

    /// Get the CSS style for this classifier
    pub fn css_style(&self) -> &'static str {
        match self {
            Self::None => "",
            Self::Static => STATIC_CSS_STYLE,
            Self::Abstract => ABSTRACT_CSS_STYLE,
        }
    }
}

/// Type of class member
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemberType {
    Method,
    Attribute,
}

/// Display details for a class member
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DisplayDetails {
    pub display_text: String,
    pub css_style: String,
}

/// A class member (method or attribute)
#[derive(Debug, Clone)]
pub struct ClassMember {
    /// The member identifier/name
    pub id: String,
    /// CSS style to apply
    pub css_style: String,
    /// Type of member (method or attribute)
    pub member_type: MemberType,
    /// Visibility modifier
    pub visibility: Visibility,
    /// Raw text representation
    pub text: String,
    /// Classifier (static/abstract)
    pub classifier: Classifier,
    /// Method parameters (empty for attributes)
    pub parameters: String,
    /// Return type for methods
    pub return_type: String,
}

impl ClassMember {
    /// Create a new class member from input string
    pub fn new(input: &str, member_type: MemberType) -> Self {
        let mut member = Self {
            id: String::new(),
            css_style: String::new(),
            member_type,
            visibility: Visibility::None,
            text: String::new(),
            classifier: Classifier::None,
            parameters: String::new(),
            return_type: String::new(),
        };
        member.parse_member(input);
        member
    }

    /// Get display details for rendering
    pub fn get_display_details(&self) -> DisplayDetails {
        let mut display_text = format!(
            "{}{}",
            self.visibility.as_str(),
            parse_generic_types(&self.id)
        );

        if self.member_type == MemberType::Method {
            display_text.push('(');
            display_text.push_str(&parse_generic_types(self.parameters.trim()));
            display_text.push(')');
            if !self.return_type.is_empty() {
                display_text.push_str(" : ");
                display_text.push_str(&parse_generic_types(&self.return_type));
            }
        }

        DisplayDetails {
            display_text: display_text.trim().to_string(),
            css_style: self.classifier.css_style().to_string(),
        }
    }

    /// Parse the member from input string
    fn parse_member(&mut self, input: &str) {
        let mut potential_classifier = Classifier::None;

        if self.member_type == MemberType::Method {
            // Regex: ([#+~-])?(.+)\((.*)\)([\s$*])?(.*)([$*])?
            // Match method pattern: visibility? name(params) classifier? return_type? classifier?
            if let Some(paren_start) = input.find('(') {
                if let Some(paren_end) = input.rfind(')') {
                    // Extract visibility from start
                    let (vis_offset, visibility) = if let Some(first_char) = input.chars().next() {
                        if let Some(v) = Visibility::from_char(first_char) {
                            (1, v)
                        } else {
                            (0, Visibility::None)
                        }
                    } else {
                        (0, Visibility::None)
                    };
                    self.visibility = visibility;

                    // Extract method name
                    self.id = input[vis_offset..paren_start].to_string();

                    // Extract parameters
                    self.parameters = input[paren_start + 1..paren_end].trim().to_string();

                    // Extract return type and classifier after )
                    let after_paren = &input[paren_end + 1..];
                    let trimmed = after_paren.trim();

                    if !trimmed.is_empty() {
                        // Check for classifier at start
                        let first = trimmed.chars().next().unwrap();
                        let (classifier_offset, classifier) =
                            if let Some(c) = Classifier::from_char(first) {
                                (1, c)
                            } else {
                                (0, Classifier::None)
                            };

                        if classifier != Classifier::None {
                            potential_classifier = classifier;
                            self.return_type = trimmed[classifier_offset..].trim().to_string();
                        } else {
                            self.return_type = trimmed.to_string();
                        }

                        // Check for classifier at end of return type
                        if potential_classifier == Classifier::None && !self.return_type.is_empty()
                        {
                            if let Some(last_char) = self.return_type.chars().last() {
                                if let Some(c) = Classifier::from_char(last_char) {
                                    potential_classifier = c;
                                    self.return_type.pop();
                                    self.return_type = self.return_type.trim().to_string();
                                }
                            }
                        }
                    }
                }
            }
        } else {
            // Attribute parsing
            let length = input.len();
            let first_char = input.chars().next();
            let last_char = input.chars().last();

            // Check visibility
            if let Some(c) = first_char {
                if let Some(v) = Visibility::from_char(c) {
                    self.visibility = v;
                }
            }

            // Check classifier at end
            if let Some(c) = last_char {
                if let Some(classifier) = Classifier::from_char(c) {
                    potential_classifier = classifier;
                }
            }

            // Extract id
            let start = if self.visibility != Visibility::None {
                1
            } else {
                0
            };
            let end = if potential_classifier != Classifier::None {
                length - 1
            } else {
                length
            };
            self.id = input[start..end].to_string();
        }

        self.classifier = potential_classifier;

        // Preserve one leading space only
        self.id = if self.id.starts_with(' ') {
            format!(" {}", self.id.trim())
        } else {
            self.id.trim().to_string()
        };

        // Build combined text (with HTML escaping)
        let combined = if self.member_type == MemberType::Method {
            let vis_str = if self.visibility != Visibility::None {
                format!("\\{}", self.visibility.as_str())
            } else {
                String::new()
            };
            let ret_str = if !self.return_type.is_empty() {
                format!(" : {}", parse_generic_types(&self.return_type))
            } else {
                String::new()
            };
            format!(
                "{}{}({}){}",
                vis_str,
                parse_generic_types(&self.id),
                parse_generic_types(&self.parameters),
                ret_str
            )
        } else {
            let vis_str = if self.visibility != Visibility::None {
                format!("\\{}", self.visibility.as_str())
            } else {
                String::new()
            };
            format!("{}{}", vis_str, parse_generic_types(&self.id))
        };

        self.text = combined.replace('<', "&lt;").replace('>', "&gt;");
        if self.text.starts_with("\\&lt;") {
            self.text = self.text.replacen("\\&lt;", "~", 1);
        }
    }
}

/// A class node in the diagram
#[derive(Debug, Clone)]
pub struct ClassNode {
    pub id: String,
    pub type_param: String,
    pub label: String,
    pub text: String,
    pub css_classes: String,
    pub methods: Vec<ClassMember>,
    pub members: Vec<ClassMember>,
    pub annotations: Vec<String>,
    pub dom_id: String,
    pub styles: Vec<String>,
    pub parent: Option<String>,
    pub link: Option<String>,
    pub link_target: Option<String>,
    pub have_callback: bool,
    pub tooltip: Option<String>,
    pub look: Option<String>,
}

impl ClassNode {
    pub fn new(id: String) -> Self {
        Self {
            id,
            type_param: String::new(),
            label: String::new(),
            text: String::new(),
            css_classes: String::new(),
            methods: Vec::new(),
            members: Vec::new(),
            annotations: Vec::new(),
            dom_id: String::new(),
            styles: Vec::new(),
            parent: None,
            link: None,
            link_target: None,
            have_callback: false,
            tooltip: None,
            look: None,
        }
    }
}

/// A note attached to a class
#[derive(Debug, Clone)]
pub struct ClassNote {
    pub id: String,
    pub class: String,
    pub text: String,
    pub index: usize,
    pub parent: Option<String>,
}

/// Relation end types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelationType {
    Aggregation = 0,
    Extension = 1,
    Composition = 2,
    Dependency = 3,
    Lollipop = 4,
}

/// Line types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineType {
    Solid = 0,
    Dotted = 1,
}

/// A relation between classes
#[derive(Debug, Clone)]
pub struct ClassRelation {
    pub id1: String,
    pub id2: String,
    pub relation_title1: String,
    pub relation_title2: String,
    pub relation_type: String,
    pub title: String,
    pub text: String,
    pub style: Vec<String>,
    pub relation: RelationDetails,
}

/// Details about a relation's endpoints and line type
#[derive(Debug, Clone)]
pub struct RelationDetails {
    pub type1: i32,
    pub type2: i32,
    pub line_type: LineType,
}

/// A namespace containing classes
#[derive(Debug, Clone)]
pub struct NamespaceNode {
    pub id: String,
    pub dom_id: String,
    pub classes: HashMap<String, ClassNode>,
    pub notes: HashMap<String, ClassNote>,
    pub children: HashMap<String, NamespaceNode>,
}

impl NamespaceNode {
    pub fn new(id: String) -> Self {
        Self {
            id,
            dom_id: String::new(),
            classes: HashMap::new(),
            notes: HashMap::new(),
            children: HashMap::new(),
        }
    }
}

/// A style class definition
#[derive(Debug, Clone)]
pub struct StyleClass {
    pub id: String,
    pub styles: Vec<String>,
    pub text_styles: Vec<String>,
}

/// The class diagram database
#[derive(Debug, Clone)]
pub struct ClassDb {
    pub classes: HashMap<String, ClassNode>,
    pub relations: Vec<ClassRelation>,
    pub notes: HashMap<String, ClassNote>,
    pub namespaces: HashMap<String, NamespaceNode>,
    pub style_classes: HashMap<String, StyleClass>,
    pub direction: String,
    pub acc_title: String,
    pub acc_descr: String,
    note_index: usize,
}

impl Default for ClassDb {
    fn default() -> Self {
        Self::new()
    }
}

impl ClassDb {
    pub fn new() -> Self {
        Self {
            classes: HashMap::new(),
            relations: Vec::new(),
            notes: HashMap::new(),
            namespaces: HashMap::new(),
            style_classes: HashMap::new(),
            direction: "TB".to_string(),
            acc_title: String::new(),
            acc_descr: String::new(),
            note_index: 0,
        }
    }

    pub fn clear(&mut self) {
        self.classes.clear();
        self.relations.clear();
        self.notes.clear();
        self.namespaces.clear();
        self.style_classes.clear();
        self.direction = "TB".to_string();
        self.acc_title.clear();
        self.acc_descr.clear();
        self.note_index = 0;
    }

    /// Add a class to the database
    pub fn add_class(&mut self, id: &str) {
        if !self.classes.contains_key(id) {
            self.classes
                .insert(id.to_string(), ClassNode::new(id.to_string()));
        }
    }

    /// Get a class by id
    pub fn get_class(&self, id: &str) -> Option<&ClassNode> {
        self.classes.get(id)
    }

    /// Get a mutable class by id
    pub fn get_class_mut(&mut self, id: &str) -> Option<&mut ClassNode> {
        self.classes.get_mut(id)
    }

    /// Add a relation
    pub fn add_relation(&mut self, relation: ClassRelation) {
        self.relations.push(relation);
    }

    /// Add a member to a class
    pub fn add_member(&mut self, class_name: &str, member: &str) {
        self.add_class(class_name);
        if let Some(class) = self.classes.get_mut(class_name) {
            // Determine if it's a method or attribute
            if member.contains('(') {
                class
                    .methods
                    .push(ClassMember::new(member, MemberType::Method));
            } else {
                class
                    .members
                    .push(ClassMember::new(member, MemberType::Attribute));
            }
        }
    }

    /// Add multiple members to a class
    pub fn add_members(&mut self, class_name: &str, members: &[&str]) {
        for member in members {
            self.add_member(class_name, member);
        }
    }

    /// Add an annotation to a class
    pub fn add_annotation(&mut self, class_name: &str, annotation: &str) {
        self.add_class(class_name);
        if let Some(class) = self.classes.get_mut(class_name) {
            class.annotations.push(annotation.to_string());
        }
    }

    /// Add a note
    pub fn add_note(&mut self, text: &str, class_name: &str) -> String {
        let id = format!("note{}", self.note_index);
        self.note_index += 1;
        let note = ClassNote {
            id: id.clone(),
            class: class_name.to_string(),
            text: text.to_string(),
            index: self.note_index - 1,
            parent: None,
        };
        self.notes.insert(id.clone(), note);
        id
    }

    /// Set CSS class on nodes
    pub fn set_css_class(&mut self, ids: &str, class_name: &str) {
        for id in ids.split(',') {
            let id = id.trim();
            self.add_class(id);
            if let Some(class) = self.classes.get_mut(id) {
                if !class.css_classes.is_empty() {
                    class.css_classes.push(' ');
                }
                class.css_classes.push_str(class_name);
            }
        }
    }

    /// Set inline CSS style on a node
    pub fn set_css_style(&mut self, id: &str, styles: Vec<String>) {
        self.add_class(id);
        if let Some(class) = self.classes.get_mut(id) {
            class.styles = styles;
        }
    }

    /// Set the diagram direction
    pub fn set_direction(&mut self, dir: &str) {
        self.direction = dir.to_string();
    }

    /// Add a namespace
    pub fn add_namespace(&mut self, id: &str) {
        if !self.namespaces.contains_key(id) {
            self.namespaces
                .insert(id.to_string(), NamespaceNode::new(id.to_string()));
        }
    }

    /// Set a link on class(es)
    pub fn set_link(&mut self, ids: &str, link: &str, target: &str) {
        for id in ids.split(',') {
            let id = id.trim();
            self.add_class(id);
            if let Some(class) = self.classes.get_mut(id) {
                class.link = Some(link.to_string());
                class.link_target = if target.is_empty() {
                    None
                } else {
                    Some(target.to_string())
                };
            }
        }
    }

    /// Set tooltip on class(es)
    pub fn set_tooltip(&mut self, ids: &str, tooltip: Option<&str>) {
        for id in ids.split(',') {
            let id = id.trim();
            self.add_class(id);
            if let Some(class) = self.classes.get_mut(id) {
                class.tooltip = tooltip.map(|s| s.to_string());
            }
        }
    }
}
