//! Entity Relationship diagram renderer

use std::collections::HashMap;

use crate::diagrams::er::{Cardinality, Direction, Entity, ErDb, Identification};
use crate::error::Result;
use crate::layout::{
    layout, CharacterSizeEstimator, LayoutDirection, LayoutEdge, LayoutGraph, LayoutNode,
    LayoutOptions, NodeShape, Padding, SizeEstimator, ToLayoutGraph,
};
use crate::render::svg::{Attrs, RenderConfig, SvgDocument, SvgElement, Theme};

/// Entity dimensions calculated from content
#[derive(Debug, Clone)]
struct EntityDimensions {
    width: f64,
    height: f64,
    /// Column widths: [type_col, name_col, keys_col]
    col_widths: [f64; 3],
}

/// Calculate entity dimensions based on content
fn calculate_entity_dimensions(
    entity: &Entity,
    display_name: &str,
    header_height: f64,
    row_height: f64,
    font_size: f64,
    padding: f64,
) -> EntityDimensions {
    // Character width estimation (matching trebuchet ms at 0.6 ratio)
    let char_width = font_size * 0.6;
    let header_char_width = 14.0 * 0.6; // Header uses font-size 14

    // Calculate column widths from content
    let mut max_type_width = 0.0_f64;
    let mut max_name_width = 0.0_f64;
    let mut max_keys_width = 0.0_f64;

    for attr in &entity.attributes {
        let type_width = attr.attr_type.len() as f64 * char_width;
        let name_width = attr.name.len() as f64 * char_width;
        let keys_str: String = attr
            .keys
            .iter()
            .map(|k| k.as_str())
            .collect::<Vec<_>>()
            .join(",");
        let keys_width = keys_str.len() as f64 * char_width;

        max_type_width = max_type_width.max(type_width);
        max_name_width = max_name_width.max(name_width);
        max_keys_width = max_keys_width.max(keys_width);
    }

    // Add padding to each column (12px left padding in each)
    let col_padding = 12.0;
    let col_right_padding = 8.0;
    let type_col_width = max_type_width + col_padding + col_right_padding;
    let name_col_width = max_name_width + col_padding + col_right_padding;
    let keys_col_width = if max_keys_width > 0.0 {
        max_keys_width + col_padding + col_right_padding
    } else {
        col_padding * 2.0 // Minimum width for empty keys column
    };

    // Calculate header width requirement
    let header_width = display_name.len() as f64 * header_char_width + padding * 4.0;

    // Total entity width is max of header and sum of columns
    let content_width = type_col_width + name_col_width + keys_col_width;
    let total_width = content_width.max(header_width);

    // Minimum width matches mermaid baseline
    let min_width = 120.0;
    let width = total_width.max(min_width);

    // Height based on rows
    let height = if entity.attributes.is_empty() {
        header_height + padding * 2.0
    } else {
        header_height + (entity.attributes.len() as f64) * row_height + padding * 2.0
    };

    EntityDimensions {
        width,
        height,
        col_widths: [type_col_width, name_col_width, keys_col_width],
    }
}

/// Implement ToLayoutGraph for ErDb to enable proper DAG layout
impl ToLayoutGraph for ErDb {
    fn to_layout_graph(&self, _size_estimator: &dyn SizeEstimator) -> Result<LayoutGraph> {
        let mut graph = LayoutGraph::new("er");

        // Set layout options from diagram direction
        graph.options = LayoutOptions {
            direction: self.preferred_direction(),
            node_spacing: 60.0,
            layer_spacing: 80.0,
            padding: Padding::uniform(30.0),
            ..Default::default()
        };

        // Layout constants (matching mermaid.js)
        let entity_header_height = 42.75;
        let attr_row_height = 42.75;
        let attr_font_size = 12.0;
        let padding = 8.0;

        // Convert entities to layout nodes
        let entities = self.get_entities();

        // Sort entities by name for deterministic ordering
        let mut sorted_entities: Vec<(&String, &Entity)> = entities.iter().collect();
        sorted_entities.sort_by(|a, b| a.0.cmp(b.0));

        for (name, entity) in &sorted_entities {
            // Calculate dynamic entity dimensions
            let display_name = if !entity.alias.is_empty() {
                &entity.alias
            } else {
                &entity.label
            };
            let dims = calculate_entity_dimensions(
                entity,
                display_name,
                entity_header_height,
                attr_row_height,
                attr_font_size,
                padding,
            );

            let node = LayoutNode::new(&entity.id, dims.width, dims.height)
                .with_shape(NodeShape::Rectangle)
                .with_label(name.as_str());

            graph.add_node(node);
        }

        // Convert relationships to edges
        // In ER diagrams, relationships indicate dependencies
        // entity_a ||--o{ entity_b means entity_a is the "parent" (one) side
        // So the edge goes from entity_a to entity_b (parent to child)
        for (i, relationship) in self.get_relationships().iter().enumerate() {
            let edge_id = format!("relationship-{}", i);

            // Create edge from source (entity_a) to target (entity_b)
            let mut edge =
                LayoutEdge::new(&edge_id, &relationship.entity_a, &relationship.entity_b);

            if !relationship.role_a.is_empty() {
                edge = edge.with_label(&relationship.role_a);
            }

            graph.add_edge(edge);
        }

        Ok(graph)
    }

    fn preferred_direction(&self) -> LayoutDirection {
        match self.get_direction() {
            Direction::TopToBottom => LayoutDirection::TopToBottom,
            Direction::BottomToTop => LayoutDirection::BottomToTop,
            Direction::LeftToRight => LayoutDirection::LeftToRight,
            Direction::RightToLeft => LayoutDirection::RightToLeft,
        }
    }
}

/// Render an ER diagram to SVG
pub fn render_er(db: &ErDb, config: &RenderConfig) -> Result<String> {
    let mut doc = SvgDocument::new();

    // Layout constants matching mermaid.js dimensions
    let entity_header_height = 42.75; // Matches mermaid's row height
    let attr_row_height = 42.75; // Each attribute row is same height as header
    let attr_font_size = 12.0;
    let margin = 50.0;
    let padding = 8.0;

    let entities = db.get_entities();

    if entities.is_empty() {
        // Empty diagram
        doc.set_size(400.0, 200.0);
        if !db.diagram_title.is_empty() {
            let title_elem = SvgElement::Text {
                x: 200.0,
                y: 30.0,
                content: db.diagram_title.clone(),
                attrs: Attrs::new()
                    .with_attr("text-anchor", "middle")
                    .with_class("er-title")
                    .with_attr("font-size", "20")
                    .with_attr("font-weight", "bold"),
            };
            doc.add_element(title_elem);
        }
        return Ok(doc.to_string());
    }

    // Calculate entity dimensions (width, height, column widths)
    let mut entity_dimensions: HashMap<String, EntityDimensions> = HashMap::new();
    for (name, entity) in entities {
        let display_name = if !entity.alias.is_empty() {
            &entity.alias
        } else {
            &entity.label
        };
        let dims = calculate_entity_dimensions(
            entity,
            display_name,
            entity_header_height,
            attr_row_height,
            attr_font_size,
            padding,
        );
        entity_dimensions.insert(name.clone(), dims);
    }

    // Sort entities for consistent ordering
    let mut sorted_entities: Vec<_> = entities.iter().collect();
    sorted_entities.sort_by(|a, b| a.0.cmp(b.0));

    // Use proper DAG layout based on relationships
    let size_estimator = CharacterSizeEstimator::default();
    let layout_input = db.to_layout_graph(&size_estimator)?;
    let layout_result = layout(layout_input)?;

    // Extract positions from layout, mapping entity IDs to (x, y)
    let mut entity_positions: HashMap<String, (f64, f64)> = HashMap::new();

    // Create a reverse mapping from entity ID to entity name
    let id_to_name: HashMap<String, String> = entities
        .iter()
        .map(|(name, entity)| (entity.id.clone(), name.clone()))
        .collect();

    for node in &layout_result.nodes {
        if let (Some(x), Some(y)) = (node.x, node.y) {
            // Map entity ID back to entity name
            if let Some(entity_name) = id_to_name.get(&node.id) {
                entity_positions.insert(entity_name.clone(), (x, y));
            }
        }
    }

    // Title offset
    let title_offset = if !db.diagram_title.is_empty() {
        40.0
    } else {
        0.0
    };

    // Calculate diagram bounds from layout
    let max_width = layout_result.width.unwrap_or(400.0) + margin * 2.0;
    let max_height = layout_result.height.unwrap_or(200.0) + margin * 2.0 + title_offset;

    doc.set_size(max_width, max_height);

    // Add theme styles
    if config.embed_css {
        doc.add_style(&config.theme.generate_css());
        doc.add_style(&generate_er_css(&config.theme));
    }

    // Add ER marker definitions
    doc.add_defs(generate_er_markers());

    // Render title
    if !db.diagram_title.is_empty() {
        let title_elem = SvgElement::Text {
            x: max_width / 2.0,
            y: 25.0,
            content: db.diagram_title.clone(),
            attrs: Attrs::new()
                .with_attr("text-anchor", "middle")
                .with_class("er-title")
                .with_attr("font-size", "20")
                .with_attr("font-weight", "bold"),
        };
        doc.add_element(title_elem);
    }

    // Create entity id to name mapping for relationship rendering
    let entity_id_to_name: HashMap<String, String> = entities
        .iter()
        .map(|(name, entity)| (entity.id.clone(), name.clone()))
        .collect();

    // Render relationships FIRST so entity boxes paint on top and clip markers
    // (SVG renders later elements on top of earlier ones)
    for relationship in db.get_relationships() {
        // Look up entity names from IDs
        let entity_a_name = entity_id_to_name.get(&relationship.entity_a);
        let entity_b_name = entity_id_to_name.get(&relationship.entity_b);

        if let (Some(a_name), Some(b_name)) = (entity_a_name, entity_b_name) {
            if let (Some(&(x1, y1)), Some(&(x2, y2))) =
                (entity_positions.get(a_name), entity_positions.get(b_name))
            {
                let dims1 = entity_dimensions.get(a_name);
                let dims2 = entity_dimensions.get(b_name);
                let h1 = dims1.map(|d| d.height).unwrap_or(entity_header_height);
                let h2 = dims2.map(|d| d.height).unwrap_or(entity_header_height);
                let w1 = dims1.map(|d| d.width).unwrap_or(188.0);
                let w2 = dims2.map(|d| d.width).unwrap_or(188.0);

                let rel_elem = render_relationship(
                    x1,
                    y1,
                    h1,
                    w1,
                    x2,
                    y2,
                    h2,
                    w2,
                    &relationship.role_a,
                    relationship.rel_spec.card_a,
                    relationship.rel_spec.card_b,
                    relationship.rel_spec.rel_type,
                );
                doc.add_element(rel_elem);
            }
        }
    }

    // Render entities AFTER relationships so entity boxes paint on top,
    // clipping the crow's feet markers behind the entity boxes
    for (name, entity) in &sorted_entities {
        if let Some(&(x, y)) = entity_positions.get(*name) {
            let dims = entity_dimensions
                .get(*name)
                .cloned()
                .unwrap_or(EntityDimensions {
                    width: 188.0,
                    height: entity_header_height + padding * 2.0,
                    col_widths: [65.8, 75.2, 47.0],
                });
            let entity_elem = render_entity(
                entity,
                x,
                y,
                dims.width,
                dims.height,
                entity_header_height,
                attr_row_height,
                padding,
                &dims.col_widths,
            );
            doc.add_element(entity_elem);
        }
    }

    Ok(doc.to_string())
}

/// Render an entity box with attributes in table-style layout
/// Matches mermaid.js with alternating row colors and column dividers
/// Uses CSS classes for theming - colors are defined in generate_er_css()
#[allow(clippy::too_many_arguments)]
fn render_entity(
    entity: &Entity,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    header_height: f64,
    attr_row_height: f64,
    _padding: f64,
    col_widths: &[f64; 3],
) -> SvgElement {
    // Collect shapes and text separately for correct z-order
    // SVG renders elements in document order - shapes must come before text
    let mut shapes = Vec::new();
    let mut text_elements = Vec::new();

    let num_attrs = entity.attributes.len();

    // Entity name for display
    let display_name = if !entity.alias.is_empty() {
        &entity.alias
    } else {
        &entity.label
    };

    // Entities without attributes: simple box with centered name (like mermaid.js)
    if num_attrs == 0 {
        shapes.push(SvgElement::Rect {
            x,
            y,
            width,
            height,
            rx: Some(0.0),
            ry: Some(0.0),
            attrs: Attrs::new().with_stroke_width(1.3).with_class("entity-box"),
        });

        text_elements.push(SvgElement::Text {
            x: x + width / 2.0,
            y: y + height / 2.0 + 5.0,
            content: display_name.clone(),
            attrs: Attrs::new()
                .with_attr("text-anchor", "middle")
                .with_class("entity-name")
                .with_attr("font-size", "14"),
        });

        let mut children = shapes;
        children.extend(text_elements);

        return SvgElement::Group {
            children,
            attrs: Attrs::new().with_class("entity-node").with_id(&entity.id),
        };
    }

    // Column positions calculated from col_widths [type, name, keys]
    let type_col_end = x + col_widths[0];
    let name_col_end = type_col_end + col_widths[1];

    // Main entity box (background)
    shapes.push(SvgElement::Rect {
        x,
        y,
        width,
        height,
        rx: Some(0.0),
        ry: Some(0.0),
        attrs: Attrs::new().with_stroke_width(1.3).with_class("entity-box"),
    });

    // Attribute rows with alternating backgrounds (starting after header)
    let content_y = y + header_height;

    for (i, attr) in entity.attributes.iter().enumerate() {
        let row_y = content_y + (i as f64) * attr_row_height;

        // Row background rectangle - CSS classes define colors
        shapes.push(SvgElement::Rect {
            x,
            y: row_y,
            width,
            height: attr_row_height,
            rx: Some(0.0),
            ry: Some(0.0),
            attrs: Attrs::new()
                .with_stroke_width(1.3)
                .with_class(if i % 2 == 0 {
                    "row-rect-odd"
                } else {
                    "row-rect-even"
                }),
        });

        // Text y position (vertically centered in row)
        let text_y = row_y + attr_row_height / 2.0 + 4.0;

        // Type column text
        text_elements.push(SvgElement::Text {
            x: x + 12.0, // left padding
            y: text_y,
            content: attr.attr_type.clone(),
            attrs: Attrs::new()
                .with_attr("text-anchor", "start")
                .with_class("entity-attr")
                .with_class("attribute-type")
                .with_attr("font-size", "12"),
        });

        // Name column text
        text_elements.push(SvgElement::Text {
            x: type_col_end + 12.0,
            y: text_y,
            content: attr.name.clone(),
            attrs: Attrs::new()
                .with_attr("text-anchor", "start")
                .with_class("entity-attr")
                .with_class("attribute-name")
                .with_attr("font-size", "12"),
        });

        // Keys column text (if present)
        if !attr.keys.is_empty() {
            let key_str = attr
                .keys
                .iter()
                .map(|k| k.as_str())
                .collect::<Vec<_>>()
                .join(",");
            text_elements.push(SvgElement::Text {
                x: name_col_end + 12.0,
                y: text_y,
                content: key_str,
                attrs: Attrs::new()
                    .with_attr("text-anchor", "start")
                    .with_class("entity-attr")
                    .with_class("attribute-key")
                    .with_attr("font-size", "12"),
            });
        }
    }

    // Divider lines - CSS class defines stroke color
    let divider_bottom = y + height;

    // Horizontal divider under header
    shapes.push(SvgElement::Line {
        x1: x,
        y1: content_y,
        x2: x + width,
        y2: content_y,
        attrs: Attrs::new().with_stroke_width(1.3).with_class("divider"),
    });

    // Vertical divider between type and name columns
    shapes.push(SvgElement::Line {
        x1: type_col_end,
        y1: content_y,
        x2: type_col_end,
        y2: divider_bottom,
        attrs: Attrs::new().with_stroke_width(1.3).with_class("divider"),
    });

    // Vertical divider between name and keys columns
    shapes.push(SvgElement::Line {
        x1: name_col_end,
        y1: content_y,
        x2: name_col_end,
        y2: divider_bottom,
        attrs: Attrs::new().with_stroke_width(1.3).with_class("divider"),
    });

    // Entity name (centered in header) - text comes after shapes
    text_elements.insert(
        0,
        SvgElement::Text {
            x: x + width / 2.0,
            y: y + header_height / 2.0 + 5.0,
            content: display_name.clone(),
            attrs: Attrs::new()
                .with_attr("text-anchor", "middle")
                .with_class("entity-name")
                .with_attr("font-size", "14"),
        },
    );

    // Combine shapes first, then text (correct z-order)
    let mut children = shapes;
    children.extend(text_elements);

    SvgElement::Group {
        children,
        attrs: Attrs::new().with_class("entity-node").with_id(&entity.id),
    }
}

/// Render a relationship line between two entities using SVG markers
/// Uses CSS classes for theming - colors are defined in generate_er_css()
#[allow(clippy::too_many_arguments)]
fn render_relationship(
    x1: f64,
    y1: f64,
    h1: f64,
    w1: f64,
    x2: f64,
    y2: f64,
    h2: f64,
    w2: f64,
    label: &str,
    card_a: Cardinality,
    card_b: Cardinality,
    rel_type: Identification,
) -> SvgElement {
    let mut children = Vec::new();

    // Calculate connection points
    let (start_x, start_y, end_x, end_y) =
        calculate_connection_points(x1, y1, h1, w1, x2, y2, h2, w2);

    // Calculate midpoint for Bezier curves (like mermaid.js)
    let mid_y = (start_y + end_y) / 2.0;

    // Create path data for the relationship line (using bezier curves like mermaid.js)
    let path_d = format!(
        "M{},{} C{},{} {},{} {},{}",
        start_x, start_y, start_x, mid_y, end_x, mid_y, end_x, end_y
    );

    // Get marker IDs for cardinalities
    // Note: Due to parser semantics, card_b is the left cardinality (for entity_a/start)
    // and card_a is the right cardinality (for entity_b/end)
    let marker_start = cardinality_to_marker_id(card_b, false);
    let marker_end = cardinality_to_marker_id(card_a, true);

    // Build path attributes with markers
    let mut path_attrs = Attrs::new()
        .with_class("relationshipLine")
        .with_attr("marker-start", &format!("url(#{})", marker_start))
        .with_attr("marker-end", &format!("url(#{})", marker_end));

    // Dotted line for non-identifying relationships
    if rel_type == Identification::NonIdentifying {
        path_attrs = path_attrs.with_stroke_dasharray("3");
    }

    children.push(SvgElement::Path {
        d: path_d,
        attrs: path_attrs,
    });

    // Relationship label
    if !label.is_empty() {
        let mid_x = (start_x + end_x) / 2.0;
        let label_mid_y = mid_y;

        // Background for label - uses CSS class for fill color
        let label_width = (label.len() as f64) * 7.0;
        children.push(SvgElement::Rect {
            x: mid_x - label_width / 2.0 - 4.0,
            y: label_mid_y - 12.0,
            width: label_width + 8.0,
            height: 23.0,
            rx: Some(0.0),
            ry: Some(0.0),
            attrs: Attrs::new().with_class("relationship-label-background"),
        });

        children.push(SvgElement::Text {
            x: mid_x,
            y: label_mid_y + 4.0,
            content: label.to_string(),
            attrs: Attrs::new()
                .with_attr("text-anchor", "middle")
                .with_class("relationship-label")
                .with_attr("font-size", "14"),
        });
    }

    SvgElement::Group {
        children,
        attrs: Attrs::new().with_class("relationship"),
    }
}

/// Calculate connection points on entity box edges
/// Uses a heuristic to prefer side attachment when there's significant horizontal offset,
/// which better matches mermaid.js behavior for diagonal relationships.
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

    // Threshold for considering positions "significantly offset" horizontally
    // If the x offset is more than 30% of the larger entity width, prefer side attachment
    let x_threshold = w1.max(w2) * 0.3;

    // Determine attachment for entity 1 (source)
    let (start_x, start_y) = if dx.abs() > dy.abs() || dx.abs() > x_threshold {
        // Horizontal offset is dominant or significant - use sides
        if dx > 0.0 {
            (x1 + w1, center1_y)
        } else {
            (x1, center1_y)
        }
    } else if dy > 0.0 {
        // Vertical relationship going down - use bottom
        (center1_x, y1 + h1)
    } else {
        // Vertical relationship going up - use top
        (center1_x, y1)
    };

    // Determine attachment for entity 2 (target)
    let (end_x, end_y) = if dx.abs() > dy.abs() || dx.abs() > x_threshold {
        // Horizontal offset is dominant or significant - use sides
        if dx > 0.0 {
            (x2, center2_y)
        } else {
            (x2 + w2, center2_y)
        }
    } else if dy > 0.0 {
        // Vertical relationship - use top of target
        (center2_x, y2)
    } else {
        // Vertical relationship going up - use bottom
        (center2_x, y2 + h2)
    };

    (start_x, start_y, end_x, end_y)
}

fn generate_er_css(theme: &Theme) -> String {
    format!(
        r#"
.er-title {{
  fill: {text_color};
}}

.entity-box {{
  fill: {primary_color};
  stroke: {border_color};
}}

.entity-header {{
  fill: {border_color};
  stroke: {border_color};
}}

.entity-name {{
  fill: {text_color};
  font-weight: bold;
}}

.entity-attr {{
  fill: {text_color};
}}

.relationshipLine {{
  stroke: {line_color};
  stroke-width: 1;
  fill: none;
}}

.relationship-label {{
  fill: {text_color};
}}

.relationship-label-background {{
  fill: {background};
  opacity: 0.7;
}}

.marker {{
  fill: none;
  stroke: {line_color};
  stroke-width: 1;
}}

.marker circle {{
  fill: {background};
}}

.row-rect-odd {{
  fill: {background};
}}

.row-rect-even {{
  fill: {primary_color};
}}

.divider {{
  stroke: {border_color};
}}
"#,
        text_color = theme.primary_text_color,
        primary_color = theme.primary_color,
        border_color = theme.primary_border_color,
        line_color = theme.line_color,
        background = theme.background,
    )
}

/// Generate SVG marker definitions for ER diagram cardinality symbols
/// These match the mermaid.js marker definitions
fn generate_er_markers() -> Vec<SvgElement> {
    vec![
        // onlyOneStart: Two vertical lines at the start (||)
        SvgElement::Marker {
            id: "er-onlyOneStart".to_string(),
            view_box: "0 0 18 18".to_string(),
            ref_x: 0.0,
            ref_y: 9.0,
            marker_width: 18.0,
            marker_height: 18.0,
            orient: "auto".to_string(),
            marker_units: None,
            children: vec![SvgElement::Path {
                d: "M9,0 L9,18 M15,0 L15,18".to_string(),
                attrs: Attrs::new().with_class("marker"),
            }],
        },
        // onlyOneEnd: Two vertical lines at the end (||)
        SvgElement::Marker {
            id: "er-onlyOneEnd".to_string(),
            view_box: "0 0 18 18".to_string(),
            ref_x: 18.0,
            ref_y: 9.0,
            marker_width: 18.0,
            marker_height: 18.0,
            orient: "auto".to_string(),
            marker_units: None,
            children: vec![SvgElement::Path {
                d: "M3,0 L3,18 M9,0 L9,18".to_string(),
                attrs: Attrs::new().with_class("marker"),
            }],
        },
        // zeroOrOneStart: Circle + one vertical line (o|)
        SvgElement::Marker {
            id: "er-zeroOrOneStart".to_string(),
            view_box: "0 0 30 18".to_string(),
            ref_x: 0.0,
            ref_y: 9.0,
            marker_width: 30.0,
            marker_height: 18.0,
            orient: "auto".to_string(),
            marker_units: None,
            children: vec![
                SvgElement::Circle {
                    cx: 21.0,
                    cy: 9.0,
                    r: 6.0,
                    attrs: Attrs::new().with_fill("white").with_class("marker"),
                },
                SvgElement::Path {
                    d: "M9,0 L9,18".to_string(),
                    attrs: Attrs::new().with_class("marker"),
                },
            ],
        },
        // zeroOrOneEnd: Circle + one vertical line (o|)
        SvgElement::Marker {
            id: "er-zeroOrOneEnd".to_string(),
            view_box: "0 0 30 18".to_string(),
            ref_x: 30.0,
            ref_y: 9.0,
            marker_width: 30.0,
            marker_height: 18.0,
            orient: "auto".to_string(),
            marker_units: None,
            children: vec![
                SvgElement::Circle {
                    cx: 9.0,
                    cy: 9.0,
                    r: 6.0,
                    attrs: Attrs::new().with_fill("white").with_class("marker"),
                },
                SvgElement::Path {
                    d: "M21,0 L21,18".to_string(),
                    attrs: Attrs::new().with_class("marker"),
                },
            ],
        },
        // oneOrMoreStart: Crow's foot + vertical line (|{)
        SvgElement::Marker {
            id: "er-oneOrMoreStart".to_string(),
            view_box: "0 0 45 36".to_string(),
            ref_x: 18.0,
            ref_y: 18.0,
            marker_width: 45.0,
            marker_height: 36.0,
            orient: "auto".to_string(),
            marker_units: None,
            children: vec![SvgElement::Path {
                d: "M0,18 Q 18,0 36,18 Q 18,36 0,18 M42,9 L42,27".to_string(),
                attrs: Attrs::new().with_class("marker"),
            }],
        },
        // oneOrMoreEnd: Vertical line + crow's foot ({|)
        SvgElement::Marker {
            id: "er-oneOrMoreEnd".to_string(),
            view_box: "0 0 45 36".to_string(),
            ref_x: 27.0,
            ref_y: 18.0,
            marker_width: 45.0,
            marker_height: 36.0,
            orient: "auto".to_string(),
            marker_units: None,
            children: vec![SvgElement::Path {
                d: "M3,9 L3,27 M9,18 Q27,0 45,18 Q27,36 9,18".to_string(),
                attrs: Attrs::new().with_class("marker"),
            }],
        },
        // zeroOrMoreStart: Crow's foot + circle (o{)
        SvgElement::Marker {
            id: "er-zeroOrMoreStart".to_string(),
            view_box: "0 0 57 36".to_string(),
            ref_x: 18.0,
            ref_y: 18.0,
            marker_width: 57.0,
            marker_height: 36.0,
            orient: "auto".to_string(),
            marker_units: None,
            children: vec![
                SvgElement::Circle {
                    cx: 48.0,
                    cy: 18.0,
                    r: 6.0,
                    attrs: Attrs::new().with_fill("white").with_class("marker"),
                },
                SvgElement::Path {
                    d: "M0,18 Q18,0 36,18 Q18,36 0,18".to_string(),
                    attrs: Attrs::new().with_class("marker"),
                },
            ],
        },
        // zeroOrMoreEnd: Circle + crow's foot ({o)
        SvgElement::Marker {
            id: "er-zeroOrMoreEnd".to_string(),
            view_box: "0 0 57 36".to_string(),
            ref_x: 39.0,
            ref_y: 18.0,
            marker_width: 57.0,
            marker_height: 36.0,
            orient: "auto".to_string(),
            marker_units: None,
            children: vec![
                SvgElement::Circle {
                    cx: 9.0,
                    cy: 18.0,
                    r: 6.0,
                    attrs: Attrs::new().with_fill("white").with_class("marker"),
                },
                SvgElement::Path {
                    d: "M21,18 Q39,0 57,18 Q39,36 21,18".to_string(),
                    attrs: Attrs::new().with_class("marker"),
                },
            ],
        },
    ]
}

/// Get the marker ID for a cardinality type
fn cardinality_to_marker_id(card: Cardinality, is_end: bool) -> String {
    let suffix = if is_end { "End" } else { "Start" };
    let name = match card {
        Cardinality::OnlyOne => "onlyOne",
        Cardinality::ZeroOrOne => "zeroOrOne",
        Cardinality::ZeroOrMore => "zeroOrMore",
        Cardinality::OneOrMore => "oneOrMore",
        Cardinality::MdParent => "onlyOne", // Use onlyOne for parent indicator
    };
    format!("er-{}{}", name, suffix)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagrams::er::parse;
    use crate::render::svg::SvgStructure;

    #[test]
    fn test_er_markers_generated() {
        // Test that ER diagrams with relationships include marker definitions
        let input = r#"erDiagram
    CUSTOMER ||--o{ ORDER : places
"#;
        let db = parse(input).unwrap();
        let config = RenderConfig::default();
        let svg = render_er(&db, &config).unwrap();

        // Should have marker definitions
        assert!(
            svg.contains("<marker id=\"er-onlyOneStart\""),
            "Should have er-onlyOneStart marker. SVG: {}",
            &svg[..500.min(svg.len())]
        );
        assert!(
            svg.contains("<marker id=\"er-zeroOrMoreEnd\""),
            "Should have er-zeroOrMoreEnd marker"
        );

        // Should have path with marker references
        assert!(
            svg.contains("marker-start=\"url(#er-onlyOneStart)\""),
            "Should have marker-start on relationship path"
        );
        assert!(
            svg.contains("marker-end=\"url(#er-zeroOrMoreEnd)\""),
            "Should have marker-end on relationship path"
        );
    }

    #[test]
    fn test_all_cardinality_markers_present() {
        // Test that all 8 marker types are generated
        let input = r#"erDiagram
    A ||--|| B : one-to-one
"#;
        let db = parse(input).unwrap();
        let config = RenderConfig::default();
        let svg = render_er(&db, &config).unwrap();

        // All 8 marker types should be defined
        let expected_markers = [
            "er-onlyOneStart",
            "er-onlyOneEnd",
            "er-zeroOrOneStart",
            "er-zeroOrOneEnd",
            "er-oneOrMoreStart",
            "er-oneOrMoreEnd",
            "er-zeroOrMoreStart",
            "er-zeroOrMoreEnd",
        ];

        for marker_id in expected_markers {
            assert!(
                svg.contains(&format!("<marker id=\"{}\"", marker_id)),
                "Should have {} marker defined",
                marker_id
            );
        }
    }

    #[test]
    fn test_relationship_uses_path_not_line() {
        // Test that relationships use path elements (for markers) not line elements
        let input = r#"erDiagram
    CUSTOMER ||--o{ ORDER : places
"#;
        let db = parse(input).unwrap();
        let config = RenderConfig::default();
        let svg = render_er(&db, &config).unwrap();

        // Parse structure
        let structure = SvgStructure::from_svg(&svg).unwrap();

        // Should have path elements for relationships (including marker paths)
        assert!(
            structure.shapes.path > 0,
            "Should have path elements for relationships. Got: {:?}",
            structure.shapes
        );

        // Should have markers defined
        assert!(
            structure.marker_count > 0,
            "Should have marker definitions. Got: {}",
            structure.marker_count
        );
    }

    #[test]
    fn test_attribute_labels_rendered_separately() {
        // Create an ER diagram with attributes
        let input = r#"erDiagram
    CUSTOMER {
        string name
        string email PK
        int id
    }
"#;
        let db = parse(input).unwrap();
        let config = RenderConfig::default();
        let svg = render_er(&db, &config).unwrap();

        // Parse the SVG structure to extract labels
        let structure = SvgStructure::from_svg(&svg).unwrap();

        // Mermaid.js renders each attribute component as a separate text element
        // So we should see "string", "name", "email", "PK", "int", "id" as separate labels
        assert!(
            structure.labels.iter().any(|l| l == "string"),
            "Should have 'string' as a separate label. Got: {:?}",
            structure.labels
        );
        assert!(
            structure.labels.iter().any(|l| l == "name"),
            "Should have 'name' as a separate label. Got: {:?}",
            structure.labels
        );
        assert!(
            structure.labels.iter().any(|l| l == "email"),
            "Should have 'email' as a separate label. Got: {:?}",
            structure.labels
        );
        assert!(
            structure.labels.iter().any(|l| l == "PK"),
            "Should have 'PK' as a separate label. Got: {:?}",
            structure.labels
        );
        assert!(
            structure.labels.iter().any(|l| l == "int"),
            "Should have 'int' as a separate label. Got: {:?}",
            structure.labels
        );
        assert!(
            structure.labels.iter().any(|l| l == "id"),
            "Should have 'id' as a separate label. Got: {:?}",
            structure.labels
        );
    }
}
