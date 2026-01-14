//! Arrow marker definitions for edges

use super::elements::SvgElement;
use super::theme::Theme;

/// Create all arrow marker definitions
/// Uses markerUnits="userSpaceOnUse" to match mermaid.js sizing
pub fn create_arrow_markers(_theme: &Theme) -> Vec<SvgElement> {
    vec![
        // Arrow point end (filled triangle) - like mermaid.js pointEnd marker
        // refX=10 places the tip (at x=10 in path) at the line endpoint
        SvgElement::Marker {
            id: "arrow_point".to_string(),
            view_box: "0 0 10 10".to_string(),
            ref_x: 10.0, // Tip of arrow at line endpoint
            ref_y: 5.0,
            marker_width: 8.0,
            marker_height: 8.0,
            orient: "auto".to_string(),
            marker_units: Some("userSpaceOnUse".to_string()),
            children: vec![SvgElement::path("M 0 0 L 10 5 L 0 10 z")],
        },
        // Arrow point start (filled triangle) - like mermaid.js pointStart marker
        SvgElement::Marker {
            id: "arrow_point_start".to_string(),
            view_box: "0 0 10 10".to_string(),
            ref_x: 4.5, // mermaid.js uses refX: 4.5
            ref_y: 5.0,
            marker_width: 8.0,
            marker_height: 8.0,
            orient: "auto".to_string(),
            marker_units: Some("userSpaceOnUse".to_string()),
            children: vec![SvgElement::path("M 0 5 L 10 10 L 10 0 z")],
        },
        // Arrow cross end (X shape) - like mermaid.js crossEnd marker
        SvgElement::Marker {
            id: "arrow_cross".to_string(),
            view_box: "0 0 11 11".to_string(), // mermaid.js uses 11x11
            ref_x: 12.0,                       // mermaid.js uses refX: 12
            ref_y: 5.2,                        // mermaid.js uses refY: 5.2
            marker_width: 11.0,                // mermaid.js uses 11
            marker_height: 11.0,               // mermaid.js uses 11
            orient: "auto".to_string(),
            marker_units: Some("userSpaceOnUse".to_string()),
            children: vec![SvgElement::Path {
                d: "M 1 1 L 10 10 M 10 1 L 1 10".to_string(), // mermaid.js path
                attrs: super::elements::Attrs::new()
                    .with_fill("none")
                    .with_stroke_width(2.0), // mermaid.js uses stroke-width: 2
            }],
        },
        // Arrow cross start (X shape) - like mermaid.js crossStart marker
        SvgElement::Marker {
            id: "arrow_cross_start".to_string(),
            view_box: "0 0 11 11".to_string(),
            ref_x: -1.0, // mermaid.js uses refX: -1
            ref_y: 5.2,
            marker_width: 11.0,
            marker_height: 11.0,
            orient: "auto".to_string(),
            marker_units: Some("userSpaceOnUse".to_string()),
            children: vec![SvgElement::Path {
                d: "M 1 1 L 10 10 M 10 1 L 1 10".to_string(),
                attrs: super::elements::Attrs::new()
                    .with_fill("none")
                    .with_stroke_width(2.0),
            }],
        },
        // Arrow circle end (filled circle) - like mermaid.js circleEnd marker
        SvgElement::Marker {
            id: "arrow_circle".to_string(),
            view_box: "0 0 10 10".to_string(),
            ref_x: 11.0, // mermaid.js uses refX: 11 for circleEnd
            ref_y: 5.0,
            marker_width: 11.0,  // mermaid.js uses 11
            marker_height: 11.0, // mermaid.js uses 11
            orient: "auto".to_string(),
            marker_units: Some("userSpaceOnUse".to_string()),
            children: vec![SvgElement::circle(5.0, 5.0, 5.0)], // mermaid.js uses r=5
        },
        // Arrow circle start (filled circle) - like mermaid.js circleStart marker
        SvgElement::Marker {
            id: "arrow_circle_start".to_string(),
            view_box: "0 0 10 10".to_string(),
            ref_x: -1.0, // mermaid.js uses refX: -1 for circleStart
            ref_y: 5.0,
            marker_width: 11.0,
            marker_height: 11.0,
            orient: "auto".to_string(),
            marker_units: Some("userSpaceOnUse".to_string()),
            children: vec![SvgElement::circle(5.0, 5.0, 5.0)],
        },
    ]
}

/// Get the marker URL for an edge type
pub fn get_marker_url(edge_type: Option<&str>) -> Option<String> {
    match edge_type {
        Some("arrow_point") => Some("url(#arrow_point)".to_string()),
        Some("arrow_cross") => Some("url(#arrow_cross)".to_string()),
        Some("arrow_circle") => Some("url(#arrow_circle)".to_string()),
        Some("double_arrow_point") => Some("url(#arrow_point)".to_string()),
        Some("double_arrow_cross") => Some("url(#arrow_cross)".to_string()),
        Some("double_arrow_circle") => Some("url(#arrow_circle)".to_string()),
        _ => None,
    }
}

/// Get the start marker URL for double-headed edges
pub fn get_start_marker_url(edge_type: Option<&str>) -> Option<String> {
    match edge_type {
        Some("double_arrow_point") => Some("url(#arrow_point_start)".to_string()),
        Some("double_arrow_cross") => Some("url(#arrow_cross_start)".to_string()),
        Some("double_arrow_circle") => Some("url(#arrow_circle_start)".to_string()),
        _ => None,
    }
}
