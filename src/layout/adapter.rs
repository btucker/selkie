//! Adapter traits for converting diagrams to layout graphs

use super::graph::LayoutGraph;
use super::types::NodeShape;
use crate::error::Result;

/// Configuration for node size estimation
#[derive(Debug, Clone)]
pub struct NodeSizeConfig {
    /// Base font size for text
    pub font_size: f64,
    /// Node padding around label content, matching mermaid's `node.padding`
    /// (flowchart default 15, see mermaid config.schema.yaml). Shape formulas
    /// derive their spacing from this single value (e.g. rects use 2x
    /// horizontally, 1x vertically).
    pub padding: f64,
    /// Label wrapping width, matching mermaid's `flowchart.wrappingWidth`
    /// (default 200, see mermaid config.schema.yaml). HTML labels wider than
    /// this are greedily word-wrapped at this pixel width.
    pub wrapping_width: f64,
    /// Horizontal padding around text (legacy estimators only)
    pub padding_horizontal: f64,
    /// Vertical padding around text (legacy estimators only)
    pub padding_vertical: f64,
    /// Minimum node width (legacy estimators only; mermaid has no clamp)
    pub min_width: f64,
    /// Minimum node height (legacy estimators only; mermaid has no clamp)
    pub min_height: f64,
    /// Maximum node width (legacy estimators only; mermaid has no clamp)
    pub max_width: Option<f64>,
}

impl Default for NodeSizeConfig {
    fn default() -> Self {
        Self {
            // mermaid.js uses 16px as the default node font size
            font_size: 16.0,
            // mermaid.js flowchart node.padding default
            padding: 15.0,
            // mermaid.js flowchart.wrappingWidth default
            wrapping_width: 200.0,
            padding_horizontal: 16.0,
            padding_vertical: 8.0,
            min_width: 50.0,
            min_height: 30.0,
            max_width: Some(300.0),
        }
    }
}

/// Trait for estimating node sizes before layout
pub trait SizeEstimator {
    /// Estimate text dimensions given content and font size
    fn estimate_text_size(&self, text: &str, font_size: f64) -> (f64, f64);

    /// Estimate node dimensions given label, shape, and config
    fn estimate_node_size(
        &self,
        label: Option<&str>,
        shape: NodeShape,
        config: &NodeSizeConfig,
    ) -> (f64, f64);

    /// Estimate edge label dimensions
    fn estimate_edge_label_size(&self, label: &str, font_size: f64) -> (f64, f64) {
        let (w, h) = self.estimate_text_size(label, font_size);
        (w + 8.0, h + 4.0) // Small padding for edge labels
    }
}

/// Trait for converting a diagram to a layout graph
pub trait ToLayoutGraph {
    /// Convert this diagram to a layout graph for positioning
    fn to_layout_graph(&self, size_estimator: &dyn SizeEstimator) -> Result<LayoutGraph>;

    /// Get the preferred layout direction for this diagram type
    fn preferred_direction(&self) -> super::types::LayoutDirection;
}
