//! ASCII renderer for pie charts.
//!
//! Renders pie charts as circular ASCII art using Unicode block characters
//! to fill pie slices, with a legend showing labels and percentages.
//! Uses polar coordinate math (same as the SVG renderer) to determine
//! slice boundaries.

use std::f64::consts::PI;

use crate::diagrams::pie::PieDb;
use crate::error::Result;

/// Radius of the pie circle in character cells.
const PIE_RADIUS: usize = 8;

/// Fill characters for different slices — visually distinct density patterns.
const SLICE_CHARS: &[char] = &['█', '▓', '▒', '░', '◆', '●', '■', '▲'];

/// Render a pie chart as a circular ASCII pie with legend.
pub fn render_pie_ascii(db: &PieDb) -> Result<String> {
    let sections = db.get_sections();
    let show_data = db.get_show_data();

    if sections.is_empty() {
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
    }

    // Build slice angle ranges: each slice spans [start_angle, end_angle)
    // Start at -π/2 (12 o'clock), sweep clockwise like the SVG renderer.
    let mut slices: Vec<(f64, f64, usize)> = Vec::new(); // (start, end, index)
    let mut angle = -PI / 2.0;
    for (i, (_label, value)) in sections.iter().enumerate() {
        let sweep = (*value / total) * 2.0 * PI;
        slices.push((angle, angle + sweep, i));
        angle += sweep;
    }

    // Render the circle row by row.
    // Character cells are taller than wide (~2:1), so we scale x by 2 to
    // compensate, making the circle appear round in a monospace terminal.
    let r = PIE_RADIUS as f64;
    let diameter = PIE_RADIUS * 2 + 1;

    for row in 0..diameter {
        let dy = row as f64 - r; // vertical offset from center (-r..+r)
        let mut line = String::new();

        // Add left padding for centering (match legend width roughly)
        line.push_str("  ");

        for col in 0..(diameter * 2) {
            let dx = (col as f64 - r * 2.0) / 2.0; // horizontal offset, scaled

            let dist = (dx * dx + dy * dy).sqrt();
            if dist > r + 0.5 {
                line.push(' ');
                continue;
            }

            // Determine which slice this pixel belongs to
            let mut theta = dy.atan2(dx);
            // Normalize to same range as slices (-π/2 .. 3π/2)
            if theta < -PI / 2.0 {
                theta += 2.0 * PI;
            }

            let slice_idx = slices
                .iter()
                .find(|(start, end, _)| theta >= *start && theta < *end)
                .map(|(_, _, idx)| *idx)
                .unwrap_or(slices.last().map(|(_, _, idx)| *idx).unwrap_or(0));

            let ch = SLICE_CHARS[slice_idx % SLICE_CHARS.len()];
            line.push(ch);
        }

        lines.push(line.trim_end().to_string());
    }

    // Legend
    lines.push(String::new());
    let entries: Vec<(&str, f64, f64)> = sections
        .iter()
        .map(|(label, value)| {
            let pct = *value / total * 100.0;
            (label.as_str(), *value, pct)
        })
        .collect();

    for (i, (label, value, pct)) in entries.iter().enumerate() {
        let marker = SLICE_CHARS[i % SLICE_CHARS.len()];
        let display_label = if show_data {
            format_label_with_data(label, *value)
        } else {
            label.to_string()
        };
        lines.push(format!("  {} {}: {:.1}%", marker, display_label, pct));
    }

    // Total line for showData
    if show_data {
        lines.push(String::new());
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
        let output = render_pie_ascii(&db).unwrap();
        assert!(
            output.contains("empty pie chart"),
            "Empty chart should say so\nOutput:\n{}",
            output
        );
    }

    #[test]
    fn empty_pie_chart_with_title() {
        let db = make_pie(Some("My Chart"), &[], false);
        let output = render_pie_ascii(&db).unwrap();
        assert!(output.contains("My Chart"), "Should show title");
        assert!(output.contains("empty pie chart"));
    }

    #[test]
    fn single_section_shows_100_percent() {
        let db = make_pie(None, &[("Only", 10.0)], false);
        let output = render_pie_ascii(&db).unwrap();
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
        let output = render_pie_ascii(&db).unwrap();
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
    fn slices_are_proportional_in_circle() {
        let db = make_pie(None, &[("Big", 75.0), ("Small", 25.0)], false);
        let output = render_pie_ascii(&db).unwrap();

        // Count fill characters for each slice in the circle body
        let big_char = SLICE_CHARS[0];
        let small_char = SLICE_CHARS[1];

        let big_count: usize = output.chars().filter(|&c| c == big_char).count();
        let small_count: usize = output.chars().filter(|&c| c == small_char).count();

        assert!(
            big_count > small_count,
            "Big slice ({} chars) should have more fill than Small ({} chars)\nOutput:\n{}",
            big_count,
            small_count,
            output
        );
    }

    #[test]
    fn title_appears_in_output() {
        let db = make_pie(
            Some("Project Distribution"),
            &[("Dev", 40.0), ("Test", 25.0)],
            false,
        );
        let output = render_pie_ascii(&db).unwrap();
        assert!(
            output.contains("Project Distribution"),
            "Title should appear\nOutput:\n{}",
            output
        );
    }

    #[test]
    fn show_data_includes_values() {
        let db = make_pie(None, &[("Dev", 40.0), ("Test", 25.0)], true);
        let output = render_pie_ascii(&db).unwrap();
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
    fn legend_contains_all_labels() {
        let db = make_pie(None, &[("Short", 10.0), ("Much Longer Label", 10.0)], false);
        let output = render_pie_ascii(&db).unwrap();
        assert!(
            output.contains("Short"),
            "Legend should contain Short\nOutput:\n{}",
            output
        );
        assert!(
            output.contains("Much Longer Label"),
            "Legend should contain Much Longer Label\nOutput:\n{}",
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
        let output = render_pie_ascii(&db).unwrap();
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
        let output = render_pie_ascii(&db).unwrap();
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
    fn small_section_appears_in_legend() {
        // Even a very small section should appear in the legend
        let db = make_pie(None, &[("Huge", 99.0), ("Tiny", 1.0)], false);
        let output = render_pie_ascii(&db).unwrap();
        assert!(
            output.contains("Tiny"),
            "Small section should appear in legend\nOutput:\n{}",
            output
        );
        assert!(
            output.contains("1.0%"),
            "Small section should show percentage\nOutput:\n{}",
            output
        );
    }

    #[test]
    fn many_slices_uses_all_slice_chars() {
        // >8 sections forces all SLICE_CHARS to appear (wrapping around)
        let db = make_pie(
            Some("Many Slices"),
            &[
                ("A", 15.0),
                ("B", 12.0),
                ("C", 10.0),
                ("D", 10.0),
                ("E", 10.0),
                ("F", 10.0),
                ("G", 10.0),
                ("H", 10.0),
                ("I", 8.0),
                ("J", 5.0),
            ],
            false,
        );
        let output = render_pie_ascii(&db).unwrap();

        // Every one of the 8 distinct slice characters should appear
        for &ch in SLICE_CHARS {
            assert!(
                output.contains(ch),
                "Output should contain slice char '{}'\nOutput:\n{}",
                ch,
                output
            );
        }
    }

    #[test]
    fn renders_as_circular_pie_not_bar_chart() {
        // The pie chart should render as a circular shape, not as horizontal bars.
        let db = make_pie(
            Some("Test Chart"),
            &[("A", 60.0), ("B", 30.0), ("C", 10.0)],
            false,
        );
        let output = render_pie_ascii(&db).unwrap();

        // A bar chart has lines matching "label │████ XX.X%" pattern.
        // A circular pie chart should NOT have this pattern.
        let bar_chart_lines = output.lines().filter(|l| l.contains('│')).count();
        assert_eq!(
            bar_chart_lines, 0,
            "Pie chart should render as a circle, not as bars with │ separators\nOutput:\n{}",
            output
        );

        // The output should still contain all labels and percentages in the legend
        assert!(output.contains("A"), "Missing label A\nOutput:\n{}", output);
        assert!(output.contains("B"), "Missing label B\nOutput:\n{}", output);
        assert!(output.contains("C"), "Missing label C\nOutput:\n{}", output);
        assert!(
            output.contains("60.0%"),
            "Missing percentage 60.0%\nOutput:\n{}",
            output
        );

        // The output should have many rows (circle body + legend), not just
        // one row per section like a bar chart.
        let non_empty_lines: Vec<&str> = output.lines().filter(|l| !l.trim().is_empty()).collect();
        assert!(
            non_empty_lines.len() > 10,
            "Circular pie should have many lines (got {}), not just a few bar lines\nOutput:\n{}",
            non_empty_lines.len(),
            output
        );
    }
}
