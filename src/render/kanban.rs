//! Kanban diagram renderer
//!
//! Renders kanban board diagrams showing columns (sections) with task cards.
//! Based on the mermaid.js reference implementation.

use crate::diagrams::kanban::{KanbanDb, KanbanNode, Priority};
use crate::error::Result;
use crate::render::svg::{Attrs, RenderConfig, SvgDocument, SvgElement};

// Layout configuration (matching mermaid.js defaults)
/// Width of each section column
const SECTION_WIDTH: f64 = 200.0;
/// Padding around elements
const PADDING: f64 = 10.0;
/// Corner radius for rectangles
const RX: f64 = 5.0;
/// Minimum section height
const MIN_SECTION_HEIGHT: f64 = 50.0;
/// Font size for labels
const FONT_SIZE: f64 = 14.0;
/// Line height for wrapped text
const LINE_HEIGHT: f64 = 18.0;
/// Max label width for wrapping
const MAX_LABEL_WIDTH: f64 = 180.0;
/// Minimum item height
const MIN_ITEM_HEIGHT: f64 = 40.0;

/// Render a kanban diagram to SVG
pub fn render_kanban(db: &KanbanDb, config: &RenderConfig) -> Result<String> {
    let mut doc = SvgDocument::new();

    let nodes = db.get_nodes();
    let sections = db.get_sections();

    // Handle empty diagram
    if nodes.is_empty() && sections.is_empty() {
        doc.set_size(400.0, 200.0);
        return Ok(doc.to_string());
    }

    // Calculate layout
    let layout = calculate_layout(db);

    doc.set_size(layout.total_width, layout.total_height);

    // Add CSS styles
    if config.embed_css {
        doc.add_style(&generate_kanban_css(config));
    }

    // Render sections and items
    let sections_elem = render_sections(db, &layout, config);
    doc.add_node(sections_elem);

    let items_elem = render_items(db, &layout, config);
    doc.add_node(items_elem);

    Ok(doc.to_string())
}

/// Layout information for the kanban board
struct KanbanLayout {
    total_width: f64,
    total_height: f64,
    /// Heights of each section (by index)
    section_heights: Vec<f64>,
    /// Y positions of items within each section
    item_positions: Vec<Vec<(f64, f64)>>, // (y, height) for each item
}

/// Calculate layout dimensions for the kanban board
fn calculate_layout(db: &KanbanDb) -> KanbanLayout {
    let sections = db.get_sections();
    let num_sections = sections.len().max(1);

    // Calculate max label height for section headers
    let mut max_label_height: f64 = 25.0;
    for section in &sections {
        let height = estimate_text_height(&section.label, MAX_LABEL_WIDTH);
        max_label_height = max_label_height.max(height);
    }

    // Calculate heights for each section based on their items
    let mut section_heights = Vec::new();
    let mut item_positions: Vec<Vec<(f64, f64)>> = Vec::new();

    for section in &sections {
        let children = db.get_children(&section.id);
        let mut section_content_height = 0.0;
        let mut positions = Vec::new();

        for child in &children {
            let item_height = calculate_item_height(child);
            let y = max_label_height + section_content_height;
            positions.push((y, item_height));
            section_content_height += item_height + PADDING / 2.0;
        }

        // Minimum section height or content height plus padding
        let height = (section_content_height + 3.0 * PADDING).max(MIN_SECTION_HEIGHT)
            + max_label_height
            - 25.0;
        section_heights.push(height);
        item_positions.push(positions);
    }

    // Total dimensions
    let total_width =
        (num_sections as f64) * SECTION_WIDTH + ((num_sections - 1).max(0) as f64) * PADDING / 2.0;
    let total_height = section_heights
        .iter()
        .cloned()
        .fold(0.0_f64, |a, b| a.max(b))
        + PADDING * 2.0;

    KanbanLayout {
        total_width,
        total_height,
        section_heights,
        item_positions,
    }
}

/// Calculate the height needed for a kanban item
fn calculate_item_height(node: &KanbanNode) -> f64 {
    let label_height = estimate_text_height(&node.label, MAX_LABEL_WIDTH - 2.0 * PADDING);

    // Add extra height for metadata (ticket, assigned)
    let mut metadata_height = 0.0;
    if node.ticket.is_some() || node.assigned.is_some() {
        metadata_height = LINE_HEIGHT;
    }

    (label_height + metadata_height + 2.0 * PADDING).max(MIN_ITEM_HEIGHT)
}

/// Estimate text height based on wrapping
fn estimate_text_height(text: &str, max_width: f64) -> f64 {
    let lines = wrap_text_lines(text, max_width);
    (lines.len() as f64) * LINE_HEIGHT
}

/// Wrap text into lines based on max width
fn wrap_text_lines(text: &str, max_width: f64) -> Vec<String> {
    let words: Vec<&str> = text.split_whitespace().collect();
    if words.is_empty() {
        return vec![String::new()];
    }

    let mut lines = Vec::new();
    let mut current_line = String::new();

    for word in words {
        if current_line.is_empty() {
            current_line = word.to_string();
        } else {
            let potential = format!("{} {}", current_line, word);
            let estimated_width = estimate_text_width(&potential);
            if estimated_width <= max_width {
                current_line = potential;
            } else {
                lines.push(current_line);
                current_line = word.to_string();
            }
        }
    }
    if !current_line.is_empty() {
        lines.push(current_line);
    }

    lines
}

/// Estimate text width in pixels
fn estimate_text_width(text: &str) -> f64 {
    // Approximate character width for proportional font
    text.chars().count() as f64 * FONT_SIZE * 0.55
}

/// Render section columns
fn render_sections(db: &KanbanDb, layout: &KanbanLayout, _config: &RenderConfig) -> SvgElement {
    let sections = db.get_sections();
    let mut children = Vec::new();

    for (idx, section) in sections.iter().enumerate() {
        let x = (idx as f64) * (SECTION_WIDTH + PADDING / 2.0);
        let height = layout.section_heights.get(idx).cloned().unwrap_or(200.0);

        // Section background
        let rect = SvgElement::Rect {
            x,
            y: 0.0,
            width: SECTION_WIDTH,
            height,
            rx: Some(RX),
            ry: Some(RX),
            attrs: Attrs::new().with_class(&format!(
                "section section-{} {}",
                (idx % 12) + 1,
                section.css_classes.as_deref().unwrap_or("")
            )),
        };
        children.push(rect);

        // Section label
        let label = render_section_label(&section.label, x, SECTION_WIDTH);
        children.push(label);
    }

    SvgElement::Group {
        children,
        attrs: Attrs::new().with_class("sections"),
    }
}

/// Render section label with text wrapping
fn render_section_label(text: &str, section_x: f64, section_width: f64) -> SvgElement {
    let lines = wrap_text_lines(text, section_width - 2.0 * PADDING);

    if lines.len() == 1 {
        SvgElement::Text {
            x: section_x + section_width / 2.0,
            y: PADDING + LINE_HEIGHT / 2.0,
            content: lines[0].clone(),
            attrs: Attrs::new()
                .with_class("section-label")
                .with_attr("text-anchor", "middle")
                .with_attr("dominant-baseline", "middle"),
        }
    } else {
        // Multi-line text using tspans
        let mut tspan_content = String::new();
        let start_y = PADDING;
        for (i, line) in lines.iter().enumerate() {
            let dy = if i == 0 {
                "0.5em".to_string()
            } else {
                "1.2em".to_string()
            };
            tspan_content.push_str(&format!(
                r#"<tspan x="{}" dy="{}">{}</tspan>"#,
                section_x + section_width / 2.0,
                dy,
                escape_xml(line)
            ));
        }

        SvgElement::Raw {
            content: format!(
                r#"<text x="{}" y="{}" text-anchor="middle" class="section-label">{}</text>"#,
                section_x + section_width / 2.0,
                start_y,
                tspan_content
            ),
        }
    }
}

/// Render kanban items (cards)
fn render_items(db: &KanbanDb, layout: &KanbanLayout, _config: &RenderConfig) -> SvgElement {
    let sections = db.get_sections();
    let mut children = Vec::new();

    for (section_idx, section) in sections.iter().enumerate() {
        let section_x = (section_idx as f64) * (SECTION_WIDTH + PADDING / 2.0);
        let items = db.get_children(&section.id);
        let positions = layout
            .item_positions
            .get(section_idx)
            .cloned()
            .unwrap_or_default();

        for (item_idx, item) in items.iter().enumerate() {
            if let Some((y, height)) = positions.get(item_idx) {
                let item_elem = render_item(
                    item,
                    section_x + PADDING,
                    *y,
                    SECTION_WIDTH - 2.0 * PADDING,
                    *height,
                );
                children.push(item_elem);
            }
        }
    }

    SvgElement::Group {
        children,
        attrs: Attrs::new().with_class("items"),
    }
}

/// Render a single kanban item (card)
fn render_item(node: &KanbanNode, x: f64, y: f64, width: f64, height: f64) -> SvgElement {
    let mut children = Vec::new();

    // Item background rectangle
    let rect = SvgElement::Rect {
        x: 0.0,
        y: 0.0,
        width,
        height,
        rx: Some(RX),
        ry: Some(RX),
        attrs: Attrs::new()
            .with_class(&format!(
                "kanban-item {}",
                node.css_classes.as_deref().unwrap_or("")
            ))
            .with_fill("#fff")
            .with_stroke("#ccc"),
    };
    children.push(rect);

    // Priority indicator (colored left border)
    if let Some(ref priority_str) = node.priority {
        if let Some(priority) = Priority::from_str(priority_str) {
            let color = priority_color(&priority);
            let line = SvgElement::Line {
                x1: 2.0,
                y1: RX / 2.0,
                x2: 2.0,
                y2: height - RX / 2.0,
                attrs: Attrs::new()
                    .with_stroke(color)
                    .with_stroke_width(4.0)
                    .with_class("priority-indicator"),
            };
            children.push(line);
        }
    }

    // Item label
    let label_y = PADDING;
    let label = render_item_label(&node.label, width / 2.0, label_y, width - 2.0 * PADDING);
    children.push(label);

    // Metadata row (ticket and assigned)
    let has_metadata = node.ticket.is_some() || node.assigned.is_some();
    if has_metadata {
        let metadata_y = height - PADDING - LINE_HEIGHT / 2.0;

        // Ticket (left side)
        if let Some(ref ticket) = node.ticket {
            let ticket_text = SvgElement::Text {
                x: PADDING,
                y: metadata_y,
                content: ticket.clone(),
                attrs: Attrs::new()
                    .with_class("kanban-ticket")
                    .with_attr("text-anchor", "start")
                    .with_attr("font-size", "12px"),
            };
            children.push(ticket_text);
        }

        // Assigned (right side)
        if let Some(ref assigned) = node.assigned {
            let assigned_text = SvgElement::Text {
                x: width - PADDING,
                y: metadata_y,
                content: assigned.clone(),
                attrs: Attrs::new()
                    .with_class("kanban-assigned")
                    .with_attr("text-anchor", "end")
                    .with_attr("font-size", "12px"),
            };
            children.push(assigned_text);
        }
    }

    SvgElement::Group {
        children,
        attrs: Attrs::new()
            .with_class("kanban-item-group")
            .with_attr("transform", &format!("translate({}, {})", x, y)),
    }
}

/// Render item label with text wrapping
fn render_item_label(text: &str, cx: f64, y: f64, max_width: f64) -> SvgElement {
    let lines = wrap_text_lines(text, max_width);

    if lines.len() == 1 {
        SvgElement::Text {
            x: cx,
            y: y + LINE_HEIGHT / 2.0,
            content: lines[0].clone(),
            attrs: Attrs::new()
                .with_class("kanban-label")
                .with_attr("text-anchor", "middle")
                .with_attr("dominant-baseline", "middle"),
        }
    } else {
        let mut tspan_content = String::new();
        for (i, line) in lines.iter().enumerate() {
            let dy = if i == 0 {
                "0.5em".to_string()
            } else {
                "1.2em".to_string()
            };
            tspan_content.push_str(&format!(
                r#"<tspan x="{}" dy="{}">{}</tspan>"#,
                cx,
                dy,
                escape_xml(line)
            ));
        }

        SvgElement::Raw {
            content: format!(
                r#"<text x="{}" y="{}" text-anchor="middle" class="kanban-label">{}</text>"#,
                cx, y, tspan_content
            ),
        }
    }
}

/// Get color for priority level
fn priority_color(priority: &Priority) -> &'static str {
    match priority {
        Priority::VeryHigh => "red",
        Priority::High => "orange",
        Priority::Medium => "#ccc", // No visible indicator
        Priority::Low => "blue",
        Priority::VeryLow => "lightblue",
    }
}

/// Generate CSS for kanban diagrams
fn generate_kanban_css(config: &RenderConfig) -> String {
    let theme = &config.theme;

    // Generate section colors (matching mermaid.js theme)
    let mut section_css = String::new();
    for i in 1..=12 {
        let hue = (i as f64 - 1.0) * 30.0;
        section_css.push_str(&format!(
            r#"
.section-{i} {{
  fill: hsl({hue}, 60%, 90%);
  stroke: hsl({hue}, 60%, 70%);
  stroke-width: 1px;
}}
"#,
            i = i,
            hue = hue
        ));
    }

    format!(
        r#"
.sections {{
  font-family: {font_family};
}}

.section {{
  fill: #f0f0f0;
  stroke: #ccc;
  stroke-width: 1px;
}}

.section-label {{
  fill: {text_color};
  font-family: {font_family};
  font-size: 14px;
  font-weight: bold;
}}

.items {{
  font-family: {font_family};
}}

.kanban-item {{
  fill: #ffffff;
  stroke: #ccc;
  stroke-width: 1px;
}}

.kanban-label {{
  fill: {text_color};
  font-family: {font_family};
  font-size: 14px;
}}

.kanban-ticket {{
  fill: #666;
  font-family: {font_family};
}}

.kanban-assigned {{
  fill: #666;
  font-family: {font_family};
}}

.priority-indicator {{
  stroke-linecap: round;
}}

{section_css}
"#,
        font_family = theme.font_family,
        text_color = theme.primary_text_color,
        section_css = section_css
    )
}

/// Escape XML special characters
fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_empty_kanban() {
        let db = KanbanDb::new();
        let config = RenderConfig::default();
        let result = render_kanban(&db, &config);
        assert!(result.is_ok());
        let svg = result.unwrap();
        assert!(svg.contains("<svg"));
    }

    #[test]
    fn test_render_kanban_with_sections() {
        let mut db = KanbanDb::new();
        use crate::diagrams::kanban::NodeShape;
        db.add_node(0, Some("todo"), "Todo", NodeShape::Default);
        db.add_node(1, Some("task1"), "Create Documentation", NodeShape::Default);
        db.add_node(0, Some("done"), "Done", NodeShape::Default);
        db.add_node(1, Some("task2"), "Completed Task", NodeShape::Default);

        let config = RenderConfig::default();
        let result = render_kanban(&db, &config);
        assert!(result.is_ok());
        let svg = result.unwrap();
        assert!(svg.contains("Todo"));
        assert!(svg.contains("Done"));
        assert!(svg.contains("Create Documentation"));
    }

    #[test]
    fn test_render_kanban_with_priority() {
        let mut db = KanbanDb::new();
        use crate::diagrams::kanban::NodeShape;
        db.add_node(0, Some("section"), "Tasks", NodeShape::Default);
        db.add_node(1, Some("task1"), "High Priority Task", NodeShape::Default);
        db.set_metadata("priority", "High");

        let config = RenderConfig::default();
        let result = render_kanban(&db, &config);
        assert!(result.is_ok());
        let svg = result.unwrap();
        assert!(svg.contains("priority-indicator"));
    }

    #[test]
    fn test_wrap_text_lines() {
        let text = "This is a very long text that should be wrapped";
        let lines = wrap_text_lines(text, 100.0);
        assert!(lines.len() > 1);
    }
}
