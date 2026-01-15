//! Gantt diagram renderer
//!
//! Renders Gantt charts following Mermaid.js conventions:
//! - Fixed width chart with time scale adapting to date range
//! - Per-task row layout (barHeight + barGap = 24px per row)
//! - Section labels on left side spanning their tasks
//! - D3-style automatic tick intervals (monthly for multi-month ranges)

use crate::diagrams::gantt::GanttDb;
use crate::error::Result;
use crate::render::svg::{Attrs, RenderConfig, SvgDocument, SvgElement};

// Mermaid-compatible layout constants (from config.schema.yaml)
const BAR_HEIGHT: f64 = 20.0;
const BAR_GAP: f64 = 4.0;
const TOP_PADDING: f64 = 50.0;
const LEFT_PADDING: f64 = 75.0;
const RIGHT_PADDING: f64 = 75.0;
const TITLE_TOP_MARGIN: f64 = 25.0;
const GRID_LINE_START_PADDING: f64 = 35.0;
const FONT_SIZE: f64 = 11.0;
const TARGET_WIDTH: f64 = 784.0; // Default mermaid width

/// Render a Gantt diagram to SVG
pub fn render_gantt(db: &mut GanttDb, config: &RenderConfig) -> Result<String> {
    let mut doc = SvgDocument::new();

    let tasks = db.get_tasks();

    if tasks.is_empty() {
        doc.set_size(400.0, 200.0);
        if !db.title.is_empty() {
            let title_elem = SvgElement::Text {
                x: 200.0,
                y: 30.0,
                content: db.title.clone(),
                attrs: Attrs::new()
                    .with_attr("text-anchor", "middle")
                    .with_class("titleText")
                    .with_attr("font-size", "18")
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
        _ => 30.0,
    };

    // Fixed width, variable height (like mermaid)
    let total_width = TARGET_WIDTH;
    let chart_width = total_width - LEFT_PADDING - RIGHT_PADDING;

    // Calculate height: 2*topPadding + numTasks * (barHeight + barGap)
    let row_height = BAR_HEIGHT + BAR_GAP;
    let total_height = 2.0 * TOP_PADDING + (tasks.len() as f64) * row_height;

    doc.set_size(total_width, total_height);

    // Add theme styles
    if config.embed_css {
        doc.add_style(&config.theme.generate_css());
        doc.add_style(&generate_gantt_css(&config.theme));
    }

    // Render title
    if !db.title.is_empty() {
        let title_elem = SvgElement::Text {
            x: total_width / 2.0,
            y: TITLE_TOP_MARGIN,
            content: db.title.clone(),
            attrs: Attrs::new()
                .with_attr("text-anchor", "middle")
                .with_class("titleText"),
        };
        doc.add_element(title_elem);
    }

    // Collect tasks grouped by section for rendering
    let sections = collect_sections(&tasks);

    // Render grid and axis FIRST (behind everything else)
    let grid_height = total_height - TOP_PADDING - GRID_LINE_START_PADDING;
    render_grid_and_axis(
        &mut doc,
        min_date,
        days_range,
        LEFT_PADDING,
        chart_width,
        total_height,
        grid_height,
    );

    // Render section backgrounds (behind tasks but over grid)
    render_section_backgrounds(
        &mut doc,
        &tasks,
        &sections,
        total_width,
        TOP_PADDING,
        row_height,
    );

    // Render task bars (on top of backgrounds and grid)
    render_task_bars(
        &mut doc,
        &tasks,
        &sections,
        min_date,
        days_range,
        LEFT_PADDING,
        TOP_PADDING,
        chart_width,
        row_height,
    );

    // Render section labels on left side
    render_section_labels(&mut doc, &tasks, &sections, TOP_PADDING, row_height);

    // Render today line
    render_today_line(
        &mut doc,
        min_date,
        days_range,
        LEFT_PADDING,
        chart_width,
        total_height,
    );

    Ok(doc.to_string())
}

/// Collect unique sections in order with their task ranges
fn collect_sections(tasks: &[crate::diagrams::gantt::Task]) -> Vec<(String, usize, usize)> {
    let mut sections: Vec<(String, usize, usize)> = Vec::new();
    let mut current_section = String::new();
    let mut section_start = 0;

    for (i, task) in tasks.iter().enumerate() {
        if task.section != current_section {
            if !current_section.is_empty() {
                sections.push((current_section.clone(), section_start, i));
            }
            current_section = task.section.clone();
            section_start = i;
        }
    }
    // Add final section
    if !current_section.is_empty() {
        sections.push((current_section, section_start, tasks.len()));
    }

    sections
}

/// Render alternating section background rows
fn render_section_backgrounds(
    doc: &mut SvgDocument,
    _tasks: &[crate::diagrams::gantt::Task],
    sections: &[(String, usize, usize)],
    total_width: f64,
    top_padding: f64,
    row_height: f64,
) {
    // In mermaid, each task row gets its own background rect
    // The section class alternates based on section index
    for (section_idx, (_section_name, start_idx, end_idx)) in sections.iter().enumerate() {
        for task_idx in *start_idx..*end_idx {
            let y = task_idx as f64 * row_height + top_padding - 2.0;
            let section_bg = SvgElement::Rect {
                x: 0.0,
                y,
                width: total_width - RIGHT_PADDING / 2.0,
                height: row_height,
                rx: None,
                ry: None,
                attrs: Attrs::new().with_class(&format!("section section{}", section_idx % 4)),
            };
            doc.add_element(section_bg);
        }
    }
}

/// Render task bars and labels
#[allow(clippy::too_many_arguments)]
fn render_task_bars(
    doc: &mut SvgDocument,
    tasks: &[crate::diagrams::gantt::Task],
    sections: &[(String, usize, usize)],
    min_date: Option<chrono::NaiveDateTime>,
    days_range: f64,
    left_padding: f64,
    top_padding: f64,
    chart_width: f64,
    row_height: f64,
) {
    let Some(min) = min_date else { return };

    // Calculate pixels per day (time scale)
    let px_per_day = chart_width / days_range;

    for (task_idx, task) in tasks.iter().enumerate() {
        let Some(start) = task.start_time else {
            continue;
        };
        let Some(end) = task.end_time else { continue };

        let start_offset = (start - min).num_days() as f64;
        let duration = (end - start).num_days().max(1) as f64;

        let bar_x = left_padding + start_offset * px_per_day;
        let bar_width = duration * px_per_day;
        let bar_y = task_idx as f64 * row_height + top_padding;

        // Find section index for this task
        let section_idx = sections
            .iter()
            .position(|(_, s, e)| task_idx >= *s && task_idx < *e)
            .unwrap_or(0);

        // Determine CSS class based on task flags
        let sec_num = section_idx % 4;
        let bar_class = if task.flags.active && task.flags.critical {
            format!("task activeCrit{}", sec_num)
        } else if task.flags.done && task.flags.critical {
            format!("task doneCrit{}", sec_num)
        } else if task.flags.done {
            format!("task done{}", sec_num)
        } else if task.flags.active {
            format!("task active{}", sec_num)
        } else if task.flags.critical {
            format!("task crit{}", sec_num)
        } else {
            format!("task task{}", sec_num)
        };

        // Handle special task types: vert markers and milestones
        let (final_x, final_y, final_width, final_height, extra_class) = if task.flags.vert {
            // Vert marker: thin vertical line spanning entire chart
            let vert_width = BAR_HEIGHT * 0.08; // Very narrow
            let vert_height = tasks.len() as f64 * row_height + BAR_HEIGHT * 2.0;
            (
                bar_x,
                GRID_LINE_START_PADDING,
                vert_width,
                vert_height,
                " vert ",
            )
        } else if task.flags.milestone {
            // Milestone: small square centered at midpoint
            let mid_x = bar_x + bar_width / 2.0 - BAR_HEIGHT / 2.0;
            (mid_x, bar_y, BAR_HEIGHT, BAR_HEIGHT, " milestone ")
        } else {
            (bar_x, bar_y, bar_width, BAR_HEIGHT, "")
        };

        // Calculate transform-origin for proper rotation (needed for milestones)
        let center_x = final_x + final_width / 2.0;
        let center_y = final_y + final_height / 2.0;
        let transform_origin = format!("{}px {}px", center_x, center_y);

        let bar_elem = SvgElement::Rect {
            x: final_x,
            y: final_y,
            width: final_width,
            height: final_height,
            rx: Some(3.0),
            ry: Some(3.0),
            attrs: Attrs::new()
                .with_class(&format!("{}{}", bar_class, extra_class))
                .with_attr("id", &task.id)
                .with_attr("transform-origin", &transform_origin),
        };
        doc.add_element(bar_elem);

        // Handle text positioning differently for vert markers
        if task.flags.vert {
            // Vert text: positioned below the chart, centered on the marker
            let vert_text_y = GRID_LINE_START_PADDING + tasks.len() as f64 * row_height + 60.0;
            let task_label = SvgElement::Text {
                x: final_x + final_width / 2.0,
                y: vert_text_y,
                content: task.task.clone(),
                attrs: Attrs::new()
                    .with_class(&format!("taskText taskText{} vertText", sec_num))
                    .with_attr("font-size", &format!("{}", FONT_SIZE as i32))
                    .with_attr("id", &format!("{}-text", task.id)),
            };
            doc.add_element(task_label);
        } else {
            // Standard task text positioning
            // Estimate text width (approx 0.5 * fontSize per character for typical fonts)
            let estimated_text_width = task.task.len() as f64 * FONT_SIZE * 0.5;
            let text_y = bar_y + BAR_HEIGHT / 2.0 + (FONT_SIZE / 2.0 - 2.0);

            // Determine if text fits inside bar, or needs to go outside
            let text_fits_inside = estimated_text_width <= final_width;
            let end_x = final_x + final_width;
            // Use a small margin (10px) - the text is placed at end_x + 5, so need 5px gap + some buffer
            let room_on_right = end_x + estimated_text_width + 10.0 <= TARGET_WIDTH;

            // Calculate text position and class based on fit
            let (text_x, text_position) = if text_fits_inside {
                (final_x + final_width / 2.0, TextPosition::Inside)
            } else if room_on_right {
                (end_x + 5.0, TextPosition::OutsideRight)
            } else {
                (final_x - 5.0, TextPosition::OutsideLeft)
            };

            // Determine text class based on position and task flags
            let text_class = build_text_class(sec_num, &task.flags, text_position);

            let milestone_text_class = if task.flags.milestone {
                " milestoneText"
            } else {
                ""
            };

            let task_label = SvgElement::Text {
                x: text_x,
                y: text_y,
                content: task.task.clone(),
                attrs: Attrs::new()
                    .with_class(&format!("{}{}", text_class, milestone_text_class))
                    .with_attr("font-size", &format!("{}", FONT_SIZE as i32))
                    .with_attr("id", &format!("{}-text", task.id)),
            };
            doc.add_element(task_label);
        }
    }
}

/// Text position relative to task bar
#[derive(Clone, Copy)]
enum TextPosition {
    Inside,
    OutsideRight,
    OutsideLeft,
}

/// Build CSS class string for task text based on position and flags
fn build_text_class(
    sec_num: usize,
    flags: &crate::diagrams::gantt::TaskFlags,
    position: TextPosition,
) -> String {
    // Base class depends on position
    let base_class = match position {
        TextPosition::Inside => format!("taskText taskText{}", sec_num),
        TextPosition::OutsideRight => {
            format!("taskTextOutsideRight taskTextOutside{}", sec_num)
        }
        TextPosition::OutsideLeft => format!("taskTextOutsideLeft taskTextOutside{}", sec_num),
    };

    // Add status-specific class
    let status_class = if flags.active {
        if flags.critical {
            format!(" activeCritText{}", sec_num)
        } else {
            format!(" activeText{}", sec_num)
        }
    } else if flags.done {
        if flags.critical {
            format!(" doneCritText{}", sec_num)
        } else {
            format!(" doneText{}", sec_num)
        }
    } else if flags.critical {
        format!(" critText{}", sec_num)
    } else {
        String::new()
    };

    format!("{}{}", base_class, status_class)
}

/// Render section labels on the left side
fn render_section_labels(
    doc: &mut SvgDocument,
    _tasks: &[crate::diagrams::gantt::Task],
    sections: &[(String, usize, usize)],
    top_padding: f64,
    row_height: f64,
) {
    for (section_idx, (section_name, start_idx, end_idx)) in sections.iter().enumerate() {
        if section_name.is_empty() {
            continue;
        }

        // Center label vertically across all tasks in section
        let section_tasks = end_idx - start_idx;
        let mid_row = *start_idx as f64 + (section_tasks as f64) / 2.0;
        let y = mid_row * row_height + top_padding;

        let label = SvgElement::Text {
            x: 10.0,
            y,
            content: section_name.clone(),
            attrs: Attrs::new()
                .with_class(&format!("sectionTitle sectionTitle{}", section_idx % 4))
                .with_attr("font-size", &format!("{}", FONT_SIZE as i32)),
        };
        doc.add_element(label);
    }
}

/// Render grid lines and axis with D3-style automatic tick intervals
fn render_grid_and_axis(
    doc: &mut SvgDocument,
    min_date: Option<chrono::NaiveDateTime>,
    days_range: f64,
    left_padding: f64,
    chart_width: f64,
    total_height: f64,
    grid_height: f64,
) {
    let Some(start) = min_date else { return };

    use chrono::{Datelike, NaiveDate};

    // Determine appropriate tick interval based on date range
    // D3's automatic behavior: monthly for multi-month, weekly for weeks, daily for days
    let (tick_dates, _format_str): (Vec<NaiveDate>, &str) = if days_range > 60.0 {
        // Monthly ticks
        let mut dates = Vec::new();
        let start_date = start.date();
        let end_date = start_date + chrono::Duration::days(days_range as i64);

        // Start from first of current or next month
        let mut current = if start_date.day() == 1 {
            start_date
        } else {
            // Move to first of next month
            let next_month = if start_date.month() == 12 {
                NaiveDate::from_ymd_opt(start_date.year() + 1, 1, 1)
            } else {
                NaiveDate::from_ymd_opt(start_date.year(), start_date.month() + 1, 1)
            };
            next_month.unwrap_or(start_date)
        };

        // Include start date if it's the first of month
        if start_date.day() == 1 {
            dates.push(start_date);
        }

        while current <= end_date {
            if current > start_date {
                dates.push(current);
            }
            // Move to next month
            let next = if current.month() == 12 {
                NaiveDate::from_ymd_opt(current.year() + 1, 1, 1)
            } else {
                NaiveDate::from_ymd_opt(current.year(), current.month() + 1, 1)
            };
            current = next.unwrap_or(end_date + chrono::Duration::days(1));
        }
        (dates, "%Y-%m-%d")
    } else if days_range > 14.0 {
        // Every 2-3 days
        let interval = 2;
        let mut dates = Vec::new();
        let start_date = start.date();
        for day in (0..=days_range as i64).step_by(interval) {
            dates.push(start_date + chrono::Duration::days(day));
        }
        (dates, "%Y-%m-%d")
    } else {
        // Daily
        let mut dates = Vec::new();
        let start_date = start.date();
        for day in 0..=days_range as i64 {
            dates.push(start_date + chrono::Duration::days(day));
        }
        (dates, "%Y-%m-%d")
    };

    let px_per_day = chart_width / days_range;
    let start_date = start.date();

    // Estimate label width for overlap detection
    // Date format "YYYY-MM-DD" = 10 chars, font-size 10, average char width ~0.6 * font-size
    let estimated_label_width = 10.0 * 10.0 * 0.6; // ~60px per label

    // Calculate minimum spacing between ticks
    let min_tick_spacing = if tick_dates.len() > 1 {
        let first_x = (tick_dates[0] - start_date).num_days() as f64 * px_per_day;
        let second_x = (tick_dates[1] - start_date).num_days() as f64 * px_per_day;
        (second_x - first_x).abs()
    } else {
        chart_width // Single tick, no overlap possible
    };

    // If labels would overlap, rotate them
    let should_rotate = estimated_label_width > min_tick_spacing * 0.9;

    // Grid group
    let mut grid_children = Vec::new();

    // Add path element for D3 compatibility
    grid_children.push(SvgElement::Path {
        d: format!(
            "M0.5,-{}V0.5H{}V-{}",
            grid_height as i32,
            chart_width as i32 + 1,
            grid_height as i32
        ),
        attrs: Attrs::new()
            .with_attr("stroke", "currentColor")
            .with_class("domain"),
    });

    for date in &tick_dates {
        let day_offset = (*date - start_date).num_days() as f64;
        let x = day_offset * px_per_day + 0.5;

        // Vertical grid line
        grid_children.push(SvgElement::Line {
            x1: x,
            y1: 0.0,
            x2: x,
            y2: -grid_height,
            attrs: Attrs::new().with_attr("stroke", "currentColor"),
        });

        // Tick label - rotate if labels would overlap
        let label = format!("{:04}-{:02}-{:02}", date.year(), date.month(), date.day());
        let label_attrs = if should_rotate {
            // Rotate -45 degrees around the label position
            // Use text-anchor: end so text extends up-left from the anchor point
            Attrs::new()
                .with_attr("fill", "#000")
                .with_attr("dy", "0.5em")
                .with_attr("stroke", "none")
                .with_attr("font-size", "10")
                .with_attr("style", "text-anchor: end;")
                .with_attr("transform", &format!("rotate(-45 {} 3)", x))
        } else {
            Attrs::new()
                .with_attr("fill", "#000")
                .with_attr("dy", "1em")
                .with_attr("stroke", "none")
                .with_attr("font-size", "10")
                .with_attr("style", "text-anchor: middle;")
        };
        grid_children.push(SvgElement::Text {
            x,
            y: 3.0,
            content: label,
            attrs: label_attrs,
        });
    }

    // Wrap in group with transform (like mermaid)
    let grid_group = SvgElement::Group {
        children: grid_children,
        attrs: Attrs::new()
            .with_class("grid")
            .with_attr(
                "transform",
                &format!("translate({}, {})", left_padding, total_height - 50.0),
            )
            .with_attr("fill", "none")
            .with_attr("font-size", "10")
            .with_attr("font-family", "sans-serif")
            .with_attr("text-anchor", "middle"),
    };
    doc.add_element(grid_group);
}

/// Render today line
fn render_today_line(
    doc: &mut SvgDocument,
    min_date: Option<chrono::NaiveDateTime>,
    days_range: f64,
    left_padding: f64,
    chart_width: f64,
    total_height: f64,
) {
    let Some(start) = min_date else { return };

    use chrono::Local;

    let today = Local::now().naive_local();
    let day_offset = (today - start).num_days() as f64;

    // Only render if today is within the chart range
    if day_offset < 0.0 || day_offset > days_range {
        return;
    }

    let px_per_day = chart_width / days_range;
    let x = left_padding + day_offset * px_per_day;

    let today_group = SvgElement::Group {
        children: vec![SvgElement::Line {
            x1: x,
            y1: TITLE_TOP_MARGIN,
            x2: x,
            y2: total_height - TOP_PADDING,
            attrs: Attrs::new().with_class("today"),
        }],
        attrs: Attrs::new().with_class("today"),
    };
    doc.add_element(today_group);
}

fn generate_gantt_css(theme: &crate::render::svg::Theme) -> String {
    // CSS closely matching mermaid.js gantt styles
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

.section0 {{
  fill: rgba(102, 102, 255, 0.49);
}}

.section2 {{
  fill: {section_bkg_color};
}}

.section1, .section3 {{
  fill: {section_bkg_color2};
  opacity: 0.2;
}}

.sectionTitle {{
  text-anchor: start;
  font-family: {font_family};
}}

.sectionTitle0, .sectionTitle1, .sectionTitle2, .sectionTitle3 {{
  fill: {text_color};
}}

.grid .tick {{
  stroke: lightgrey;
  opacity: 0.8;
  shape-rendering: crispEdges;
}}

.grid .tick text {{
  font-family: {font_family};
  fill: {text_color};
}}

.grid path {{
  stroke-width: 0;
}}

.today {{
  fill: none;
  stroke: red;
  stroke-width: 2px;
}}

.task {{
  stroke-width: 2;
}}

.taskText {{
  text-anchor: middle;
  font-family: {font_family};
}}

.taskTextOutsideRight {{
  fill: black;
  text-anchor: start;
  font-family: {font_family};
}}

.taskTextOutsideLeft {{
  fill: black;
  text-anchor: end;
}}

.taskText0, .taskText1, .taskText2, .taskText3 {{
  fill: {task_text_light_color};
}}

.task0, .task1, .task2, .task3 {{
  fill: {task_bkg_color};
  stroke: {task_border_color};
}}

.taskTextOutside0, .taskTextOutside2 {{
  fill: black;
}}

.taskTextOutside1, .taskTextOutside3 {{
  fill: black;
}}

.active0, .active1, .active2, .active3 {{
  fill: {active_task_bkg_color};
  stroke: {task_border_color};
}}

.activeText0, .activeText1, .activeText2, .activeText3 {{
  fill: {task_text_dark_color} !important;
}}

.done0, .done1, .done2, .done3 {{
  stroke: grey;
  fill: lightgrey;
  stroke-width: 2;
}}

.doneText0, .doneText1, .doneText2, .doneText3 {{
  fill: {task_text_dark_color} !important;
}}

.crit0, .crit1, .crit2, .crit3 {{
  stroke: #ff8888;
  fill: red;
  stroke-width: 2;
}}

.activeCrit0, .activeCrit1, .activeCrit2, .activeCrit3 {{
  stroke: #ff8888;
  fill: {active_task_bkg_color};
  stroke-width: 2;
}}

.doneCrit0, .doneCrit1, .doneCrit2, .doneCrit3 {{
  stroke: #ff8888;
  fill: lightgrey;
  stroke-width: 2;
}}

.milestone {{
  transform: rotate(45deg) scale(0.8,0.8);
}}

.milestoneText {{
  font-style: italic;
}}

.doneCritText0, .doneCritText1, .doneCritText2, .doneCritText3 {{
  fill: {task_text_dark_color} !important;
}}

.activeCritText0, .activeCritText1, .activeCritText2, .activeCritText3 {{
  fill: {task_text_dark_color} !important;
}}

.vert {{
  stroke: navy;
}}

.vertText {{
  font-size: 15px;
  text-anchor: middle;
  fill: navy !important;
}}

/* critText does not need a separate fill rule - it inherits white from taskText,
   which provides good contrast on red crit task backgrounds. The critText class
   exists only for programmatic identification, not styling. */
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
    )
}
