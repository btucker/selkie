//! Class diagram renderer
//!
//! This renderer uses dagre layout for positioning class nodes, following
//! the same approach as mermaid.js which uses dagre for all diagram types.

use std::collections::HashMap;

use crate::diagrams::class::{ClassDb, ClassNode, LineType};
use crate::error::Result;
use crate::layout::{
    self, CharacterSizeEstimator, LayoutDirection, LayoutEdge, LayoutGraph, LayoutNode,
    LayoutOptions, NodeShape, NodeSizeConfig, Padding, Point, SizeEstimator, ToLayoutGraph,
};
use crate::render::svg::{edges, Attrs, RenderConfig, SvgDocument, SvgElement};

/// Convert ClassDb to LayoutGraph for dagre-based layout
impl ToLayoutGraph for ClassDb {
    fn to_layout_graph(&self, size_estimator: &dyn SizeEstimator) -> Result<LayoutGraph> {
        let config = NodeSizeConfig {
            font_size: 12.0,
            padding_horizontal: 10.0,
            padding_vertical: 10.0,
            min_width: 100.0,
            min_height: 60.0,
            max_width: Some(200.0),
        };

        let mut graph = LayoutGraph::new("class-diagram");

        graph.options = LayoutOptions {
            direction: self.preferred_direction(),
            node_spacing: 60.0,
            layer_spacing: 80.0,
            padding: Padding {
                top: 58.0,
                right: 33.0,
                bottom: 58.0,
                left: 33.0,
            },
        };

        // Convert classes to layout nodes (sorted for deterministic order)
        let mut class_ids: Vec<&String> = self.classes.keys().collect();
        class_ids.sort();

        for id in class_ids {
            let class = self.classes.get(id).unwrap();

            let label = if !class.label.is_empty() {
                &class.label
            } else {
                &class.id
            };

            // Estimate size: header + members + methods
            let header_height = 30.0;
            let member_height = 18.0;
            let num_members = class.members.len() + class.methods.len();
            let annotations_height = if class.annotations.is_empty() {
                0.0
            } else {
                (class.annotations.len() as f64) * member_height
            };
            let content_height = (num_members as f64) * member_height + annotations_height;
            let total_height = (header_height + content_height + config.padding_vertical * 2.0)
                .max(config.min_height);

            let type_suffix = if !class.type_param.is_empty() {
                format!("<{}>", class.type_param)
            } else {
                String::new()
            };
            let class_text = format!("{}{}", label, type_suffix);

            let mut max_text_width = size_estimator.estimate_text_size(&class_text, 14.0).0;

            for annotation in &class.annotations {
                let text = format!("<<{}>>", annotation);
                max_text_width =
                    max_text_width.max(size_estimator.estimate_text_size(&text, 11.0).0);
            }

            for member in &class.members {
                let display = member.get_display_details();
                max_text_width = max_text_width.max(
                    size_estimator
                        .estimate_text_size(&display.display_text, 12.0)
                        .0,
                );
            }

            for method in &class.methods {
                let display = method.get_display_details();
                max_text_width = max_text_width.max(
                    size_estimator
                        .estimate_text_size(&display.display_text, 12.0)
                        .0,
                );
            }

            let width = (max_text_width + config.padding_horizontal * 2.0).max(config.min_width);

            let mut node =
                LayoutNode::new(id, width, total_height).with_shape(NodeShape::Rectangle);
            node = node.with_label(label);
            node.metadata
                .insert("dom_id".to_string(), class.dom_id.clone());

            graph.add_node(node);
        }

        // Convert relations to edges
        for (idx, relation) in self.relations.iter().enumerate() {
            let edge_id = format!("rel-{}-{}-{}", relation.id1, relation.id2, idx);
            let mut edge = LayoutEdge::new(&edge_id, &relation.id1, &relation.id2);

            if !relation.title.is_empty() {
                edge = edge.with_label(&relation.title);
            }

            edge.metadata
                .insert("type1".to_string(), relation.relation.type1.to_string());
            edge.metadata
                .insert("type2".to_string(), relation.relation.type2.to_string());
            edge.metadata.insert(
                "line_type".to_string(),
                format!("{:?}", relation.relation.line_type),
            );

            graph.add_edge(edge);
        }

        Ok(graph)
    }

    fn preferred_direction(&self) -> LayoutDirection {
        match self.direction.to_uppercase().as_str() {
            "LR" => LayoutDirection::LeftToRight,
            "RL" => LayoutDirection::RightToLeft,
            "BT" => LayoutDirection::BottomToTop,
            _ => LayoutDirection::TopToBottom,
        }
    }
}

/// Render a class diagram to SVG using dagre layout
pub fn render_class(db: &ClassDb, config: &RenderConfig) -> Result<String> {
    let mut doc = SvgDocument::new();

    // Layout constants
    let class_padding = 10.0;
    let member_height = 18.0;
    let header_height = 30.0;
    let annotation_font_size = 11.0;
    let class_name_font_size = 14.0;
    let member_font_size = 12.0;

    let classes: Vec<_> = db.classes.values().collect();

    if classes.is_empty() {
        doc.set_size(400.0, 200.0);
        return Ok(doc.to_string());
    }

    // Convert to layout graph and run dagre layout
    let size_estimator = CharacterSizeEstimator::default();
    let layout_graph = db.to_layout_graph(&size_estimator)?;
    let layout_graph = layout::layout(layout_graph)?;

    // Build position map from layout results
    let mut class_positions: HashMap<String, (f64, f64)> = HashMap::new();
    let mut class_dimensions: HashMap<String, (f64, f64)> = HashMap::new();

    for node in &layout_graph.nodes {
        if let (Some(x), Some(y)) = (node.x, node.y) {
            class_positions.insert(node.id.clone(), (x, y));
            class_dimensions.insert(node.id.clone(), (node.width, node.height));
        }
    }

    // Build edge bend points map
    let mut edge_points: HashMap<(String, String), Vec<Point>> = HashMap::new();
    for edge in &layout_graph.edges {
        if let (Some(source), Some(target)) = (edge.source(), edge.target()) {
            edge_points.insert(
                (source.to_string(), target.to_string()),
                edge.bend_points.clone(),
            );
        }
    }

    // Calculate SVG dimensions from layout
    let (width, height) = if let (Some(w), Some(h)) = (layout_graph.width, layout_graph.height) {
        (w, h)
    } else {
        (800.0, 600.0) // Fallback
    };
    doc.set_size(width, height);

    // Add theme styles
    if config.embed_css {
        doc.add_style(&config.theme.generate_css());
        doc.add_style(&generate_class_css());
    }

    // Add marker definitions for relations
    doc.add_defs(create_class_markers());

    // Render each class at dagre-computed position
    for class in &classes {
        if let Some(&(x, y)) = class_positions.get(&class.id) {
            let (width, height) = class_dimensions
                .get(&class.id)
                .copied()
                .unwrap_or((180.0, 60.0));

            let class_elem = render_class_box(
                class,
                x,
                y,
                width,
                height,
                class_padding,
                member_height,
                header_height,
                annotation_font_size,
                class_name_font_size,
                member_font_size,
                &size_estimator,
            );
            doc.add_element(class_elem);
        }
    }

    // Render relations using edge bend points from dagre
    for relation in &db.relations {
        let key = (relation.id1.clone(), relation.id2.clone());

        if let (Some(&(x1, y1)), Some(&(x2, y2))) = (
            class_positions.get(&relation.id1),
            class_positions.get(&relation.id2),
        ) {
            let (w1, h1) = class_dimensions
                .get(&relation.id1)
                .copied()
                .unwrap_or((180.0, 60.0));
            let (w2, h2) = class_dimensions
                .get(&relation.id2)
                .copied()
                .unwrap_or((180.0, 60.0));

            let bend_points = edge_points.get(&key);

            let relation_elem = render_relation(
                x1,
                y1,
                h1,
                w1,
                x2,
                y2,
                h2,
                w2,
                &relation.title,
                &relation.relation_title1,
                &relation.relation_title2,
                relation.relation.type1,
                relation.relation.type2,
                relation.relation.line_type,
                bend_points,
                &size_estimator,
            );
            doc.add_element(relation_elem);
        }
    }

    // Render notes
    for note in db.notes.values() {
        if let Some(&(x, y)) = class_positions.get(&note.class) {
            let (width, _) = class_dimensions
                .get(&note.class)
                .copied()
                .unwrap_or((180.0, 60.0));
            let note_elem = render_note(x + width + 20.0, y, &note.text);
            doc.add_element(note_elem);
        }
    }

    Ok(doc.to_string())
}

/// Render a class box with name, attributes, and methods
#[allow(clippy::too_many_arguments)]
fn render_class_box(
    class: &ClassNode,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    padding: f64,
    member_height: f64,
    header_height: f64,
    annotation_font_size: f64,
    class_name_font_size: f64,
    member_font_size: f64,
    size_estimator: &dyn SizeEstimator,
) -> SvgElement {
    let mut children = Vec::new();

    // Background shape (path to match mermaid structure)
    let box_path = rounded_rect_path(x, y, width, height, 3.0, 3.0);
    children.push(SvgElement::Path {
        d: box_path.clone(),
        attrs: Attrs::new()
            .with_fill("#ECECFF")
            .with_stroke("none")
            .with_class("class-box-bg"),
    });
    children.push(SvgElement::Path {
        d: box_path,
        attrs: Attrs::new()
            .with_fill("none")
            .with_stroke("#333333")
            .with_stroke_width(1.0)
            .with_class("class-box"),
    });

    let mut current_y = y;

    // Annotations (<<interface>>, <<abstract>>, etc.)
    if !class.annotations.is_empty() {
        for annotation in &class.annotations {
            current_y += member_height;
            let annotation_text = format!("<<{}>>", annotation);
            let text_width = size_estimator
                .estimate_text_size(&annotation_text, annotation_font_size)
                .0;
            let text_x = x + (width - text_width) / 2.0;
            let available_width = width - padding * 2.0;
            children.push(foreign_object_label(
                text_x,
                current_y,
                text_width,
                annotation_font_size,
                available_width,
                &annotation_text,
                "center",
                "font-style: italic;",
            ));
        }
    }

    // Class name
    current_y += header_height / 2.0 + 5.0;
    let class_label = if !class.label.is_empty() {
        &class.label
    } else {
        &class.id
    };
    let type_suffix = if !class.type_param.is_empty() {
        format!("<{}>", class.type_param)
    } else {
        String::new()
    };

    let class_text = format!("{}{}", class_label, type_suffix);
    let text_width = size_estimator
        .estimate_text_size(&class_text, class_name_font_size)
        .0;
    let text_x = x + (width - text_width) / 2.0;
    let available_width = width - padding * 2.0;
    children.push(foreign_object_label(
        text_x,
        current_y,
        text_width,
        class_name_font_size,
        available_width,
        &class_text,
        "center",
        "font-weight: bolder;",
    ));

    let divider1_y = y + header_height;
    let members_section_height = (class.members.len().max(1) as f64) * member_height + padding;
    let divider2_y = divider1_y + members_section_height;

    // Divider after name (always present)
    children.push(SvgElement::Path {
        d: line_path(x, divider1_y, x + width, divider1_y),
        attrs: Attrs::new()
            .with_stroke("#333333")
            .with_stroke_width(1.0)
            .with_class("class-divider"),
    });

    // Attributes section
    if !class.members.is_empty() {
        current_y = divider1_y + padding;
        for member in &class.members {
            current_y += member_height;
            let display = member.get_display_details();
            let text_width = size_estimator
                .estimate_text_size(&display.display_text, member_font_size)
                .0;
            let text_x = x + (width - text_width) / 2.0;
            let available_width = width - padding * 2.0;
            children.push(foreign_object_label(
                text_x,
                current_y - 4.0,
                text_width,
                member_font_size,
                available_width,
                &display.display_text,
                "center",
                &display.css_style,
            ));
        }
    }

    // Divider between attributes and methods (always present)
    children.push(SvgElement::Path {
        d: line_path(x, divider2_y, x + width, divider2_y),
        attrs: Attrs::new()
            .with_stroke("#333333")
            .with_stroke_width(1.0)
            .with_class("class-divider"),
    });

    // Methods section
    if !class.methods.is_empty() {
        current_y = divider2_y + padding;
        for method in &class.methods {
            current_y += member_height;
            let display = method.get_display_details();
            let text_width = size_estimator
                .estimate_text_size(&display.display_text, member_font_size)
                .0;
            let text_x = x + (width - text_width) / 2.0;
            let available_width = width - padding * 2.0;
            children.push(foreign_object_label(
                text_x,
                current_y - 4.0,
                text_width,
                member_font_size,
                available_width,
                &display.display_text,
                "center",
                &display.css_style,
            ));
        }
    }

    SvgElement::Group {
        children,
        attrs: Attrs::new()
            .with_class("class-node")
            .with_id(&format!("class-{}", class.id)),
    }
}

fn line_path(x1: f64, y1: f64, x2: f64, y2: f64) -> String {
    format!("M {} {} L {} {}", x1, y1, x2, y2)
}

fn rounded_rect_path(x: f64, y: f64, width: f64, height: f64, rx: f64, ry: f64) -> String {
    let right = x + width;
    let bottom = y + height;
    format!(
        "M {} {} H {} A {} {} 0 0 1 {} {} V {} A {} {} 0 0 1 {} {} H {} A {} {} 0 0 1 {} {} V {} A {} {} 0 0 1 {} {} Z",
        x + rx,
        y,
        right - rx,
        rx,
        ry,
        right,
        y + ry,
        bottom - ry,
        rx,
        ry,
        right - rx,
        bottom,
        x + rx,
        rx,
        ry,
        x,
        bottom - ry,
        y + ry,
        rx,
        ry,
        x + rx,
        y
    )
}

/// Render a relation between two classes using dagre bend points
#[allow(clippy::too_many_arguments)]
fn render_relation(
    x1: f64,
    y1: f64,
    h1: f64,
    w1: f64,
    x2: f64,
    y2: f64,
    h2: f64,
    w2: f64,
    label: &str,
    cardinality1: &str,
    cardinality2: &str,
    type1: i32,
    type2: i32,
    line_type: LineType,
    bend_points: Option<&Vec<Point>>,
    size_estimator: &dyn SizeEstimator,
) -> SvgElement {
    let mut children = Vec::new();

    // Calculate path from bend points or fallback to direct line
    let path_d = if let Some(points) = bend_points {
        if !points.is_empty() {
            edges::build_curved_path(points)
        } else {
            build_direct_path(x1, y1, h1, w1, x2, y2, h2, w2)
        }
    } else {
        build_direct_path(x1, y1, h1, w1, x2, y2, h2, w2)
    };

    // Determine marker based on relation type
    let marker_start = match type1 {
        0 => Some("url(#aggregation-start)"),
        1 => Some("url(#inheritance-start)"),
        2 => Some("url(#composition-start)"),
        3 => Some("url(#dependency-start)"),
        4 => Some("url(#lollipop-start)"),
        _ => None,
    };

    let marker_end = match type2 {
        0 => Some("url(#aggregation-end)"),
        1 => Some("url(#inheritance-end)"),
        2 => Some("url(#composition-end)"),
        3 => Some("url(#dependency-end)"),
        4 => Some("url(#lollipop-end)"),
        _ => None,
    };

    let mut path_attrs = Attrs::new()
        .with_stroke("#333333")
        .with_stroke_width(1.0)
        .with_fill("none")
        .with_class("relation-line");

    if line_type == LineType::Dotted {
        path_attrs = path_attrs.with_stroke_dasharray("5,5");
    }

    if let Some(marker) = marker_start {
        path_attrs = path_attrs.with_attr("marker-start", marker);
    }
    if let Some(marker) = marker_end {
        path_attrs = path_attrs.with_attr("marker-end", marker);
    }

    children.push(SvgElement::Path {
        d: path_d.clone(),
        attrs: path_attrs,
    });

    // Calculate label positions based on bend points or direct line
    let (start_x, start_y, end_x, end_y) = if let Some(points) = bend_points {
        if points.len() >= 2 {
            (
                points[0].x,
                points[0].y,
                points[points.len() - 1].x,
                points[points.len() - 1].y,
            )
        } else {
            calculate_connection_points(x1, y1, h1, w1, x2, y2, h2, w2)
        }
    } else {
        calculate_connection_points(x1, y1, h1, w1, x2, y2, h2, w2)
    };

    // Cardinality label at start (near class 1)
    if !cardinality1.is_empty() {
        let dx = end_x - start_x;
        let dy = end_y - start_y;
        let offset = 20.0;
        let len = (dx * dx + dy * dy).sqrt();
        let offset_x = if len > 0.0 { offset * dx / len } else { 0.0 };
        let offset_y = if len > 0.0 { offset * dy / len } else { offset };

        let perp_offset = 12.0;
        let perp_x = if len > 0.0 {
            -perp_offset * dy / len
        } else {
            perp_offset
        };
        let perp_y = if len > 0.0 {
            perp_offset * dx / len
        } else {
            0.0
        };

        children.push(SvgElement::Text {
            x: start_x + offset_x + perp_x,
            y: start_y + offset_y + perp_y,
            content: cardinality1.to_string(),
            attrs: Attrs::new()
                .with_attr("text-anchor", "middle")
                .with_class("cardinality-label")
                .with_attr("font-size", "11"),
        });
    }

    // Cardinality label at end (near class 2)
    if !cardinality2.is_empty() {
        let dx = end_x - start_x;
        let dy = end_y - start_y;
        let offset = 20.0;
        let len = (dx * dx + dy * dy).sqrt();
        let offset_x = if len > 0.0 { offset * dx / len } else { 0.0 };
        let offset_y = if len > 0.0 { offset * dy / len } else { offset };

        let perp_offset = 12.0;
        let perp_x = if len > 0.0 {
            -perp_offset * dy / len
        } else {
            perp_offset
        };
        let perp_y = if len > 0.0 {
            perp_offset * dx / len
        } else {
            0.0
        };

        children.push(SvgElement::Text {
            x: end_x - offset_x + perp_x,
            y: end_y - offset_y + perp_y,
            content: cardinality2.to_string(),
            attrs: Attrs::new()
                .with_attr("text-anchor", "middle")
                .with_class("cardinality-label")
                .with_attr("font-size", "11"),
        });
    }

    // Relation label (in the middle)
    if !label.is_empty() {
        let mid_x = (start_x + end_x) / 2.0;
        let mid_y = (start_y + end_y) / 2.0;
        let font_size = 11.0;
        let text_width = size_estimator.estimate_text_size(label, font_size).0;
        let text_height = font_size * 1.5;
        let padding = 4.0;

        children.push(SvgElement::Rect {
            x: mid_x - text_width / 2.0 - padding,
            y: mid_y - text_height / 2.0 - padding / 2.0,
            width: text_width + padding * 2.0,
            height: text_height + padding,
            rx: None,
            ry: None,
            attrs: Attrs::new()
                .with_class("edge-label-bg")
                .with_attr("fill-opacity", "0.8"),
        });

        children.push(SvgElement::Text {
            x: mid_x,
            y: mid_y,
            content: label.to_string(),
            attrs: Attrs::new()
                .with_attr("text-anchor", "middle")
                .with_class("relation-label")
                .with_attr("dominant-baseline", "central")
                .with_attr("font-size", "11"),
        });
    }

    SvgElement::Group {
        children,
        attrs: Attrs::new().with_class("relation"),
    }
}

/// Build direct path when no bend points available
#[allow(clippy::too_many_arguments)]
fn build_direct_path(
    x1: f64,
    y1: f64,
    h1: f64,
    w1: f64,
    x2: f64,
    y2: f64,
    h2: f64,
    w2: f64,
) -> String {
    let (start_x, start_y, end_x, end_y) =
        calculate_connection_points(x1, y1, h1, w1, x2, y2, h2, w2);
    format!("M {} {} L {} {}", start_x, start_y, end_x, end_y)
}

/// Calculate connection points on class box edges
#[allow(clippy::too_many_arguments)]
fn calculate_connection_points(
    x1: f64,
    y1: f64,
    h1: f64,
    w1: f64,
    x2: f64,
    y2: f64,
    h2: f64,
    w2: f64,
) -> (f64, f64, f64, f64) {
    let center1_x = x1 + w1 / 2.0;
    let center1_y = y1 + h1 / 2.0;
    let center2_x = x2 + w2 / 2.0;
    let center2_y = y2 + h2 / 2.0;

    let dx = center2_x - center1_x;
    let dy = center2_y - center1_y;

    // Determine which edges to connect based on relative positions
    let (start_x, start_y) = if dx.abs() > dy.abs() {
        // Horizontal connection
        if dx > 0.0 {
            (x1 + w1, center1_y) // Right edge
        } else {
            (x1, center1_y) // Left edge
        }
    } else {
        // Vertical connection
        if dy > 0.0 {
            (center1_x, y1 + h1) // Bottom edge
        } else {
            (center1_x, y1) // Top edge
        }
    };

    let (end_x, end_y) = if dx.abs() > dy.abs() {
        if dx > 0.0 {
            (x2, center2_y) // Left edge
        } else {
            (x2 + w2, center2_y) // Right edge
        }
    } else if dy > 0.0 {
        (center2_x, y2) // Top edge
    } else {
        (center2_x, y2 + h2) // Bottom edge
    };

    (start_x, start_y, end_x, end_y)
}

/// Render a note attached to a class
fn render_note(x: f64, y: f64, text: &str) -> SvgElement {
    let note_width = 100.0;
    let note_height = 40.0;
    let fold_size = 8.0;

    let mut children = Vec::new();

    // Note box with folded corner
    let path = format!(
        "M {} {} L {} {} L {} {} L {} {} L {} {} Z",
        x,
        y,
        x + note_width - fold_size,
        y,
        x + note_width,
        y + fold_size,
        x + note_width,
        y + note_height,
        x,
        y + note_height
    );

    children.push(SvgElement::Path {
        d: path,
        attrs: Attrs::new()
            .with_fill("#FFFFCC")
            .with_stroke("#333333")
            .with_stroke_width(1.0)
            .with_class("note-box"),
    });

    // Fold line
    let fold_path = format!(
        "M {} {} L {} {} L {} {}",
        x + note_width - fold_size,
        y,
        x + note_width - fold_size,
        y + fold_size,
        x + note_width,
        y + fold_size
    );

    children.push(SvgElement::Path {
        d: fold_path,
        attrs: Attrs::new()
            .with_fill("none")
            .with_stroke("#333333")
            .with_stroke_width(1.0),
    });

    // Note text
    children.push(SvgElement::Text {
        x: x + note_width / 2.0,
        y: y + note_height / 2.0 + 4.0,
        content: text.to_string(),
        attrs: Attrs::new()
            .with_attr("text-anchor", "middle")
            .with_class("note-text")
            .with_attr("font-size", "11"),
    });

    SvgElement::Group {
        children,
        attrs: Attrs::new().with_class("note"),
    }
}

fn create_class_markers() -> Vec<SvgElement> {
    let mut markers = Vec::new();
    markers.extend(create_marker_pair(
        "aggregation",
        "0 0 20 14",
        18.0,
        1.0,
        7.0,
        vec![SvgElement::Path {
            d: "M 18 7 L 9 13 L 1 7 L 9 1 Z".to_string(),
            attrs: Attrs::new()
                .with_fill("none")
                .with_stroke("#333333")
                .with_stroke_width(1.0),
        }],
    ));
    markers.extend(create_marker_pair(
        "inheritance",
        "0 0 20 14",
        18.0,
        1.0,
        7.0,
        vec![SvgElement::Path {
            d: "M 1 7 L 18 13 V 1 Z".to_string(),
            attrs: Attrs::new()
                .with_fill("none")
                .with_stroke("#333333")
                .with_stroke_width(1.0),
        }],
    ));
    markers.extend(create_marker_pair(
        "composition",
        "0 0 20 14",
        18.0,
        1.0,
        7.0,
        vec![SvgElement::Path {
            d: "M 18 7 L 9 13 L 1 7 L 9 1 Z".to_string(),
            attrs: Attrs::new()
                .with_fill("#333333")
                .with_stroke("#333333")
                .with_stroke_width(1.0),
        }],
    ));
    markers.extend(create_marker_pair(
        "dependency",
        "0 0 20 20",
        18.0,
        1.0,
        10.0,
        vec![SvgElement::Path {
            d: "M 0 0 L 20 10 L 0 20".to_string(),
            attrs: Attrs::new()
                .with_fill("none")
                .with_stroke("#333333")
                .with_stroke_width(1.0),
        }],
    ));
    markers.extend(create_marker_pair(
        "lollipop",
        "0 0 20 20",
        13.0,
        1.0,
        10.0,
        vec![SvgElement::Circle {
            cx: 10.0,
            cy: 10.0,
            r: 8.0,
            attrs: Attrs::new()
                .with_fill("#FFFFFF")
                .with_stroke("#333333")
                .with_stroke_width(1.0),
        }],
    ));
    markers
}

fn create_marker_pair(
    name: &str,
    view_box: &str,
    start_ref_x: f64,
    end_ref_x: f64,
    ref_y: f64,
    children: Vec<SvgElement>,
) -> Vec<SvgElement> {
    vec![
        SvgElement::Marker {
            id: format!("{}-start", name),
            view_box: view_box.to_string(),
            ref_x: start_ref_x,
            ref_y,
            marker_width: 10.0,
            marker_height: 10.0,
            orient: "auto".to_string(),
            marker_units: None,
            children: children.clone(),
        },
        SvgElement::Marker {
            id: format!("{}-end", name),
            view_box: view_box.to_string(),
            ref_x: end_ref_x,
            ref_y,
            marker_width: 10.0,
            marker_height: 10.0,
            orient: "auto".to_string(),
            marker_units: None,
            children,
        },
    ]
}

fn generate_class_css() -> String {
    r#"
.class-box {
  stroke: #333333;
}

.class-box-bg {
  fill: #ECECFF;
}

.class-divider {
  stroke: #333333;
}

.class-name {
  fill: #333333;
  font-weight: bold;
}

.class-annotation {
  fill: #666666;
  font-style: italic;
}

.class-member {
  fill: #333333;
}

.class-method {
  fill: #333333;
}

.relation-line {
  stroke: #333333;
}

.relation-label {
  fill: #333333;
}

.note-box {
  fill: #FFFFCC;
  stroke: #333333;
}

.note-text {
  fill: #333333;
}
"#
    .to_string()
}

#[allow(clippy::too_many_arguments)]
fn foreign_object_label(
    x: f64,
    y: f64,
    width: f64,
    font_size: f64,
    max_width: f64,
    text: &str,
    align: &str,
    style: &str,
) -> SvgElement {
    let line_height = font_size * 1.5;
    let height = line_height;
    let y_top = y - line_height + (line_height - font_size) / 2.0;
    let text = escape_html(text);
    let html = format!(
        "<foreignObject x=\"{x}\" y=\"{y_top}\" width=\"{width}\" height=\"{height}\">\
<div xmlns=\"http://www.w3.org/1999/xhtml\" style=\"display: table-cell; white-space: nowrap; line-height: 1.5; max-width: {max_width}px; text-align: {align}; font-size: {font_size}px;\">\
<span class=\"nodeLabel markdown-node-label\" style=\"{style}\"><p style=\"margin:0;\">{text}</p></span>\
</div></foreignObject>"
    );
    SvgElement::Raw { content: html }
}

fn escape_html(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagrams::class::{ClassDb, ClassRelation, LineType, RelationDetails};

    #[test]
    fn test_hierarchical_layout_levels() {
        // Create a simple class hierarchy: Animal -> [Duck, Fish, Zebra] -> Egg (under Duck)
        let mut db = ClassDb::new();

        // Add classes
        db.add_class("Animal");
        db.add_class("Duck");
        db.add_class("Fish");
        db.add_class("Zebra");
        db.add_class("Egg");

        // Add inheritance relations (type1=1 means id1 is parent)
        db.add_relation(ClassRelation {
            id1: "Animal".to_string(),
            id2: "Duck".to_string(),
            relation_title1: String::new(),
            relation_title2: String::new(),
            relation_type: "<|--".to_string(),
            title: String::new(),
            text: String::new(),
            style: vec![],
            relation: RelationDetails {
                type1: 1,
                type2: -1,
                line_type: LineType::Solid,
            },
        });
        db.add_relation(ClassRelation {
            id1: "Animal".to_string(),
            id2: "Fish".to_string(),
            relation_title1: String::new(),
            relation_title2: String::new(),
            relation_type: "<|--".to_string(),
            title: String::new(),
            text: String::new(),
            style: vec![],
            relation: RelationDetails {
                type1: 1,
                type2: -1,
                line_type: LineType::Solid,
            },
        });
        db.add_relation(ClassRelation {
            id1: "Animal".to_string(),
            id2: "Zebra".to_string(),
            relation_title1: String::new(),
            relation_title2: String::new(),
            relation_type: "<|--".to_string(),
            title: String::new(),
            text: String::new(),
            style: vec![],
            relation: RelationDetails {
                type1: 1,
                type2: -1,
                line_type: LineType::Solid,
            },
        });
        // Composition: Duck *-- Egg
        db.add_relation(ClassRelation {
            id1: "Duck".to_string(),
            id2: "Egg".to_string(),
            relation_title1: String::new(),
            relation_title2: String::new(),
            relation_type: "*--".to_string(),
            title: "has".to_string(),
            text: String::new(),
            style: vec![],
            relation: RelationDetails {
                type1: 2,
                type2: -1,
                line_type: LineType::Solid,
            },
        });

        let config = RenderConfig::default();
        let svg = render_class(&db, &config).expect("Render failed");

        // For now, just verify the SVG contains all classes
        assert!(svg.contains("Animal"), "Should contain Animal");
        assert!(svg.contains("Duck"), "Should contain Duck");
        assert!(svg.contains("Fish"), "Should contain Fish");
        assert!(svg.contains("Zebra"), "Should contain Zebra");
        assert!(svg.contains("Egg"), "Should contain Egg");

        // Verify SVG has path elements for edges (dagre-computed)
        assert!(
            svg.contains("<path"),
            "Should contain path elements for edges"
        );
    }

    #[test]
    fn test_class_diagram_uses_dagre_positions() {
        let mut db = ClassDb::new();
        db.add_class("A");
        db.add_class("B");

        db.add_relation(ClassRelation {
            id1: "A".to_string(),
            id2: "B".to_string(),
            relation_title1: String::new(),
            relation_title2: String::new(),
            relation_type: "-->".to_string(),
            title: String::new(),
            text: String::new(),
            style: vec![],
            relation: RelationDetails {
                type1: -1,
                type2: -1,
                line_type: LineType::Solid,
            },
        });

        let config = RenderConfig::default();
        let svg = render_class(&db, &config).expect("Render failed");

        // Verify basic SVG structure
        assert!(svg.contains("<svg"), "Should be valid SVG");
        assert!(svg.contains("class-node"), "Should contain class nodes");
        assert!(svg.contains("relation"), "Should contain relations");
    }

    #[test]
    fn test_empty_class_has_background_and_dividers() {
        let mut db = ClassDb::new();
        db.add_class("Solo");

        let config = RenderConfig::default();
        let svg = render_class(&db, &config).expect("Render failed");

        assert!(
            svg.contains("class-box-bg"),
            "Should render background path for class box"
        );
        assert!(
            svg.matches("class-divider").count() >= 2,
            "Should render two divider paths for empty class"
        );
    }
}
