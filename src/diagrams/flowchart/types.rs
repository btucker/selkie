//! Flowchart types

use std::collections::HashMap;
use std::str::FromStr;

use crate::common::CommonDb;

/// Valid vertex types in flowcharts
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum FlowVertexType {
    #[default]
    Square,
    DoubleCircle,
    Circle,
    Ellipse,
    Stadium,
    Subroutine,
    Rect,
    Cylinder,
    Round,
    Diamond,
    Hexagon,
    Odd,
    Trapezoid,
    InvTrapezoid,
    LeanRight,
    LeanLeft,
}

impl FromStr for FlowVertexType {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "square" => Ok(Self::Square),
            "doublecircle" => Ok(Self::DoubleCircle),
            "circle" => Ok(Self::Circle),
            "ellipse" => Ok(Self::Ellipse),
            "stadium" => Ok(Self::Stadium),
            "subroutine" => Ok(Self::Subroutine),
            "rect" => Ok(Self::Rect),
            "cylinder" => Ok(Self::Cylinder),
            "round" => Ok(Self::Round),
            "diamond" => Ok(Self::Diamond),
            "hexagon" => Ok(Self::Hexagon),
            "odd" => Ok(Self::Odd),
            "trapezoid" => Ok(Self::Trapezoid),
            "inv_trapezoid" => Ok(Self::InvTrapezoid),
            "lean_right" => Ok(Self::LeanRight),
            "lean_left" => Ok(Self::LeanLeft),
            _ => Err(()),
        }
    }
}

/// Text type for flowchart labels
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum FlowTextType {
    #[default]
    Text,
    Markdown,
}

/// Text content for a flowchart element
#[derive(Debug, Clone, Default)]
pub struct FlowText {
    pub text: String,
    pub text_type: FlowTextType,
}

impl FlowText {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            text_type: FlowTextType::Text,
        }
    }

    pub fn markdown(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            text_type: FlowTextType::Markdown,
        }
    }
}

/// A vertex (node) in a flowchart
#[derive(Debug, Clone)]
pub struct FlowVertex {
    pub id: String,
    pub dom_id: String,
    pub text: Option<String>,
    pub label_type: FlowTextType,
    pub vertex_type: Option<FlowVertexType>,
    pub styles: Vec<String>,
    pub classes: Vec<String>,
    pub dir: Option<String>,
    pub link: Option<String>,
    pub link_target: Option<String>,
    pub have_callback: bool,
    pub icon: Option<String>,
    pub form: Option<String>,
    pub pos: Option<String>,
    pub img: Option<String>,
    pub constraint: Option<String>,
}

impl FlowVertex {
    pub fn new(id: impl Into<String>, dom_id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            dom_id: dom_id.into(),
            text: None,
            label_type: FlowTextType::Text,
            vertex_type: None,
            styles: Vec::new(),
            classes: Vec::new(),
            dir: None,
            link: None,
            link_target: None,
            have_callback: false,
            icon: None,
            form: None,
            pos: None,
            img: None,
            constraint: None,
        }
    }
}

/// Edge stroke types
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum EdgeStroke {
    #[default]
    Normal,
    Thick,
    Invisible,
    Dotted,
}

/// An edge (link) between nodes in a flowchart
#[derive(Debug, Clone)]
pub struct FlowEdge {
    pub id: Option<String>,
    pub is_user_defined_id: bool,
    pub start: String,
    pub end: String,
    pub interpolate: Option<String>,
    pub edge_type: Option<String>,
    pub stroke: EdgeStroke,
    pub style: Vec<String>,
    pub length: Option<u32>,
    pub text: String,
    pub label_type: FlowTextType,
    pub classes: Vec<String>,
    pub animation: Option<String>,
    pub animate: Option<bool>,
}

impl FlowEdge {
    pub fn new(start: impl Into<String>, end: impl Into<String>) -> Self {
        Self {
            id: None,
            is_user_defined_id: false,
            start: start.into(),
            end: end.into(),
            interpolate: None,
            edge_type: None,
            stroke: EdgeStroke::Normal,
            style: Vec::new(),
            length: None,
            text: String::new(),
            label_type: FlowTextType::Text,
            classes: Vec::new(),
            animation: None,
            animate: None,
        }
    }
}

/// A class definition for styling
#[derive(Debug, Clone, Default)]
pub struct FlowClass {
    pub id: String,
    pub styles: Vec<String>,
    pub text_styles: Vec<String>,
}

impl FlowClass {
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            styles: Vec::new(),
            text_styles: Vec::new(),
        }
    }
}

/// A subgraph (container for nodes)
#[derive(Debug, Clone, Default)]
pub struct FlowSubGraph {
    pub id: String,
    pub title: String,
    pub label_type: String,
    pub nodes: Vec<String>,
    pub classes: Vec<String>,
    pub dir: Option<String>,
}

/// Link type information from parser
#[derive(Debug, Clone, Default)]
pub struct FlowLink {
    pub link_type: Option<String>,
    pub stroke: EdgeStroke,
    pub length: Option<u32>,
    pub text: Option<FlowText>,
    pub id: Option<String>,
}

/// Data returned from get_data()
#[derive(Debug, Clone)]
pub struct FlowData {
    pub vertices: HashMap<String, FlowVertex>,
    pub edges: Vec<FlowEdge>,
    pub classes: HashMap<String, FlowClass>,
    pub subgraphs: Vec<FlowSubGraph>,
}

/// Flowchart direction
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Direction {
    #[default]
    TopToBottom,
    BottomToTop,
    LeftToRight,
    RightToLeft,
}

impl Direction {
    /// Parse direction from mermaid syntax
    pub fn parse(s: &str) -> Self {
        let s = s.trim();
        if s.contains('<') {
            Self::RightToLeft
        } else if s.contains('^') {
            Self::BottomToTop
        } else if s.contains('>') {
            Self::LeftToRight
        } else if s.contains('v') || s == "TD" || s == "TB" {
            Self::TopToBottom
        } else {
            match s {
                "RL" => Self::RightToLeft,
                "BT" => Self::BottomToTop,
                "LR" => Self::LeftToRight,
                _ => Self::TopToBottom,
            }
        }
    }

    /// Convert to short string format (TB, BT, LR, RL)
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::TopToBottom => "TB",
            Self::BottomToTop => "BT",
            Self::LeftToRight => "LR",
            Self::RightToLeft => "RL",
        }
    }
}

/// The flowchart database
#[derive(Debug, Clone)]
pub struct FlowchartDb {
    common: CommonDb,
    vertex_counter: u32,
    vertices: HashMap<String, FlowVertex>,
    edges: Vec<FlowEdge>,
    default_interpolate: Option<String>,
    default_style: Option<Vec<String>>,
    classes: HashMap<String, FlowClass>,
    subgraphs: Vec<FlowSubGraph>,
    subgraph_lookup: HashMap<String, usize>,
    tooltips: HashMap<String, String>,
    direction: Direction,
}

impl Default for FlowchartDb {
    fn default() -> Self {
        Self::new()
    }
}

/// Parse an arrow string to extract edge type, stroke style, and length.
/// Returns (edge_type, stroke, length).
///
/// Arrow formats:
/// - Normal: `-->`, `---`, `-->`
/// - Thick: `==>`, `===`, `==>`
/// - Dotted: `-.->`, `-.-`, `-..->`
/// - With starts: `<-->`, `x--x`, `o--o`
fn parse_arrow(arrow: &str) -> (String, EdgeStroke, u32) {
    let arrow = arrow.trim();

    // Determine stroke type based on characters
    let stroke = if arrow.contains("-.") || arrow.contains(".-") {
        EdgeStroke::Dotted
    } else if arrow.contains('=') {
        EdgeStroke::Thick
    } else if arrow.starts_with('~') {
        EdgeStroke::Invisible
    } else {
        EdgeStroke::Normal
    };

    // Determine edge type based on start/end markers
    let has_start_arrow = arrow.starts_with('<')
        || arrow.starts_with("x-")
        || arrow.starts_with("o-")
        || arrow.starts_with("x=")
        || arrow.starts_with("o=")
        || arrow.starts_with("<-")
        || arrow.starts_with("<=");
    let has_end_arrow = arrow.ends_with('>');
    let has_end_cross = arrow.ends_with('x') && !arrow.starts_with('x');
    let has_end_circle = arrow.ends_with('o') && !arrow.starts_with('o');
    let has_start_cross = arrow.starts_with("x-") || arrow.starts_with("x=");
    let has_start_circle = arrow.starts_with("o-") || arrow.starts_with("o=");
    let has_end_cross_double = arrow.ends_with('x') && has_start_cross;
    let has_end_circle_double = arrow.ends_with('o') && has_start_circle;

    let edge_type = if has_start_arrow && has_end_arrow {
        "double_arrow_point".to_string()
    } else if has_end_cross_double || (has_start_cross && arrow.ends_with('x')) {
        "double_arrow_cross".to_string()
    } else if has_end_circle_double || (has_start_circle && arrow.ends_with('o')) {
        "double_arrow_circle".to_string()
    } else if has_end_arrow {
        "arrow_point".to_string()
    } else if has_end_cross {
        "arrow_cross".to_string()
    } else if has_end_circle {
        "arrow_circle".to_string()
    } else {
        "arrow_open".to_string()
    };

    // Calculate length based on repeated characters
    let length = match stroke {
        EdgeStroke::Normal | EdgeStroke::Invisible => {
            // Count consecutive dashes or tildes
            let dash_count = arrow.chars().filter(|&c| c == '-' || c == '~').count();
            // Minimum length is 1, max is 3 for display purposes
            (dash_count.saturating_sub(1).max(1).min(3)) as u32
        }
        EdgeStroke::Thick => {
            let eq_count = arrow.chars().filter(|&c| c == '=').count();
            (eq_count.saturating_sub(1).max(1).min(3)) as u32
        }
        EdgeStroke::Dotted => {
            let dot_count = arrow.chars().filter(|&c| c == '.').count();
            (dot_count.max(1).min(3)) as u32
        }
    };

    (edge_type, stroke, length)
}

impl FlowchartDb {
    const DOM_ID_PREFIX: &'static str = "flowchart-";

    pub fn new() -> Self {
        Self {
            common: CommonDb::new(),
            vertex_counter: 0,
            vertices: HashMap::new(),
            edges: Vec::new(),
            default_interpolate: None,
            default_style: None,
            classes: HashMap::new(),
            subgraphs: Vec::new(),
            subgraph_lookup: HashMap::new(),
            tooltips: HashMap::new(),
            direction: Direction::default(),
        }
    }

    pub fn clear(&mut self) {
        self.common.clear();
        self.vertex_counter = 0;
        self.vertices.clear();
        self.edges.clear();
        self.default_interpolate = None;
        self.default_style = None;
        self.classes.clear();
        self.subgraphs.clear();
        self.subgraph_lookup.clear();
        self.tooltips.clear();
        self.direction = Direction::default();
    }

    /// Check if a node exists in any of the given subgraphs
    pub fn exists(&self, subgraphs: &[FlowSubGraph], node_id: &str) -> bool {
        subgraphs
            .iter()
            .any(|sg| sg.nodes.iter().any(|n| n == node_id))
    }

    /// Remove nodes from a subgraph that already exist in other subgraphs
    pub fn make_uniq(&self, subgraph: &mut FlowSubGraph, existing: &[FlowSubGraph]) {
        subgraph.nodes.retain(|node| !self.exists(existing, node));
    }

    /// Add a vertex to the flowchart
    pub fn add_vertex(
        &mut self,
        id: &str,
        text_obj: Option<FlowText>,
        vertex_type: Option<FlowVertexType>,
        styles: Vec<String>,
        classes: Vec<String>,
        dir: Option<&str>,
        _metadata: Option<&str>,
    ) {
        let id = id.trim();
        if id.is_empty() {
            return;
        }

        let vertex = self.vertices.entry(id.to_string()).or_insert_with(|| {
            let dom_id = format!("{}{}-{}", Self::DOM_ID_PREFIX, id, self.vertex_counter);
            FlowVertex::new(id, dom_id)
        });

        self.vertex_counter += 1;

        if let Some(text_obj) = text_obj {
            let txt = text_obj.text.trim();
            // Strip surrounding quotes
            let txt = txt
                .strip_prefix('"')
                .and_then(|s| s.strip_suffix('"'))
                .unwrap_or(txt);
            vertex.text = Some(txt.to_string());
            vertex.label_type = text_obj.text_type;
        } else if vertex.text.is_none() {
            vertex.text = Some(id.to_string());
        }

        if let Some(vt) = vertex_type {
            vertex.vertex_type = Some(vt);
        }

        vertex.styles.extend(styles);
        vertex.classes.extend(classes);

        if let Some(d) = dir {
            vertex.dir = Some(d.to_string());
        }
    }

    /// Add an edge between nodes
    fn add_single_link(
        &mut self,
        start: &str,
        end: &str,
        link_data: Option<&FlowLink>,
        id: Option<&str>,
    ) {
        let mut edge = FlowEdge::new(start, end);
        edge.interpolate.clone_from(&self.default_interpolate);

        if let Some(link) = link_data {
            if let Some(text) = &link.text {
                let txt = text.text.trim();
                let txt = txt
                    .strip_prefix('"')
                    .and_then(|s| s.strip_suffix('"'))
                    .unwrap_or(txt);
                edge.text = txt.to_string();
                edge.label_type = text.text_type.clone();
            }
            edge.edge_type.clone_from(&link.link_type);
            edge.stroke = link.stroke.clone();
            edge.length = link.length.map(|l| l.min(10));
        }

        if let Some(user_id) = id {
            let id_exists = self.edges.iter().any(|e| e.id.as_deref() == Some(user_id));
            if !id_exists {
                edge.id = Some(user_id.to_string());
                edge.is_user_defined_id = true;
            }
        }

        if edge.id.is_none() {
            let existing_count = self
                .edges
                .iter()
                .filter(|e| e.start == start && e.end == end)
                .count();
            edge.id = Some(format!("L-{}-{}-{}", start, end, existing_count));
        }

        self.edges.push(edge);
    }

    /// Add links between multiple start and end nodes
    pub fn add_link(&mut self, starts: &[&str], ends: &[&str], link_data: Option<&FlowLink>) {
        let id = link_data.and_then(|l| l.id.as_deref());
        let last_start_idx = starts.len().saturating_sub(1);

        for (si, start) in starts.iter().enumerate() {
            for (ei, end) in ends.iter().enumerate() {
                // Only use ID for last start and first end
                let use_id = si == last_start_idx && ei == 0;
                self.add_single_link(start, end, link_data, if use_id { id } else { None });
            }
        }
    }

    /// Update link interpolation
    pub fn update_link_interpolate(&mut self, positions: &[String], interpolate: &str) {
        let interpolate = interpolate.to_string();

        for pos in positions {
            if pos == "default" {
                self.default_interpolate = Some(interpolate.clone());
                // Apply to existing edges without explicit interpolate
                for edge in &mut self.edges {
                    if edge.interpolate.is_none() {
                        edge.interpolate = Some(interpolate.clone());
                    }
                }
            } else if let Ok(idx) = pos.parse::<usize>() {
                if let Some(edge) = self.edges.get_mut(idx) {
                    edge.interpolate = Some(interpolate.clone());
                }
            }
        }
    }

    /// Update link style
    pub fn update_link(&mut self, positions: &[usize], style: &[String]) {
        for &pos in positions {
            if pos == usize::MAX {
                self.default_style = Some(style.to_vec());
            } else if let Some(edge) = self.edges.get_mut(pos) {
                edge.style = style.to_vec();
                // Add fill:none if not already present
                let has_fill = edge.style.iter().any(|s| s.starts_with("fill"));
                if !has_fill {
                    edge.style.push("fill:none".to_string());
                }
            }
        }
    }

    /// Add a CSS class definition
    pub fn add_class(&mut self, ids: &str, styles: &[String]) {
        // Process styles: handle escaped commas, convert commas to semicolons
        let processed: Vec<String> = styles
            .join(",")
            .replace("\\,", "\x00") // Temporary placeholder
            .replace(',', ";")
            .replace('\x00', ",")
            .split(';')
            .filter(|s| !s.is_empty())
            .map(String::from)
            .collect();

        for id in ids.split(',').map(str::trim) {
            let class_node = self
                .classes
                .entry(id.to_string())
                .or_insert_with(|| FlowClass::new(id));

            for style in &processed {
                if style.contains("color") {
                    class_node.text_styles.push(style.replace("fill", "bgFill"));
                }
                class_node.styles.push(style.clone());
            }
        }
    }

    /// Set the direction of the flowchart
    pub fn set_direction(&mut self, dir: &str) {
        self.direction = Direction::parse(dir);
    }

    /// Get the direction of the flowchart as a string
    pub fn get_direction(&self) -> &'static str {
        self.direction.as_str()
    }

    /// Set class on elements
    pub fn set_class(&mut self, ids: &str, class_name: &str) {
        for id in ids.split(',').map(str::trim) {
            if let Some(vertex) = self.vertices.get_mut(id) {
                vertex.classes.push(class_name.to_string());
            }

            for edge in &mut self.edges {
                if edge.id.as_deref() == Some(id) {
                    edge.classes.push(class_name.to_string());
                }
            }

            if let Some(&idx) = self.subgraph_lookup.get(id) {
                if let Some(sg) = self.subgraphs.get_mut(idx) {
                    sg.classes.push(class_name.to_string());
                }
            }
        }
    }

    /// Add a subgraph
    pub fn add_sub_graph(&mut self, nodes: Vec<String>, id: &str, title: &str, dir: &str) {
        let subgraph = FlowSubGraph {
            id: id.to_string(),
            title: title.to_string(),
            label_type: "text".to_string(),
            nodes,
            classes: Vec::new(),
            dir: if dir.is_empty() {
                None
            } else {
                Some(dir.to_string())
            },
        };

        let idx = self.subgraphs.len();
        self.subgraph_lookup.insert(id.to_string(), idx);
        self.subgraphs.push(subgraph);
    }

    /// Get all vertices
    pub fn vertices(&self) -> &HashMap<String, FlowVertex> {
        &self.vertices
    }

    /// Get vertices (alias for compatibility with parser)
    pub fn get_vertices(&self) -> &HashMap<String, FlowVertex> {
        &self.vertices
    }

    /// Get a mutable vertex by ID
    pub fn get_vertex_mut(&mut self, id: &str) -> Option<&mut FlowVertex> {
        self.vertices.get_mut(id)
    }

    /// Get all edges
    pub fn edges(&self) -> &[FlowEdge] {
        &self.edges
    }

    /// Get edges (alias for compatibility with parser)
    pub fn get_edges(&self) -> &[FlowEdge] {
        &self.edges
    }

    /// Get all classes
    pub fn get_classes(&self) -> &HashMap<String, FlowClass> {
        &self.classes
    }

    /// Get all subgraphs
    pub fn subgraphs(&self) -> &[FlowSubGraph] {
        &self.subgraphs
    }

    /// Simplified add_vertex for parser - just id, optional text and type
    pub fn add_vertex_simple(
        &mut self,
        id: &str,
        text: Option<&str>,
        vertex_type: Option<FlowVertexType>,
    ) {
        let text_obj = text.map(|t| FlowText::new(t));
        self.add_vertex(
            id,
            text_obj,
            vertex_type,
            Vec::new(),
            Vec::new(),
            None,
            None,
        );
    }

    /// Add an edge between two nodes (simplified for parser)
    pub fn add_edge(
        &mut self,
        start: &str,
        end: &str,
        arrow: &str,
        text: Option<&str>,
        link_id: Option<&str>,
    ) {
        // Ensure vertices exist
        if !self.vertices.contains_key(start) {
            self.add_vertex_simple(start, None, None);
        }
        if !self.vertices.contains_key(end) {
            self.add_vertex_simple(end, None, None);
        }

        // Parse arrow string to extract edge type, stroke, and length
        let (edge_type, stroke, length) = parse_arrow(arrow);

        let flow_link = FlowLink {
            text: text.map(|t| FlowText::new(t)),
            id: link_id.map(String::from),
            link_type: Some(edge_type),
            stroke,
            length: Some(length),
        };

        self.add_single_link(start, end, Some(&flow_link), link_id);
    }

    /// Add a subgraph (simplified for parser)
    pub fn add_subgraph(&mut self, id: &str, title: &str) {
        self.add_sub_graph(Vec::new(), id, title, "");
    }

    /// Add a subgraph with member nodes
    pub fn add_subgraph_with_nodes(&mut self, id: &str, title: &str, nodes: Vec<String>) {
        self.add_sub_graph(nodes, id, title, "");
    }

    /// Set link on a vertex (for click handler)
    pub fn set_link(&mut self, id: &str, link: &str, target: Option<&str>) {
        if let Some(vertex) = self.vertices.get_mut(id) {
            vertex.link = Some(link.to_string());
            vertex.link_target = target.map(String::from);
        }
    }

    /// Set click event on a vertex
    pub fn set_click_event(&mut self, id: &str, callback: &str) {
        if let Some(vertex) = self.vertices.get_mut(id) {
            vertex.have_callback = true;
            // Store callback name (would need additional field)
        }
        let _ = callback; // TODO: store callback
    }

    /// Set tooltip on a vertex
    pub fn set_tooltip(&mut self, id: &str, tooltip: &str) {
        self.tooltips.insert(id.to_string(), tooltip.to_string());
    }

    /// Set default link style
    pub fn set_default_link_style(&mut self, styles: &[String]) {
        self.default_style = Some(styles.to_vec());
    }

    /// Set link style by index
    pub fn set_link_style(&mut self, idx: usize, styles: &[String]) {
        if let Some(edge) = self.edges.get_mut(idx) {
            edge.style = styles.to_vec();
        }
    }

    /// Set default link interpolate
    pub fn set_default_link_interpolate(&mut self, interpolate: &str) {
        self.default_interpolate = Some(interpolate.to_string());
    }

    /// Get data for rendering
    pub fn get_data(&self) -> FlowData {
        FlowData {
            vertices: self.vertices.clone(),
            edges: self.edges.clone(),
            classes: self.classes.clone(),
            subgraphs: self.subgraphs.clone(),
        }
    }

    // Common DB delegation
    pub fn set_acc_title(&mut self, title: impl Into<String>) {
        self.common.set_acc_title(title);
    }

    pub fn get_acc_title(&self) -> Option<&str> {
        self.common.get_acc_title()
    }

    pub fn set_acc_description(&mut self, desc: impl Into<String>) {
        self.common.set_acc_description(desc);
    }

    pub fn get_acc_description(&self) -> Option<&str> {
        self.common.get_acc_description()
    }

    pub fn set_diagram_title(&mut self, title: impl Into<String>) {
        self.common.set_diagram_title(title);
    }

    pub fn get_diagram_title(&self) -> Option<&str> {
        self.common.get_diagram_title()
    }
}
