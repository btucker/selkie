//! TUI (Text User Interface) renderer for diagrams.
//!
//! Produces character-art output using box-drawing characters for node shapes
//! and braille dots for edge routing. Pipe-friendly, works in every terminal.

pub mod canvas;
pub mod edges;
pub mod scale;
pub mod shapes;

use crate::diagrams::flowchart::FlowchartDb;
use crate::error::Result;
use crate::layout::LayoutGraph;

use scale::CellScale;
use shapes::render_shape;

/// Render a flowchart as character art.
///
/// Takes the parsed diagram DB and a positioned layout graph (after dagre),
/// and produces a String of character art with nodes at their correct positions
/// and edges rendered as braille lines with arrow tips.
pub fn render_flowchart_tui(db: &FlowchartDb, graph: &LayoutGraph) -> Result<String> {
    let scale = CellScale::default();

    // Determine canvas dimensions from graph bounds
    let graph_width = graph.width.unwrap_or(400.0);
    let graph_height = graph.height.unwrap_or(300.0);
    let offset_x = graph.bounds_x.unwrap_or(0.0);
    let offset_y = graph.bounds_y.unwrap_or(0.0);

    let canvas_cols = scale.to_col(graph_width) + 4;
    let canvas_rows = scale.to_row(graph_height) + 2;

    // Create a canvas (2D grid of characters)
    let mut canvas: Vec<Vec<char>> = vec![vec![' '; canvas_cols]; canvas_rows];
    // Track which cells are occupied by nodes (for edge compositing)
    let mut occupied: Vec<Vec<bool>> = vec![vec![false; canvas_cols]; canvas_rows];

    // Render each node
    for node in &graph.nodes {
        if node.is_dummy {
            continue;
        }

        let (nx, ny) = match (node.x, node.y) {
            (Some(x), Some(y)) => (x - offset_x, y - offset_y),
            _ => continue,
        };

        // Get the label: prefer the flowchart DB label, fall back to layout label, then ID
        let label = db
            .vertices()
            .iter()
            .find(|(id, _)| *id == &node.id)
            .and_then(|(_, v)| v.text.as_deref())
            .or(node.label.as_deref())
            .unwrap_or(&node.id);

        let cell_w = scale.to_cell_width(node.width);
        let cell_h = scale.to_cell_height(node.height);

        let rendered = render_shape(&node.shape, label, cell_w, cell_h);

        // Position: node x,y is center, so offset by half the rendered size
        let col_start = scale.to_col(nx).saturating_sub(rendered.width / 2);
        let row_start = scale.to_row(ny).saturating_sub(rendered.height / 2);

        // Blit the rendered shape onto the canvas
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
    fn single_node_renders() {
        let (db, graph) = parse_and_layout("flowchart TD\n    A[Hello]");
        let output = render_flowchart_tui(&db, &graph).unwrap();
        assert!(output.contains("Hello"), "Output should contain the label");
        assert!(
            output.contains('┌') || output.contains('╭'),
            "Output should contain box-drawing chars"
        );
    }

    #[test]
    fn two_nodes_render() {
        let (db, graph) = parse_and_layout("flowchart TD\n    A[Start] --> B[End]");
        let output = render_flowchart_tui(&db, &graph).unwrap();
        assert!(output.contains("Start"), "Should contain Start label");
        assert!(output.contains("End"), "Should contain End label");
    }

    #[test]
    fn round_node_uses_rounded_corners() {
        let (db, graph) = parse_and_layout("flowchart TD\n    A(Round)");
        let output = render_flowchart_tui(&db, &graph).unwrap();
        assert!(output.contains('╭'), "Round node should use ╭");
        assert!(output.contains('╯'), "Round node should use ╯");
    }

    #[test]
    fn diamond_node_renders() {
        let (db, graph) = parse_and_layout("flowchart TD\n    A{Decision}");
        let output = render_flowchart_tui(&db, &graph).unwrap();
        assert!(output.contains("Decision"), "Diamond should contain label");
    }

    #[test]
    fn output_is_nonempty() {
        let (db, graph) = parse_and_layout("flowchart TD\n    A[X]");
        let output = render_flowchart_tui(&db, &graph).unwrap();
        assert!(!output.trim().is_empty(), "Output should not be empty");
    }

    #[test]
    fn edges_produce_braille_chars() {
        let (db, graph) = parse_and_layout("flowchart TD\n    A[Start] --> B[End]");
        let output = render_flowchart_tui(&db, &graph).unwrap();
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
        let output = render_flowchart_tui(&db, &graph).unwrap();
        assert!(output.contains("Yes"), "Edge label 'Yes' should appear");
    }

    #[test]
    fn down_arrow_in_td_flow() {
        let (db, graph) = parse_and_layout("flowchart TD\n    A[Top] --> B[Bottom]");
        let output = render_flowchart_tui(&db, &graph).unwrap();
        assert!(output.contains('▼'), "TD flow should have down arrow ▼");
    }
}
