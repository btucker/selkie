//! Sequence diagram renderer

use crate::diagrams::sequence::{LineType, ParticipantType, SequenceDb};
use crate::error::Result;
use crate::render::svg::{Attrs, RenderConfig, SvgDocument, SvgElement};

/// Render a sequence diagram to SVG
pub fn render_sequence(db: &SequenceDb, config: &RenderConfig) -> Result<String> {
    let mut doc = SvgDocument::new();

    // Layout constants (matching mermaid.js default theme)
    let actor_spacing = 200.0;
    let actor_width = 150.0; // mermaid.js uses 150
    let actor_height = 65.0; // mermaid.js uses 65
    let message_spacing = 44.0; // mermaid.js uses ~44px
    let margin_top = 10.0; // Small top margin (viewBox offset handles visual padding)
    let _margin_left = 0.0; // No left margin (handled by viewBox offset)
    let actor_box_padding = 0.0; // No padding - full width box

    // Get actors in order
    let actors = db.get_actors_in_order();
    let messages = db.get_messages();

    if actors.is_empty() {
        // Empty diagram
        doc.set_size(400.0, 200.0);
        if !db.diagram_title.is_empty() {
            let title_elem = SvgElement::Text {
                x: 200.0,
                y: 30.0,
                content: db.diagram_title.clone(),
                attrs: Attrs::new()
                    .with_attr("text-anchor", "middle")
                    .with_class("sequence-title")
                    .with_attr("font-size", "20")
                    .with_attr("font-weight", "bold"),
            };
            doc.add_element(title_elem);
        }
        return Ok(doc.to_string());
    }

    // Calculate dimensions (mimicking mermaid.js layout)
    // mermaid.js uses negative viewBox offset for visual padding
    let content_width = (actors.len() as f64 - 1.0) * actor_spacing + actor_width;
    let content_height = actor_height  // Top actor
        + message_spacing  // Gap before first message
        + (messages.len() as f64) * message_spacing  // Messages
        + 20.0  // Gap after last message to bottom actor
        + actor_height; // Bottom actor

    // Add visual padding via width/viewBox (mermaid.js style)
    let width = content_width + 100.0; // Total viewBox width with padding
    let height = content_height + 20.0; // Total viewBox height with padding

    doc.set_size(width, height);

    // Add theme styles
    if config.embed_css {
        doc.add_style(&config.theme.generate_css());
        doc.add_style(&generate_sequence_css(&config.theme));
    }

    // Add arrow markers
    doc.add_defs(vec![
        create_arrow_marker("arrow-filled", true),
        create_arrow_marker("arrow-open", false),
        create_cross_marker(),
    ]);

    // Title offset
    let title_offset = if !db.diagram_title.is_empty() {
        40.0
    } else {
        0.0
    };

    // Render title
    if !db.diagram_title.is_empty() {
        let title_elem = SvgElement::Text {
            x: width / 2.0,
            y: 25.0,
            content: db.diagram_title.clone(),
            attrs: Attrs::new()
                .with_attr("text-anchor", "middle")
                .with_class("sequence-title")
                .with_attr("font-size", "20")
                .with_attr("font-weight", "bold"),
        };
        doc.add_element(title_elem);
    }

    // Calculate actor positions (with padding offset for visual alignment)
    let padding_x = 50.0; // Horizontal padding offset
    let padding_y = margin_top; // Vertical padding offset

    let actor_y = padding_y + title_offset;
    let lifeline_start_y = actor_y + actor_height;
    // Bottom actor position: after all messages + 20px gap
    let bottom_actor_y =
        lifeline_start_y + message_spacing + (messages.len() as f64) * message_spacing + 20.0;
    let lifeline_end_y = bottom_actor_y;

    // Create actor position map
    let mut actor_positions: std::collections::HashMap<String, f64> =
        std::collections::HashMap::new();
    let mut actor_centers: Vec<f64> = Vec::with_capacity(actors.len());
    let mut actor_index: std::collections::HashMap<String, usize> =
        std::collections::HashMap::new();
    for (i, actor) in actors.iter().enumerate() {
        let center_x = padding_x + (i as f64) * actor_spacing + actor_width / 2.0;
        actor_positions.insert(actor.name.clone(), center_x);
        actor_centers.push(center_x);
        actor_index.insert(actor.name.clone(), i);
    }

    // Render actors at top and bottom
    for (i, actor) in actors.iter().enumerate() {
        let x = padding_x + (i as f64) * actor_spacing;
        let center_x = x + actor_width / 2.0;

        // Top actor box/stick figure
        let top_actor = render_actor(
            center_x,
            actor_y,
            actor_width,
            actor_height,
            &actor.description,
            actor.actor_type,
            actor_box_padding,
        );
        doc.add_element(top_actor);

        // Lifeline (mermaid.js style)
        let lifeline = SvgElement::Line {
            x1: center_x,
            y1: lifeline_start_y,
            x2: center_x,
            y2: lifeline_end_y,
            attrs: Attrs::new()
                .with_attr("stroke-width", "0.5px")
                .with_class("actor-line"),
        };
        doc.add_element(lifeline);

        // Bottom actor box/stick figure
        let bottom_actor = render_actor(
            center_x,
            bottom_actor_y,
            actor_width,
            actor_height,
            &actor.description,
            actor.actor_type,
            actor_box_padding,
        );
        doc.add_element(bottom_actor);
    }

    // Render messages and notes in timeline order
    let mut events: Vec<(usize, TimelineEvent)> = Vec::new();
    for message in messages {
        events.push((message.order, TimelineEvent::Message(message)));
    }
    for note in db.get_notes() {
        events.push((note.order, TimelineEvent::Note(note)));
    }
    events.sort_by_key(|(order, _)| *order);

    let mut current_y = lifeline_start_y + message_spacing;
    let mut last_message_y: Option<f64> = None;
    let mut activation_stacks: std::collections::HashMap<String, Vec<f64>> =
        std::collections::HashMap::new();
    let fragment_left = padding_x;
    let fragment_width = content_width;
    let mut fragment_stack: Vec<FragmentState> = Vec::new();
    for (_, event) in events {
        match event {
            TimelineEvent::Message(message) => match message.message_type {
                LineType::ActiveStart => {
                    if let Some(actor) = message.message.split_whitespace().next() {
                        let start_y = last_message_y.unwrap_or(current_y);
                        activation_stacks
                            .entry(actor.to_string())
                            .or_default()
                            .push(start_y);
                    }
                }
                LineType::ActiveEnd => {
                    if let Some(actor) = message.message.split_whitespace().next() {
                        if let Some(stack) = activation_stacks.get_mut(actor) {
                            if let Some(start_y) = stack.pop() {
                                if let Some(&actor_x) = actor_positions.get(actor) {
                                    let end_y = last_message_y.unwrap_or(current_y);
                                    let activation = render_activation(actor_x, start_y, end_y);
                                    doc.add_element(activation);
                                }
                            }
                        }
                    }
                }
                LineType::LoopStart
                | LineType::AltStart
                | LineType::OptStart
                | LineType::ParStart
                | LineType::CriticalStart
                | LineType::BreakStart
                | LineType::RectStart => {
                    let kind = FragmentKind::from_message_type(message.message_type);
                    let start_y = current_y - message_spacing / 2.0;
                    fragment_stack.push(FragmentState {
                        start_y,
                        kind,
                        label: message.message.trim().to_string(),
                        min_actor_idx: None,
                        max_actor_idx: None,
                        color: if matches!(kind, FragmentKind::Rect) {
                            if message.message.is_empty() {
                                None
                            } else {
                                Some(message.message.clone())
                            }
                        } else {
                            None
                        },
                    });
                    current_y += message_spacing;
                }
                LineType::AltElse | LineType::ParAnd | LineType::CriticalOption => {
                    if let Some(fragment) = fragment_stack.last() {
                        let label = message.message.trim();
                        let depth = fragment_stack.len().saturating_sub(1);
                        let (frame_x, frame_width) = fragment_bounds_for_state(
                            fragment,
                            fragment_left,
                            fragment_width,
                            depth,
                            &actor_centers,
                            actor_width,
                        );
                        let divider =
                            render_fragment_divider(frame_x, frame_width, current_y, true);
                        doc.add_cluster(divider);
                        let label_elements = render_fragment_label(
                            FragmentKind::from_message_type(message.message_type),
                            frame_x,
                            frame_width,
                            current_y,
                            label,
                        );
                        for element in label_elements {
                            doc.add_element(element);
                        }
                    }
                    current_y += message_spacing;
                }
                LineType::LoopEnd
                | LineType::AltEnd
                | LineType::OptEnd
                | LineType::ParEnd
                | LineType::CriticalEnd
                | LineType::BreakEnd
                | LineType::RectEnd => {
                    if let Some(fragment) = fragment_stack.pop() {
                        let end_y = current_y - message_spacing / 2.0;
                        let depth = fragment_stack.len();
                        let (frame_x, frame_width) = fragment_bounds_for_state(
                            &fragment,
                            fragment_left,
                            fragment_width,
                            depth,
                            &actor_centers,
                            actor_width,
                        );
                        let frame = render_fragment_frame(
                            frame_x,
                            frame_width,
                            fragment.start_y,
                            end_y,
                            fragment.color.as_deref(),
                        );
                        doc.add_cluster(frame);
                        let label_elements = render_fragment_label(
                            fragment.kind,
                            frame_x,
                            frame_width,
                            fragment.start_y,
                            &fragment.label,
                        );
                        for element in label_elements {
                            doc.add_element(element);
                        }
                    }
                    current_y += message_spacing;
                }
                LineType::Autonumber => {}
                _ => {
                    if let (Some(from), Some(to)) = (&message.from, &message.to) {
                        if let (Some(&from_x), Some(&to_x)) =
                            (actor_positions.get(from), actor_positions.get(to))
                        {
                            let msg_element = render_message(
                                from_x,
                                to_x,
                                current_y,
                                &message.message,
                                message.message_type,
                            );
                            doc.add_element(msg_element);
                        }
                    }
                    if let (Some(from_idx), Some(to_idx)) = (
                        message
                            .from
                            .as_ref()
                            .and_then(|name| actor_index.get(name).copied()),
                        message
                            .to
                            .as_ref()
                            .and_then(|name| actor_index.get(name).copied()),
                    ) {
                        let min_idx = from_idx.min(to_idx);
                        let max_idx = from_idx.max(to_idx);
                        for fragment in &mut fragment_stack {
                            fragment.update_bounds(min_idx, max_idx);
                        }
                    }
                    last_message_y = Some(current_y);
                    current_y += message_spacing;
                }
            },
            TimelineEvent::Note(note) => {
                if let Some(&actor_x) = actor_positions.get(&note.actor) {
                    let span_x = note
                        .actor_to
                        .as_ref()
                        .and_then(|actor| actor_positions.get(actor))
                        .copied();
                    let note_element =
                        render_note(actor_x, span_x, current_y, &note.message, note.placement);
                    doc.add_element(note_element);
                }
                if let Some(actor_idx) = actor_index.get(&note.actor).copied() {
                    let mut min_idx = actor_idx;
                    let mut max_idx = actor_idx;
                    if let Some(other) = note
                        .actor_to
                        .as_ref()
                        .and_then(|name| actor_index.get(name).copied())
                    {
                        min_idx = min_idx.min(other);
                        max_idx = max_idx.max(other);
                    }
                    for fragment in &mut fragment_stack {
                        fragment.update_bounds(min_idx, max_idx);
                    }
                }
                last_message_y = Some(current_y);
                current_y += message_spacing;
            }
        }
    }

    Ok(doc.to_string())
}

/// Check if a message type is a control structure
enum TimelineEvent<'a> {
    Message(&'a crate::diagrams::sequence::Message),
    Note(&'a crate::diagrams::sequence::Note),
}

#[derive(Clone, Copy)]
enum FragmentKind {
    Loop,
    Alt,
    Opt,
    Par,
    Critical,
    Break,
    Rect,
    Else,
    And,
    Option,
}

impl FragmentKind {
    fn from_message_type(msg_type: LineType) -> Self {
        match msg_type {
            LineType::LoopStart | LineType::LoopEnd => FragmentKind::Loop,
            LineType::AltStart | LineType::AltEnd => FragmentKind::Alt,
            LineType::AltElse => FragmentKind::Else,
            LineType::OptStart | LineType::OptEnd => FragmentKind::Opt,
            LineType::ParStart | LineType::ParEnd => FragmentKind::Par,
            LineType::ParAnd => FragmentKind::And,
            LineType::CriticalStart | LineType::CriticalEnd => FragmentKind::Critical,
            LineType::CriticalOption => FragmentKind::Option,
            LineType::BreakStart | LineType::BreakEnd => FragmentKind::Break,
            LineType::RectStart | LineType::RectEnd => FragmentKind::Rect,
            _ => FragmentKind::Loop,
        }
    }
}

struct FragmentState {
    start_y: f64,
    kind: FragmentKind,
    label: String,
    min_actor_idx: Option<usize>,
    max_actor_idx: Option<usize>,
    color: Option<String>,
}

impl FragmentState {
    fn update_bounds(&mut self, min_idx: usize, max_idx: usize) {
        self.min_actor_idx = Some(
            self.min_actor_idx
                .map_or(min_idx, |value| value.min(min_idx)),
        );
        self.max_actor_idx = Some(
            self.max_actor_idx
                .map_or(max_idx, |value| value.max(max_idx)),
        );
    }
}

/// Render an actor (participant box or stick figure)
fn render_actor(
    center_x: f64,
    top_y: f64,
    width: f64,
    height: f64,
    label: &str,
    actor_type: ParticipantType,
    padding: f64,
) -> SvgElement {
    let mut children = Vec::new();

    match actor_type {
        ParticipantType::Actor => {
            // Stick figure
            let head_radius = 10.0;
            let body_length = 15.0;
            let arm_length = 12.0;
            let leg_length = 12.0;

            // Head
            children.push(SvgElement::Circle {
                cx: center_x,
                cy: top_y + head_radius,
                r: head_radius,
                attrs: Attrs::new().with_fill("none").with_stroke_width(2.0),
            });

            // Body
            children.push(SvgElement::Line {
                x1: center_x,
                y1: top_y + head_radius * 2.0,
                x2: center_x,
                y2: top_y + head_radius * 2.0 + body_length,
                attrs: Attrs::new() /* stroke via CSS */
                    .with_stroke_width(2.0),
            });

            // Arms
            children.push(SvgElement::Line {
                x1: center_x - arm_length,
                y1: top_y + head_radius * 2.0 + 5.0,
                x2: center_x + arm_length,
                y2: top_y + head_radius * 2.0 + 5.0,
                attrs: Attrs::new() /* stroke via CSS */
                    .with_stroke_width(2.0),
            });

            // Left leg
            children.push(SvgElement::Line {
                x1: center_x,
                y1: top_y + head_radius * 2.0 + body_length,
                x2: center_x - 8.0,
                y2: top_y + head_radius * 2.0 + body_length + leg_length,
                attrs: Attrs::new() /* stroke via CSS */
                    .with_stroke_width(2.0),
            });

            // Right leg
            children.push(SvgElement::Line {
                x1: center_x,
                y1: top_y + head_radius * 2.0 + body_length,
                x2: center_x + 8.0,
                y2: top_y + head_radius * 2.0 + body_length + leg_length,
                attrs: Attrs::new() /* stroke via CSS */
                    .with_stroke_width(2.0),
            });

            // Label below
            children.push(SvgElement::Text {
                x: center_x,
                y: top_y + height + 15.0,
                content: label.to_string(),
                attrs: Attrs::new()
                    .with_attr("text-anchor", "middle")
                    .with_class("actor-label")
                    .with_attr("font-size", "12"),
            });
        }
        ParticipantType::Database => {
            // Cylinder shape
            let cylinder_height = height - 10.0;
            let ellipse_ry = 6.0;

            // Cylinder body path
            let path = format!(
                "M {} {} L {} {} A {} {} 0 0 0 {} {} L {} {} A {} {} 0 0 0 {} {} Z",
                center_x - width / 2.0 + padding,
                top_y + ellipse_ry,
                center_x - width / 2.0 + padding,
                top_y + cylinder_height - ellipse_ry,
                (width - padding * 2.0) / 2.0,
                ellipse_ry,
                center_x + width / 2.0 - padding,
                top_y + cylinder_height - ellipse_ry,
                center_x + width / 2.0 - padding,
                top_y + ellipse_ry,
                (width - padding * 2.0) / 2.0,
                ellipse_ry,
                center_x - width / 2.0 + padding,
                top_y + ellipse_ry
            );

            children.push(SvgElement::Path {
                d: path,
                attrs: Attrs::new()
                    .with_class("actor")
                    .with_class("actor-box")
                    .with_stroke_width(1.0)
                    .with_class("actor-box"),
            });

            // Top ellipse
            children.push(SvgElement::Ellipse {
                cx: center_x,
                cy: top_y + ellipse_ry,
                rx: (width - padding * 2.0) / 2.0,
                ry: ellipse_ry,
                attrs: Attrs::new()
                    .with_class("actor")
                    .with_class("actor-box")
                    .with_stroke_width(1.0),
            });

            // Label
            children.push(SvgElement::Text {
                x: center_x,
                y: top_y + cylinder_height / 2.0 + 4.0,
                content: label.to_string(),
                attrs: Attrs::new()
                    .with_attr("text-anchor", "middle")
                    .with_class("actor-label")
                    .with_attr("font-size", "12"),
            });
        }
        _ => {
            // Default participant box (mermaid.js style)
            children.push(SvgElement::Rect {
                x: center_x - width / 2.0 + padding,
                y: top_y,
                width: width - padding * 2.0,
                height,
                rx: Some(3.0),
                ry: Some(3.0),
                attrs: Attrs::new()
                    .with_stroke_width(1.0)
                    .with_class("actor")
                    .with_class("actor-box"),
            });

            // Label (centered, mermaid.js style)
            children.push(SvgElement::Text {
                x: center_x,
                y: top_y + height / 2.0,
                content: label.to_string(),
                attrs: Attrs::new()
                    .with_attr("text-anchor", "middle")
                    .with_attr("dominant-baseline", "central")
                    .with_class("actor")
                    .with_class("actor-box")
                    .with_attr("font-size", "16"),
            });
        }
    }

    SvgElement::Group {
        children,
        attrs: Attrs::new().with_class("actor"),
    }
}

/// Render a message between two actors
fn render_message(from_x: f64, to_x: f64, y: f64, label: &str, msg_type: LineType) -> SvgElement {
    let mut children = Vec::new();

    let (is_dotted, marker_id) = match msg_type {
        LineType::Solid => (false, Some("arrow-filled")),
        LineType::Dotted => (true, Some("arrow-filled")),
        LineType::SolidOpen => (false, None),
        LineType::DottedOpen => (true, None),
        LineType::SolidCross => (false, Some("arrow-cross")),
        LineType::DottedCross => (true, Some("arrow-cross")),
        LineType::SolidPoint | LineType::DottedPoint => {
            // Self-message (loop back to same actor)
            return render_self_message(from_x, y, label, msg_type == LineType::DottedPoint);
        }
        _ => (false, Some("arrow-filled")),
    };

    // Determine direction
    let is_self_message = (from_x - to_x).abs() < 1.0;
    if is_self_message {
        return render_self_message(from_x, y, label, is_dotted);
    }

    // Message line
    let mut line_attrs = Attrs::new()
        .with_stroke_width(1.0)
        .with_class("message-line");
    if let Some(marker_id) = marker_id {
        line_attrs = line_attrs.with_attr("marker-end", &format!("url(#{})", marker_id));
    }

    if is_dotted {
        line_attrs = line_attrs.with_stroke_dasharray("5,5");
    }

    children.push(SvgElement::Line {
        x1: from_x,
        y1: y,
        x2: to_x,
        y2: y,
        attrs: line_attrs,
    });

    // Message label
    let label_x = (from_x + to_x) / 2.0;
    let label_y = y - 10.0;

    children.push(SvgElement::Text {
        x: label_x,
        y: label_y,
        content: label.to_string(),
        attrs: Attrs::new()
            .with_attr("text-anchor", "middle")
            .with_class("message-label")
            .with_attr("font-size", "16"),
    });

    SvgElement::Group {
        children,
        attrs: Attrs::new().with_class("message"),
    }
}

fn render_activation(actor_x: f64, start_y: f64, end_y: f64) -> SvgElement {
    let width = 10.0;
    let height = (end_y - start_y).max(1.0);

    SvgElement::Rect {
        x: actor_x - width / 2.0,
        y: start_y,
        width,
        height,
        rx: Some(1.0),
        ry: Some(1.0),
        attrs: Attrs::new().with_class("activation"),
    }
}

fn render_fragment_frame(
    x: f64,
    width: f64,
    start_y: f64,
    end_y: f64,
    fill: Option<&str>,
) -> SvgElement {
    let height = (end_y - start_y).max(1.0);
    let mut attrs = Attrs::new().with_class("loopLine");
    if let Some(color) = fill {
        attrs = attrs.with_fill(color);
    } else {
        attrs = attrs.with_fill("none");
    }
    SvgElement::Rect {
        x,
        y: start_y,
        width,
        height,
        rx: Some(3.0),
        ry: Some(3.0),
        attrs,
    }
}

fn render_fragment_divider(x: f64, width: f64, y: f64, dashed: bool) -> SvgElement {
    let mut attrs = Attrs::new().with_class("loopLine");
    if dashed {
        attrs = attrs.with_stroke_dasharray("3,3");
    }
    SvgElement::Line {
        x1: x,
        y1: y,
        x2: x + width,
        y2: y,
        attrs,
    }
}

fn fragment_bounds(left: f64, width: f64, depth: usize) -> (f64, f64) {
    let inset = depth as f64 * 10.0;
    let frame_x = left + inset;
    let frame_width = (width - inset * 2.0).max(20.0);
    (frame_x, frame_width)
}

fn fragment_bounds_for_state(
    fragment: &FragmentState,
    left: f64,
    width: f64,
    depth: usize,
    actor_centers: &[f64],
    actor_width: f64,
) -> (f64, f64) {
    let (mut frame_x, mut frame_width) =
        if let (Some(min_idx), Some(max_idx)) = (fragment.min_actor_idx, fragment.max_actor_idx) {
            let min_center = actor_centers[min_idx];
            let max_center = actor_centers[max_idx];
            let left = min_center - actor_width / 2.0 - 10.0;
            let right = max_center + actor_width / 2.0 + 10.0;
            (left, (right - left).max(20.0))
        } else {
            fragment_bounds(left, width, 0)
        };

    let inset = depth as f64 * 10.0;
    frame_x += inset;
    frame_width = (frame_width - inset * 2.0).max(20.0);
    (frame_x, frame_width)
}

fn render_fragment_label(
    kind: FragmentKind,
    x: f64,
    width: f64,
    y: f64,
    text: &str,
) -> Vec<SvgElement> {
    let mut elements = Vec::new();
    let label_height = 20.0;
    let label_y = y;

    let (prefix, condition) = match kind {
        FragmentKind::Else | FragmentKind::And | FragmentKind::Option => (None, Some(text)),
        _ => (
            Some(fragment_prefix(kind)),
            if text.is_empty() { None } else { Some(text) },
        ),
    };

    if let Some(prefix) = prefix {
        let label_width = (prefix.len() as f64 * 7.0 + 16.0).max(50.0);
        let label_x = x + 10.0;
        let notch_y = label_y + label_height;
        let notch_mid_y = label_y + label_height * 0.65;
        let notch_x = label_x + label_width * 0.84;
        let points = vec![
            crate::layout::Point {
                x: label_x,
                y: label_y,
            },
            crate::layout::Point {
                x: label_x + label_width,
                y: label_y,
            },
            crate::layout::Point {
                x: label_x + label_width,
                y: notch_mid_y,
            },
            crate::layout::Point {
                x: notch_x,
                y: notch_y,
            },
            crate::layout::Point {
                x: label_x,
                y: notch_y,
            },
        ];

        elements.push(SvgElement::Polygon {
            points,
            attrs: Attrs::new().with_class("labelBox"),
        });
        elements.push(SvgElement::Text {
            x: label_x + label_width / 2.0,
            y: label_y + 13.0,
            content: prefix.to_string(),
            attrs: Attrs::new()
                .with_attr("text-anchor", "middle")
                .with_class("labelText")
                .with_attr("font-size", "16"),
        });
    }

    if let Some(condition) = condition {
        let condition_text = condition.trim();
        if !condition_text.is_empty() {
            let wrapped = if condition_text.starts_with('[') && condition_text.ends_with(']') {
                condition_text.to_string()
            } else {
                format!("[{}]", condition_text)
            };
            elements.push(SvgElement::Text {
                x: x + width / 2.0,
                y: label_y + label_height - 2.0,
                content: wrapped,
                attrs: Attrs::new()
                    .with_attr("text-anchor", "middle")
                    .with_class("loopText")
                    .with_attr("font-size", "16"),
            });
        }
    }

    elements
}

fn fragment_prefix(kind: FragmentKind) -> &'static str {
    match kind {
        FragmentKind::Loop => "loop",
        FragmentKind::Alt => "alt",
        FragmentKind::Opt => "opt",
        FragmentKind::Par => "par",
        FragmentKind::Critical => "critical",
        FragmentKind::Break => "break",
        FragmentKind::Rect => "rect",
        FragmentKind::Else => "else",
        FragmentKind::And => "and",
        FragmentKind::Option => "option",
    }
}

/// Render a self-message (message to the same actor)
fn render_self_message(x: f64, y: f64, label: &str, is_dotted: bool) -> SvgElement {
    let mut children = Vec::new();
    let loop_width = 40.0;
    let loop_height = 30.0;

    let path = format!(
        "M {} {} L {} {} L {} {} L {} {}",
        x,
        y,
        x + loop_width,
        y,
        x + loop_width,
        y + loop_height,
        x,
        y + loop_height
    );

    let mut path_attrs = Attrs::new()
        .with_fill("none")
        .with_stroke_width(1.0)
        .with_class("message-line")
        .with_attr("marker-end", "url(#arrow-filled)");

    if is_dotted {
        path_attrs = path_attrs.with_stroke_dasharray("5,5");
    }

    children.push(SvgElement::Path {
        d: path,
        attrs: path_attrs,
    });

    children.push(SvgElement::Text {
        x: x + loop_width + 5.0,
        y: y + loop_height / 2.0,
        content: label.to_string(),
        attrs: Attrs::new()
            .with_attr("text-anchor", "start")
            .with_class("message-label")
            .with_attr("font-size", "16"),
    });

    SvgElement::Group {
        children,
        attrs: Attrs::new().with_class("message self-message"),
    }
}

/// Render a note
fn render_note(
    actor_x: f64,
    span_x: Option<f64>,
    y: f64,
    message: &str,
    placement: crate::diagrams::sequence::Placement,
) -> SvgElement {
    use crate::diagrams::sequence::Placement;

    let font_size = 11.0_f64;
    let line_height = (font_size * 1.2_f64).round();
    let text_padding = 10.0;
    let min_note_height = 40.0;
    let fold_size = 8.0;
    let min_note_width = 100.0;

    let line_count = count_text_lines(message);
    let note_height = (line_count as f64 * line_height + text_padding * 2.0).max(min_note_height);

    let (note_width, x_center) = match placement {
        Placement::Over => {
            if let Some(span_x) = span_x {
                let span = (span_x - actor_x).abs();
                let width = (span + 20.0).max(min_note_width);
                (width, (actor_x + span_x) / 2.0)
            } else {
                (min_note_width, actor_x)
            }
        }
        _ => (min_note_width, actor_x),
    };

    let x = match placement {
        Placement::LeftOf => actor_x - note_width - 20.0,
        Placement::RightOf => actor_x + 20.0,
        Placement::Over => x_center - note_width / 2.0,
    };
    let top_y = y - note_height / 2.0;

    let mut children = Vec::new();

    // Note box with folded corner
    let path = format!(
        "M {} {} L {} {} L {} {} L {} {} L {} {} Z",
        x,
        top_y,
        x + note_width - fold_size,
        top_y,
        x + note_width,
        top_y + fold_size,
        x + note_width,
        top_y + note_height,
        x,
        top_y + note_height
    );

    children.push(SvgElement::Path {
        d: path,
        attrs: Attrs::new()
            .with_class("note")
            .with_stroke_width(1.0)
            .with_class("note-box"),
    });

    // Fold line
    let fold_path = format!(
        "M {} {} L {} {} L {} {}",
        x + note_width - fold_size,
        top_y,
        x + note_width - fold_size,
        top_y + fold_size,
        x + note_width,
        top_y + fold_size
    );

    children.push(SvgElement::Path {
        d: fold_path,
        attrs: Attrs::new()
            .with_fill("none")
            .with_stroke_width(1.0)
            .with_class("note"),
    });

    // Note text
    children.push(SvgElement::Text {
        x: x + note_width / 2.0,
        y: top_y + text_padding + font_size,
        content: message.to_string(),
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

fn count_text_lines(message: &str) -> usize {
    let normalized = message
        .replace("<br />", "\n")
        .replace("<br/>", "\n")
        .replace("<br>", "\n");
    normalized.lines().count().max(1)
}

/// Create an arrow marker definition
fn create_arrow_marker(id: &str, filled: bool) -> SvgElement {
    let path = if filled {
        "M 0 0 L 10 5 L 0 10 z"
    } else {
        "M 0 0 L 10 5 L 0 10"
    };

    // Use class for theming - fill handled by CSS .sequence-marker rule
    let class_name = if filled {
        "sequence-marker-filled"
    } else {
        "sequence-marker-open"
    };

    SvgElement::Marker {
        id: id.to_string(),
        view_box: "0 0 10 10".to_string(),
        ref_x: 10.0,
        ref_y: 5.0,
        marker_width: 6.0,
        marker_height: 6.0,
        orient: "auto".to_string(),
        marker_units: None,
        children: vec![SvgElement::Path {
            d: path.to_string(),
            attrs: Attrs::new().with_class(class_name).with_stroke_width(1.0),
        }],
    }
}

/// Create a cross marker for async messages
fn create_cross_marker() -> SvgElement {
    SvgElement::Marker {
        id: "arrow-cross".to_string(),
        view_box: "0 0 10 10".to_string(),
        ref_x: 5.0,
        ref_y: 5.0,
        marker_width: 6.0,
        marker_height: 6.0,
        orient: "auto".to_string(),
        marker_units: None,
        children: vec![
            SvgElement::Line {
                x1: 0.0,
                y1: 0.0,
                x2: 10.0,
                y2: 10.0,
                attrs: Attrs::new()
                    .with_class("sequence-marker-cross")
                    .with_stroke_width(2.0),
            },
            SvgElement::Line {
                x1: 10.0,
                y1: 0.0,
                x2: 0.0,
                y2: 10.0,
                attrs: Attrs::new()
                    .with_class("sequence-marker-cross")
                    .with_stroke_width(2.0),
            },
        ],
    }
}

fn generate_sequence_css(theme: &crate::render::svg::Theme) -> String {
    format!(
        r#"
.sequence-title {{
  fill: {signal_text_color};
}}

.actor {{
  stroke: {actor_border};
  fill: {actor_bkg};
}}

.actor-box {{
  stroke: {actor_border};
  fill: {actor_bkg};
}}

/* Actor text - no stroke (avoid outlined appearance) */
text.actor, text.actor > tspan, text.actor-box, text.actor-label {{
  fill: {actor_text_color};
  stroke: none;
}}

.actor-line {{
  stroke: {actor_line_color};
  stroke-width: 0.5px;
}}

.messageLine0 {{
  stroke-width: 1.5;
  stroke-dasharray: none;
  stroke: {signal_color};
}}

.messageLine1 {{
  stroke-width: 1.5;
  stroke-dasharray: 2, 2;
  stroke: {signal_color};
}}

.message-line {{
  stroke: {signal_color};
}}

.messageText {{
  fill: {signal_text_color};
  stroke: none;
}}

.message-label {{
  fill: {signal_text_color};
  stroke: none;
}}

.note {{
  stroke: {note_border_color};
  fill: {note_bkg_color};
}}

.noteText, .noteText > tspan {{
  fill: {note_text_color};
  stroke: none;
}}

.note-text, .note-text > tspan {{
  fill: {note_text_color};
  stroke: none;
}}

.activation {{
  fill: {activation_bkg_color};
  stroke: {activation_border_color};
}}

.loopLine {{
  stroke: {actor_border};
  fill: none;
  stroke-width: 2px;
  stroke-dasharray: 2, 2;
}}

.loopText {{
  fill: {signal_text_color};
}}

.labelBox {{
  stroke: {actor_border};
  fill: {actor_bkg};
}}

.sequence-marker-filled {{
  fill: {signal_color};
  stroke: {signal_color};
}}

.sequence-marker-open {{
  fill: none;
  stroke: {signal_color};
}}

.sequence-marker-cross {{
  stroke: {signal_color};
}}
"#,
        signal_text_color = theme.signal_text_color,
        actor_border = theme.actor_border,
        actor_bkg = theme.actor_bkg,
        actor_text_color = theme.actor_text_color,
        actor_line_color = theme.actor_line_color,
        signal_color = theme.signal_color,
        note_border_color = theme.note_border_color,
        note_bkg_color = theme.note_bkg_color,
        note_text_color = theme.note_text_color,
        activation_bkg_color = theme.activation_bkg_color,
        activation_border_color = theme.activation_border_color,
    )
}
