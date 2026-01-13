//! Arrow marker definitions for edges

use super::elements::SvgElement;
use super::theme::Theme;

/// Create all arrow marker definitions
/// Uses markerUnits="userSpaceOnUse" to match mermaid.js sizing
pub fn create_arrow_markers(_theme: &Theme) -> Vec<SvgElement> {
    vec![
        // Arrow point (filled triangle) - like mermaid.js point marker
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
        // Arrow open (lines, no fill)
        // Tip is at x=9 in path
        SvgElement::Marker {
            id: "arrow_open".to_string(),
            view_box: "0 0 10 10".to_string(),
            ref_x: 9.0, // Tip of arrow at line endpoint
            ref_y: 5.0,
            marker_width: 8.0,
            marker_height: 8.0,
            orient: "auto".to_string(),
            marker_units: Some("userSpaceOnUse".to_string()),
            children: vec![SvgElement::Path {
                d: "M 1 1 L 9 5 L 1 9".to_string(),
                attrs: super::elements::Attrs::new()
                    .with_fill("none")
                    .with_stroke_width(1.5),
            }],
        },
        // Arrow cross (X shape) - like mermaid.js cross marker
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
        // Double arrow point start
        // Path: "M 0 5 L 10 10 L 10 0 z" - tip is at x=0
        SvgElement::Marker {
            id: "double_arrow_point_start".to_string(),
            view_box: "0 0 10 10".to_string(),
            ref_x: 0.0, // Tip at x=0 should be at line start
            ref_y: 5.0,
            marker_width: 8.0,
            marker_height: 8.0,
            orient: "auto".to_string(),
            marker_units: Some("userSpaceOnUse".to_string()),
            children: vec![SvgElement::path("M 0 5 L 10 10 L 10 0 z")],
        },
        // Double arrow point end
        // Path: "M 0 0 L 10 5 L 0 10 z" - tip is at x=10
        SvgElement::Marker {
            id: "double_arrow_point_end".to_string(),
            view_box: "0 0 10 10".to_string(),
            ref_x: 10.0, // Tip at x=10 should be at line end
            ref_y: 5.0,
            marker_width: 8.0,
            marker_height: 8.0,
            orient: "auto".to_string(),
            marker_units: Some("userSpaceOnUse".to_string()),
            children: vec![SvgElement::path("M 0 0 L 10 5 L 0 10 z")],
        },
    ]
}

/// Get the marker URL for an edge type
pub fn get_marker_url(edge_type: Option<&str>) -> Option<String> {
    match edge_type {
        Some("arrow_point") => Some("url(#arrow_point)".to_string()),
        Some("arrow_open") => Some("url(#arrow_open)".to_string()),
        Some("arrow_cross") => Some("url(#arrow_cross)".to_string()),
        Some("arrow_circle") => Some("url(#arrow_circle)".to_string()),
        Some("double_arrow_point") => Some("url(#double_arrow_point_end)".to_string()),
        Some("double_arrow_cross") => Some("url(#arrow_cross)".to_string()),
        Some("double_arrow_circle") => Some("url(#arrow_circle)".to_string()),
        _ => None,
    }
}

/// Get the start marker URL for double-headed edges
pub fn get_start_marker_url(edge_type: Option<&str>) -> Option<String> {
    match edge_type {
        Some("double_arrow_point") => Some("url(#double_arrow_point_start)".to_string()),
        Some("double_arrow_cross") => Some("url(#arrow_cross)".to_string()),
        Some("double_arrow_circle") => Some("url(#arrow_circle_start)".to_string()),
        _ => None,
    }
}
