//! Treemap diagram renderer
//!
//! Renders treemap diagrams using a squarified layout algorithm.
//! Section nodes (branches) get headers with labels.
//! Leaf nodes fill the remaining space with values displayed.

use crate::diagrams::treemap::{TreemapDb, TreemapNode};
use crate::error::Result;
use crate::render::svg::color::{contrasting_text, Color};
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

/// Font size for leaf labels (matches mermaid.js default of 38px)
const LEAF_FONT_SIZE: f64 = 38.0;

/// Font size for section labels
const SECTION_FONT_SIZE: f64 = 12.0;

/// Font size for section value labels (smaller than leaf values)
const SECTION_VALUE_FONT_SIZE: f64 = 10.0;

/// Font size for leaf value labels (approximately 60% of label font, matching mermaid.js ~23px)
const VALUE_FONT_SIZE: f64 = 23.0;

/// Default diagram width
const DEFAULT_WIDTH: f64 = 960.0;

/// Default diagram height (matches mermaid.js ~400px content area)
const DEFAULT_HEIGHT: f64 = 400.0;

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
    for (idx, rect) in ctx
        .positioned
        .iter()
        .filter(|r| !r.is_leaf && r.depth > 0)
        .enumerate()
    {
        section_elements.push(render_section(rect, idx, config));
    }

    // Render leaf nodes
    let mut leaf_elements = Vec::new();
    for (idx, rect) in ctx.positioned.iter().filter(|r| r.is_leaf).enumerate() {
        leaf_elements.push(render_leaf(rect, idx, config));
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
    // All depths apply padding for section headers (including depth=0 for outer padding)
    // This matches mermaid.js where the first visible section is inset from the canvas
    let child_bounds = Bounds {
        x: bounds.x + SECTION_PADDING,
        y: bounds.y + SECTION_HEADER_HEIGHT + SECTION_PADDING,
        width: bounds.width - 2.0 * SECTION_PADDING,
        height: bounds.height - SECTION_HEADER_HEIGHT - 2.0 * SECTION_PADDING,
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

    // Choose layout algorithm based on depth and node type
    // Mermaid.js uses horizontal strip layout for top-level sections
    // For leaves, it uses a squarified layout that considers aspect ratios
    let all_children_are_sections = children.iter().all(|c| !c.children.is_empty());
    let all_children_are_leaves = children.iter().all(|c| c.value.is_some());

    let rects = if depth <= 1 && all_children_are_sections {
        // Top-level sections: use horizontal strip layout (width proportional to value)
        horizontal_strip_layout(&child_values, child_bounds, children_total)
    } else if all_children_are_leaves && children.len() == 2 {
        // For exactly 2 leaves, choose layout based on container aspect ratio
        // Wide containers -> horizontal strip (better aspect ratios)
        // Tall containers -> vertical strip (better aspect ratios)
        if child_bounds.width >= child_bounds.height * 1.2 {
            horizontal_strip_layout(&child_values, child_bounds, children_total)
        } else {
            vertical_strip_layout(&child_values, child_bounds, children_total)
        }
    } else {
        // Use squarified layout for 3+ leaves or mixed content
        squarify_layout(&child_values, child_bounds, children_total)
    };

    // Recursively layout children
    for (i, (child, rect)) in children.iter().zip(rects.iter()).enumerate() {
        // Determine if this child is a section (has children) or a leaf
        let child_is_section = !child.children.is_empty();

        // Assign section based on depth and node type
        // Section colors should be assigned to branch nodes (sections), not leaves
        // depth=0 is the virtual root, depth=1+ are visible levels
        let child_section = if child_is_section && depth == 0 {
            // Children of virtual root (first visible level) get their own section
            i % (MAX_SECTIONS - 1)
        } else if child_is_section && depth == 1 {
            // Children of first-level sections (like subsections) get offset section colors
            // This ensures they don't conflict with their parent's section
            (i + 1) % (MAX_SECTIONS - 1)
        } else {
            // Leaves and deeper children inherit parent section
            section
        };

        layout_treemap(child, *rect, depth + 1, child_section, ctx);
    }
}

/// Horizontal strip layout - arranges all items in a single row with widths proportional to values
/// This is used for top-level sections to match mermaid.js behavior
fn horizontal_strip_layout(values: &[f64], bounds: Bounds, total: f64) -> Vec<Bounds> {
    if values.is_empty() || total <= 0.0 {
        return vec![];
    }

    let mut result = Vec::with_capacity(values.len());
    let mut x = bounds.x;

    for (i, value) in values.iter().enumerate() {
        let ratio = value / total;
        let width = if i == values.len() - 1 {
            // Last item takes remaining width to avoid floating point gaps
            bounds.x + bounds.width - x
        } else {
            bounds.width * ratio
        };

        result.push(Bounds {
            x,
            y: bounds.y,
            width,
            height: bounds.height,
        });

        x += width;
    }

    result
}

/// Vertical strip layout - arranges all items in a single column with heights proportional to values
/// This is used when vertical arrangement gives better aspect ratios
fn vertical_strip_layout(values: &[f64], bounds: Bounds, total: f64) -> Vec<Bounds> {
    if values.is_empty() || total <= 0.0 {
        return vec![];
    }

    let mut result = Vec::with_capacity(values.len());
    let mut y = bounds.y;

    for (i, value) in values.iter().enumerate() {
        let ratio = value / total;
        let height = if i == values.len() - 1 {
            // Last item takes remaining height to avoid floating point gaps
            bounds.y + bounds.height - y
        } else {
            bounds.height * ratio
        };

        result.push(Bounds {
            x: bounds.x,
            y,
            width: bounds.width,
            height,
        });

        y += height;
    }

    result
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

    // Start with direction based on aspect ratio:
    // - If wider than tall, start with horizontal=true (horizontal split, items side by side)
    // - If taller than wide, start with horizontal=false (vertical split, items stacked)
    // This creates more square-like rectangles in the available space
    let start_horizontal = bounds.width >= bounds.height;
    squarify_recursive(&normalized, bounds, start_horizontal)
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

/// Find the best split point using mermaid-style squarify heuristic
/// Groups larger items together to create more visually balanced treemaps
fn find_best_split(areas: &[f64]) -> (Vec<f64>, Vec<f64>, f64) {
    let total: f64 = areas.iter().sum();

    if areas.len() <= 1 {
        return (areas.to_vec(), vec![], 1.0);
    }

    if areas.len() == 2 {
        // For 2 items, just split them
        let split_ratio = areas[0] / total;
        return (vec![areas[0]], vec![areas[1]], split_ratio);
    }

    // For 3+ items, mermaid-style: prefer keeping the largest items together
    // This creates layouts with one large block and one smaller block
    // Score: prefer splits where left side has higher total (grouping large items)
    let mut best_split = 1;
    let mut best_score = f64::MIN;

    for split in 1..areas.len() {
        let left_sum: f64 = areas[..split].iter().sum();
        let right_sum: f64 = areas[split..].iter().sum();
        let left_count = split;
        let right_count = areas.len() - split;

        // Mermaid tends to:
        // 1. Group multiple items together when their total is larger
        // 2. Isolate single items when they're relatively small
        // Score formula: prefer splits where we have many items on one side
        // and their combined value makes sense

        let left_ratio = left_sum / total;
        let right_ratio = right_sum / total;

        // Prefer splits where:
        // - The larger group (by value) has more items
        // - Single items are isolated only when they're small
        let score = if left_count > right_count {
            // More items on left - prefer if left has larger total
            left_ratio - right_ratio * 0.5
        } else if right_count > left_count {
            // More items on right - prefer if right has larger total
            right_ratio - left_ratio * 0.5
        } else {
            // Equal counts - prefer balanced value split
            -((left_ratio - 0.5).abs())
        };

        if score > best_score {
            best_score = score;
            best_split = split;
        }
    }

    let left_areas = areas[..best_split].to_vec();
    let right_areas = areas[best_split..].to_vec();
    let left_sum: f64 = left_areas.iter().sum();
    let split_ratio = left_sum / total;

    (left_areas, right_areas, split_ratio)
}

/// Parse an HSL string like "hsl(240, 100%, 96%)" into (h, s, l) components
fn parse_hsl_string(hsl: &str) -> Option<(f64, f64, f64)> {
    let trimmed = hsl.trim();
    if !trimmed.starts_with("hsl(") || !trimmed.ends_with(')') {
        return None;
    }
    let inner = &trimmed[4..trimmed.len() - 1];
    let parts: Vec<&str> = inner.split(',').map(|s| s.trim()).collect();
    if parts.len() != 3 {
        return None;
    }
    let h: f64 = parts[0].parse().ok()?;
    let s: f64 = parts[1].trim_end_matches('%').parse().ok()?;
    let l: f64 = parts[2].trim_end_matches('%').parse().ok()?;
    Some((h, s, l))
}

/// Format lightness percentage matching mermaid's precision (10 significant digits)
fn format_lightness(value: f64) -> String {
    // Mermaid uses 10 decimal places, trim trailing zeros
    format!("{:.10}", value)
        .trim_end_matches('0')
        .trim_end_matches('.')
        .to_string()
}

/// Get the fill color for a section index from the theme
/// Generates colors matching mermaid.js treemap cScale (distinct from pie colors)
fn get_section_fill_color(section: usize, _config: &RenderConfig) -> String {
    // Mermaid treemap uses a cScale with evenly distributed hues at ~76% lightness
    // Base hue starts at 240 (blue) and increments for each section
    // This matches the reference output:
    // - section 0: hsl(240, 100%, 76.2745098039%)
    // - section 1: hsl(60, 100%, 73.5294117647%)
    // - section 2: hsl(80, 100%, 76.2745098039%)
    // - section 3: hsl(270, 100%, 76.2745098039%)
    // etc.

    // Define the hue sequence to match mermaid's cScale pattern
    let hues = [
        240.0, // blue (index 0)
        60.0,  // yellow (index 1)
        80.0,  // yellow-green (index 2)
        270.0, // purple (index 3)
        0.0,   // red (index 4)
        180.0, // cyan (index 5)
        300.0, // magenta (index 6)
        120.0, // green (index 7)
        30.0,  // orange (index 8)
        150.0, // teal (index 9)
        210.0, // sky blue (index 10)
        330.0, // pink (index 11)
    ];

    // Mermaid uses slightly different lightness values
    // 76.2745098039% = 195/255 * 100 and 73.5294117647% = 187.5/255 * 100
    let (lightness, hue) = if section < hues.len() {
        let l = if section == 1 {
            73.5294117647 // yellow section has slightly lower lightness
        } else {
            76.2745098039
        };
        (l, hues[section])
    } else {
        // Fall back to computed hue for additional sections
        let h = ((section as f64) * 30.0) % 360.0;
        (76.2745098039, h)
    };

    format!(
        "hsl({}, {}%, {}%)",
        hue.round() as i32,
        100,
        format_lightness(lightness)
    )
}

/// Get contrasting text color (white or #333) for a given fill color string
fn get_text_color_for_fill(fill_color: &str) -> String {
    // Try parsing as HSL string first
    if let Some((h, s, l)) = parse_hsl_string(fill_color) {
        let color = Color::from_hsl(h, s, l);
        let text = contrasting_text(&color);
        if text.r == 255 {
            return "#ffffff".to_string();
        } else {
            return "#333".to_string();
        }
    }

    // Fall back to hex parsing
    if let Some(color) = Color::from_hex(fill_color) {
        let text = contrasting_text(&color);
        if text.r == 255 {
            return "#ffffff".to_string();
        } else {
            return "#333".to_string();
        }
    }

    // Default to #333 text (standard mermaid dark text color)
    "#333".to_string()
}

/// Render a section (branch node with header)
fn render_section(rect: &TreemapRect, index: usize, config: &RenderConfig) -> SvgElement {
    let mut children = Vec::new();

    // Section color class
    let section_class = format!("section-{}", rect.section);

    // Get the inline fill color for this section
    let fill_color = get_section_fill_color(rect.section, config);

    // Get contrasting text color for the section background
    let text_color = get_text_color_for_fill(&fill_color);

    // Build style string from classDef
    let style_str = if rect.styles.is_empty() {
        String::new()
    } else {
        rect.styles.join(";")
    };

    // Section background rect with inline fill color
    children.push(SvgElement::Rect {
        x: rect.x,
        y: rect.y,
        width: rect.width,
        height: rect.height,
        rx: None,
        ry: None,
        attrs: Attrs::new()
            .with_class(&format!("treemapSection {}", section_class))
            .with_fill(&fill_color)
            .with_stroke(&fill_color)
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

    // ClipPath for section header text overflow handling
    let clip_id = format!("clip-section-{}", index);
    let clip_width = (rect.width - 12.0).max(0.0); // 6px padding on each side
    children.push(SvgElement::Raw {
        content: format!(
            "<clipPath id=\"{}\"><rect width=\"{}\" height=\"{}\"/></clipPath>",
            clip_id, clip_width, SECTION_HEADER_HEIGHT
        ),
    });

    // Section label (left-aligned)
    let label_x = rect.x + 6.0;
    let label_y = rect.y + SECTION_HEADER_HEIGHT / 2.0;

    // Extract text color from styles (classDef can override)
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
            .with_fill(&text_color)
            .with_style_if(!text_style.is_empty(), &text_style),
    });

    // Section value (right-aligned, uses smaller font for section headers)
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
            .with_attr("font-size", &format!("{}px", SECTION_VALUE_FONT_SIZE))
            .with_fill(&text_color)
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
fn render_leaf(rect: &TreemapRect, index: usize, config: &RenderConfig) -> SvgElement {
    let mut children = Vec::new();

    // Section color class (inherited from parent)
    let section_class = format!("section-{}", rect.section);

    // Get the inline fill color for this leaf (inherits from parent section)
    let fill_color = get_section_fill_color(rect.section, config);

    // Get contrasting text color for the leaf background
    let text_color = get_text_color_for_fill(&fill_color);

    // Build style string from classDef
    let style_str = if rect.styles.is_empty() {
        String::new()
    } else {
        rect.styles.join(";")
    };

    // Leaf background rect with inline fill color
    children.push(SvgElement::Rect {
        x: rect.x,
        y: rect.y,
        width: rect.width,
        height: rect.height,
        rx: None,
        ry: None,
        attrs: Attrs::new()
            .with_class(&format!("treemapLeaf {}", section_class))
            .with_fill(&fill_color)
            .with_stroke(&fill_color)
            .with_attr("fill-opacity", "0.3")
            .with_attr("stroke-width", "3")
            .with_style_if(!style_str.is_empty(), &style_str),
    });

    // ClipPath for text overflow handling (matching mermaid.js)
    // Position the clipPath rect at the leaf's position since we use absolute coordinates
    let clip_id = format!("clip-leaf-{}", index);
    let clip_width = (rect.width - 4.0).max(0.0);
    let clip_height = (rect.height - 4.0).max(0.0);
    children.push(SvgElement::Raw {
        content: format!(
            "<clipPath id=\"{}\"><rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\"/></clipPath>",
            clip_id,
            rect.x + 2.0,
            rect.y + 2.0,
            clip_width,
            clip_height
        ),
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
    let label_font_size = (LEAF_FONT_SIZE * font_scale).max(10.0);
    let value_font_size = (VALUE_FONT_SIZE * font_scale).max(10.0);

    // Only show text if there's enough space
    let min_display_size = 20.0;
    if rect.width >= min_display_size && rect.height >= min_display_size {
        // Calculate positions to center label + value together
        // The label and value form a text block that should be centered
        let gap = 2.0; // Gap between label and value
        let total_text_height = label_font_size + gap + value_font_size;
        let label_y = center_y - total_text_height / 2.0 + label_font_size / 2.0;
        let value_y = label_y + label_font_size / 2.0 + gap;

        // Leaf label (centered) with clip-path reference
        children.push(SvgElement::Text {
            x: center_x,
            y: label_y,
            content: rect.name.clone(),
            attrs: Attrs::new()
                .with_class(&format!("treemapLabel {}", section_class))
                .with_attr("text-anchor", "middle")
                .with_attr("dominant-baseline", "middle")
                .with_attr("font-size", &format!("{}px", label_font_size))
                .with_attr("clip-path", &format!("url(#{})", clip_id))
                .with_fill(&text_color)
                .with_style_if(!text_style.is_empty(), &text_style),
        });

        // Leaf value (below label) with clip-path reference
        if let Some(value) = rect.value {
            children.push(SvgElement::Text {
                x: center_x,
                y: value_y,
                content: format_value(value),
                attrs: Attrs::new()
                    .with_class(&format!("treemapValue {}", section_class))
                    .with_attr("text-anchor", "middle")
                    .with_attr("dominant-baseline", "hanging")
                    .with_attr("font-size", &format!("{}px", value_font_size))
                    .with_attr("clip-path", &format!("url(#{})", clip_id))
                    .with_fill(&text_color)
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

    // Note: Section and leaf colors are set inline based on the section index,
    // and text colors are calculated inline based on background contrast.
    // This matches mermaid.js which uses inline styles rather than CSS classes.
    // We keep section classes for potential custom styling but don't set fills in CSS.

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
"#,
        font_family = theme.font_family,
        text_color = theme.primary_text_color,
        leaf_font_size = LEAF_FONT_SIZE,
        value_font_size = VALUE_FONT_SIZE,
    )
}
