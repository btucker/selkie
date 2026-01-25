//! Quadrant chart renderer
//!
//! Renders quadrant charts that divide data into four quadrants based on two axes,
//! commonly used for prioritization matrices like the Gartner Magic Quadrant.

use crate::diagrams::quadrant::QuadrantDb;
use crate::error::Result;
use crate::render::svg::{Attrs, RenderConfig, SvgDocument, SvgElement};

/// Default chart dimensions (matching mermaid.js defaults)
const DEFAULT_WIDTH: f64 = 500.0;
const DEFAULT_HEIGHT: f64 = 500.0;

/// Mermaid.js quadrant chart defaults from defaultConfig.js
const QUADRANT_PADDING: f64 = 5.0;
const TITLE_PADDING: f64 = 10.0;
const X_AXIS_LABEL_PADDING: f64 = 5.0;
const Y_AXIS_LABEL_PADDING: f64 = 5.0;

/// Default point radius
const DEFAULT_POINT_RADIUS: f64 = 5.0;

/// Default font sizes (matching mermaid.js)
const TITLE_FONT_SIZE: f64 = 20.0;
const QUADRANT_LABEL_FONT_SIZE: f64 = 16.0;
const X_AXIS_LABEL_FONT_SIZE: f64 = 16.0;
const Y_AXIS_LABEL_FONT_SIZE: f64 = 16.0;
const POINT_LABEL_FONT_SIZE: f64 = 12.0;

/// Padding for quadrant text when points exist (text moves to top)
const QUADRANT_TEXT_TOP_PADDING: f64 = 5.0;
/// Padding between point and its label
const POINT_TEXT_PADDING: f64 = 5.0;

/// Render a quadrant chart to SVG
pub fn render_quadrant(db: &QuadrantDb, config: &RenderConfig) -> Result<String> {
    let mut doc = SvgDocument::new();

    let width = DEFAULT_WIDTH;
    let height = DEFAULT_HEIGHT;
    doc.set_size(width, height);

    // Add theme styles
    if config.embed_css {
        doc.add_style(&config.theme.generate_css());
        doc.add_style(&generate_quadrant_css(&config.theme));
    }

    // Check if there are any points (affects axis position and layout)
    let has_points = !db.get_points().is_empty();

    // Calculate layout areas matching mermaid.js quadrantBuilder.ts
    // X-axis position: bottom when points exist, top when no points
    let x_axis_at_bottom = has_points;

    // Axis label space calculation: padding * 2 + fontSize
    let x_axis_space_calculation = X_AXIS_LABEL_PADDING * 2.0 + X_AXIS_LABEL_FONT_SIZE;
    let y_axis_space_calculation = Y_AXIS_LABEL_PADDING * 2.0 + Y_AXIS_LABEL_FONT_SIZE;

    // Show axes if labels are defined
    let show_x_axis = !db.x_axis_left.is_empty() || !db.x_axis_right.is_empty();
    let show_y_axis = !db.y_axis_bottom.is_empty() || !db.y_axis_top.is_empty();
    let show_title = !db.title.is_empty();

    // Calculate space allocations
    let x_axis_space_top = if !x_axis_at_bottom && show_x_axis {
        x_axis_space_calculation
    } else {
        0.0
    };
    let x_axis_space_bottom = if x_axis_at_bottom && show_x_axis {
        x_axis_space_calculation
    } else {
        0.0
    };
    let y_axis_space_left = if show_y_axis {
        y_axis_space_calculation
    } else {
        0.0
    };
    let title_space_top = if show_title {
        TITLE_FONT_SIZE + TITLE_PADDING * 2.0
    } else {
        0.0
    };

    // Quadrant area calculation (matches mermaid.js)
    let quadrant_left = QUADRANT_PADDING + y_axis_space_left;
    let quadrant_top = QUADRANT_PADDING + x_axis_space_top + title_space_top;
    let quadrant_width = width - QUADRANT_PADDING * 2.0 - y_axis_space_left; // Only left y-axis
    let quadrant_height =
        height - QUADRANT_PADDING * 2.0 - x_axis_space_top - x_axis_space_bottom - title_space_top;

    // Alias for compatibility with existing code
    let chart_left = quadrant_left;
    let chart_top = quadrant_top;
    let chart_width = quadrant_width;
    let chart_height = quadrant_height;
    let chart_right = chart_left + chart_width;
    let chart_bottom = chart_top + chart_height;

    // Quadrant dimensions (each quadrant is half the chart)
    let quadrant_width = chart_width / 2.0;
    let quadrant_height = chart_height / 2.0;

    // Note: No background rect - matching mermaid.js reference implementation
    // The SVG background is controlled by the container or CSS

    // Render the four quadrants
    // Quadrant 2 (top-left)
    doc.add_element(SvgElement::Rect {
        x: chart_left,
        y: chart_top,
        width: quadrant_width,
        height: quadrant_height,
        rx: None,
        ry: None,
        attrs: Attrs::new()
            .with_fill(&config.theme.quadrant2_fill)
            .with_class("quadrant quadrant-2"),
    });

    // Quadrant 1 (top-right)
    doc.add_element(SvgElement::Rect {
        x: chart_left + quadrant_width,
        y: chart_top,
        width: quadrant_width,
        height: quadrant_height,
        rx: None,
        ry: None,
        attrs: Attrs::new()
            .with_fill(&config.theme.quadrant1_fill)
            .with_class("quadrant quadrant-1"),
    });

    // Quadrant 3 (bottom-left)
    doc.add_element(SvgElement::Rect {
        x: chart_left,
        y: chart_top + quadrant_height,
        width: quadrant_width,
        height: quadrant_height,
        rx: None,
        ry: None,
        attrs: Attrs::new()
            .with_fill(&config.theme.quadrant3_fill)
            .with_class("quadrant quadrant-3"),
    });

    // Quadrant 4 (bottom-right)
    doc.add_element(SvgElement::Rect {
        x: chart_left + quadrant_width,
        y: chart_top + quadrant_height,
        width: quadrant_width,
        height: quadrant_height,
        rx: None,
        ry: None,
        attrs: Attrs::new()
            .with_fill(&config.theme.quadrant4_fill)
            .with_class("quadrant quadrant-4"),
    });

    // Render borders using 6 lines (matching mermaid.js reference)
    // External border stroke width
    let ext_border_width = 2.0;
    let half_ext_border = ext_border_width / 2.0;

    // Top border
    doc.add_element(SvgElement::Line {
        x1: chart_left - half_ext_border,
        y1: chart_top,
        x2: chart_right + half_ext_border,
        y2: chart_top,
        attrs: Attrs::new()
            .with_stroke(&config.theme.quadrant_external_border_stroke)
            .with_stroke_width(ext_border_width)
            .with_class("quadrant-border quadrant-border-top"),
    });

    // Right border
    doc.add_element(SvgElement::Line {
        x1: chart_right,
        y1: chart_top + half_ext_border,
        x2: chart_right,
        y2: chart_bottom - half_ext_border,
        attrs: Attrs::new()
            .with_stroke(&config.theme.quadrant_external_border_stroke)
            .with_stroke_width(ext_border_width)
            .with_class("quadrant-border quadrant-border-right"),
    });

    // Bottom border
    doc.add_element(SvgElement::Line {
        x1: chart_left - half_ext_border,
        y1: chart_bottom,
        x2: chart_right + half_ext_border,
        y2: chart_bottom,
        attrs: Attrs::new()
            .with_stroke(&config.theme.quadrant_external_border_stroke)
            .with_stroke_width(ext_border_width)
            .with_class("quadrant-border quadrant-border-bottom"),
    });

    // Left border
    doc.add_element(SvgElement::Line {
        x1: chart_left,
        y1: chart_top + half_ext_border,
        x2: chart_left,
        y2: chart_bottom - half_ext_border,
        attrs: Attrs::new()
            .with_stroke(&config.theme.quadrant_external_border_stroke)
            .with_stroke_width(ext_border_width)
            .with_class("quadrant-border quadrant-border-left"),
    });

    // Vertical center line (internal divider)
    doc.add_element(SvgElement::Line {
        x1: chart_left + quadrant_width,
        y1: chart_top + half_ext_border,
        x2: chart_left + quadrant_width,
        y2: chart_bottom - half_ext_border,
        attrs: Attrs::new()
            .with_stroke(&config.theme.quadrant_internal_border_stroke)
            .with_stroke_width(1.0)
            .with_class("quadrant-border quadrant-border-vertical"),
    });

    // Horizontal center line (internal divider)
    doc.add_element(SvgElement::Line {
        x1: chart_left + half_ext_border,
        y1: chart_top + quadrant_height,
        x2: chart_right - half_ext_border,
        y2: chart_top + quadrant_height,
        attrs: Attrs::new()
            .with_stroke(&config.theme.quadrant_internal_border_stroke)
            .with_stroke_width(1.0)
            .with_class("quadrant-border quadrant-border-horizontal"),
    });

    // Render quadrant labels (part of quadrants group in reference)
    render_quadrant_labels(
        &mut doc,
        db,
        config,
        chart_left,
        chart_top,
        quadrant_width,
        quadrant_height,
        has_points,
    );

    // Render data points (before axis labels per mermaid.js reference)
    render_points(
        &mut doc,
        db,
        config,
        chart_left,
        chart_top,
        chart_width,
        chart_height,
    );

    // Render axis labels (after data points per mermaid.js reference)
    render_axis_labels(
        &mut doc,
        db,
        config,
        chart_left,
        chart_top,
        chart_width,
        chart_height,
        has_points,
    );

    // Render title last (matching mermaid.js reference - title appears on top)
    if !db.title.is_empty() {
        // Title y position matches mermaid.js: titlePadding (10)
        let title_y = TITLE_PADDING;
        let title_elem = SvgElement::Text {
            x: width / 2.0,
            y: title_y,
            content: db.title.clone(),
            attrs: Attrs::new()
                .with_attr("text-anchor", "middle")
                .with_attr("dominant-baseline", "hanging") // Match mermaid's "top" horizontalPos
                .with_class("quadrant-title")
                .with_attr("font-size", &format!("{}", TITLE_FONT_SIZE))
                .with_attr("font-weight", "bold")
                .with_fill(&config.theme.quadrant_title_fill),
        };
        doc.add_element(title_elem);
    }

    Ok(doc.to_string())
}

/// Render quadrant labels
/// If points exist, labels are placed at the top of each quadrant.
/// If no points, labels are centered in each quadrant.
#[allow(clippy::too_many_arguments)]
fn render_quadrant_labels(
    doc: &mut SvgDocument,
    db: &QuadrantDb,
    config: &RenderConfig,
    chart_left: f64,
    chart_top: f64,
    quadrant_width: f64,
    quadrant_height: f64,
    has_points: bool,
) {
    // Determine Y position and dominant-baseline based on whether there are points
    let (q1_y, q2_y, q3_y, q4_y, dominant_baseline) = if has_points {
        // When points exist, place labels at top of each quadrant
        (
            chart_top + QUADRANT_TEXT_TOP_PADDING,
            chart_top + QUADRANT_TEXT_TOP_PADDING,
            chart_top + quadrant_height + QUADRANT_TEXT_TOP_PADDING,
            chart_top + quadrant_height + QUADRANT_TEXT_TOP_PADDING,
            "hanging",
        )
    } else {
        // When no points, center labels in each quadrant
        (
            chart_top + quadrant_height / 2.0,
            chart_top + quadrant_height / 2.0,
            chart_top + quadrant_height + quadrant_height / 2.0,
            chart_top + quadrant_height + quadrant_height / 2.0,
            "middle",
        )
    };

    // Quadrant 1 (top-right)
    if !db.quadrant1.is_empty() {
        doc.add_element(SvgElement::Text {
            x: chart_left + quadrant_width + quadrant_width / 2.0,
            y: q1_y,
            content: db.quadrant1.clone(),
            attrs: Attrs::new()
                .with_attr("text-anchor", "middle")
                .with_attr("dominant-baseline", dominant_baseline)
                .with_class("quadrant-label")
                .with_attr("font-size", &format!("{}", QUADRANT_LABEL_FONT_SIZE))
                .with_fill(&config.theme.quadrant1_text_fill),
        });
    }

    // Quadrant 2 (top-left)
    if !db.quadrant2.is_empty() {
        doc.add_element(SvgElement::Text {
            x: chart_left + quadrant_width / 2.0,
            y: q2_y,
            content: db.quadrant2.clone(),
            attrs: Attrs::new()
                .with_attr("text-anchor", "middle")
                .with_attr("dominant-baseline", dominant_baseline)
                .with_class("quadrant-label")
                .with_attr("font-size", &format!("{}", QUADRANT_LABEL_FONT_SIZE))
                .with_fill(&config.theme.quadrant2_text_fill),
        });
    }

    // Quadrant 3 (bottom-left)
    if !db.quadrant3.is_empty() {
        doc.add_element(SvgElement::Text {
            x: chart_left + quadrant_width / 2.0,
            y: q3_y,
            content: db.quadrant3.clone(),
            attrs: Attrs::new()
                .with_attr("text-anchor", "middle")
                .with_attr("dominant-baseline", dominant_baseline)
                .with_class("quadrant-label")
                .with_attr("font-size", &format!("{}", QUADRANT_LABEL_FONT_SIZE))
                .with_fill(&config.theme.quadrant3_text_fill),
        });
    }

    // Quadrant 4 (bottom-right)
    if !db.quadrant4.is_empty() {
        doc.add_element(SvgElement::Text {
            x: chart_left + quadrant_width + quadrant_width / 2.0,
            y: q4_y,
            content: db.quadrant4.clone(),
            attrs: Attrs::new()
                .with_attr("text-anchor", "middle")
                .with_attr("dominant-baseline", dominant_baseline)
                .with_class("quadrant-label")
                .with_attr("font-size", &format!("{}", QUADRANT_LABEL_FONT_SIZE))
                .with_fill(&config.theme.quadrant4_text_fill),
        });
    }
}

/// Render axis labels on the edges.
/// X-axis position depends on whether points exist:
/// - No points: x-axis at top (default per mermaid.js)
/// - With points: x-axis at bottom
///
/// When both labels exist for an axis, they are centered in each half.
#[allow(clippy::too_many_arguments)]
fn render_axis_labels(
    doc: &mut SvgDocument,
    db: &QuadrantDb,
    config: &RenderConfig,
    chart_left: f64,
    chart_top: f64,
    chart_width: f64,
    chart_height: f64,
    has_points: bool,
) {
    let half_width = chart_width / 2.0;
    let half_height = chart_height / 2.0;

    // Check if both labels exist for each axis (affects positioning)
    let draw_x_labels_in_middle = !db.x_axis_right.is_empty();
    let draw_y_labels_in_middle = !db.y_axis_top.is_empty();

    // X-axis Y position depends on whether there are points
    // Mermaid.js: xAxisLabelPadding + (titleSpace if xAxisPosition === 'top')
    // or: xAxisLabelPadding + quadrantTop + quadrantHeight + quadrantPadding
    let x_axis_y = if has_points {
        // With points: x-axis at bottom
        // Position: xAxisLabelPadding + quadrantTop + quadrantHeight + quadrantPadding
        chart_top + chart_height + QUADRANT_PADDING + X_AXIS_LABEL_PADDING
    } else {
        // No points: x-axis at top (just below title area)
        // Position: xAxisLabelPadding + titleSpace.top
        X_AXIS_LABEL_PADDING
    };

    // X-axis left label
    if !db.x_axis_left.is_empty() {
        let (x_pos, text_anchor) = if draw_x_labels_in_middle {
            // Center in left half when both labels exist
            (chart_left + half_width / 2.0, "middle")
        } else {
            // Align to left edge when only left label exists
            (chart_left, "start")
        };
        doc.add_element(SvgElement::Text {
            x: x_pos,
            y: x_axis_y,
            content: db.x_axis_left.clone(),
            attrs: Attrs::new()
                .with_attr("text-anchor", text_anchor)
                .with_attr("dominant-baseline", "hanging")
                .with_class("axis-label x-axis-left")
                .with_attr("font-size", &format!("{}", X_AXIS_LABEL_FONT_SIZE))
                .with_fill(&config.theme.quadrant_x_axis_text_fill),
        });
    }

    // X-axis right label
    if !db.x_axis_right.is_empty() {
        let (x_pos, text_anchor) = if draw_x_labels_in_middle {
            // Center in right half when both labels exist
            (chart_left + half_width + half_width / 2.0, "middle")
        } else {
            // Align to right edge when only right label exists
            (chart_left + chart_width, "end")
        };
        doc.add_element(SvgElement::Text {
            x: x_pos,
            y: x_axis_y,
            content: db.x_axis_right.clone(),
            attrs: Attrs::new()
                .with_attr("text-anchor", text_anchor)
                .with_attr("dominant-baseline", "hanging")
                .with_class("axis-label x-axis-right")
                .with_attr("font-size", &format!("{}", X_AXIS_LABEL_FONT_SIZE))
                .with_fill(&config.theme.quadrant_x_axis_text_fill),
        });
    }

    // Y-axis x position matches mermaid.js: yAxisLabelPadding = 5
    let y_axis_x = Y_AXIS_LABEL_PADDING;

    // Y-axis bottom label (left side, at bottom)
    if !db.y_axis_bottom.is_empty() {
        let y_pos = if draw_y_labels_in_middle {
            // Center in bottom half when both labels exist
            chart_top + half_height + half_height / 2.0
        } else {
            // At bottom edge when only bottom label exists
            chart_top + chart_height
        };
        // Mermaid always uses text-anchor="middle" for y-axis labels
        doc.add_element(SvgElement::Text {
            x: y_axis_x,
            y: y_pos,
            content: db.y_axis_bottom.clone(),
            attrs: Attrs::new()
                .with_attr("text-anchor", "middle")
                .with_attr("dominant-baseline", "hanging")
                .with_attr(
                    "transform",
                    &format!("rotate(-90, {}, {})", y_axis_x, y_pos),
                )
                .with_class("axis-label y-axis-bottom")
                .with_attr("font-size", &format!("{}", Y_AXIS_LABEL_FONT_SIZE))
                .with_fill(&config.theme.quadrant_y_axis_text_fill),
        });
    }

    // Y-axis top label (left side, at top)
    if !db.y_axis_top.is_empty() {
        let y_pos = if draw_y_labels_in_middle {
            // Center in top half when both labels exist
            chart_top + half_height / 2.0
        } else {
            // At top edge when only top label exists
            chart_top
        };
        // Mermaid always uses text-anchor="middle" for y-axis labels
        doc.add_element(SvgElement::Text {
            x: y_axis_x,
            y: y_pos,
            content: db.y_axis_top.clone(),
            attrs: Attrs::new()
                .with_attr("text-anchor", "middle")
                .with_attr("dominant-baseline", "hanging")
                .with_attr(
                    "transform",
                    &format!("rotate(-90, {}, {})", y_axis_x, y_pos),
                )
                .with_class("axis-label y-axis-top")
                .with_attr("font-size", &format!("{}", Y_AXIS_LABEL_FONT_SIZE))
                .with_fill(&config.theme.quadrant_y_axis_text_fill),
        });
    }
}

/// Render data points
fn render_points(
    doc: &mut SvgDocument,
    db: &QuadrantDb,
    config: &RenderConfig,
    chart_left: f64,
    chart_top: f64,
    chart_width: f64,
    chart_height: f64,
) {
    for point in db.get_points() {
        // Convert normalized coordinates (0-1) to pixel coordinates
        // x: 0 = left, 1 = right
        // y: 0 = bottom, 1 = top (inverted for SVG)
        let px = chart_left + point.x * chart_width;
        let py = chart_top + (1.0 - point.y) * chart_height;

        // Get point styling
        let radius = point.style.radius.unwrap_or(DEFAULT_POINT_RADIUS);
        let default_point_color = &config.theme.quadrant_point_fill;
        let fill = point.style.color.as_deref().unwrap_or(default_point_color);
        let stroke_color = point.style.stroke_color.as_deref();
        let stroke_width = point
            .style
            .stroke_width
            .as_ref()
            .and_then(|s| s.strip_suffix("px"))
            .and_then(|s| s.parse::<f64>().ok())
            .unwrap_or(0.0);

        // Build point attributes
        let mut point_attrs = Attrs::new().with_fill(fill).with_class("quadrant-point");

        if let Some(sc) = stroke_color {
            point_attrs = point_attrs.with_stroke(sc).with_stroke_width(stroke_width);
        }

        // Render point circle
        doc.add_element(SvgElement::Circle {
            cx: px,
            cy: py,
            r: radius,
            attrs: point_attrs,
        });

        // Render point label (below the point per mermaid.js reference)
        if !point.text.is_empty() {
            doc.add_element(SvgElement::Text {
                x: px,
                y: py + POINT_TEXT_PADDING, // Position below the point
                content: point.text.clone(),
                attrs: Attrs::new()
                    .with_attr("text-anchor", "middle")
                    .with_attr("dominant-baseline", "hanging")
                    .with_class("quadrant-point-label")
                    .with_attr("font-size", &format!("{}", POINT_LABEL_FONT_SIZE))
                    .with_fill(&config.theme.quadrant_point_text_fill),
            });
        }
    }
}

/// Generate CSS for quadrant chart styling
/// Note: Text elements use inline fill attributes, so CSS should NOT override fill
/// to allow per-quadrant text colors (matching mermaid.js behavior)
fn generate_quadrant_css(theme: &crate::render::svg::Theme) -> String {
    format!(
        r#"
.quadrant-title {{
  font-family: {font_family};
}}

.quadrant {{
  stroke: none;
}}

.quadrant-border {{
  stroke-width: 1px;
}}

.quadrant-label {{
  font-family: {font_family};
}}

.axis-label {{
  font-family: {font_family};
}}

.quadrant-point {{
  stroke-width: 0;
}}

.quadrant-point-label {{
  font-family: {font_family};
}}
"#,
        font_family = theme.font_family,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_basic_quadrant() {
        let mut db = QuadrantDb::new();
        db.set_diagram_title("Test Quadrant");
        db.set_x_axis_left_text("Low");
        db.set_x_axis_right_text("High");
        db.set_y_axis_bottom_text("Low");
        db.set_y_axis_top_text("High");
        db.set_quadrant1_text("Q1");
        db.set_quadrant2_text("Q2");
        db.set_quadrant3_text("Q3");
        db.set_quadrant4_text("Q4");

        let config = RenderConfig::default();
        let svg = render_quadrant(&db, &config).unwrap();

        // Check SVG structure
        assert!(svg.contains("<svg"));
        assert!(svg.contains("</svg>"));
        assert!(svg.contains("Test Quadrant"));
        assert!(svg.contains("Q1"));
        assert!(svg.contains("Q2"));
        assert!(svg.contains("Q3"));
        assert!(svg.contains("Q4"));
        assert!(svg.contains("Low"));
        assert!(svg.contains("High"));
    }

    #[test]
    fn test_render_quadrant_with_points() {
        let mut db = QuadrantDb::new();
        db.add_point("Point A", "", "0.25", "0.75", &[]);
        db.add_point("Point B", "", "0.75", "0.25", &[]);

        let config = RenderConfig::default();
        let svg = render_quadrant(&db, &config).unwrap();

        assert!(svg.contains("Point A"));
        assert!(svg.contains("Point B"));
        assert!(svg.contains("<circle")); // Points are rendered as circles
    }

    #[test]
    fn test_render_quadrant_with_styled_points() {
        let mut db = QuadrantDb::new();
        db.add_point(
            "Styled Point",
            "",
            "0.5",
            "0.5",
            &["radius: 10", "color: #ff0000"],
        );

        let config = RenderConfig::default();
        let svg = render_quadrant(&db, &config).unwrap();

        assert!(svg.contains("Styled Point"));
        assert!(svg.contains("r=\"10\"")); // Custom radius
        assert!(svg.contains("fill=\"#ff0000\"")); // Custom color
    }

    #[test]
    fn test_render_empty_quadrant() {
        let db = QuadrantDb::new();
        let config = RenderConfig::default();
        let svg = render_quadrant(&db, &config).unwrap();

        // Should still render the basic structure
        assert!(svg.contains("<svg"));
        assert!(svg.contains("</svg>"));
        assert!(svg.contains("quadrant-1"));
        assert!(svg.contains("quadrant-2"));
        assert!(svg.contains("quadrant-3"));
        assert!(svg.contains("quadrant-4"));
    }
}
