//! Kanban diagram parser
//!
//! Parses kanban diagrams using pest grammar.

use pest::Parser;
use pest_derive::Parser;

use super::{KanbanDb, NodeShape};

#[derive(Parser)]
#[grammar = "diagrams/kanban/kanban.pest"]
struct KanbanParser;

/// Parse a kanban diagram string into a database
pub fn parse(input: &str) -> Result<KanbanDb, Box<dyn std::error::Error>> {
    let pairs = KanbanParser::parse(Rule::diagram, input)?;
    let mut db = KanbanDb::new();

    for pair in pairs {
        if pair.as_rule() == Rule::diagram {
            for inner in pair.into_inner() {
                if inner.as_rule() == Rule::document {
                    process_document(inner, &mut db)?;
                }
            }
        }
    }

    Ok(db)
}

fn process_document(
    pair: pest::iterators::Pair<Rule>,
    db: &mut KanbanDb,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut base_indent: Option<usize> = None;
    for stmt in pair.into_inner() {
        if stmt.as_rule() == Rule::statement {
            process_statement(stmt, db, &mut base_indent)?;
        }
    }
    Ok(())
}

fn process_statement(
    pair: pest::iterators::Pair<Rule>,
    db: &mut KanbanDb,
    base_indent: &mut Option<usize>,
) -> Result<(), Box<dyn std::error::Error>> {
    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::node_stmt => process_node(inner, db, base_indent)?,
            Rule::icon_decorator => process_icon_decorator(inner, db),
            Rule::class_decorator => process_class_decorator(inner, db),
            Rule::comment_stmt => {} // Skip comments
            _ => {}
        }
    }
    Ok(())
}

fn process_node(
    pair: pest::iterators::Pair<Rule>,
    db: &mut KanbanDb,
    base_indent: &mut Option<usize>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut indent_level = 0usize;
    let mut id: Option<String> = None;
    let mut label: Option<String> = None;
    let mut shape = NodeShape::Default;
    let mut metadata: Vec<(String, String)> = Vec::new();

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::indent => {
                indent_level = inner.as_str().len();
            }
            Rule::node => {
                let (parsed_id, parsed_label, parsed_shape) = process_node_inner(inner);
                id = parsed_id;
                label = parsed_label;
                shape = parsed_shape;
            }
            Rule::metadata => {
                metadata = process_metadata(inner);
            }
            _ => {}
        }
    }

    // Determine hierarchy level (section = 0, items = 1+)
    // The first node establishes the base indent level for sections
    let level = match *base_indent {
        None => {
            // First node - this is a section
            *base_indent = Some(indent_level);
            0
        }
        Some(base) => {
            if indent_level <= base {
                // Same or less indent as base - this is a section
                0
            } else {
                // More indented - this is an item
                1
            }
        }
    };

    // If we have only a shape without ID, use the label as ID
    let final_id = id.clone().or_else(|| label.clone());
    let final_label = label.or_else(|| id.clone()).unwrap_or_default();

    db.add_node(level, final_id.as_deref(), &final_label, shape);

    // Apply metadata
    for (key, value) in metadata {
        db.set_metadata(&key, &value);
    }

    Ok(())
}

fn process_node_inner(
    pair: pest::iterators::Pair<Rule>,
) -> (Option<String>, Option<String>, NodeShape) {
    let mut id: Option<String> = None;
    let mut label: Option<String> = None;
    let mut shape = NodeShape::Default;

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::node_with_id => {
                for node_inner in inner.into_inner() {
                    match node_inner.as_rule() {
                        Rule::node_id => {
                            id = Some(node_inner.as_str().to_string());
                        }
                        Rule::node_shape => {
                            let (s, l) = process_shape(node_inner);
                            shape = s;
                            label = l;
                        }
                        _ => {}
                    }
                }
                // If no shape was provided, the ID is also the label
                if label.is_none() {
                    label = id.clone();
                }
            }
            Rule::node_without_id => {
                for shape_pair in inner.into_inner() {
                    if shape_pair.as_rule() == Rule::node_shape {
                        let (s, l) = process_shape(shape_pair);
                        shape = s;
                        label = l.clone();
                        id = l; // For node without ID, label becomes the ID
                    }
                }
            }
            _ => {}
        }
    }

    (id, label, shape)
}

fn process_shape(pair: pest::iterators::Pair<Rule>) -> (NodeShape, Option<String>) {
    let mut shape = NodeShape::Default;
    let mut label: Option<String> = None;

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::square_shape => {
                shape = NodeShape::Rect;
                for descr in inner.into_inner() {
                    if descr.as_rule() == Rule::node_descr {
                        label = Some(extract_descr(descr));
                    }
                }
            }
            Rule::rounded_shape => {
                shape = NodeShape::RoundedRect;
                for descr in inner.into_inner() {
                    if descr.as_rule() == Rule::node_descr {
                        label = Some(extract_descr(descr));
                    }
                }
            }
            Rule::circle_shape => {
                shape = NodeShape::Circle;
                for descr in inner.into_inner() {
                    if descr.as_rule() == Rule::node_descr {
                        label = Some(extract_descr(descr));
                    }
                }
            }
            Rule::double_circle_shape => {
                shape = NodeShape::Circle; // Treat double circle as circle
                for descr in inner.into_inner() {
                    if descr.as_rule() == Rule::node_descr {
                        label = Some(extract_descr(descr));
                    }
                }
            }
            Rule::hexagon_shape => {
                shape = NodeShape::Hexagon;
                for descr in inner.into_inner() {
                    if descr.as_rule() == Rule::node_descr {
                        label = Some(extract_descr(descr));
                    }
                }
            }
            Rule::cloud_shape => {
                shape = NodeShape::Cloud;
                for descr in inner.into_inner() {
                    if descr.as_rule() == Rule::node_descr {
                        label = Some(extract_descr(descr));
                    }
                }
            }
            Rule::bang_shape => {
                shape = NodeShape::Bang;
                for descr in inner.into_inner() {
                    if descr.as_rule() == Rule::node_descr {
                        label = Some(extract_descr(descr));
                    }
                }
            }
            _ => {}
        }
    }

    (shape, label)
}

fn extract_descr(pair: pest::iterators::Pair<Rule>) -> String {
    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::quoted_descr => {
                let s = inner.as_str();
                // Remove surrounding quotes
                return s[1..s.len() - 1].to_string();
            }
            Rule::md_descr => {
                let s = inner.as_str();
                // Remove surrounding "`" and `"`
                return s[2..s.len() - 2].to_string();
            }
            Rule::raw_descr => {
                return inner.as_str().to_string();
            }
            _ => {}
        }
    }
    String::new()
}

fn process_metadata(pair: pest::iterators::Pair<Rule>) -> Vec<(String, String)> {
    let mut result = Vec::new();

    for inner in pair.into_inner() {
        if inner.as_rule() == Rule::metadata_content {
            for item in inner.into_inner() {
                if item.as_rule() == Rule::metadata_item {
                    let mut key = String::new();
                    let mut value = String::new();

                    for part in item.into_inner() {
                        match part.as_rule() {
                            Rule::metadata_key => {
                                key = part.as_str().to_string();
                            }
                            Rule::metadata_value => {
                                for val_inner in part.into_inner() {
                                    match val_inner.as_rule() {
                                        Rule::single_quoted_value | Rule::double_quoted_value => {
                                            let s = val_inner.as_str();
                                            // Remove surrounding quotes
                                            value = s[1..s.len() - 1].to_string();
                                        }
                                        Rule::unquoted_value => {
                                            value = val_inner.as_str().to_string();
                                        }
                                        _ => {}
                                    }
                                }
                            }
                            _ => {}
                        }
                    }

                    if !key.is_empty() {
                        result.push((key, value));
                    }
                }
            }
        }
    }

    result
}

fn process_icon_decorator(pair: pest::iterators::Pair<Rule>, db: &mut KanbanDb) {
    for inner in pair.into_inner() {
        if inner.as_rule() == Rule::icon_name {
            db.decorate_icon(inner.as_str().trim());
        }
    }
}

fn process_class_decorator(pair: pest::iterators::Pair<Rule>, db: &mut KanbanDb) {
    for inner in pair.into_inner() {
        if inner.as_rule() == Rule::class_list {
            db.decorate_classes(inner.as_str().trim());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_root() {
        let input = "kanban
    root";
        let result = parse(input).unwrap();
        let sections = result.get_sections();
        assert_eq!(sections.len(), 1);
        assert_eq!(sections[0].label, "root");
    }

    #[test]
    fn test_hierarchical_kanban() {
        let input = "kanban
    root
      child1
      child2
 ";
        let result = parse(input).unwrap();
        let sections = result.get_sections();
        assert_eq!(sections.len(), 1);
        assert_eq!(sections[0].label, "root");

        let children = result.get_children(&sections[0].id);
        assert_eq!(children.len(), 2);
        assert_eq!(children[0].label, "child1");
        assert_eq!(children[1].label, "child2");
    }

    #[test]
    fn test_root_with_rounded_shape() {
        let input = "kanban
    (root)";
        let result = parse(input).unwrap();
        let sections = result.get_sections();
        assert_eq!(sections[0].label, "root");
    }

    #[test]
    fn test_deeper_hierarchical_levels() {
        let input = "kanban
    root
      child1
        leaf1
      child2";
        let result = parse(input).unwrap();
        let sections = result.get_sections();
        assert_eq!(sections.len(), 1);

        let children = result.get_children(&sections[0].id);
        assert_eq!(children.len(), 3);
    }

    #[test]
    fn test_multiple_sections() {
        let input = "kanban
    section1
    section2";
        let result = parse(input).unwrap();
        let sections = result.get_sections();
        assert_eq!(sections.len(), 2);
        assert_eq!(sections[0].label, "section1");
        assert_eq!(sections[1].label, "section2");
    }

    #[test]
    fn test_id_and_label_square() {
        let input = "kanban
    root[The root]
      ";
        let result = parse(input).unwrap();
        let sections = result.get_sections();
        assert_eq!(sections[0].id, "root");
        assert_eq!(sections[0].label, "The root");
    }

    #[test]
    fn test_id_and_label_rounded() {
        let input = "kanban
    root
      theId(child1)";
        let result = parse(input).unwrap();
        let sections = result.get_sections();
        assert_eq!(sections[0].label, "root");

        let children = result.get_children(&sections[0].id);
        assert_eq!(children.len(), 1);
        assert_eq!(children[0].label, "child1");
        assert_eq!(children[0].id, "theId");
    }

    #[test]
    fn test_icon_decorator() {
        let input = "kanban
    root[The root]
    ::icon(bomb)
    ";
        let result = parse(input).unwrap();
        let sections = result.get_sections();
        assert_eq!(sections[0].id, "root");
        assert_eq!(sections[0].label, "The root");
        assert_eq!(sections[0].icon, Some("bomb".to_string()));
    }

    #[test]
    fn test_class_decorator() {
        let input = "kanban
    root[The root]
    :::m-4 p-8
    ";
        let result = parse(input).unwrap();
        let sections = result.get_sections();
        assert_eq!(sections[0].id, "root");
        assert_eq!(sections[0].label, "The root");
        assert_eq!(sections[0].css_classes, Some("m-4 p-8".to_string()));
    }

    #[test]
    fn test_classes_and_icon() {
        let input = "kanban
    root[The root]
    :::m-4 p-8
    ::icon(bomb)
    ";
        let result = parse(input).unwrap();
        let sections = result.get_sections();
        assert_eq!(sections[0].id, "root");
        assert_eq!(sections[0].label, "The root");
        assert_eq!(sections[0].css_classes, Some("m-4 p-8".to_string()));
        assert_eq!(sections[0].icon, Some("bomb".to_string()));
    }

    #[test]
    fn test_icon_and_classes() {
        let input = "kanban
    root[The root]
    ::icon(bomb)
    :::m-4 p-8
    ";
        let result = parse(input).unwrap();
        let sections = result.get_sections();
        assert_eq!(sections[0].id, "root");
        assert_eq!(sections[0].label, "The root");
        assert_eq!(sections[0].css_classes, Some("m-4 p-8".to_string()));
        assert_eq!(sections[0].icon, Some("bomb".to_string()));
    }

    #[test]
    fn test_special_chars_in_label() {
        let input = r#"kanban
    root["String containing []"]
"#;
        let result = parse(input).unwrap();
        let sections = result.get_sections();
        assert_eq!(sections[0].id, "root");
        assert_eq!(sections[0].label, "String containing []");
    }

    #[test]
    fn test_special_chars_in_child() {
        let input = r#"kanban
    root["String containing []"]
      child1["String containing ()"]
"#;
        let result = parse(input).unwrap();
        let sections = result.get_sections();
        assert_eq!(sections[0].id, "root");
        assert_eq!(sections[0].label, "String containing []");

        let children = result.get_children(&sections[0].id);
        assert_eq!(children.len(), 1);
        assert_eq!(children[0].label, "String containing ()");
    }

    #[test]
    fn test_child_after_class() {
        let input = "kanban
  root(Root)
    Child(Child)
    :::hot
      a(a)
      b[New Stuff]";
        let result = parse(input).unwrap();
        let sections = result.get_sections();
        assert_eq!(sections[0].id, "root");
        assert_eq!(sections[0].label, "Root");

        let children = result.get_children(&sections[0].id);
        assert_eq!(children.len(), 3);
        assert_eq!(children[0].id, "Child");
        assert_eq!(children[1].id, "a");
        assert_eq!(children[2].id, "b");
    }

    #[test]
    fn test_empty_rows() {
        let input = "kanban
  root(Root)
    Child(Child)
      a(a)

      b[New Stuff]";
        let result = parse(input).unwrap();
        let sections = result.get_sections();
        assert_eq!(sections[0].id, "root");
        assert_eq!(sections[0].label, "Root");

        let children = result.get_children(&sections[0].id);
        assert_eq!(children.len(), 3);
        assert_eq!(children[0].id, "Child");
        assert_eq!(children[1].id, "a");
        assert_eq!(children[2].id, "b");
    }

    #[test]
    fn test_comments() {
        let input = "kanban
  root(Root)
    Child(Child)
      a(a)

      %% This is a comment
      b[New Stuff]";
        let result = parse(input).unwrap();
        let sections = result.get_sections();
        let children = result.get_children(&sections[0].id);
        assert_eq!(children.len(), 3);
    }

    #[test]
    fn test_rows_with_spaces() {
        let input = "kanban\nroot\n A\n \n\n B";
        let result = parse(input).unwrap();
        let sections = result.get_sections();
        assert_eq!(sections[0].id, "root");

        let children = result.get_children(&sections[0].id);
        assert_eq!(children.len(), 2);
        assert_eq!(children[0].id, "A");
        assert_eq!(children[1].id, "B");
    }

    #[test]
    fn test_rows_above_kanban() {
        let input = "\n \nkanban\nroot\n A\n \n\n B";
        let result = parse(input).unwrap();
        let sections = result.get_sections();
        assert_eq!(sections[0].id, "root");

        let children = result.get_children(&sections[0].id);
        assert_eq!(children.len(), 2);
        assert_eq!(children[0].id, "A");
        assert_eq!(children[1].id, "B");
    }

    #[test]
    fn test_metadata_priority() {
        let input = "kanban
        root@{ priority: high }
    ";
        let result = parse(input).unwrap();
        let sections = result.get_sections();
        assert_eq!(sections[0].id, "root");
        assert_eq!(sections[0].priority, Some("high".to_string()));
    }

    #[test]
    fn test_metadata_assigned() {
        let input = "kanban
        root@{ assigned: knsv }
    ";
        let result = parse(input).unwrap();
        let sections = result.get_sections();
        assert_eq!(sections[0].id, "root");
        assert_eq!(sections[0].assigned, Some("knsv".to_string()));
    }

    #[test]
    fn test_metadata_icon() {
        let input = "kanban
        root@{ icon: star }
    ";
        let result = parse(input).unwrap();
        let sections = result.get_sections();
        assert_eq!(sections[0].id, "root");
        assert_eq!(sections[0].icon, Some("star".to_string()));
    }

    #[test]
    fn test_metadata_multiline() {
        // Note: Multiline metadata is supported but opening brace must be on same line as first item
        // or content can be separated by commas on one line
        let input = "kanban
        root@{ icon: star,
          assigned: knsv }
    ";
        let result = parse(input).unwrap();
        let sections = result.get_sections();
        assert_eq!(sections[0].id, "root");
        assert_eq!(sections[0].icon, Some("star".to_string()));
        assert_eq!(sections[0].assigned, Some("knsv".to_string()));
    }

    #[test]
    fn test_metadata_one_line() {
        let input = "kanban
        root@{ icon: star, assigned: knsv }
    ";
        let result = parse(input).unwrap();
        let sections = result.get_sections();
        assert_eq!(sections[0].id, "root");
        assert_eq!(sections[0].icon, Some("star".to_string()));
        assert_eq!(sections[0].assigned, Some("knsv".to_string()));
    }

    #[test]
    fn test_metadata_label() {
        let input = "kanban
        root@{ icon: star, label: 'fix things' }
    ";
        let result = parse(input).unwrap();
        let sections = result.get_sections();
        assert_eq!(sections[0].label, "fix things");
    }

    #[test]
    fn test_metadata_ticket() {
        let input = "kanban
        root@{ ticket: MC-1234 }
    ";
        let result = parse(input).unwrap();
        let sections = result.get_sections();
        assert_eq!(sections[0].id, "root");
        assert_eq!(sections[0].ticket, Some("MC-1234".to_string()));
    }
}
