//! ASCII renderer for radar/spider chart diagrams.
//!
//! Renders radar charts as a true radial shape using braille characters
//! for the chart body (graticule, axes, curves), with text labels placed
//! around the perimeter and a legend below.

use std::f64::consts::PI;

use crate::diagrams::radar::{Graticule, RadarDb};
use crate::error::Result;

use super::canvas::BrailleCanvas;

/// Chart radius in character cell columns.
const CHART_CELL_RADIUS: usize = 12;

/// How far outside the chart perimeter to place axis labels
/// (as a fraction of the radius in cell coordinates).
const LABEL_FACTOR: f64 = 1.15;

/// Number of line segments used to approximate a circle.
const CIRCLE_SEGMENTS: usize = 64;

/// Markers to visually differentiate curves in monochrome output.
const CURVE_MARKERS: &[char] = &['●', '◆', '■', '▲', '★', '◉', '▶', '◈'];

/// Render a radar chart as character art with a radial layout.
pub fn render_radar_ascii(db: &RadarDb) -> Result<String> {
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

    let n = axes.len();
    let max_val = options.max.unwrap_or_else(|| {
        curves
            .iter()
            .flat_map(|c| c.entries.iter().copied())
            .fold(0.0f64, f64::max)
    });
    let min_val = options.min;

    // Compute axis angles: start from top (-π/2), go clockwise.
    let angles: Vec<f64> = (0..n)
        .map(|i| -PI / 2.0 + (i as f64) * 2.0 * PI / (n as f64))
        .collect();

    // ── Braille canvas ──────────────────────────────────────
    // Terminal characters are ~2× taller than wide. Braille encodes 2×4
    // dots per cell, making braille "pixels" physically square. To render
    // a circular chart we use cell_rows ≈ cell_cols/2.
    let chart_cols = 2 * CHART_CELL_RADIUS + 1;
    let chart_rows = CHART_CELL_RADIUS + 1;
    let mut canvas = BrailleCanvas::new(chart_cols, chart_rows);

    let px_cx = canvas.pixel_width() as f64 / 2.0;
    let px_cy = canvas.pixel_height() as f64 / 2.0;
    let px_r = (CHART_CELL_RADIUS * 2) as f64;

    // Draw graticule (concentric rings)
    for t in 1..=options.ticks {
        let frac = t as f64 / options.ticks as f64;
        let ring_r = px_r * frac;
        match options.graticule {
            Graticule::Circle => draw_circle(&mut canvas, px_cx, px_cy, ring_r),
            Graticule::Polygon => draw_polygon(&mut canvas, px_cx, px_cy, ring_r, &angles),
        }
    }

    // Draw axis spokes (from center to perimeter)
    for angle in &angles {
        let ex = px_cx + px_r * angle.cos();
        let ey = px_cy + px_r * angle.sin();
        canvas.draw_line(px_cx as isize, px_cy as isize, ex as isize, ey as isize);
    }

    // Compute curve data points in pixel coordinates
    let curve_points: Vec<Vec<(f64, f64)>> = curves
        .iter()
        .map(|curve| {
            (0..n)
                .map(|i| {
                    let val = curve.entries.get(i).copied().unwrap_or(0.0);
                    let frac = relative_radius(val, min_val, max_val);
                    let x = px_cx + px_r * frac * angles[i].cos();
                    let y = px_cy + px_r * frac * angles[i].sin();
                    (x, y)
                })
                .collect()
        })
        .collect();

    // Draw curve polygons on braille canvas
    for points in &curve_points {
        let len = points.len();
        for j in 0..len {
            let k = (j + 1) % len;
            canvas.draw_line(
                points[j].0 as isize,
                points[j].1 as isize,
                points[k].0 as isize,
                points[k].1 as isize,
            );
        }
    }

    // ── Compose text buffer ─────────────────────────────────
    let braille_grid = canvas.to_char_grid();

    let axis_labels: Vec<&str> = axes
        .iter()
        .map(|a| {
            if !a.label.is_empty() {
                a.label.as_str()
            } else {
                a.name.as_str()
            }
        })
        .collect();

    let max_label_len = axis_labels
        .iter()
        .map(|l| l.chars().count())
        .max()
        .unwrap_or(0);

    // Buffer margins (enough room for labels on all sides)
    let margin_x = max_label_len + 3;
    let margin_top: usize = 2;
    let margin_bottom: usize = 2;

    let buf_cols = margin_x + chart_cols + margin_x;
    let buf_rows = margin_top + chart_rows + margin_bottom;

    let mut buf: Vec<Vec<char>> = vec![vec![' '; buf_cols]; buf_rows];

    // Place braille grid in center of buffer
    for (row, braille_row) in braille_grid.iter().enumerate() {
        for (col, &ch) in braille_row.iter().enumerate() {
            let br = margin_top + row;
            let bc = margin_x + col;
            if br < buf_rows && bc < buf_cols {
                buf[br][bc] = ch;
            }
        }
    }

    // Place axis labels around the perimeter
    let center_buf_col = margin_x + chart_cols / 2;
    let center_buf_row = margin_top + chart_rows / 2;

    // Radii in buffer cell coordinates
    let cell_r_x = CHART_CELL_RADIUS as f64;
    let cell_r_y = CHART_CELL_RADIUS as f64 / 2.0;

    for (i, label) in axis_labels.iter().enumerate() {
        let theta = angles[i];
        let lx = center_buf_col as f64 + LABEL_FACTOR * cell_r_x * theta.cos();
        let ly = center_buf_row as f64 + LABEL_FACTOR * cell_r_y * theta.sin();

        let label_len = label.chars().count();
        let cos_t = theta.cos();

        let start_col: isize = if cos_t > 0.3 {
            // Right half: left-align starting just after the position
            lx.round() as isize + 1
        } else if cos_t < -0.3 {
            // Left half: right-align ending just before the position
            lx.round() as isize - label_len as isize
        } else {
            // Top/bottom: center
            lx.round() as isize - (label_len as isize) / 2
        };

        let row = ly.round().max(0.0) as usize;

        for (j, ch) in label.chars().enumerate() {
            let col = start_col + j as isize;
            if col >= 0 && (col as usize) < buf_cols && row < buf_rows {
                buf[row][col as usize] = ch;
            }
        }
    }

    // Place curve data point markers in the text buffer
    for (ci, points) in curve_points.iter().enumerate() {
        let marker = CURVE_MARKERS[ci % CURVE_MARKERS.len()];
        for &(px, py) in points {
            let col = margin_x + (px.round() as usize / 2);
            let row = margin_top + (py.round() as usize / 4);
            if row < buf_rows && col < buf_cols {
                buf[row][col] = marker;
            }
        }
    }

    // ── Build output ────────────────────────────────────────
    let mut lines: Vec<String> = Vec::new();

    // Title
    let title = db.get_title();
    if !title.is_empty() {
        lines.push(title.to_string());
        lines.push("─".repeat(title.chars().count().max(40)));
        lines.push(String::new());
    }

    // Chart
    for row in &buf {
        let line: String = row.iter().collect();
        lines.push(line.trim_end().to_string());
    }

    // Legend
    if options.show_legend && !curves.is_empty() {
        lines.push(String::new());
        let legend_parts: Vec<String> = curves
            .iter()
            .enumerate()
            .map(|(i, c)| {
                let marker = CURVE_MARKERS[i % CURVE_MARKERS.len()];
                let label = if !c.label.is_empty() {
                    &c.label
                } else {
                    &c.name
                };
                format!("{} {}", marker, label)
            })
            .collect();
        lines.push(format!("  Legend: {}", legend_parts.join("  ")));
    }

    lines.push(String::new());
    Ok(lines.join("\n"))
}

/// Calculate relative radius (0.0 to 1.0) for a value within min/max range.
fn relative_radius(value: f64, min_value: f64, max_value: f64) -> f64 {
    let clipped = value.clamp(min_value, max_value);
    if (max_value - min_value).abs() < f64::EPSILON {
        return 1.0;
    }
    (clipped - min_value) / (max_value - min_value)
}

/// Draw a circle approximated by line segments on the braille canvas.
fn draw_circle(canvas: &mut BrailleCanvas, cx: f64, cy: f64, r: f64) {
    for i in 0..CIRCLE_SEGMENTS {
        let t0 = 2.0 * PI * i as f64 / CIRCLE_SEGMENTS as f64;
        let t1 = 2.0 * PI * (i + 1) as f64 / CIRCLE_SEGMENTS as f64;
        canvas.draw_line(
            (cx + r * t0.cos()) as isize,
            (cy + r * t0.sin()) as isize,
            (cx + r * t1.cos()) as isize,
            (cy + r * t1.sin()) as isize,
        );
    }
}

/// Draw a polygon connecting axis positions at a given radius.
fn draw_polygon(canvas: &mut BrailleCanvas, cx: f64, cy: f64, r: f64, angles: &[f64]) {
    let n = angles.len();
    for i in 0..n {
        let j = (i + 1) % n;
        canvas.draw_line(
            (cx + r * angles[i].cos()) as isize,
            (cy + r * angles[i].sin()) as isize,
            (cx + r * angles[j].cos()) as isize,
            (cy + r * angles[j].sin()) as isize,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_radar() {
        let db = RadarDb::new();
        let output = render_radar_ascii(&db).unwrap();
        assert!(output.contains("empty radar"));
    }

    #[test]
    fn renders_radial_shape_not_bars() {
        let input = std::fs::read_to_string("docs/sources/radar.mmd").unwrap();
        let diagram = crate::parse(&input).unwrap();
        let db = match diagram {
            crate::diagrams::Diagram::Radar(db) => db,
            _ => panic!("Expected radar"),
        };
        let output = render_radar_ascii(&db).unwrap();

        // Should use braille characters for radar shape (radial layout)
        let has_braille = output
            .chars()
            .any(|c| ('\u{2800}'..='\u{28FF}').contains(&c));
        assert!(
            has_braille,
            "Radar chart should use braille characters for radial shape\nOutput:\n{}",
            output
        );

        // Should NOT render as a bar chart
        assert!(
            !output.contains('█'),
            "Should not render as bar chart\nOutput:\n{}",
            output
        );
        assert!(
            !output.contains('░'),
            "Should not render as bar chart\nOutput:\n{}",
            output
        );
    }

    #[test]
    fn gallery_radar_renders() {
        let input = std::fs::read_to_string("docs/sources/radar.mmd").unwrap();
        let diagram = crate::parse(&input).unwrap();
        let db = match diagram {
            crate::diagrams::Diagram::Radar(db) => db,
            _ => panic!("Expected radar"),
        };
        let output = render_radar_ascii(&db).unwrap();
        assert!(output.contains("Skills Assessment"), "Output:\n{}", output);
        assert!(output.contains("Coding"), "Output:\n{}", output);
        assert!(output.contains("Testing"), "Output:\n{}", output);
        assert!(output.contains("Design"), "Output:\n{}", output);
        assert!(output.contains("Code Review"), "Output:\n{}", output);
        assert!(output.contains("Documentation"), "Output:\n{}", output);
    }

    #[test]
    fn curves_appear_in_legend() {
        let input = std::fs::read_to_string("docs/sources/radar.mmd").unwrap();
        let diagram = crate::parse(&input).unwrap();
        let db = match diagram {
            crate::diagrams::Diagram::Radar(db) => db,
            _ => panic!("Expected radar"),
        };
        let output = render_radar_ascii(&db).unwrap();
        assert!(
            output.contains("Team Alpha"),
            "Should show curve name\nOutput:\n{}",
            output
        );
        assert!(
            output.contains("Team Beta"),
            "Should show curve name\nOutput:\n{}",
            output
        );
        assert!(
            output.contains("Legend"),
            "Should have legend section\nOutput:\n{}",
            output
        );
    }

    #[test]
    fn has_data_point_markers() {
        let input = std::fs::read_to_string("docs/sources/radar.mmd").unwrap();
        let diagram = crate::parse(&input).unwrap();
        let db = match diagram {
            crate::diagrams::Diagram::Radar(db) => db,
            _ => panic!("Expected radar"),
        };
        let output = render_radar_ascii(&db).unwrap();
        assert!(
            output.contains('●'),
            "Should have data point markers for first curve\nOutput:\n{}",
            output
        );
        assert!(
            output.contains('◆'),
            "Should have data point markers for second curve\nOutput:\n{}",
            output
        );
    }

    #[test]
    fn complex_radar_renders() {
        let input = std::fs::read_to_string("docs/sources/radar_complex.mmd").unwrap();
        let diagram = crate::parse(&input).unwrap();
        let db = match diagram {
            crate::diagrams::Diagram::Radar(db) => db,
            _ => panic!("Expected radar"),
        };
        let output = render_radar_ascii(&db).unwrap();
        assert!(
            output.contains("Programming Language Comparison"),
            "Should show title\nOutput:\n{}",
            output
        );
        assert!(
            output.contains("Performance"),
            "Should have axis label\nOutput:\n{}",
            output
        );
        assert!(
            output.contains("Rust"),
            "Should show curve\nOutput:\n{}",
            output
        );
        assert!(
            output.contains("Python"),
            "Should show curve\nOutput:\n{}",
            output
        );
    }
}
