//! Core types for the layout engine

use std::collections::HashMap;

/// A point in 2D space
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

impl Point {
    pub fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }
}

/// Padding specification for compound nodes
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Padding {
    pub top: f64,
    pub right: f64,
    pub bottom: f64,
    pub left: f64,
}

impl Default for Padding {
    fn default() -> Self {
        Self {
            top: 10.0,
            right: 10.0,
            bottom: 10.0,
            left: 10.0,
        }
    }
}

impl Padding {
    pub fn uniform(value: f64) -> Self {
        Self {
            top: value,
            right: value,
            bottom: value,
            left: value,
        }
    }
}

/// Layout direction (flow of the graph)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LayoutDirection {
    #[default]
    TopToBottom,
    BottomToTop,
    LeftToRight,
    RightToLeft,
}

impl LayoutDirection {
    /// Check if this direction is horizontal (LR or RL)
    pub fn is_horizontal(&self) -> bool {
        matches!(self, Self::LeftToRight | Self::RightToLeft)
    }

    /// Check if this direction is reversed (BT or RL)
    pub fn is_reversed(&self) -> bool {
        matches!(self, Self::BottomToTop | Self::RightToLeft)
    }
}

/// Node shape hint for size estimation and rendering
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum NodeShape {
    #[default]
    Rectangle,
    RoundedRect,
    Circle,
    DoubleCircle,
    Ellipse,
    Diamond,
    Hexagon,
    Stadium,
    Cylinder,
    Subroutine,
    Trapezoid,
    InvTrapezoid,
    LeanRight,
    LeanLeft,
    Odd,
}

/// A node in the layout graph
#[derive(Debug, Clone)]
pub struct LayoutNode {
    /// Unique identifier
    pub id: String,
    /// Width of the node
    pub width: f64,
    /// Height of the node
    pub height: f64,
    /// Shape for rendering
    pub shape: NodeShape,
    /// Label text
    pub label: Option<String>,
    /// X position (set by layout)
    pub x: Option<f64>,
    /// Y position (set by layout)
    pub y: Option<f64>,
    /// Layer/rank assignment (set by layering phase)
    pub layer: Option<usize>,
    /// Order within layer (set by ordering phase)
    pub order: Option<usize>,
    /// Child nodes for compound/nested graphs (subgraphs)
    pub children: Vec<LayoutNode>,
    /// Padding for compound nodes
    pub padding: Padding,
    /// Whether this is a dummy node for edge routing
    pub is_dummy: bool,
    /// Original edge ID if this is a dummy node
    pub dummy_edge_id: Option<String>,
    /// Additional metadata
    pub metadata: HashMap<String, String>,
}

impl LayoutNode {
    pub fn new(id: impl Into<String>, width: f64, height: f64) -> Self {
        Self {
            id: id.into(),
            width,
            height,
            shape: NodeShape::default(),
            label: None,
            x: None,
            y: None,
            layer: None,
            order: None,
            children: Vec::new(),
            padding: Padding::default(),
            is_dummy: false,
            dummy_edge_id: None,
            metadata: HashMap::new(),
        }
    }

    pub fn with_shape(mut self, shape: NodeShape) -> Self {
        self.shape = shape;
        self
    }

    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn with_padding(mut self, padding: Padding) -> Self {
        self.padding = padding;
        self
    }

    /// Create a dummy node for long edge routing
    pub fn dummy(id: impl Into<String>, edge_id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            width: 0.0,
            height: 0.0,
            shape: NodeShape::Rectangle,
            label: None,
            x: None,
            y: None,
            layer: None,
            order: None,
            children: Vec::new(),
            padding: Padding::default(),
            is_dummy: true,
            dummy_edge_id: Some(edge_id.into()),
            metadata: HashMap::new(),
        }
    }

    /// Check if this node has children (is a compound node)
    pub fn is_compound(&self) -> bool {
        !self.children.is_empty()
    }

    /// Get the center point of the node
    pub fn center(&self) -> Option<Point> {
        match (self.x, self.y) {
            (Some(x), Some(y)) => Some(Point::new(x + self.width / 2.0, y + self.height / 2.0)),
            _ => None,
        }
    }
}

/// An edge in the layout graph
#[derive(Debug, Clone)]
pub struct LayoutEdge {
    /// Unique identifier
    pub id: String,
    /// Source node IDs
    pub sources: Vec<String>,
    /// Target node IDs
    pub targets: Vec<String>,
    /// Edge label
    pub label: Option<String>,
    /// Bend points (set by routing phase)
    pub bend_points: Vec<Point>,
    /// Label position (set by routing phase)
    pub label_position: Option<Point>,
    /// Edge weight for layout prioritization
    pub weight: u32,
    /// Whether this edge was reversed for cycle removal
    pub reversed: bool,
    /// Additional metadata
    pub metadata: HashMap<String, String>,
}

impl LayoutEdge {
    pub fn new(
        id: impl Into<String>,
        source: impl Into<String>,
        target: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            sources: vec![source.into()],
            targets: vec![target.into()],
            label: None,
            bend_points: Vec::new(),
            label_position: None,
            weight: 1,
            reversed: false,
            metadata: HashMap::new(),
        }
    }

    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn with_weight(mut self, weight: u32) -> Self {
        self.weight = weight;
        self
    }

    /// Get the primary source node ID
    pub fn source(&self) -> Option<&str> {
        self.sources.first().map(|s| s.as_str())
    }

    /// Get the primary target node ID
    pub fn target(&self) -> Option<&str> {
        self.targets.first().map(|s| s.as_str())
    }
}

/// Layout options for the graph
#[derive(Debug, Clone)]
pub struct LayoutOptions {
    /// Direction of the graph flow
    pub direction: LayoutDirection,
    /// Spacing between nodes in the same layer
    pub node_spacing: f64,
    /// Spacing between layers
    pub layer_spacing: f64,
    /// Padding around the entire graph
    pub padding: Padding,
}

impl Default for LayoutOptions {
    fn default() -> Self {
        Self {
            direction: LayoutDirection::TopToBottom,
            node_spacing: 50.0,
            layer_spacing: 50.0,
            padding: Padding::uniform(20.0),
        }
    }
}
