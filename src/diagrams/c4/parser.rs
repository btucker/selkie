//! C4 diagram parser using pest grammar

use pest::Parser;
use pest_derive::Parser;

use super::types::{C4Db, C4ShapeType};

#[derive(Parser)]
#[grammar = "diagrams/c4/c4.pest"]
pub struct C4Parser;

/// Parse a C4 diagram and return the populated database
pub fn parse(input: &str) -> Result<C4Db, String> {
    let mut db = C4Db::new();

    let pairs = C4Parser::parse(Rule::diagram, input).map_err(|e| format!("Parse error: {}", e))?;

    for pair in pairs {
        if pair.as_rule() == Rule::diagram {
            for inner in pair.into_inner() {
                match inner.as_rule() {
                    Rule::c4_type => {
                        // Store the C4 type (Context, Container, Component, etc.)
                        // Could be used for validation
                    }
                    Rule::document => {
                        process_document(&mut db, inner)?;
                    }
                    _ => {}
                }
            }
        }
    }

    Ok(db)
}

fn process_document(db: &mut C4Db, pair: pest::iterators::Pair<Rule>) -> Result<(), String> {
    for stmt in pair.into_inner() {
        process_statement(db, stmt)?;
    }
    Ok(())
}

fn process_statement(db: &mut C4Db, pair: pest::iterators::Pair<Rule>) -> Result<(), String> {
    match pair.as_rule() {
        Rule::statement => {
            for inner in pair.into_inner() {
                process_statement(db, inner)?;
            }
        }
        Rule::comment_stmt => {
            // Ignore comments
        }
        Rule::title_stmt => {
            for inner in pair.into_inner() {
                if inner.as_rule() == Rule::title_text {
                    // Title could be stored in db if needed
                }
            }
        }
        Rule::direction_stmt => {
            // Direction could be stored in db if needed
        }
        // Person
        Rule::person_stmt => {
            let attrs = extract_all_attributes(pair);
            if !attrs.is_empty() {
                db.add_person_with_type(
                    &attrs.first().cloned().unwrap_or_default(),
                    &attrs.get(1).cloned().unwrap_or_default(),
                    &attrs.get(2).cloned().unwrap_or_default(),
                    C4ShapeType::Person,
                );
            }
        }
        Rule::person_ext_stmt => {
            let attrs = extract_all_attributes(pair);
            if !attrs.is_empty() {
                db.add_person_with_type(
                    &attrs.first().cloned().unwrap_or_default(),
                    &attrs.get(1).cloned().unwrap_or_default(),
                    &attrs.get(2).cloned().unwrap_or_default(),
                    C4ShapeType::PersonExt,
                );
            }
        }
        // System
        Rule::system_stmt => {
            let attrs = extract_all_attributes(pair);
            if !attrs.is_empty() {
                db.add_system_with_type(
                    &attrs.first().cloned().unwrap_or_default(),
                    &attrs.get(1).cloned().unwrap_or_default(),
                    &attrs.get(2).cloned().unwrap_or_default(),
                    C4ShapeType::System,
                );
            }
        }
        Rule::system_db_stmt => {
            let attrs = extract_all_attributes(pair);
            if !attrs.is_empty() {
                db.add_system_with_type(
                    &attrs.first().cloned().unwrap_or_default(),
                    &attrs.get(1).cloned().unwrap_or_default(),
                    &attrs.get(2).cloned().unwrap_or_default(),
                    C4ShapeType::SystemDb,
                );
            }
        }
        Rule::system_queue_stmt => {
            let attrs = extract_all_attributes(pair);
            if !attrs.is_empty() {
                db.add_system_with_type(
                    &attrs.first().cloned().unwrap_or_default(),
                    &attrs.get(1).cloned().unwrap_or_default(),
                    &attrs.get(2).cloned().unwrap_or_default(),
                    C4ShapeType::SystemQueue,
                );
            }
        }
        Rule::system_ext_stmt => {
            let attrs = extract_all_attributes(pair);
            if !attrs.is_empty() {
                db.add_system_with_type(
                    &attrs.first().cloned().unwrap_or_default(),
                    &attrs.get(1).cloned().unwrap_or_default(),
                    &attrs.get(2).cloned().unwrap_or_default(),
                    C4ShapeType::SystemExt,
                );
            }
        }
        Rule::system_ext_db_stmt => {
            let attrs = extract_all_attributes(pair);
            if !attrs.is_empty() {
                db.add_system_with_type(
                    &attrs.first().cloned().unwrap_or_default(),
                    &attrs.get(1).cloned().unwrap_or_default(),
                    &attrs.get(2).cloned().unwrap_or_default(),
                    C4ShapeType::SystemDbExt,
                );
            }
        }
        Rule::system_ext_queue_stmt => {
            let attrs = extract_all_attributes(pair);
            if !attrs.is_empty() {
                db.add_system_with_type(
                    &attrs.first().cloned().unwrap_or_default(),
                    &attrs.get(1).cloned().unwrap_or_default(),
                    &attrs.get(2).cloned().unwrap_or_default(),
                    C4ShapeType::SystemQueueExt,
                );
            }
        }
        // Container
        Rule::container_stmt => {
            let attrs = extract_all_attributes(pair);
            if !attrs.is_empty() {
                db.add_container_with_type(
                    &attrs.first().cloned().unwrap_or_default(),
                    &attrs.get(1).cloned().unwrap_or_default(),
                    &attrs.get(2).cloned().unwrap_or_default(),
                    &attrs.get(3).cloned().unwrap_or_default(),
                    C4ShapeType::Container,
                );
            }
        }
        Rule::container_db_stmt => {
            let attrs = extract_all_attributes(pair);
            if !attrs.is_empty() {
                db.add_container_with_type(
                    &attrs.first().cloned().unwrap_or_default(),
                    &attrs.get(1).cloned().unwrap_or_default(),
                    &attrs.get(2).cloned().unwrap_or_default(),
                    &attrs.get(3).cloned().unwrap_or_default(),
                    C4ShapeType::ContainerDb,
                );
            }
        }
        Rule::container_queue_stmt => {
            let attrs = extract_all_attributes(pair);
            if !attrs.is_empty() {
                db.add_container_with_type(
                    &attrs.first().cloned().unwrap_or_default(),
                    &attrs.get(1).cloned().unwrap_or_default(),
                    &attrs.get(2).cloned().unwrap_or_default(),
                    &attrs.get(3).cloned().unwrap_or_default(),
                    C4ShapeType::ContainerQueue,
                );
            }
        }
        Rule::container_ext_stmt => {
            let attrs = extract_all_attributes(pair);
            if !attrs.is_empty() {
                db.add_container_with_type(
                    &attrs.first().cloned().unwrap_or_default(),
                    &attrs.get(1).cloned().unwrap_or_default(),
                    &attrs.get(2).cloned().unwrap_or_default(),
                    &attrs.get(3).cloned().unwrap_or_default(),
                    C4ShapeType::ContainerExt,
                );
            }
        }
        Rule::container_ext_db_stmt => {
            let attrs = extract_all_attributes(pair);
            if !attrs.is_empty() {
                db.add_container_with_type(
                    &attrs.first().cloned().unwrap_or_default(),
                    &attrs.get(1).cloned().unwrap_or_default(),
                    &attrs.get(2).cloned().unwrap_or_default(),
                    &attrs.get(3).cloned().unwrap_or_default(),
                    C4ShapeType::ContainerDbExt,
                );
            }
        }
        Rule::container_ext_queue_stmt => {
            let attrs = extract_all_attributes(pair);
            if !attrs.is_empty() {
                db.add_container_with_type(
                    &attrs.first().cloned().unwrap_or_default(),
                    &attrs.get(1).cloned().unwrap_or_default(),
                    &attrs.get(2).cloned().unwrap_or_default(),
                    &attrs.get(3).cloned().unwrap_or_default(),
                    C4ShapeType::ContainerQueueExt,
                );
            }
        }
        // Component
        Rule::component_stmt => {
            let attrs = extract_all_attributes(pair);
            if !attrs.is_empty() {
                db.add_component_with_type(
                    &attrs.first().cloned().unwrap_or_default(),
                    &attrs.get(1).cloned().unwrap_or_default(),
                    &attrs.get(2).cloned().unwrap_or_default(),
                    &attrs.get(3).cloned().unwrap_or_default(),
                    C4ShapeType::Component,
                );
            }
        }
        Rule::component_db_stmt => {
            let attrs = extract_all_attributes(pair);
            if !attrs.is_empty() {
                db.add_component_with_type(
                    &attrs.first().cloned().unwrap_or_default(),
                    &attrs.get(1).cloned().unwrap_or_default(),
                    &attrs.get(2).cloned().unwrap_or_default(),
                    &attrs.get(3).cloned().unwrap_or_default(),
                    C4ShapeType::ComponentDb,
                );
            }
        }
        Rule::component_queue_stmt => {
            let attrs = extract_all_attributes(pair);
            if !attrs.is_empty() {
                db.add_component_with_type(
                    &attrs.first().cloned().unwrap_or_default(),
                    &attrs.get(1).cloned().unwrap_or_default(),
                    &attrs.get(2).cloned().unwrap_or_default(),
                    &attrs.get(3).cloned().unwrap_or_default(),
                    C4ShapeType::ComponentQueue,
                );
            }
        }
        Rule::component_ext_stmt => {
            let attrs = extract_all_attributes(pair);
            if !attrs.is_empty() {
                db.add_component_with_type(
                    &attrs.first().cloned().unwrap_or_default(),
                    &attrs.get(1).cloned().unwrap_or_default(),
                    &attrs.get(2).cloned().unwrap_or_default(),
                    &attrs.get(3).cloned().unwrap_or_default(),
                    C4ShapeType::ComponentExt,
                );
            }
        }
        Rule::component_ext_db_stmt => {
            let attrs = extract_all_attributes(pair);
            if !attrs.is_empty() {
                db.add_component_with_type(
                    &attrs.first().cloned().unwrap_or_default(),
                    &attrs.get(1).cloned().unwrap_or_default(),
                    &attrs.get(2).cloned().unwrap_or_default(),
                    &attrs.get(3).cloned().unwrap_or_default(),
                    C4ShapeType::ComponentDbExt,
                );
            }
        }
        Rule::component_ext_queue_stmt => {
            let attrs = extract_all_attributes(pair);
            if !attrs.is_empty() {
                db.add_component_with_type(
                    &attrs.first().cloned().unwrap_or_default(),
                    &attrs.get(1).cloned().unwrap_or_default(),
                    &attrs.get(2).cloned().unwrap_or_default(),
                    &attrs.get(3).cloned().unwrap_or_default(),
                    C4ShapeType::ComponentQueueExt,
                );
            }
        }
        // Boundaries
        Rule::boundary_block => {
            process_boundary(db, pair, "boundary")?;
        }
        Rule::enterprise_boundary_block => {
            process_boundary(db, pair, "enterprise")?;
        }
        Rule::system_boundary_block => {
            process_boundary(db, pair, "system")?;
        }
        Rule::container_boundary_block => {
            process_boundary(db, pair, "container")?;
        }
        // Relationships
        Rule::rel_stmt | Rule::birel_stmt | Rule::rel_direction_stmt => {
            let attrs = extract_all_attributes(pair);
            if attrs.len() >= 2 {
                db.add_relationship(
                    &attrs.first().cloned().unwrap_or_default(),
                    &attrs.get(1).cloned().unwrap_or_default(),
                    &attrs.get(2).cloned().unwrap_or_default(),
                    &attrs.get(3).cloned().unwrap_or_default(),
                );
            }
        }
        _ => {}
    }
    Ok(())
}

fn process_boundary(
    db: &mut C4Db,
    pair: pest::iterators::Pair<Rule>,
    boundary_type: &str,
) -> Result<(), String> {
    let mut attrs: Vec<String> = Vec::new();
    let mut inner_statements: Vec<pest::iterators::Pair<Rule>> = Vec::new();

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::attributes => {
                attrs = extract_attributes(inner);
            }
            Rule::statement => {
                inner_statements.push(inner);
            }
            _ => {}
        }
    }

    // Start the boundary
    let alias = attrs.first().cloned().unwrap_or_default();
    let label = attrs.get(1).cloned().unwrap_or_default();
    db.start_boundary(&alias, &label, boundary_type);

    // Process inner statements
    for stmt in inner_statements {
        process_statement(db, stmt)?;
    }

    // End the boundary
    db.end_boundary();

    Ok(())
}

fn extract_all_attributes(pair: pest::iterators::Pair<Rule>) -> Vec<String> {
    let mut result = Vec::new();
    for inner in pair.into_inner() {
        if inner.as_rule() == Rule::attributes {
            result = extract_attributes(inner);
            break;
        }
    }
    result
}

fn extract_attributes(pair: pest::iterators::Pair<Rule>) -> Vec<String> {
    let mut attrs = Vec::new();
    for inner in pair.into_inner() {
        if inner.as_rule() == Rule::attribute {
            attrs.push(extract_attribute(inner));
        }
    }
    attrs
}

fn extract_attribute(pair: pest::iterators::Pair<Rule>) -> String {
    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::quoted_string => {
                return unquote(inner.as_str());
            }
            Rule::unquoted_attribute => {
                return inner.as_str().trim().to_string();
            }
            Rule::kv_attribute => {
                // For key-value attributes, just return the value part
                for kv_inner in inner.into_inner() {
                    if kv_inner.as_rule() == Rule::quoted_string {
                        return unquote(kv_inner.as_str());
                    }
                }
            }
            _ => {}
        }
    }
    String::new()
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
        fn should_parse_c4_context() {
            let result = parse("C4Context");
            assert!(result.is_ok(), "Parse error: {:?}", result.err());
        }

        #[test]
        fn should_parse_c4_container() {
            let result = parse("C4Container");
            assert!(result.is_ok(), "Parse error: {:?}", result.err());
        }

        #[test]
        fn should_parse_c4_component() {
            let result = parse("C4Component");
            assert!(result.is_ok(), "Parse error: {:?}", result.err());
        }

        #[test]
        fn should_handle_trailing_whitespace() {
            let result = parse("C4Context \ntitle System Context diagram \nPerson(customerA, \"Banking Customer A\", \"A customer of the bank.\") ");
            assert!(result.is_ok(), "Parse error: {:?}", result.err());
        }
    }

    mod person_parsing {
        use super::*;

        #[test]
        fn should_parse_person() {
            let input = r#"C4Context
title System Context diagram for Internet Banking System
Person(customerA, "Banking Customer A", "A customer of the bank, with personal bank accounts.")"#;

            let result = parse(input);
            assert!(result.is_ok(), "Parse error: {:?}", result.err());

            let db = result.unwrap();
            let elements = db.get_elements();
            assert_eq!(elements.len(), 1);
            assert_eq!(elements[0].alias, "customerA");
            assert_eq!(elements[0].label, "Banking Customer A");
            assert_eq!(
                elements[0].description,
                "A customer of the bank, with personal bank accounts."
            );
            assert_eq!(elements[0].shape_type, C4ShapeType::Person);
        }

        #[test]
        fn should_parse_person_ext() {
            let input = r#"C4Context
Person_Ext(customerA, "Banking Customer A")"#;

            let result = parse(input);
            assert!(result.is_ok(), "Parse error: {:?}", result.err());

            let db = result.unwrap();
            let elements = db.get_elements();
            assert_eq!(elements[0].shape_type, C4ShapeType::PersonExt);
        }
    }

    mod system_parsing {
        use super::*;

        #[test]
        fn should_parse_system() {
            let input = r#"C4Context
System(SystemAA, "Internet Banking System", "Allows customers to view information.")"#;

            let result = parse(input);
            assert!(result.is_ok(), "Parse error: {:?}", result.err());

            let db = result.unwrap();
            let elements = db.get_elements();
            assert_eq!(elements.len(), 1);
            assert_eq!(elements[0].alias, "SystemAA");
            assert_eq!(elements[0].shape_type, C4ShapeType::System);
        }

        #[test]
        fn should_parse_system_db() {
            let input = r#"C4Context
SystemDb(db, "Database")"#;

            let result = parse(input);
            assert!(result.is_ok(), "Parse error: {:?}", result.err());

            let db = result.unwrap();
            let elements = db.get_elements();
            assert_eq!(elements[0].shape_type, C4ShapeType::SystemDb);
        }

        #[test]
        fn should_parse_system_queue() {
            let input = r#"C4Context
SystemQueue(queue, "Message Queue")"#;

            let result = parse(input);
            assert!(result.is_ok(), "Parse error: {:?}", result.err());

            let db = result.unwrap();
            let elements = db.get_elements();
            assert_eq!(elements[0].shape_type, C4ShapeType::SystemQueue);
        }

        #[test]
        fn should_parse_system_ext() {
            let input = r#"C4Context
System_Ext(ext, "External System")"#;

            let result = parse(input);
            assert!(result.is_ok(), "Parse error: {:?}", result.err());

            let db = result.unwrap();
            let elements = db.get_elements();
            assert_eq!(elements[0].shape_type, C4ShapeType::SystemExt);
        }
    }

    mod container_parsing {
        use super::*;

        #[test]
        fn should_parse_container() {
            let input = r#"C4Container
Container(api, "API Application", "Node.js", "Provides Internet banking functionality")"#;

            let result = parse(input);
            assert!(result.is_ok(), "Parse error: {:?}", result.err());

            let db = result.unwrap();
            let elements = db.get_elements();
            assert_eq!(elements.len(), 1);
            assert_eq!(elements[0].alias, "api");
            assert_eq!(elements[0].technology, "Node.js");
            assert_eq!(elements[0].shape_type, C4ShapeType::Container);
        }

        #[test]
        fn should_parse_container_db() {
            let input = r#"C4Container
ContainerDb(db, "Database", "PostgreSQL")"#;

            let result = parse(input);
            assert!(result.is_ok(), "Parse error: {:?}", result.err());

            let db = result.unwrap();
            let elements = db.get_elements();
            assert_eq!(elements[0].shape_type, C4ShapeType::ContainerDb);
        }
    }

    mod component_parsing {
        use super::*;

        #[test]
        fn should_parse_component() {
            let input = r#"C4Component
Component(auth, "Auth Service", "Python", "Handles authentication")"#;

            let result = parse(input);
            assert!(result.is_ok(), "Parse error: {:?}", result.err());

            let db = result.unwrap();
            let elements = db.get_elements();
            assert_eq!(elements[0].shape_type, C4ShapeType::Component);
        }
    }

    mod boundary_parsing {
        use super::*;

        #[test]
        fn should_parse_boundary() {
            let input = r#"C4Context
Boundary(b1, "BankBoundary") {
System(SystemAA, "Internet Banking System")
}"#;

            let result = parse(input);
            assert!(result.is_ok(), "Parse error: {:?}", result.err());

            let db = result.unwrap();
            let boundaries = db.get_boundaries();
            assert_eq!(boundaries.len(), 1);
            assert_eq!(boundaries[0].alias, "b1");
            assert_eq!(boundaries[0].label, "BankBoundary");

            // Element should be inside the boundary
            let elements = db.get_elements();
            assert_eq!(elements[0].parent_boundary, "b1");
        }

        #[test]
        fn should_parse_system_boundary() {
            let input = r#"C4Context
System_Boundary(sb, "System Boundary") {
System(SystemAA, "Internet Banking System")
}"#;

            let result = parse(input);
            assert!(result.is_ok(), "Parse error: {:?}", result.err());

            let db = result.unwrap();
            let boundaries = db.get_boundaries();
            assert_eq!(boundaries[0].boundary_type, "system");
        }

        #[test]
        fn should_parse_enterprise_boundary() {
            let input = r#"C4Context
Enterprise_Boundary(eb, "Enterprise") {
System(SystemAA, "Internet Banking System")
}"#;

            let result = parse(input);
            assert!(result.is_ok(), "Parse error: {:?}", result.err());

            let db = result.unwrap();
            let boundaries = db.get_boundaries();
            assert_eq!(boundaries[0].boundary_type, "enterprise");
        }
    }

    mod relationship_parsing {
        use super::*;

        #[test]
        fn should_parse_rel() {
            let input = r#"C4Context
Person(user, "User")
System(sys, "System")
Rel(user, sys, "Uses", "HTTPS")"#;

            let result = parse(input);
            assert!(result.is_ok(), "Parse error: {:?}", result.err());

            let db = result.unwrap();
            let rels = db.get_relationships();
            assert_eq!(rels.len(), 1);
            assert_eq!(rels[0].from, "user");
            assert_eq!(rels[0].to, "sys");
            assert_eq!(rels[0].label, "Uses");
            assert_eq!(rels[0].technology, "HTTPS");
        }

        #[test]
        fn should_parse_birel() {
            let input = r#"C4Context
Person(user, "User")
System(sys, "System")
BiRel(user, sys, "Communicates")"#;

            let result = parse(input);
            assert!(result.is_ok(), "Parse error: {:?}", result.err());

            let db = result.unwrap();
            let rels = db.get_relationships();
            assert_eq!(rels.len(), 1);
        }
    }

    mod special_cases {
        use super::*;

        #[test]
        fn should_parse_keyword_as_parameter() {
            let input = r#"C4Context
title title
Person(Person, "Person", "Person")"#;

            let result = parse(input);
            assert!(result.is_ok(), "Parse error: {:?}", result.err());

            let db = result.unwrap();
            let elements = db.get_elements();
            assert_eq!(elements[0].alias, "Person");
            assert_eq!(elements[0].label, "Person");
            assert_eq!(elements[0].description, "Person");
        }

        #[test]
        fn should_allow_default_in_parameters() {
            let input = r#"C4Context
Person(default, "default", "default")"#;

            let result = parse(input);
            assert!(result.is_ok(), "Parse error: {:?}", result.err());

            let db = result.unwrap();
            let elements = db.get_elements();
            assert_eq!(elements[0].alias, "default");
        }
    }
}
