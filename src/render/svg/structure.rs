//! SVG structure analysis for comparison testing
//!
//! This module provides tools to analyze SVG documents and extract
//! structural information for comparison between different renderers.

use serde::{Deserialize, Serialize};

/// Structural analysis of an SVG document
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SvgStructure {
    /// Width of the SVG (from viewBox or width attribute)
    pub width: f64,
    /// Height of the SVG (from viewBox or height attribute)
    pub height: f64,
    /// Number of node elements detected
    pub node_count: usize,
    /// Number of edge elements detected
    pub edge_count: usize,
    /// Text labels found in the SVG
    pub labels: Vec<String>,
    /// Count of each shape type
    pub shapes: ShapeCounts,
    /// Number of marker definitions
    pub marker_count: usize,
    /// Whether the SVG has a defs section
    pub has_defs: bool,
    /// Whether the SVG has embedded styles
    pub has_style: bool,
    /// Z-order analysis: tracks element rendering order
    pub z_order: ZOrderAnalysis,
    /// Stroke width analysis: tracks stroke-width values on key elements
    pub stroke_analysis: StrokeAnalysis,
    /// Edge geometry analysis: tracks edge endpoint positions
    pub edge_geometry: EdgeGeometry,
}

/// Analysis of SVG element rendering order (z-order)
/// In SVG, later elements are drawn on top of earlier ones
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct ZOrderAnalysis {
    /// Text elements that appear before shapes in the same group (potentially obscured)
    pub text_before_shapes: usize,
    /// Text elements that appear after shapes in the same group (correct order)
    pub text_after_shapes: usize,
    /// Labels that may be obscured (text rendered before overlapping shapes)
    pub potentially_obscured_labels: Vec<String>,
    /// Element order summary: list of (element_type, count) in render order
    pub element_order: Vec<(String, usize)>,
}

/// Counts of different SVG shape elements
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct ShapeCounts {
    pub rect: usize,
    pub circle: usize,
    pub ellipse: usize,
    pub polygon: usize,
    pub path: usize,
    pub line: usize,
    pub polyline: usize,
}

/// Analysis of stroke-width values across the SVG
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct StrokeAnalysis {
    /// Stroke widths found on rect elements (typically entity/node borders)
    pub rect_stroke_widths: Vec<f64>,
    /// Stroke widths found on path elements (typically edges/lines)
    pub path_stroke_widths: Vec<f64>,
    /// Stroke widths found on line elements
    pub line_stroke_widths: Vec<f64>,
    /// Average stroke width on rects (0 if none)
    pub avg_rect_stroke: f64,
    /// Average stroke width on paths (0 if none)
    pub avg_path_stroke: f64,
}

/// Analysis of edge/path geometry
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct EdgeGeometry {
    /// Edge endpoints: list of (start_x, start_y, end_x, end_y)
    pub edge_endpoints: Vec<(f64, f64, f64, f64)>,
    /// Node bounding boxes: list of (x, y, width, height, id/class)
    pub node_bounds: Vec<NodeBounds>,
    /// Edges that attach to top/bottom of nodes (vertical attachment)
    pub vertical_attachments: usize,
    /// Edges that attach to left/right of nodes (horizontal attachment)
    pub horizontal_attachments: usize,
}

/// Bounding box of a node element
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct NodeBounds {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub id: String,
}

impl SvgStructure {
    /// Parse an SVG string and extract its structure
    pub fn from_svg(svg: &str) -> Result<Self, String> {
        let doc =
            roxmltree::Document::parse(svg).map_err(|e| format!("Failed to parse SVG: {}", e))?;

        let root = doc.root_element();
        if root.tag_name().name() != "svg" {
            return Err("Root element is not <svg>".to_string());
        }

        // Parse dimensions
        let (width, height) = parse_dimensions(&root);

        // Count shapes
        let shapes = count_shapes(&doc);

        // Count nodes and edges (elements with specific classes)
        let (node_count, edge_count) = count_nodes_and_edges(&doc);

        // Extract labels
        let labels = extract_labels(&doc);

        // Count markers
        let marker_count = count_elements(&doc, "marker");

        // Check for defs and style
        let has_defs = doc.descendants().any(|n| n.tag_name().name() == "defs");
        let has_style = doc.descendants().any(|n| n.tag_name().name() == "style");

        // Analyze z-order (element rendering order)
        let z_order = analyze_z_order(&doc);

        // Analyze stroke widths
        let stroke_analysis = analyze_stroke_widths(&doc);

        // Analyze edge geometry
        let edge_geometry = analyze_edge_geometry(&doc);

        Ok(SvgStructure {
            width,
            height,
            node_count,
            edge_count,
            labels,
            shapes,
            marker_count,
            has_defs,
            has_style,
            z_order,
            stroke_analysis,
            edge_geometry,
        })
    }
}

// Helper functions

fn parse_dimensions(root: &roxmltree::Node) -> (f64, f64) {
    // Try viewBox first
    if let Some(viewbox) = root.attribute("viewBox") {
        let parts: Vec<f64> = viewbox
            .split_whitespace()
            .filter_map(|s| s.parse().ok())
            .collect();
        if parts.len() >= 4 {
            return (parts[2], parts[3]);
        }
    }

    // Fall back to width/height attributes
    let width = root
        .attribute("width")
        .and_then(|s| s.trim_end_matches("px").parse().ok())
        .unwrap_or(0.0);
    let height = root
        .attribute("height")
        .and_then(|s| s.trim_end_matches("px").parse().ok())
        .unwrap_or(0.0);

    (width, height)
}

fn count_shapes(doc: &roxmltree::Document) -> ShapeCounts {
    ShapeCounts {
        rect: count_visible_rects(doc),
        circle: count_elements(doc, "circle"),
        ellipse: count_elements(doc, "ellipse"),
        polygon: count_elements(doc, "polygon"),
        path: count_visible_paths(doc),
        line: count_elements(doc, "line"),
        polyline: count_elements(doc, "polyline"),
    }
}

/// Count only visible rects (those with width and height > 0)
/// This excludes helper/placeholder rects used by mermaid.js for sizing
/// and edge label background rects (class="edge-label-bg")
fn count_visible_rects(doc: &roxmltree::Document) -> usize {
    doc.descendants()
        .filter(|n| n.tag_name().name() == "rect")
        .filter(|n| {
            // Exclude edge label backgrounds (not structural elements)
            let class = n.attribute("class").unwrap_or("");
            if class.contains("edge-label-bg") {
                return false;
            }

            // Check if rect has non-zero dimensions
            let width = n
                .attribute("width")
                .and_then(|s| s.parse::<f64>().ok())
                .unwrap_or(0.0);
            let height = n
                .attribute("height")
                .and_then(|s| s.parse::<f64>().ok())
                .unwrap_or(0.0);
            width > 0.0 && height > 0.0
        })
        .count()
}

fn count_elements(doc: &roxmltree::Document, tag: &str) -> usize {
    doc.descendants()
        .filter(|n| n.tag_name().name() == tag)
        .count()
}

fn count_visible_paths(doc: &roxmltree::Document) -> usize {
    doc.descendants()
        .filter(|n| n.tag_name().name() == "path")
        .filter(|n| {
            let stroke = n.attribute("stroke");
            if stroke == Some("none") {
                return false;
            }

            if let Some(width) = n.attribute("stroke-width") {
                if width.parse::<f64>().ok() == Some(0.0) {
                    return false;
                }
            }

            true
        })
        .count()
}

fn count_nodes_and_edges(doc: &roxmltree::Document) -> (usize, usize) {
    let mut node_count = 0;
    let mut edge_count = 0;

    // Node class patterns used by different diagram types in selkie and mermaid.js
    const NODE_CLASSES: &[&str] = &[
        "node",           // flowchart (selkie)
        "flowchart-node", // flowchart (mermaid.js)
        "class-node",     // class diagram (selkie)
        "state-node",     // state diagram (selkie)
        "entity-node",    // ER diagram (selkie)
        "architecture-service",
        "architecture-junction",
    ];

    // Edge class patterns used by different diagram types
    const EDGE_CLASSES: &[&str] = &[
        "edge",         // flowchart (selkie)
        "relation",     // class diagram (selkie)
        "transition",   // state diagram (selkie)
        "relationship", // ER diagram (selkie)
    ];

    for node in doc.descendants() {
        // Check for data-edge attribute (mermaid.js uses this)
        if node.attribute("data-edge").is_some() {
            edge_count += 1;
            continue;
        }

        if let Some(class) = node.attribute("class") {
            let classes: Vec<&str> = class.split_whitespace().collect();

            // Count nodes - elements with any node class pattern
            if classes.iter().any(|c| NODE_CLASSES.contains(c)) {
                node_count += 1;
            }

            // Count edges - handle group containers and architecture edge paths
            // mermaid.js uses "flowchart-link" on <path> elements with data-edge
            // (handled above with data-edge attribute check)
            if classes.iter().any(|c| EDGE_CLASSES.contains(c)) {
                let tag = node.tag_name().name();
                if tag == "g" || tag == "path" {
                    edge_count += 1;
                }
            }
        }
    }

    (node_count, edge_count)
}

fn extract_labels(doc: &roxmltree::Document) -> Vec<String> {
    let mut labels = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for node in doc.descendants() {
        let tag = node.tag_name().name();

        // For text elements, check if they have tspan children
        if tag == "text" {
            let tspans: Vec<_> = node
                .children()
                .filter(|c| c.tag_name().name() == "tspan")
                .collect();

            // Check if this is multi-line text (tspans with dy attribute)
            // vs multi-word single-line text (tspans without dy)
            let is_multiline =
                tspans.len() > 1 && tspans.iter().skip(1).any(|t| t.attribute("dy").is_some());

            if is_multiline {
                // Multi-line text: capture only the first line, matching HTML <p> extraction.
                if let Some(first) = tspans.first() {
                    let text: String = first
                        .text()
                        .unwrap_or("")
                        .split_whitespace()
                        .collect::<Vec<_>>()
                        .join(" ");
                    if !text.is_empty() && !seen.contains(&text) {
                        seen.insert(text.clone());
                        labels.push(text);
                    }
                }
            } else {
                // Single-line or multi-word: get combined content
                let combined = collect_text_content(&node);
                // Normalize whitespace: collapse multiple spaces/newlines into single space
                let combined: String = combined.split_whitespace().collect::<Vec<_>>().join(" ");
                if !combined.is_empty() && !seen.contains(&combined) {
                    seen.insert(combined.clone());
                    labels.push(combined);
                }
            }
        }
        // For tspan directly under text, handled above
        // For p/span (mermaid.js foreignObject HTML), get direct text content
        else if tag == "p" || tag == "span" {
            // Only get direct text, not combined content, to avoid duplicates
            if let Some(text) = node.text() {
                let text = text.trim();
                if !text.is_empty() && !seen.contains(text) {
                    seen.insert(text.to_string());
                    labels.push(text.to_string());
                }
            }
        }
    }

    labels.sort();
    labels
}

/// Recursively collect all text content from a node and its descendants
fn collect_text_content(node: &roxmltree::Node) -> String {
    let mut result = String::new();

    for child in node.children() {
        if child.is_text() {
            if let Some(text) = child.text() {
                result.push_str(text);
            }
        } else {
            result.push_str(&collect_text_content(&child));
        }
    }

    result
}

/// Analyze z-order (rendering order) of SVG elements
/// In SVG, later elements are rendered on top of earlier ones
fn analyze_z_order(doc: &roxmltree::Document) -> ZOrderAnalysis {
    let mut analysis = ZOrderAnalysis::default();
    let mut element_counts: std::collections::HashMap<String, usize> =
        std::collections::HashMap::new();

    // Shape element types that could obscure text
    const SHAPE_TAGS: &[&str] = &[
        "rect", "circle", "ellipse", "polygon", "path", "line", "polyline",
    ];
    const TEXT_TAGS: &[&str] = &["text", "tspan", "foreignObject"];

    // Analyze each group (g element) for text/shape ordering
    for group in doc.descendants().filter(|n| n.tag_name().name() == "g") {
        let mut last_shape_index: Option<usize> = None;
        let mut last_text_index: Option<usize> = None;

        for (i, child) in group.children().enumerate() {
            let tag = child.tag_name().name();

            if SHAPE_TAGS.contains(&tag) {
                last_shape_index = Some(i);

                // If text was rendered before this shape, it might be obscured
                if let Some(text_idx) = last_text_index {
                    if text_idx < i {
                        analysis.text_before_shapes += 1;
                        // Try to extract the label that might be obscured
                        if let Some(text_node) = group.children().nth(text_idx) {
                            let label = collect_text_content(&text_node)
                                .split_whitespace()
                                .collect::<Vec<_>>()
                                .join(" ");
                            if !label.is_empty()
                                && !analysis.potentially_obscured_labels.contains(&label)
                            {
                                analysis.potentially_obscured_labels.push(label);
                            }
                        }
                    }
                }
            }

            if TEXT_TAGS.contains(&tag) {
                last_text_index = Some(i);

                // Check if text comes after shapes (correct order)
                if last_shape_index.is_some() {
                    analysis.text_after_shapes += 1;
                }
            }
        }
    }

    // Build element order summary (top-level elements in the main SVG)
    for node in doc.root_element().children() {
        let tag = node.tag_name().name();
        if !tag.is_empty() {
            *element_counts.entry(tag.to_string()).or_insert(0) += 1;
        }
    }

    // Convert to ordered list
    let mut order: Vec<_> = element_counts.into_iter().collect();
    order.sort_by(|a, b| a.0.cmp(&b.0));
    analysis.element_order = order;

    analysis
}

/// Analyze stroke-width values across the SVG
/// Extracts from both inline attributes and CSS <style> blocks
fn analyze_stroke_widths(doc: &roxmltree::Document) -> StrokeAnalysis {
    let mut analysis = StrokeAnalysis::default();

    // First, extract stroke-width values from CSS <style> blocks
    let css_stroke_widths = extract_css_stroke_widths(doc);

    for node in doc.descendants() {
        let tag = node.tag_name().name();

        // Get stroke-width from inline attribute
        let inline_stroke_width = node
            .attribute("stroke-width")
            .and_then(|s| s.parse::<f64>().ok());

        // Get stroke-width from CSS class or element type selector
        let class = node.attribute("class").unwrap_or("");
        let css_stroke_width = class
            .split_whitespace()
            .find_map(|c| css_stroke_widths.get(c).copied())
            .or_else(|| {
                css_stroke_widths
                    .get(&format!("__element_{}", tag))
                    .copied()
            });

        // Use inline if present, otherwise CSS, otherwise check if has stroke
        let stroke_width = inline_stroke_width.or(css_stroke_width);

        // Only count if element has a visible stroke
        let has_stroke = node
            .attribute("stroke")
            .map(|s| s != "none")
            .unwrap_or(false)
            || stroke_width.is_some()
            || class
                .split_whitespace()
                .any(|c| css_stroke_widths.contains_key(c));

        if !has_stroke {
            continue;
        }

        let width = stroke_width.unwrap_or(1.0);

        match tag {
            "rect" => analysis.rect_stroke_widths.push(width),
            "path" => analysis.path_stroke_widths.push(width),
            "line" => analysis.line_stroke_widths.push(width),
            _ => {}
        }
    }

    // Calculate averages
    if !analysis.rect_stroke_widths.is_empty() {
        analysis.avg_rect_stroke = analysis.rect_stroke_widths.iter().sum::<f64>()
            / analysis.rect_stroke_widths.len() as f64;
    }
    if !analysis.path_stroke_widths.is_empty() {
        analysis.avg_path_stroke = analysis.path_stroke_widths.iter().sum::<f64>()
            / analysis.path_stroke_widths.len() as f64;
    }

    analysis
}

/// Extract stroke-width values from CSS <style> blocks
/// Returns a map of selector component -> stroke-width value
#[cfg(feature = "eval")]
fn extract_css_stroke_widths(doc: &roxmltree::Document) -> std::collections::HashMap<String, f64> {
    use simplecss::StyleSheet;

    let mut css_strokes = std::collections::HashMap::new();

    for node in doc.descendants() {
        if node.tag_name().name() == "style" {
            if let Some(css_text) = node.text() {
                // Parse CSS using simplecss
                let stylesheet = StyleSheet::parse(css_text);

                for rule in stylesheet.rules {
                    // Check if this rule has a stroke-width declaration
                    let mut stroke_width: Option<f64> = None;

                    for decl in &rule.declarations {
                        if decl.name == "stroke-width" {
                            // Parse value, stripping 'px' suffix if present
                            let value = decl.value.trim().trim_end_matches("px");
                            if let Ok(width) = value.parse::<f64>() {
                                stroke_width = Some(width);
                            }
                        }
                    }

                    // If we found a stroke-width, associate it with selector components
                    if let Some(width) = stroke_width {
                        let selector_str = rule.selector.to_string();

                        // Extract class names from selector
                        for part in selector_str.split(&[' ', ',', '>', '+', '~'][..]) {
                            let part = part.trim();
                            if part.starts_with('.') {
                                let class = part.trim_start_matches('.');
                                css_strokes.insert(class.to_string(), width);
                            }
                            // Also track element type selectors
                            match part {
                                "rect" | "path" | "line" | "circle" | "ellipse" => {
                                    css_strokes.insert(format!("__element_{}", part), width);
                                }
                                _ => {}
                            }
                        }
                    }
                }
            }
        }
    }

    css_strokes
}

/// Fallback when eval feature is disabled - returns empty map
#[cfg(not(feature = "eval"))]
fn extract_css_stroke_widths(_doc: &roxmltree::Document) -> std::collections::HashMap<String, f64> {
    std::collections::HashMap::new()
}

/// Analyze edge geometry - endpoints and attachment points
fn analyze_edge_geometry(doc: &roxmltree::Document) -> EdgeGeometry {
    let mut geometry = EdgeGeometry::default();

    // Collect node bounding boxes from rects with node-related classes
    for node in doc.descendants() {
        if node.tag_name().name() == "rect" {
            let class = node.attribute("class").unwrap_or("");
            // Look for entity boxes, node boxes, etc.
            if class.contains("entity-box")
                || class.contains("node")
                || class.contains("actor")
                || class.contains("label-container")
            {
                let x = node
                    .attribute("x")
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0.0);
                let y = node
                    .attribute("y")
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0.0);
                let width = node
                    .attribute("width")
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0.0);
                let height = node
                    .attribute("height")
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0.0);
                let id = node.attribute("id").unwrap_or("").to_string();

                if width > 0.0 && height > 0.0 {
                    geometry.node_bounds.push(NodeBounds {
                        x,
                        y,
                        width,
                        height,
                        id,
                    });
                }
            }
        }
    }

    // Collect edge endpoints from paths
    for node in doc.descendants() {
        if node.tag_name().name() == "path" {
            let class = node.attribute("class").unwrap_or("");
            // Look for relationship/edge paths
            if class.contains("relationship")
                || class.contains("edge")
                || class.contains("link")
                || class.contains("transition")
            {
                if let Some(d) = node.attribute("d") {
                    if let Some((start, end)) = parse_path_endpoints(d) {
                        geometry
                            .edge_endpoints
                            .push((start.0, start.1, end.0, end.1));

                        // Determine attachment type based on node positions
                        for bounds in &geometry.node_bounds {
                            let (attach_type_start, _) = classify_attachment(start, bounds);
                            let (attach_type_end, _) = classify_attachment(end, bounds);

                            if attach_type_start == AttachmentType::Vertical
                                || attach_type_end == AttachmentType::Vertical
                            {
                                geometry.vertical_attachments += 1;
                            }
                            if attach_type_start == AttachmentType::Horizontal
                                || attach_type_end == AttachmentType::Horizontal
                            {
                                geometry.horizontal_attachments += 1;
                            }
                        }
                    }
                }
            }
        }
    }

    geometry
}

#[derive(Debug, PartialEq)]
enum AttachmentType {
    Vertical,   // top or bottom
    Horizontal, // left or right
    None,
}

/// Classify how a point attaches to a node bounds
fn classify_attachment(point: (f64, f64), bounds: &NodeBounds) -> (AttachmentType, f64) {
    let (px, py) = point;
    let tolerance = 5.0; // pixels

    let left = bounds.x;
    let right = bounds.x + bounds.width;
    let top = bounds.y;
    let bottom = bounds.y + bounds.height;

    // Check if point is near the node
    let near_left = (px - left).abs() < tolerance;
    let near_right = (px - right).abs() < tolerance;
    let near_top = (py - top).abs() < tolerance;
    let near_bottom = (py - bottom).abs() < tolerance;

    let within_x = px >= left - tolerance && px <= right + tolerance;
    let within_y = py >= top - tolerance && py <= bottom + tolerance;

    // Vertical attachment (top or bottom edge)
    if (near_top || near_bottom) && within_x {
        let dist = if near_top {
            (py - top).abs()
        } else {
            (py - bottom).abs()
        };
        return (AttachmentType::Vertical, dist);
    }

    // Horizontal attachment (left or right edge)
    if (near_left || near_right) && within_y {
        let dist = if near_left {
            (px - left).abs()
        } else {
            (px - right).abs()
        };
        return (AttachmentType::Horizontal, dist);
    }

    (AttachmentType::None, f64::MAX)
}

/// Parse start and end points from an SVG path d attribute
fn parse_path_endpoints(d: &str) -> Option<((f64, f64), (f64, f64))> {
    let parts: Vec<&str> = d.split_whitespace().collect();
    if parts.is_empty() {
        return None;
    }

    let mut start: Option<(f64, f64)> = None;
    let mut end: Option<(f64, f64)> = None;
    let mut i = 0;

    while i < parts.len() {
        let part = parts[i];

        // Handle M (moveto) command - sets start point
        if part == "M" || part.starts_with('M') {
            let (x, y) = if part == "M" {
                i += 1;
                parse_coord_pair(&parts, &mut i)?
            } else {
                // M followed directly by coords like "M10,20"
                parse_inline_coords(&part[1..])?
            };
            if start.is_none() {
                start = Some((x, y));
            }
            end = Some((x, y));
        }
        // Handle L (lineto) command
        else if part == "L" || part.starts_with('L') {
            let (x, y) = if part == "L" {
                i += 1;
                parse_coord_pair(&parts, &mut i)?
            } else {
                parse_inline_coords(&part[1..])?
            };
            end = Some((x, y));
        }
        // Handle C (curveto) command - takes 3 coordinate pairs
        else if part == "C" || part.starts_with('C') {
            if part == "C" {
                i += 1;
                // Skip first two control points
                parse_coord_pair(&parts, &mut i)?;
                parse_coord_pair(&parts, &mut i)?;
                // Third point is the endpoint
                let (x, y) = parse_coord_pair(&parts, &mut i)?;
                end = Some((x, y));
            } else {
                // Inline coords after C
                let coords_str = &part[1..];
                let coords: Vec<f64> = coords_str
                    .split([',', ' '])
                    .filter_map(|s| s.parse().ok())
                    .collect();
                if coords.len() >= 6 {
                    end = Some((coords[4], coords[5]));
                }
            }
        }
        // Handle numbers that might be continuation of previous command
        else if let Some((x, y)) = parse_inline_coords(part) {
            end = Some((x, y));
        }

        i += 1;
    }

    match (start, end) {
        (Some(s), Some(e)) => Some((s, e)),
        _ => None,
    }
}

fn parse_coord_pair(parts: &[&str], i: &mut usize) -> Option<(f64, f64)> {
    if *i >= parts.len() {
        return None;
    }

    let part = parts[*i];

    // Try to parse as "x,y" or "x y"
    if let Some((x, y)) = parse_inline_coords(part) {
        return Some((x, y));
    }

    // Try separate x and y values
    let x: f64 = part.parse().ok()?;
    *i += 1;
    if *i >= parts.len() {
        return None;
    }
    let y: f64 = parts[*i].trim_start_matches(',').parse().ok()?;
    Some((x, y))
}

fn parse_inline_coords(s: &str) -> Option<(f64, f64)> {
    let parts: Vec<&str> = s.split(',').collect();
    if parts.len() == 2 {
        let x: f64 = parts[0].parse().ok()?;
        let y: f64 = parts[1].parse().ok()?;
        return Some((x, y));
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_labels_combines_tspans() {
        // Mermaid.js splits multi-word text into separate tspan elements
        let mermaid_style_svg = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 200 100">
            <text>
                <tspan>Main</tspan>
                <tspan> Flow</tspan>
            </text>
        </svg>"#;

        let structure = SvgStructure::from_svg(mermaid_style_svg).unwrap();

        // Should extract "Main Flow" as a single label, not ["Main", " Flow"]
        assert!(
            structure.labels.contains(&"Main Flow".to_string()),
            "Should combine tspans into single label. Got: {:?}",
            structure.labels
        );
        assert!(
            !structure.labels.iter().any(|l| l == "Main" || l == " Flow"),
            "Should not have separate tspan fragments. Got: {:?}",
            structure.labels
        );
    }

    #[test]
    fn test_extract_multiline_tspans_uses_first_line() {
        // Multi-line text uses dy attribute to position lines
        let multiline_svg = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 200 100">
            <text x="10" y="20">
                <tspan x="10" y="20">Line one</tspan>
                <tspan x="10" dy="1.2em">Line two</tspan>
                <tspan x="10" dy="1.2em">Line three</tspan>
            </text>
        </svg>"#;

        let structure = SvgStructure::from_svg(multiline_svg).unwrap();

        // Should use only the first line
        assert!(
            structure.labels.contains(&"Line one".to_string()),
            "Should extract first line only. Got: {:?}",
            structure.labels
        );
    }

    #[test]
    fn test_count_visible_rects_only() {
        // Mermaid.js style SVG with helper rects (empty rects inside labels)
        let mermaid_style_svg = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 200 100">
            <g class="nodes">
                <g class="node">
                    <rect class="label-container" x="10" y="10" width="80" height="40"/>
                    <g class="label">
                        <rect></rect>
                        <text>Label</text>
                    </g>
                </g>
            </g>
            <g class="edgeLabels">
                <g><rect class="background" style="stroke: none"></rect></g>
            </g>
        </svg>"#;

        // Our clean SVG with just the visible rect
        let clean_svg = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 200 100">
            <g class="nodes">
                <g class="node">
                    <rect x="10" y="10" width="80" height="40"/>
                    <text>Label</text>
                </g>
            </g>
        </svg>"#;

        let mermaid_structure = SvgStructure::from_svg(mermaid_style_svg).unwrap();
        let clean_structure = SvgStructure::from_svg(clean_svg).unwrap();

        // Both should report the same number of VISIBLE rects (1)
        // Currently this will fail because we count all rects
        assert_eq!(
            mermaid_structure.shapes.rect, clean_structure.shapes.rect,
            "Should count only visible rects, not helper elements. Mermaid has {} rects, clean has {}",
            mermaid_structure.shapes.rect, clean_structure.shapes.rect
        );
    }

    #[test]
    fn test_architecture_counts_nodes_and_edges() {
        let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 200 100">
            <g class="architecture-edges">
                <g><path class="edge" d="M 0 0 L 10 10"/></g>
            </g>
            <g class="architecture-services">
                <g class="architecture-service"></g>
                <g class="architecture-junction"></g>
            </g>
        </svg>"#;

        let structure = SvgStructure::from_svg(svg).unwrap();
        assert_eq!(structure.node_count, 2);
        assert_eq!(structure.edge_count, 1);
    }

    #[test]
    fn test_parse_simple_svg() {
        let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 200 100">
            <rect x="10" y="10" width="80" height="40"/>
            <text x="50" y="35">Hello</text>
        </svg>"#;

        let structure = SvgStructure::from_svg(svg).unwrap();
        assert_eq!(structure.width, 200.0);
        assert_eq!(structure.height, 100.0);
        assert_eq!(structure.shapes.rect, 1);
        assert!(structure.labels.contains(&"Hello".to_string()));
    }

    #[test]
    fn test_compare_identical() {
        let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 200 100">
            <rect class="node" x="10" y="10" width="80" height="40"/>
            <text>Label</text>
        </svg>"#;

        let s1 = SvgStructure::from_svg(svg).unwrap();
        let s2 = SvgStructure::from_svg(svg).unwrap();

        assert_eq!(s1, s2);
    }

    #[test]
    fn test_compare_different_dimensions() {
        let svg1 = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 200 100"></svg>"#;
        let svg2 = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 400 200"></svg>"#;

        let s1 = SvgStructure::from_svg(svg1).unwrap();
        let s2 = SvgStructure::from_svg(svg2).unwrap();

        assert_ne!(s1.width, s2.width);
        assert_ne!(s1.height, s2.height);
    }
}
