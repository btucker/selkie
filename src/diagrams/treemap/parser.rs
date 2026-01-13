//! Treemap diagram parser
//!
//! Parses treemap diagrams using pest grammar.

use pest::Parser;
use pest_derive::Parser;

use super::types::{build_hierarchy, TreemapDb, TreemapNode};

#[derive(Parser)]
#[grammar = "diagrams/treemap/treemap.pest"]
struct TreemapParser;

/// Parse a treemap diagram string into a database
pub fn parse(input: &str) -> Result<TreemapDb, Box<dyn std::error::Error>> {
    let pairs = TreemapParser::parse(Rule::diagram, input)?;
    let mut db = TreemapDb::new();
    let mut flat_items: Vec<(usize, TreemapNode)> = Vec::new();

    for pair in pairs {
        if pair.as_rule() == Rule::diagram {
            for inner in pair.into_inner() {
                if inner.as_rule() == Rule::document {
                    process_document(inner, &mut db, &mut flat_items)?;
                }
            }
        }
    }

    // Build hierarchy from flat items
    let root_nodes = build_hierarchy(flat_items);
    for node in root_nodes {
        db.add_root_node(node);
    }

    Ok(db)
}

fn process_document(
    pair: pest::iterators::Pair<Rule>,
    db: &mut TreemapDb,
    flat_items: &mut Vec<(usize, TreemapNode)>,
) -> Result<(), Box<dyn std::error::Error>> {
    for stmt in pair.into_inner() {
        if stmt.as_rule() == Rule::statement {
            process_statement(stmt, db, flat_items)?;
        }
    }
    Ok(())
}

fn process_statement(
    pair: pest::iterators::Pair<Rule>,
    db: &mut TreemapDb,
    flat_items: &mut Vec<(usize, TreemapNode)>,
) -> Result<(), Box<dyn std::error::Error>> {
    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::title_stmt => process_title(inner, db),
            Rule::acc_title_stmt => process_acc_title(inner, db),
            Rule::acc_descr_stmt => process_acc_descr(inner, db),
            Rule::acc_descr_multiline_stmt => process_acc_descr_multiline(inner, db),
            Rule::class_def_stmt => process_class_def(inner, db),
            Rule::row_stmt => process_row(inner, flat_items)?,
            Rule::comment_stmt => {} // Skip comments
            _ => {}
        }
    }
    Ok(())
}

fn process_title(pair: pest::iterators::Pair<Rule>, db: &mut TreemapDb) {
    for inner in pair.into_inner() {
        if inner.as_rule() == Rule::title_text {
            db.set_title(inner.as_str().trim());
        }
    }
}

fn process_acc_title(pair: pest::iterators::Pair<Rule>, db: &mut TreemapDb) {
    for inner in pair.into_inner() {
        if inner.as_rule() == Rule::acc_title_text {
            db.set_acc_title(inner.as_str().trim());
        }
    }
}

fn process_acc_descr(pair: pest::iterators::Pair<Rule>, db: &mut TreemapDb) {
    for inner in pair.into_inner() {
        if inner.as_rule() == Rule::acc_descr_text {
            db.set_acc_description(inner.as_str().trim());
        }
    }
}

fn process_acc_descr_multiline(pair: pest::iterators::Pair<Rule>, db: &mut TreemapDb) {
    for inner in pair.into_inner() {
        if inner.as_rule() == Rule::acc_descr_multiline_text {
            db.set_acc_description(inner.as_str().trim());
        }
    }
}

fn process_class_def(pair: pest::iterators::Pair<Rule>, db: &mut TreemapDb) {
    let mut class_name = String::new();
    let mut styles = String::new();

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::class_name => {
                class_name = inner.as_str().to_string();
            }
            Rule::class_styles => {
                styles = inner.as_str().trim().to_string();
            }
            _ => {}
        }
    }

    if !class_name.is_empty() {
        db.add_class(&class_name, &styles);
    }
}

fn process_row(
    pair: pest::iterators::Pair<Rule>,
    flat_items: &mut Vec<(usize, TreemapNode)>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut indent_level = 0;
    let mut node: Option<TreemapNode> = None;

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::indent => {
                indent_level = inner.as_str().len();
            }
            Rule::item => {
                node = Some(process_item(inner)?);
            }
            _ => {}
        }
    }

    if let Some(n) = node {
        flat_items.push((indent_level, n));
    }

    Ok(())
}

fn process_item(
    pair: pest::iterators::Pair<Rule>,
) -> Result<TreemapNode, Box<dyn std::error::Error>> {
    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::leaf_item => return process_leaf_item(inner),
            Rule::section_item => return process_section_item(inner),
            _ => {}
        }
    }
    Ok(TreemapNode::section(""))
}

fn process_leaf_item(
    pair: pest::iterators::Pair<Rule>,
) -> Result<TreemapNode, Box<dyn std::error::Error>> {
    let mut name = String::new();
    let mut value: f64 = 0.0;
    let mut class_selector: Option<String> = None;

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::item_name => {
                name = extract_quoted_string(inner.as_str());
            }
            Rule::item_value => {
                value = inner.as_str().parse()?;
            }
            Rule::class_selector => {
                for sel in inner.into_inner() {
                    if sel.as_rule() == Rule::selector_name {
                        class_selector = Some(sel.as_str().to_string());
                    }
                }
            }
            _ => {}
        }
    }

    let mut node = TreemapNode::leaf(&name, value);
    if let Some(cls) = class_selector {
        node = node.with_class(&cls);
    }
    Ok(node)
}

fn process_section_item(
    pair: pest::iterators::Pair<Rule>,
) -> Result<TreemapNode, Box<dyn std::error::Error>> {
    let mut name = String::new();
    let mut class_selector: Option<String> = None;

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::item_name => {
                name = extract_quoted_string(inner.as_str());
            }
            Rule::class_selector => {
                for sel in inner.into_inner() {
                    if sel.as_rule() == Rule::selector_name {
                        class_selector = Some(sel.as_str().to_string());
                    }
                }
            }
            _ => {}
        }
    }

    let mut node = TreemapNode::section(&name);
    if let Some(cls) = class_selector {
        node = node.with_class(&cls);
    }
    Ok(node)
}

/// Extract the content from a quoted string (removes surrounding quotes)
fn extract_quoted_string(s: &str) -> String {
    if (s.starts_with('"') && s.ends_with('"')) || (s.starts_with('\'') && s.ends_with('\'')) {
        s[1..s.len() - 1].to_string()
    } else {
        s.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_treemap() {
        let input = r#"treemap-beta
"Category A"
    "Item A1": 10
    "Item A2": 20
"Category B"
    "Item B1": 15
    "Item B2": 25
"#;
        let result = parse(input).unwrap();
        let root_nodes = result.get_root_nodes();
        assert_eq!(root_nodes.len(), 2);
        assert_eq!(root_nodes[0].name, "Category A");
        assert_eq!(root_nodes[0].children.len(), 2);
        assert_eq!(root_nodes[0].children[0].name, "Item A1");
        assert_eq!(root_nodes[0].children[0].value, Some(10.0));
    }

    #[test]
    fn test_treemap_keyword() {
        let input = r#"treemap
"Root"
    "Leaf": 10
"#;
        let result = parse(input);
        assert!(result.is_ok());
    }

    #[test]
    fn test_hierarchical_treemap() {
        let input = r#"treemap-beta
"Products"
    "Electronics"
        "Phones": 50
        "Computers": 30
    "Clothing"
        "Shirts": 10
        "Pants": 15
"#;
        let result = parse(input).unwrap();
        let root_nodes = result.get_root_nodes();
        assert_eq!(root_nodes.len(), 1);
        assert_eq!(root_nodes[0].name, "Products");
        assert_eq!(root_nodes[0].children.len(), 2);
        assert_eq!(root_nodes[0].children[0].name, "Electronics");
        assert_eq!(root_nodes[0].children[0].children.len(), 2);
    }

    #[test]
    fn test_class_def() {
        let input = r#"treemap-beta
"Root"
    "Item": 10:::class1

classDef class1 fill:red,stroke:blue;
"#;
        let result = parse(input).unwrap();
        let classes = result.get_classes();
        assert!(classes.contains_key("class1"));
        let class = classes.get("class1").unwrap();
        assert!(class.styles.iter().any(|s| s.contains("fill")));
    }

    #[test]
    fn test_section_with_class() {
        let input = r#"treemap-beta
"Section":::myClass
    "Leaf": 10
"#;
        let result = parse(input).unwrap();
        let root_nodes = result.get_root_nodes();
        assert_eq!(root_nodes[0].class_selector, Some("myClass".to_string()));
    }

    #[test]
    fn test_leaf_with_class() {
        let input = r#"treemap-beta
"Root"
    "Leaf": 10:::highlight
"#;
        let result = parse(input).unwrap();
        let root_nodes = result.get_root_nodes();
        assert_eq!(
            root_nodes[0].children[0].class_selector,
            Some("highlight".to_string())
        );
    }

    #[test]
    fn test_comments() {
        let input = r#"treemap-beta
%% This is a comment
"Category A"
    "Item A1": 10
%% Another comment
"Category B"
    "Item B1": 15
"#;
        let result = parse(input).unwrap();
        let root_nodes = result.get_root_nodes();
        assert_eq!(root_nodes.len(), 2);
    }

    #[test]
    fn test_decimal_values() {
        let input = r#"treemap-beta
"Root"
    "Item": 10.5
"#;
        let result = parse(input).unwrap();
        let root_nodes = result.get_root_nodes();
        assert_eq!(root_nodes[0].children[0].value, Some(10.5));
    }

    #[test]
    fn test_comma_separator() {
        let input = r#"treemap-beta
"Root"
    "Item", 10
"#;
        let result = parse(input).unwrap();
        let root_nodes = result.get_root_nodes();
        assert_eq!(root_nodes[0].children[0].value, Some(10.0));
    }

    #[test]
    fn test_count_nodes() {
        let input = r#"treemap-beta
"Root"
    "Branch"
        "Leaf 1": 10
        "Leaf 2": 20
    "Leaf 3": 30
"#;
        let result = parse(input).unwrap();
        assert_eq!(result.count_nodes(), 5);
    }

    #[test]
    fn test_leading_comment() {
        let input = r#"%% Comment before keyword
treemap-beta
"Root"
    "Leaf": 10
"#;
        let result = parse(input);
        assert!(result.is_ok());
    }

    #[test]
    fn test_single_quoted_names() {
        let input = r#"treemap-beta
'Category A'
    'Item A1': 10
"#;
        let result = parse(input).unwrap();
        let root_nodes = result.get_root_nodes();
        assert_eq!(root_nodes[0].name, "Category A");
    }

    #[test]
    fn test_multiple_class_defs() {
        let input = r#"treemap-beta
"Root"
    "A": 20
    "B":::important
        "B1": 10
    "C": 5:::secondary

classDef important fill:#f96,stroke:#333;
classDef secondary fill:#6cf,stroke:#333;
"#;
        let result = parse(input).unwrap();
        let classes = result.get_classes();
        assert!(classes.contains_key("important"));
        assert!(classes.contains_key("secondary"));
    }

    #[test]
    fn test_get_root() {
        let input = r#"treemap-beta
"A"
    "A1": 10
"B"
    "B1": 20
"#;
        let result = parse(input).unwrap();
        let root = result.get_root();
        assert!(root.name.is_empty()); // Root wrapper has empty name
        assert_eq!(root.children.len(), 2);
    }

    #[test]
    fn test_deep_nesting() {
        let input = r#"treemap-beta
"Level 1"
    "Level 2"
        "Level 3"
            "Level 4": 10
"#;
        let result = parse(input).unwrap();
        let root_nodes = result.get_root_nodes();
        assert_eq!(
            root_nodes[0].children[0].children[0].children[0].name,
            "Level 4"
        );
    }
}
