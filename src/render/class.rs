//! Class diagram renderer

use std::collections::HashMap;

use crate::diagrams::class::{ClassDb, ClassNode, LineType};
use crate::error::Result;
use crate::render::svg::{Attrs, RenderConfig, SvgDocument, SvgElement};

/// Render a class diagram to SVG
pub fn render_class(db: &ClassDb, config: &RenderConfig) -> Result<String> {
    let mut doc = SvgDocument::new();

    // Layout constants
    let class_width = 180.0;
    let class_min_height = 60.0;
    let class_padding = 10.0;
    let member_height = 18.0;
    let header_height = 30.0;
    let class_spacing_x = 60.0;
    let class_spacing_y = 80.0;
    let margin = 50.0;

    let classes: Vec<_> = db.classes.values().collect();

    if classes.is_empty() {
        // Empty diagram
        doc.set_size(400.0, 200.0);
        return Ok(doc.to_string());
    }

    // Calculate class heights and positions
    let mut class_heights: HashMap<String, f64> = HashMap::new();
    let mut class_positions: HashMap<String, (f64, f64)> = HashMap::new();

    // Calculate height for each class
    for class in &classes {
        let num_members = class.members.len() + class.methods.len();
        let content_height = (num_members as f64) * member_height;
        let annotations_height = if class.annotations.is_empty() {
            0.0
        } else {
            (class.annotations.len() as f64) * member_height
        };
        let total_height =
            header_height + annotations_height + content_height + class_padding * 2.0;
        let height = total_height.max(class_min_height);
        class_heights.insert(class.id.clone(), height);
    }

    // Build hierarchical layout based on relationships
    // Step 1: Build parent-child relationships for ALL relation types
    // Inheritance (type1/type2 == 1) takes priority, but other relations also affect layout
    let mut children_of: HashMap<String, Vec<String>> = HashMap::new();
    let mut parent_of: HashMap<String, String> = HashMap::new();

    // First pass: handle inheritance relationships (these define primary hierarchy)
    for relation in &db.relations {
        // type1 and type2: 1 = inheritance arrow (the arrow points toward parent)
        // In "Animal <|-- Dog", Animal is parent, Dog is child
        if relation.relation.type1 == 1 {
            // id1 is parent, id2 is child
            children_of
                .entry(relation.id1.clone())
                .or_default()
                .push(relation.id2.clone());
            parent_of.insert(relation.id2.clone(), relation.id1.clone());
        } else if relation.relation.type2 == 1 {
            // id2 is parent, id1 is child
            children_of
                .entry(relation.id2.clone())
                .or_default()
                .push(relation.id1.clone());
            parent_of.insert(relation.id1.clone(), relation.id2.clone());
        }
    }

    // Second pass: handle composition/aggregation (these also affect hierarchy)
    // type 0 = aggregation, type 2 = composition - the containing class is "parent"
    for relation in &db.relations {
        let is_composition_or_aggregation = |t: i32| t == 0 || t == 2;

        if is_composition_or_aggregation(relation.relation.type1)
            && !parent_of.contains_key(&relation.id2)
        {
            // id1 has the composition marker, so id1 contains id2
            // id2 should be below id1
            children_of
                .entry(relation.id1.clone())
                .or_default()
                .push(relation.id2.clone());
            parent_of.insert(relation.id2.clone(), relation.id1.clone());
        } else if is_composition_or_aggregation(relation.relation.type2)
            && !parent_of.contains_key(&relation.id1)
        {
            // id2 has the composition marker, so id2 contains id1
            children_of
                .entry(relation.id2.clone())
                .or_default()
                .push(relation.id1.clone());
            parent_of.insert(relation.id1.clone(), relation.id2.clone());
        }
    }

    // Step 2: Assign levels using BFS from root classes (those with no parents)
    let mut class_levels: HashMap<String, usize> = HashMap::new();
    let all_class_ids: Vec<_> = classes.iter().map(|c| c.id.clone()).collect();

    // Find root classes (no parent in inheritance hierarchy)
    let roots: Vec<_> = all_class_ids
        .iter()
        .filter(|id| !parent_of.contains_key(*id))
        .cloned()
        .collect();

    // BFS to assign levels
    let mut queue: std::collections::VecDeque<(String, usize)> =
        roots.iter().map(|id| (id.clone(), 0)).collect();

    while let Some((id, level)) = queue.pop_front() {
        if class_levels.contains_key(&id) {
            continue;
        }
        class_levels.insert(id.clone(), level);

        if let Some(children) = children_of.get(&id) {
            for child in children {
                if !class_levels.contains_key(child) {
                    queue.push_back((child.clone(), level + 1));
                }
            }
        }
    }

    // Assign level 0 to any remaining classes (not in inheritance hierarchy)
    for class in &classes {
        class_levels.entry(class.id.clone()).or_insert(0);
    }

    // Step 3: Group classes by level and sort for consistent ordering
    let max_level = class_levels.values().copied().max().unwrap_or(0);
    let mut levels: Vec<Vec<String>> = vec![Vec::new(); max_level + 1];
    for (id, level) in &class_levels {
        levels[*level].push(id.clone());
    }
    // Sort each level alphabetically for consistent layout
    for level in &mut levels {
        level.sort();
    }

    // Step 4: Position classes in hierarchical layout (parent at top, children below)
    // Use tree centering algorithm: parents are centered over their children
    let is_horizontal = db.direction == "LR" || db.direction == "RL";

    let mut max_width = margin;
    let mut max_height = margin;

    if is_horizontal {
        // Horizontal layout: levels go left-to-right
        let mut current_x = margin;
        for level in 0..=max_level {
            let level_classes = &levels[level];
            if level_classes.is_empty() {
                continue;
            }

            let mut current_y = margin;
            for class_id in level_classes {
                let height = class_heights
                    .get(class_id)
                    .copied()
                    .unwrap_or(class_min_height);
                class_positions.insert(class_id.clone(), (current_x, current_y));
                current_y += height + class_spacing_y;
            }

            max_height = max_height.max(current_y);
            current_x += class_width + class_spacing_x;
        }
        max_width = current_x + margin;
    } else {
        // Vertical layout with tree centering: parents centered over children
        // Step 4a: Calculate subtree widths (bottom-up from leaves)
        let mut subtree_widths: HashMap<String, f64> = HashMap::new();

        // Process levels from bottom to top
        for level in (0..=max_level).rev() {
            for class_id in &levels[level] {
                let children: Vec<_> = children_of
                    .get(class_id)
                    .map(|c| c.clone())
                    .unwrap_or_default();

                if children.is_empty() {
                    // Leaf node: width is just the class width
                    subtree_widths.insert(class_id.clone(), class_width);
                } else {
                    // Parent node: width is sum of children's subtree widths + spacing
                    let children_width: f64 =
                        children.iter().filter_map(|c| subtree_widths.get(c)).sum();
                    let spacing = (children.len().saturating_sub(1) as f64) * class_spacing_x;
                    let total_width = (children_width + spacing).max(class_width);
                    subtree_widths.insert(class_id.clone(), total_width);
                }
            }
        }

        // Step 4b: Position nodes (top-down), centering parents over children
        let mut current_y = margin;

        // First pass: calculate y positions for each level
        let mut level_y: Vec<f64> = Vec::new();
        for level in 0..=max_level {
            let level_classes = &levels[level];
            if level_classes.is_empty() {
                level_y.push(current_y);
                continue;
            }

            let level_height: f64 = level_classes
                .iter()
                .filter_map(|id| class_heights.get(id).copied())
                .fold(0.0_f64, f64::max)
                .max(class_min_height);

            level_y.push(current_y);
            current_y += level_height + class_spacing_y;
        }
        max_height = current_y + margin;

        // Second pass: calculate x positions using tree centering
        // Start with root nodes, position them, then recursively position children
        fn position_subtree(
            node_id: &str,
            start_x: f64,
            children_of: &HashMap<String, Vec<String>>,
            class_levels: &HashMap<String, usize>,
            subtree_widths: &HashMap<String, f64>,
            level_y: &[f64],
            class_positions: &mut HashMap<String, (f64, f64)>,
            class_width: f64,
            class_spacing_x: f64,
        ) {
            let level = *class_levels.get(node_id).unwrap_or(&0);
            let y = level_y.get(level).copied().unwrap_or(0.0);
            let subtree_width = subtree_widths.get(node_id).copied().unwrap_or(class_width);

            // Center the node within its subtree width
            let x = start_x + (subtree_width - class_width) / 2.0;
            class_positions.insert(node_id.to_string(), (x, y));

            // Position children
            if let Some(children) = children_of.get(node_id) {
                let mut child_x = start_x;
                for child in children {
                    let child_width = subtree_widths.get(child).copied().unwrap_or(class_width);
                    position_subtree(
                        child,
                        child_x,
                        children_of,
                        class_levels,
                        subtree_widths,
                        level_y,
                        class_positions,
                        class_width,
                        class_spacing_x,
                    );
                    child_x += child_width + class_spacing_x;
                }
            }
        }

        // Position all root nodes (level 0 nodes without parents)
        let mut root_x = margin;
        for root_id in &levels[0] {
            let root_width = subtree_widths.get(root_id).copied().unwrap_or(class_width);
            position_subtree(
                root_id,
                root_x,
                &children_of,
                &class_levels,
                &subtree_widths,
                &level_y,
                &mut class_positions,
                class_width,
                class_spacing_x,
            );
            root_x += root_width + class_spacing_x;
            max_width = max_width.max(root_x + margin);
        }

        // Position any orphan nodes (not in any tree) - place them at the end
        for level in 0..=max_level {
            for class_id in &levels[level] {
                if !class_positions.contains_key(class_id) {
                    let y = level_y.get(level).copied().unwrap_or(0.0);
                    class_positions.insert(class_id.clone(), (root_x, y));
                    root_x += class_width + class_spacing_x;
                    max_width = max_width.max(root_x + margin);
                }
            }
        }
    }

    doc.set_size(max_width, max_height);

    // Add theme styles
    if config.embed_css {
        doc.add_style(&config.theme.generate_css());
        doc.add_style(&generate_class_css());
    }

    // Add marker definitions for relations
    doc.add_defs(vec![
        create_inheritance_marker(),
        create_aggregation_marker(),
        create_composition_marker(),
        create_dependency_marker(),
        create_lollipop_marker(),
    ]);

    // Render each class
    for class in &classes {
        if let Some(&(x, y)) = class_positions.get(&class.id) {
            let height = class_heights
                .get(&class.id)
                .copied()
                .unwrap_or(class_min_height);
            let class_elem = render_class_box(
                class,
                x,
                y,
                class_width,
                height,
                class_padding,
                member_height,
                header_height,
            );
            doc.add_element(class_elem);
        }
    }

    // Render relations
    for relation in &db.relations {
        if let (Some(&(x1, y1)), Some(&(x2, y2))) = (
            class_positions.get(&relation.id1),
            class_positions.get(&relation.id2),
        ) {
            let h1 = class_heights
                .get(&relation.id1)
                .copied()
                .unwrap_or(class_min_height);
            let h2 = class_heights
                .get(&relation.id2)
                .copied()
                .unwrap_or(class_min_height);
            let relation_elem = render_relation(
                x1,
                y1,
                h1,
                x2,
                y2,
                h2,
                class_width,
                &relation.title,
                &relation.relation_title1,
                &relation.relation_title2,
                relation.relation.type1,
                relation.relation.type2,
                relation.relation.line_type,
            );
            doc.add_element(relation_elem);
        }
    }

    // Render notes
    for note in db.notes.values() {
        if let Some(&(x, y)) = class_positions.get(&note.class) {
            let note_elem = render_note(x + class_width + 20.0, y, &note.text);
            doc.add_element(note_elem);
        }
    }

    Ok(doc.to_string())
}

/// Render a class box with name, attributes, and methods
fn render_class_box(
    class: &ClassNode,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    padding: f64,
    member_height: f64,
    header_height: f64,
) -> SvgElement {
    let mut children = Vec::new();

    // Background rectangle
    children.push(SvgElement::Rect {
        x,
        y,
        width,
        height,
        rx: Some(3.0),
        ry: Some(3.0),
        attrs: Attrs::new()
            .with_fill("#ECECFF")
            .with_stroke("#333333")
            .with_stroke_width(1.0)
            .with_class("class-box"),
    });

    let mut current_y = y;

    // Annotations (<<interface>>, <<abstract>>, etc.)
    if !class.annotations.is_empty() {
        for annotation in &class.annotations {
            current_y += member_height;
            children.push(SvgElement::Text {
                x: x + width / 2.0,
                y: current_y,
                content: format!("<<{}>>", annotation),
                attrs: Attrs::new()
                    .with_attr("text-anchor", "middle")
                    .with_class("class-annotation")
                    .with_attr("font-size", "11")
                    .with_attr("font-style", "italic"),
            });
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

    children.push(SvgElement::Text {
        x: x + width / 2.0,
        y: current_y,
        content: format!("{}{}", class_label, type_suffix),
        attrs: Attrs::new()
            .with_attr("text-anchor", "middle")
            .with_class("class-name")
            .with_attr("font-size", "14")
            .with_attr("font-weight", "bold"),
    });

    current_y = y + header_height;

    // Separator line after name
    if !class.members.is_empty() || !class.methods.is_empty() {
        children.push(SvgElement::Line {
            x1: x,
            y1: current_y,
            x2: x + width,
            y2: current_y,
            attrs: Attrs::new().with_stroke("#333333").with_stroke_width(1.0),
        });
    }

    // Attributes section
    if !class.members.is_empty() {
        current_y += padding;
        for member in &class.members {
            current_y += member_height;
            let display = member.get_display_details();
            let mut text_attrs = Attrs::new()
                .with_attr("text-anchor", "start")
                .with_class("class-member")
                .with_attr("font-size", "12");

            if !display.css_style.is_empty() {
                if display.css_style.contains("underline") {
                    text_attrs = text_attrs.with_attr("text-decoration", "underline");
                }
                if display.css_style.contains("italic") {
                    text_attrs = text_attrs.with_attr("font-style", "italic");
                }
            }

            children.push(SvgElement::Text {
                x: x + padding,
                y: current_y - 4.0,
                content: display.display_text,
                attrs: text_attrs,
            });
        }
    }

    // Separator line between attributes and methods
    if !class.members.is_empty() && !class.methods.is_empty() {
        current_y += padding / 2.0;
        children.push(SvgElement::Line {
            x1: x,
            y1: current_y,
            x2: x + width,
            y2: current_y,
            attrs: Attrs::new().with_stroke("#333333").with_stroke_width(1.0),
        });
    }

    // Methods section
    if !class.methods.is_empty() {
        if class.members.is_empty() {
            current_y += padding;
        }
        for method in &class.methods {
            current_y += member_height;
            let display = method.get_display_details();
            let mut text_attrs = Attrs::new()
                .with_attr("text-anchor", "start")
                .with_class("class-method")
                .with_attr("font-size", "12");

            if !display.css_style.is_empty() {
                if display.css_style.contains("underline") {
                    text_attrs = text_attrs.with_attr("text-decoration", "underline");
                }
                if display.css_style.contains("italic") {
                    text_attrs = text_attrs.with_attr("font-style", "italic");
                }
            }

            children.push(SvgElement::Text {
                x: x + padding,
                y: current_y - 4.0,
                content: display.display_text,
                attrs: text_attrs,
            });
        }
    }

    SvgElement::Group {
        children,
        attrs: Attrs::new()
            .with_class("class-node")
            .with_id(&format!("class-{}", class.id)),
    }
}

/// Render a relation between two classes
fn render_relation(
    x1: f64,
    y1: f64,
    h1: f64,
    x2: f64,
    y2: f64,
    h2: f64,
    class_width: f64,
    label: &str,
    cardinality1: &str,
    cardinality2: &str,
    type1: i32,
    type2: i32,
    line_type: LineType,
) -> SvgElement {
    let mut children = Vec::new();

    // Calculate edge connection points based on relative positions
    let (start_x, start_y, end_x, end_y) =
        calculate_connection_points(x1, y1, h1, x2, y2, h2, class_width);

    // Determine marker based on relation type
    let marker_start = match type1 {
        0 => Some("url(#aggregation)"),
        1 => Some("url(#inheritance)"),
        2 => Some("url(#composition)"),
        4 => Some("url(#lollipop)"),
        _ => None,
    };

    let marker_end = match type2 {
        0 => Some("url(#aggregation)"),
        1 => Some("url(#inheritance)"),
        2 => Some("url(#composition)"),
        4 => Some("url(#lollipop)"),
        _ => None,
    };

    let mut line_attrs = Attrs::new()
        .with_stroke("#333333")
        .with_stroke_width(1.0)
        .with_fill("none")
        .with_class("relation-line");

    if line_type == LineType::Dotted {
        line_attrs = line_attrs.with_stroke_dasharray("5,5");
    }

    if let Some(marker) = marker_start {
        line_attrs = line_attrs.with_attr("marker-start", marker);
    }
    if let Some(marker) = marker_end {
        line_attrs = line_attrs.with_attr("marker-end", marker);
    }

    children.push(SvgElement::Line {
        x1: start_x,
        y1: start_y,
        x2: end_x,
        y2: end_y,
        attrs: line_attrs,
    });

    // Cardinality label at start (near class 1)
    if !cardinality1.is_empty() {
        let dx = end_x - start_x;
        let dy = end_y - start_y;
        let offset = 20.0; // Distance from the class edge
        let len = (dx * dx + dy * dy).sqrt();
        let offset_x = if len > 0.0 { offset * dx / len } else { 0.0 };
        let offset_y = if len > 0.0 { offset * dy / len } else { offset };

        // Offset perpendicular to the line
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
        let offset = 20.0; // Distance from the class edge
        let len = (dx * dx + dy * dy).sqrt();
        let offset_x = if len > 0.0 { offset * dx / len } else { 0.0 };
        let offset_y = if len > 0.0 { offset * dy / len } else { offset };

        // Offset perpendicular to the line
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

        children.push(SvgElement::Text {
            x: mid_x,
            y: mid_y - 5.0,
            content: label.to_string(),
            attrs: Attrs::new()
                .with_attr("text-anchor", "middle")
                .with_class("relation-label")
                .with_attr("font-size", "11"),
        });
    }

    SvgElement::Group {
        children,
        attrs: Attrs::new().with_class("relation"),
    }
}

/// Calculate connection points on class box edges
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

    // Silence unused variable warnings - centers are used for calculating dx/dy
    let _ = (center1_x, center1_y, center2_x, center2_y);

    // Determine which edges to connect based on relative positions
    let (start_x, start_y) = if dx.abs() > dy.abs() {
        // Horizontal connection
        if dx > 0.0 {
            (x1 + width, center1_y) // Right edge
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
            (x2 + width, center2_y) // Right edge
        }
    } else {
        if dy > 0.0 {
            (center2_x, y2) // Top edge
        } else {
            (center2_x, y2 + h2) // Bottom edge
        }
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

/// Create inheritance marker (hollow triangle - UML extension/inheritance)
/// Per mermaid.js extensionStart: apex at x=1, refX=18
/// The triangle points toward the parent class (line origin for marker-start)
fn create_inheritance_marker() -> SvgElement {
    SvgElement::Marker {
        id: "inheritance".to_string(),
        view_box: "0 0 20 14".to_string(),
        ref_x: 18.0, // Line connects at x=18, arrow points back toward x=1
        ref_y: 7.0,
        marker_width: 10.0,
        marker_height: 10.0,
        orient: "auto".to_string(),
        marker_units: None,
        children: vec![SvgElement::Path {
            // Path from mermaid.js extensionStart: apex at x=1, opens toward x=18
            d: "M 1 7 L 18 13 V 1 Z".to_string(),
            attrs: Attrs::new()
                .with_fill("none") // Hollow/transparent per UML convention
                .with_stroke("#333333")
                .with_stroke_width(1.0),
        }],
    }
}

/// Create aggregation marker (hollow diamond)
/// Per mermaid.js: aggregation has fill:transparent
fn create_aggregation_marker() -> SvgElement {
    SvgElement::Marker {
        id: "aggregation".to_string(),
        view_box: "0 0 20 14".to_string(),
        ref_x: 18.0, // Like inheritance, line connects at right side
        ref_y: 7.0,
        marker_width: 10.0,
        marker_height: 10.0,
        orient: "auto".to_string(),
        marker_units: None,
        children: vec![SvgElement::Path {
            // Diamond shape: apex left, points at top and bottom, flat right
            d: "M 18 7 L 9 13 L 1 7 L 9 1 Z".to_string(),
            attrs: Attrs::new()
                .with_fill("none") // Hollow per UML aggregation convention
                .with_stroke("#333333")
                .with_stroke_width(1.0),
        }],
    }
}

/// Create composition marker (filled diamond)
/// Per mermaid.js: composition has fill:#333333 (solid/filled)
fn create_composition_marker() -> SvgElement {
    SvgElement::Marker {
        id: "composition".to_string(),
        view_box: "0 0 20 14".to_string(),
        ref_x: 18.0, // Consistent with other markers
        ref_y: 7.0,
        marker_width: 10.0,
        marker_height: 10.0,
        orient: "auto".to_string(),
        marker_units: None,
        children: vec![SvgElement::Path {
            // Same diamond shape as aggregation
            d: "M 18 7 L 9 13 L 1 7 L 9 1 Z".to_string(),
            attrs: Attrs::new()
                .with_fill("#333333") // Filled per UML composition convention
                .with_stroke("#333333")
                .with_stroke_width(1.0),
        }],
    }
}

/// Create dependency marker (open arrow)
fn create_dependency_marker() -> SvgElement {
    SvgElement::Marker {
        id: "dependency".to_string(),
        view_box: "0 0 20 20".to_string(),
        ref_x: 20.0,
        ref_y: 10.0,
        marker_width: 10.0,
        marker_height: 10.0,
        orient: "auto".to_string(),
        marker_units: None,
        children: vec![SvgElement::Path {
            d: "M 0 0 L 20 10 L 0 20".to_string(),
            attrs: Attrs::new()
                .with_fill("none")
                .with_stroke("#333333")
                .with_stroke_width(1.0),
        }],
    }
}

/// Create lollipop marker (circle for interface realization)
fn create_lollipop_marker() -> SvgElement {
    SvgElement::Marker {
        id: "lollipop".to_string(),
        view_box: "0 0 20 20".to_string(),
        ref_x: 10.0,
        ref_y: 10.0,
        marker_width: 10.0,
        marker_height: 10.0,
        orient: "auto".to_string(),
        marker_units: None,
        children: vec![SvgElement::Circle {
            cx: 10.0,
            cy: 10.0,
            r: 8.0,
            attrs: Attrs::new()
                .with_fill("#FFFFFF")
                .with_stroke("#333333")
                .with_stroke_width(1.0),
        }],
    }
}

fn generate_class_css() -> String {
    r#"
.class-box {
  fill: #ECECFF;
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

        // Parse positions from SVG to verify layout
        // Animal should be at top (smallest y), Egg at bottom (largest y)
        // Duck, Fish, Zebra should be in the middle

        // For now, just verify the SVG contains all classes
        assert!(svg.contains("Animal"), "Should contain Animal");
        assert!(svg.contains("Duck"), "Should contain Duck");
        assert!(svg.contains("Fish"), "Should contain Fish");
        assert!(svg.contains("Zebra"), "Should contain Zebra");
        assert!(svg.contains("Egg"), "Should contain Egg");

        // Print SVG for manual inspection
        println!("SVG output:\n{}", svg);
    }
}
