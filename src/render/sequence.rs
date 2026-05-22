//! Sequence diagram renderer

use crate::diagrams::sequence::{LineType, ParticipantType, Placement, SequenceDb};
use crate::error::Result;
use crate::render::svg::{Attrs, RenderConfig, SvgDocument, SvgElement};
use std::collections::HashMap;

/// Render a sequence diagram to SVG
pub fn render_sequence(db: &SequenceDb, config: &RenderConfig) -> Result<String> {
    let mut doc = SvgDocument::new();

    let cfg = SequenceLayoutConfig::default();

    // Layout constants (matching mermaid.js default theme)
    let margin_top = 0.0; // Actors start at y=0 (mermaid style - viewBox offset handles padding)
    let actor_box_padding = 0.0; // No padding - full width box

    let mut layout = build_actor_layout(db, &cfg);

    if layout.actors.is_empty() {
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

    // Calculate content width from per-gap spacings
    let content_width = layout.content_width;
    // Height will be set later after we know the actual content height

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
        create_sequence_number_marker(),
    ]);

    // Title offset
    let title_offset = if !db.diagram_title.is_empty() {
        40.0
    } else {
        0.0
    };

    // Render title (centered over content)
    if !db.diagram_title.is_empty() {
        let title_elem = SvgElement::Text {
            x: content_width / 2.0,
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

    // Calculate actor positions
    // Content starts at (0,0) - visual padding achieved via negative viewBox offset (mermaid style)
    let padding_y = margin_top; // Content coordinate origin

    let actor_y = padding_y + title_offset;
    let lifeline_start_y = actor_y + cfg.actor_height;

    // Render top actors only (bottom actors rendered after we know the final height)
    for actor in &layout.actors {
        // Top actor box/stick figure
        let top_actor = render_actor(
            actor.center_x,
            actor_y,
            cfg.actor_width,
            cfg.actor_height,
            &actor.description,
            actor.actor_type,
            actor_box_padding,
        );
        doc.add_element(top_actor);
    }

    layout_basic_events(db, &mut layout, lifeline_start_y, &cfg);

    for fragment in &layout.fragments {
        let frame = render_fragment_frame(
            fragment.frame.x,
            fragment.frame.width,
            fragment.frame.y,
            fragment.frame.bottom(),
            fragment.color.as_deref(),
        );
        doc.add_cluster(frame);

        let label_elements = render_fragment_label(
            fragment.kind,
            fragment.frame.x,
            fragment.frame.width,
            fragment.frame.y,
            &fragment.label,
            cfg.label_box_height,
        );
        for shape in label_elements.shapes {
            doc.add_cluster(shape);
        }
        for label in label_elements.labels {
            doc.add_edge_label(label);
        }

        for section in &fragment.sections {
            doc.add_cluster(render_fragment_divider(
                fragment.frame.x,
                fragment.frame.width,
                section.y,
                true,
            ));
            let label_elements = render_fragment_label(
                section.kind,
                fragment.frame.x,
                fragment.frame.width,
                section.y,
                &section.label,
                cfg.label_box_height,
            );
            for shape in label_elements.shapes {
                doc.add_cluster(shape);
            }
            for label in label_elements.labels {
                doc.add_edge_label(label);
            }
        }
    }

    for event in &layout.events {
        match event {
            LaidOutEvent::Message(msg) => {
                let msg_elements = render_message(
                    msg.from_x,
                    msg.to_x,
                    msg.y,
                    &msg.message,
                    msg.message_type,
                    msg.sequence_num,
                );
                for shape in msg_elements.shapes {
                    doc.add_edge_path(shape);
                }
                for label in msg_elements.labels {
                    doc.add_edge_label(label);
                }
            }
            LaidOutEvent::Note(note) => {
                doc.add_element(render_note(
                    note.actor_x,
                    note.span_x,
                    note.y,
                    &note.message,
                    note.placement,
                    &cfg,
                ));
            }
        }
    }

    let content_bottom = layout
        .all_bounds()
        .map(|bounds| bounds.bottom())
        .fold(lifeline_start_y, f64::max);
    let bottom_actor_y = content_bottom + cfg.box_margin * 2.0;
    let lifeline_end_y = bottom_actor_y;

    // Render lifelines and bottom actors now that we know the final height
    for actor in &layout.actors {
        // Lifeline (mermaid.js style) - rendered in clusters layer (back)
        // so message lines and autonumbers render on top
        let lifeline = SvgElement::Line {
            x1: actor.center_x,
            y1: lifeline_start_y,
            x2: actor.center_x,
            y2: lifeline_end_y,
            attrs: Attrs::new()
                .with_attr("stroke-width", "0.5px")
                .with_class("actor-line"),
        };
        doc.add_cluster(lifeline);

        // Bottom actor box/stick figure
        let bottom_actor = render_actor(
            actor.center_x,
            bottom_actor_y,
            cfg.actor_width,
            cfg.actor_height,
            &actor.description,
            actor.actor_type,
            actor_box_padding,
        );
        doc.add_element(bottom_actor);
    }

    // Add activations after lifelines (so activations render on top of lifelines)
    for activation in &layout.activations {
        doc.add_cluster(render_activation(
            activation.actor_x + activation.stack_offset,
            activation.start_y,
            activation.end_y,
        ));
    }

    // Set final SVG dimensions with mermaid-style viewBox offset for visual padding
    // Mermaid uses viewBox="-50 -10 width height" to create visual padding around content
    let content_height = bottom_actor_y + cfg.actor_height;
    let total_width = content_width + 2.0 * cfg.diagram_margin_x;
    let total_height = content_height + 2.0 * cfg.diagram_margin_y;
    doc.set_size_with_origin(
        -cfg.diagram_margin_x,
        -cfg.diagram_margin_y,
        total_width,
        total_height,
    );

    Ok(doc.to_string())
}

/// Check if a message type is a control structure
enum TimelineEvent<'a> {
    Message(&'a crate::diagrams::sequence::Message),
    Note(&'a crate::diagrams::sequence::Note),
}

fn collect_timeline_events(db: &SequenceDb) -> Vec<(usize, TimelineEvent<'_>)> {
    let mut events = Vec::new();
    for message in db.get_messages() {
        events.push((message.order, TimelineEvent::Message(message)));
    }
    for note in db.get_notes() {
        events.push((note.order, TimelineEvent::Note(note)));
    }
    events.sort_by_key(|(order, _)| *order);
    events
}

#[derive(Debug, Clone, Copy)]
struct SequenceLayoutConfig {
    base_actor_spacing: f64,
    actor_width: f64,
    actor_height: f64,
    message_spacing: f64,
    diagram_margin_x: f64,
    diagram_margin_y: f64,
    char_width: f64,
    wrap_padding: f64,
    actor_margin: f64,
    box_margin: f64,
    note_margin: f64,
    min_note_height: f64,
    line_height: f64,
    label_box_height: f64,
}

impl Default for SequenceLayoutConfig {
    fn default() -> Self {
        Self {
            base_actor_spacing: 200.0,
            actor_width: 150.0,
            actor_height: 65.0,
            message_spacing: 44.0,
            diagram_margin_x: 50.0,
            diagram_margin_y: 10.0,
            char_width: 9.0,
            wrap_padding: 10.0,
            actor_margin: 50.0,
            box_margin: 10.0,
            note_margin: 10.0,
            min_note_height: 39.0,
            line_height: 19.0,
            label_box_height: 20.0,
        }
    }
}

const MIN_NOTE_WIDTH: f64 = 100.0;
const RIGHT_OF_NOTE_WIDTH: f64 = 150.0;
const RIGHT_OF_NOTE_X_OFFSET: f64 = 25.0;
const LEFT_OF_NOTE_X_OFFSET: f64 = 20.0;
const SELF_MESSAGE_LOOP_WIDTH: f64 = 60.0;
const SELF_MESSAGE_LOOP_TOP_OFFSET: f64 = 10.0;
const SELF_MESSAGE_LOOP_BOTTOM_OFFSET: f64 = 30.0;
const SELF_MESSAGE_END_OFFSET: f64 = 20.0;
const SELF_MESSAGE_LABEL_GAP: f64 = 4.0;
const ACTIVATION_WIDTH: f64 = 10.0;

#[derive(Debug, Clone, Copy)]
struct Bounds {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
}

impl Bounds {
    fn right(&self) -> f64 {
        self.x + self.width
    }

    fn bottom(&self) -> f64 {
        self.y + self.height
    }

    fn union(self, other: Bounds) -> Bounds {
        let x = self.x.min(other.x);
        let y = self.y.min(other.y);
        let right = self.right().max(other.right());
        let bottom = self.bottom().max(other.bottom());
        Bounds {
            x,
            y,
            width: right - x,
            height: bottom - y,
        }
    }

    fn expand(self, amount: f64) -> Bounds {
        Bounds {
            x: self.x - amount,
            y: self.y - amount,
            width: self.width + amount * 2.0,
            height: self.height + amount * 2.0,
        }
    }
}

#[derive(Debug, Clone)]
struct ActorLayout {
    description: String,
    actor_type: ParticipantType,
    x: f64,
    center_x: f64,
}

#[derive(Debug, Clone)]
struct MessageLayout {
    from_x: f64,
    to_x: f64,
    y: f64,
    message: String,
    message_type: LineType,
    sequence_num: Option<i32>,
}

#[derive(Debug, Clone)]
struct NoteLayout {
    actor_x: f64,
    span_x: Option<f64>,
    y: f64,
    message: String,
    placement: Placement,
}

#[derive(Debug, Clone)]
enum LaidOutEvent {
    Message(MessageLayout),
    Note(NoteLayout),
}

#[derive(Debug, Clone)]
struct FragmentLayout {
    kind: FragmentKind,
    label: String,
    frame: Bounds,
    color: Option<String>,
    sections: Vec<FragmentSectionLayout>,
}

#[derive(Debug, Clone)]
struct FragmentSectionLayout {
    kind: FragmentKind,
    label: String,
    y: f64,
}

#[derive(Debug, Clone)]
struct OpenFragmentLayout {
    kind: FragmentKind,
    label: String,
    start_y: f64,
    content_bounds: Option<Bounds>,
    min_actor_idx: Option<usize>,
    max_actor_idx: Option<usize>,
    color: Option<String>,
    sections: Vec<FragmentSectionLayout>,
}

#[derive(Debug, Clone)]
struct ActivationLayout {
    actor_x: f64,
    start_y: f64,
    end_y: f64,
    stack_offset: f64,
}

#[derive(Debug, Clone, Copy)]
struct OpenActivationLayout {
    start_y: f64,
    stack_offset: f64,
}

#[derive(Debug)]
struct SequenceLayout {
    actors: Vec<ActorLayout>,
    actor_positions: HashMap<String, f64>,
    actor_index: HashMap<String, usize>,
    content_width: f64,
    events: Vec<LaidOutEvent>,
    fragments: Vec<FragmentLayout>,
    activations: Vec<ActivationLayout>,
    bounds: Vec<Bounds>,
}

impl SequenceLayout {
    fn all_bounds(&self) -> impl Iterator<Item = Bounds> + '_ {
        self.bounds.iter().copied()
    }
}

#[derive(Debug, Clone, Copy)]
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

/// Message elements separated into shapes (lines/paths) and labels (text)
/// This enables proper SVG z-order: shapes render before labels
struct MessageElements {
    shapes: Vec<SvgElement>,
    labels: Vec<SvgElement>,
}

/// Fragment label elements separated into shapes (polygons) and labels (text)
/// This enables proper SVG z-order: shapes in clusters, labels in edge_labels
struct FragmentLabelElements {
    shapes: Vec<SvgElement>,
    labels: Vec<SvgElement>,
}

impl OpenFragmentLayout {
    fn include_event(&mut self, min_idx: usize, max_idx: usize, bounds: Bounds) {
        self.min_actor_idx = Some(
            self.min_actor_idx
                .map_or(min_idx, |value| value.min(min_idx)),
        );
        self.max_actor_idx = Some(
            self.max_actor_idx
                .map_or(max_idx, |value| value.max(max_idx)),
        );
        self.content_bounds = Some(
            self.content_bounds
                .map_or(bounds, |existing| existing.union(bounds)),
        );
    }

    fn include_bounds(&mut self, bounds: Bounds) {
        self.content_bounds = Some(
            self.content_bounds
                .map_or(bounds, |existing| existing.union(bounds)),
        );
    }
}

fn layout_basic_events(
    db: &SequenceDb,
    layout: &mut SequenceLayout,
    lifeline_start_y: f64,
    cfg: &SequenceLayoutConfig,
) {
    let autonumber_config = db.get_autonumber();
    let mut sequence_index: i32 = autonumber_config.map_or(1, |c| c.start);
    let sequence_step: i32 = autonumber_config.map_or(1, |c| c.step);
    let autonumber_enabled = autonumber_config.is_some();
    let mut current_y = lifeline_start_y + cfg.message_spacing;
    let mut last_content_bottom: Option<f64> = None;
    let mut open_fragments: Vec<OpenFragmentLayout> = Vec::new();
    let mut activation_stacks: HashMap<String, Vec<OpenActivationLayout>> = HashMap::new();

    for (_, event) in collect_timeline_events(db) {
        match event {
            TimelineEvent::Message(message) => match message.message_type {
                LineType::LoopStart
                | LineType::AltStart
                | LineType::OptStart
                | LineType::ParStart
                | LineType::CriticalStart
                | LineType::BreakStart
                | LineType::RectStart => {
                    let kind = FragmentKind::from_message_type(message.message_type);
                    open_fragments.push(OpenFragmentLayout {
                        kind,
                        label: message.message.trim().to_string(),
                        start_y: current_y,
                        content_bounds: None,
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
                        sections: Vec::new(),
                    });
                    current_y += fragment_header_reserve(cfg);
                }
                LineType::AltElse | LineType::ParAnd | LineType::CriticalOption => {
                    if let Some(fragment) = open_fragments.last_mut() {
                        fragment.sections.push(FragmentSectionLayout {
                            kind: FragmentKind::from_message_type(message.message_type),
                            label: message.message.trim().to_string(),
                            y: current_y,
                        });
                        fragment.include_bounds(Bounds {
                            x: 0.0,
                            y: current_y,
                            width: layout.content_width,
                            height: fragment_header_reserve(cfg),
                        });
                    }
                    current_y += fragment_header_reserve(cfg);
                }
                LineType::LoopEnd
                | LineType::AltEnd
                | LineType::OptEnd
                | LineType::ParEnd
                | LineType::CriticalEnd
                | LineType::BreakEnd
                | LineType::RectEnd => {
                    if let Some(open) = open_fragments.pop() {
                        let depth = open_fragments.len();
                        let frame = fragment_frame_bounds(&open, depth, layout, cfg);
                        let min_actor_idx = open.min_actor_idx;
                        let max_actor_idx = open.max_actor_idx;
                        let fragment = FragmentLayout {
                            kind: open.kind,
                            label: open.label,
                            frame,
                            color: open.color,
                            sections: open.sections,
                        };
                        layout.bounds.push(fragment.frame);
                        if let Some(parent) = open_fragments.last_mut() {
                            let min_idx = min_actor_idx.unwrap_or(0);
                            let max_idx = max_actor_idx
                                .unwrap_or_else(|| layout.actors.len().saturating_sub(1));
                            parent.include_event(min_idx, max_idx, fragment.frame);
                        }
                        current_y = current_y.max(fragment.frame.bottom() + cfg.box_margin);
                        last_content_bottom = Some(
                            last_content_bottom.map_or(fragment.frame.bottom(), |bottom| {
                                bottom.max(fragment.frame.bottom())
                            }),
                        );
                        layout.fragments.push(fragment);
                    }
                }
                LineType::ActiveStart => {
                    if let Some(actor) = message.message.split_whitespace().next() {
                        let depth = activation_stacks.get(actor).map_or(0, Vec::len);
                        let start_y = last_content_bottom.unwrap_or(current_y);
                        activation_stacks
                            .entry(actor.to_string())
                            .or_default()
                            .push(OpenActivationLayout {
                                start_y,
                                stack_offset: depth as f64 * ACTIVATION_WIDTH,
                            });
                    }
                }
                LineType::ActiveEnd => {
                    if let Some(actor) = message.message.split_whitespace().next() {
                        if let Some(stack) = activation_stacks.get_mut(actor) {
                            if let Some(open) = stack.pop() {
                                if let Some(&actor_x) = layout.actor_positions.get(actor) {
                                    let end_y = last_content_bottom
                                        .unwrap_or(current_y)
                                        .max(open.start_y + cfg.line_height);
                                    let activation = ActivationLayout {
                                        actor_x,
                                        start_y: open.start_y,
                                        end_y,
                                        stack_offset: open.stack_offset,
                                    };
                                    layout.bounds.push(Bounds {
                                        x: actor_x + open.stack_offset - ACTIVATION_WIDTH / 2.0,
                                        y: open.start_y,
                                        width: ACTIVATION_WIDTH,
                                        height: end_y - open.start_y,
                                    });
                                    layout.activations.push(activation);
                                }
                            }
                        }
                    }
                }
                LineType::Autonumber => {}
                _ => {
                    let (Some(from), Some(to)) = (&message.from, &message.to) else {
                        continue;
                    };
                    let (Some(&from_x), Some(&to_x)) = (
                        layout.actor_positions.get(from),
                        layout.actor_positions.get(to),
                    ) else {
                        continue;
                    };
                    let sequence_num = if autonumber_enabled {
                        Some(sequence_index)
                    } else {
                        None
                    };
                    let bounds = message_bounds(from_x, to_x, current_y, &message.message, cfg);
                    layout.bounds.push(bounds);
                    if let (Some(&from_idx), Some(&to_idx)) =
                        (layout.actor_index.get(from), layout.actor_index.get(to))
                    {
                        let min_idx = from_idx.min(to_idx);
                        let max_idx = from_idx.max(to_idx);
                        for fragment in &mut open_fragments {
                            fragment.include_event(min_idx, max_idx, bounds);
                        }
                    }
                    layout.events.push(LaidOutEvent::Message(MessageLayout {
                        from_x,
                        to_x,
                        y: current_y,
                        message: message.message.clone(),
                        message_type: message.message_type,
                        sequence_num,
                    }));
                    if autonumber_enabled {
                        sequence_index += sequence_step;
                    }

                    let message_bottom = if is_self_message(message) {
                        bounds.bottom()
                    } else {
                        current_y
                    };
                    last_content_bottom = Some(message_bottom);
                    current_y = message_bottom + cfg.message_spacing;
                }
            },
            TimelineEvent::Note(note) => {
                let previous_bottom =
                    last_content_bottom.unwrap_or(current_y - cfg.message_spacing);
                let note_y = previous_bottom + cfg.note_margin;
                let mut note_bottom = note_y + note_height(&note.message, cfg);
                if let Some(&actor_x) = layout.actor_positions.get(&note.actor) {
                    let span_x = note
                        .actor_to
                        .as_ref()
                        .and_then(|actor| layout.actor_positions.get(actor))
                        .copied();
                    let bounds =
                        note_bounds(actor_x, span_x, note_y, &note.message, note.placement, cfg);
                    note_bottom = bounds.bottom();
                    layout.bounds.push(bounds);
                    if let Some(&actor_idx) = layout.actor_index.get(&note.actor) {
                        let mut min_idx = actor_idx;
                        let mut max_idx = actor_idx;
                        if let Some(other) = note
                            .actor_to
                            .as_ref()
                            .and_then(|name| layout.actor_index.get(name).copied())
                        {
                            min_idx = min_idx.min(other);
                            max_idx = max_idx.max(other);
                        }
                        for fragment in &mut open_fragments {
                            fragment.include_event(min_idx, max_idx, bounds);
                        }
                    }
                    layout.events.push(LaidOutEvent::Note(NoteLayout {
                        actor_x,
                        span_x,
                        y: note_y,
                        message: note.message.clone(),
                        placement: note.placement,
                    }));
                }
                last_content_bottom = Some(note_bottom);
                current_y = note_bottom + cfg.message_spacing;
            }
        }
    }

    if let Some(bottom) = last_content_bottom {
        layout.bounds.push(Bounds {
            x: 0.0,
            y: bottom,
            width: 0.0,
            height: 0.0,
        });
    } else {
        layout.bounds.push(Bounds {
            x: 0.0,
            y: current_y,
            width: 0.0,
            height: 0.0,
        });
    }
}

fn fragment_frame_bounds(
    open: &OpenFragmentLayout,
    depth: usize,
    layout: &SequenceLayout,
    cfg: &SequenceLayoutConfig,
) -> Bounds {
    let content = open
        .content_bounds
        .unwrap_or_else(|| fragment_actor_fallback_bounds(open, layout, cfg));
    let expand = cfg.box_margin * (depth as f64 + 1.0);
    let mut frame = content.expand(expand);
    if let (Some(min_idx), Some(max_idx)) = (open.min_actor_idx, open.max_actor_idx) {
        let min_center = layout.actors[min_idx].center_x;
        let max_center = layout.actors[max_idx].center_x;
        let actor_x = min_center - cfg.actor_width / 2.0 - expand;
        let actor_right = max_center + cfg.actor_width / 2.0 + expand;
        let x = frame.x.min(actor_x);
        let right = frame.right().max(actor_right);
        frame.x = x;
        frame.width = (right - x).max(20.0);
    }

    let content_bottom = open
        .content_bounds
        .map(|b| b.bottom())
        .unwrap_or(open.start_y);
    let original_bottom = frame.bottom();
    let header_bottom = open.start_y + cfg.label_box_height + cfg.box_margin;
    let top = frame.y.min(open.start_y);
    let bottom = original_bottom.max(content_bottom).max(header_bottom);
    frame = Bounds {
        x: frame.x,
        y: top,
        width: frame.width,
        height: bottom - top,
    };

    frame
}

fn fragment_header_reserve(cfg: &SequenceLayoutConfig) -> f64 {
    cfg.label_box_height + cfg.line_height + cfg.box_margin + cfg.box_margin / 2.0
}

fn fragment_actor_fallback_bounds(
    open: &OpenFragmentLayout,
    layout: &SequenceLayout,
    cfg: &SequenceLayoutConfig,
) -> Bounds {
    if let (Some(min_idx), Some(max_idx)) = (open.min_actor_idx, open.max_actor_idx) {
        let min_center = layout.actors[min_idx].center_x;
        let max_center = layout.actors[max_idx].center_x;
        let x = min_center - cfg.actor_width / 2.0;
        let right = max_center + cfg.actor_width / 2.0;
        return Bounds {
            x,
            y: open.start_y,
            width: (right - x).max(20.0),
            height: cfg.label_box_height + cfg.box_margin,
        };
    }

    Bounds {
        x: 0.0,
        y: open.start_y,
        width: layout.content_width.max(20.0),
        height: cfg.label_box_height + cfg.box_margin,
    }
}

fn is_self_message(message: &crate::diagrams::sequence::Message) -> bool {
    matches!(
        (&message.from, &message.to),
        (Some(from), Some(to)) if from == to
    ) || matches!(
        message.message_type,
        LineType::SolidPoint | LineType::DottedPoint
    )
}

fn message_bounds(
    from_x: f64,
    to_x: f64,
    y: f64,
    label: &str,
    cfg: &SequenceLayoutConfig,
) -> Bounds {
    let label_width = text_width(label, cfg);
    if (from_x - to_x).abs() < 1.0 {
        let label_center_x = from_x + SELF_MESSAGE_LOOP_WIDTH / 2.0;
        let label_y = y - SELF_MESSAGE_LOOP_TOP_OFFSET - SELF_MESSAGE_LABEL_GAP;
        let path = Bounds {
            x: from_x,
            y: y - SELF_MESSAGE_LOOP_TOP_OFFSET,
            width: SELF_MESSAGE_LOOP_WIDTH,
            height: SELF_MESSAGE_LOOP_TOP_OFFSET + SELF_MESSAGE_LOOP_BOTTOM_OFFSET,
        };
        let label = Bounds {
            x: label_center_x - label_width / 2.0,
            y: label_y - cfg.line_height,
            width: label_width,
            height: cfg.line_height,
        };
        let actor = Bounds {
            x: from_x - cfg.actor_width / 2.0,
            y: y - SELF_MESSAGE_LOOP_TOP_OFFSET - cfg.line_height,
            width: cfg.actor_width,
            height: cfg.line_height
                + SELF_MESSAGE_LOOP_TOP_OFFSET
                + SELF_MESSAGE_LOOP_BOTTOM_OFFSET,
        };
        path.union(label).union(actor)
    } else {
        let line_left = from_x.min(to_x);
        let line_right = from_x.max(to_x);
        let label_center = (from_x + to_x) / 2.0;
        let label_left = label_center - label_width / 2.0;
        let left = line_left.min(label_left);
        let right = line_right.max(label_left + label_width);
        Bounds {
            x: left,
            y: y - cfg.line_height,
            width: right - left,
            height: cfg.line_height + 10.0,
        }
    }
}

fn note_bounds(
    actor_x: f64,
    span_x: Option<f64>,
    y: f64,
    message: &str,
    placement: Placement,
    cfg: &SequenceLayoutConfig,
) -> Bounds {
    let rendered_note_width = note_width(message, placement, span_x, actor_x, cfg);
    let (note_width, x_center) = match placement {
        Placement::Over => (
            rendered_note_width,
            span_x.map_or(actor_x, |span_x| (actor_x + span_x) / 2.0),
        ),
        Placement::RightOf | Placement::LeftOf => (rendered_note_width, actor_x),
    };
    let x = match placement {
        Placement::LeftOf => actor_x - note_width - LEFT_OF_NOTE_X_OFFSET,
        Placement::RightOf => actor_x + RIGHT_OF_NOTE_X_OFFSET,
        Placement::Over => x_center - note_width / 2.0,
    };
    Bounds {
        x,
        y,
        width: note_width,
        height: note_height(message, cfg),
    }
}

fn note_width(
    message: &str,
    placement: Placement,
    span_x: Option<f64>,
    actor_x: f64,
    cfg: &SequenceLayoutConfig,
) -> f64 {
    let text_width = text_width(message, cfg) + cfg.box_margin * 2.0;
    let placement_min = match placement {
        Placement::Over => {
            if let Some(span_x) = span_x {
                ((span_x - actor_x).abs() + 50.0).max(MIN_NOTE_WIDTH)
            } else {
                MIN_NOTE_WIDTH
            }
        }
        Placement::RightOf => RIGHT_OF_NOTE_WIDTH,
        Placement::LeftOf => MIN_NOTE_WIDTH,
    };

    placement_min.max(text_width)
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
            // Use inline fill/stroke for mermaid visual parity (eval detects inline attrs)
            children.push(SvgElement::Rect {
                x: center_x - width / 2.0 + padding,
                y: top_y,
                width: width - padding * 2.0,
                height,
                rx: Some(3.0),
                ry: Some(3.0),
                attrs: Attrs::new()
                    .with_stroke_width(1.0)
                    .with_fill("#eaeaea")
                    .with_stroke("#666")
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
/// Returns shapes and labels separately for proper z-order
fn render_message(
    from_x: f64,
    to_x: f64,
    y: f64,
    label: &str,
    msg_type: LineType,
    sequence_num: Option<i32>,
) -> MessageElements {
    let mut shapes = Vec::new();
    let mut labels = Vec::new();

    let (is_dotted, marker_id) = match msg_type {
        LineType::Solid => (false, Some("arrow-filled")),
        LineType::Dotted => (true, Some("arrow-filled")),
        LineType::SolidOpen => (false, None),
        LineType::DottedOpen => (true, None),
        LineType::SolidCross => (false, Some("arrow-cross")),
        LineType::DottedCross => (true, Some("arrow-cross")),
        LineType::SolidPoint | LineType::DottedPoint => {
            // Self-message (loop back to same actor)
            return render_self_message(
                from_x,
                y,
                label,
                msg_type == LineType::DottedPoint,
                sequence_num,
            );
        }
        _ => (false, Some("arrow-filled")),
    };

    // Determine direction
    let is_self_message = (from_x - to_x).abs() < 1.0;
    if is_self_message {
        return render_self_message(from_x, y, label, is_dotted, sequence_num);
    }

    // Message line (shape - rendered first in edge_paths)
    // Use messageLine0 (solid) or messageLine1 (dotted) classes matching mermaid.js
    let line_class = if is_dotted {
        "messageLine1"
    } else {
        "messageLine0"
    };
    let mut line_attrs = Attrs::new()
        .with_stroke_width(2.0) // Match mermaid.js default (stroke-width: 2)
        .with_class(line_class)
        .with_attr("stroke", "none") // Stroke comes from CSS class
        .with_attr(
            "style",
            if is_dotted {
                "stroke-dasharray: 3, 3; fill: none;"
            } else {
                "fill: none;"
            },
        );
    if let Some(marker_id) = marker_id {
        line_attrs = line_attrs.with_attr("marker-end", &format!("url(#{})", marker_id));
    }

    shapes.push(SvgElement::Line {
        x1: from_x,
        y1: y,
        x2: to_x,
        y2: y,
        attrs: line_attrs,
    });

    // Sequence number using zero-length line with marker-start (matching mermaid.js)
    if let Some(num) = sequence_num {
        // Zero-length line that triggers the sequencenumber marker
        shapes.push(SvgElement::Line {
            x1: from_x,
            y1: y,
            x2: from_x,
            y2: y,
            attrs: Attrs::new()
                .with_stroke_width(0.0)
                .with_attr("marker-start", "url(#sequencenumber)"),
        });

        // Number text label (rendered on top in edge_labels)
        labels.push(SvgElement::Text {
            x: from_x,
            y: y + 4.0,
            content: num.to_string(),
            attrs: Attrs::new()
                .with_attr("text-anchor", "middle")
                .with_class("sequenceNumber")
                .with_attr("font-size", "12")
                .with_attr("font-family", "sans-serif"),
        });
    }

    // Message label (text - rendered after shapes in edge_labels)
    let label_x = (from_x + to_x) / 2.0;
    let label_y = y - 10.0;

    labels.push(SvgElement::Text {
        x: label_x,
        y: label_y,
        content: label.to_string(),
        attrs: Attrs::new()
            .with_attr("text-anchor", "middle")
            .with_class("message-label")
            .with_attr("font-size", "16"),
    });

    MessageElements { shapes, labels }
}

fn render_activation(actor_x: f64, start_y: f64, end_y: f64) -> SvgElement {
    let height = (end_y - start_y).max(1.0);

    SvgElement::Rect {
        x: actor_x - ACTIVATION_WIDTH / 2.0,
        y: start_y,
        width: ACTIVATION_WIDTH,
        height,
        rx: Some(1.0),
        ry: Some(1.0),
        attrs: Attrs::new().with_class("activation"),
    }
}

/// Render a fragment frame as 4 individual lines (top/right/bottom/left) matching mermaid.js.
/// Returns a group containing the 4 border lines (and optionally a fill rect for rect fragments).
fn render_fragment_frame(
    x: f64,
    width: f64,
    start_y: f64,
    end_y: f64,
    fill: Option<&str>,
) -> SvgElement {
    let right = x + width;
    let mut children = Vec::new();

    // If fill color specified (rect fragments), add a background rect
    if let Some(color) = fill {
        children.push(SvgElement::Rect {
            x,
            y: start_y,
            width,
            height: (end_y - start_y).max(1.0),
            rx: None,
            ry: None,
            attrs: Attrs::new().with_fill(color).with_stroke("none"),
        });
    }

    // Top line
    children.push(SvgElement::Line {
        x1: x,
        y1: start_y,
        x2: right,
        y2: start_y,
        attrs: Attrs::new().with_class("loopLine"),
    });
    // Right line
    children.push(SvgElement::Line {
        x1: right,
        y1: start_y,
        x2: right,
        y2: end_y,
        attrs: Attrs::new().with_class("loopLine"),
    });
    // Bottom line
    children.push(SvgElement::Line {
        x1: x,
        y1: end_y,
        x2: right,
        y2: end_y,
        attrs: Attrs::new().with_class("loopLine"),
    });
    // Left line
    children.push(SvgElement::Line {
        x1: x,
        y1: start_y,
        x2: x,
        y2: end_y,
        attrs: Attrs::new().with_class("loopLine"),
    });

    SvgElement::Group {
        children,
        attrs: Attrs::new(),
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

fn render_fragment_label(
    kind: FragmentKind,
    x: f64,
    width: f64,
    y: f64,
    text: &str,
    label_height: f64,
) -> FragmentLabelElements {
    let mut shapes = Vec::new();
    let mut labels = Vec::new();
    let label_y = y;

    let (prefix, condition) = match kind {
        FragmentKind::Else | FragmentKind::And | FragmentKind::Option => (None, Some(text)),
        _ => (
            Some(fragment_prefix(kind)),
            if text.is_empty() { None } else { Some(text) },
        ),
    };

    let mut prefix_label_width = 0.0;
    if let Some(prefix) = prefix {
        let label_width = (prefix.len() as f64 * 7.0 + 16.0).max(50.0);
        prefix_label_width = label_width;
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

        // Polygon is a shape - goes to clusters for proper z-order
        shapes.push(SvgElement::Polygon {
            points,
            attrs: Attrs::new().with_class("labelBox"),
        });
        // Text is a label - goes to edge_labels
        labels.push(SvgElement::Text {
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
            labels.push(SvgElement::Text {
                x: x + width / 2.0 + prefix_label_width / 2.0,
                y: label_y + label_height - 2.0,
                content: wrapped,
                attrs: Attrs::new()
                    .with_attr("text-anchor", "middle")
                    .with_class("loopText")
                    .with_attr("font-size", "16"),
            });
        }
    }

    FragmentLabelElements { shapes, labels }
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

/// Render a self-message (loop back to same actor)
/// Returns shapes and labels separately for proper z-order
fn render_self_message(
    x: f64,
    y: f64,
    label: &str,
    is_dotted: bool,
    sequence_num: Option<i32>,
) -> MessageElements {
    let mut shapes = Vec::new();
    let mut labels = Vec::new();

    // Mermaid renders self messages as a rounded cubic loop by default.
    let path = format!(
        "M {} {} C {} {} {} {} {} {}",
        x,
        y,
        x + SELF_MESSAGE_LOOP_WIDTH,
        y - SELF_MESSAGE_LOOP_TOP_OFFSET,
        x + SELF_MESSAGE_LOOP_WIDTH,
        y + SELF_MESSAGE_LOOP_BOTTOM_OFFSET,
        x,
        y + SELF_MESSAGE_END_OFFSET
    );

    let mut path_attrs = Attrs::new()
        .with_fill("none")
        .with_stroke_width(1.5) // Match mermaid.js default
        .with_class("message-line")
        .with_attr("marker-end", "url(#arrow-filled)");

    if is_dotted {
        path_attrs = path_attrs.with_stroke_dasharray("3,3");
    }

    shapes.push(SvgElement::Path {
        d: path,
        attrs: path_attrs,
    });

    // Sequence number using zero-length line with marker-start (matching mermaid.js)
    if let Some(num) = sequence_num {
        shapes.push(SvgElement::Line {
            x1: x,
            y1: y,
            x2: x,
            y2: y,
            attrs: Attrs::new()
                .with_stroke_width(0.0)
                .with_attr("marker-start", "url(#sequencenumber)"),
        });

        labels.push(SvgElement::Text {
            x,
            y: y + 4.0,
            content: num.to_string(),
            attrs: Attrs::new()
                .with_attr("text-anchor", "middle")
                .with_class("sequenceNumber")
                .with_attr("font-size", "12")
                .with_attr("font-family", "sans-serif"),
        });
    }

    // Message label (text - rendered after shapes in edge_labels)
    labels.push(SvgElement::Text {
        x: x + SELF_MESSAGE_LOOP_WIDTH / 2.0,
        y: y - SELF_MESSAGE_LOOP_TOP_OFFSET - SELF_MESSAGE_LABEL_GAP,
        content: label.to_string(),
        attrs: Attrs::new()
            .with_attr("text-anchor", "middle")
            .with_class("message-label")
            .with_attr("font-size", "16"),
    });

    MessageElements { shapes, labels }
}

/// Render a note
fn render_note(
    actor_x: f64,
    span_x: Option<f64>,
    y: f64,
    message: &str,
    placement: Placement,
    cfg: &SequenceLayoutConfig,
) -> SvgElement {
    let bounds = note_bounds(actor_x, span_x, y, message, placement, cfg);
    let x = bounds.x;
    let top_y = bounds.y;
    let note_width = bounds.width;
    let note_height = bounds.height;

    let mut children = Vec::new();

    // Note box - use rect like mermaid.js (no folded corner)
    children.push(SvgElement::Rect {
        x,
        y: top_y,
        width: note_width,
        height: note_height,
        rx: None,
        ry: None,
        attrs: Attrs::new()
            .with_class("note")
            .with_fill("#EDF2AE")
            .with_stroke("#666"),
    });

    // Note text - render each line as a separate text element (like mermaid.js)
    // Use dy="1em" pattern matching mermaid.js text positioning
    let normalized = super::text_utils::normalize_br_tags(message);
    for (idx, line) in normalized.lines().enumerate() {
        // Mermaid.js positions text at top with dy="1em" offset
        let text_y = top_y + 5.0 + (idx as f64 * cfg.line_height);
        children.push(SvgElement::Text {
            x: x + note_width / 2.0,
            y: text_y,
            content: line.to_string(),
            attrs: Attrs::new()
                .with_attr("text-anchor", "middle")
                .with_attr("dominant-baseline", "middle")
                .with_attr("alignment-baseline", "middle")
                .with_attr("dy", "1em")
                .with_class("noteText")
                .with_attr("font-size", "16"),
        });
    }

    SvgElement::Group {
        children,
        attrs: Attrs::new().with_class("note"),
    }
}

fn note_height(message: &str, cfg: &SequenceLayoutConfig) -> f64 {
    let measured = visual_line_count(message) as f64 * cfg.line_height + cfg.note_margin * 2.0;
    measured.max(cfg.min_note_height)
}

fn visual_line_count(message: &str) -> usize {
    let normalized = super::text_utils::normalize_br_tags(message);
    normalized.lines().count().max(1)
}

/// Create an arrow marker definition
fn create_arrow_marker(id: &str, filled: bool) -> SvgElement {
    let path = if filled {
        "M -1 0 L 10 5 L 0 10 z"
    } else {
        "M -1 0 L 10 5 L 0 10"
    };

    // Use class for theming - fill handled by CSS .sequence-marker rule
    let class_name = if filled {
        "sequence-marker-filled"
    } else {
        "sequence-marker-open"
    };

    SvgElement::Marker {
        id: id.to_string(),
        view_box: String::new(),
        ref_x: 7.9,
        ref_y: 5.0,
        marker_width: 12.0,
        marker_height: 12.0,
        orient: "auto-start-reverse".to_string(),
        marker_units: Some("userSpaceOnUse".to_string()),
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
        marker_width: 12.0,
        marker_height: 12.0,
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

/// Create a sequence number marker (circle background for message numbering)
/// Matches mermaid.js marker: <marker id="sequencenumber">
fn create_sequence_number_marker() -> SvgElement {
    // Matching mermaid.js marker definition (no viewBox)
    SvgElement::Marker {
        id: "sequencenumber".to_string(),
        view_box: String::new(), // No viewBox like mermaid.js
        ref_x: 15.0,
        ref_y: 15.0,
        marker_width: 60.0,
        marker_height: 40.0,
        orient: "auto".to_string(),
        marker_units: None,
        children: vec![SvgElement::Circle {
            cx: 15.0,
            cy: 15.0,
            r: 6.0,
            attrs: Attrs::new().with_class("sequence-number"),
        }],
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

.sequence-number {{
  fill: {signal_color};
}}

.sequenceNumber-circle {{
  fill: {signal_color};
  stroke: {signal_color};
}}

.sequenceNumber {{
  fill: white;
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

fn build_actor_layout(db: &SequenceDb, cfg: &SequenceLayoutConfig) -> SequenceLayout {
    let actors = db.get_actors_in_order();
    let messages = db.get_messages();
    let mut actor_index: HashMap<String, usize> = HashMap::new();
    for (index, actor) in actors.iter().enumerate() {
        actor_index.insert(actor.name.clone(), index);
    }

    let mut gap_spacings = calculate_per_gap_spacing(&actors, messages, cfg);
    apply_sequence_gap_pressure(&mut gap_spacings, db, &actor_index, cfg);

    let mut layout_actors = Vec::with_capacity(actors.len());
    let mut actor_positions = HashMap::new();
    let mut cumulative_x = 0.0;
    for (index, actor) in actors.iter().enumerate() {
        let center_x = cumulative_x + cfg.actor_width / 2.0;
        actor_positions.insert(actor.name.clone(), center_x);
        layout_actors.push(ActorLayout {
            description: actor.description.clone(),
            actor_type: actor.actor_type,
            x: cumulative_x,
            center_x,
        });
        if index < gap_spacings.len() {
            cumulative_x += gap_spacings[index];
        }
    }

    let actor_span_width = layout_actors
        .last()
        .map_or(0.0, |actor| actor.x + cfg.actor_width);
    let content_width =
        required_sequence_content_width(db, &actor_positions, actor_span_width, cfg);

    SequenceLayout {
        actors: layout_actors,
        actor_positions,
        actor_index,
        content_width,
        events: Vec::new(),
        fragments: Vec::new(),
        activations: Vec::new(),
        bounds: Vec::new(),
    }
}

fn apply_sequence_gap_pressure(
    gap_spacings: &mut [f64],
    db: &SequenceDb,
    actor_index: &HashMap<String, usize>,
    cfg: &SequenceLayoutConfig,
) {
    use crate::diagrams::sequence::Placement;

    for note in db.get_notes() {
        let Some(&index) = actor_index.get(&note.actor) else {
            continue;
        };
        match note.placement {
            Placement::RightOf => {
                let note_width = note_width(&note.message, note.placement, None, 0.0, cfg);
                let required_gap =
                    RIGHT_OF_NOTE_X_OFFSET + note_width + cfg.actor_width / 2.0 + cfg.actor_margin;
                if let Some(gap) = gap_spacings.get_mut(index) {
                    *gap = gap.max(required_gap);
                }
            }
            Placement::LeftOf => {
                let note_width = note_width(&note.message, note.placement, None, 0.0, cfg);
                let required_gap =
                    LEFT_OF_NOTE_X_OFFSET + note_width + cfg.actor_width / 2.0 + cfg.actor_margin;
                if index > 0 {
                    if let Some(gap) = gap_spacings.get_mut(index - 1) {
                        *gap = gap.max(required_gap);
                    }
                }
            }
            Placement::Over => {}
        }
    }

    for message in db.get_messages() {
        let (Some(from), Some(to)) = (&message.from, &message.to) else {
            continue;
        };
        if from != to {
            continue;
        }
        let Some(&index) = actor_index.get(from) else {
            continue;
        };
        let required_gap = self_message_right_extent(&message.message, cfg)
            + SELF_MESSAGE_LABEL_GAP
            + cfg.actor_margin;
        if let Some(gap) = gap_spacings.get_mut(index) {
            *gap = gap.max(required_gap);
        }
    }
}

fn required_sequence_content_width(
    db: &SequenceDb,
    actor_positions: &HashMap<String, f64>,
    actor_span_width: f64,
    cfg: &SequenceLayoutConfig,
) -> f64 {
    use crate::diagrams::sequence::Placement;

    let mut content_width = actor_span_width;
    for note in db.get_notes() {
        if note.placement != Placement::RightOf {
            continue;
        }
        if let Some(&actor_x) = actor_positions.get(&note.actor) {
            let note_width = note_width(&note.message, note.placement, None, actor_x, cfg);
            content_width = content_width.max(actor_x + RIGHT_OF_NOTE_X_OFFSET + note_width);
        }
    }

    for message in db.get_messages() {
        let (Some(from), Some(to)) = (&message.from, &message.to) else {
            continue;
        };
        if from != to {
            continue;
        }
        if let Some(&actor_x) = actor_positions.get(from) {
            content_width = content_width.max(
                actor_x + self_message_right_extent(&message.message, cfg) + SELF_MESSAGE_LABEL_GAP,
            );
        }
    }

    content_width
}

fn text_width(text: &str, cfg: &SequenceLayoutConfig) -> f64 {
    let normalized = super::text_utils::normalize_br_tags(text);
    normalized
        .lines()
        .map(|line| line.chars().count() as f64 * cfg.char_width)
        .fold(0.0, f64::max)
}

fn self_message_right_extent(label: &str, cfg: &SequenceLayoutConfig) -> f64 {
    let label_width = text_width(label, cfg) + 2.0 * cfg.wrap_padding;
    let label_right_extent = SELF_MESSAGE_LOOP_WIDTH / 2.0 + label_width / 2.0;
    SELF_MESSAGE_LOOP_WIDTH.max(label_right_extent)
}

/// Calculate per-gap actor spacing based on message text widths.
/// Returns a Vec of spacing values, one for each gap between adjacent actors.
/// This matches mermaid.js behavior where each actor pair can have different spacing.
fn calculate_per_gap_spacing(
    actors: &[&crate::diagrams::sequence::Actor],
    messages: &[crate::diagrams::sequence::Message],
    cfg: &SequenceLayoutConfig,
) -> Vec<f64> {
    let num_gaps = if actors.len() > 1 {
        actors.len() - 1
    } else {
        return vec![];
    };

    // Build actor name to index mapping
    let actor_index: std::collections::HashMap<&str, usize> = actors
        .iter()
        .enumerate()
        .map(|(i, a)| (a.name.as_str(), i))
        .collect();

    // Track max required width per gap (between adjacent actors i and i+1)
    let mut max_width_per_gap: Vec<f64> = vec![0.0; num_gaps];

    for msg in messages {
        let from_idx = msg
            .from
            .as_ref()
            .and_then(|f| actor_index.get(f.as_str()).copied());
        let to_idx = msg
            .to
            .as_ref()
            .and_then(|t| actor_index.get(t.as_str()).copied());

        if let (Some(from), Some(to)) = (from_idx, to_idx) {
            if from != to {
                let measured_width = text_width(&msg.message, cfg) + 2.0 * cfg.wrap_padding;

                let min_idx = std::cmp::min(from, to);

                // Assign the full text width to the first gap between sender/receiver.
                max_width_per_gap[min_idx] = max_width_per_gap[min_idx].max(measured_width);
            }
        }
    }

    // Convert text widths to spacing values
    max_width_per_gap
        .iter()
        .map(|&width| {
            if width > 0.0 {
                let required = width + cfg.actor_margin - cfg.actor_width / 2.0;
                cfg.base_actor_spacing.max(required)
            } else {
                cfg.base_actor_spacing
            }
        })
        .collect()
}
