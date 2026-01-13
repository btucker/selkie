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
    for (i, actor) in actors.iter().enumerate() {
        let x = padding_x + (i as f64) * actor_spacing + actor_width / 2.0;
        actor_positions.insert(actor.name.clone(), x);
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
                .with_stroke("#999")
                .with_stroke_width(0.5)
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

    // Render messages
    let mut current_y = lifeline_start_y + message_spacing;

    for message in messages {
        // Skip control structure messages for now (loop, alt, etc.)
        if is_control_message(message.message_type) {
            continue;
        }

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
                current_y += message_spacing;
            }
        }
    }

    // Render notes
    for note in db.get_notes() {
        if let Some(&actor_x) = actor_positions.get(&note.actor) {
            let note_y = lifeline_start_y + message_spacing / 2.0;
            let note_element = render_note(actor_x, note_y, &note.message, note.placement);
            doc.add_element(note_element);
        }
    }

    Ok(doc.to_string())
}

/// Check if a message type is a control structure
fn is_control_message(msg_type: LineType) -> bool {
    matches!(
        msg_type,
        LineType::LoopStart
            | LineType::LoopEnd
            | LineType::AltStart
            | LineType::AltElse
            | LineType::AltEnd
            | LineType::OptStart
            | LineType::OptEnd
            | LineType::ParStart
            | LineType::ParAnd
            | LineType::ParEnd
            | LineType::RectStart
            | LineType::RectEnd
            | LineType::CriticalStart
            | LineType::CriticalOption
            | LineType::CriticalEnd
            | LineType::BreakStart
            | LineType::BreakEnd
            | LineType::ActiveStart
            | LineType::ActiveEnd
            | LineType::Autonumber
    )
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
                attrs: Attrs::new().with_class("actor-box").with_stroke_width(1.0),
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
                    .with_class("activation")
                    .with_stroke("#666")
                    .with_stroke_width(1.0)
                    .with_class("actor actor-box"),
            });

            // Label (centered, mermaid.js style)
            children.push(SvgElement::Text {
                x: center_x,
                y: top_y + height / 2.0,
                content: label.to_string(),
                attrs: Attrs::new()
                    .with_attr("text-anchor", "middle")
                    .with_attr("dominant-baseline", "central")
                    .with_class("actor actor-box")
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
        LineType::Solid => (false, "arrow-filled"),
        LineType::Dotted => (true, "arrow-filled"),
        LineType::SolidOpen => (false, "arrow-open"),
        LineType::DottedOpen => (true, "arrow-open"),
        LineType::SolidCross => (false, "arrow-cross"),
        LineType::DottedCross => (true, "arrow-cross"),
        LineType::SolidPoint | LineType::DottedPoint => {
            // Self-message (loop back to same actor)
            return render_self_message(from_x, y, label, msg_type == LineType::DottedPoint);
        }
        _ => (false, "arrow-filled"),
    };

    // Determine direction
    let is_self_message = (from_x - to_x).abs() < 1.0;
    if is_self_message {
        return render_self_message(from_x, y, label, is_dotted);
    }

    // Message line
    let mut line_attrs = Attrs::new()
        .with_stroke_width(1.0)
        .with_class("message-line")
        .with_attr("marker-end", &format!("url(#{})", marker_id));

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
            .with_attr("font-size", "11"),
    });

    SvgElement::Group {
        children,
        attrs: Attrs::new().with_class("message"),
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
            .with_attr("font-size", "11"),
    });

    SvgElement::Group {
        children,
        attrs: Attrs::new().with_class("message self-message"),
    }
}

/// Render a note
fn render_note(
    actor_x: f64,
    y: f64,
    message: &str,
    placement: crate::diagrams::sequence::Placement,
) -> SvgElement {
    use crate::diagrams::sequence::Placement;

    let note_width = 100.0;
    let note_height = 40.0;
    let fold_size = 8.0;

    let x = match placement {
        Placement::LeftOf => actor_x - note_width - 20.0,
        Placement::RightOf => actor_x + 20.0,
        Placement::Over => actor_x - note_width / 2.0,
    };

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
            .with_class("note")
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
        attrs: Attrs::new().with_fill("none").with_stroke_width(1.0),
    });

    // Note text
    children.push(SvgElement::Text {
        x: x + note_width / 2.0,
        y: y + note_height / 2.0 + 4.0,
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
                attrs: Attrs::new() /* stroke via CSS */
                    .with_stroke_width(2.0),
            },
            SvgElement::Line {
                x1: 10.0,
                y1: 0.0,
                x2: 0.0,
                y2: 10.0,
                attrs: Attrs::new() /* stroke via CSS */
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

.messageText {{
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

.activation {{
  fill: {activation_bkg_color};
  stroke: {activation_border_color};
}}

.loopLine {{
  stroke: {label_box_border_color};
}}

.loopText {{
  fill: {signal_text_color};
}}

.labelBox {{
  stroke: {label_box_border_color};
  fill: {label_box_bkg_color};
}}

.sequence-marker-filled {{
  fill: {signal_color};
  stroke: {signal_color};
}}

.sequence-marker-open {{
  fill: none;
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
        label_box_border_color = theme.label_box_border_color,
        label_box_bkg_color = theme.label_box_bkg_color,
    )
}
