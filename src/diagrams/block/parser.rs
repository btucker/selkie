//! Block diagram parser using pest grammar

use pest::Parser;
use pest_derive::Parser;

use super::types::{BlockDb, BlockType};

#[derive(Parser)]
#[grammar = "diagrams/block/block.pest"]
pub struct BlockParser;

/// Parse a block diagram and return the populated database
pub fn parse(input: &str) -> Result<BlockDb, String> {
    let mut db = BlockDb::new();

    let pairs =
        BlockParser::parse(Rule::diagram, input).map_err(|e| format!("Parse error: {}", e))?;

    for pair in pairs {
        if pair.as_rule() == Rule::diagram {
            for inner in pair.into_inner() {
                if inner.as_rule() == Rule::document {
                    process_document(&mut db, inner, None)?;
                }
            }
        }
    }

    Ok(db)
}

fn process_document(
    db: &mut BlockDb,
    pair: pest::iterators::Pair<Rule>,
    parent_id: Option<&str>,
) -> Result<(), String> {
    for stmt in pair.into_inner() {
        process_statement(db, stmt, parent_id)?;
    }
    Ok(())
}

fn process_statement(
    db: &mut BlockDb,
    pair: pest::iterators::Pair<Rule>,
    parent_id: Option<&str>,
) -> Result<(), String> {
    match pair.as_rule() {
        Rule::statement => {
            for inner in pair.into_inner() {
                process_statement(db, inner, parent_id)?;
            }
        }
        Rule::comment_stmt => {
            // Ignore comments
        }
        Rule::columns_stmt => {
            let mut columns: i32 = -1;
            for inner in pair.into_inner() {
                if inner.as_rule() == Rule::column_count {
                    columns = inner.as_str().parse().unwrap_or(-1);
                }
            }
            // Set columns on parent or root
            let target = parent_id.unwrap_or("root");
            if columns > 0 {
                db.set_columns(target, columns as usize);
            }
        }
        Rule::block_stmt => {
            let (id, label, block_type, width) = extract_block_info(pair)?;
            db.add_block_with_parent(&id, label.as_deref(), block_type, parent_id);
            if let Some(w) = width {
                db.set_width(&id, w);
            }
        }
        Rule::space_stmt => {
            // Generate unique ID for space
            let id = db.generate_space_id();
            db.add_block_with_parent(&id, None, BlockType::Space, parent_id);
        }
        Rule::composite_block => {
            process_composite(db, pair, parent_id)?;
        }
        Rule::edge_stmt => {
            process_edge(db, pair, parent_id)?;
        }
        Rule::class_def_stmt => {
            process_class_def(db, pair)?;
        }
        Rule::class_stmt => {
            process_class_assignment(db, pair)?;
        }
        Rule::style_stmt => {
            process_style(db, pair)?;
        }
        _ => {}
    }
    Ok(())
}

fn extract_block_info(
    pair: pest::iterators::Pair<Rule>,
) -> Result<(String, Option<String>, BlockType, Option<usize>), String> {
    let mut id = String::new();
    let mut label: Option<String> = None;
    let mut block_type = BlockType::Square;
    let mut width: Option<usize> = None;

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::block_id => {
                id = inner.as_str().to_string();
            }
            Rule::shape_with_label => {
                let (lbl, btype) = extract_shape(inner)?;
                label = Some(lbl);
                block_type = btype;
            }
            Rule::width_spec => {
                for w in inner.into_inner() {
                    if w.as_rule() == Rule::width_num {
                        if let Ok(n) = w.as_str().parse::<usize>() {
                            width = Some(n);
                        }
                    }
                }
            }
            Rule::arrow_dirs => {
                block_type = BlockType::BlockArrow;
            }
            _ => {}
        }
    }

    // If no label was set, use the id as label
    if label.is_none() {
        label = Some(id.clone());
    }

    Ok((id, label, block_type, width))
}

fn extract_shape(pair: pest::iterators::Pair<Rule>) -> Result<(String, BlockType), String> {
    let mut label = String::new();
    let mut block_type = BlockType::Square;

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::square_shape => {
                block_type = BlockType::Square;
                label = extract_label(inner);
            }
            Rule::round_shape => {
                block_type = BlockType::Round;
                label = extract_label(inner);
            }
            Rule::stadium_shape => {
                block_type = BlockType::Stadium;
                label = extract_label(inner);
            }
            Rule::subroutine_shape => {
                block_type = BlockType::Subroutine;
                label = extract_label(inner);
            }
            Rule::cylinder_shape => {
                block_type = BlockType::Cylinder;
                label = extract_label(inner);
            }
            Rule::circle_shape => {
                block_type = BlockType::Circle;
                label = extract_label(inner);
            }
            Rule::double_circle_shape => {
                block_type = BlockType::DoubleCircle;
                label = extract_label(inner);
            }
            Rule::diamond_shape => {
                block_type = BlockType::Diamond;
                label = extract_label(inner);
            }
            Rule::hexagon_shape => {
                block_type = BlockType::Hexagon;
                label = extract_label(inner);
            }
            Rule::parallelogram_shape => {
                block_type = BlockType::LeanRight;
                label = extract_label(inner);
            }
            Rule::trapezoid_shape => {
                block_type = BlockType::Trapezoid;
                label = extract_label(inner);
            }
            Rule::block_arrow_shape => {
                block_type = BlockType::BlockArrow;
                label = extract_label(inner);
            }
            _ => {}
        }
    }

    Ok((label, block_type))
}

fn extract_label(pair: pest::iterators::Pair<Rule>) -> String {
    for inner in pair.into_inner() {
        if inner.as_rule() == Rule::node_label {
            for label_inner in inner.into_inner() {
                match label_inner.as_rule() {
                    Rule::quoted_string => {
                        return unquote(label_inner.as_str());
                    }
                    Rule::md_string => {
                        let s = label_inner.as_str();
                        if s.starts_with("\"`") && s.ends_with("`\"") {
                            return s[2..s.len() - 2].to_string();
                        }
                        return s.to_string();
                    }
                    Rule::raw_label => {
                        return label_inner.as_str().trim().to_string();
                    }
                    _ => {}
                }
            }
        }
    }
    String::new()
}

fn process_composite(
    db: &mut BlockDb,
    pair: pest::iterators::Pair<Rule>,
    parent_id: Option<&str>,
) -> Result<(), String> {
    let mut composite_id = db.generate_composite_id();
    let mut label = String::new();

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::composite_start => {
                for start_inner in inner.into_inner() {
                    match start_inner.as_rule() {
                        Rule::block_id => {
                            composite_id = start_inner.as_str().to_string();
                        }
                        Rule::shape_with_label => {
                            let (lbl, _) = extract_shape(start_inner)?;
                            label = lbl;
                        }
                        _ => {}
                    }
                }
            }
            Rule::statement => {
                // First add the composite block with its parent
                if !db.get_blocks().contains_key(&composite_id) {
                    db.add_block_with_parent(
                        &composite_id,
                        Some(&label),
                        BlockType::Composite,
                        parent_id,
                    );
                }
                // Then process statements within it
                process_statement(db, inner, Some(&composite_id))?;
            }
            _ => {}
        }
    }

    // Ensure composite is added even if empty
    if !db.get_blocks().contains_key(&composite_id) {
        db.add_block_with_parent(&composite_id, Some(&label), BlockType::Composite, parent_id);
    }

    Ok(())
}

fn process_edge(
    db: &mut BlockDb,
    pair: pest::iterators::Pair<Rule>,
    parent_id: Option<&str>,
) -> Result<(), String> {
    let mut blocks: Vec<(String, Option<String>, BlockType, Option<usize>)> = Vec::new();
    let mut edge_label: Option<String> = None;

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::block_stmt => {
                blocks.push(extract_block_info(inner)?);
            }
            Rule::link => {
                for link_inner in inner.into_inner() {
                    if link_inner.as_rule() == Rule::link_with_label {
                        for lbl in link_inner.into_inner() {
                            if lbl.as_rule() == Rule::quoted_string {
                                edge_label = Some(unquote(lbl.as_str()));
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }

    // Add both blocks and the edge
    if blocks.len() >= 2 {
        let (id1, label1, type1, width1) = &blocks[0];
        let (id2, label2, type2, width2) = &blocks[1];

        // Add blocks if they don't exist (with parent if specified)
        if !db.get_blocks().contains_key(id1) {
            db.add_block_with_parent(id1, label1.as_deref(), type1.clone(), parent_id);
            if let Some(w) = width1 {
                db.set_width(id1, *w);
            }
        }
        if !db.get_blocks().contains_key(id2) {
            db.add_block_with_parent(id2, label2.as_deref(), type2.clone(), parent_id);
            if let Some(w) = width2 {
                db.set_width(id2, *w);
            }
        }

        db.add_edge(id1, id2, edge_label.as_deref());
    }

    Ok(())
}

fn process_class_def(db: &mut BlockDb, pair: pest::iterators::Pair<Rule>) -> Result<(), String> {
    let mut class_name = String::new();
    let mut styles: Vec<String> = Vec::new();

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::class_name => {
                class_name = inner.as_str().to_string();
            }
            Rule::style_list => {
                // Split by semicolon or comma
                styles = inner
                    .as_str()
                    .split([';', ','])
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
            }
            _ => {}
        }
    }

    if !class_name.is_empty() {
        let style_refs: Vec<&str> = styles.iter().map(|s| s.as_str()).collect();
        db.define_class(&class_name, &style_refs);
    }

    Ok(())
}

fn process_class_assignment(
    db: &mut BlockDb,
    pair: pest::iterators::Pair<Rule>,
) -> Result<(), String> {
    let mut ids: Vec<String> = Vec::new();
    let mut class_name = String::new();

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::id_list => {
                for id in inner.into_inner() {
                    if id.as_rule() == Rule::block_id {
                        ids.push(id.as_str().to_string());
                    }
                }
            }
            Rule::class_name => {
                class_name = inner.as_str().to_string();
            }
            _ => {}
        }
    }

    for id in &ids {
        db.apply_class(id, &class_name);
    }

    Ok(())
}

fn process_style(db: &mut BlockDb, pair: pest::iterators::Pair<Rule>) -> Result<(), String> {
    let mut ids: Vec<String> = Vec::new();
    let mut styles: Vec<String> = Vec::new();

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::id_list => {
                for id in inner.into_inner() {
                    if id.as_rule() == Rule::block_id {
                        ids.push(id.as_str().to_string());
                    }
                }
            }
            Rule::style_list => {
                styles = inner
                    .as_str()
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
            }
            _ => {}
        }
    }

    for id in &ids {
        db.apply_styles(id, &styles);
    }

    Ok(())
}

/// Remove surrounding quotes from a string
fn unquote(s: &str) -> String {
    if s.len() >= 2 && s.starts_with('"') && s.ends_with('"') {
        s[1..s.len() - 1].to_string()
    } else {
        s.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod basic_parsing {
        use super::*;

        #[test]
        fn should_parse_empty_block_diagram() {
            let result = parse("block-beta");
            assert!(result.is_ok(), "Parse error: {:?}", result.err());
        }

        #[test]
        fn should_parse_block_keyword() {
            let result = parse("block");
            assert!(result.is_ok(), "Parse error: {:?}", result.err());
        }

        #[test]
        fn should_parse_single_node() {
            let result = parse("block-beta\n    id");
            assert!(result.is_ok(), "Parse error: {:?}", result.err());
            let db = result.unwrap();
            assert_eq!(db.get_blocks().len(), 1);
            assert!(db.get_blocks().contains_key("id"));
        }

        #[test]
        fn should_parse_node_with_label() {
            let result = parse("block\n    id[\"A label\"]");
            assert!(result.is_ok(), "Parse error: {:?}", result.err());
            let db = result.unwrap();
            let block = db.get_blocks().get("id").unwrap();
            assert_eq!(block.label, Some("A label".to_string()));
            assert_eq!(block.block_type, BlockType::Square);
        }

        #[test]
        fn should_parse_multiple_nodes() {
            let result = parse("block\n    id1\n    id2\n    id3");
            assert!(result.is_ok(), "Parse error: {:?}", result.err());
            let db = result.unwrap();
            assert_eq!(db.get_blocks().len(), 3);
        }
    }

    mod edge_parsing {
        use super::*;

        #[test]
        fn should_parse_edge() {
            let result = parse("block\n    id1[\"first\"] --> id2[\"second\"]");
            assert!(result.is_ok(), "Parse error: {:?}", result.err());
            let db = result.unwrap();
            assert_eq!(db.get_blocks().len(), 2);
            assert_eq!(db.get_edges().len(), 1);
            assert_eq!(db.get_edges()[0].start, "id1");
            assert_eq!(db.get_edges()[0].end, "id2");
        }

        #[test]
        fn should_parse_edge_with_label() {
            let result = parse("block\n    id1[\"first\"] -- \"a label\" --> id2[\"second\"]");
            assert!(result.is_ok(), "Parse error: {:?}", result.err());
            let db = result.unwrap();
            let edge = &db.get_edges()[0];
            assert_eq!(edge.label, Some("a label".to_string()));
        }
    }

    mod columns_parsing {
        use super::*;

        #[test]
        fn should_parse_columns() {
            let result = parse("block\n    columns 2\n    block1[\"Block 1\"]");
            assert!(result.is_ok(), "Parse error: {:?}", result.err());
        }

        #[test]
        fn should_parse_columns_auto() {
            let result = parse("block\n    columns auto\n    block1[\"Block 1\"]");
            assert!(result.is_ok(), "Parse error: {:?}", result.err());
        }
    }

    mod composite_parsing {
        use super::*;

        #[test]
        fn should_parse_composite_block() {
            let result = parse("block\n    block\n        aBlock[\"ABlock\"]\n    end");
            assert!(result.is_ok(), "Parse error: {:?}", result.err());
            let db = result.unwrap();
            // Should have the composite and the inner block
            assert!(!db.get_blocks().is_empty());
        }

        #[test]
        fn should_parse_composite_with_id() {
            let result = parse("block\n    block:compoundBlock[\"Compound block\"]\n        columns 1\n        block2[\"Block 2\"]\n    end");
            assert!(result.is_ok(), "Parse error: {:?}", result.err());
            let db = result.unwrap();
            assert!(db.get_blocks().contains_key("compoundBlock"));
        }
    }

    mod space_parsing {
        use super::*;

        #[test]
        fn should_parse_space() {
            let result =
                parse("block\n    columns 3\n    space\n    middle[\"In the middle\"]\n    space");
            assert!(result.is_ok(), "Parse error: {:?}", result.err());
            let db = result.unwrap();
            // Should have 3 blocks (2 spaces + 1 named)
            assert_eq!(db.get_blocks().len(), 3);
        }
    }

    mod styling_parsing {
        use super::*;

        #[test]
        fn should_parse_class_def() {
            let result = parse("block\n    classDef black color:#ffffff, fill:#000000\n    mc[\"Memcache\"]\n    class mc black");
            assert!(result.is_ok(), "Parse error: {:?}", result.err());
            let db = result.unwrap();
            let classes = db.get_classes();
            assert!(classes.contains_key("black"));
        }

        #[test]
        fn should_parse_style_statement() {
            let result = parse(
                "block\n    columns 1\n    B[\"A wide one\"]\n    style B fill:#f9F,stroke:#333",
            );
            assert!(result.is_ok(), "Parse error: {:?}", result.err());
            let db = result.unwrap();
            let block = db.get_blocks().get("B").unwrap();
            assert!(!block.styles.is_empty());
        }
    }

    mod special_cases {
        use super::*;

        #[test]
        fn should_handle_proto_property() {
            let result = parse("block\n    __proto__");
            assert!(result.is_ok(), "Parse error: {:?}", result.err());
        }

        #[test]
        fn should_handle_constructor_property() {
            let result = parse("block\n    constructor");
            assert!(result.is_ok(), "Parse error: {:?}", result.err());
        }
    }

    mod width_parsing {
        use super::*;

        #[test]
        fn should_parse_width_spec() {
            let result =
                parse("block\n    columns 3\n    one[\"One Slot\"]\n    two[\"Two slots\"]:2");
            assert!(result.is_ok(), "Parse error: {:?}", result.err());
            let db = result.unwrap();
            let two = db.get_blocks().get("two").unwrap();
            assert_eq!(two.width_in_columns, Some(2));
        }
    }

    // Tests ported from mermaid Cypress tests (block.spec.js)
    mod cypress_tests {
        use super::*;

        #[test]
        fn test_cypress_bl1_block_widths() {
            // From Cypress BL1: should calculate the block widths
            let input = r#"block-beta
  columns 2
  block
    id2["I am a wide one"]
    id1
  end
  id["Next row"]"#;
            let result = parse(input);
            assert!(result.is_ok(), "Failed to parse: {:?}", result);
        }

        #[test]
        fn test_cypress_bl2_columns_in_subblocks() {
            // From Cypress BL2: should handle columns statement in sub-blocks
            let input = r#"block
  id1["Hello"]
  block
    columns 3
    id2["to"]
    id3["the"]
    id4["World"]
    id5["World"]
  end"#;
            let result = parse(input);
            assert!(result.is_ok(), "Failed to parse: {:?}", result);
        }

        #[test]
        #[ignore = "TODO: ID format with dots (id2.1) not supported"]
        fn test_cypress_bl3_align_widths() {
            // From Cypress BL3: should align block widths and handle columns statement in sub-blocks
            let input = r#"block
  block
    columns 1
    id1
    id2
    id2.1
  end
  id3
  id4"#;
            let result = parse(input);
            assert!(result.is_ok(), "Failed to parse: {:?}", result);
        }

        #[test]
        #[ignore = "TODO: ID format with dots (id2.1) not supported"]
        fn test_cypress_bl4_deeper_subblocks() {
            // From Cypress BL4: should align block widths and handle columns statements in deeper sub-blocks
            let input = r#"block
  columns 1
  block
    columns 1
    block
      columns 3
      id1
      id2
      id2.1(("XYZ"))
    end
    id48
  end
  id3"#;
            let result = parse(input);
            assert!(result.is_ok(), "Failed to parse: {:?}", result);
        }

        #[test]
        fn test_cypress_bl6_arrows_space() {
            // From Cypress BL6: should handle block arrows and space statements
            let input = r#"block
    columns 3
    space:3
    ida idb idc
    id1  id2"#;
            let result = parse(input);
            assert!(result.is_ok(), "Failed to parse: {:?}", result);
        }
    }
}
