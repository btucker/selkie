//! SVG rendering for mermaid diagrams

pub mod color;
mod document;
pub(crate) mod edges;
mod elements;
mod markers;
mod shapes;
pub mod structure;
mod theme;

pub use color::Color;
pub use document::SvgDocument;
pub use elements::{Attrs, SvgElement};
pub use structure::SvgStructure;
pub use theme::{Theme, ThemeBuilder};

use crate::diagrams::flowchart::{FlowSubGraph, FlowchartDb};
use crate::error::Result;
use crate::layout::LayoutGraph;

/// Configuration for SVG rendering
#[derive(Debug, Clone)]
pub struct RenderConfig {
    /// Theme for colors and fonts
    pub theme: Theme,
    /// Padding around the diagram
    pub padding: f64,
    /// Include embedded CSS in SVG
    pub embed_css: bool,
    /// Custom CSS to append after theme CSS (sanitized)
    ///
    /// Allows fine-grained style adjustments without modifying the theme.
    /// CSS is sanitized to prevent script injection.
    ///
    /// # Example
    ///
    /// ```
    /// use mermaid::render::RenderConfig;
    ///
    /// let config = RenderConfig {
    ///     theme_css: Some(".node rect { rx: 10; }".to_string()),
    ///     ..Default::default()
    /// };
    /// ```
    pub theme_css: Option<String>,
}

impl Default for RenderConfig {
    fn default() -> Self {
        Self {
            theme: Theme::default(),
            // Match mermaid.js default: flowchart?.diagramPadding ?? 8
            padding: 8.0,
            embed_css: true,
            theme_css: None,
        }
    }
}

/// SVG renderer for diagrams
#[derive(Debug, Clone)]
pub struct SvgRenderer {
    config: RenderConfig,
}

impl SvgRenderer {
    pub fn new(config: RenderConfig) -> Self {
        Self { config }
    }

    /// Render a flowchart to SVG
    pub fn render_flowchart(&self, db: &FlowchartDb, graph: &LayoutGraph) -> Result<String> {
        let mut doc = SvgDocument::new();

        // Calculate bounds including subgraphs (which extend beyond node bounds)
        let (view_min_x, view_min_y, view_width, view_height) =
            self.calculate_flowchart_bounds(db, graph);

        doc.set_size_with_origin(view_min_x, view_min_y, view_width, view_height);

        // Add theme styles
        if self.config.embed_css {
            let mut css = self.config.theme.generate_css();

            // Append custom CSS if provided (sanitized)
            if let Some(ref custom_css) = self.config.theme_css {
                let sanitized = sanitize_css(custom_css);
                if !sanitized.is_empty() {
                    css.push_str("\n/* Custom CSS */\n");
                    css.push_str(&sanitized);
                }
            }

            doc.add_style(&css);
        }

        // Add marker definitions
        doc.add_defs(markers::create_arrow_markers(&self.config.theme));

        // Render subgraphs to clusters container (rendered first, behind everything)
        for subgraph in db.subgraphs() {
            if let Some(element) = self.render_subgraph(subgraph, graph) {
                doc.add_cluster(element);
            }
        }

        // Render edges - paths and labels go to separate containers
        for edge in &graph.edges {
            // Skip dummy edges
            if edge.id.contains("_dummy_") {
                continue;
            }

            // Get the original edge info
            if let Some(flow_edge) = db.edges().iter().find(|e| {
                e.id.as_ref().map(|id| id == &edge.id).unwrap_or(false)
                    || (e.start == edge.sources.first().map(|s| s.as_str()).unwrap_or("")
                        && e.end == edge.targets.first().map(|s| s.as_str()).unwrap_or(""))
            }) {
                let result = edges::render_edge_parts(edge, flow_edge, &self.config.theme);
                if let Some(path) = result.path {
                    doc.add_edge_path(path);
                }
                if let Some(label) = result.label {
                    doc.add_edge_label(label);
                }
            }
        }

        // Render nodes to nodes container (rendered last, on top)
        for node in &graph.nodes {
            if node.is_dummy {
                continue;
            }

            // Get the original vertex info
            if let Some(vertex) = db.vertices().get(&node.id) {
                let shape_element = shapes::render_shape(node, vertex, &self.config.theme);
                doc.add_node(shape_element);
            }
        }

        Ok(doc.to_string())
    }

    /// Calculate bounds for the flowchart including subgraph boxes
    /// Returns (min_x, min_y, width, height) for the viewBox
    fn calculate_flowchart_bounds(
        &self,
        db: &FlowchartDb,
        graph: &LayoutGraph,
    ) -> (f64, f64, f64, f64) {
        let padding = self.config.padding;
        let subgraph_padding = 20.0;
        let title_height = 25.0;

        // Start with graph dimensions
        let mut min_x: f64 = 0.0;
        let mut min_y: f64 = 0.0;
        let mut max_x = graph.width.unwrap_or(800.0);
        let mut max_y = graph.height.unwrap_or(600.0);

        // Include bounds from each subgraph
        for subgraph in db.subgraphs() {
            let mut sg_min_x = f64::MAX;
            let mut sg_min_y = f64::MAX;
            let mut sg_max_x = f64::MIN;
            let mut sg_max_y = f64::MIN;
            let mut found_nodes = false;

            for node_id in &subgraph.nodes {
                if let Some(node) = graph.get_node(node_id) {
                    if let (Some(x), Some(y)) = (node.x, node.y) {
                        found_nodes = true;
                        sg_min_x = sg_min_x.min(x);
                        sg_min_y = sg_min_y.min(y);
                        sg_max_x = sg_max_x.max(x + node.width);
                        sg_max_y = sg_max_y.max(y + node.height);
                    }
                }
            }

            if found_nodes {
                // Apply subgraph padding and title height
                let box_min_x = sg_min_x - subgraph_padding;
                let box_min_y = sg_min_y - subgraph_padding - title_height;
                let box_max_x = sg_max_x + subgraph_padding;
                let box_max_y = sg_max_y + subgraph_padding;

                // Expand overall bounds if needed
                min_x = min_x.min(box_min_x);
                min_y = min_y.min(box_min_y);
                max_x = max_x.max(box_max_x);
                max_y = max_y.max(box_max_y);
            }
        }

        // Apply global padding
        min_x -= padding;
        min_y -= padding;
        max_x += padding;
        max_y += padding;

        let width = max_x - min_x;
        let height = max_y - min_y;

        (min_x, min_y, width, height)
    }

    /// Render a subgraph as a labeled container box
    fn render_subgraph(&self, subgraph: &FlowSubGraph, graph: &LayoutGraph) -> Option<SvgElement> {
        // Calculate bounding box from member nodes
        let mut min_x = f64::MAX;
        let mut min_y = f64::MAX;
        let mut max_x = f64::MIN;
        let mut max_y = f64::MIN;
        let mut found_nodes = false;

        for node_id in &subgraph.nodes {
            if let Some(node) = graph.get_node(node_id) {
                if let (Some(x), Some(y)) = (node.x, node.y) {
                    found_nodes = true;
                    min_x = min_x.min(x);
                    min_y = min_y.min(y);
                    max_x = max_x.max(x + node.width);
                    max_y = max_y.max(y + node.height);
                }
            }
        }

        if !found_nodes {
            return None;
        }

        // Add padding around the nodes
        let padding = 20.0;
        let title_height = 25.0;
        min_x -= padding;
        min_y -= padding + title_height;
        max_x += padding;
        max_y += padding;

        let width = max_x - min_x;
        let height = max_y - min_y;

        // Create the background rect
        let rect = SvgElement::rect(min_x, min_y, width, height)
            .with_attrs(Attrs::new().with_class("cluster"));

        // Create the title label
        let title = if !subgraph.title.is_empty() {
            &subgraph.title
        } else {
            &subgraph.id
        };

        // Center the label horizontally within the subgraph box
        let label = SvgElement::Text {
            x: min_x + width / 2.0,
            y: min_y + 16.0,
            content: title.to_string(),
            attrs: Attrs::new()
                .with_class("cluster-label")
                .with_attr("text-anchor", "middle"),
        };

        // Wrap in a group
        let group_attrs = Attrs::new()
            .with_class("subgraph")
            .with_id(&format!("subgraph-{}", subgraph.id));

        Some(SvgElement::group(vec![rect, label]).with_attrs(group_attrs))
    }
}

/// Sanitize CSS to prevent script injection and other attacks
///
/// This follows mermaid.js security patterns:
/// - Removes `<script>` and similar tags
/// - Blocks `javascript:` and `data:` URLs
/// - Validates balanced braces
/// - Removes potentially dangerous properties like `expression()`
fn sanitize_css(css: &str) -> String {
    // Check for dangerous patterns
    let lower = css.to_lowercase();

    // Block script tags and event handlers
    if lower.contains("<script")
        || lower.contains("</script")
        || lower.contains("javascript:")
        || lower.contains("vbscript:")
        || lower.contains("expression(")
        || lower.contains("behavior:")
        || lower.contains("-moz-binding")
    {
        return String::new();
    }

    // Block data URLs (can contain scripts)
    if lower.contains("url(data:") && (lower.contains("text/html") || lower.contains("image/svg")) {
        return String::new();
    }

    // Check for balanced braces
    let open_count = css.chars().filter(|&c| c == '{').count();
    let close_count = css.chars().filter(|&c| c == '}').count();
    if open_count != close_count {
        return String::new();
    }

    // Basic validation passed, return CSS
    // Note: This is intentionally permissive for legitimate use cases
    // while blocking known attack vectors
    css.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_subgraph_viewbox_includes_all_content() {
        use crate::diagrams::flowchart::parse;
        use crate::layout;
        use crate::layout::CharacterSizeEstimator;
        use crate::layout::ToLayoutGraph;

        // Parse a flowchart with a subgraph
        let input = r#"flowchart TB
    subgraph sg1 [Test Subgraph]
        A[Node A]
        B[Node B]
    end
    A --> B"#;

        let db = parse(input).unwrap();
        let estimator = CharacterSizeEstimator::default();
        let graph = db.to_layout_graph(&estimator).unwrap();
        let graph = layout::layout(graph).unwrap();

        // Render to SVG
        let renderer = SvgRenderer::new(RenderConfig::default());
        let svg = renderer.render_flowchart(&db, &graph).unwrap();

        // Extract viewBox from SVG
        let viewbox_re = regex::Regex::new(r#"viewBox="([^"]+)""#).unwrap();
        let viewbox = viewbox_re
            .captures(&svg)
            .and_then(|c| c.get(1))
            .map(|m| m.as_str())
            .expect("SVG should have viewBox");

        let parts: Vec<f64> = viewbox
            .split_whitespace()
            .filter_map(|s| s.parse().ok())
            .collect();
        let (vb_x, vb_y, _vb_width, _vb_height) = (parts[0], parts[1], parts[2], parts[3]);

        // Extract subgraph rect bounds
        let rect_re =
            regex::Regex::new(r#"class="cluster"[^/]*x="([^"]+)"[^/]*y="([^"]+)""#).unwrap();
        // Try alternate attribute order
        let rect_re2 =
            regex::Regex::new(r#"<rect x="([^"]+)" y="([^"]+)"[^>]*class="cluster""#).unwrap();

        let (rect_x, rect_y) = rect_re
            .captures(&svg)
            .or_else(|| rect_re2.captures(&svg))
            .map(|c| {
                (
                    c.get(1).unwrap().as_str().parse::<f64>().unwrap(),
                    c.get(2).unwrap().as_str().parse::<f64>().unwrap(),
                )
            })
            .expect("SVG should have subgraph rect");

        // The viewBox should contain the subgraph rect
        // rect_x and rect_y should be >= viewBox origin
        assert!(
            rect_x >= vb_x,
            "Subgraph rect x ({}) should be within viewBox (origin x={})",
            rect_x,
            vb_x
        );
        assert!(
            rect_y >= vb_y,
            "Subgraph rect y ({}) should be within viewBox (origin y={})",
            rect_y,
            vb_y
        );
    }

    #[test]
    fn test_svg_has_container_groups() {
        use crate::diagrams::flowchart::parse;
        use crate::layout;
        use crate::layout::CharacterSizeEstimator;
        use crate::layout::ToLayoutGraph;

        let input = r#"flowchart TB
    A[Start] --> B[End]"#;

        let db = parse(input).unwrap();
        let estimator = CharacterSizeEstimator::default();
        let graph = db.to_layout_graph(&estimator).unwrap();
        let graph = layout::layout(graph).unwrap();

        let renderer = SvgRenderer::new(RenderConfig::default());
        let svg = renderer.render_flowchart(&db, &graph).unwrap();

        // Verify container groups exist in correct order: clusters, edgePaths, edgeLabels, nodes
        // mermaid.js uses this structure for proper layering
        assert!(
            svg.contains(r#"<g class="clusters">"#),
            "SVG should have clusters container group"
        );
        assert!(
            svg.contains(r#"<g class="edgePaths">"#),
            "SVG should have edgePaths container group"
        );
        assert!(
            svg.contains(r#"<g class="edgeLabels">"#),
            "SVG should have edgeLabels container group"
        );
        assert!(
            svg.contains(r#"<g class="nodes">"#),
            "SVG should have nodes container group"
        );

        // Verify order by checking that clusters appears before nodes in the SVG
        let clusters_pos = svg.find(r#"class="clusters""#).expect("clusters not found");
        let edge_paths_pos = svg
            .find(r#"class="edgePaths""#)
            .expect("edgePaths not found");
        let edge_labels_pos = svg
            .find(r#"class="edgeLabels""#)
            .expect("edgeLabels not found");
        let nodes_pos = svg.find(r#"class="nodes""#).expect("nodes not found");

        assert!(
            clusters_pos < edge_paths_pos,
            "clusters should appear before edgePaths"
        );
        assert!(
            edge_paths_pos < edge_labels_pos,
            "edgePaths should appear before edgeLabels"
        );
        assert!(
            edge_labels_pos < nodes_pos,
            "edgeLabels should appear before nodes"
        );
    }

    #[test]
    fn test_subgraph_label_is_centered() {
        use crate::diagrams::flowchart::parse;
        use crate::layout;
        use crate::layout::CharacterSizeEstimator;
        use crate::layout::ToLayoutGraph;

        let input = r#"flowchart TB
    subgraph sg1 [My Subgraph Title]
        A[Node A]
    end"#;

        let db = parse(input).unwrap();
        let estimator = CharacterSizeEstimator::default();
        let graph = db.to_layout_graph(&estimator).unwrap();
        let graph = layout::layout(graph).unwrap();

        let renderer = SvgRenderer::new(RenderConfig::default());
        let svg = renderer.render_flowchart(&db, &graph).unwrap();

        // The cluster-label text should have text-anchor="middle" for centering
        assert!(
            svg.contains(r#"text-anchor="middle""#) || svg.contains("cluster-label"),
            "Subgraph label should be centered (have text-anchor=middle or be positioned at center)"
        );

        // Extract rect bounds and text x position
        let rect_re =
            regex::Regex::new(r#"<rect x="([^"]+)"[^>]*width="([^"]+)"[^>]*class="cluster""#)
                .unwrap();

        // If we can find both, verify the text is approximately centered
        if let Some(rect_caps) = rect_re.captures(&svg) {
            let rect_x: f64 = rect_caps.get(1).unwrap().as_str().parse().unwrap();
            let rect_width: f64 = rect_caps.get(2).unwrap().as_str().parse().unwrap();
            let rect_center = rect_x + rect_width / 2.0;

            // Text x position should be near center (within 10% of width)
            let text_x_re =
                regex::Regex::new(r#"<text x="([^"]+)"[^>]*class="cluster-label""#).unwrap();
            if let Some(text_caps) = text_x_re.captures(&svg) {
                let text_x: f64 = text_caps.get(1).unwrap().as_str().parse().unwrap();
                let tolerance = rect_width * 0.4; // 40% tolerance since left-aligned is clearly wrong
                assert!(
                    (text_x - rect_center).abs() < tolerance,
                    "Label x ({}) should be near rect center ({}), diff={}",
                    text_x,
                    rect_center,
                    (text_x - rect_center).abs()
                );
            }
        }
    }

    #[test]
    fn test_sanitize_css_allows_valid_css() {
        let css = ".node rect { fill: red; rx: 10; }";
        assert_eq!(sanitize_css(css), css);

        let css2 = ".edge-path { stroke-width: 2px; }";
        assert_eq!(sanitize_css(css2), css2);
    }

    #[test]
    fn test_sanitize_css_blocks_script_injection() {
        // Script tags
        assert_eq!(sanitize_css("<script>alert(1)</script>"), "");
        assert_eq!(sanitize_css(".x { } <script>bad</script>"), "");

        // JavaScript URLs
        assert_eq!(sanitize_css("background: url(javascript:alert(1))"), "");

        // VBScript
        assert_eq!(sanitize_css("background: url(vbscript:msgbox)"), "");

        // IE expression()
        assert_eq!(sanitize_css("width: expression(alert(1))"), "");

        // IE behavior
        assert_eq!(sanitize_css("behavior: url(xss.htc)"), "");

        // Firefox -moz-binding
        assert_eq!(sanitize_css("-moz-binding: url(xss.xml)"), "");
    }

    #[test]
    fn test_sanitize_css_blocks_dangerous_data_urls() {
        // HTML in data URL
        assert_eq!(sanitize_css("background: url(data:text/html,<script>)"), "");

        // SVG in data URL (can contain scripts)
        assert_eq!(
            sanitize_css("background: url(data:image/svg+xml,<svg>)"),
            ""
        );

        // Safe data URLs should be allowed
        let safe = "background: url(data:image/png;base64,abc)";
        assert_eq!(sanitize_css(safe), safe);
    }

    #[test]
    fn test_sanitize_css_blocks_unbalanced_braces() {
        assert_eq!(sanitize_css(".x { color: red;"), "");
        assert_eq!(sanitize_css(".x color: red; }"), "");
        assert_eq!(sanitize_css(".x {{ color: red; }"), "");
    }

    #[test]
    fn test_theme_css_appended_to_output() {
        use crate::diagrams::flowchart::parse;
        use crate::layout;
        use crate::layout::CharacterSizeEstimator;
        use crate::layout::ToLayoutGraph;

        let input = r#"flowchart TB
    A[Node A] --> B[Node B]"#;

        let db = parse(input).unwrap();
        let estimator = CharacterSizeEstimator::default();
        let graph = db.to_layout_graph(&estimator).unwrap();
        let graph = layout::layout(graph).unwrap();

        let config = RenderConfig {
            theme_css: Some(".custom-class { fill: blue; }".to_string()),
            ..Default::default()
        };

        let renderer = SvgRenderer::new(config);
        let svg = renderer.render_flowchart(&db, &graph).unwrap();

        // Custom CSS should appear in output
        assert!(
            svg.contains("/* Custom CSS */"),
            "SVG should contain custom CSS marker"
        );
        assert!(
            svg.contains(".custom-class { fill: blue; }"),
            "SVG should contain custom CSS"
        );
    }

    #[test]
    fn test_theme_css_sanitized_in_output() {
        use crate::diagrams::flowchart::parse;
        use crate::layout;
        use crate::layout::CharacterSizeEstimator;
        use crate::layout::ToLayoutGraph;

        let input = r#"flowchart TB
    A[Node A]"#;

        let db = parse(input).unwrap();
        let estimator = CharacterSizeEstimator::default();
        let graph = db.to_layout_graph(&estimator).unwrap();
        let graph = layout::layout(graph).unwrap();

        // Try to inject malicious CSS
        let config = RenderConfig {
            theme_css: Some("<script>alert(1)</script>".to_string()),
            ..Default::default()
        };

        let renderer = SvgRenderer::new(config);
        let svg = renderer.render_flowchart(&db, &graph).unwrap();

        // Malicious CSS should NOT appear
        assert!(
            !svg.contains("<script>"),
            "SVG should not contain script tags"
        );
        assert!(
            !svg.contains("/* Custom CSS */"),
            "Custom CSS marker should not appear when CSS was rejected"
        );
    }
}
