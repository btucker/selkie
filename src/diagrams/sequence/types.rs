//! Sequence diagram types

use std::collections::HashMap;

/// Line/message types for sequence diagrams
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum LineType {
    Solid = 0,
    Dotted = 1,
    Note = 2,
    SolidCross = 3,
    DottedCross = 4,
    SolidOpen = 5,
    DottedOpen = 6,
    LoopStart = 10,
    LoopEnd = 11,
    AltStart = 12,
    AltElse = 13,
    AltEnd = 14,
    OptStart = 15,
    OptEnd = 16,
    ActiveStart = 17,
    ActiveEnd = 18,
    ParStart = 19,
    ParAnd = 20,
    ParEnd = 21,
    RectStart = 22,
    RectEnd = 23,
    SolidPoint = 24,
    DottedPoint = 25,
    Autonumber = 26,
    CriticalStart = 27,
    CriticalOption = 28,
    CriticalEnd = 29,
    BreakStart = 30,
    BreakEnd = 31,
    ParOverStart = 32,
    BidirectionalSolid = 33,
    BidirectionalDotted = 34,
    SolidTop = 41,
    SolidBottom = 42,
    StickTop = 43,
    StickBottom = 44,
    SolidArrowTopReverse = 45,
    SolidArrowBottomReverse = 46,
    StickArrowTopReverse = 47,
    StickArrowBottomReverse = 48,
    SolidTopDotted = 51,
    SolidBottomDotted = 52,
    StickTopDotted = 53,
    StickBottomDotted = 54,
    SolidArrowTopReverseDotted = 55,
    SolidArrowBottomReverseDotted = 56,
    StickArrowTopReverseDotted = 57,
    StickArrowBottomReverseDotted = 58,
    CentralConnection = 59,
    CentralConnectionReverse = 60,
    CentralConnectionDual = 61,
}

impl LineType {
    pub fn as_num(&self) -> i32 {
        *self as i32
    }
}

/// Arrow types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ArrowType {
    #[default]
    Filled = 0,
    Open = 1,
}

/// Note placement
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Placement {
    #[default]
    LeftOf = 0,
    RightOf = 1,
    Over = 2,
}

impl Placement {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "rightof" | "right of" => Self::RightOf,
            "over" => Self::Over,
            _ => Self::LeftOf,
        }
    }
}

/// Participant/actor types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ParticipantType {
    #[default]
    Participant,
    Actor,
    Boundary,
    Control,
    Entity,
    Database,
    Collections,
    Queue,
}

impl ParticipantType {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "actor" => Self::Actor,
            "boundary" => Self::Boundary,
            "control" => Self::Control,
            "entity" => Self::Entity,
            "database" => Self::Database,
            "collections" => Self::Collections,
            "queue" => Self::Queue,
            _ => Self::Participant,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Participant => "participant",
            Self::Actor => "actor",
            Self::Boundary => "boundary",
            Self::Control => "control",
            Self::Entity => "entity",
            Self::Database => "database",
            Self::Collections => "collections",
            Self::Queue => "queue",
        }
    }
}

/// A box grouping actors
#[derive(Debug, Clone)]
pub struct Box {
    pub name: String,
    pub wrap: bool,
    pub fill: String,
    pub actor_keys: Vec<String>,
}

impl Box {
    pub fn new(name: String) -> Self {
        Self {
            name,
            wrap: false,
            fill: String::new(),
            actor_keys: Vec::new(),
        }
    }
}

/// An actor/participant in the sequence diagram
#[derive(Debug, Clone)]
pub struct Actor {
    pub name: String,
    pub description: String,
    pub wrap: bool,
    pub prev_actor: Option<String>,
    pub next_actor: Option<String>,
    pub links: HashMap<String, String>,
    pub properties: HashMap<String, String>,
    pub actor_type: ParticipantType,
    pub box_ref: Option<usize>,
}

impl Actor {
    pub fn new(name: String, actor_type: ParticipantType) -> Self {
        Self {
            name: name.clone(),
            description: name,
            wrap: false,
            prev_actor: None,
            next_actor: None,
            links: HashMap::new(),
            properties: HashMap::new(),
            actor_type,
            box_ref: None,
        }
    }
}

/// Autonumber configuration
#[derive(Debug, Clone)]
pub struct AutonumberConfig {
    pub start: i32,
    pub step: i32,
    pub visible: bool,
}

impl Default for AutonumberConfig {
    fn default() -> Self {
        Self {
            start: 1,
            step: 1,
            visible: true,
        }
    }
}

/// A message in the sequence diagram
#[derive(Debug, Clone)]
pub struct Message {
    pub id: String,
    pub from: Option<String>,
    pub to: Option<String>,
    pub message: String,
    pub wrap: bool,
    pub message_type: LineType,
    pub activate: bool,
    pub central_connection: i32,
}

impl Message {
    pub fn new(
        id: String,
        from: Option<String>,
        to: Option<String>,
        message: String,
        message_type: LineType,
    ) -> Self {
        Self {
            id,
            from,
            to,
            message,
            wrap: false,
            message_type,
            activate: false,
            central_connection: 0,
        }
    }
}

/// A note in the sequence diagram
#[derive(Debug, Clone)]
pub struct Note {
    pub actor: String,
    pub placement: Placement,
    pub message: String,
    pub wrap: bool,
}

impl Note {
    pub fn new(actor: String, placement: Placement, message: String) -> Self {
        Self {
            actor,
            placement,
            message,
            wrap: false,
        }
    }
}

/// The sequence diagram database
#[derive(Debug, Clone)]
pub struct SequenceDb {
    /// Actors in the diagram
    actors: HashMap<String, Actor>,
    /// Ordered list of actor names
    actor_order: Vec<String>,
    /// Created actors (actor name -> message index)
    created_actors: HashMap<String, usize>,
    /// Destroyed actors (actor name -> message index)
    destroyed_actors: HashMap<String, usize>,
    /// Boxes grouping actors
    boxes: Vec<Box>,
    /// Messages in the diagram
    messages: Vec<Message>,
    /// Notes in the diagram
    notes: Vec<Note>,
    /// Whether sequence numbers are enabled
    sequence_numbers_enabled: bool,
    /// Whether wrap is enabled globally
    wrap_enabled: bool,
    /// Current autonumber configuration
    autonumber: Option<AutonumberConfig>,
    /// Message ID counter
    message_counter: usize,
    /// Accessibility title
    pub acc_title: String,
    /// Accessibility description
    pub acc_descr: String,
    /// Diagram title
    pub diagram_title: String,
    /// Current open box index (for tracking actors added within a box)
    current_box_index: Option<usize>,
}

impl Default for SequenceDb {
    fn default() -> Self {
        Self::new()
    }
}

impl SequenceDb {
    pub fn new() -> Self {
        Self {
            actors: HashMap::new(),
            actor_order: Vec::new(),
            created_actors: HashMap::new(),
            destroyed_actors: HashMap::new(),
            boxes: Vec::new(),
            messages: Vec::new(),
            notes: Vec::new(),
            sequence_numbers_enabled: false,
            wrap_enabled: false,
            autonumber: None,
            message_counter: 0,
            acc_title: String::new(),
            acc_descr: String::new(),
            diagram_title: String::new(),
            current_box_index: None,
        }
    }

    pub fn clear(&mut self) {
        self.actors.clear();
        self.actor_order.clear();
        self.created_actors.clear();
        self.destroyed_actors.clear();
        self.boxes.clear();
        self.messages.clear();
        self.notes.clear();
        self.sequence_numbers_enabled = false;
        self.wrap_enabled = false;
        self.autonumber = None;
        self.message_counter = 0;
        self.acc_title.clear();
        self.acc_descr.clear();
        self.diagram_title.clear();
        self.current_box_index = None;
    }

    /// Add an actor to the diagram
    pub fn add_actor(
        &mut self,
        name: &str,
        description: Option<&str>,
        actor_type: ParticipantType,
    ) {
        if self.actors.contains_key(name) {
            return;
        }

        let mut actor = Actor::new(name.to_string(), actor_type);
        if let Some(desc) = description {
            actor.description = desc.to_string();
        }

        // Link to previous actor
        if let Some(prev_name) = self.actor_order.last() {
            actor.prev_actor = Some(prev_name.clone());
            if let Some(prev_actor) = self.actors.get_mut(prev_name) {
                prev_actor.next_actor = Some(name.to_string());
            }
        }

        self.actor_order.push(name.to_string());
        self.actors.insert(name.to_string(), actor);

        // Add to current box if one is open
        if let Some(box_idx) = self.current_box_index {
            if let Some(current_box) = self.boxes.get_mut(box_idx) {
                current_box.actor_keys.push(name.to_string());
            }
        }
    }

    /// Get all actors
    pub fn get_actors(&self) -> &HashMap<String, Actor> {
        &self.actors
    }

    /// Get actors in order
    pub fn get_actors_in_order(&self) -> Vec<&Actor> {
        self.actor_order
            .iter()
            .filter_map(|name| self.actors.get(name))
            .collect()
    }

    /// Add a message to the diagram
    pub fn add_message(
        &mut self,
        from: &str,
        to: &str,
        message: &str,
        message_type: LineType,
        activate: bool,
    ) {
        // Auto-create actors if they don't exist
        if !self.actors.contains_key(from) {
            self.add_actor(from, None, ParticipantType::Participant);
        }
        if !self.actors.contains_key(to) {
            self.add_actor(to, None, ParticipantType::Participant);
        }

        let id = self.message_counter.to_string();
        self.message_counter += 1;

        let mut msg = Message::new(
            id,
            Some(from.to_string()),
            Some(to.to_string()),
            message.to_string(),
            message_type,
        );
        msg.activate = activate;
        msg.wrap = self.wrap_enabled;

        self.messages.push(msg);
    }

    /// Get all messages
    pub fn get_messages(&self) -> &[Message] {
        &self.messages
    }

    /// Add a note to the diagram
    pub fn add_note(&mut self, actor: &str, placement: Placement, message: &str) {
        let mut note = Note::new(actor.to_string(), placement, message.to_string());
        note.wrap = self.wrap_enabled;
        self.notes.push(note);
    }

    /// Get all notes
    pub fn get_notes(&self) -> &[Note] {
        &self.notes
    }

    /// Enable/disable autonumbering
    pub fn set_autonumber(&mut self, enabled: bool, start: Option<i32>, step: Option<i32>) {
        if enabled {
            self.autonumber = Some(AutonumberConfig {
                start: start.unwrap_or(1),
                step: step.unwrap_or(1),
                visible: true,
            });
            self.sequence_numbers_enabled = true;
        } else {
            self.autonumber = None;
            self.sequence_numbers_enabled = false;
        }
    }

    /// Check if sequence numbers are enabled
    pub fn sequence_numbers_enabled(&self) -> bool {
        self.sequence_numbers_enabled
    }

    /// Set global wrap
    pub fn set_wrap(&mut self, wrap: bool) {
        self.wrap_enabled = wrap;
    }

    /// Add a box (starts collecting actors for this box)
    pub fn add_box(&mut self, name: &str, color: &str) {
        let mut box_item = Box::new(name.to_string());
        box_item.fill = color.to_string();
        self.boxes.push(box_item);
        self.current_box_index = Some(self.boxes.len() - 1);
    }

    /// End the current box (stop collecting actors)
    pub fn end_box(&mut self) {
        self.current_box_index = None;
    }

    /// Get all boxes
    pub fn get_boxes(&self) -> &[Box] {
        &self.boxes
    }

    /// Create an actor (tracks when created)
    pub fn create_actor(
        &mut self,
        name: &str,
        description: Option<&str>,
        actor_type: ParticipantType,
    ) {
        self.add_actor(name, description, actor_type);
        self.created_actors
            .insert(name.to_string(), self.messages.len());
    }

    /// Destroy an actor
    pub fn destroy_actor(&mut self, name: &str) {
        if self.actors.contains_key(name) {
            self.destroyed_actors
                .insert(name.to_string(), self.messages.len());
        }
    }

    /// Check if actor was created dynamically
    pub fn is_created(&self, name: &str) -> bool {
        self.created_actors.contains_key(name)
    }

    /// Check if actor was destroyed
    pub fn is_destroyed(&self, name: &str) -> bool {
        self.destroyed_actors.contains_key(name)
    }

    /// Add a control structure message (loop, alt, etc.)
    pub fn add_signal(&mut self, message_type: LineType, message: Option<&str>) {
        let id = self.message_counter.to_string();
        self.message_counter += 1;

        let msg = Message::new(
            id,
            None,
            None,
            message.unwrap_or("").to_string(),
            message_type,
        );
        self.messages.push(msg);
    }
}
