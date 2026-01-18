//! C4 diagram renderer
//!
//! Renders C4 diagrams (Context, Container, Component, Dynamic, Deployment)
//! following the C4 model visualization conventions.

use std::collections::HashMap;

use crate::diagrams::c4::{C4Boundary, C4Db, C4Element, C4Relationship, C4ShapeType};
use crate::error::Result;
use crate::render::svg::{Attrs, RenderConfig, SvgDocument, SvgElement};

// C4 element dimensions (approximating mermaid.js)
const ELEMENT_WIDTH: f64 = 200.0;
const ELEMENT_HEIGHT: f64 = 120.0;
const PERSON_HEIGHT: f64 = 140.0;
const ELEMENT_SPACING: f64 = 40.0;
const BOUNDARY_PADDING: f64 = 30.0;
const TITLE_HEIGHT: f64 = 40.0;

// C4 colors (mermaid.js defaults)
const COLOR_PERSON: &str = "#08427b";
const COLOR_PERSON_EXT: &str = "#999999";
const COLOR_SYSTEM: &str = "#1168bd";
const COLOR_SYSTEM_EXT: &str = "#999999";
const COLOR_CONTAINER: &str = "#438dd5";
const COLOR_CONTAINER_EXT: &str = "#999999";
const COLOR_COMPONENT: &str = "#85bbf0";
const COLOR_COMPONENT_EXT: &str = "#cccccc";
const COLOR_BOUNDARY: &str = "#444444";
const COLOR_TEXT_LIGHT: &str = "#ffffff";
const COLOR_TEXT_DARK: &str = "#333333";
const COLOR_REL: &str = "#666666";

/// Render a C4 diagram to SVG
pub fn render_c4(db: &C4Db, config: &RenderConfig) -> Result<String> {
    let mut doc = SvgDocument::new();

    // Calculate layout
    let layout = calculate_layout(db);

    // Set document size based on layout bounds
    let (width, height) = calculate_bounds(&layout);
    doc.set_size(width + ELEMENT_SPACING * 2.0, height + TITLE_HEIGHT + ELEMENT_SPACING * 2.0);

    // Add theme styles
    if config.embed_css {
        doc.add_style(&generate_c4_css(config));
    }

    // Add marker definitions for arrows
    doc.add_defs(create_c4_markers());

    // Render boundaries (background)
    for boundary in db.get_boundaries() {
        if let Some(bounds) = layout.boundary_bounds.get(&boundary.alias) {
            let element = render_boundary(boundary, bounds);
            doc.add_element(element);
        }
    }

    // Render elements
    for (element, position) in &layout.element_positions {
        let elem = render_element(element, position);
        doc.add_element(elem);
    }

    // Render relationships
    for relationship in db.get_relationships() {
        if let Some(element) = render_relationship(relationship, &layout) {
            doc.add_element(element);
        }
    }

    Ok(doc.to_string())
}

/// Position of an element
#[derive(Debug, Clone)]
struct Position {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
}

/// Boundary bounds
#[derive(Debug, Clone)]
struct BoundaryBounds {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    label: String,
}

/// Layout information for the diagram
struct Layout {
    element_positions: Vec<(C4Element, Position)>,
    boundary_bounds: HashMap<String, BoundaryBounds>,
}

/// Calculate the layout for all elements
fn calculate_layout(db: &C4Db) -> Layout {
    let mut element_positions: Vec<(C4Element, Position)> = Vec::new();
    let mut boundary_bounds: HashMap<String, BoundaryBounds> = HashMap::new();

    // Group elements by their parent boundary
    let mut elements_by_boundary: HashMap<String, Vec<&C4Element>> = HashMap::new();
    for element in db.get_elements() {
        let parent = element.parent_boundary.clone();
        elements_by_boundary.entry(parent).or_default().push(element);
    }

    // Track boundary nesting
    let mut boundary_parents: HashMap<String, String> = HashMap::new();
    for boundary in db.get_boundaries() {
        if !boundary.parent_boundary.is_empty() {
            boundary_parents.insert(boundary.alias.clone(), boundary.parent_boundary.clone());
        }
    }

    // Calculate positions - simple vertical stacking layout
    let mut current_y = TITLE_HEIGHT + ELEMENT_SPACING;
    let start_x = ELEMENT_SPACING;

    // First, process root elements (no boundary)
    if let Some(root_elements) = elements_by_boundary.get("") {
        let mut x = start_x;
        for element in root_elements {
            let height = element_height(&element.shape_type);
            element_positions.push((
                (*element).clone(),
                Position {
                    x,
                    y: current_y,
                    width: ELEMENT_WIDTH,
                    height,
                },
            ));
            x += ELEMENT_WIDTH + ELEMENT_SPACING;
        }
        if !root_elements.is_empty() {
            current_y += PERSON_HEIGHT + ELEMENT_SPACING;
        }
    }

    // Process each boundary
    for boundary in db.get_boundaries() {
        let boundary_start_y = current_y;
        let boundary_start_x = start_x;

        // Get elements in this boundary
        let boundary_elements = elements_by_boundary.get(&boundary.alias).cloned().unwrap_or_default();

        // Get nested boundaries
        let nested: Vec<_> = db.get_boundaries().iter()
            .filter(|b| b.parent_boundary == boundary.alias)
            .collect();

        // Calculate boundary content
        let mut max_x = boundary_start_x + BOUNDARY_PADDING;
        let mut inner_y = boundary_start_y + TITLE_HEIGHT;

        // Layout elements in this boundary
        let mut x = boundary_start_x + BOUNDARY_PADDING;
        for element in &boundary_elements {
            let height = element_height(&element.shape_type);
            element_positions.push((
                (*element).clone(),
                Position {
                    x,
                    y: inner_y,
                    width: ELEMENT_WIDTH,
                    height,
                },
            ));
            x += ELEMENT_WIDTH + ELEMENT_SPACING;
            max_x = max_x.max(x);
        }

        if !boundary_elements.is_empty() {
            inner_y += PERSON_HEIGHT + ELEMENT_SPACING;
        }

        // Process nested boundaries
        for nested_boundary in &nested {
            let nested_elements = elements_by_boundary.get(&nested_boundary.alias).cloned().unwrap_or_default();

            let nested_start_y = inner_y;
            let nested_start_x = boundary_start_x + BOUNDARY_PADDING;

            let mut nested_x = nested_start_x + BOUNDARY_PADDING;
            let mut nested_inner_y = nested_start_y + TITLE_HEIGHT;

            for element in &nested_elements {
                let height = element_height(&element.shape_type);
                element_positions.push((
                    (*element).clone(),
                    Position {
                        x: nested_x,
                        y: nested_inner_y,
                        width: ELEMENT_WIDTH,
                        height,
                    },
                ));
                nested_x += ELEMENT_WIDTH + ELEMENT_SPACING;
                max_x = max_x.max(nested_x + BOUNDARY_PADDING);
            }

            if !nested_elements.is_empty() {
                nested_inner_y += PERSON_HEIGHT + ELEMENT_SPACING;
            }

            // Store nested boundary bounds
            let nested_width = (nested_x - nested_start_x).max(ELEMENT_WIDTH + BOUNDARY_PADDING * 2.0);
            let nested_height = nested_inner_y - nested_start_y + BOUNDARY_PADDING;

            boundary_bounds.insert(nested_boundary.alias.clone(), BoundaryBounds {
                x: nested_start_x,
                y: nested_start_y,
                width: nested_width,
                height: nested_height,
                label: nested_boundary.label.clone(),
            });

            inner_y = nested_start_y + nested_height + ELEMENT_SPACING;
        }

        // Store boundary bounds
        let width = (max_x - boundary_start_x).max(ELEMENT_WIDTH + BOUNDARY_PADDING * 2.0);
        let height = inner_y - boundary_start_y + BOUNDARY_PADDING;

        boundary_bounds.insert(boundary.alias.clone(), BoundaryBounds {
            x: boundary_start_x,
            y: boundary_start_y,
            width,
            height,
            label: boundary.label.clone(),
        });

        current_y = boundary_start_y + height + ELEMENT_SPACING;
    }

    Layout {
        element_positions,
        boundary_bounds,
    }
}

/// Calculate the overall bounds of the diagram
fn calculate_bounds(layout: &Layout) -> (f64, f64) {
    let mut max_x: f64 = 400.0;
    let mut max_y: f64 = 300.0;

    for (_, pos) in &layout.element_positions {
        max_x = max_x.max(pos.x + pos.width);
        max_y = max_y.max(pos.y + pos.height);
    }

    for bounds in layout.boundary_bounds.values() {
        max_x = max_x.max(bounds.x + bounds.width);
        max_y = max_y.max(bounds.y + bounds.height);
    }

    (max_x, max_y)
}

/// Get element height based on shape type
fn element_height(shape_type: &C4ShapeType) -> f64 {
    match shape_type {
        C4ShapeType::Person | C4ShapeType::PersonExt => PERSON_HEIGHT,
        _ => ELEMENT_HEIGHT,
    }
}

/// Render a C4 element
fn render_element(element: &C4Element, position: &Position) -> SvgElement {
    let (bg_color, text_color) = element_colors(&element.shape_type);
    let mut children = Vec::new();

    // Create the shape based on type
    match element.shape_type {
        C4ShapeType::Person | C4ShapeType::PersonExt => {
            // Person shape: circle head + body rectangle
            let head_r = 25.0;
            let head_cx = position.x + position.width / 2.0;
            let head_cy = position.y + head_r + 5.0;

            children.push(SvgElement::Circle {
                cx: head_cx,
                cy: head_cy,
                r: head_r,
                attrs: Attrs::new().with_fill(bg_color).with_class("c4-person-head"),
            });

            let body_y = head_cy + head_r + 5.0;
            let body_height = position.height - (body_y - position.y);
            children.push(SvgElement::Rect {
                x: position.x,
                y: body_y,
                width: position.width,
                height: body_height,
                rx: Some(5.0),
                ry: Some(5.0),
                attrs: Attrs::new().with_fill(bg_color).with_class("c4-person-body"),
            });
        }
        C4ShapeType::SystemDb | C4ShapeType::SystemDbExt |
        C4ShapeType::ContainerDb | C4ShapeType::ContainerDbExt |
        C4ShapeType::ComponentDb | C4ShapeType::ComponentDbExt => {
            // Database shape: cylinder
            let rx = position.width / 2.0;
            let ry = 15.0;
            let cx = position.x + position.width / 2.0;

            // Top ellipse
            children.push(SvgElement::Ellipse {
                cx,
                cy: position.y + ry,
                rx,
                ry,
                attrs: Attrs::new().with_fill(bg_color).with_class("c4-db-top"),
            });

            // Body rectangle
            children.push(SvgElement::Rect {
                x: position.x,
                y: position.y + ry,
                width: position.width,
                height: position.height - ry * 2.0,
                rx: None,
                ry: None,
                attrs: Attrs::new().with_fill(bg_color).with_class("c4-db-body"),
            });

            // Bottom ellipse (just the visible part)
            let path = format!(
                "M {} {} A {} {} 0 0 0 {} {} L {} {} A {} {} 0 0 0 {} {} Z",
                position.x, position.y + position.height - ry,
                rx, ry,
                position.x + position.width, position.y + position.height - ry,
                position.x + position.width, position.y + position.height - ry,
                rx, ry,
                position.x, position.y + position.height - ry
            );
            children.push(SvgElement::Path {
                d: path,
                attrs: Attrs::new().with_fill(bg_color).with_class("c4-db-bottom"),
            });
        }
        C4ShapeType::SystemQueue | C4ShapeType::SystemQueueExt |
        C4ShapeType::ContainerQueue | C4ShapeType::ContainerQueueExt |
        C4ShapeType::ComponentQueue | C4ShapeType::ComponentQueueExt => {
            // Queue shape: rectangle with rounded ends (like a pipe)
            children.push(SvgElement::Rect {
                x: position.x,
                y: position.y,
                width: position.width,
                height: position.height,
                rx: Some(position.height / 2.0),
                ry: Some(position.height / 2.0),
                attrs: Attrs::new().with_fill(bg_color).with_class("c4-queue"),
            });
        }
        _ => {
            // Standard rectangle for systems, containers, components
            children.push(SvgElement::Rect {
                x: position.x,
                y: position.y,
                width: position.width,
                height: position.height,
                rx: Some(5.0),
                ry: Some(5.0),
                attrs: Attrs::new().with_fill(bg_color).with_class("c4-element"),
            });
        }
    }

    // Add text labels
    let text_y = position.y + 35.0;
    let text_x = position.x + position.width / 2.0;

    // Element name
    children.push(SvgElement::Text {
        x: text_x,
        y: text_y,
        content: element.label.clone(),
        attrs: Attrs::new()
            .with_fill(text_color)
            .with_attr("text-anchor", "middle")
            .with_attr("font-weight", "bold")
            .with_attr("font-size", "14")
            .with_class("c4-label"),
    });

    // Technology (if present)
    if !element.technology.is_empty() {
        children.push(SvgElement::Text {
            x: text_x,
            y: text_y + 18.0,
            content: format!("[{}]", element.technology),
            attrs: Attrs::new()
                .with_fill(text_color)
                .with_attr("text-anchor", "middle")
                .with_attr("font-size", "11")
                .with_class("c4-technology"),
        });
    }

    // Description
    if !element.description.is_empty() {
        let desc_y = if element.technology.is_empty() {
            text_y + 25.0
        } else {
            text_y + 40.0
        };

        // Wrap long descriptions
        let wrapped = wrap_text(&element.description, 30);
        for (i, line) in wrapped.iter().enumerate() {
            children.push(SvgElement::Text {
                x: text_x,
                y: desc_y + (i as f64 * 14.0),
                content: line.clone(),
                attrs: Attrs::new()
                    .with_fill(text_color)
                    .with_attr("text-anchor", "middle")
                    .with_attr("font-size", "11")
                    .with_class("c4-description"),
            });
        }
    }

    SvgElement::Group {
        children,
        attrs: Attrs::new()
            .with_class("c4-element-group")
            .with_id(&element.alias),
    }
}

/// Render a boundary
fn render_boundary(boundary: &C4Boundary, bounds: &BoundaryBounds) -> SvgElement {
    let mut children = Vec::new();

    // Boundary rectangle with dashed border
    children.push(SvgElement::Rect {
        x: bounds.x,
        y: bounds.y,
        width: bounds.width,
        height: bounds.height,
        rx: Some(5.0),
        ry: Some(5.0),
        attrs: Attrs::new()
            .with_fill("none")
            .with_stroke(COLOR_BOUNDARY)
            .with_stroke_width(2.0)
            .with_attr("stroke-dasharray", "10,5")
            .with_class("c4-boundary"),
    });

    // Boundary label
    children.push(SvgElement::Text {
        x: bounds.x + 10.0,
        y: bounds.y + 20.0,
        content: bounds.label.clone(),
        attrs: Attrs::new()
            .with_fill(COLOR_BOUNDARY)
            .with_attr("font-weight", "bold")
            .with_attr("font-size", "14")
            .with_class("c4-boundary-label"),
    });

    SvgElement::Group {
        children,
        attrs: Attrs::new()
            .with_class("c4-boundary-group")
            .with_id(&boundary.alias),
    }
}

/// Render a relationship
fn render_relationship(rel: &C4Relationship, layout: &Layout) -> Option<SvgElement> {
    // Find source and target positions
    let source_pos = layout.element_positions.iter()
        .find(|(e, _)| e.alias == rel.from)
        .map(|(_, p)| p)?;

    let target_pos = layout.element_positions.iter()
        .find(|(e, _)| e.alias == rel.to)
        .map(|(_, p)| p)?;

    let mut children = Vec::new();

    // Calculate connection points (center of each element)
    let start_x = source_pos.x + source_pos.width / 2.0;
    let start_y = source_pos.y + source_pos.height;
    let end_x = target_pos.x + target_pos.width / 2.0;
    let end_y = target_pos.y;

    // If elements are side by side, connect horizontally
    let (sx, sy, ex, ey) = if (source_pos.y - target_pos.y).abs() < source_pos.height {
        // Horizontal connection
        let sy = source_pos.y + source_pos.height / 2.0;
        let ey = target_pos.y + target_pos.height / 2.0;
        if source_pos.x < target_pos.x {
            (source_pos.x + source_pos.width, sy, target_pos.x, ey)
        } else {
            (source_pos.x, sy, target_pos.x + target_pos.width, ey)
        }
    } else {
        (start_x, start_y, end_x, end_y)
    };

    // Draw the line
    let path = format!("M {} {} L {} {}", sx, sy, ex, ey);
    children.push(SvgElement::Path {
        d: path,
        attrs: Attrs::new()
            .with_fill("none")
            .with_stroke(COLOR_REL)
            .with_stroke_width(1.5)
            .with_attr("marker-end", "url(#c4-arrow)")
            .with_class("c4-relationship"),
    });

    // Add label at midpoint
    if !rel.label.is_empty() {
        let mid_x = (sx + ex) / 2.0;
        let mid_y = (sy + ey) / 2.0;

        // Background for label
        let label_width = rel.label.len() as f64 * 7.0 + 10.0;
        children.push(SvgElement::Rect {
            x: mid_x - label_width / 2.0,
            y: mid_y - 10.0,
            width: label_width,
            height: 20.0,
            rx: Some(3.0),
            ry: Some(3.0),
            attrs: Attrs::new()
                .with_fill("#ffffff")
                .with_stroke(COLOR_REL)
                .with_stroke_width(1.0)
                .with_class("c4-rel-label-bg"),
        });

        children.push(SvgElement::Text {
            x: mid_x,
            y: mid_y + 4.0,
            content: rel.label.clone(),
            attrs: Attrs::new()
                .with_fill(COLOR_REL)
                .with_attr("text-anchor", "middle")
                .with_attr("font-size", "11")
                .with_class("c4-rel-label"),
        });
    }

    // Add technology label if present
    if !rel.technology.is_empty() {
        let mid_x = (sx + ex) / 2.0;
        let mid_y = (sy + ey) / 2.0 + 15.0;

        children.push(SvgElement::Text {
            x: mid_x,
            y: mid_y,
            content: format!("[{}]", rel.technology),
            attrs: Attrs::new()
                .with_fill(COLOR_REL)
                .with_attr("text-anchor", "middle")
                .with_attr("font-size", "10")
                .with_class("c4-rel-technology"),
        });
    }

    Some(SvgElement::Group {
        children,
        attrs: Attrs::new().with_class("c4-relationship-group"),
    })
}

/// Get colors for an element type
fn element_colors(shape_type: &C4ShapeType) -> (&'static str, &'static str) {
    match shape_type {
        C4ShapeType::Person => (COLOR_PERSON, COLOR_TEXT_LIGHT),
        C4ShapeType::PersonExt => (COLOR_PERSON_EXT, COLOR_TEXT_LIGHT),
        C4ShapeType::System | C4ShapeType::SystemDb | C4ShapeType::SystemQueue => {
            (COLOR_SYSTEM, COLOR_TEXT_LIGHT)
        }
        C4ShapeType::SystemExt | C4ShapeType::SystemDbExt | C4ShapeType::SystemQueueExt => {
            (COLOR_SYSTEM_EXT, COLOR_TEXT_LIGHT)
        }
        C4ShapeType::Container | C4ShapeType::ContainerDb | C4ShapeType::ContainerQueue => {
            (COLOR_CONTAINER, COLOR_TEXT_LIGHT)
        }
        C4ShapeType::ContainerExt | C4ShapeType::ContainerDbExt | C4ShapeType::ContainerQueueExt => {
            (COLOR_CONTAINER_EXT, COLOR_TEXT_LIGHT)
        }
        C4ShapeType::Component | C4ShapeType::ComponentDb | C4ShapeType::ComponentQueue => {
            (COLOR_COMPONENT, COLOR_TEXT_DARK)
        }
        C4ShapeType::ComponentExt | C4ShapeType::ComponentDbExt | C4ShapeType::ComponentQueueExt => {
            (COLOR_COMPONENT_EXT, COLOR_TEXT_DARK)
        }
    }
}

/// Wrap text to fit within a character limit
fn wrap_text(text: &str, max_chars: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current_line = String::new();

    for word in text.split_whitespace() {
        if current_line.is_empty() {
            current_line = word.to_string();
        } else if current_line.len() + word.len() + 1 <= max_chars {
            current_line.push(' ');
            current_line.push_str(word);
        } else {
            lines.push(current_line);
            current_line = word.to_string();
        }
    }

    if !current_line.is_empty() {
        lines.push(current_line);
    }

    // Limit to 3 lines to prevent overflow
    if lines.len() > 3 {
        lines.truncate(2);
        if let Some(last) = lines.last_mut() {
            last.push_str("...");
        }
    }

    lines
}

/// Create arrow markers for relationships
fn create_c4_markers() -> Vec<SvgElement> {
    vec![SvgElement::Raw {
        content: r##"<marker id="c4-arrow" viewBox="0 0 10 10" refX="9" refY="5" markerWidth="6" markerHeight="6" orient="auto-start-reverse">
    <path d="M 0 0 L 10 5 L 0 10 z" fill="#666666"/>
</marker>"##.to_string(),
    }]
}

/// Generate CSS for C4 diagrams
fn generate_c4_css(config: &RenderConfig) -> String {
    format!(
        r#"
.c4-element-group {{
  cursor: pointer;
}}

.c4-element, .c4-person-body {{
  stroke: rgba(0,0,0,0.3);
  stroke-width: 1px;
}}

.c4-person-head {{
  stroke: rgba(0,0,0,0.3);
  stroke-width: 1px;
}}

.c4-db-top, .c4-db-body {{
  stroke: rgba(0,0,0,0.3);
  stroke-width: 1px;
}}

.c4-queue {{
  stroke: rgba(0,0,0,0.3);
  stroke-width: 1px;
}}

.c4-boundary {{
  fill: none;
}}

.c4-label, .c4-technology, .c4-description {{
  font-family: {font_family};
}}

.c4-boundary-label {{
  font-family: {font_family};
}}

.c4-rel-label, .c4-rel-technology {{
  font-family: {font_family};
}}

.c4-relationship {{
  stroke-linecap: round;
}}
"#,
        font_family = config.theme.font_family
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wrap_text_short() {
        let result = wrap_text("Hello world", 30);
        assert_eq!(result, vec!["Hello world"]);
    }

    #[test]
    fn test_wrap_text_long() {
        let result = wrap_text("This is a very long description that needs to be wrapped", 20);
        assert!(result.len() > 1);
    }

    #[test]
    fn test_element_colors() {
        let (bg, text) = element_colors(&C4ShapeType::Person);
        assert_eq!(bg, COLOR_PERSON);
        assert_eq!(text, COLOR_TEXT_LIGHT);

        let (bg, text) = element_colors(&C4ShapeType::Component);
        assert_eq!(bg, COLOR_COMPONENT);
        assert_eq!(text, COLOR_TEXT_DARK);
    }
}
