//! Entity Relationship diagram renderer

use std::collections::HashMap;

use crate::diagrams::er::{Cardinality, Direction, Entity, ErDb, Identification};
use crate::error::Result;
use crate::layout::{
    layout, CharacterSizeEstimator, LayoutDirection, LayoutEdge, LayoutGraph, LayoutNode,
    LayoutOptions, NodeShape, Padding, SizeEstimator, ToLayoutGraph,
};
use crate::render::svg::{Attrs, RenderConfig, SvgDocument, SvgElement};

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

        // Layout constants for entity sizing
        let entity_width = 160.0;
        let entity_header_height = 30.0;
        let attr_row_height = 20.0;
        let padding = 8.0;

        // Convert entities to layout nodes
        let entities = self.get_entities();

        // Sort entities by name for deterministic ordering
        let mut sorted_entities: Vec<(&String, &Entity)> = entities.iter().collect();
        sorted_entities.sort_by(|a, b| a.0.cmp(b.0));

        for (name, entity) in &sorted_entities {
            // Calculate entity height based on attributes
            let height = entity_header_height
                + (entity.attributes.len() as f64) * attr_row_height
                + padding * 2.0;
            let height = height.max(entity_header_height + padding * 2.0);

            let node = LayoutNode::new(&entity.id, entity_width, height)
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

    // Layout constants
    let entity_width = 160.0;
    let entity_header_height = 30.0;
    let attr_row_height = 20.0;
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

    // Calculate entity heights
    let mut entity_heights: HashMap<String, f64> = HashMap::new();
    for (name, entity) in entities {
        let height = entity_header_height
            + (entity.attributes.len() as f64) * attr_row_height
            + padding * 2.0;
        entity_heights.insert(
            name.clone(),
            height.max(entity_header_height + padding * 2.0),
        );
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
        doc.add_style(&generate_er_css());
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

    // Render each entity
    for (name, entity) in &sorted_entities {
        if let Some(&(x, y)) = entity_positions.get(*name) {
            let height = entity_heights
                .get(*name)
                .copied()
                .unwrap_or(entity_header_height);
            let entity_elem = render_entity(
                entity,
                x,
                y,
                entity_width,
                height,
                entity_header_height,
                attr_row_height,
                padding,
            );
            doc.add_element(entity_elem);
        }
    }

    // Create entity id to name mapping for relationship rendering
    let entity_id_to_name: HashMap<String, String> = entities
        .iter()
        .map(|(name, entity)| (entity.id.clone(), name.clone()))
        .collect();

    // Render relationships
    for relationship in db.get_relationships() {
        // Look up entity names from IDs
        let entity_a_name = entity_id_to_name.get(&relationship.entity_a);
        let entity_b_name = entity_id_to_name.get(&relationship.entity_b);

        if let (Some(a_name), Some(b_name)) = (entity_a_name, entity_b_name) {
            if let (Some(&(x1, y1)), Some(&(x2, y2))) =
                (entity_positions.get(a_name), entity_positions.get(b_name))
            {
                let h1 = entity_heights
                    .get(a_name)
                    .copied()
                    .unwrap_or(entity_header_height);
                let h2 = entity_heights
                    .get(b_name)
                    .copied()
                    .unwrap_or(entity_header_height);

                let rel_elem = render_relationship(
                    x1,
                    y1,
                    h1,
                    x2,
                    y2,
                    h2,
                    entity_width,
                    &relationship.role_a,
                    relationship.rel_spec.card_a,
                    relationship.rel_spec.card_b,
                    relationship.rel_spec.rel_type,
                );
                doc.add_element(rel_elem);
            }
        }
    }

    Ok(doc.to_string())
}

/// Render an entity box with attributes
#[allow(clippy::too_many_arguments)]
fn render_entity(
    entity: &Entity,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    header_height: f64,
    attr_row_height: f64,
    padding: f64,
) -> SvgElement {
    let mut children = Vec::new();

    // Entity box
    children.push(SvgElement::Rect {
        x,
        y,
        width,
        height,
        rx: Some(0.0),
        ry: Some(0.0),
        attrs: Attrs::new()
            .with_fill("#ECECFF")
            .with_stroke("#333333")
            .with_stroke_width(1.0)
            .with_class("entity-box"),
    });

    // Header background
    children.push(SvgElement::Rect {
        x,
        y,
        width,
        height: header_height,
        rx: Some(0.0),
        ry: Some(0.0),
        attrs: Attrs::new()
            .with_fill("#9370DB")
            .with_stroke("#333333")
            .with_stroke_width(1.0)
            .with_class("entity-header"),
    });

    // Entity name
    let display_name = if !entity.alias.is_empty() {
        &entity.alias
    } else {
        &entity.label
    };
    children.push(SvgElement::Text {
        x: x + width / 2.0,
        y: y + header_height / 2.0 + 5.0,
        content: display_name.clone(),
        attrs: Attrs::new()
            .with_attr("text-anchor", "middle")
            .with_class("entity-name")
            .with_attr("font-size", "14")
            .with_attr("font-weight", "bold")
            .with_fill("#FFFFFF"),
    });

    // Attributes - rendered as separate text elements per mermaid.js format
    // Column positions within the entity box
    let type_x = x + padding;
    let name_x = x + padding + 50.0; // After type column
    let keys_x = x + width - padding - 20.0; // Right-aligned

    let mut attr_y = y + header_height + padding;
    for attr in &entity.attributes {
        attr_y += attr_row_height;
        let text_y = attr_y - 4.0;

        // Type column (e.g., "string", "int", "date")
        children.push(SvgElement::Text {
            x: type_x,
            y: text_y,
            content: attr.attr_type.clone(),
            attrs: Attrs::new()
                .with_attr("text-anchor", "start")
                .with_class("entity-attr")
                .with_class("attribute-type")
                .with_attr("font-size", "11"),
        });

        // Name column (e.g., "name", "email", "id")
        children.push(SvgElement::Text {
            x: name_x,
            y: text_y,
            content: attr.name.clone(),
            attrs: Attrs::new()
                .with_attr("text-anchor", "start")
                .with_class("entity-attr")
                .with_class("attribute-name")
                .with_attr("font-size", "11"),
        });

        // Keys column (e.g., "PK", "FK", "UK" - if present)
        if !attr.keys.is_empty() {
            let key_str = attr
                .keys
                .iter()
                .map(|k| k.as_str())
                .collect::<Vec<_>>()
                .join(",");
            children.push(SvgElement::Text {
                x: keys_x,
                y: text_y,
                content: key_str,
                attrs: Attrs::new()
                    .with_attr("text-anchor", "start")
                    .with_class("entity-attr")
                    .with_class("attribute-key")
                    .with_attr("font-size", "11"),
            });
        }
    }

    SvgElement::Group {
        children,
        attrs: Attrs::new().with_class("entity-node").with_id(&entity.id),
    }
}

/// Render a relationship line between two entities using SVG markers
#[allow(clippy::too_many_arguments)]
fn render_relationship(
    x1: f64,
    y1: f64,
    h1: f64,
    x2: f64,
    y2: f64,
    h2: f64,
    width: f64,
    label: &str,
    card_a: Cardinality,
    card_b: Cardinality,
    rel_type: Identification,
) -> SvgElement {
    let mut children = Vec::new();

    // Calculate connection points
    let (start_x, start_y, end_x, end_y) =
        calculate_connection_points(x1, y1, h1, x2, y2, h2, width);

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

        // Background for label
        let label_width = (label.len() as f64) * 7.0;
        children.push(SvgElement::Rect {
            x: mid_x - label_width / 2.0 - 4.0,
            y: label_mid_y - 12.0,
            width: label_width + 8.0,
            height: 23.0,
            rx: Some(0.0),
            ry: Some(0.0),
            attrs: Attrs::new().with_class("background").with_fill("#FFFFFF"),
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
fn calculate_connection_points(
    x1: f64,
    y1: f64,
    h1: f64,
    x2: f64,
    y2: f64,
    h2: f64,
    width: f64,
) -> (f64, f64, f64, f64) {
    let center1_x = x1 + width / 2.0;
    let center1_y = y1 + h1 / 2.0;
    let center2_x = x2 + width / 2.0;
    let center2_y = y2 + h2 / 2.0;

    let dx = center2_x - center1_x;
    let dy = center2_y - center1_y;

    // Determine which edges to connect based on relative positions
    let (start_x, start_y) = if dx.abs() > dy.abs() {
        if dx > 0.0 {
            (x1 + width, center1_y)
        } else {
            (x1, center1_y)
        }
    } else if dy > 0.0 {
        (center1_x, y1 + h1)
    } else {
        (center1_x, y1)
    };

    let (end_x, end_y) = if dx.abs() > dy.abs() {
        if dx > 0.0 {
            (x2, center2_y)
        } else {
            (x2 + width, center2_y)
        }
    } else if dy > 0.0 {
        (center2_x, y2)
    } else {
        (center2_x, y2 + h2)
    };

    (start_x, start_y, end_x, end_y)
}

fn generate_er_css() -> String {
    r#"
.er-title {
  fill: #333333;
}

.entity-box {
  fill: #ECECFF;
  stroke: #333333;
}

.entity-header {
  fill: #9370DB;
  stroke: #333333;
}

.entity-name {
  fill: #FFFFFF;
  font-weight: bold;
}

.entity-attr {
  fill: #333333;
}

.relationshipLine {
  stroke: #333333;
  stroke-width: 1;
  fill: none;
}

.relationship-label {
  fill: #333333;
}

.marker {
  fill: none;
  stroke: #333333;
  stroke-width: 1;
}

.marker circle {
  fill: white;
}
"#
    .to_string()
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
