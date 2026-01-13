//! Edge rendering for flowcharts

use crate::diagrams::flowchart::{EdgeStroke, FlowEdge};
use crate::layout::LayoutEdge;

use super::elements::{Attrs, SvgElement};
use super::markers;
use super::theme::Theme;

/// Result of rendering an edge - separate path and label for container groups
pub struct EdgeRenderResult {
    /// The edge path element (goes in edgePaths container)
    pub path: Option<SvgElement>,
    /// The edge label element (goes in edgeLabels container)
    pub label: Option<SvgElement>,
}

/// Render an edge with separate path and label for container groups
pub fn render_edge_parts(
    layout_edge: &LayoutEdge,
    flow_edge: &FlowEdge,
    _theme: &Theme,
) -> EdgeRenderResult {
    let edge_id = &layout_edge.id;

    // Build edge path
    let path = if !layout_edge.bend_points.is_empty() {
        let path_d = build_curved_path(&layout_edge.bend_points);

        let mut attrs = Attrs::new().with_class("edge-path").with_fill("none");

        // Apply stroke style
        match flow_edge.stroke {
            EdgeStroke::Normal => {
                attrs = attrs.with_stroke_width(1.0);
            }
            EdgeStroke::Thick => {
                attrs = attrs.with_stroke_width(3.5);
            }
            EdgeStroke::Dotted => {
                attrs = attrs.with_stroke_width(2.0).with_stroke_dasharray("3,3");
            }
            EdgeStroke::Invisible => {
                attrs = attrs.with_stroke_width(0.0);
            }
        }

        // Apply arrow markers
        if let Some(marker_url) = markers::get_marker_url(flow_edge.edge_type.as_deref()) {
            attrs = attrs.with_attr("marker-end", &marker_url);
        }
        if let Some(start_marker_url) =
            markers::get_start_marker_url(flow_edge.edge_type.as_deref())
        {
            attrs = attrs.with_attr("marker-start", &start_marker_url);
        }

        let path_element = SvgElement::path(path_d).with_attrs(attrs);
        let group_attrs = Attrs::new()
            .with_class("edge")
            .with_id(&format!("edge-{}", edge_id));
        Some(SvgElement::group(vec![path_element]).with_attrs(group_attrs))
    } else {
        None
    };

    // Build edge label
    let label = if !flow_edge.text.is_empty() {
        if let Some(label_pos) = &layout_edge.label_position {
            let mut label_elements = Vec::new();

            // Estimate text width
            let text_width = flow_edge.text.len() as f64 * 8.0;
            let text_height = 16.0;
            let padding = 4.0;

            // Background rect - uses CSS class for theming
            let bg_attrs = Attrs::new()
                .with_class("edge-label-bg")
                .with_attr("fill-opacity", "0.8");

            label_elements.push(
                SvgElement::rect(
                    label_pos.x - text_width / 2.0 - padding,
                    label_pos.y - text_height / 2.0 - padding / 2.0,
                    text_width + padding * 2.0,
                    text_height + padding,
                )
                .with_attrs(bg_attrs),
            );

            // Text element
            let label_attrs = Attrs::new()
                .with_class("edge-label")
                .with_attr("text-anchor", "middle")
                .with_attr("dominant-baseline", "central");

            label_elements.push(
                SvgElement::text(label_pos.x, label_pos.y, &flow_edge.text).with_attrs(label_attrs),
            );

            let group_attrs = Attrs::new()
                .with_class("edgeLabel")
                .with_id(&format!("edge-label-{}", edge_id));
            Some(SvgElement::group(label_elements).with_attrs(group_attrs))
        } else {
            None
        }
    } else {
        None
    };

    EdgeRenderResult { path, label }
}

/// Build SVG path from bend points (straight lines)
#[allow(dead_code)]
fn build_path(points: &[crate::layout::Point]) -> String {
    if points.is_empty() {
        return String::new();
    }

    let mut d = String::new();

    // Move to first point
    d.push_str(&format!("M {} {}", points[0].x, points[0].y));

    // Line to each subsequent point
    for point in &points[1..] {
        d.push_str(&format!(" L {} {}", point.x, point.y));
    }

    d
}

/// Build curved SVG path from bend points using basis spline interpolation
/// This matches d3's curveBasis for smooth curves like mermaid.js
fn build_curved_path(points: &[crate::layout::Point]) -> String {
    if points.is_empty() {
        return String::new();
    }

    if points.len() == 1 {
        return format!("M {} {}", points[0].x, points[0].y);
    }

    if points.len() == 2 {
        // For 2 points, use a straight line
        return format!(
            "M {} {} L {} {}",
            points[0].x, points[0].y, points[1].x, points[1].y
        );
    }

    // Use basis spline interpolation (like d3's curveBasis)
    // This creates smooth curves through the control points
    build_basis_path(points)
}

/// Build a basis spline path (B-spline) through the given points
/// This is equivalent to d3's curveBasis interpolation
fn build_basis_path(points: &[crate::layout::Point]) -> String {
    let n = points.len();
    if n < 2 {
        return String::new();
    }

    let mut d = String::new();

    // For basis splines, we need to handle the start and end specially
    // The curve passes near (but not necessarily through) interior points

    // Move to the starting point
    d.push_str(&format!("M {:.2} {:.2}", points[0].x, points[0].y));

    if n == 2 {
        // Just two points - straight line
        d.push_str(&format!(" L {:.2} {:.2}", points[1].x, points[1].y));
        return d;
    }

    if n == 3 {
        // Three points - single quadratic curve
        let x1 = (2.0 * points[0].x + points[1].x) / 3.0;
        let y1 = (2.0 * points[0].y + points[1].y) / 3.0;
        let x2 = (points[0].x + 2.0 * points[1].x) / 3.0;
        let y2 = (points[0].y + 2.0 * points[1].y) / 3.0;
        let x3 = (points[0].x + 4.0 * points[1].x + points[2].x) / 6.0;
        let y3 = (points[0].y + 4.0 * points[1].y + points[2].y) / 6.0;
        d.push_str(&format!(
            " C {:.2} {:.2}, {:.2} {:.2}, {:.2} {:.2}",
            x1, y1, x2, y2, x3, y3
        ));

        // Finish to end point
        let x4 = (2.0 * points[1].x + points[2].x) / 3.0;
        let y4 = (2.0 * points[1].y + points[2].y) / 3.0;
        let x5 = (points[1].x + 2.0 * points[2].x) / 3.0;
        let y5 = (points[1].y + 2.0 * points[2].y) / 3.0;
        d.push_str(&format!(
            " C {:.2} {:.2}, {:.2} {:.2}, {:.2} {:.2}",
            x4, y4, x5, y5, points[2].x, points[2].y
        ));
        return d;
    }

    // For 4+ points, use full basis spline
    // First segment (quadratic start)
    let x1 = (2.0 * points[0].x + points[1].x) / 3.0;
    let y1 = (2.0 * points[0].y + points[1].y) / 3.0;
    let x2 = (points[0].x + 2.0 * points[1].x) / 3.0;
    let y2 = (points[0].y + 2.0 * points[1].y) / 3.0;
    let x3 = (points[0].x + 4.0 * points[1].x + points[2].x) / 6.0;
    let y3 = (points[0].y + 4.0 * points[1].y + points[2].y) / 6.0;
    d.push_str(&format!(
        " C {:.2} {:.2}, {:.2} {:.2}, {:.2} {:.2}",
        x1, y1, x2, y2, x3, y3
    ));

    // Middle segments (cubic)
    for i in 2..n - 1 {
        let p0 = &points[i - 2];
        let p1 = &points[i - 1];
        let p2 = &points[i];
        let p3 = if i + 1 < n { &points[i + 1] } else { p2 };

        let x1 = (p0.x + 4.0 * p1.x + p2.x) / 6.0 + (p2.x - p0.x) / 6.0;
        let y1 = (p0.y + 4.0 * p1.y + p2.y) / 6.0 + (p2.y - p0.y) / 6.0;
        let x2 = (p1.x + 4.0 * p2.x + p3.x) / 6.0 - (p3.x - p1.x) / 6.0;
        let y2 = (p1.y + 4.0 * p2.y + p3.y) / 6.0 - (p3.y - p1.y) / 6.0;
        let x3 = (p1.x + 4.0 * p2.x + p3.x) / 6.0;
        let y3 = (p1.y + 4.0 * p2.y + p3.y) / 6.0;

        d.push_str(&format!(
            " C {:.2} {:.2}, {:.2} {:.2}, {:.2} {:.2}",
            x1, y1, x2, y2, x3, y3
        ));
    }

    // Last segment (end at final point)
    let p_last = &points[n - 1];
    let p_prev = &points[n - 2];
    let x1 = (p_prev.x + 2.0 * p_last.x) / 3.0;
    let y1 = (p_prev.y + 2.0 * p_last.y) / 3.0;
    d.push_str(&format!(
        " C {:.2} {:.2}, {:.2} {:.2}, {:.2} {:.2}",
        x1, y1, p_last.x, p_last.y, p_last.x, p_last.y
    ));

    d
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::Point;

    #[test]
    fn test_build_path() {
        let points = vec![
            Point::new(0.0, 0.0),
            Point::new(50.0, 0.0),
            Point::new(50.0, 50.0),
        ];

        let path = build_path(&points);
        assert_eq!(path, "M 0 0 L 50 0 L 50 50");
    }

    #[test]
    fn test_empty_path() {
        let points: Vec<Point> = vec![];
        let path = build_path(&points);
        assert!(path.is_empty());
    }

    #[test]
    fn test_build_curved_path_contains_bezier() {
        // Curved paths should use quadratic bezier (Q) or cubic bezier (C) commands
        let points = vec![
            Point::new(0.0, 0.0),
            Point::new(50.0, 0.0),
            Point::new(50.0, 50.0),
            Point::new(100.0, 50.0),
        ];

        let path = build_curved_path(&points);

        // Should start with M (move to)
        assert!(path.starts_with("M"), "Path should start with M command");
        // Should contain curve commands (Q for quadratic bezier or C for cubic)
        assert!(
            path.contains("Q") || path.contains("C") || path.contains("S"),
            "Curved path should contain bezier curve commands, got: {}",
            path
        );
        // Should NOT be all straight lines
        let l_count = path.matches(" L ").count();
        assert!(
            l_count < points.len() - 1,
            "Curved path should not use only L commands"
        );
    }

    #[test]
    fn test_build_curved_path_two_points() {
        // With only two points, should be a straight line (no curve possible)
        let points = vec![Point::new(0.0, 0.0), Point::new(100.0, 100.0)];

        let path = build_curved_path(&points);
        assert!(path.starts_with("M"));
        assert!(path.contains("L") || path.contains("100"));
    }

    #[test]
    fn test_edge_label_has_background_rect() {
        use crate::diagrams::flowchart::{EdgeStroke, FlowEdge, FlowTextType};
        use std::collections::HashMap;

        let layout_edge = LayoutEdge {
            id: "e1".to_string(),
            sources: vec!["a".to_string()],
            targets: vec!["b".to_string()],
            label: Some("label".to_string()),
            bend_points: vec![Point::new(0.0, 0.0), Point::new(100.0, 100.0)],
            label_position: Some(Point::new(50.0, 50.0)),
            weight: 1,
            reversed: false,
            metadata: HashMap::new(),
        };

        let flow_edge = FlowEdge {
            id: None,
            is_user_defined_id: false,
            start: "a".to_string(),
            end: "b".to_string(),
            interpolate: None,
            edge_type: Some("arrow_point".to_string()),
            stroke: EdgeStroke::Normal,
            style: vec![],
            length: None,
            text: "label".to_string(),
            label_type: FlowTextType::Text,
            classes: vec![],
            animation: None,
            animate: None,
        };

        let theme = Theme::default();
        let result = render_edge_parts(&layout_edge, &flow_edge, &theme);

        // The label should exist
        assert!(result.label.is_some(), "Edge should have a label element");
        let label_svg = result.label.unwrap().to_svg(0);

        // Edge label should have a background rect before the text
        assert!(
            label_svg.contains("<rect") && label_svg.contains("<text"),
            "Edge label should have background rect, got: {}",
            label_svg
        );

        // The rect should have some opacity for the translucent background
        assert!(
            label_svg.contains("opacity") || label_svg.contains("fill-opacity"),
            "Edge label background should have opacity for translucent effect"
        );
    }

    #[test]
    fn test_edge_label_background_uses_css_class_not_hardcoded_color() {
        // This test verifies that the edge label background does NOT have
        // a hardcoded fill color, allowing CSS theme styling to work.
        // Hardcoded inline fill attributes override CSS rules.
        use crate::diagrams::flowchart::{EdgeStroke, FlowEdge, FlowTextType};
        use std::collections::HashMap;

        let layout_edge = LayoutEdge {
            id: "e1".to_string(),
            sources: vec!["a".to_string()],
            targets: vec!["b".to_string()],
            label: Some("label".to_string()),
            bend_points: vec![Point::new(0.0, 0.0), Point::new(100.0, 100.0)],
            label_position: Some(Point::new(50.0, 50.0)),
            weight: 1,
            reversed: false,
            metadata: HashMap::new(),
        };

        let flow_edge = FlowEdge {
            id: None,
            is_user_defined_id: false,
            start: "a".to_string(),
            end: "b".to_string(),
            interpolate: None,
            edge_type: Some("arrow_point".to_string()),
            stroke: EdgeStroke::Normal,
            style: vec![],
            length: None,
            text: "label".to_string(),
            label_type: FlowTextType::Text,
            classes: vec![],
            animation: None,
            animate: None,
        };

        let theme = Theme::default();
        let result = render_edge_parts(&layout_edge, &flow_edge, &theme);

        // Get the label SVG to check for hardcoded colors
        assert!(result.label.is_some(), "Edge should have a label element");
        let svg = result.label.unwrap().to_svg(0);

        // The edge-label-bg rect should NOT have a hardcoded fill color
        // It should use the CSS class for theming
        assert!(
            !svg.contains("fill=\"#e8e8e8\""),
            "Edge label background should not have hardcoded fill '#e8e8e8', got: {}",
            svg
        );
    }
}
