//! ASCII renderer for timeline diagrams.
//!
//! Renders timeline periods with their events in a horizontal flow,
//! using box-drawing characters for structure.

use crate::diagrams::timeline::TimelineDb;
use crate::error::Result;

/// Render a timeline diagram as character art.
pub fn render_timeline_ascii(db: &TimelineDb) -> Result<String> {
    let tasks = db.get_tasks();
    if tasks.is_empty() {
        let title = db.get_title();
        if !title.is_empty() {
            return Ok(format!("{}\n\n(empty timeline)\n", title));
        }
        return Ok("(empty timeline)\n".to_string());
    }

    let mut lines: Vec<String> = Vec::new();

    // Title
    let title = db.get_title();
    if !title.is_empty() {
        lines.push(title.to_string());
        lines.push("─".repeat(title.chars().count().max(40)));
        lines.push(String::new());
    }

    // Group tasks by section
    let sections = db.get_sections();
    let mut current_section = String::new();

    for task in tasks {
        // Section header
        if task.section != current_section {
            current_section = task.section.clone();
            if !current_section.is_empty() {
                if lines.last().is_some_and(|l| !l.is_empty()) {
                    lines.push(String::new());
                }
                lines.push(format!("  ┌─ {} ─┐", current_section));
            }
        }

        // Period name
        lines.push(format!("  │ ◆ {}", task.task));

        // Events
        for event in &task.events {
            lines.push(format!("  │   ├─ {}", event));
        }
    }

    // Close last section
    if !current_section.is_empty() || !sections.is_empty() {
        lines.push("  └────────┘".to_string());
    }

    lines.push(String::new());
    Ok(lines.join("\n"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_timeline() {
        let db = TimelineDb::new();
        let output = render_timeline_ascii(&db).unwrap();
        assert!(output.contains("empty timeline"));
    }

    #[test]
    fn title_appears() {
        let input = "timeline\n    title History\n    section Ancient\n        Egypt : Pyramids";
        let diagram = crate::parse(input).unwrap();
        let db = match diagram {
            crate::diagrams::Diagram::Timeline(db) => db,
            _ => panic!("Expected timeline"),
        };
        let output = render_timeline_ascii(&db).unwrap();
        assert!(output.contains("History"), "Output:\n{}", output);
    }

    #[test]
    fn periods_and_events_appear() {
        let input =
            "timeline\n    title Test\n    section Era\n        Period1 : Event A : Event B";
        let diagram = crate::parse(input).unwrap();
        let db = match diagram {
            crate::diagrams::Diagram::Timeline(db) => db,
            _ => panic!("Expected timeline"),
        };
        let output = render_timeline_ascii(&db).unwrap();
        assert!(output.contains("Period1"), "Output:\n{}", output);
        assert!(output.contains("Event A"), "Output:\n{}", output);
        assert!(output.contains("Event B"), "Output:\n{}", output);
    }

    #[test]
    fn sections_appear() {
        let input =
            "timeline\n    title Test\n    section S1\n        P1 : E1\n    section S2\n        P2 : E2";
        let diagram = crate::parse(input).unwrap();
        let db = match diagram {
            crate::diagrams::Diagram::Timeline(db) => db,
            _ => panic!("Expected timeline"),
        };
        let output = render_timeline_ascii(&db).unwrap();
        assert!(output.contains("S1"), "Output:\n{}", output);
        assert!(output.contains("S2"), "Output:\n{}", output);
    }
}
