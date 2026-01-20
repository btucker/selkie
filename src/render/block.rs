//! Block diagram renderer
//!
//! Renders block diagrams using a grid layout.
//! Based on mermaid.js block diagram implementation.

use crate::diagrams::block::{Block, BlockDb, BlockType};
use crate::error::Result;
use crate::render::svg::markers::create_arrow_markers;
use crate::render::svg::{Attrs, RenderConfig, SvgDocument, SvgElement};
use std::collections::HashMap;

/// Padding around blocks
const BLOCK_PADDING: f64 = 10.0;

/// Spacing between blocks
const BLOCK_SPACING: f64 = 20.0;

/// Minimum block width
const MIN_BLOCK_WIDTH: f64 = 80.0;

/// Minimum block height
const MIN_BLOCK_HEIGHT: f64 = 40.0;

/// Font size for labels
const FONT_SIZE: f64 = 14.0;

/// Character width estimate for text sizing
const CHAR_WIDTH: f64 = 8.0;

/// Default number of columns
const DEFAULT_COLUMNS: usize = 1;

/// A positioned block for rendering
#[derive(Debug, Clone)]
struct PositionedBlock {
    /// Block ID
    id: String,
    /// Display label
    label: String,
    /// Block shape type
    block_type: BlockType,
    /// X position
    x: f64,
    /// Y position
    y: f64,
    /// Width
    width: f64,
    /// Height
    height: f64,
    /// Column span (reserved for future use)
    #[allow(dead_code)]
    column_span: usize,
    /// Custom styles (reserved for future use)
    #[allow(dead_code)]
    styles: Vec<String>,
    /// CSS classes
    classes: Vec<String>,
}

/// Edge between blocks
#[derive(Debug, Clone)]
struct PositionedEdge {
    /// Start block ID (reserved for future use)
    #[allow(dead_code)]
    start: String,
    /// End block ID (reserved for future use)
    #[allow(dead_code)]
    end: String,
    /// Edge label
    label: Option<String>,
    /// Start X
    start_x: f64,
    /// Start Y
    start_y: f64,
    /// End X
    end_x: f64,
    /// End Y
    end_y: f64,
}

/// Render a block diagram to SVG
pub fn render_block(db: &BlockDb, config: &RenderConfig) -> Result<String> {
    let mut doc = SvgDocument::new();

    // Get all blocks
    let blocks = db.get_blocks();
    let block_order = db.get_block_order();
    let edges = db.get_edges();
    let classes = db.get_classes();

    if blocks.is_empty() {
        // Empty diagram
        doc.set_size(100.0, 100.0);
        return Ok(doc.to_string());
    }

    // Calculate block sizes
    let mut block_sizes: HashMap<String, (f64, f64)> = HashMap::new();
    for (id, block) in blocks.iter() {
        let (width, height) = calculate_block_size(block);
        block_sizes.insert(id.clone(), (width, height));
    }

    // Determine columns from root or default
    let columns = db.get_columns().unwrap_or(DEFAULT_COLUMNS);

    // Position blocks in grid layout (preserving insertion order)
    let positioned_blocks = layout_blocks(blocks, block_order, &block_sizes, columns);

    // Calculate bounds
    let (min_x, max_x, min_y, max_y) = calculate_bounds(&positioned_blocks);
    // Use smaller padding similar to mermaid (5px)
    let svg_padding = 5.0;
    let width = (max_x - min_x) + svg_padding * 2.0;
    let height = (max_y - min_y) + svg_padding * 2.0;

    doc.set_size_with_origin(min_x - svg_padding, min_y - svg_padding, width, height);

    // Add CSS styles
    if config.embed_css {
        doc.add_style(&generate_block_css(config, classes));
    }

    // Add arrow marker definitions for edges
    doc.add_defs(create_arrow_markers(&config.theme));

    // Render edges first (behind blocks)
    if !edges.is_empty() {
        let block_map: HashMap<&str, &PositionedBlock> = positioned_blocks
            .iter()
            .map(|b| (b.id.as_str(), b))
            .collect();

        let positioned_edges = position_edges(edges, &block_map);
        let edges_group = render_edges(&positioned_edges, config);
        doc.add_edge_path(edges_group);
    }

    // Render blocks
    let blocks_group = render_blocks(&positioned_blocks, config);
    doc.add_node(blocks_group);

    Ok(doc.to_string())
}

/// Calculate block size based on label and type
fn calculate_block_size(block: &Block) -> (f64, f64) {
    let label = block.label.as_deref().unwrap_or(&block.id);

    // Calculate text dimensions
    let lines: Vec<&str> = label.split("<br/>").collect();
    let max_line_len = lines.iter().map(|l| l.len()).max().unwrap_or(0);
    let num_lines = lines.len();

    let text_width = (max_line_len as f64) * CHAR_WIDTH;
    let text_height = (num_lines as f64) * (FONT_SIZE + 4.0);

    // Base size with padding
    let mut width = (text_width + BLOCK_PADDING * 4.0).max(MIN_BLOCK_WIDTH);
    let mut height = (text_height + BLOCK_PADDING * 2.0).max(MIN_BLOCK_HEIGHT);

    // Adjust for shape type
    match block.block_type {
        BlockType::Circle | BlockType::DoubleCircle => {
            let size = width.max(height);
            width = size;
            height = size;
        }
        BlockType::Diamond => {
            // Diamond needs more space
            width *= 1.3;
            height *= 1.3;
        }
        BlockType::Hexagon => {
            width += 20.0;
        }
        BlockType::BlockArrow => {
            width += 40.0;
        }
        _ => {}
    }

    (width, height)
}

/// Layout blocks in a grid with proper nesting for composite blocks
/// Following mermaid's approach: all blocks in same row share same height,
/// children inside composites are at same y-level as parent row
fn layout_blocks(
    blocks: &HashMap<String, Block>,
    block_order: &[String],
    sizes: &HashMap<String, (f64, f64)>,
    columns: usize,
) -> Vec<PositionedBlock> {
    // Build parent-child relationships preserving insertion order
    let mut children_by_parent: HashMap<String, Vec<String>> = HashMap::new();
    for id in block_order {
        if let Some(block) = blocks.get(id) {
            if let Some(ref parent_id) = block.parent_id {
                children_by_parent
                    .entry(parent_id.clone())
                    .or_default()
                    .push(id.clone());
            }
        }
    }

    // Calculate sizes for composite blocks based on their children
    let mut effective_sizes: HashMap<String, (f64, f64)> = sizes.clone();
    for id in block_order {
        if let Some(block) = blocks.get(id) {
            if block.block_type == BlockType::Composite {
                if let Some(child_ids) = children_by_parent.get(id) {
                    let child_refs: Vec<&str> = child_ids.iter().map(|s| s.as_str()).collect();
                    let (comp_w, comp_h) =
                        calculate_composite_size(&child_refs, blocks, &effective_sizes);
                    effective_sizes.insert(id.clone(), (comp_w, comp_h));
                }
            }
        }
    }

    // Get root-level blocks (preserving insertion order)
    let root_blocks: Vec<(&str, &Block)> = block_order
        .iter()
        .filter_map(|id| {
            blocks.get(id).and_then(|b| {
                if b.parent_id.is_none() {
                    Some((id.as_str(), b))
                } else {
                    None
                }
            })
        })
        .collect();

    // First pass: determine row assignments and max heights per row
    let mut row_info = calculate_row_info(&root_blocks, &effective_sizes, columns);

    // Normalize heights - all blocks in same row get same height
    for row in &mut row_info {
        let max_height = row.iter().map(|(_, _, _, h)| *h).fold(0.0_f64, f64::max);
        for item in row.iter_mut() {
            item.3 = max_height;
        }
    }

    // Position all blocks based on row info
    let mut positioned = Vec::new();
    let mut current_y = 0.0;

    for row in &row_info {
        let mut current_x = 0.0;
        let row_height = row
            .first()
            .map(|(_, _, _, h)| *h)
            .unwrap_or(MIN_BLOCK_HEIGHT);

        for (id, block, width, height) in row {
            // Skip space blocks in rendering but account for their space
            if block.block_type == BlockType::Space {
                current_x += *width + BLOCK_SPACING;
                continue;
            }

            // Skip edge types
            if block.block_type == BlockType::Edge {
                continue;
            }

            positioned.push(PositionedBlock {
                id: id.to_string(),
                label: block.label.clone().unwrap_or_else(|| id.to_string()),
                block_type: block.block_type.clone(),
                x: current_x,
                y: current_y,
                width: *width,
                height: *height,
                column_span: block.width_in_columns.unwrap_or(1),
                styles: block.styles.clone(),
                classes: block.classes.clone(),
            });

            // If this is a composite, position children inside it
            if block.block_type == BlockType::Composite {
                if let Some(child_ids) = children_by_parent.get(*id) {
                    let child_blocks: Vec<(&str, &Block)> = child_ids
                        .iter()
                        .filter_map(|cid| blocks.get(cid).map(|b| (cid.as_str(), b)))
                        .collect();

                    // Use minimal padding (8px like mermaid) for children inside composite
                    let inner_padding = 8.0;
                    let child_positioned = layout_composite_children(
                        &child_blocks,
                        &effective_sizes,
                        current_x + inner_padding,
                        current_y + inner_padding,
                        *width - inner_padding * 2.0,
                        *height - inner_padding * 2.0,
                    );
                    positioned.extend(child_positioned);
                }
            }

            current_x += *width + BLOCK_SPACING;
        }

        current_y += row_height + BLOCK_SPACING;
    }

    positioned
}

/// Calculate row assignments and sizes for blocks
fn calculate_row_info<'a>(
    block_list: &[(&'a str, &'a Block)],
    sizes: &HashMap<String, (f64, f64)>,
    columns: usize,
) -> Vec<Vec<(&'a str, &'a Block, f64, f64)>> {
    let mut rows: Vec<Vec<(&str, &Block, f64, f64)>> = Vec::new();
    let mut current_row: Vec<(&str, &Block, f64, f64)> = Vec::new();
    let mut col = 0;

    for &(id, block) in block_list {
        let (width, height) = sizes
            .get(id)
            .cloned()
            .unwrap_or((MIN_BLOCK_WIDTH, MIN_BLOCK_HEIGHT));
        let span = block.width_in_columns.unwrap_or(1);

        // Adjust width for column span
        let block_width = if span > 1 {
            (span as f64) * MIN_BLOCK_WIDTH + ((span - 1) as f64) * BLOCK_SPACING
        } else {
            width
        };

        // Check if we need to wrap to next row
        if col + span > columns && col > 0 {
            rows.push(current_row);
            current_row = Vec::new();
            col = 0;
        }

        current_row.push((id, block, block_width, height));
        col += span;

        // Wrap if we've filled the row
        if col >= columns {
            rows.push(current_row);
            current_row = Vec::new();
            col = 0;
        }
    }

    // Don't forget the last row if it has items
    if !current_row.is_empty() {
        rows.push(current_row);
    }

    rows
}

/// Layout children inside a composite block
fn layout_composite_children(
    child_blocks: &[(&str, &Block)],
    _sizes: &HashMap<String, (f64, f64)>,
    start_x: f64,
    start_y: f64,
    available_width: f64,
    available_height: f64,
) -> Vec<PositionedBlock> {
    let mut positioned = Vec::new();
    let num_children = child_blocks.len();

    if num_children == 0 {
        return positioned;
    }

    // Calculate uniform size for children
    let child_spacing = BLOCK_SPACING;
    let total_spacing = (num_children - 1) as f64 * child_spacing;
    let child_width = (available_width - total_spacing) / num_children as f64;
    let child_height = available_height;

    let mut current_x = start_x;

    for &(id, block) in child_blocks {
        if block.block_type == BlockType::Space {
            current_x += child_width + child_spacing;
            continue;
        }

        positioned.push(PositionedBlock {
            id: id.to_string(),
            label: block.label.clone().unwrap_or_else(|| id.to_string()),
            block_type: block.block_type.clone(),
            x: current_x,
            y: start_y,
            width: child_width,
            height: child_height,
            column_span: block.width_in_columns.unwrap_or(1),
            styles: block.styles.clone(),
            classes: block.classes.clone(),
        });

        current_x += child_width + child_spacing;
    }

    positioned
}

/// Calculate size needed for a composite block to contain its children
fn calculate_composite_size(
    child_ids: &[&str],
    blocks: &HashMap<String, Block>,
    sizes: &HashMap<String, (f64, f64)>,
) -> (f64, f64) {
    let padding = BLOCK_PADDING;
    let mut total_width: f64 = 0.0;
    let mut max_height: f64 = 0.0;

    for id in child_ids {
        if let Some(block) = blocks.get(*id) {
            if block.block_type == BlockType::Space {
                total_width += MIN_BLOCK_WIDTH + BLOCK_SPACING;
                continue;
            }
            let (w, h) = sizes
                .get(*id)
                .cloned()
                .unwrap_or((MIN_BLOCK_WIDTH, MIN_BLOCK_HEIGHT));
            total_width += w + BLOCK_SPACING;
            max_height = max_height.max(h);
        }
    }

    // Remove trailing spacing, add padding
    if total_width > BLOCK_SPACING {
        total_width -= BLOCK_SPACING;
    }

    (total_width + padding * 2.0, max_height + padding * 2.0)
}

/// Calculate bounds of all blocks
fn calculate_bounds(blocks: &[PositionedBlock]) -> (f64, f64, f64, f64) {
    if blocks.is_empty() {
        return (0.0, 100.0, 0.0, 100.0);
    }

    let mut min_x = f64::MAX;
    let mut max_x = f64::MIN;
    let mut min_y = f64::MAX;
    let mut max_y = f64::MIN;

    for block in blocks {
        min_x = min_x.min(block.x);
        max_x = max_x.max(block.x + block.width);
        min_y = min_y.min(block.y);
        max_y = max_y.max(block.y + block.height);
    }

    (min_x, max_x, min_y, max_y)
}

/// Position edges based on block positions
fn position_edges(
    edges: &[crate::diagrams::block::Edge],
    block_map: &HashMap<&str, &PositionedBlock>,
) -> Vec<PositionedEdge> {
    let mut positioned = Vec::new();

    for edge in edges {
        if let (Some(start_block), Some(end_block)) = (
            block_map.get(edge.start.as_str()),
            block_map.get(edge.end.as_str()),
        ) {
            // Calculate block centers
            let start_cx = start_block.x + start_block.width / 2.0;
            let start_cy = start_block.y + start_block.height / 2.0;
            let end_cx = end_block.x + end_block.width / 2.0;
            let end_cy = end_block.y + end_block.height / 2.0;

            // Check if blocks overlap in x or y ranges
            let x_overlap = blocks_overlap_x(start_block, end_block);
            let y_overlap = blocks_overlap_y(start_block, end_block);

            let (start_x, start_y, end_x, end_y) = if x_overlap && !y_overlap {
                // Vertical edge - use shared x coordinate
                let shared_x = (start_cx + end_cx) / 2.0;
                let start_y = if start_cy < end_cy {
                    start_block.y + start_block.height // Bottom of start
                } else {
                    start_block.y // Top of start
                };
                let end_y = if start_cy < end_cy {
                    end_block.y // Top of end
                } else {
                    end_block.y + end_block.height // Bottom of end
                };
                (shared_x, start_y, shared_x, end_y)
            } else if y_overlap && !x_overlap {
                // Horizontal edge - use shared y coordinate
                let shared_y = (start_cy + end_cy) / 2.0;
                let start_x = if start_cx < end_cx {
                    start_block.x + start_block.width // Right of start
                } else {
                    start_block.x // Left of start
                };
                let end_x = if start_cx < end_cx {
                    end_block.x // Left of end
                } else {
                    end_block.x + end_block.width // Right of end
                };
                (start_x, shared_y, end_x, shared_y)
            } else {
                // Diagonal/curved edge - use original calculation
                let (sx, sy) = get_edge_point(start_block, start_cx, start_cy, end_cx, end_cy);
                let (ex, ey) = get_edge_point(end_block, end_cx, end_cy, start_cx, start_cy);
                (sx, sy, ex, ey)
            };

            positioned.push(PositionedEdge {
                start: edge.start.clone(),
                end: edge.end.clone(),
                label: edge.label.clone(),
                start_x,
                start_y,
                end_x,
                end_y,
            });
        }
    }

    positioned
}

/// Check if two blocks overlap in x range
fn blocks_overlap_x(a: &PositionedBlock, b: &PositionedBlock) -> bool {
    let a_left = a.x;
    let a_right = a.x + a.width;
    let b_left = b.x;
    let b_right = b.x + b.width;
    // Overlap if one block's left is before other's right and vice versa
    a_left < b_right && b_left < a_right
}

/// Check if two blocks overlap in y range
fn blocks_overlap_y(a: &PositionedBlock, b: &PositionedBlock) -> bool {
    let a_top = a.y;
    let a_bottom = a.y + a.height;
    let b_top = b.y;
    let b_bottom = b.y + b.height;
    a_top < b_bottom && b_top < a_bottom
}

/// Get the edge connection point on a block's boundary
fn get_edge_point(
    block: &PositionedBlock,
    from_x: f64,
    from_y: f64,
    to_x: f64,
    to_y: f64,
) -> (f64, f64) {
    let dx = to_x - from_x;
    let dy = to_y - from_y;

    let half_width = block.width / 2.0;
    let half_height = block.height / 2.0;

    // Simple edge point calculation - find intersection with rectangle
    if dx.abs() > dy.abs() {
        // Horizontal dominant - connect to left or right side
        let sign = if dx > 0.0 { 1.0 } else { -1.0 };
        (from_x + sign * half_width, from_y)
    } else {
        // Vertical dominant - connect to top or bottom
        let sign = if dy > 0.0 { 1.0 } else { -1.0 };
        (from_x, from_y + sign * half_height)
    }
}

/// Render all edges
fn render_edges(edges: &[PositionedEdge], _config: &RenderConfig) -> SvgElement {
    let mut children = Vec::new();

    for edge in edges {
        // Calculate midpoint for curved path
        let mid_x = (edge.start_x + edge.end_x) / 2.0;
        let mid_y = (edge.start_y + edge.end_y) / 2.0;

        // Create path with slight curve
        let path = format!(
            "M {} {} Q {} {} {} {}",
            edge.start_x, edge.start_y, mid_x, mid_y, edge.end_x, edge.end_y
        );

        children.push(SvgElement::Path {
            d: path,
            attrs: Attrs::new()
                .with_class("block-edge")
                .with_fill("none")
                .with_stroke_width(1.0)
                .with_attr("marker-end", "url(#arrowhead)"),
        });

        // Add label if present
        if let Some(label) = &edge.label {
            let label_x = mid_x;
            let label_y = mid_y - 10.0;

            children.push(SvgElement::Text {
                x: label_x,
                y: label_y,
                content: label.clone(),
                attrs: Attrs::new()
                    .with_class("edge-label")
                    .with_attr("text-anchor", "middle")
                    .with_attr("dominant-baseline", "middle")
                    .with_attr("font-size", &format!("{}px", FONT_SIZE - 2.0)),
            });
        }
    }

    SvgElement::Group {
        children,
        attrs: Attrs::new().with_class("block-edges"),
    }
}

/// Render all blocks
fn render_blocks(blocks: &[PositionedBlock], config: &RenderConfig) -> SvgElement {
    let mut children = Vec::new();

    for block in blocks {
        children.push(render_block_node(block, config));
    }

    SvgElement::Group {
        children,
        attrs: Attrs::new().with_class("block-nodes"),
    }
}

/// Render a single block
fn render_block_node(block: &PositionedBlock, _config: &RenderConfig) -> SvgElement {
    let mut node_children = Vec::new();

    // Build class string - include "node" for eval detection
    let mut class_list = vec![
        "node".to_string(),
        "block-node".to_string(),
        format!("block-{:?}", block.block_type).to_lowercase(),
    ];
    class_list.extend(block.classes.clone());

    // Render shape based on type
    let shape = render_block_shape(block);
    node_children.push(shape);

    // Render text label
    let text = render_block_text(block);
    node_children.push(text);

    // Wrap in a group
    SvgElement::Group {
        children: node_children,
        attrs: Attrs::new()
            .with_class(&class_list.join(" "))
            .with_id(&format!("block-{}", block.id))
            .with_attr("transform", &format!("translate({}, {})", block.x, block.y)),
    }
}

/// Build inline style string from block styles
fn build_inline_style(styles: &[String]) -> Option<String> {
    if styles.is_empty() {
        return None;
    }
    // Join styles with semicolons
    let style_str = styles.join(";");
    if style_str.is_empty() {
        None
    } else {
        Some(style_str)
    }
}

/// Render block shape based on type
fn render_block_shape(block: &PositionedBlock) -> SvgElement {
    let w = block.width;
    let h = block.height;
    let inline_style = build_inline_style(&block.styles);

    // Helper to create attrs with optional inline style
    let make_attrs = |class: &str| {
        let mut attrs = Attrs::new().with_class(class);
        if let Some(ref style) = inline_style {
            attrs = attrs.with_attr("style", style);
        }
        attrs
    };

    match block.block_type {
        BlockType::Square => SvgElement::Rect {
            x: 0.0,
            y: 0.0,
            width: w,
            height: h,
            rx: None,
            ry: None,
            attrs: make_attrs("node-bkg"),
        },
        BlockType::Round => SvgElement::Rect {
            x: 0.0,
            y: 0.0,
            width: w,
            height: h,
            rx: Some(BLOCK_PADDING),
            ry: Some(BLOCK_PADDING),
            attrs: make_attrs("node-bkg"),
        },
        BlockType::Circle => {
            let radius = w.min(h) / 2.0;
            SvgElement::Circle {
                cx: w / 2.0,
                cy: h / 2.0,
                r: radius,
                attrs: make_attrs("node-bkg"),
            }
        }
        BlockType::DoubleCircle => {
            let outer_radius = w.min(h) / 2.0;
            let inner_radius = outer_radius * 0.85;
            // Create two circles using a group
            let outer = SvgElement::Circle {
                cx: w / 2.0,
                cy: h / 2.0,
                r: outer_radius,
                attrs: make_attrs("node-bkg"),
            };
            let inner = SvgElement::Circle {
                cx: w / 2.0,
                cy: h / 2.0,
                r: inner_radius,
                attrs: Attrs::new().with_class("node-bkg-inner").with_fill("none"),
            };
            SvgElement::Group {
                children: vec![outer, inner],
                attrs: Attrs::new(),
            }
        }
        BlockType::Diamond => {
            let cx = w / 2.0;
            let cy = h / 2.0;
            let points = format!("{},{} {},{} {},{} {},{}", cx, 0.0, w, cy, cx, h, 0.0, cy);
            SvgElement::PolygonStr {
                points,
                attrs: make_attrs("node-bkg"),
            }
        }
        BlockType::Hexagon => {
            let m = h / 4.0;
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
                attrs: make_attrs("node-bkg"),
            }
        }
        BlockType::Stadium => {
            let r = h / 2.0;
            SvgElement::Rect {
                x: 0.0,
                y: 0.0,
                width: w,
                height: h,
                rx: Some(r),
                ry: Some(r),
                attrs: make_attrs("node-bkg"),
            }
        }
        BlockType::Subroutine => {
            // Double rectangle
            let outer = SvgElement::Rect {
                x: 0.0,
                y: 0.0,
                width: w,
                height: h,
                rx: None,
                ry: None,
                attrs: make_attrs("node-bkg"),
            };
            let inner = SvgElement::Rect {
                x: 5.0,
                y: 0.0,
                width: w - 10.0,
                height: h,
                rx: None,
                ry: None,
                attrs: Attrs::new().with_class("node-bkg-inner").with_fill("none"),
            };
            SvgElement::Group {
                children: vec![outer, inner],
                attrs: Attrs::new(),
            }
        }
        BlockType::Cylinder => {
            let ry = h * 0.1;
            let path = format!(
                "M 0 {} A {} {} 0 1 0 {} {} V {} A {} {} 0 1 0 0 {} V {} Z \
                 M 0 {} A {} {} 0 1 1 {} {} A {} {} 0 1 1 0 {}",
                ry,
                w / 2.0,
                ry,
                w,
                ry,
                h - ry,
                w / 2.0,
                ry,
                h - ry,
                ry,
                ry,
                w / 2.0,
                ry,
                w,
                ry,
                w / 2.0,
                ry,
                ry
            );
            SvgElement::Path {
                d: path,
                attrs: make_attrs("node-bkg"),
            }
        }
        BlockType::LeanRight | BlockType::Trapezoid => {
            let skew = 10.0;
            let points = format!(
                "{},{} {},{} {},{} {},{}",
                skew,
                0.0,
                w,
                0.0,
                w - skew,
                h,
                0.0,
                h
            );
            SvgElement::PolygonStr {
                points,
                attrs: make_attrs("node-bkg"),
            }
        }
        BlockType::LeanLeft | BlockType::InvTrapezoid => {
            let skew = 10.0;
            let points = format!(
                "{},{} {},{} {},{} {},{}",
                0.0,
                0.0,
                w - skew,
                0.0,
                w,
                h,
                skew,
                h
            );
            SvgElement::PolygonStr {
                points,
                attrs: make_attrs("node-bkg"),
            }
        }
        BlockType::BlockArrow => {
            // Arrow pointing right
            let arrow_width = w * 0.7;
            let arrow_height = h * 0.3;
            let path = format!(
                "M 0 {} V {} H {} V 0 L {} {} L {} {} V {} H 0 Z",
                arrow_height,
                h - arrow_height,
                arrow_width,
                w,
                h / 2.0,
                arrow_width,
                h,
                h - arrow_height
            );
            SvgElement::Path {
                d: path,
                attrs: make_attrs("node-bkg"),
            }
        }
        BlockType::Composite => {
            // Composite blocks render as dashed rectangle containers
            SvgElement::Rect {
                x: 0.0,
                y: 0.0,
                width: w,
                height: h,
                rx: None,
                ry: None,
                attrs: make_attrs("block-composite"),
            }
        }
        BlockType::Space | BlockType::Edge => {
            // These don't render shapes
            SvgElement::Group {
                children: Vec::new(),
                attrs: Attrs::new(),
            }
        }
    }
}

/// Render block text label
fn render_block_text(block: &PositionedBlock) -> SvgElement {
    SvgElement::Text {
        x: block.width / 2.0,
        y: block.height / 2.0,
        content: block.label.clone(),
        attrs: Attrs::new()
            .with_class("block-label")
            .with_attr("text-anchor", "middle")
            .with_attr("dominant-baseline", "middle")
            .with_attr("font-size", &format!("{}px", FONT_SIZE)),
    }
}

/// Generate CSS for block diagrams
fn generate_block_css(
    config: &RenderConfig,
    classes: &HashMap<String, crate::diagrams::block::ClassDef>,
) -> String {
    let theme = &config.theme;

    // Generate custom class styles
    let mut custom_css = String::new();
    for (name, class_def) in classes {
        let styles: String = class_def.styles.join(";");
        custom_css.push_str(&format!(
            ".{} .node-bkg {{ {} }}\n",
            name,
            styles.replace(',', ";")
        ));
    }

    format!(
        r#"
.block-node {{
  cursor: pointer;
}}
.block-label {{
  font-family: {font_family};
  fill: {text_color};
}}
.node-bkg {{
  fill: {node_fill};
  stroke: {node_border};
  stroke-width: 2px;
}}
.node-bkg-inner {{
  stroke: {node_border};
  stroke-width: 1px;
}}
.block-edge {{
  stroke: {line_color};
  stroke-width: 1px;
  fill: none;
}}
.edge-label {{
  font-family: {font_family};
  fill: {text_color};
  font-size: 12px;
}}
.block-edges marker {{
  fill: {line_color};
}}
.block-composite {{
  fill: {secondary_color};
  stroke: {node_border};
  stroke-width: 1px;
  stroke-dasharray: 5,5;
}}
{custom_css}
"#,
        font_family = theme.font_family,
        text_color = theme.primary_text_color,
        node_fill = theme.primary_color,
        node_border = theme.primary_border_color,
        line_color = theme.line_color,
        secondary_color = theme.secondary_color,
        custom_css = custom_css
    )
}
