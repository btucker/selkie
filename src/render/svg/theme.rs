//! Theme configuration for SVG rendering

/// Color theme for diagram rendering
#[derive(Debug, Clone)]
pub struct Theme {
    /// Primary node fill color
    pub primary_color: String,
    /// Primary text color
    pub primary_text_color: String,
    /// Primary border color
    pub primary_border_color: String,
    /// Secondary node color
    pub secondary_color: String,
    /// Tertiary color (subgraph backgrounds)
    pub tertiary_color: String,
    /// Cluster/subgraph border color
    pub cluster_border_color: String,
    /// Edge/line color
    pub line_color: String,
    /// Background color
    pub background: String,
    /// Font family
    pub font_family: String,
    /// Base font size
    pub font_size: String,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            // Default mermaid theme colors
            primary_color: "#ECECFF".to_string(),
            primary_text_color: "#333333".to_string(),
            primary_border_color: "#9370DB".to_string(),
            secondary_color: "#ffffde".to_string(),
            tertiary_color: "#fafafa".to_string(),
            cluster_border_color: "#aaaa33".to_string(), // Yellow-green for subgraph borders
            line_color: "#333333".to_string(),
            background: "#ffffff".to_string(),
            font_family: "trebuchet ms, verdana, arial, sans-serif".to_string(),
            font_size: "16px".to_string(),
        }
    }
}

impl Theme {
    /// Create a dark theme
    pub fn dark() -> Self {
        Self {
            primary_color: "#1f2020".to_string(),
            primary_text_color: "#ccc".to_string(),
            primary_border_color: "#81B1DB".to_string(),
            secondary_color: "#8a8a8a".to_string(),
            tertiary_color: "#333333".to_string(),
            cluster_border_color: "#666666".to_string(),
            line_color: "#81B1DB".to_string(),
            background: "#1f2020".to_string(),
            font_family: "trebuchet ms, verdana, arial, sans-serif".to_string(),
            font_size: "16px".to_string(),
        }
    }

    /// Create a neutral theme
    pub fn neutral() -> Self {
        Self {
            primary_color: "#f0f0f0".to_string(),
            primary_text_color: "#333333".to_string(),
            primary_border_color: "#666666".to_string(),
            secondary_color: "#e0e0e0".to_string(),
            tertiary_color: "#fafafa".to_string(),
            cluster_border_color: "#999999".to_string(),
            line_color: "#666666".to_string(),
            background: "#ffffff".to_string(),
            font_family: "trebuchet ms, verdana, arial, sans-serif".to_string(),
            font_size: "16px".to_string(),
        }
    }

    /// Create a forest theme (nature-inspired green palette)
    pub fn forest() -> Self {
        Self {
            // Green nature-inspired palette from mermaid.js theme-forest.js
            primary_color: "#cde498".to_string(),      // Light green (mainBkg)
            primary_text_color: "#333333".to_string(), // Dark text on light background
            primary_border_color: "#13540c".to_string(), // Dark green (border1)
            secondary_color: "#cdffb2".to_string(),    // Mint green (secondBkg)
            tertiary_color: "#e0f2c8".to_string(),     // Lighter green
            cluster_border_color: "#6eaa49".to_string(), // Medium green (border2)
            line_color: "#008000".to_string(),         // Green for edges
            background: "#ffffff".to_string(),
            font_family: "trebuchet ms, verdana, arial, sans-serif".to_string(),
            font_size: "16px".to_string(),
        }
    }

    /// Create a base theme (neutral foundation for customization)
    /// This theme provides neutral starting points that can be fully
    /// customized via themeVariables overrides.
    pub fn base() -> Self {
        Self {
            // Neutral warm palette from mermaid.js theme-base.js
            primary_color: "#fff4dd".to_string(),      // Light warm cream
            primary_text_color: "#333333".to_string(), // Dark text
            primary_border_color: "#9370DB".to_string(), // Muted purple border
            secondary_color: "#dde4ff".to_string(),    // Light blue-purple
            tertiary_color: "#f4ffdd".to_string(),     // Light yellow-green
            cluster_border_color: "#9370DB".to_string(), // Muted purple
            line_color: "#333333".to_string(),         // Dark lines
            background: "#f4f4f4".to_string(),         // Light gray background
            font_family: "trebuchet ms, verdana, arial, sans-serif".to_string(),
            font_size: "16px".to_string(),
        }
    }

    /// Generate CSS for embedding in SVG
    pub fn generate_css(&self) -> String {
        format!(
            r#"
.mermaid {{
  font-family: {font_family};
  font-size: {font_size};
}}

.node rect,
.node polygon,
.node circle,
.node ellipse,
.node path {{
  fill: {primary_color};
  stroke: {primary_border_color};
  stroke-width: 1px;
}}

.node line {{
  stroke: {primary_border_color};
  stroke-width: 1px;
}}

.node .label {{
  fill: {primary_text_color};
}}

.node text {{
  fill: {primary_text_color};
  font-family: {font_family};
  font-size: {font_size};
}}

.edge-path {{
  fill: none;
  stroke: {line_color};
  stroke-width: 1px;
}}

.edge-label {{
  fill: {primary_text_color};
  font-family: {font_family};
  font-size: 12px;
}}

.edge-label-bg {{
  fill: {background};
}}

.subgraph {{
  fill: {secondary_color};
  stroke: {cluster_border_color};
  stroke-width: 1px;
}}

.subgraph-title {{
  fill: {primary_text_color};
  font-weight: bold;
}}

.cluster rect {{
  fill: {secondary_color};
  stroke: {cluster_border_color};
  stroke-width: 1px;
  rx: 5px;
  ry: 5px;
}}

.cluster-label {{
  fill: {primary_text_color};
  font-family: {font_family};
  font-size: {font_size};
  font-weight: bold;
}}

marker path {{
  fill: {line_color};
  stroke: {line_color};
}}
"#,
            font_family = self.font_family,
            font_size = self.font_size,
            primary_color = self.primary_color,
            primary_border_color = self.primary_border_color,
            primary_text_color = self.primary_text_color,
            secondary_color = self.secondary_color,
            cluster_border_color = self.cluster_border_color,
            line_color = self.line_color,
            background = self.background,
        )
    }
}
