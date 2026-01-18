//! ER diagram parser
//!
//! This module provides parsing for ER diagrams using pest.

use pest::Parser;
use pest_derive::Parser;

use super::types::{
    Attribute, AttributeKey, Cardinality, Direction, ErDb, Identification, RelSpec,
};
use crate::error::{MermaidError, Result};

#[derive(Parser)]
#[grammar = "diagrams/er/er.pest"]
struct ErParser;

/// Parse an ER diagram
pub fn parse(input: &str) -> Result<ErDb> {
    let mut db = ErDb::new();
    parse_into(input, &mut db)?;
    Ok(db)
}

/// Parse into an existing database
pub fn parse_into(input: &str, db: &mut ErDb) -> Result<()> {
    let pairs = ErParser::parse(Rule::diagram, input)
        .map_err(|e| MermaidError::ParseError(e.to_string()))?;

    for pair in pairs {
        if pair.as_rule() == Rule::diagram {
            for inner in pair.into_inner() {
                process_rule(inner, db)?;
            }
        }
    }

    Ok(())
}

fn process_rule(pair: pest::iterators::Pair<Rule>, db: &mut ErDb) -> Result<()> {
    match pair.as_rule() {
        Rule::frontmatter => {
            process_frontmatter(pair, db)?;
        }
        Rule::document => {
            for inner in pair.into_inner() {
                process_rule(inner, db)?;
            }
        }
        Rule::statement => {
            for inner in pair.into_inner() {
                process_rule(inner, db)?;
            }
        }
        Rule::direction_stmt => {
            for inner in pair.into_inner() {
                if inner.as_rule() == Rule::direction {
                    let dir = Direction::from_str(inner.as_str());
                    db.set_direction(dir);
                }
            }
        }
        Rule::acc_title_stmt => {
            for inner in pair.into_inner() {
                if inner.as_rule() == Rule::line_content {
                    db.acc_title = inner.as_str().trim().to_string();
                }
            }
        }
        Rule::acc_descr_stmt => {
            process_acc_descr(pair, db)?;
        }
        Rule::entity_stmt => {
            process_entity_stmt(pair, db)?;
        }
        Rule::entity_with_attrs => {
            process_entity_with_attrs(pair, db)?;
        }
        Rule::relationship_stmt => {
            process_relationship(pair, db)?;
        }
        Rule::class_def_stmt => {
            process_class_def(pair, db)?;
        }
        Rule::class_stmt => {
            process_class_stmt(pair, db)?;
        }
        Rule::style_stmt => {
            process_style_stmt(pair, db)?;
        }
        _ => {}
    }
    Ok(())
}

fn process_acc_descr(pair: pest::iterators::Pair<Rule>, db: &mut ErDb) -> Result<()> {
    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::acc_descr_single => {
                for i in inner.into_inner() {
                    if i.as_rule() == Rule::line_content {
                        db.acc_descr = i.as_str().trim().to_string();
                    }
                }
            }
            Rule::acc_descr_multi => {
                for i in inner.into_inner() {
                    if i.as_rule() == Rule::multiline_content {
                        // Normalize multiline content - trim and normalize newlines
                        let content = i.as_str().trim();
                        db.acc_descr = content.to_string();
                    }
                }
            }
            _ => {}
        }
    }
    Ok(())
}

fn process_frontmatter(pair: pest::iterators::Pair<Rule>, db: &mut ErDb) -> Result<()> {
    for inner in pair.into_inner() {
        if inner.as_rule() == Rule::frontmatter_content {
            for line in inner.into_inner() {
                if line.as_rule() == Rule::frontmatter_line {
                    for item in line.into_inner() {
                        if item.as_rule() == Rule::title_line {
                            for val in item.into_inner() {
                                if val.as_rule() == Rule::frontmatter_value {
                                    db.diagram_title = val.as_str().trim().to_string();
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    Ok(())
}

fn process_entity_stmt(pair: pest::iterators::Pair<Rule>, db: &mut ErDb) -> Result<()> {
    let mut entity_name = String::new();
    let mut alias = None;
    let mut classes = Vec::new();

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::entity_name => {
                entity_name = extract_entity_name(inner.as_str());
            }
            Rule::alias_def => {
                for a in inner.into_inner() {
                    match a.as_rule() {
                        Rule::alias_quoted => {
                            // Strip quotes from alias
                            let raw = a.as_str();
                            alias = Some(
                                raw.strip_prefix('"')
                                    .and_then(|s| s.strip_suffix('"'))
                                    .unwrap_or(raw)
                                    .to_string(),
                            );
                        }
                        Rule::alias_unquoted => {
                            alias = Some(a.as_str().to_string());
                        }
                        _ => {}
                    }
                }
            }
            Rule::class_shorthand => {
                classes = extract_id_list(inner);
            }
            _ => {}
        }
    }

    db.add_entity(&entity_name, alias.as_deref());

    if !classes.is_empty() {
        let class_refs: Vec<&str> = classes.iter().map(|s| s.as_str()).collect();
        db.set_class(&[&entity_name], &class_refs);
    }

    Ok(())
}

fn process_entity_with_attrs(pair: pest::iterators::Pair<Rule>, db: &mut ErDb) -> Result<()> {
    let mut entity_name = String::new();
    let mut alias = None;
    let mut classes = Vec::new();
    let mut attributes = Vec::new();

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::entity_name => {
                entity_name = extract_entity_name(inner.as_str());
            }
            Rule::alias_def => {
                for a in inner.into_inner() {
                    match a.as_rule() {
                        Rule::alias_quoted => {
                            // Strip quotes from alias
                            let raw = a.as_str();
                            alias = Some(
                                raw.strip_prefix('"')
                                    .and_then(|s| s.strip_suffix('"'))
                                    .unwrap_or(raw)
                                    .to_string(),
                            );
                        }
                        Rule::alias_unquoted => {
                            alias = Some(a.as_str().to_string());
                        }
                        _ => {}
                    }
                }
            }
            Rule::class_shorthand => {
                classes = extract_id_list(inner);
            }
            Rule::attribute_block => {
                attributes = process_attribute_block(inner)?;
            }
            _ => {}
        }
    }

    db.add_entity(&entity_name, alias.as_deref());

    if !classes.is_empty() {
        let class_refs: Vec<&str> = classes.iter().map(|s| s.as_str()).collect();
        db.set_class(&[&entity_name], &class_refs);
    }

    if !attributes.is_empty() {
        db.add_attributes(&entity_name, attributes);
    }

    Ok(())
}

fn process_attribute_block(pair: pest::iterators::Pair<Rule>) -> Result<Vec<Attribute>> {
    let mut attributes = Vec::new();

    for inner in pair.into_inner() {
        if inner.as_rule() == Rule::attribute {
            let attr = process_attribute(inner)?;
            attributes.push(attr);
        }
    }

    Ok(attributes)
}

fn process_attribute(pair: pest::iterators::Pair<Rule>) -> Result<Attribute> {
    let mut attr_type = String::new();
    let mut attr_name = String::new();
    let mut keys = Vec::new();
    let mut comment = String::new();

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::attribute_type => {
                attr_type = inner.as_str().to_string();
            }
            Rule::attribute_name => {
                attr_name = inner.as_str().to_string();
            }
            Rule::attribute_key_list => {
                for key_inner in inner.into_inner() {
                    if key_inner.as_rule() == Rule::attribute_key {
                        if let Some(key) = AttributeKey::from_str(key_inner.as_str()) {
                            keys.push(key);
                        }
                    }
                }
            }
            Rule::attribute_comment => {
                // Remove quotes from comment
                let text = inner.as_str();
                comment = text
                    .strip_prefix('"')
                    .and_then(|s| s.strip_suffix('"'))
                    .unwrap_or(text)
                    .to_string();
            }
            _ => {}
        }
    }

    let mut attr = Attribute::new(attr_type, attr_name);
    if !keys.is_empty() {
        attr = attr.with_keys(keys);
    }
    if !comment.is_empty() {
        attr = attr.with_comment(comment);
    }

    Ok(attr)
}

fn process_relationship(pair: pest::iterators::Pair<Rule>, db: &mut ErDb) -> Result<()> {
    let mut entity_a = String::new();
    let mut entity_b = String::new();
    let mut classes_a = Vec::new();
    let mut classes_b = Vec::new();
    let mut rel_spec = None;
    let mut role = String::new();
    let mut entity_count = 0;

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::entity_ref => {
                let (name, classes) = extract_entity_ref(inner);
                if entity_count == 0 {
                    entity_a = name;
                    classes_a = classes;
                } else {
                    entity_b = name;
                    classes_b = classes;
                }
                entity_count += 1;
            }
            Rule::rel_spec => {
                rel_spec = Some(parse_rel_spec(inner)?);
            }
            Rule::role => {
                role = extract_role(inner.as_str());
            }
            _ => {}
        }
    }

    // Add entities
    db.add_entity(&entity_a, None);
    db.add_entity(&entity_b, None);

    // Apply classes if present
    if !classes_a.is_empty() {
        let class_refs: Vec<&str> = classes_a.iter().map(|s| s.as_str()).collect();
        db.set_class(&[&entity_a], &class_refs);
    }
    if !classes_b.is_empty() {
        let class_refs: Vec<&str> = classes_b.iter().map(|s| s.as_str()).collect();
        db.set_class(&[&entity_b], &class_refs);
    }

    // Add relationship
    if let Some(spec) = rel_spec {
        db.add_relationship(&entity_a, &role, &entity_b, spec);
    }

    Ok(())
}

fn extract_entity_ref(pair: pest::iterators::Pair<Rule>) -> (String, Vec<String>) {
    let mut name = String::new();
    let mut classes = Vec::new();

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::entity_name => {
                name = extract_entity_name(inner.as_str());
            }
            Rule::class_shorthand => {
                classes = extract_id_list(inner);
            }
            _ => {}
        }
    }

    (name, classes)
}

fn parse_rel_spec(pair: pest::iterators::Pair<Rule>) -> Result<RelSpec> {
    let mut card_a = Cardinality::OnlyOne;
    let mut card_b = Cardinality::OnlyOne;
    let mut rel_type = Identification::Identifying;

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::cardinality_left => {
                card_b = parse_cardinality_left(inner);
            }
            Rule::cardinality_right => {
                card_a = parse_cardinality_right(inner);
            }
            Rule::rel_type => {
                rel_type = parse_rel_type(inner);
            }
            _ => {}
        }
    }

    Ok(RelSpec::new(card_a, card_b, rel_type))
}

fn parse_cardinality_left(pair: pest::iterators::Pair<Rule>) -> Cardinality {
    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::only_one_left => return Cardinality::OnlyOne,
            Rule::zero_or_one_left => return Cardinality::ZeroOrOne,
            Rule::zero_or_more_left => return Cardinality::ZeroOrMore,
            Rule::one_or_more_left => return Cardinality::OneOrMore,
            Rule::md_parent => return Cardinality::MdParent,
            _ => {}
        }
    }
    Cardinality::OnlyOne
}

fn parse_cardinality_right(pair: pest::iterators::Pair<Rule>) -> Cardinality {
    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::only_one_right => return Cardinality::OnlyOne,
            Rule::zero_or_one_right => return Cardinality::ZeroOrOne,
            Rule::zero_or_more_right => return Cardinality::ZeroOrMore,
            Rule::one_or_more_right => return Cardinality::OneOrMore,
            _ => {}
        }
    }
    Cardinality::OnlyOne
}

fn parse_rel_type(pair: pest::iterators::Pair<Rule>) -> Identification {
    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::identifying => return Identification::Identifying,
            Rule::non_identifying => return Identification::NonIdentifying,
            _ => {}
        }
    }
    Identification::Identifying
}

fn extract_entity_name(s: &str) -> String {
    let trimmed = s.trim();
    if trimmed.starts_with('"') && trimmed.ends_with('"') {
        trimmed[1..trimmed.len() - 1].to_string()
    } else {
        trimmed.to_string()
    }
}

fn extract_role(s: &str) -> String {
    let trimmed = s.trim();
    if trimmed.starts_with('"') && trimmed.ends_with('"') {
        trimmed[1..trimmed.len() - 1].to_string()
    } else {
        trimmed.to_string()
    }
}

fn extract_id_list(pair: pest::iterators::Pair<Rule>) -> Vec<String> {
    let mut ids = Vec::new();
    for inner in pair.into_inner() {
        if inner.as_rule() == Rule::id_list {
            for id_inner in inner.into_inner() {
                if id_inner.as_rule() == Rule::identifier {
                    ids.push(id_inner.as_str().to_string());
                }
            }
        }
    }
    ids
}

fn process_class_def(pair: pest::iterators::Pair<Rule>, db: &mut ErDb) -> Result<()> {
    let mut ids = Vec::new();
    let mut styles_str = String::new();

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::id_list => {
                for id_inner in inner.into_inner() {
                    if id_inner.as_rule() == Rule::identifier {
                        ids.push(id_inner.as_str().to_string());
                    }
                }
            }
            Rule::styles => {
                styles_str = inner.as_str().to_string();
            }
            _ => {}
        }
    }

    // Parse styles (comma-separated, trim whitespace around colons)
    let styles: Vec<String> = styles_str
        .split(',')
        .map(|s| s.trim().replace(": ", ":").replace(" :", ":"))
        .filter(|s| !s.is_empty())
        .collect();

    let id_refs: Vec<&str> = ids.iter().map(|s| s.as_str()).collect();
    let style_refs: Vec<&str> = styles.iter().map(|s| s.as_str()).collect();
    db.add_class(&id_refs, &style_refs);

    Ok(())
}

fn process_class_stmt(pair: pest::iterators::Pair<Rule>, db: &mut ErDb) -> Result<()> {
    let mut id_lists: Vec<Vec<String>> = Vec::new();

    for inner in pair.into_inner() {
        if inner.as_rule() == Rule::id_list {
            let mut ids = Vec::new();
            for id_inner in inner.into_inner() {
                if id_inner.as_rule() == Rule::identifier {
                    ids.push(id_inner.as_str().to_string());
                }
            }
            id_lists.push(ids);
        }
    }

    // First id_list is the entity ids, second is the class names
    if id_lists.len() >= 2 {
        let entity_ids: Vec<&str> = id_lists[0].iter().map(|s| s.as_str()).collect();
        let class_names: Vec<&str> = id_lists[1].iter().map(|s| s.as_str()).collect();
        db.set_class(&entity_ids, &class_names);
    }

    Ok(())
}

fn process_style_stmt(pair: pest::iterators::Pair<Rule>, db: &mut ErDb) -> Result<()> {
    let mut ids = Vec::new();
    let mut styles_str = String::new();

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::id_list => {
                for id_inner in inner.into_inner() {
                    if id_inner.as_rule() == Rule::identifier {
                        ids.push(id_inner.as_str().to_string());
                    }
                }
            }
            Rule::styles => {
                styles_str = inner.as_str().to_string();
            }
            _ => {}
        }
    }

    // Parse styles
    let styles: Vec<String> = styles_str
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    let id_refs: Vec<&str> = ids.iter().map(|s| s.as_str()).collect();
    let style_refs: Vec<&str> = styles.iter().map(|s| s.as_str()).collect();
    db.add_css_styles(&id_refs, &style_refs);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_allow_standalone_entities() {
        let input = "erDiagram\nISLAND\nMAINLAND";
        let result = parse(input);
        assert!(result.is_ok(), "Failed to parse: {:?}", result);

        let db = result.unwrap();
        assert_eq!(db.get_entities().len(), 2);
        assert!(db.get_entities().contains_key("ISLAND"));
        assert!(db.get_entities().contains_key("MAINLAND"));
        assert_eq!(db.get_relationships().len(), 0);
    }

    #[test]
    fn should_allow_entity_with_hyphen_underscore() {
        let input = "erDiagram\nDUCK-BILLED_PLATYPUS";
        let result = parse(input);
        assert!(result.is_ok());

        let db = result.unwrap();
        assert!(db.get_entities().contains_key("DUCK-BILLED_PLATYPUS"));
    }

    #[test]
    fn should_allow_entity_with_alias() {
        let input = "erDiagram\nfoo[\"bar\"]";
        let result = parse(input);
        assert!(result.is_ok(), "Failed to parse: {:?}", result);

        let db = result.unwrap();
        let entity = db.get_entity("foo").unwrap();
        assert_eq!(entity.alias, "bar");
    }

    #[test]
    fn should_allow_entity_starting_with_underscore() {
        let input = "erDiagram\n_foo";
        let result = parse(input);
        assert!(result.is_ok());

        let db = result.unwrap();
        assert!(db.get_entities().contains_key("_foo"));
    }

    #[test]
    fn should_allow_entity_with_single_attribute() {
        let input = "erDiagram\nBOOK {\nstring title\n}";
        let result = parse(input);
        assert!(result.is_ok(), "Failed to parse: {:?}", result);

        let db = result.unwrap();
        let entity = db.get_entity("BOOK").unwrap();
        assert_eq!(entity.attributes.len(), 1);
        assert_eq!(entity.attributes[0].attr_type, "string");
        assert_eq!(entity.attributes[0].name, "title");
    }

    #[test]
    fn should_allow_attribute_with_key() {
        let input = "erDiagram\nBOOK {\nstring title PK\n}";
        let result = parse(input);
        assert!(result.is_ok(), "Failed to parse: {:?}", result);

        let db = result.unwrap();
        let entity = db.get_entity("BOOK").unwrap();
        assert_eq!(entity.attributes.len(), 1);
        assert!(entity.attributes[0]
            .keys
            .contains(&AttributeKey::PrimaryKey));
    }

    #[test]
    fn should_allow_attribute_with_comment() {
        let input = "erDiagram\nBOOK {\nstring title \"comment\"\n}";
        let result = parse(input);
        assert!(result.is_ok(), "Failed to parse: {:?}", result);

        let db = result.unwrap();
        let entity = db.get_entity("BOOK").unwrap();
        assert_eq!(entity.attributes[0].comment, "comment");
    }

    #[test]
    fn should_allow_attribute_with_key_and_comment() {
        let input = "erDiagram\nBOOK {\nstring title PK \"comment\"\n}";
        let result = parse(input);
        assert!(result.is_ok(), "Failed to parse: {:?}", result);

        let db = result.unwrap();
        let entity = db.get_entity("BOOK").unwrap();
        assert!(entity.attributes[0]
            .keys
            .contains(&AttributeKey::PrimaryKey));
        assert_eq!(entity.attributes[0].comment, "comment");
    }

    #[test]
    fn should_allow_multiple_attribute_keys() {
        let input = "erDiagram\nCUSTOMER {\nint customer_number PK,FK \"comment1\"\n}";
        let result = parse(input);
        assert!(result.is_ok(), "Failed to parse: {:?}", result);

        let db = result.unwrap();
        let entity = db.get_entity("CUSTOMER").unwrap();
        assert!(entity.attributes[0]
            .keys
            .contains(&AttributeKey::PrimaryKey));
        assert!(entity.attributes[0]
            .keys
            .contains(&AttributeKey::ForeignKey));
    }

    #[test]
    fn should_allow_generic_type_attribute() {
        let input = "erDiagram\nBOOK {\ntype~T~ type\n}";
        let result = parse(input);
        assert!(result.is_ok(), "Failed to parse: {:?}", result);

        let db = result.unwrap();
        let entity = db.get_entity("BOOK").unwrap();
        assert_eq!(entity.attributes.len(), 1);
    }

    #[test]
    fn should_allow_array_type_attribute() {
        let input = "erDiagram\nBOOK {\nstring[] readers FK \"comment\"\n}";
        let result = parse(input);
        assert!(result.is_ok(), "Failed to parse: {:?}", result);

        let db = result.unwrap();
        let entity = db.get_entity("BOOK").unwrap();
        assert_eq!(entity.attributes[0].attr_type, "string[]");
    }

    #[test]
    fn should_allow_parameterized_type_attribute() {
        let input = "erDiagram\nBOOK {\ncharacter(10) isbn FK\nvarchar(5) postal_code\n}";
        let result = parse(input);
        assert!(result.is_ok(), "Failed to parse: {:?}", result);

        let db = result.unwrap();
        let entity = db.get_entity("BOOK").unwrap();
        assert_eq!(entity.attributes[0].attr_type, "character(10)");
    }

    #[test]
    fn should_allow_empty_attribute_block() {
        let input = "erDiagram\nBOOK {}";
        let result = parse(input);
        assert!(result.is_ok());

        let db = result.unwrap();
        assert!(db.get_entities().contains_key("BOOK"));
        assert_eq!(db.get_entity("BOOK").unwrap().attributes.len(), 0);
    }

    #[test]
    fn should_associate_two_entities() {
        let input = "erDiagram\nCAR ||--o{ DRIVER : \"insured for\"";
        let result = parse(input);
        assert!(result.is_ok(), "Failed to parse: {:?}", result);

        let db = result.unwrap();
        assert!(db.get_entities().contains_key("CAR"));
        assert!(db.get_entities().contains_key("DRIVER"));
        assert_eq!(db.get_relationships().len(), 1);

        let rel = &db.get_relationships()[0];
        assert_eq!(rel.rel_spec.card_a, Cardinality::ZeroOrMore);
        assert_eq!(rel.rel_spec.card_b, Cardinality::OnlyOne);
        assert_eq!(rel.rel_spec.rel_type, Identification::Identifying);
    }

    #[test]
    fn should_not_create_duplicate_entities() {
        let input = "erDiagram\nCAR ||--o{ DRIVER : \"insured for\"\nDRIVER ||--|| LICENSE : has";
        let result = parse(input);
        assert!(result.is_ok());

        let db = result.unwrap();
        assert_eq!(db.get_entities().len(), 3);
    }

    #[test]
    fn should_create_role() {
        let input = "erDiagram\nTEACHER }o--o{ STUDENT : \"is teacher of\"";
        let result = parse(input);
        assert!(result.is_ok());

        let db = result.unwrap();
        let rels = db.get_relationships();
        assert_eq!(rels[0].role_a, "is teacher of");
    }

    #[test]
    fn should_allow_recursive_relationships() {
        let input = "erDiagram\nNODE ||--o{ NODE : \"leads to\"";
        let result = parse(input);
        assert!(result.is_ok());

        let db = result.unwrap();
        assert_eq!(db.get_entities().len(), 1);
    }

    #[test]
    fn should_handle_acc_title_and_descr() {
        let input = "erDiagram\naccTitle: graph title\naccDescr: this graph is about stuff\nA ||--|| B : has";
        let result = parse(input);
        assert!(result.is_ok(), "Failed to parse: {:?}", result);

        let db = result.unwrap();
        assert_eq!(db.acc_title, "graph title");
        assert_eq!(db.acc_descr, "this graph is about stuff");
    }

    #[test]
    fn should_handle_multiline_acc_descr() {
        let input = "erDiagram\naccTitle: graph title\naccDescr { this graph is\nabout\nstuff\n}\nA ||--|| B : has";
        let result = parse(input);
        assert!(result.is_ok(), "Failed to parse: {:?}", result);

        let db = result.unwrap();
        assert_eq!(db.acc_title, "graph title");
        assert!(db.acc_descr.contains("this graph is"));
    }

    // Cardinality tests
    #[test]
    fn should_handle_only_one_to_one_or_more() {
        let input = "erDiagram\nA ||--|{ B : has";
        let result = parse(input);
        assert!(result.is_ok());

        let db = result.unwrap();
        let rels = db.get_relationships();
        assert_eq!(rels[0].rel_spec.card_a, Cardinality::OneOrMore);
        assert_eq!(rels[0].rel_spec.card_b, Cardinality::OnlyOne);
    }

    #[test]
    fn should_handle_only_one_to_zero_or_more() {
        let input = "erDiagram\nA ||..o{ B : has";
        let result = parse(input);
        assert!(result.is_ok());

        let db = result.unwrap();
        let rels = db.get_relationships();
        assert_eq!(rels[0].rel_spec.card_a, Cardinality::ZeroOrMore);
        assert_eq!(rels[0].rel_spec.card_b, Cardinality::OnlyOne);
        assert_eq!(rels[0].rel_spec.rel_type, Identification::NonIdentifying);
    }

    #[test]
    fn should_handle_zero_or_one_to_zero_or_more() {
        let input = "erDiagram\nA |o..o{ B : has";
        let result = parse(input);
        assert!(result.is_ok());

        let db = result.unwrap();
        let rels = db.get_relationships();
        assert_eq!(rels[0].rel_spec.card_a, Cardinality::ZeroOrMore);
        assert_eq!(rels[0].rel_spec.card_b, Cardinality::ZeroOrOne);
    }

    #[test]
    fn should_handle_zero_or_more_to_zero_or_more() {
        let input = "erDiagram\nA }o--o{ B : has";
        let result = parse(input);
        assert!(result.is_ok());

        let db = result.unwrap();
        let rels = db.get_relationships();
        assert_eq!(rels[0].rel_spec.card_a, Cardinality::ZeroOrMore);
        assert_eq!(rels[0].rel_spec.card_b, Cardinality::ZeroOrMore);
    }

    #[test]
    fn should_handle_one_or_more_to_one_or_more() {
        let input = "erDiagram\nA }|..|{ B : has";
        let result = parse(input);
        assert!(result.is_ok());

        let db = result.unwrap();
        let rels = db.get_relationships();
        assert_eq!(rels[0].rel_spec.card_a, Cardinality::OneOrMore);
        assert_eq!(rels[0].rel_spec.card_b, Cardinality::OneOrMore);
    }

    #[test]
    fn should_represent_identifying_relationships() {
        let input = "erDiagram\nHOUSE ||--|{ ROOM : contains";
        let result = parse(input);
        assert!(result.is_ok());

        let db = result.unwrap();
        let rels = db.get_relationships();
        assert_eq!(rels[0].rel_spec.rel_type, Identification::Identifying);
    }

    #[test]
    fn should_represent_non_identifying_relationships() {
        let input = "erDiagram\nPERSON ||..o{ POSSESSION : owns";
        let result = parse(input);
        assert!(result.is_ok());

        let db = result.unwrap();
        let rels = db.get_relationships();
        assert_eq!(rels[0].rel_spec.rel_type, Identification::NonIdentifying);
    }

    #[test]
    fn should_handle_md_parent() {
        let input = "erDiagram\nPROJECT u--o{ TEAM_MEMBER : \"parent\"";
        let result = parse(input);
        assert!(result.is_ok(), "Failed to parse: {:?}", result);

        let db = result.unwrap();
        let rels = db.get_relationships();
        assert_eq!(rels[0].rel_spec.card_b, Cardinality::MdParent);
    }

    #[test]
    fn should_allow_empty_quoted_role() {
        let input = "erDiagram\nCUSTOMER ||--|{ ORDER : \"\"";
        let result = parse(input);
        assert!(result.is_ok());

        let db = result.unwrap();
        let rels = db.get_relationships();
        assert_eq!(rels[0].role_a, "");
    }

    #[test]
    fn should_allow_unquoted_role() {
        let input = "erDiagram\nCUSTOMER ||--|{ ORDER : places";
        let result = parse(input);
        assert!(result.is_ok());

        let db = result.unwrap();
        let rels = db.get_relationships();
        assert_eq!(rels[0].role_a, "places");
    }

    // Class assignment tests
    #[test]
    fn should_apply_style_to_entity() {
        let input = "erDiagram\nCUSTOMER\nstyle CUSTOMER color:red";
        let result = parse(input);
        assert!(result.is_ok(), "Failed to parse: {:?}", result);

        let db = result.unwrap();
        let entity = db.get_entity("CUSTOMER").unwrap();
        assert!(entity.css_styles.contains(&"color:red".to_string()));
    }

    #[test]
    fn should_apply_multiple_styles() {
        let input = "erDiagram\nCUSTOMER\nstyle CUSTOMER color:red,stroke:blue,fill:#f9f";
        let result = parse(input);
        assert!(result.is_ok());

        let db = result.unwrap();
        let entity = db.get_entity("CUSTOMER").unwrap();
        assert_eq!(entity.css_styles.len(), 3);
    }

    #[test]
    fn should_assign_class_to_entity() {
        let input = "erDiagram\nCUSTOMER\nclass CUSTOMER myClass";
        let result = parse(input);
        assert!(result.is_ok());

        let db = result.unwrap();
        let entity = db.get_entity("CUSTOMER").unwrap();
        assert!(entity.css_classes.contains("myClass"));
    }

    #[test]
    fn should_define_class_with_styles() {
        let input = "erDiagram\nclassDef myClass fill:#f9f, stroke: red, color: pink";
        let result = parse(input);
        assert!(result.is_ok(), "Failed to parse: {:?}", result);

        let db = result.unwrap();
        let classes = db.get_classes();
        assert!(classes.contains_key("myClass"));
        let class = classes.get("myClass").unwrap();
        assert!(class.styles.iter().any(|s| s.contains("fill")));
    }

    #[test]
    fn should_assign_class_using_shorthand() {
        let input = "erDiagram\nCUSTOMER:::myClass";
        let result = parse(input);
        assert!(result.is_ok(), "Failed to parse: {:?}", result);

        let db = result.unwrap();
        let entity = db.get_entity("CUSTOMER").unwrap();
        assert!(entity.css_classes.contains("myClass"));
    }

    #[test]
    fn should_assign_class_shorthand_with_empty_block() {
        let input = "erDiagram\nCUSTOMER:::myClass {}";
        let result = parse(input);
        assert!(result.is_ok(), "Failed to parse: {:?}", result);

        let db = result.unwrap();
        let entity = db.get_entity("CUSTOMER").unwrap();
        assert!(entity.css_classes.contains("myClass"));
    }

    #[test]
    fn should_assign_class_shorthand_with_attributes() {
        let input = "erDiagram\nCUSTOMER:::myClass {\nstring name\n}";
        let result = parse(input);
        assert!(result.is_ok(), "Failed to parse: {:?}", result);

        let db = result.unwrap();
        let entity = db.get_entity("CUSTOMER").unwrap();
        assert!(entity.css_classes.contains("myClass"));
        assert_eq!(entity.attributes.len(), 1);
    }

    #[test]
    fn should_assign_class_shorthand_in_relationship() {
        let input = "erDiagram\nCUSTOMER:::myClass ||--o{ PERSON:::myClass : allows";
        let result = parse(input);
        assert!(result.is_ok(), "Failed to parse: {:?}", result);

        let db = result.unwrap();
        assert!(db
            .get_entity("CUSTOMER")
            .unwrap()
            .css_classes
            .contains("myClass"));
        assert!(db
            .get_entity("PERSON")
            .unwrap()
            .css_classes
            .contains("myClass"));
    }

    // Cypress test diagrams
    #[test]
    fn test_cypress_er_basic() {
        let input = r#"erDiagram
          CUSTOMER ||--o{ ORDER : places
          ORDER ||--|{ LINE-ITEM : contains"#;
        let result = parse(input);
        assert!(result.is_ok(), "Failed: {:?}", result.err());
    }

    #[test]
    fn test_cypress_er_self_reference() {
        let input = r#"erDiagram
          CUSTOMER ||..o{ CUSTOMER : refers
          CUSTOMER ||--o{ ORDER : places
          ORDER ||--|{ LINE-ITEM : contains"#;
        let result = parse(input);
        assert!(result.is_ok(), "Failed: {:?}", result.err());
    }

    #[test]
    fn test_cypress_er_with_attributes() {
        let input = r#"erDiagram
          BOOK { string title }
          AUTHOR }|..|{ BOOK : writes
          BOOK { float price }"#;
        let result = parse(input);
        assert!(result.is_ok(), "Failed: {:?}", result.err());
    }

    #[test]
    fn test_cypress_er_generic_types() {
        let input = r#"erDiagram
          BOOK {
            string title
            string[] authors
            type~T~ type
          }"#;
        let result = parse(input);
        assert!(result.is_ok(), "Failed: {:?}", result.err());
    }

    // Prototype pollution tests
    #[test]
    fn should_work_with_proto_property() {
        let input = "erDiagram\n__proto__ ||--|{ ORDER : place";
        let result = parse(input);
        assert!(result.is_ok());
    }

    #[test]
    fn should_work_with_constructor_property() {
        let input = "erDiagram\nconstructor ||--|{ ORDER : place";
        let result = parse(input);
        assert!(result.is_ok());
    }

    // =========================================================================
    // Cypress test ports from mermaid.js erDiagram.spec.js
    // =========================================================================

    #[test]
    fn test_cypress_multiple_relationships_same_entities() {
        // From: "should render an ER diagram with multiple relationships between the same two entities"
        let input = r#"erDiagram
            CUSTOMER ||--|{ ADDRESS : "invoiced at"
            CUSTOMER ||--|{ ADDRESS : "receives goods at""#;
        let result = parse(input);
        assert!(result.is_ok(), "Failed: {:?}", result.err());
        let db = result.unwrap();
        assert_eq!(db.get_relationships().len(), 2);
    }

    #[test]
    fn test_cypress_cyclical_relationships() {
        // From: "should render a cyclical ER diagram"
        let input = r#"erDiagram
            A ||--|{ B : likes
            B ||--|{ C : likes
            C ||--|{ A : likes"#;
        let result = parse(input);
        assert!(result.is_ok(), "Failed: {:?}", result.err());
        let db = result.unwrap();
        assert_eq!(db.get_relationships().len(), 3);
    }

    #[test]
    fn test_cypress_blank_empty_labels() {
        // From: "should render an ER diagram with blank or empty labels"
        let input = r#"erDiagram
            BOOK }|..|{ AUTHOR : ""
            BOOK }|..|{ GENRE : " "
            AUTHOR }|..|{ GENRE : "  ""#;
        let result = parse(input);
        assert!(result.is_ok(), "Failed: {:?}", result.err());
    }

    #[test]
    fn test_cypress_entities_no_relationships() {
        // From: "should render entities that have no relationships"
        let input = r#"erDiagram
            DEAD_PARROT
            HERMIT
            RECLUSE
            SOCIALITE }o--o{ SOCIALITE : "interacts with"
            RECLUSE }o--o{ SOCIALITE : avoids"#;
        let result = parse(input);
        assert!(result.is_ok(), "Failed: {:?}", result.err());
        let db = result.unwrap();
        // Should have all 4 entities
        assert_eq!(db.get_entities().len(), 4);
    }

    #[test]
    fn test_cypress_varchar_length_in_type() {
        // From: "should render entities with length in attributes type"
        let input = r#"erDiagram
            CLUSTER {
              varchar(99) name
              string(255) description
            }"#;
        let result = parse(input);
        assert!(result.is_ok(), "Failed: {:?}", result.err());
        let db = result.unwrap();
        let entity = db.get_entities().get("CLUSTER").unwrap();
        assert_eq!(entity.attributes.len(), 2);
        assert_eq!(entity.attributes[0].attr_type, "varchar(99)");
        assert_eq!(entity.attributes[1].attr_type, "string(255)");
    }

    #[test]
    fn test_cypress_asterisk_prefix_attributes() {
        // From: "should render entities with attributes that begin with asterisk"
        let input = r#"erDiagram
            BOOK {
              int         *id
              string      name
              varchar(99) summary
            }
            BOOK }o..o{ STORE : soldBy
            STORE {
              int         *id
              string      name
              varchar(50) address
            }"#;
        let result = parse(input);
        assert!(result.is_ok(), "Failed: {:?}", result.err());
    }

    #[test]
    fn test_cypress_keys_and_comments() {
        // From: "should render entities with keys and comments"
        let input = r#"erDiagram
          AUTHOR_WITH_LONG_ENTITY_NAME {
            string name PK "comment"
          }
          AUTHOR_WITH_LONG_ENTITY_NAME }|..|{ BOOK : writes
          BOOK {
              string description
              float price "price comment"
              string title PK "title comment"
              string author FK
            }"#;
        let result = parse(input);
        assert!(result.is_ok(), "Failed: {:?}", result.err());
        let db = result.unwrap();
        let book = db.get_entities().get("BOOK").unwrap();
        // Check that title has PK key
        let title_attr = book.attributes.iter().find(|a| a.name == "title").unwrap();
        assert!(title_attr
            .keys
            .contains(&super::super::types::AttributeKey::PrimaryKey));
    }

    #[test]
    fn test_cypress_entity_name_aliases() {
        // From: "should render entities with entity name aliases"
        // TODO: Support unquoted bracket alias syntax like mermaid
        let input = r#"erDiagram
          p[Person] {
            varchar(64) firstName
            varchar(64) lastName
          }
          c["Customer Account"] {
            varchar(128) email
          }
          p ||--o| c : has"#;
        let result = parse(input);
        assert!(result.is_ok(), "Failed: {:?}", result.err());
        let db = result.unwrap();
        // Check alias is parsed
        let person = db.get_entities().get("p").unwrap();
        assert_eq!(person.alias, "Person");
    }

    #[test]
    fn test_cypress_numeric_entity_names() {
        // From: "should render ER diagram with numeric entity names"
        let input = r#"erDiagram
            1 ||--|| ORDER : places
            ORDER ||--|{ 2 : contains
            2 ||--o{ 3.5 : references"#;
        let result = parse(input);
        assert!(result.is_ok(), "Failed: {:?}", result.err());
    }

    #[test]
    fn test_cypress_decimal_numbers_in_relationships() {
        // From: "should render ER diagram with decimal numbers in relationships"
        let input = r#"erDiagram
            2.5 ||--|| 1.5 : has
            CUSTOMER ||--o{ 3.14 : references
            1.0 ||--|{ ORDER : contains"#;
        let result = parse(input);
        assert!(result.is_ok(), "Failed: {:?}", result.err());
    }

    #[test]
    fn test_cypress_standalone_numeric_entities() {
        // From: "should render ER diagram with standalone numeric entities"
        let input = r#"erDiagram
           PRODUCT ||--o{ ORDER-ITEM : has
           1.5
           u
           1"#;
        let result = parse(input);
        assert!(result.is_ok(), "Failed: {:?}", result.err());
        let db = result.unwrap();
        // Should have PRODUCT, ORDER-ITEM, 1.5, u, 1 as entities
        assert!(db.get_entities().contains_key("1.5"));
        assert!(db.get_entities().contains_key("u"));
        assert!(db.get_entities().contains_key("1"));
    }

    #[test]
    fn test_cypress_title_frontmatter() {
        // From: "1433: should render a simple ER diagram with a title"
        // TODO: Support YAML frontmatter like mermaid
        let input = r#"---
title: simple ER diagram
---
erDiagram
CUSTOMER ||--o{ ORDER : places
ORDER ||--|{ LINE-ITEM : contains"#;
        let result = parse(input);
        assert!(result.is_ok(), "Failed: {:?}", result.err());
        let db = result.unwrap();
        assert_eq!(db.diagram_title, "simple ER diagram");
    }

    #[test]
    fn test_cypress_complex_mixed_entity_names() {
        // From: "should render complex ER diagram with mixed special entity names"
        let input = r#"erDiagram
            CUSTOMER ||--o{ 1 : places
            1 ||--|{ u : contains
            1.5
            u ||--|| 2.5 : processes
            2.5 {
              string id
              float value
            }
            u {
              varchar(50) name
              int count
            }"#;
        let result = parse(input);
        assert!(result.is_ok(), "Failed: {:?}", result.err());
    }

    #[test]
    fn test_cypress_relationship_labels_special_chars() {
        // From: "should render edge labels correctly"
        let input = r#"erDiagram
            CUSTOMER ||--o{ ORDER : places
            ORDER ||--|{ LINE-ITEM : contains
            CUSTOMER ||--|{ ADDRESS : "invoiced at"
            CUSTOMER ||--|{ ADDRESS : "receives goods at"
            ORDER ||--o{ INVOICE : "liable for""#;
        let result = parse(input);
        assert!(result.is_ok(), "Failed: {:?}", result.err());
    }

    #[test]
    fn test_cypress_cardinality_aliases() {
        // From: "should render entities with aliases"
        // TODO: Support verbose cardinality aliases like mermaid
        let input = r#"erDiagram
          T1 one or zero to one or more T2 : test
          T2 one or many optionally to zero or one T3 : test
          T3 zero or more to zero or many T4 : test
          T4 many(0) to many(1) T5 : test
          T5 many optionally to one T6 : test
          T6 only one optionally to only one T1 : test
          T4 0+ to 1+ T6 : test
          T1 1 to 1 T3 : test"#;
        let result = parse(input);
        assert!(result.is_ok(), "Failed: {:?}", result.err());
    }
}
