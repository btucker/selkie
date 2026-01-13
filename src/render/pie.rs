//! Pie chart renderer

use std::f64::consts::PI;

use crate::diagrams::pie::PieDb;
use crate::error::Result;
use crate::render::svg::{Attrs, RenderConfig, SvgDocument, SvgElement};

/// Render a pie chart to SVG
pub fn render_pie(db: &PieDb, config: &RenderConfig) -> Result<String> {
    let mut doc = SvgDocument::new();

    // Default pie chart dimensions (sized to match mermaid.js)
    let radius = 185.0; // mermaid.js uses 185
    let pie_diameter = radius * 2.0;
    let pie_width = pie_diameter + 50.0; // Space for pie + padding
    let legend_width = 180.0; // Space for legend
    let width = pie_width + legend_width;
    let height = 450.0; // mermaid.js uses 450
    let cx = pie_width / 2.0; // center x (in pie area)
    let cy = height / 2.0; // center y

    doc.set_size(width, height);

    // Add theme styles
    if config.embed_css {
        doc.add_style(&config.theme.generate_css());
        doc.add_style(&generate_pie_css(&config.theme));
    }

    // Calculate total
    let sections = db.get_sections();
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
                    .with_class("pie-title")
                    .with_attr("font-size", "20")
                    .with_attr("font-weight", "bold"),
            };
            doc.add_element(title_elem);
        }
        return Ok(doc.to_string());
    }

    // Mermaid.js assigns colors based on ORIGINAL input order, but renders slices
    // sorted by value descending. Legend stays in original order.

    // Step 1: Create color mapping based on original order
    let sections_vec: Vec<_> = sections.to_vec();
    let label_to_color_index: std::collections::HashMap<_, _> = sections_vec
        .iter()
        .enumerate()
        .map(|(i, (label, _))| (label.clone(), i))
        .collect();

    // Step 2: Sort by value descending for slice rendering
    let mut sorted_for_rendering: Vec<_> = sections_vec.clone();
    sorted_for_rendering.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    // Pie colors from theme
    let colors: Vec<&str> = config.theme.pie_colors.iter().map(|s| s.as_str()).collect();

    let mut start_angle = -PI / 2.0; // Start at top (12 o'clock)

    // Title offset
    let title_height = if db.get_diagram_title().is_some() {
        40.0
    } else {
        0.0
    };
    let pie_cy = cy + title_height / 2.0;

    // Legend dimensions (positioned to the right of the pie)
    let legend_x = pie_width + 10.0;
    let legend_y = height / 2.0 - 50.0; // Vertically centered
    let legend_item_height = 22.0;
    let legend_box_size = 18.0; // mermaid.js uses 18x18

    // Render title
    if let Some(title) = db.get_diagram_title() {
        let title_elem = SvgElement::Text {
            x: cx,
            y: 25.0,
            content: title.to_string(),
            attrs: Attrs::new()
                .with_attr("text-anchor", "middle")
                .with_class("pie-title")
                .with_attr("font-size", "20")
                .with_attr("font-weight", "bold"),
        };
        doc.add_element(title_elem);
    }

    // Render outer circle (pieOuterCircle) - frames the pie chart
    // mermaid.js uses radius + 1 for the outer circle
    let outer_circle = SvgElement::Circle {
        cx,
        cy: pie_cy,
        r: radius + 1.0,
        attrs: Attrs::new()
            .with_fill("none")
            .with_stroke(&config.theme.pie_outer_stroke_color)
            .with_stroke_width(2.0)
            .with_class("pieOuterCircle"),
    };
    doc.add_element(outer_circle);

    // Render each slice (sorted by value descending)
    for (label, value) in sorted_for_rendering.iter() {
        let percentage = *value / total;
        let angle = percentage * 2.0 * PI;
        let end_angle = start_angle + angle;

        // Calculate arc points
        let x1 = cx + radius * start_angle.cos();
        let y1 = pie_cy + radius * start_angle.sin();
        let x2 = cx + radius * end_angle.cos();
        let y2 = pie_cy + radius * end_angle.sin();

        // Large arc flag (1 if angle > 180 degrees)
        let large_arc = if angle > PI { 1 } else { 0 };

        // Create pie slice path
        let path = format!(
            "M {} {} L {} {} A {} {} 0 {} 1 {} {} Z",
            cx,
            pie_cy, // Move to center
            x1,
            y1, // Line to start of arc
            radius,
            radius,    // Arc radii
            large_arc, // Large arc flag
            x2,
            y2 // End of arc
        );

        // Use color based on ORIGINAL input order, not sorted order
        let color_index = label_to_color_index.get(label).copied().unwrap_or(0);
        let color = colors[color_index % colors.len()];
        let slice = SvgElement::Path {
            d: path,
            attrs: Attrs::new()
                .with_fill(color)
                .with_stroke(&config.theme.pie_stroke_color)
                .with_stroke_width(2.0)
                .with_attr("opacity", &config.theme.pie_opacity)
                .with_class("pieCircle"),
        };
        doc.add_element(slice);

        // Add percentage label inside slice (for larger slices)
        if percentage >= 0.05 {
            // Only show if slice is at least 5%
            let mid_angle = start_angle + angle / 2.0;
            let label_radius = radius * 0.75; // Position inside slice (mermaid.js uses ~0.75)
            let label_x = cx + label_radius * mid_angle.cos();
            let label_y = pie_cy + label_radius * mid_angle.sin();

            let pct_label = SvgElement::Text {
                x: label_x,
                y: label_y,
                content: format!("{}%", (percentage * 100.0).round() as i32),
                attrs: Attrs::new()
                    .with_attr("text-anchor", "middle")
                    .with_attr("dominant-baseline", "middle")
                    .with_class("slice")
                    .with_attr("font-size", "17"),
            };
            doc.add_element(pct_label);
        }

        start_angle = end_angle;
    }

    // Build legend items in ORIGINAL input order (not sorted)
    let legend_items: Vec<(String, String, f64)> = sections_vec
        .iter()
        .enumerate()
        .map(|(i, (label, value))| {
            let color = colors[i % colors.len()];
            let percentage = *value / total;
            (color.to_string(), label.clone(), percentage)
        })
        .collect();

    // Render legend
    let legend_group = render_legend(
        &legend_items,
        legend_x,
        legend_y,
        legend_item_height,
        legend_box_size,
    );
    doc.add_element(legend_group);

    Ok(doc.to_string())
}

/// Render a legend for the pie chart
fn render_legend(
    items: &[(String, String, f64)], // (color, label, percentage)
    x: f64,
    y: f64,
    item_height: f64,
    box_size: f64,
) -> SvgElement {
    let mut children = Vec::new();

    for (i, (color, label, _percentage)) in items.iter().enumerate() {
        let item_y = y + (i as f64) * item_height;

        // Colored box with matching stroke (mermaid.js style)
        children.push(SvgElement::Rect {
            x,
            y: item_y,
            width: box_size,
            height: box_size,
            rx: None,
            ry: None,
            attrs: Attrs::new()
                .with_fill(color)
                .with_stroke(color)
                .with_class("legend"),
        });

        // Label text only (no percentage - mermaid.js style)
        // mermaid.js uses x="22" relative to rect x (i.e., box_size + 4)
        children.push(SvgElement::Text {
            x: x + box_size + 4.0,
            y: item_y + 14.0, // mermaid.js uses y="14" relative to rect
            content: label.clone(),
            attrs: Attrs::new()
                .with_class("legend")
                .with_attr("font-size", "17"),
        });
    }

    SvgElement::Group {
        children,
        attrs: Attrs::new().with_class("legend"),
    }
}

fn generate_pie_css(theme: &crate::render::svg::Theme) -> String {
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
  fill: {pie_title_color};
  font-family: {font_family};
}}

.slice {{
  font-family: {font_family};
  fill: {pie_title_color};
  font-size: 17px;
}}

.legend text {{
  fill: {pie_legend_color};
  font-family: {font_family};
  font-size: 17px;
}}
"#,
        pie_stroke = theme.pie_stroke_color,
        pie_outer_stroke = theme.pie_outer_stroke_color,
        pie_opacity = theme.pie_opacity,
        pie_title_color = theme.pie_title_text_color,
        pie_legend_color = theme.pie_legend_text_color,
        font_family = theme.font_family,
    )
}
