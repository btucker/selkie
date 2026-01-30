//! TUI renderer for Gantt charts.
//!
//! Renders gantt charts as horizontal timeline bars in character art.
//! Each task gets a row with its name, status indicators, and a proportional
//! bar positioned on a time axis. Section headers act as visual separators.

use chrono::NaiveDateTime;

use crate::diagrams::gantt::GanttDb;
use crate::error::Result;

/// Width of the timeline bar area in characters.
const TIMELINE_WIDTH: usize = 50;

/// Block characters for task bars.
const FULL_BLOCK: char = '█';
const LIGHT_BLOCK: char = '░';

/// Render a Gantt chart as character art.
pub fn render_gantt_tui(db: &mut GanttDb) -> Result<String> {
    let tasks = db.get_tasks();

    if tasks.is_empty() {
        let title = db.get_diagram_title();
        if !title.is_empty() {
            return Ok(format!("{}\n\n(empty gantt chart)\n", title));
        }
        return Ok("(empty gantt chart)\n".to_string());
    }

    // Find the overall time range
    let mut min_time: Option<NaiveDateTime> = None;
    let mut max_time: Option<NaiveDateTime> = None;

    for task in &tasks {
        if task.flags.vert {
            continue; // Skip vertical markers for range calculation
        }
        if let Some(start) = task.start_time {
            min_time = Some(min_time.map_or(start, |m: NaiveDateTime| m.min(start)));
        }
        if let Some(end) = task.end_time {
            max_time = Some(max_time.map_or(end, |m: NaiveDateTime| m.max(end)));
        }
    }

    let (chart_start, chart_end) = match (min_time, max_time) {
        (Some(s), Some(e)) => {
            if s == e {
                // Single-point timeline — extend by 1 day
                (s, e + chrono::Duration::days(1))
            } else {
                (s, e)
            }
        }
        _ => {
            return Ok("(no resolved dates)\n".to_string());
        }
    };

    let total_duration = (chart_end - chart_start).num_seconds().max(1) as f64;

    // Calculate label widths for alignment
    let max_task_name_len = tasks
        .iter()
        .filter(|t| !t.flags.vert)
        .map(|t| t.task.chars().count())
        .max()
        .unwrap_or(0);

    // Add space for status prefix (e.g., "✓ " or "► ")
    let label_col_width = max_task_name_len + 3;

    let mut lines: Vec<String> = Vec::new();

    // Title
    let title = db.get_diagram_title();
    if !title.is_empty() {
        lines.push(title.to_string());
        lines.push("─".repeat(label_col_width + 1 + TIMELINE_WIDTH));
    }

    // Date axis header
    let start_str = chart_start.format("%Y-%m-%d").to_string();
    let end_str = chart_end.format("%Y-%m-%d").to_string();
    let axis_padding = TIMELINE_WIDTH.saturating_sub(start_str.len() + end_str.len());
    lines.push(format!(
        "{:width$} │{}{}{}",
        "",
        start_str,
        " ".repeat(axis_padding),
        end_str,
        width = label_col_width
    ));
    lines.push(format!(
        "{:width$} │{}",
        "",
        "─".repeat(TIMELINE_WIDTH),
        width = label_col_width
    ));

    // Track current section for headers
    let mut current_section = String::new();

    for task in &tasks {
        // Section header (emit even for vert-only sections)
        if task.section != current_section && !task.section.is_empty() {
            current_section = task.section.clone();
            lines.push(format!("  [{}]", current_section));
        }

        // Skip vert markers in main task list (no bar rendering)
        if task.flags.vert {
            continue;
        }

        // Status prefix
        let prefix = if task.flags.milestone {
            "◆"
        } else if task.flags.done {
            "✓"
        } else if task.flags.active {
            "►"
        } else if task.flags.critical {
            "!"
        } else {
            " "
        };

        // Task name with padding
        let label = format!("{} {:width$}", prefix, task.task, width = max_task_name_len);

        // Calculate bar position
        let (bar_start_col, bar_end_col) = match (task.start_time, task.end_time) {
            (Some(start), Some(end)) => {
                let start_offset = (start - chart_start).num_seconds() as f64;
                let end_offset = (end - chart_start).num_seconds() as f64;
                let col_start =
                    ((start_offset / total_duration) * TIMELINE_WIDTH as f64).round() as usize;
                let col_end =
                    ((end_offset / total_duration) * TIMELINE_WIDTH as f64).round() as usize;
                (
                    col_start.min(TIMELINE_WIDTH),
                    col_end.min(TIMELINE_WIDTH).max(col_start + 1),
                )
            }
            (Some(start), None) => {
                // No end — show a single marker
                let start_offset = (start - chart_start).num_seconds() as f64;
                let col =
                    ((start_offset / total_duration) * TIMELINE_WIDTH as f64).round() as usize;
                (
                    col.min(TIMELINE_WIDTH.saturating_sub(1)),
                    col.min(TIMELINE_WIDTH.saturating_sub(1)) + 1,
                )
            }
            _ => (0, 1),
        };

        // Build the timeline bar
        let mut bar = String::with_capacity(TIMELINE_WIDTH);
        let bar_char = if task.flags.milestone {
            '◆'
        } else if task.flags.done {
            LIGHT_BLOCK
        } else {
            FULL_BLOCK
        };

        for col in 0..TIMELINE_WIDTH {
            if col >= bar_start_col && col < bar_end_col {
                bar.push(bar_char);
            } else {
                bar.push(' ');
            }
        }

        // Date range suffix
        let date_suffix = match (task.start_time, task.end_time) {
            (Some(s), Some(e)) => {
                let days = (e - s).num_days();
                if task.flags.milestone {
                    s.format("%m-%d").to_string()
                } else {
                    format!("{}d", days)
                }
            }
            _ => String::new(),
        };

        lines.push(format!("{} │{} {}", label, bar, date_suffix));
    }

    lines.push(String::new());
    Ok(lines.join("\n"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagrams::gantt::GanttDb;

    fn make_gantt(input: &str) -> GanttDb {
        let diagram = crate::parse(input).unwrap();
        match diagram {
            crate::diagrams::Diagram::Gantt(db) => db,
            _ => panic!("Expected gantt diagram"),
        }
    }

    #[test]
    fn empty_gantt_chart() {
        let mut db = GanttDb::new();
        let output = render_gantt_tui(&mut db).unwrap();
        assert!(
            output.contains("empty gantt chart"),
            "Should indicate empty\nOutput:\n{}",
            output
        );
    }

    #[test]
    fn single_task_renders() {
        let mut db = make_gantt(
            "gantt\n    dateFormat YYYY-MM-DD\n    section Dev\n    Task1 :a1, 2024-01-01, 5d",
        );
        let output = render_gantt_tui(&mut db).unwrap();
        assert!(
            output.contains("Task1"),
            "Should contain task name\nOutput:\n{}",
            output
        );
        assert!(
            output.contains(FULL_BLOCK),
            "Should have bar blocks\nOutput:\n{}",
            output
        );
    }

    #[test]
    fn section_headers_appear() {
        let mut db = make_gantt(
            "gantt\n    dateFormat YYYY-MM-DD\n    section Planning\n    Task1 :a1, 2024-01-01, 5d\n    section Dev\n    Task2 :a2, after a1, 3d",
        );
        let output = render_gantt_tui(&mut db).unwrap();
        assert!(
            output.contains("[Planning]"),
            "Should show Planning section\nOutput:\n{}",
            output
        );
        assert!(
            output.contains("[Dev]"),
            "Should show Dev section\nOutput:\n{}",
            output
        );
    }

    #[test]
    fn title_appears() {
        let mut db = make_gantt(
            "gantt\n    title My Project\n    dateFormat YYYY-MM-DD\n    section Dev\n    Task1 :a1, 2024-01-01, 5d",
        );
        let output = render_gantt_tui(&mut db).unwrap();
        assert!(
            output.contains("My Project"),
            "Should show title\nOutput:\n{}",
            output
        );
    }

    #[test]
    fn done_task_shows_checkmark() {
        let mut db = make_gantt(
            "gantt\n    dateFormat YYYY-MM-DD\n    section Dev\n    Task1 :done, a1, 2024-01-01, 5d",
        );
        let output = render_gantt_tui(&mut db).unwrap();
        assert!(
            output.contains('✓'),
            "Done task should show ✓\nOutput:\n{}",
            output
        );
        assert!(
            output.contains(LIGHT_BLOCK),
            "Done task should use light block\nOutput:\n{}",
            output
        );
    }

    #[test]
    fn active_task_shows_arrow() {
        let mut db = make_gantt(
            "gantt\n    dateFormat YYYY-MM-DD\n    section Dev\n    Task1 :active, a1, 2024-01-01, 5d",
        );
        let output = render_gantt_tui(&mut db).unwrap();
        assert!(
            output.contains('►'),
            "Active task should show ►\nOutput:\n{}",
            output
        );
    }

    #[test]
    fn critical_task_shows_bang() {
        let mut db = make_gantt(
            "gantt\n    dateFormat YYYY-MM-DD\n    section Dev\n    Task1 :crit, a1, 2024-01-01, 5d",
        );
        let output = render_gantt_tui(&mut db).unwrap();
        assert!(
            output.contains('!'),
            "Critical task should show !\nOutput:\n{}",
            output
        );
    }

    #[test]
    fn milestone_shows_diamond() {
        let mut db = make_gantt(
            "gantt\n    dateFormat YYYY-MM-DD\n    section Dev\n    Task1 :a1, 2024-01-01, 5d\n    Release :milestone, m1, after a1, 1d",
        );
        let output = render_gantt_tui(&mut db).unwrap();
        assert!(
            output.contains('◆'),
            "Milestone should show ◆\nOutput:\n{}",
            output
        );
    }

    #[test]
    fn date_axis_shows_range() {
        let mut db = make_gantt(
            "gantt\n    dateFormat YYYY-MM-DD\n    section Dev\n    Task1 :a1, 2024-01-01, 5d",
        );
        let output = render_gantt_tui(&mut db).unwrap();
        assert!(
            output.contains("2024-01-01"),
            "Should show start date\nOutput:\n{}",
            output
        );
        assert!(
            output.contains("2024-01-06"),
            "Should show end date\nOutput:\n{}",
            output
        );
    }

    #[test]
    fn duration_shown_in_days() {
        let mut db = make_gantt(
            "gantt\n    dateFormat YYYY-MM-DD\n    section Dev\n    Task1 :a1, 2024-01-01, 10d",
        );
        let output = render_gantt_tui(&mut db).unwrap();
        assert!(
            output.contains("10d"),
            "Should show duration in days\nOutput:\n{}",
            output
        );
    }

    #[test]
    fn gallery_gantt_renders() {
        let input = std::fs::read_to_string("docs/sources/gantt.mmd").unwrap();
        let mut db = make_gantt(&input);
        let output = render_gantt_tui(&mut db).unwrap();
        assert!(
            output.contains("Project Timeline"),
            "Should show title\nOutput:\n{}",
            output
        );
        assert!(
            output.contains("Requirements"),
            "Should contain task\nOutput:\n{}",
            output
        );
        assert!(
            output.contains("[Planning]"),
            "Should show section\nOutput:\n{}",
            output
        );
        assert!(
            output.contains("[Development]"),
            "Should show section\nOutput:\n{}",
            output
        );
        assert!(
            output.contains("[Testing]"),
            "Should show section\nOutput:\n{}",
            output
        );
    }

    #[test]
    fn gallery_gantt_complex_renders() {
        let input = std::fs::read_to_string("docs/sources/gantt_complex.mmd").unwrap();
        let mut db = make_gantt(&input);
        let output = render_gantt_tui(&mut db).unwrap();
        assert!(
            output.contains("Product Launch Timeline"),
            "Should show title\nOutput:\n{}",
            output
        );
        // Check flags
        assert!(
            output.contains('✓'),
            "Should have done tasks\nOutput:\n{}",
            output
        );
        assert!(
            output.contains('►'),
            "Should have active task\nOutput:\n{}",
            output
        );
        assert!(
            output.contains('◆'),
            "Should have milestone\nOutput:\n{}",
            output
        );
    }

    #[test]
    fn longer_task_has_wider_bar() {
        let mut db = make_gantt(
            "gantt\n    dateFormat YYYY-MM-DD\n    section Dev\n    Short :a1, 2024-01-01, 2d\n    Long  :a2, 2024-01-01, 20d",
        );
        let output = render_gantt_tui(&mut db).unwrap();
        let lines: Vec<&str> = output.lines().collect();

        let count_blocks = |line: &str| -> usize {
            line.chars()
                .filter(|&c| c == FULL_BLOCK || c == LIGHT_BLOCK)
                .count()
        };

        let short_line = lines.iter().find(|l| l.contains("Short")).unwrap();
        let long_line = lines.iter().find(|l| l.contains("Long")).unwrap();
        assert!(
            count_blocks(long_line) > count_blocks(short_line),
            "Long task should have wider bar\nShort: {}\nLong: {}",
            short_line,
            long_line
        );
    }

    #[test]
    fn bars_are_aligned() {
        let mut db = make_gantt(
            "gantt\n    dateFormat YYYY-MM-DD\n    section Dev\n    Short :a1, 2024-01-01, 5d\n    Much Longer Name :a2, after a1, 3d",
        );
        let output = render_gantt_tui(&mut db).unwrap();
        let lines: Vec<&str> = output.lines().collect();
        let bar_lines: Vec<&str> = lines
            .iter()
            .filter(|l| l.contains(FULL_BLOCK))
            .copied()
            .collect();
        assert_eq!(bar_lines.len(), 2, "Should have 2 bar lines");
        // The │ separator should be at the same column
        let pipe_pos = |line: &str| line.find('│').unwrap();
        assert_eq!(
            pipe_pos(bar_lines[0]),
            pipe_pos(bar_lines[1]),
            "Bars should be vertically aligned\nOutput:\n{}",
            output
        );
    }
}
