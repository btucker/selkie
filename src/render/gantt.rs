//! Gantt diagram renderer

use crate::diagrams::gantt::GanttDb;
use crate::error::Result;
use crate::render::svg::{Attrs, RenderConfig, SvgDocument, SvgElement};

/// Render a Gantt diagram to SVG
pub fn render_gantt(db: &mut GanttDb, config: &RenderConfig) -> Result<String> {
    let mut doc = SvgDocument::new();

    // Layout constants
    let margin = 50.0;
    let section_height = 25.0;
    let task_height = 20.0;
    let task_spacing = 5.0;
    let timeline_height = 40.0;
    let left_label_width = 100.0; // Only for section labels, task labels are inside bars
    let day_width = 30.0;

    let tasks = db.get_tasks();

    if tasks.is_empty() {
        // Empty diagram
        doc.set_size(400.0, 200.0);
        if !db.title.is_empty() {
            let title_elem = SvgElement::Text {
                x: 200.0,
                y: 30.0,
                content: db.title.clone(),
                attrs: Attrs::new()
                    .with_attr("text-anchor", "middle")
                    .with_class("gantt-title")
                    .with_attr("font-size", "20")
                    .with_attr("font-weight", "bold"),
            };
            doc.add_element(title_elem);
        }
        return Ok(doc.to_string());
    }

    // Find min and max dates
    let mut min_date: Option<chrono::NaiveDateTime> = None;
    let mut max_date: Option<chrono::NaiveDateTime> = None;

    for task in &tasks {
        if let Some(start) = task.start_time {
            min_date = Some(match min_date {
                Some(current) => current.min(start),
                None => start,
            });
        }
        if let Some(end) = task.end_time {
            max_date = Some(match max_date {
                Some(current) => current.max(end),
                None => end,
            });
        }
    }

    // Calculate time range in days
    let days_range = match (min_date, max_date) {
        (Some(min), Some(max)) => {
            let duration = max - min;
            duration.num_days().max(1) as f64
        }
        _ => 30.0, // Default to 30 days if no dates
    };

    // Calculate dimensions
    let title_height = if !db.title.is_empty() { 40.0 } else { 0.0 };
    let chart_width = days_range * day_width;
    let total_width = margin + left_label_width + chart_width + margin;

    // Count sections and tasks
    let sections = db.get_sections();
    let num_sections = sections.len().max(1);
    let chart_height = timeline_height
        + (num_sections as f64) * section_height
        + (tasks.len() as f64) * (task_height + task_spacing);

    let total_height = margin + title_height + chart_height + margin;

    doc.set_size(total_width, total_height);

    // Add theme styles
    if config.embed_css {
        doc.add_style(&config.theme.generate_css());
        doc.add_style(&generate_gantt_css());
    }

    let mut current_y = margin;

    // Render title
    if !db.title.is_empty() {
        let title_elem = SvgElement::Text {
            x: total_width / 2.0,
            y: current_y + 20.0,
            content: db.title.clone(),
            attrs: Attrs::new()
                .with_attr("text-anchor", "middle")
                .with_class("gantt-title")
                .with_attr("font-size", "18")
                .with_attr("font-weight", "bold"),
        };
        doc.add_element(title_elem);
        current_y += title_height;
    }

    let chart_start_x = margin + left_label_width;
    let chart_start_y = current_y;

    // Render timeline axis with vertical grid lines
    let tasks_area_height = chart_height - timeline_height; // Height of tasks area below timeline
    let axis_elem = render_timeline_axis(
        chart_start_x,
        chart_start_y,
        chart_width,
        timeline_height,
        min_date,
        days_range as i64,
        day_width,
        tasks_area_height,
    );
    doc.add_element(axis_elem);
    current_y += timeline_height;

    // Section colors from mermaid.js default theme
    let section_colors = [
        "rgba(102, 102, 255, 0.49)", // section0 - purple
        "rgba(255, 255, 255, 0.2)",  // section1 - white with opacity
        "#fff400",                   // section2 - yellow
        "rgba(255, 255, 255, 0.2)",  // section3 - white with opacity
    ];

    // Track current section for alternating colors
    let mut current_section = String::new();
    let mut section_start_y = current_y;
    let mut section_index = 0;

    // Render tasks grouped by section
    for task in &tasks {
        // Check if section changed
        if task.section != current_section {
            // Render previous section background
            if !current_section.is_empty() && section_start_y < current_y {
                let color_idx = (section_index - 1) % section_colors.len();
                let section_bg = SvgElement::Rect {
                    x: margin,
                    y: section_start_y,
                    width: total_width - margin * 2.0,
                    height: current_y - section_start_y,
                    rx: None,
                    ry: None,
                    attrs: Attrs::new()
                        .with_fill(section_colors[color_idx])
                        .with_class(&format!("section section{}", color_idx)),
                };
                doc.add_element(section_bg);
            }

            current_section = task.section.clone();
            section_start_y = current_y;
            section_index += 1;

            // Render section label
            if !current_section.is_empty() {
                let section_label = SvgElement::Text {
                    x: margin + 10.0,
                    y: current_y + section_height / 2.0 + 4.0,
                    content: current_section.clone(),
                    attrs: Attrs::new()
                        .with_attr("text-anchor", "start")
                        .with_class("section-label")
                        .with_attr("font-size", "12")
                        .with_attr("font-weight", "bold"),
                };
                doc.add_element(section_label);
                current_y += section_height;
            }
        }

        // Render task
        let task_y = current_y + task_spacing / 2.0;

        // Task bar and label (label goes inside the bar)
        if let (Some(start), Some(end), Some(min)) = (task.start_time, task.end_time, min_date) {
            let start_offset = (start - min).num_days() as f64;
            let duration = (end - start).num_days().max(1) as f64;

            let bar_x = chart_start_x + start_offset * day_width;
            let bar_width = duration * day_width;

            // Determine bar color based on flags (mermaid.js default theme colors)
            let (bar_color, bar_stroke, _text_color) = if task.flags.done {
                ("#d3d3d3", "#808080", "#000000") // lightgrey/grey for done
            } else if task.flags.critical {
                ("#ff0000", "#ff8888", "#ffffff") // red for critical
            } else if task.flags.active {
                ("#bfc7ff", "#534fbc", "#000000") // light purple for active
            } else {
                ("#8a90dd", "#534fbc", "#ffffff") // mermaid.js default purple
            };

            let bar_elem = SvgElement::Rect {
                x: bar_x,
                y: task_y,
                width: bar_width,
                height: task_height,
                rx: Some(3.0),
                ry: Some(3.0),
                attrs: Attrs::new()
                    .with_fill(bar_color)
                    .with_stroke(bar_stroke)
                    .with_stroke_width(2.0)
                    .with_class("task-bar"),
            };
            doc.add_element(bar_elem);

            // Task label inside the bar (centered like mermaid.js)
            let task_label = SvgElement::Text {
                x: bar_x + bar_width / 2.0, // Center of bar
                y: task_y + task_height / 2.0 + 4.0,
                content: task.task.clone(),
                attrs: Attrs::new()
                    .with_attr("text-anchor", "middle")
                    .with_class(&format!("taskText taskText{}", section_index % 4))
                    .with_attr("font-size", "11"),
            };
            doc.add_element(task_label);

            // Milestone marker
            if task.flags.milestone {
                let milestone = SvgElement::Polygon {
                    points: vec![
                        crate::layout::Point {
                            x: bar_x,
                            y: task_y + task_height / 2.0,
                        },
                        crate::layout::Point {
                            x: bar_x + 8.0,
                            y: task_y,
                        },
                        crate::layout::Point {
                            x: bar_x + 16.0,
                            y: task_y + task_height / 2.0,
                        },
                        crate::layout::Point {
                            x: bar_x + 8.0,
                            y: task_y + task_height,
                        },
                    ],
                    attrs: Attrs::new()
                        .with_fill("#9370DB")
                        .with_stroke("#333333")
                        .with_stroke_width(1.0)
                        .with_class("milestone"),
                };
                doc.add_element(milestone);
            }
        }

        current_y += task_height + task_spacing;
    }

    // Render final section background
    if !current_section.is_empty() && section_start_y < current_y {
        let color_idx = (section_index - 1) % section_colors.len();
        let section_bg = SvgElement::Rect {
            x: margin,
            y: section_start_y,
            width: total_width - margin * 2.0,
            height: current_y - section_start_y,
            rx: None,
            ry: None,
            attrs: Attrs::new()
                .with_fill(section_colors[color_idx])
                .with_class(&format!("section section{}", color_idx)),
        };
        // Insert at beginning so it's behind tasks
        doc.add_element(section_bg);
    }

    Ok(doc.to_string())
}

/// Render the timeline axis and vertical grid lines
#[allow(clippy::too_many_arguments)]
fn render_timeline_axis(
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    start_date: Option<chrono::NaiveDateTime>,
    days: i64,
    day_width: f64,
    chart_height: f64, // Height of the chart area below the axis
) -> SvgElement {
    let mut children = Vec::new();

    // Background
    children.push(SvgElement::Rect {
        x,
        y,
        width,
        height,
        rx: None,
        ry: None,
        attrs: Attrs::new()
            .with_fill("#f8f8f8")
            .with_stroke("#cccccc")
            .with_stroke_width(1.0)
            .with_class("timeline-bg"),
    });

    // Axis line
    children.push(SvgElement::Line {
        x1: x,
        y1: y + height,
        x2: x + width,
        y2: y + height,
        attrs: Attrs::new().with_stroke("#333333").with_stroke_width(1.0),
    });

    // Grid lines and day markers
    let tick_interval = if days > 30 {
        7
    } else if days > 14 {
        2
    } else {
        1
    };

    if let Some(start) = start_date {
        use chrono::Datelike;

        // Grid group for vertical lines
        let mut grid_children = Vec::new();

        for day in (0..=days).step_by(tick_interval as usize) {
            let tick_x = x + (day as f64) * day_width;

            // Vertical grid line extending through chart area
            grid_children.push(SvgElement::Line {
                x1: tick_x,
                y1: y + height,
                x2: tick_x,
                y2: y + height + chart_height,
                attrs: Attrs::new()
                    .with_stroke("#d3d3d3")
                    .with_stroke_width(1.0)
                    .with_class("tick"),
            });

            // Tick mark on axis
            children.push(SvgElement::Line {
                x1: tick_x,
                y1: y + height - 5.0,
                x2: tick_x,
                y2: y + height,
                attrs: Attrs::new().with_stroke("#333333").with_stroke_width(1.0),
            });

            // Date label (YYYY-MM-DD format like mermaid.js)
            let date = start + chrono::Duration::days(day);
            let label = format!("{:04}-{:02}-{:02}", date.year(), date.month(), date.day());

            children.push(SvgElement::Text {
                x: tick_x,
                y: y + height - 10.0,
                content: label,
                attrs: Attrs::new()
                    .with_attr("text-anchor", "middle")
                    .with_class("axis-label")
                    .with_attr("font-size", "9"),
            });
        }

        // Add grid group
        children.push(SvgElement::Group {
            children: grid_children,
            attrs: Attrs::new().with_class("grid"),
        });
    }

    SvgElement::Group {
        children,
        attrs: Attrs::new().with_class("timeline-axis"),
    }
}

fn generate_gantt_css() -> String {
    r#"
.titleText {
  text-anchor: middle;
  font-size: 18px;
  fill: #333;
  font-family: "trebuchet ms", verdana, arial, sans-serif;
}

.section {
  stroke: none;
  opacity: 0.2;
}

.section0 {
  fill: rgba(102, 102, 255, 0.49);
}

.section1, .section3 {
  fill: white;
  opacity: 0.2;
}

.section2 {
  fill: #fff400;
}

.sectionTitle {
  text-anchor: start;
  font-family: "trebuchet ms", verdana, arial, sans-serif;
  fill: #333;
}

.task {
  stroke-width: 2;
}

.task0, .task1, .task2, .task3 {
  fill: #8a90dd;
  stroke: #534fbc;
}

.taskText {
  text-anchor: middle;
  font-family: "trebuchet ms", verdana, arial, sans-serif;
}

.taskText0, .taskText1, .taskText2, .taskText3 {
  fill: white;
}

.active0, .active1, .active2, .active3 {
  fill: #bfc7ff;
  stroke: #534fbc;
}

.activeText0, .activeText1, .activeText2, .activeText3 {
  fill: black !important;
}

.done0, .done1, .done2, .done3 {
  stroke: grey;
  fill: lightgrey;
  stroke-width: 2;
}

.doneText0, .doneText1, .doneText2, .doneText3 {
  fill: black !important;
}

.crit0, .crit1, .crit2, .crit3 {
  stroke: #ff8888;
  fill: red;
  stroke-width: 2;
}

.milestone {
  transform: rotate(45deg) scale(0.8, 0.8);
}

.grid .tick {
  stroke: lightgrey;
  opacity: 0.8;
  shape-rendering: crispEdges;
}

.grid path {
  stroke-width: 0;
}

.timeline-bg {
  fill: #f8f8f8;
}

.axis-label {
  fill: #666666;
}
"#
    .to_string()
}
