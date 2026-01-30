//! Theme configuration for SVG rendering

use super::color::{adjust, adjust_to_hsl_string, Color};

/// Compute journey fillType colors dynamically from theme base colors.
/// This matches mermaid.js theme-default.js journey color calculation.
/// fillType0 = primaryColor
/// fillType1 = secondaryColor
/// fillType2 = adjust(primaryColor, { h: 64 })
/// fillType3 = adjust(secondaryColor, { h: 64 })
/// fillType4 = adjust(primaryColor, { h: -64 })
/// fillType5 = adjust(secondaryColor, { h: -64 })
/// fillType6 = adjust(primaryColor, { h: 128 })
/// fillType7 = adjust(secondaryColor, { h: 128 })
fn compute_journey_fill_types(primary: &str, secondary: &str) -> Vec<String> {
    let primary_color = Color::from_hex(primary).unwrap_or_else(|| Color::rgb(236, 236, 255));
    let secondary_color = Color::from_hex(secondary).unwrap_or_else(|| Color::rgb(255, 255, 222));

    vec![
        primary_color.to_hex(),                             // fillType0 = primaryColor
        secondary_color.to_hex(),                           // fillType1 = secondaryColor
        adjust(&primary_color, 64.0, 0.0, 0.0).to_hex(),    // fillType2 = adjust(primary, h: 64)
        adjust(&secondary_color, 64.0, 0.0, 0.0).to_hex(),  // fillType3 = adjust(secondary, h: 64)
        adjust(&primary_color, -64.0, 0.0, 0.0).to_hex(),   // fillType4 = adjust(primary, h: -64)
        adjust(&secondary_color, -64.0, 0.0, 0.0).to_hex(), // fillType5 = adjust(secondary, h: -64)
        adjust(&primary_color, 128.0, 0.0, 0.0).to_hex(),   // fillType6 = adjust(primary, h: 128)
        adjust(&secondary_color, 128.0, 0.0, 0.0).to_hex(), // fillType7 = adjust(secondary, h: 128)
    ]
}

/// Compute pie chart colors dynamically from theme base colors.
/// This matches mermaid.js theme-default.js pie color calculation.
///
/// In mermaid.js:
/// - tertiaryColor = adjust(primaryColor, { h: -160 }) - NOT a hardcoded value
/// - pie1 = primaryColor (kept in original hex format)
/// - pie2 = secondaryColor (kept in original hex format)
/// - pie3+ = computed colors (output as HSL to match mermaid)
fn compute_pie_colors(primary: &str, secondary: &str) -> Vec<String> {
    let primary_color = Color::from_hex(primary).unwrap_or_else(|| Color::rgb(236, 236, 255));
    let secondary_color = Color::from_hex(secondary).unwrap_or_else(|| Color::rgb(255, 255, 222));
    // tertiaryColor is derived from primaryColor by hue shift, not a hardcoded value!
    // mermaid.js: this.tertiaryColor = adjust(this.primaryColor, { h: -160 });
    // Note: We compute tertiary's HSL directly to avoid precision loss
    let (primary_h, primary_s, primary_l) = primary_color.to_hsl();
    let tertiary_h = ((primary_h - 160.0) % 360.0 + 360.0) % 360.0;
    let tertiary_s = primary_s;
    let tertiary_l = primary_l;

    // Use adjust_to_hsl_string to avoid RGB roundtrip precision loss
    vec![
        primary.to_lowercase(), // pie1 = primaryColor (keep original hex format like mermaid)
        secondary.to_lowercase(), // pie2 = secondaryColor (keep original hex format like mermaid)
        // pie3 = adjust(tertiaryColor, { l: -40 })
        format!(
            "hsl({}, {}%, {}%)",
            tertiary_h.round() as i32,
            format_hsl_pct(tertiary_s),
            format_hsl_pct((tertiary_l - 40.0).clamp(0.0, 100.0))
        ),
        adjust_to_hsl_string(&primary_color, 0.0, 0.0, -10.0), // pie4 = adjust(primaryColor, { l: -10 })
        adjust_to_hsl_string(&secondary_color, 0.0, 0.0, -30.0), // pie5 = adjust(secondaryColor, { l: -30 })
        // pie6 = adjust(tertiaryColor, { l: -20 })
        format!(
            "hsl({}, {}%, {}%)",
            tertiary_h.round() as i32,
            format_hsl_pct(tertiary_s),
            format_hsl_pct((tertiary_l - 20.0).clamp(0.0, 100.0))
        ),
        adjust_to_hsl_string(&primary_color, 60.0, 0.0, -20.0), // pie7 = adjust(primaryColor, { h: +60, l: -20 })
        adjust_to_hsl_string(&primary_color, -60.0, 0.0, -40.0), // pie8 = adjust(primaryColor, { h: -60, l: -40 })
        adjust_to_hsl_string(&primary_color, 120.0, 0.0, -40.0), // pie9 = adjust(primaryColor, { h: 120, l: -40 })
        adjust_to_hsl_string(&primary_color, 60.0, 0.0, -40.0), // pie10 = adjust(primaryColor, { h: +60, l: -40 })
        adjust_to_hsl_string(&primary_color, -90.0, 0.0, -40.0), // pie11 = adjust(primaryColor, { h: -90, l: -40 })
        adjust_to_hsl_string(&primary_color, 120.0, 0.0, -30.0), // pie12 = adjust(primaryColor, { h: 120, l: -30 })
    ]
}

/// Helper to format HSL percentage matching mermaid's precision
fn format_hsl_pct(value: f64) -> String {
    let rounded = value.round();
    if (value - rounded).abs() < 0.0001 {
        format!("{}", rounded as i32)
    } else {
        format!("{:.10}", value)
            .trim_end_matches('0')
            .trim_end_matches('.')
            .to_string()
    }
}

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

    // === Sankey diagram colors ===
    /// Sankey node color palette (10 colors matching d3 Tableau 10)
    pub sankey_node_colors: Vec<String>,
    /// Sankey link fill opacity
    pub sankey_link_opacity: String,
    /// Sankey label text color
    pub sankey_label_color: String,

    // === Quadrant chart colors ===
    /// Quadrant 1 (top-right) background color
    pub quadrant1_fill: String,
    /// Quadrant 2 (top-left) background color
    pub quadrant2_fill: String,
    /// Quadrant 3 (bottom-left) background color
    pub quadrant3_fill: String,
    /// Quadrant 4 (bottom-right) background color
    pub quadrant4_fill: String,
    /// Quadrant internal border color
    pub quadrant_internal_border_stroke: String,
    /// Quadrant external border color
    pub quadrant_external_border_stroke: String,
    /// Quadrant title text color
    pub quadrant_title_fill: String,
    /// Quadrant label text color (fallback)
    pub quadrant_text_fill: String,
    /// Quadrant 1 text fill color
    pub quadrant1_text_fill: String,
    /// Quadrant 2 text fill color
    pub quadrant2_text_fill: String,
    /// Quadrant 3 text fill color
    pub quadrant3_text_fill: String,
    /// Quadrant 4 text fill color
    pub quadrant4_text_fill: String,
    /// Default point fill color
    pub quadrant_point_fill: String,
    /// Point label text color
    pub quadrant_point_text_fill: String,
    /// X-axis text color
    pub quadrant_x_axis_text_fill: String,
    /// Y-axis text color
    pub quadrant_y_axis_text_fill: String,

    // === Journey diagram colors ===
    /// Journey fillType colors (0-7) for CSS classes
    /// Computed from primary/secondary colors with hue adjustments
    pub journey_fill_types: Vec<String>,
    /// Journey sectionFills colors for inline fill attributes
    /// These are the dark colors used directly on rect elements
    pub journey_section_fills: Vec<String>,
    /// Journey face color (default: #FFF8DC cornsilk)
    pub journey_face_color: String,
    /// Journey text color for legend
    pub journey_text_color: String,
    /// Journey actor colors (matching mermaid.js actorColours)
    pub journey_actor_colors: Vec<String>,

    // === XY Chart colors ===
    /// XY Chart background color
    pub xychart_background_color: String,
    /// XY Chart title color
    pub xychart_title_color: String,
    /// XY Chart x-axis title color
    pub xychart_x_axis_title_color: String,
    /// XY Chart x-axis label color
    pub xychart_x_axis_label_color: String,
    /// XY Chart x-axis tick color
    pub xychart_x_axis_tick_color: String,
    /// XY Chart x-axis line color
    pub xychart_x_axis_line_color: String,
    /// XY Chart y-axis title color
    pub xychart_y_axis_title_color: String,
    /// XY Chart y-axis label color
    pub xychart_y_axis_label_color: String,
    /// XY Chart y-axis tick color
    pub xychart_y_axis_tick_color: String,
    /// XY Chart y-axis line color
    pub xychart_y_axis_line_color: String,
    /// XY Chart plot color palette (comma-separated colors)
    pub xychart_plot_color_palette: Vec<String>,
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
            background: "white".to_string(),
            edge_label_background: "rgba(232, 232, 232, 0.8)".to_string(),
            font_family: "trebuchet ms, verdana, arial, sans-serif".to_string(),
            font_size: "16px".to_string(),
            // Pie chart - dynamically computed from theme colors (matches mermaid.js)
            pie_colors: compute_pie_colors("#ECECFF", "#ffffde"),
            pie_stroke_color: "black".to_string(),
            pie_outer_stroke_color: "black".to_string(),
            pie_opacity: "0.7".to_string(),
            pie_title_text_color: "#333333".to_string(),
            pie_legend_text_color: "#333333".to_string(),
            // Sequence diagram - default theme
            actor_bkg: "#ECECFF".to_string(),
            actor_border: "#9370DB".to_string(),
            actor_text_color: "#333333".to_string(),
            actor_line_color: "#999999".to_string(),
            signal_color: "#333333".to_string(),
            signal_text_color: "#333333".to_string(),
            note_bkg_color: "#fff5ad".to_string(),
            note_border_color: "#aaaa33".to_string(),
            note_text_color: "black".to_string(),
            activation_bkg_color: "#f4f4f4".to_string(),
            activation_border_color: "#666666".to_string(),
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
            // Sankey diagram - default theme (D3 Tableau 10 color scheme)
            sankey_node_colors: vec![
                "#4e79a7".to_string(), // blue
                "#f28e2c".to_string(), // orange
                "#e15759".to_string(), // red
                "#76b7b2".to_string(), // teal
                "#59a14f".to_string(), // green
                "#edc949".to_string(), // yellow
                "#af7aa1".to_string(), // purple
                "#ff9da7".to_string(), // pink
                "#9c755f".to_string(), // brown
                "#bab0ab".to_string(), // gray
            ],
            sankey_link_opacity: "0.5".to_string(),
            sankey_label_color: "#333333".to_string(),
            // Quadrant chart - default theme (mermaid.js defaults derived from primaryColor #ECECFF)
            // Text colors are computed from invert(primaryColor) = invert(#ECECFF) = #131300
            // Then adjusted by r:-5, g:-5, b:-5 increments for each quadrant
            quadrant1_fill: "#ECECFF".to_string(),
            quadrant2_fill: "#f1f1ff".to_string(),
            quadrant3_fill: "#f6f6ff".to_string(),
            quadrant4_fill: "#fbfbff".to_string(),
            quadrant_internal_border_stroke: "#c7c7f1".to_string(),
            quadrant_external_border_stroke: "#c7c7f1".to_string(),
            quadrant_title_fill: "#131300".to_string(), // invert(#ECECFF)
            quadrant_text_fill: "#131300".to_string(),  // invert(#ECECFF)
            quadrant1_text_fill: "#131300".to_string(), // invert(#ECECFF)
            quadrant2_text_fill: "#0e0e00".to_string(), // adjust_rgb(#131300, -5, -5, -5)
            quadrant3_text_fill: "#090900".to_string(), // adjust_rgb(#131300, -10, -10, -10)
            quadrant4_text_fill: "#040400".to_string(), // adjust_rgb(#131300, -15, -15, -15)
            quadrant_point_fill: "#9370DB".to_string(),
            quadrant_point_text_fill: "#131300".to_string(), // same as quadrant text fill per mermaid
            quadrant_x_axis_text_fill: "#131300".to_string(), // same as quadrant text fill per mermaid
            quadrant_y_axis_text_fill: "#131300".to_string(), // same as quadrant text fill per mermaid
            // Journey diagram - default theme (computed from primary/secondary colors)
            journey_fill_types: compute_journey_fill_types("#ECECFF", "#ffffde"),
            // sectionFills - dark colors used for inline fill attributes (mermaid.js defaults)
            journey_section_fills: vec![
                "#191970".to_string(), // Midnight Blue
                "#8B008B".to_string(), // Dark Magenta
                "#4B0082".to_string(), // Indigo
                "#2F4F4F".to_string(), // Dark Slate Gray
                "#800000".to_string(), // Maroon
                "#8B4513".to_string(), // Saddle Brown
                "#00008B".to_string(), // Dark Blue
            ],
            journey_face_color: "#FFF8DC".to_string(), // cornsilk - mermaid.js default
            journey_text_color: "#333".to_string(),
            journey_actor_colors: vec![
                "#8FBC8F".to_string(), // Dark Sea Green
                "#7CFC00".to_string(), // Lawn Green
                "#00FFFF".to_string(), // Cyan
                "#20B2AA".to_string(), // Light Sea Green
                "#B0E0E6".to_string(), // Powder Blue
                "#FFFFE0".to_string(), // Light Yellow
            ],
            // XY Chart - default theme (matching mermaid.js)
            // mermaid.js computes primaryTextColor = invert(primaryColor)
            // invert('#ECECFF') = '#131300' (dark olive-brown)
            // All xychart text/line colors use primaryTextColor
            xychart_background_color: "white".to_string(),
            xychart_title_color: "#131300".to_string(),
            xychart_x_axis_title_color: "#131300".to_string(),
            xychart_x_axis_label_color: "#131300".to_string(),
            xychart_x_axis_tick_color: "#131300".to_string(),
            xychart_x_axis_line_color: "#131300".to_string(),
            xychart_y_axis_title_color: "#131300".to_string(),
            xychart_y_axis_label_color: "#131300".to_string(),
            xychart_y_axis_tick_color: "#131300".to_string(),
            xychart_y_axis_line_color: "#131300".to_string(),
            xychart_plot_color_palette: vec![
                "#ECECFF".to_string(),
                "#8493A6".to_string(),
                "#FFC3A0".to_string(),
                "#DCDDE1".to_string(),
                "#B8E994".to_string(),
                "#D1A36F".to_string(),
                "#C3CDE6".to_string(),
                "#FFB6C1".to_string(),
                "#496078".to_string(),
                "#F8F3E3".to_string(),
            ],
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
            // Pie chart - dynamically computed from dark theme colors
            pie_colors: compute_pie_colors("#1f2020", "#8a8a8a"),
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
            // Sankey diagram - dark theme (muted colors for dark background)
            sankey_node_colors: vec![
                "#5c8eb8".to_string(), // muted blue
                "#d9a066".to_string(), // muted orange
                "#c86b6d".to_string(), // muted red
                "#6aa8a3".to_string(), // muted teal
                "#5a9c52".to_string(), // muted green
                "#d4bc5a".to_string(), // muted yellow
                "#9d7090".to_string(), // muted purple
                "#e08e96".to_string(), // muted pink
                "#8d6b54".to_string(), // muted brown
                "#9e958f".to_string(), // muted gray
            ],
            sankey_link_opacity: "0.5".to_string(),
            sankey_label_color: "#ccc".to_string(),
            // Quadrant chart - dark theme
            quadrant1_fill: "#2a2a2a".to_string(),
            quadrant2_fill: "#3a3a3a".to_string(),
            quadrant3_fill: "#3a3a3a".to_string(),
            quadrant4_fill: "#2a2a2a".to_string(),
            quadrant_internal_border_stroke: "#81B1DB".to_string(),
            quadrant_external_border_stroke: "#81B1DB".to_string(),
            quadrant_title_fill: "#ccc".to_string(),
            quadrant_text_fill: "#ccc".to_string(),
            quadrant1_text_fill: "#ccc".to_string(),
            quadrant2_text_fill: "#ccc".to_string(),
            quadrant3_text_fill: "#ccc".to_string(),
            quadrant4_text_fill: "#ccc".to_string(),
            quadrant_point_fill: "#81B1DB".to_string(),
            quadrant_point_text_fill: "#ccc".to_string(),
            quadrant_x_axis_text_fill: "#ccc".to_string(),
            quadrant_y_axis_text_fill: "#ccc".to_string(),
            // Journey diagram - dark theme (computed from dark theme colors)
            journey_fill_types: compute_journey_fill_types("#1f2020", "#8a8a8a"),
            journey_section_fills: vec![
                "#191970".to_string(),
                "#8B008B".to_string(),
                "#4B0082".to_string(),
                "#2F4F4F".to_string(),
                "#800000".to_string(),
                "#8B4513".to_string(),
                "#00008B".to_string(),
            ],
            journey_face_color: "#FFF8DC".to_string(), // cornsilk - mermaid.js default
            journey_text_color: "#ccc".to_string(),
            journey_actor_colors: vec![
                "#8FBC8F".to_string(),
                "#7CFC00".to_string(),
                "#00FFFF".to_string(),
                "#20B2AA".to_string(),
                "#B0E0E6".to_string(),
                "#FFFFE0".to_string(),
            ],
            // XY Chart - dark theme
            xychart_background_color: "#1f2020".to_string(),
            xychart_title_color: "#ccc".to_string(),
            xychart_x_axis_title_color: "#ccc".to_string(),
            xychart_x_axis_label_color: "#ccc".to_string(),
            xychart_x_axis_tick_color: "#ccc".to_string(),
            xychart_x_axis_line_color: "#ccc".to_string(),
            xychart_y_axis_title_color: "#ccc".to_string(),
            xychart_y_axis_label_color: "#ccc".to_string(),
            xychart_y_axis_tick_color: "#ccc".to_string(),
            xychart_y_axis_line_color: "#ccc".to_string(),
            xychart_plot_color_palette: vec![
                "#1f2020".to_string(),
                "#8493A6".to_string(),
                "#FFC3A0".to_string(),
                "#DCDDE1".to_string(),
                "#B8E994".to_string(),
                "#D1A36F".to_string(),
                "#C3CDE6".to_string(),
                "#FFB6C1".to_string(),
                "#496078".to_string(),
                "#F8F3E3".to_string(),
            ],
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
            // Pie chart - dynamically computed from neutral theme colors
            pie_colors: compute_pie_colors("#f0f0f0", "#e0e0e0"),
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
            // Sankey diagram - neutral theme (grayscale)
            sankey_node_colors: vec![
                "#808080".to_string(), // gray 1
                "#606060".to_string(), // gray 2
                "#a0a0a0".to_string(), // gray 3
                "#505050".to_string(), // gray 4
                "#909090".to_string(), // gray 5
                "#707070".to_string(), // gray 6
                "#b0b0b0".to_string(), // gray 7
                "#404040".to_string(), // gray 8
                "#c0c0c0".to_string(), // gray 9
                "#303030".to_string(), // gray 10
            ],
            sankey_link_opacity: "0.5".to_string(),
            sankey_label_color: "#333333".to_string(),
            // Quadrant chart - neutral theme (grayscale)
            quadrant1_fill: "#f0f0f0".to_string(),
            quadrant2_fill: "#e0e0e0".to_string(),
            quadrant3_fill: "#e0e0e0".to_string(),
            quadrant4_fill: "#f0f0f0".to_string(),
            quadrant_internal_border_stroke: "#666666".to_string(),
            quadrant_external_border_stroke: "#666666".to_string(),
            quadrant_title_fill: "#333333".to_string(),
            quadrant_text_fill: "#333333".to_string(),
            quadrant1_text_fill: "#333333".to_string(),
            quadrant2_text_fill: "#333333".to_string(),
            quadrant3_text_fill: "#333333".to_string(),
            quadrant4_text_fill: "#333333".to_string(),
            quadrant_point_fill: "#666666".to_string(),
            quadrant_point_text_fill: "#333333".to_string(),
            quadrant_x_axis_text_fill: "#333333".to_string(),
            quadrant_y_axis_text_fill: "#333333".to_string(),
            // Journey diagram - neutral theme (computed from neutral colors)
            journey_fill_types: compute_journey_fill_types("#f0f0f0", "#e0e0e0"),
            journey_section_fills: vec![
                "#191970".to_string(),
                "#8B008B".to_string(),
                "#4B0082".to_string(),
                "#2F4F4F".to_string(),
                "#800000".to_string(),
                "#8B4513".to_string(),
                "#00008B".to_string(),
            ],
            journey_face_color: "#FFF8DC".to_string(),
            journey_text_color: "#333".to_string(),
            journey_actor_colors: vec![
                "#8FBC8F".to_string(),
                "#7CFC00".to_string(),
                "#00FFFF".to_string(),
                "#20B2AA".to_string(),
                "#B0E0E6".to_string(),
                "#FFFFE0".to_string(),
            ],
            // XY Chart - neutral theme
            xychart_background_color: "white".to_string(),
            xychart_title_color: "#333333".to_string(),
            xychart_x_axis_title_color: "#333333".to_string(),
            xychart_x_axis_label_color: "#333333".to_string(),
            xychart_x_axis_tick_color: "#333333".to_string(),
            xychart_x_axis_line_color: "#333333".to_string(),
            xychart_y_axis_title_color: "#333333".to_string(),
            xychart_y_axis_label_color: "#333333".to_string(),
            xychart_y_axis_tick_color: "#333333".to_string(),
            xychart_y_axis_line_color: "#333333".to_string(),
            xychart_plot_color_palette: vec![
                "#f0f0f0".to_string(),
                "#808080".to_string(),
                "#a0a0a0".to_string(),
                "#606060".to_string(),
                "#c0c0c0".to_string(),
                "#707070".to_string(),
                "#b0b0b0".to_string(),
                "#505050".to_string(),
                "#d0d0d0".to_string(),
                "#404040".to_string(),
            ],
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
            // Pie chart - dynamically computed from forest theme colors
            pie_colors: compute_pie_colors("#cde498", "#cdffb2"),
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
            // Sankey diagram - forest theme (green palette)
            sankey_node_colors: vec![
                "#6eaa49".to_string(), // medium green
                "#487e3a".to_string(), // dark green
                "#cde498".to_string(), // light green
                "#13540c".to_string(), // very dark green
                "#98d439".to_string(), // lime
                "#4caf50".to_string(), // material green
                "#8bc34a".to_string(), // light green
                "#009688".to_string(), // teal
                "#00695c".to_string(), // dark teal
                "#2e7d32".to_string(), // forest green
            ],
            sankey_link_opacity: "0.5".to_string(),
            sankey_label_color: "#333333".to_string(),
            // Quadrant chart - forest theme (green palette)
            quadrant1_fill: "#cde498".to_string(),
            quadrant2_fill: "#cdffb2".to_string(),
            quadrant3_fill: "#cdffb2".to_string(),
            quadrant4_fill: "#cde498".to_string(),
            quadrant_internal_border_stroke: "#13540c".to_string(),
            quadrant_external_border_stroke: "#13540c".to_string(),
            quadrant_title_fill: "#333333".to_string(),
            quadrant_text_fill: "#333333".to_string(),
            quadrant1_text_fill: "#333333".to_string(),
            quadrant2_text_fill: "#333333".to_string(),
            quadrant3_text_fill: "#333333".to_string(),
            quadrant4_text_fill: "#333333".to_string(),
            quadrant_point_fill: "#13540c".to_string(),
            quadrant_point_text_fill: "#333333".to_string(),
            quadrant_x_axis_text_fill: "#333333".to_string(),
            quadrant_y_axis_text_fill: "#333333".to_string(),
            // Journey diagram - forest theme (computed from forest colors)
            journey_fill_types: compute_journey_fill_types("#cde498", "#cdffb2"),
            journey_section_fills: vec![
                "#191970".to_string(),
                "#8B008B".to_string(),
                "#4B0082".to_string(),
                "#2F4F4F".to_string(),
                "#800000".to_string(),
                "#8B4513".to_string(),
                "#00008B".to_string(),
            ],
            journey_face_color: "#FFF8DC".to_string(),
            journey_text_color: "#333".to_string(),
            journey_actor_colors: vec![
                "#8FBC8F".to_string(),
                "#7CFC00".to_string(),
                "#00FFFF".to_string(),
                "#20B2AA".to_string(),
                "#B0E0E6".to_string(),
                "#FFFFE0".to_string(),
            ],
            // XY Chart - forest theme (green palette)
            // invert('#cde498') gives a dark purple/brown
            xychart_background_color: "white".to_string(),
            xychart_title_color: "#321b67".to_string(),
            xychart_x_axis_title_color: "#321b67".to_string(),
            xychart_x_axis_label_color: "#321b67".to_string(),
            xychart_x_axis_tick_color: "#321b67".to_string(),
            xychart_x_axis_line_color: "#321b67".to_string(),
            xychart_y_axis_title_color: "#321b67".to_string(),
            xychart_y_axis_label_color: "#321b67".to_string(),
            xychart_y_axis_tick_color: "#321b67".to_string(),
            xychart_y_axis_line_color: "#321b67".to_string(),
            xychart_plot_color_palette: vec![
                "#cde498".to_string(),
                "#487e3a".to_string(),
                "#6eaa49".to_string(),
                "#13540c".to_string(),
                "#98d439".to_string(),
                "#4caf50".to_string(),
                "#8bc34a".to_string(),
                "#009688".to_string(),
                "#00695c".to_string(),
                "#2e7d32".to_string(),
            ],
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
            // Sankey diagram - base theme (warm neutral palette)
            sankey_node_colors: vec![
                "#4e79a7".to_string(), // blue
                "#f28e2c".to_string(), // orange
                "#e15759".to_string(), // red
                "#76b7b2".to_string(), // teal
                "#59a14f".to_string(), // green
                "#edc949".to_string(), // yellow
                "#af7aa1".to_string(), // purple
                "#ff9da7".to_string(), // pink
                "#9c755f".to_string(), // brown
                "#bab0ab".to_string(), // gray
            ],
            sankey_link_opacity: "0.5".to_string(),
            sankey_label_color: "#333333".to_string(),
            // Quadrant chart - base theme (warm pastels)
            quadrant1_fill: "#fff4dd".to_string(),
            quadrant2_fill: "#dde4ff".to_string(),
            quadrant3_fill: "#dde4ff".to_string(),
            quadrant4_fill: "#fff4dd".to_string(),
            quadrant_internal_border_stroke: "#9370DB".to_string(),
            quadrant_external_border_stroke: "#9370DB".to_string(),
            quadrant_title_fill: "#333333".to_string(),
            quadrant_text_fill: "#333333".to_string(),
            quadrant1_text_fill: "#333333".to_string(),
            quadrant2_text_fill: "#333333".to_string(),
            quadrant3_text_fill: "#333333".to_string(),
            quadrant4_text_fill: "#333333".to_string(),
            quadrant_point_fill: "#9370DB".to_string(),
            quadrant_point_text_fill: "#333333".to_string(),
            quadrant_x_axis_text_fill: "#333333".to_string(),
            quadrant_y_axis_text_fill: "#333333".to_string(),
            // Journey diagram - base theme (computed from base colors)
            journey_fill_types: compute_journey_fill_types("#fff4dd", "#dde4ff"),
            journey_section_fills: vec![
                "#191970".to_string(),
                "#8B008B".to_string(),
                "#4B0082".to_string(),
                "#2F4F4F".to_string(),
                "#800000".to_string(),
                "#8B4513".to_string(),
                "#00008B".to_string(),
            ],
            journey_face_color: "#FFF8DC".to_string(),
            journey_text_color: "#333".to_string(),
            journey_actor_colors: vec![
                "#8FBC8F".to_string(),
                "#7CFC00".to_string(),
                "#00FFFF".to_string(),
                "#20B2AA".to_string(),
                "#B0E0E6".to_string(),
                "#FFFFE0".to_string(),
            ],
            // XY Chart - base theme (warm pastels)
            // invert('#fff4dd') gives a dark blue
            xychart_background_color: "#f4f4f4".to_string(),
            xychart_title_color: "#000b22".to_string(),
            xychart_x_axis_title_color: "#000b22".to_string(),
            xychart_x_axis_label_color: "#000b22".to_string(),
            xychart_x_axis_tick_color: "#000b22".to_string(),
            xychart_x_axis_line_color: "#000b22".to_string(),
            xychart_y_axis_title_color: "#000b22".to_string(),
            xychart_y_axis_label_color: "#000b22".to_string(),
            xychart_y_axis_tick_color: "#000b22".to_string(),
            xychart_y_axis_line_color: "#000b22".to_string(),
            xychart_plot_color_palette: vec![
                "#fff4dd".to_string(),
                "#dde4ff".to_string(),
                "#f4ffdd".to_string(),
                "#ffe4dd".to_string(),
                "#e4ddff".to_string(),
                "#ddfff4".to_string(),
                "#fff0b3".to_string(),
                "#ffddee".to_string(),
                "#ddf4ff".to_string(),
                "#f4ddff".to_string(),
            ],
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

            // Sankey diagram colors (derived from primary colors + D3 Tableau 10)
            sankey_node_colors: if dark_mode {
                vec![
                    "#5c8eb8".to_string(),
                    "#d9a066".to_string(),
                    "#c86b6d".to_string(),
                    "#6aa8a3".to_string(),
                    "#5a9c52".to_string(),
                    "#d4bc5a".to_string(),
                    "#9d7090".to_string(),
                    "#e08e96".to_string(),
                    "#8d6b54".to_string(),
                    "#9e958f".to_string(),
                ]
            } else {
                vec![
                    "#4e79a7".to_string(),
                    "#f28e2c".to_string(),
                    "#e15759".to_string(),
                    "#76b7b2".to_string(),
                    "#59a14f".to_string(),
                    "#edc949".to_string(),
                    "#af7aa1".to_string(),
                    "#ff9da7".to_string(),
                    "#9c755f".to_string(),
                    "#bab0ab".to_string(),
                ]
            },
            sankey_link_opacity: "0.5".to_string(),
            sankey_label_color: primary_text.to_hex(),

            // Quadrant chart colors (derived)
            quadrant1_fill: color::lighten(&primary_color, 20.0).to_hex(),
            quadrant2_fill: color::lighten(&secondary_color, 10.0).to_hex(),
            quadrant3_fill: color::lighten(&secondary_color, 10.0).to_hex(),
            quadrant4_fill: color::lighten(&primary_color, 20.0).to_hex(),
            quadrant_internal_border_stroke: primary_border.to_hex(),
            quadrant_external_border_stroke: primary_border.to_hex(),
            quadrant_title_fill: primary_text.to_hex(),
            quadrant_text_fill: primary_text.to_hex(),
            quadrant1_text_fill: primary_text.to_hex(),
            quadrant2_text_fill: primary_text.to_hex(),
            quadrant3_text_fill: primary_text.to_hex(),
            quadrant4_text_fill: primary_text.to_hex(),
            quadrant_point_fill: primary_border.to_hex(),
            quadrant_point_text_fill: primary_text.to_hex(),
            quadrant_x_axis_text_fill: primary_text.to_hex(),
            quadrant_y_axis_text_fill: primary_text.to_hex(),

            // Journey diagram colors (derived from primary/secondary)
            journey_fill_types: compute_journey_fill_types(primary, secondary),
            journey_section_fills: vec![
                "#191970".to_string(),
                "#8B008B".to_string(),
                "#4B0082".to_string(),
                "#2F4F4F".to_string(),
                "#800000".to_string(),
                "#8B4513".to_string(),
                "#00008B".to_string(),
            ],
            journey_face_color: "#FFF8DC".to_string(),
            journey_text_color: if dark_mode {
                "#ccc".to_string()
            } else {
                "#333".to_string()
            },
            journey_actor_colors: vec![
                "#8FBC8F".to_string(),
                "#7CFC00".to_string(),
                "#00FFFF".to_string(),
                "#20B2AA".to_string(),
                "#B0E0E6".to_string(),
                "#FFFFE0".to_string(),
            ],

            // XY Chart colors (derived from primary colors)
            // xychart uses inverted primaryColor for text/line colors (matching mermaid.js)
            xychart_background_color: if dark_mode {
                bg_color.to_hex()
            } else {
                "white".to_string()
            },
            xychart_title_color: color::invert(&primary_color).to_hex(),
            xychart_x_axis_title_color: color::invert(&primary_color).to_hex(),
            xychart_x_axis_label_color: color::invert(&primary_color).to_hex(),
            xychart_x_axis_tick_color: color::invert(&primary_color).to_hex(),
            xychart_x_axis_line_color: color::invert(&primary_color).to_hex(),
            xychart_y_axis_title_color: color::invert(&primary_color).to_hex(),
            xychart_y_axis_label_color: color::invert(&primary_color).to_hex(),
            xychart_y_axis_tick_color: color::invert(&primary_color).to_hex(),
            xychart_y_axis_line_color: color::invert(&primary_color).to_hex(),
            xychart_plot_color_palette: vec![
                primary_color.to_hex(),
                "#8493A6".to_string(),
                "#FFC3A0".to_string(),
                "#DCDDE1".to_string(),
                "#B8E994".to_string(),
                "#D1A36F".to_string(),
                "#C3CDE6".to_string(),
                "#FFB6C1".to_string(),
                "#496078".to_string(),
                "#F8F3E3".to_string(),
            ],
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

            // Quadrant chart colors
            "quadrant1Fill" => {
                self.quadrant1_fill = value.to_string();
                true
            }
            "quadrant2Fill" => {
                self.quadrant2_fill = value.to_string();
                true
            }
            "quadrant3Fill" => {
                self.quadrant3_fill = value.to_string();
                true
            }
            "quadrant4Fill" => {
                self.quadrant4_fill = value.to_string();
                true
            }
            "quadrantInternalBorderStroke" => {
                self.quadrant_internal_border_stroke = value.to_string();
                true
            }
            "quadrantExternalBorderStroke" => {
                self.quadrant_external_border_stroke = value.to_string();
                true
            }
            "quadrantTitleFill" => {
                self.quadrant_title_fill = value.to_string();
                true
            }
            "quadrantTextFill" => {
                self.quadrant_text_fill = value.to_string();
                true
            }
            "quadrant1TextFill" => {
                self.quadrant1_text_fill = value.to_string();
                true
            }
            "quadrant2TextFill" => {
                self.quadrant2_text_fill = value.to_string();
                true
            }
            "quadrant3TextFill" => {
                self.quadrant3_text_fill = value.to_string();
                true
            }
            "quadrant4TextFill" => {
                self.quadrant4_text_fill = value.to_string();
                true
            }
            "quadrantPointFill" => {
                self.quadrant_point_fill = value.to_string();
                true
            }
            "quadrantPointTextFill" => {
                self.quadrant_point_text_fill = value.to_string();
                true
            }
            "quadrantXAxisTextFill" => {
                self.quadrant_x_axis_text_fill = value.to_string();
                true
            }
            "quadrantYAxisTextFill" => {
                self.quadrant_y_axis_text_fill = value.to_string();
                true
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
            // Quadrant chart colors
            "quadrant1Fill" => Some(&self.quadrant1_fill),
            "quadrant2Fill" => Some(&self.quadrant2_fill),
            "quadrant3Fill" => Some(&self.quadrant3_fill),
            "quadrant4Fill" => Some(&self.quadrant4_fill),
            "quadrantInternalBorderStroke" => Some(&self.quadrant_internal_border_stroke),
            "quadrantExternalBorderStroke" => Some(&self.quadrant_external_border_stroke),
            "quadrantTitleFill" => Some(&self.quadrant_title_fill),
            "quadrantTextFill" => Some(&self.quadrant_text_fill),
            "quadrant1TextFill" => Some(&self.quadrant1_text_fill),
            "quadrant2TextFill" => Some(&self.quadrant2_text_fill),
            "quadrant3TextFill" => Some(&self.quadrant3_text_fill),
            "quadrant4TextFill" => Some(&self.quadrant4_text_fill),
            "quadrantPointFill" => Some(&self.quadrant_point_fill),
            "quadrantPointTextFill" => Some(&self.quadrant_point_text_fill),
            "quadrantXAxisTextFill" => Some(&self.quadrant_x_axis_text_fill),
            "quadrantYAxisTextFill" => Some(&self.quadrant_y_axis_text_fill),
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
        assert_eq!(theme.background, "white");
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
        assert_eq!(theme.get_variable("background"), Some("white"));
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
        let config = DiagramConfig {
            theme: Some("dark".to_string()),
            ..Default::default()
        };
        let theme = Theme::from_directive(&config);
        assert_eq!(theme.background, "#1f2020"); // dark background

        // Test forest theme selection
        let config = DiagramConfig {
            theme: Some("forest".to_string()),
            ..Default::default()
        };
        let theme = Theme::from_directive(&config);
        assert_eq!(theme.line_color, "#008000"); // forest green

        // Test default theme (no theme specified)
        let config = DiagramConfig::default();
        let theme = Theme::from_directive(&config);
        assert_eq!(theme.primary_color, "#ECECFF"); // default light purple
    }

    #[test]
    fn test_from_directive_applies_overrides() {
        use crate::diagrams::DiagramConfig;
        use std::collections::HashMap;

        let mut theme_variables = HashMap::new();
        theme_variables.insert("primaryColor".to_string(), "#ff0000".to_string());
        theme_variables.insert("lineColor".to_string(), "#00ff00".to_string());

        let config = DiagramConfig {
            theme: Some("default".to_string()),
            theme_variables,
            ..Default::default()
        };

        let theme = Theme::from_directive(&config);

        // Overrides should be applied
        assert_eq!(theme.primary_color, "#ff0000");
        assert_eq!(theme.line_color, "#00ff00");
        // Other values should remain default
        assert_eq!(theme.background, "white");
    }

    #[test]
    fn test_from_directive_unknown_theme_falls_back() {
        use crate::diagrams::DiagramConfig;

        let config = DiagramConfig {
            theme: Some("nonexistent_theme".to_string()),
            ..Default::default()
        };

        let theme = Theme::from_directive(&config);
        // Should fall back to default theme
        assert_eq!(theme.primary_color, "#ECECFF");
    }

    #[test]
    fn test_quadrant_text_fill_colors_match_mermaid() {
        // Mermaid.js computes quadrant text fill colors from primaryTextColor:
        // - primaryTextColor = invert(primaryColor) = invert(#ECECFF) = #131300
        // - quadrant1TextFill = primaryTextColor = #131300
        // - quadrant2TextFill = adjust(primaryTextColor, r:-5, g:-5, b:-5) = #0e0e00
        // - quadrant3TextFill = adjust(primaryTextColor, r:-10, g:-10, b:-10) = #090900
        // - quadrant4TextFill = adjust(primaryTextColor, r:-15, g:-15, b:-15) = #040400
        //
        // Note: The blue channel is already 0, so subtracting clamped to 0.
        let theme = Theme::default();

        // primaryColor = #ECECFF = RGB(236, 236, 255)
        // Inverted = RGB(255-236, 255-236, 255-255) = RGB(19, 19, 0) = #131300
        assert_eq!(
            theme.quadrant1_text_fill.to_lowercase(),
            "#131300",
            "quadrant1_text_fill should be inverted primaryColor"
        );

        // quadrant2TextFill = RGB(19-5, 19-5, 0-5) = RGB(14, 14, 0) = #0e0e00
        // (blue clamps to 0)
        assert_eq!(
            theme.quadrant2_text_fill.to_lowercase(),
            "#0e0e00",
            "quadrant2_text_fill should be adjust(primaryTextColor, r:-5, g:-5, b:-5)"
        );

        // quadrant3TextFill = RGB(19-10, 19-10, 0-10) = RGB(9, 9, 0) = #090900
        assert_eq!(
            theme.quadrant3_text_fill.to_lowercase(),
            "#090900",
            "quadrant3_text_fill should be adjust(primaryTextColor, r:-10, g:-10, b:-10)"
        );

        // quadrant4TextFill = RGB(19-15, 19-15, 0-15) = RGB(4, 4, 0) = #040400
        assert_eq!(
            theme.quadrant4_text_fill.to_lowercase(),
            "#040400",
            "quadrant4_text_fill should be adjust(primaryTextColor, r:-15, g:-15, b:-15)"
        );
    }

    #[test]
    fn test_pie_colors_match_mermaid_default_theme() {
        // This test ensures pie colors match mermaid.js default theme exactly.
        // Bug: tertiaryColor was #fafafa (gray) instead of derived from primaryColor,
        // causing pie3 and pie6 to be gray instead of having proper hue.
        //
        // In mermaid.js theme-default.js:
        // - primaryColor = '#ECECFF' (light purple, hsl(240, 100%, 96%))
        // - secondaryColor = '#ffffde' (light yellow, hsl(60, 100%, 93%))
        // - tertiaryColor = adjust(primaryColor, { h: -160 }) = hsl(80, 100%, 96%)
        //
        // Pie colors:
        // - pie1 = primaryColor = #ECECFF
        // - pie2 = secondaryColor = #ffffde
        // - pie3 = adjust(tertiaryColor, { l: -40 }) = hsl(80, 100%, 56%)
        // - pie4 = adjust(primaryColor, { l: -10 }) = hsl(240, 100%, 86%)
        // - etc.
        let theme = Theme::default();

        // Verify no pie color has 0% saturation (would indicate bug with grayscale tertiary)
        for (i, color_str) in theme.pie_colors.iter().enumerate() {
            // Parse the HSL string to check saturation
            if color_str.starts_with("hsl(") {
                let inner = &color_str[4..color_str.len() - 1];
                let parts: Vec<&str> = inner.split(',').collect();
                if parts.len() >= 2 {
                    let saturation = parts[1]
                        .trim()
                        .trim_end_matches('%')
                        .parse::<f64>()
                        .unwrap_or(100.0);
                    assert!(
                        saturation > 0.0,
                        "pie{} has 0% saturation (gray): {} - tertiaryColor may not be derived correctly",
                        i + 1,
                        color_str
                    );
                }
            }
        }

        // pie1 should be primaryColor (light purple)
        // pie2 should be secondaryColor (light yellow)
        let pie1 = &theme.pie_colors[0];
        let pie2 = &theme.pie_colors[1];

        // Parse pie1 to verify it's the purple color (hue ~240)
        if pie1.starts_with("hsl(") {
            let inner = &pie1[4..pie1.len() - 1];
            let hue: f64 = inner
                .split(',')
                .next()
                .unwrap()
                .trim()
                .parse()
                .unwrap_or(0.0);
            assert!(
                (hue - 240.0).abs() < 5.0,
                "pie1 should have hue ~240 (purple), got {}",
                hue
            );
        }

        // Parse pie2 to verify it's the yellow color (hue ~60)
        if pie2.starts_with("hsl(") {
            let inner = &pie2[4..pie2.len() - 1];
            let hue: f64 = inner
                .split(',')
                .next()
                .unwrap()
                .trim()
                .parse()
                .unwrap_or(0.0);
            assert!(
                (hue - 60.0).abs() < 5.0,
                "pie2 should have hue ~60 (yellow), got {}",
                hue
            );
        }
    }
}
