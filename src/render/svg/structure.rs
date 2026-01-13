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
        rect: count_elements(doc, "rect"),
        circle: count_elements(doc, "circle"),
        ellipse: count_elements(doc, "ellipse"),
        polygon: count_elements(doc, "polygon"),
        path: count_elements(doc, "path"),
        line: count_elements(doc, "line"),
        polyline: count_elements(doc, "polyline"),
    }
}

fn count_elements(doc: &roxmltree::Document, tag: &str) -> usize {
    doc.descendants()
        .filter(|n| n.tag_name().name() == tag)
        .count()
}

fn count_nodes_and_edges(doc: &roxmltree::Document) -> (usize, usize) {
    let mut node_count = 0;
    let mut edge_count = 0;

    for node in doc.descendants() {
        // Check for data-edge attribute (mermaid.js uses this)
        if node.attribute("data-edge").is_some() {
            edge_count += 1;
            continue;
        }

        if let Some(class) = node.attribute("class") {
            let classes: Vec<&str> = class.split_whitespace().collect();

            // Count nodes - elements with "node" class (both implementations)
            if classes
                .iter()
                .any(|c| *c == "node" || *c == "flowchart-node")
            {
                node_count += 1;
            }

            // Count edges - only count edge group containers, not child elements
            // selkie uses "edge" class on <g> elements
            // mermaid.js uses "flowchart-link" on <path> elements with data-edge
            // We only count "edge" here since data-edge is handled above
            if node.tag_name().name() == "g" && classes.contains(&"edge") {
                edge_count += 1;
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
        if tag == "text" || tag == "tspan" {
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

#[cfg(test)]
mod tests {
    use super::*;

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
