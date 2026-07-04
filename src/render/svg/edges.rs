//! Edge rendering for flowcharts

use crate::diagrams::flowchart::{EdgeStroke, FlowEdge, FlowTextType};
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

    // Build edge path following mermaid's insertEdge pipeline:
    // filter NaN points -> fixCorners -> marker endpoint offsets -> curveBasis
    let path = if !layout_edge.bend_points.is_empty() {
        let line_data: Vec<crate::layout::Point> = layout_edge
            .bend_points
            .iter()
            .filter(|p| !p.y.is_nan())
            .copied()
            .collect();
        let line_data = fix_corners(&line_data);
        let start_offset = markers::start_marker_offset(flow_edge.edge_type.as_deref());
        let end_offset = markers::end_marker_offset(flow_edge.edge_type.as_deref());
        let line_data = apply_marker_offsets(&line_data, start_offset, end_offset);
        let path_d = build_curved_path(&line_data);

        // Stroke classes - port of mermaid insertEdge (rendering-elements/edges.js):
        // edge.thickness and edge.pattern both derive from the flowDb stroke.
        let stroke_classes = match flow_edge.stroke {
            EdgeStroke::Normal => "edge-thickness-normal edge-pattern-solid",
            EdgeStroke::Thick => "edge-thickness-thick edge-pattern-solid",
            EdgeStroke::Dotted => "edge-thickness-normal edge-pattern-dotted",
            EdgeStroke::Invisible => "edge-thickness-invisible edge-pattern-solid",
        };
        let mut attrs = Attrs::new().with_class(stroke_classes);
        // flowDb getData: invisible edges get no flowchart-link classes
        if flow_edge.stroke != EdgeStroke::Invisible {
            attrs = attrs.with_class("edge-thickness-normal edge-pattern-solid flowchart-link");
        }

        // linkStyle / class styles applied inline, like mermaid's pathStyle
        if !flow_edge.style.is_empty() {
            let style = flow_edge
                .style
                .iter()
                .fold(String::new(), |acc, s| acc + s + ";");
            attrs = attrs.with_style(&style);
        }

        // Apply arrow markers (suppressed for invisible edges, per flowDb)
        if flow_edge.stroke != EdgeStroke::Invisible {
            if let Some(marker_url) = markers::get_marker_url(flow_edge.edge_type.as_deref()) {
                attrs = attrs.with_attr("marker-end", &marker_url);
            }
            if let Some(start_marker_url) =
                markers::get_start_marker_url(flow_edge.edge_type.as_deref())
            {
                attrs = attrs.with_attr("marker-start", &start_marker_url);
            }
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

            // Estimate text size for background
            // Use font-size 12 (matching .edge-label style) and approximate char width
            let font_size = 12.0;
            let char_width_ratio = 0.6;

            // Handle multiline text (split by <br> or newlines), applying
            // mermaid's wrappingWidth word-wrap (measured at the 16px label
            // font, like the layout's edge label measurement).
            // Markdown labels are measured as their marker-stripped visible
            // text (mermaid measures the rendered label, not the source markers).
            let raw_text = if flow_edge.label_type == FlowTextType::Markdown {
                crate::render::text_utils::strip_markdown_markers(&flow_edge.text)
            } else {
                flow_edge.text.clone()
            };
            let text = crate::render::text_utils::normalize_mermaid_label_markup(&raw_text);
            let text = crate::render::text_utils::wrap_label_text_mermaid(&text, 16.0);
            let lines: Vec<&str> = text.lines().collect();
            let max_chars = lines.iter().map(|l| l.chars().count()).max().unwrap_or(0);
            let num_lines = lines.len().max(1);

            let text_width = (max_chars as f64) * font_size * char_width_ratio;
            let text_height = (num_lines as f64) * font_size * 1.5;
            let padding = 4.0;

            // Background rectangle
            label_elements.push(SvgElement::Rect {
                x: label_pos.x - text_width / 2.0 - padding,
                y: label_pos.y - text_height / 2.0 - padding / 2.0,
                width: text_width + padding * 2.0,
                height: text_height + padding,
                rx: None,
                ry: None,
                attrs: Attrs::new().with_class("edge-label-bg"),
            });

            // Text element
            let label_attrs = Attrs::new()
                .with_class("edge-label")
                .with_attr("text-anchor", "middle")
                .with_attr("dominant-baseline", "central");

            // Markdown edge labels render their emphasis as styled tspans; the
            // marker-stripped visible text drives the background sizing above.
            let text_element = if flow_edge.label_type == FlowTextType::Markdown {
                SvgElement::markdown_text(label_pos.x, label_pos.y, flow_edge.text.clone())
            } else {
                SvgElement::text(label_pos.x, label_pos.y, text)
            };
            label_elements.push(text_element.with_attrs(label_attrs));

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

/// Build curved SVG path from points using d3-shape's curveBasis.
///
/// This is a faithful port of d3-shape `curveBasis` combined with d3-path
/// serialization, matching what mermaid.js produces (`line().curve(curveBasis)`).
/// ALL points are rendered - mermaid performs no bend-point simplification.
///
/// d3 Basis emits, for points p0..p(n-1) with n >= 3:
///   M p0
///   L (5*p0 + p1) / 6
///   C segments for each subsequent point (B-spline blending)
///   a closing C segment toward (p(n-2) + 5*p(n-1)) / 6
///   L p(n-1)
pub(crate) fn build_curved_path(points: &[crate::layout::Point]) -> String {
    let n = points.len();
    if n == 0 {
        return String::new();
    }

    let mut d = String::new();
    // d3-path moveTo: "M{x},{y}"
    d.push_str(&format!("M{},{}", points[0].x, points[0].y));

    if n == 1 {
        return d;
    }

    if n == 2 {
        // d3 Basis lineEnd with _point == 2: lineTo the second point
        d.push_str(&format!("L{},{}", points[1].x, points[1].y));
        return d;
    }

    // Basis "point" helper: bezierCurveTo using the two previous points
    // (x0, y0) and (x1, y1) blended toward the incoming point (x, y).
    let bezier =
        |d: &mut String, p0: &crate::layout::Point, p1: &crate::layout::Point, x: f64, y: f64| {
            d.push_str(&format!(
                "C{},{},{},{},{},{}",
                (2.0 * p0.x + p1.x) / 3.0,
                (2.0 * p0.y + p1.y) / 3.0,
                (p0.x + 2.0 * p1.x) / 3.0,
                (p0.y + 2.0 * p1.y) / 3.0,
                (p0.x + 4.0 * p1.x + x) / 6.0,
                (p0.y + 4.0 * p1.y + y) / 6.0,
            ));
        };

    // Third point (case 2 -> 3): lineTo the (5*p0 + p1)/6 blend point first
    d.push_str(&format!(
        "L{},{}",
        (5.0 * points[0].x + points[1].x) / 6.0,
        (5.0 * points[0].y + points[1].y) / 6.0,
    ));

    // Each point from index 2 onward emits one bezier segment
    for i in 2..n {
        bezier(
            &mut d,
            &points[i - 2],
            &points[i - 1],
            points[i].x,
            points[i].y,
        );
    }

    // lineEnd (case 3): one more bezier re-using the final point, then lineTo it
    let last = &points[n - 1];
    bezier(&mut d, &points[n - 2], last, last.x, last.y);
    d.push_str(&format!("L{},{}", last.x, last.y));

    d
}

/// Find the positions of orthogonal corner points.
/// Port of mermaid's `extractCornerPoints` (rendering-elements/edges.js).
fn extract_corner_point_positions(points: &[crate::layout::Point]) -> Vec<usize> {
    let mut positions = Vec::new();
    if points.len() < 3 {
        return positions;
    }
    for i in 1..points.len() - 1 {
        let prev = &points[i - 1];
        let curr = &points[i];
        let next = &points[i + 1];
        // Vertical-then-horizontal corner, or horizontal-then-vertical corner
        let vertical_corner = prev.x == curr.x
            && curr.y == next.y
            && (curr.x - next.x).abs() > 5.0
            && (curr.y - prev.y).abs() > 5.0;
        let horizontal_corner = prev.y == curr.y
            && curr.x == next.x
            && (curr.x - prev.x).abs() > 5.0
            && (curr.y - next.y).abs() > 5.0;
        if vertical_corner || horizontal_corner {
            positions.push(i);
        }
    }
    positions
}

/// Return the point at `distance` from `point_b`, along the direction toward
/// `point_a`. Port of mermaid's `findAdjacentPoint`.
fn find_adjacent_point(
    point_a: &crate::layout::Point,
    point_b: &crate::layout::Point,
    distance: f64,
) -> crate::layout::Point {
    let x_diff = point_b.x - point_a.x;
    let y_diff = point_b.y - point_a.y;
    let length = (x_diff * x_diff + y_diff * y_diff).sqrt();
    let ratio = distance / length;
    crate::layout::Point::new(point_b.x - ratio * x_diff, point_b.y - ratio * y_diff)
}

/// Round off sharp orthogonal corners before curve interpolation.
/// Port of mermaid's `fixCorners` (rendering-elements/edges.js).
fn fix_corners(line_data: &[crate::layout::Point]) -> Vec<crate::layout::Point> {
    let corner_point_positions = extract_corner_point_positions(line_data);
    let mut new_line_data = Vec::with_capacity(line_data.len());
    for (i, point) in line_data.iter().enumerate() {
        if corner_point_positions.contains(&i) {
            let prev_point = &line_data[i - 1];
            let next_point = &line_data[i + 1];
            let corner_point = point;

            let new_prev_point = find_adjacent_point(prev_point, corner_point, 5.0);
            let new_next_point = find_adjacent_point(next_point, corner_point, 5.0);

            let x_diff = new_next_point.x - new_prev_point.x;
            let y_diff = new_next_point.y - new_prev_point.y;
            new_line_data.push(new_prev_point);

            let a = std::f64::consts::SQRT_2 * 2.0;
            let mut new_corner_point = crate::layout::Point::new(corner_point.x, corner_point.y);
            if (next_point.x - prev_point.x).abs() > 10.0
                && (next_point.y - prev_point.y).abs() >= 10.0
            {
                let r = 5.0;
                if corner_point.x == new_prev_point.x {
                    new_corner_point = crate::layout::Point::new(
                        if x_diff < 0.0 {
                            new_prev_point.x - r + a
                        } else {
                            new_prev_point.x + r - a
                        },
                        if y_diff < 0.0 {
                            new_prev_point.y - a
                        } else {
                            new_prev_point.y + a
                        },
                    );
                } else {
                    new_corner_point = crate::layout::Point::new(
                        if x_diff < 0.0 {
                            new_prev_point.x - a
                        } else {
                            new_prev_point.x + a
                        },
                        if y_diff < 0.0 {
                            new_prev_point.y - r + a
                        } else {
                            new_prev_point.y + r - a
                        },
                    );
                }
            }
            new_line_data.push(new_corner_point);
            new_line_data.push(new_next_point);
        } else {
            new_line_data.push(*point);
        }
    }
    new_line_data
}

/// Calculate the angle and deltas between two points.
/// Port of mermaid's `calculateDeltaAndAngle` (utils/lineWithOffset.ts).
fn calculate_delta_and_angle(
    point1: &crate::layout::Point,
    point2: &crate::layout::Point,
) -> (f64, f64, f64) {
    let delta_x = point2.x - point1.x;
    let delta_y = point2.y - point1.y;
    let angle = (delta_y / delta_x).atan();
    (angle, delta_x, delta_y)
}

/// Inset line endpoints so they do not draw under transparent arrow markers.
/// Port of mermaid's `getLineFunctionsWithOffset` (utils/lineWithOffset.ts),
/// applied eagerly to the point list instead of via d3 accessors.
fn apply_marker_offsets(
    data: &[crate::layout::Point],
    start_marker_height: Option<f64>,
    end_marker_height: Option<f64>,
) -> Vec<crate::layout::Point> {
    let n = data.len();
    if n < 2 || (start_marker_height.is_none() && end_marker_height.is_none()) {
        return data.to_vec();
    }

    let first = &data[0];
    let last = &data[n - 1];
    // DIRECTION in the x accessor: 'left' if data[0].x < last.x, else 'right'
    let direction_x_right = first.x >= last.x;
    // DIRECTION in the y accessor: 'down' if data[0].y < last.y, else 'up'
    let direction_y_up = first.y >= last.y;
    let extra_room = 1.0;

    data.iter()
        .enumerate()
        .map(|(i, d)| {
            // x accessor
            let mut x_offset = 0.0;
            if i == 0 {
                if let Some(height) = start_marker_height {
                    let (angle, delta_x, _) = calculate_delta_and_angle(&data[0], &data[1]);
                    x_offset = height * angle.cos() * if delta_x >= 0.0 { 1.0 } else { -1.0 };
                }
            } else if i == n - 1 {
                if let Some(height) = end_marker_height {
                    let (angle, delta_x, _) = calculate_delta_and_angle(&data[n - 1], &data[n - 2]);
                    x_offset = height * angle.cos() * if delta_x >= 0.0 { 1.0 } else { -1.0 };
                }
            }
            if x_offset.is_nan() {
                x_offset = 0.0;
            }

            let difference_to_end = (d.x - last.x).abs();
            let difference_in_y_end = (d.y - last.y).abs();
            let difference_to_start = (d.x - first.x).abs();
            let difference_in_y_start = (d.y - first.y).abs();

            if let Some(end_height) = end_marker_height {
                if difference_to_end < end_height
                    && difference_to_end > 0.0
                    && difference_in_y_end < end_height
                {
                    let mut adjustment = end_height + extra_room - difference_to_end;
                    adjustment *= if direction_x_right { -1.0 } else { 1.0 };
                    x_offset -= adjustment;
                }
            }
            if let Some(start_height) = start_marker_height {
                if difference_to_start < start_height
                    && difference_to_start > 0.0
                    && difference_in_y_start < start_height
                {
                    let mut adjustment = start_height + extra_room - difference_to_start;
                    adjustment *= if direction_x_right { -1.0 } else { 1.0 };
                    x_offset += adjustment;
                }
            }

            // y accessor
            let mut y_offset = 0.0;
            if i == 0 {
                if let Some(height) = start_marker_height {
                    let (angle, _, delta_y) = calculate_delta_and_angle(&data[0], &data[1]);
                    y_offset = height * angle.sin().abs() * if delta_y >= 0.0 { 1.0 } else { -1.0 };
                }
            } else if i == n - 1 {
                if let Some(height) = end_marker_height {
                    let (angle, _, delta_y) = calculate_delta_and_angle(&data[n - 1], &data[n - 2]);
                    y_offset = height * angle.sin().abs() * if delta_y >= 0.0 { 1.0 } else { -1.0 };
                }
            }
            if y_offset.is_nan() {
                y_offset = 0.0;
            }

            let difference_to_end_y = (d.y - last.y).abs();
            let difference_in_x_end = (d.x - last.x).abs();
            let difference_to_start_y = (d.y - first.y).abs();
            let difference_in_x_start = (d.x - first.x).abs();

            if let Some(end_height) = end_marker_height {
                if difference_to_end_y < end_height
                    && difference_to_end_y > 0.0
                    && difference_in_x_end < end_height
                {
                    let mut adjustment = end_height + extra_room - difference_to_end_y;
                    adjustment *= if direction_y_up { -1.0 } else { 1.0 };
                    y_offset -= adjustment;
                }
            }
            if let Some(start_height) = start_marker_height {
                if difference_to_start_y < start_height
                    && difference_to_start_y > 0.0
                    && difference_in_x_start < start_height
                {
                    let mut adjustment = start_height + extra_room - difference_to_start_y;
                    adjustment *= if direction_y_up { -1.0 } else { 1.0 };
                    y_offset += adjustment;
                }
            }

            crate::layout::Point::new(d.x + x_offset, d.y + y_offset)
        })
        .collect()
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
    fn test_edge_label_renders_text() {
        use crate::diagrams::flowchart::{EdgeStroke, FlowEdge, FlowTextType};
        use std::collections::HashMap;

        let layout_edge = LayoutEdge {
            id: "e1".to_string(),
            sources: vec!["a".to_string()],
            targets: vec!["b".to_string()],
            label: Some("label".to_string()),
            bend_points: vec![Point::new(0.0, 0.0), Point::new(100.0, 100.0)],
            label_position: Some(Point::new(50.0, 50.0)),
            label_width: 60.0,
            label_height: 20.0,
            weight: 1,
            minlen: 1,
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

        // Edge label should render text content
        assert!(
            label_svg.contains("<text"),
            "Edge label should render text, got: {}",
            label_svg
        );
    }

    #[test]
    fn test_edge_label_uses_css_class_not_hardcoded_color() {
        use crate::diagrams::flowchart::{EdgeStroke, FlowEdge, FlowTextType};
        use std::collections::HashMap;

        let layout_edge = LayoutEdge {
            id: "e1".to_string(),
            sources: vec!["a".to_string()],
            targets: vec!["b".to_string()],
            label: Some("label".to_string()),
            bend_points: vec![Point::new(0.0, 0.0), Point::new(100.0, 100.0)],
            label_position: Some(Point::new(50.0, 50.0)),
            label_width: 60.0,
            label_height: 20.0,
            weight: 1,
            minlen: 1,
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

        // The edge-label text should NOT have a hardcoded fill color
        // It should use the CSS class for theming
        assert!(
            !svg.contains("fill=\"#e8e8e8\""),
            "Edge label text should not have hardcoded fill '#e8e8e8', got: {}",
            svg
        );
    }

    #[test]
    fn test_curve_basis_matches_d3_exactly() {
        // Faithful port of d3-shape curveBasis: for points (0,0),(6,0),(6,6)
        // d3 produces: M0,0 L(5*p0+p1)/6 C... C... L p_last
        let points = vec![
            Point::new(0.0, 0.0),
            Point::new(6.0, 0.0),
            Point::new(6.0, 6.0),
        ];

        let path = build_curved_path(&points);
        assert_eq!(path, "M0,0L1,0C2,0,4,0,5,1C6,2,6,4,6,5L6,6");
    }

    #[test]
    fn test_curve_basis_two_points_is_line() {
        // d3 curveBasis with two points: M p0 L p1
        let points = vec![Point::new(0.0, 0.0), Point::new(100.0, 100.0)];
        let path = build_curved_path(&points);
        assert_eq!(path, "M0,0L100,100");
    }

    #[test]
    fn test_no_bend_point_simplification() {
        // A 3-point edge whose interior waypoint deviates slightly (10px) from
        // the straight line must NOT be simplified away - mermaid renders ALL
        // dagre points through curveBasis. The interior point (10,50) shows up
        // as the on-curve blend (p0 + 4*p1 + p2)/6 => x = 40/6 = 6.666...
        let points = vec![
            Point::new(0.0, 0.0),
            Point::new(10.0, 50.0),
            Point::new(0.0, 100.0),
        ];

        let path = build_curved_path(&points);
        assert!(
            path.contains('C'),
            "3-point edge must render cubic segments, got: {}",
            path
        );
        assert!(
            path.contains("6.666666666666667"),
            "Interior waypoint must influence the curve shape, got: {}",
            path
        );
    }

    #[test]
    fn test_fix_corners_rounds_right_angle() {
        // Port of mermaid fixCorners: a 90-degree corner at (0,50) between
        // (0,0) and (50,50) is replaced by prev/corner/next points 5px away
        // with the corner nudged by a = 2*sqrt(2).
        let points = vec![
            Point::new(0.0, 0.0),
            Point::new(0.0, 50.0),
            Point::new(50.0, 50.0),
        ];

        let fixed = fix_corners(&points);
        assert_eq!(fixed.len(), 5, "corner expands into 3 points: {:?}", fixed);
        // newPrevPoint: 5px from corner toward prev
        assert!((fixed[1].x - 0.0).abs() < 1e-9 && (fixed[1].y - 45.0).abs() < 1e-9);
        // newCornerPoint: x = prev.x + r - a, y = prev.y + a (r=5, a=2*sqrt(2))
        let a = std::f64::consts::SQRT_2 * 2.0;
        assert!((fixed[2].x - (5.0 - a)).abs() < 1e-9, "got {:?}", fixed[2]);
        assert!((fixed[2].y - (45.0 + a)).abs() < 1e-9, "got {:?}", fixed[2]);
        // newNextPoint: 5px from corner toward next
        assert!((fixed[3].x - 5.0).abs() < 1e-9 && (fixed[3].y - 50.0).abs() < 1e-9);
    }

    #[test]
    fn test_fix_corners_ignores_non_corners() {
        // Diagonal points are not orthogonal corners - passed through unchanged
        let points = vec![
            Point::new(0.0, 0.0),
            Point::new(10.0, 50.0),
            Point::new(0.0, 100.0),
        ];
        let fixed = fix_corners(&points);
        assert_eq!(fixed.len(), 3);
    }

    #[test]
    fn test_marker_offset_insets_arrow_point_endpoint() {
        // Port of mermaid getLineFunctionsWithOffset: an arrow_point end marker
        // insets the final point by 4px along the incoming direction.
        use crate::diagrams::flowchart::{EdgeStroke, FlowEdge, FlowTextType};
        use std::collections::HashMap;

        let layout_edge = LayoutEdge {
            id: "e1".to_string(),
            sources: vec!["a".to_string()],
            targets: vec!["b".to_string()],
            label: None,
            bend_points: vec![
                Point::new(100.0, 0.0),
                Point::new(100.0, 50.0),
                Point::new(100.0, 100.0),
            ],
            label_position: None,
            label_width: 0.0,
            label_height: 0.0,
            weight: 1,
            minlen: 1,
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
            text: String::new(),
            label_type: FlowTextType::Text,
            classes: vec![],
            animation: None,
            animate: None,
        };

        let theme = Theme::default();
        let result = render_edge_parts(&layout_edge, &flow_edge, &theme);
        let svg = result.path.expect("edge path").to_svg(0);

        // Final point y = 100 - 4 = 96; start point untouched (arrowTypeStart = none)
        assert!(
            svg.contains("L100,96"),
            "arrow_point end must inset final point by 4px, got: {}",
            svg
        );
        assert!(
            svg.contains("M100,0"),
            "start point must be untouched, got: {}",
            svg
        );
    }

    fn make_edge(stroke: EdgeStroke) -> (LayoutEdge, FlowEdge) {
        use std::collections::HashMap;

        let layout_edge = LayoutEdge {
            id: "e1".to_string(),
            sources: vec!["a".to_string()],
            targets: vec!["b".to_string()],
            label: None,
            bend_points: vec![Point::new(0.0, 0.0), Point::new(100.0, 100.0)],
            label_position: None,
            label_width: 0.0,
            label_height: 0.0,
            weight: 1,
            minlen: 1,
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
            stroke,
            style: vec![],
            length: None,
            text: String::new(),
            label_type: FlowTextType::Text,
            classes: vec![],
            animation: None,
            animate: None,
        };

        (layout_edge, flow_edge)
    }

    use crate::diagrams::flowchart::FlowTextType;

    #[test]
    fn test_dotted_edge_emits_mermaid_pattern_classes() {
        // Port of mermaid insertEdge: dotted edges are styled via CSS classes
        // (edge-pattern-dotted => stroke-dasharray: 2), not inline attributes.
        let (layout_edge, flow_edge) = make_edge(EdgeStroke::Dotted);
        let theme = Theme::default();
        let result = render_edge_parts(&layout_edge, &flow_edge, &theme);
        let svg = result.path.expect("edge path").to_svg(0);

        assert!(
            svg.contains(
                "edge-thickness-normal edge-pattern-dotted edge-thickness-normal edge-pattern-solid flowchart-link"
            ),
            "dotted edge must carry mermaid stroke classes plus flowDb edge classes, got: {}",
            svg
        );
        assert!(
            !svg.contains("stroke-dasharray=\""),
            "dotted edge must not use an inline stroke-dasharray attribute, got: {}",
            svg
        );
        assert!(
            !svg.contains("stroke-width=\""),
            "edge stroke width must come from CSS classes, got: {}",
            svg
        );
    }

    #[test]
    fn test_thick_edge_emits_mermaid_thickness_classes() {
        let (layout_edge, flow_edge) = make_edge(EdgeStroke::Thick);
        let theme = Theme::default();
        let result = render_edge_parts(&layout_edge, &flow_edge, &theme);
        let svg = result.path.expect("edge path").to_svg(0);

        assert!(
            svg.contains(
                "edge-thickness-thick edge-pattern-solid edge-thickness-normal edge-pattern-solid flowchart-link"
            ),
            "thick edge must carry edge-thickness-thick class, got: {}",
            svg
        );
    }

    #[test]
    fn test_invisible_edge_emits_invisible_class_only() {
        // flowDb getData: invisible edges get empty flowDb classes (no
        // flowchart-link) and arrow markers are suppressed.
        let (layout_edge, flow_edge) = make_edge(EdgeStroke::Invisible);
        let theme = Theme::default();
        let result = render_edge_parts(&layout_edge, &flow_edge, &theme);
        let svg = result.path.expect("edge path").to_svg(0);

        assert!(
            svg.contains("edge-thickness-invisible edge-pattern-solid"),
            "invisible edge must carry edge-thickness-invisible class, got: {}",
            svg
        );
        assert!(
            !svg.contains("flowchart-link"),
            "invisible edge must not carry flowchart-link class, got: {}",
            svg
        );
        assert!(
            !svg.contains("marker-end"),
            "invisible edge must not have arrow markers, got: {}",
            svg
        );
    }

    #[test]
    fn test_edge_inline_styles_from_link_style() {
        // mermaid insertEdge applies edge.style entries as an inline style
        // attribute on the path.
        let (layout_edge, mut flow_edge) = make_edge(EdgeStroke::Normal);
        flow_edge.style = vec!["stroke:#ff3".to_string(), "stroke-width:4px".to_string()];
        let theme = Theme::default();
        let result = render_edge_parts(&layout_edge, &flow_edge, &theme);
        let svg = result.path.expect("edge path").to_svg(0);

        assert!(
            svg.contains("style=\"stroke:#ff3;stroke-width:4px;\""),
            "linkStyle styles must be applied inline on the path, got: {}",
            svg
        );
    }

    #[test]
    fn test_vertical_edge_produces_curved_path() {
        // Vertical points should produce a curved path (C commands), not straight (L)
        // but x-coordinates should remain constant (matching mermaid reference behavior)
        let points = vec![
            Point::new(100.0, 0.0),
            Point::new(100.0, 50.0),
            Point::new(100.0, 100.0),
        ];

        let path = build_curved_path(&points);

        // Should contain curve commands
        assert!(
            path.contains("C"),
            "Vertical edge should produce curved path, got: {}",
            path
        );

        // X-coordinates should all be 100 (no artificial variation)
        // Mermaid keeps vertical edges perfectly aligned
        // The d3-path format is "M{x},{y}L{x},{y}C{x1},{y1},{x2},{y2},{x},{y}..."
        assert!(
            path.contains("100,"),
            "Path should contain x-coordinate 100, got: {}",
            path
        );
        // And there should be no other x values (variations)
        assert!(
            !path.contains("99.") && !path.contains("101."),
            "Vertical edge should not have x variations, got: {}",
            path
        );
    }
}
