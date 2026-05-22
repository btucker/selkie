//! Sequence-diagram SVG geometry helpers.
//!
//! This module is unstable support for eval and integration tests. It inspects
//! rendered SVG output; it is not part of the sequence renderer layout model.

pub const SEQUENCE_OVERLAP_TOLERANCE: f64 = 4.0;
const SEQUENCE_CHAR_WIDTH: f64 = 8.0;
const SEQUENCE_LINE_HEIGHT: f64 = 18.0;

#[derive(Debug, Clone)]
pub struct SequenceBox {
    pub kind: &'static str,
    pub label: String,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

#[derive(Debug, Clone)]
pub struct SequenceMarker {
    pub id: String,
    pub ref_x: f64,
    pub ref_y: f64,
    pub marker_width: f64,
    pub marker_height: f64,
    pub marker_units: Option<String>,
}

#[derive(Debug)]
pub struct SequenceGeometry {
    svg_width: Option<f64>,
    view_box_min_x: f64,
    notes: Vec<SequenceBox>,
    message_texts: Vec<SequenceBox>,
    note_texts: Vec<SequenceBox>,
    loop_texts: Vec<SequenceBox>,
    label_texts: Vec<SequenceBox>,
    actor_boxes: Vec<SequenceBox>,
    actor_lifelines: Vec<(f64, f64)>,
    markers: Vec<SequenceMarker>,
    self_message_path_boxes: Vec<SequenceBox>,
    self_message_paths: Vec<String>,
    aggregate_fragment_frame: Option<SequenceBox>,
    fragments: Vec<SequenceBox>,
    fragment_frame_groups: Vec<SequenceBox>,
    fragment_headers: Vec<SequenceBox>,
    fragment_borders: Vec<SequenceBox>,
}

#[derive(Debug, Clone, Copy)]
struct SequenceLine {
    x1: f64,
    y1: f64,
    x2: f64,
    y2: f64,
}

impl SequenceLine {
    fn new(x1: f64, y1: f64, x2: f64, y2: f64) -> Self {
        Self { x1, y1, x2, y2 }
    }

    fn is_horizontal(&self) -> bool {
        (self.y1 - self.y2).abs() < 0.5
    }

    fn is_vertical(&self) -> bool {
        (self.x1 - self.x2).abs() < 0.5
    }

    fn min_x(&self) -> f64 {
        self.x1.min(self.x2)
    }

    fn max_x(&self) -> f64 {
        self.x1.max(self.x2)
    }

    fn min_y(&self) -> f64 {
        self.y1.min(self.y2)
    }

    fn max_y(&self) -> f64 {
        self.y1.max(self.y2)
    }
}

impl SequenceBox {
    fn new(
        kind: &'static str,
        label: impl Into<String>,
        x: f64,
        y: f64,
        width: f64,
        height: f64,
    ) -> Self {
        Self {
            kind,
            label: label.into(),
            x,
            y,
            width,
            height,
        }
    }

    pub fn right(&self) -> f64 {
        self.x + self.width
    }

    pub fn bottom(&self) -> f64 {
        self.y + self.height
    }

    pub fn intersects_with_tolerance(&self, other: &Self, tolerance: f64) -> bool {
        let left = self.x + tolerance;
        let right = self.x + self.width - tolerance;
        let top = self.y + tolerance;
        let bottom = self.y + self.height - tolerance;
        let other_left = other.x + tolerance;
        let other_right = other.x + other.width - tolerance;
        let other_top = other.y + tolerance;
        let other_bottom = other.y + other.height - tolerance;

        left < other_right && right > other_left && top < other_bottom && bottom > other_top
    }

    pub fn contains_with_tolerance(&self, inner: &Self, tolerance: f64) -> bool {
        inner.x >= self.x - tolerance
            && inner.y >= self.y - tolerance
            && inner.x + inner.width <= self.x + self.width + tolerance
            && inner.y + inner.height <= self.y + self.height + tolerance
    }
}

impl SequenceGeometry {
    pub fn parse(svg: &str) -> Option<Self> {
        let doc = roxmltree::Document::parse(svg).ok()?;
        let root = doc.root_element();
        let svg_width = root.attribute("width").and_then(parse_f64);
        let view_box_min_x = root
            .attribute("viewBox")
            .and_then(|view_box| view_box.split_whitespace().next())
            .and_then(parse_f64)
            .unwrap_or(0.0);

        let mut notes = Vec::new();
        let mut message_texts = Vec::new();
        let mut note_texts = Vec::new();
        let mut loop_texts = Vec::new();
        let mut label_texts = Vec::new();
        let mut actor_boxes = Vec::new();
        let mut actor_lifelines = Vec::new();
        let mut markers = Vec::new();
        let mut self_message_path_boxes = Vec::new();
        let mut self_message_paths = Vec::new();
        let mut loop_lines = Vec::new();

        for node in doc.descendants().filter(|node| node.is_element()) {
            match node.tag_name().name() {
                "rect" if has_class(&node, "note") => {
                    if let Some(note) = rect_sequence_box(&node, "note", note_label(&node)) {
                        notes.push(note);
                    }
                }
                "text" if has_class(&node, "messageText") || has_class(&node, "message-label") => {
                    let kind = if has_class(&node, "message-label") {
                        "message-label"
                    } else {
                        "messageText"
                    };
                    if let Some(text) = text_sequence_box(&node, kind) {
                        message_texts.push(text);
                    }
                }
                "text" if has_class(&node, "noteText") => {
                    if let Some(text) = text_sequence_box(&node, "noteText") {
                        note_texts.push(text);
                    }
                }
                "text" if has_class(&node, "loopText") => {
                    if let Some(text) = text_sequence_box(&node, "loopText") {
                        loop_texts.push(text);
                    }
                }
                "text" if has_class(&node, "labelText") => {
                    if let Some(text) = text_sequence_box(&node, "labelText") {
                        label_texts.push(text);
                    }
                }
                "rect" if has_class(&node, "actor-box") => {
                    if let Some(actor) = rect_sequence_box(&node, "actor", actor_label(&node)) {
                        actor_boxes.push(actor);
                    }
                }
                "line" if has_class(&node, "actor-line") => {
                    if let Some((x1, _, x2, _)) = line_coords(&node) {
                        actor_lifelines.push((x1, x2));
                    }
                }
                "line" if has_class(&node, "loopLine") => {
                    if let Some((x1, y1, x2, y2)) = line_coords(&node) {
                        loop_lines.push(SequenceLine::new(x1, y1, x2, y2));
                    }
                }
                "path" if node.attribute("marker-end").is_some() && is_message_path(&node) => {
                    let path = node.attribute("d").unwrap_or("");
                    let points = path_points(path);
                    if points.len() >= 3 {
                        self_message_paths.push(path.to_string());
                        self_message_path_boxes.push(box_from_points(
                            "self_message_path",
                            "self-message path",
                            &points,
                        ));
                    }
                }
                "marker" => {
                    if let Some(marker) = marker_geometry(&node) {
                        markers.push(marker);
                    }
                }
                _ => {}
            }
        }

        let aggregate_fragment_frame = aggregate_fragment_box(&loop_lines);
        let fragments = fragment_boxes_from_lines(&loop_lines);
        let fragment_frame_groups = fragment_frame_groups(&doc);
        let fragment_headers: Vec<_> = fragments
            .iter()
            .map(|frame| {
                SequenceBox::new(
                    "fragment_header",
                    "loop fragment header",
                    frame.x,
                    frame.y,
                    frame.width,
                    20.0 + SEQUENCE_OVERLAP_TOLERANCE,
                )
            })
            .collect();
        let fragment_borders: Vec<_> = fragments.iter().flat_map(fragment_border_boxes).collect();

        Some(Self {
            svg_width,
            view_box_min_x,
            notes,
            message_texts,
            note_texts,
            loop_texts,
            label_texts,
            actor_boxes,
            actor_lifelines,
            markers,
            self_message_path_boxes,
            self_message_paths,
            aggregate_fragment_frame,
            fragments,
            fragment_frame_groups,
            fragment_headers,
            fragment_borders,
        })
    }

    pub fn notes(&self) -> &[SequenceBox] {
        &self.notes
    }

    pub fn message_texts(&self) -> &[SequenceBox] {
        &self.message_texts
    }

    pub fn note_texts(&self) -> &[SequenceBox] {
        &self.note_texts
    }

    pub fn loop_texts(&self) -> &[SequenceBox] {
        &self.loop_texts
    }

    pub fn label_texts(&self) -> &[SequenceBox] {
        &self.label_texts
    }

    pub fn actor_boxes(&self) -> &[SequenceBox] {
        &self.actor_boxes
    }

    pub fn marker(&self, id: &str) -> Option<&SequenceMarker> {
        self.markers.iter().find(|marker| marker.id == id)
    }

    pub fn self_message_path_boxes(&self) -> &[SequenceBox] {
        &self.self_message_path_boxes
    }

    pub fn self_message_paths(&self) -> &[String] {
        &self.self_message_paths
    }

    pub fn fragments(&self) -> &[SequenceBox] {
        &self.fragments
    }

    pub fn fragment_frame_groups(&self) -> &[SequenceBox] {
        &self.fragment_frame_groups
    }

    pub fn fragment_headers(&self) -> &[SequenceBox] {
        &self.fragment_headers
    }

    pub fn fragment_borders(&self) -> &[SequenceBox] {
        &self.fragment_borders
    }

    pub fn text_box_containing(&self, label: &str) -> Option<SequenceBox> {
        self.message_texts
            .iter()
            .chain(&self.note_texts)
            .chain(&self.loop_texts)
            .chain(&self.label_texts)
            .find(|text| text.label.contains(label))
            .cloned()
    }

    pub fn note_box_containing(&self, label: &str) -> Option<SequenceBox> {
        self.notes
            .iter()
            .find(|note| note.label.contains(label))
            .cloned()
            .or_else(|| {
                let note_text = self
                    .note_texts
                    .iter()
                    .find(|text| text.label.contains(label))?;
                self.notes
                    .iter()
                    .find(|note| {
                        note.contains_with_tolerance(note_text, SEQUENCE_OVERLAP_TOLERANCE)
                    })
                    .cloned()
            })
    }

    pub fn actor_box_containing(&self, label: &str) -> Option<SequenceBox> {
        self.actor_boxes
            .iter()
            .find(|actor| actor.label.contains(label))
            .cloned()
    }

    pub fn lifeline_x_for_actor(&self, label: &str) -> Option<f64> {
        let actor = self.actor_box_containing(label)?;
        let center_x = actor.x + actor.width / 2.0;
        self.actor_lifelines
            .iter()
            .find(|(x1, x2)| (x1 - center_x).abs() < 0.1 && (x2 - center_x).abs() < 0.1)
            .map(|(x1, _)| *x1)
    }

    pub fn self_message_path_box(&self) -> Option<SequenceBox> {
        self.self_message_path_boxes.first().cloned()
    }

    pub fn first_fragment_frame(&self) -> Option<SequenceBox> {
        self.aggregate_fragment_frame.clone()
    }

    pub fn first_fragment_frame_group(&self) -> Option<SequenceBox> {
        self.fragment_frame_groups.first().cloned()
    }

    pub fn svg_width(&self) -> Option<f64> {
        self.svg_width
    }

    pub fn svg_visible_right(&self) -> Option<f64> {
        Some(self.view_box_min_x + self.svg_width?)
    }
}

fn rect_sequence_box(
    node: &roxmltree::Node<'_, '_>,
    kind: &'static str,
    fallback_label: String,
) -> Option<SequenceBox> {
    Some(SequenceBox::new(
        kind,
        node.attribute("id").unwrap_or(&fallback_label),
        parse_attr(node, "x")?,
        parse_attr(node, "y")?,
        parse_attr(node, "width")?,
        parse_attr(node, "height")?,
    ))
}

fn text_sequence_box(node: &roxmltree::Node<'_, '_>, kind: &'static str) -> Option<SequenceBox> {
    let text = node_text(node);
    let mut x = parse_attr(node, "x")?;
    let y = parse_attr(node, "y")?;
    let width = text.chars().count() as f64 * SEQUENCE_CHAR_WIDTH;
    match node.attribute("text-anchor").unwrap_or("start") {
        "middle" => x -= width / 2.0,
        "end" => x -= width,
        _ => {}
    }

    Some(SequenceBox::new(
        kind,
        text.clone(),
        x,
        text_box_y(node, kind, y),
        width,
        SEQUENCE_LINE_HEIGHT,
    ))
}

fn marker_geometry(node: &roxmltree::Node<'_, '_>) -> Option<SequenceMarker> {
    Some(SequenceMarker {
        id: node.attribute("id")?.to_string(),
        ref_x: parse_attr(node, "refX")?,
        ref_y: parse_attr(node, "refY")?,
        marker_width: parse_attr(node, "markerWidth")?,
        marker_height: parse_attr(node, "markerHeight")?,
        marker_units: node.attribute("markerUnits").map(str::to_string),
    })
}

fn text_box_y(node: &roxmltree::Node<'_, '_>, kind: &'static str, y: f64) -> f64 {
    if kind == "noteText" && has_middle_baseline(node) {
        let dy = parse_text_dy(node).unwrap_or(0.0);
        return y + dy - (SEQUENCE_LINE_HEIGHT / 2.0);
    }

    y - SEQUENCE_LINE_HEIGHT
}

fn has_middle_baseline(node: &roxmltree::Node<'_, '_>) -> bool {
    matches!(
        node.attribute("dominant-baseline"),
        Some("middle" | "central")
    ) || matches!(
        node.attribute("alignment-baseline"),
        Some("middle" | "central")
    )
}

fn parse_text_dy(node: &roxmltree::Node<'_, '_>) -> Option<f64> {
    let dy = node.attribute("dy")?;
    if let Some(em) = dy.strip_suffix("em") {
        let font_size = parse_attr(node, "font-size").unwrap_or(16.0);
        return Some(em.parse::<f64>().ok()? * font_size);
    }

    dy.parse::<f64>().ok()
}

fn line_coords(node: &roxmltree::Node<'_, '_>) -> Option<(f64, f64, f64, f64)> {
    Some((
        parse_attr(node, "x1")?,
        parse_attr(node, "y1")?,
        parse_attr(node, "x2")?,
        parse_attr(node, "y2")?,
    ))
}

fn fragment_boxes_from_lines(lines: &[SequenceLine]) -> Vec<SequenceBox> {
    let horizontal: Vec<_> = lines.iter().filter(|line| line.is_horizontal()).collect();
    let vertical: Vec<_> = lines.iter().filter(|line| line.is_vertical()).collect();
    let mut fragments = Vec::new();

    for top in &horizontal {
        for bottom in &horizontal {
            if bottom.y1 <= top.y1 {
                continue;
            }

            let min_x = top.min_x();
            let max_x = top.max_x();
            if (bottom.min_x() - min_x).abs() > 0.5 || (bottom.max_x() - max_x).abs() > 0.5 {
                continue;
            }

            let has_left = vertical.iter().any(|line| {
                (line.x1 - min_x).abs() <= 0.5
                    && (line.min_y() - top.y1).abs() <= 0.5
                    && (line.max_y() - bottom.y1).abs() <= 0.5
            });
            let has_right = vertical.iter().any(|line| {
                (line.x1 - max_x).abs() <= 0.5
                    && (line.min_y() - top.y1).abs() <= 0.5
                    && (line.max_y() - bottom.y1).abs() <= 0.5
            });

            if has_left && has_right {
                fragments.push(SequenceBox::new(
                    "fragment",
                    "loop fragment",
                    min_x,
                    top.y1,
                    max_x - min_x,
                    bottom.y1 - top.y1,
                ));
            }
        }
    }

    fragments.sort_by(|a, b| {
        a.y.partial_cmp(&b.y)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.x.partial_cmp(&b.x).unwrap_or(std::cmp::Ordering::Equal))
    });
    fragments.dedup_by(|a, b| {
        (a.x - b.x).abs() <= 0.5
            && (a.y - b.y).abs() <= 0.5
            && (a.width - b.width).abs() <= 0.5
            && (a.height - b.height).abs() <= 0.5
    });

    if fragments.is_empty() {
        return aggregate_fragment_box(lines).into_iter().collect();
    }

    fragments
}

fn aggregate_fragment_box(lines: &[SequenceLine]) -> Option<SequenceBox> {
    if lines.is_empty() {
        return None;
    }

    let min_x = lines
        .iter()
        .map(|line| line.min_x())
        .fold(f64::INFINITY, f64::min);
    let max_x = lines
        .iter()
        .map(|line| line.max_x())
        .fold(f64::NEG_INFINITY, f64::max);
    let min_y = lines
        .iter()
        .map(|line| line.min_y())
        .fold(f64::INFINITY, f64::min);
    let max_y = lines
        .iter()
        .map(|line| line.max_y())
        .fold(f64::NEG_INFINITY, f64::max);

    Some(SequenceBox::new(
        "fragment",
        "loop fragment",
        min_x,
        min_y,
        max_x - min_x,
        max_y - min_y,
    ))
}

fn fragment_frame_groups(doc: &roxmltree::Document<'_>) -> Vec<SequenceBox> {
    let mut groups = Vec::new();
    for group in doc
        .descendants()
        .filter(|node| node.tag_name().name() == "g")
    {
        let lines: Vec<_> = group
            .children()
            .filter(|node| node.tag_name().name() == "line" && has_class(node, "loopLine"))
            .filter_map(|line| {
                line_coords(&line).map(|(x1, y1, x2, y2)| SequenceLine::new(x1, y1, x2, y2))
            })
            .collect();
        if lines.len() == 4 {
            if let Some(frame) = aggregate_fragment_box(&lines) {
                groups.push(frame);
            }
        }
    }
    groups
}

fn fragment_border_boxes(fragment: &SequenceBox) -> Vec<SequenceBox> {
    let thickness = SEQUENCE_OVERLAP_TOLERANCE * 2.0;
    vec![
        SequenceBox::new(
            "fragment_border",
            "loop fragment top border",
            fragment.x,
            fragment.y - SEQUENCE_OVERLAP_TOLERANCE,
            fragment.width,
            thickness,
        ),
        SequenceBox::new(
            "fragment_border",
            "loop fragment bottom border",
            fragment.x,
            fragment.y + fragment.height - SEQUENCE_OVERLAP_TOLERANCE,
            fragment.width,
            thickness,
        ),
        SequenceBox::new(
            "fragment_border",
            "loop fragment left border",
            fragment.x - SEQUENCE_OVERLAP_TOLERANCE,
            fragment.y,
            thickness,
            fragment.height,
        ),
        SequenceBox::new(
            "fragment_border",
            "loop fragment right border",
            fragment.x + fragment.width - SEQUENCE_OVERLAP_TOLERANCE,
            fragment.y,
            thickness,
            fragment.height,
        ),
    ]
}

fn is_message_path(node: &roxmltree::Node<'_, '_>) -> bool {
    let class = node.attribute("class").unwrap_or("");
    class.contains("message-line")
        || class.contains("messageLine0")
        || class.contains("messageLine1")
}

fn path_points(path: &str) -> Vec<(f64, f64)> {
    let normalized: String = path
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphabetic() || ch == ',' {
                ' '
            } else {
                ch
            }
        })
        .collect();
    let nums: Vec<f64> = normalized
        .split_whitespace()
        .filter_map(|part| part.parse::<f64>().ok())
        .collect();

    nums.chunks_exact(2)
        .map(|chunk| (chunk[0], chunk[1]))
        .collect()
}

fn box_from_points(kind: &'static str, label: &str, points: &[(f64, f64)]) -> SequenceBox {
    let mut min_x = f64::INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut max_y = f64::NEG_INFINITY;

    for (x, y) in points {
        min_x = min_x.min(*x);
        min_y = min_y.min(*y);
        max_x = max_x.max(*x);
        max_y = max_y.max(*y);
    }

    SequenceBox::new(kind, label, min_x, min_y, max_x - min_x, max_y - min_y)
}

fn actor_label(node: &roxmltree::Node<'_, '_>) -> String {
    enclosing_group_text(node, |text| has_class(text, "actor-box")).unwrap_or_else(|| {
        node.attribute("id")
            .map(str::to_string)
            .unwrap_or_else(|| "actor".to_string())
    })
}

fn note_label(node: &roxmltree::Node<'_, '_>) -> String {
    enclosing_group_text(node, |text| has_class(text, "noteText")).unwrap_or_else(|| {
        node.attribute("id")
            .map(str::to_string)
            .unwrap_or_else(|| "note".to_string())
    })
}

fn enclosing_group_text<F>(node: &roxmltree::Node<'_, '_>, predicate: F) -> Option<String>
where
    F: Fn(&roxmltree::Node<'_, '_>) -> bool,
{
    let group = node
        .ancestors()
        .find(|node| node.tag_name().name() == "g")?;
    group
        .descendants()
        .find(|node| node.tag_name().name() == "text" && predicate(node))
        .map(|text| node_text(&text))
}

fn has_class(node: &roxmltree::Node<'_, '_>, class_name: &str) -> bool {
    node.attribute("class")
        .unwrap_or("")
        .split_whitespace()
        .any(|class| class == class_name)
}

fn parse_attr(node: &roxmltree::Node<'_, '_>, attr: &str) -> Option<f64> {
    node.attribute(attr).and_then(parse_f64)
}

fn parse_f64(value: &str) -> Option<f64> {
    value.parse::<f64>().ok()
}

fn node_text(node: &roxmltree::Node<'_, '_>) -> String {
    node.descendants()
        .filter(|descendant| descendant.is_text())
        .filter_map(|descendant| descendant.text())
        .collect::<Vec<_>>()
        .join("")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_sequence_svg_geometry_once() {
        let svg = r##"<svg width="300" height="220" viewBox="-10 0 300 220" xmlns="http://www.w3.org/2000/svg">
          <g class="actor">
            <rect class="actor actor-box" x="0" y="0" width="150" height="65"/>
            <text class="actor actor-box" x="75" y="32.5" text-anchor="middle">Alice</text>
          </g>
          <line class="actor-line" x1="75" y1="65" x2="75" y2="180"/>
          <g>
            <line class="loopLine" x1="40" y1="80" x2="220" y2="80"/>
            <line class="loopLine" x1="220" y1="80" x2="220" y2="160"/>
            <line class="loopLine" x1="40" y1="160" x2="220" y2="160"/>
            <line class="loopLine" x1="40" y1="80" x2="40" y2="160"/>
            <text class="labelText" x="65" y="95" text-anchor="middle">loop</text>
            <text class="loopText" x="130" y="98" text-anchor="middle">[Healthcheck]</text>
          </g>
          <defs>
            <marker id="arrow-filled" refX="7.9" refY="5" markerWidth="12" markerHeight="12" markerUnits="userSpaceOnUse" orient="auto"/>
          </defs>
          <path class="message-line" marker-end="url(#arrow-filled)" d="M 75 110 L 115 110 L 115 140 L 75 140"/>
          <text class="message-label" x="120" y="125" text-anchor="start">Fight</text>
          <g class="note">
            <rect class="note" x="95" y="165" width="150" height="39"/>
            <text class="noteText" x="170" y="170" text-anchor="middle" dominant-baseline="middle" dy="1em">Rational</text>
          </g>
        </svg>"##;

        let geometry = SequenceGeometry::parse(svg).expect("geometry");

        assert_eq!(geometry.svg_width(), Some(300.0));
        assert_eq!(geometry.svg_visible_right(), Some(290.0));
        assert_eq!(
            geometry.actor_box_containing("Alice").expect("actor").x,
            0.0
        );
        assert_eq!(
            geometry.actor_box_containing("Alice").expect("actor").label,
            "Alice"
        );
        assert_eq!(geometry.lifeline_x_for_actor("Alice"), Some(75.0));
        let message = geometry.text_box_containing("Fight").expect("message");
        assert_eq!(message.kind, "message-label");
        assert_eq!(message.label, "Fight");
        assert_eq!(
            geometry
                .text_box_containing("loop")
                .expect("label text")
                .kind,
            "labelText"
        );
        assert_eq!(
            geometry
                .marker("arrow-filled")
                .expect("arrow marker")
                .marker_units
                .as_deref(),
            Some("userSpaceOnUse")
        );
        assert_eq!(
            geometry
                .note_box_containing("Rational")
                .expect("note")
                .width,
            150.0
        );
        assert!(geometry.self_message_path_box().expect("self path").width >= 40.0);
        assert_eq!(
            geometry.self_message_paths().first().expect("self path"),
            "M 75 110 L 115 110 L 115 140 L 75 140"
        );
        assert_eq!(
            geometry.first_fragment_frame_group().expect("frame").x,
            40.0
        );
        assert!(geometry.fragment_headers().len() == 1);
        assert!(geometry.fragment_borders().len() == 4);
    }
}
