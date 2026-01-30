//! TUI renderer for pie charts.
//!
//! Since circular shapes don't render well in character art, pie charts are
//! displayed as horizontal bar charts with proportional widths, percentages,
//! and section labels. This preserves the key information while being readable
//! in any terminal.

use crate::diagrams::pie::PieDb;
use crate::error::Result;

/// Maximum width for the bar portion of the chart.
const BAR_WIDTH: usize = 40;

/// Block characters for filled bars.
const FULL_BLOCK: char = '█';
const HALF_BLOCK: char = '▌';

/// Render a pie chart as a horizontal bar chart in character art.
pub fn render_pie_tui(db: &PieDb) -> Result<String> {
    let sections = db.get_sections();
    let show_data = db.get_show_data();

    if sections.is_empty() {
        // Empty chart — just show title if present
        if let Some(title) = db.get_diagram_title() {
            return Ok(format!("{}\n\n(empty pie chart)\n", title));
        }
        return Ok("(empty pie chart)\n".to_string());
    }

    let total: f64 = sections.iter().map(|(_, v)| *v).sum();
    if total <= 0.0 {
        if let Some(title) = db.get_diagram_title() {
            return Ok(format!("{}\n\n(no data)\n", title));
        }
        return Ok("(no data)\n".to_string());
    }

    let mut lines: Vec<String> = Vec::new();

    // Title
    if let Some(title) = db.get_diagram_title() {
        lines.push(title.to_string());
        lines.push("─".repeat(title.chars().count().max(BAR_WIDTH)));
    }

    // Calculate percentages and find longest label for alignment
    let entries: Vec<(&str, f64, f64)> = sections
        .iter()
        .map(|(label, value)| {
            let pct = *value / total * 100.0;
            (label.as_str(), *value, pct)
        })
        .collect();

    let max_label_len = entries
        .iter()
        .map(|(label, value, _)| {
            if show_data {
                format_label_with_data(label, *value).chars().count()
            } else {
                label.chars().count()
            }
        })
        .max()
        .unwrap_or(0);

    // Render each section as a bar
    for (label, value, pct) in &entries {
        let display_label = if show_data {
            format_label_with_data(label, *value)
        } else {
            label.to_string()
        };

        // Pad label to align bars
        let padded_label = format!("{:width$}", display_label, width = max_label_len);

        // Calculate bar width proportional to percentage
        let bar_cells = (*pct / 100.0) * BAR_WIDTH as f64;
        let full_cells = bar_cells.floor() as usize;
        let has_half = (bar_cells - full_cells as f64) >= 0.5;

        let mut bar = String::new();
        for _ in 0..full_cells {
            bar.push(FULL_BLOCK);
        }
        if has_half {
            bar.push(HALF_BLOCK);
        }

        // Ensure at least one block for non-zero values
        if bar.is_empty() && *pct > 0.0 {
            bar.push(HALF_BLOCK);
        }

        let pct_str = format!("{:.1}%", pct);
        lines.push(format!("  {} │{} {}", padded_label, bar, pct_str));
    }

    // Add a total line
    lines.push(String::new());
    if show_data {
        let total_str = if total.fract() == 0.0 {
            format!("{}", total as i64)
        } else {
            format!("{:.1}", total)
        };
        lines.push(format!("  Total: {}", total_str));
    }

    lines.push(String::new());
    Ok(lines.join("\n"))
}

/// Format a label with its data value (for showData mode).
fn format_label_with_data(label: &str, value: f64) -> String {
    let value_str = if value.fract() == 0.0 {
        format!("{}", value as i64)
    } else {
        format!("{}", value)
    };
    format!("{} [{}]", label, value_str)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_pie(title: Option<&str>, sections: &[(&str, f64)], show_data: bool) -> PieDb {
        let mut db = PieDb::new();
        if let Some(t) = title {
            db.set_diagram_title(t);
        }
        for (label, value) in sections {
            db.add_section(*label, *value).unwrap();
        }
        db.set_show_data(show_data);
        db
    }

    #[test]
    fn empty_pie_chart() {
        let db = make_pie(None, &[], false);
        let output = render_pie_tui(&db).unwrap();
        assert!(
            output.contains("empty pie chart"),
            "Empty chart should say so\nOutput:\n{}",
            output
        );
    }

    #[test]
    fn empty_pie_chart_with_title() {
        let db = make_pie(Some("My Chart"), &[], false);
        let output = render_pie_tui(&db).unwrap();
        assert!(output.contains("My Chart"), "Should show title");
        assert!(output.contains("empty pie chart"));
    }

    #[test]
    fn single_section_shows_100_percent() {
        let db = make_pie(None, &[("Only", 10.0)], false);
        let output = render_pie_tui(&db).unwrap();
        assert!(
            output.contains("100.0%"),
            "Single section should be 100%\nOutput:\n{}",
            output
        );
        assert!(output.contains("Only"), "Should contain label");
    }

    #[test]
    fn two_sections_show_percentages() {
        let db = make_pie(None, &[("A", 75.0), ("B", 25.0)], false);
        let output = render_pie_tui(&db).unwrap();
        assert!(
            output.contains("75.0%"),
            "Should show 75%\nOutput:\n{}",
            output
        );
        assert!(
            output.contains("25.0%"),
            "Should show 25%\nOutput:\n{}",
            output
        );
    }

    #[test]
    fn bars_are_proportional() {
        let db = make_pie(None, &[("Big", 75.0), ("Small", 25.0)], false);
        let output = render_pie_tui(&db).unwrap();
        let lines: Vec<&str> = output.lines().collect();
        // Find the bar lines (contain █)
        let bar_lines: Vec<&str> = lines
            .iter()
            .filter(|l| l.contains(FULL_BLOCK) || l.contains(HALF_BLOCK))
            .copied()
            .collect();
        assert_eq!(
            bar_lines.len(),
            2,
            "Should have 2 bar lines\nOutput:\n{}",
            output
        );

        // Count block chars in each bar
        let count_blocks = |line: &str| -> usize {
            line.chars()
                .filter(|&c| c == FULL_BLOCK || c == HALF_BLOCK)
                .count()
        };
        let big_blocks = count_blocks(bar_lines[0]);
        let small_blocks = count_blocks(bar_lines[1]);
        assert!(
            big_blocks > small_blocks,
            "Big section ({}) should have more blocks than Small ({})",
            big_blocks,
            small_blocks
        );
    }

    #[test]
    fn title_appears_in_output() {
        let db = make_pie(
            Some("Project Distribution"),
            &[("Dev", 40.0), ("Test", 25.0)],
            false,
        );
        let output = render_pie_tui(&db).unwrap();
        assert!(
            output.contains("Project Distribution"),
            "Title should appear\nOutput:\n{}",
            output
        );
    }

    #[test]
    fn show_data_includes_values() {
        let db = make_pie(None, &[("Dev", 40.0), ("Test", 25.0)], true);
        let output = render_pie_tui(&db).unwrap();
        assert!(
            output.contains("[40]"),
            "Should show data value 40\nOutput:\n{}",
            output
        );
        assert!(
            output.contains("[25]"),
            "Should show data value 25\nOutput:\n{}",
            output
        );
        assert!(
            output.contains("Total: 65"),
            "Should show total\nOutput:\n{}",
            output
        );
    }

    #[test]
    fn labels_are_aligned() {
        let db = make_pie(None, &[("Short", 10.0), ("Much Longer Label", 10.0)], false);
        let output = render_pie_tui(&db).unwrap();
        let lines: Vec<&str> = output.lines().collect();
        let bar_lines: Vec<&str> = lines.iter().filter(|l| l.contains('│')).copied().collect();
        assert_eq!(bar_lines.len(), 2);
        // The │ separator should be at the same column position
        let pipe_pos = |line: &str| line.find('│').unwrap();
        assert_eq!(
            pipe_pos(bar_lines[0]),
            pipe_pos(bar_lines[1]),
            "Bars should be aligned\nOutput:\n{}",
            output
        );
    }

    #[test]
    fn gallery_pie_renders() {
        let input = std::fs::read_to_string("docs/sources/pie.mmd").unwrap();
        let diagram = crate::parse(&input).unwrap();
        let db = match diagram {
            crate::diagrams::Diagram::Pie(db) => db,
            _ => panic!("Expected pie diagram"),
        };
        let output = render_pie_tui(&db).unwrap();
        assert!(
            output.contains("Development"),
            "Should contain Development\nOutput:\n{}",
            output
        );
        assert!(
            output.contains("Testing"),
            "Should contain Testing\nOutput:\n{}",
            output
        );
        assert!(
            output.contains("40.0%"),
            "Dev should be 40%\nOutput:\n{}",
            output
        );
    }

    #[test]
    fn gallery_pie_complex_renders() {
        let input = std::fs::read_to_string("docs/sources/pie_complex.mmd").unwrap();
        let diagram = crate::parse(&input).unwrap();
        let db = match diagram {
            crate::diagrams::Diagram::Pie(db) => db,
            _ => panic!("Expected pie diagram"),
        };
        let output = render_pie_tui(&db).unwrap();
        // Complex pie has showData and 8 slices
        assert!(
            output.contains("Compute"),
            "Should contain Compute\nOutput:\n{}",
            output
        );
        assert!(
            output.contains("[35]"),
            "showData should show value 35\nOutput:\n{}",
            output
        );
        assert!(
            output.contains("Cloud Infrastructure Costs"),
            "Should show title\nOutput:\n{}",
            output
        );
    }

    #[test]
    fn nonzero_section_gets_at_least_one_block() {
        // A very small section should still show at least one block character
        let db = make_pie(None, &[("Huge", 99.0), ("Tiny", 1.0)], false);
        let output = render_pie_tui(&db).unwrap();
        let tiny_line = output.lines().find(|l| l.contains("Tiny")).unwrap();
        let has_block = tiny_line.contains(FULL_BLOCK) || tiny_line.contains(HALF_BLOCK);
        assert!(
            has_block,
            "Even 1% should show at least one block\nLine: {}",
            tiny_line
        );
    }
}
