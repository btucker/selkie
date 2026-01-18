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
    /// Font analysis: tracks font-size and font-weight on text elements
    pub font_analysis: FontAnalysis,
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
    /// Initial direction points for each edge (the second point in the path)
    /// Used to determine the initial tangent direction for curved paths
    pub edge_initial_directions: Vec<Option<(f64, f64)>>,
    /// Node bounding boxes: list of (x, y, width, height, id/class)
    pub node_bounds: Vec<NodeBounds>,
    /// Edges that attach to top/bottom of nodes (vertical attachment)
    pub vertical_attachments: usize,
    /// Edges that attach to left/right of nodes (horizontal attachment)
    pub horizontal_attachments: usize,
    /// Detailed edge attachment information
    pub edge_details: Vec<EdgeDetail>,
}

/// Detailed information about a single edge
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct EdgeDetail {
    /// Start point coordinates
    pub start: (f64, f64),
    /// End point coordinates
    pub end: (f64, f64),
    /// Node ID at start (if identified)
    pub start_node: Option<String>,
    /// Node ID at end (if identified)
    pub end_node: Option<String>,
    /// Which edge of the start node (top, bottom, left, right)
    pub start_edge: String,
    /// Which edge of the end node (top, bottom, left, right)
    pub end_edge: String,
    /// Offset from center of start edge (0 = centered)
    pub start_center_offset: f64,
    /// Offset from center of end edge (0 = centered)
    pub end_center_offset: f64,
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

/// Analysis of font styles used in text elements
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct FontAnalysis {
    /// Font sizes found (class/context -> size)
    pub font_sizes: Vec<FontStyle>,
    /// Font weights found (class/context -> weight)
    pub font_weights: Vec<FontStyle>,
    /// Count of text elements analyzed
    pub text_count: usize,
}

/// A font style value with its context
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FontStyle {
    /// CSS class or context where this style was found
    pub context: String,
    /// The value (e.g., "14" for font-size, "bold" for font-weight)
    pub value: String,
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

        // Analyze font styles
        let font_analysis = analyze_fonts(&doc);

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
            font_analysis,
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

        // Also check for node containers (<g class="node" transform="translate(x,y)">)
        // These contain child <path> or <rect> elements that define the box bounds
        // Handles: ER diagrams (entity-*), block diagrams (block-*), mermaid.js nodes (id-*)
        if node.tag_name().name() == "g" {
            let class = node.attribute("class").unwrap_or("");
            let id = node.attribute("id").unwrap_or("");

            // Match nodes from various diagram types
            let is_node_group = class.contains("node")
                && (id.contains("entity")
                    || id.starts_with("block-")
                    || id.starts_with("id-")
                    || id.starts_with("id")
                    || id.starts_with("node-"));

            if is_node_group {
                // Parse transform="translate(x, y)"
                if let Some(transform) = node.attribute("transform") {
                    if let Some((cx, cy)) = parse_translate(transform) {
                        // Find first child path to get bounds
                        for child in node.children() {
                            if child.tag_name().name() == "path" {
                                if let Some(d) = child.attribute("d") {
                                    if let Some((half_w, half_h)) = parse_rect_path_dimensions(d) {
                                        geometry.node_bounds.push(NodeBounds {
                                            x: cx - half_w,
                                            y: cy - half_h,
                                            width: half_w * 2.0,
                                            height: half_h * 2.0,
                                            id: id.to_string(),
                                        });
                                        break;
                                    }
                                }
                            }
                            // Also check for rect child (LINE-ITEM uses this)
                            if child.tag_name().name() == "rect" {
                                let rx = child
                                    .attribute("x")
                                    .and_then(|s| s.parse::<f64>().ok())
                                    .unwrap_or(0.0);
                                let ry = child
                                    .attribute("y")
                                    .and_then(|s| s.parse::<f64>().ok())
                                    .unwrap_or(0.0);
                                let rw = child
                                    .attribute("width")
                                    .and_then(|s| s.parse::<f64>().ok())
                                    .unwrap_or(0.0);
                                let rh = child
                                    .attribute("height")
                                    .and_then(|s| s.parse::<f64>().ok())
                                    .unwrap_or(0.0);
                                if rw > 0.0 && rh > 0.0 {
                                    geometry.node_bounds.push(NodeBounds {
                                        x: cx + rx,
                                        y: cy + ry,
                                        width: rw,
                                        height: rh,
                                        id: id.to_string(),
                                    });
                                    break;
                                }
                            }
                        }
                    }
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
                    // Use parse_path_with_directions to capture initial direction for curved paths
                    if let Some((start, second_point, end)) = parse_path_with_directions(d) {
                        geometry
                            .edge_endpoints
                            .push((start.0, start.1, end.0, end.1));
                        geometry.edge_initial_directions.push(second_point);

                        // Find best matching nodes for start and end
                        let mut best_start: Option<AttachmentInfo> = None;
                        let mut best_end: Option<AttachmentInfo> = None;

                        for bounds in &geometry.node_bounds {
                            let start_info = classify_attachment_detailed(start, bounds);
                            let end_info = classify_attachment_detailed(end, bounds);

                            if start_info.attach_type != AttachmentType::None
                                && (best_start.is_none()
                                    || start_info.distance < best_start.as_ref().unwrap().distance)
                            {
                                best_start = Some(start_info);
                            }
                            if end_info.attach_type != AttachmentType::None
                                && (best_end.is_none()
                                    || end_info.distance < best_end.as_ref().unwrap().distance)
                            {
                                best_end = Some(end_info);
                            }

                            // Count attachment types
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

                        // Create edge detail
                        let detail = EdgeDetail {
                            start,
                            end,
                            start_node: best_start.as_ref().and_then(|i| i.node_id.clone()),
                            end_node: best_end.as_ref().and_then(|i| i.node_id.clone()),
                            start_edge: best_start
                                .as_ref()
                                .map(|i| i.edge_name.clone())
                                .unwrap_or_else(|| "none".to_string()),
                            end_edge: best_end
                                .as_ref()
                                .map(|i| i.edge_name.clone())
                                .unwrap_or_else(|| "none".to_string()),
                            start_center_offset: best_start
                                .as_ref()
                                .map(|i| i.center_offset)
                                .unwrap_or(0.0),
                            end_center_offset: best_end
                                .as_ref()
                                .map(|i| i.center_offset)
                                .unwrap_or(0.0),
                        };
                        geometry.edge_details.push(detail);
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

/// Detailed attachment info for an edge endpoint
struct AttachmentInfo {
    attach_type: AttachmentType,
    edge_name: String,       // "top", "bottom", "left", "right", "none"
    node_id: Option<String>, // ID of the node this attaches to
    center_offset: f64,      // Distance from center of that edge (0 = centered)
    distance: f64,           // Distance from the edge
}

/// Classify how a point attaches to a node bounds with detailed info
/// Returns the CLOSEST matching edge within tolerance, not the first match
fn classify_attachment_detailed(point: (f64, f64), bounds: &NodeBounds) -> AttachmentInfo {
    let (px, py) = point;
    let tolerance = 25.0; // Increased tolerance to account for marker offsets

    let left = bounds.x;
    let right = bounds.x + bounds.width;
    let top = bounds.y;
    let bottom = bounds.y + bounds.height;
    let center_x = bounds.x + bounds.width / 2.0;
    let center_y = bounds.y + bounds.height / 2.0;

    // Check proximity to each edge
    let dist_top = (py - top).abs();
    let dist_bottom = (py - bottom).abs();
    let dist_left = (px - left).abs();
    let dist_right = (px - right).abs();

    let within_x = px >= left - tolerance && px <= right + tolerance;
    let within_y = py >= top - tolerance && py <= bottom + tolerance;

    // Collect all matching edges within tolerance
    let mut candidates = Vec::new();

    if dist_top < tolerance && within_x {
        candidates.push(AttachmentInfo {
            attach_type: AttachmentType::Vertical,
            edge_name: "top".to_string(),
            node_id: Some(bounds.id.clone()),
            center_offset: px - center_x,
            distance: dist_top,
        });
    }
    if dist_bottom < tolerance && within_x {
        candidates.push(AttachmentInfo {
            attach_type: AttachmentType::Vertical,
            edge_name: "bottom".to_string(),
            node_id: Some(bounds.id.clone()),
            center_offset: px - center_x,
            distance: dist_bottom,
        });
    }
    if dist_left < tolerance && within_y {
        candidates.push(AttachmentInfo {
            attach_type: AttachmentType::Horizontal,
            edge_name: "left".to_string(),
            node_id: Some(bounds.id.clone()),
            center_offset: py - center_y,
            distance: dist_left,
        });
    }
    if dist_right < tolerance && within_y {
        candidates.push(AttachmentInfo {
            attach_type: AttachmentType::Horizontal,
            edge_name: "right".to_string(),
            node_id: Some(bounds.id.clone()),
            center_offset: py - center_y,
            distance: dist_right,
        });
    }

    // Return the candidate with the smallest distance
    candidates
        .into_iter()
        .min_by(|a, b| {
            a.distance
                .partial_cmp(&b.distance)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .unwrap_or_else(|| AttachmentInfo {
            attach_type: AttachmentType::None,
            edge_name: "none".to_string(),
            node_id: None,
            center_offset: 0.0,
            distance: f64::MAX,
        })
}

/// Classify how a point attaches to a node bounds (legacy simple version)
fn classify_attachment(point: (f64, f64), bounds: &NodeBounds) -> (AttachmentType, f64) {
    let info = classify_attachment_detailed(point, bounds);
    (info.attach_type, info.distance)
}

/// Parse transform="translate(x, y)" or "translate(x,y)"
fn parse_translate(transform: &str) -> Option<(f64, f64)> {
    // Look for translate(x, y) pattern
    if let Some(start) = transform.find("translate(") {
        let rest = &transform[start + 10..];
        if let Some(end) = rest.find(')') {
            let coords = &rest[..end];
            // Split by comma or space
            let parts: Vec<&str> = coords.split([',', ' ']).collect();
            if parts.len() >= 2 {
                let x = parts[0].trim().parse::<f64>().ok()?;
                let y = parts[1].trim().parse::<f64>().ok()?;
                return Some((x, y));
            }
        }
    }
    None
}

/// Parse rectangular path dimensions from mermaid's path d attribute
/// e.g., "M-93.828125 -85.5 L93.828125 -85.5 L93.828125 85.5 L-93.828125 85.5"
/// Returns (half_width, half_height)
fn parse_rect_path_dimensions(d: &str) -> Option<(f64, f64)> {
    // Mermaid paths start with M followed by negative half-width and half-height
    // e.g., M-93.828125 -85.5 means center is at (0,0), box is from -93.8 to +93.8
    let parts: Vec<&str> = d.split_whitespace().collect();
    if parts.is_empty() {
        return None;
    }

    // Parse first M command to get the top-left corner (negative values)
    let first = parts.first()?;
    if let Some(coords) = first.strip_prefix('M') {
        // Handle "M-93.828125" followed by "-85.5" or "M-93,-85"
        let x = if coords.is_empty() {
            parts.get(1)?.parse::<f64>().ok()?
        } else {
            coords.parse::<f64>().ok()?
        };

        // Get y value (might be second element or after comma)
        let y = if coords.is_empty() || !coords.contains(',') {
            let y_str = if coords.is_empty() {
                parts.get(2)?
            } else {
                parts.get(1)?
            };
            y_str.parse::<f64>().ok()?
        } else {
            let comma_idx = coords.find(',')?;
            coords[comma_idx + 1..].parse::<f64>().ok()?
        };

        // Return absolute values as half-dimensions
        return Some((x.abs(), y.abs()));
    }

    None
}

/// Parse path with initial direction: returns (start, second_point, end)
/// The second_point is used to determine the initial tangent direction of curved paths.
/// For paths like "M122,451 L122,459 C...", the second point is (122,459) which shows
/// the edge starts going DOWN even if the overall direction is diagonal.
#[allow(clippy::type_complexity)]
fn parse_path_with_directions(d: &str) -> Option<((f64, f64), Option<(f64, f64)>, (f64, f64))> {
    let normalized = normalize_path_commands(d);
    let parts: Vec<&str> = normalized.split_whitespace().collect();
    if parts.is_empty() {
        return None;
    }

    let mut start: Option<(f64, f64)> = None;
    let mut second_point: Option<(f64, f64)> = None;
    let mut end: Option<(f64, f64)> = None;
    let mut point_count = 0;
    let mut i = 0;

    while i < parts.len() {
        let part = parts[i];

        // Handle M (moveto) command - sets start point
        if part == "M" || part.starts_with('M') {
            let (x, y) = if part == "M" {
                i += 1;
                let coords = parse_coord_pair(&parts, &mut i)?;
                // parse_coord_pair already advanced i, so continue to skip the i += 1 at end
                if start.is_none() {
                    start = Some(coords);
                    point_count = 1;
                }
                end = Some(coords);
                continue;
            } else {
                parse_inline_coords(&part[1..])?
            };
            if start.is_none() {
                start = Some((x, y));
                point_count = 1;
            }
            end = Some((x, y));
        }
        // Handle L (lineto) command
        else if part == "L" || part.starts_with('L') {
            let (x, y) = if part == "L" {
                i += 1;
                let coords = parse_coord_pair(&parts, &mut i)?;
                point_count += 1;
                if point_count == 2 && second_point.is_none() {
                    second_point = Some(coords);
                }
                end = Some(coords);
                continue;
            } else {
                parse_inline_coords(&part[1..])?
            };
            point_count += 1;
            if point_count == 2 && second_point.is_none() {
                second_point = Some((x, y));
            }
            end = Some((x, y));
        }
        // Handle C (curveto) command - takes 3 coordinate pairs
        else if part == "C" || part.starts_with('C') {
            if part == "C" {
                i += 1;
                // First control point - this is the initial direction for a curve
                let (cx1, cy1) = parse_coord_pair(&parts, &mut i)?;
                if point_count == 1 && second_point.is_none() {
                    // Use first control point as direction indicator
                    second_point = Some((cx1, cy1));
                }
                // Skip second control point
                parse_coord_pair(&parts, &mut i)?;
                // Third point is the endpoint
                let (x, y) = parse_coord_pair(&parts, &mut i)?;
                point_count += 1;
                end = Some((x, y));
                continue;
            } else {
                let coords_str = &part[1..];
                let coords: Vec<f64> = coords_str
                    .split([',', ' '])
                    .filter_map(|s| s.parse().ok())
                    .collect();
                if coords.len() >= 6 {
                    if point_count == 1 && second_point.is_none() {
                        second_point = Some((coords[0], coords[1]));
                    }
                    point_count += 1;
                    end = Some((coords[4], coords[5]));
                }
            }
        }
        // Handle Q (quadratic Bezier) command - takes 2 coordinate pairs (control point + endpoint)
        else if part == "Q" || part.starts_with('Q') {
            if part == "Q" {
                i += 1;
                // Control point - this is the initial direction for the curve
                let (cx, cy) = parse_coord_pair(&parts, &mut i)?;
                if point_count == 1 && second_point.is_none() {
                    // Use control point as direction indicator
                    second_point = Some((cx, cy));
                }
                // Endpoint
                let (x, y) = parse_coord_pair(&parts, &mut i)?;
                point_count += 1;
                end = Some((x, y));
                continue;
            } else {
                let coords_str = &part[1..];
                let coords: Vec<f64> = coords_str
                    .split([',', ' '])
                    .filter_map(|s| s.parse().ok())
                    .collect();
                if coords.len() >= 4 {
                    if point_count == 1 && second_point.is_none() {
                        second_point = Some((coords[0], coords[1]));
                    }
                    point_count += 1;
                    end = Some((coords[2], coords[3]));
                }
            }
        }
        // Handle numbers that might be continuation of previous command
        else if let Some((x, y)) = parse_inline_coords(part) {
            point_count += 1;
            if point_count == 2 && second_point.is_none() {
                second_point = Some((x, y));
            }
            end = Some((x, y));
        }

        i += 1;
    }

    match (start, end) {
        (Some(s), Some(e)) => Some((s, second_point, e)),
        _ => None,
    }
}

/// Normalize SVG path commands by inserting spaces before command letters.
/// This handles compact mermaid paths like "M122,179L122,280C..." by converting to
/// "M122,179 L122,280 C..."
fn normalize_path_commands(d: &str) -> String {
    let mut result = String::with_capacity(d.len() * 2);

    for c in d.chars() {
        // Insert space before command letters (except for first character and after another space)
        if matches!(
            c,
            'M' | 'L'
                | 'C'
                | 'Q'
                | 'A'
                | 'H'
                | 'V'
                | 'Z'
                | 'm'
                | 'l'
                | 'c'
                | 'q'
                | 'a'
                | 'h'
                | 'v'
                | 'z'
        ) {
            // Add space before command letter if not at start and previous char isn't space
            if !result.is_empty() && !result.ends_with(' ') {
                result.push(' ');
            }
            result.push(c);
        } else {
            result.push(c);
        }
    }

    result
}

fn parse_coord_pair(parts: &[&str], i: &mut usize) -> Option<(f64, f64)> {
    if *i >= parts.len() {
        return None;
    }

    let part = parts[*i];

    // Try to parse as "x,y" or "x y"
    if let Some((x, y)) = parse_inline_coords(part) {
        *i += 1; // Advance past this part
        return Some((x, y));
    }

    // Try separate x and y values
    // Strip leading/trailing commas that may appear in paths like "C x y, x y, x y"
    let x: f64 = part.trim_matches(',').parse().ok()?;
    *i += 1;
    if *i >= parts.len() {
        return None;
    }
    let y: f64 = parts[*i].trim_matches(',').parse().ok()?;
    *i += 1; // Advance past y value
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

/// Analyze font styles (size, weight) on text elements
fn analyze_fonts(doc: &roxmltree::Document) -> FontAnalysis {
    let mut analysis = FontAnalysis::default();

    // Extract CSS font rules if present (for eval feature)
    #[cfg(feature = "eval")]
    let css_fonts = extract_css_font_styles(doc);
    #[cfg(not(feature = "eval"))]
    let css_fonts: std::collections::HashMap<String, (Option<String>, Option<String>)> =
        std::collections::HashMap::new();

    for node in doc.descendants() {
        if node.tag_name().name() == "text" {
            analysis.text_count += 1;

            // Get context from class attribute
            let class = node.attribute("class").unwrap_or("").to_string();
            let context = if class.is_empty() {
                "text".to_string()
            } else {
                class.clone()
            };

            // Check inline font-size attribute
            if let Some(size) = node.attribute("font-size") {
                analysis.font_sizes.push(FontStyle {
                    context: context.clone(),
                    value: size.to_string(),
                });
            } else {
                // Check CSS rules for matching class
                for css_class in class.split_whitespace() {
                    if let Some((Some(s), _)) = css_fonts.get(css_class) {
                        analysis.font_sizes.push(FontStyle {
                            context: context.clone(),
                            value: s.clone(),
                        });
                        break;
                    }
                }
            }

            // Check inline font-weight attribute
            if let Some(weight) = node.attribute("font-weight") {
                analysis.font_weights.push(FontStyle {
                    context: context.clone(),
                    value: weight.to_string(),
                });
            } else {
                // Check CSS rules for matching class
                for css_class in class.split_whitespace() {
                    if let Some((_, Some(w))) = css_fonts.get(css_class) {
                        analysis.font_weights.push(FontStyle {
                            context: context.clone(),
                            value: w.clone(),
                        });
                        break;
                    }
                }
            }

            // Also check inline style attribute
            if let Some(style) = node.attribute("style") {
                if let Some(size) = extract_style_property(style, "font-size") {
                    analysis.font_sizes.push(FontStyle {
                        context: context.clone(),
                        value: size,
                    });
                }
                if let Some(weight) = extract_style_property(style, "font-weight") {
                    analysis.font_weights.push(FontStyle {
                        context,
                        value: weight,
                    });
                }
            }
        }
    }

    analysis
}

/// Extract a property value from an inline style string
fn extract_style_property(style: &str, property: &str) -> Option<String> {
    for part in style.split(';') {
        let trimmed = part.trim();
        if let Some(value) = trimmed.strip_prefix(property) {
            if let Some(v) = value.strip_prefix(':') {
                return Some(v.trim().to_string());
            }
        }
    }
    None
}

/// Extract font-size and font-weight from CSS style blocks
#[cfg(feature = "eval")]
fn extract_css_font_styles(
    doc: &roxmltree::Document,
) -> std::collections::HashMap<String, (Option<String>, Option<String>)> {
    use simplecss::StyleSheet;
    let mut css_fonts = std::collections::HashMap::new();

    for node in doc.descendants() {
        if node.tag_name().name() == "style" {
            if let Some(css_text) = node.text() {
                let stylesheet = StyleSheet::parse(css_text);
                for rule in stylesheet.rules {
                    let mut font_size: Option<String> = None;
                    let mut font_weight: Option<String> = None;

                    for decl in &rule.declarations {
                        if decl.name == "font-size" {
                            font_size = Some(decl.value.trim().to_string());
                        } else if decl.name == "font-weight" {
                            font_weight = Some(decl.value.trim().to_string());
                        }
                    }

                    // Associate with each selector in the rule
                    if font_size.is_some() || font_weight.is_some() {
                        let selector_str = rule.selector.to_string();
                        for selector in selector_str.split(',') {
                            let sel = selector.trim();
                            // Extract class name from selector (e.g., ".entity-name" -> "entity-name")
                            if let Some(class_name) = sel.strip_prefix('.') {
                                let class_name = class_name.split_whitespace().next().unwrap_or("");
                                css_fonts.insert(
                                    class_name.to_string(),
                                    (font_size.clone(), font_weight.clone()),
                                );
                            }
                            // Also handle ID selectors (e.g., "#my-svg" -> "root")
                            // and element selectors (e.g., "svg" -> "root")
                            // These are typically used for default/inherited font sizes
                            else if sel.starts_with('#') || sel == "svg" || sel.ends_with(" svg")
                            {
                                css_fonts.insert(
                                    "root".to_string(),
                                    (font_size.clone(), font_weight.clone()),
                                );
                            }
                        }
                    }
                }
            }
        }
    }

    css_fonts
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

    #[test]
    fn test_mermaid_er_data_edge_detection() {
        // Simplified mermaid ER diagram SVG with data-edge attribute
        let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 500 600">
            <g class="edgePaths">
                <path d="M122,179 L122,280"
                      class="edge-thickness-normal edge-pattern-solid relationshipLine"
                      data-edge="true"
                      marker-start="url(#er-onlyOneStart)"
                      marker-end="url(#er-zeroOrMoreEnd)"/>
                <path d="M122,451 L237,564"
                      class="edge-thickness-normal edge-pattern-solid relationshipLine"
                      data-edge="true"/>
                <path d="M463,451 L348,564"
                      class="edge-thickness-normal edge-pattern-solid relationshipLine"
                      data-edge="true"/>
            </g>
            <g class="nodes">
                <g class="node default" id="entity-CUSTOMER-0" transform="translate(122, 93.5)">
                    <path d="M-94 -85.5 L94 -85.5 L94 85.5 L-94 85.5"/>
                </g>
                <g class="node default" id="entity-ORDER-1" transform="translate(122, 365.5)">
                    <path d="M-114 -85.5 L114 -85.5 L114 85.5 L-114 85.5"/>
                </g>
            </g>
        </svg>"#;

        let structure = SvgStructure::from_svg(svg).unwrap();

        // Should detect 3 edges via data-edge attribute
        assert_eq!(
            structure.edge_count, 3,
            "Expected 3 edges from data-edge attribute, got {}",
            structure.edge_count
        );

        // Should detect nodes
        assert!(
            structure.node_count >= 2,
            "Expected at least 2 nodes, got {}",
            structure.node_count
        );

        // Should have edge geometry details
        assert_eq!(
            structure.edge_geometry.edge_endpoints.len(),
            3,
            "Expected 3 edge endpoints"
        );
    }

    #[test]
    fn test_mermaid_minified_data_edge_detection() {
        // Minified mermaid SVG (all on one line) - this is what we get from mermaid.js
        let svg = r#"<svg id="my-svg" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 500 600"><g><g class="edgePaths"><path d="M122,179L122,280" class="edge-thickness-normal edge-pattern-solid relationshipLine" data-edge="true" marker-start="url(#er-onlyOneStart)" marker-end="url(#er-zeroOrMoreEnd)"/><path d="M122,451L237,564" class="edge-thickness-normal edge-pattern-solid relationshipLine" data-edge="true"/><path d="M463,451L348,564" class="edge-thickness-normal edge-pattern-solid relationshipLine" data-edge="true"/></g></g></svg>"#;

        let structure = SvgStructure::from_svg(svg).unwrap();

        // Should detect 3 edges via data-edge attribute
        assert_eq!(
            structure.edge_count, 3,
            "Expected 3 edges from minified SVG data-edge attribute, got {}",
            structure.edge_count
        );
    }

    #[test]
    fn test_real_mermaid_er_reference_svg() {
        // Read the actual mermaid reference SVG file if it exists
        let path = "docs/images/er.svg";
        if !std::path::Path::new(path).exists() {
            eprintln!("Skipping test: {} not found", path);
            return;
        }

        let svg = std::fs::read_to_string(path).unwrap();

        // First check how many data-edge attributes we can find in the raw string
        let data_edge_count = svg.matches("data-edge").count();
        eprintln!("Raw data-edge count in file: {}", data_edge_count);

        let structure = SvgStructure::from_svg(&svg).unwrap();

        eprintln!("edge_count: {}", structure.edge_count);
        eprintln!(
            "edge_endpoints: {:?}",
            structure.edge_geometry.edge_endpoints
        );
        eprintln!(
            "edge_details: {:?}",
            structure.edge_geometry.edge_details.len()
        );

        // Should detect edges if data-edge is present
        if data_edge_count > 0 {
            assert!(
                structure.edge_count > 0,
                "Expected edges to be detected, got edge_count={}",
                structure.edge_count
            );
        }
    }

    #[test]
    fn test_mermaid_reference_from_eval_report() {
        // Try to find and read a mermaid reference SVG from the eval-report directory
        // This tests the actual mermaid-rendered SVG, not the selkie output
        let pattern = "eval-report/selkie-eval-*/er/er_reference.svg";
        let paths: Vec<_> = glob::glob(pattern)
            .expect("Failed to read glob pattern")
            .filter_map(|r| r.ok())
            .collect();

        if paths.is_empty() {
            eprintln!("Skipping test: no eval-report reference SVG found");
            return;
        }

        let path = &paths[0];
        eprintln!("Testing mermaid reference: {}", path.display());

        let svg = std::fs::read_to_string(path).unwrap();

        // Count raw data-edge occurrences in the file
        let data_edge_count = svg.matches("data-edge=\"true\"").count();
        eprintln!("Raw data-edge=\"true\" count: {}", data_edge_count);

        // Parse the structure
        let structure = SvgStructure::from_svg(&svg).unwrap();

        eprintln!("Parsed edge_count: {}", structure.edge_count);
        eprintln!(
            "Edge endpoints: {}",
            structure.edge_geometry.edge_endpoints.len()
        );

        // Mermaid ER diagrams should have edges detected via data-edge attribute
        assert_eq!(
            structure.edge_count, data_edge_count,
            "Edge count ({}) should match data-edge count ({})",
            structure.edge_count, data_edge_count
        );
    }

    #[test]
    fn test_selkie_er_svg_edge_attachment_detection() {
        // Test the actual selkie-generated ER SVG to trace edge attachment detection
        let pattern = "eval-report/selkie-eval-*/er/er_selkie.svg";
        let mut paths: Vec<_> = glob::glob(pattern)
            .expect("Failed to read glob pattern")
            .filter_map(|r| r.ok())
            .collect();

        if paths.is_empty() {
            eprintln!("Skipping test: no selkie SVG found");
            return;
        }

        // Sort by modification time to get the most recent
        paths.sort_by(|a, b| {
            let a_time = std::fs::metadata(a).and_then(|m| m.modified()).ok();
            let b_time = std::fs::metadata(b).and_then(|m| m.modified()).ok();
            b_time.cmp(&a_time) // Reverse order: most recent first
        });
        let path = &paths[0]; // Use the most recent
        eprintln!("Testing selkie SVG: {}", path.display());

        let svg = std::fs::read_to_string(path).unwrap();
        let structure = SvgStructure::from_svg(&svg).unwrap();

        eprintln!("node_count: {}", structure.node_count);
        eprintln!("edge_count: {}", structure.edge_count);
        eprintln!(
            "node_bounds count: {}",
            structure.edge_geometry.node_bounds.len()
        );

        // Print all node bounds
        for (i, bounds) in structure.edge_geometry.node_bounds.iter().enumerate() {
            eprintln!(
                "Node bounds {}: id={} x={:.1} y={:.1} w={:.1} h={:.1}",
                i, bounds.id, bounds.x, bounds.y, bounds.width, bounds.height
            );
        }

        // Print edge details
        eprintln!(
            "edge_endpoints count: {}",
            structure.edge_geometry.edge_endpoints.len()
        );
        for (i, (sx, sy, ex, ey)) in structure.edge_geometry.edge_endpoints.iter().enumerate() {
            eprintln!(
                "Edge endpoint {}: ({:.1}, {:.1}) → ({:.1}, {:.1})",
                i, sx, sy, ex, ey
            );
        }

        for (i, detail) in structure.edge_geometry.edge_details.iter().enumerate() {
            eprintln!(
                "Edge detail {}: start_edge={} end_edge={} start_offset={:.1} end_offset={:.1}",
                i,
                detail.start_edge,
                detail.end_edge,
                detail.start_center_offset,
                detail.end_center_offset
            );
        }

        // The rendering is correct - let's verify the coordinates
        // Edge 2 should end at LINE-ITEM's LEFT side (x=175.05)
        // Edge 3 should end at LINE-ITEM's RIGHT side (x=304.95)
        if structure.edge_geometry.edge_endpoints.len() >= 3 {
            let edge2 = &structure.edge_geometry.edge_endpoints[1];
            let edge3 = &structure.edge_geometry.edge_endpoints[2];

            // Edge 2 endpoint (end_x, end_y)
            let (_, _, end_x2, end_y2) = edge2;
            // Edge 3 endpoint (end_x, end_y)
            let (_, _, end_x3, end_y3) = edge3;

            eprintln!("Edge 2 end: ({:.2}, {:.2})", end_x2, end_y2);
            eprintln!("Edge 3 end: ({:.2}, {:.2})", end_x3, end_y3);

            // Find LINE-ITEM bounds
            let line_item_bounds = structure
                .edge_geometry
                .node_bounds
                .iter()
                .find(|b| b.id.contains("LINE-ITEM") || b.x > 150.0 && b.y > 500.0);

            if let Some(bounds) = line_item_bounds {
                eprintln!(
                    "LINE-ITEM bounds: x={:.1} y={:.1} w={:.1} h={:.1}",
                    bounds.x, bounds.y, bounds.width, bounds.height
                );

                // Check if edge 2 ends at left side
                let dist_left = (*end_x2 - bounds.x).abs();
                let dist_right = (*end_x2 - (bounds.x + bounds.width)).abs();
                eprintln!(
                    "Edge 2 distance from left={:.1}, right={:.1}",
                    dist_left, dist_right
                );

                // Check if edge 3 ends at right side
                let dist_left3 = (*end_x3 - bounds.x).abs();
                let dist_right3 = (*end_x3 - (bounds.x + bounds.width)).abs();
                eprintln!(
                    "Edge 3 distance from left={:.1}, right={:.1}",
                    dist_left3, dist_right3
                );
            }
        }
    }
}
