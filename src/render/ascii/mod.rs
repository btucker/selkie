//! ASCII (Text User Interface) renderer for diagrams.
//!
//! Produces character-art output using box-drawing characters for node shapes
//! and braille dots for edge routing. Pipe-friendly, works in every terminal.
//!
//! Supports any diagram type that implements `ToLayoutGraph`, not just flowcharts.

pub mod block;
pub mod c4;
pub mod canvas;
pub mod edges;
pub mod gantt;
pub mod gitgraph;
pub mod journey;
pub mod kanban;
pub mod mindmap;
pub mod packet;
pub mod pie;
pub mod quadrant;
pub mod radar;
pub mod sankey;
pub mod scale;
pub mod sequence;
pub mod shapes;
pub mod timeline;
pub mod treemap;
pub mod xychart;

use std::collections::{HashMap, HashSet};

use crate::diagrams::class::ClassDb;
use crate::diagrams::er::ErDb;
use crate::diagrams::flowchart::FlowchartDb;
use crate::error::Result;
use crate::layout::{LayoutGraph, Point, ToLayoutGraph};

use shapes::RenderedShape;

pub use sequence::render_sequence_ascii;

use scale::CellScale;
use shapes::{render_class_box, render_shape};

/// Render any laid-out graph as character art.
///
/// This is the generic entry point for ASCII rendering. It works with any diagram
/// type that produces a `LayoutGraph` via `ToLayoutGraph`. Node labels are taken
/// from `node.label` (falling back to `node.id`), with HTML tags cleaned.
pub fn render_graph_ascii(graph: &LayoutGraph) -> Result<String> {
    render_ascii_impl(graph, &|node| generic_node_label(node))
}

/// Render a flowchart as character art.
///
/// Uses `FlowchartDb` for richer label lookup (vertex text), falling back to
/// the layout node label. For non-flowchart diagrams, use `render_graph_ascii`.
pub fn render_flowchart_ascii(db: &FlowchartDb, graph: &LayoutGraph) -> Result<String> {
    render_ascii_impl(graph, &|node| flowchart_node_label(db, node))
}

/// Simplify ER edge routes by replacing dagre's multi-point routing with
/// direct source→target lines. Dagre inserts dummy nodes for edges that span
/// multiple layers, which can create routes that swing far beyond entity bounds.
/// For ASCII ER diagrams, direct lines produce more compact output.
fn simplify_er_edge_routes(graph: &LayoutGraph) -> LayoutGraph {
    let mut simplified = graph.clone();

    // Build node position lookup (non-dummy nodes only)
    let node_positions: HashMap<&str, (f64, f64, f64, f64)> = graph
        .nodes
        .iter()
        .filter(|n| !n.is_dummy)
        .filter_map(|n| {
            n.x.zip(n.y)
                .map(|(x, y)| (n.id.as_str(), (x, y, n.width, n.height)))
        })
        .collect();

    for edge in &mut simplified.edges {
        if edge.bend_points.len() <= 2 {
            continue;
        }

        let source_id = edge.sources.first().map(|s| s.as_str());
        let target_id = edge.targets.first().map(|s| s.as_str());

        let (src_pos, tgt_pos) = match (
            source_id.and_then(|id| node_positions.get(id)),
            target_id.and_then(|id| node_positions.get(id)),
        ) {
            (Some(s), Some(t)) => (*s, *t),
            _ => continue,
        };

        // Compute attachment points on entity boundaries
        let (src_x, src_y, src_w, src_h) = src_pos;
        let (tgt_x, tgt_y, tgt_w, tgt_h) = tgt_pos;

        let src_cx = src_x + src_w / 2.0;
        let src_cy = src_y + src_h / 2.0;
        let tgt_cx = tgt_x + tgt_w / 2.0;
        let tgt_cy = tgt_y + tgt_h / 2.0;

        let dx = tgt_cx - src_cx;
        let dy = tgt_cy - src_cy;

        // Determine start point (on source entity boundary)
        let start = if dy.abs() > dx.abs() {
            // Vertical dominant: exit from bottom or top
            if dy > 0.0 {
                Point::new(src_cx, src_y + src_h) // bottom
            } else {
                Point::new(src_cx, src_y) // top
            }
        } else if dx > 0.0 {
            Point::new(src_x + src_w, src_cy) // right
        } else {
            Point::new(src_x, src_cy) // left
        };

        // Determine end point (on target entity boundary)
        let end = if dy.abs() > dx.abs() {
            if dy > 0.0 {
                Point::new(tgt_cx, tgt_y) // top
            } else {
                Point::new(tgt_cx, tgt_y + tgt_h) // bottom
            }
        } else if dx > 0.0 {
            Point::new(tgt_x, tgt_cy) // left
        } else {
            Point::new(tgt_x + tgt_w, tgt_cy) // right
        };

        edge.bend_points = vec![start, end];
        edge.label_position = Some(Point::new((start.x + end.x) / 2.0, (start.y + end.y) / 2.0));
    }

    simplified
}

/// Render an ER diagram as character art.
///
/// Uses `ErDb` to render entity boxes with attribute tables (type, name, keys
/// columns), matching the SVG renderer's table-style layout. Relationship
/// edges are rendered using the standard braille edge router.
pub fn render_er_ascii(db: &ErDb, graph: &LayoutGraph) -> Result<String> {
    let scale = CellScale::default();

    // Simplify edge routes: replace dagre's complex multi-point routing with
    // direct source→target lines. Dagre routes long-span edges through dummy
    // nodes that can extend far beyond entity bounds (e.g., the "has" edge
    // taking a 30-row vertical detour). Direct lines keep the diagram compact;
    // braille compositing already skips occupied cells, so edges naturally
    // "pass behind" any intervening entity boxes.
    let simplified_graph = simplify_er_edge_routes(graph);
    let graph = &simplified_graph;

    // Determine canvas dimensions from graph bounds
    let graph_width = graph.width.unwrap_or(400.0);
    let graph_height = graph.height.unwrap_or(300.0);
    let offset_x = graph.bounds_x.unwrap_or(0.0);
    let offset_y = graph.bounds_y.unwrap_or(0.0);

    let canvas_cols = scale.to_col(graph_width) + 8;
    let canvas_rows = scale.to_row(graph_height) + 4;

    let mut canvas: Vec<Vec<char>> = vec![vec![' '; canvas_cols]; canvas_rows];
    let mut occupied: Vec<Vec<bool>> = vec![vec![false; canvas_cols]; canvas_rows];

    // Build entity name lookup: entity_id → entity name
    let entities = db.get_entities();
    let id_to_name: HashMap<&str, &str> = entities
        .iter()
        .map(|(name, entity)| (entity.id.as_str(), name.as_str()))
        .collect();

    // Render each entity node as a table box
    for node in &graph.nodes {
        if node.is_dummy {
            continue;
        }
        let (nx, ny) = match (node.x, node.y) {
            (Some(x), Some(y)) => (x - offset_x, y - offset_y),
            _ => continue,
        };

        let entity_name = id_to_name.get(node.id.as_str()).copied();
        let entity = entity_name.and_then(|n| entities.get(n));

        // node.x/y are top-left coordinates (dagre center coords converted
        // by apply_results_recursive), so use them directly as cell start.
        let cell_w = scale.to_cell_width(node.width);
        let cell_h = scale.to_cell_height(node.height);
        let col_start = scale.to_col(nx);
        let row_start = scale.to_row(ny);

        // Get display name
        let display_name = entity
            .map(|e| {
                if !e.alias.is_empty() {
                    e.alias.as_str()
                } else {
                    e.label.as_str()
                }
            })
            .or(node.label.as_deref())
            .unwrap_or(&node.id);

        let rendered_lines = if let Some(entity) = entity {
            render_er_entity_box(display_name, &entity.attributes, cell_w)
        } else {
            // Fallback: simple box with label
            let shape = render_shape(&node.shape, display_name, cell_w, cell_h);
            shape.lines
        };

        // Blit onto canvas
        for (r, line) in rendered_lines.iter().enumerate() {
            let canvas_row = row_start + r;
            if canvas_row >= canvas_rows {
                break;
            }
            for (c, ch) in line.chars().enumerate() {
                let canvas_col = col_start + c;
                if canvas_col >= canvas_cols {
                    break;
                }
                if ch != ' ' {
                    canvas[canvas_row][canvas_col] = ch;
                }
                occupied[canvas_row][canvas_col] = true;
            }
        }
    }

    // Render edges
    edges::render_edges(
        graph,
        &scale,
        canvas_cols,
        canvas_rows,
        offset_x,
        offset_y,
        &occupied,
        &mut canvas,
    );

    // Convert canvas to string
    let mut result = String::new();
    let mut last_non_empty = 0;
    for (i, row) in canvas.iter().enumerate() {
        if row.iter().any(|&c| c != ' ') {
            last_non_empty = i;
        }
    }

    for row in &canvas[..=last_non_empty] {
        let line: String = row.iter().collect();
        result.push_str(line.trim_end());
        result.push('\n');
    }

    Ok(result)
}

/// Render an ER entity as a box with attribute table rows.
///
/// Layout:
/// ```text
/// ┌──────────────────────┐
/// │      CUSTOMER        │
/// ├──────┬───────┬───────┤
/// │string│ name  │       │
/// │string│ email │  PK   │
/// └──────┴───────┴───────┘
/// ```
fn render_er_entity_box(
    name: &str,
    attributes: &[crate::diagrams::er::Attribute],
    min_width: usize,
) -> Vec<String> {
    if attributes.is_empty() {
        // Simple box for entities without attributes
        let name_len = name.chars().count();
        let inner_w = (name_len + 2).max(min_width.saturating_sub(2));
        let width = inner_w + 2; // +2 for borders

        let mut lines = Vec::new();
        lines.push(format!("┌{}┐", "─".repeat(inner_w)));
        // Center the name
        let pad_total = inner_w.saturating_sub(name_len);
        let pad_left = pad_total / 2;
        let pad_right = pad_total - pad_left;
        lines.push(format!(
            "│{}{}{}│",
            " ".repeat(pad_left),
            name,
            " ".repeat(pad_right)
        ));
        lines.push(format!("└{}┘", "─".repeat(inner_w)));
        let _ = width; // used for consistency check
        return lines;
    }

    // Calculate column widths from content
    let mut max_type_w = 0usize;
    let mut max_name_w = 0usize;
    let mut max_keys_w = 0usize;

    for attr in attributes {
        max_type_w = max_type_w.max(attr.attr_type.chars().count());
        max_name_w = max_name_w.max(attr.name.chars().count());
        let keys_str: String = attr
            .keys
            .iter()
            .map(|k| k.as_str())
            .collect::<Vec<_>>()
            .join(",");
        max_keys_w = max_keys_w.max(keys_str.chars().count());
    }

    // Add padding (1 char each side)
    let type_col = max_type_w + 2;
    let name_col = max_name_w + 2;
    let keys_col = max_keys_w.max(1) + 2; // at least 3 wide

    let inner_w = type_col + 1 + name_col + 1 + keys_col; // +1 for each │ divider
    let name_len = name.chars().count();
    // Ensure header can fit entity name
    let inner_w = inner_w.max(name_len + 2);

    // Recalculate keys column to absorb any extra width
    let keys_col_adjusted = inner_w - type_col - 1 - name_col - 1;

    let mut lines = Vec::new();

    // Top border
    lines.push(format!("┌{}┐", "─".repeat(inner_w)));

    // Header row (entity name centered)
    let pad_total = inner_w.saturating_sub(name_len);
    let pad_left = pad_total / 2;
    let pad_right = pad_total - pad_left;
    lines.push(format!(
        "│{}{}{}│",
        " ".repeat(pad_left),
        name,
        " ".repeat(pad_right)
    ));

    // Divider between header and attributes
    lines.push(format!(
        "├{}┬{}┬{}┤",
        "─".repeat(type_col),
        "─".repeat(name_col),
        "─".repeat(keys_col_adjusted)
    ));

    // Attribute rows
    for attr in attributes {
        let keys_str: String = attr
            .keys
            .iter()
            .map(|k| k.as_str())
            .collect::<Vec<_>>()
            .join(",");

        let type_str = format_cell(&attr.attr_type, type_col);
        let name_str = format_cell(&attr.name, name_col);
        let keys_str = format_cell(&keys_str, keys_col_adjusted);
        lines.push(format!("│{}│{}│{}│", type_str, name_str, keys_str));
    }

    // Bottom border
    lines.push(format!(
        "└{}┴{}┴{}┘",
        "─".repeat(type_col),
        "─".repeat(name_col),
        "─".repeat(keys_col_adjusted)
    ));

    lines
}

/// Format a string into a fixed-width cell, left-aligned with padding.
fn format_cell(text: &str, width: usize) -> String {
    let text_len = text.chars().count();
    if text_len >= width {
        text.chars().take(width).collect()
    } else {
        // 1 char left padding, rest right
        let content_w = width - 1;
        let pad_right = content_w.saturating_sub(text_len);
        format!(" {}{}", text, " ".repeat(pad_right))
    }
}

/// Core ASCII renderer implementation, parameterized by a label lookup function.
fn render_ascii_impl(
    graph: &LayoutGraph,
    label_fn: &dyn Fn(&crate::layout::LayoutNode) -> String,
) -> Result<String> {
    let scale = CellScale::default();

    // Determine canvas dimensions from graph bounds
    let graph_width = graph.width.unwrap_or(400.0);
    let graph_height = graph.height.unwrap_or(300.0);
    let offset_x = graph.bounds_x.unwrap_or(0.0);
    let offset_y = graph.bounds_y.unwrap_or(0.0);

    let canvas_cols = scale.to_col(graph_width) + 8;
    let canvas_rows = scale.to_row(graph_height) + 4;

    // Create a canvas (2D grid of characters)
    let mut canvas: Vec<Vec<char>> = vec![vec![' '; canvas_cols]; canvas_rows];
    // Track which cells are occupied by nodes (for edge compositing)
    let mut occupied: Vec<Vec<bool>> = vec![vec![false; canvas_cols]; canvas_rows];

    // Collect container node IDs — these are compound nodes whose bounding box
    // encompasses their children. We render them as just a label, not a full box.
    // For flowcharts these are subgraphs; for other diagram types they may be
    // composite states, packages, etc.
    //
    // Detection: a node is a container if it has children OR if any other node
    // has parent_id pointing to it.
    let parent_ids: HashSet<&str> = graph
        .nodes
        .iter()
        .filter_map(|n| n.parent_id.as_deref())
        .collect();
    let container_ids: HashSet<&str> = graph
        .nodes
        .iter()
        .filter(|n| !n.children.is_empty() || parent_ids.contains(n.id.as_str()))
        .map(|n| n.id.as_str())
        .collect();

    // Render container nodes first (background layer — bordered box with title).
    // Sort by area descending so largest containers render first (nested ones on top).
    struct SubgraphLabel {
        row: usize,
        label_col_start: usize,
        label: String,
        box_col_start: usize,
        box_w: usize,
    }
    let mut subgraph_labels: Vec<SubgraphLabel> = Vec::new();

    let mut container_nodes: Vec<&crate::layout::LayoutNode> = graph
        .nodes
        .iter()
        .filter(|n| !n.is_dummy && container_ids.contains(n.id.as_str()))
        .collect();
    container_nodes.sort_by(|a, b| {
        let area_a = a.width * a.height;
        let area_b = b.width * b.height;
        area_b
            .partial_cmp(&area_a)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    for node in container_nodes {
        let (nx, ny) = match (node.x, node.y) {
            (Some(x), Some(y)) => (x - offset_x, y - offset_y),
            _ => continue,
        };

        let label = label_fn(node);

        // Calculate the bounding box from the node's layout dimensions
        let label_chars = label.chars().count();
        let box_w = scale.to_cell_width(node.width).max(label_chars + 4);
        let box_h = scale.to_cell_height(node.height).max(3);
        let col_center = scale.to_col(nx + node.width / 2.0);
        let row_center = scale.to_row(ny + node.height / 2.0);
        let col_start = col_center.saturating_sub(box_w / 2);
        let row_start = row_center.saturating_sub(box_h / 2);

        // Label goes right after "╭─" at the start of the top border
        let label_col_start = col_start + 2;

        // Draw top border: ╭─Label─────────╮
        if row_start < canvas_rows {
            if col_start < canvas_cols {
                canvas[row_start][col_start] = '╭';
                occupied[row_start][col_start] = true;
            }
            if col_start + 1 < canvas_cols {
                canvas[row_start][col_start + 1] = '─';
                occupied[row_start][col_start + 1] = true;
            }
            // Label text
            for (i, ch) in label.chars().enumerate() {
                let c = label_col_start + i;
                if c >= canvas_cols {
                    break;
                }
                canvas[row_start][c] = ch;
                occupied[row_start][c] = true;
            }
            // Remaining border after label
            let after_label = label_col_start + label_chars;
            let right_col = col_start + box_w - 1;
            for c in after_label..right_col {
                if c >= canvas_cols {
                    break;
                }
                canvas[row_start][c] = '─';
                occupied[row_start][c] = true;
            }
            if right_col < canvas_cols {
                canvas[row_start][right_col] = '╮';
                occupied[row_start][right_col] = true;
            }
        }

        // Side borders
        for r in 1..box_h.saturating_sub(1) {
            let row = row_start + r;
            if row >= canvas_rows {
                break;
            }
            if col_start < canvas_cols {
                canvas[row][col_start] = '│';
                occupied[row][col_start] = true;
            }
            let right = col_start + box_w - 1;
            if right < canvas_cols {
                canvas[row][right] = '│';
                occupied[row][right] = true;
            }
        }

        // Bottom border: ╰─────────────────╯
        let bot_row = row_start + box_h - 1;
        if bot_row < canvas_rows {
            if col_start < canvas_cols {
                canvas[bot_row][col_start] = '╰';
                occupied[bot_row][col_start] = true;
            }
            for i in 1..box_w.saturating_sub(1) {
                let c = col_start + i;
                if c >= canvas_cols {
                    break;
                }
                canvas[bot_row][c] = '─';
                occupied[bot_row][c] = true;
            }
            let br = col_start + box_w - 1;
            if br < canvas_cols {
                canvas[bot_row][br] = '╯';
                occupied[bot_row][br] = true;
            }
        }

        subgraph_labels.push(SubgraphLabel {
            row: row_start,
            label_col_start,
            label,
            box_col_start: col_start,
            box_w,
        });
    }

    // Render regular (non-subgraph) nodes, sorted by area ascending so
    // smaller nodes render first and aren't occluded by large shapes (e.g., diamonds).
    let mut regular_nodes: Vec<&crate::layout::LayoutNode> = graph
        .nodes
        .iter()
        .filter(|n| !n.is_dummy && !container_ids.contains(n.id.as_str()))
        .collect();
    // Sort by area ascending so smaller nodes render first. The blit logic
    // protects existing label text from being overwritten by border characters,
    // ensuring all node labels remain readable even when cells overlap.
    regular_nodes.sort_by(|a, b| {
        let area_a = a.width * a.height;
        let area_b = b.width * b.height;
        area_a
            .partial_cmp(&area_b)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    // Two-pass rendering: first blit all shapes (borders + labels), then
    // re-stamp all labels on top so they're never occluded by overlapping borders.
    // This handles coarse cell-grid quantization where nodes can overlap.
    struct NodePlacement {
        col_start: usize,
        row_start: usize,
        rendered: shapes::RenderedShape,
        label_row: usize, // which row of the rendered shape contains the label
    }
    let mut placements: Vec<NodePlacement> = Vec::new();

    for node in regular_nodes {
        let (nx, ny) = match (node.x, node.y) {
            (Some(x), Some(y)) => (x - offset_x, y - offset_y),
            _ => continue,
        };

        let label = label_fn(node);

        let cell_w = scale.to_cell_width(node.width);
        let cell_h = scale.to_cell_height(node.height);

        let rendered = render_shape(&node.shape, &label, cell_w, cell_h);

        // node.x/y are top-left coordinates (dagre center was converted by
        // apply_results_recursive). Compute the true center, then center the
        // rendered shape on it — shapes can be wider/taller than the layout
        // allocation when the label is longer than the size estimate.
        let center_col = scale.to_col(nx + node.width / 2.0);
        let center_row = scale.to_row(ny + node.height / 2.0);
        let col_start = center_col.saturating_sub(rendered.width / 2);
        let row_start = center_row.saturating_sub(rendered.height / 2);

        // Pass 1: Blit shape (borders can overwrite each other)
        for (r, line) in rendered.lines.iter().enumerate() {
            let canvas_row = row_start + r;
            if canvas_row >= canvas_rows {
                break;
            }
            for (c, ch) in line.chars().enumerate() {
                let canvas_col = col_start + c;
                if canvas_col >= canvas_cols {
                    break;
                }
                if ch != ' ' {
                    canvas[canvas_row][canvas_col] = ch;
                }
            }
        }
        // Mark the entire bounding box as occupied (not just non-space chars).
        // This prevents edges and arrow tips from overlapping node content.
        for r in 0..rendered.height {
            let canvas_row = row_start + r;
            if canvas_row >= canvas_rows {
                break;
            }
            for c in 0..rendered.width {
                let canvas_col = col_start + c;
                if canvas_col >= canvas_cols {
                    break;
                }
                occupied[canvas_row][canvas_col] = true;
            }
        }

        // Record label row for pass 2
        let label_row = rendered.height / 2;
        placements.push(NodePlacement {
            col_start,
            row_start,
            rendered,
            label_row,
        });
    }

    // Pass 2: Re-stamp all label rows so they're never occluded by borders.
    // This covers both regular node labels and subgraph labels.
    for p in &placements {
        let canvas_row = p.row_start + p.label_row;
        if canvas_row >= canvas_rows {
            continue;
        }
        if let Some(line) = p.rendered.lines.get(p.label_row) {
            for (c, ch) in line.chars().enumerate() {
                let canvas_col = p.col_start + c;
                if canvas_col >= canvas_cols {
                    break;
                }
                if ch != ' ' {
                    canvas[canvas_row][canvas_col] = ch;
                }
            }
        }
    }
    // Re-stamp subgraph labels on top of everything (container borders + labels)
    for sg in &subgraph_labels {
        if sg.row >= canvas_rows {
            continue;
        }
        let right_col = sg.box_col_start + sg.box_w.saturating_sub(1);
        // Re-stamp the full top border: ╭─Label─────╮
        if sg.box_col_start < canvas_cols {
            canvas[sg.row][sg.box_col_start] = '╭';
        }
        if sg.box_col_start + 1 < canvas_cols {
            canvas[sg.row][sg.box_col_start + 1] = '─';
        }
        let label_chars = sg.label.chars().count();
        for (i, ch) in sg.label.chars().enumerate() {
            let c = sg.label_col_start + i;
            if c < canvas_cols {
                canvas[sg.row][c] = ch;
            }
        }
        let after_label = sg.label_col_start + label_chars;
        let border_end = right_col.min(canvas_cols);
        if after_label < border_end {
            for col in &mut canvas[sg.row][after_label..border_end] {
                *col = '─';
            }
        }
        if right_col < canvas_cols {
            canvas[sg.row][right_col] = '╮';
        }
    }

    // Render edges (braille lines + arrows + labels)
    edges::render_edges(
        graph,
        &scale,
        canvas_cols,
        canvas_rows,
        offset_x,
        offset_y,
        &occupied,
        &mut canvas,
    );

    // Convert canvas to string, trimming trailing empty lines
    let mut result = String::new();
    let mut last_non_empty = 0;
    for (i, row) in canvas.iter().enumerate() {
        if row.iter().any(|&c| c != ' ') {
            last_non_empty = i;
        }
    }

    for row in &canvas[..=last_non_empty] {
        let line: String = row.iter().collect();
        result.push_str(line.trim_end());
        result.push('\n');
    }

    Ok(result)
}

/// Get the display label for a generic layout node, cleaning HTML tags.
///
/// Circle and DoubleCircle nodes with no label are start/end markers in state
/// diagrams — their IDs (e.g. `root_start`) should not be displayed.
fn generic_node_label(node: &crate::layout::LayoutNode) -> String {
    use crate::layout::NodeShape;
    // Start/end circle nodes have no meaningful label — return empty string
    // so the shape renderer produces just the ●/◉ symbol.
    if matches!(node.shape, NodeShape::Circle | NodeShape::DoubleCircle)
        && node.label.as_deref().is_none_or(|l| l.is_empty())
    {
        return String::new();
    }
    let raw = node.label.as_deref().unwrap_or(&node.id);
    clean_html_label(raw)
}

/// Get the display label for a flowchart node, preferring vertex text from the DB.
fn flowchart_node_label(db: &FlowchartDb, node: &crate::layout::LayoutNode) -> String {
    let raw = db
        .vertices()
        .iter()
        .find(|(id, _)| *id == &node.id)
        .and_then(|(_, v)| v.text.as_deref())
        .or(node.label.as_deref())
        .unwrap_or(&node.id);
    clean_html_label(raw)
}

/// Clean HTML line breaks and normalize whitespace for ASCII display.
fn clean_html_label(raw: &str) -> String {
    let cleaned = raw.replace("<br/>", " ").replace("<br>", " ");
    cleaned.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Pre-render a class node to its ASCII box, returning the `RenderedShape`.
fn render_class_node(db: &ClassDb, node_id: &str, label: &str) -> RenderedShape {
    if let Some(cn) = db.classes.get(node_id) {
        let annotations: Vec<&str> = cn.annotations.iter().map(|a| a.as_str()).collect();
        let members: Vec<String> = cn
            .members
            .iter()
            .map(|m| m.get_display_details().display_text)
            .collect();
        let methods: Vec<String> = cn
            .methods
            .iter()
            .map(|m| m.get_display_details().display_text)
            .collect();
        // Pass cell_w=0, cell_h=0 so the box sizes itself purely from content
        render_class_box(
            label,
            &annotations,
            &members.iter().map(|s| s.as_str()).collect::<Vec<_>>(),
            &methods.iter().map(|s| s.as_str()).collect::<Vec<_>>(),
            0,
            0,
        )
    } else {
        // Fallback for nodes not in the ClassDb (shouldn't happen, but safe)
        render_shape(
            &crate::layout::NodeShape::Rectangle,
            label,
            label.chars().count() + 4,
            3,
        )
    }
}

/// Render a class diagram as character art.
///
/// Each class becomes a multi-section box with optional annotations,
/// a class name header, attributes, and methods separated by horizontal
/// dividers (├─┤). Relations are rendered as braille edges with arrow tips.
///
/// This function re-computes layout internally using the actual ASCII cell
/// dimensions of each class box, ensuring that dagre positions nodes with
/// correct sizes so boxes never overlap.
pub fn render_class_ascii(db: &ClassDb, _graph: &LayoutGraph) -> Result<String> {
    let scale = CellScale::default();

    // Phase 1: Pre-render every class box to learn its actual cell dimensions.
    let mut rendered_shapes: HashMap<String, RenderedShape> = HashMap::new();
    for (id, class) in &db.classes {
        let label = if !class.label.is_empty() {
            &class.label
        } else {
            &class.id
        };
        rendered_shapes.insert(id.clone(), render_class_node(db, id, label));
    }

    // Phase 2: Build a new layout graph with node sizes matching the actual
    // ASCII cell dimensions (converted back to pixel-space for dagre).
    let estimator = crate::layout::CharacterSizeEstimator::default();
    let mut layout_graph = db.to_layout_graph(&estimator)?;

    for node in &mut layout_graph.nodes {
        if let Some(shape) = rendered_shapes.get(&node.id) {
            // Convert cell dimensions to pixel-space so dagre allocates
            // exactly the right amount of room.
            node.width = shape.width as f64 * scale.cell_width;
            node.height = shape.height as f64 * scale.cell_height;
        }
    }

    // Increase spacing for ASCII clarity (wider gaps between boxes).
    layout_graph.options.node_spacing = 80.0;
    layout_graph.options.layer_spacing = 48.0;

    let graph = crate::layout::layout(layout_graph)?;

    // Phase 3: Render onto canvas using the corrected layout positions.
    let graph_width = graph.width.unwrap_or(400.0);
    let graph_height = graph.height.unwrap_or(300.0);
    let offset_x = graph.bounds_x.unwrap_or(0.0);
    let offset_y = graph.bounds_y.unwrap_or(0.0);

    let canvas_cols = scale.to_col(graph_width) + 4;
    let canvas_rows = scale.to_row(graph_height) + 2;

    let mut canvas: Vec<Vec<char>> = vec![vec![' '; canvas_cols]; canvas_rows];
    let mut occupied: Vec<Vec<bool>> = vec![vec![false; canvas_cols]; canvas_rows];

    for node in &graph.nodes {
        if node.is_dummy {
            continue;
        }

        let (nx, ny) = match (node.x, node.y) {
            (Some(x), Some(y)) => (x - offset_x, y - offset_y),
            _ => continue,
        };

        // Use the pre-rendered shape (already correctly sized).
        let rendered = if let Some(shape) = rendered_shapes.get(&node.id) {
            shape.clone()
        } else {
            // Fallback for nodes not in the pre-render map
            let label = node.label.as_deref().unwrap_or(&node.id);
            let cell_w = scale.to_cell_width(node.width);
            let cell_h = scale.to_cell_height(node.height);
            render_shape(&node.shape, label, cell_w, cell_h)
        };

        // node.x/y are top-left coordinates (dagre center coords converted
        // by apply_results_recursive), so use them directly as cell start.
        let col_start = scale.to_col(nx);
        let row_start = scale.to_row(ny);

        for (r, line) in rendered.lines.iter().enumerate() {
            let canvas_row = row_start + r;
            if canvas_row >= canvas_rows {
                break;
            }
            for (c, ch) in line.chars().enumerate() {
                let canvas_col = col_start + c;
                if canvas_col >= canvas_cols {
                    break;
                }
                if ch != ' ' {
                    canvas[canvas_row][canvas_col] = ch;
                    occupied[canvas_row][canvas_col] = true;
                }
            }
        }
    }

    // Render edges (braille lines + arrows + labels)
    edges::render_edges(
        &graph,
        &scale,
        canvas_cols,
        canvas_rows,
        offset_x,
        offset_y,
        &occupied,
        &mut canvas,
    );

    // Convert canvas to string, trimming trailing empty lines
    let mut result = String::new();
    let mut last_non_empty = 0;
    for (i, row) in canvas.iter().enumerate() {
        if row.iter().any(|&c| c != ' ') {
            last_non_empty = i;
        }
    }

    for row in &canvas[..=last_non_empty] {
        let line: String = row.iter().collect();
        result.push_str(line.trim_end());
        result.push('\n');
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::{CharacterSizeEstimator, ToLayoutGraph};

    fn parse_and_layout(input: &str) -> (FlowchartDb, LayoutGraph) {
        let diagram = crate::parse(input).unwrap();
        let db = match diagram {
            crate::diagrams::Diagram::Flowchart(db) => db,
            _ => panic!("Expected flowchart"),
        };
        let estimator = CharacterSizeEstimator::default();
        let graph = db.to_layout_graph(&estimator).unwrap();
        let graph = crate::layout::layout(graph).unwrap();
        (db, graph)
    }

    #[test]
    fn complex_flowchart_has_all_labels() {
        let (db, graph) = parse_and_layout(
            &std::fs::read_to_string("docs/sources/flowchart_complex.mmd").unwrap(),
        );
        let output = render_flowchart_ascii(&db, &graph).unwrap();

        // Check that key labels appear in the output
        for label in &[
            "CLI Tool",
            "Mobile App",
            "Web Interface",
            "Authentication",
            "Rate Limiter",
            "Redis Cache",
            "PostgreSQL",
            "Elasticsearch",
            "Frontend Layer",
            "API Gateway",
        ] {
            assert!(
                output.contains(label),
                "Output should contain '{}'\nOutput:\n{}",
                label,
                output
            );
        }
    }

    #[test]
    fn arrow_tip_not_inside_node() {
        let (db, graph) = parse_and_layout("flowchart TD\n    A[Start] --> B[End]");
        let output = render_flowchart_ascii(&db, &graph).unwrap();
        // Arrow tips must not appear inside node labels
        // "End" should appear as-is, not "E▼d" or similar
        assert!(
            output.contains("End"),
            "Node label 'End' must not be corrupted by arrow tips\nOutput:\n{}",
            output
        );
        // Also check Start
        assert!(
            output.contains("Start"),
            "Node label 'Start' must not be corrupted\nOutput:\n{}",
            output
        );
    }

    #[test]
    fn subgraph_does_not_overlap_children() {
        let (db, graph) = parse_and_layout(
            "flowchart TD\n    subgraph sg[My Group]\n        A[NodeA]\n        B[NodeB]\n    end",
        );
        let output = render_flowchart_ascii(&db, &graph).unwrap();
        // All node labels must be present and intact
        assert!(
            output.contains("NodeA"),
            "NodeA must be visible\nOutput:\n{}",
            output
        );
        assert!(
            output.contains("NodeB"),
            "NodeB must be visible\nOutput:\n{}",
            output
        );
        assert!(
            output.contains("My Group"),
            "Subgraph label must be visible\nOutput:\n{}",
            output
        );
    }

    #[test]
    fn diamond_does_not_corrupt_adjacent_nodes() {
        let (db, graph) = parse_and_layout(
            "flowchart TD\n    A[Start] --> B{Decision}\n    B --> C[Action 1]\n    B --> D[End]",
        );
        let output = render_flowchart_ascii(&db, &graph).unwrap();
        assert!(
            output.contains("Decision"),
            "Diamond label must be readable\nOutput:\n{}",
            output
        );
        assert!(
            output.contains("Start"),
            "Start must not be corrupted by adjacent diamond\nOutput:\n{}",
            output
        );
        assert!(
            output.contains("Action 1"),
            "Action 1 label must be intact\nOutput:\n{}",
            output
        );
    }

    #[test]
    fn cyrillic_label_renders() {
        let (db, graph) =
            parse_and_layout("graph TB\n    cyr[Cyrillic]-->cyr2((Circle shape Начало))");
        let output = render_flowchart_ascii(&db, &graph).unwrap();
        assert!(
            output.contains("Начало"),
            "Cyrillic label must be visible\nOutput:\n{}",
            output
        );
    }

    #[test]
    #[ignore = "Circle node label corrupted by overlapping box borders (dagre layout quantization)"]
    fn styled_flowchart_has_cyrillic() {
        let input = r#"graph TB
    sq[Square shape] --> ci((Circle shape))

    subgraph A
        od>Odd shape]-- Two line<br/>edge comment --> ro
        di{Diamond with <br/> line break} -.-> ro(Rounded<br>square<br>shape)
        di==>ro2(Rounded square shape)
    end

    e --> od3>Really long text with linebreak<br>in an Odd shape]

    e((Inner / circle<br>and some odd <br>special characters)) --> f(,.?!+-*ز)

    cyr[Cyrillic]-->cyr2((Circle shape Начало))

     classDef green fill:#9f6,stroke:#333,stroke-width:2px
     classDef orange fill:#f96,stroke:#333,stroke-width:4px
     class sq,e green
     class di orange"#;
        let (db, graph) = parse_and_layout(input);
        let output = render_flowchart_ascii(&db, &graph).unwrap();
        assert!(
            output.contains("Circle shape Начало"),
            "Cyrillic circle label must be visible\nOutput:\n{}",
            output
        );
    }

    #[test]
    fn single_node_renders() {
        let (db, graph) = parse_and_layout("flowchart TD\n    A[Hello]");
        let output = render_flowchart_ascii(&db, &graph).unwrap();
        assert!(output.contains("Hello"), "Output should contain the label");
        assert!(
            output.contains('┌') || output.contains('╭'),
            "Output should contain box-drawing chars"
        );
    }

    #[test]
    fn two_nodes_render() {
        let (db, graph) = parse_and_layout("flowchart TD\n    A[Start] --> B[End]");
        let output = render_flowchart_ascii(&db, &graph).unwrap();
        assert!(output.contains("Start"), "Should contain Start label");
        assert!(output.contains("End"), "Should contain End label");
    }

    #[test]
    fn round_node_uses_rounded_corners() {
        let (db, graph) = parse_and_layout("flowchart TD\n    A(Round)");
        let output = render_flowchart_ascii(&db, &graph).unwrap();
        assert!(output.contains('╭'), "Round node should use ╭");
        assert!(output.contains('╯'), "Round node should use ╯");
    }

    #[test]
    fn diamond_node_renders() {
        let (db, graph) = parse_and_layout("flowchart TD\n    A{Decision}");
        let output = render_flowchart_ascii(&db, &graph).unwrap();
        assert!(output.contains("Decision"), "Diamond should contain label");
    }

    #[test]
    fn output_is_nonempty() {
        let (db, graph) = parse_and_layout("flowchart TD\n    A[X]");
        let output = render_flowchart_ascii(&db, &graph).unwrap();
        assert!(!output.trim().is_empty(), "Output should not be empty");
    }

    #[test]
    fn edges_produce_braille_chars() {
        let (db, graph) = parse_and_layout("flowchart TD\n    A[Start] --> B[End]");
        let output = render_flowchart_ascii(&db, &graph).unwrap();
        // Edge should produce at least some braille characters or arrow tips
        let has_braille = output
            .chars()
            .any(|c| ('\u{2800}'..='\u{28FF}').contains(&c));
        let has_arrow = output.contains('▼') || output.contains('▶');
        assert!(
            has_braille || has_arrow,
            "Edge should produce braille dots or arrows"
        );
    }

    #[test]
    fn edge_labels_render() {
        let (db, graph) = parse_and_layout("flowchart TD\n    A[Start] -->|Yes| B[End]");
        let output = render_flowchart_ascii(&db, &graph).unwrap();
        assert!(output.contains("Yes"), "Edge label 'Yes' should appear");
    }

    #[test]
    fn down_arrow_in_td_flow() {
        let (db, graph) = parse_and_layout("flowchart TD\n    A[Top] --> B[Bottom]");
        let output = render_flowchart_ascii(&db, &graph).unwrap();
        assert!(output.contains('▼'), "TD flow should have down arrow ▼");
    }

    // --- Generic renderer tests for non-flowchart diagram types ---

    /// Parse any diagram type and produce a layout graph for ASCII rendering.
    fn parse_and_layout_generic(input: &str) -> crate::layout::LayoutGraph {
        let diagram = crate::parse(input).unwrap();
        let estimator = CharacterSizeEstimator::default();
        let graph = match diagram {
            crate::diagrams::Diagram::State(ref db) => db.to_layout_graph(&estimator).unwrap(),
            crate::diagrams::Diagram::Class(ref db) => db.to_layout_graph(&estimator).unwrap(),
            crate::diagrams::Diagram::Er(ref db) => db.to_layout_graph(&estimator).unwrap(),
            crate::diagrams::Diagram::Architecture(ref db) => {
                db.to_layout_graph(&estimator).unwrap()
            }
            crate::diagrams::Diagram::Requirement(ref db) => {
                db.to_layout_graph(&estimator).unwrap()
            }
            _ => panic!("Unsupported diagram type for generic ASCII test"),
        };
        crate::layout::layout(graph).unwrap()
    }

    #[test]
    fn state_diagram_renders_ascii() {
        let input = "stateDiagram-v2\n    [*] --> Idle\n    Idle --> Running : start\n    Running --> Idle : stop";
        let graph = parse_and_layout_generic(input);
        let output = render_graph_ascii(&graph).unwrap();
        assert!(
            !output.trim().is_empty(),
            "State diagram ASCII output should not be empty"
        );
        assert!(
            output.contains("Idle"),
            "State diagram should contain 'Idle' label\nOutput:\n{}",
            output
        );
        assert!(
            output.contains("Running"),
            "State diagram should contain 'Running' label\nOutput:\n{}",
            output
        );
    }

    #[test]
    fn class_diagram_renders_ascii() {
        let input =
            "classDiagram\n    Animal <|-- Duck\n    Animal <|-- Fish\n    Animal : +int age";
        let graph = parse_and_layout_generic(input);
        let output = render_graph_ascii(&graph).unwrap();
        assert!(
            !output.trim().is_empty(),
            "Class diagram ASCII output should not be empty"
        );
        assert!(
            output.contains("Animal"),
            "Class diagram should contain 'Animal' label\nOutput:\n{}",
            output
        );
        assert!(
            output.contains("Duck"),
            "Class diagram should contain 'Duck' label\nOutput:\n{}",
            output
        );
    }

    #[test]
    fn er_diagram_renders_ascii() {
        let input =
            "erDiagram\n    CUSTOMER ||--o{ ORDER : places\n    ORDER ||--|{ LINE-ITEM : contains";
        let graph = parse_and_layout_generic(input);
        let output = render_graph_ascii(&graph).unwrap();
        assert!(
            !output.trim().is_empty(),
            "ER diagram ASCII output should not be empty"
        );
        assert!(
            output.contains("CUSTOMER"),
            "ER diagram should contain 'CUSTOMER' label\nOutput:\n{}",
            output
        );
        assert!(
            output.contains("ORDER"),
            "ER diagram should contain 'ORDER' label\nOutput:\n{}",
            output
        );
    }

    #[test]
    fn state_diagram_from_file() {
        let input = std::fs::read_to_string("docs/sources/state.mmd").unwrap();
        let graph = parse_and_layout_generic(&input);
        let output = render_graph_ascii(&graph).unwrap();
        for label in &["Idle", "Running", "Error"] {
            assert!(
                output.contains(label),
                "State diagram should contain '{}'\nOutput:\n{}",
                label,
                output
            );
        }
    }

    #[test]
    fn class_diagram_from_file() {
        let input = std::fs::read_to_string("docs/sources/class.mmd").unwrap();
        let graph = parse_and_layout_generic(&input);
        let output = render_graph_ascii(&graph).unwrap();
        for label in &["Animal", "Duck", "Fish", "Zebra"] {
            assert!(
                output.contains(label),
                "Class diagram should contain '{}'\nOutput:\n{}",
                label,
                output
            );
        }
    }

    #[test]
    fn er_diagram_from_file() {
        let input = std::fs::read_to_string("docs/sources/er.mmd").unwrap();
        let graph = parse_and_layout_generic(&input);
        let output = render_graph_ascii(&graph).unwrap();
        for label in &["CUSTOMER", "ORDER", "PRODUCT"] {
            assert!(
                output.contains(label),
                "ER diagram should contain '{}'\nOutput:\n{}",
                label,
                output
            );
        }
    }

    #[test]
    fn state_diagram_start_end_rendered_as_symbols() {
        let input = "stateDiagram-v2\n    [*] --> Idle\n    Idle --> [*]";
        let graph = parse_and_layout_generic(input);
        let output = render_graph_ascii(&graph).unwrap();
        // Start/end nodes should render as ● or ◉ symbols, not rectangles with ID labels
        let has_start = output.contains('●');
        let has_end = output.contains('◉');
        assert!(
            has_start,
            "Start node should render as ● (filled circle)\nOutput:\n{}",
            output
        );
        assert!(
            has_end,
            "End node should render as ◉ (bullseye)\nOutput:\n{}",
            output
        );
    }

    #[test]
    fn state_complex_fork_renders_as_bar() {
        let input = std::fs::read_to_string("docs/sources/state_complex.mmd").unwrap();
        let graph = parse_and_layout_generic(&input);
        let output = render_graph_ascii(&graph).unwrap();
        // Fork/join should render as solid bars (█), not as labeled boxes
        assert!(
            output.contains('█'),
            "Fork/join states should render as solid bars\nOutput:\n{}",
            output
        );
        // Should NOT show the internal IDs as labels
        assert!(
            !output.contains("fork_state"),
            "Fork state ID should not appear as label\nOutput:\n{}",
            output
        );
        assert!(
            !output.contains("join_state"),
            "Join state ID should not appear as label\nOutput:\n{}",
            output
        );
    }

    #[test]
    fn state_complex_has_all_expected_labels() {
        let input = std::fs::read_to_string("docs/sources/state_complex.mmd").unwrap();
        let graph = parse_and_layout_generic(&input);
        let output = render_graph_ascii(&graph).unwrap();
        for label in &[
            "Idle",
            "Ready",
            "Validation",
            "ResourceAlloc",
            "Processing",
            "Validating",
            "Executing",
            "Init",
            "Done",
        ] {
            assert!(
                output.contains(label),
                "State diagram should contain '{}'\nOutput:\n{}",
                label,
                output
            );
        }
    }

    #[test]
    fn state_complex2_node_labels_in_boxes() {
        let input = std::fs::read_to_string("docs/sources/state_complex2.mmd").unwrap();
        let graph = parse_and_layout_generic(&input);
        let output = render_graph_ascii(&graph).unwrap();
        // Key nested states should appear
        for label in &["Initializing", "Finalizing", "WaitingResume"] {
            assert!(
                output.contains(label),
                "State diagram should contain nested state '{}'\nOutput:\n{}",
                label,
                output
            );
        }
    }

    #[test]
    fn state_complex2_has_all_expected_labels() {
        let input = std::fs::read_to_string("docs/sources/state_complex2.mmd").unwrap();
        let graph = parse_and_layout_generic(&input);
        let output = render_graph_ascii(&graph).unwrap();
        // Core state labels must appear
        for label in &[
            "Idle",
            "Ready",
            "Validating",
            "Queued",
            "Running",
            "Completed",
            "Failed",
            "Paused",
            "Cancelled",
            "Timeout",
            "WaitingResume",
        ] {
            assert!(
                output.contains(label),
                "State diagram should contain '{}'\nOutput:\n{}",
                label,
                output
            );
        }
        // Start/end symbols should be present (not ID text like "Idle_start")
        assert!(
            output.contains('●') || output.contains('◉'),
            "Start/end nodes should use circle symbols\nOutput:\n{}",
            output
        );
    }

    #[test]
    fn state_diagram_has_edges() {
        let input = "stateDiagram-v2\n    [*] --> Idle\n    Idle --> Running : start";
        let graph = parse_and_layout_generic(input);
        let output = render_graph_ascii(&graph).unwrap();
        let has_braille = output
            .chars()
            .any(|c| ('\u{2800}'..='\u{28FF}').contains(&c));
        let has_arrow = output.contains('▼')
            || output.contains('▶')
            || output.contains('◀')
            || output.contains('▲');
        assert!(
            has_braille || has_arrow,
            "State diagram should have edges rendered\nOutput:\n{}",
            output
        );
    }

    // --- ER diagram ASCII renderer tests ---

    fn parse_er_and_layout(input: &str) -> (crate::diagrams::er::ErDb, crate::layout::LayoutGraph) {
        let diagram = crate::parse(input).unwrap();
        let db = match diagram {
            crate::diagrams::Diagram::Er(db) => db,
            _ => panic!("Expected ER diagram"),
        };
        let estimator = CharacterSizeEstimator::default();
        let graph = db.to_layout_graph(&estimator).unwrap();
        let graph = crate::layout::layout(graph).unwrap();
        (db, graph)
    }

    #[test]
    fn er_ascii_renders_entity_names() {
        let input = "erDiagram\n    CUSTOMER ||--o{ ORDER : places";
        let (db, graph) = parse_er_and_layout(input);
        let output = render_er_ascii(&db, &graph).unwrap();
        assert!(
            output.contains("CUSTOMER"),
            "ER ASCII should contain 'CUSTOMER'\nOutput:\n{}",
            output
        );
        assert!(
            output.contains("ORDER"),
            "ER ASCII should contain 'ORDER'\nOutput:\n{}",
            output
        );
    }

    #[test]
    fn er_ascii_renders_attributes() {
        let input = r#"erDiagram
    CUSTOMER {
        string name
        string email PK
        int id
    }
"#;
        let (db, graph) = parse_er_and_layout(input);
        let output = render_er_ascii(&db, &graph).unwrap();
        // Entity name
        assert!(
            output.contains("CUSTOMER"),
            "Should contain entity name\nOutput:\n{}",
            output
        );
        // Attribute types
        assert!(
            output.contains("string"),
            "Should contain attribute type 'string'\nOutput:\n{}",
            output
        );
        assert!(
            output.contains("int"),
            "Should contain attribute type 'int'\nOutput:\n{}",
            output
        );
        // Attribute names
        assert!(
            output.contains("name"),
            "Should contain attribute name 'name'\nOutput:\n{}",
            output
        );
        assert!(
            output.contains("email"),
            "Should contain attribute name 'email'\nOutput:\n{}",
            output
        );
        // Key markers
        assert!(
            output.contains("PK"),
            "Should contain key marker 'PK'\nOutput:\n{}",
            output
        );
    }

    #[test]
    fn er_ascii_has_table_dividers() {
        let input = r#"erDiagram
    CUSTOMER {
        string name
        string email PK
    }
"#;
        let (db, graph) = parse_er_and_layout(input);
        let output = render_er_ascii(&db, &graph).unwrap();
        // Should have table structure with ├ ┤ ┬ ┴ dividers
        assert!(
            output.contains('├'),
            "Should have ├ for header divider\nOutput:\n{}",
            output
        );
        assert!(
            output.contains('┬'),
            "Should have ┬ for column separators\nOutput:\n{}",
            output
        );
        assert!(
            output.contains('┴'),
            "Should have ┴ for bottom column separators\nOutput:\n{}",
            output
        );
    }

    #[test]
    fn er_ascii_entity_without_attributes() {
        let input = "erDiagram\n    CUSTOMER ||--o{ ORDER : places";
        let (db, graph) = parse_er_and_layout(input);
        let output = render_er_ascii(&db, &graph).unwrap();
        // Entities without attributes should be simple boxes
        assert!(
            output.contains('┌'),
            "Should have box corners\nOutput:\n{}",
            output
        );
        assert!(
            output.contains('┘'),
            "Should have box corners\nOutput:\n{}",
            output
        );
    }

    #[test]
    fn er_ascii_from_file_has_all_entities() {
        let input = std::fs::read_to_string("docs/sources/er.mmd").unwrap();
        let (db, graph) = parse_er_and_layout(&input);
        let output = render_er_ascii(&db, &graph).unwrap();
        for label in &["CUSTOMER", "ORDER", "PRODUCT", "LINE-ITEM"] {
            assert!(
                output.contains(label),
                "ER ASCII should contain '{}'\nOutput:\n{}",
                label,
                output
            );
        }
        // Should have attributes rendered
        assert!(
            output.contains("string"),
            "Should have attribute types\nOutput:\n{}",
            output
        );
    }

    #[test]
    fn er_ascii_has_edges() {
        let input = "erDiagram\n    CUSTOMER ||--o{ ORDER : places";
        let (db, graph) = parse_er_and_layout(input);
        let output = render_er_ascii(&db, &graph).unwrap();
        let has_braille = output
            .chars()
            .any(|c| ('\u{2800}'..='\u{28FF}').contains(&c));
        let has_arrow = output.contains('▼')
            || output.contains('▶')
            || output.contains('◀')
            || output.contains('▲');
        assert!(
            has_braille || has_arrow,
            "ER diagram should have edges rendered\nOutput:\n{}",
            output
        );
    }

    #[test]
    fn er_ascii_complex_from_file() {
        let input = std::fs::read_to_string("docs/sources/er_complex.mmd").unwrap();
        let (db, graph) = parse_er_and_layout(&input);
        let output = render_er_ascii(&db, &graph).unwrap();
        for label in &["CUSTOMER", "ORDER", "PRODUCT", "CATEGORY", "PAYMENT"] {
            assert!(
                output.contains(label),
                "Complex ER ASCII should contain '{}'\nOutput:\n{}",
                label,
                output
            );
        }
    }

    #[test]
    fn er_ascii_relationship_labels_not_truncated() {
        let input = r#"erDiagram
    CUSTOMER ||--o{ ORDER : places
    ORDER ||--|{ LINE-ITEM : contains
    PRODUCT ||--o{ LINE-ITEM : includes
    CUSTOMER {
        string name
        string email PK
        string address
    }
    ORDER {
        int orderNumber PK
        date orderDate
        string status
    }
    PRODUCT {
        int id PK
        string name
        float price
    }
"#;
        let (db, graph) = parse_er_and_layout(input);
        let output = render_er_ascii(&db, &graph).unwrap();
        // Relationship labels must appear in full, not truncated
        assert!(
            output.contains("places"),
            "Relationship label 'places' should appear in full\nOutput:\n{}",
            output
        );
        assert!(
            output.contains("contains"),
            "Relationship label 'contains' should appear in full (not truncated to 'conta')\nOutput:\n{}",
            output
        );
        assert!(
            output.contains("includes"),
            "Relationship label 'includes' should appear in full\nOutput:\n{}",
            output
        );
    }

    #[test]
    fn er_ascii_complex_relationship_labels_not_truncated() {
        let input = std::fs::read_to_string("docs/sources/er_complex.mmd").unwrap();
        let (db, graph) = parse_er_and_layout(&input);
        let output = render_er_ascii(&db, &graph).unwrap();
        for label in &[
            "places",
            "contains",
            "references",
            "belongs_to",
            "has",
            "ships_to",
            "paid_by",
        ] {
            assert!(
                output.contains(label),
                "Relationship label '{}' should appear in full\nOutput:\n{}",
                label,
                output
            );
        }
    }

    #[test]
    fn er_ascii_complex_compact_width() {
        // The complex ER diagram should have a compact width.
        // Previously, the "has" edge routing created long vertical runs far
        // to the right, making the diagram unnecessarily wide.
        let input = std::fs::read_to_string("docs/sources/er_complex.mmd").unwrap();
        let (db, graph) = parse_er_and_layout(&input);
        let output = render_er_ascii(&db, &graph).unwrap();

        let max_line_width = output.lines().map(|l| l.chars().count()).max().unwrap_or(0);

        // The diagram has 8 entities. With reasonable layout, width should
        // stay under 120 chars (roughly two side-by-side entity boxes + edges).
        assert!(
            max_line_width <= 120,
            "ER complex diagram is {} columns wide, expected <= 120. \
             This suggests edges are routed too far from entities.\nOutput:\n{}",
            max_line_width,
            output
        );
    }

    // --- Class diagram specialized ASCII tests ---

    fn parse_and_layout_class(input: &str) -> (ClassDb, LayoutGraph) {
        let diagram = crate::parse(input).unwrap();
        let db = match diagram {
            crate::diagrams::Diagram::Class(db) => db,
            other => panic!(
                "Expected class diagram, got {:?}",
                std::mem::discriminant(&other)
            ),
        };
        let estimator = CharacterSizeEstimator::default();
        let graph = db.to_layout_graph(&estimator).unwrap();
        let graph = crate::layout::layout(graph).unwrap();
        (db, graph)
    }

    #[test]
    fn class_single_renders() {
        let input =
            "classDiagram\n    class Animal {\n        +int age\n        +isMammal()\n    }";
        let (db, graph) = parse_and_layout_class(input);
        let output = render_class_ascii(&db, &graph).unwrap();
        assert!(
            output.contains("Animal"),
            "Should contain class name 'Animal'"
        );
        assert!(output.contains('┌'), "Should have box-drawing chars");
        assert!(output.contains('├'), "Should have section dividers");
    }

    #[test]
    fn class_two_with_relation_renders() {
        let input = "classDiagram\n    Animal <|-- Duck\n    Animal : +int age\n    Duck : +swim()";
        let (db, graph) = parse_and_layout_class(input);
        let output = render_class_ascii(&db, &graph).unwrap();
        assert!(output.contains("Animal"), "Should contain 'Animal'");
        assert!(output.contains("Duck"), "Should contain 'Duck'");
    }

    #[test]
    fn class_output_is_nonempty() {
        let input = "classDiagram\n    class Foo";
        let (db, graph) = parse_and_layout_class(input);
        let output = render_class_ascii(&db, &graph).unwrap();
        assert!(!output.trim().is_empty(), "Output should not be empty");
    }

    #[cfg(feature = "eval")]
    #[test]
    fn class_ascii_labels_detected_by_eval() {
        let input = "classDiagram\n    Animal <|-- Duck\n    Animal <|-- Fish\n    Animal <|-- Zebra\n    Animal : +int age\n    Animal : +String gender\n    Animal: +isMammal()\n    Animal: +mate()\n    class Duck{\n        +String beakColor\n        +swim()\n        +quack()\n    }";
        let (db, graph) = parse_and_layout_class(input);
        let output = render_class_ascii(&db, &graph).unwrap();

        let ascii_out = crate::eval::ascii_checks::parse_ascii(&output);

        // All class names should be found
        for name in ["Animal", "Duck", "Fish", "Zebra"] {
            assert!(
                ascii_out.labels.iter().any(|l: &String| l.contains(name)),
                "Should find class '{}' in labels {:?}",
                name,
                ascii_out.labels
            );
        }
    }

    /// Check that a horizontal box-drawing span (┌→┐, ├→┤, └→┘) is properly
    /// closed before another box's border character appears.
    fn assert_no_box_overlap(output: &str) {
        let left_chars = ['┌', '├', '└'];
        let right_chars = ['┐', '┤', '┘'];
        let lines: Vec<&str> = output.lines().collect();
        for (row_idx, line) in lines.iter().enumerate() {
            let chars: Vec<char> = line.chars().collect();
            // Find all left-border positions (starts of box horizontal spans)
            let lefts: Vec<(usize, char)> = chars
                .iter()
                .enumerate()
                .filter(|(_, &c)| left_chars.contains(&c))
                .map(|(i, &c)| (i, c))
                .collect();
            for &(col, lc) in &lefts {
                // Expected matching right border
                let expected_right = match lc {
                    '┌' => '┐',
                    '├' => '┤',
                    '└' => '┘',
                    _ => unreachable!(),
                };
                let rest = &chars[col + 1..];
                let mut found_match = false;
                for (j, &c) in rest.iter().enumerate() {
                    if c == expected_right {
                        found_match = true;
                        break;
                    }
                    // Any other box border character before the expected match = overlap
                    if left_chars.contains(&c) || right_chars.contains(&c) {
                        panic!(
                            "Box overlap at row {}: '{}' at col {} found '{}' at col {} before matching '{}'\nLine: {}",
                            row_idx,
                            lc,
                            col,
                            c,
                            col + 1 + j,
                            expected_right,
                            line
                        );
                    }
                }
                assert!(
                    found_match,
                    "Unclosed box border '{}' at row {}, col {}\nLine: {}",
                    lc, row_idx, col, line
                );
            }
        }
    }

    /// Verify that class boxes do not overlap each other in the ASCII output.
    #[test]
    fn class_boxes_do_not_overlap() {
        let input = std::fs::read_to_string("docs/sources/class.mmd").unwrap();
        let (db, graph) = parse_and_layout_class(&input);
        let output = render_class_ascii(&db, &graph).unwrap();
        assert_no_box_overlap(&output);
    }

    /// Verify that class_complex.mmd renders without box overlap.
    #[test]
    fn class_complex_boxes_do_not_overlap() {
        let input = std::fs::read_to_string("docs/sources/class_complex.mmd").unwrap();
        let (db, graph) = parse_and_layout_class(&input);
        let output = render_class_ascii(&db, &graph).unwrap();
        assert_no_box_overlap(&output);
    }

    /// Verify that render_ascii_impl centers shapes on the node's true center,
    /// computed from top-left coordinates (not treating top-left as center).
    ///
    /// apply_results_recursive converts dagre center coords to top-left. The old
    /// code used `to_col(nx).saturating_sub(width/2)` which treats top-left as
    /// center. The fix computes true center: `to_col(nx + node.width/2)`.
    #[test]
    fn render_ascii_impl_uses_top_left_coords() {
        let (db, graph) = parse_and_layout("flowchart TD\n    A[Hello] --> B[World]");
        let output = render_flowchart_ascii(&db, &graph).unwrap();
        let scale = CellScale::default();
        let offset_y = graph.bounds_y.unwrap_or(0.0);

        // Node B is positioned below A. Its center should be at ny + height/2.
        let node_b = graph.nodes.iter().find(|n| n.id == "B").unwrap();
        let ny_b = node_b.y.unwrap() - offset_y;

        // Correct center row (computed from top-left)
        let center_row = scale.to_row(ny_b + node_b.height / 2.0);
        // Buggy center row (treats top-left as center)
        let buggy_center_row = scale.to_row(ny_b);

        // The label "World" appears at the center row of the rendered shape
        let lines: Vec<&str> = output.lines().collect();
        let world_row = lines
            .iter()
            .position(|line| line.contains("World"))
            .expect("Should contain World");

        // Label should be at the true center row, not the buggy one
        assert_eq!(
            world_row, center_row,
            "World at row {} should be at center row {} (not buggy row {})\nOutput:\n{}",
            world_row, center_row, buggy_center_row, output
        );
    }

    /// Verify that render_er_ascii positions entities at their top-left coordinates.
    ///
    /// For the ORDER entity (second in vertical layout), the correct row is
    /// to_row(ny)=10 but the buggy saturating_sub gives row=8 — a 2-row shift.
    #[test]
    fn render_er_ascii_uses_top_left_coords() {
        let input =
            "erDiagram\n    CUSTOMER ||--o{ ORDER : places\n    ORDER ||--|{ LINE-ITEM : contains";
        let (db, graph) = parse_er_and_layout(input);
        let output = render_er_ascii(&db, &graph).unwrap();
        let scale = CellScale::default();

        let offset_x = graph.bounds_x.unwrap_or(0.0);
        let offset_y = graph.bounds_y.unwrap_or(0.0);

        // Build entity name lookup
        let entities = db.get_entities();
        let id_to_name: HashMap<&str, &str> = entities
            .iter()
            .map(|(name, entity)| (entity.id.as_str(), name.as_str()))
            .collect();

        // Find ORDER node — it's positioned in the middle row, not at y=0
        let order_node = graph
            .nodes
            .iter()
            .find(|n| id_to_name.get(n.id.as_str()) == Some(&"ORDER"))
            .expect("Should find ORDER node");

        let nx = order_node.x.unwrap() - offset_x;
        let ny = order_node.y.unwrap() - offset_y;
        let expected_col = scale.to_col(nx);
        let expected_row = scale.to_row(ny);
        let cell_w = scale.to_cell_width(order_node.width);
        let cell_h = scale.to_cell_height(order_node.height);
        let buggy_col = expected_col.saturating_sub(cell_w / 2);
        let buggy_row = expected_row.saturating_sub(cell_h / 2);

        // Find where ORDER appears in the output
        let lines: Vec<&str> = output.lines().collect();
        let order_row = lines
            .iter()
            .position(|line| line.contains("ORDER") && !line.contains("CUSTOMER"))
            .expect("Should contain ORDER label");

        // ORDER label appears 1 row below the box top border (inside the header).
        // Correct: row = expected_row + 1 = 11
        // Buggy:   row = buggy_row + 1 = 9
        assert_eq!(
            order_row,
            expected_row + 1,
            "ORDER label at row {} should be at row {} (top-left {} + 1)\n\
             With the bug it would be at row {} (buggy {} + 1)\n\
             expected_col={} buggy_col={}\nOutput:\n{}",
            order_row,
            expected_row + 1,
            expected_row,
            buggy_row + 1,
            buggy_row,
            expected_col,
            buggy_col,
            output
        );
    }

    #[test]
    fn class_edges_produce_visual() {
        let input = "classDiagram\n    Animal <|-- Duck\n    Animal <|-- Fish";
        let (db, graph) = parse_and_layout_class(input);
        let output = render_class_ascii(&db, &graph).unwrap();
        let has_braille = output
            .chars()
            .any(|c| ('\u{2800}'..='\u{28FF}').contains(&c));
        let has_arrow = output.contains('▼')
            || output.contains('▶')
            || output.contains('▲')
            || output.contains('◀');
        assert!(
            has_braille || has_arrow,
            "Edges should produce braille dots or arrows"
        );
    }

    // --- Requirement diagram ASCII tests ---

    fn parse_and_layout_requirement(input: &str) -> crate::layout::LayoutGraph {
        let diagram = crate::parse(input).unwrap();
        let db = match &diagram {
            crate::diagrams::Diagram::Requirement(db) => db,
            _ => panic!("Expected requirement diagram"),
        };
        let estimator = CharacterSizeEstimator::default();
        let graph = db.to_layout_graph(&estimator).unwrap();
        crate::layout::layout(graph).unwrap()
    }

    #[test]
    fn requirement_ascii_boxes_dont_overlap() {
        let input = std::fs::read_to_string("docs/sources/requirement.mmd").unwrap();
        let graph = parse_and_layout_requirement(&input);
        let output = render_graph_ascii(&graph).unwrap();

        // No line should have a box corner character immediately starting another box
        // This pattern indicates overlapping boxes
        for (i, line) in output.lines().enumerate() {
            // Check for overlapping top-left corners: ┌───...┌
            let chars: Vec<char> = line.chars().collect();
            let mut corner_positions: Vec<usize> = Vec::new();
            for (j, &ch) in chars.iter().enumerate() {
                if ch == '┌' || ch == '└' {
                    corner_positions.push(j);
                }
            }
            // Two box-drawing corners should not be within a line without a closing corner between them
            if corner_positions.len() >= 2 {
                for window in corner_positions.windows(2) {
                    let between = &chars[window[0]..=window[1]];
                    let has_closing = between.iter().any(|&c| c == '┐' || c == '┘');
                    assert!(
                        has_closing,
                        "Boxes overlap on line {}: two corners at positions {} and {} without closing corner between\nLine: {}\nFull output:\n{}",
                        i, window[0], window[1], line, output
                    );
                }
            }
        }
    }

    #[test]
    fn requirement_ascii_edge_labels_not_truncated() {
        let input = std::fs::read_to_string("docs/sources/requirement.mmd").unwrap();
        let graph = parse_and_layout_requirement(&input);
        let output = render_graph_ascii(&graph).unwrap();

        // All relationship labels should appear in full
        assert!(
            output.contains("<<contains>>"),
            "Edge label '<<contains>>' should not be truncated\nOutput:\n{}",
            output
        );
        assert!(
            output.contains("<<verifies>>"),
            "Edge label '<<verifies>>' should not be truncated\nOutput:\n{}",
            output
        );
        assert!(
            output.contains("<<satisfies>>"),
            "Edge label '<<satisfies>>' should not be truncated\nOutput:\n{}",
            output
        );
    }

    #[test]
    fn requirement_complex_ascii_boxes_dont_overlap() {
        let input = std::fs::read_to_string("docs/sources/requirement_complex.mmd").unwrap();
        let graph = parse_and_layout_requirement(&input);
        let output = render_graph_ascii(&graph).unwrap();

        // No line should have overlapping box corners
        for (i, line) in output.lines().enumerate() {
            let chars: Vec<char> = line.chars().collect();
            let mut corner_positions: Vec<usize> = Vec::new();
            for (j, &ch) in chars.iter().enumerate() {
                if ch == '┌' || ch == '└' {
                    corner_positions.push(j);
                }
            }
            if corner_positions.len() >= 2 {
                for window in corner_positions.windows(2) {
                    let between = &chars[window[0]..=window[1]];
                    let has_closing = between.iter().any(|&c| c == '┐' || c == '┘');
                    assert!(
                        has_closing,
                        "Boxes overlap on line {}: corners at {} and {} without closing between\nLine: {}\nFull output:\n{}",
                        i, window[0], window[1], line, output
                    );
                }
            }
        }
    }

    #[test]
    fn requirement_complex_ascii_edge_labels_not_truncated() {
        let input = std::fs::read_to_string("docs/sources/requirement_complex.mmd").unwrap();
        let graph = parse_and_layout_requirement(&input);
        let output = render_graph_ascii(&graph).unwrap();

        // Key relationship labels should appear in full
        assert!(
            output.contains("<<contains>>"),
            "Edge label '<<contains>>' should not be truncated\nOutput:\n{}",
            output
        );
        assert!(
            output.contains("<<verifies>>"),
            "Edge label '<<verifies>>' should not be truncated\nOutput:\n{}",
            output
        );
    }

    // --- ER diagram ASCII overlap regression tests ---

    #[test]
    fn er_ascii_boxes_dont_overlap() {
        let input = std::fs::read_to_string("docs/sources/er.mmd").unwrap();
        let (db, graph) = parse_er_and_layout(&input);
        let output = render_er_ascii(&db, &graph).unwrap();

        // No line should have overlapping box corners (same check as requirement tests)
        for (i, line) in output.lines().enumerate() {
            let chars: Vec<char> = line.chars().collect();
            let mut corner_positions: Vec<usize> = Vec::new();
            for (j, &ch) in chars.iter().enumerate() {
                if ch == '┌' || ch == '└' {
                    corner_positions.push(j);
                }
            }
            if corner_positions.len() >= 2 {
                for window in corner_positions.windows(2) {
                    let between = &chars[window[0]..=window[1]];
                    let has_closing = between
                        .iter()
                        .any(|&c| c == '┐' || c == '┘' || c == '┤' || c == '┴');
                    assert!(
                        has_closing,
                        "ER boxes overlap on line {}: corners at {} and {} without closing between\nLine: {}\nFull output:\n{}",
                        i, window[0], window[1], line, output
                    );
                }
            }
        }
    }

    #[test]
    fn er_ascii_edge_labels_visible() {
        let input = std::fs::read_to_string("docs/sources/er.mmd").unwrap();
        let (db, graph) = parse_er_and_layout(&input);
        let output = render_er_ascii(&db, &graph).unwrap();

        // Edge labels (relationship roles) should appear in the output
        // The er.mmd file has roles like "places", "contains"
        assert!(
            output.contains("places") || output.contains("contains"),
            "ER edge labels should be visible\nOutput:\n{}",
            output
        );
    }

    // --- Class diagram ASCII overlap regression tests ---

    #[test]
    fn class_ascii_boxes_dont_overlap() {
        let input = std::fs::read_to_string("docs/sources/class.mmd").unwrap();
        let (db, graph) = parse_and_layout_class(&input);
        let output = render_class_ascii(&db, &graph).unwrap();

        // No line should have overlapping box corners
        for (i, line) in output.lines().enumerate() {
            let chars: Vec<char> = line.chars().collect();
            let mut corner_positions: Vec<usize> = Vec::new();
            for (j, &ch) in chars.iter().enumerate() {
                if ch == '┌' || ch == '└' {
                    corner_positions.push(j);
                }
            }
            if corner_positions.len() >= 2 {
                for window in corner_positions.windows(2) {
                    let between = &chars[window[0]..=window[1]];
                    let has_closing = between
                        .iter()
                        .any(|&c| c == '┐' || c == '┘' || c == '┤' || c == '┴');
                    assert!(
                        has_closing,
                        "Class boxes overlap on line {}: corners at {} and {} without closing between\nLine: {}\nFull output:\n{}",
                        i, window[0], window[1], line, output
                    );
                }
            }
        }
    }

    #[test]
    fn class_ascii_all_labels_visible() {
        let input = std::fs::read_to_string("docs/sources/class.mmd").unwrap();
        let (db, graph) = parse_and_layout_class(&input);
        let output = render_class_ascii(&db, &graph).unwrap();

        // All class names should be visible
        for label in &["Animal", "Duck", "Fish", "Zebra"] {
            assert!(
                output.contains(label),
                "Class diagram should contain '{}' after coordinate fix\nOutput:\n{}",
                label,
                output
            );
        }
    }

    #[test]
    fn flowchart_edge_label_invalid_not_truncated() {
        let input = std::fs::read_to_string("docs/sources/flowchart_complex.mmd").unwrap();
        let (db, graph) = parse_and_layout(&input);
        let output = render_flowchart_ascii(&db, &graph).unwrap();

        // Edge labels near the Authentication diamond must render in full,
        // not be truncated by the diamond's occupied cells.
        for label in &["Invalid", "Valid", "Cache Hit", "Cache Miss"] {
            assert!(
                output.contains(label),
                "Flowchart edge label '{}' should not be truncated\nOutput:\n{}",
                label,
                output
            );
        }
    }
}
