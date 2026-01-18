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
const NODE_WIDTH: f64 = 150.0; // Reference uses width: 150 for node content (total = 150 + 2*padding = 190)
const TEXT_WRAP_WIDTH: f64 = 150.0; // Reference uses width: 150 for text wrapping
const NODE_PADDING: f64 = 20.0;
const COLUMN_WIDTH: f64 = 200.0; // Reference uses masterX += 200 spacing between task centers
const SECTION_HEIGHT: f64 = 68.0; // ~68px in reference
const TASK_HEIGHT: f64 = 68.0; // ~68px in reference
const EVENT_HEIGHT: f64 = 50.0; // ~45-50px minimum in reference
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
            let height = estimate_node_height(section, TEXT_WRAP_WIDTH);
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
        let height = estimate_node_height(&task.task, TEXT_WRAP_WIDTH);
        max_task_height = max_task_height.max(height);
        _max_event_count = _max_event_count.max(task.events.len());

        // Calculate event line length for this task
        let mut event_line_length: f64 = 0.0;
        for event in &task.events {
            event_line_length += estimate_node_height(event, TEXT_WRAP_WIDTH);
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
    // Reference uses ~150px left margin and ~340px right margin for the arrow
    let total_width = LEFT_MARGIN + (total_columns as f64) * COLUMN_WIDTH + LEFT_MARGIN * 3.0;

    // Calculate depth_y (position of timeline line)
    let section_begin_y = TOP_MARGIN;
    let depth_y = if has_sections {
        max_section_height + max_task_height + 150.0
    } else {
        max_task_height + 100.0
    };

    // Total height includes title, sections, tasks, events, and timeline line
    // Add extra margin for bottom spacing
    let total_height = depth_y + max_event_line_length + 250.0;

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
fn estimate_node_height(text: &str, max_width: f64) -> f64 {
    // Split on <br> tags and whitespace to simulate wrap_text
    let text = text
        .replace("<br>", "\n")
        .replace("<br/>", "\n")
        .replace("<br />", "\n");
    let words: Vec<&str> = text.split_whitespace().collect();

    if words.is_empty() {
        return EVENT_HEIGHT;
    }

    // Count lines using same algorithm as wrap_text
    let mut line_count = 0;
    let mut current_line = String::new();

    for word in words {
        if current_line.is_empty() {
            current_line = word.to_string();
        } else {
            let potential_line = format!("{} {}", current_line, word);
            let estimated_width = estimate_text_width(&potential_line);

            if estimated_width <= max_width {
                current_line = potential_line;
            } else {
                line_count += 1;
                current_line = word.to_string();
            }
        }
    }
    if !current_line.is_empty() {
        line_count += 1;
    }

    // Height formula matches reference: bbox.height + fontSize * 1.1 * 0.5 + padding
    // For multi-line text: lines * lineHeight + extra padding for text y-offset (10px)
    let line_height = FONT_SIZE * 1.1;
    let height = line_count as f64 * line_height + NODE_PADDING + 10.0;
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
            section_number as i32,
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
            section_number as i32,
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
    section_num: i32,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
) {
    let section_class = format!("timeline-node section-{}", section_num);

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
        attrs: Attrs::new().with_class(&format!("node-bkg node-section-{}", section_num)),
    });

    // Bottom line
    group_children.push(SvgElement::Line {
        x1: 0.0,
        y1: height,
        x2: width,
        y2: height,
        attrs: Attrs::new().with_class(&format!("node-line-{}", section_num)),
    });

    // Text - positioned near top to match mermaid.js (translate(_, 10))
    let text_y = 10.0; // Match mermaid's translate(_, 10) for text group position
    let text_elem = wrap_text(text, width / 2.0, text_y, width - NODE_PADDING * 2.0);
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
    section_color: i32,
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
/// Uses section indices starting from -1 to match mermaid.js behavior
fn render_tasks_multicolor(
    doc: &mut SvgDocument,
    tasks: &[&TimelineTask],
    start_x: f64,
    start_y: f64,
    layout: &TimelineLayout,
) {
    let mut master_x = start_x;

    for (idx, task) in tasks.iter().enumerate() {
        // Start from -1 to match mermaid.js (section--1, section-0, section-1, ...)
        let section_idx = idx as i32 - 1;
        render_task_node(doc, task, section_idx, master_x, start_y, layout);
        master_x += COLUMN_WIDTH;
    }
}

/// Render a single task node with its events
fn render_task_node(
    doc: &mut SvgDocument,
    task: &TimelineTask,
    section_color: i32,
    x: f64,
    y: f64,
    layout: &TimelineLayout,
) {
    // Format section class (negative indices like -1 become "section--1")
    let node_class = format!("timeline-node section-{}", section_color);
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
        attrs: Attrs::new().with_class(&format!("node-bkg node-section-{}", section_color)),
    });

    // Bottom line
    task_children.push(SvgElement::Line {
        x1: 0.0,
        y1: height,
        x2: width,
        y2: height,
        attrs: Attrs::new().with_class(&format!("node-line-{}", section_color)),
    });

    // Text - positioned near top to match mermaid.js (translate(_, 10))
    // Reference uses y=10 from node top, not centered
    let text_y = 10.0; // Match mermaid's translate(_, 10) for text group position
    let text_elem = wrap_text(&task.task, width / 2.0, text_y, TEXT_WRAP_WIDTH);
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
    section_color: i32,
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
        let event_height = estimate_node_height(event, TEXT_WRAP_WIDTH);
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
    section_color: i32,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
) {
    let node_class = format!("timeline-node section-{}", section_color);

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
        attrs: Attrs::new().with_class(&format!("node-bkg node-section-{}", section_color)),
    });

    // Bottom line
    event_children.push(SvgElement::Line {
        x1: 0.0,
        y1: height,
        x2: width,
        y2: height,
        attrs: Attrs::new().with_class(&format!("node-line-{}", section_color)),
    });

    // Text - positioned near top to match mermaid.js (translate(_, 10))
    let text_y = 10.0; // Match mermaid's translate(_, 10) for text group position
    let text_elem = wrap_text(text, width / 2.0, text_y, TEXT_WRAP_WIDTH);
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

    // Build lines using weighted width estimation
    // Instead of character count, estimate actual pixel width
    let mut lines: Vec<String> = Vec::new();
    let mut current_line = String::new();

    for word in words {
        if current_line.is_empty() {
            current_line = word.to_string();
        } else {
            // Estimate width of current line + space + new word
            let potential_line = format!("{} {}", current_line, word);
            let estimated_width = estimate_text_width(&potential_line);

            if estimated_width <= max_width {
                current_line = potential_line;
            } else {
                lines.push(current_line);
                current_line = word.to_string();
            }
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
        // Reference uses y=padding/2 (10) with dy="1em" to push first line down
        // Don't try to vertically center - just flow text downward from near top

        let mut tspans = Vec::new();
        for (i, line) in lines.iter().enumerate() {
            tspans.push(SvgElement::Raw {
                content: format!(
                    r#"<tspan x="{}" dy="{}">{}</tspan>"#,
                    cx,
                    if i == 0 {
                        "1em".to_string() // Push first line down by 1em
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
                    r#"<text x="{}" y="{}" text-anchor="middle" dominant-baseline="middle" alignment-baseline="middle">{}</text>"#,
                    cx,
                    cy, // Use cy directly (10), don't try to center vertically
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

/// Estimate text width in pixels using weighted character widths
/// This approximates browser text rendering for proportional fonts
fn estimate_text_width(text: &str) -> f64 {
    let mut total_width = 0.0;

    for c in text.chars() {
        // Approximate relative widths for proportional fonts like Trebuchet MS
        let char_width = match c {
            // Narrow characters (~0.3 of average)
            'i' | 'l' | 'I' | '!' | '|' | '\'' | '.' | ',' | ':' | ';' | 'j' | 'f' | 't' | 'r' => {
                FONT_SIZE * 0.35
            }
            // Wide characters (~1.5 of average)
            'M' | 'W' | 'm' | 'w' | '@' => FONT_SIZE * 0.9,
            // Semi-wide characters (~1.2 of average)
            'N' | 'O' | 'Q' | 'G' | 'D' | 'H' | 'U' | 'A' | 'V' | 'X' | 'Y' | 'Z' | 'K' | 'R'
            | 'B' | 'P' => FONT_SIZE * 0.65,
            // Space
            ' ' => FONT_SIZE * 0.35,
            // Regular lowercase characters (~0.5 of em)
            'a'..='z' => FONT_SIZE * 0.5,
            // Regular uppercase characters (~0.6 of em)
            'A'..='Z' => FONT_SIZE * 0.6,
            // Numbers (~0.5 of em)
            '0'..='9' => FONT_SIZE * 0.55,
            // Default for other characters
            _ => FONT_SIZE * 0.5,
        };
        total_width += char_width;
    }

    total_width
}

/// Generate timeline-specific CSS using mermaid.js-compatible HSL colors
fn generate_timeline_css(theme: &crate::render::svg::Theme) -> String {
    // Mermaid.js timeline uses specific HSL colors for each section
    // The hue values and lightness follow a specific pattern from the reference
    let timeline_colors: Vec<(f64, f64, f64)> = vec![
        // (hue, saturation, lightness)
        (60.0, 100.0, 73.53), // section-0: yellow (slightly different lightness)
        (80.0, 100.0, 76.27), // section-1: yellow-green
        (270.0, 100.0, 76.27), // section-2: purple
        (300.0, 100.0, 76.27), // section-3: magenta
        (330.0, 100.0, 76.27), // section-4: pink
        (0.0, 100.0, 76.27),  // section-5: red
        (30.0, 100.0, 76.27), // section-6: orange
        (90.0, 100.0, 76.27), // section-7: green
        (150.0, 100.0, 76.27), // section-8: cyan-green
        (180.0, 100.0, 76.27), // section-9: cyan
        (210.0, 100.0, 76.27), // section-10: light blue
        (240.0, 100.0, 76.27), // section-11: blue (wraps to -1 pattern)
    ];

    // Section -1 uses blue/violet (hsl(240, 100%, 76.27%))
    // This is used for sectionless timelines where each task is its own section
    let section_minus1 = (240.0, 100.0, 76.27);

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

.section--1 {{
  fill: hsl({h_m1}, {s_m1}%, {l_m1}%);
}}

.section--1 text {{
  fill: #ffffff;
}}

.node-section--1 {{
  fill: hsl({h_m1}, {s_m1}%, {l_m1}%);
  stroke: {stroke};
  stroke-width: 1px;
}}

.node-line--1 {{
  stroke: hsl({inv_h_m1}, {s_m1}%, {inv_l_m1}%);
  stroke-width: 3px;
}}
"#,
        text_color = theme.primary_text_color,
        font_family = theme.font_family,
        font_size = FONT_SIZE as i32,
        line_color = theme.line_color,
        h_m1 = section_minus1.0,
        s_m1 = section_minus1.1,
        l_m1 = section_minus1.2,
        stroke = theme.primary_border_color,
        inv_h_m1 = (section_minus1.0 + 180.0) % 360.0,
        inv_l_m1 = (section_minus1.2 + 10.0_f64).min(90.0),
    );

    // Generate section-specific styles using HSL colors
    for i in 0..MAX_SECTIONS {
        let (h, s, l) = if i < timeline_colors.len() {
            timeline_colors[i]
        } else {
            // Wrap around for sections beyond our defined list
            timeline_colors[i % timeline_colors.len()]
        };

        // Determine text color based on lightness
        let text_color = if l > 60.0 { "black" } else { "#ffffff" };

        // Calculate line/border color (inverted hue, higher lightness)
        let inv_h = (h + 180.0) % 360.0;
        let inv_l = (l + 10.0).min(90.0);

        css.push_str(&format!(
            r#"
.section-{i} {{
  fill: hsl({h}, {s}%, {l}%);
}}

.section-{i} text {{
  fill: {text_color};
}}

.node-section-{i} {{
  fill: hsl({h}, {s}%, {l}%);
  stroke: {stroke};
  stroke-width: 1px;
}}

.node-line-{i} {{
  stroke: hsl({inv_h}, {s}%, {inv_l}%);
  stroke-width: 3px;
}}
"#,
            i = i,
            h = h,
            s = s,
            l = l,
            text_color = text_color,
            stroke = theme.primary_border_color,
            inv_h = inv_h,
            inv_l = inv_l,
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
}
