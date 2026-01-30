//! TUI renderer for XY chart diagrams.
//!
//! Renders bar charts as horizontal bars (like pie) and line charts
//! as sparkline-style output, with axis labels and data values.

use crate::diagrams::xychart::{PlotType, XAxisData, XYChartDb, YAxisData};
use crate::error::Result;

const BAR_WIDTH: usize = 40;
const FULL_BLOCK: char = '█';
const HALF_BLOCK: char = '▌';

/// Render an XY chart as character art.
pub fn render_xychart_tui(db: &XYChartDb) -> Result<String> {
    let plots = db.get_plots();
    if plots.is_empty() {
        let title = &db.title;
        if !title.is_empty() {
            return Ok(format!("{}\n\n(empty chart)\n", title));
        }
        return Ok("(empty chart)\n".to_string());
    }

    let mut lines: Vec<String> = Vec::new();

    // Title
    if !db.title.is_empty() {
        lines.push(db.title.clone());
        lines.push("─".repeat(db.title.chars().count().max(BAR_WIDTH)));
    }

    // Get categories from x-axis
    let categories: Vec<String> = match &db.x_axis {
        Some(XAxisData::Band(band)) => band.categories.clone(),
        _ => {
            // For linear x-axis, use data point labels
            plots
                .first()
                .map(|p| p.data.iter().map(|d| d.label.clone()).collect())
                .unwrap_or_default()
        }
    };

    // Find global max value across all plots for scaling
    let max_value = plots
        .iter()
        .flat_map(|p| p.data.iter().map(|d| d.value))
        .fold(0.0f64, f64::max);

    if max_value <= 0.0 {
        lines.push("(no data)".to_string());
        lines.push(String::new());
        return Ok(lines.join("\n"));
    }

    // Y-axis label
    if let Some(ref y_axis) = db.y_axis {
        let y_title = match y_axis {
            YAxisData::Linear(data) => &data.title,
        };
        if !y_title.is_empty() {
            lines.push(format!("  Y: {}", y_title));
        }
    }

    let max_cat_len = categories
        .iter()
        .map(|c| c.chars().count())
        .max()
        .unwrap_or(5);

    // Render each plot
    for (pi, plot) in plots.iter().enumerate() {
        let plot_label = match plot.plot_type {
            PlotType::Bar => format!("Bar {}", pi + 1),
            PlotType::Line => format!("Line {}", pi + 1),
        };
        if plots.len() > 1 {
            lines.push(String::new());
            lines.push(format!("  [{}]", plot_label));
        }

        for (i, dp) in plot.data.iter().enumerate() {
            let cat = categories.get(i).map(|s| s.as_str()).unwrap_or("?");

            let pct = dp.value / max_value;
            let bar_cells = pct * BAR_WIDTH as f64;
            let full_cells = bar_cells.floor() as usize;
            let has_half = (bar_cells - full_cells as f64) >= 0.5;

            let marker = match plot.plot_type {
                PlotType::Bar => {
                    let mut bar = String::new();
                    for _ in 0..full_cells {
                        bar.push(FULL_BLOCK);
                    }
                    if has_half {
                        bar.push(HALF_BLOCK);
                    }
                    if bar.is_empty() && dp.value > 0.0 {
                        bar.push(HALF_BLOCK);
                    }
                    bar
                }
                PlotType::Line => {
                    let pos = (pct * BAR_WIDTH as f64).round() as usize;
                    let mut marker = " ".repeat(pos.min(BAR_WIDTH));
                    marker.push('●');
                    marker
                }
            };

            let value_str = if dp.value.fract() == 0.0 {
                format!("{}", dp.value as i64)
            } else {
                format!("{:.1}", dp.value)
            };

            lines.push(format!(
                "  {:width$} │{} {}",
                cat,
                marker,
                value_str,
                width = max_cat_len,
            ));
        }
    }

    // X-axis label
    if let Some(XAxisData::Band(ref band)) = db.x_axis {
        if !band.title.is_empty() {
            lines.push(String::new());
            lines.push(format!("  X: {}", band.title));
        }
    }

    lines.push(String::new());
    Ok(lines.join("\n"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_chart() {
        let db = XYChartDb::new();
        let output = render_xychart_tui(&db).unwrap();
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
        let output = render_xychart_tui(&db).unwrap();
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
        let output = render_xychart_tui(&db).unwrap();
        assert!(
            output.contains(FULL_BLOCK),
            "Should have bar blocks\nOutput:\n{}",
            output
        );
    }
}
