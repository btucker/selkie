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

/// Result of comparing two SVG structures
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComparisonResult {
    /// Whether the structures are considered equivalent
    pub matches: bool,
    /// Overall similarity score (0.0 - 1.0)
    pub similarity: f64,
    /// Detailed differences found
    pub differences: Vec<Difference>,
}

/// A specific difference between two SVG structures
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Difference {
    pub field: String,
    pub expected: String,
    pub actual: String,
    pub severity: Severity,
}

/// Severity of a difference
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum Severity {
    /// Critical difference that indicates a bug
    Critical,
    /// Important difference that should be investigated
    Major,
    /// Minor difference that may be acceptable
    Minor,
    /// Informational, likely acceptable variation
    Info,
}

/// Configuration for structural comparison
#[derive(Debug, Clone)]
pub struct CompareConfig {
    /// Tolerance for dimension comparison (percentage)
    pub dimension_tolerance: f64,
    /// Whether label order matters
    pub strict_label_order: bool,
    /// Whether to require exact shape counts
    pub strict_shape_counts: bool,
}

impl Default for CompareConfig {
    fn default() -> Self {
        Self {
            dimension_tolerance: 0.1, // 10% tolerance
            strict_label_order: false,
            strict_shape_counts: false,
        }
    }
}

impl SvgStructure {
    /// Compare this structure against another (expected) structure
    pub fn compare(&self, expected: &SvgStructure, config: &CompareConfig) -> ComparisonResult {
        let mut differences = Vec::new();
        let mut score_parts: Vec<f64> = Vec::new();

        // Compare dimensions
        let width_diff = (self.width - expected.width).abs() / expected.width.max(1.0);
        let height_diff = (self.height - expected.height).abs() / expected.height.max(1.0);

        if width_diff > config.dimension_tolerance {
            differences.push(Difference {
                field: "width".to_string(),
                expected: format!("{:.1}", expected.width),
                actual: format!("{:.1}", self.width),
                severity: Severity::Major,
            });
            score_parts.push(1.0 - width_diff.min(1.0));
        } else {
            score_parts.push(1.0);
        }

        if height_diff > config.dimension_tolerance {
            differences.push(Difference {
                field: "height".to_string(),
                expected: format!("{:.1}", expected.height),
                actual: format!("{:.1}", self.height),
                severity: Severity::Major,
            });
            score_parts.push(1.0 - height_diff.min(1.0));
        } else {
            score_parts.push(1.0);
        }

        // Compare node count
        if self.node_count != expected.node_count {
            differences.push(Difference {
                field: "node_count".to_string(),
                expected: expected.node_count.to_string(),
                actual: self.node_count.to_string(),
                severity: Severity::Critical,
            });
            let ratio = self.node_count.min(expected.node_count) as f64
                / self.node_count.max(expected.node_count).max(1) as f64;
            score_parts.push(ratio);
        } else {
            score_parts.push(1.0);
        }

        // Compare edge count
        if self.edge_count != expected.edge_count {
            differences.push(Difference {
                field: "edge_count".to_string(),
                expected: expected.edge_count.to_string(),
                actual: self.edge_count.to_string(),
                severity: Severity::Critical,
            });
            let ratio = self.edge_count.min(expected.edge_count) as f64
                / self.edge_count.max(expected.edge_count).max(1) as f64;
            score_parts.push(ratio);
        } else {
            score_parts.push(1.0);
        }

        // Compare labels
        let expected_labels: std::collections::HashSet<_> = expected.labels.iter().collect();
        let actual_labels: std::collections::HashSet<_> = self.labels.iter().collect();

        let missing: Vec<_> = expected_labels.difference(&actual_labels).collect();
        let extra: Vec<_> = actual_labels.difference(&expected_labels).collect();

        if !missing.is_empty() {
            differences.push(Difference {
                field: "labels_missing".to_string(),
                expected: format!("{:?}", missing),
                actual: "[]".to_string(),
                severity: Severity::Critical,
            });
        }
        if !extra.is_empty() {
            differences.push(Difference {
                field: "labels_extra".to_string(),
                expected: "[]".to_string(),
                actual: format!("{:?}", extra),
                severity: Severity::Minor,
            });
        }

        let label_match = expected_labels.intersection(&actual_labels).count() as f64
            / expected_labels.len().max(1) as f64;
        score_parts.push(label_match);

        // Compare shape counts (if strict)
        if config.strict_shape_counts {
            compare_shape_counts(
                &self.shapes,
                &expected.shapes,
                &mut differences,
                &mut score_parts,
            );
        }

        // Check for defs and style
        if expected.has_defs && !self.has_defs {
            differences.push(Difference {
                field: "has_defs".to_string(),
                expected: "true".to_string(),
                actual: "false".to_string(),
                severity: Severity::Minor,
            });
        }

        // Calculate overall similarity
        let similarity = if score_parts.is_empty() {
            1.0
        } else {
            score_parts.iter().sum::<f64>() / score_parts.len() as f64
        };

        // Determine if structures match (no critical differences and high similarity)
        let has_critical = differences.iter().any(|d| d.severity == Severity::Critical);
        let matches = !has_critical && similarity >= 0.8;

        ComparisonResult {
            matches,
            similarity,
            differences,
        }
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

fn compare_shape_counts(
    actual: &ShapeCounts,
    expected: &ShapeCounts,
    differences: &mut Vec<Difference>,
    score_parts: &mut Vec<f64>,
) {
    let fields = [
        ("rect", actual.rect, expected.rect),
        ("circle", actual.circle, expected.circle),
        ("ellipse", actual.ellipse, expected.ellipse),
        ("polygon", actual.polygon, expected.polygon),
        ("path", actual.path, expected.path),
        ("line", actual.line, expected.line),
        ("polyline", actual.polyline, expected.polyline),
    ];

    for (name, actual_count, expected_count) in fields {
        if actual_count != expected_count {
            differences.push(Difference {
                field: format!("shapes.{}", name),
                expected: expected_count.to_string(),
                actual: actual_count.to_string(),
                severity: Severity::Minor,
            });
            let ratio = actual_count.min(expected_count) as f64
                / actual_count.max(expected_count).max(1) as f64;
            score_parts.push(ratio);
        } else {
            score_parts.push(1.0);
        }
    }
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

        let structure = SvgStructure::from_svg(svg).unwrap();
        let result = structure.compare(&structure, &CompareConfig::default());

        assert!(result.matches);
        assert_eq!(result.similarity, 1.0);
        assert!(result.differences.is_empty());
    }

    #[test]
    fn test_compare_different_dimensions() {
        let svg1 = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 200 100"></svg>"#;
        let svg2 = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 400 200"></svg>"#;

        let s1 = SvgStructure::from_svg(svg1).unwrap();
        let s2 = SvgStructure::from_svg(svg2).unwrap();

        let result = s1.compare(&s2, &CompareConfig::default());
        assert!(!result.matches);
        assert!(result.differences.iter().any(|d| d.field == "width"));
    }
}
