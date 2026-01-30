//! Mindmap diagram renderer
//!
//! Renders mindmap diagrams using a radial tree layout.
//! The root is centered with branches spreading outward at angles.

use crate::diagrams::mindmap::{MindmapDb, MindmapNode, NodeType};
use crate::error::Result;
use crate::render::svg::{Attrs, RenderConfig, SvgDocument, SvgElement};

/// Padding around nodes
const NODE_PADDING: f64 = 15.0;

/// Minimum node width
const MIN_NODE_WIDTH: f64 = 50.0;

/// Minimum node height
const MIN_NODE_HEIGHT: f64 = 34.0;

/// Radial distance from parent to child
const RADIAL_DISTANCE: f64 = 85.0;

/// Maximum number of color sections (matches mermaid.js)
const MAX_SECTIONS: usize = 12;

/// Font size for node labels (mermaid uses 16px)
const FONT_SIZE: f64 = 16.0;

/// Character width estimate for text sizing (increased for 16px font)
const CHAR_WIDTH: f64 = 9.0;

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
    /// X position (top-left corner)
    x: f64,
    /// Y position (top-left corner)
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
    /// Depth in the tree (0 = root)
    depth: usize,
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

    // Position all nodes using radial tree layout
    let mut positioned_nodes = Vec::new();
    let mut node_counter = 0;

    // Calculate root node size
    let (root_width, root_height) = calculate_node_size(&root.descr, root.node_type);

    // Position root at center (0,0)
    positioned_nodes.push(PositionedNode {
        id: "mindmap-root".to_string(),
        text: root.descr.clone(),
        node_type: root.node_type,
        x: -root_width / 2.0,
        y: -root_height / 2.0,
        width: root_width,
        height: root_height,
        section: -1, // Root section
        parent_id: None,
        icon: root.icon.clone(),
        class: root.class.clone(),
        depth: 0,
    });

    // Position children using radial layout
    // Distribute root's children across angles around the root
    let children = &root.children;
    let num_children = children.len();

    if num_children > 0 {
        // Assign angles to each top-level branch
        // Mermaid pattern: first branch goes right, then spread downward and upward
        let branch_angles = calculate_branch_angles(num_children);

        for (i, (child, &angle)) in children.iter().zip(branch_angles.iter()).enumerate() {
            let section = (i as i32) % (MAX_SECTIONS as i32 - 1);
            position_radial_tree(
                child,
                0.0, // parent center x
                0.0, // parent center y
                angle,
                section,
                Some("mindmap-root".to_string()),
                &mut positioned_nodes,
                &mut node_counter,
                1, // depth
            );
        }
    }

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

/// Calculate branch angles for root children
/// Layout pattern derived from analyzing mermaid.js cose-bilkent reference output:
/// - First branch goes right (0°)
/// - Second branch goes down-left (~145°)
/// - Third branch goes up-left (~-72°)
fn calculate_branch_angles(num_children: usize) -> Vec<f64> {
    use std::f64::consts::PI;

    match num_children {
        0 => vec![],
        1 => vec![0.04],            // Single child goes right
        2 => vec![0.04, PI * 0.80], // First right, second left-down (~144°)
        3 => {
            // Derived from mermaid reference output analysis:
            // - Origins: ~2° (almost pure right)
            // - Research: ~145° (down-left)
            // - Tools: ~-72° (up-left)
            vec![0.04, PI * 0.80, -PI * 0.40] // ~2°, ~144°, ~-72°
        }
        _ => {
            let mut angles = Vec::with_capacity(num_children);

            // First child always goes right
            angles.push(0.04);

            // Distribute remaining children alternating between down-left and up-left
            let remaining = num_children - 1;
            if remaining > 0 {
                let down_count = remaining.div_ceil(2);
                let up_count = remaining - down_count;

                // Down-left quadrant (angles from ~130° to ~160°)
                for i in 0..down_count {
                    let t = (i as f64 + 0.5) / (down_count.max(1) as f64);
                    let angle = PI * 0.72 + t * PI * 0.17; // ~130° to ~160°
                    angles.push(angle);
                }

                // Up-left quadrant (angles from ~-50° to ~-90°)
                for i in 0..up_count {
                    let t = (i as f64 + 0.5) / (up_count.max(1) as f64);
                    let angle = -PI * 0.28 - t * PI * 0.22; // ~-50° to ~-90°
                    angles.push(angle);
                }
            }

            angles
        }
    }
}

/// Position nodes in a radial tree layout
#[allow(clippy::too_many_arguments)]
fn position_radial_tree(
    node: &MindmapNode,
    parent_cx: f64,
    parent_cy: f64,
    angle: f64,
    section: i32,
    parent_id: Option<String>,
    positioned: &mut Vec<PositionedNode>,
    counter: &mut usize,
    depth: usize,
) {
    use std::f64::consts::PI;

    // Generate node ID
    let id = node
        .node_id
        .clone()
        .unwrap_or_else(|| format!("mindmap-node-{}", *counter));
    *counter += 1;

    // Calculate node dimensions based on text
    let text = &node.descr;
    let (width, height) = calculate_node_size(text, node.node_type);

    // Determine if this is a "right-going" branch (to create landscape layout)
    let is_right_branch = angle.abs() < PI / 3.0;

    // Calculate distance based on depth and direction
    // Mermaid's cose-bilkent spreads nodes based on edge tensions
    // We approximate this with moderate distances
    let base_distance = if is_right_branch {
        RADIAL_DISTANCE * 1.35 // More distance for right-going branches
    } else {
        RADIAL_DISTANCE * 1.15 // Slightly less for other branches
    };
    // Increase distance with depth to prevent crowding
    let distance = base_distance * (1.0 + (depth.saturating_sub(1)) as f64 * 0.15);

    // Calculate node center position
    let node_cx = parent_cx + distance * angle.cos();
    let node_cy = parent_cy + distance * angle.sin();

    // Convert to top-left corner
    let node_x = node_cx - width / 2.0;
    let node_y = node_cy - height / 2.0;

    // Add this node
    positioned.push(PositionedNode {
        id: id.clone(),
        text: text.clone(),
        node_type: node.node_type,
        x: node_x,
        y: node_y,
        width,
        height,
        section,
        parent_id,
        icon: node.icon.clone(),
        class: node.class.clone(),
        depth,
    });

    // Position children
    if !node.children.is_empty() {
        let num_children = node.children.len();

        // Calculate angular spread for children
        // Right-going branches keep children more horizontal
        let spread_angle = if is_right_branch {
            // Keep right-branch children tightly spread vertically around horizontal
            calculate_children_spread(num_children, depth) * 0.6
        } else {
            calculate_children_spread(num_children, depth)
        };

        // Start angle - children spread around the parent's direction
        let start_angle = angle - spread_angle / 2.0;

        for (i, child) in node.children.iter().enumerate() {
            // Calculate child's angle
            let child_angle = if num_children == 1 {
                // Single child continues in same direction (with slight horizontal bias for right branches)
                if is_right_branch {
                    angle * 0.7 // Trend toward horizontal
                } else {
                    angle
                }
            } else {
                start_angle + (i as f64) * spread_angle / (num_children - 1).max(1) as f64
            };

            position_radial_tree(
                child,
                node_cx,
                node_cy,
                child_angle,
                section, // Children inherit parent's section
                Some(id.clone()),
                positioned,
                counter,
                depth + 1,
            );
        }
    }
}

/// Calculate angular spread for children based on count and depth
fn calculate_children_spread(num_children: usize, depth: usize) -> f64 {
    use std::f64::consts::PI;

    // Base spread - mermaid groups children tighter than pure radial layouts
    let base_spread = match depth {
        1 => PI / 2.5, // First level: 72 degrees
        2 => PI / 3.5, // Second level: ~51 degrees
        _ => PI / 4.5, // Deeper: 40 degrees
    };

    // Adjust for number of children (less aggressive scaling)
    let spread = base_spread * (num_children as f64 * 0.6).sqrt();

    spread.min(PI * 0.55) // Cap at ~99 degrees
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
                // Draw edge from parent center to child center
                let parent_cx = parent.x + parent.width / 2.0;
                let parent_cy = parent.y + parent.height / 2.0;
                let child_cx = node.x + node.width / 2.0;
                let child_cy = node.y + node.height / 2.0;

                // Calculate edge attachment points on node boundaries
                let (start_x, start_y) = get_edge_attachment_point(
                    parent_cx,
                    parent_cy,
                    parent.width,
                    parent.height,
                    child_cx,
                    child_cy,
                    parent.node_type,
                );
                let (end_x, end_y) = get_edge_attachment_point(
                    child_cx,
                    child_cy,
                    node.width,
                    node.height,
                    parent_cx,
                    parent_cy,
                    node.node_type,
                );

                // Use a curved path with cubic bezier to match mermaid's "basis" curve style
                // Mermaid uses d3-shape's curveBasis which creates smooth S-curves
                let dx = end_x - start_x;
                let dy = end_y - start_y;

                // Calculate control points for smooth cubic bezier curve
                // Similar to d3's basis curve interpolation
                let t1 = 0.3;
                let t2 = 0.7;

                let ctrl1_x = start_x + dx * t1;
                let ctrl1_y = start_y + dy * t1 * 0.5; // Bias toward horizontal
                let ctrl2_x = start_x + dx * t2;
                let ctrl2_y = end_y - dy * (1.0 - t2) * 0.5; // Bias toward end

                let path = format!(
                    "M{:.1},{:.1} C{:.1},{:.1} {:.1},{:.1} {:.1},{:.1}",
                    start_x, start_y, ctrl1_x, ctrl1_y, ctrl2_x, ctrl2_y, end_x, end_y
                );

                // Get section class for edge color
                let section_class = if node.section >= 0 {
                    format!("section-edge-{}", node.section % (MAX_SECTIONS as i32 - 1))
                } else {
                    "section-edge-root".to_string()
                };

                // Add depth class for stroke width
                let depth_class = format!("edge-depth-{}", node.depth.min(10));

                children.push(SvgElement::Path {
                    d: path,
                    attrs: Attrs::new()
                        .with_class("edge")
                        .with_class(&section_class)
                        .with_class(&depth_class)
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

/// Calculate the attachment point on a node's boundary for an edge
fn get_edge_attachment_point(
    cx: f64,
    cy: f64,
    width: f64,
    height: f64,
    target_x: f64,
    target_y: f64,
    node_type: NodeType,
) -> (f64, f64) {
    // For circles, use the circle boundary
    if matches!(node_type, NodeType::Circle) {
        let radius = width.min(height) / 2.0;
        let dx = target_x - cx;
        let dy = target_y - cy;
        let dist = (dx * dx + dy * dy).sqrt().max(0.001);
        return (cx + dx / dist * radius, cy + dy / dist * radius);
    }

    // For other shapes, use rectangle boundary intersection
    let dx = target_x - cx;
    let dy = target_y - cy;

    if dx.abs() < 0.001 && dy.abs() < 0.001 {
        return (cx, cy);
    }

    // Calculate intersection with rectangle boundary
    let half_w = width / 2.0;
    let half_h = height / 2.0;

    // Check which edge we intersect
    let tx = if dx.abs() > 0.001 {
        half_w / dx.abs()
    } else {
        f64::MAX
    };
    let ty = if dy.abs() > 0.001 {
        half_h / dy.abs()
    } else {
        f64::MAX
    };

    let t = tx.min(ty);

    (cx + dx * t, cy + dy * t)
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

    // Build node class - root nodes get both section-root and section--1 (matching mermaid.js)
    let mut classes = vec!["mindmap-node".to_string(), section_class.clone()];
    if node.section < 0 {
        classes.push("section--1".to_string());
    }
    if let Some(ref class) = node.class {
        classes.push(class.clone());
    }

    // Render shape based on node type (may include multiple elements like path + line)
    let shapes = render_node_shape(node);
    node_children.extend(shapes);

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
/// Returns a vector of SVG elements (some shapes like Default include multiple elements)
fn render_node_shape(node: &PositionedNode) -> Vec<SvgElement> {
    match node.node_type {
        NodeType::Default => {
            // Default shape: rounded rectangle with decorative bottom line (mermaid.js style)
            let rd = 5.0;
            let path = format!(
                "M0 {} v{} q0,-5 5,-5 h{} q5,0 5,5 v{} H0 Z",
                node.height - rd,
                -(node.height - 2.0 * rd),
                node.width - 2.0 * rd,
                node.height - rd
            );

            // Line class uses the section number for color coordination
            let line_class = if node.section >= 0 {
                format!("node-line-{}", node.section % (MAX_SECTIONS as i32 - 1))
            } else {
                "node-line-root".to_string()
            };

            vec![
                SvgElement::Path {
                    d: path,
                    attrs: Attrs::new().with_class("node-bkg node-default"),
                },
                // Decorative line at the bottom of the node (matches mermaid.js)
                SvgElement::Line {
                    x1: 0.0,
                    y1: node.height,
                    x2: node.width,
                    y2: node.height,
                    attrs: Attrs::new().with_class(&line_class),
                },
            ]
        }
        NodeType::Rect => {
            // Square/rectangle
            vec![SvgElement::Rect {
                x: 0.0,
                y: 0.0,
                width: node.width,
                height: node.height,
                rx: None,
                ry: None,
                attrs: Attrs::new().with_class("node-bkg node-rect"),
            }]
        }
        NodeType::RoundedRect => {
            // Rounded rectangle
            vec![SvgElement::Rect {
                x: 0.0,
                y: 0.0,
                width: node.width,
                height: node.height,
                rx: Some(NODE_PADDING),
                ry: Some(NODE_PADDING),
                attrs: Attrs::new().with_class("node-bkg node-rounded"),
            }]
        }
        NodeType::Circle => {
            // Circle - centered in the node box
            let radius = node.width.min(node.height) / 2.0;
            vec![SvgElement::Circle {
                cx: node.width / 2.0,
                cy: node.height / 2.0,
                r: radius,
                attrs: Attrs::new().with_class("node-bkg node-circle"),
            }]
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

            vec![SvgElement::Path {
                d: path,
                attrs: Attrs::new().with_class("node-bkg node-cloud"),
            }]
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

            vec![SvgElement::Path {
                d: path,
                attrs: Attrs::new().with_class("node-bkg node-bang"),
            }]
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

            vec![SvgElement::PolygonStr {
                points,
                attrs: Attrs::new().with_class("node-bkg node-hexagon"),
            }]
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

    // Root section uses a distinctive blue (matching mermaid.js exactly)
    // mermaid uses hsl(240, 100%, 46.2745098039%) for section-root fill
    // and hsl(240, 100%, 76.2745098039%) for section--1 shapes
    // section--1 is used alongside section-root in the reference
    section_css.push_str(
        r#"
.section--1 rect, .section--1 path, .section--1 circle, .section--1 polygon {
  fill: hsl(240, 100%, 76.2745098039%);
}
.section--1 text {
  fill: #ffffff;
}
.section-edge--1 {
  stroke: hsl(240, 100%, 76.2745098039%);
}
.edge-depth--1 {
  stroke-width: 17;
}
.section--1 line {
  stroke: hsl(60, 100%, 86.2745098039%);
  stroke-width: 3;
}
.section-root rect, .section-root path, .section-root circle, .section-root polygon {
  fill: hsl(240, 100%, 46.2745098039%);
}
.section-root text {
  fill: #ffffff;
}
.section-edge-root {
  stroke: hsl(240, 100%, 76.2745098039%);
}
.edge-depth-root {
  stroke-width: 17;
}
"#,
    );

    // Section colors matching mermaid.js pattern exactly
    // mermaid uses specific hue sequence with precise HSL values
    // Each section also has a complementary line color (hue + 180, lightness + 10)
    let mindmap_colors = [
        (
            "hsl(60, 100%, 73.5294117647%)",
            "hsl(240, 100%, 83.5294117647%)",
        ), // Section 0: yellow, line purple
        (
            "hsl(80, 100%, 76.2745098039%)",
            "hsl(260, 100%, 86.2745098039%)",
        ), // Section 1: lime, line purple
        (
            "hsl(270, 100%, 76.2745098039%)",
            "hsl(90, 100%, 86.2745098039%)",
        ), // Section 2: purple, line green
        (
            "hsl(300, 100%, 76.2745098039%)",
            "hsl(120, 100%, 86.2745098039%)",
        ), // Section 3: pink, line green
        (
            "hsl(330, 100%, 76.2745098039%)",
            "hsl(150, 100%, 86.2745098039%)",
        ), // Section 4: rose, line teal
        (
            "hsl(0, 100%, 76.2745098039%)",
            "hsl(180, 100%, 86.2745098039%)",
        ), // Section 5: red, line cyan
        (
            "hsl(30, 100%, 76.2745098039%)",
            "hsl(210, 100%, 86.2745098039%)",
        ), // Section 6: orange, line blue
        (
            "hsl(90, 100%, 76.2745098039%)",
            "hsl(270, 100%, 86.2745098039%)",
        ), // Section 7: green, line purple
        (
            "hsl(150, 100%, 76.2745098039%)",
            "hsl(330, 100%, 86.2745098039%)",
        ), // Section 8: teal, line rose
        (
            "hsl(180, 100%, 76.2745098039%)",
            "hsl(0, 100%, 86.2745098039%)",
        ), // Section 9: cyan, line red
        (
            "hsl(210, 100%, 76.2745098039%)",
            "hsl(30, 100%, 86.2745098039%)",
        ), // Section 10: blue, line orange
    ];

    for i in 0..(MAX_SECTIONS - 1) {
        let (fill_color, line_color) = mindmap_colors
            .get(i)
            .unwrap_or(&("hsl(60, 100%, 73.5%)", "hsl(240, 100%, 83.5%)"));
        let stroke_width = 17 - 3 * (i as i32);

        // Determine text color based on section (section 2 is purple which needs white text)
        let text_color = if i == 2 { "#ffffff" } else { "black" };

        section_css.push_str(&format!(
            r#"
.section-{i} rect, .section-{i} path, .section-{i} circle, .section-{i} polygon {{
  fill: {fill_color};
}}
.section-{i} text {{
  fill: {text_color};
}}
.section-edge-{i} {{
  stroke: {fill_color};
}}
.edge-depth-{i} {{
  stroke-width: {stroke_width};
}}
.node-line-{i} {{
  stroke: {line_color};
  stroke-width: 3;
}}
"#,
            i = i,
            fill_color = fill_color,
            text_color = text_color,
            stroke_width = stroke_width,
            line_color = line_color
        ));
    }

    format!(
        r#"
.mindmap-node {{
  cursor: pointer;
}}
.error-icon {{
  fill: #552222;
}}
.error-text {{
  fill: #552222;
  stroke: #552222;
}}
.disabled, .disabled circle, .disabled text {{
  fill: lightgray;
}}
.disabled text {{
  fill: #efefef;
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
.node-line-root {{
  stroke: hsl(60, 100%, 86.2745098039%);
  stroke-width: 3;
}}
{section_css}
"#,
        font_family = theme.font_family,
        line_color = theme.line_color,
        section_css = section_css
    )
}
