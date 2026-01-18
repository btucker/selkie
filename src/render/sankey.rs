//! Sankey diagram renderer
//!
//! Renders Sankey diagrams showing flow between nodes with weighted connections.
//! The layout algorithm follows d3-sankey's approach:
//! 1. Assign nodes to columns based on link depth
//! 2. Calculate node heights proportional to their flow values
//! 3. Position nodes vertically with padding
//! 4. Compute link paths as curved bands between nodes

use std::collections::HashMap;

use crate::diagrams::sankey::SankeyDb;
use crate::error::Result;
use crate::render::svg::{Attrs, RenderConfig, SvgDocument, SvgElement};

/// Default dimensions matching mermaid.js
const DEFAULT_WIDTH: f64 = 600.0;
const DEFAULT_HEIGHT: f64 = 400.0;
const NODE_WIDTH: f64 = 10.0;
/// Node padding: 10 base + 15 for showValues (mermaid default)
const NODE_PADDING: f64 = 25.0;
const LABEL_PADDING: f64 = 6.0;
const FONT_SIZE: f64 = 14.0;

/// Escape special XML characters for use in SVG text content
fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

/// Computed node position and dimensions
#[derive(Debug, Clone)]
struct LayoutNode {
    id: String,
    index: usize, // Original index in input order (for node-N id)
    #[allow(dead_code)]
    column: usize,
    x0: f64,
    y0: f64,
    x1: f64,
    y1: f64,
    #[allow(dead_code)]
    value: f64,
}

/// Computed link with path info
#[derive(Debug, Clone)]
struct LayoutLink {
    source_id: String,
    target_id: String,
    #[allow(dead_code)]
    value: f64,
    width: f64,    // Stroke width for the link
    source_y: f64, // Center Y at source
    target_y: f64, // Center Y at target
    source_x: f64, // X position at source (right edge of node)
    target_x: f64, // X position at target (left edge of node)
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
    let gradients = create_gradient_defs(&layout_nodes, &layout_links, config);
    doc.add_defs(gradients);

    // Render order matches mermaid.js: nodes, labels, then links on top
    // Links use mix-blend-mode: multiply so they blend with background
    let nodes_group = render_nodes(&layout_nodes, config);
    doc.add_element(nodes_group);

    let labels_group = render_labels(&layout_nodes, DEFAULT_WIDTH, config);
    doc.add_element(labels_group);

    let links_group = render_links(&layout_links, config);
    doc.add_element(links_group);

    Ok(doc.to_string())
}

/// Compute the sankey layout following d3-sankey algorithm
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

    // Step 5: Build node index map (for node-N id assignment matching input order)
    let node_indices: HashMap<String, usize> = graph
        .nodes
        .iter()
        .enumerate()
        .map(|(i, n)| (n.id.clone(), i))
        .collect();

    // Step 6: Group nodes by column
    let mut nodes_by_column: Vec<Vec<String>> = vec![Vec::new(); num_columns];
    for node in &graph.nodes {
        let col = node_columns.get(&node.id).copied().unwrap_or(0);
        nodes_by_column[col].push(node.id.clone());
    }

    // Step 6: Calculate ky (scale factor) based on the most constrained column
    // This ensures consistent scaling across all columns
    // Nodes should fill available space (matching mermaid.js d3-sankey behavior)
    let mut ky = f64::MAX;
    for col_nodes in &nodes_by_column {
        if col_nodes.is_empty() {
            continue;
        }
        let total_value: f64 = col_nodes
            .iter()
            .map(|id| node_values.get(id).copied().unwrap_or(0.0))
            .sum();
        let padding_total = NODE_PADDING * (col_nodes.len().saturating_sub(1)) as f64;
        let available = height - padding_total;
        if total_value > 0.0 {
            ky = ky.min(available / total_value);
        }
    }

    // Fallback if no valid ky was computed
    if ky == f64::MAX {
        ky = 1.0;
    }

    // Step 7: Compute initial y positions within each column
    // d3-sankey initializes nodes at y=0 and uses relaxation to spread them
    let mut node_y_positions: HashMap<String, f64> = HashMap::new();
    let mut node_heights: HashMap<String, f64> = HashMap::new();

    for col_nodes in &nodes_by_column {
        let mut current_y = 0.0;

        for node_id in col_nodes {
            let value = node_values.get(node_id).copied().unwrap_or(0.0);
            let node_height = (value * ky).max(1.0);

            node_y_positions.insert(node_id.clone(), current_y);
            node_heights.insert(node_id.clone(), node_height);
            current_y += node_height + NODE_PADDING;
        }
    }

    // Step 8: Apply relaxation to minimize link displacement
    // First forward pass: align nodes with their sources
    for col_nodes in nodes_by_column.iter().skip(1) {
        for node_id in col_nodes {
            let node_height = node_heights.get(node_id).copied().unwrap_or(0.0);

            // Find the source with the largest flow
            if let Some(edges) = incoming.get(node_id) {
                if let Some((primary_source, _)) = edges
                    .iter()
                    .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
                {
                    let source_y = node_y_positions.get(primary_source).copied().unwrap_or(0.0);
                    let source_h = node_heights.get(primary_source).copied().unwrap_or(0.0);

                    // Align center of this node with center of primary source
                    let source_center = source_y + source_h / 2.0;
                    let target_y = source_center - node_height / 2.0;
                    let new_y = target_y.max(0.0).min(height - node_height);
                    node_y_positions.insert(node_id.clone(), new_y);
                }
            }
        }

        resolve_collisions(col_nodes, &mut node_y_positions, &node_heights, height);
    }

    // Backward pass: align nodes with their targets
    for col in (0..num_columns - 1).rev() {
        let col_nodes = &nodes_by_column[col];

        for node_id in col_nodes {
            let node_height = node_heights.get(node_id).copied().unwrap_or(0.0);
            let current_y = node_y_positions.get(node_id).copied().unwrap_or(0.0);

            // Find weighted center of all targets
            if let Some(edges) = outgoing.get(node_id) {
                let mut total_weight = 0.0;
                let mut weighted_sum = 0.0;

                for (target_id, value) in edges {
                    let target_y = node_y_positions.get(target_id).copied().unwrap_or(0.0);
                    let target_h = node_heights.get(target_id).copied().unwrap_or(0.0);
                    let target_center = target_y + target_h / 2.0;
                    total_weight += value;
                    weighted_sum += target_center * value;
                }

                if total_weight > 0.0 {
                    let avg_target_center = weighted_sum / total_weight;
                    let my_center = current_y + node_height / 2.0;

                    // Move halfway toward the target center (relaxation)
                    let new_center = my_center + (avg_target_center - my_center) * 0.5;
                    let new_y = (new_center - node_height / 2.0)
                        .max(0.0)
                        .min(height - node_height);
                    node_y_positions.insert(node_id.clone(), new_y);
                }
            }
        }

        resolve_collisions(col_nodes, &mut node_y_positions, &node_heights, height);
    }

    // Step 9: Build final layout nodes
    let mut layout_nodes: Vec<LayoutNode> = Vec::new();
    let mut node_positions: HashMap<String, (f64, f64, f64, f64)> = HashMap::new();

    for (col, col_nodes) in nodes_by_column.iter().enumerate() {
        let x0 = col as f64 * column_width;
        let x1 = x0 + NODE_WIDTH;

        for node_id in col_nodes {
            let y0 = node_y_positions.get(node_id).copied().unwrap_or(0.0);
            let node_height = node_heights.get(node_id).copied().unwrap_or(0.0);
            let y1 = y0 + node_height;
            let value = node_values.get(node_id).copied().unwrap_or(0.0);
            let index = node_indices.get(node_id).copied().unwrap_or(0);

            layout_nodes.push(LayoutNode {
                id: node_id.clone(),
                index,
                column: col,
                x0,
                y0,
                x1,
                y1,
                value,
            });

            node_positions.insert(node_id.clone(), (x0, y0, x1, y1));
        }
    }

    // Step 10: Compute link positions (as strokes with width)
    let layout_links = compute_link_positions(&graph.links, &node_positions, ky);

    (layout_nodes, layout_links)
}

/// Compute node columns using topological sort from sources
/// Implements d3-sankey "justify" alignment: sink nodes (no outgoing edges)
/// are pushed to the rightmost column.
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

    // BFS from sources to find longest path to each node
    let mut queue: Vec<(String, usize)> = source_nodes.iter().map(|id| (id.clone(), 0)).collect();

    while let Some((node_id, col)) = queue.pop() {
        // Update column to max seen (longest path)
        let current_col = columns.entry(node_id.clone()).or_insert(0);
        if col > *current_col {
            *current_col = col;
        }

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

    // Apply "justify" alignment: push sink nodes (no outgoing edges) to rightmost column
    // This matches d3-sankey's sankeyJustify behavior
    let max_column = columns.values().copied().max().unwrap_or(0);
    for node in nodes {
        let has_outgoing = outgoing
            .get(&node.id)
            .map(|edges| !edges.is_empty())
            .unwrap_or(false);
        if !has_outgoing {
            // Sink node: push to rightmost column
            columns.insert(node.id.clone(), max_column);
        }
    }

    columns
}

/// Resolve vertical collisions within a column by pushing overlapping nodes apart
fn resolve_collisions(
    col_nodes: &[String],
    node_y_positions: &mut HashMap<String, f64>,
    node_heights: &HashMap<String, f64>,
    height: f64,
) {
    if col_nodes.is_empty() {
        return;
    }

    // Sort nodes by y position
    let mut sorted_nodes: Vec<_> = col_nodes.iter().collect();
    sorted_nodes.sort_by(|a, b| {
        let ya = node_y_positions.get(*a).copied().unwrap_or(0.0);
        let yb = node_y_positions.get(*b).copied().unwrap_or(0.0);
        ya.partial_cmp(&yb).unwrap_or(std::cmp::Ordering::Equal)
    });

    // Push nodes down to resolve overlaps
    let mut y = 0.0;
    for node_id in &sorted_nodes {
        let current_y = node_y_positions.get(*node_id).copied().unwrap_or(0.0);
        let node_height = node_heights.get(*node_id).copied().unwrap_or(0.0);

        if current_y < y {
            node_y_positions.insert((*node_id).clone(), y);
        } else {
            y = current_y;
        }
        y += node_height + NODE_PADDING;
    }

    // If we exceeded the height, push nodes back up
    let last_node = sorted_nodes.last().unwrap();
    let last_y = node_y_positions.get(*last_node).copied().unwrap_or(0.0);
    let last_height = node_heights.get(*last_node).copied().unwrap_or(0.0);

    let overflow = last_y + last_height - height;
    if overflow > 0.0 {
        // Push all nodes up proportionally
        for node_id in sorted_nodes.iter().rev() {
            let current_y = node_y_positions.get(*node_id).copied().unwrap_or(0.0);
            let new_y = (current_y - overflow).max(0.0);
            node_y_positions.insert((*node_id).clone(), new_y);
        }
    }
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

    // Sum incoming and outgoing values, take max (d3-sankey approach)
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

/// Compute link positions - links are drawn as strokes with width
fn compute_link_positions(
    links: &[crate::diagrams::sankey::GraphLink],
    node_positions: &HashMap<String, (f64, f64, f64, f64)>,
    ky: f64,
) -> Vec<LayoutLink> {
    // Track current y offset at each node for stacking links
    let mut source_offsets: HashMap<String, f64> = HashMap::new();
    let mut target_offsets: HashMap<String, f64> = HashMap::new();

    let mut layout_links = Vec::new();

    for link in links {
        let (_source_x0, source_y0, source_x1, _source_y1) = node_positions
            .get(&link.source)
            .copied()
            .unwrap_or((0.0, 0.0, NODE_WIDTH, 10.0));

        let (target_x0, target_y0, _target_x1, _target_y1) = node_positions
            .get(&link.target)
            .copied()
            .unwrap_or((0.0, 0.0, NODE_WIDTH, 10.0));

        // Link width is proportional to value
        let link_width = (link.value * ky).max(1.0);

        // Get current offset at source and target
        let source_offset = source_offsets.entry(link.source.clone()).or_insert(0.0);
        let target_offset = target_offsets.entry(link.target.clone()).or_insert(0.0);

        // Center Y positions for the link at source and target
        let source_y = source_y0 + *source_offset + link_width / 2.0;
        let target_y = target_y0 + *target_offset + link_width / 2.0;

        // Update offsets for next link
        *source_offset += link_width;
        *target_offset += link_width;

        layout_links.push(LayoutLink {
            source_id: link.source.clone(),
            target_id: link.target.clone(),
            value: link.value,
            width: link_width,
            source_y,
            target_y,
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
) -> Vec<SvgElement> {
    let colors = &config.theme.sankey_node_colors;
    let node_colors: HashMap<_, _> = nodes
        .iter()
        .map(|n| (n.id.clone(), colors[n.index % colors.len()].as_str()))
        .collect();

    let mut gradients = Vec::new();

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

        let gradient = SvgElement::Raw {
            content: format!(
                "<linearGradient id=\"{}\" gradientUnits=\"userSpaceOnUse\" x1=\"{}\" x2=\"{}\">\
                 <stop offset=\"0%\" stop-color=\"{}\"/>\
                 <stop offset=\"100%\" stop-color=\"{}\"/>\
                 </linearGradient>",
                gradient_id, link.source_x, link.target_x, source_color, target_color
            ),
        };

        gradients.push(gradient);
    }

    gradients
}

/// Render all links as strokes (matching mermaid.js d3SankeyLinkHorizontal)
fn render_links(links: &[LayoutLink], config: &RenderConfig) -> SvgElement {
    let mut children = Vec::new();

    for (i, link) in links.iter().enumerate() {
        // d3SankeyLinkHorizontal generates a cubic bezier curve
        // M source_x,source_y C mid_x,source_y mid_x,target_y target_x,target_y
        let mid_x = (link.source_x + link.target_x) / 2.0;

        let d = format!(
            "M{},{} C{},{} {},{} {},{}",
            link.source_x,
            link.source_y,
            mid_x,
            link.source_y,
            mid_x,
            link.target_y,
            link.target_x,
            link.target_y,
        );

        let gradient_id = format!("url(#linearGradient-{})", i + 1);

        // Path element matches mermaid.js reference - no class, no redundant attrs
        // Parent groups handle fill="none" and stroke-opacity
        let link_path = SvgElement::Path {
            d,
            attrs: Attrs::new()
                .with_stroke(&gradient_id)
                .with_stroke_width(link.width),
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
        attrs: Attrs::new()
            .with_class("links")
            .with_fill("none")
            .with_attr("stroke-opacity", &config.theme.sankey_link_opacity),
    }
}

/// Render all nodes
fn render_nodes(nodes: &[LayoutNode], config: &RenderConfig) -> SvgElement {
    let mut children = Vec::new();
    let colors = &config.theme.sankey_node_colors;

    for node in nodes.iter() {
        let color = &colors[node.index % colors.len()];

        // Rect uses local coordinates (0,0) since group has the transform
        let rect = SvgElement::Rect {
            x: 0.0,
            y: 0.0,
            width: node.x1 - node.x0,
            height: node.y1 - node.y0,
            rx: None,
            ry: None,
            attrs: Attrs::new().with_fill(color).with_class("sankey-node"),
        };

        // Group has transform for positioning, x/y are data attributes for tests
        let node_group = SvgElement::Group {
            children: vec![rect],
            attrs: Attrs::new()
                .with_class("node")
                .with_id(&format!("node-{}", node.index + 1))
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

/// Render node labels with values (showValues=true is mermaid default)
fn render_labels(nodes: &[LayoutNode], width: f64, _config: &RenderConfig) -> SvgElement {
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

        // Format value: round to 2 decimal places, remove trailing zeros
        let value_str = format_value(node.value);

        // Label content: "node_id\nvalue" (matches mermaid showValues=true)
        // Use raw SVG with actual newline (matching mermaid.js output)
        // dy="0em" when showing values (multiline text)
        // Escape node.id for XML safety (handles &, <, >, etc.)
        let label = SvgElement::Raw {
            content: format!(
                "<text x=\"{}\" y=\"{}\" dy=\"0em\" text-anchor=\"{}\">{}\n{}</text>",
                label_x,
                label_y,
                text_anchor,
                escape_xml(&node.id),
                value_str
            ),
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

/// Format a value for display: round to 2 decimal places, remove trailing zeros
fn format_value(value: f64) -> String {
    let rounded = (value * 100.0).round() / 100.0;
    if rounded == rounded.trunc() {
        // Integer value
        format!("{}", rounded as i64)
    } else {
        // Decimal value - remove trailing zeros
        let s = format!("{:.2}", rounded);
        s.trim_end_matches('0').trim_end_matches('.').to_string()
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
  stroke-opacity: {link_opacity};
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

    #[test]
    fn test_node_heights_fill_available_space() {
        // Test that nodes fill available space (matching mermaid.js d3-sankey behavior)
        let mut db = SankeyDb::new();
        db.add_link("A", "B", 10.0);

        let (layout_nodes, _) = compute_layout(&db, DEFAULT_WIDTH, DEFAULT_HEIGHT);

        // With single column per side, nodes should fill the height
        // Both nodes have the same value (10), so they should have equal height
        assert_eq!(layout_nodes.len(), 2);
        let node_a = &layout_nodes[0];
        let node_b = &layout_nodes[1];

        // Both nodes should have the same height (full available space)
        let height_a = node_a.y1 - node_a.y0;
        let height_b = node_b.y1 - node_b.y0;
        assert!(
            (height_a - height_b).abs() < 1.0,
            "Node heights should be equal: A={}, B={}",
            height_a,
            height_b
        );

        // Nodes should fill available height (400px)
        assert!(
            height_a > DEFAULT_HEIGHT * 0.9,
            "Node should fill available height: {} > {}",
            height_a,
            DEFAULT_HEIGHT * 0.9
        );
    }

    #[test]
    fn test_links_are_strokes() {
        // Test that links use stroke, not fill
        let mut db = SankeyDb::new();
        db.add_link("A", "B", 10.0);

        let config = RenderConfig::default();
        let result = render_sankey(&db, &config).unwrap();

        // Links should have stroke-width attribute
        assert!(result.contains("stroke-width="));
        // Links should have fill="none"
        assert!(result.contains("fill=\"none\""));
    }
}
