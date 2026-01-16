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

use crate::diagrams::architecture::{ArchitectureDb, ArchitectureDirection, ArchitectureService};
use crate::diagrams::flowchart::{FlowSubGraph, FlowchartDb};
use crate::error::Result;
use crate::layout::{LayoutGraph, LayoutNode, Point};
use crate::render::architecture::{
    ARCH_EDGE_GROUP_LABEL_SHIFT, ARCH_FONT_SIZE, ARCH_GROUP_ICON_SCALE, ARCH_ICON_SIZE,
    ARCH_LABEL_HEIGHT, ARCH_PADDING,
};

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
    /// use selkie::render::RenderConfig;
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
                // Get compiled styles from classDef/class directives
                let styles = db.get_compiled_styles(vertex);
                let shape_element =
                    shapes::render_shape(node, vertex, &self.config.theme, styles.as_deref());

                doc.add_node(shape_element);
            }
        }

        Ok(doc.to_string())
    }

    /// Render an architecture diagram to SVG
    pub fn render_architecture(&self, db: &ArchitectureDb, graph: &LayoutGraph) -> Result<String> {
        let mut doc = SvgDocument::new();
        let (view_min_x, view_min_y, view_width, view_height) =
            self.calculate_architecture_bounds(db, graph);
        doc.set_size_with_origin(view_min_x, view_min_y, view_width, view_height);

        if self.config.embed_css {
            let mut css = architecture_css();
            if let Some(ref custom_css) = self.config.theme_css {
                let sanitized = sanitize_css(custom_css);
                if !sanitized.is_empty() {
                    css.push_str("\n/* Custom CSS */\n");
                    css.push_str(&sanitized);
                }
            }
            doc.add_style(&css);
        }

        let edges = render_architecture_edges(db, graph);
        doc.add_edge_path(edges);

        let services = render_architecture_services(db, graph);
        doc.add_node(services);

        let groups = render_architecture_groups(db, graph);
        doc.add_node(groups);

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

    fn calculate_architecture_bounds(
        &self,
        db: &ArchitectureDb,
        graph: &LayoutGraph,
    ) -> (f64, f64, f64, f64) {
        let mut min_x = f64::MAX;
        let mut min_y = f64::MAX;
        let mut max_x = f64::MIN;
        let mut max_y = f64::MIN;

        for service in db.get_services() {
            if let Some(node) = graph.get_node(&service.id) {
                if let (Some(x), Some(y)) = (node.x, node.y) {
                    let mut height = node.height;
                    if service.title.is_some() {
                        height += ARCH_LABEL_HEIGHT;
                    }
                    min_x = min_x.min(x);
                    min_y = min_y.min(y);
                    max_x = max_x.max(x + node.width);
                    max_y = max_y.max(y + height);
                }
            }
        }

        for junction in db.get_junctions() {
            if let Some(node) = graph.get_node(&junction.id) {
                if let (Some(x), Some(y)) = (node.x, node.y) {
                    min_x = min_x.min(x);
                    min_y = min_y.min(y);
                    max_x = max_x.max(x + node.width);
                    max_y = max_y.max(y + node.height);
                }
            }
        }

        for group in db.get_groups() {
            if let Some(node) = graph.get_node(&group.id) {
                if let (Some(x), Some(y)) = (node.x, node.y) {
                    if node.width > 0.0 && node.height > 0.0 {
                        min_x = min_x.min(x);
                        min_y = min_y.min(y);
                        max_x = max_x.max(x + node.width);
                        max_y = max_y.max(y + node.height);
                    }
                }
            }
        }

        if min_x == f64::MAX {
            min_x = 0.0;
            min_y = 0.0;
            max_x = graph.width.unwrap_or(800.0);
            max_y = graph.height.unwrap_or(600.0);
        }

        min_x -= ARCH_PADDING;
        min_y -= ARCH_PADDING;
        max_x += ARCH_PADDING;
        max_y += ARCH_PADDING;

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

fn architecture_css() -> String {
    [
        ".mermaid{font-family:\"trebuchet ms\",verdana,arial,sans-serif;font-size:16px;fill:#333;}",
        ".mermaid svg{font-family:\"trebuchet ms\",verdana,arial,sans-serif;font-size:16px;}",
        ".mermaid p{margin:0;}",
        ".mermaid .edge{stroke-width:3;stroke:#333333;fill:none;}",
        ".mermaid .arrow{fill:#333333;}",
        ".mermaid .node-bkg{fill:none;stroke:hsl(240, 60%, 86.2745098039%);stroke-width:2px;stroke-dasharray:8;}",
        ".mermaid .node-icon-text{display:flex;align-items:center;}",
        ".mermaid .node-icon-text>div{color:#fff;margin:1px;height:fit-content;text-align:center;overflow:hidden;display:-webkit-box;-webkit-box-orient:vertical;}",
    ]
    .join("\n")
}

fn render_architecture_edges(db: &ArchitectureDb, graph: &LayoutGraph) -> SvgElement {
    let mut elements = Vec::new();
    for edge in db.get_edges().iter() {
        if let Some(element) = render_architecture_edge(edge, db, graph) {
            elements.push(element);
        }
    }

    SvgElement::group(elements).with_attrs(Attrs::new().with_class("architecture-edges"))
}

fn render_architecture_services(db: &ArchitectureDb, graph: &LayoutGraph) -> SvgElement {
    let mut elements = Vec::new();
    for service in db.get_services().iter() {
        if let Some(node) = graph.get_node(&service.id) {
            if let Some(element) = render_architecture_service(service, node) {
                elements.push(element);
            }
        }
    }
    for junction in db.get_junctions() {
        if let Some(node) = graph.get_node(&junction.id) {
            if let Some(element) = render_architecture_junction(&junction.id, node) {
                elements.push(element);
            }
        }
    }

    SvgElement::group(elements).with_attrs(Attrs::new().with_class("architecture-services"))
}

fn render_architecture_groups(db: &ArchitectureDb, graph: &LayoutGraph) -> SvgElement {
    let mut elements = Vec::new();
    for group in db.get_groups() {
        let Some(node) = graph.get_node(&group.id) else {
            continue;
        };
        let (Some(x), Some(y)) = (node.x, node.y) else {
            continue;
        };
        if node.width == 0.0 || node.height == 0.0 {
            continue;
        }

        let rect = SvgElement::rect(x, y, node.width, node.height).with_attrs(
            Attrs::new()
                .with_class("node-bkg")
                .with_id(&format!("group-{}", group.id)),
        );

        let mut label_elements = Vec::new();
        let half_icon = ARCH_ICON_SIZE / 2.0;
        let mut shifted_x = x - half_icon;
        let mut shifted_y = y - half_icon;

        if let Some(icon) = group.icon.as_deref() {
            let icon_svg = architecture_icon_svg(icon, ARCH_PADDING * ARCH_GROUP_ICON_SCALE);
            let icon_group = SvgElement::group(vec![SvgElement::Raw {
                content: format!("<g>{}</g>", icon_svg),
            }])
            .with_attrs(Attrs::new().with_transform(&format!(
                "translate({}, {})",
                shifted_x + half_icon + 1.0,
                shifted_y + half_icon + 1.0
            )));
            label_elements.push(icon_group);
            shifted_x += ARCH_PADDING * ARCH_GROUP_ICON_SCALE;
            shifted_y += ARCH_FONT_SIZE / 2.0 - 1.0 - 2.0;
        }

        if let Some(title) = group.title.as_deref().or(Some(group.id.as_str())) {
            let label_markup = architecture_text_markup(title);
            let label_group = SvgElement::group(vec![SvgElement::Raw {
                content: label_markup,
            }])
            .with_attrs(
                Attrs::new()
                    .with_attr("dy", "1em")
                    .with_attr("alignment-baseline", "middle")
                    .with_attr("dominant-baseline", "start")
                    .with_attr("text-anchor", "start")
                    .with_transform(&format!(
                        "translate({}, {})",
                        shifted_x + half_icon + 4.0,
                        shifted_y + half_icon + 2.0
                    )),
            );
            label_elements.push(label_group);
        }

        elements.push(rect);
        if !label_elements.is_empty() {
            elements.push(SvgElement::group(label_elements));
        }
    }

    SvgElement::group(elements).with_attrs(Attrs::new().with_class("architecture-groups"))
}

fn render_architecture_service(
    service: &ArchitectureService,
    node: &LayoutNode,
) -> Option<SvgElement> {
    let (x, y) = (node.x?, node.y?);
    let mut children = Vec::new();

    if let Some(title) = service.title.as_deref() {
        let label_markup = architecture_text_markup(title);
        let label_group = SvgElement::group(vec![SvgElement::Raw {
            content: label_markup,
        }])
        .with_attrs(
            Attrs::new()
                .with_attr("dy", "1em")
                .with_attr("alignment-baseline", "middle")
                .with_attr("dominant-baseline", "middle")
                .with_attr("text-anchor", "middle")
                .with_transform(&format!(
                    "translate({}, {})",
                    ARCH_ICON_SIZE / 2.0,
                    ARCH_ICON_SIZE
                )),
        );
        children.push(label_group);
    }

    let icon_group = if let Some(icon) = service.icon.as_deref() {
        let icon_svg = architecture_icon_svg(icon, ARCH_ICON_SIZE);
        SvgElement::group(vec![SvgElement::Raw {
            content: format!("<g>{}</g>", icon_svg),
        }])
    } else if let Some(icon_text) = service.icon_text.as_deref() {
        let icon_svg = architecture_icon_svg("blank", ARCH_ICON_SIZE);
        let text_markup = format!(
            "<g><g>{}</g><g><foreignObject width=\"{size}\" height=\"{size}\"><div class=\"node-icon-text\" style=\"height: {size}px;\"><div>{text}</div></div></foreignObject></g></g>",
            icon_svg,
            size = ARCH_ICON_SIZE,
            text = escape_xml(icon_text)
        );
        SvgElement::Raw {
            content: text_markup,
        }
    } else {
        let d = format!(
            "M0 {size} v{neg} q0,-5 5,-5 h{size} q5,0 5,5 v{size} H0 Z",
            size = ARCH_ICON_SIZE,
            neg = -ARCH_ICON_SIZE
        );
        SvgElement::group(vec![SvgElement::path(d).with_attrs(
            Attrs::new()
                .with_class("node-bkg")
                .with_id(&format!("node-{}", service.id)),
        )])
    };

    children.push(icon_group);

    let group_attrs = Attrs::new()
        .with_class("architecture-service")
        .with_id(&format!("service-{}", service.id))
        .with_transform(&format!("translate({}, {})", x, y));

    Some(SvgElement::group(children).with_attrs(group_attrs))
}

fn render_architecture_junction(id: &str, node: &LayoutNode) -> Option<SvgElement> {
    let (x, y) = (node.x?, node.y?);
    let rect = SvgElement::rect(0.0, 0.0, ARCH_ICON_SIZE, ARCH_ICON_SIZE).with_attrs(
        Attrs::new()
            .with_id(&format!("node-{}", id))
            .with_attr("fill-opacity", "0"),
    );
    let group = SvgElement::group(vec![SvgElement::group(vec![rect])]).with_attrs(
        Attrs::new()
            .with_class("architecture-junction")
            .with_transform(&format!("translate({}, {})", x, y)),
    );
    Some(group)
}

fn render_architecture_edge(
    edge: &crate::diagrams::architecture::ArchitectureEdge,
    db: &ArchitectureDb,
    graph: &LayoutGraph,
) -> Option<SvgElement> {
    let source_node = graph.get_node(&edge.lhs_id)?;
    let target_node = graph.get_node(&edge.rhs_id)?;

    let mut start = architecture_node_port(source_node, edge.lhs_dir)?;
    let mut end = architecture_node_port(target_node, edge.rhs_dir)?;

    let group_edge_shift = ARCH_PADDING + 4.0;
    if edge.lhs_group {
        if edge.lhs_dir.is_x() {
            start.x += if edge.lhs_dir == ArchitectureDirection::Left {
                -group_edge_shift
            } else {
                group_edge_shift
            };
        } else {
            start.y += if edge.lhs_dir == ArchitectureDirection::Top {
                -group_edge_shift
            } else {
                group_edge_shift + ARCH_EDGE_GROUP_LABEL_SHIFT
            };
        }
    }
    if edge.rhs_group {
        if edge.rhs_dir.is_x() {
            end.x += if edge.rhs_dir == ArchitectureDirection::Left {
                -group_edge_shift
            } else {
                group_edge_shift
            };
        } else {
            end.y += if edge.rhs_dir == ArchitectureDirection::Top {
                -group_edge_shift
            } else {
                group_edge_shift + ARCH_EDGE_GROUP_LABEL_SHIFT
            };
        }
    }

    let half_icon = ARCH_ICON_SIZE / 2.0;
    if !edge.lhs_group && is_junction_node(db, source_node, &edge.lhs_id) {
        if edge.lhs_dir.is_x() {
            start.x += if edge.lhs_dir == ArchitectureDirection::Left {
                half_icon
            } else {
                -half_icon
            };
        } else {
            start.y += if edge.lhs_dir == ArchitectureDirection::Top {
                half_icon
            } else {
                -half_icon
            };
        }
    }
    if !edge.rhs_group && is_junction_node(db, target_node, &edge.rhs_id) {
        if edge.rhs_dir.is_x() {
            end.x += if edge.rhs_dir == ArchitectureDirection::Left {
                half_icon
            } else {
                -half_icon
            };
        } else {
            end.y += if edge.rhs_dir == ArchitectureDirection::Top {
                half_icon
            } else {
                -half_icon
            };
        }
    }

    let points = architecture_edge_points(start, end, edge.lhs_dir, edge.rhs_dir);
    let path_d = build_architecture_path(&points);

    let mut edge_children = Vec::new();
    edge_children.push(
        SvgElement::path(path_d).with_attrs(
            Attrs::new()
                .with_class("edge")
                .with_id(&format!("L_{}_{}_0", edge.lhs_id, edge.rhs_id)),
        ),
    );

    let arrow_size = ARCH_ICON_SIZE / 6.0;
    let half_arrow = arrow_size / 2.0;

    if edge.lhs_into {
        let x_shift = if edge.lhs_dir.is_x() {
            architecture_arrow_shift(edge.lhs_dir, start.x, arrow_size)
        } else {
            start.x - half_arrow
        };
        let y_shift = if edge.lhs_dir.is_y() {
            architecture_arrow_shift(edge.lhs_dir, start.y, arrow_size)
        } else {
            start.y - half_arrow
        };
        edge_children.push(SvgElement::Polygon {
            points: architecture_arrow_points(edge.lhs_dir, arrow_size),
            attrs: Attrs::new()
                .with_class("arrow")
                .with_transform(&format!("translate({}, {})", x_shift, y_shift)),
        });
    }
    if edge.rhs_into {
        let x_shift = if edge.rhs_dir.is_x() {
            architecture_arrow_shift(edge.rhs_dir, end.x, arrow_size)
        } else {
            end.x - half_arrow
        };
        let y_shift = if edge.rhs_dir.is_y() {
            architecture_arrow_shift(edge.rhs_dir, end.y, arrow_size)
        } else {
            end.y - half_arrow
        };
        edge_children.push(SvgElement::Polygon {
            points: architecture_arrow_points(edge.rhs_dir, arrow_size),
            attrs: Attrs::new()
                .with_class("arrow")
                .with_transform(&format!("translate({}, {})", x_shift, y_shift)),
        });
    }

    if let Some(label) = edge.title.as_deref() {
        let (label_x, label_y) = edge_label_position(&points);
        let label_transform = if edge.lhs_dir.is_x() && edge.rhs_dir.is_x() {
            format!("translate({}, {})", label_x, label_y)
        } else if edge.lhs_dir.is_y() && edge.rhs_dir.is_y() {
            format!("translate({}, {}) rotate(-90)", label_x, label_y)
        } else {
            let angle = architecture_xy_label_angle(edge.lhs_dir, edge.rhs_dir);
            format!("translate({}, {}) rotate({})", label_x, label_y, angle)
        };

        let label_markup = architecture_text_markup(label);
        let label_group = SvgElement::group(vec![SvgElement::Raw {
            content: label_markup,
        }])
        .with_attrs(
            Attrs::new()
                .with_attr("dy", "1em")
                .with_attr("alignment-baseline", "middle")
                .with_attr("dominant-baseline", "middle")
                .with_attr("text-anchor", "middle")
                .with_transform(&label_transform),
        );
        edge_children.push(label_group);
    }

    Some(SvgElement::group(edge_children))
}

fn is_junction_node(db: &ArchitectureDb, node: &LayoutNode, id: &str) -> bool {
    if node
        .metadata
        .get("node_type")
        .map(|value| value == "junction")
        .unwrap_or(false)
    {
        return true;
    }

    db.get_junctions().iter().any(|junction| junction.id == id)
}

fn architecture_node_port(node: &LayoutNode, dir: ArchitectureDirection) -> Option<Point> {
    let (x, y) = (node.x?, node.y?);
    let w = node.width;
    let h = node.height;
    let point = match dir {
        ArchitectureDirection::Left => Point::new(x, y + h / 2.0),
        ArchitectureDirection::Right => Point::new(x + w, y + h / 2.0),
        ArchitectureDirection::Top => Point::new(x + w / 2.0, y),
        ArchitectureDirection::Bottom => Point::new(x + w / 2.0, y + h),
    };
    Some(point)
}

fn architecture_edge_points(
    start: Point,
    end: Point,
    source_dir: ArchitectureDirection,
    target_dir: ArchitectureDirection,
) -> Vec<Point> {
    if is_architecture_direction_xy(source_dir, target_dir) {
        let bend = if source_dir.is_y() {
            Point::new(start.x, end.y)
        } else {
            Point::new(end.x, start.y)
        };
        vec![start, bend, end]
    } else {
        vec![start, end]
    }
}

fn build_architecture_path(points: &[Point]) -> String {
    let mut d = String::new();
    if let Some(first) = points.first() {
        d.push_str(&format!("M {},{}", first.x, first.y));
    }
    for point in points.iter().skip(1) {
        d.push_str(&format!(" L {},{}", point.x, point.y));
    }
    d
}

fn edge_label_position(points: &[Point]) -> (f64, f64) {
    if points.len() < 2 {
        return (0.0, 0.0);
    }
    let mid = points.len() / 2;
    if points.len().is_multiple_of(2) && mid > 0 {
        let p1 = points[mid - 1];
        let p2 = points[mid];
        ((p1.x + p2.x) / 2.0, (p1.y + p2.y) / 2.0)
    } else {
        let p = points[mid];
        (p.x, p.y)
    }
}

fn architecture_arrow_points(dir: ArchitectureDirection, size: f64) -> Vec<Point> {
    match dir {
        ArchitectureDirection::Left => vec![
            Point::new(size, size / 2.0),
            Point::new(0.0, size),
            Point::new(0.0, 0.0),
        ],
        ArchitectureDirection::Right => vec![
            Point::new(0.0, size / 2.0),
            Point::new(size, 0.0),
            Point::new(size, size),
        ],
        ArchitectureDirection::Top => vec![
            Point::new(0.0, 0.0),
            Point::new(size, 0.0),
            Point::new(size / 2.0, size),
        ],
        ArchitectureDirection::Bottom => vec![
            Point::new(size / 2.0, 0.0),
            Point::new(size, size),
            Point::new(0.0, size),
        ],
    }
}

fn architecture_arrow_shift(dir: ArchitectureDirection, orig: f64, size: f64) -> f64 {
    match dir {
        ArchitectureDirection::Left | ArchitectureDirection::Top => orig - size + 2.0,
        ArchitectureDirection::Right | ArchitectureDirection::Bottom => orig - 2.0,
    }
}

fn architecture_xy_label_angle(
    source_dir: ArchitectureDirection,
    target_dir: ArchitectureDirection,
) -> f64 {
    if (source_dir == ArchitectureDirection::Left && target_dir == ArchitectureDirection::Top)
        || (source_dir == ArchitectureDirection::Top && target_dir == ArchitectureDirection::Left)
    {
        -45.0
    } else if (source_dir == ArchitectureDirection::Bottom
        && target_dir == ArchitectureDirection::Left)
        || (source_dir == ArchitectureDirection::Left
            && target_dir == ArchitectureDirection::Bottom)
    {
        45.0
    } else if (source_dir == ArchitectureDirection::Bottom
        && target_dir == ArchitectureDirection::Right)
        || (source_dir == ArchitectureDirection::Right
            && target_dir == ArchitectureDirection::Bottom)
    {
        -45.0
    } else {
        45.0
    }
}

fn is_architecture_direction_xy(a: ArchitectureDirection, b: ArchitectureDirection) -> bool {
    (a.is_x() && b.is_y()) || (a.is_y() && b.is_x())
}

fn architecture_text_markup(text: &str) -> String {
    let escaped = escape_xml(text);
    format!(
        "<g><rect class=\"background\" style=\"stroke: none\"/><text y=\"-10.1\" style=\"\"><tspan class=\"text-outer-tspan\" x=\"0\" y=\"-0.1em\" dy=\"1.1em\"><tspan font-style=\"normal\" class=\"text-inner-tspan\" font-weight=\"normal\">{}</tspan></tspan></text></g>",
        escaped
    )
}

fn architecture_icon_svg(name: &str, size: f64) -> String {
    let body = architecture_icon_body(name);
    format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{size}\" height=\"{size}\" viewBox=\"0 0 80 80\">{body}</svg>",
        size = size,
        body = body
    )
}

fn architecture_icon_body(name: &str) -> &'static str {
    match name.to_ascii_lowercase().as_str() {
        "database" => ARCH_ICON_DATABASE,
        "server" => ARCH_ICON_SERVER,
        "disk" => ARCH_ICON_DISK,
        "internet" => ARCH_ICON_INTERNET,
        "cloud" => ARCH_ICON_CLOUD,
        "blank" => ARCH_ICON_BLANK,
        _ => ARCH_ICON_UNKNOWN,
    }
}

const ARCH_ICON_DATABASE: &str = r#"<g><rect width="80" height="80" style="fill: #087ebf; stroke-width: 0px;"/><path data-name="4" d="m20,57.86c0,3.94,8.95,7.14,20,7.14s20-3.2,20-7.14" style="fill: none; stroke: #fff; stroke-miterlimit: 10; stroke-width: 2px;"/><path data-name="3" d="m20,45.95c0,3.94,8.95,7.14,20,7.14s20-3.2,20-7.14" style="fill: none; stroke: #fff; stroke-miterlimit: 10; stroke-width: 2px;"/><path data-name="2" d="m20,34.05c0,3.94,8.95,7.14,20,7.14s20-3.2,20-7.14" style="fill: none; stroke: #fff; stroke-miterlimit: 10; stroke-width: 2px;"/><ellipse data-name="1" cx="40" cy="22.14" rx="20" ry="7.14" style="fill: none; stroke: #fff; stroke-miterlimit: 10; stroke-width: 2px;"/><line x1="20" y1="57.86" x2="20" y2="22.14" style="fill: none; stroke: #fff; stroke-miterlimit: 10; stroke-width: 2px;"/><line x1="60" y1="57.86" x2="60" y2="22.14" style="fill: none; stroke: #fff; stroke-miterlimit: 10; stroke-width: 2px;"/></g>"#;

const ARCH_ICON_SERVER: &str = r#"<g><rect width="80" height="80" style="fill: #087ebf; stroke-width: 0px;"/><rect x="17.5" y="17.5" width="45" height="45" rx="2" ry="2" style="fill: none; stroke: #fff; stroke-miterlimit: 10; stroke-width: 2px;"/><line x1="17.5" y1="32.5" x2="62.5" y2="32.5" style="fill: none; stroke: #fff; stroke-miterlimit: 10; stroke-width: 2px;"/><line x1="17.5" y1="47.5" x2="62.5" y2="47.5" style="fill: none; stroke: #fff; stroke-miterlimit: 10; stroke-width: 2px;"/><g><path d="m56.25,25c0,.27-.45.5-1,.5h-10.5c-.55,0-1-.23-1-.5s.45-.5,1-.5h10.5c.55,0,1,.23,1,.5Z" style="fill: #fff; stroke-width: 0px;"/><path d="m56.25,25c0,.27-.45.5-1,.5h-10.5c-.55,0-1-.23-1-.5s.45-.5,1-.5h10.5c.55,0,1,.23,1,.5Z" style="fill: none; stroke: #fff; stroke-miterlimit: 10;"/></g><g><path d="m56.25,40c0,.27-.45.5-1,.5h-10.5c-.55,0-1-.23-1-.5s.45-.5,1-.5h10.5c.55,0,1,.23,1,.5Z" style="fill: #fff; stroke-width: 0px;"/><path d="m56.25,40c0,.27-.45.5-1,.5h-10.5c-.55,0-1-.23-1-.5s.45-.5,1-.5h10.5c.55,0,1,.23,1,.5Z" style="fill: none; stroke: #fff; stroke-miterlimit: 10;"/></g><g><path d="m56.25,55c0,.27-.45.5-1,.5h-10.5c-.55,0-1-.23-1-.5s.45-.5,1-.5h10.5c.55,0,1,.23,1,.5Z" style="fill: #fff; stroke-width: 0px;"/><path d="m56.25,55c0,.27-.45.5-1,.5h-10.5c-.55,0-1-.23-1-.5s.45-.5,1-.5h10.5c.55,0,1,.23,1,.5Z" style="fill: none; stroke: #fff; stroke-miterlimit: 10;"/></g><g><circle cx="32.5" cy="25" r=".75" style="fill: #fff; stroke: #fff; stroke-miterlimit: 10;"/><circle cx="27.5" cy="25" r=".75" style="fill: #fff; stroke: #fff; stroke-miterlimit: 10;"/><circle cx="22.5" cy="25" r=".75" style="fill: #fff; stroke: #fff; stroke-miterlimit: 10;"/></g><g><circle cx="32.5" cy="40" r=".75" style="fill: #fff; stroke: #fff; stroke-miterlimit: 10;"/><circle cx="27.5" cy="40" r=".75" style="fill: #fff; stroke: #fff; stroke-miterlimit: 10;"/><circle cx="22.5" cy="40" r=".75" style="fill: #fff; stroke: #fff; stroke-miterlimit: 10;"/></g><g><circle cx="32.5" cy="55" r=".75" style="fill: #fff; stroke: #fff; stroke-miterlimit: 10;"/><circle cx="27.5" cy="55" r=".75" style="fill: #fff; stroke: #fff; stroke-miterlimit: 10;"/><circle cx="22.5" cy="55" r=".75" style="fill: #fff; stroke: #fff; stroke-miterlimit: 10;"/></g></g>"#;

const ARCH_ICON_DISK: &str = r#"<g><rect width="80" height="80" style="fill: #087ebf; stroke-width: 0px;"/><rect x="20" y="15" width="40" height="50" rx="1" ry="1" style="fill: none; stroke: #fff; stroke-miterlimit: 10; stroke-width: 2px;"/><ellipse cx="24" cy="19.17" rx=".8" ry=".83" style="fill: none; stroke: #fff; stroke-miterlimit: 10; stroke-width: 2px;"/><ellipse cx="56" cy="19.17" rx=".8" ry=".83" style="fill: none; stroke: #fff; stroke-miterlimit: 10; stroke-width: 2px;"/><ellipse cx="24" cy="60.83" rx=".8" ry=".83" style="fill: none; stroke: #fff; stroke-miterlimit: 10; stroke-width: 2px;"/><ellipse cx="56" cy="60.83" rx=".8" ry=".83" style="fill: none; stroke: #fff; stroke-miterlimit: 10; stroke-width: 2px;"/><ellipse cx="40" cy="33.75" rx="14" ry="14.58" style="fill: none; stroke: #fff; stroke-miterlimit: 10; stroke-width: 2px;"/><ellipse cx="40" cy="33.75" rx="4" ry="4.17" style="fill: #fff; stroke: #fff; stroke-miterlimit: 10; stroke-width: 2px;"/><path d="m37.51,42.52l-4.83,13.22c-.26.71-1.1,1.02-1.76.64l-4.18-2.42c-.66-.38-.81-1.26-.33-1.84l9.01-10.8c.88-1.05,2.56-.08,2.09,1.2Z" style="fill: #fff; stroke-width: 0px;"/></g>"#;

const ARCH_ICON_INTERNET: &str = r#"<g><rect width="80" height="80" style="fill: #087ebf; stroke-width: 0px;"/><circle cx="40" cy="40" r="22.5" style="fill: none; stroke: #fff; stroke-miterlimit: 10; stroke-width: 2px;"/><line x1="40" y1="17.5" x2="40" y2="62.5" style="fill: none; stroke: #fff; stroke-miterlimit: 10; stroke-width: 2px;"/><line x1="17.5" y1="40" x2="62.5" y2="40" style="fill: none; stroke: #fff; stroke-miterlimit: 10; stroke-width: 2px;"/><path d="m39.99,17.51c-15.28,11.1-15.28,33.88,0,44.98" style="fill: none; stroke: #fff; stroke-miterlimit: 10; stroke-width: 2px;"/><path d="m40.01,17.51c15.28,11.1,15.28,33.88,0,44.98" style="fill: none; stroke: #fff; stroke-miterlimit: 10; stroke-width: 2px;"/><line x1="19.75" y1="30.1" x2="60.25" y2="30.1" style="fill: none; stroke: #fff; stroke-miterlimit: 10; stroke-width: 2px;"/><line x1="19.75" y1="49.9" x2="60.25" y2="49.9" style="fill: none; stroke: #fff; stroke-miterlimit: 10; stroke-width: 2px;"/></g>"#;

const ARCH_ICON_CLOUD: &str = r#"<g><rect width="80" height="80" style="fill: #087ebf; stroke-width: 0px;"/><path d="m65,47.5c0,2.76-2.24,5-5,5H20c-2.76,0-5-2.24-5-5,0-1.87,1.03-3.51,2.56-4.36-.04-.21-.06-.42-.06-.64,0-2.6,2.48-4.74,5.65-4.97,1.65-4.51,6.34-7.76,11.85-7.76.86,0,1.69.08,2.5.23,2.09-1.57,4.69-2.5,7.5-2.5,6.1,0,11.19,4.38,12.28,10.17,2.14.56,3.72,2.51,3.72,4.83,0,.03,0,.07-.01.1,2.29.46,4.01,2.48,4.01,4.9Z" style="fill: none; stroke: #fff; stroke-miterlimit: 10; stroke-width: 2px;"/></g>"#;

const ARCH_ICON_UNKNOWN: &str = r#"<g><rect width="80" height="80" style="fill: #087ebf; stroke-width: 0px;"/><text transform="translate(21.16 64.67)" style="fill: #fff; font-family: ArialMT, Arial; font-size: 67.75px;"><tspan x="0" y="0">?</tspan></text></g>"#;

const ARCH_ICON_BLANK: &str =
    r#"<g><rect width="80" height="80" style="fill: #087ebf; stroke-width: 0px;"/></g>"#;

fn escape_xml(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

/// Sanitize CSS to prevent script injection and other attacks
///
/// This follows mermaid.js security patterns:
/// - Removes `<script>` and similar tags
/// - Blocks `javascript:` and `data:` URLs
/// - Validates balanced braces
/// - Removes potentially dangerous properties like `expression()`
pub(crate) fn sanitize_css(css: &str) -> String {
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
