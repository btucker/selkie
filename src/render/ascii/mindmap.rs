//! ASCII renderer for mindmap diagrams.
//!
//! Renders mindmaps as indented tree structures with branch connectors,
//! similar to the `tree` command output. Node shapes are indicated by
//! visual Unicode symbols (e.g. `☁` for cloud, `⬡` for hexagon).

use crate::diagrams::mindmap::{MindmapDb, MindmapNode, NodeType};
use crate::error::Result;

/// Render a mindmap as an indented tree in character art.
pub fn render_mindmap_ascii(db: &MindmapDb) -> Result<String> {
    let root = match db.get_mindmap() {
        Some(node) => node,
        None => return Ok("(empty mindmap)\n".to_string()),
    };

    let mut lines: Vec<String> = Vec::new();

    // Render root node
    lines.push(format_node(root));

    // Render children recursively
    render_children(&root.children, "", &mut lines);

    lines.push(String::new());
    Ok(lines.join("\n"))
}

/// Format a node label with visual ASCII shape indicators.
///
/// Each shape type uses a distinctive bracket/border style so the
/// node type is visually apparent without echoing raw Mermaid syntax.
fn format_node(node: &MindmapNode) -> String {
    let text = clean_text(&node.descr);
    match node.node_type {
        NodeType::Default => text,
        NodeType::Rect => format!("[ {} ]", text),
        NodeType::RoundedRect => format!("( {} )", text),
        NodeType::Circle => format!("o {} o", text),
        NodeType::Cloud => format!("☁ {} ☁", text),
        NodeType::Bang => format!("⚡ {} ⚡", text),
        NodeType::Hexagon => format!("⬡ {} ⬡", text),
    }
}

/// Clean HTML line breaks and normalize whitespace.
fn clean_text(raw: &str) -> String {
    let cleaned = raw.replace("<br/>", " ").replace("<br>", " ");
    cleaned.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Recursively render children with tree branch connectors.
fn render_children(children: &[MindmapNode], prefix: &str, lines: &mut Vec<String>) {
    for (i, child) in children.iter().enumerate() {
        let is_last = i == children.len() - 1;
        let connector = if is_last { "└── " } else { "├── " };
        let child_prefix = if is_last { "    " } else { "│   " };

        lines.push(format!("{}{}{}", prefix, connector, format_node(child)));

        // Recurse into grandchildren
        let new_prefix = format!("{}{}", prefix, child_prefix);
        render_children(&child.children, &new_prefix, lines);
    }
}

/// Collect all node labels from the mindmap tree (for eval).
pub fn collect_labels(node: &MindmapNode) -> Vec<String> {
    let mut labels = vec![clean_text(&node.descr)];
    for child in &node.children {
        labels.extend(collect_labels(child));
    }
    labels
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_mindmap(input: &str) -> MindmapDb {
        let diagram = crate::parse(input).unwrap();
        match diagram {
            crate::diagrams::Diagram::Mindmap(db) => db,
            _ => panic!("Expected mindmap diagram"),
        }
    }

    #[test]
    fn empty_mindmap() {
        let db = MindmapDb::new();
        let output = render_mindmap_ascii(&db).unwrap();
        assert!(
            output.contains("empty mindmap"),
            "Should indicate empty\nOutput:\n{}",
            output
        );
    }

    #[test]
    fn root_only() {
        let db = make_mindmap("mindmap\n  root((Main Topic))");
        let output = render_mindmap_ascii(&db).unwrap();
        assert!(
            output.contains("o Main Topic o"),
            "Root should show circle shape\nOutput:\n{}",
            output
        );
    }

    #[test]
    fn simple_tree_structure() {
        let db = make_mindmap("mindmap\n  root\n    Child1\n    Child2");
        let output = render_mindmap_ascii(&db).unwrap();
        assert!(output.contains("root"), "Should contain root");
        assert!(
            output.contains("Child1"),
            "Should contain Child1\nOutput:\n{}",
            output
        );
        assert!(
            output.contains("Child2"),
            "Should contain Child2\nOutput:\n{}",
            output
        );
    }

    #[test]
    fn tree_connectors_present() {
        let db = make_mindmap("mindmap\n  root\n    Child1\n    Child2");
        let output = render_mindmap_ascii(&db).unwrap();
        assert!(
            output.contains("├──"),
            "Should have branch connector\nOutput:\n{}",
            output
        );
        assert!(
            output.contains("└──"),
            "Should have last-child connector\nOutput:\n{}",
            output
        );
    }

    #[test]
    fn nested_children() {
        let db = make_mindmap(
            "mindmap\n  root\n    Parent\n      GrandChild1\n      GrandChild2\n    Sibling",
        );
        let output = render_mindmap_ascii(&db).unwrap();
        assert!(
            output.contains("GrandChild1"),
            "Should contain nested child\nOutput:\n{}",
            output
        );
        assert!(
            output.contains("│"),
            "Should have vertical connector for nested\nOutput:\n{}",
            output
        );
    }

    #[test]
    fn node_shapes_rendered() {
        let db = make_mindmap(
            "mindmap\n  root((Circle))\n    [Rectangle]\n    (Rounded)\n    )Cloud(\n    ))Bang((",
        );
        let output = render_mindmap_ascii(&db).unwrap();
        assert!(
            output.contains("o Circle o"),
            "Circle shape\nOutput:\n{}",
            output
        );
        assert!(
            output.contains("[ Rectangle ]"),
            "Rect shape\nOutput:\n{}",
            output
        );
        assert!(
            output.contains("( Rounded )"),
            "Rounded shape\nOutput:\n{}",
            output
        );
        assert!(
            output.contains("☁ Cloud ☁"),
            "Cloud shape\nOutput:\n{}",
            output
        );
        assert!(
            output.contains("⚡ Bang ⚡"),
            "Bang shape\nOutput:\n{}",
            output
        );
    }

    #[test]
    fn gallery_mindmap_renders() {
        let input = std::fs::read_to_string("docs/sources/mindmap.mmd").unwrap();
        let db = make_mindmap(&input);
        let output = render_mindmap_ascii(&db).unwrap();
        assert!(
            output.contains("mindmap"),
            "Should contain root label\nOutput:\n{}",
            output
        );
        assert!(
            output.contains("Origins"),
            "Should contain Origins\nOutput:\n{}",
            output
        );
        assert!(
            output.contains("Research"),
            "Should contain Research\nOutput:\n{}",
            output
        );
        assert!(
            output.contains("Tools"),
            "Should contain Tools\nOutput:\n{}",
            output
        );
        assert!(
            output.contains("Mermaid"),
            "Should contain Mermaid\nOutput:\n{}",
            output
        );
    }

    #[test]
    fn gallery_mindmap_complex_renders() {
        let input = std::fs::read_to_string("docs/sources/mindmap_complex.mmd").unwrap();
        let db = make_mindmap(&input);
        let output = render_mindmap_ascii(&db).unwrap();
        assert!(
            output.contains("mindmap"),
            "Should contain root\nOutput:\n{}",
            output
        );
        assert!(
            output.contains("Creative techniques"),
            "Should contain deep nested node\nOutput:\n{}",
            output
        );
        assert!(
            output.contains("☁ I am a cloud ☁"),
            "Should show cloud shape\nOutput:\n{}",
            output
        );
        assert!(
            output.contains("⚡ I am a bang ⚡"),
            "Should show bang shape\nOutput:\n{}",
            output
        );
    }

    #[test]
    fn shapes_show_text_not_raw_syntax() {
        let db = make_mindmap(
            "mindmap\n  root((Circle))\n    )Cloud(\n    ))Bang((\n    {{Hexagon}}\n    [Rect]\n    (Rounded)",
        );
        let output = render_mindmap_ascii(&db).unwrap();
        // Text content must appear
        assert!(
            output.contains("Circle"),
            "Should contain Circle text\nOutput:\n{}",
            output
        );
        assert!(
            output.contains("Cloud"),
            "Should contain Cloud text\nOutput:\n{}",
            output
        );
        assert!(
            output.contains("Bang"),
            "Should contain Bang text\nOutput:\n{}",
            output
        );
        assert!(
            output.contains("Hexagon"),
            "Should contain Hexagon text\nOutput:\n{}",
            output
        );
        assert!(
            output.contains("Rect"),
            "Should contain Rect text\nOutput:\n{}",
            output
        );
        assert!(
            output.contains("Rounded"),
            "Should contain Rounded text\nOutput:\n{}",
            output
        );
        // Raw syntax must NOT appear
        assert!(
            !output.contains("((Circle))"),
            "Should NOT show raw circle syntax\nOutput:\n{}",
            output
        );
        assert!(
            !output.contains(")Cloud("),
            "Should NOT show raw cloud syntax\nOutput:\n{}",
            output
        );
        assert!(
            !output.contains("))Bang(("),
            "Should NOT show raw bang syntax\nOutput:\n{}",
            output
        );
        assert!(
            !output.contains("{{Hexagon}}"),
            "Should NOT show raw hexagon syntax\nOutput:\n{}",
            output
        );
    }

    #[test]
    fn complex_mindmap_no_raw_syntax() {
        let input = std::fs::read_to_string("docs/sources/mindmap_complex.mmd").unwrap();
        let db = make_mindmap(&input);
        let output = render_mindmap_ascii(&db).unwrap();
        // Text content should appear
        assert!(
            output.contains("mindmap"),
            "Should contain root label\nOutput:\n{}",
            output
        );
        assert!(
            output.contains("I am a cloud"),
            "Should contain cloud text\nOutput:\n{}",
            output
        );
        assert!(
            output.contains("I am a bang"),
            "Should contain bang text\nOutput:\n{}",
            output
        );
        // Raw syntax must NOT appear
        assert!(
            !output.contains("((mindmap))"),
            "Should NOT show raw circle syntax\nOutput:\n{}",
            output
        );
        assert!(
            !output.contains(")I am a cloud("),
            "Should NOT show raw cloud syntax\nOutput:\n{}",
            output
        );
        assert!(
            !output.contains("))I am a bang(("),
            "Should NOT show raw bang syntax\nOutput:\n{}",
            output
        );
    }

    #[test]
    fn collect_labels_gets_all() {
        let db = make_mindmap("mindmap\n  root\n    A\n      B\n    C");
        let root = db.get_mindmap().unwrap();
        let labels = collect_labels(root);
        assert!(labels.contains(&"root".to_string()));
        assert!(labels.contains(&"A".to_string()));
        assert!(labels.contains(&"B".to_string()));
        assert!(labels.contains(&"C".to_string()));
    }

    #[test]
    fn last_child_uses_corner_connector() {
        let db = make_mindmap("mindmap\n  root\n    OnlyChild");
        let output = render_mindmap_ascii(&db).unwrap();
        // Single child should use └── (last-child connector)
        assert!(
            output.contains("└── OnlyChild"),
            "Only child should use └──\nOutput:\n{}",
            output
        );
    }
}
