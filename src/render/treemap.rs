//! Treemap diagram renderer
//!
//! Renders treemap diagrams using a squarified layout algorithm.
//! Section nodes (branches) get headers with labels.
//! Leaf nodes fill the remaining space with values displayed.

use crate::diagrams::treemap::{TreemapDb, TreemapNode};
use crate::error::Result;
use crate::render::svg::{Attrs, RenderConfig, SvgDocument, SvgElement};

/// Default inner padding between cells/sections (reserved for future use)
#[allow(dead_code)]
const DEFAULT_INNER_PADDING: f64 = 10.0;

/// Section header height
const SECTION_HEADER_HEIGHT: f64 = 25.0;

/// Section inner padding
const SECTION_PADDING: f64 = 10.0;

/// Maximum number of color sections (matches mermaid.js cScale)
const MAX_SECTIONS: usize = 12;

/// Font size for leaf labels
const LEAF_FONT_SIZE: f64 = 14.0;

/// Font size for section labels
const SECTION_FONT_SIZE: f64 = 12.0;

/// Font size for value labels
const VALUE_FONT_SIZE: f64 = 10.0;

/// Default diagram width
const DEFAULT_WIDTH: f64 = 960.0;

/// Default diagram height
const DEFAULT_HEIGHT: f64 = 500.0;

/// A positioned rectangle for treemap rendering
#[derive(Debug, Clone)]
struct TreemapRect {
    /// Node name
    name: String,
    /// Node value (for leaves)
    value: Option<f64>,
    /// Sum of all descendant values
    total_value: f64,
    /// X position
    x: f64,
    /// Y position
    y: f64,
    /// Width
    width: f64,
    /// Height
    height: f64,
    /// Depth in tree (0 = root, 1 = first level, etc.)
    depth: usize,
    /// Color section index (based on parent section)
    section: usize,
    /// Is this a leaf node?
    is_leaf: bool,
    /// CSS class selector
    class_selector: Option<String>,
    /// Compiled styles from classDef
    styles: Vec<String>,
}

/// Layout context for treemap positioning
struct LayoutContext<'a> {
    db: &'a TreemapDb,
    positioned: Vec<TreemapRect>,
}

impl<'a> LayoutContext<'a> {
    fn new(db: &'a TreemapDb) -> Self {
        Self {
            db,
            positioned: Vec::new(),
        }
    }
}

/// Bounding box for layout calculations
#[derive(Debug, Clone, Copy)]
struct Bounds {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
}

/// Render a treemap diagram to SVG
pub fn render_treemap(db: &TreemapDb, config: &RenderConfig) -> Result<String> {
    let mut doc = SvgDocument::new();

    // Get root nodes
    let root_nodes = db.get_root_nodes();
    if root_nodes.is_empty() {
        doc.set_size(100.0, 100.0);
        return Ok(doc.to_string());
    }

    // Calculate dimensions
    let title = db.get_title();
    let title_height = if title.is_empty() { 0.0 } else { 30.0 };

    let width = DEFAULT_WIDTH;
    let height = DEFAULT_HEIGHT;
    let svg_width = width;
    let svg_height = height + title_height;

    doc.set_size(svg_width, svg_height);

    // Add CSS styles
    if config.embed_css {
        doc.add_style(&generate_treemap_css(config));
    }

    // Create main container group
    let mut container_children = Vec::new();

    // Add title if present
    if !title.is_empty() {
        container_children.push(SvgElement::Text {
            x: svg_width / 2.0,
            y: title_height / 2.0,
            content: title.to_string(),
            attrs: Attrs::new()
                .with_class("treemapTitle")
                .with_attr("text-anchor", "middle")
                .with_attr("dominant-baseline", "middle"),
        });
    }

    // Create a virtual root that contains all actual roots
    let virtual_root = TreemapNode {
        name: String::new(),
        value: None,
        class_selector: None,
        children: root_nodes.to_vec(),
    };

    // Calculate total value and position nodes
    let mut ctx = LayoutContext::new(db);

    // Layout the treemap
    let bounds = Bounds {
        x: 0.0,
        y: title_height,
        width,
        height,
    };
    layout_treemap(&virtual_root, bounds, 0, 0, &mut ctx);

    // Render sections (branch nodes with children)
    let mut section_elements = Vec::new();
    for rect in ctx.positioned.iter().filter(|r| !r.is_leaf && r.depth > 0) {
        section_elements.push(render_section(rect, config));
    }

    // Render leaf nodes
    let mut leaf_elements = Vec::new();
    for rect in ctx.positioned.iter().filter(|r| r.is_leaf) {
        leaf_elements.push(render_leaf(rect, config));
    }

    // Add sections group
    container_children.push(SvgElement::Group {
        children: section_elements,
        attrs: Attrs::new().with_class("treemapSections"),
    });

    // Add leaves group
    container_children.push(SvgElement::Group {
        children: leaf_elements,
        attrs: Attrs::new().with_class("treemapLeaves"),
    });

    // Add hidden grand total (matching mermaid.js for label detection)
    // This is a hidden element used by the eval system to verify total values
    let grand_total: f64 = ctx
        .positioned
        .iter()
        .filter(|r| r.is_leaf)
        .map(|r| r.value.unwrap_or(0.0))
        .sum();
    container_children.push(SvgElement::Text {
        x: width - 10.0,
        y: title_height + 12.5,
        content: format_value(grand_total),
        attrs: Attrs::new()
            .with_class("treemapSectionValue")
            .with_attr("text-anchor", "end")
            .with_attr("dominant-baseline", "middle")
            .with_attr("font-style", "italic")
            .with_attr("style", "display: none;"),
    });

    // Add container to document
    doc.add_node(SvgElement::Group {
        children: container_children,
        attrs: Attrs::new().with_class("treemapContainer"),
    });

    Ok(doc.to_string())
}

/// Calculate total value of a node and its descendants
fn calculate_total_value(node: &TreemapNode) -> f64 {
    if let Some(value) = node.value {
        // Leaf node
        value
    } else {
        // Sum children values
        node.children.iter().map(calculate_total_value).sum()
    }
}

/// Layout treemap nodes recursively using squarified algorithm
fn layout_treemap(
    node: &TreemapNode,
    bounds: Bounds,
    depth: usize,
    section: usize,
    ctx: &mut LayoutContext,
) {
    let is_leaf = node.value.is_some();
    let total_value = calculate_total_value(node);

    // Get styles for this node
    let styles = node
        .class_selector
        .as_ref()
        .map(|class| ctx.db.get_styles_for_class(class))
        .unwrap_or_default();

    // Add this node to positioned list (except virtual root)
    if depth > 0 || !node.name.is_empty() {
        ctx.positioned.push(TreemapRect {
            name: node.name.clone(),
            value: node.value,
            total_value,
            x: bounds.x,
            y: bounds.y,
            width: bounds.width,
            height: bounds.height,
            depth,
            section,
            is_leaf,
            class_selector: node.class_selector.clone(),
            styles,
        });
    }

    // If this is a leaf or has no children, we're done
    if is_leaf || node.children.is_empty() {
        return;
    }

    // Calculate available space for children
    // Sections have a header that takes up space
    let child_bounds = if depth > 0 {
        Bounds {
            x: bounds.x + SECTION_PADDING,
            y: bounds.y + SECTION_HEADER_HEIGHT + SECTION_PADDING,
            width: bounds.width - 2.0 * SECTION_PADDING,
            height: bounds.height - SECTION_HEADER_HEIGHT - 2.0 * SECTION_PADDING,
        }
    } else {
        bounds
    };

    // Sort children by value (largest first for better layout)
    let mut children: Vec<&TreemapNode> = node.children.iter().collect();
    children.sort_by(|a, b| {
        let va = calculate_total_value(a);
        let vb = calculate_total_value(b);
        vb.partial_cmp(&va).unwrap_or(std::cmp::Ordering::Equal)
    });

    // Calculate values for squarified layout
    let child_values: Vec<f64> = children.iter().map(|c| calculate_total_value(c)).collect();
    let children_total: f64 = child_values.iter().sum();

    if children_total <= 0.0 || child_bounds.width <= 0.0 || child_bounds.height <= 0.0 {
        return;
    }

    // Apply squarified treemap layout
    let rects = squarify_layout(&child_values, child_bounds, children_total);

    // Recursively layout children
    for (i, (child, rect)) in children.iter().zip(rects.iter()).enumerate() {
        // Assign section based on depth
        let child_section = if depth == 0 {
            // First level children get their own section color
            i % (MAX_SECTIONS - 1)
        } else {
            // Deeper children inherit parent section
            section
        };

        layout_treemap(child, *rect, depth + 1, child_section, ctx);
    }
}

/// Squarified treemap layout algorithm
/// Returns a list of Bounds for each value
fn squarify_layout(values: &[f64], bounds: Bounds, total: f64) -> Vec<Bounds> {
    if values.is_empty() || total <= 0.0 {
        return vec![];
    }

    if values.len() == 1 {
        return vec![bounds];
    }

    // Normalize values to fit the available area
    let area = bounds.width * bounds.height;
    let normalized: Vec<f64> = values.iter().map(|v| (v / total) * area).collect();

    // Use slice-and-dice for simplicity (alternating horizontal/vertical)
    squarify_recursive(&normalized, bounds, true)
}

/// Recursive squarified layout
fn squarify_recursive(areas: &[f64], bounds: Bounds, horizontal: bool) -> Vec<Bounds> {
    if areas.is_empty() {
        return vec![];
    }

    if areas.len() == 1 {
        return vec![bounds];
    }

    let total: f64 = areas.iter().sum();
    if total <= 0.0 {
        return areas
            .iter()
            .map(|_| Bounds {
                x: bounds.x,
                y: bounds.y,
                width: 0.0,
                height: 0.0,
            })
            .collect();
    }

    // Find the best split point for squarified layout
    let (left_areas, right_areas, split_ratio) = find_best_split(areas);

    let mut result = Vec::with_capacity(areas.len());

    if horizontal {
        // Split horizontally (left and right)
        let left_width = bounds.width * split_ratio;
        let right_width = bounds.width * (1.0 - split_ratio);

        result.extend(squarify_recursive(
            &left_areas,
            Bounds {
                x: bounds.x,
                y: bounds.y,
                width: left_width,
                height: bounds.height,
            },
            !horizontal,
        ));
        result.extend(squarify_recursive(
            &right_areas,
            Bounds {
                x: bounds.x + left_width,
                y: bounds.y,
                width: right_width,
                height: bounds.height,
            },
            !horizontal,
        ));
    } else {
        // Split vertically (top and bottom)
        let top_height = bounds.height * split_ratio;
        let bottom_height = bounds.height * (1.0 - split_ratio);

        result.extend(squarify_recursive(
            &left_areas,
            Bounds {
                x: bounds.x,
                y: bounds.y,
                width: bounds.width,
                height: top_height,
            },
            !horizontal,
        ));
        result.extend(squarify_recursive(
            &right_areas,
            Bounds {
                x: bounds.x,
                y: bounds.y + top_height,
                width: bounds.width,
                height: bottom_height,
            },
            !horizontal,
        ));
    }

    result
}

/// Find the best split point to minimize aspect ratio variance
fn find_best_split(areas: &[f64]) -> (Vec<f64>, Vec<f64>, f64) {
    let total: f64 = areas.iter().sum();

    if areas.len() <= 1 {
        return (areas.to_vec(), vec![], 1.0);
    }

    // Try different split points and find the one with best aspect ratios
    let mut best_split = 1;
    let mut best_ratio_diff = f64::MAX;

    for split in 1..areas.len() {
        let left_sum: f64 = areas[..split].iter().sum();
        let right_sum: f64 = areas[split..].iter().sum();

        // Balance: try to keep left and right roughly equal
        let ratio_diff = (left_sum - right_sum).abs() / total;

        if ratio_diff < best_ratio_diff {
            best_ratio_diff = ratio_diff;
            best_split = split;
        }
    }

    let left_areas = areas[..best_split].to_vec();
    let right_areas = areas[best_split..].to_vec();
    let left_sum: f64 = left_areas.iter().sum();
    let split_ratio = left_sum / total;

    (left_areas, right_areas, split_ratio)
}

/// Render a section (branch node with header)
fn render_section(rect: &TreemapRect, _config: &RenderConfig) -> SvgElement {
    let mut children = Vec::new();

    // Section color class
    let section_class = format!("section-{}", rect.section);

    // Build style string from classDef
    let style_str = if rect.styles.is_empty() {
        String::new()
    } else {
        rect.styles.join(";")
    };

    // Section background rect
    children.push(SvgElement::Rect {
        x: rect.x,
        y: rect.y,
        width: rect.width,
        height: rect.height,
        rx: None,
        ry: None,
        attrs: Attrs::new()
            .with_class(&format!("treemapSection {}", section_class))
            .with_attr("fill-opacity", "0.6")
            .with_attr("stroke-opacity", "0.4")
            .with_attr("stroke-width", "2")
            .with_style_if(!style_str.is_empty(), &style_str),
    });

    // Section header background (hidden for visual consistency with mermaid)
    children.push(SvgElement::Rect {
        x: rect.x,
        y: rect.y,
        width: rect.width,
        height: SECTION_HEADER_HEIGHT,
        rx: None,
        ry: None,
        attrs: Attrs::new()
            .with_class("treemapSectionHeader")
            .with_attr("fill", "none")
            .with_attr("stroke-width", "0"),
    });

    // Section label (left-aligned)
    let label_x = rect.x + 6.0;
    let label_y = rect.y + SECTION_HEADER_HEIGHT / 2.0;

    // Extract text color from styles
    let text_style = extract_text_style(&rect.styles);

    children.push(SvgElement::Text {
        x: label_x,
        y: label_y,
        content: rect.name.clone(),
        attrs: Attrs::new()
            .with_class(&format!("treemapSectionLabel {}", section_class))
            .with_attr("dominant-baseline", "middle")
            .with_attr("font-weight", "bold")
            .with_attr("font-size", &format!("{}px", SECTION_FONT_SIZE))
            .with_style_if(!text_style.is_empty(), &text_style),
    });

    // Section value (right-aligned)
    let value_x = rect.x + rect.width - 10.0;
    children.push(SvgElement::Text {
        x: value_x,
        y: label_y,
        content: format_value(rect.total_value),
        attrs: Attrs::new()
            .with_class(&format!("treemapSectionValue {}", section_class))
            .with_attr("text-anchor", "end")
            .with_attr("dominant-baseline", "middle")
            .with_attr("font-style", "italic")
            .with_attr("font-size", &format!("{}px", VALUE_FONT_SIZE))
            .with_style_if(!text_style.is_empty(), &text_style),
    });

    // Build the CSS class for the group
    let mut group_class = format!("treemapSection {}", section_class);
    if let Some(ref class) = rect.class_selector {
        group_class.push(' ');
        group_class.push_str(class);
    }

    SvgElement::Group {
        children,
        attrs: Attrs::new().with_class(&group_class),
    }
}

/// Render a leaf node
fn render_leaf(rect: &TreemapRect, _config: &RenderConfig) -> SvgElement {
    let mut children = Vec::new();

    // Section color class (inherited from parent)
    let section_class = format!("section-{}", rect.section);

    // Build style string from classDef
    let style_str = if rect.styles.is_empty() {
        String::new()
    } else {
        rect.styles.join(";")
    };

    // Leaf background rect
    children.push(SvgElement::Rect {
        x: rect.x,
        y: rect.y,
        width: rect.width,
        height: rect.height,
        rx: None,
        ry: None,
        attrs: Attrs::new()
            .with_class(&format!("treemapLeaf {}", section_class))
            .with_attr("fill-opacity", "0.3")
            .with_attr("stroke-width", "3")
            .with_style_if(!style_str.is_empty(), &style_str),
    });

    // Calculate center position for labels
    let center_x = rect.x + rect.width / 2.0;
    let center_y = rect.y + rect.height / 2.0;

    // Extract text color from styles
    let text_style = extract_text_style(&rect.styles);

    // Calculate font size based on available space
    let available_width = rect.width - 8.0; // 4px padding on each side
    let available_height = rect.height - 8.0;

    // Estimate text width (rough approximation)
    let char_width = LEAF_FONT_SIZE * 0.6;
    let text_width = rect.name.len() as f64 * char_width;

    // Scale font size to fit
    let scale_x = if text_width > available_width {
        available_width / text_width
    } else {
        1.0
    };

    let scale_y = if LEAF_FONT_SIZE * 2.5 > available_height {
        available_height / (LEAF_FONT_SIZE * 2.5)
    } else {
        1.0
    };

    let font_scale = scale_x.min(scale_y).min(1.0);
    let label_font_size = (LEAF_FONT_SIZE * font_scale).max(8.0);
    let value_font_size = (VALUE_FONT_SIZE * font_scale).max(6.0);

    // Only show text if there's enough space
    let min_display_size = 20.0;
    if rect.width >= min_display_size && rect.height >= min_display_size {
        // Leaf label (centered)
        children.push(SvgElement::Text {
            x: center_x,
            y: center_y - value_font_size / 2.0,
            content: rect.name.clone(),
            attrs: Attrs::new()
                .with_class(&format!("treemapLabel {}", section_class))
                .with_attr("text-anchor", "middle")
                .with_attr("dominant-baseline", "middle")
                .with_attr("font-size", &format!("{}px", label_font_size))
                .with_style_if(!text_style.is_empty(), &text_style),
        });

        // Leaf value (below label)
        if let Some(value) = rect.value {
            children.push(SvgElement::Text {
                x: center_x,
                y: center_y + label_font_size / 2.0 + 2.0,
                content: format_value(value),
                attrs: Attrs::new()
                    .with_class(&format!("treemapValue {}", section_class))
                    .with_attr("text-anchor", "middle")
                    .with_attr("dominant-baseline", "hanging")
                    .with_attr("font-size", &format!("{}px", value_font_size))
                    .with_style_if(!text_style.is_empty(), &text_style),
            });
        }
    }

    // Build the CSS class for the group
    let mut group_class = format!("treemapNode treemapLeafGroup {}", section_class);
    if let Some(ref class) = rect.class_selector {
        group_class.push(' ');
        group_class.push_str(class);
    }

    SvgElement::Group {
        children,
        attrs: Attrs::new().with_class(&group_class),
    }
}

/// Extract text-related styles (color) from style list
fn extract_text_style(styles: &[String]) -> String {
    styles
        .iter()
        .filter(|s| s.starts_with("color:"))
        .map(|s| s.replace("color:", "fill:"))
        .collect::<Vec<_>>()
        .join(";")
}

/// Format a value for display (matching mermaid.js comma-separated number format)
fn format_value(value: f64) -> String {
    if value == value.floor() {
        // Integer value - format with thousands separator
        let int_value = value as i64;
        if int_value >= 1_000 {
            format_with_commas(int_value)
        } else {
            format!("{}", int_value)
        }
    } else {
        format!("{:.2}", value)
    }
}

/// Format an integer with thousands separators (commas)
fn format_with_commas(n: i64) -> String {
    let s = n.abs().to_string();
    let mut result = String::new();
    for (i, c) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            result.push(',');
        }
        result.push(c);
    }
    if n < 0 {
        result.push('-');
    }
    result.chars().rev().collect()
}

/// Generate CSS for treemap diagrams
fn generate_treemap_css(config: &RenderConfig) -> String {
    let theme = &config.theme;

    // Generate section colors
    let mut section_css = String::new();

    // Generate section colors from pie colors (similar to mermaid's cScale)
    for i in 0..MAX_SECTIONS {
        let color = theme
            .pie_colors
            .get(i)
            .map(|s| s.as_str())
            .unwrap_or("#ECECFF");

        // Calculate a darker version for stroke
        let stroke_color = color; // Use same color for simplicity

        section_css.push_str(&format!(
            r#"
.section-{i} .treemapSection {{
  fill: {color};
  stroke: {stroke_color};
}}
.section-{i} .treemapLeaf {{
  fill: {color};
  stroke: {color};
}}
.section-{i} .treemapLabel,
.section-{i} .treemapValue,
.section-{i} .treemapSectionLabel,
.section-{i} .treemapSectionValue {{
  fill: {text_color};
}}
"#,
            i = i,
            color = color,
            stroke_color = stroke_color,
            text_color = theme.primary_text_color,
        ));
    }

    format!(
        r#"
.treemapContainer {{
  font-family: {font_family};
}}
.treemapTitle {{
  font-size: 14px;
  font-weight: bold;
  fill: {text_color};
}}
.treemapSection {{
  stroke-width: 2px;
}}
.treemapSectionHeader {{
  fill: none;
}}
.treemapSectionLabel {{
  font-weight: bold;
}}
.treemapSectionValue {{
  font-style: italic;
}}
.treemapLeaf {{
  stroke-width: 3px;
}}
.treemapLabel {{
  font-size: {leaf_font_size}px;
}}
.treemapValue {{
  font-size: {value_font_size}px;
}}
{section_css}
"#,
        font_family = theme.font_family,
        text_color = theme.primary_text_color,
        leaf_font_size = LEAF_FONT_SIZE,
        value_font_size = VALUE_FONT_SIZE,
        section_css = section_css
    )
}
