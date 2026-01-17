//! State diagram types

use std::collections::HashMap;

use crate::common::remove_script;
pub use crate::diagrams::direction::Direction;

/// State types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StateType {
    #[default]
    Default,
    Start,
    End,
    Fork,
    Join,
    Choice,
    Divider,
}

impl StateType {
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "start" => Self::Start,
            "end" => Self::End,
            "fork" => Self::Fork,
            "join" => Self::Join,
            "choice" => Self::Choice,
            "divider" => Self::Divider,
            _ => Self::Default,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Default => "default",
            Self::Start => "start",
            Self::End => "end",
            Self::Fork => "fork",
            Self::Join => "join",
            Self::Choice => "choice",
            Self::Divider => "divider",
        }
    }
}

/// Note position
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum NotePosition {
    #[default]
    RightOf,
    LeftOf,
}

impl NotePosition {
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "left of" | "leftof" => Self::LeftOf,
            _ => Self::RightOf,
        }
    }
}

/// A note attached to a state
#[derive(Debug, Clone)]
pub struct Note {
    pub position: NotePosition,
    pub text: String,
}

impl Note {
    pub fn new(position: NotePosition, text: String) -> Self {
        Self { position, text }
    }
}

/// A state in the diagram
#[derive(Debug, Clone)]
pub struct State {
    pub id: String,
    pub state_type: StateType,
    pub description: Option<String>,
    pub descriptions: Vec<String>,
    pub note: Option<Note>,
    pub classes: Vec<String>,
    pub styles: Vec<String>,
    pub text_styles: Vec<String>,
    /// Nested states for composite states
    pub doc: Vec<Statement>,
    /// Alias for display
    pub alias: Option<String>,
    /// Parent state ID for nested states
    pub parent: Option<String>,
}

impl State {
    pub fn new(id: String) -> Self {
        Self {
            id,
            state_type: StateType::Default,
            description: None,
            descriptions: Vec::new(),
            note: None,
            classes: Vec::new(),
            styles: Vec::new(),
            text_styles: Vec::new(),
            doc: Vec::new(),
            alias: None,
            parent: None,
        }
    }

    pub fn with_type(id: String, state_type: StateType) -> Self {
        let mut state = Self::new(id);
        state.state_type = state_type;
        state
    }
}

/// A relation/transition between states
#[derive(Debug, Clone)]
pub struct Relation {
    pub state1: String,
    pub state2: String,
    pub description: Option<String>,
}

impl Relation {
    pub fn new(state1: String, state2: String) -> Self {
        Self {
            state1,
            state2,
            description: None,
        }
    }
}

/// Style class definition
#[derive(Debug, Clone)]
pub struct StyleClass {
    pub id: String,
    pub styles: Vec<String>,
    pub text_styles: Vec<String>,
}

impl StyleClass {
    pub fn new(id: String) -> Self {
        Self {
            id,
            styles: Vec::new(),
            text_styles: Vec::new(),
        }
    }
}

/// Statement types in the state diagram
#[derive(Debug, Clone)]
pub enum Statement {
    State(State),
    Relation(Relation),
    ClassDef { id: String, classes: String },
    ApplyClass { id: String, style_class: String },
    Style { id: String, style_class: String },
    Direction(Direction),
}

/// The state diagram database
#[derive(Debug, Clone)]
pub struct StateDb {
    /// States in the diagram
    states: HashMap<String, State>,
    /// Relations between states
    relations: Vec<Relation>,
    /// Style class definitions
    classes: HashMap<String, StyleClass>,
    /// The root document statements
    root_doc: Vec<Statement>,
    /// Diagram direction
    direction: Direction,
    /// Divider counter for unique IDs
    divider_cnt: usize,
    /// Start state counter for unique IDs
    start_cnt: usize,
    /// End state counter for unique IDs
    end_cnt: usize,
    /// Accessibility title
    pub acc_title: String,
    /// Accessibility description
    pub acc_descr: String,
    /// Diagram title
    pub diagram_title: String,
    /// Hide empty descriptions flag
    pub hide_empty_descriptions: bool,
}

impl Default for StateDb {
    fn default() -> Self {
        Self::new()
    }
}

impl StateDb {
    pub fn new() -> Self {
        Self {
            states: HashMap::new(),
            relations: Vec::new(),
            classes: HashMap::new(),
            root_doc: Vec::new(),
            direction: Direction::TopToBottom,
            divider_cnt: 0,
            start_cnt: 0,
            end_cnt: 0,
            acc_title: String::new(),
            acc_descr: String::new(),
            diagram_title: String::new(),
            hide_empty_descriptions: false,
        }
    }

    pub fn clear(&mut self) {
        self.states.clear();
        self.relations.clear();
        self.classes.clear();
        self.root_doc.clear();
        self.direction = Direction::TopToBottom;
        self.divider_cnt = 0;
        self.start_cnt = 0;
        self.end_cnt = 0;
        self.acc_title.clear();
        self.acc_descr.clear();
        self.diagram_title.clear();
        self.hide_empty_descriptions = false;
    }

    /// Set hide empty descriptions flag
    pub fn set_hide_empty_descriptions(&mut self, value: bool) {
        self.hide_empty_descriptions = value;
    }

    /// Set the accessibility title
    pub fn set_acc_title(&mut self, title: &str) {
        self.acc_title = title.to_string();
    }

    /// Set the accessibility description
    pub fn set_acc_description(&mut self, desc: &str) {
        self.acc_descr = desc.to_string();
    }

    /// Set the parent of a state
    pub fn set_parent(&mut self, state_id: &str, parent_id: &str) {
        self.add_state(state_id);
        if let Some(state) = self.states.get_mut(state_id) {
            state.parent = Some(parent_id.to_string());
        }
    }

    /// Add a state to the diagram
    pub fn add_state(&mut self, id: &str) {
        if !self.states.contains_key(id) {
            // Check for special state types
            let state = if id == "[*]" {
                // Could be start or end depending on context
                State::new(id.to_string())
            } else {
                State::new(id.to_string())
            };
            self.states.insert(id.to_string(), state);
        }
    }

    /// Add a state with a specific type
    pub fn add_state_with_type(&mut self, id: &str, state_type: StateType) {
        let state = State::with_type(id.to_string(), state_type);
        self.states.insert(id.to_string(), state);
    }

    /// Get a state by ID
    pub fn get_state(&self, id: &str) -> Option<&State> {
        self.states.get(id)
    }

    /// Get a mutable state by ID
    pub fn get_state_mut(&mut self, id: &str) -> Option<&mut State> {
        self.states.get_mut(id)
    }

    /// Get all states
    pub fn get_states(&self) -> &HashMap<String, State> {
        &self.states
    }

    /// Add a description to a state
    pub fn add_description(&mut self, state_id: &str, description: &str) {
        self.add_state(state_id);
        if let Some(state) = self.states.get_mut(state_id) {
            // Remove leading colon if present
            let desc = Self::trim_colon(description);
            // Sanitize the description
            let sanitized = remove_script(&desc);
            state.descriptions.push(sanitized);
        }
    }

    /// Trim the leading colon from a string
    pub fn trim_colon(s: &str) -> String {
        if let Some(stripped) = s.strip_prefix(':') {
            stripped.to_string()
        } else {
            s.to_string()
        }
    }

    /// Add a relation between two states
    /// Handles [*] specially: creates unique start/end state IDs
    /// If parent is provided, sets parent on auto-created [*] states
    pub fn add_relation(
        &mut self,
        state1: &str,
        state2: &str,
        description: Option<&str>,
        parent: Option<&str>,
    ) {
        // Handle [*] as source (start state) - create unique ID
        let actual_state1 = if state1 == "[*]" {
            let id = format!("[*]_start_{}", self.start_cnt);
            self.start_cnt += 1;
            // Add as a start state type
            let mut state = State::new(id.clone());
            state.state_type = StateType::Start;
            // Set parent if inside a composite state
            if let Some(p) = parent {
                state.parent = Some(p.to_string());
            }
            self.states.insert(id.clone(), state);
            id
        } else {
            self.add_state(state1);
            state1.to_string()
        };

        // Handle [*] as target (end state) - create unique ID
        let actual_state2 = if state2 == "[*]" {
            let id = format!("[*]_end_{}", self.end_cnt);
            self.end_cnt += 1;
            // Add as an end state type
            let mut state = State::new(id.clone());
            state.state_type = StateType::End;
            // Set parent if inside a composite state
            if let Some(p) = parent {
                state.parent = Some(p.to_string());
            }
            self.states.insert(id.clone(), state);
            id
        } else {
            self.add_state(state2);
            state2.to_string()
        };

        let mut relation = Relation::new(actual_state1, actual_state2);
        if let Some(desc) = description {
            relation.description = Some(desc.to_string());
        }
        self.relations.push(relation);
    }

    /// Get all relations
    pub fn get_relations(&self) -> &[Relation] {
        &self.relations
    }

    /// Add a style class definition
    pub fn add_style_class(&mut self, id: &str, attribs: &str) {
        let mut style_class = StyleClass::new(id.to_string());

        // Parse the attributes (comma or semicolon separated)
        for attr in attribs.split([',', ';']) {
            let attr = attr.trim();
            if !attr.is_empty() {
                style_class.styles.push(attr.to_string());
            }
        }

        self.classes.insert(id.to_string(), style_class);
    }

    /// Get all style classes
    pub fn get_classes(&self) -> &HashMap<String, StyleClass> {
        &self.classes
    }

    /// Apply a style class to a state
    pub fn apply_class(&mut self, state_id: &str, class_id: &str) {
        self.add_state(state_id);
        if let Some(state) = self.states.get_mut(state_id) {
            state.classes.push(class_id.to_string());
        }
    }

    /// Set the diagram direction
    pub fn set_direction(&mut self, dir: Direction) {
        self.direction = dir;
    }

    /// Get the diagram direction
    pub fn get_direction(&self) -> Direction {
        self.direction
    }

    /// Generate a unique divider ID
    pub fn get_divider_id(&mut self) -> String {
        let id = format!("divider-id-{}", self.divider_cnt);
        self.divider_cnt += 1;
        id
    }

    /// Set the root document
    pub fn set_root_doc(&mut self, doc: Vec<Statement>) {
        self.root_doc = doc;
    }

    /// Get the root document
    pub fn get_root_doc(&self) -> &[Statement] {
        &self.root_doc
    }

    /// Add a note to a state
    pub fn add_note(&mut self, state_id: &str, position: NotePosition, text: &str) {
        self.add_state(state_id);
        if let Some(state) = self.states.get_mut(state_id) {
            state.note = Some(Note::new(position, text.to_string()));
        }
    }
}
