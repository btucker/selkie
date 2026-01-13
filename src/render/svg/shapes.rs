//! Shape rendering for flowchart nodes

use crate::diagrams::flowchart::{FlowVertex, FlowVertexType};
use crate::layout::{LayoutNode, Point};

use super::elements::{Attrs, SvgElement};
use super::theme::Theme;

/// Render a node shape based on its type
pub fn render_shape(node: &LayoutNode, vertex: &FlowVertex, _theme: &Theme) -> SvgElement {
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
            let rx = 5.0;
            SvgElement::rounded_rect(x, y, w, h, rx)
        }
        FlowVertexType::Stadium => {
            // Stadium (pill) shape - rectangle with fully rounded ends
            let rx = h / 2.0;
            SvgElement::rounded_rect(x, y, w, h, rx)
        }
        FlowVertexType::Circle => {
            let r = w.max(h) / 2.0;
            SvgElement::circle(cx, cy, r)
        }
        FlowVertexType::DoubleCircle => {
            // Double circle - we'll use a group with two circles
            let r = w.max(h) / 2.0;
            let inner_r = r - 4.0;
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
            let points = vec![
                Point::new(x + inset, y),         // top-left
                Point::new(x + w - inset, y),     // top-right
                Point::new(x + w, cy),            // right
                Point::new(x + w - inset, y + h), // bottom-right
                Point::new(x + inset, y + h),     // bottom-left
                Point::new(x, cy),                // left
            ];
            SvgElement::polygon(points)
        }
        FlowVertexType::Cylinder => {
            // Cylinder (database) shape using path
            let ry = h * 0.15; // ellipse height for top/bottom
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
            // Odd shape (flag-like) - rectangle with notch
            let notch = w * 0.15;
            let points = vec![
                Point::new(x, y),              // top-left
                Point::new(x + w, y),          // top-right
                Point::new(x + w - notch, cy), // right notch
                Point::new(x + w, y + h),      // bottom-right
                Point::new(x, y + h),          // bottom-left
            ];
            SvgElement::polygon(points)
        }
    };

    // Create label
    let label_text = vertex.text.as_deref().unwrap_or(&node.id);
    let label = SvgElement::text(cx, cy, label_text).with_attrs(
        Attrs::new()
            .with_class("label")
            .with_attr("text-anchor", "middle")
            .with_attr("dominant-baseline", "central"),
    );

    // Wrap shape and label in a group with class="node"
    // This allows CSS selectors like ".node rect" to work correctly
    let group_attrs = Attrs::new()
        .with_class("node")
        .with_id(&format!("node-{}", node.id));

    SvgElement::group(vec![shape, label]).with_attrs(group_attrs)
}

#[cfg(test)]
mod tests {
    use super::*;

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
        let shape_element = render_shape(&node, &vertex, &theme);
        let svg = shape_element.to_svg(0);

        // The subroutine lines should NOT have hardcoded stroke color
        // They should use CSS class for theming
        assert!(
            !svg.contains("stroke=\"#9370DB\""),
            "Subroutine lines should not have hardcoded stroke '#9370DB', got: {}",
            svg
        );
    }
}
