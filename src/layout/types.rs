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

    /// Calculate the Euclidean distance to another point
    pub fn distance_to(&self, other: &Point) -> f64 {
        let dx = other.x - self.x;
        let dy = other.y - self.y;
        (dx * dx + dy * dy).sqrt()
    }
}

/// Find the geometric midpoint of a path (point at half the total path length)
/// This is more accurate than array index midpoint for paths with varying segment lengths
pub fn geometric_midpoint(points: &[Point]) -> Option<Point> {
    if points.is_empty() {
        return None;
    }
    if points.len() == 1 {
        return Some(points[0]);
    }

    // Calculate total path length
    let mut total_length = 0.0;
    for i in 1..points.len() {
        total_length += points[i - 1].distance_to(&points[i]);
    }

    if total_length == 0.0 {
        return Some(points[0]);
    }

    // Find the point at half the total distance
    let target_distance = total_length / 2.0;
    let mut accumulated = 0.0;

    for i in 1..points.len() {
        let segment_length = points[i - 1].distance_to(&points[i]);
        if accumulated + segment_length >= target_distance {
            // The midpoint is on this segment
            let remaining = target_distance - accumulated;
            let t = remaining / segment_length;
            return Some(Point::new(
                points[i - 1].x + t * (points[i].x - points[i - 1].x),
                points[i - 1].y + t * (points[i].y - points[i - 1].y),
            ));
        }
        accumulated += segment_length;
    }

    // Fallback to last point (shouldn't happen)
    Some(points[points.len() - 1])
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
    /// Parent node ID for compound graph layout (alternative to nesting in children)
    pub parent_id: Option<String>,
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
            parent_id: None,
            padding: Padding::default(),
            is_dummy: false,
            dummy_edge_id: None,
            metadata: HashMap::new(),
        }
    }

    pub fn with_parent(mut self, parent_id: impl Into<String>) -> Self {
        self.parent_id = Some(parent_id.into());
        self
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

    /// Set children for compound nodes (subgraphs)
    pub fn with_children(mut self, children: Vec<LayoutNode>) -> Self {
        self.children = children;
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
            parent_id: None,
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
    /// Label width (estimated from text)
    pub label_width: f64,
    /// Label height (estimated from text)
    pub label_height: f64,
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
            label_width: 0.0,
            label_height: 0.0,
            weight: 1,
            reversed: false,
            metadata: HashMap::new(),
        }
    }

    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        let text = label.into();
        // Estimate label dimensions (same as renderer: 16px font, 0.6 char ratio)
        self.label_width = text.len() as f64 * 16.0 * 0.6 + 4.0; // + padding
        self.label_height = 16.0 * 1.1 + 4.0; // + padding
        self.label = Some(text);
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

/// Layout ranker algorithm
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LayoutRanker {
    /// Network simplex algorithm (default, more optimization)
    #[default]
    NetworkSimplex,
    /// Longest path algorithm (simpler, used by mermaid's tight-tree)
    LongestPath,
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
    /// Ranker algorithm to use
    pub ranker: LayoutRanker,
}

impl Default for LayoutOptions {
    fn default() -> Self {
        Self {
            direction: LayoutDirection::TopToBottom,
            node_spacing: 50.0,
            layer_spacing: 50.0,
            padding: Padding::uniform(20.0),
            ranker: LayoutRanker::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_point_distance() {
        let p1 = Point::new(0.0, 0.0);
        let p2 = Point::new(3.0, 4.0);
        assert!((p1.distance_to(&p2) - 5.0).abs() < 0.001);
    }

    #[test]
    fn test_geometric_midpoint_empty() {
        assert!(geometric_midpoint(&[]).is_none());
    }

    #[test]
    fn test_geometric_midpoint_single_point() {
        let points = vec![Point::new(10.0, 20.0)];
        let mid = geometric_midpoint(&points).unwrap();
        assert!((mid.x - 10.0).abs() < 0.001);
        assert!((mid.y - 20.0).abs() < 0.001);
    }

    #[test]
    fn test_geometric_midpoint_two_points() {
        let points = vec![Point::new(0.0, 0.0), Point::new(100.0, 0.0)];
        let mid = geometric_midpoint(&points).unwrap();
        assert!((mid.x - 50.0).abs() < 0.001);
        assert!((mid.y - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_geometric_midpoint_unequal_segments() {
        // Path: A(0,0) -> B(10,0) -> C(100,0)
        // Total length = 10 + 90 = 100
        // Midpoint should be at distance 50 from A
        // That's 10 units on segment AB, then 40 units on segment BC
        // So midpoint = (10 + 40, 0) = (50, 0)
        let points = vec![
            Point::new(0.0, 0.0),
            Point::new(10.0, 0.0),
            Point::new(100.0, 0.0),
        ];
        let mid = geometric_midpoint(&points).unwrap();
        assert!(
            (mid.x - 50.0).abs() < 0.001,
            "Expected x=50, got x={}",
            mid.x
        );
        assert!((mid.y - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_geometric_midpoint_vertical_path() {
        // Vertical path with equal segments
        let points = vec![
            Point::new(0.0, 0.0),
            Point::new(0.0, 50.0),
            Point::new(0.0, 100.0),
        ];
        let mid = geometric_midpoint(&points).unwrap();
        assert!((mid.x - 0.0).abs() < 0.001);
        assert!(
            (mid.y - 50.0).abs() < 0.001,
            "Expected y=50, got y={}",
            mid.y
        );
    }

    #[test]
    fn test_geometric_midpoint_diagonal() {
        // Diagonal path: (0,0) to (100,100)
        // Length = sqrt(100^2 + 100^2) = 141.42
        // Midpoint = (50, 50)
        let points = vec![Point::new(0.0, 0.0), Point::new(100.0, 100.0)];
        let mid = geometric_midpoint(&points).unwrap();
        assert!(
            (mid.x - 50.0).abs() < 0.001,
            "Expected x=50, got x={}",
            mid.x
        );
        assert!(
            (mid.y - 50.0).abs() < 0.001,
            "Expected y=50, got y={}",
            mid.y
        );
    }
}
