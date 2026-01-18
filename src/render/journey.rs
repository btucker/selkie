//! Journey diagram renderer
//!
//! Renders user journey diagrams showing tasks, actors, and satisfaction scores.
//! Based on the mermaid.js reference implementation.

use crate::diagrams::journey::JourneyDb;
use crate::error::Result;
use crate::render::svg::{Attrs, RenderConfig, SvgDocument, SvgElement};

// Layout configuration (matching mermaid.js defaults from config.schema.yaml)
/// Margin on the left for actor legend
const LEFT_MARGIN: f64 = 150.0;
/// Margin around the diagram
const DIAGRAM_MARGIN_X: f64 = 50.0;
const DIAGRAM_MARGIN_Y: f64 = 10.0;
/// Width of each task box
const WIDTH: f64 = 150.0;
/// Height of each task/section box
const HEIGHT: f64 = 50.0;
/// Height reserved for the title
const TITLE_HEIGHT: f64 = 50.0;
/// Task vertical position factor
const TASK_VERTICAL_OFFSET: f64 = 100.0;
/// Face vertical base position
const FACE_BASE_Y: f64 = 300.0;
/// Face score multiplier (how much each score point moves the face)
const FACE_SCORE_MULTIPLIER: f64 = 30.0;
/// Face radius
const FACE_RADIUS: f64 = 15.0;
/// Task font size (matching mermaid.js default from config.schema.yaml)
const TASK_FONT_SIZE: f64 = 14.0;

// Note: Actor colors, section fill colors, and text colors are now
// derived from the theme in RenderConfig. See theme.journey_section_fills,
// theme.journey_actor_colors, and theme.journey_text_color.

/// Render a journey diagram to SVG
pub fn render_journey(db: &JourneyDb, config: &RenderConfig) -> Result<String> {
    let mut doc = SvgDocument::new();

    let tasks = db.get_tasks();
    let actors = db.get_actors();
    let has_title = !db.title.is_empty();

    // Calculate dimensions (matching mermaid.js width calculation)
    let num_tasks = tasks.len().max(1);
    let left_margin = LEFT_MARGIN;
    // Use full taskMargin spacing for each task, plus margins on both ends
    let task_total_width = (num_tasks as f64) * (WIDTH + DIAGRAM_MARGIN_X);
    let width = left_margin + task_total_width + DIAGRAM_MARGIN_X * 2.0;
    let height = FACE_BASE_Y + 5.0 * FACE_SCORE_MULTIPLIER + DIAGRAM_MARGIN_Y * 2.0 + 50.0;

    doc.set_size(width, height);

    // Add CSS styles with theme support
    if config.embed_css {
        doc.add_style(&generate_journey_css(config));
    }

    // Add arrow marker definition
    doc.add_defs(vec![create_arrow_defs()]);

    // Build actor color map from theme
    let theme_actor_colors = &config.theme.journey_actor_colors;
    let actor_colors: std::collections::HashMap<String, (String, usize)> = actors
        .iter()
        .enumerate()
        .map(|(i, actor)| {
            let color = theme_actor_colors[i % theme_actor_colors.len()].clone();
            (actor.clone(), (color, i))
        })
        .collect();

    // Calculate title offset
    let title_offset = if has_title { TITLE_HEIGHT } else { 0.0 };

    // Render actor legend first (matching mermaid.js render order)
    let legend = render_actor_legend(&actors, &actor_colors, title_offset, config);
    doc.add_node(legend);

    // Render sections and tasks
    let (sections_element, tasks_element) =
        render_sections_and_tasks(db, left_margin, title_offset, &actor_colors, config);
    doc.add_node(sections_element);
    doc.add_node(tasks_element);

    // Render title AFTER tasks (matching mermaid.js z-order where title appears last)
    if has_title {
        let title_element = render_title(&db.title, left_margin, config);
        doc.add_node(title_element);
    }

    // Render activity line (arrow at the bottom)
    let line_y = HEIGHT * 4.0 + title_offset; // One section head + one task + margins
    let activity_line = render_activity_line(left_margin, line_y, task_total_width);
    doc.add_edge_path(activity_line);

    Ok(doc.to_string())
}

/// Create arrow marker definition
fn create_arrow_defs() -> SvgElement {
    SvgElement::Raw {
        content: r#"<marker id="arrowhead" refX="5" refY="2" markerWidth="6" markerHeight="4" orient="auto">
      <path d="M 0,0 V 4 L6,2 Z"/>
    </marker>"#
            .to_string(),
    }
}

/// Render the actor legend on the left side
/// Each actor's circle+text is wrapped in its own group to maintain proper z-order
fn render_actor_legend(
    actors: &[String],
    actor_colors: &std::collections::HashMap<String, (String, usize)>,
    title_offset: f64,
    config: &RenderConfig,
) -> SvgElement {
    let mut children = Vec::new();
    let start_y = 60.0 + title_offset;
    let default_color = config
        .theme
        .journey_actor_colors
        .first()
        .map(|s| s.as_str())
        .unwrap_or("#8FBC8F");

    for (i, actor) in actors.iter().enumerate() {
        let y_pos = start_y + (i as f64) * 25.0;

        // Get actor color
        let (color, pos) = actor_colors
            .get(actor)
            .map(|(c, p)| (c.as_str(), *p))
            .unwrap_or((default_color, 0));

        // Draw colored circle
        let circle = SvgElement::Circle {
            cx: 20.0,
            cy: y_pos,
            r: 7.0,
            attrs: Attrs::new()
                .with_class(&format!("actor-{}", pos))
                .with_fill(color)
                .with_stroke("#000"),
        };

        // Draw actor name with legend class (for CSS targeting)
        let text = SvgElement::Text {
            x: 40.0,
            y: y_pos + 7.0,
            content: actor.clone(),
            attrs: Attrs::new()
                .with_class("legend")
                .with_fill(&config.theme.journey_text_color)
                .with_attr("text-anchor", "start"),
        };

        // Wrap each actor in its own group so text comes after its own circle
        // This prevents z-order detection issues where text appears before next actor's circle
        let actor_group = SvgElement::Group {
            children: vec![circle, text],
            attrs: Attrs::new().with_class(&format!("actor-legend-{}", pos)),
        };
        children.push(actor_group);
    }

    SvgElement::Group {
        children,
        attrs: Attrs::new().with_class("actor-legend"),
    }
}

/// Render the diagram title (matching mermaid.js with font-size: 4ex)
fn render_title(title: &str, left_margin: f64, config: &RenderConfig) -> SvgElement {
    SvgElement::Text {
        x: left_margin,
        y: 25.0,
        content: title.to_string(),
        attrs: Attrs::new()
            .with_class("journey-title")
            .with_fill(&config.theme.primary_text_color)
            .with_attr("font-size", "4ex")
            .with_attr("font-weight", "bold"),
    }
}

/// Render sections and tasks
fn render_sections_and_tasks(
    db: &JourneyDb,
    left_margin: f64,
    title_offset: f64,
    actor_colors: &std::collections::HashMap<String, (String, usize)>,
    config: &RenderConfig,
) -> (SvgElement, SvgElement) {
    let tasks = db.get_tasks();
    let sections = db.get_sections();
    let mut section_elements = Vec::new();
    let mut task_elements = Vec::new();

    let section_y = 50.0 + title_offset;
    let task_y = TASK_VERTICAL_OFFSET + title_offset;
    // Use section_fills for inline fill attributes (dark colors matching mermaid.js)
    let section_fills = &config.theme.journey_section_fills;

    // If there are no tasks but there are sections, render the sections
    if tasks.is_empty() {
        for (section_idx, section_name) in sections.iter().enumerate() {
            let section_x = (section_idx as f64) * (WIDTH + DIAGRAM_MARGIN_X) + left_margin;

            let section = render_section(
                section_name,
                section_x,
                section_y,
                WIDTH,
                section_idx,
                section_fills,
            );
            section_elements.push(section);
        }
    } else {
        let mut last_section = String::new();
        let mut section_number: usize = 0;

        for (i, task) in tasks.iter().enumerate() {
            // Check if we're entering a new section
            if task.section != last_section {
                // Count how many consecutive tasks share this section
                let task_count = tasks
                    .iter()
                    .skip(i)
                    .take_while(|t| t.section == task.section)
                    .count();

                // Render section header
                let section_x = (i as f64) * (WIDTH + DIAGRAM_MARGIN_X) + left_margin;
                // Section width covers all nested tasks
                let section_width = (task_count as f64) * WIDTH
                    + ((task_count.saturating_sub(1)) as f64) * DIAGRAM_MARGIN_X;

                let section = render_section(
                    &task.section,
                    section_x,
                    section_y,
                    section_width,
                    section_number,
                    section_fills,
                );
                section_elements.push(section);

                last_section = task.section.clone();
                section_number += 1;
            }

            // Render task
            let task_x = (i as f64) * (WIDTH + DIAGRAM_MARGIN_X) + left_margin;
            let section_num = (section_number - 1) % section_fills.len();

            let task_elem = render_task(task, task_x, task_y, section_num, actor_colors, i, config);
            task_elements.push(task_elem);
        }
    }

    (
        SvgElement::Group {
            children: section_elements,
            attrs: Attrs::new().with_class("journey-sections"),
        },
        SvgElement::Group {
            children: task_elements,
            attrs: Attrs::new().with_class("journey-tasks"),
        },
    )
}

/// Render a section header
fn render_section(
    text: &str,
    x: f64,
    y: f64,
    width: f64,
    section_num: usize,
    section_fills: &[String],
) -> SvgElement {
    let mut children = Vec::new();
    let section_idx = section_num % section_fills.len();
    let fill = &section_fills[section_idx];

    // Section background rectangle
    let rect = SvgElement::Rect {
        x,
        y,
        width,
        height: HEIGHT,
        rx: Some(3.0),
        ry: Some(3.0),
        attrs: Attrs::new()
            .with_class(&format!("journey-section section-type-{}", section_idx))
            .with_fill(fill),
    };
    children.push(rect);

    // Section label - centered text with label class (matching mermaid.js)
    // Text color comes from CSS .label { fill: #333; } like the reference
    let label = SvgElement::Text {
        x: x + width / 2.0,
        y: y + HEIGHT / 2.0 + 5.0,
        content: text.to_string(),
        attrs: Attrs::new()
            .with_class("label")
            .with_attr("text-anchor", "middle")
            .with_attr("dominant-baseline", "middle"),
    };
    children.push(label);

    SvgElement::Group {
        children,
        attrs: Attrs::new(),
    }
}

/// Render a task with its face and actor indicators
fn render_task(
    task: &crate::diagrams::journey::JourneyTask,
    x: f64,
    y: f64,
    section_num: usize,
    actor_colors: &std::collections::HashMap<String, (String, usize)>,
    task_index: usize,
    config: &RenderConfig,
) -> SvgElement {
    let mut children = Vec::new();
    let section_fills = &config.theme.journey_section_fills;
    let fill = &section_fills[section_num % section_fills.len()];

    // Task vertical line (dashed) - from task to face area
    let center_x = x + WIDTH / 2.0;
    let max_height = FACE_BASE_Y + 5.0 * FACE_SCORE_MULTIPLIER;
    let line = SvgElement::Line {
        x1: center_x,
        y1: y,
        x2: center_x,
        y2: max_height,
        attrs: Attrs::new()
            .with_class("task-line")
            .with_stroke("#666")
            .with_stroke_width(1.0)
            .with_stroke_dasharray("4 2")
            .with_id(&format!("task{}", task_index)),
    };
    children.push(line);

    // Face element based on score - positioned at 300 + (5 - score) * 30
    let face_y = FACE_BASE_Y + ((5 - task.score) as f64) * FACE_SCORE_MULTIPLIER;
    let face = render_face(
        center_x,
        face_y,
        task.score,
        &config.theme.journey_face_color,
    );
    children.push(face);

    // Task background rectangle
    let rect = SvgElement::Rect {
        x,
        y,
        width: WIDTH,
        height: HEIGHT,
        rx: Some(3.0),
        ry: Some(3.0),
        attrs: Attrs::new()
            .with_class(&format!("task task-type-{}", section_num))
            .with_fill(fill),
    };
    children.push(rect);

    // Actor circles on the task
    let mut actor_x = x + 14.0;
    for person in &task.people {
        if let Some((actor_color, pos)) = actor_colors.get(person) {
            let circle = SvgElement::Circle {
                cx: actor_x,
                cy: y,
                r: 7.0,
                attrs: Attrs::new()
                    .with_class(&format!("actor-{}", pos))
                    .with_fill(actor_color)
                    .with_stroke("#000"),
            };
            // Add title element for hover text
            let circle_with_title = SvgElement::Group {
                children: vec![
                    circle,
                    SvgElement::Raw {
                        content: format!("<title>{}</title>", escape_xml(person)),
                    },
                ],
                attrs: Attrs::new(),
            };
            children.push(circle_with_title);
            actor_x += 10.0;
        }
    }

    // Task label using class="label" for CSS-based text color
    let task_label = render_task_label(&task.task, x, y, WIDTH, HEIGHT);
    children.push(task_label);

    SvgElement::Group {
        children,
        attrs: Attrs::new()
            .with_class(&format!("task-group task-{}", task_index))
            .with_id(&format!("task{}", task_index)),
    }
}

/// Render task label with proper text positioning
/// Uses class="label" for CSS-based fill color (matching mermaid.js)
fn render_task_label(content: &str, x: f64, y: f64, width: f64, height: f64) -> SvgElement {
    // Split on <br> tags for multiline support
    let lines: Vec<&str> = content.split("<br>").collect();
    let line_count = lines.len();

    if line_count == 1 {
        // Single line - simple text element with label class for CSS fill
        SvgElement::Text {
            x: x + width / 2.0,
            y: y + height / 2.0 + 5.0,
            content: content.to_string(),
            attrs: Attrs::new()
                .with_class("label")
                .with_attr("text-anchor", "middle")
                .with_attr("dominant-baseline", "middle")
                .with_attr("font-size", &format!("{}px", TASK_FONT_SIZE)),
        }
    } else {
        // Multiple lines using tspan - use label class for CSS fill
        let mut tspans = String::new();
        for (i, line) in lines.iter().enumerate() {
            let dy =
                (i as f64) * TASK_FONT_SIZE - (TASK_FONT_SIZE * ((line_count - 1) as f64)) / 2.0;
            tspans.push_str(&format!(
                r#"<tspan x="{}" dy="{}">{}</tspan>"#,
                x + width / 2.0,
                dy,
                escape_xml(line)
            ));
        }

        SvgElement::Raw {
            content: format!(
                r#"<text x="{}" y="{}" text-anchor="middle" dominant-baseline="central" alignment-baseline="central" class="label" font-size="{}px">{}</text>"#,
                x + width / 2.0,
                y + height / 2.0,
                TASK_FONT_SIZE,
                tspans
            ),
        }
    }
}

/// Render a face emoji based on score
fn render_face(cx: f64, cy: f64, score: i32, face_color: &str) -> SvgElement {
    let mut children = Vec::new();

    // Face circle
    let face_circle = SvgElement::Circle {
        cx,
        cy,
        r: FACE_RADIUS,
        attrs: Attrs::new()
            .with_class("face")
            .with_fill(face_color)
            .with_stroke_width(2.0)
            .with_attr("overflow", "visible"),
    };
    children.push(face_circle);

    // Left eye
    let left_eye = SvgElement::Circle {
        cx: cx - FACE_RADIUS / 3.0,
        cy: cy - FACE_RADIUS / 3.0,
        r: 1.5,
        attrs: Attrs::new()
            .with_fill("#666")
            .with_stroke("#666")
            .with_stroke_width(2.0),
    };
    children.push(left_eye);

    // Right eye
    let right_eye = SvgElement::Circle {
        cx: cx + FACE_RADIUS / 3.0,
        cy: cy - FACE_RADIUS / 3.0,
        r: 1.5,
        attrs: Attrs::new()
            .with_fill("#666")
            .with_stroke("#666")
            .with_stroke_width(2.0),
    };
    children.push(right_eye);

    // Mouth based on score
    // Using d3.arc equivalent: innerRadius/outerRadius with start/end angles
    // Note: Reference mermaid.js doesn't set explicit stroke-width on mouth paths
    // (they use CSS styling), so we don't set stroke-width on arc paths here
    let mouth = if score > 3 {
        // Happy face - smile arc (startAngle: PI/2, endAngle: 3*PI/2)
        // This creates a downward arc (smile)
        let inner_r = FACE_RADIUS / 2.0;
        let outer_r = FACE_RADIUS / 2.2;
        // SVG arc approximation for smile
        let path = format!(
            "M {},{} A {},{} 0 0,0 {},{}",
            cx - inner_r,
            cy + 2.0,
            inner_r,
            outer_r,
            cx + inner_r,
            cy + 2.0
        );
        SvgElement::Path {
            d: path,
            attrs: Attrs::new()
                .with_class("mouth")
                .with_stroke("#666")
                .with_fill("none"),
        }
    } else if score < 3 {
        // Sad face - frown arc (startAngle: 3*PI/2, endAngle: 5*PI/2)
        let inner_r = FACE_RADIUS / 2.0;
        let outer_r = FACE_RADIUS / 2.2;
        // SVG arc approximation for frown
        let path = format!(
            "M {},{} A {},{} 0 0,1 {},{}",
            cx - inner_r,
            cy + 7.0,
            inner_r,
            outer_r,
            cx + inner_r,
            cy + 7.0
        );
        SvgElement::Path {
            d: path,
            attrs: Attrs::new()
                .with_class("mouth")
                .with_stroke("#666")
                .with_fill("none"),
        }
    } else {
        // Neutral face - straight line (reference uses stroke-width: 1px)
        SvgElement::Line {
            x1: cx - 5.0,
            y1: cy + 7.0,
            x2: cx + 5.0,
            y2: cy + 7.0,
            attrs: Attrs::new()
                .with_class("mouth")
                .with_stroke("#666")
                .with_stroke_width(1.0),
        }
    };
    children.push(mouth);

    SvgElement::Group {
        children,
        attrs: Attrs::new().with_class("face-group"),
    }
}

/// Render the activity line with arrow
fn render_activity_line(left_margin: f64, y: f64, task_width: f64) -> SvgElement {
    let x1 = left_margin;
    let x2 = left_margin + task_width - 4.0; // Subtract stroke width for arrow

    SvgElement::Line {
        x1,
        y1: y,
        x2,
        y2: y,
        attrs: Attrs::new()
            .with_class("activity-line")
            .with_stroke("black")
            .with_stroke_width(4.0)
            .with_attr("marker-end", "url(#arrowhead)"),
    }
}

/// Generate CSS for journey diagrams with theme support
fn generate_journey_css(config: &RenderConfig) -> String {
    let theme = &config.theme;

    let mut section_css = String::new();

    // Generate section type CSS styles using fillType colors (light theme colors)
    // Note: These get overridden by inline fill attributes, but we match mermaid.js CSS
    for (i, fill) in theme.journey_fill_types.iter().enumerate() {
        section_css.push_str(&format!(
            r#"
.section-type-{i} {{
  fill: {fill};
}}
.task-type-{i} {{
  fill: {fill};
}}
"#,
            i = i,
            fill = fill
        ));
    }

    // Generate actor styles from theme actor colors
    let mut actor_css = String::new();
    for (i, color) in theme.journey_actor_colors.iter().enumerate() {
        actor_css.push_str(&format!(
            r#"
.actor-{i} {{
  fill: {color};
}}
"#,
            i = i,
            color = color
        ));
    }

    format!(
        r#"
.journey-title {{
  font-family: {font_family};
  fill: {title_color};
}}
.journey-section text {{
  font-family: {font_family};
}}
.task {{
  cursor: pointer;
  font-family: {font_family};
}}
.task-line {{
  stroke: #666;
  stroke-width: 1px;
  stroke-dasharray: 4 2;
}}
.face {{
  fill: {face_color};
  stroke: #999;
}}
.mouth {{
  stroke: #666;
}}
.legend {{
  fill: {legend_color};
  font-family: {font_family};
  font-size: 14px;
}}
.label {{
  font-family: {font_family};
  color: #333;
  fill: #333;
}}
.activity-line {{
  fill: none;
}}
{section_css}
{actor_css}
"#,
        font_family = theme.font_family,
        title_color = theme.primary_text_color,
        face_color = theme.journey_face_color,
        legend_color = theme.journey_text_color,
        section_css = section_css,
        actor_css = actor_css
    )
}

/// Escape special XML characters
fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}
