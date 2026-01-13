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
        doc.add_style(&generate_gantt_css(&config.theme));
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

    // Track current section for alternating colors
    let mut current_section = String::new();
    let mut section_start_y = current_y;
    let mut section_index = 0;

    // Render tasks grouped by section
    for task in &tasks {
        // Check if section changed
        if task.section != current_section {
            // Render previous section background
            // Colors are controlled by CSS .section0, .section1, etc. classes
            if !current_section.is_empty() && section_start_y < current_y {
                let color_idx = (section_index - 1) % 4;
                let section_bg = SvgElement::Rect {
                    x: margin,
                    y: section_start_y,
                    width: total_width - margin * 2.0,
                    height: current_y - section_start_y,
                    rx: None,
                    ry: None,
                    attrs: Attrs::new().with_class(&format!("section section{}", color_idx)),
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

            // Determine CSS class based on task flags
            // mermaid.js uses task0-3, done0-3, crit0-3, active0-3 classes
            // We also include "task-bar" for backward compatibility with tests
            let class_suffix = section_index % 4;
            let bar_class = if task.flags.done {
                format!("task task-bar done done{}", class_suffix)
            } else if task.flags.critical {
                format!("task task-bar crit crit{}", class_suffix)
            } else if task.flags.active {
                format!("task task-bar active active{}", class_suffix)
            } else {
                format!("task task-bar task{}", class_suffix)
            };

            let bar_elem = SvgElement::Rect {
                x: bar_x,
                y: task_y,
                width: bar_width,
                height: task_height,
                rx: Some(3.0),
                ry: Some(3.0),
                attrs: Attrs::new().with_class(&bar_class),
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
                    attrs: Attrs::new().with_class("milestone"),
                };
                doc.add_element(milestone);
            }
        }

        current_y += task_height + task_spacing;
    }

    // Render final section background
    if !current_section.is_empty() && section_start_y < current_y {
        let color_idx = (section_index - 1) % 4;
        let section_bg = SvgElement::Rect {
            x: margin,
            y: section_start_y,
            width: total_width - margin * 2.0,
            height: current_y - section_start_y,
            rx: None,
            ry: None,
            attrs: Attrs::new().with_class(&format!("section section{}", color_idx)),
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

    // Background - styled via CSS .timeline-bg class
    children.push(SvgElement::Rect {
        x,
        y,
        width,
        height,
        rx: None,
        ry: None,
        attrs: Attrs::new().with_class("timeline-bg"),
    });

    // Axis line - styled via CSS .axis-line class
    children.push(SvgElement::Line {
        x1: x,
        y1: y + height,
        x2: x + width,
        y2: y + height,
        attrs: Attrs::new().with_class("axis-line"),
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
                attrs: Attrs::new().with_class("tick"),
            });

            // Tick mark on axis
            children.push(SvgElement::Line {
                x1: tick_x,
                y1: y + height - 5.0,
                x2: tick_x,
                y2: y + height,
                attrs: Attrs::new().with_class("tick-mark"),
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

fn generate_gantt_css(theme: &crate::render::svg::Theme) -> String {
    format!(
        r#"
.titleText {{
  text-anchor: middle;
  font-size: 18px;
  fill: {text_color};
  font-family: {font_family};
}}

.section {{
  stroke: none;
  opacity: 0.2;
}}

.section0, .section2 {{
  fill: {section_bkg_color};
}}

.section1, .section3 {{
  fill: {section_bkg_color2};
}}

.sectionTitle {{
  text-anchor: start;
  font-family: {font_family};
  fill: {text_color};
}}

.task {{
  stroke-width: 2;
}}

.task0, .task1, .task2, .task3 {{
  fill: {task_bkg_color};
  stroke: {task_border_color};
}}

.taskText {{
  text-anchor: middle;
  font-family: {font_family};
}}

.taskText0, .taskText1, .taskText2, .taskText3 {{
  fill: {task_text_light_color};
}}

.active0, .active1, .active2, .active3 {{
  fill: {active_task_bkg_color};
  stroke: {active_task_border_color};
}}

.activeText0, .activeText1, .activeText2, .activeText3 {{
  fill: {task_text_dark_color} !important;
}}

.done0, .done1, .done2, .done3 {{
  fill: {done_task_bkg_color};
  stroke: {done_task_border_color};
  stroke-width: 2;
}}

.doneText0, .doneText1, .doneText2, .doneText3 {{
  fill: {task_text_dark_color} !important;
}}

.crit0, .crit1, .crit2, .crit3 {{
  fill: {crit_bkg_color};
  stroke: {crit_border_color};
  stroke-width: 2;
}}

.milestone {{
  fill: {task_bkg_color};
  stroke: {task_border_color};
}}

.grid .tick {{
  stroke: {grid_color};
  opacity: 0.8;
  shape-rendering: crispEdges;
}}

.tick-mark {{
  stroke: {text_color};
  stroke-width: 1;
}}

.grid path {{
  stroke-width: 0;
}}

.timeline-bg {{
  fill: {section_bkg_color2};
  stroke: {grid_color};
  stroke-width: 1;
}}

.axis-line {{
  stroke: {text_color};
  stroke-width: 1;
}}

.axis-label {{
  fill: {text_color};
}}

.today-line {{
  stroke: {today_line_color};
  stroke-width: 2;
}}
"#,
        font_family = theme.font_family,
        text_color = theme.primary_text_color,
        section_bkg_color = theme.section_bkg_color,
        section_bkg_color2 = theme.section_bkg_color2,
        task_bkg_color = theme.task_bkg_color,
        task_border_color = theme.task_border_color,
        task_text_light_color = theme.task_text_light_color,
        task_text_dark_color = theme.task_text_dark_color,
        active_task_bkg_color = theme.active_task_bkg_color,
        active_task_border_color = theme.active_task_border_color,
        done_task_bkg_color = theme.done_task_bkg_color,
        done_task_border_color = theme.done_task_border_color,
        crit_bkg_color = theme.crit_bkg_color,
        crit_border_color = theme.crit_border_color,
        grid_color = theme.grid_color,
        today_line_color = theme.today_line_color,
    )
}
