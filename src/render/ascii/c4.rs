//! ASCII renderer for C4 architecture diagrams.
//!
//! Renders C4 elements (Person, System, Container, Component) as labeled
//! boxes with technology and description info. Boundaries are shown as
//! indented sections. Relationships are listed as connections.

use crate::diagrams::c4::{C4Db, C4ShapeType};
use crate::error::Result;

/// Render a C4 diagram as character art.
pub fn render_c4_ascii(db: &C4Db) -> Result<String> {
    let elements = db.get_elements();
    let boundaries = db.get_boundaries();
    let relationships = db.get_relationships();

    if elements.is_empty() && boundaries.is_empty() {
        let title = db.get_title().map(|t| t.to_string());
        if let Some(title) = title {
            return Ok(format!("{}\n\n(empty C4 diagram)\n", title));
        }
        return Ok("(empty C4 diagram)\n".to_string());
    }

    let mut lines: Vec<String> = Vec::new();

    // Title
    if let Some(title) = db.get_title() {
        lines.push(title.to_string());
        lines.push("═".repeat(title.chars().count().max(40)));
    }

    // Render boundaries as sections
    for boundary in boundaries {
        lines.push(String::new());
        let b_type = if !boundary.boundary_type.is_empty() {
            format!(" [{}]", boundary.boundary_type)
        } else {
            String::new()
        };
        lines.push(format!("┌─ {}{} ─┐", boundary.label, b_type));

        // Find elements in this boundary
        for element in elements
            .iter()
            .filter(|e| e.parent_boundary == boundary.alias)
        {
            render_element(&mut lines, element, "  ");
        }

        lines.push("└────────────┘".to_string());
    }

    // Render top-level elements (no boundary)
    for element in elements.iter().filter(|e| e.parent_boundary.is_empty()) {
        render_element(&mut lines, element, "");
    }

    // Render relationships
    if !relationships.is_empty() {
        lines.push(String::new());
        lines.push("  Relationships:".to_string());
        for rel in relationships {
            let tech = if !rel.technology.is_empty() {
                format!(" [{}]", rel.technology)
            } else {
                String::new()
            };
            let label = if !rel.label.is_empty() {
                format!(" \"{}\"", rel.label)
            } else {
                String::new()
            };
            lines.push(format!("    {} →{}{} {}", rel.from, label, tech, rel.to));
        }
    }

    lines.push(String::new());
    Ok(lines.join("\n"))
}

fn render_element(lines: &mut Vec<String>, element: &crate::diagrams::c4::C4Element, indent: &str) {
    let type_label = match element.shape_type {
        C4ShapeType::Person | C4ShapeType::PersonExt => "[Person]",
        C4ShapeType::System
        | C4ShapeType::SystemExt
        | C4ShapeType::SystemDb
        | C4ShapeType::SystemDbExt
        | C4ShapeType::SystemQueue
        | C4ShapeType::SystemQueueExt => "[System]",
        C4ShapeType::Container
        | C4ShapeType::ContainerExt
        | C4ShapeType::ContainerDb
        | C4ShapeType::ContainerDbExt
        | C4ShapeType::ContainerQueue
        | C4ShapeType::ContainerQueueExt => "[Container]",
        C4ShapeType::Component
        | C4ShapeType::ComponentExt
        | C4ShapeType::ComponentDb
        | C4ShapeType::ComponentDbExt
        | C4ShapeType::ComponentQueue
        | C4ShapeType::ComponentQueueExt => "[Component]",
    };

    let is_ext = matches!(
        element.shape_type,
        C4ShapeType::PersonExt
            | C4ShapeType::SystemExt
            | C4ShapeType::SystemDbExt
            | C4ShapeType::SystemQueueExt
            | C4ShapeType::ContainerExt
            | C4ShapeType::ContainerDbExt
            | C4ShapeType::ContainerQueueExt
            | C4ShapeType::ComponentExt
            | C4ShapeType::ComponentDbExt
            | C4ShapeType::ComponentQueueExt
    );

    let ext_marker = if is_ext { " [ext]" } else { "" };

    let tech = if !element.technology.is_empty() {
        format!(" ({})", element.technology)
    } else {
        String::new()
    };

    lines.push(format!(
        "{}  {} {}{}{}",
        indent, type_label, element.label, tech, ext_marker
    ));

    if !element.description.is_empty() {
        lines.push(format!("{}      {}", indent, element.description));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_c4() {
        let db = C4Db::new();
        let output = render_c4_ascii(&db).unwrap();
        assert!(output.contains("empty C4"));
    }

    #[test]
    fn gallery_c4_renders() {
        let input = std::fs::read_to_string("docs/sources/c4.mmd").unwrap();
        let diagram = crate::parse(&input).unwrap();
        let db = match diagram {
            crate::diagrams::Diagram::C4(db) => db,
            _ => panic!("Expected C4"),
        };
        let output = render_c4_ascii(&db).unwrap();
        assert!(!output.trim().is_empty(), "Output should not be empty");
    }

    #[test]
    fn relationships_appear() {
        let input = std::fs::read_to_string("docs/sources/c4.mmd").unwrap();
        let diagram = crate::parse(&input).unwrap();
        let db = match diagram {
            crate::diagrams::Diagram::C4(db) => db,
            _ => panic!("Expected C4"),
        };
        let output = render_c4_ascii(&db).unwrap();
        // C4 diagrams typically have relationships shown with arrows
        if !db.get_relationships().is_empty() {
            assert!(
                output.contains('→'),
                "Should have relationship arrows\nOutput:\n{}",
                output
            );
        }
    }
}
