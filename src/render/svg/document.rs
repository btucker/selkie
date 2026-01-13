//! SVG document builder

use super::elements::{Attrs, SvgElement};
use std::fmt;

/// SVG document builder
#[derive(Debug, Clone)]
pub struct SvgDocument {
    /// Document width
    width: f64,
    /// Document height
    height: f64,
    /// View box (minX, minY, width, height)
    view_box: Option<(f64, f64, f64, f64)>,
    /// Style content
    styles: Vec<String>,
    /// Definition elements (markers, gradients, etc.)
    defs: Vec<SvgElement>,
    /// Cluster/subgraph elements (rendered first, behind everything)
    clusters: Vec<SvgElement>,
    /// Edge path elements
    edge_paths: Vec<SvgElement>,
    /// Edge label elements
    edge_labels: Vec<SvgElement>,
    /// Node elements (rendered last, on top)
    nodes: Vec<SvgElement>,
    /// Legacy element storage (for backwards compatibility)
    elements: Vec<SvgElement>,
}

impl SvgDocument {
    pub fn new() -> Self {
        Self {
            width: 800.0,
            height: 600.0,
            view_box: None,
            styles: Vec::new(),
            defs: Vec::new(),
            clusters: Vec::new(),
            edge_paths: Vec::new(),
            edge_labels: Vec::new(),
            nodes: Vec::new(),
            elements: Vec::new(),
        }
    }

    /// Set the document size
    pub fn set_size(&mut self, width: f64, height: f64) {
        self.width = width;
        self.height = height;
        self.view_box = Some((0.0, 0.0, width, height));
    }

    /// Set the document size with custom viewBox origin
    /// Use this when content has negative coordinates
    pub fn set_size_with_origin(&mut self, min_x: f64, min_y: f64, width: f64, height: f64) {
        self.width = width;
        self.height = height;
        self.view_box = Some((min_x, min_y, width, height));
    }

    /// Add a style block
    pub fn add_style(&mut self, css: &str) {
        self.styles.push(css.to_string());
    }

    /// Add definition elements (markers, etc.)
    pub fn add_defs(&mut self, elements: Vec<SvgElement>) {
        self.defs.extend(elements);
    }

    /// Add a content element (legacy - adds to nodes group)
    pub fn add_element(&mut self, element: SvgElement) {
        self.elements.push(element);
    }

    /// Add a cluster/subgraph element
    pub fn add_cluster(&mut self, element: SvgElement) {
        self.clusters.push(element);
    }

    /// Add an edge path element
    pub fn add_edge_path(&mut self, element: SvgElement) {
        self.edge_paths.push(element);
    }

    /// Add an edge label element
    pub fn add_edge_label(&mut self, element: SvgElement) {
        self.edge_labels.push(element);
    }

    /// Add a node element
    pub fn add_node(&mut self, element: SvgElement) {
        self.nodes.push(element);
    }
}

impl Default for SvgDocument {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for SvgDocument {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // XML declaration
        writeln!(f, "<?xml version=\"1.0\" encoding=\"UTF-8\"?>")?;

        // SVG opening tag
        let view_box_str = self
            .view_box
            .map(|(x, y, w, h)| format!(" viewBox=\"{} {} {} {}\"", x, y, w, h))
            .unwrap_or_default();

        writeln!(
            f,
            "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\"{} class=\"mermaid\">",
            self.width, self.height, view_box_str
        )?;

        // Styles
        if !self.styles.is_empty() {
            writeln!(f, "  <style>")?;
            for style in &self.styles {
                writeln!(f, "{}", style)?;
            }
            writeln!(f, "  </style>")?;
        }

        // Defs
        if !self.defs.is_empty() {
            writeln!(f, "  <defs>")?;
            for def in &self.defs {
                writeln!(f, "{}", def.to_svg(2))?;
            }
            writeln!(f, "  </defs>")?;
        }

        // Content group (root)
        writeln!(f, "  <g class=\"root\">")?;

        // Container groups in mermaid.js order
        for (class, elements) in [
            ("clusters", &self.clusters),
            ("edgePaths", &self.edge_paths),
            ("edgeLabels", &self.edge_labels),
            ("nodes", &self.nodes),
        ] {
            let group =
                SvgElement::group(elements.to_vec()).with_attrs(Attrs::new().with_class(class));
            writeln!(f, "{}", group.to_svg(2))?;
        }

        // Legacy elements
        for element in &self.elements {
            writeln!(f, "{}", element.to_svg(2))?;
        }

        writeln!(f, "  </g>")?;
        writeln!(f, "</svg>")
    }
}
