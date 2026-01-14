//! State diagram renderer

use std::collections::HashMap;

use crate::diagrams::state::{Direction, NotePosition, State, StateDb, StateType};
use crate::error::Result;
use crate::layout::Point;
use crate::layout::{
    layout, CharacterSizeEstimator, LayoutDirection, LayoutEdge, LayoutGraph, LayoutNode,
    LayoutOptions, NodeShape, NodeSizeConfig, Padding, SizeEstimator, ToLayoutGraph,
};
use crate::render::svg::edges::build_curved_path;
use crate::render::svg::{Attrs, RenderConfig, SvgDocument, SvgElement};

/// Implement ToLayoutGraph for StateDb to enable proper DAG layout
impl ToLayoutGraph for StateDb {
    fn to_layout_graph(&self, size_estimator: &dyn SizeEstimator) -> Result<LayoutGraph> {
        let config = NodeSizeConfig::default();
        let mut graph = LayoutGraph::new("state");

        // Set layout options from diagram direction
        graph.options = LayoutOptions {
            direction: self.preferred_direction(),
            node_spacing: 50.0,
            layer_spacing: 60.0,
            padding: Padding::uniform(20.0),
        };

        // Determine start/end states based on transitions
        let start_end_states = determine_start_end_states(self);

        // Convert states to layout nodes (sorted for deterministic order)
        let states = self.get_states();
        let mut state_ids: Vec<&String> = states.keys().collect();
        state_ids.sort();

        for id in state_ids {
            let state = states.get(id).unwrap();

            // Determine shape based on state type
            let (shape, label) = match state.state_type {
                StateType::Start => (NodeShape::Circle, None),
                StateType::End => (NodeShape::DoubleCircle, None),
                StateType::Fork | StateType::Join => (NodeShape::Rectangle, None),
                StateType::Choice => (NodeShape::Diamond, None),
                StateType::Divider => (NodeShape::Rectangle, None),
                StateType::Default => {
                    // Check if this is a [*] state that's been classified
                    if id.starts_with("[*]") {
                        if let Some(state_info) = start_end_states.get(id.as_str()) {
                            if state_info.is_start {
                                (NodeShape::Circle, None)
                            } else {
                                (NodeShape::DoubleCircle, None)
                            }
                        } else {
                            (NodeShape::Circle, None)
                        }
                    } else {
                        let desc = state.descriptions.first().map(|s| s.as_str());
                        (NodeShape::RoundedRect, desc.or(Some(id.as_str())))
                    }
                }
            };

            let label_text = label.unwrap_or(if id.starts_with("[*]") { "" } else { &state.id });
            let (width, height) = if id.starts_with("[*]")
                || matches!(state.state_type, StateType::Start | StateType::End)
            {
                // Small fixed size for start/end circles
                (24.0, 24.0)
            } else if matches!(state.state_type, StateType::Fork | StateType::Join) {
                (8.0, 60.0)
            } else {
                size_estimator.estimate_node_size(Some(label_text), shape, &config)
            };

            let mut node = LayoutNode::new(id, width, height).with_shape(shape);

            if !label_text.is_empty() {
                node = node.with_label(label_text);
            }

            // Store state type in metadata
            node.metadata
                .insert("state_type".to_string(), format!("{:?}", state.state_type));

            graph.add_node(node);
        }

        // Convert relations to edges
        for (i, relation) in self.get_relations().iter().enumerate() {
            let edge_id = format!("transition-{}", i);
            let mut edge = LayoutEdge::new(&edge_id, &relation.state1, &relation.state2);

            if let Some(desc) = &relation.description {
                edge = edge.with_label(desc);
            }

            graph.add_edge(edge);
        }

        Ok(graph)
    }

    fn preferred_direction(&self) -> LayoutDirection {
        match self.get_direction() {
            Direction::TopToBottom => LayoutDirection::TopToBottom,
            Direction::BottomToTop => LayoutDirection::BottomToTop,
            Direction::LeftToRight => LayoutDirection::LeftToRight,
            Direction::RightToLeft => LayoutDirection::RightToLeft,
        }
    }
}

/// Render a state diagram to SVG
pub fn render_state(db: &StateDb, config: &RenderConfig) -> Result<String> {
    let mut doc = SvgDocument::new();

    // Layout constants
    let start_end_radius = 12.0;
    let margin = 20.0;

    // Determine which [*] states are starts vs ends based on transitions
    let start_end_states = determine_start_end_states(db);

    let states = db.get_states();

    if states.is_empty() {
        // Empty diagram
        doc.set_size(400.0, 200.0);
        if !db.diagram_title.is_empty() {
            let title_elem = SvgElement::Text {
                x: 200.0,
                y: 30.0,
                content: db.diagram_title.clone(),
                attrs: Attrs::new()
                    .with_attr("text-anchor", "middle")
                    .with_class("state-title")
                    .with_attr("font-size", "20")
                    .with_attr("font-weight", "bold"),
            };
            doc.add_element(title_elem);
        }
        return Ok(doc.to_string());
    }

    // Use proper DAG layout
    let size_estimator = CharacterSizeEstimator::default();
    let layout_input = db.to_layout_graph(&size_estimator)?;
    let layout_result = layout(layout_input)?;

    // Extract positions from layout
    let mut state_positions: HashMap<String, (f64, f64, f64, f64)> = HashMap::new();
    for node in &layout_result.nodes {
        if let (Some(x), Some(y)) = (node.x, node.y) {
            state_positions.insert(node.id.clone(), (x, y, node.width, node.height));
        }
    }

    // Extract edge bend points from layout
    let mut edge_bend_points: HashMap<(String, String), Vec<Point>> = HashMap::new();
    for edge in &layout_result.edges {
        if let (Some(source), Some(target)) = (edge.source(), edge.target()) {
            edge_bend_points.insert(
                (source.to_string(), target.to_string()),
                edge.bend_points.clone(),
            );
        }
    }

    // Title offset
    let title_offset = if !db.diagram_title.is_empty() {
        40.0
    } else {
        0.0
    };

    // Calculate diagram bounds
    let max_width = layout_result.width.unwrap_or(400.0) + margin * 2.0;
    let max_height = layout_result.height.unwrap_or(200.0) + margin * 2.0 + title_offset;

    doc.set_size(max_width, max_height);

    // Add theme styles
    if config.embed_css {
        doc.add_style(&config.theme.generate_css());
        doc.add_style(&generate_state_css());
    }

    // Add arrow marker
    doc.add_defs(vec![create_arrow_marker()]);

    // Render title
    if !db.diagram_title.is_empty() {
        let title_elem = SvgElement::Text {
            x: max_width / 2.0,
            y: 25.0,
            content: db.diagram_title.clone(),
            attrs: Attrs::new()
                .with_attr("text-anchor", "middle")
                .with_class("state-title")
                .with_attr("font-size", "20")
                .with_attr("font-weight", "bold"),
        };
        doc.add_element(title_elem);
    }

    // Sort states for consistent ordering
    let mut sorted_states: Vec<_> = states.iter().collect();
    sorted_states.sort_by(|a, b| a.0.cmp(b.0));

    // Render each state
    for (id, state) in &sorted_states {
        if let Some(&(x, y, width, height)) = state_positions.get(*id) {
            // Check if this [*] state is a start or end
            let state_info = start_end_states.get(id.as_str());
            let is_end_state = state_info.map(|info| !info.is_start).unwrap_or(false);

            let state_elem = render_state_node(
                state,
                x,
                y,
                width,
                height,
                start_end_radius,
                8.0,  // fork_join_width
                60.0, // fork_join_height
                is_end_state,
            );
            doc.add_element(state_elem);

            // Render note if present
            if let Some(note) = &state.note {
                let note_x = match note.position {
                    NotePosition::LeftOf => x - 120.0,
                    NotePosition::RightOf => x + width + 20.0,
                };
                let note_elem = render_note(note_x, y, &note.text);
                doc.add_element(note_elem);
            }
        }
    }

    // Render transitions
    for relation in db.get_relations() {
        if let (Some(&(x1, y1, w1, h1)), Some(&(x2, y2, w2, h2))) = (
            state_positions.get(&relation.state1),
            state_positions.get(&relation.state2),
        ) {
            let state1 = states.get(&relation.state1);
            let state2 = states.get(&relation.state2);

            // Get bend points from layout
            let edge_key = (relation.state1.clone(), relation.state2.clone());
            let bend_points = edge_bend_points.get(&edge_key);

            let transition_elem = render_transition(
                x1,
                y1,
                x2,
                y2,
                w1,
                h1,
                w2,
                h2,
                start_end_radius,
                8.0,  // fork_join_width
                60.0, // fork_join_height
                state1.map(|s| s.state_type),
                state2.map(|s| s.state_type),
                relation.description.as_deref(),
                bend_points,
            );
            doc.add_element(transition_elem);
        }
    }

    Ok(doc.to_string())
}

/// Render a state node based on its type
#[allow(clippy::too_many_arguments)]
fn render_state_node(
    state: &State,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    start_end_radius: f64,
    fork_join_width: f64,
    fork_join_height: f64,
    is_end_state: bool,
) -> SvgElement {
    let mut children = Vec::new();

    match state.state_type {
        StateType::Start => {
            // Filled circle for start state
            children.push(SvgElement::Circle {
                cx: x + width / 2.0,
                cy: y + height / 2.0,
                r: start_end_radius,
                attrs: Attrs::new().with_fill("#333333").with_class("state-start"),
            });
        }
        StateType::End => {
            render_end_state_bullseye(&mut children, x, y, width, height, start_end_radius);
        }
        StateType::Fork | StateType::Join => {
            // Black bar for fork/join
            children.push(SvgElement::Rect {
                x: x + (width - fork_join_width) / 2.0,
                y: y + (height - fork_join_height) / 2.0,
                width: fork_join_width,
                height: fork_join_height.min(height),
                rx: Some(2.0),
                ry: Some(2.0),
                attrs: Attrs::new()
                    .with_fill("#333333")
                    .with_class("state-fork-join"),
            });
        }
        StateType::Choice => {
            // Diamond for choice/decision
            let cx = x + width / 2.0;
            let cy = y + height / 2.0;
            let size = height / 2.0;

            children.push(SvgElement::Polygon {
                points: vec![
                    crate::layout::Point {
                        x: cx,
                        y: cy - size,
                    },
                    crate::layout::Point {
                        x: cx + size,
                        y: cy,
                    },
                    crate::layout::Point {
                        x: cx,
                        y: cy + size,
                    },
                    crate::layout::Point {
                        x: cx - size,
                        y: cy,
                    },
                ],
                attrs: Attrs::new()
                    .with_fill("#ECECFF")
                    .with_stroke("#333333")
                    .with_stroke_width(1.0)
                    .with_class("state-choice"),
            });
        }
        StateType::Divider => {
            // Horizontal line for divider
            children.push(SvgElement::Line {
                x1: x,
                y1: y + height / 2.0,
                x2: x + width,
                y2: y + height / 2.0,
                attrs: Attrs::new()
                    .with_stroke("#333333")
                    .with_stroke_width(2.0)
                    .with_stroke_dasharray("5,5")
                    .with_class("state-divider"),
            });
        }
        StateType::Default => {
            // Check if this is [*] which could be start or end
            if state.id == "[*]" {
                if is_end_state {
                    // End state: double circle (bullseye)
                    render_end_state_bullseye(&mut children, x, y, width, height, start_end_radius);
                } else {
                    // Start state: filled circle
                    children.push(SvgElement::Circle {
                        cx: x + width / 2.0,
                        cy: y + height / 2.0,
                        r: start_end_radius,
                        attrs: Attrs::new().with_fill("#333333").with_class("state-start"),
                    });
                }
            } else {
                // Rounded rectangle for regular state
                children.push(SvgElement::Rect {
                    x,
                    y,
                    width,
                    height,
                    rx: Some(10.0),
                    ry: Some(10.0),
                    attrs: Attrs::new()
                        .with_fill("#ECECFF")
                        .with_stroke("#333333")
                        .with_stroke_width(1.0)
                        .with_class("state-box"),
                });

                // State label
                let label = state.alias.as_ref().unwrap_or(&state.id);
                children.push(SvgElement::Text {
                    x: x + width / 2.0,
                    y: y + height / 2.0 + 5.0,
                    content: label.clone(),
                    attrs: Attrs::new()
                        .with_attr("text-anchor", "middle")
                        .with_class("state-label")
                        .with_attr("font-size", "12"),
                });

                // State descriptions
                if !state.descriptions.is_empty() {
                    let desc_y = y + height / 2.0 + 18.0;
                    for (i, desc) in state.descriptions.iter().enumerate() {
                        children.push(SvgElement::Text {
                            x: x + width / 2.0,
                            y: desc_y + (i as f64) * 14.0,
                            content: desc.clone(),
                            attrs: Attrs::new()
                                .with_attr("text-anchor", "middle")
                                .with_class("state-description")
                                .with_attr("font-size", "10"),
                        });
                    }
                }
            }
        }
    }

    SvgElement::Group {
        children,
        attrs: Attrs::new()
            .with_class("state-node")
            .with_id(&format!("state-{}", state.id)),
    }
}

/// Render a transition between two states
#[allow(clippy::too_many_arguments)]
fn render_transition(
    x1: f64,
    y1: f64,
    x2: f64,
    y2: f64,
    w1: f64,
    h1: f64,
    w2: f64,
    h2: f64,
    start_end_radius: f64,
    _fork_join_width: f64,
    _fork_join_height: f64,
    state1_type: Option<StateType>,
    state2_type: Option<StateType>,
    label: Option<&str>,
    bend_points: Option<&Vec<Point>>,
) -> SvgElement {
    let mut children = Vec::new();

    // Use bend points from layout if available, otherwise calculate connection points
    let (path_d, label_x, label_y) = if let Some(points) = bend_points {
        if !points.is_empty() {
            // Use dagre's bend points to create a curved path
            let curved_path = build_curved_path(points);

            // Calculate label position at midpoint of the path
            let mid_idx = points.len() / 2;
            let (lx, ly) = if points.len() > 1 && mid_idx > 0 {
                let p1 = &points[mid_idx - 1];
                let p2 = &points[mid_idx.min(points.len() - 1)];
                ((p1.x + p2.x) / 2.0, (p1.y + p2.y) / 2.0)
            } else {
                (points[0].x, points[0].y)
            };

            (curved_path, lx, ly)
        } else {
            // Empty bend points - fallback to calculated path
            calculate_fallback_path(
                x1,
                y1,
                x2,
                y2,
                w1,
                h1,
                w2,
                h2,
                start_end_radius,
                state1_type,
                state2_type,
            )
        }
    } else {
        // No bend points - fallback to calculated path
        calculate_fallback_path(
            x1,
            y1,
            x2,
            y2,
            w1,
            h1,
            w2,
            h2,
            start_end_radius,
            state1_type,
            state2_type,
        )
    };

    // Transition path (curved)
    children.push(SvgElement::Path {
        d: path_d,
        attrs: Attrs::new()
            .with_stroke("#333333")
            .with_stroke_width(1.0)
            .with_fill("none")
            .with_attr("marker-end", "url(#arrow)")
            .with_class("transition-path"),
    });

    // Transition label
    if let Some(text) = label {
        if !text.is_empty() {
            children.push(SvgElement::Text {
                x: label_x,
                y: label_y - 5.0,
                content: text.to_string(),
                attrs: Attrs::new()
                    .with_attr("text-anchor", "middle")
                    .with_class("transition-label")
                    .with_attr("font-size", "11"),
            });
        }
    }

    SvgElement::Group {
        children,
        attrs: Attrs::new().with_class("transition"),
    }
}

/// Calculate fallback path when no bend points are available
#[allow(clippy::too_many_arguments)]
fn calculate_fallback_path(
    x1: f64,
    y1: f64,
    x2: f64,
    y2: f64,
    w1: f64,
    h1: f64,
    w2: f64,
    h2: f64,
    start_end_radius: f64,
    state1_type: Option<StateType>,
    state2_type: Option<StateType>,
) -> (String, f64, f64) {
    // Calculate connection points based on state types
    let (start_x, start_y) = calculate_exit_point(
        x1,
        y1,
        w1,
        h1,
        x2 + w2 / 2.0,
        y2 + h2 / 2.0,
        state1_type,
        start_end_radius,
    );

    let (end_x, end_y) = calculate_entry_point(
        x2,
        y2,
        w2,
        h2,
        x1 + w1 / 2.0,
        y1 + h1 / 2.0,
        state2_type,
        start_end_radius,
    );

    // Create a curved path between the two points
    // Add a control point in the middle to create a slight curve
    let mid_x = (start_x + end_x) / 2.0;
    let mid_y = (start_y + end_y) / 2.0;

    // For simple two-point path, create a slight curve
    let points = vec![
        Point::new(start_x, start_y),
        Point::new(mid_x, mid_y),
        Point::new(end_x, end_y),
    ];

    let path = build_curved_path(&points);
    (path, mid_x, mid_y)
}

/// Calculate exit point from a state
#[allow(clippy::too_many_arguments)]
fn calculate_exit_point(
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    target_x: f64,
    target_y: f64,
    state_type: Option<StateType>,
    start_end_radius: f64,
) -> (f64, f64) {
    let cx = x + width / 2.0;
    let cy = y + height / 2.0;

    match state_type {
        Some(StateType::Start) | Some(StateType::End) => {
            // Circle - calculate intersection
            let dx = target_x - cx;
            let dy = target_y - cy;
            let dist = (dx * dx + dy * dy).sqrt();
            if dist > 0.0 {
                (
                    cx + dx / dist * start_end_radius,
                    cy + dy / dist * start_end_radius,
                )
            } else {
                (cx + start_end_radius, cy)
            }
        }
        _ => {
            // Rectangle - calculate edge intersection
            let dx = target_x - cx;
            let dy = target_y - cy;

            if dx.abs() > dy.abs() {
                if dx > 0.0 {
                    (x + width, cy)
                } else {
                    (x, cy)
                }
            } else if dy > 0.0 {
                (cx, y + height)
            } else {
                (cx, y)
            }
        }
    }
}

/// Calculate entry point into a state
#[allow(clippy::too_many_arguments)]
fn calculate_entry_point(
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    source_x: f64,
    source_y: f64,
    state_type: Option<StateType>,
    start_end_radius: f64,
) -> (f64, f64) {
    let cx = x + width / 2.0;
    let cy = y + height / 2.0;

    match state_type {
        Some(StateType::Start) | Some(StateType::End) => {
            // Circle - calculate intersection
            let dx = source_x - cx;
            let dy = source_y - cy;
            let dist = (dx * dx + dy * dy).sqrt();
            if dist > 0.0 {
                (
                    cx + dx / dist * start_end_radius,
                    cy + dy / dist * start_end_radius,
                )
            } else {
                (cx - start_end_radius, cy)
            }
        }
        _ => {
            // Rectangle - calculate edge intersection
            let dx = source_x - cx;
            let dy = source_y - cy;

            if dx.abs() > dy.abs() {
                if dx > 0.0 {
                    (x + width, cy)
                } else {
                    (x, cy)
                }
            } else if dy > 0.0 {
                (cx, y + height)
            } else {
                (cx, y)
            }
        }
    }
}

/// Render a note
fn render_note(x: f64, y: f64, text: &str) -> SvgElement {
    let note_width = 100.0;
    let note_height = 40.0;
    let fold_size = 8.0;

    let mut children = Vec::new();

    // Note box with folded corner
    let path = format!(
        "M {} {} L {} {} L {} {} L {} {} L {} {} Z",
        x,
        y,
        x + note_width - fold_size,
        y,
        x + note_width,
        y + fold_size,
        x + note_width,
        y + note_height,
        x,
        y + note_height
    );

    children.push(SvgElement::Path {
        d: path,
        attrs: Attrs::new()
            .with_fill("#FFFFCC")
            .with_stroke("#333333")
            .with_stroke_width(1.0)
            .with_class("note-box"),
    });

    // Fold line
    let fold_path = format!(
        "M {} {} L {} {} L {} {}",
        x + note_width - fold_size,
        y,
        x + note_width - fold_size,
        y + fold_size,
        x + note_width,
        y + fold_size
    );

    children.push(SvgElement::Path {
        d: fold_path,
        attrs: Attrs::new()
            .with_fill("none")
            .with_stroke("#333333")
            .with_stroke_width(1.0),
    });

    // Note text
    children.push(SvgElement::Text {
        x: x + note_width / 2.0,
        y: y + note_height / 2.0 + 4.0,
        content: text.to_string(),
        attrs: Attrs::new()
            .with_attr("text-anchor", "middle")
            .with_class("note-text")
            .with_attr("font-size", "11"),
    });

    SvgElement::Group {
        children,
        attrs: Attrs::new().with_class("note"),
    }
}

/// Create arrow marker
fn create_arrow_marker() -> SvgElement {
    SvgElement::Marker {
        id: "arrow".to_string(),
        view_box: "0 0 10 10".to_string(),
        ref_x: 10.0,
        ref_y: 5.0,
        marker_width: 6.0,
        marker_height: 6.0,
        orient: "auto".to_string(),
        marker_units: None,
        children: vec![SvgElement::Path {
            d: "M 0 0 L 10 5 L 0 10 z".to_string(),
            attrs: Attrs::new()
                .with_fill("#333333")
                .with_stroke("#333333")
                .with_stroke_width(1.0),
        }],
    }
}

/// Info about whether a [*] state is a start or end state
#[derive(Clone, Copy)]
struct StartEndInfo {
    is_start: bool,
}

/// Determine which [*] states are start vs end states based on transitions
/// Creates unique IDs for each [*] occurrence to handle multiple start/end states
fn determine_start_end_states(db: &StateDb) -> HashMap<&str, StartEndInfo> {
    let mut result = HashMap::new();
    let relations = db.get_relations();

    // Classify states in the states map
    for id in db.get_states().keys() {
        if id.starts_with("[*]_start") {
            result.insert(id.as_str(), StartEndInfo { is_start: true });
        } else if id.starts_with("[*]_end") {
            result.insert(id.as_str(), StartEndInfo { is_start: false });
        } else if id == "[*]" {
            // Single [*] - check if it's source or target in relations
            let is_source = relations.iter().any(|r| r.state1 == "[*]");
            let is_target = relations.iter().any(|r| r.state2 == "[*]");

            // Start if it's only a source, end if it's only a target or both
            result.insert(
                "[*]",
                StartEndInfo {
                    is_start: is_source && !is_target,
                },
            );
        }
    }

    result
}

/// Render end state bullseye (double circle)
fn render_end_state_bullseye(
    children: &mut Vec<SvgElement>,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    start_end_radius: f64,
) {
    // Outer circle
    children.push(SvgElement::Circle {
        cx: x + width / 2.0,
        cy: y + height / 2.0,
        r: start_end_radius,
        attrs: Attrs::new()
            .with_fill("none")
            .with_stroke("#333333")
            .with_stroke_width(2.0)
            .with_class("state-end-outer"),
    });
    // Inner circle
    children.push(SvgElement::Circle {
        cx: x + width / 2.0,
        cy: y + height / 2.0,
        r: start_end_radius * 0.6,
        attrs: Attrs::new()
            .with_fill("#333333")
            .with_class("state-end-inner"),
    });
}

fn generate_state_css() -> String {
    r#"
.state-title {
  fill: #333333;
}

.state-box {
  fill: #ECECFF;
  stroke: #333333;
}

.state-label {
  fill: #333333;
}

.state-description {
  fill: #666666;
}

.state-start {
  fill: #333333;
}

.state-end-outer {
  stroke: #333333;
}

.state-end-inner {
  fill: #333333;
}

.state-fork-join {
  fill: #333333;
}

.state-choice {
  fill: #ECECFF;
  stroke: #333333;
}

.state-divider {
  stroke: #333333;
  stroke-dasharray: 5, 5;
}

.transition-path {
  stroke: #333333;
  fill: none;
}

.transition-label {
  fill: #333333;
}

.note-box {
  fill: #FFFFCC;
  stroke: #333333;
}

.note-text {
  fill: #333333;
}
"#
    .to_string()
}
