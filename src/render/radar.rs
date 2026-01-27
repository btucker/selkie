//! Radar chart renderer
//!
//! Renders radar/spider charts with multiple data curves plotted on axes
//! radiating from a center point.

use std::f64::consts::PI;

use crate::diagrams::radar::{Graticule, RadarDb};
use crate::error::Result;
use crate::render::svg::{Attrs, RenderConfig, SvgDocument, SvgElement};

/// Default radar chart dimensions (matching mermaid.js defaults)
/// Mermaid uses width=600, height=600 with margins of 50 each = 700x700 total
const DEFAULT_WIDTH: f64 = 600.0;
const DEFAULT_HEIGHT: f64 = 600.0;
const MARGIN_TOP: f64 = 50.0;
const MARGIN_RIGHT: f64 = 50.0;
const MARGIN_BOTTOM: f64 = 50.0;
const MARGIN_LEFT: f64 = 50.0;

/// Axis scale and label factors (matching mermaid.js defaults)
/// axisScaleFactor=1.0: axes extend to full radius
/// axisLabelFactor=1.05: labels positioned just outside the chart
const AXIS_SCALE_FACTOR: f64 = 1.0;
const AXIS_LABEL_FACTOR: f64 = 1.05;

/// Curve tension for smooth curves (Catmull-Rom spline)
const CURVE_TENSION: f64 = 0.167;

/// Radar colors (matching mermaid.js default theme - pastel cScale colors)
/// Mermaid uses HSL colors with ~76% lightness for a pastel look
const RADAR_COLORS: &[&str] = &[
    "#8686FF", // hsl(240, 100%, 76%) - Light blue/lavender
    "#FFFF78", // hsl(60, 100%, 73%) - Light yellow
    "#9FFF9F", // hsl(120, 100%, 76%) - Light green
    "#C986FF", // hsl(270, 100%, 76%) - Light purple
    "#FF86FF", // hsl(300, 100%, 76%) - Light magenta
    "#FF86C9", // hsl(330, 100%, 76%) - Light pink
    "#FF8686", // hsl(0, 100%, 76%) - Light red
    "#FFC986", // hsl(30, 100%, 76%) - Light orange
];

/// Render a radar chart to SVG
pub fn render_radar(db: &RadarDb, config: &RenderConfig) -> Result<String> {
    let mut doc = SvgDocument::new();

    let chart_width = DEFAULT_WIDTH;
    let chart_height = DEFAULT_HEIGHT;
    let total_width = chart_width + MARGIN_LEFT + MARGIN_RIGHT;
    let total_height = chart_height + MARGIN_TOP + MARGIN_BOTTOM;

    doc.set_size(total_width, total_height);

    // Add theme styles
    if config.embed_css {
        doc.add_style(&config.theme.generate_css());
        doc.add_style(&generate_radar_css(&config.theme));
    }

    let center_x = MARGIN_LEFT + chart_width / 2.0;
    let center_y = MARGIN_TOP + chart_height / 2.0;
    let radius = chart_width.min(chart_height) / 2.0;

    let axes = db.get_axes();
    let curves = db.get_curves();
    let options = db.get_options();
    let title = db.get_title();

    // Calculate min/max values
    let max_value: f64 = options.max.unwrap_or_else(|| {
        curves
            .iter()
            .flat_map(|c| c.entries.iter())
            .copied()
            .fold(0.0, f64::max)
    });
    let min_value: f64 = options.min;

    // Create main group centered on chart
    let mut main_group_children: Vec<SvgElement> = Vec::new();

    // PHASE 1: Draw all shapes first (for correct z-order)

    // Draw graticule (background grid) - shapes
    draw_graticule(
        &mut main_group_children,
        axes.len(),
        radius,
        options.ticks,
        &options.graticule,
    );

    // Draw axis lines - shapes
    draw_axis_lines(&mut main_group_children, axes, radius);

    // Draw curves (paths/polygons) - shapes
    draw_curves(
        &mut main_group_children,
        axes,
        curves,
        min_value,
        max_value,
        &options.graticule,
        radius,
    );

    // Draw legend boxes if enabled - shapes
    if options.show_legend {
        draw_legend_boxes(&mut main_group_children, curves, chart_width, chart_height);
    }

    // PHASE 2: Draw all text after shapes (text appears on top)

    // Draw axis labels - text
    draw_axis_labels(&mut main_group_children, axes, radius, config);

    // Draw legend labels if enabled - text
    if options.show_legend {
        draw_legend_labels(&mut main_group_children, curves, chart_width, chart_height);
    }

    // Draw title last - text
    if !title.is_empty() {
        let title_elem = SvgElement::Text {
            x: 0.0,
            y: -(chart_height / 2.0) - MARGIN_TOP,
            content: title.to_string(),
            attrs: Attrs::new()
                .with_attr("text-anchor", "middle")
                .with_attr("dominant-baseline", "hanging")
                .with_class("radarTitle")
                .with_attr("font-size", "16")
                .with_attr("font-weight", "bold")
                .with_fill(&config.theme.primary_text_color),
        };
        main_group_children.push(title_elem);
    }

    // Wrap in centered group
    let main_group = SvgElement::Group {
        children: main_group_children,
        attrs: Attrs::new().with_attr(
            "transform",
            &format!("translate({}, {})", center_x, center_y),
        ),
    };
    doc.add_element(main_group);

    Ok(doc.to_string())
}

/// Draw the graticule (background grid)
fn draw_graticule(
    children: &mut Vec<SvgElement>,
    num_axes: usize,
    radius: f64,
    ticks: usize,
    graticule: &Graticule,
) {
    match graticule {
        Graticule::Circle => {
            // Draw concentric circles
            for i in 1..=ticks {
                let r = (radius * i as f64) / ticks as f64;
                let circle = SvgElement::Circle {
                    cx: 0.0,
                    cy: 0.0,
                    r,
                    attrs: Attrs::new()
                        .with_fill("#DEDEDE")
                        .with_attr("fill-opacity", "0.3")
                        .with_stroke("#DEDEDE")
                        .with_stroke_width(1.0)
                        .with_class("radarGraticule"),
                };
                children.push(circle);
            }
        }
        Graticule::Polygon => {
            // Draw concentric polygons
            for i in 1..=ticks {
                let r = (radius * i as f64) / ticks as f64;
                let points = (0..num_axes)
                    .map(|j| {
                        let angle = (2.0 * PI * j as f64) / num_axes as f64 - PI / 2.0;
                        let x = r * angle.cos();
                        let y = r * angle.sin();
                        format!("{},{}", x, y)
                    })
                    .collect::<Vec<_>>()
                    .join(" ");

                let polygon = SvgElement::PolygonStr {
                    points,
                    attrs: Attrs::new()
                        .with_fill("#DEDEDE")
                        .with_attr("fill-opacity", "0.3")
                        .with_stroke("#DEDEDE")
                        .with_stroke_width(1.0)
                        .with_class("radarGraticule"),
                };
                children.push(polygon);
            }
        }
    }
}

/// Draw the radar axes (lines only - shapes)
fn draw_axis_lines(
    children: &mut Vec<SvgElement>,
    axes: &[crate::diagrams::radar::RadarAxis],
    radius: f64,
) {
    let num_axes = axes.len();

    for (i, _axis) in axes.iter().enumerate() {
        let angle = (2.0 * PI * i as f64) / num_axes as f64 - PI / 2.0;

        // Draw axis line
        let line = SvgElement::Line {
            x1: 0.0,
            y1: 0.0,
            x2: radius * AXIS_SCALE_FACTOR * angle.cos(),
            y2: radius * AXIS_SCALE_FACTOR * angle.sin(),
            attrs: Attrs::new()
                .with_stroke("#333333")
                .with_stroke_width(1.0)
                .with_class("radarAxisLine"),
        };
        children.push(line);
    }
}

/// Draw the radar axis labels (text only)
fn draw_axis_labels(
    children: &mut Vec<SvgElement>,
    axes: &[crate::diagrams::radar::RadarAxis],
    radius: f64,
    config: &RenderConfig,
) {
    let num_axes = axes.len();

    for (i, axis) in axes.iter().enumerate() {
        let angle = (2.0 * PI * i as f64) / num_axes as f64 - PI / 2.0;

        // Draw axis label
        let label_x = radius * AXIS_LABEL_FACTOR * angle.cos();
        let label_y = radius * AXIS_LABEL_FACTOR * angle.sin();
        let label = SvgElement::Text {
            x: label_x,
            y: label_y,
            content: axis.label.clone(),
            attrs: Attrs::new()
                .with_attr("text-anchor", "middle")
                .with_attr("dominant-baseline", "middle")
                .with_class("radarAxisLabel")
                .with_attr("font-size", "12")
                .with_fill(&config.theme.primary_text_color),
        };
        children.push(label);
    }
}

/// Draw the radar curves (data series)
fn draw_curves(
    children: &mut Vec<SvgElement>,
    axes: &[crate::diagrams::radar::RadarAxis],
    curves: &[crate::diagrams::radar::RadarCurve],
    min_value: f64,
    max_value: f64,
    graticule: &Graticule,
    radius: f64,
) {
    let num_axes = axes.len();

    for (index, curve) in curves.iter().enumerate() {
        if curve.entries.len() != num_axes {
            // Skip curves that don't have an entry for each axis
            continue;
        }

        let color = RADAR_COLORS[index % RADAR_COLORS.len()];

        // Compute points for the curve
        let points: Vec<(f64, f64)> = curve
            .entries
            .iter()
            .enumerate()
            .map(|(i, &value)| {
                let angle = (2.0 * PI * i as f64) / num_axes as f64 - PI / 2.0;
                let r = relative_radius(value, min_value, max_value, radius);
                let x = r * angle.cos();
                let y = r * angle.sin();
                (x, y)
            })
            .collect();

        match graticule {
            Graticule::Circle => {
                // Draw a smooth closed curve (Catmull-Rom spline)
                let path_d = closed_round_curve(&points, CURVE_TENSION);
                let path = SvgElement::Path {
                    d: path_d,
                    attrs: Attrs::new()
                        .with_fill(color)
                        .with_stroke(color)
                        .with_class(&format!("radarCurve-{}", index)),
                };
                children.push(path);
            }
            Graticule::Polygon => {
                // Draw a polygon
                let points_str = points
                    .iter()
                    .map(|(x, y)| format!("{},{}", x, y))
                    .collect::<Vec<_>>()
                    .join(" ");
                let polygon = SvgElement::PolygonStr {
                    points: points_str,
                    attrs: Attrs::new()
                        .with_fill(color)
                        .with_stroke(color)
                        .with_class(&format!("radarCurve-{}", index)),
                };
                children.push(polygon);
            }
        }
    }
}

/// Calculate the relative radius for a value
fn relative_radius(value: f64, min_value: f64, max_value: f64, radius: f64) -> f64 {
    let clipped_value = value.clamp(min_value, max_value);
    if (max_value - min_value).abs() < f64::EPSILON {
        return radius;
    }
    (radius * (clipped_value - min_value)) / (max_value - min_value)
}

/// Generate a closed smooth curve path using Catmull-Rom splines
fn closed_round_curve(points: &[(f64, f64)], tension: f64) -> String {
    let num_points = points.len();
    if num_points == 0 {
        return String::new();
    }

    let mut d = format!("M{},{}", points[0].0, points[0].1);

    for i in 0..num_points {
        let p0 = points[(i + num_points - 1) % num_points];
        let p1 = points[i];
        let p2 = points[(i + 1) % num_points];
        let p3 = points[(i + 2) % num_points];

        // Calculate control points for cubic Bezier segment
        let cp1 = (
            p1.0 + (p2.0 - p0.0) * tension,
            p1.1 + (p2.1 - p0.1) * tension,
        );
        let cp2 = (
            p2.0 - (p3.0 - p1.0) * tension,
            p2.1 - (p3.1 - p1.1) * tension,
        );

        d.push_str(&format!(
            " C{},{} {},{} {},{}",
            cp1.0, cp1.1, cp2.0, cp2.1, p2.0, p2.1
        ));
    }

    d.push_str(" Z");
    d
}

/// Draw legend boxes (shapes only)
fn draw_legend_boxes(
    children: &mut Vec<SvgElement>,
    curves: &[crate::diagrams::radar::RadarCurve],
    chart_width: f64,
    chart_height: f64,
) {
    let legend_x = (chart_width / 2.0 + MARGIN_RIGHT) * 3.0 / 4.0;
    let legend_y = -(chart_height / 2.0 + MARGIN_TOP) * 3.0 / 4.0;
    let line_height = 20.0;

    for (index, _curve) in curves.iter().enumerate() {
        let item_y = legend_y + (index as f64) * line_height;
        let color = RADAR_COLORS[index % RADAR_COLORS.len()];

        // Colored box
        let box_elem = SvgElement::Rect {
            x: legend_x,
            y: item_y,
            width: 12.0,
            height: 12.0,
            rx: None,
            ry: None,
            attrs: Attrs::new()
                .with_fill(color)
                .with_class(&format!("radarLegendBox-{}", index)),
        };
        children.push(box_elem);
    }
}

/// Draw legend labels (text only)
fn draw_legend_labels(
    children: &mut Vec<SvgElement>,
    curves: &[crate::diagrams::radar::RadarCurve],
    chart_width: f64,
    chart_height: f64,
) {
    let legend_x = (chart_width / 2.0 + MARGIN_RIGHT) * 3.0 / 4.0;
    let legend_y = -(chart_height / 2.0 + MARGIN_TOP) * 3.0 / 4.0;
    let line_height = 20.0;

    for (index, curve) in curves.iter().enumerate() {
        let item_y = legend_y + (index as f64) * line_height;

        // Label text
        let text_elem = SvgElement::Text {
            x: legend_x + 16.0,
            y: item_y,
            content: curve.label.clone(),
            attrs: Attrs::new()
                .with_attr("dominant-baseline", "hanging")
                .with_class("radarLegendText")
                .with_attr("font-size", "12"),
        };
        children.push(text_elem);
    }
}

/// Generate CSS for curve colors
fn generate_curve_color_css() -> String {
    let mut css = String::new();
    for (i, color) in RADAR_COLORS.iter().enumerate() {
        css.push_str(&format!(
            r#".radarCurve-{i} {{
    fill: {color};
    stroke: {color};
}}
.radarLegendBox-{i} {{
    fill: {color};
    stroke: {color};
}}
"#,
            i = i,
            color = color
        ));
    }
    css
}

/// Generate radar-specific CSS
fn generate_radar_css(theme: &crate::render::svg::Theme) -> String {
    format!(
        r#"
.radarGraticule {{
    fill: #DEDEDE;
    fill-opacity: 0.3;
    stroke: #DEDEDE;
    stroke-width: 1px;
}}

.radarAxisLine {{
    stroke: #333333;
    stroke-width: 2px;
}}

.radarAxisLabel {{
    font-family: {font_family};
    font-size: 12px;
    fill: {text_color};
}}

.radarTitle {{
    font-family: {font_family};
    font-size: 16px;
    font-weight: bold;
    fill: {text_color};
}}

.radarLegendText {{
    font-family: {font_family};
    font-size: 12px;
    fill: {text_color};
}}

[class^="radarCurve-"] {{
    fill-opacity: 0.5;
    stroke-width: 2px;
}}

{curve_colors}
"#,
        font_family = theme.font_family,
        text_color = theme.primary_text_color,
        curve_colors = generate_curve_color_css(),
    )
}
