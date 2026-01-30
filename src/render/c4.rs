//! C4 diagram renderer
//!
//! Renders C4 diagrams (Context, Container, Component, Dynamic, Deployment)
//! following the C4 model visualization conventions.

use std::collections::HashMap;

use crate::diagrams::c4::{C4Boundary, C4Db, C4Element, C4Relationship, C4ShapeType};
use crate::error::Result;
use crate::render::svg::{Attrs, RenderConfig, SvgDocument, SvgElement};

// C4 layout configuration (matching mermaid.js defaults)
const SHAPES_PER_ROW: usize = 4;
const ELEMENT_WIDTH: f64 = 216.0;
const ELEMENT_HEIGHT: f64 = 119.0; // mermaid default is 60, but text content typically makes it ~119
const PERSON_HEIGHT: f64 = 167.0; // Person shapes are taller due to icon
const ELEMENT_MARGIN: f64 = 50.0; // mermaid c4ShapeMargin default
const DIAGRAM_MARGIN: f64 = 50.0; // mermaid diagramMarginX default
const BOUNDARY_PADDING: f64 = 20.0; // mermaid c4ShapePadding default
const MAX_DIAGRAM_WIDTH: f64 = 800.0; // Max width before wrapping to new row

// C4 colors (mermaid.js defaults)
const COLOR_PERSON: &str = "#08427b";
const COLOR_PERSON_EXT: &str = "#62717c";
const COLOR_SYSTEM: &str = "#1168bd";
const COLOR_SYSTEM_EXT: &str = "#999999";
const COLOR_CONTAINER: &str = "#438dd5";
const COLOR_CONTAINER_EXT: &str = "#999999";
const COLOR_COMPONENT: &str = "#85bbf0";
const COLOR_COMPONENT_EXT: &str = "#cccccc";
const COLOR_BOUNDARY: &str = "#444444";
const COLOR_TEXT_LIGHT: &str = "#ffffff";
const COLOR_TEXT_DARK: &str = "#333333";
const COLOR_REL: &str = "#444444";

/// Render a C4 diagram to SVG
pub fn render_c4(db: &C4Db, config: &RenderConfig) -> Result<String> {
    let mut doc = SvgDocument::new();

    // Calculate layout using grid-based algorithm
    let layout = calculate_layout(db);

    // Set document size based on layout bounds
    let padding = DIAGRAM_MARGIN * 2.0;
    doc.set_size(layout.total_width + padding, layout.total_height + padding);

    // Add theme styles
    if config.embed_css {
        doc.add_style(&generate_c4_css(config));
    }

    // Add marker definitions for arrows
    doc.add_defs(create_c4_markers());

    // Render title if present
    if let Some(title) = db.get_title() {
        doc.add_element(render_title(title, layout.total_width + padding));
    }

    // Render boundaries first (background)
    for (alias, bounds) in &layout.boundary_bounds {
        if let Some(boundary) = db.get_boundaries().iter().find(|b| &b.alias == alias) {
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
    for (index, relationship) in db.get_relationships().iter().enumerate() {
        if let Some(element) = render_relationship(relationship, &layout, index) {
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
    total_width: f64,
    total_height: f64,
}

/// Bounds tracker for layout algorithm (matches mermaid.js Bounds class)
#[derive(Debug, Clone, Default)]
struct Bounds {
    start_x: f64,
    stop_x: f64,
    start_y: f64,
    stop_y: f64,
    // For tracking current row
    next_x: f64,
    next_y: f64,
    row_count: usize,
    row_max_height: f64,
    // Width limit for wrapping
    width_limit: f64,
}

impl Bounds {
    fn new(x: f64, y: f64) -> Self {
        Self {
            start_x: x,
            stop_x: x,
            start_y: y,
            stop_y: y,
            next_x: x,
            next_y: y,
            row_count: 0,
            row_max_height: 0.0,
            width_limit: MAX_DIAGRAM_WIDTH,
        }
    }

    /// Insert an element into the bounds, using grid layout
    fn insert(&mut self, width: f64, height: f64) -> (f64, f64) {
        self.row_count += 1;

        // Check if we need to start a new row (either by count or width limit)
        let would_exceed_width = self.next_x + width > self.width_limit;
        if self.row_count > SHAPES_PER_ROW || would_exceed_width {
            self.next_x = self.start_x;
            self.next_y = self.stop_y + ELEMENT_MARGIN;
            self.row_count = 1;
            self.row_max_height = 0.0;
        }

        let x = self.next_x;
        let y = self.next_y;

        // Update bounds
        self.next_x = x + width + ELEMENT_MARGIN;
        self.stop_x = self.stop_x.max(x + width);
        self.row_max_height = self.row_max_height.max(height);
        self.stop_y = self.stop_y.max(y + self.row_max_height);

        (x, y)
    }

    /// Get the width of the bounded area
    fn width(&self) -> f64 {
        self.stop_x - self.start_x
    }

    /// Get the height of the bounded area
    fn height(&self) -> f64 {
        self.stop_y - self.start_y
    }

    /// Add margin after all elements
    fn add_margin(&mut self, margin: f64) {
        self.stop_x += margin;
        self.stop_y += margin;
    }
}

/// Calculate the layout for all elements using grid-based algorithm
fn calculate_layout(db: &C4Db) -> Layout {
    let mut element_positions: Vec<(C4Element, Position)> = Vec::new();
    let mut boundary_bounds: HashMap<String, BoundaryBounds> = HashMap::new();

    // Group elements by their parent boundary
    let mut elements_by_boundary: HashMap<String, Vec<&C4Element>> = HashMap::new();
    for element in db.get_elements() {
        let parent = element.parent_boundary.clone();
        elements_by_boundary
            .entry(parent)
            .or_default()
            .push(element);
    }

    // Create root bounds
    let mut root_bounds = Bounds::new(DIAGRAM_MARGIN, DIAGRAM_MARGIN);

    // Process root-level elements (those with no parent boundary)
    if let Some(root_elements) = elements_by_boundary.get("") {
        for element in root_elements {
            let height = element_height(&element.shape_type);
            let (x, y) = root_bounds.insert(ELEMENT_WIDTH, height);
            element_positions.push((
                (*element).clone(),
                Position {
                    x,
                    y,
                    width: ELEMENT_WIDTH,
                    height,
                },
            ));
        }
        if !root_elements.is_empty() {
            root_bounds.add_margin(ELEMENT_MARGIN);
        }
    }

    // Process boundaries recursively
    let root_boundaries: Vec<_> = db
        .get_boundaries()
        .iter()
        .filter(|b| b.parent_boundary.is_empty())
        .collect();

    for boundary in root_boundaries {
        process_boundary(
            boundary,
            db,
            &elements_by_boundary,
            &mut root_bounds,
            &mut element_positions,
            &mut boundary_bounds,
        );
    }

    Layout {
        element_positions,
        boundary_bounds,
        total_width: root_bounds.stop_x,
        total_height: root_bounds.stop_y,
    }
}

/// Process a boundary and its contents recursively
fn process_boundary(
    boundary: &C4Boundary,
    db: &C4Db,
    elements_by_boundary: &HashMap<String, Vec<&C4Element>>,
    parent_bounds: &mut Bounds,
    element_positions: &mut Vec<(C4Element, Position)>,
    boundary_bounds: &mut HashMap<String, BoundaryBounds>,
) {
    // Start position for this boundary
    let boundary_x = parent_bounds.next_x;
    let boundary_y = if parent_bounds.row_count > 0 {
        parent_bounds.stop_y + ELEMENT_MARGIN
    } else {
        parent_bounds.next_y
    };

    // Create bounds for content inside this boundary
    let mut content_bounds = Bounds::new(
        boundary_x + BOUNDARY_PADDING,
        boundary_y + BOUNDARY_PADDING + 30.0, // Extra space for label
    );

    // Process elements in this boundary
    if let Some(elements) = elements_by_boundary.get(&boundary.alias) {
        for element in elements {
            let height = element_height(&element.shape_type);
            let (x, y) = content_bounds.insert(ELEMENT_WIDTH, height);
            element_positions.push((
                (*element).clone(),
                Position {
                    x,
                    y,
                    width: ELEMENT_WIDTH,
                    height,
                },
            ));
        }
    }

    // Process nested boundaries
    let nested_boundaries: Vec<_> = db
        .get_boundaries()
        .iter()
        .filter(|b| b.parent_boundary == boundary.alias)
        .collect();

    for nested in nested_boundaries {
        process_boundary(
            nested,
            db,
            elements_by_boundary,
            &mut content_bounds,
            element_positions,
            boundary_bounds,
        );
    }

    // Calculate boundary dimensions
    let content_width = content_bounds.width().max(ELEMENT_WIDTH);
    let content_height = content_bounds.height().max(ELEMENT_HEIGHT / 2.0);

    let boundary_width = content_width + BOUNDARY_PADDING * 2.0;
    let boundary_height = content_height + BOUNDARY_PADDING * 2.0 + 30.0; // Label space

    // Store boundary bounds
    boundary_bounds.insert(
        boundary.alias.clone(),
        BoundaryBounds {
            x: boundary_x,
            y: boundary_y,
            width: boundary_width,
            height: boundary_height,
            label: boundary.label.clone(),
        },
    );

    // Update parent bounds
    parent_bounds.stop_x = parent_bounds.stop_x.max(boundary_x + boundary_width);
    parent_bounds.stop_y = parent_bounds.stop_y.max(boundary_y + boundary_height);
    parent_bounds.next_y = parent_bounds.stop_y + ELEMENT_MARGIN;
    parent_bounds.row_count = 0; // Reset row for next boundary
}

/// Calculate the overall bounds of the diagram
fn element_height(shape_type: &C4ShapeType) -> f64 {
    match shape_type {
        C4ShapeType::Person | C4ShapeType::PersonExt => PERSON_HEIGHT,
        _ => ELEMENT_HEIGHT,
    }
}

/// Render a C4 element
fn render_element(element: &C4Element, position: &Position) -> SvgElement {
    let (bg_color, stroke_color, text_color) = element_colors(&element.shape_type);
    let mut children = Vec::new();

    // Create the shape based on type (using mermaid's approach)
    match element.shape_type {
        C4ShapeType::SystemDb
        | C4ShapeType::SystemDbExt
        | C4ShapeType::ContainerDb
        | C4ShapeType::ContainerDbExt
        | C4ShapeType::ComponentDb
        | C4ShapeType::ComponentDbExt => {
            // Database shape: cylinder using SVG path (matching mermaid)
            let half = position.width / 2.0;
            let height = position.height;

            // Main cylinder body path
            let d = format!(
                "M{},{}c0,-10 {},-10 {},-10c0,0 {},0 {},10l0,{}c0,10 -{},-10 -{},-10c0,0 -{},0 -{},-10l0,-{}",
                position.x, position.y,
                half, half,
                half, half,
                height,
                half, half,
                half, half,
                height
            );
            children.push(SvgElement::Path {
                d,
                attrs: Attrs::new()
                    .with_fill(bg_color)
                    .with_stroke(stroke_color)
                    .with_stroke_width(0.5)
                    .with_class("c4-db"),
            });

            // Top ellipse highlight path
            let d2 = format!(
                "M{},{}c0,10 {},10 {},10c0,0 {},0 {},-10",
                position.x, position.y, half, half, half, half
            );
            children.push(SvgElement::Path {
                d: d2,
                attrs: Attrs::new()
                    .with_fill("none")
                    .with_stroke(stroke_color)
                    .with_stroke_width(0.5)
                    .with_class("c4-db-top"),
            });
        }
        C4ShapeType::SystemQueue
        | C4ShapeType::SystemQueueExt
        | C4ShapeType::ContainerQueue
        | C4ShapeType::ContainerQueueExt
        | C4ShapeType::ComponentQueue
        | C4ShapeType::ComponentQueueExt => {
            // Queue shape: using SVG path (matching mermaid)
            let width = position.width;
            let half_h = position.height / 2.0;

            // Main queue body path
            let d =
                format!(
                "M{},{}l{},0c5,0 5,{} 5,{}c0,0 0,{} -5,{}l-{},0c-5,0 -5,-{} -5,-{}c0,0 0,-{} 5,-{}",
                position.x, position.y,
                width,
                half_h, half_h,
                half_h, half_h,
                width,
                half_h, half_h,
                half_h, half_h
            );
            children.push(SvgElement::Path {
                d,
                attrs: Attrs::new()
                    .with_fill(bg_color)
                    .with_stroke(stroke_color)
                    .with_stroke_width(0.5)
                    .with_class("c4-queue"),
            });

            // Right side curve path
            let d2 = format!(
                "M{},{}c-5,0 -5,{} -5,{}c0,{} 5,{} 5,{}",
                position.x + width,
                position.y,
                half_h,
                half_h,
                half_h,
                half_h,
                half_h
            );
            children.push(SvgElement::Path {
                d: d2,
                attrs: Attrs::new()
                    .with_fill("none")
                    .with_stroke(stroke_color)
                    .with_stroke_width(0.5)
                    .with_class("c4-queue-right"),
            });
        }
        _ => {
            // Standard rectangle for persons, systems, containers, components
            // Using rx/ry = 2.5 to match mermaid
            children.push(SvgElement::Rect {
                x: position.x,
                y: position.y,
                width: position.width,
                height: position.height,
                rx: Some(2.5),
                ry: Some(2.5),
                attrs: Attrs::new()
                    .with_fill(bg_color)
                    .with_stroke(stroke_color)
                    .with_stroke_width(0.5)
                    .with_class("c4-element"),
            });
        }
    }

    // Calculate text starting position
    let text_x = position.x + position.width / 2.0;
    let mut text_y = position.y + 18.0;

    // For person types, add person icon image (matching mermaid's approach)
    let is_person = matches!(
        element.shape_type,
        C4ShapeType::Person | C4ShapeType::PersonExt
    );
    if is_person {
        // Use base64 PNG image for person icon (matching mermaid)
        let icon_x = text_x - 24.0; // Center the 48x48 icon
        let icon_y = text_y + 16.0; // After type label

        // Select icon based on person type (internal vs external)
        let img_data = if matches!(element.shape_type, C4ShapeType::PersonExt) {
            // External person (gray)
            "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAADAAAAAwCAIAAADYYG7QAAAB6ElEQVR4Xu2YLY+EMBCG9+dWr0aj0Wg0Go1Go0+j8Xdv2uTCvv1gpt0ebHKPuhDaeW4605Z9mJvx4AdXUyTUdd08z+u6flmWZRnHsWkafk9DptAwDPu+f0eAYtu2PEaGWuj5fCIZrBAC2eLBAnRCsEkkxmeaJp7iDJ2QMDdHsLg8SxKFEJaAo8lAXnmuOFIhTMpxxKATebo4UiFknuNo4OniSIXQyRxEA3YsnjGCVEjVXD7yLUAqxBGUyPv/Y4W2beMgGuS7kVQIBycH0fD+oi5pezQETxdHKmQKGk1eQEYldK+jw5GxPfZ9z7Mk0Qnhf1W1m3w//EUn5BDmSZsbR44QQLBEqrBHqOrmSKaQAxdnLArCrxZcM7A7ZKs4ioRq8LFC+NpC3WCBJsvpVw5edm9iEXFuyNfxXAgSwfrFQ1c0iNda8AdejvUgnktOtJQQxmcfFzGglc5WVCj7oDgFqU18boeFSs52CUh8LE8BIVQDT1ABrB0HtgSEYlX5doJnCwv9TXocKCaKbnwhdDKPq4lf3SwU3HLq4V/+WYhHVMa/3b4IlfyikAduCkcBc7mQ3/z/Qq/cTuikhkzB12Ae/mcJC9U+Vo8Ej1gWAtgbeGgFsAMHr50BIWOLCbezvhpBFUdY6EJuJ/QDW0XoMX60zZ0AAAAASUVORK5CYII="
        } else {
            // Internal person (blue)
            "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAADAAAAAwCAIAAADYYG7QAAACD0lEQVR4Xu2YoU4EMRCGT+4j8Ai8AhaH4QHgAUjQuFMECUgMIUgwJAgMhgQsAYUiJCiQIBBY+EITsjfTdme6V24v4c8vyGbb+ZjOtN0bNcvjQXmkH83WvYBWto6PLm6v7p7uH1/w2fXD+PBycX1Pv2l3IdDm/vn7x+dXQiAubRzoURa7gRZWd0iGRIiJbOnhnfYBQZNJjNbuyY2eJG8fkDE3bbG4ep6MHUAsgYxmE3nVs6VsBWJSGccsOlFPmLIViMzLOB7pCVO2AtHJMohH7Fh6zqitQK7m0rJvAVYgGcEpe//PLdDz65sM4pF9N7ICcXDKIB5Nv6j7tD0NoSdM2QrU9Gg0ewE1LqBhHR3BBdvj2vapnidjHxD/q6vd7Pvhr31AwcY8eXMTXAKECZZJFXuEq27aLgQK5uLMohCenGGuGewOxSjBvYBqeG6B+Nqiblggdjnc+ZXDy+FNFpFzw76O3UBAROuXh6FoiAcf5g9eTvUgzy0nWg6I8cXHRUpg5bOVBCo+KDpFajOf23GgPme7RSQ+lacIENUgJ6gg1k6HjgOlqnLqip4tEuhv0hNEMXUD0clyXE3p6pZA0S2nnvTlXwLJEZWlb7cTQH1+USgTN4VhAenm/wea1OCAOmqo6fE1WCb9WSKBah+rbUWPWAmE2Rvk0ApiB45eOyNAzU8xcTvj8KvkKEoOaIYeHNA3ZuygAvFMUO0AAAAASUVORK5CYII="
        };

        children.push(SvgElement::Raw {
            content: format!(
                r#"<image x="{}" y="{}" width="48" height="48" href="{}"/>"#,
                icon_x, icon_y, img_data
            ),
        });
    }

    // Type label (<<system>>, <<container>>, etc.) - matching mermaid
    let type_label = shape_type_label(&element.shape_type);
    children.push(SvgElement::Text {
        x: text_x,
        y: text_y,
        content: format!("<<{}>>", type_label),
        attrs: Attrs::new()
            .with_fill(text_color)
            .with_attr("text-anchor", "middle")
            .with_attr("font-size", "11")
            .with_attr("font-style", "italic")
            .with_class("c4-type"),
    });
    text_y += 16.0;

    // Advance text_y past the person icon if present (48px image + 8px padding)
    if is_person {
        text_y += 56.0;
    }

    // Element label (name)
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
    text_y += 16.0;

    // Technology (if present)
    if !element.technology.is_empty() {
        children.push(SvgElement::Text {
            x: text_x,
            y: text_y,
            content: format!("[{}]", element.technology),
            attrs: Attrs::new()
                .with_fill(text_color)
                .with_attr("text-anchor", "middle")
                .with_attr("font-size", "11")
                .with_attr("font-style", "italic")
                .with_class("c4-technology"),
        });
        text_y += 14.0;
    }

    // Description (wrapped)
    if !element.description.is_empty() {
        text_y += 6.0;
        let wrapped = wrap_text(&element.description, 55);
        for line in wrapped {
            children.push(SvgElement::Text {
                x: text_x,
                y: text_y,
                content: line,
                attrs: Attrs::new()
                    .with_fill(text_color)
                    .with_attr("text-anchor", "middle")
                    .with_attr("font-size", "11")
                    .with_class("c4-description"),
            });
            text_y += 14.0;
        }
    }

    SvgElement::Group {
        children,
        attrs: Attrs::new()
            .with_class("c4-element-group")
            .with_id(&element.alias),
    }
}

/// Get the type label for a shape
fn shape_type_label(shape_type: &C4ShapeType) -> &'static str {
    match shape_type {
        C4ShapeType::Person => "person",
        C4ShapeType::PersonExt => "external_person",
        C4ShapeType::System => "system",
        C4ShapeType::SystemExt => "external_system",
        C4ShapeType::SystemDb => "system_db",
        C4ShapeType::SystemDbExt => "external_system_db",
        C4ShapeType::SystemQueue => "system_queue",
        C4ShapeType::SystemQueueExt => "external_system_queue",
        C4ShapeType::Container => "container",
        C4ShapeType::ContainerExt => "external_container",
        C4ShapeType::ContainerDb => "container_db",
        C4ShapeType::ContainerDbExt => "external_container_db",
        C4ShapeType::ContainerQueue => "container_queue",
        C4ShapeType::ContainerQueueExt => "external_container_queue",
        C4ShapeType::Component => "component",
        C4ShapeType::ComponentExt => "external_component",
        C4ShapeType::ComponentDb => "component_db",
        C4ShapeType::ComponentDbExt => "external_component_db",
        C4ShapeType::ComponentQueue => "component_queue",
        C4ShapeType::ComponentQueueExt => "external_component_queue",
    }
}

/// Render the diagram title
fn render_title(title: &str, width: f64) -> SvgElement {
    SvgElement::Text {
        x: width / 2.0,
        y: 25.0,
        content: title.to_string(),
        attrs: Attrs::new()
            .with_fill(COLOR_TEXT_DARK)
            .with_attr("text-anchor", "middle")
            .with_attr("font-size", "16")
            .with_attr("font-weight", "bold")
            .with_class("c4-title"),
    }
}

/// Render a boundary
fn render_boundary(boundary: &C4Boundary, bounds: &BoundaryBounds) -> SvgElement {
    // Deployment nodes use solid borders, other boundaries use dashed
    let is_deployment = boundary.boundary_type.starts_with("deployment");

    let mut rect_attrs = Attrs::new()
        .with_fill("none")
        .with_stroke(COLOR_BOUNDARY)
        .with_stroke_width(1.0)
        .with_class("c4-boundary");

    if !is_deployment {
        rect_attrs = rect_attrs.with_attr("stroke-dasharray", "7,7");
    }

    let mut children = vec![
        // Boundary rectangle
        SvgElement::Rect {
            x: bounds.x,
            y: bounds.y,
            width: bounds.width,
            height: bounds.height,
            rx: Some(2.5),
            ry: Some(2.5),
            attrs: rect_attrs,
        },
        // Boundary label
        SvgElement::Text {
            x: bounds.x + 10.0,
            y: bounds.y + 20.0,
            content: bounds.label.clone(),
            attrs: Attrs::new()
                .with_fill(COLOR_BOUNDARY)
                .with_attr("font-weight", "bold")
                .with_attr("font-size", "14")
                .with_class("c4-boundary-label"),
        },
    ];

    // Add boundary type label if not deployment
    if !boundary.boundary_type.is_empty() && !is_deployment {
        let type_label = format!("[{}]", boundary.boundary_type.to_uppercase());
        children.push(SvgElement::Text {
            x: bounds.x + 10.0,
            y: bounds.y + 35.0,
            content: type_label,
            attrs: Attrs::new()
                .with_fill(COLOR_BOUNDARY)
                .with_attr("font-size", "12")
                .with_class("c4-boundary-type"),
        });
    }

    SvgElement::Group {
        children,
        attrs: Attrs::new()
            .with_class("c4-boundary-group")
            .with_id(&boundary.alias),
    }
}

/// Render a relationship
fn render_relationship(
    rel: &C4Relationship,
    layout: &Layout,
    rel_index: usize,
) -> Option<SvgElement> {
    // Find source and target positions
    let source_pos = layout
        .element_positions
        .iter()
        .find(|(e, _)| e.alias == rel.from)
        .map(|(_, p)| p)?;

    let target_pos = layout
        .element_positions
        .iter()
        .find(|(e, _)| e.alias == rel.to)
        .map(|(_, p)| p)?;

    let mut children = Vec::new();

    // Calculate intersection points (where line meets element edge)
    let (start, end) = calculate_intersection_points(source_pos, target_pos);

    // First relationship uses <line>, others use <path> with quadratic Bezier (matching mermaid)
    if rel_index == 0 {
        // Use line element for straight connection
        let mut line_attrs = Attrs::new()
            .with_stroke(COLOR_REL)
            .with_stroke_width(1.0)
            .with_attr("marker-end", "url(#c4-arrow)")
            .with_class("c4-relationship");

        // BiRel has arrows on both ends
        if rel.rel_type == "BiRel" {
            line_attrs = line_attrs.with_attr("marker-start", "url(#c4-arrow-reverse)");
        }

        children.push(SvgElement::Line {
            x1: start.0,
            y1: start.1,
            x2: end.0,
            y2: end.1,
            attrs: line_attrs,
        });
    } else {
        // Use path with quadratic Bezier curve for subsequent relationships
        let control_x = start.0 + (end.0 - start.0) / 2.0 - (end.0 - start.0) / 4.0;
        let control_y = start.1 + (end.1 - start.1) / 2.0;
        let path = format!(
            "M{},{} Q{},{} {},{}",
            start.0, start.1, control_x, control_y, end.0, end.1
        );
        let mut line_attrs = Attrs::new()
            .with_fill("none")
            .with_stroke(COLOR_REL)
            .with_stroke_width(1.0)
            .with_attr("marker-end", "url(#c4-arrow)")
            .with_class("c4-relationship");

        // BiRel has arrows on both ends
        if rel.rel_type == "BiRel" {
            line_attrs = line_attrs.with_attr("marker-start", "url(#c4-arrow-reverse)");
        }

        children.push(SvgElement::Path {
            d: path,
            attrs: line_attrs,
        });
    }

    // Add label at midpoint (without background rect to match mermaid)
    if !rel.label.is_empty() {
        let mid_x = (start.0 + end.0) / 2.0;
        let mid_y = (start.1 + end.1) / 2.0;

        children.push(SvgElement::Text {
            x: mid_x,
            y: mid_y + 4.0,
            content: rel.label.clone(),
            attrs: Attrs::new()
                .with_fill(COLOR_REL)
                .with_attr("text-anchor", "middle")
                .with_attr("font-size", "12")
                .with_class("c4-rel-label"),
        });
    }

    // Add technology label if present
    if !rel.technology.is_empty() {
        let mid_x = (start.0 + end.0) / 2.0;
        let mid_y = (start.1 + end.1) / 2.0 + 17.0;

        children.push(SvgElement::Text {
            x: mid_x,
            y: mid_y,
            content: format!("[{}]", rel.technology),
            attrs: Attrs::new()
                .with_fill(COLOR_REL)
                .with_attr("text-anchor", "middle")
                .with_attr("font-size", "12")
                .with_attr("font-style", "italic")
                .with_class("c4-rel-technology"),
        });
    }

    Some(SvgElement::Group {
        children,
        attrs: Attrs::new().with_class("c4-relationship-group"),
    })
}

/// Calculate intersection points between element centers and their edges
fn calculate_intersection_points(from: &Position, to: &Position) -> ((f64, f64), (f64, f64)) {
    // Calculate centers
    let from_cx = from.x + from.width / 2.0;
    let from_cy = from.y + from.height / 2.0;
    let to_cx = to.x + to.width / 2.0;
    let to_cy = to.y + to.height / 2.0;

    // Direction from source to target
    let dx = to_cx - from_cx;
    let dy = to_cy - from_cy;

    // Calculate start point (edge of source element)
    let start = get_rect_intersection(from, from_cx, from_cy, dx, dy);

    // Calculate end point (edge of target element) - reverse direction
    let end = get_rect_intersection(to, to_cx, to_cy, -dx, -dy);

    (start, end)
}

/// Get intersection point of a line from center going in direction (dx, dy) with rectangle edge
fn get_rect_intersection(pos: &Position, cx: f64, cy: f64, dx: f64, dy: f64) -> (f64, f64) {
    if dx.abs() < 0.001 && dy.abs() < 0.001 {
        return (cx, cy);
    }

    // Calculate intersection with each edge
    let half_w = pos.width / 2.0;
    let half_h = pos.height / 2.0;

    // Horizontal edges (top/bottom)
    if dy.abs() > 0.001 {
        let t_top = -half_h / dy;
        let t_bottom = half_h / dy;
        let t = if dy > 0.0 { t_bottom } else { t_top };

        if t > 0.0 {
            let ix = cx + dx * t;
            if ix >= pos.x && ix <= pos.x + pos.width {
                let iy = if dy > 0.0 { pos.y + pos.height } else { pos.y };
                return (ix, iy);
            }
        }
    }

    // Vertical edges (left/right)
    if dx.abs() > 0.001 {
        let t_left = -half_w / dx;
        let t_right = half_w / dx;
        let t = if dx > 0.0 { t_right } else { t_left };

        if t > 0.0 {
            let iy = cy + dy * t;
            if iy >= pos.y && iy <= pos.y + pos.height {
                let ix = if dx > 0.0 { pos.x + pos.width } else { pos.x };
                return (ix, iy);
            }
        }
    }

    // Fallback to center
    (cx, cy)
}

/// Get colors for an element type (background, stroke, text)
fn element_colors(shape_type: &C4ShapeType) -> (&'static str, &'static str, &'static str) {
    // Stroke colors are slightly darker versions of bg (matching mermaid)
    match shape_type {
        C4ShapeType::Person => (COLOR_PERSON, "#073b6f", COLOR_TEXT_LIGHT),
        C4ShapeType::PersonExt => (COLOR_PERSON_EXT, "#4e5b63", COLOR_TEXT_LIGHT),
        C4ShapeType::System | C4ShapeType::SystemDb | C4ShapeType::SystemQueue => {
            (COLOR_SYSTEM, "#0e5ea8", COLOR_TEXT_LIGHT)
        }
        C4ShapeType::SystemExt | C4ShapeType::SystemDbExt | C4ShapeType::SystemQueueExt => {
            (COLOR_SYSTEM_EXT, "#8a8a8a", COLOR_TEXT_LIGHT)
        }
        C4ShapeType::Container | C4ShapeType::ContainerDb | C4ShapeType::ContainerQueue => {
            (COLOR_CONTAINER, "#3c7fc0", COLOR_TEXT_LIGHT)
        }
        C4ShapeType::ContainerExt
        | C4ShapeType::ContainerDbExt
        | C4ShapeType::ContainerQueueExt => (COLOR_CONTAINER_EXT, "#8a8a8a", COLOR_TEXT_LIGHT),
        C4ShapeType::Component | C4ShapeType::ComponentDb | C4ShapeType::ComponentQueue => {
            (COLOR_COMPONENT, "#6fa8dc", COLOR_TEXT_DARK)
        }
        C4ShapeType::ComponentExt
        | C4ShapeType::ComponentDbExt
        | C4ShapeType::ComponentQueueExt => (COLOR_COMPONENT_EXT, "#a6a6a6", COLOR_TEXT_DARK),
    }
}

/// Wrap text to fit within a character limit
fn wrap_text(text: &str, max_chars: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current_line = String::new();

    for word in text.split_whitespace() {
        if current_line.is_empty() {
            current_line = word.to_string();
        } else if current_line.len() + word.len() < max_chars {
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

/// Create arrow markers and symbol definitions for C4 diagrams (matching mermaid.js)
fn create_c4_markers() -> Vec<SvgElement> {
    vec![
        // Symbol definitions (matching mermaid.js C4 icons)
        SvgElement::Raw {
            content: r##"<symbol id="computer" width="24" height="24"><path transform="scale(.5)" d="M2 2v13h20v-13h-20zm18 11h-16v-9h16v9zm-10.228 6l.466-1h3.524l.467 1h-4.457zm14.228 3h-24l2-6h2.104l-1.33 4h18.45l-1.297-4h2.073l2 6zm-5-10h-14v-7h14v7z"/></symbol>"##.to_string(),
        },
        SvgElement::Raw {
            content: r##"<symbol id="database" fill-rule="evenodd" clip-rule="evenodd"><path transform="scale(.5)" d="M12.258.001l.256.004.255.005.253.008.251.01.249.012.247.015.246.016.242.019.241.02.239.023.236.024.233.027.231.028.229.031.225.032.223.034.22.036.217.038.214.04.211.041.208.043.205.045.201.046.198.048.194.05.191.051.187.053.183.054.18.056.175.057.172.059.168.06.163.061.16.063.155.064.15.066.074.033.073.033.071.034.07.034.069.035.068.035.067.035.066.035.064.036.064.036.062.036.06.036.06.037.058.037.058.037.055.038.055.038.053.038.052.038.051.039.05.039.048.039.047.039.045.04.044.04.043.04.041.04.04.041.039.041.037.041.036.041.034.041.033.042.032.042.03.042.029.042.027.042.026.043.024.043.023.043.021.043.02.043.018.044.017.043.015.044.013.044.012.044.011.045.009.044.007.045.006.045.004.045.002.045.001.045v17l-.001.045-.002.045-.004.045-.006.045-.007.045-.009.044-.011.045-.012.044-.013.044-.015.044-.017.043-.018.044-.02.043-.021.043-.023.043-.024.043-.026.043-.027.042-.029.042-.03.042-.032.042-.033.042-.034.041-.036.041-.037.041-.039.041-.04.041-.041.04-.043.04-.044.04-.045.04-.047.039-.048.039-.05.039-.051.039-.052.038-.053.038-.055.038-.055.038-.058.037-.058.037-.06.037-.06.036-.062.036-.064.036-.064.036-.066.035-.067.035-.068.035-.069.035-.07.034-.071.034-.073.033-.074.033-.15.066-.155.064-.16.063-.163.061-.168.06-.172.059-.175.057-.18.056-.183.054-.187.053-.191.051-.194.05-.198.048-.201.046-.205.045-.208.043-.211.041-.214.04-.217.038-.22.036-.223.034-.225.032-.229.031-.231.028-.233.027-.236.024-.239.023-.241.02-.242.019-.246.016-.247.015-.249.012-.251.01-.253.008-.255.005-.256.004-.258.001-.258-.001-.256-.004-.255-.005-.253-.008-.251-.01-.249-.012-.247-.015-.245-.016-.243-.019-.241-.02-.238-.023-.236-.024-.234-.027-.231-.028-.228-.031-.226-.032-.223-.034-.22-.036-.217-.038-.214-.04-.211-.041-.208-.043-.204-.045-.201-.046-.198-.048-.195-.05-.19-.051-.187-.053-.184-.054-.179-.056-.176-.057-.172-.059-.167-.06-.164-.061-.159-.063-.155-.064-.151-.066-.074-.033-.072-.033-.072-.034-.07-.034-.069-.035-.068-.035-.067-.035-.066-.035-.064-.036-.063-.036-.062-.036-.061-.036-.06-.037-.058-.037-.057-.037-.056-.038-.055-.038-.053-.038-.052-.038-.051-.039-.049-.039-.049-.039-.046-.039-.046-.04-.044-.04-.043-.04-.041-.04-.04-.041-.039-.041-.037-.041-.036-.041-.034-.041-.033-.042-.032-.042-.03-.042-.029-.042-.027-.042-.026-.043-.024-.043-.023-.043-.021-.043-.02-.043-.018-.044-.017-.043-.015-.044-.013-.044-.012-.044-.011-.045-.009-.044-.007-.045-.006-.045-.004-.045-.002-.045-.001-.045v-17l.001-.045.002-.045.004-.045.006-.045.007-.045.009-.044.011-.045.012-.044.013-.044.015-.044.017-.043.018-.044.02-.043.021-.043.023-.043.024-.043.026-.043.027-.042.029-.042.03-.042.032-.042.033-.042.034-.041.036-.041.037-.041.039-.041.04-.041.041-.04.043-.04.044-.04.046-.04.046-.039.049-.039.049-.039.051-.039.052-.038.053-.038.055-.038.056-.038.057-.037.058-.037.06-.037.061-.036.062-.036.063-.036.064-.036.066-.035.067-.035.068-.035.069-.035.07-.034.072-.034.072-.033.074-.033.151-.066.155-.064.159-.063.164-.061.167-.06.172-.059.176-.057.179-.056.184-.054.187-.053.19-.051.195-.05.198-.048.201-.046.204-.045.208-.043.211-.041.214-.04.217-.038.22-.036.223-.034.226-.032.228-.031.231-.028.234-.027.236-.024.238-.023.241-.02.243-.019.245-.016.247-.015.249-.012.251-.01.253-.008.255-.005.256-.004.258-.001.258.001z"/></symbol>"##.to_string(),
        },
        SvgElement::Raw {
            content: r##"<symbol id="clock" width="24" height="24"><path transform="scale(.5)" d="M12 2c5.514 0 10 4.486 10 10s-4.486 10-10 10-10-4.486-10-10 4.486-10 10-10zm0-2c-6.627 0-12 5.373-12 12s5.373 12 12 12 12-5.373 12-12-5.373-12-12-12zm5.848 12.459c.202.038.202.333.001.372-1.907.361-6.045 1.111-6.547 1.111-.719 0-1.301-.582-1.301-1.301 0-.512.77-5.447 1.125-7.445.034-.192.312-.181.343.014l.985 6.238 5.394 1.011z"/></symbol>"##.to_string(),
        },
        // Forward arrow (arrowhead) - matching mermaid.js
        SvgElement::Raw {
            content: r##"<marker id="c4-arrow" refX="9" refY="5" markerUnits="userSpaceOnUse" markerWidth="12" markerHeight="12" orient="auto">
    <path d="M 0 0 L 10 5 L 0 10 z"/>
</marker>"##
                .to_string(),
        },
        // Reverse arrow (arrowend) for BiRel
        SvgElement::Raw {
            content: r##"<marker id="c4-arrow-reverse" refX="1" refY="5" markerUnits="userSpaceOnUse" markerWidth="12" markerHeight="12" orient="auto">
    <path d="M 10 0 L 0 5 L 10 10 z"/>
</marker>"##
                .to_string(),
        },
        // Crosshead marker (matching mermaid.js)
        SvgElement::Raw {
            content: r##"<marker id="c4-crosshead" markerWidth="15" markerHeight="8" orient="auto" refX="16" refY="4">
    <path fill="black" stroke="#000000" stroke-width="1px" d="M 9,2 V 6 L16,4 Z" style="stroke-dasharray: 0, 0;"/>
    <path fill="none" stroke="#000000" stroke-width="1px" d="M 0,1 L 6,7 M 6,1 L 0,7" style="stroke-dasharray: 0, 0;"/>
</marker>"##
                .to_string(),
        },
        // Filled-head marker (matching mermaid.js)
        SvgElement::Raw {
            content: r##"<marker id="c4-filled-head" refX="18" refY="7" markerWidth="20" markerHeight="28" orient="auto">
    <path d="M 18,7 L9,13 L14,7 L9,1 Z"/>
</marker>"##
                .to_string(),
        },
    ]
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

.c4-db-top, .c4-db-body, .c4-db-bottom {{
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
        let result = wrap_text(
            "This is a very long description that needs to be wrapped",
            20,
        );
        assert!(result.len() > 1);
    }

    #[test]
    fn test_element_colors() {
        let (bg, _stroke, text) = element_colors(&C4ShapeType::Person);
        assert_eq!(bg, COLOR_PERSON);
        assert_eq!(text, COLOR_TEXT_LIGHT);

        let (bg, _stroke, text) = element_colors(&C4ShapeType::Component);
        assert_eq!(bg, COLOR_COMPONENT);
        assert_eq!(text, COLOR_TEXT_DARK);
    }

    #[test]
    fn test_bounds_grid_layout() {
        let mut bounds = Bounds::new(10.0, 10.0);

        // Insert 5 elements - should wrap to new row after 4
        let (x1, y1) = bounds.insert(100.0, 50.0);
        let (x2, _y2) = bounds.insert(100.0, 50.0);
        let (_x3, _y3) = bounds.insert(100.0, 50.0);
        let (_x4, _y4) = bounds.insert(100.0, 50.0);
        let (x5, y5) = bounds.insert(100.0, 50.0);

        // First element at start
        assert_eq!(x1, 10.0);
        assert_eq!(y1, 10.0);

        // Second element next to first
        assert!(x2 > x1);

        // Fifth element should wrap to new row
        assert_eq!(x5, 10.0);
        assert!(y5 > y1);
    }
}
