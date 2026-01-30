//! TUI renderer for radar/spider chart diagrams.
//!
//! Since radial charts don't render well in character art, radar charts
//! are displayed as a comparison table with bar indicators for each axis value.

use crate::diagrams::radar::RadarDb;
use crate::error::Result;

const BAR_WIDTH: usize = 20;
const FULL_BLOCK: char = '█';
const EMPTY_BLOCK: char = '░';

/// Render a radar chart as character art.
pub fn render_radar_tui(db: &RadarDb) -> Result<String> {
    let axes = db.get_axes();
    let curves = db.get_curves();
    let options = db.get_options();

    if axes.is_empty() || curves.is_empty() {
        let title = db.get_title();
        if !title.is_empty() {
            return Ok(format!("{}\n\n(empty radar chart)\n", title));
        }
        return Ok("(empty radar chart)\n".to_string());
    }

    let mut lines: Vec<String> = Vec::new();

    // Title
    let title = db.get_title();
    if !title.is_empty() {
        lines.push(title.to_string());
        lines.push("─".repeat(title.chars().count().max(40)));
    }

    // max is Option<f64>; fall back to max value found in data
    let max_val = options.max.unwrap_or_else(|| {
        curves
            .iter()
            .flat_map(|c| c.entries.iter().copied())
            .fold(0.0f64, f64::max)
    });

    // Find max axis label width
    let max_axis_len = axes
        .iter()
        .map(|a| {
            let label = if !a.label.is_empty() {
                &a.label
            } else {
                &a.name
            };
            label.chars().count()
        })
        .max()
        .unwrap_or(5);

    // Render each curve
    for curve in curves {
        let label = if !curve.label.is_empty() {
            &curve.label
        } else {
            &curve.name
        };
        lines.push(String::new());
        lines.push(format!("  ◆ {}", label));
        lines.push(format!("  {}", "─".repeat(label.chars().count() + 2)));

        for (i, axis) in axes.iter().enumerate() {
            let axis_label = if !axis.label.is_empty() {
                &axis.label
            } else {
                &axis.name
            };

            let value = curve.entries.get(i).copied().unwrap_or(0.0);
            let pct = if max_val > 0.0 {
                (value / max_val).min(1.0)
            } else {
                0.0
            };

            let filled = (pct * BAR_WIDTH as f64).round() as usize;
            let empty = BAR_WIDTH.saturating_sub(filled);
            let bar: String = std::iter::repeat_n(FULL_BLOCK, filled)
                .chain(std::iter::repeat_n(EMPTY_BLOCK, empty))
                .collect();

            let value_str = if value.fract() == 0.0 {
                format!("{}", value as i64)
            } else {
                format!("{:.1}", value)
            };

            let max_str = if max_val.fract() == 0.0 {
                format!("{}", max_val as i64)
            } else {
                format!("{:.1}", max_val)
            };

            lines.push(format!(
                "    {:width$} │{} {}/{}",
                axis_label,
                bar,
                value_str,
                max_str,
                width = max_axis_len,
            ));
        }
    }

    lines.push(String::new());
    Ok(lines.join("\n"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_radar() {
        let db = RadarDb::new();
        let output = render_radar_tui(&db).unwrap();
        assert!(output.contains("empty radar"));
    }

    #[test]
    fn gallery_radar_renders() {
        let input = std::fs::read_to_string("docs/sources/radar.mmd").unwrap();
        let diagram = crate::parse(&input).unwrap();
        let db = match diagram {
            crate::diagrams::Diagram::Radar(db) => db,
            _ => panic!("Expected radar"),
        };
        let output = render_radar_tui(&db).unwrap();
        assert!(output.contains("Skills Assessment"), "Output:\n{}", output);
        assert!(output.contains("Coding"), "Output:\n{}", output);
        assert!(output.contains("Testing"), "Output:\n{}", output);
    }

    #[test]
    fn curves_appear() {
        let input = std::fs::read_to_string("docs/sources/radar.mmd").unwrap();
        let diagram = crate::parse(&input).unwrap();
        let db = match diagram {
            crate::diagrams::Diagram::Radar(db) => db,
            _ => panic!("Expected radar"),
        };
        let output = render_radar_tui(&db).unwrap();
        assert!(output.contains("Team Alpha"), "Output:\n{}", output);
        assert!(output.contains("Team Beta"), "Output:\n{}", output);
    }

    #[test]
    fn has_bar_chars() {
        let input = std::fs::read_to_string("docs/sources/radar.mmd").unwrap();
        let diagram = crate::parse(&input).unwrap();
        let db = match diagram {
            crate::diagrams::Diagram::Radar(db) => db,
            _ => panic!("Expected radar"),
        };
        let output = render_radar_tui(&db).unwrap();
        assert!(output.contains(FULL_BLOCK), "Output:\n{}", output);
    }
}
