//! Mindmap diagram renderer
//!
//! Renders mindmap diagrams using a radial tree layout.

use crate::diagrams::mindmap::{MindmapDb, MindmapNode, NodeType};
use crate::error::Result;
use crate::render::svg::{Attrs, RenderConfig, SvgDocument, SvgElement};

/// Padding around nodes
const NODE_PADDING: f64 = 10.0;

/// Minimum node width
const MIN_NODE_WIDTH: f64 = 50.0;

/// Minimum node height
const MIN_NODE_HEIGHT: f64 = 30.0;

/// Horizontal spacing between nodes
const NODE_SPACING_H: f64 = 80.0;

/// Vertical spacing between sibling nodes
const NODE_SPACING_V: f64 = 40.0;

/// Maximum number of color sections (matches mermaid.js)
const MAX_SECTIONS: usize = 12;

/// Font size for node labels
const FONT_SIZE: f64 = 14.0;

/// Character width estimate for text sizing
const CHAR_WIDTH: f64 = 8.0;

/// A positioned node for rendering
#[derive(Debug, Clone)]
#[allow(dead_code)]
struct PositionedNode {
    /// Unique ID for this node
    id: String,
    /// Display text
    text: String,
    /// Node type (shape)
    node_type: NodeType,
    /// X position (center)
    x: f64,
    /// Y position (center)
    y: f64,
    /// Width
    width: f64,
    /// Height
    height: f64,
    /// Section/color index
    section: i32,
    /// Parent node ID (if any)
    parent_id: Option<String>,
    /// Icon name (if any)
    icon: Option<String>,
    /// CSS class (if any)
    class: Option<String>,
}

/// Render a mindmap diagram to SVG
pub fn render_mindmap(db: &MindmapDb, config: &RenderConfig) -> Result<String> {
    let mut doc = SvgDocument::new();

    // Get the root node
    let root = match db.get_mindmap() {
        Some(root) => root,
        None => {
            // Empty mindmap
            doc.set_size(100.0, 100.0);
            return Ok(doc.to_string());
        }
    };

    // Position all nodes using tree layout
    let mut positioned_nodes = Vec::new();
    let mut node_counter = 0;
    position_tree(
        root,
        0.0,
        0.0,
        -1, // Root section is -1 (section-root)
        None,
        &mut positioned_nodes,
        &mut node_counter,
    );

    // Calculate bounds
    let (min_x, max_x, min_y, max_y) = calculate_bounds(&positioned_nodes);
    let padding = 20.0;
    let width = (max_x - min_x) + padding * 2.0;
    let height = (max_y - min_y) + padding * 2.0;

    doc.set_size_with_origin(min_x - padding, min_y - padding, width, height);

    // Add CSS styles
    if config.embed_css {
        doc.add_style(&generate_mindmap_css(config));
    }

    // Render edges first (behind nodes)
    let edges_group = render_edges(&positioned_nodes, config);
    doc.add_edge_path(edges_group);

    // Render nodes
    let nodes_group = render_nodes(&positioned_nodes, config);
    doc.add_node(nodes_group);

    Ok(doc.to_string())
}

/// Position nodes in a tree layout
fn position_tree(
    node: &MindmapNode,
    x: f64,
    y: f64,
    section: i32,
    parent_id: Option<String>,
    positioned: &mut Vec<PositionedNode>,
    counter: &mut usize,
) -> (f64, f64) {
    // Generate node ID
    let id = node
        .node_id
        .clone()
        .unwrap_or_else(|| format!("mindmap-node-{}", *counter));
    *counter += 1;

    // Calculate node dimensions based on text
    let text = &node.descr;
    let (width, height) = calculate_node_size(text, node.node_type);

    // Position children
    let mut child_y_offset: f64 = 0.0;
    let mut max_subtree_width: f64 = 0.0;

    for (i, child) in node.children.iter().enumerate() {
        // Assign section based on position (for root's children) or inherit
        let child_section = if section == -1 {
            (i as i32) % (MAX_SECTIONS as i32 - 1)
        } else {
            section
        };

        let child_x = x + width / 2.0 + NODE_SPACING_H;
        let child_y = y + child_y_offset;

        let (subtree_width, subtree_height) = position_tree(
            child,
            child_x,
            child_y,
            child_section,
            Some(id.clone()),
            positioned,
            counter,
        );

        child_y_offset += subtree_height + NODE_SPACING_V;
        max_subtree_width = max_subtree_width.max(subtree_width);
    }

    // Calculate this subtree's height
    let subtree_height = if node.children.is_empty() {
        height
    } else {
        (child_y_offset - NODE_SPACING_V).max(height)
    };

    // Center this node vertically relative to its children
    let node_y = if node.children.is_empty() {
        y
    } else {
        y + (subtree_height - height) / 2.0
    };

    // Add this node
    positioned.push(PositionedNode {
        id,
        text: text.clone(),
        node_type: node.node_type,
        x,
        y: node_y,
        width,
        height,
        section,
        parent_id,
        icon: node.icon.clone(),
        class: node.class.clone(),
    });

    // Return subtree dimensions
    let total_width = width
        + if node.children.is_empty() {
            0.0
        } else {
            NODE_SPACING_H + max_subtree_width
        };

    (total_width, subtree_height)
}

/// Calculate node size based on text content
fn calculate_node_size(text: &str, node_type: NodeType) -> (f64, f64) {
    // Handle line breaks
    let lines: Vec<&str> = text.split("<br/>").collect();
    let max_line_len = lines.iter().map(|l| l.len()).max().unwrap_or(0);
    let num_lines = lines.len();

    // Base size from text
    let text_width = (max_line_len as f64) * CHAR_WIDTH;
    let text_height = (num_lines as f64) * (FONT_SIZE + 4.0);

    // Add padding
    let mut width = (text_width + NODE_PADDING * 2.0).max(MIN_NODE_WIDTH);
    let mut height = (text_height + NODE_PADDING * 2.0).max(MIN_NODE_HEIGHT);

    // Adjust for shape type
    match node_type {
        NodeType::Circle => {
            // Circle needs to be square and large enough
            let size = width.max(height) * 1.2;
            width = size;
            height = size;
        }
        NodeType::Hexagon => {
            // Hexagon needs extra width for the points
            width += 20.0;
        }
        NodeType::Cloud | NodeType::Bang => {
            // Cloud and bang need extra space for irregular shape
            width += 30.0;
            height += 20.0;
        }
        _ => {}
    }

    (width, height)
}

/// Calculate bounds of all nodes
fn calculate_bounds(nodes: &[PositionedNode]) -> (f64, f64, f64, f64) {
    if nodes.is_empty() {
        return (0.0, 100.0, 0.0, 100.0);
    }

    let mut min_x = f64::MAX;
    let mut max_x = f64::MIN;
    let mut min_y = f64::MAX;
    let mut max_y = f64::MIN;

    for node in nodes {
        min_x = min_x.min(node.x);
        max_x = max_x.max(node.x + node.width);
        min_y = min_y.min(node.y);
        max_y = max_y.max(node.y + node.height);
    }

    (min_x, max_x, min_y, max_y)
}

/// Render all edges
fn render_edges(nodes: &[PositionedNode], _config: &RenderConfig) -> SvgElement {
    let mut children = Vec::new();

    // Build a map of id -> node for parent lookup
    let node_map: std::collections::HashMap<&str, &PositionedNode> =
        nodes.iter().map(|n| (n.id.as_str(), n)).collect();

    for node in nodes {
        if let Some(parent_id) = &node.parent_id {
            if let Some(parent) = node_map.get(parent_id.as_str()) {
                // Draw edge from parent to child
                let parent_cx = parent.x + parent.width / 2.0;
                let parent_cy = parent.y + parent.height / 2.0;
                let child_cx = node.x + node.width / 2.0;
                let child_cy = node.y + node.height / 2.0;

                // Use a curved path (quadratic bezier)
                let control_x = (parent_cx + child_cx) / 2.0;
                let control_y = parent_cy;

                let path = format!(
                    "M {} {} Q {} {} {} {}",
                    parent.x + parent.width,
                    parent_cy,
                    control_x,
                    control_y,
                    node.x,
                    child_cy
                );

                // Get section class for edge color
                let section_class = if node.section >= 0 {
                    format!("section-edge-{}", node.section % (MAX_SECTIONS as i32 - 1))
                } else {
                    "section-edge-root".to_string()
                };

                children.push(SvgElement::Path {
                    d: path,
                    attrs: Attrs::new()
                        .with_class("edge")
                        .with_class(&section_class)
                        .with_fill("none")
                        .with_stroke_width(3.0),
                });
            }
        }
    }

    SvgElement::Group {
        children,
        attrs: Attrs::new().with_class("mindmap-edges"),
    }
}

/// Render all nodes
fn render_nodes(nodes: &[PositionedNode], config: &RenderConfig) -> SvgElement {
    let mut children = Vec::new();

    for node in nodes {
        children.push(render_node(node, config));
    }

    SvgElement::Group {
        children,
        attrs: Attrs::new().with_class("mindmap-nodes"),
    }
}

/// Render a single node
fn render_node(node: &PositionedNode, _config: &RenderConfig) -> SvgElement {
    let mut node_children = Vec::new();

    // Determine section class
    let section_class = if node.section < 0 {
        "section-root".to_string()
    } else {
        format!("section-{}", node.section % (MAX_SECTIONS as i32 - 1))
    };

    // Build node class
    let mut classes = vec!["mindmap-node".to_string(), section_class.clone()];
    if let Some(ref class) = node.class {
        classes.push(class.clone());
    }

    // Render shape based on node type
    let shape = render_node_shape(node);
    node_children.push(shape);

    // Render icon if present
    if let Some(ref icon) = node.icon {
        let icon_elem = render_node_icon(node, icon, &section_class);
        node_children.push(icon_elem);
    }

    // Render text label
    let text = render_node_text(node);
    node_children.push(text);

    // Wrap in a group and translate to position
    SvgElement::Group {
        children: node_children,
        attrs: Attrs::new()
            .with_class(&classes.join(" "))
            .with_id(&format!("node-{}", node.id))
            .with_attr("transform", &format!("translate({}, {})", node.x, node.y)),
    }
}

/// Render node shape based on type
fn render_node_shape(node: &PositionedNode) -> SvgElement {
    match node.node_type {
        NodeType::Default => {
            // Default shape: rounded rectangle with bottom line
            let rd = 5.0;
            let path = format!(
                "M0 {} v{} q0,-5 5,-5 h{} q5,0 5,5 v{} H0 Z",
                node.height - rd,
                -(node.height - 2.0 * rd),
                node.width - 2.0 * rd,
                node.height - rd
            );
            SvgElement::Path {
                d: path,
                attrs: Attrs::new().with_class("node-bkg node-default"),
            }
        }
        NodeType::Rect => {
            // Square/rectangle
            SvgElement::Rect {
                x: 0.0,
                y: 0.0,
                width: node.width,
                height: node.height,
                rx: None,
                ry: None,
                attrs: Attrs::new().with_class("node-bkg node-rect"),
            }
        }
        NodeType::RoundedRect => {
            // Rounded rectangle
            SvgElement::Rect {
                x: 0.0,
                y: 0.0,
                width: node.width,
                height: node.height,
                rx: Some(NODE_PADDING),
                ry: Some(NODE_PADDING),
                attrs: Attrs::new().with_class("node-bkg node-rounded"),
            }
        }
        NodeType::Circle => {
            // Circle - centered in the node box
            let radius = node.width.min(node.height) / 2.0;
            SvgElement::Circle {
                cx: node.width / 2.0,
                cy: node.height / 2.0,
                r: radius,
                attrs: Attrs::new().with_class("node-bkg node-circle"),
            }
        }
        NodeType::Cloud => {
            // Cloud shape using arcs
            let w = node.width;
            let h = node.height;
            let r1 = 0.15 * w;
            let r2 = 0.25 * w;
            let r3 = 0.35 * w;
            let r4 = 0.2 * w;

            let path = format!(
                "M0 0 a{r1},{r1} 0 0,1 {},{} a{r3},{r3} 1 0,1 {},{} a{r2},{r2} 1 0,1 {},{} \
                 a{r1},{r1} 1 0,1 {},{} a{r4},{r4} 1 0,1 {},{} \
                 a{r2},{r1} 1 0,1 {},{} a{r3},{r3} 1 0,1 {},0 a{r1},{r1} 1 0,1 {},{} \
                 a{r1},{r1} 1 0,1 {},{} a{r4},{r4} 1 0,1 {},{} H0 V0 Z",
                w * 0.25,
                -w * 0.1,
                w * 0.4,
                -w * 0.1,
                w * 0.35,
                w * 0.2,
                w * 0.15,
                h * 0.35,
                -w * 0.15,
                h * 0.65,
                -w * 0.25,
                w * 0.15,
                -w * 0.5,
                -w * 0.25,
                -w * 0.15,
                -w * 0.1,
                -h * 0.35,
                w * 0.1,
                -h * 0.65,
                r1 = r1,
                r2 = r2,
                r3 = r3,
                r4 = r4
            );

            SvgElement::Path {
                d: path,
                attrs: Attrs::new().with_class("node-bkg node-cloud"),
            }
        }
        NodeType::Bang => {
            // Bang/explosion shape
            let w = node.width;
            let h = node.height;
            let r = 0.15 * w;

            let path = format!(
                "M0 0 a{r},{r} 1 0,0 {},{} a{r},{r} 1 0,0 {},0 a{r},{r} 1 0,0 {},0 a{r},{r} 1 0,0 {},{} \
                 a{r},{r} 1 0,0 {},{} a{},{} 1 0,0 0,{} a{r},{r} 1 0,0 {},{} \
                 a{r},{r} 1 0,0 {},{} a{r},{r} 1 0,0 {},0 a{r},{r} 1 0,0 {},0 a{r},{r} 1 0,0 {},{} \
                 a{r},{r} 1 0,0 {},{} a{},{} 1 0,0 0,{} a{r},{r} 1 0,0 {},{} H0 V0 Z",
                w * 0.25, -h * 0.1,
                w * 0.25,
                w * 0.25,
                w * 0.25, h * 0.1,
                w * 0.15, h * 0.33,
                r * 0.8, r * 0.8, h * 0.34,
                -w * 0.15, h * 0.33,
                -w * 0.25, h * 0.15,
                -w * 0.25,
                -w * 0.25,
                -w * 0.25, -h * 0.15,
                -w * 0.1, -h * 0.33,
                r * 0.8, r * 0.8, -h * 0.34,
                w * 0.1, -h * 0.33,
                r = r
            );

            SvgElement::Path {
                d: path,
                attrs: Attrs::new().with_class("node-bkg node-bang"),
            }
        }
        NodeType::Hexagon => {
            // Hexagon shape
            let h = node.height;
            let m = h / 4.0;
            let w = node.width;
            let points = format!(
                "{},{} {},{} {},{} {},{} {},{} {},{}",
                m,
                0.0,
                w - m,
                0.0,
                w,
                h / 2.0,
                w - m,
                h,
                m,
                h,
                0.0,
                h / 2.0
            );

            SvgElement::PolygonStr {
                points,
                attrs: Attrs::new().with_class("node-bkg node-hexagon"),
            }
        }
    }
}

/// Render node text
fn render_node_text(node: &PositionedNode) -> SvgElement {
    // SvgElement::Text already handles <br/> by splitting into tspans
    SvgElement::Text {
        x: node.width / 2.0,
        y: node.height / 2.0,
        content: node.text.clone(),
        attrs: Attrs::new()
            .with_class("mindmap-node-label")
            .with_attr("text-anchor", "middle")
            .with_attr("dominant-baseline", "middle")
            .with_attr("font-size", &format!("{}px", FONT_SIZE)),
    }
}

/// Render an icon for the node using foreignObject with Font Awesome classes
fn render_node_icon(node: &PositionedNode, icon: &str, section_class: &str) -> SvgElement {
    // Mermaid.js uses foreignObject with an <i> tag for Font Awesome icons
    // This approach allows the icon to render properly in browsers that support foreignObject
    let icon_class = format!(
        "node-icon-{} {}",
        section_class.replace("section-", ""),
        icon
    );
    let icon_size = 40.0;

    // Position icon above the text
    let y_offset = -10.0;

    // Create foreignObject containing a div with the icon
    let html_content = format!(
        r#"<div xmlns="http://www.w3.org/1999/xhtml" class="icon-container" style="height:100%;display:flex;justify-content:center;align-items:center;"><i class="{}"></i></div>"#,
        icon_class
    );

    SvgElement::Raw {
        content: format!(
            r#"<foreignObject x="{}" y="{}" width="{}" height="{}" style="text-align:center;">{}</foreignObject>"#,
            (node.width - icon_size) / 2.0,
            y_offset,
            icon_size,
            icon_size,
            html_content
        ),
    }
}

/// Generate CSS for mindmap diagrams
fn generate_mindmap_css(config: &RenderConfig) -> String {
    let theme = &config.theme;

    // Generate section colors
    let mut section_css = String::new();

    // Root section uses primary color (similar to mermaid.js)
    section_css.push_str(&format!(
        r#"
.section-root rect, .section-root path, .section-root circle, .section-root polygon {{
  fill: {};
}}
.section-root text {{
  fill: {};
}}
.section-edge-root {{
  stroke: {};
}}
"#,
        theme.primary_color, theme.primary_text_color, theme.primary_color
    ));

    // Generate section colors from pie colors (similar to mermaid's cScale)
    for i in 0..(MAX_SECTIONS - 1) {
        let color = theme
            .pie_colors
            .get(i)
            .map(|s| s.as_str())
            .unwrap_or("#ECECFF");

        let stroke_width = 17 - 3 * (i as i32);

        section_css.push_str(&format!(
            r#"
.section-{i} rect, .section-{i} path, .section-{i} circle, .section-{i} polygon {{
  fill: {color};
}}
.section-{i} text {{
  fill: {text_color};
}}
.section-edge-{i} {{
  stroke: {color};
}}
.edge-depth-{i} {{
  stroke-width: {stroke_width};
}}
"#,
            i = i,
            color = color,
            text_color = theme.primary_text_color,
            stroke_width = stroke_width
        ));
    }

    format!(
        r#"
.mindmap-node {{
  cursor: pointer;
}}
.mindmap-node-label {{
  font-family: {font_family};
}}
.node-bkg {{
  stroke: {line_color};
  stroke-width: 1px;
}}
.edge {{
  fill: none;
  stroke-width: 3px;
}}
.icon-container {{
  height: 100%;
  display: flex;
  justify-content: center;
  align-items: center;
}}
.icon-container i {{
  font-size: 40px;
}}
{section_css}
"#,
        font_family = theme.font_family,
        line_color = theme.line_color,
        section_css = section_css
    )
}
