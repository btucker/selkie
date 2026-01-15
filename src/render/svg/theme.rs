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
    /// Edge label background color
    pub edge_label_background: String,
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
            edge_label_background: "rgba(232, 232, 232, 0.8)".to_string(),
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
            edge_label_background: "#4a4a4a".to_string(),
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
            edge_label_background: "white".to_string(),
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
            edge_label_background: "#e8e8e8".to_string(),
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
            edge_label_background: "rgba(232, 232, 232, 0.8)".to_string(),
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
}}

.edge-label-bg {{
  fill: {edge_label_background};
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
            edge_label_background = self.edge_label_background,
        )
    }

    /// Create a theme from base colors with automatic derivation
    ///
    /// This follows the mermaid.js pattern where setting just a few base colors
    /// automatically derives all other colors for consistency.
    ///
    /// # Arguments
    /// * `primary` - Primary color (node fills)
    /// * `secondary` - Secondary color (subgraph backgrounds)
    /// * `tertiary` - Tertiary color (alternate backgrounds)
    /// * `background` - Background color
    /// * `dark_mode` - Whether to derive colors for dark mode
    pub fn from_base_colors(
        primary: &str,
        secondary: &str,
        tertiary: &str,
        background: &str,
        dark_mode: bool,
    ) -> Self {
        use super::color::{self, Color};

        let primary_color =
            Color::parse(primary).unwrap_or_else(|| Color::from_hex("#ECECFF").unwrap());
        let secondary_color =
            Color::parse(secondary).unwrap_or_else(|| Color::from_hex("#ffffde").unwrap());
        let tertiary_color =
            Color::parse(tertiary).unwrap_or_else(|| Color::from_hex("#fafafa").unwrap());
        let bg_color =
            Color::parse(background).unwrap_or_else(|| Color::from_hex("#ffffff").unwrap());

        // Derive border colors
        let primary_border = color::mk_border(&primary_color, dark_mode);
        let secondary_border = color::mk_border(&secondary_color, dark_mode);

        // Derive text colors (contrast with backgrounds)
        let primary_text = color::contrasting_text(&primary_color);
        let line_color = color::contrasting_text(&bg_color);

        // Derive pie colors by hue rotation from primary
        let pie_colors: Vec<String> = (0..10)
            .map(|i| {
                let hue_shift = (i as f64) * 36.0; // Distribute around color wheel
                let adjusted = color::adjust(&primary_color, hue_shift, 0.0, 0.0);
                adjusted.to_hex()
            })
            .collect();

        // Derive sequence diagram colors
        let actor_border = color::lighten(&primary_border, 10.0);
        let note_border = color::mk_border(&secondary_color, dark_mode);

        // Derive gantt colors
        let task_border = color::mk_border(&primary_color, dark_mode);
        let active_task_bkg = color::lighten(&primary_color, 15.0);
        let grid_color = if dark_mode {
            Color::from_hex("#444444").unwrap()
        } else {
            Color::from_hex("#d3d3d3").unwrap()
        };

        Self {
            // Base colors
            primary_color: primary_color.to_hex(),
            primary_text_color: primary_text.to_hex(),
            primary_border_color: primary_border.to_hex(),
            secondary_color: secondary_color.to_hex(),
            tertiary_color: tertiary_color.to_hex(),
            cluster_border_color: secondary_border.to_hex(),
            line_color: line_color.to_hex(),
            background: bg_color.to_hex(),
            edge_label_background: if dark_mode {
                "#4a4a4a".to_string()
            } else {
                "rgba(232, 232, 232, 0.8)".to_string()
            },
            font_family: "trebuchet ms, verdana, arial, sans-serif".to_string(),
            font_size: "16px".to_string(),

            // Pie chart colors (derived)
            pie_colors,
            pie_stroke_color: if dark_mode {
                line_color.to_hex()
            } else {
                "black".to_string()
            },
            pie_outer_stroke_color: if dark_mode {
                line_color.to_hex()
            } else {
                "black".to_string()
            },
            pie_opacity: "0.7".to_string(),
            pie_title_text_color: primary_text.to_hex(),
            pie_legend_text_color: primary_text.to_hex(),

            // Sequence diagram colors (derived)
            actor_bkg: primary_color.to_hex(),
            actor_border: actor_border.to_hex(),
            actor_text_color: primary_text.to_hex(),
            actor_line_color: actor_border.to_hex(),
            signal_color: line_color.to_hex(),
            signal_text_color: line_color.to_hex(),
            note_bkg_color: secondary_color.to_hex(),
            note_border_color: note_border.to_hex(),
            note_text_color: color::contrasting_text(&secondary_color).to_hex(),
            activation_bkg_color: tertiary_color.to_hex(),
            activation_border_color: primary_border.to_hex(),
            label_box_bkg_color: secondary_color.to_hex(),
            label_box_border_color: note_border.to_hex(),

            // Gantt chart colors (derived)
            section_bkg_color: primary_color.to_hex(),
            section_bkg_color2: bg_color.to_hex(),
            task_bkg_color: primary_color.to_hex(),
            task_border_color: task_border.to_hex(),
            task_text_light_color: "#ffffff".to_string(),
            task_text_dark_color: "#000000".to_string(),
            active_task_bkg_color: active_task_bkg.to_hex(),
            active_task_border_color: task_border.to_hex(),
            done_task_bkg_color: "#d3d3d3".to_string(),
            done_task_border_color: "#808080".to_string(),
            crit_bkg_color: "#ff0000".to_string(),
            crit_border_color: "#ff8888".to_string(),
            grid_color: grid_color.to_hex(),
            today_line_color: "#ff0000".to_string(),
        }
    }

    /// Apply variable overrides to the theme
    ///
    /// This follows the mermaid.js themeVariables pattern where users can
    /// override specific theme variables while keeping others intact.
    ///
    /// # Arguments
    /// * `overrides` - A map of variable names to values
    ///
    /// # Example
    /// ```
    /// use std::collections::HashMap;
    /// use selkie::render::svg::Theme;
    ///
    /// let mut overrides = HashMap::new();
    /// overrides.insert("primaryColor".to_string(), "#ff0000".to_string());
    /// overrides.insert("nodeBkg".to_string(), "#00ff00".to_string());
    ///
    /// let theme = Theme::default().with_overrides(&overrides);
    /// ```
    pub fn with_overrides(mut self, overrides: &std::collections::HashMap<String, String>) -> Self {
        for (key, value) in overrides {
            self.set_variable(key, value);
        }
        self
    }

    /// Apply overrides in place (mutating version)
    pub fn apply_overrides(&mut self, overrides: &std::collections::HashMap<String, String>) {
        for (key, value) in overrides {
            self.set_variable(key, value);
        }
    }

    /// Set a single theme variable by name
    ///
    /// Variable names follow mermaid.js conventions (camelCase).
    /// Returns true if the variable was found and set.
    pub fn set_variable(&mut self, name: &str, value: &str) -> bool {
        match name {
            // Common colors
            "primaryColor" => {
                self.primary_color = value.to_string();
                true
            }
            "primaryTextColor" => {
                self.primary_text_color = value.to_string();
                true
            }
            "primaryBorderColor" => {
                self.primary_border_color = value.to_string();
                true
            }
            "secondaryColor" => {
                self.secondary_color = value.to_string();
                true
            }
            "tertiaryColor" => {
                self.tertiary_color = value.to_string();
                true
            }
            "clusterBorderColor" | "clusterBorder" => {
                self.cluster_border_color = value.to_string();
                true
            }
            "lineColor" => {
                self.line_color = value.to_string();
                true
            }
            "background" => {
                self.background = value.to_string();
                true
            }
            "fontFamily" => {
                self.font_family = value.to_string();
                true
            }
            "fontSize" => {
                self.font_size = value.to_string();
                true
            }

            // Flowchart aliases (mermaid.js compatibility)
            "nodeBkg" | "mainBkg" => {
                self.primary_color = value.to_string();
                true
            }
            "nodeBorder" | "border1" => {
                self.primary_border_color = value.to_string();
                true
            }
            "clusterBkg" | "secondBkg" => {
                self.secondary_color = value.to_string();
                true
            }
            "edgeLabelBackground" | "labelBackground" => {
                self.background = value.to_string();
                true
            }

            // Pie chart colors
            "pieStrokeColor" => {
                self.pie_stroke_color = value.to_string();
                true
            }
            "pieOuterStrokeColor" => {
                self.pie_outer_stroke_color = value.to_string();
                true
            }
            "pieOpacity" => {
                self.pie_opacity = value.to_string();
                true
            }
            "pieTitleTextColor" => {
                self.pie_title_text_color = value.to_string();
                true
            }
            "pieLegendTextColor" => {
                self.pie_legend_text_color = value.to_string();
                true
            }

            // Sequence diagram colors
            "actorBkg" => {
                self.actor_bkg = value.to_string();
                true
            }
            "actorBorder" => {
                self.actor_border = value.to_string();
                true
            }
            "actorTextColor" => {
                self.actor_text_color = value.to_string();
                true
            }
            "actorLineColor" => {
                self.actor_line_color = value.to_string();
                true
            }
            "signalColor" => {
                self.signal_color = value.to_string();
                true
            }
            "signalTextColor" => {
                self.signal_text_color = value.to_string();
                true
            }
            "noteBkgColor" => {
                self.note_bkg_color = value.to_string();
                true
            }
            "noteBorderColor" => {
                self.note_border_color = value.to_string();
                true
            }
            "noteTextColor" => {
                self.note_text_color = value.to_string();
                true
            }
            "activationBkgColor" => {
                self.activation_bkg_color = value.to_string();
                true
            }
            "activationBorderColor" => {
                self.activation_border_color = value.to_string();
                true
            }
            "labelBoxBkgColor" => {
                self.label_box_bkg_color = value.to_string();
                true
            }
            "labelBoxBorderColor" => {
                self.label_box_border_color = value.to_string();
                true
            }

            // Gantt chart colors
            "sectionBkgColor" => {
                self.section_bkg_color = value.to_string();
                true
            }
            "sectionBkgColor2" | "altSectionBkgColor" => {
                self.section_bkg_color2 = value.to_string();
                true
            }
            "taskBkgColor" => {
                self.task_bkg_color = value.to_string();
                true
            }
            "taskBorderColor" => {
                self.task_border_color = value.to_string();
                true
            }
            "taskTextLightColor" => {
                self.task_text_light_color = value.to_string();
                true
            }
            "taskTextDarkColor" => {
                self.task_text_dark_color = value.to_string();
                true
            }
            "activeTaskBkgColor" => {
                self.active_task_bkg_color = value.to_string();
                true
            }
            "activeTaskBorderColor" => {
                self.active_task_border_color = value.to_string();
                true
            }
            "doneTaskBkgColor" => {
                self.done_task_bkg_color = value.to_string();
                true
            }
            "doneTaskBorderColor" => {
                self.done_task_border_color = value.to_string();
                true
            }
            "critBkgColor" => {
                self.crit_bkg_color = value.to_string();
                true
            }
            "critBorderColor" => {
                self.crit_border_color = value.to_string();
                true
            }
            "gridColor" => {
                self.grid_color = value.to_string();
                true
            }
            "todayLineColor" => {
                self.today_line_color = value.to_string();
                true
            }

            // Pie colors (pie1-pie12)
            name if name.starts_with("pie") && name.len() <= 5 => {
                if let Ok(idx) = name[3..].parse::<usize>() {
                    if idx >= 1 && idx <= self.pie_colors.len() {
                        self.pie_colors[idx - 1] = value.to_string();
                        return true;
                    }
                }
                false
            }

            _ => false,
        }
    }

    /// Get a theme variable by name
    ///
    /// Returns None if the variable name is not recognized.
    pub fn get_variable(&self, name: &str) -> Option<&str> {
        match name {
            "primaryColor" => Some(&self.primary_color),
            "primaryTextColor" => Some(&self.primary_text_color),
            "primaryBorderColor" => Some(&self.primary_border_color),
            "secondaryColor" => Some(&self.secondary_color),
            "tertiaryColor" => Some(&self.tertiary_color),
            "clusterBorderColor" | "clusterBorder" => Some(&self.cluster_border_color),
            "lineColor" => Some(&self.line_color),
            "background" => Some(&self.background),
            "fontFamily" => Some(&self.font_family),
            "fontSize" => Some(&self.font_size),
            "nodeBkg" | "mainBkg" => Some(&self.primary_color),
            "nodeBorder" | "border1" => Some(&self.primary_border_color),
            "clusterBkg" | "secondBkg" => Some(&self.secondary_color),
            "pieStrokeColor" => Some(&self.pie_stroke_color),
            "actorBkg" => Some(&self.actor_bkg),
            "taskBkgColor" => Some(&self.task_bkg_color),
            _ => None,
        }
    }

    /// Create a Theme from a diagram directive configuration
    ///
    /// This method:
    /// 1. Selects the base theme by name (default, dark, forest, etc.)
    /// 2. Applies any themeVariables overrides
    ///
    /// # Example
    ///
    /// ```
    /// use selkie::diagrams::DiagramConfig;
    /// use selkie::render::svg::Theme;
    /// use std::collections::HashMap;
    ///
    /// let mut config = DiagramConfig::default();
    /// config.theme = Some("dark".to_string());
    /// config.theme_variables.insert("primaryColor".to_string(), "#ff0000".to_string());
    ///
    /// let theme = Theme::from_directive(&config);
    /// assert_eq!(theme.primary_color, "#ff0000");
    /// ```
    pub fn from_directive(config: &crate::diagrams::DiagramConfig) -> Self {
        // Select base theme
        let mut theme = match config.theme.as_deref() {
            Some("dark") => Self::dark(),
            Some("forest") => Self::forest(),
            Some("default") | Some("base") | None => Self::default(),
            // Unknown theme name - fall back to default
            _ => Self::default(),
        };

        // Apply themeVariables overrides
        theme.apply_overrides(&config.theme_variables);

        theme
    }

    /// Create a custom theme with builder-style overrides
    ///
    /// Start with base colors and override specific values as needed.
    pub fn custom() -> ThemeBuilder {
        ThemeBuilder::new()
    }
}

/// Builder for creating custom themes
pub struct ThemeBuilder {
    base: Theme,
}

impl ThemeBuilder {
    pub fn new() -> Self {
        Self {
            base: Theme::default(),
        }
    }

    /// Set the primary color (affects nodes, actors, etc.)
    pub fn primary_color(mut self, color: &str) -> Self {
        self.base.primary_color = color.to_string();
        self
    }

    /// Set the secondary color (affects subgraphs, notes, etc.)
    pub fn secondary_color(mut self, color: &str) -> Self {
        self.base.secondary_color = color.to_string();
        self
    }

    /// Set the background color
    pub fn background(mut self, color: &str) -> Self {
        self.base.background = color.to_string();
        self
    }

    /// Set the line/edge color
    pub fn line_color(mut self, color: &str) -> Self {
        self.base.line_color = color.to_string();
        self
    }

    /// Set the font family
    pub fn font_family(mut self, font: &str) -> Self {
        self.base.font_family = font.to_string();
        self
    }

    /// Build the theme, deriving any colors that weren't explicitly set
    pub fn build(self) -> Theme {
        // For now, return the base with modifications
        // Future: could derive unset colors from base colors
        self.base
    }
}

impl Default for ThemeBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_base_colors_derives_consistent_theme() {
        let theme = Theme::from_base_colors(
            "#ECECFF", // primary (light purple)
            "#ffffde", // secondary (light yellow)
            "#fafafa", // tertiary (light gray)
            "#ffffff", // background (white)
            false,     // light mode
        );

        // Primary color should be set
        assert_eq!(theme.primary_color, "#ececff");

        // Border should be derived (darker than primary)
        let primary = super::super::color::Color::from_hex(&theme.primary_color).unwrap();
        let border = super::super::color::Color::from_hex(&theme.primary_border_color).unwrap();
        let (_, _, primary_l) = primary.to_hsl();
        let (_, _, border_l) = border.to_hsl();
        assert!(
            border_l < primary_l,
            "Border should be darker than primary in light mode"
        );

        // Pie colors should be derived (10 colors)
        assert_eq!(theme.pie_colors.len(), 10);
    }

    #[test]
    fn test_from_base_colors_dark_mode() {
        let theme = Theme::from_base_colors(
            "#1f2020", // primary (dark)
            "#8a8a8a", // secondary (gray)
            "#333333", // tertiary (dark gray)
            "#1f2020", // background (dark)
            true,      // dark mode
        );

        // Border should be lighter in dark mode
        let primary = super::super::color::Color::from_hex(&theme.primary_color).unwrap();
        let border = super::super::color::Color::from_hex(&theme.primary_border_color).unwrap();
        let (_, _, primary_l) = primary.to_hsl();
        let (_, _, border_l) = border.to_hsl();
        assert!(
            border_l > primary_l,
            "Border should be lighter than primary in dark mode"
        );
    }

    #[test]
    fn test_theme_builder() {
        let theme = Theme::custom()
            .primary_color("#ff0000")
            .secondary_color("#00ff00")
            .background("#0000ff")
            .build();

        assert_eq!(theme.primary_color, "#ff0000");
        assert_eq!(theme.secondary_color, "#00ff00");
        assert_eq!(theme.background, "#0000ff");
    }

    #[test]
    fn test_with_overrides() {
        use std::collections::HashMap;

        let mut overrides = HashMap::new();
        overrides.insert("primaryColor".to_string(), "#ff0000".to_string());
        overrides.insert("secondaryColor".to_string(), "#00ff00".to_string());

        let theme = Theme::default().with_overrides(&overrides);

        assert_eq!(theme.primary_color, "#ff0000");
        assert_eq!(theme.secondary_color, "#00ff00");
        // Other colors should remain default
        assert_eq!(theme.background, "#ffffff");
    }

    #[test]
    fn test_apply_overrides_mutating() {
        use std::collections::HashMap;

        let mut theme = Theme::dark();
        let mut overrides = HashMap::new();
        overrides.insert("primaryColor".to_string(), "#123456".to_string());

        theme.apply_overrides(&overrides);

        assert_eq!(theme.primary_color, "#123456");
    }

    #[test]
    fn test_set_variable_aliases() {
        let mut theme = Theme::default();

        // nodeBkg should set primary_color
        assert!(theme.set_variable("nodeBkg", "#aabbcc"));
        assert_eq!(theme.primary_color, "#aabbcc");

        // clusterBkg should set secondary_color
        assert!(theme.set_variable("clusterBkg", "#ddeeff"));
        assert_eq!(theme.secondary_color, "#ddeeff");

        // Unknown variable should return false
        assert!(!theme.set_variable("unknownVar", "#000000"));
    }

    #[test]
    fn test_set_pie_color() {
        let mut theme = Theme::default();

        // pie1 should set first pie color
        assert!(theme.set_variable("pie1", "#ff0000"));
        assert_eq!(theme.pie_colors[0], "#ff0000");

        // pie5 should set fifth pie color
        assert!(theme.set_variable("pie5", "#00ff00"));
        assert_eq!(theme.pie_colors[4], "#00ff00");

        // pie0 should fail (1-indexed)
        assert!(!theme.set_variable("pie0", "#0000ff"));
    }

    #[test]
    fn test_get_variable() {
        let theme = Theme::default();

        assert_eq!(theme.get_variable("primaryColor"), Some("#ECECFF"));
        assert_eq!(theme.get_variable("background"), Some("#ffffff"));
        assert_eq!(theme.get_variable("unknownVar"), None);

        // Aliases should work
        assert_eq!(theme.get_variable("nodeBkg"), Some("#ECECFF"));
    }

    #[test]
    fn test_theme_variables_integration() {
        // Test the full mermaid.js pattern: start with a theme, apply overrides
        use std::collections::HashMap;

        let mut overrides = HashMap::new();
        overrides.insert("primaryColor".to_string(), "#ff6600".to_string());
        overrides.insert("actorBkg".to_string(), "#ffcc00".to_string());
        overrides.insert("taskBkgColor".to_string(), "#00ccff".to_string());

        let theme = Theme::forest().with_overrides(&overrides);

        // Overridden values
        assert_eq!(theme.primary_color, "#ff6600");
        assert_eq!(theme.actor_bkg, "#ffcc00");
        assert_eq!(theme.task_bkg_color, "#00ccff");

        // Non-overridden values should keep forest theme colors
        assert_eq!(theme.line_color, "#008000"); // forest green
    }

    #[test]
    fn test_from_directive_selects_theme() {
        use crate::diagrams::DiagramConfig;

        // Test dark theme selection
        let mut config = DiagramConfig::default();
        config.theme = Some("dark".to_string());
        let theme = Theme::from_directive(&config);
        assert_eq!(theme.background, "#1f2020"); // dark background

        // Test forest theme selection
        config.theme = Some("forest".to_string());
        let theme = Theme::from_directive(&config);
        assert_eq!(theme.line_color, "#008000"); // forest green

        // Test default theme (no theme specified)
        config.theme = None;
        let theme = Theme::from_directive(&config);
        assert_eq!(theme.primary_color, "#ECECFF"); // default light purple
    }

    #[test]
    fn test_from_directive_applies_overrides() {
        use crate::diagrams::DiagramConfig;

        let mut config = DiagramConfig::default();
        config.theme = Some("default".to_string());
        config
            .theme_variables
            .insert("primaryColor".to_string(), "#ff0000".to_string());
        config
            .theme_variables
            .insert("lineColor".to_string(), "#00ff00".to_string());

        let theme = Theme::from_directive(&config);

        // Overrides should be applied
        assert_eq!(theme.primary_color, "#ff0000");
        assert_eq!(theme.line_color, "#00ff00");
        // Other values should remain default
        assert_eq!(theme.background, "#ffffff");
    }

    #[test]
    fn test_from_directive_unknown_theme_falls_back() {
        use crate::diagrams::DiagramConfig;

        let mut config = DiagramConfig::default();
        config.theme = Some("nonexistent_theme".to_string());

        let theme = Theme::from_directive(&config);
        // Should fall back to default theme
        assert_eq!(theme.primary_color, "#ECECFF");
    }
}
