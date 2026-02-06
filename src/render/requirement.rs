//! Requirement diagram renderer
//!
//! Renders requirement diagrams showing requirements, elements, and their relationships.

use std::collections::HashMap;

use crate::diagrams::requirement::{
    Element, RelationshipType, Requirement, RequirementDb, RequirementType, RiskLevel, VerifyType,
};
use crate::error::Result;
use crate::layout::{
    layout, CharacterSizeEstimator, LayoutDirection, LayoutEdge, LayoutGraph, LayoutNode,
    LayoutOptions, LayoutRanker, NodeShape, Padding, Point, SizeEstimator, ToLayoutGraph,
};
use crate::render::svg::edges::build_curved_path;
use crate::render::svg::{Attrs, RenderConfig, SvgDocument, SvgElement, Theme};

/// Box dimensions for requirements and elements
#[derive(Debug, Clone)]
struct BoxDimensions {
    width: f64,
    height: f64,
}

/// Box dimensions sized to match mermaid reference exactly
/// Mermaid requirement boxes are ~158-241px wide (content-based), 184px tall
/// Element boxes are ~138-203px wide, 112-136px tall
/// Mermaid uses 16px base font with 24px line height
const DEFAULT_REQ_BOX_WIDTH: f64 = 158.0; // Match mermaid minimum
const DEFAULT_ELEM_BOX_WIDTH: f64 = 138.0; // Match mermaid element minimum
const DEFAULT_BOX_HEIGHT: f64 = 184.0; // Match mermaid requirement height
const BOX_PADDING: f64 = 10.0; // Inner padding (mermaid uses ~10px)
const LINE_HEIGHT: f64 = 24.0; // Match mermaid's 24px line height exactly
const FONT_SIZE: f64 = 16.0; // Match mermaid's base font size

/// Calculate box dimensions based on content - matches mermaid's sizing
fn calculate_requirement_dimensions(req: &Requirement) -> BoxDimensions {
    // Character width tuned to match reference diagram widths
    // Reference: 551px, with 7.0 we get 530px, with 7.5 we should get ~560px
    let char_width = 7.5;

    // Calculate width needed for each line of content
    let type_width = format_requirement_type(&req.req_type).len() as f64 * char_width;
    let name_width = req.name.len() as f64 * char_width;
    let id_width = if !req.requirement_id.is_empty() {
        format!("ID: {}", req.requirement_id).len() as f64 * char_width
    } else {
        0.0
    };
    let text_width = if !req.text.is_empty() {
        format!("Text: {}", req.text).len() as f64 * char_width
    } else {
        0.0
    };
    let risk_width = format!("Risk: {}", format_risk(&req.risk)).len() as f64 * char_width;
    let verify_width = format!("Verification: {}", format_verify_method(&req.verify_method)).len()
        as f64
        * char_width;

    // Find widest content line
    let content_width = type_width
        .max(name_width)
        .max(id_width)
        .max(text_width)
        .max(risk_width)
        .max(verify_width);

    // Add padding on both sides
    let width = (content_width + BOX_PADDING * 2.0).max(DEFAULT_REQ_BOX_WIDTH);

    // Mermaid requirement boxes are consistently 184px tall
    // This provides enough space for: type label, name, ID, text, risk, verification
    let height = DEFAULT_BOX_HEIGHT;

    BoxDimensions { width, height }
}

/// Calculate element dimensions based on content - matches mermaid's sizing
/// Mermaid element boxes are 112-136px tall depending on content
fn calculate_element_dimensions(elem: &Element) -> BoxDimensions {
    // Character width tuned to match reference diagram widths
    let char_width = 7.5;

    // Calculate width for each line
    let header_width = "<<Element>>".len() as f64 * char_width;
    let name_width = elem.name.len() as f64 * char_width;
    let type_width = if !elem.element_type.is_empty() {
        let type_text = elem.element_type.trim_matches('"');
        format!("Type: {}", type_text).len() as f64 * char_width
    } else {
        0.0
    };
    let docref_width = if !elem.doc_ref.is_empty() {
        format!("Doc Ref: {}", elem.doc_ref).len() as f64 * char_width
    } else {
        0.0
    };

    let content_width = header_width
        .max(name_width)
        .max(type_width)
        .max(docref_width);
    let width = (content_width + BOX_PADDING * 2.0).max(DEFAULT_ELEM_BOX_WIDTH);

    // Calculate height based on content lines
    // Base: header line + name line + divider area = ~48px
    // Plus content lines at 24px each
    let mut content_lines = 0;
    if !elem.element_type.is_empty() {
        content_lines += 1;
    }
    if !elem.doc_ref.is_empty() {
        content_lines += 1;
    }

    // Match mermaid's element heights: 112px (2 lines), 136px (3 lines)
    let height = match content_lines {
        0 => 88.0,  // Just header + name
        1 => 112.0, // + type OR docref
        _ => 136.0, // + type AND docref
    };

    BoxDimensions { width, height }
}

/// Format requirement type for display (with guillemets to match mermaid)
fn format_requirement_type(req_type: &RequirementType) -> &'static str {
    match req_type {
        RequirementType::Requirement => "<<Requirement>>",
        RequirementType::FunctionalRequirement => "<<Functional Requirement>>",
        RequirementType::InterfaceRequirement => "<<Interface Requirement>>",
        RequirementType::PerformanceRequirement => "<<Performance Requirement>>",
        RequirementType::PhysicalRequirement => "<<Physical Requirement>>",
        RequirementType::DesignConstraint => "<<Design Constraint>>",
    }
}

/// Format relationship type for display (with guillemets to match mermaid)
fn format_relationship_type(rel_type: &RelationshipType) -> &'static str {
    match rel_type {
        RelationshipType::Contains => "<<contains>>",
        RelationshipType::Copies => "<<copies>>",
        RelationshipType::Derives => "<<derives>>",
        RelationshipType::Satisfies => "<<satisfies>>",
        RelationshipType::Verifies => "<<verifies>>",
        RelationshipType::Refines => "<<refines>>",
        RelationshipType::Traces => "<<traces>>",
    }
}

/// Format risk level for display
fn format_risk(risk: &RiskLevel) -> &'static str {
    match risk {
        RiskLevel::Low => "Low",
        RiskLevel::Medium => "Medium",
        RiskLevel::High => "High",
    }
}

/// Format verify method for display
fn format_verify_method(method: &VerifyType) -> &'static str {
    match method {
        VerifyType::Analysis => "Analysis",
        VerifyType::Demonstration => "Demonstration",
        VerifyType::Inspection => "Inspection",
        VerifyType::Test => "Test",
    }
}

/// Implement ToLayoutGraph for RequirementDb
impl ToLayoutGraph for RequirementDb {
    fn to_layout_graph(&self, _size_estimator: &dyn SizeEstimator) -> Result<LayoutGraph> {
        let mut graph = LayoutGraph::new("requirement");

        // Set layout options
        let direction = match self.get_direction() {
            "TB" => LayoutDirection::TopToBottom,
            "BT" => LayoutDirection::BottomToTop,
            "LR" => LayoutDirection::LeftToRight,
            "RL" => LayoutDirection::RightToLeft,
            _ => LayoutDirection::TopToBottom,
        };

        // Match mermaid's dagre configuration for requirement diagrams.
        // Use longest-path ranking which produces better column ordering,
        // combined with post-processing to pull source-only nodes down
        // (see pull_sources_toward_targets in layout/dagre/rank/mod.rs).
        graph.options = LayoutOptions {
            direction,
            node_spacing: 50.0,
            layer_spacing: 50.0,
            padding: Padding::uniform(8.0),
            ranker: LayoutRanker::LongestPath,
        };

        // Add nodes in declaration order: requirements first, then elements.
        // This matches mermaid's dagre behavior where node insertion order
        // affects the initial ordering heuristic for column placement.
        for name in self.requirement_names_ordered() {
            let req = self.get_requirements().get(name).unwrap();
            let dims = calculate_requirement_dimensions(req);
            let node = LayoutNode::new(name, dims.width, dims.height)
                .with_shape(NodeShape::Rectangle)
                .with_label(name);
            graph.add_node(node);
        }

        for name in self.element_names_ordered() {
            let elem = self.get_elements().get(name).unwrap();
            let dims = calculate_element_dimensions(elem);
            let node = LayoutNode::new(name, dims.width, dims.height)
                .with_shape(NodeShape::Rectangle)
                .with_label(name);
            graph.add_node(node);
        }

        // Add relationship edges
        for (i, rel) in self.get_relationships().iter().enumerate() {
            let edge_id = format!("rel-{}", i);
            let label = format_relationship_type(&rel.rel_type);
            let edge = LayoutEdge::new(&edge_id, &rel.src, &rel.dst).with_label(label);
            graph.add_edge(edge);
        }

        Ok(graph)
    }

    fn preferred_direction(&self) -> LayoutDirection {
        match self.get_direction() {
            "TB" => LayoutDirection::TopToBottom,
            "BT" => LayoutDirection::BottomToTop,
            "LR" => LayoutDirection::LeftToRight,
            "RL" => LayoutDirection::RightToLeft,
            _ => LayoutDirection::TopToBottom,
        }
    }
}

/// Render a requirement diagram to SVG
pub fn render_requirement(db: &RequirementDb, config: &RenderConfig) -> Result<String> {
    let mut doc = SvgDocument::new();
    let margin = 8.0; // Match mermaid's minimal padding

    let requirements = db.get_requirements();
    let elements = db.get_elements();

    if requirements.is_empty() && elements.is_empty() {
        // Empty diagram
        doc.set_size(400.0, 200.0);
        return Ok(doc.to_string());
    }

    // Calculate dimensions for all nodes
    let mut node_dimensions: HashMap<String, BoxDimensions> = HashMap::new();
    for (name, req) in requirements {
        node_dimensions.insert(name.clone(), calculate_requirement_dimensions(req));
    }
    for (name, elem) in elements {
        node_dimensions.insert(name.clone(), calculate_element_dimensions(elem));
    }

    // Use layout algorithm
    let size_estimator = CharacterSizeEstimator::default();
    let layout_input = db.to_layout_graph(&size_estimator)?;
    let layout_result = layout(layout_input)?;

    // Extract positions
    let mut node_positions: HashMap<String, (f64, f64)> = HashMap::new();
    for node in &layout_result.nodes {
        if let (Some(x), Some(y)) = (node.x, node.y) {
            node_positions.insert(node.id.clone(), (x, y));
        }
    }

    // Extract edge bend points
    let mut edge_bend_points: HashMap<String, Vec<Point>> = HashMap::new();
    for edge in &layout_result.edges {
        if !edge.bend_points.is_empty() {
            edge_bend_points.insert(edge.id.clone(), edge.bend_points.clone());
        }
    }

    // Calculate diagram bounds
    let max_width = layout_result.width.unwrap_or(400.0) + margin * 2.0;
    let max_height = layout_result.height.unwrap_or(200.0) + margin * 2.0;

    doc.set_size(max_width, max_height);

    // Add theme styles
    if config.embed_css {
        doc.add_style(&config.theme.generate_css());
        doc.add_style(&generate_requirement_css(&config.theme));
    }

    // Add marker definitions
    doc.add_defs(generate_requirement_markers());

    // Render relationships first (so nodes paint on top)
    let relationships = db.get_relationships();
    for (idx, rel) in relationships.iter().enumerate() {
        let edge_id = format!("rel-{}", idx);

        if let Some(bend_points) = edge_bend_points.get(&edge_id) {
            let rel_elem = render_relationship_from_bend_points(
                bend_points,
                format_relationship_type(&rel.rel_type),
                &rel.rel_type,
            );
            doc.add_element(rel_elem);
        } else {
            // Fallback: straight line between nodes
            let src_pos = node_positions.get(&rel.src);
            let dst_pos = node_positions.get(&rel.dst);
            let src_dims = node_dimensions.get(&rel.src);
            let dst_dims = node_dimensions.get(&rel.dst);

            if let (Some(&(x1, y1)), Some(&(x2, y2)), Some(d1), Some(d2)) =
                (src_pos, dst_pos, src_dims, dst_dims)
            {
                let rel_elem = render_relationship_line(
                    x1,
                    y1,
                    d1.width,
                    d1.height,
                    x2,
                    y2,
                    d2.width,
                    d2.height,
                    format_relationship_type(&rel.rel_type),
                    &rel.rel_type,
                );
                doc.add_element(rel_elem);
            }
        }
    }

    // Render requirements
    for (name, req) in requirements {
        if let Some(&(x, y)) = node_positions.get(name) {
            let dims = node_dimensions.get(name).cloned().unwrap_or(BoxDimensions {
                width: DEFAULT_REQ_BOX_WIDTH,
                height: DEFAULT_BOX_HEIGHT,
            });
            let req_elem =
                render_requirement_box(req, x, y, dims.width, dims.height, &config.theme);
            doc.add_element(req_elem);
        }
    }

    // Render elements
    for (name, elem) in elements {
        if let Some(&(x, y)) = node_positions.get(name) {
            let dims = node_dimensions.get(name).cloned().unwrap_or(BoxDimensions {
                width: DEFAULT_ELEM_BOX_WIDTH,
                height: 80.0,
            });
            let elem_elem = render_element_box(elem, x, y, dims.width, dims.height, &config.theme);
            doc.add_element(elem_elem);
        }
    }

    Ok(doc.to_string())
}

/// Render a requirement box - matches mermaid's layout
/// Mermaid uses centered type label at top, then left-aligned content below divider
fn render_requirement_box(
    req: &Requirement,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    theme: &Theme,
) -> SvgElement {
    // IMPORTANT: Render all shapes first, then all text elements
    // This ensures proper z-order (text appears on top of shapes)
    let mut children = Vec::new();

    // Mermaid positions:
    // - Type label at y + 8 (centered)
    // - Name at y + 32 (bold, centered)
    // - Divider at y + 68
    // - Content starts at y + 76

    let divider_y = y + 68.0; // Match mermaid's divider position
    let content_start_y = divider_y + 8.0;

    // === SHAPES FIRST ===
    // Main box
    children.push(SvgElement::Rect {
        x,
        y,
        width,
        height,
        rx: Some(0.0),
        ry: Some(0.0),
        attrs: Attrs::new()
            .with_fill(&theme.primary_color)
            .with_stroke(&theme.primary_border_color)
            .with_stroke_width(2.0)
            .with_class("requirement-box"),
    });

    // Header background (same color, for visual consistency)
    children.push(SvgElement::Rect {
        x,
        y,
        width,
        height: divider_y - y,
        rx: Some(0.0),
        ry: Some(0.0),
        attrs: Attrs::new()
            .with_fill(&theme.primary_color)
            .with_stroke(&theme.primary_border_color)
            .with_class("requirement-header"),
    });

    // Divider line
    children.push(SvgElement::Line {
        x1: x,
        y1: divider_y,
        x2: x + width,
        y2: divider_y,
        attrs: Attrs::new()
            .with_stroke(&theme.primary_border_color)
            .with_stroke_width(1.0)
            .with_class("divider"),
    });

    // === TEXT ELEMENTS AFTER SHAPES ===

    // Type label (centered, at top of header)
    let type_text = format_requirement_type(&req.req_type);
    children.push(SvgElement::Text {
        x: x + width / 2.0,
        y: y + 8.0 + 12.0, // y + 8 + half line height for baseline
        content: type_text.to_string(),
        attrs: Attrs::new()
            .with_attr("text-anchor", "middle")
            .with_class("requirement-type")
            .with_attr("font-size", &FONT_SIZE.to_string()),
    });

    // Name (bold, centered, below type)
    children.push(SvgElement::Text {
        x: x + width / 2.0,
        y: y + 32.0 + 12.0,
        content: req.name.clone(),
        attrs: Attrs::new()
            .with_attr("text-anchor", "middle")
            .with_class("requirement-name")
            .with_attr("font-size", &FONT_SIZE.to_string())
            .with_attr("font-weight", "bold"),
    });

    // Content area - all left-aligned, starting below divider
    let mut current_y = content_start_y;
    let left_margin = x + BOX_PADDING;

    // ID
    if !req.requirement_id.is_empty() {
        children.push(SvgElement::Text {
            x: left_margin,
            y: current_y + 16.0, // baseline position
            content: format!("ID: {}", req.requirement_id),
            attrs: Attrs::new()
                .with_attr("text-anchor", "start")
                .with_class("requirement-attr")
                .with_attr("font-size", &FONT_SIZE.to_string()),
        });
        current_y += LINE_HEIGHT;
    }

    // Text
    if !req.text.is_empty() {
        children.push(SvgElement::Text {
            x: left_margin,
            y: current_y + 16.0,
            content: format!("Text: {}", req.text),
            attrs: Attrs::new()
                .with_attr("text-anchor", "start")
                .with_class("requirement-attr")
                .with_attr("font-size", &FONT_SIZE.to_string()),
        });
        current_y += LINE_HEIGHT;
    }

    // Risk
    children.push(SvgElement::Text {
        x: left_margin,
        y: current_y + 16.0,
        content: format!("Risk: {}", format_risk(&req.risk)),
        attrs: Attrs::new()
            .with_attr("text-anchor", "start")
            .with_class("requirement-attr")
            .with_attr("font-size", &FONT_SIZE.to_string()),
    });
    current_y += LINE_HEIGHT;

    // Verify method
    children.push(SvgElement::Text {
        x: left_margin,
        y: current_y + 16.0,
        content: format!("Verification: {}", format_verify_method(&req.verify_method)),
        attrs: Attrs::new()
            .with_attr("text-anchor", "start")
            .with_class("requirement-attr")
            .with_attr("font-size", &FONT_SIZE.to_string()),
    });

    SvgElement::Group {
        children,
        attrs: Attrs::new()
            .with_class("requirement-node")
            .with_id(&req.name),
    }
}

/// Render an element box - matches mermaid's layout
/// Mermaid elements have: <<Element>> header, name (bold), then type/docref below divider
fn render_element_box(
    elem: &Element,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    theme: &Theme,
) -> SvgElement {
    // Mermaid element layout:
    // - <<Element>> at y + 8 (centered)
    // - Name at y + 32 (bold, centered)
    // - Divider at y + 56
    // - Content starts at y + 64

    let divider_y = y + 56.0;
    let content_start_y = divider_y + 8.0;
    let left_margin = x + BOX_PADDING;

    let mut children = vec![
        // === SHAPES FIRST ===
        // Main box
        SvgElement::Rect {
            x,
            y,
            width,
            height,
            rx: Some(0.0),
            ry: Some(0.0),
            attrs: Attrs::new()
                .with_fill(&theme.primary_color)
                .with_stroke(&theme.primary_border_color)
                .with_stroke_width(2.0)
                .with_class("element-box"),
        },
        // Header background
        SvgElement::Rect {
            x,
            y,
            width,
            height: divider_y - y,
            rx: Some(0.0),
            ry: Some(0.0),
            attrs: Attrs::new()
                .with_fill(&theme.primary_color)
                .with_stroke(&theme.primary_border_color)
                .with_class("element-header"),
        },
        // Divider line
        SvgElement::Line {
            x1: x,
            y1: divider_y,
            x2: x + width,
            y2: divider_y,
            attrs: Attrs::new()
                .with_stroke(&theme.primary_border_color)
                .with_stroke_width(1.0)
                .with_class("divider"),
        },
        // === TEXT ELEMENTS AFTER SHAPES ===
        // Element label (centered, at top)
        SvgElement::Text {
            x: x + width / 2.0,
            y: y + 8.0 + 12.0,
            content: "<<Element>>".to_string(),
            attrs: Attrs::new()
                .with_attr("text-anchor", "middle")
                .with_class("element-type")
                .with_attr("font-size", &FONT_SIZE.to_string()),
        },
        // Name (bold, centered)
        SvgElement::Text {
            x: x + width / 2.0,
            y: y + 32.0 + 12.0,
            content: elem.name.clone(),
            attrs: Attrs::new()
                .with_attr("text-anchor", "middle")
                .with_class("element-name")
                .with_attr("font-size", &FONT_SIZE.to_string())
                .with_attr("font-weight", "bold"),
        },
    ];

    // Content below divider
    let mut current_y = content_start_y;

    // Type (strip surrounding quotes if present)
    if !elem.element_type.is_empty() {
        let type_text = elem.element_type.trim_matches('"');
        children.push(SvgElement::Text {
            x: left_margin,
            y: current_y + 16.0,
            content: format!("Type: {}", type_text),
            attrs: Attrs::new()
                .with_attr("text-anchor", "start")
                .with_class("element-attr")
                .with_attr("font-size", &FONT_SIZE.to_string()),
        });
        current_y += LINE_HEIGHT;
    }

    // Doc ref
    if !elem.doc_ref.is_empty() {
        children.push(SvgElement::Text {
            x: left_margin,
            y: current_y + 16.0,
            content: format!("Doc Ref: {}", elem.doc_ref),
            attrs: Attrs::new()
                .with_attr("text-anchor", "start")
                .with_class("element-attr")
                .with_attr("font-size", &FONT_SIZE.to_string()),
        });
    }

    SvgElement::Group {
        children,
        attrs: Attrs::new().with_class("element-node").with_id(&elem.name),
    }
}

/// Render a relationship line using bend points from layout
fn render_relationship_from_bend_points(
    bend_points: &[Point],
    label: &str,
    rel_type: &RelationshipType,
) -> SvgElement {
    let mut children = Vec::new();

    if bend_points.is_empty() {
        return SvgElement::Group {
            children,
            attrs: Attrs::new().with_class("relationship"),
        };
    }

    // Build curved path
    let path_d = build_curved_path(bend_points);

    // Determine line styling based on relationship type
    // Mermaid uses solid lines for "contains", dashed for all others
    let is_contains = matches!(rel_type, RelationshipType::Contains);

    let mut path_attrs = Attrs::new()
        .with_class("relationship-line")
        .with_attr("marker-end", "url(#requirement-arrow)");

    if is_contains {
        // Contains relationship: solid line with start marker (circle with +)
        path_attrs = path_attrs.with_attr("marker-start", "url(#requirement-contains-start)");
    } else {
        // All other relationships: dashed line (matches mermaid's stroke-dasharray: 10,7)
        path_attrs = path_attrs.with_attr("stroke-dasharray", "10,7");
    }

    children.push(SvgElement::Path {
        d: path_d,
        attrs: path_attrs,
    });

    // Add label at midpoint
    if !label.is_empty() {
        if let Some(mid) = crate::layout::geometric_midpoint(bend_points) {
            let label_width = (label.len() as f64) * 7.0;

            // Label background
            children.push(SvgElement::Rect {
                x: mid.x - label_width / 2.0 - 4.0,
                y: mid.y - 10.0,
                width: label_width + 8.0,
                height: 20.0,
                rx: Some(3.0),
                ry: Some(3.0),
                attrs: Attrs::new().with_class("relationship-label-bg"),
            });

            children.push(SvgElement::Text {
                x: mid.x,
                y: mid.y + 4.0,
                content: label.to_string(),
                attrs: Attrs::new()
                    .with_attr("text-anchor", "middle")
                    .with_class("relationship-label")
                    .with_attr("font-size", "11"),
            });
        }
    }

    SvgElement::Group {
        children,
        attrs: Attrs::new().with_class("relationship"),
    }
}

/// Render a straight relationship line (fallback when no bend points)
#[allow(clippy::too_many_arguments)]
fn render_relationship_line(
    x1: f64,
    y1: f64,
    w1: f64,
    h1: f64,
    x2: f64,
    y2: f64,
    w2: f64,
    h2: f64,
    label: &str,
    rel_type: &RelationshipType,
) -> SvgElement {
    let mut children = Vec::new();

    // Calculate connection points
    let center1_x = x1 + w1 / 2.0;
    let center1_y = y1 + h1 / 2.0;
    let center2_x = x2 + w2 / 2.0;
    let center2_y = y2 + h2 / 2.0;

    let dx = center2_x - center1_x;
    let dy = center2_y - center1_y;

    // Determine attachment points
    let (start_x, start_y) = if dy.abs() > dx.abs() {
        if dy > 0.0 {
            (center1_x, y1 + h1)
        } else {
            (center1_x, y1)
        }
    } else if dx > 0.0 {
        (x1 + w1, center1_y)
    } else {
        (x1, center1_y)
    };

    let (end_x, end_y) = if dy.abs() > dx.abs() {
        if dy > 0.0 {
            (center2_x, y2)
        } else {
            (center2_x, y2 + h2)
        }
    } else if dx > 0.0 {
        (x2, center2_y)
    } else {
        (x2 + w2, center2_y)
    };

    // Path with curve
    let mid_x = (start_x + end_x) / 2.0;
    let mid_y = (start_y + end_y) / 2.0;

    let path_d = if dy.abs() > dx.abs() {
        format!(
            "M{},{} C{},{} {},{} {},{}",
            start_x, start_y, start_x, mid_y, end_x, mid_y, end_x, end_y
        )
    } else {
        format!(
            "M{},{} C{},{} {},{} {},{}",
            start_x, start_y, mid_x, start_y, mid_x, end_y, end_x, end_y
        )
    };

    // Determine line styling based on relationship type
    // Mermaid uses solid lines for "contains", dashed for all others
    let is_contains = matches!(rel_type, RelationshipType::Contains);

    let mut path_attrs = Attrs::new()
        .with_class("relationship-line")
        .with_attr("marker-end", "url(#requirement-arrow)");

    if is_contains {
        // Contains relationship: solid line with start marker (circle with +)
        path_attrs = path_attrs.with_attr("marker-start", "url(#requirement-contains-start)");
    } else {
        // All other relationships: dashed line (matches mermaid's stroke-dasharray: 10,7)
        path_attrs = path_attrs.with_attr("stroke-dasharray", "10,7");
    }

    children.push(SvgElement::Path {
        d: path_d,
        attrs: path_attrs,
    });

    // Add label at midpoint
    if !label.is_empty() {
        let label_x = mid_x;
        let label_y = mid_y;
        let label_width = (label.len() as f64) * 7.0;

        // Label background
        children.push(SvgElement::Rect {
            x: label_x - label_width / 2.0 - 4.0,
            y: label_y - 10.0,
            width: label_width + 8.0,
            height: 20.0,
            rx: Some(3.0),
            ry: Some(3.0),
            attrs: Attrs::new().with_class("relationship-label-bg"),
        });

        children.push(SvgElement::Text {
            x: label_x,
            y: label_y + 4.0,
            content: label.to_string(),
            attrs: Attrs::new()
                .with_attr("text-anchor", "middle")
                .with_class("relationship-label")
                .with_attr("font-size", "11"),
        });
    }

    SvgElement::Group {
        children,
        attrs: Attrs::new().with_class("relationship"),
    }
}

/// Generate CSS for requirement diagrams
/// Colors match mermaid.js default theme for requirement diagrams
/// Reference: mermaid's styles.js for requirementDiagram
fn generate_requirement_css(theme: &Theme) -> String {
    // Mermaid uses specific colors for requirement diagrams:
    // - .reqTitle, .reqLabel { fill: #131300 } (dark label color)
    // - .relationshipLabel { fill: black }
    // - .relationshipLine { stroke-width: 1 }
    // - .error-icon, .error-text { fill: #552222 }
    let label_color = "#131300"; // mermaid's reqTitle/reqLabel fill
    format!(
        r#"
.requirement-box {{
  fill: {primary_color};
  stroke: {border_color};
}}

.requirement-header {{
  fill: {primary_color};
  stroke: {border_color};
}}

.requirement-type {{
  fill: {label_color};
  font-weight: bold;
}}

.requirement-name {{
  fill: {label_color};
}}

.requirement-attr {{
  fill: {text_color};
}}

.element-box {{
  fill: {primary_color};
  stroke: {border_color};
}}

.element-header {{
  fill: {primary_color};
  stroke: {border_color};
}}

.element-type {{
  fill: {label_color};
  font-weight: bold;
}}

.element-name {{
  fill: {label_color};
}}

.element-attr {{
  fill: {text_color};
}}

.divider {{
  stroke: {border_color};
}}

.relationship-line {{
  fill: none;
  stroke: {line_color};
  stroke-width: 1;
}}

.relationship-label {{
  fill: black;
}}

.relationship-label-bg {{
  fill: rgba(232, 232, 232, 0.8);
  stroke: none;
}}

.marker {{
  fill: {line_color};
  stroke: none;
}}

.error-icon {{
  fill: #552222;
}}

.error-text {{
  fill: #552222;
  stroke: #552222;
}}

.label {{
  fill: #333;
}}
"#,
        primary_color = theme.primary_color,
        border_color = theme.primary_border_color,
        text_color = theme.primary_text_color,
        line_color = theme.line_color,
        label_color = label_color,
    )
}

/// Generate SVG marker definitions for requirement diagram arrows
fn generate_requirement_markers() -> Vec<SvgElement> {
    vec![
        // Standard arrow marker (end marker for all relationships)
        SvgElement::Marker {
            id: "requirement-arrow".to_string(),
            view_box: "0 0 10 10".to_string(),
            ref_x: 9.0,
            ref_y: 5.0,
            marker_width: 6.0,
            marker_height: 6.0,
            orient: "auto".to_string(),
            marker_units: None,
            children: vec![SvgElement::Path {
                d: "M0,0 L10,5 L0,10 z".to_string(),
                attrs: Attrs::new().with_class("marker"),
            }],
        },
        // Contains start marker - circle with + symbol (matches mermaid.js)
        // This is placed at the source end of "contains" relationships
        SvgElement::Marker {
            id: "requirement-contains-start".to_string(),
            view_box: "0 0 20 20".to_string(),
            ref_x: 0.0,
            ref_y: 10.0,
            marker_width: 20.0,
            marker_height: 20.0,
            orient: "auto".to_string(),
            marker_units: None,
            children: vec![SvgElement::Group {
                children: vec![
                    // Circle outline (unfilled)
                    SvgElement::Circle {
                        cx: 10.0,
                        cy: 10.0,
                        r: 9.0,
                        attrs: Attrs::new()
                            .with_fill("none")
                            .with_stroke("#333333")
                            .with_stroke_width(1.0),
                    },
                    // Horizontal line of the +
                    SvgElement::Line {
                        x1: 1.0,
                        y1: 10.0,
                        x2: 19.0,
                        y2: 10.0,
                        attrs: Attrs::new().with_stroke("#333333").with_stroke_width(1.0),
                    },
                    // Vertical line of the +
                    SvgElement::Line {
                        x1: 10.0,
                        y1: 1.0,
                        x2: 10.0,
                        y2: 19.0,
                        attrs: Attrs::new().with_stroke("#333333").with_stroke_width(1.0),
                    },
                ],
                attrs: Attrs::new(),
            }],
        },
        // Note: mermaid reference only defines 2 markers (arrow + contains-start)
        // The diamond "contains" marker is not used - removed for parity
    ]
}
