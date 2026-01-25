//! Pie chart renderer
//!
//! Renders pie charts to SVG, matching mermaid.js visual output.
//! Uses a center-transform approach where all pie elements are positioned
//! relative to the pie center via a group transform.

use std::f64::consts::PI;

use crate::diagrams::pie::PieDb;
use crate::error::Result;
use crate::render::svg::{Attrs, RenderConfig, SvgDocument, SvgElement};

/// Render a pie chart to SVG
pub fn render_pie(db: &PieDb, config: &RenderConfig) -> Result<String> {
    let mut doc = SvgDocument::new();

    // Mermaid.js layout constants (from pieRenderer.ts)
    const MARGIN: f64 = 40.0;
    const LEGEND_RECT_SIZE: f64 = 18.0;
    const LEGEND_SPACING: f64 = 4.0;
    let height: f64 = 450.0;
    let pie_width: f64 = height; // mermaid.js uses pieWidth = height (square area for pie)

    // Radius calculation matches mermaid.js exactly
    let radius = (pie_width.min(height) / 2.0) - MARGIN; // = 185.0

    // Pie center - exactly at center of pie area
    let cx = pie_width / 2.0; // = 225.0
    let cy = height / 2.0; // = 225.0

    // Calculate legend text width estimate (mermaid.js measures actual DOM text width)
    let sections = db.get_sections();
    let longest_label = sections
        .iter()
        .map(|(label, value)| {
            if db.get_show_data() {
                format!(
                    "{} [{}]",
                    label,
                    if value.fract() == 0.0 {
                        format!("{}", *value as i64)
                    } else {
                        format!("{}", value)
                    }
                )
            } else {
                label.clone()
            }
        })
        .max_by_key(|s| s.len())
        .unwrap_or_default();
    // Approximate text width for 17px Trebuchet MS font
    // Mermaid.js uses getBoundingClientRect() for actual DOM measurement
    // We use 9.0px per character as a reasonable approximation
    let estimated_text_width = longest_label.len() as f64 * 9.0;

    // Dynamic width calculation
    let width = pie_width + MARGIN + LEGEND_RECT_SIZE + LEGEND_SPACING + estimated_text_width;

    doc.set_size(width, height);

    // Add theme styles
    if config.embed_css {
        doc.add_style(&config.theme.generate_css());
        doc.add_style(&generate_pie_css(&config.theme));
    }

    // Calculate total
    let total: f64 = sections.iter().map(|(_, v)| v).sum();

    if total <= 0.0 {
        // Empty pie chart - just render the title if present
        if let Some(title) = db.get_diagram_title() {
            let title_elem = SvgElement::Text {
                x: cx,
                y: 30.0,
                content: title.to_string(),
                attrs: Attrs::new()
                    .with_attr("text-anchor", "middle")
                    .with_class("pieTitleText")
                    .with_attr("font-size", "25"),
            };
            doc.add_element(title_elem);
        }
        return Ok(doc.to_string());
    }

    // Step 1: Get sections and sort by value descending (like mermaid)
    let sections_vec: Vec<_> = sections.to_vec();
    let mut sorted_for_rendering: Vec<_> = sections_vec.clone();
    sorted_for_rendering.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    // Step 2: Create color mapping based on SORTED order (not input order!)
    // D3 scaleOrdinal assigns colors as labels are first seen - in sorted order
    let label_to_color_index: std::collections::HashMap<_, _> = sorted_for_rendering
        .iter()
        .enumerate()
        .map(|(i, (label, _))| (label.clone(), i))
        .collect();

    // Pie colors from theme
    let colors: Vec<&str> = config.theme.pie_colors.iter().map(|s| s.as_str()).collect();

    // Legend dimensions matching mermaid.js exactly:
    // horizontal = 12 * LEGEND_RECT_SIZE (from pie center) = 216
    let legend_item_height = LEGEND_RECT_SIZE + LEGEND_SPACING; // 22.0
    let num_items = sections_vec.len() as f64;
    let legend_vertical_offset = (legend_item_height * num_items) / 2.0;
    let legend_x_from_center = 12.0 * LEGEND_RECT_SIZE; // = 216

    // Build all pie elements in a group with center transform (mermaid.js approach)
    let mut pie_group_children: Vec<SvgElement> = Vec::new();

    // Outer circle (pieOuterCircle) - at center (0,0) in transformed coords
    let outer_circle = SvgElement::Circle {
        cx: 0.0,
        cy: 0.0,
        r: radius + 1.0,
        attrs: Attrs::new().with_class("pieOuterCircle"),
    };
    pie_group_children.push(outer_circle);

    // Render slices using mermaid.js path format:
    // M{start_x},{start_y} A{r},{r},0,{large_arc},1,{end_x},{end_y} L0,0 Z
    // (arc from perimeter point, then line to center, close)
    let mut start_angle = -PI / 2.0; // Start at top (12 o'clock)
    let mut percentage_labels: Vec<(f64, f64, String)> = Vec::new();

    for (label, value) in sorted_for_rendering.iter() {
        let percentage = *value / total;
        let angle = percentage * 2.0 * PI;
        let end_angle = start_angle + angle;

        // Calculate arc points (relative to center at 0,0)
        let x1 = radius * start_angle.cos();
        let y1 = radius * start_angle.sin();
        let x2 = radius * end_angle.cos();
        let y2 = radius * end_angle.sin();

        // Large arc flag (1 if angle > 180 degrees)
        let large_arc = if angle > PI { 1 } else { 0 };

        // Create pie slice path in mermaid.js format:
        // Start at arc start, arc to end, line to center, close
        let path = format!(
            "M{:.3},{:.3}A{},{},0,{},1,{:.3},{:.3}L0,0Z",
            x1, y1, radius as i32, radius as i32, large_arc, x2, y2
        );

        let color_index = label_to_color_index.get(label).copied().unwrap_or(0);
        let color = colors[color_index % colors.len()];
        let slice = SvgElement::Path {
            d: path,
            attrs: Attrs::new().with_fill(color).with_class("pieCircle"),
        };
        pie_group_children.push(slice);

        // Collect percentage label data
        if percentage >= 0.02 {
            let mid_angle = start_angle + angle / 2.0;
            let label_radius = radius * 0.75;
            let label_x = label_radius * mid_angle.cos();
            let label_y = label_radius * mid_angle.sin();
            percentage_labels.push((
                label_x,
                label_y,
                format!("{}%", (percentage * 100.0).round() as i32),
            ));
        }

        start_angle = end_angle;
    }

    // Render percentage labels with transform (mermaid.js style)
    for (label_x, label_y, content) in percentage_labels {
        let pct_label = SvgElement::Text {
            x: 0.0,
            y: 0.0,
            content,
            attrs: Attrs::new()
                .with_transform(&format!("translate({:.3},{:.3})", label_x, label_y))
                .with_class("slice")
                .with_style("text-anchor: middle;"),
        };
        pie_group_children.push(pct_label);
    }

    // Render title (positioned relative to center)
    if let Some(title) = db.get_diagram_title() {
        let title_elem = SvgElement::Text {
            x: 0.0,
            y: -200.0, // Above the pie (mermaid.js uses y=-200 from center)
            content: title.to_string(),
            attrs: Attrs::new().with_class("pieTitleText"),
        };
        pie_group_children.push(title_elem);
    }

    // Render legend items - each in its own group with transform (mermaid.js style)
    for (i, (label, value)) in sections_vec.iter().enumerate() {
        let color_index = label_to_color_index.get(label).copied().unwrap_or(0);
        let color = colors[color_index % colors.len()];

        // Position: x = 216 from center, y centered around 0
        let item_y = (i as f64) * legend_item_height - legend_vertical_offset;

        // Format label text
        let display_label = if db.get_show_data() {
            let value_str = if value.fract() == 0.0 {
                format!("{}", *value as i64)
            } else {
                format!("{}", value)
            };
            format!("{} [{}]", label, value_str)
        } else {
            label.clone()
        };

        // Create legend item group
        let legend_item = SvgElement::Group {
            children: vec![
                SvgElement::Rect {
                    x: 0.0,
                    y: 0.0,
                    width: LEGEND_RECT_SIZE,
                    height: LEGEND_RECT_SIZE,
                    rx: None,
                    ry: None,
                    attrs: Attrs::new().with_style(&format!(
                        "fill: {}; stroke: {};",
                        color_to_rgb(color),
                        color_to_rgb(color)
                    )),
                },
                SvgElement::Text {
                    x: 22.0, // mermaid.js uses x="22" for legend text
                    y: 14.0, // mermaid.js uses y="14" for legend text
                    content: display_label,
                    attrs: Attrs::new(),
                },
            ],
            attrs: Attrs::new().with_class("legend").with_transform(&format!(
                "translate({},{:.0})",
                legend_x_from_center, item_y
            )),
        };
        pie_group_children.push(legend_item);
    }

    // Create the main pie group with center transform
    let pie_group = SvgElement::Group {
        children: pie_group_children,
        attrs: Attrs::new().with_transform(&format!("translate({},{})", cx as i32, cy as i32)),
    };
    doc.add_element(pie_group);

    Ok(doc.to_string())
}

/// Convert color string to RGB format for inline styles (mermaid.js compatibility)
fn color_to_rgb(color: &str) -> String {
    // Handle HSL colors - convert to rgb() format
    if color.starts_with("hsl(") {
        // Parse hsl(h, s%, l%) format
        let inner = color
            .trim_start_matches("hsl(")
            .trim_end_matches(')')
            .trim_end_matches('%');
        let parts: Vec<&str> = inner
            .split(',')
            .map(|s| s.trim().trim_end_matches('%'))
            .collect();
        if parts.len() >= 3 {
            if let (Ok(h), Ok(s), Ok(l)) = (
                parts[0].parse::<f64>(),
                parts[1].parse::<f64>(),
                parts[2].parse::<f64>(),
            ) {
                let (r, g, b) = hsl_to_rgb(h, s / 100.0, l / 100.0);
                return format!("rgb({}, {}, {})", r, g, b);
            }
        }
    }
    // Handle hex colors - convert to rgb() format
    if color.starts_with('#') && color.len() == 7 {
        if let (Ok(r), Ok(g), Ok(b)) = (
            u8::from_str_radix(&color[1..3], 16),
            u8::from_str_radix(&color[3..5], 16),
            u8::from_str_radix(&color[5..7], 16),
        ) {
            return format!("rgb({}, {}, {})", r, g, b);
        }
    }
    // Return as-is for other formats
    color.to_string()
}

/// Convert HSL to RGB values (0-255 range)
fn hsl_to_rgb(h: f64, s: f64, l: f64) -> (u8, u8, u8) {
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let h_prime = h / 60.0;
    let x = c * (1.0 - ((h_prime % 2.0) - 1.0).abs());
    let m = l - c / 2.0;

    let (r1, g1, b1) = if h_prime < 1.0 {
        (c, x, 0.0)
    } else if h_prime < 2.0 {
        (x, c, 0.0)
    } else if h_prime < 3.0 {
        (0.0, c, x)
    } else if h_prime < 4.0 {
        (0.0, x, c)
    } else if h_prime < 5.0 {
        (x, 0.0, c)
    } else {
        (c, 0.0, x)
    };

    (
        ((r1 + m) * 255.0).round() as u8,
        ((g1 + m) * 255.0).round() as u8,
        ((b1 + m) * 255.0).round() as u8,
    )
}

fn generate_pie_css(theme: &crate::render::svg::Theme) -> String {
    // CSS matching mermaid.js pieStyles exactly
    format!(
        r#"
.pieCircle {{
  stroke: {pie_stroke};
  stroke-width: 2px;
  opacity: {pie_opacity};
}}

.pieOuterCircle {{
  stroke: {pie_outer_stroke};
  stroke-width: 2px;
  fill: none;
}}

.pieTitleText {{
  text-anchor: middle;
  font-size: 25px;
  fill: black;
  font-family: {font_family};
}}

.slice {{
  font-family: {font_family};
  fill: #333;
  font-size: 17px;
}}

.legend text {{
  fill: black;
  font-family: {font_family};
  font-size: 17px;
}}
"#,
        pie_stroke = theme.pie_stroke_color,
        pie_outer_stroke = theme.pie_outer_stroke_color,
        pie_opacity = theme.pie_opacity,
        font_family = theme.font_family,
    )
}
