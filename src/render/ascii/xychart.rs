//! ASCII renderer for XY chart diagrams.
//!
//! Renders a unified vertical chart where bar columns rise from the bottom
//! and line markers (●) are overlaid at their respective heights, all sharing
//! a single Y-axis grid. Categories appear along the x-axis at the bottom.

use crate::diagrams::xychart::{PlotType, XAxisData, XYChartDb, YAxisData};
use crate::error::Result;

/// Height of the chart area in character rows.
const CHART_HEIGHT: usize = 16;
/// Width allocated per category column in characters.
const COL_WIDTH: usize = 6;
/// Characters used for bar rendering.
const FULL_BLOCK: char = '█';
const UPPER_HALF: char = '▀';
/// Character for line data points.
const LINE_MARKER: char = '●';

/// Render an XY chart as character art.
pub fn render_xychart_ascii(db: &XYChartDb) -> Result<String> {
    let plots = db.get_plots();
    if plots.is_empty() {
        let title = &db.title;
        if !title.is_empty() {
            return Ok(format!("{}\n\n(empty chart)\n", title));
        }
        return Ok("(empty chart)\n".to_string());
    }

    let mut out: Vec<String> = Vec::new();

    // Title
    if !db.title.is_empty() {
        out.push(db.title.clone());
        out.push("─".repeat(db.title.chars().count()));
    }

    // Get categories from x-axis
    let categories: Vec<String> = match &db.x_axis {
        Some(XAxisData::Band(band)) => band.categories.clone(),
        _ => plots
            .first()
            .map(|p| p.data.iter().map(|d| d.label.clone()).collect())
            .unwrap_or_default(),
    };

    let num_cats = categories.len();
    if num_cats == 0 {
        out.push("(no data)".to_string());
        out.push(String::new());
        return Ok(out.join("\n"));
    }

    // Find global max value across all plots for scaling
    let max_value = plots
        .iter()
        .flat_map(|p| p.data.iter().map(|d| d.value))
        .fold(0.0f64, f64::max);

    if max_value <= 0.0 {
        out.push("(no data)".to_string());
        out.push(String::new());
        return Ok(out.join("\n"));
    }

    // Compute nice Y-axis tick values
    let y_ticks = compute_y_ticks(max_value, 5);
    let y_max = *y_ticks.last().unwrap_or(&max_value);
    let y_label_width = y_ticks
        .iter()
        .map(|v| format_value(*v).len())
        .max()
        .unwrap_or(3);

    // Y-axis title
    if let Some(ref y_axis) = db.y_axis {
        let y_title = match y_axis {
            YAxisData::Linear(data) => &data.title,
        };
        if !y_title.is_empty() {
            out.push(format!(
                "{:>width$}  {}",
                "",
                y_title,
                width = y_label_width
            ));
        }
    }

    // Collect bar heights and line heights per category (in chart rows)
    let bar_heights: Vec<Option<usize>> = {
        let bar_plot = plots.iter().find(|p| p.plot_type == PlotType::Bar);
        (0..num_cats)
            .map(|i| {
                bar_plot.and_then(|p| {
                    p.data
                        .get(i)
                        .map(|d| ((d.value / y_max) * CHART_HEIGHT as f64).round() as usize)
                })
            })
            .collect()
    };

    // Collect line marker rows for ALL line plots per category.
    // Each category gets a vec of rows (one per line plot).
    let line_plots: Vec<_> = plots
        .iter()
        .filter(|p| p.plot_type == PlotType::Line)
        .collect();
    let line_rows_per_cat: Vec<Vec<usize>> = (0..num_cats)
        .map(|i| {
            line_plots
                .iter()
                .filter_map(|p| {
                    p.data.get(i).map(|d| {
                        let row_from_bottom =
                            ((d.value / y_max) * CHART_HEIGHT as f64).round() as usize;
                        CHART_HEIGHT.saturating_sub(row_from_bottom)
                    })
                })
                .collect()
        })
        .collect();

    let chart_width = num_cats * COL_WIDTH;

    // Render chart rows from top (row 0) to bottom (row CHART_HEIGHT-1)
    for row in 0..CHART_HEIGHT {
        let rows_from_bottom = CHART_HEIGHT - row;

        // Y-axis tick label
        let y_value = (rows_from_bottom as f64 / CHART_HEIGHT as f64) * y_max;
        let y_label = if y_ticks.iter().any(|t| {
            let tick_row = ((*t / y_max) * CHART_HEIGHT as f64).round() as usize;
            tick_row == rows_from_bottom
        }) {
            format!("{:>width$}", format_value(y_value), width = y_label_width)
        } else {
            " ".repeat(y_label_width)
        };

        // Build the row content across all categories
        let mut row_chars: Vec<char> = vec![' '; chart_width];
        for cat_i in 0..num_cats {
            let col_start = cat_i * COL_WIDTH;
            let bar_center = col_start + COL_WIDTH / 2;

            // Draw bar fill: if this row is within the bar height, fill center columns
            if let Some(bh) = bar_heights[cat_i] {
                if rows_from_bottom <= bh {
                    // Fill the center portion of the column with block chars
                    let fill_start = col_start + 1;
                    let fill_end = (col_start + COL_WIDTH - 1).min(chart_width);
                    for ch in row_chars.iter_mut().take(fill_end).skip(fill_start) {
                        *ch = FULL_BLOCK;
                    }
                } else if rows_from_bottom == bh + 1 && bh > 0 {
                    // Top cap of bar: use upper-half block for sub-row precision
                    let fill_start = col_start + 1;
                    let fill_end = (col_start + COL_WIDTH - 1).min(chart_width);
                    for ch in row_chars.iter_mut().take(fill_end).skip(fill_start) {
                        *ch = UPPER_HALF;
                    }
                }
            }

            // Overlay line markers for all line plots at this category
            for &lr in &line_rows_per_cat[cat_i] {
                if row == lr && bar_center < chart_width {
                    row_chars[bar_center] = LINE_MARKER;
                }
            }
        }

        let row_str: String = row_chars.iter().collect();
        out.push(format!("{} │{}", y_label, row_str));
    }

    // X-axis baseline
    out.push(format!(
        "{} └{}",
        " ".repeat(y_label_width),
        "─".repeat(chart_width)
    ));

    // Category labels
    let mut label_line = format!("{}  ", " ".repeat(y_label_width));
    for cat in &categories {
        label_line.push_str(&format!("{:^width$}", cat, width = COL_WIDTH));
    }
    out.push(label_line);

    // X-axis title
    if let Some(XAxisData::Band(ref band)) = db.x_axis {
        if !band.title.is_empty() {
            out.push(format!("{}  {}", " ".repeat(y_label_width), band.title));
        }
    }

    // Legend (only if multiple plot types)
    let has_bars = plots.iter().any(|p| p.plot_type == PlotType::Bar);
    let has_lines = plots.iter().any(|p| p.plot_type == PlotType::Line);
    if has_bars && has_lines {
        out.push(format!(
            "{}  {} Bar  {} Line",
            " ".repeat(y_label_width),
            FULL_BLOCK,
            LINE_MARKER
        ));
    }

    out.push(String::new());
    Ok(out.join("\n"))
}

/// Compute nice Y-axis tick values.
fn compute_y_ticks(max_value: f64, desired_ticks: usize) -> Vec<f64> {
    let raw_step = max_value / desired_ticks as f64;
    let magnitude = 10.0f64.powf(raw_step.log10().floor());
    let residual = raw_step / magnitude;
    let nice_step = if residual <= 1.5 {
        magnitude
    } else if residual <= 3.5 {
        2.0 * magnitude
    } else if residual <= 7.5 {
        5.0 * magnitude
    } else {
        10.0 * magnitude
    };

    let mut ticks = Vec::new();
    let mut v = nice_step;
    while v <= max_value + nice_step * 0.01 {
        ticks.push(v);
        v += nice_step;
    }
    // Ensure we have a tick at or above max_value
    if ticks.is_empty() || *ticks.last().unwrap() < max_value {
        ticks.push(max_value);
    }
    ticks
}

/// Format a numeric value for axis labels.
fn format_value(v: f64) -> String {
    if v.fract() == 0.0 {
        format!("{}", v as i64)
    } else {
        format!("{:.1}", v)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_chart() {
        let db = XYChartDb::new();
        let output = render_xychart_ascii(&db).unwrap();
        assert!(output.contains("empty chart"));
    }

    #[test]
    fn gallery_xychart_renders() {
        let input = std::fs::read_to_string("docs/sources/xychart.mmd").unwrap();
        let diagram = crate::parse(&input).unwrap();
        let db = match diagram {
            crate::diagrams::Diagram::XyChart(db) => db,
            _ => panic!("Expected xychart"),
        };
        let output = render_xychart_ascii(&db).unwrap();
        assert!(output.contains("Monthly Sales"), "Output:\n{}", output);
        assert!(output.contains("Jan"), "Output:\n{}", output);
        assert!(output.contains("Jun"), "Output:\n{}", output);
    }

    #[test]
    fn bars_have_block_chars() {
        let input = std::fs::read_to_string("docs/sources/xychart.mmd").unwrap();
        let diagram = crate::parse(&input).unwrap();
        let db = match diagram {
            crate::diagrams::Diagram::XyChart(db) => db,
            _ => panic!("Expected xychart"),
        };
        let output = render_xychart_ascii(&db).unwrap();
        assert!(
            output.contains(FULL_BLOCK),
            "Should have bar blocks\nOutput:\n{}",
            output
        );
    }

    #[test]
    fn bars_and_lines_unified_not_separate() {
        // When a chart has both bar and line plots, they should be rendered
        // on a single unified grid, NOT as separate sections with [Bar 1] / [Line 1] headers.
        let input = std::fs::read_to_string("docs/sources/xychart.mmd").unwrap();
        let diagram = crate::parse(&input).unwrap();
        let db = match diagram {
            crate::diagrams::Diagram::XyChart(db) => db,
            _ => panic!("Expected xychart"),
        };
        let output = render_xychart_ascii(&db).unwrap();

        // Should NOT have separate section headers for each plot
        assert!(
            !output.contains("[Bar"),
            "Should not have separate [Bar] section headers in unified chart\nOutput:\n{}",
            output
        );
        assert!(
            !output.contains("[Line"),
            "Should not have separate [Line] section headers in unified chart\nOutput:\n{}",
            output
        );

        // Each category (Jan..Jun) should appear exactly once, not duplicated per plot
        let jan_count = output.matches("Jan").count();
        assert_eq!(
            jan_count, 1,
            "Category 'Jan' should appear exactly once in unified chart, found {}\nOutput:\n{}",
            jan_count, output
        );

        // Should have both bar blocks and line markers in the output
        assert!(
            output.contains('█') || output.contains('▌'),
            "Unified chart should contain bar blocks\nOutput:\n{}",
            output
        );
        assert!(
            output.contains('●'),
            "Unified chart should contain line markers\nOutput:\n{}",
            output
        );
    }

    #[test]
    fn x_axis_title_rendered() {
        use crate::diagrams::xychart::DataPoint;
        // When the x-axis has a title, it should appear in the output.
        let mut db = XYChartDb::new();
        db.title = "Test Chart".to_string();
        db.set_x_axis_band("Months", vec!["A".to_string(), "B".to_string()]);
        db.add_bar_plot(vec![
            DataPoint {
                label: "A".to_string(),
                value: 10.0,
            },
            DataPoint {
                label: "B".to_string(),
                value: 20.0,
            },
        ]);
        let output = render_xychart_ascii(&db).unwrap();
        assert!(
            output.contains("Months"),
            "X-axis title 'Months' should appear in chart output\nOutput:\n{}",
            output
        );
    }

    #[test]
    fn multiple_line_plots_all_rendered() {
        // When a chart has multiple line plots, all should have markers in the output.
        use crate::diagrams::xychart::DataPoint;
        let mut db = XYChartDb::new();
        db.title = "Multi-Line".to_string();
        db.set_x_axis_band("", vec!["A".to_string(), "B".to_string(), "C".to_string()]);
        db.add_bar_plot(vec![
            DataPoint {
                label: "A".to_string(),
                value: 30.0,
            },
            DataPoint {
                label: "B".to_string(),
                value: 50.0,
            },
            DataPoint {
                label: "C".to_string(),
                value: 40.0,
            },
        ]);
        // Line 1: high values
        db.add_line_plot(vec![
            DataPoint {
                label: "A".to_string(),
                value: 45.0,
            },
            DataPoint {
                label: "B".to_string(),
                value: 50.0,
            },
            DataPoint {
                label: "C".to_string(),
                value: 48.0,
            },
        ]);
        // Line 2: low values
        db.add_line_plot(vec![
            DataPoint {
                label: "A".to_string(),
                value: 10.0,
            },
            DataPoint {
                label: "B".to_string(),
                value: 15.0,
            },
            DataPoint {
                label: "C".to_string(),
                value: 12.0,
            },
        ]);
        let output = render_xychart_ascii(&db).unwrap();

        // Count line markers — should have at least 6 (3 categories x 2 lines)
        let marker_count = output.matches(LINE_MARKER).count();
        assert!(
            marker_count >= 6,
            "Expected at least 6 line markers for 2 lines x 3 categories, got {}\nOutput:\n{}",
            marker_count,
            output
        );
    }

    #[test]
    fn complex_xychart_renders_all_lines() {
        // The complex xychart has 1 bar + 2 lines; all should appear.
        let input = std::fs::read_to_string("docs/sources/xychart_complex.mmd").unwrap();
        let diagram = crate::parse(&input).unwrap();
        let db = match diagram {
            crate::diagrams::Diagram::XyChart(db) => db,
            _ => panic!("Expected xychart"),
        };
        let output = render_xychart_ascii(&db).unwrap();

        // 7 categories, 2 lines = at least 14 markers
        let marker_count = output.matches(LINE_MARKER).count();
        assert!(
            marker_count >= 14,
            "Expected at least 14 line markers for 2 lines x 7 categories, got {}\nOutput:\n{}",
            marker_count,
            output
        );
    }

    #[test]
    fn unified_chart_is_vertical_grid() {
        // The unified chart should render as a vertical grid (bars going up)
        // with Y-axis scale on the left and category labels at the bottom.
        let input = std::fs::read_to_string("docs/sources/xychart.mmd").unwrap();
        let diagram = crate::parse(&input).unwrap();
        let db = match diagram {
            crate::diagrams::Diagram::XyChart(db) => db,
            _ => panic!("Expected xychart"),
        };
        let output = render_xychart_ascii(&db).unwrap();

        // Should have category labels along the bottom (on the x-axis line)
        // and numeric Y-axis values along the left side
        let lines: Vec<&str> = output.lines().collect();
        assert!(
            lines.len() >= 10,
            "Vertical chart should have enough rows for the grid\nOutput:\n{}",
            output
        );
    }
}
