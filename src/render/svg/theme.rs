//! Theme configuration for SVG rendering

/// Color theme for diagram rendering
#[derive(Debug, Clone)]
pub struct Theme {
    // === Common colors ===
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

    // === Pie chart colors ===
    /// Pie chart color palette (pie1-pie12)
    pub pie_colors: Vec<String>,
    /// Pie chart stroke color
    pub pie_stroke_color: String,
    /// Pie chart outer stroke color
    pub pie_outer_stroke_color: String,
    /// Pie chart slice opacity
    pub pie_opacity: String,
    /// Pie chart title text color
    pub pie_title_text_color: String,
    /// Pie chart legend text color
    pub pie_legend_text_color: String,

    // === Sequence diagram colors ===
    /// Actor box background color
    pub actor_bkg: String,
    /// Actor box border color
    pub actor_border: String,
    /// Actor text color
    pub actor_text_color: String,
    /// Actor lifeline color
    pub actor_line_color: String,
    /// Signal/message line color
    pub signal_color: String,
    /// Signal/message text color
    pub signal_text_color: String,
    /// Note background color
    pub note_bkg_color: String,
    /// Note border color
    pub note_border_color: String,
    /// Note text color
    pub note_text_color: String,
    /// Activation box background color
    pub activation_bkg_color: String,
    /// Activation box border color
    pub activation_border_color: String,
    /// Loop/box label background color
    pub label_box_bkg_color: String,
    /// Loop/box label border color
    pub label_box_border_color: String,

    // === Gantt chart colors ===
    /// Section background color (odd rows)
    pub section_bkg_color: String,
    /// Section background color (even rows)
    pub section_bkg_color2: String,
    /// Task bar background color
    pub task_bkg_color: String,
    /// Task bar border color
    pub task_border_color: String,
    /// Task text color (light, for dark backgrounds)
    pub task_text_light_color: String,
    /// Task text color (dark, for light backgrounds)
    pub task_text_dark_color: String,
    /// Active task background color
    pub active_task_bkg_color: String,
    /// Active task border color
    pub active_task_border_color: String,
    /// Done task background color
    pub done_task_bkg_color: String,
    /// Done task border color
    pub done_task_border_color: String,
    /// Critical task background color
    pub crit_bkg_color: String,
    /// Critical task border color
    pub crit_border_color: String,
    /// Grid line color
    pub grid_color: String,
    /// Today line color
    pub today_line_color: String,
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
            cluster_border_color: "#aaaa33".to_string(),
            line_color: "#333333".to_string(),
            background: "#ffffff".to_string(),
            font_family: "trebuchet ms, verdana, arial, sans-serif".to_string(),
            font_size: "16px".to_string(),
            // Pie chart - default theme (mermaid.js derived from primary/secondary)
            pie_colors: vec![
                "#ECECFF".to_string(), // pie1 - primary
                "#ffffde".to_string(), // pie2 - secondary
                "#b9b9ff".to_string(), // pie3 - tertiary
                "#b5ff20".to_string(), // pie4
                "#d4ffb2".to_string(), // pie5
                "#ffb3e6".to_string(), // pie6
                "#ffd700".to_string(), // pie7
                "#c4c4ff".to_string(), // pie8
                "#ffe6cc".to_string(), // pie9
                "#ccffcc".to_string(), // pie10
            ],
            pie_stroke_color: "black".to_string(),
            pie_outer_stroke_color: "black".to_string(),
            pie_opacity: "0.7".to_string(),
            pie_title_text_color: "#333333".to_string(),
            pie_legend_text_color: "#333333".to_string(),
            // Sequence diagram - default theme
            actor_bkg: "#ECECFF".to_string(),
            actor_border: "#9370DB".to_string(),
            actor_text_color: "#333333".to_string(),
            actor_line_color: "#333333".to_string(),
            signal_color: "#333333".to_string(),
            signal_text_color: "#333333".to_string(),
            note_bkg_color: "#FFFFCC".to_string(),
            note_border_color: "#aaaa33".to_string(),
            note_text_color: "#333333".to_string(),
            activation_bkg_color: "#eaeaea".to_string(),
            activation_border_color: "#333333".to_string(),
            label_box_bkg_color: "#fff5ad".to_string(),
            label_box_border_color: "#aaaa33".to_string(),
            // Gantt chart - default theme (mermaid.js purple palette)
            section_bkg_color: "#fff400".to_string(),
            section_bkg_color2: "#ffffff".to_string(),
            task_bkg_color: "#8a90dd".to_string(),
            task_border_color: "#534fbc".to_string(),
            task_text_light_color: "#ffffff".to_string(),
            task_text_dark_color: "#000000".to_string(),
            active_task_bkg_color: "#bfc7ff".to_string(),
            active_task_border_color: "#534fbc".to_string(),
            done_task_bkg_color: "#d3d3d3".to_string(),
            done_task_border_color: "#808080".to_string(),
            crit_bkg_color: "#ff0000".to_string(),
            crit_border_color: "#ff8888".to_string(),
            grid_color: "#d3d3d3".to_string(),
            today_line_color: "#ff0000".to_string(),
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
            // Pie chart - dark theme (lighter colors for dark background)
            pie_colors: vec![
                "#1f2020".to_string(), // pie1 - primary (dark)
                "#8a8a8a".to_string(), // pie2 - secondary
                "#333333".to_string(), // pie3 - tertiary
                "#5f9ea0".to_string(), // pie4 - cadet blue
                "#6b8e23".to_string(), // pie5 - olive
                "#b8860b".to_string(), // pie6 - dark goldenrod
                "#8b4513".to_string(), // pie7 - saddle brown
                "#4682b4".to_string(), // pie8 - steel blue
                "#9932cc".to_string(), // pie9 - dark orchid
                "#2f4f4f".to_string(), // pie10 - dark slate gray
            ],
            pie_stroke_color: "#81B1DB".to_string(),
            pie_outer_stroke_color: "#81B1DB".to_string(),
            pie_opacity: "0.7".to_string(),
            pie_title_text_color: "#ccc".to_string(),
            pie_legend_text_color: "#ccc".to_string(),
            // Sequence diagram - dark theme
            actor_bkg: "#1f2020".to_string(),
            actor_border: "#81B1DB".to_string(),
            actor_text_color: "#ccc".to_string(),
            actor_line_color: "#81B1DB".to_string(),
            signal_color: "#81B1DB".to_string(),
            signal_text_color: "#ccc".to_string(),
            note_bkg_color: "#3d3d3d".to_string(),
            note_border_color: "#81B1DB".to_string(),
            note_text_color: "#ccc".to_string(),
            activation_bkg_color: "#333333".to_string(),
            activation_border_color: "#81B1DB".to_string(),
            label_box_bkg_color: "#2d2d2d".to_string(),
            label_box_border_color: "#81B1DB".to_string(),
            // Gantt chart - dark theme
            section_bkg_color: "#3d3d3d".to_string(),
            section_bkg_color2: "#2d2d2d".to_string(),
            task_bkg_color: "#4a5568".to_string(),
            task_border_color: "#81B1DB".to_string(),
            task_text_light_color: "#ffffff".to_string(),
            task_text_dark_color: "#ccc".to_string(),
            active_task_bkg_color: "#5a6a7a".to_string(),
            active_task_border_color: "#81B1DB".to_string(),
            done_task_bkg_color: "#555555".to_string(),
            done_task_border_color: "#666666".to_string(),
            crit_bkg_color: "#8b0000".to_string(),
            crit_border_color: "#ff6666".to_string(),
            grid_color: "#444444".to_string(),
            today_line_color: "#ff6666".to_string(),
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
            // Pie chart - neutral theme (grayscale palette)
            pie_colors: vec![
                "#f0f0f0".to_string(), // pie1 - primary
                "#e0e0e0".to_string(), // pie2 - secondary
                "#d0d0d0".to_string(), // pie3
                "#c0c0c0".to_string(), // pie4
                "#b0b0b0".to_string(), // pie5
                "#a0a0a0".to_string(), // pie6
                "#909090".to_string(), // pie7
                "#808080".to_string(), // pie8
                "#707070".to_string(), // pie9
                "#606060".to_string(), // pie10
            ],
            pie_stroke_color: "#333333".to_string(),
            pie_outer_stroke_color: "#333333".to_string(),
            pie_opacity: "0.7".to_string(),
            pie_title_text_color: "#333333".to_string(),
            pie_legend_text_color: "#333333".to_string(),
            // Sequence diagram - neutral theme (grayscale)
            actor_bkg: "#f0f0f0".to_string(),
            actor_border: "#666666".to_string(),
            actor_text_color: "#333333".to_string(),
            actor_line_color: "#666666".to_string(),
            signal_color: "#666666".to_string(),
            signal_text_color: "#333333".to_string(),
            note_bkg_color: "#fafafa".to_string(),
            note_border_color: "#999999".to_string(),
            note_text_color: "#333333".to_string(),
            activation_bkg_color: "#e0e0e0".to_string(),
            activation_border_color: "#666666".to_string(),
            label_box_bkg_color: "#f5f5f5".to_string(),
            label_box_border_color: "#999999".to_string(),
            // Gantt chart - neutral theme (grayscale)
            section_bkg_color: "#e8e8e8".to_string(),
            section_bkg_color2: "#f8f8f8".to_string(),
            task_bkg_color: "#a0a0a0".to_string(),
            task_border_color: "#666666".to_string(),
            task_text_light_color: "#ffffff".to_string(),
            task_text_dark_color: "#333333".to_string(),
            active_task_bkg_color: "#c0c0c0".to_string(),
            active_task_border_color: "#666666".to_string(),
            done_task_bkg_color: "#d0d0d0".to_string(),
            done_task_border_color: "#909090".to_string(),
            crit_bkg_color: "#606060".to_string(),
            crit_border_color: "#404040".to_string(),
            grid_color: "#cccccc".to_string(),
            today_line_color: "#333333".to_string(),
        }
    }

    /// Create a forest theme (nature-inspired green palette)
    pub fn forest() -> Self {
        Self {
            // Green nature-inspired palette from mermaid.js theme-forest.js
            primary_color: "#cde498".to_string(),
            primary_text_color: "#333333".to_string(),
            primary_border_color: "#13540c".to_string(),
            secondary_color: "#cdffb2".to_string(),
            tertiary_color: "#e0f2c8".to_string(),
            cluster_border_color: "#6eaa49".to_string(),
            line_color: "#008000".to_string(),
            background: "#ffffff".to_string(),
            font_family: "trebuchet ms, verdana, arial, sans-serif".to_string(),
            font_size: "16px".to_string(),
            // Pie chart - forest theme (green palette)
            pie_colors: vec![
                "#cde498".to_string(), // pie1 - primary light green
                "#cdffb2".to_string(), // pie2 - secondary mint
                "#6eaa49".to_string(), // pie3 - medium green
                "#487e3a".to_string(), // pie4 - darker green
                "#13540c".to_string(), // pie5 - dark green
                "#98d439".to_string(), // pie6 - lime
                "#4caf50".to_string(), // pie7 - material green
                "#8bc34a".to_string(), // pie8 - light green
                "#009688".to_string(), // pie9 - teal
                "#00695c".to_string(), // pie10 - dark teal
            ],
            pie_stroke_color: "black".to_string(),
            pie_outer_stroke_color: "black".to_string(),
            pie_opacity: "0.7".to_string(),
            pie_title_text_color: "#333333".to_string(),
            pie_legend_text_color: "#333333".to_string(),
            // Sequence diagram - forest theme (green palette)
            actor_bkg: "#cde498".to_string(),
            actor_border: "#13540c".to_string(),
            actor_text_color: "#333333".to_string(),
            actor_line_color: "#008000".to_string(),
            signal_color: "#008000".to_string(),
            signal_text_color: "#333333".to_string(),
            note_bkg_color: "#cdffb2".to_string(),
            note_border_color: "#6eaa49".to_string(),
            note_text_color: "#333333".to_string(),
            activation_bkg_color: "#e0f2c8".to_string(),
            activation_border_color: "#13540c".to_string(),
            label_box_bkg_color: "#cdffb2".to_string(),
            label_box_border_color: "#6eaa49".to_string(),
            // Gantt chart - forest theme (green palette from mermaid.js)
            section_bkg_color: "#6eaa49".to_string(),
            section_bkg_color2: "#ffffff".to_string(),
            task_bkg_color: "#487e3a".to_string(),
            task_border_color: "#13540c".to_string(),
            task_text_light_color: "#ffffff".to_string(),
            task_text_dark_color: "#333333".to_string(),
            active_task_bkg_color: "#cde498".to_string(),
            active_task_border_color: "#13540c".to_string(),
            done_task_bkg_color: "#d3d3d3".to_string(),
            done_task_border_color: "#808080".to_string(),
            crit_bkg_color: "#ff0000".to_string(),
            crit_border_color: "#ff8888".to_string(),
            grid_color: "#6eaa49".to_string(),
            today_line_color: "#ff0000".to_string(),
        }
    }

    /// Create a base theme (neutral foundation for customization)
    /// This theme provides neutral starting points that can be fully
    /// customized via themeVariables overrides.
    pub fn base() -> Self {
        Self {
            // Neutral warm palette from mermaid.js theme-base.js
            primary_color: "#fff4dd".to_string(),
            primary_text_color: "#333333".to_string(),
            primary_border_color: "#9370DB".to_string(),
            secondary_color: "#dde4ff".to_string(),
            tertiary_color: "#f4ffdd".to_string(),
            cluster_border_color: "#9370DB".to_string(),
            line_color: "#333333".to_string(),
            background: "#f4f4f4".to_string(),
            font_family: "trebuchet ms, verdana, arial, sans-serif".to_string(),
            font_size: "16px".to_string(),
            // Pie chart - base theme (warm pastels)
            pie_colors: vec![
                "#fff4dd".to_string(), // pie1 - primary warm cream
                "#dde4ff".to_string(), // pie2 - secondary light blue
                "#f4ffdd".to_string(), // pie3 - tertiary light green
                "#ffe4dd".to_string(), // pie4 - light coral
                "#e4ddff".to_string(), // pie5 - light purple
                "#ddfff4".to_string(), // pie6 - light mint
                "#fff0b3".to_string(), // pie7 - light gold
                "#ffddee".to_string(), // pie8 - light pink
                "#ddf4ff".to_string(), // pie9 - light cyan
                "#f4ddff".to_string(), // pie10 - light magenta
            ],
            pie_stroke_color: "black".to_string(),
            pie_outer_stroke_color: "black".to_string(),
            pie_opacity: "0.7".to_string(),
            pie_title_text_color: "#333333".to_string(),
            pie_legend_text_color: "#333333".to_string(),
            // Sequence diagram - base theme (warm pastels)
            actor_bkg: "#fff4dd".to_string(),
            actor_border: "#9370DB".to_string(),
            actor_text_color: "#333333".to_string(),
            actor_line_color: "#333333".to_string(),
            signal_color: "#333333".to_string(),
            signal_text_color: "#333333".to_string(),
            note_bkg_color: "#fff5ad".to_string(),
            note_border_color: "#9370DB".to_string(),
            note_text_color: "#333333".to_string(),
            activation_bkg_color: "#dde4ff".to_string(),
            activation_border_color: "#9370DB".to_string(),
            label_box_bkg_color: "#f4ffdd".to_string(),
            label_box_border_color: "#9370DB".to_string(),
            // Gantt chart - base theme (warm neutral palette)
            section_bkg_color: "#fff4dd".to_string(),
            section_bkg_color2: "#ffffff".to_string(),
            task_bkg_color: "#dde4ff".to_string(),
            task_border_color: "#9370DB".to_string(),
            task_text_light_color: "#ffffff".to_string(),
            task_text_dark_color: "#333333".to_string(),
            active_task_bkg_color: "#f4ffdd".to_string(),
            active_task_border_color: "#9370DB".to_string(),
            done_task_bkg_color: "#d3d3d3".to_string(),
            done_task_border_color: "#808080".to_string(),
            crit_bkg_color: "#ff0000".to_string(),
            crit_border_color: "#ff8888".to_string(),
            grid_color: "#cccccc".to_string(),
            today_line_color: "#9370DB".to_string(),
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
