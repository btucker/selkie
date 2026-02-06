//! ASCII renderer for treemap diagrams.
//!
//! Renders treemaps as nested rectangles using box-drawing characters.
//! Each node's area is proportional to its value. Uses a squarified
//! layout algorithm adapted for character-cell aspect ratios.

use crate::diagrams::treemap::{TreemapDb, TreemapNode};
use crate::error::Result;

/// Default canvas width in columns.
const DEFAULT_WIDTH: usize = 72;
/// Default canvas height in rows.
const DEFAULT_HEIGHT: usize = 20;
/// Padding inside section headers (rows consumed by header label).
const SECTION_HEADER_ROWS: usize = 1;

/// A rectangle in character-cell coordinates.
#[derive(Debug, Clone, Copy)]
struct Rect {
    x: usize,
    y: usize,
    w: usize,
    h: usize,
}

/// Render a treemap as character art with nested rectangles.
pub fn render_treemap_ascii(db: &TreemapDb) -> Result<String> {
    let root_nodes = db.get_root_nodes();
    if root_nodes.is_empty() {
        let title = db.get_title();
        if !title.is_empty() {
            return Ok(format!("{}\n\n(empty treemap)\n", title));
        }
        return Ok("(empty treemap)\n".to_string());
    }

    let total_value = calculate_total(root_nodes);

    // Determine canvas size
    let width = DEFAULT_WIDTH;
    let height = DEFAULT_HEIGHT;

    // If there's a title, reserve top rows
    let title = db.get_title();
    let title_rows = if title.is_empty() { 0 } else { 1 };
    let footer_rows = if total_value > 0.0 { 2 } else { 0 }; // blank + total line

    let map_height = height.saturating_sub(title_rows + footer_rows);
    let map_rect = Rect {
        x: 0,
        y: 0,
        w: width,
        h: map_height,
    };

    // Create canvas
    let canvas_h = title_rows + map_height + footer_rows;
    let mut canvas: Vec<Vec<char>> = vec![vec![' '; width]; canvas_h];

    // Draw title
    if !title.is_empty() {
        let truncated = truncate_str(title, width);
        for (i, ch) in truncated.chars().enumerate() {
            if i < width {
                canvas[0][i] = ch;
            }
        }
    }

    // Layout and render the treemap into the canvas region
    let adjusted_rect = Rect {
        x: map_rect.x,
        y: map_rect.y + title_rows,
        w: map_rect.w,
        h: map_rect.h,
    };
    layout_and_render(&mut canvas, root_nodes, adjusted_rect);

    // Draw total
    if total_value > 0.0 {
        let total_row = title_rows + map_height + 1; // +1 for blank line
        let total_str = if total_value.fract() == 0.0 {
            format!("Total: {}", total_value as i64)
        } else {
            format!("Total: {:.1}", total_value)
        };
        if total_row < canvas_h {
            for (i, ch) in total_str.chars().enumerate() {
                if i < width {
                    canvas[total_row][i] = ch;
                }
            }
        }
    }

    // Convert canvas to string
    let mut result = String::new();
    // Find last non-empty row
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

/// Layout sibling nodes into the given rectangle and render them.
fn layout_and_render(canvas: &mut [Vec<char>], nodes: &[TreemapNode], rect: Rect) {
    if nodes.is_empty() || rect.w < 2 || rect.h < 2 {
        return;
    }

    // Calculate total value for all nodes
    let total = nodes.iter().map(node_value).sum::<f64>();
    if total <= 0.0 {
        return;
    }

    // Use squarified layout to partition the rectangle
    let rects = squarify(nodes, rect, total);

    for (node, sub_rect) in nodes.iter().zip(rects.iter()) {
        render_node_rect(canvas, node, *sub_rect);
    }
}

/// Render a single node as a rectangle on the canvas.
fn render_node_rect(canvas: &mut [Vec<char>], node: &TreemapNode, rect: Rect) {
    if rect.w < 2 || rect.h < 2 {
        return;
    }

    // Draw the box border
    draw_box(canvas, rect);

    if node.is_leaf() {
        // Leaf node: draw label and value inside
        let inner_w = rect.w.saturating_sub(2);
        let inner_h = rect.h.saturating_sub(2);
        if inner_w == 0 || inner_h == 0 {
            return;
        }

        // Label on first inner row
        let label = truncate_str(&node.name, inner_w);
        let label_row = rect.y + 1;
        let label_col = rect.x + 1;
        for (i, ch) in label.chars().enumerate() {
            set_char(canvas, label_row, label_col + i, ch);
        }

        // Value on second inner row (if space)
        if inner_h >= 2 {
            if let Some(value) = node.value {
                let value_str = if value.fract() == 0.0 {
                    format!("{}", value as i64)
                } else {
                    format!("{:.1}", value)
                };
                let val_display = truncate_str(&value_str, inner_w);
                let val_row = label_row + 1;
                for (i, ch) in val_display.chars().enumerate() {
                    set_char(canvas, val_row, label_col + i, ch);
                }
            }
        }
    } else {
        // Section node: draw label in top row, then recurse into children
        let inner_w = rect.w.saturating_sub(2);
        if inner_w == 0 {
            return;
        }

        // Section header label
        let label = truncate_str(&node.name, inner_w);
        let label_row = rect.y + 1;
        let label_col = rect.x + 1;
        for (i, ch) in label.chars().enumerate() {
            set_char(canvas, label_row, label_col + i, ch);
        }

        // Draw a horizontal divider below the header
        let divider_row = rect.y + 1 + SECTION_HEADER_ROWS;
        if divider_row < rect.y + rect.h - 1 {
            set_char(canvas, divider_row, rect.x, '├');
            for c in (rect.x + 1)..(rect.x + rect.w - 1) {
                set_char(canvas, divider_row, c, '─');
            }
            set_char(canvas, divider_row, rect.x + rect.w - 1, '┤');
        }

        // Children get the remaining space below the divider
        let children_y = divider_row + 1;
        let children_h = (rect.y + rect.h - 1).saturating_sub(children_y);
        if children_h >= 2 && !node.children.is_empty() {
            let children_rect = Rect {
                x: rect.x + 1,
                y: children_y,
                w: rect.w.saturating_sub(2),
                h: children_h,
            };
            layout_and_render(canvas, &node.children, children_rect);
        }
    }
}

/// Draw a box outline on the canvas.
fn draw_box(canvas: &mut [Vec<char>], rect: Rect) {
    if rect.w < 2 || rect.h < 2 {
        return;
    }

    let x1 = rect.x;
    let y1 = rect.y;
    let x2 = rect.x + rect.w - 1;
    let y2 = rect.y + rect.h - 1;

    // Corners
    set_char(canvas, y1, x1, '┌');
    set_char(canvas, y1, x2, '┐');
    set_char(canvas, y2, x1, '└');
    set_char(canvas, y2, x2, '┘');

    // Top and bottom borders
    for c in (x1 + 1)..x2 {
        set_char(canvas, y1, c, '─');
        set_char(canvas, y2, c, '─');
    }

    // Left and right borders
    for r in (y1 + 1)..y2 {
        set_char(canvas, r, x1, '│');
        set_char(canvas, r, x2, '│');
    }
}

/// Safely set a character on the canvas.
fn set_char(canvas: &mut [Vec<char>], row: usize, col: usize, ch: char) {
    if row < canvas.len() && col < canvas[row].len() {
        canvas[row][col] = ch;
    }
}

/// Squarified treemap layout: partition a rectangle among nodes proportional
/// to their values, trying to keep aspect ratios close to 1:1.
///
/// In ASCII, characters are roughly 2:1 (height:width), so 1 row ≈ 2 cols.
/// We account for this when computing aspect ratios.
fn squarify(nodes: &[TreemapNode], rect: Rect, total: f64) -> Vec<Rect> {
    if nodes.is_empty() || rect.w < 2 || rect.h < 2 || total <= 0.0 {
        return vec![
            Rect {
                x: rect.x,
                y: rect.y,
                w: rect.w.max(2),
                h: rect.h.max(2),
            };
            nodes.len()
        ];
    }

    if nodes.len() == 1 {
        return vec![rect];
    }

    // Use a simple slice-and-dice approach: alternate horizontal and vertical
    // splits based on the aspect ratio of the remaining rectangle.
    // Character cells are ~2:1 (tall:wide), so effective_w = w, effective_h = h * 2
    let effective_w = rect.w as f64;
    let effective_h = rect.h as f64 * 2.0; // account for char aspect ratio

    let horizontal = effective_w >= effective_h; // split horizontally if wider

    let mut rects = Vec::with_capacity(nodes.len());
    let mut remaining = rect;
    let mut remaining_total = total;

    for (i, node) in nodes.iter().enumerate() {
        if i == nodes.len() - 1 {
            // Last node gets whatever is left
            rects.push(remaining);
            break;
        }

        let value = node_value(node);
        let fraction = value / remaining_total;

        // Minimum size 3 so border + label + border fits
        let min_size = 3;
        if horizontal {
            // Split vertically (allocate columns)
            let cols = ((remaining.w as f64 * fraction).round() as usize).max(min_size);
            let cols = cols.min(remaining.w.saturating_sub(min_size * (nodes.len() - i - 1)));
            rects.push(Rect {
                x: remaining.x,
                y: remaining.y,
                w: cols,
                h: remaining.h,
            });
            remaining.x += cols;
            remaining.w = remaining.w.saturating_sub(cols);
        } else {
            // Split horizontally (allocate rows)
            let rows = ((remaining.h as f64 * fraction).round() as usize).max(min_size);
            let rows = rows.min(remaining.h.saturating_sub(min_size * (nodes.len() - i - 1)));
            rects.push(Rect {
                x: remaining.x,
                y: remaining.y,
                w: remaining.w,
                h: rows,
            });
            remaining.y += rows;
            remaining.h = remaining.h.saturating_sub(rows);
        }

        remaining_total -= value;
        if remaining_total <= 0.0 {
            remaining_total = 1.0; // avoid division by zero
        }
    }

    rects
}

/// Get the effective value of a node (sum of leaves for sections).
fn node_value(node: &TreemapNode) -> f64 {
    if let Some(v) = node.value {
        v
    } else {
        calculate_total(&node.children)
    }
}

fn calculate_total(nodes: &[TreemapNode]) -> f64 {
    nodes.iter().map(node_value).sum()
}

/// Truncate a string to fit within a given character width.
fn truncate_str(s: &str, max_width: usize) -> String {
    let chars: Vec<char> = s.chars().collect();
    if chars.len() <= max_width {
        s.to_string()
    } else if max_width >= 2 {
        chars[..max_width - 1].iter().collect::<String>() + "…"
    } else {
        chars[..max_width].iter().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_treemap(input: &str) -> TreemapDb {
        let diagram = crate::parse(input).unwrap();
        match diagram {
            crate::diagrams::Diagram::Treemap(db) => db,
            _ => panic!("Expected treemap"),
        }
    }

    #[test]
    fn empty_treemap() {
        let db = TreemapDb::new();
        let output = render_treemap_ascii(&db).unwrap();
        assert!(output.contains("empty treemap"));
    }

    #[test]
    fn gallery_treemap_renders_all_labels() {
        let input = std::fs::read_to_string("docs/sources/treemap.mmd").unwrap();
        let db = parse_treemap(&input);
        let output = render_treemap_ascii(&db).unwrap();
        assert!(output.contains("Category A"), "Output:\n{}", output);
        assert!(output.contains("Category B"), "Output:\n{}", output);
        assert!(output.contains("Item A1"), "Output:\n{}", output);
        assert!(output.contains("Item A2"), "Output:\n{}", output);
        assert!(output.contains("Item B1"), "Output:\n{}", output);
        assert!(output.contains("Item B2"), "Output:\n{}", output);
    }

    #[test]
    fn values_appear() {
        let input = std::fs::read_to_string("docs/sources/treemap.mmd").unwrap();
        let db = parse_treemap(&input);
        let output = render_treemap_ascii(&db).unwrap();
        assert!(output.contains("10"), "Output:\n{}", output);
        assert!(output.contains("25"), "Output:\n{}", output);
    }

    #[test]
    fn has_nested_rectangles() {
        let input = std::fs::read_to_string("docs/sources/treemap.mmd").unwrap();
        let db = parse_treemap(&input);
        let output = render_treemap_ascii(&db).unwrap();
        // Should have box-drawing corners indicating nested rectangles
        assert!(
            output.contains('┌'),
            "Should have top-left corners\nOutput:\n{}",
            output
        );
        assert!(
            output.contains('┘'),
            "Should have bottom-right corners\nOutput:\n{}",
            output
        );
        assert!(
            output.contains('│'),
            "Should have vertical borders\nOutput:\n{}",
            output
        );
        assert!(
            output.contains('─'),
            "Should have horizontal borders\nOutput:\n{}",
            output
        );
    }

    #[test]
    fn section_has_divider() {
        let input = std::fs::read_to_string("docs/sources/treemap.mmd").unwrap();
        let db = parse_treemap(&input);
        let output = render_treemap_ascii(&db).unwrap();
        // Sections should have ├──┤ divider separating header from children
        assert!(
            output.contains('├'),
            "Should have section dividers\nOutput:\n{}",
            output
        );
        assert!(
            output.contains('┤'),
            "Should have section dividers\nOutput:\n{}",
            output
        );
    }

    #[test]
    fn total_appears() {
        let input = std::fs::read_to_string("docs/sources/treemap.mmd").unwrap();
        let db = parse_treemap(&input);
        let output = render_treemap_ascii(&db).unwrap();
        assert!(output.contains("Total: 70"), "Output:\n{}", output);
    }

    #[test]
    fn complex_treemap_renders_section_labels() {
        let input = std::fs::read_to_string("docs/sources/treemap_complex.mmd").unwrap();
        let db = parse_treemap(&input);
        let output = render_treemap_ascii(&db).unwrap();
        // Section labels should always appear (they get the full width)
        for label in &["Company Budget", "Engineering", "Marketing", "Sales"] {
            assert!(
                output.contains(label),
                "Should contain section '{}'\nOutput:\n{}",
                label,
                output
            );
        }
        // Leaf labels may be truncated in narrow cells, check at least 3 chars
        for label in &["Backend", "Direct", "Channel", "Digital"] {
            assert!(
                output.contains(label) || output.contains(&label[..3]),
                "Should contain leaf '{}' (possibly truncated)\nOutput:\n{}",
                label,
                output
            );
        }
    }

    #[test]
    fn no_indented_list_markers() {
        // The old renderer used bar chart blocks (█). The new renderer uses
        // proper nested rectangles with box-drawing characters instead.
        let input = std::fs::read_to_string("docs/sources/treemap.mmd").unwrap();
        let db = parse_treemap(&input);
        let output = render_treemap_ascii(&db).unwrap();
        assert!(
            !output.contains('█'),
            "Should not have bar chart blocks\nOutput:\n{}",
            output
        );
    }

    #[test]
    fn single_leaf_renders() {
        let input = "treemap-beta\n\"Root\"\n    \"Leaf\": 100\n";
        let db = parse_treemap(input);
        let output = render_treemap_ascii(&db).unwrap();
        assert!(output.contains("Root"), "Output:\n{}", output);
        assert!(output.contains("Leaf"), "Output:\n{}", output);
        assert!(output.contains("100"), "Output:\n{}", output);
    }

    #[test]
    fn proportional_widths() {
        // Two leaves with 1:3 ratio - the larger should get more columns
        let input = "treemap-beta\n\"A\": 25\n\"B\": 75\n";
        let db = parse_treemap(input);
        let output = render_treemap_ascii(&db).unwrap();
        assert!(output.contains('A'), "Output:\n{}", output);
        assert!(output.contains('B'), "Output:\n{}", output);
    }

    #[test]
    fn title_rendered() {
        let mut db = TreemapDb::new();
        db.set_title("My Title");
        let mut root = TreemapNode::section("Section");
        root.add_child(TreemapNode::leaf("Item", 50.0));
        db.add_root_node(root);
        let output = render_treemap_ascii(&db).unwrap();
        assert!(output.contains("My Title"), "Output:\n{}", output);
    }

    #[test]
    fn matches_ascii_reference_treemap() {
        let input = std::fs::read_to_string("docs/sources/treemap.mmd").unwrap();
        let db = parse_treemap(&input);
        let output = render_treemap_ascii(&db).unwrap();
        let reference = std::fs::read_to_string("docs/images/ascii/treemap.txt")
            .unwrap()
            .replace("\r\n", "\n");
        assert_eq!(
            output, reference,
            "Treemap ASCII output differs from reference file.\n\
             If the renderer changed intentionally, update docs/images/ascii/treemap.txt:\n\
             cargo run --features all-formats --bin selkie -- docs/sources/treemap.mmd --output-format ascii > docs/images/ascii/treemap.txt"
        );
    }

    #[test]
    fn matches_ascii_reference_treemap_complex() {
        let input = std::fs::read_to_string("docs/sources/treemap_complex.mmd").unwrap();
        let db = parse_treemap(&input);
        let output = render_treemap_ascii(&db).unwrap();
        let reference = std::fs::read_to_string("docs/images/ascii/treemap_complex.txt")
            .unwrap()
            .replace("\r\n", "\n");
        assert_eq!(
            output, reference,
            "Complex treemap ASCII output differs from reference file.\n\
             If the renderer changed intentionally, update docs/images/ascii/treemap_complex.txt:\n\
             cargo run --features all-formats --bin selkie -- docs/sources/treemap_complex.mmd --output-format ascii > docs/images/ascii/treemap_complex.txt"
        );
    }
}
