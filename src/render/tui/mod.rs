//! TUI (Text User Interface) renderer for diagrams.
//!
//! Produces character-art output using box-drawing characters for node shapes
//! and braille dots for edge routing. Pipe-friendly, works in every terminal.

pub mod canvas;
pub mod edges;
pub mod scale;
pub mod shapes;

use std::collections::HashSet;

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

    let canvas_cols = scale.to_col(graph_width) + 8;
    let canvas_rows = scale.to_row(graph_height) + 4;

    // Create a canvas (2D grid of characters)
    let mut canvas: Vec<Vec<char>> = vec![vec![' '; canvas_cols]; canvas_rows];
    // Track which cells are occupied by nodes (for edge compositing)
    let mut occupied: Vec<Vec<bool>> = vec![vec![false; canvas_cols]; canvas_rows];

    // Collect subgraph IDs — these are container nodes whose bounding box
    // encompasses their children. We render them as just a label, not a full box.
    let subgraph_ids: HashSet<&str> = db.subgraphs().iter().map(|sg| sg.id.as_str()).collect();

    // Render subgraph nodes first (background layer — just a label).
    // Collect positions so we can re-stamp them after regular nodes (pass 2).
    struct SubgraphLabel {
        row: usize,
        col_start: usize,
        label: String,
    }
    let mut subgraph_labels: Vec<SubgraphLabel> = Vec::new();

    for node in &graph.nodes {
        if node.is_dummy || !subgraph_ids.contains(node.id.as_str()) {
            continue;
        }

        let (nx, ny) = match (node.x, node.y) {
            (Some(x), Some(y)) => (x - offset_x, y - offset_y),
            _ => continue,
        };

        let label = node_label(db, node);

        // For subgraphs, render label at top-center of the bounding box
        let col_center = scale.to_col(nx + node.width / 2.0);
        let row_top = scale.to_row(ny);
        let label_char_count = label.chars().count();
        let label_start = col_center.saturating_sub(label_char_count / 2);

        if row_top < canvas_rows {
            for (i, ch) in label.chars().enumerate() {
                let c = label_start + i;
                if c < canvas_cols {
                    canvas[row_top][c] = ch;
                    occupied[row_top][c] = true;
                }
            }
        }

        subgraph_labels.push(SubgraphLabel {
            row: row_top,
            col_start: label_start,
            label,
        });
    }

    // Render regular (non-subgraph) nodes, sorted by area ascending so
    // smaller nodes render first and aren't occluded by large shapes (e.g., diamonds).
    let mut regular_nodes: Vec<&crate::layout::LayoutNode> = graph
        .nodes
        .iter()
        .filter(|n| !n.is_dummy && !subgraph_ids.contains(n.id.as_str()))
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

        let label = node_label(db, node);

        let cell_w = scale.to_cell_width(node.width);
        let cell_h = scale.to_cell_height(node.height);

        let rendered = render_shape(&node.shape, &label, cell_w, cell_h);

        // Position: node x,y is center, so offset by half the rendered size
        let col_start = scale.to_col(nx).saturating_sub(rendered.width / 2);
        let row_start = scale.to_row(ny).saturating_sub(rendered.height / 2);

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
    // Re-stamp subgraph labels (they were rendered before regular nodes)
    for sg in &subgraph_labels {
        if sg.row >= canvas_rows {
            continue;
        }
        for (i, ch) in sg.label.chars().enumerate() {
            let c = sg.col_start + i;
            if c < canvas_cols {
                canvas[sg.row][c] = ch;
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

/// Get the display label for a node, cleaning HTML tags like `<br/>`.
fn node_label(db: &FlowchartDb, node: &crate::layout::LayoutNode) -> String {
    let raw = db
        .vertices()
        .iter()
        .find(|(id, _)| *id == &node.id)
        .and_then(|(_, v)| v.text.as_deref())
        .or(node.label.as_deref())
        .unwrap_or(&node.id);
    // Clean HTML line breaks and normalize whitespace for TUI display
    let cleaned = raw.replace("<br/>", " ").replace("<br>", " ");
    cleaned.split_whitespace().collect::<Vec<_>>().join(" ")
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
        let output = render_flowchart_tui(&db, &graph).unwrap();

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
        let output = render_flowchart_tui(&db, &graph).unwrap();
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
        let output = render_flowchart_tui(&db, &graph).unwrap();
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
        let output = render_flowchart_tui(&db, &graph).unwrap();
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
        let output = render_flowchart_tui(&db, &graph).unwrap();
        assert!(
            output.contains("Начало"),
            "Cyrillic label must be visible\nOutput:\n{}",
            output
        );
    }

    #[test]
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
        let output = render_flowchart_tui(&db, &graph).unwrap();
        assert!(
            output.contains("Circle shape Начало"),
            "Cyrillic circle label must be visible\nOutput:\n{}",
            output
        );
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
