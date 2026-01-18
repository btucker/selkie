//! Sankey diagram renderer
//!
//! Renders Sankey diagrams showing flow between nodes with weighted connections.
//! The layout algorithm assigns nodes to columns based on their position in the
//! flow graph, then calculates vertical positions based on flow values.

use std::collections::{HashMap, HashSet};

use crate::diagrams::sankey::SankeyDb;
use crate::error::Result;
use crate::render::svg::{Attrs, RenderConfig, SvgDocument, SvgElement};

/// Default dimensions matching mermaid.js
const DEFAULT_WIDTH: f64 = 600.0;
const DEFAULT_HEIGHT: f64 = 400.0;
const NODE_WIDTH: f64 = 10.0;
const NODE_PADDING: f64 = 10.0;
const LABEL_PADDING: f64 = 6.0;
const FONT_SIZE: f64 = 14.0;

/// Computed node position and dimensions
#[derive(Debug, Clone)]
#[allow(dead_code)] // Some fields reserved for future enhancements (e.g., showValues)
struct LayoutNode {
    id: String,
    column: usize,
    x0: f64,
    y0: f64,
    x1: f64,
    y1: f64,
    value: f64, // Total flow through this node
}

/// Computed link position
#[derive(Debug, Clone)]
#[allow(dead_code)] // Value field reserved for future tooltip/label features
struct LayoutLink {
    source_id: String,
    target_id: String,
    value: f64,
    source_y0: f64, // Start y position at source
    source_y1: f64,
    target_y0: f64, // End y position at target
    target_y1: f64,
    source_x: f64,  // x position at source (right edge of node)
    target_x: f64,  // x position at target (left edge of node)
}

/// Render a sankey diagram to SVG
pub fn render_sankey(db: &SankeyDb, config: &RenderConfig) -> Result<String> {
    let mut doc = SvgDocument::new();

    let graph = db.get_graph();

    // Handle empty graph
    if graph.nodes.is_empty() {
        doc.set_size(DEFAULT_WIDTH, DEFAULT_HEIGHT);
        return Ok(doc.to_string());
    }

    // Compute layout
    let (layout_nodes, layout_links) = compute_layout(db, DEFAULT_WIDTH, DEFAULT_HEIGHT);

    doc.set_size(DEFAULT_WIDTH, DEFAULT_HEIGHT);

    // Add theme styles
    if config.embed_css {
        doc.add_style(&config.theme.generate_css());
        doc.add_style(&generate_sankey_css(&config.theme));
    }

    // Add gradient definitions for links
    let defs = create_gradient_defs(&layout_nodes, &layout_links, config);
    doc.add_defs(vec![defs]);

    // Render links first (behind nodes)
    let links_group = render_links(&layout_links, config);
    doc.add_element(links_group);

    // Render nodes
    let nodes_group = render_nodes(&layout_nodes, config);
    doc.add_element(nodes_group);

    // Render labels
    let labels_group = render_labels(&layout_nodes, DEFAULT_WIDTH, config);
    doc.add_element(labels_group);

    Ok(doc.to_string())
}

/// Compute the sankey layout - assigns positions to all nodes and links
fn compute_layout(db: &SankeyDb, width: f64, height: f64) -> (Vec<LayoutNode>, Vec<LayoutLink>) {
    let graph = db.get_graph();

    if graph.nodes.is_empty() {
        return (Vec::new(), Vec::new());
    }

    // Step 1: Build adjacency info
    let mut outgoing: HashMap<String, Vec<(String, f64)>> = HashMap::new();
    let mut incoming: HashMap<String, Vec<(String, f64)>> = HashMap::new();

    for link in &graph.links {
        outgoing
            .entry(link.source.clone())
            .or_default()
            .push((link.target.clone(), link.value));
        incoming
            .entry(link.target.clone())
            .or_default()
            .push((link.source.clone(), link.value));
    }

    // Step 2: Compute node columns (depth) using BFS from sources
    let node_columns = compute_node_columns(&graph.nodes, &outgoing, &incoming);

    // Find max column
    let max_column = node_columns.values().copied().max().unwrap_or(0);
    let num_columns = max_column + 1;

    // Step 3: Compute node values (total flow through each node)
    let node_values = compute_node_values(&graph.nodes, &graph.links);

    // Step 4: Calculate x positions based on columns
    let column_width = if num_columns > 1 {
        (width - NODE_WIDTH) / (num_columns - 1) as f64
    } else {
        0.0
    };

    // Step 5: Group nodes by column and compute y positions
    let mut nodes_by_column: Vec<Vec<String>> = vec![Vec::new(); num_columns];
    for node in &graph.nodes {
        let col = node_columns.get(&node.id).copied().unwrap_or(0);
        nodes_by_column[col].push(node.id.clone());
    }

    // Compute y positions within each column
    let mut layout_nodes: Vec<LayoutNode> = Vec::new();
    let mut node_positions: HashMap<String, (f64, f64, f64, f64)> = HashMap::new(); // id -> (x0, y0, x1, y1)

    for (col, col_nodes) in nodes_by_column.iter().enumerate() {
        let x0 = col as f64 * column_width;
        let x1 = x0 + NODE_WIDTH;

        // Calculate total value in this column for scaling
        let total_value: f64 = col_nodes
            .iter()
            .map(|id| node_values.get(id).copied().unwrap_or(0.0))
            .sum();

        // Available height minus padding between nodes
        let padding_total = NODE_PADDING * (col_nodes.len().saturating_sub(1)) as f64;
        let available_height = height - padding_total;

        // Position nodes vertically
        let mut current_y = 0.0;

        for node_id in col_nodes {
            let value = node_values.get(node_id).copied().unwrap_or(0.0);
            let node_height = if total_value > 0.0 {
                (value / total_value) * available_height
            } else {
                available_height / col_nodes.len() as f64
            }
            .max(1.0); // Minimum height of 1

            let y0 = current_y;
            let y1 = y0 + node_height;

            layout_nodes.push(LayoutNode {
                id: node_id.clone(),
                column: col,
                x0,
                y0,
                x1,
                y1,
                value,
            });

            node_positions.insert(node_id.clone(), (x0, y0, x1, y1));
            current_y = y1 + NODE_PADDING;
        }
    }

    // Step 6: Compute link positions
    let layout_links = compute_link_positions(&graph.links, &node_positions, &node_values);

    (layout_nodes, layout_links)
}

/// Compute node columns using topological sort from sources
fn compute_node_columns(
    nodes: &[crate::diagrams::sankey::GraphNode],
    outgoing: &HashMap<String, Vec<(String, f64)>>,
    incoming: &HashMap<String, Vec<(String, f64)>>,
) -> HashMap<String, usize> {
    let mut columns: HashMap<String, usize> = HashMap::new();

    // Find source nodes (no incoming edges)
    let source_nodes: Vec<_> = nodes
        .iter()
        .filter(|n| !incoming.contains_key(&n.id) || incoming.get(&n.id).unwrap().is_empty())
        .map(|n| n.id.clone())
        .collect();

    // BFS from sources
    let mut queue: Vec<(String, usize)> = source_nodes.iter().map(|id| (id.clone(), 0)).collect();
    let mut visited: HashSet<String> = HashSet::new();

    while let Some((node_id, col)) = queue.pop() {
        // Update column to max seen
        let current_col = columns.entry(node_id.clone()).or_insert(0);
        *current_col = (*current_col).max(col);

        if visited.contains(&node_id) {
            continue;
        }
        visited.insert(node_id.clone());

        // Process outgoing edges
        if let Some(edges) = outgoing.get(&node_id) {
            for (target, _) in edges {
                queue.push((target.clone(), col + 1));
            }
        }
    }

    // Handle any unvisited nodes (disconnected components)
    for node in nodes {
        columns.entry(node.id.clone()).or_insert(0);
    }

    columns
}

/// Compute total flow through each node
fn compute_node_values(
    nodes: &[crate::diagrams::sankey::GraphNode],
    links: &[crate::diagrams::sankey::GraphLink],
) -> HashMap<String, f64> {
    let mut values: HashMap<String, f64> = HashMap::new();

    // Initialize all nodes
    for node in nodes {
        values.insert(node.id.clone(), 0.0);
    }

    // Sum incoming and outgoing values, take max
    let mut incoming_values: HashMap<String, f64> = HashMap::new();
    let mut outgoing_values: HashMap<String, f64> = HashMap::new();

    for link in links {
        *incoming_values.entry(link.target.clone()).or_insert(0.0) += link.value;
        *outgoing_values.entry(link.source.clone()).or_insert(0.0) += link.value;
    }

    for node in nodes {
        let incoming = incoming_values.get(&node.id).copied().unwrap_or(0.0);
        let outgoing = outgoing_values.get(&node.id).copied().unwrap_or(0.0);
        values.insert(node.id.clone(), incoming.max(outgoing));
    }

    values
}

/// Compute link positions based on node positions
fn compute_link_positions(
    links: &[crate::diagrams::sankey::GraphLink],
    node_positions: &HashMap<String, (f64, f64, f64, f64)>,
    node_values: &HashMap<String, f64>,
) -> Vec<LayoutLink> {
    // Track current y offset at each node for stacking links
    let mut source_offsets: HashMap<String, f64> = HashMap::new();
    let mut target_offsets: HashMap<String, f64> = HashMap::new();

    let mut layout_links = Vec::new();

    for link in links {
        let (_source_x0, source_y0, source_x1, source_y1) = node_positions
            .get(&link.source)
            .copied()
            .unwrap_or((0.0, 0.0, NODE_WIDTH, 10.0));

        let (target_x0, target_y0, _target_x1, target_y1) = node_positions
            .get(&link.target)
            .copied()
            .unwrap_or((0.0, 0.0, NODE_WIDTH, 10.0));

        let source_value = node_values.get(&link.source).copied().unwrap_or(1.0);
        let target_value = node_values.get(&link.target).copied().unwrap_or(1.0);

        // Calculate link height at source and target based on proportion
        let source_height = source_y1 - source_y0;
        let target_height = target_y1 - target_y0;

        let link_height_at_source = if source_value > 0.0 {
            (link.value / source_value) * source_height
        } else {
            source_height
        };

        let link_height_at_target = if target_value > 0.0 {
            (link.value / target_value) * target_height
        } else {
            target_height
        };

        // Get current offset at source and target
        let source_offset = source_offsets.entry(link.source.clone()).or_insert(0.0);
        let target_offset = target_offsets.entry(link.target.clone()).or_insert(0.0);

        let link_source_y0 = source_y0 + *source_offset;
        let link_source_y1 = link_source_y0 + link_height_at_source;
        let link_target_y0 = target_y0 + *target_offset;
        let link_target_y1 = link_target_y0 + link_height_at_target;

        // Update offsets for next link
        *source_offset += link_height_at_source;
        *target_offset += link_height_at_target;

        layout_links.push(LayoutLink {
            source_id: link.source.clone(),
            target_id: link.target.clone(),
            value: link.value,
            source_y0: link_source_y0,
            source_y1: link_source_y1,
            target_y0: link_target_y0,
            target_y1: link_target_y1,
            source_x: source_x1, // Right edge of source node
            target_x: target_x0, // Left edge of target node
        });
    }

    layout_links
}

/// Create gradient definitions for links
fn create_gradient_defs(
    nodes: &[LayoutNode],
    links: &[LayoutLink],
    config: &RenderConfig,
) -> SvgElement {
    let colors = &config.theme.sankey_node_colors;
    let node_colors: HashMap<_, _> = nodes
        .iter()
        .enumerate()
        .map(|(i, n)| (n.id.clone(), colors[i % colors.len()].as_str()))
        .collect();

    let mut children = Vec::new();

    for (i, link) in links.iter().enumerate() {
        let source_color = node_colors
            .get(&link.source_id)
            .copied()
            .unwrap_or(colors[0].as_str());
        let target_color = node_colors
            .get(&link.target_id)
            .copied()
            .unwrap_or(colors[1 % colors.len()].as_str());

        // Create linear gradient
        let gradient_id = format!("linearGradient-{}", i + 1);

        let stop1 = SvgElement::Raw {
            content: format!(
                "<stop offset=\"0%\" stop-color=\"{}\"/>",
                source_color
            ),
        };

        let stop2 = SvgElement::Raw {
            content: format!(
                "<stop offset=\"100%\" stop-color=\"{}\"/>",
                target_color
            ),
        };

        let gradient = SvgElement::Raw {
            content: format!(
                "<linearGradient id=\"{}\" gradientUnits=\"userSpaceOnUse\" x1=\"{}\" x2=\"{}\">{}{}</linearGradient>",
                gradient_id,
                link.source_x,
                link.target_x,
                stop1.to_svg(0),
                stop2.to_svg(0)
            ),
        };

        children.push(gradient);
    }

    SvgElement::Defs { children }
}

/// Render all links
fn render_links(links: &[LayoutLink], config: &RenderConfig) -> SvgElement {
    let mut children = Vec::new();

    for (i, link) in links.iter().enumerate() {
        // Create a smooth horizontal link path using cubic Bezier curves
        // d3-sankey uses a specific curve that we approximate
        // Control points at horizontal midpoint
        let mid_x = (link.source_x + link.target_x) / 2.0;

        // Path for the link (as a filled shape)
        let d = format!(
            "M{},{} C{},{} {},{} {},{} L{},{} C{},{} {},{} {},{} Z",
            // Start at top of source
            link.source_x,
            link.source_y0,
            // Control point 1 (horizontal middle, source y)
            mid_x,
            link.source_y0,
            // Control point 2 (horizontal middle, target y)
            mid_x,
            link.target_y0,
            // End at top of target
            link.target_x,
            link.target_y0,
            // Line to bottom of target
            link.target_x,
            link.target_y1,
            // Control point 3 (horizontal middle, target y)
            mid_x,
            link.target_y1,
            // Control point 4 (horizontal middle, source y)
            mid_x,
            link.source_y1,
            // End at bottom of source
            link.source_x,
            link.source_y1,
        );

        let gradient_id = format!("url(#linearGradient-{})", i + 1);

        let link_path = SvgElement::Path {
            d,
            attrs: Attrs::new()
                .with_fill(&gradient_id)
                .with_attr("fill-opacity", &config.theme.sankey_link_opacity)
                .with_class("sankey-link"),
        };

        // Wrap in group for the link
        let link_group = SvgElement::Group {
            children: vec![link_path],
            attrs: Attrs::new()
                .with_class("link")
                .with_attr("style", "mix-blend-mode: multiply"),
        };

        children.push(link_group);
    }

    SvgElement::Group {
        children,
        attrs: Attrs::new().with_class("links"),
    }
}

/// Render all nodes
fn render_nodes(nodes: &[LayoutNode], config: &RenderConfig) -> SvgElement {
    let mut children = Vec::new();
    let colors = &config.theme.sankey_node_colors;

    for (i, node) in nodes.iter().enumerate() {
        let color = &colors[i % colors.len()];

        let rect = SvgElement::Rect {
            x: node.x0,
            y: node.y0,
            width: node.x1 - node.x0,
            height: node.y1 - node.y0,
            rx: None,
            ry: None,
            attrs: Attrs::new().with_fill(color).with_class("sankey-node"),
        };

        let node_group = SvgElement::Group {
            children: vec![rect],
            attrs: Attrs::new()
                .with_class("node")
                .with_id(&format!("node-{}", i + 1))
                .with_attr("transform", &format!("translate({},{})", node.x0, node.y0))
                .with_attr("x", &format!("{}", node.x0))
                .with_attr("y", &format!("{}", node.y0)),
        };

        children.push(node_group);
    }

    SvgElement::Group {
        children,
        attrs: Attrs::new().with_class("nodes"),
    }
}

/// Render node labels
fn render_labels(nodes: &[LayoutNode], width: f64, config: &RenderConfig) -> SvgElement {
    let mut children = Vec::new();

    for node in nodes {
        // Position label to right of node if in left half, otherwise to left
        let node_center_x = (node.x0 + node.x1) / 2.0;
        let is_left_side = node_center_x < width / 2.0;

        let (label_x, text_anchor) = if is_left_side {
            (node.x1 + LABEL_PADDING, "start")
        } else {
            (node.x0 - LABEL_PADDING, "end")
        };

        let label_y = (node.y0 + node.y1) / 2.0;

        let label = SvgElement::Text {
            x: label_x,
            y: label_y,
            content: node.id.clone(),
            attrs: Attrs::new()
                .with_attr("text-anchor", text_anchor)
                .with_attr("dominant-baseline", "middle")
                .with_attr("font-size", &format!("{}", FONT_SIZE))
                .with_fill(&config.theme.sankey_label_color)
                .with_class("sankey-label"),
        };

        children.push(label);
    }

    SvgElement::Group {
        children,
        attrs: Attrs::new()
            .with_class("node-labels")
            .with_attr("font-size", &format!("{}", FONT_SIZE)),
    }
}

/// Generate CSS for sankey diagrams
fn generate_sankey_css(theme: &crate::render::svg::Theme) -> String {
    format!(
        r#"
.sankey-node {{
  stroke: none;
}}

.sankey-link {{
  fill-opacity: {link_opacity};
}}

.sankey-label {{
  fill: {label_color};
  font-family: {font_family};
}}

.link {{
  mix-blend-mode: multiply;
}}
"#,
        link_opacity = theme.sankey_link_opacity,
        label_color = theme.sankey_label_color,
        font_family = theme.font_family,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_empty_sankey() {
        let db = SankeyDb::new();
        let config = RenderConfig::default();
        let result = render_sankey(&db, &config);
        assert!(result.is_ok());
        let svg = result.unwrap();
        assert!(svg.contains("<svg"));
    }

    #[test]
    fn test_render_simple_sankey() {
        let mut db = SankeyDb::new();
        db.add_link("A", "B", 10.0);

        let config = RenderConfig::default();
        let result = render_sankey(&db, &config);
        assert!(result.is_ok());
        let svg = result.unwrap();
        assert!(svg.contains("<svg"));
        assert!(svg.contains("class=\"nodes\""));
        assert!(svg.contains("class=\"links\""));
        assert!(svg.contains("class=\"node-labels\""));
    }

    #[test]
    fn test_render_multi_link_sankey() {
        let mut db = SankeyDb::new();
        db.add_link("Source", "Middle", 20.0);
        db.add_link("Middle", "Target", 15.0);
        db.add_link("Source", "Target", 5.0);

        let config = RenderConfig::default();
        let result = render_sankey(&db, &config);
        assert!(result.is_ok());
        let svg = result.unwrap();
        assert!(svg.contains("<svg"));
        // Should have gradient definitions
        assert!(svg.contains("linearGradient"));
    }

    #[test]
    fn test_compute_node_columns() {
        let nodes = vec![
            crate::diagrams::sankey::GraphNode {
                id: "A".to_string(),
            },
            crate::diagrams::sankey::GraphNode {
                id: "B".to_string(),
            },
            crate::diagrams::sankey::GraphNode {
                id: "C".to_string(),
            },
        ];

        let mut outgoing: HashMap<String, Vec<(String, f64)>> = HashMap::new();
        outgoing.insert("A".to_string(), vec![("B".to_string(), 10.0)]);
        outgoing.insert("B".to_string(), vec![("C".to_string(), 10.0)]);

        let mut incoming: HashMap<String, Vec<(String, f64)>> = HashMap::new();
        incoming.insert("B".to_string(), vec![("A".to_string(), 10.0)]);
        incoming.insert("C".to_string(), vec![("B".to_string(), 10.0)]);

        let columns = compute_node_columns(&nodes, &outgoing, &incoming);

        assert_eq!(columns.get("A"), Some(&0));
        assert_eq!(columns.get("B"), Some(&1));
        assert_eq!(columns.get("C"), Some(&2));
    }
}
