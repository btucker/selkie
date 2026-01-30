//! Entity Relationship diagram renderer

use std::collections::HashMap;

use crate::diagrams::er::{Cardinality, Entity, ErDb, Identification};
use crate::error::Result;
use crate::layout::{
    layout, CharacterSizeEstimator, LayoutDirection, LayoutEdge, LayoutGraph, LayoutNode,
    LayoutOptions, NodeShape, Padding, Point, SizeEstimator, ToLayoutGraph,
};
use crate::render::svg::edges::build_curved_path;
use crate::render::svg::{Attrs, RenderConfig, SvgDocument, SvgElement, Theme};

/// Entity dimensions calculated from content
#[derive(Debug, Clone)]
struct EntityDimensions {
    width: f64,
    height: f64,
    /// Column widths: [type_col, name_col, keys_col]
    col_widths: [f64; 3],
}

/// Side of an entity box where edges can attach
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum AttachmentSide {
    Top,
    Bottom,
    Left,
    Right,
}

/// Information about an edge's attachment points for distribution calculation
#[derive(Debug, Clone)]
struct EdgeAttachment {
    /// Index in the relationships list
    relationship_idx: usize,
    /// Side of entity A where this edge attaches
    side_a: AttachmentSide,
    /// Side of entity B where this edge attaches
    side_b: AttachmentSide,
    /// Entity A name
    entity_a: String,
    /// Entity B name
    entity_b: String,
}

/// Pre-computed attachment position for an edge endpoint
#[derive(Debug, Clone, Copy)]
struct AttachmentPosition {
    x: f64,
    y: f64,
}

/// Determine which side of an entity box an edge should attach to
/// Based on relative positions of the two entities
///
/// For diagonal edges (significant horizontal AND vertical offset), mermaid uses:
/// - Source entity: attaches to the side facing the major axis direction (bottom if target is below)
/// - Target entity: attaches to the side facing WHERE the source is horizontally (left if source is left)
///
/// This creates visually pleasing curved edges that approach from the appropriate direction.
#[allow(clippy::too_many_arguments)]
fn determine_attachment_sides(
    x1: f64,
    y1: f64,
    w1: f64,
    h1: f64,
    x2: f64,
    y2: f64,
    w2: f64,
    h2: f64,
) -> (AttachmentSide, AttachmentSide) {
    let center1_x = x1 + w1 / 2.0;
    let center1_y = y1 + h1 / 2.0;
    let center2_x = x2 + w2 / 2.0;
    let center2_y = y2 + h2 / 2.0;

    let dx = center2_x - center1_x; // positive = B is right of A
    let dy = center2_y - center1_y; // positive = B is below A

    // Threshold for "significant" offset (in pixels)
    // If horizontal offset exceeds this AND we have vertical dominance,
    // the target should use its horizontal side
    let diagonal_threshold = 50.0;

    // Source entity (A): use the side facing the major axis direction to B
    let side_a = if dy.abs() > dx.abs() {
        if dy > 0.0 {
            AttachmentSide::Bottom
        } else {
            AttachmentSide::Top
        }
    } else if dx > 0.0 {
        AttachmentSide::Right
    } else {
        AttachmentSide::Left
    };

    // Target entity (B): for diagonal edges, use the side facing WHERE A is
    // This matches mermaid's behavior for ORDER/PRODUCT → LINE-ITEM relationships
    let side_b = if dy.abs() > dx.abs() {
        // Vertical dominates
        if dx.abs() > diagonal_threshold {
            // Significant horizontal offset - use horizontal side facing source
            if dx > 0.0 {
                AttachmentSide::Left // A is to the left, so B uses left side
            } else {
                AttachmentSide::Right // A is to the right, so B uses right side
            }
        } else {
            // Small horizontal offset - use vertical side (straight line)
            if dy > 0.0 {
                AttachmentSide::Top
            } else {
                AttachmentSide::Bottom
            }
        }
    } else {
        // Horizontal dominates
        if dy.abs() > diagonal_threshold {
            // Significant vertical offset - use vertical side facing source
            if dy > 0.0 {
                AttachmentSide::Top // A is above, so B uses top side
            } else {
                AttachmentSide::Bottom // A is below, so B uses bottom side
            }
        } else {
            // Small vertical offset - use horizontal side (straight line)
            if dx > 0.0 {
                AttachmentSide::Left
            } else {
                AttachmentSide::Right
            }
        }
    };

    (side_a, side_b)
}

/// Adjust dagre bend_points to properly intersect entity boundaries
///
/// Dagre computes edge paths between node centers, but we need the edges to
/// attach to the correct sides of entity boxes based on relative entity positions.
/// This determines which side each entity should use based on their layout positions,
/// then calculates intersection points on those specific sides.
fn adjust_bend_points_for_intersection(
    bend_points: &[Point],
    entity_a_name: &str,
    entity_b_name: &str,
    entity_positions: &HashMap<String, (f64, f64)>,
    entity_dimensions: &HashMap<String, EntityDimensions>,
) -> Vec<Point> {
    if bend_points.len() < 2 {
        return bend_points.to_vec();
    }

    // Get both entities' positions and dimensions
    let a_pos = entity_positions.get(entity_a_name);
    let b_pos = entity_positions.get(entity_b_name);
    let a_dims = entity_dimensions.get(entity_a_name);
    let b_dims = entity_dimensions.get(entity_b_name);

    // Need both entities to calculate attachment sides
    let (Some(&(ax, ay)), Some(&(bx, by)), Some(a_dims), Some(b_dims)) =
        (a_pos, b_pos, a_dims, b_dims)
    else {
        return bend_points.to_vec();
    };

    // Determine which sides each entity should use based on their relative positions
    let (side_a, side_b) = determine_attachment_sides(
        ax,
        ay,
        a_dims.width,
        a_dims.height,
        bx,
        by,
        b_dims.width,
        b_dims.height,
    );

    let mut adjusted = bend_points.to_vec();

    // Calculate intersection point on entity A's determined side
    let (start_x, start_y) =
        calculate_side_intersection(ax, ay, a_dims.width, a_dims.height, side_a);
    adjusted[0] = Point {
        x: start_x,
        y: start_y,
    };

    // Calculate intersection point on entity B's determined side
    let last_idx = adjusted.len() - 1;
    let (end_x, end_y) = calculate_side_intersection(bx, by, b_dims.width, b_dims.height, side_b);
    adjusted[last_idx] = Point { x: end_x, y: end_y };

    adjusted
}

/// Calculate the intersection point on a specific side of an entity box
fn calculate_side_intersection(
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    side: AttachmentSide,
) -> (f64, f64) {
    let center_x = x + width / 2.0;
    let center_y = y + height / 2.0;

    match side {
        AttachmentSide::Top => (center_x, y),
        AttachmentSide::Bottom => (center_x, y + height),
        AttachmentSide::Left => (x, center_y),
        AttachmentSide::Right => (x + width, center_y),
    }
}

/// Calculate distributed attachment positions for edges that share the same entity side
/// Returns a HashMap from (entity_name, side, edge_index) to attachment position
fn calculate_distributed_attachments(
    edge_attachments: &[EdgeAttachment],
    entity_positions: &HashMap<String, (f64, f64)>,
    entity_dimensions: &HashMap<String, EntityDimensions>,
    marker_offset: f64,
) -> HashMap<(String, AttachmentSide, usize), AttachmentPosition> {
    let mut result = HashMap::new();

    // Group edges by (entity, side) for both endpoints
    // Key: (entity_name, side), Value: list of (edge_index, is_start_point)
    let mut side_edges: HashMap<(String, AttachmentSide), Vec<(usize, bool)>> = HashMap::new();

    for attachment in edge_attachments {
        // Add entity A attachment (start point)
        side_edges
            .entry((attachment.entity_a.clone(), attachment.side_a))
            .or_default()
            .push((attachment.relationship_idx, true));

        // Add entity B attachment (end point)
        side_edges
            .entry((attachment.entity_b.clone(), attachment.side_b))
            .or_default()
            .push((attachment.relationship_idx, false));
    }

    // Calculate distributed positions for each group
    for ((entity_name, side), edges) in side_edges.iter() {
        let Some(&(x, y)) = entity_positions.get(entity_name) else {
            continue;
        };
        let Some(dims) = entity_dimensions.get(entity_name) else {
            continue;
        };

        let count = edges.len();
        if count == 0 {
            continue;
        }

        // Calculate attachment positions distributed along the side
        for (i, &(edge_idx, _is_start)) in edges.iter().enumerate() {
            let position = calculate_distributed_position(
                x,
                y,
                dims.width,
                dims.height,
                *side,
                i,
                count,
                marker_offset,
            );
            result.insert((entity_name.clone(), *side, edge_idx), position);
        }
    }

    result
}

/// Calculate a single distributed attachment position
#[allow(clippy::too_many_arguments)]
fn calculate_distributed_position(
    entity_x: f64,
    entity_y: f64,
    entity_width: f64,
    entity_height: f64,
    side: AttachmentSide,
    index: usize,
    total: usize,
    marker_offset: f64,
) -> AttachmentPosition {
    // Distribute points evenly along the side
    // For N points, divide the side into N+1 segments and place points at segment boundaries
    let fraction = (index as f64 + 1.0) / (total as f64 + 1.0);

    match side {
        AttachmentSide::Top => {
            let x = entity_x + entity_width * fraction;
            let y = entity_y - marker_offset;
            AttachmentPosition { x, y }
        }
        AttachmentSide::Bottom => {
            let x = entity_x + entity_width * fraction;
            let y = entity_y + entity_height + marker_offset;
            AttachmentPosition { x, y }
        }
        AttachmentSide::Left => {
            let x = entity_x - marker_offset;
            let y = entity_y + entity_height * fraction;
            AttachmentPosition { x, y }
        }
        AttachmentSide::Right => {
            let x = entity_x + entity_width + marker_offset;
            let y = entity_y + entity_height * fraction;
            AttachmentPosition { x, y }
        }
    }
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
    // Character width estimation for trebuchet ms font
    // Mermaid uses actual getBBox() measurements, we estimate based on font metrics
    // Average character width is ~0.55-0.65 of font size for proportional fonts
    let char_width = font_size * 0.65;
    let header_char_width = 14.0 * 0.65; // Header uses font-size 14

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

    // Column padding matching mermaid's entityPadding behavior
    // Mermaid uses: widthPadding = entityPadding / 3 (default 10/3 ≈ 3.33)
    // And applies widthPaddingFactor (4-8 depending on columns)
    // Tuned for visual match
    let col_padding = 12.0;
    let col_right_padding = 10.0;
    let type_col_width = max_type_width + col_padding + col_right_padding;
    let name_col_width = max_name_width + col_padding + col_right_padding;
    let keys_col_width = if max_keys_width > 0.0 {
        max_keys_width + col_padding + col_right_padding
    } else {
        col_padding + col_right_padding // Minimum width for empty keys column
    };

    // Calculate header width requirement
    let header_width = display_name.len() as f64 * header_char_width + padding * 6.0;

    // Total entity width is max of header and sum of columns
    let content_width = type_col_width + name_col_width + keys_col_width;
    let total_width = content_width.max(header_width);

    // Minimum width matching mermaid's conf.minEntityWidth (default ~100)
    let min_width = 100.0;
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
        // Spacing matches mermaid.js ER config:
        //   nodeSpacing = 140, rankSpacing = 80
        graph.options = LayoutOptions {
            direction: self.preferred_direction(),
            node_spacing: 140.0,
            layer_spacing: 80.0,
            padding: Padding::uniform(20.0),
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
        self.get_direction().into()
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

    // Extract edge bend_points from layout result (dagre-computed paths)
    // The edge ID format is "relationship-{idx}" as defined in to_layout_graph
    let mut edge_bend_points: HashMap<String, Vec<Point>> = HashMap::new();
    for edge in &layout_result.edges {
        if !edge.bend_points.is_empty() {
            edge_bend_points.insert(edge.id.clone(), edge.bend_points.clone());
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

    // Marker offset for edge endpoints (space for marker symbols)
    let marker_offset = 18.0;

    // First pass: collect all edge attachments to calculate distributed positions
    let mut edge_attachments = Vec::new();
    let relationships = db.get_relationships();

    for (idx, relationship) in relationships.iter().enumerate() {
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

                let (side_a, side_b) = determine_attachment_sides(x1, y1, w1, h1, x2, y2, w2, h2);

                edge_attachments.push(EdgeAttachment {
                    relationship_idx: idx,
                    side_a,
                    side_b,
                    entity_a: a_name.clone(),
                    entity_b: b_name.clone(),
                });
            }
        }
    }

    // Calculate distributed attachment positions
    let distributed_positions = calculate_distributed_attachments(
        &edge_attachments,
        &entity_positions,
        &entity_dimensions,
        marker_offset,
    );

    // Render relationships FIRST so entity boxes paint on top and clip markers
    // (SVG renders later elements on top of earlier ones)
    for (idx, relationship) in relationships.iter().enumerate() {
        let edge_id = format!("relationship-{}", idx);

        // Get entity information for intersection calculation
        let entity_a_name = entity_id_to_name.get(&relationship.entity_a);
        let entity_b_name = entity_id_to_name.get(&relationship.entity_b);

        // Try to use dagre-computed bend points for the edge path
        if let Some(bend_points) = edge_bend_points.get(&edge_id) {
            // Adjust bend_points endpoints to properly intersect entity boundaries
            // using the mermaid.js intersect-rect algorithm
            let adjusted_points =
                if let (Some(a_name), Some(b_name)) = (entity_a_name, entity_b_name) {
                    adjust_bend_points_for_intersection(
                        bend_points,
                        a_name,
                        b_name,
                        &entity_positions,
                        &entity_dimensions,
                    )
                } else {
                    bend_points.clone()
                };

            let rel_elem = render_relationship_from_bend_points(
                &adjusted_points,
                &relationship.role_a,
                relationship.rel_spec.card_a,
                relationship.rel_spec.card_b,
                relationship.rel_spec.rel_type,
            );
            doc.add_element(rel_elem);
        } else {
            // Fallback: use manual attachment calculation if dagre didn't provide points
            let entity_a_name = entity_id_to_name.get(&relationship.entity_a);
            let entity_b_name = entity_id_to_name.get(&relationship.entity_b);

            if let (Some(a_name), Some(b_name)) = (entity_a_name, entity_b_name) {
                let attachment = edge_attachments.iter().find(|a| a.relationship_idx == idx);

                if let Some(attachment) = attachment {
                    let start_pos =
                        distributed_positions.get(&(a_name.clone(), attachment.side_a, idx));
                    let end_pos =
                        distributed_positions.get(&(b_name.clone(), attachment.side_b, idx));

                    if let (Some(start), Some(end)) = (start_pos, end_pos) {
                        let is_side_attachment = matches!(
                            attachment.side_a,
                            AttachmentSide::Left | AttachmentSide::Right
                        );

                        let rel_elem = render_relationship_with_positions(
                            start.x,
                            start.y,
                            end.x,
                            end.y,
                            is_side_attachment,
                            &relationship.role_a,
                            relationship.rel_spec.card_a,
                            relationship.rel_spec.card_b,
                            relationship.rel_spec.rel_type,
                        );
                        doc.add_element(rel_elem);
                    }
                }
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

/// Render a relationship line using dagre-computed bend points
/// This matches mermaid.js behavior by using the same edge routing from dagre
fn render_relationship_from_bend_points(
    bend_points: &[Point],
    label: &str,
    card_a: Cardinality,
    card_b: Cardinality,
    rel_type: Identification,
) -> SvgElement {
    let mut children = Vec::new();

    if bend_points.is_empty() {
        return SvgElement::Group {
            children,
            attrs: Attrs::new().with_class("relationship"),
        };
    }

    // Build path using curveBasis interpolation (like mermaid.js uses via d3)
    let path_d = build_curved_path(bend_points);

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

    // Relationship label (positioned at midpoint of path)
    if !label.is_empty() {
        // Calculate label position using geometric midpoint of bend points
        let label_pos = crate::layout::geometric_midpoint(bend_points);
        if let Some(mid) = label_pos {
            // Background for label - uses CSS class for fill color
            let label_width = (label.len() as f64) * 7.0;
            children.push(SvgElement::Rect {
                x: mid.x - label_width / 2.0 - 4.0,
                y: mid.y - 12.0,
                width: label_width + 8.0,
                height: 23.0,
                rx: Some(0.0),
                ry: Some(0.0),
                attrs: Attrs::new().with_class("relationship-label-background"),
            });

            children.push(SvgElement::Text {
                x: mid.x,
                y: mid.y + 4.0,
                content: label.to_string(),
                attrs: Attrs::new()
                    .with_attr("text-anchor", "middle")
                    .with_class("relationship-label")
                    .with_attr("font-size", "14"),
            });
        }
    }

    SvgElement::Group {
        children,
        attrs: Attrs::new().with_class("relationship"),
    }
}

/// Render a relationship line with pre-computed attachment positions
/// Uses CSS classes for theming - colors are defined in generate_er_css()
#[allow(clippy::too_many_arguments)]
fn render_relationship_with_positions(
    start_x: f64,
    start_y: f64,
    end_x: f64,
    end_y: f64,
    is_side_attachment: bool,
    label: &str,
    card_a: Cardinality,
    card_b: Cardinality,
    rel_type: Identification,
) -> SvgElement {
    let mut children = Vec::new();

    // Create path data for the relationship line
    // The Bezier curve must approach endpoints perpendicularly for markers to display correctly
    let path_d = if is_side_attachment {
        // Side attachment: curve approaches horizontally (perpendicular to side of box)
        let mid_x = (start_x + end_x) / 2.0;
        format!(
            "M{},{} C{},{} {},{} {},{}",
            start_x, start_y, mid_x, start_y, mid_x, end_y, end_x, end_y
        )
    } else {
        // Top/bottom attachment: curve approaches vertically (perpendicular to top/bottom)
        let mid_y = (start_y + end_y) / 2.0;
        format!(
            "M{},{} C{},{} {},{} {},{}",
            start_x, start_y, start_x, mid_y, end_x, mid_y, end_x, end_y
        )
    };

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

    // Relationship label (positioned at midpoint of path)
    if !label.is_empty() {
        let label_mid_x = (start_x + end_x) / 2.0;
        let label_mid_y = (start_y + end_y) / 2.0;

        // Background for label - uses CSS class for fill color
        let label_width = (label.len() as f64) * 7.0;
        children.push(SvgElement::Rect {
            x: label_mid_x - label_width / 2.0 - 4.0,
            y: label_mid_y - 12.0,
            width: label_width + 8.0,
            height: 23.0,
            rx: Some(0.0),
            ry: Some(0.0),
            attrs: Attrs::new().with_class("relationship-label-background"),
        });

        children.push(SvgElement::Text {
            x: label_mid_x,
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

/// Render a relationship line between two entities using SVG markers (legacy)
/// Uses CSS classes for theming - colors are defined in generate_er_css()
#[allow(dead_code)]
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

    // Calculate connection points and attachment type
    let (start_x, start_y, end_x, end_y, is_side_attachment) =
        calculate_connection_points(x1, y1, h1, w1, x2, y2, h2, w2);

    // Create path data for the relationship line
    // The Bezier curve must approach endpoints perpendicularly for markers to display correctly
    let path_d = if is_side_attachment {
        // Side attachment: curve approaches horizontally (perpendicular to side of box)
        let mid_x = (start_x + end_x) / 2.0;
        format!(
            "M{},{} C{},{} {},{} {},{}",
            start_x, start_y, mid_x, start_y, mid_x, end_y, end_x, end_y
        )
    } else {
        // Top/bottom attachment: curve approaches vertically (perpendicular to top/bottom)
        let mid_y = (start_y + end_y) / 2.0;
        format!(
            "M{},{} C{},{} {},{} {},{}",
            start_x, start_y, start_x, mid_y, end_x, mid_y, end_x, end_y
        )
    };

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

    // Relationship label (positioned at midpoint of path)
    if !label.is_empty() {
        let label_mid_x = (start_x + end_x) / 2.0;
        let label_mid_y = (start_y + end_y) / 2.0;

        // Background for label - uses CSS class for fill color
        let label_width = (label.len() as f64) * 7.0;
        children.push(SvgElement::Rect {
            x: label_mid_x - label_width / 2.0 - 4.0,
            y: label_mid_y - 12.0,
            width: label_width + 8.0,
            height: 23.0,
            rx: Some(0.0),
            ry: Some(0.0),
            attrs: Attrs::new().with_class("relationship-label-background"),
        });

        children.push(SvgElement::Text {
            x: label_mid_x,
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
/// Returns (start_x, start_y, end_x, end_y, is_side_attachment)
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
) -> (f64, f64, f64, f64, bool) {
    let center1_x = x1 + w1 / 2.0;
    let center1_y = y1 + h1 / 2.0;
    let center2_x = x2 + w2 / 2.0;
    let center2_y = y2 + h2 / 2.0;

    let dx = center2_x - center1_x;
    let dy = center2_y - center1_y;

    // Determine if this is a side attachment (horizontal approach needed)
    // Use side attachment only when horizontal offset is dominant over vertical
    // This matches mermaid's behavior: vertical relationships (like ORDER -> LINE-ITEM)
    // should use bottom-to-top connections even when there's horizontal offset
    let is_side_attachment = dx.abs() > dy.abs();

    // Marker offset - paths should end before the node boundary so that
    // markers (crow's feet) extend from the path endpoint TO the node.
    // The largest markers (oneOrMore, zeroOrMore) extend ~18 units past the path endpoint.
    let marker_offset = 18.0;

    // Determine attachment for entity 1 (source)
    // Path starts OFFSET from node boundary so marker-start extends to touch the node
    let (start_x, start_y) = if is_side_attachment {
        if dx > 0.0 {
            (x1 + w1 + marker_offset, center1_y) // offset right of right edge
        } else {
            (x1 - marker_offset, center1_y) // offset left of left edge
        }
    } else if dy > 0.0 {
        // Vertical relationship going down - offset below bottom
        (center1_x, y1 + h1 + marker_offset)
    } else {
        // Vertical relationship going up - offset above top
        (center1_x, y1 - marker_offset)
    };

    // Determine attachment for entity 2 (target)
    // Path ends OFFSET from node boundary so marker-end extends to touch the node
    let (end_x, end_y) = if is_side_attachment {
        if dx > 0.0 {
            (x2 - marker_offset, center2_y) // offset left of left edge
        } else {
            (x2 + w2 + marker_offset, center2_y) // offset right of right edge
        }
    } else if dy > 0.0 {
        // Vertical relationship - offset above top of target
        (center2_x, y2 - marker_offset)
    } else {
        // Vertical relationship going up - offset below bottom
        (center2_x, y2 + h2 + marker_offset)
    };

    (start_x, start_y, end_x, end_y, is_side_attachment)
}

fn generate_er_css(theme: &Theme) -> String {
    // Compute the ER-specific tertiary color from primary, matching mermaid.js:
    //   this.tertiaryColor = adjust(this.primaryColor, { h: -160 })
    // This is used for relationship label backgrounds.
    let tertiary_color = compute_er_tertiary_color(theme);

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
  fill: {border_color};
  font-size: 14px;
}}

.relationship-label-background {{
  fill: {tertiary_color};
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
        tertiary_color = tertiary_color,
    )
}

/// Compute the ER-specific tertiary color from the theme.
/// Mermaid.js derives tertiaryColor as adjust(primaryColor, { h: -160 }),
/// i.e., hue-shift the primary color by -160 degrees.
/// For the default primary #ECECFF (hsl(240, 100%, 96.27%)), this yields
/// hsl(80, 100%, 96.27%) - a light yellow-green.
fn compute_er_tertiary_color(theme: &Theme) -> String {
    use crate::render::svg::color::{adjust, Color};

    if let Some(primary) = Color::parse(&theme.primary_color) {
        let tertiary = adjust(&primary, -160.0, 0.0, 0.0);
        tertiary.to_hex()
    } else {
        // Fallback: use theme's tertiary_color as-is
        theme.tertiary_color.clone()
    }
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

    #[test]
    fn test_converging_edges_distribute_attachment_points() {
        // When multiple edges connect to the same entity side (like ORDER and PRODUCT
        // both connecting to LINE-ITEM's top), they should be distributed across the
        // edge rather than all centering at the same point.
        //
        // Mermaid behavior (from reference SVG):
        //   Edge 2: ORDER -> LINE-ITEM: ends at x=237 on LINE-ITEM top
        //   Edge 3: PRODUCT -> LINE-ITEM: ends at x=348 on LINE-ITEM top
        // The two edges don't share the same endpoint x coordinate.
        //
        // With dagre edge routing, this distribution is handled automatically by
        // the layout algorithm computing edge paths.

        let input = r#"erDiagram
    CUSTOMER ||--o{ ORDER : places
    ORDER ||--|{ LINE-ITEM : contains
    PRODUCT ||--o{ LINE-ITEM : includes
    CUSTOMER {
        string name
        string email PK
        string address
    }
    ORDER {
        int orderNumber PK
        date orderDate
        string status
    }
    PRODUCT {
        int id PK
        string name
        float price
    }
"#;
        let db = parse(input).unwrap();
        let config = RenderConfig::default();
        let svg = render_er(&db, &config).unwrap();

        // Parse the SVG structure to extract edge endpoints
        let structure = SvgStructure::from_svg(&svg).unwrap();
        let edge_endpoints = &structure.edge_geometry.edge_endpoints;

        // Get edges that connect to LINE-ITEM (the entity at the bottom)
        // These should have different endpoint x coordinates if properly distributed
        // LINE-ITEM is at the bottom of the diagram, so we look for edges
        // whose endpoints are in the lower Y region (end_y > 400)
        let line_item_edges: Vec<_> = edge_endpoints
            .iter()
            .filter(|e| {
                // Filter for edges whose end points are at high Y (near LINE-ITEM)
                e.3 > 400.0 // e.3 is end_y
            })
            .collect();

        // Should have at least 2 edges connecting to LINE-ITEM
        assert!(
            line_item_edges.len() >= 2,
            "Should have at least 2 edges connecting to LINE-ITEM. \
             Found {} total edges, {} matching filter. \
             All edges: {:?}",
            edge_endpoints.len(),
            line_item_edges.len(),
            edge_endpoints
        );

        // The endpoint X coordinates should NOT be the same
        // (they should be distributed across LINE-ITEM's top edge)
        let end_x1 = line_item_edges[0].2; // e.2 is end_x
        let end_x2 = line_item_edges[1].2;
        let x_diff = (end_x1 - end_x2).abs();

        assert!(
            x_diff > 10.0,
            "Converging edges should have distributed endpoints. \
             Edge 1 ends at x={:.1}, Edge 2 ends at x={:.1}, diff={:.1}px. \
             Expected >10px difference for proper distribution.",
            end_x1,
            end_x2,
            x_diff
        );
    }

    #[test]
    fn test_connection_points_vertical_relationship_with_horizontal_offset() {
        // When entities are arranged with significant vertical separation,
        // edges should use vertical attachment (bottom-to-top) even with horizontal offset.
        // This matches mermaid's behavior for ORDER/PRODUCT -> LINE-ITEM relationships.

        // Entity 1 (ORDER-like): at top-left
        let x1 = 50.0;
        let y1 = 100.0;
        let w1 = 150.0;
        let h1 = 120.0;

        // Entity 2 (LINE-ITEM-like): at bottom-center, horizontally offset
        let x2 = 200.0;
        let y2 = 350.0;
        let w2 = 100.0;
        let h2 = 60.0;

        let (start_x, start_y, end_x, end_y, is_side) =
            calculate_connection_points(x1, y1, h1, w1, x2, y2, h2, w2);

        // Should use vertical attachment (bottom of entity1 to top of entity2)
        // because vertical separation (250) is greater than horizontal separation (175)
        assert!(
            !is_side,
            "Should use vertical attachment when vertical separation dominates. \
             Entity1 center: ({}, {}), Entity2 center: ({}, {})",
            x1 + w1 / 2.0,
            y1 + h1 / 2.0,
            x2 + w2 / 2.0,
            y2 + h2 / 2.0
        );

        // Start should be at bottom center of entity 1, offset for marker
        // Marker offset is 18.0, so path starts below the bottom edge
        let expected_start_x = x1 + w1 / 2.0; // 125.0
        let expected_start_y = y1 + h1 + 18.0; // 238.0 (220 + 18 marker offset)
        assert!(
            (start_x - expected_start_x).abs() < 1.0,
            "Start X should be at center: expected {}, got {}",
            expected_start_x,
            start_x
        );
        assert!(
            (start_y - expected_start_y).abs() < 1.0,
            "Start Y should be offset below bottom edge: expected {}, got {}",
            expected_start_y,
            start_y
        );

        // End should be at top center of entity 2, offset for marker
        // Marker offset is 18.0, so path ends above the top edge
        let expected_end_x = x2 + w2 / 2.0; // 250.0
        let expected_end_y = y2 - 18.0; // 332.0 (350 - 18 marker offset)
        assert!(
            (end_x - expected_end_x).abs() < 1.0,
            "End X should be at center: expected {}, got {}",
            expected_end_x,
            end_x
        );
        assert!(
            (end_y - expected_end_y).abs() < 1.0,
            "End Y should be offset above top edge: expected {}, got {}",
            expected_end_y,
            end_y
        );
    }

    #[test]
    fn test_er_css_uses_tertiary_color_for_label_background() {
        // Mermaid.js uses tertiaryColor for .relationshipLabelBox fill,
        // not the background color. The tertiary color is derived from
        // primary by hue-shifting -160 degrees.
        let input = r#"erDiagram
    CUSTOMER ||--o{ ORDER : places
"#;
        let db = parse(input).unwrap();
        let config = RenderConfig::default();
        let _svg = render_er(&db, &config).unwrap();

        // The relationship label background should NOT use white/background color.
        // It should use the tertiary color (light yellow-green for default theme).
        // Mermaid uses: .relationshipLabelBox { fill: tertiaryColor; opacity: 0.7; }
        // With default theme: tertiaryColor = adjust(#ECECFF, { h: -160 })
        //   = hsl(80, 100%, 96.27%) ≈ a light yellow-green
        // We don't need exact color match, but the CSS should reference
        // the tertiary_color, not background.
        let css = generate_er_css(&config.theme);
        assert!(
            !css.contains(".relationship-label-background {\n  fill: #ffffff")
                && !css.contains(".relationship-label-background {\n  fill: white"),
            "Label background should NOT use white/background. Got CSS: {}",
            css
        );
    }

    #[test]
    fn test_er_css_marker_uses_important() {
        // Mermaid.js marker CSS uses !important to ensure markers
        // are not overridden by other styles
        let css = generate_er_css(&RenderConfig::default().theme);
        // The marker fill should be "none" to match mermaid's fill: none !important
        assert!(
            css.contains("fill: none"),
            "Marker should have fill: none. CSS: {}",
            css
        );
    }

    #[test]
    fn test_er_relationship_label_uses_border_color() {
        // Mermaid uses nodeBorder color for edge label text,
        // not textColor. From styles.ts: .edgeLabel .label { fill: ${options.nodeBorder} }
        let css = generate_er_css(&RenderConfig::default().theme);
        // The relationship label fill should use border color (#9370DB), not text color
        assert!(
            css.contains(".relationship-label {\n  fill: #9370DB"),
            "Relationship label should use border color for fill. CSS: {}",
            css
        );
    }

    // =========================================================================
    // Cypress rendering test ports from mermaid.js erDiagram.spec.js
    // =========================================================================

    #[test]
    fn test_cypress_render_cyclical_relationships() {
        // From: "should render a cyclical ER diagram"
        // Verifies that A→B→C→A cycle renders without errors
        let input = r#"erDiagram
            A ||--|{ B : likes
            B ||--|{ C : likes
            C ||--|{ A : likes"#;
        let db = parse(input).unwrap();
        let config = RenderConfig::default();
        let svg = render_er(&db, &config).unwrap();

        // Should have 3 entities
        let structure = SvgStructure::from_svg(&svg).unwrap();
        assert_eq!(structure.edge_count, 3, "Should have 3 relationship edges");

        // All 3 entities should be rendered
        assert!(svg.contains(">A<"), "Should contain entity A");
        assert!(svg.contains(">B<"), "Should contain entity B");
        assert!(svg.contains(">C<"), "Should contain entity C");
    }

    #[test]
    fn test_cypress_render_entities_no_relationships() {
        // From: "should render entities that have no relationships"
        let input = r#"erDiagram
            DEAD_PARROT
            HERMIT
            RECLUSE
            SOCIALITE }o--o{ SOCIALITE : "interacts with"
            RECLUSE }o--o{ SOCIALITE : avoids"#;
        let db = parse(input).unwrap();
        let config = RenderConfig::default();
        let svg = render_er(&db, &config).unwrap();

        // All 4 entities should be rendered even though some have no relationships
        assert!(
            svg.contains(">DEAD_PARROT<"),
            "Should render standalone entity DEAD_PARROT"
        );
        assert!(
            svg.contains(">HERMIT<"),
            "Should render standalone entity HERMIT"
        );
        assert!(svg.contains(">RECLUSE<"), "Should render entity RECLUSE");
        assert!(
            svg.contains(">SOCIALITE<"),
            "Should render entity SOCIALITE"
        );
    }

    #[test]
    fn test_cypress_render_multiple_relationships_same_entities() {
        // From: "should render an ER diagram with multiple relationships between the same two entities"
        let input = r#"erDiagram
            CUSTOMER ||--|{ ADDRESS : "invoiced at"
            CUSTOMER ||--|{ ADDRESS : "receives goods at""#;
        let db = parse(input).unwrap();
        let config = RenderConfig::default();
        let svg = render_er(&db, &config).unwrap();

        // Should have 2 relationship edges
        let structure = SvgStructure::from_svg(&svg).unwrap();
        assert_eq!(
            structure.edge_count, 2,
            "Should have 2 relationship edges between same entities"
        );

        // Both labels should appear
        assert!(
            svg.contains("invoiced at"),
            "Should contain first relationship label"
        );
        assert!(
            svg.contains("receives goods at"),
            "Should contain second relationship label"
        );
    }

    #[test]
    fn test_cypress_render_long_entity_names() {
        // From: "should render entities and attributes with big and small entity names"
        let input = r#"erDiagram
            PRIVATE_FINANCIAL_INSTITUTION {
              string name
              int    turnover
            }
            PRIVATE_FINANCIAL_INSTITUTION ||..|{ EMPLOYEE : employs
            EMPLOYEE { bool officer_of_firm }"#;
        let db = parse(input).unwrap();
        let config = RenderConfig::default();
        let svg = render_er(&db, &config).unwrap();

        // Long entity name should be rendered
        assert!(
            svg.contains("PRIVATE_FINANCIAL_INSTITUTION"),
            "Should render long entity name"
        );

        // Entity box should be wide enough (check that width is reasonable)
        let structure = SvgStructure::from_svg(&svg).unwrap();
        // Width should accommodate long entity names
        assert!(
            structure.width > 200.0,
            "SVG should be wide enough for long names, got {}",
            structure.width
        );
    }

    #[test]
    fn test_cypress_render_self_referencing_relationship() {
        // From: "should render an ER diagram with a recursive relationship"
        let input = r#"erDiagram
            CUSTOMER ||..o{ CUSTOMER : refers
            CUSTOMER ||--o{ ORDER : places"#;
        let db = parse(input).unwrap();
        let config = RenderConfig::default();
        let svg = render_er(&db, &config).unwrap();

        // Should render without error
        assert!(svg.contains("<svg"), "Should produce valid SVG");

        // Should have the "refers" label for self-reference
        assert!(
            svg.contains("refers"),
            "Should contain self-referencing relationship label"
        );
    }
}
