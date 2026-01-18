//! Timeline diagram renderer
//!
//! Renders timeline diagrams following Mermaid.js conventions:
//! - Sections displayed as boxes at the top (spanning the width of their tasks)
//! - Tasks displayed below sections in columns
//! - Events displayed below tasks with dashed lines connecting them
//! - A horizontal timeline line with an arrow at the bottom

use crate::diagrams::timeline::{TimelineDb, TimelineTask};
use crate::error::Result;
use crate::render::svg::{Attrs, RenderConfig, SvgDocument, SvgElement};

// Mermaid-compatible layout constants
const LEFT_MARGIN: f64 = 100.0; // Match reference (starts content at x=200 from viewBox x=100)
const TOP_MARGIN: f64 = 50.0;
const NODE_WIDTH: f64 = 180.0; // Match reference (h180 in path)
const NODE_PADDING: f64 = 20.0;
const COLUMN_WIDTH: f64 = 200.0;
const SECTION_HEIGHT: f64 = 68.0; // ~68px in reference
const TASK_HEIGHT: f64 = 68.0; // ~68px in reference
const EVENT_HEIGHT: f64 = 65.0; // ~65px minimum in reference
const EVENT_SPACING: f64 = 10.0;
const SECTION_GAP: f64 = 50.0;
const TASK_GAP: f64 = 100.0;
const FONT_SIZE: f64 = 16.0; // Match mermaid.js default
const TITLE_FONT_SIZE: f64 = 24.0;
const MAX_SECTIONS: usize = 12;

/// Render a timeline diagram to SVG
pub fn render_timeline(db: &TimelineDb, config: &RenderConfig) -> Result<String> {
    let mut doc = SvgDocument::new();

    let tasks = db.get_tasks();
    let sections = db.get_sections();

    // Handle empty diagram
    if tasks.is_empty() && sections.is_empty() {
        doc.set_size(400.0, 200.0);
        if !db.title.is_empty() {
            let title_elem = SvgElement::Text {
                x: 200.0,
                y: 30.0,
                content: db.title.clone(),
                attrs: Attrs::new()
                    .with_attr("text-anchor", "middle")
                    .with_class("titleText")
                    .with_attr("font-size", &format!("{}", TITLE_FONT_SIZE as i32))
                    .with_attr("font-weight", "bold"),
            };
            doc.add_element(title_elem);
        }
        return Ok(doc.to_string());
    }

    // Calculate layout
    let has_sections = !sections.is_empty();
    let layout = calculate_layout(tasks, sections, has_sections);

    doc.set_size(layout.total_width, layout.total_height);

    // Add theme styles
    if config.embed_css {
        doc.add_style(&config.theme.generate_css());
        doc.add_style(&generate_timeline_css(&config.theme));
    }

    // Add arrowhead marker
    add_arrowhead_marker(&mut doc);

    // Render title (at top)
    if !db.title.is_empty() {
        let title_elem = SvgElement::Text {
            x: layout.total_width / 2.0 - LEFT_MARGIN,
            y: 20.0,
            content: db.title.clone(),
            attrs: Attrs::new()
                .with_attr("text-anchor", "middle")
                .with_class("titleText")
                .with_attr("font-size", "4ex")
                .with_attr("font-weight", "bold"),
        };
        doc.add_element(title_elem);
    }

    // Render based on whether we have sections
    if has_sections {
        render_with_sections(&mut doc, db, &layout);
    } else {
        render_without_sections(&mut doc, db, &layout);
    }

    // Render horizontal timeline line at the bottom
    render_timeline_line(&mut doc, &layout);

    Ok(doc.to_string())
}

/// Layout information for the timeline
struct TimelineLayout {
    total_width: f64,
    total_height: f64,
    depth_y: f64,               // Y position of the timeline line
    section_begin_y: f64,       // Y position where sections start
    max_section_height: f64,    // Maximum height of section boxes
    max_task_height: f64,       // Maximum height of task boxes
    max_event_line_length: f64, // Maximum total height of events for any task
}

/// Calculate layout dimensions
fn calculate_layout(
    tasks: &[TimelineTask],
    sections: &[String],
    has_sections: bool,
) -> TimelineLayout {
    // Calculate maximum section height based on text wrapping
    let max_section_height: f64 = if has_sections {
        let mut max_height: f64 = 0.0;
        for section in sections {
            let height = estimate_node_height(section, NODE_WIDTH);
            max_height = max_height.max(height);
        }
        max_height.max(SECTION_HEIGHT)
    } else {
        0.0
    };

    // Calculate maximum task height and event line length
    let mut max_task_height: f64 = 0.0;
    let mut _max_event_count = 0;
    let mut max_event_line_length: f64 = 0.0;

    for task in tasks {
        let height = estimate_node_height(&task.task, NODE_WIDTH);
        max_task_height = max_task_height.max(height);
        _max_event_count = _max_event_count.max(task.events.len());

        // Calculate event line length for this task
        let mut event_line_length: f64 = 0.0;
        for event in &task.events {
            event_line_length += estimate_node_height(event, NODE_WIDTH);
        }
        if !task.events.is_empty() {
            event_line_length += (task.events.len() - 1) as f64 * EVENT_SPACING;
        }
        max_event_line_length = max_event_line_length.max(event_line_length);
    }
    max_task_height = max_task_height.max(TASK_HEIGHT);

    // Calculate total number of columns (tasks across all sections)
    let total_columns = tasks.len().max(1);
    // Width: left margin + columns + right margin for timeline arrow
    let total_width = LEFT_MARGIN + (total_columns as f64) * COLUMN_WIDTH + LEFT_MARGIN * 2.0;

    // Calculate depth_y (position of timeline line)
    let section_begin_y = TOP_MARGIN;
    let depth_y = if has_sections {
        max_section_height + max_task_height + 150.0
    } else {
        max_task_height + 100.0
    };

    // Total height includes title, sections, tasks, events, and timeline line
    // Add extra margin to match reference which has ~340px below the timeline line
    let total_height = depth_y + max_event_line_length + 280.0;

    TimelineLayout {
        total_width,
        total_height,
        depth_y,
        section_begin_y,
        max_section_height,
        max_task_height,
        max_event_line_length,
    }
}

/// Estimate node height based on text content and wrapping
fn estimate_node_height(text: &str, width: f64) -> f64 {
    // Split on <br> tags
    let lines: Vec<&str> = text.split("<br>").collect();
    let mut total_lines = 0;

    for line in lines {
        // Estimate characters per line
        let chars_per_line = (width / (FONT_SIZE * 0.5)).floor() as usize;
        let line_count = if chars_per_line > 0 {
            (line.len() / chars_per_line).max(1)
        } else {
            1
        };
        total_lines += line_count;
    }

    let height = total_lines as f64 * FONT_SIZE * 1.1 + NODE_PADDING;
    height.max(EVENT_HEIGHT)
}

/// Add arrowhead marker definition
fn add_arrowhead_marker(doc: &mut SvgDocument) {
    let marker = SvgElement::Defs {
        children: vec![SvgElement::Raw {
            content: r#"<marker id="arrowhead" refX="5" refY="2" markerWidth="6" markerHeight="4" orient="auto">
                <path d="M 0,0 V 4 L6,2 Z"></path>
            </marker>"#.to_string(),
        }],
    };
    doc.add_element(marker);
}

/// Render timeline with sections
fn render_with_sections(doc: &mut SvgDocument, db: &TimelineDb, layout: &TimelineLayout) {
    let sections = db.get_sections();
    let tasks = db.get_tasks();

    let mut master_x = LEFT_MARGIN;

    for (section_number, section) in sections.iter().enumerate() {
        // Filter tasks for this section
        let section_tasks: Vec<&TimelineTask> =
            tasks.iter().filter(|t| t.section == *section).collect();

        let task_count = section_tasks.len().max(1);
        let section_width = (task_count as f64) * COLUMN_WIDTH - SECTION_GAP;

        // Render section box
        render_section_node(
            doc,
            section,
            section_number,
            master_x,
            layout.section_begin_y,
            section_width,
            layout.max_section_height,
        );

        // Render tasks for this section
        let master_y = layout.section_begin_y + layout.max_section_height + SECTION_GAP;
        render_tasks(
            doc,
            &section_tasks,
            section_number,
            master_x,
            master_y,
            layout,
        );

        // Move to next section column
        master_x += section_width + SECTION_GAP;
    }
}

/// Render timeline without sections
fn render_without_sections(doc: &mut SvgDocument, db: &TimelineDb, layout: &TimelineLayout) {
    let tasks = db.get_tasks();
    let tasks_ref: Vec<&TimelineTask> = tasks.iter().collect();

    let master_x = LEFT_MARGIN;
    let master_y = layout.section_begin_y;

    // Render tasks, each in a different section color
    render_tasks_multicolor(doc, &tasks_ref, master_x, master_y, layout);
}

/// Render a section node
fn render_section_node(
    doc: &mut SvgDocument,
    text: &str,
    section_num: usize,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
) {
    let section_class = format!("timeline-node section-{}", section_num % MAX_SECTIONS);

    // Background path with rounded top
    let rd = 5.0;
    let path_d = format!(
        "M0 {} v{} q0,-5 5,-5 h{} q5,0 5,5 v{} H0 Z",
        height - rd,
        -(height - 2.0 * rd),
        width - 2.0 * rd,
        height - rd
    );

    let mut group_children = Vec::new();

    // Background
    group_children.push(SvgElement::Path {
        d: path_d,
        attrs: Attrs::new().with_class(&format!(
            "node-bkg node-section-{}",
            section_num % MAX_SECTIONS
        )),
    });

    // Bottom line
    group_children.push(SvgElement::Line {
        x1: 0.0,
        y1: height,
        x2: width,
        y2: height,
        attrs: Attrs::new().with_class(&format!("node-line-{}", section_num % MAX_SECTIONS)),
    });

    // Text (centered)
    let text_elem = wrap_text(text, width / 2.0, height / 2.0, width - NODE_PADDING * 2.0);
    group_children.push(text_elem);

    let group = SvgElement::Group {
        children: group_children,
        attrs: Attrs::new()
            .with_class(&section_class)
            .with_attr("transform", &format!("translate({}, {})", x, y)),
    };
    doc.add_element(group);
}

/// Render tasks for a section
fn render_tasks(
    doc: &mut SvgDocument,
    tasks: &[&TimelineTask],
    section_color: usize,
    start_x: f64,
    start_y: f64,
    layout: &TimelineLayout,
) {
    let mut master_x = start_x;

    for task in tasks {
        render_task_node(doc, task, section_color, master_x, start_y, layout);
        master_x += COLUMN_WIDTH;
    }
}

/// Render tasks with multicolor (no sections)
fn render_tasks_multicolor(
    doc: &mut SvgDocument,
    tasks: &[&TimelineTask],
    start_x: f64,
    start_y: f64,
    layout: &TimelineLayout,
) {
    let mut master_x = start_x;

    for (idx, task) in tasks.iter().enumerate() {
        render_task_node(doc, task, idx, master_x, start_y, layout);
        master_x += COLUMN_WIDTH;
    }
}

/// Render a single task node with its events
fn render_task_node(
    doc: &mut SvgDocument,
    task: &TimelineTask,
    section_color: usize,
    x: f64,
    y: f64,
    layout: &TimelineLayout,
) {
    let node_class = format!("timeline-node section-{}", section_color % MAX_SECTIONS);
    let width = NODE_WIDTH + NODE_PADDING * 2.0;
    let height = layout.max_task_height;

    // Task box background
    let rd = 5.0;
    let path_d = format!(
        "M0 {} v{} q0,-5 5,-5 h{} q5,0 5,5 v{} H0 Z",
        height - rd,
        -(height - 2.0 * rd),
        width - 2.0 * rd,
        height - rd
    );

    let mut task_children = Vec::new();

    // Background
    task_children.push(SvgElement::Path {
        d: path_d,
        attrs: Attrs::new().with_class(&format!(
            "node-bkg node-section-{}",
            section_color % MAX_SECTIONS
        )),
    });

    // Bottom line
    task_children.push(SvgElement::Line {
        x1: 0.0,
        y1: height,
        x2: width,
        y2: height,
        attrs: Attrs::new().with_class(&format!("node-line-{}", section_color % MAX_SECTIONS)),
    });

    // Text
    let text_elem = wrap_text(&task.task, width / 2.0, height / 2.0, NODE_WIDTH);
    task_children.push(text_elem);

    let task_group = SvgElement::Group {
        children: task_children,
        attrs: Attrs::new()
            .with_class(&format!("taskWrapper {}", node_class))
            .with_attr("transform", &format!("translate({}, {})", x, y)),
    };
    doc.add_element(task_group);

    // Render events if present
    if !task.events.is_empty() {
        render_events(doc, task, section_color, x, y + height, layout);
    }
}

/// Render events for a task
fn render_events(
    doc: &mut SvgDocument,
    task: &TimelineTask,
    section_color: usize,
    task_x: f64,
    task_bottom_y: f64,
    layout: &TimelineLayout,
) {
    let width = NODE_WIDTH + NODE_PADDING * 2.0;
    let center_x = task_x + width / 2.0;

    // Draw vertical dashed line from task to events
    let line_end_y = task_bottom_y + TASK_GAP + layout.max_event_line_length + 100.0;

    let line_wrapper = SvgElement::Group {
        children: vec![SvgElement::Line {
            x1: center_x,
            y1: task_bottom_y,
            x2: center_x,
            y2: line_end_y,
            attrs: Attrs::new()
                .with_attr("stroke-width", "2")
                .with_attr("stroke", "black")
                .with_attr("marker-end", "url(#arrowhead)")
                .with_attr("stroke-dasharray", "5,5"),
        }],
        attrs: Attrs::new().with_class("lineWrapper"),
    };
    doc.add_element(line_wrapper);

    // Render each event
    let mut event_y = task_bottom_y + TASK_GAP;
    for event in &task.events {
        let event_height = estimate_node_height(event, NODE_WIDTH);
        render_event_node(
            doc,
            event,
            section_color,
            task_x,
            event_y,
            width,
            event_height,
        );
        event_y += event_height + EVENT_SPACING;
    }
}

/// Render a single event node
fn render_event_node(
    doc: &mut SvgDocument,
    text: &str,
    section_color: usize,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
) {
    let node_class = format!("timeline-node section-{}", section_color % MAX_SECTIONS);

    // Event box background
    let rd = 5.0;
    let path_d = format!(
        "M0 {} v{} q0,-5 5,-5 h{} q5,0 5,5 v{} H0 Z",
        height - rd,
        -(height - 2.0 * rd),
        width - 2.0 * rd,
        height - rd
    );

    let mut event_children = Vec::new();

    // Background
    event_children.push(SvgElement::Path {
        d: path_d,
        attrs: Attrs::new().with_class(&format!(
            "node-bkg node-section-{}",
            section_color % MAX_SECTIONS
        )),
    });

    // Bottom line
    event_children.push(SvgElement::Line {
        x1: 0.0,
        y1: height,
        x2: width,
        y2: height,
        attrs: Attrs::new().with_class(&format!("node-line-{}", section_color % MAX_SECTIONS)),
    });

    // Text
    let text_elem = wrap_text(text, width / 2.0, height / 2.0, width - NODE_PADDING);
    event_children.push(text_elem);

    let event_group = SvgElement::Group {
        children: event_children,
        attrs: Attrs::new()
            .with_class(&format!("eventWrapper {}", node_class))
            .with_attr("transform", &format!("translate({}, {})", x, y)),
    };
    doc.add_element(event_group);
}

/// Render the horizontal timeline line
fn render_timeline_line(doc: &mut SvgDocument, layout: &TimelineLayout) {
    let line_wrapper = SvgElement::Group {
        children: vec![SvgElement::Line {
            x1: LEFT_MARGIN,
            y1: layout.depth_y,
            x2: layout.total_width - LEFT_MARGIN,
            y2: layout.depth_y,
            attrs: Attrs::new()
                .with_attr("stroke-width", "4")
                .with_attr("stroke", "black")
                .with_attr("marker-end", "url(#arrowhead)"),
        }],
        attrs: Attrs::new().with_class("lineWrapper"),
    };
    doc.add_element(line_wrapper);
}

/// Create wrapped text element
fn wrap_text(text: &str, cx: f64, cy: f64, max_width: f64) -> SvgElement {
    // Split text on <br> and whitespace
    let text = text
        .replace("<br>", "\n")
        .replace("<br/>", "\n")
        .replace("<br />", "\n");
    let words: Vec<&str> = text.split_whitespace().collect();

    if words.is_empty() {
        return SvgElement::Text {
            x: cx,
            y: cy,
            content: String::new(),
            attrs: Attrs::new()
                .with_attr("text-anchor", "middle")
                .with_attr("dominant-baseline", "middle")
                .with_attr("alignment-baseline", "middle"),
        };
    }

    // Estimate characters per line
    let chars_per_line = (max_width / (FONT_SIZE * 0.5)).floor() as usize;

    // Build lines
    let mut lines: Vec<String> = Vec::new();
    let mut current_line = String::new();

    for word in words {
        if current_line.is_empty() {
            current_line = word.to_string();
        } else if current_line.len() + 1 + word.len() <= chars_per_line {
            current_line.push(' ');
            current_line.push_str(word);
        } else {
            lines.push(current_line);
            current_line = word.to_string();
        }
    }
    if !current_line.is_empty() {
        lines.push(current_line);
    }

    if lines.len() == 1 {
        SvgElement::Text {
            x: cx,
            y: cy,
            content: lines[0].clone(),
            attrs: Attrs::new()
                .with_attr("text-anchor", "middle")
                .with_attr("dominant-baseline", "middle")
                .with_attr("alignment-baseline", "middle")
                .with_attr("dy", "1em"),
        }
    } else {
        // Multi-line: create tspans
        let total_height = lines.len() as f64 * FONT_SIZE * 1.1;
        let start_y = cy - total_height / 2.0;

        let mut tspans = Vec::new();
        for (i, line) in lines.iter().enumerate() {
            tspans.push(SvgElement::Raw {
                content: format!(
                    r#"<tspan x="{}" dy="{}">{}</tspan>"#,
                    cx,
                    if i == 0 {
                        "0em".to_string()
                    } else {
                        "1.1em".to_string()
                    },
                    escape_xml(line)
                ),
            });
        }

        SvgElement::Group {
            children: vec![SvgElement::Raw {
                content: format!(
                    r#"<text x="{}" y="{}" text-anchor="middle" dominant-baseline="middle" alignment-baseline="middle" dy="1em">{}</text>"#,
                    cx,
                    start_y,
                    tspans
                        .iter()
                        .map(|t| match t {
                            SvgElement::Raw { content } => content.clone(),
                            _ => String::new(),
                        })
                        .collect::<Vec<_>>()
                        .join("")
                ),
            }],
            attrs: Attrs::new(),
        }
    }
}

/// Escape XML special characters
fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

/// Determine if a color is dark (for choosing contrasting text color)
fn is_dark_color(color: &str) -> bool {
    // Parse hex color
    let color = color.trim_start_matches('#');
    if color.len() < 6 {
        return false;
    }

    let r = u8::from_str_radix(&color[0..2], 16).unwrap_or(128);
    let g = u8::from_str_radix(&color[2..4], 16).unwrap_or(128);
    let b = u8::from_str_radix(&color[4..6], 16).unwrap_or(128);

    // Calculate relative luminance
    let luminance = (0.299 * r as f64 + 0.587 * g as f64 + 0.114 * b as f64) / 255.0;
    luminance < 0.5
}

/// Darken a hex color by a percentage
fn darken_color(color: &str, amount: f64) -> String {
    let color = color.trim_start_matches('#');
    if color.len() < 6 {
        return format!("#{}", color);
    }

    let r = u8::from_str_radix(&color[0..2], 16).unwrap_or(128);
    let g = u8::from_str_radix(&color[2..4], 16).unwrap_or(128);
    let b = u8::from_str_radix(&color[4..6], 16).unwrap_or(128);

    let factor = 1.0 - amount;
    let r = ((r as f64) * factor) as u8;
    let g = ((g as f64) * factor) as u8;
    let b = ((b as f64) * factor) as u8;

    format!("#{:02x}{:02x}{:02x}", r, g, b)
}

/// Generate timeline-specific CSS using theme colors
fn generate_timeline_css(theme: &crate::render::svg::Theme) -> String {
    // Use theme's pie_colors as cScale colors for timeline
    // Fall back to default colors if not enough pie_colors
    let default_colors = vec![
        "#f9f".to_string(),
        "#bbf".to_string(),
        "#bfb".to_string(),
        "#fbf".to_string(),
        "#ff9".to_string(),
        "#9ff".to_string(),
        "#f99".to_string(),
        "#9f9".to_string(),
        "#99f".to_string(),
        "#fc9".to_string(),
        "#c9f".to_string(),
        "#9fc".to_string(),
    ];

    let colors: Vec<&str> = if theme.pie_colors.len() >= 10 {
        theme.pie_colors.iter().map(|s| s.as_str()).collect()
    } else {
        default_colors.iter().map(|s| s.as_str()).collect()
    };

    let mut css = format!(
        r#"
.titleText {{
  text-anchor: middle;
  font-size: 24px;
  fill: {text_color};
  font-family: {font_family};
}}

.timeline-node {{
  font-family: {font_family};
}}

.timeline-node text {{
  fill: {text_color};
  font-size: {font_size}px;
}}

.lineWrapper line {{
  stroke: {line_color};
}}
"#,
        text_color = theme.primary_text_color,
        font_family = theme.font_family,
        font_size = FONT_SIZE as i32,
        line_color = theme.line_color,
    );

    // Generate section-specific styles using theme colors
    for i in 0..MAX_SECTIONS {
        let bg_color = colors.get(i % colors.len()).unwrap_or(&"#f9f");
        let text_color = if is_dark_color(bg_color) {
            "#fff"
        } else {
            "#333"
        };
        let line_color = darken_color(bg_color, 0.2);

        css.push_str(&format!(
            r#"
.section-{i} {{
  fill: {bg};
}}

.section-{i} text {{
  fill: {text_color};
}}

.node-section-{i} {{
  fill: {bg};
  stroke: {stroke};
  stroke-width: 1px;
}}

.node-line-{i} {{
  stroke: {line};
  stroke-width: 3px;
}}
"#,
            i = i,
            bg = bg_color,
            text_color = text_color,
            stroke = theme.primary_border_color,
            line = line_color,
        ));
    }

    css
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_empty_timeline() {
        let db = TimelineDb::new();
        let config = RenderConfig::default();
        let result = render_timeline(&db, &config);
        assert!(result.is_ok());
        let svg = result.unwrap();
        assert!(svg.contains("<svg"));
    }

    #[test]
    fn test_render_timeline_with_title() {
        let mut db = TimelineDb::new();
        db.set_title("Test Timeline");
        let config = RenderConfig::default();
        let result = render_timeline(&db, &config);
        assert!(result.is_ok());
        let svg = result.unwrap();
        assert!(svg.contains("Test Timeline"));
    }

    #[test]
    fn test_render_simple_timeline() {
        let mut db = TimelineDb::new();
        db.set_title("History of Social Media");
        db.add_task("2002: LinkedIn", &[]);
        db.add_task("2004: Facebook: Google", &[]);

        let config = RenderConfig::default();
        let result = render_timeline(&db, &config);
        assert!(result.is_ok());
        let svg = result.unwrap();
        assert!(svg.contains("2002"));
        assert!(svg.contains("LinkedIn"));
    }

    #[test]
    fn test_render_timeline_with_sections() {
        let mut db = TimelineDb::new();
        db.set_title("Industrial Revolution");
        db.add_section("17th-20th century");
        db.add_task("Industry 1.0: Steam power", &[]);
        db.add_section("21st century");
        db.add_task("Industry 4.0: IoT", &[]);

        let config = RenderConfig::default();
        let result = render_timeline(&db, &config);
        assert!(result.is_ok());
        let svg = result.unwrap();
        // Text may be wrapped across tspans, so check for parts
        assert!(svg.contains("17th-20th") || svg.contains("century"));
        assert!(svg.contains("21st century") || svg.contains("21st"));
    }

    #[test]
    fn test_render_timeline_with_dark_theme() {
        use crate::render::svg::Theme;

        let mut db = TimelineDb::new();
        db.set_title("Dark Theme Timeline");
        db.add_task("2020: Event 1", &[]);
        db.add_task("2021: Event 2", &[]);

        let config = RenderConfig {
            theme: Theme::dark(),
            ..RenderConfig::default()
        };
        let result = render_timeline(&db, &config);
        assert!(result.is_ok());
        let svg = result.unwrap();
        // Dark theme should have dark text color
        assert!(svg.contains("#ccc") || svg.contains("ccc"));
    }

    #[test]
    fn test_render_timeline_with_forest_theme() {
        use crate::render::svg::Theme;

        let mut db = TimelineDb::new();
        db.set_title("Forest Theme Timeline");
        db.add_section("Nature");
        db.add_task("Spring: Bloom", &[]);

        let config = RenderConfig {
            theme: Theme::forest(),
            ..RenderConfig::default()
        };
        let result = render_timeline(&db, &config);
        assert!(result.is_ok());
        let svg = result.unwrap();
        // Forest theme uses green colors
        assert!(svg.contains("cde498") || svg.contains("#cde498"));
    }

    #[test]
    fn test_is_dark_color() {
        assert!(super::is_dark_color("#000000"));
        assert!(super::is_dark_color("#333333"));
        assert!(!super::is_dark_color("#ffffff"));
        assert!(!super::is_dark_color("#f9f"));
        assert!(!super::is_dark_color("#ECECFF"));
    }

    #[test]
    fn test_darken_color() {
        let darkened = super::darken_color("#ffffff", 0.2);
        assert_eq!(darkened, "#cccccc");

        let darkened = super::darken_color("#ff0000", 0.5);
        assert_eq!(darkened, "#7f0000");
    }
}
