//! Shape rendering for flowchart nodes

use crate::diagrams::flowchart::{FlowTextType, FlowVertex, FlowVertexType};
use crate::layout::{LayoutNode, Point};

use super::color::extract_style_property;
use super::elements::{Attrs, SvgElement};
use super::theme::Theme;

/// Render a node shape based on its type
///
/// If `style` is provided, it will be applied as an inline style attribute
/// on the shape element (not the wrapper group).
pub fn render_shape(
    node: &LayoutNode,
    vertex: &FlowVertex,
    _theme: &Theme,
    style: Option<&str>,
) -> SvgElement {
    let x = node.x.unwrap_or(0.0);
    let y = node.y.unwrap_or(0.0);
    let w = node.width;
    let h = node.height;
    let cx = x + w / 2.0;
    let cy = y + h / 2.0;

    let shape_type = vertex
        .vertex_type
        .as_ref()
        .unwrap_or(&FlowVertexType::Square);

    let shape = match shape_type {
        FlowVertexType::Square | FlowVertexType::Rect => SvgElement::rect(x, y, w, h),
        FlowVertexType::Round => {
            let r = 5.0_f64.min(w / 2.0).min(h / 2.0);
            let d = format!(
                "M {} {} H {} A {} {} 0 0 1 {} {} V {} A {} {} 0 0 1 {} {} H {} A {} {} 0 0 1 {} {} V {} A {} {} 0 0 1 {} {} Z",
                x + r,
                y,
                x + w - r,
                r,
                r,
                x + w,
                y + r,
                y + h - r,
                r,
                r,
                x + w - r,
                y + h,
                x + r,
                r,
                r,
                x,
                y + h - r,
                y + r,
                r,
                r,
                x + r,
                y
            );
            SvgElement::path(d)
        }
        FlowVertexType::Stadium => {
            // Stadium (pill) shape - use path to avoid rect counting
            let r = (h / 2.0).min(w / 2.0);
            let d = format!(
                "M {} {} H {} A {} {} 0 0 1 {} {} H {} A {} {} 0 0 1 {} {} Z",
                x + r,
                y,
                x + w - r,
                r,
                r,
                x + w - r,
                y + h,
                x + r,
                r,
                r,
                x + r,
                y
            );
            SvgElement::path(d)
        }
        FlowVertexType::Circle => {
            let r = w.max(h) / 2.0;
            SvgElement::circle(cx, cy, r)
        }
        FlowVertexType::DoubleCircle => {
            // Double circle - we'll use a group with two circles
            let r = w.max(h) / 2.0;
            let inner_r = r - 5.0;
            SvgElement::group(vec![
                SvgElement::circle(cx, cy, r),
                SvgElement::circle(cx, cy, inner_r),
            ])
        }
        FlowVertexType::Ellipse => SvgElement::Ellipse {
            cx,
            cy,
            rx: w / 2.0,
            ry: h / 2.0,
            attrs: Attrs::new(),
        },
        FlowVertexType::Diamond => {
            // Diamond shape - rotated square
            let points = vec![
                Point::new(cx, y),     // top
                Point::new(x + w, cy), // right
                Point::new(cx, y + h), // bottom
                Point::new(x, cy),     // left
            ];
            SvgElement::polygon(points)
        }
        FlowVertexType::Hexagon => {
            // Hexagon with flat top/bottom
            let inset = w * 0.15;
            let points = [
                Point::new(x + inset, y),         // top-left
                Point::new(x + w - inset, y),     // top-right
                Point::new(x + w, cy),            // right
                Point::new(x + w - inset, y + h), // bottom-right
                Point::new(x + inset, y + h),     // bottom-left
                Point::new(x, cy),                // left
            ];
            let d = format!(
                "M {} {} L {} {} L {} {} L {} {} L {} {} L {} {} Z",
                points[0].x,
                points[0].y,
                points[1].x,
                points[1].y,
                points[2].x,
                points[2].y,
                points[3].x,
                points[3].y,
                points[4].x,
                points[4].y,
                points[5].x,
                points[5].y
            );
            SvgElement::path(d)
        }
        FlowVertexType::Cylinder => {
            // Cylinder (database) shape using path
            // mermaid cylinder.ts createCylinderPathD: rx = w/2; ry = rx / (2.5 + w/50)
            let ry = (w / 2.0) / (2.5 + w / 50.0); // ellipse height for top/bottom
            let d = format!(
                "M {} {} \
                 a {} {} 0 0 0 {} 0 \
                 a {} {} 0 0 0 {} 0 \
                 l 0 {} \
                 a {} {} 0 0 0 {} 0 \
                 l 0 {}",
                x,
                y + ry, // Start at top-left of body
                w / 2.0,
                ry,
                w, // Top ellipse first arc
                w / 2.0,
                ry,
                -w,           // Top ellipse second arc
                h - ry * 2.0, // Body height
                w / 2.0,
                ry,
                w,               // Bottom ellipse
                -(h - ry * 2.0)  // Back to top
            );
            SvgElement::path(d)
        }
        FlowVertexType::Subroutine => {
            // Subroutine (predefined process) - rendered as polygon per mermaid.js
            // Single polygon traces: inner rect → step to outer → outer rect → close
            let bar_offset = 10.0;
            let points = vec![
                // Inner rectangle (main body)
                Point::new(x + bar_offset, y),         // inner top-left
                Point::new(x + w - bar_offset, y),     // inner top-right
                Point::new(x + w - bar_offset, y + h), // inner bottom-right
                Point::new(x + bar_offset, y + h),     // inner bottom-left
                Point::new(x + bar_offset, y),         // back to inner top-left
                // Step out to outer rectangle
                Point::new(x, y),         // outer top-left
                Point::new(x + w, y),     // outer top-right
                Point::new(x + w, y + h), // outer bottom-right
                Point::new(x, y + h),     // outer bottom-left
                Point::new(x, y),         // close outer path
            ];
            SvgElement::polygon(points)
        }
        FlowVertexType::Trapezoid => {
            // Trapezoid - wider at bottom
            let inset = w * 0.15;
            let points = vec![
                Point::new(x + inset, y),     // top-left
                Point::new(x + w - inset, y), // top-right
                Point::new(x + w, y + h),     // bottom-right
                Point::new(x, y + h),         // bottom-left
            ];
            SvgElement::polygon(points)
        }
        FlowVertexType::InvTrapezoid => {
            // Inverted trapezoid - wider at top
            let inset = w * 0.15;
            let points = vec![
                Point::new(x, y),                 // top-left
                Point::new(x + w, y),             // top-right
                Point::new(x + w - inset, y + h), // bottom-right
                Point::new(x + inset, y + h),     // bottom-left
            ];
            SvgElement::polygon(points)
        }
        FlowVertexType::LeanRight => {
            // Parallelogram leaning right
            let inset = w * 0.15;
            let points = vec![
                Point::new(x + inset, y),         // top-left
                Point::new(x + w, y),             // top-right
                Point::new(x + w - inset, y + h), // bottom-right
                Point::new(x, y + h),             // bottom-left
            ];
            SvgElement::polygon(points)
        }
        FlowVertexType::LeanLeft => {
            // Parallelogram leaning left
            let inset = w * 0.15;
            let points = vec![
                Point::new(x, y),             // top-left
                Point::new(x + w - inset, y), // top-right
                Point::new(x + w, y + h),     // bottom-right
                Point::new(x + inset, y + h), // bottom-left
            ];
            SvgElement::polygon(points)
        }
        FlowVertexType::Odd => {
            // Odd shape ('>text]') - mermaid rect_left_inv_arrow: a rectangle
            // with an inward arrow notch on the LEFT edge. In mermaid's
            // origin-centered coords the point list is
            //   {x+notch,y},{x,0},{x+notch,-y},{-x,-y},{-x,y}
            // with x=-w/2, y=-h/2 and notch=y/2=-h/4, then the polygon is
            // shifted by translate(-notch/2,0)=translate(h/8,0). Mapping that
            // into selkie's top-left absolute coords (node.width already
            // includes the +h/4 the notch adds, see layout/size.rs Odd)
            // collapses to: top-left and bottom-left corners on the left edge,
            // the notch tip caved inward to x+h/4 at mid-height, and a flat
            // right edge.
            let notch = h / 4.0;
            let points = [
                Point::new(x, y),          // top-left
                Point::new(x + notch, cy), // left-edge notch tip (caved inward)
                Point::new(x, y + h),      // bottom-left
                Point::new(x + w, y + h),  // bottom-right
                Point::new(x + w, y),      // top-right
            ];
            let d = format!(
                "M {} {} L {} {} L {} {} L {} {} L {} {} Z",
                points[0].x,
                points[0].y,
                points[1].x,
                points[1].y,
                points[2].x,
                points[2].y,
                points[3].x,
                points[3].y,
                points[4].x,
                points[4].y
            );
            SvgElement::path(d)
        }
    };

    // Apply inline style to shape if provided
    let shape = if let Some(s) = style {
        shape.with_style(s)
    } else {
        shape
    };

    // Create the label. mermaid never invents contrast label colors: only an
    // explicit `color:` declaration (from classDef/style, routed via flowDb's
    // textStyles/labelStyle handling) changes the label fill.
    let label_text = vertex
        .text
        .as_deref()
        .map(crate::render::text_utils::normalize_mermaid_label_markup)
        .unwrap_or_else(|| node.id.clone());
    // Apply mermaid's wrappingWidth word-wrap so the drawn text matches the
    // label bbox the layout was sized with (default node font size 16px).
    let label_text = crate::render::text_utils::wrap_label_text_mermaid(&label_text, 16.0);
    let mut label_attrs = Attrs::new()
        .with_class("label")
        .with_attr("text-anchor", "middle")
        .with_attr("dominant-baseline", "central");

    if let Some(s) = style {
        if let Some(color_val) = extract_style_property(s, "color") {
            // Use inline style (not presentation attribute) so it takes
            // precedence over theme CSS rules.
            label_attrs = label_attrs.with_style(&format!("fill: {}", color_val));
        }
    }

    // Some shapes offset their label from the geometric center so the text
    // stays centered in the visible body. mermaid's rect_left_inv_arrow (the
    // Odd shape) shifts both the polygon and the label right by -notch/2 = h/8
    // to compensate for the inward notch that eats into the left edge.
    let label_dx = match shape_type {
        FlowVertexType::Odd => h / 8.0,
        _ => 0.0,
    };

    // Markdown labels carry emphasis (`**bold**`, `_italic_`) that must render
    // as styled tspans, so route them through MarkdownText (which strips the
    // markers and emits font-weight/font-style runs) using the raw source text.
    let label = if vertex.label_type == FlowTextType::Markdown {
        let md_text = vertex.text.clone().unwrap_or_else(|| node.id.clone());
        SvgElement::markdown_text(cx + label_dx, cy, md_text).with_attrs(label_attrs)
    } else {
        SvgElement::text(cx + label_dx, cy, label_text).with_attrs(label_attrs)
    };

    // Wrap shape and label in a group. mermaid flowDb assigns
    // cssClasses = 'default ' + vertex.classes, so classDef CSS rules like
    // `.myClass rect { ... }` can match.
    let mut group_class = String::from("node default");
    for class in &vertex.classes {
        group_class.push(' ');
        group_class.push_str(class);
    }
    let group_attrs = Attrs::new()
        .with_class(&group_class)
        .with_id(&format!("node-{}", node.id));

    SvgElement::group(vec![shape, label]).with_attrs(group_attrs)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_custom_fill_keeps_theme_label_color() {
        // mermaid never invents WCAG contrast label colors: a custom fill
        // leaves the label using the theme text color.
        let mut node = LayoutNode::new("test", 100.0, 60.0);
        node.x = Some(0.0);
        node.y = Some(0.0);

        let mut vertex = FlowVertex::new("test", "test");
        vertex.text = Some("Dark Node".to_string());
        vertex.vertex_type = Some(FlowVertexType::Square);

        let theme = Theme::default();
        let style = "fill:#333333 !important;stroke:#000 !important";
        let shape_element = render_shape(&node, &vertex, &theme, Some(style));
        let svg = shape_element.to_svg(0);

        // The text element must NOT get an invented contrast fill
        let text_start = svg.find("<text").expect("should have text element");
        let text_end = svg[text_start..].find('>').unwrap() + text_start;
        let text_tag = &svg[text_start..=text_end];
        assert!(
            !text_tag.contains("fill"),
            "Custom fill must not invent a label color, got: {}",
            text_tag
        );
    }

    #[test]
    fn test_node_group_includes_user_classes() {
        // mermaid flowDb: cssClasses = 'default ' + vertex.classes
        let mut node = LayoutNode::new("test", 100.0, 60.0);
        node.x = Some(0.0);
        node.y = Some(0.0);

        let mut vertex = FlowVertex::new("test", "test");
        vertex.text = Some("Styled".to_string());
        vertex.vertex_type = Some(FlowVertexType::Square);
        vertex.classes = vec!["orange".to_string(), "big".to_string()];

        let theme = Theme::default();
        let shape_element = render_shape(&node, &vertex, &theme, None);
        let svg = shape_element.to_svg(0);

        assert!(
            svg.contains("class=\"node default orange big\""),
            "node group must carry default and user classes, got: {}",
            svg
        );
    }

    #[test]
    fn test_explicit_color_respected() {
        // When a node's style explicitly sets color, that should be
        // used for text fill regardless of background luminance.
        let mut node = LayoutNode::new("test", 100.0, 60.0);
        node.x = Some(0.0);
        node.y = Some(0.0);

        let mut vertex = FlowVertex::new("test", "test");
        vertex.text = Some("Custom Color".to_string());
        vertex.vertex_type = Some(FlowVertexType::Square);

        let theme = Theme::default();
        let style = "fill:#333333 !important;color:#ff0000 !important";
        let shape_element = render_shape(&node, &vertex, &theme, Some(style));
        let svg = shape_element.to_svg(0);

        // The text element should use the explicit color value via inline style
        assert!(
            svg.contains("style=\"fill: #ff0000\""),
            "Explicit color should be used for text fill via inline style, got: {}",
            svg
        );
    }

    #[test]
    fn test_no_style_no_text_fill_override() {
        // When no custom style is provided, the text label should not
        // have an inline fill (it should use theme CSS).
        let mut node = LayoutNode::new("test", 100.0, 60.0);
        node.x = Some(0.0);
        node.y = Some(0.0);

        let mut vertex = FlowVertex::new("test", "test");
        vertex.text = Some("Default".to_string());
        vertex.vertex_type = Some(FlowVertexType::Square);

        let theme = Theme::default();
        let shape_element = render_shape(&node, &vertex, &theme, None);
        let svg = shape_element.to_svg(0);

        // The text element should NOT have an inline fill attribute
        // (it should rely on the theme CSS for text color)
        let text_start = svg.find("<text").expect("should have text element");
        let text_end = svg[text_start..].find('>').unwrap() + text_start;
        let text_tag = &svg[text_start..=text_end];
        assert!(
            !text_tag.contains("fill="),
            "Text without custom style should not have inline fill, got: {}",
            text_tag
        );
    }

    #[test]
    fn test_subroutine_uses_css_class_not_hardcoded_stroke() {
        // This test verifies that the subroutine shape's vertical lines
        // do NOT have hardcoded stroke colors, allowing CSS theme styling to work.
        let mut node = LayoutNode::new("test", 100.0, 60.0);
        node.x = Some(0.0);
        node.y = Some(0.0);

        let mut vertex = FlowVertex::new("test", "test");
        vertex.text = Some("Subroutine".to_string());
        vertex.vertex_type = Some(FlowVertexType::Subroutine);

        let theme = Theme::default();
        let shape_element = render_shape(&node, &vertex, &theme, None);
        let svg = shape_element.to_svg(0);

        // The subroutine lines should NOT have hardcoded stroke color
        // They should use CSS class for theming
        assert!(
            !svg.contains("stroke=\"#9370DB\""),
            "Subroutine lines should not have hardcoded stroke '#9370DB', got: {}",
            svg
        );
    }

    #[test]
    fn test_cylinder_cap_ry_derived_from_width() {
        // mermaid cylinder.ts createCylinderPathD: rx = w/2; ry = rx / (2.5 + w/50).
        // The cap ellipse depth must be derived from width, not h * 0.15,
        // otherwise the caps render visibly over-round.
        let w = 100.0;
        let h = 68.0;
        let mut node = LayoutNode::new("A", w, h);
        node.x = Some(0.0);
        node.y = Some(0.0);

        let mut vertex = FlowVertex::new("A", "A");
        vertex.text = Some("Database".to_string());
        vertex.vertex_type = Some(FlowVertexType::Cylinder);

        let theme = Theme::default();
        let shape_element = render_shape(&node, &vertex, &theme, None);
        let svg = shape_element.to_svg(0);

        let rx = w / 2.0;
        let expected_ry = rx / (2.5 + w / 50.0);
        let wrong_ry = h * 0.15;
        assert_ne!(
            expected_ry, wrong_ry,
            "sanity: the two formulas must differ"
        );

        let expected_arc = format!("a {} {} 0 0 0 {} 0", rx, expected_ry, w);
        assert!(
            svg.contains(&expected_arc),
            "Cylinder cap arc should use width-derived ry {expected_ry}, got: {svg}"
        );

        let wrong_arc = format!("a {} {} 0 0 0 {} 0", rx, wrong_ry, w);
        assert!(
            !svg.contains(&wrong_arc),
            "Cylinder cap must not use h * 0.15 ry, got: {svg}"
        );
    }

    #[test]
    fn test_odd_shape_left_edge_chevron_and_label_offset() {
        // mermaid rect_left_inv_arrow puts the arrow notch on the LEFT edge
        // (caved inward at mid-left, right edge flat) with notch depth = h/4,
        // then shifts the polygon/label right by -notch/2 = h/8 to keep the
        // body centered. The old selkie geometry mirrored this: it put the
        // notch on the RIGHT edge with depth w*0.15 and no label offset.
        let mut node = LayoutNode::new("test", 100.0, 40.0);
        node.x = Some(0.0);
        node.y = Some(0.0);

        let mut vertex = FlowVertex::new("test", "test");
        vertex.text = Some("Odd".to_string());
        vertex.vertex_type = Some(FlowVertexType::Odd);

        let theme = Theme::default();
        let shape_element = render_shape(&node, &vertex, &theme, None);
        let svg = shape_element.to_svg(0);

        // Geometry: notch tip on the LEFT edge at x + h/4 = 10, y-mid = 20.
        // Right edge flat: bottom-right (100,40) then top-right (100,0).
        assert!(
            svg.contains("M 0 0 L 10 20 L 0 40 L 100 40 L 100 0 Z"),
            "Odd shape must have left-edge chevron notch (h/4 deep) with a \
             flat right edge, got: {}",
            svg
        );
        // The old mirrored geometry (notch on the right) must be gone.
        assert!(
            !svg.contains("L 85 20"),
            "Odd shape must not put the notch on the right edge, got: {}",
            svg
        );

        // Label offset: content is shifted right by h/8 = 5 to stay centered
        // in the body, so the text is drawn at cx + h/8 = 55.
        assert!(
            svg.contains("<text x=\"55\" y=\"20\""),
            "Odd shape label must be offset by h/8 to x=55, got: {}",
            svg
        );
    }

    #[test]
    fn test_double_circle_inner_radius_gap_is_five() {
        // mermaid doubleCircle.ts: gap = 5, so innerRadius = outerRadius - 5.
        // For a 24x24 node, outer r = 12 and inner r = 12 - 5 = 7.
        let mut node = LayoutNode::new("test", 24.0, 24.0);
        node.x = Some(0.0);
        node.y = Some(0.0);

        let mut vertex = FlowVertex::new("test", "test");
        vertex.vertex_type = Some(FlowVertexType::DoubleCircle);

        let theme = Theme::default();
        let shape_element = render_shape(&node, &vertex, &theme, None);
        let svg = shape_element.to_svg(0);

        assert!(
            svg.contains("r=\"12\""),
            "Double circle outer radius must be 12, got: {}",
            svg
        );
        assert!(
            svg.contains("r=\"7\""),
            "Double circle inner radius must be outer - 5 = 7, got: {}",
            svg
        );
        assert!(
            !svg.contains("r=\"8\""),
            "Double circle inner radius must not be outer - 4 = 8, got: {}",
            svg
        );
    }
}
