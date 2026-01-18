//! Requirement diagram parser using pest grammar

use pest::Parser;
use pest_derive::Parser;

use super::types::RequirementDb;

#[derive(Parser)]
#[grammar = "diagrams/requirement/requirement.pest"]
pub struct RequirementParser;

/// Parse a requirement diagram and return the populated database
pub fn parse(input: &str) -> Result<RequirementDb, String> {
    let mut db = RequirementDb::new();

    let pairs = RequirementParser::parse(Rule::diagram, input)
        .map_err(|e| format!("Parse error: {}", e))?;

    for pair in pairs {
        if pair.as_rule() == Rule::diagram {
            for inner in pair.into_inner() {
                if inner.as_rule() == Rule::document {
                    process_document(&mut db, inner)?;
                }
            }
        }
    }

    Ok(db)
}

fn process_document(
    db: &mut RequirementDb,
    pair: pest::iterators::Pair<Rule>,
) -> Result<(), String> {
    for stmt in pair.into_inner() {
        process_statement(db, stmt)?;
    }
    Ok(())
}

fn process_statement(
    db: &mut RequirementDb,
    pair: pest::iterators::Pair<Rule>,
) -> Result<(), String> {
    match pair.as_rule() {
        Rule::statement => {
            for inner in pair.into_inner() {
                process_statement(db, inner)?;
            }
        }
        Rule::comment_stmt => {
            // Ignore comments
        }
        Rule::acc_title_stmt => {
            // TODO: Set accessibility title
        }
        Rule::acc_descr_stmt | Rule::acc_descr_multiline_stmt => {
            // TODO: Set accessibility description
        }
        Rule::direction_stmt => {
            for inner in pair.into_inner() {
                if inner.as_rule() == Rule::direction_value {
                    db.set_direction(inner.as_str().to_uppercase().as_str());
                }
            }
        }
        Rule::requirement_def => {
            process_requirement(db, pair)?;
        }
        Rule::element_def => {
            process_element(db, pair)?;
        }
        Rule::relationship_def => {
            process_relationship(db, pair)?;
        }
        Rule::style_stmt => {
            process_style(db, pair)?;
        }
        Rule::class_def_stmt => {
            process_class_def(db, pair)?;
        }
        Rule::class_stmt => {
            process_class_assignment(db, pair)?;
        }
        Rule::class_shorthand_stmt => {
            process_class_shorthand(db, pair)?;
        }
        _ => {}
    }
    Ok(())
}

fn process_requirement(
    db: &mut RequirementDb,
    pair: pest::iterators::Pair<Rule>,
) -> Result<(), String> {
    let mut req_type = String::new();
    let mut req_name = String::new();
    let mut classes: Vec<String> = Vec::new();

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::requirement_type => {
                req_type = match inner.as_str().to_lowercase().as_str() {
                    "functionalrequirement" => "Functional Requirement".to_string(),
                    "interfacerequirement" => "Interface Requirement".to_string(),
                    "performancerequirement" => "Performance Requirement".to_string(),
                    "physicalrequirement" => "Physical Requirement".to_string(),
                    "designconstraint" => "Design Constraint".to_string(),
                    _ => "Requirement".to_string(),
                };
            }
            Rule::requirement_name => {
                req_name = extract_name(inner);
            }
            Rule::class_ref => {
                classes = extract_id_list(inner);
            }
            Rule::requirement_body => {
                // Add requirement first so we can set its attributes
                db.add_requirement(&req_name, &req_type);

                // Apply classes if any
                if !classes.is_empty() {
                    let class_refs: Vec<&str> = classes.iter().map(|s| s.as_str()).collect();
                    db.set_class(&[&req_name], &class_refs);
                }

                // Process body attributes
                for attr in inner.into_inner() {
                    process_requirement_attr(db, &req_name, attr)?;
                }
            }
            _ => {}
        }
    }

    Ok(())
}

fn process_requirement_attr(
    db: &mut RequirementDb,
    req_name: &str,
    pair: pest::iterators::Pair<Rule>,
) -> Result<(), String> {
    match pair.as_rule() {
        Rule::requirement_attr => {
            for inner in pair.into_inner() {
                process_requirement_attr(db, req_name, inner)?;
            }
        }
        Rule::id_attr => {
            for inner in pair.into_inner() {
                if inner.as_rule() == Rule::attr_value {
                    db.set_req_id(req_name, inner.as_str().trim());
                }
            }
        }
        Rule::text_attr => {
            for inner in pair.into_inner() {
                if inner.as_rule() == Rule::attr_value {
                    db.set_req_text(req_name, inner.as_str().trim());
                }
            }
        }
        Rule::risk_attr => {
            for inner in pair.into_inner() {
                if inner.as_rule() == Rule::risk_level {
                    db.set_req_risk(req_name, inner.as_str().trim());
                }
            }
        }
        Rule::verify_method_attr => {
            for inner in pair.into_inner() {
                if inner.as_rule() == Rule::verify_type {
                    db.set_req_verify_method(req_name, inner.as_str().trim());
                }
            }
        }
        _ => {}
    }
    Ok(())
}

fn process_element(
    db: &mut RequirementDb,
    pair: pest::iterators::Pair<Rule>,
) -> Result<(), String> {
    let mut elem_name = String::new();
    let mut classes: Vec<String> = Vec::new();

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::element_name => {
                elem_name = extract_name(inner);
            }
            Rule::class_ref => {
                classes = extract_id_list(inner);
            }
            Rule::element_body => {
                // Add element first so we can set its attributes
                db.add_element(&elem_name);

                // Apply classes if any
                if !classes.is_empty() {
                    let class_refs: Vec<&str> = classes.iter().map(|s| s.as_str()).collect();
                    db.set_class(&[&elem_name], &class_refs);
                }

                // Process body attributes
                for attr in inner.into_inner() {
                    process_element_attr(db, &elem_name, attr)?;
                }
            }
            _ => {}
        }
    }

    Ok(())
}

fn process_element_attr(
    db: &mut RequirementDb,
    elem_name: &str,
    pair: pest::iterators::Pair<Rule>,
) -> Result<(), String> {
    match pair.as_rule() {
        Rule::element_attr => {
            for inner in pair.into_inner() {
                process_element_attr(db, elem_name, inner)?;
            }
        }
        Rule::type_attr => {
            for inner in pair.into_inner() {
                if inner.as_rule() == Rule::attr_value {
                    db.set_element_type(elem_name, inner.as_str().trim());
                }
            }
        }
        Rule::docref_attr => {
            for inner in pair.into_inner() {
                if inner.as_rule() == Rule::attr_value {
                    db.set_element_docref(elem_name, inner.as_str().trim());
                }
            }
        }
        _ => {}
    }
    Ok(())
}

fn process_relationship(
    db: &mut RequirementDb,
    pair: pest::iterators::Pair<Rule>,
) -> Result<(), String> {
    let mut src = String::new();
    let mut dst = String::new();
    let mut rel_type = String::new();

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::forward_relationship => {
                // src - type -> dst
                for rel_inner in inner.into_inner() {
                    match rel_inner.as_rule() {
                        Rule::relationship_src => {
                            src = extract_id(rel_inner);
                        }
                        Rule::relationship_dst => {
                            dst = extract_id(rel_inner);
                        }
                        Rule::relationship_type => {
                            rel_type = rel_inner.as_str().to_lowercase();
                        }
                        _ => {}
                    }
                }
            }
            Rule::reverse_relationship => {
                // dst <- type - src (note: order is reversed in the syntax)
                for rel_inner in inner.into_inner() {
                    match rel_inner.as_rule() {
                        Rule::relationship_src => {
                            src = extract_id(rel_inner);
                        }
                        Rule::relationship_dst => {
                            dst = extract_id(rel_inner);
                        }
                        Rule::relationship_type => {
                            rel_type = rel_inner.as_str().to_lowercase();
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }

    db.add_relationship(&rel_type, &src, &dst);
    Ok(())
}

fn process_style(db: &mut RequirementDb, pair: pest::iterators::Pair<Rule>) -> Result<(), String> {
    let mut ids: Vec<String> = Vec::new();
    let mut styles: Vec<String> = Vec::new();

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::id_list => {
                ids = extract_id_list(inner);
            }
            Rule::style_list => {
                // Split styles by comma
                styles = inner
                    .as_str()
                    .trim()
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .collect();
            }
            _ => {}
        }
    }

    let id_refs: Vec<&str> = ids.iter().map(|s| s.as_str()).collect();
    let style_refs: Vec<&str> = styles.iter().map(|s| s.as_str()).collect();
    db.set_css_style(&id_refs, &style_refs);
    Ok(())
}

fn process_class_def(
    db: &mut RequirementDb,
    pair: pest::iterators::Pair<Rule>,
) -> Result<(), String> {
    let mut ids: Vec<String> = Vec::new();
    let mut styles: Vec<String> = Vec::new();

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::id_list => {
                ids = extract_id_list(inner);
            }
            Rule::style_list => {
                // Split styles by comma
                styles = inner
                    .as_str()
                    .trim()
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .collect();
            }
            _ => {}
        }
    }

    let id_refs: Vec<&str> = ids.iter().map(|s| s.as_str()).collect();
    let style_refs: Vec<&str> = styles.iter().map(|s| s.as_str()).collect();
    db.define_class(&id_refs, &style_refs);
    Ok(())
}

fn process_class_assignment(
    db: &mut RequirementDb,
    pair: pest::iterators::Pair<Rule>,
) -> Result<(), String> {
    let mut id_lists: Vec<Vec<String>> = Vec::new();

    for inner in pair.into_inner() {
        if inner.as_rule() == Rule::id_list {
            id_lists.push(extract_id_list(inner));
        }
    }

    if id_lists.len() >= 2 {
        let ids = &id_lists[0];
        let class_names = &id_lists[1];

        let id_refs: Vec<&str> = ids.iter().map(|s| s.as_str()).collect();
        let class_refs: Vec<&str> = class_names.iter().map(|s| s.as_str()).collect();
        db.set_class(&id_refs, &class_refs);
    }

    Ok(())
}

fn process_class_shorthand(
    db: &mut RequirementDb,
    pair: pest::iterators::Pair<Rule>,
) -> Result<(), String> {
    let mut id = String::new();
    let mut classes: Vec<String> = Vec::new();

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::identifier => {
                id = inner.as_str().to_string();
            }
            Rule::id_list => {
                classes = extract_id_list(inner);
            }
            _ => {}
        }
    }

    let class_refs: Vec<&str> = classes.iter().map(|s| s.as_str()).collect();
    db.set_class(&[&id], &class_refs);
    Ok(())
}

fn extract_name(pair: pest::iterators::Pair<Rule>) -> String {
    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::quoted_string => {
                return unquote(inner.as_str());
            }
            Rule::unquoted_name => {
                return inner.as_str().trim().to_string();
            }
            _ => {}
        }
    }
    String::new()
}

fn extract_id(pair: pest::iterators::Pair<Rule>) -> String {
    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::quoted_string => {
                return unquote(inner.as_str());
            }
            Rule::unquoted_id => {
                return inner.as_str().trim().to_string();
            }
            _ => {}
        }
    }
    String::new()
}

fn extract_id_list(pair: pest::iterators::Pair<Rule>) -> Vec<String> {
    let mut ids = Vec::new();
    for inner in pair.into_inner() {
        if inner.as_rule() == Rule::identifier {
            ids.push(inner.as_str().to_string());
        } else if inner.as_rule() == Rule::id_list {
            ids.extend(extract_id_list(inner));
        }
    }
    ids
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
    use crate::diagrams::requirement::types::{RelationshipType, RiskLevel, VerifyType};

    mod basic_parsing {
        use super::*;

        #[test]
        fn should_parse_empty_requirement_diagram() {
            let result = parse("requirementDiagram");
            assert!(result.is_ok(), "Parse error: {:?}", result.err());
        }

        #[test]
        fn should_reject_invalid_diagram() {
            let result = parse("requirementDiagram-1");
            assert!(result.is_err());
        }
    }

    mod requirement_parsing {
        use super::*;

        #[test]
        fn should_parse_full_requirement() {
            let input = r#"requirementDiagram

requirement test_req {
id: test_id
text: the test text.
risk: high
verifymethod: analysis
}"#;
            let result = parse(input);
            assert!(result.is_ok(), "Parse error: {:?}", result.err());
            let db = result.unwrap();

            let reqs = db.get_requirements();
            assert_eq!(reqs.len(), 1);

            let req = reqs.get("test_req").unwrap();
            assert_eq!(req.requirement_id, "test_id");
            assert_eq!(req.text, "the test text.");
            assert_eq!(req.risk, RiskLevel::High);
            assert_eq!(req.verify_method, VerifyType::Analysis);
        }

        #[test]
        fn should_parse_functional_requirement() {
            let input = r#"requirementDiagram

functionalRequirement test_req {
id: test_id
}"#;
            let result = parse(input);
            assert!(result.is_ok(), "Parse error: {:?}", result.err());
            let db = result.unwrap();

            let req = db.get_requirements().get("test_req").unwrap();
            assert_eq!(
                req.req_type,
                crate::diagrams::requirement::RequirementType::FunctionalRequirement
            );
        }

        #[test]
        fn should_parse_interface_requirement() {
            let input = r#"requirementDiagram

interfaceRequirement test_req {
}"#;
            let result = parse(input);
            assert!(result.is_ok(), "Parse error: {:?}", result.err());
        }

        #[test]
        fn should_parse_performance_requirement() {
            let input = r#"requirementDiagram

performanceRequirement test_req {
}"#;
            let result = parse(input);
            assert!(result.is_ok(), "Parse error: {:?}", result.err());
        }

        #[test]
        fn should_parse_physical_requirement() {
            let input = r#"requirementDiagram

physicalRequirement test_req {
}"#;
            let result = parse(input);
            assert!(result.is_ok(), "Parse error: {:?}", result.err());
        }

        #[test]
        fn should_parse_design_constraint() {
            let input = r#"requirementDiagram

designConstraint test_req {
}"#;
            let result = parse(input);
            assert!(result.is_ok(), "Parse error: {:?}", result.err());
        }

        #[test]
        fn should_parse_low_risk() {
            let input = r#"requirementDiagram

requirement test_req {
risk: low
}"#;
            let result = parse(input);
            assert!(result.is_ok(), "Parse error: {:?}", result.err());
            let db = result.unwrap();
            let req = db.get_requirements().get("test_req").unwrap();
            assert_eq!(req.risk, RiskLevel::Low);
        }

        #[test]
        fn should_parse_medium_risk() {
            let input = r#"requirementDiagram

requirement test_req {
risk: medium
}"#;
            let result = parse(input);
            assert!(result.is_ok(), "Parse error: {:?}", result.err());
            let db = result.unwrap();
            let req = db.get_requirements().get("test_req").unwrap();
            assert_eq!(req.risk, RiskLevel::Medium);
        }

        #[test]
        fn should_parse_inspection_verify() {
            let input = r#"requirementDiagram

requirement test_req {
verifymethod: inspection
}"#;
            let result = parse(input);
            assert!(result.is_ok(), "Parse error: {:?}", result.err());
            let db = result.unwrap();
            let req = db.get_requirements().get("test_req").unwrap();
            assert_eq!(req.verify_method, VerifyType::Inspection);
        }

        #[test]
        fn should_parse_test_verify() {
            let input = r#"requirementDiagram

requirement test_req {
verifymethod: test
}"#;
            let result = parse(input);
            assert!(result.is_ok(), "Parse error: {:?}", result.err());
            let db = result.unwrap();
            let req = db.get_requirements().get("test_req").unwrap();
            assert_eq!(req.verify_method, VerifyType::Test);
        }

        #[test]
        fn should_parse_demonstration_verify() {
            let input = r#"requirementDiagram

designConstraint test_req {
verifymethod: demonstration
}"#;
            let result = parse(input);
            assert!(result.is_ok(), "Parse error: {:?}", result.err());
            let db = result.unwrap();
            let req = db.get_requirements().get("test_req").unwrap();
            assert_eq!(req.verify_method, VerifyType::Demonstration);
        }
    }

    mod element_parsing {
        use super::*;

        #[test]
        fn should_parse_full_element() {
            let input = r#"requirementDiagram

element test_el {
type: test_type
docref: test_ref
}"#;
            let result = parse(input);
            assert!(result.is_ok(), "Parse error: {:?}", result.err());
            let db = result.unwrap();

            let elems = db.get_elements();
            assert_eq!(elems.len(), 1);

            let elem = elems.get("test_el").unwrap();
            assert_eq!(elem.element_type, "test_type");
            assert_eq!(elem.doc_ref, "test_ref");
        }
    }

    mod relationship_parsing {
        use super::*;

        #[test]
        fn should_parse_contains_relationship() {
            let input = r#"requirementDiagram

a - contains -> b"#;
            let result = parse(input);
            assert!(result.is_ok(), "Parse error: {:?}", result.err());
            let db = result.unwrap();

            let rels = db.get_relationships();
            assert_eq!(rels.len(), 1);
            assert_eq!(rels[0].src, "a");
            assert_eq!(rels[0].dst, "b");
            assert_eq!(rels[0].rel_type, RelationshipType::Contains);
        }

        #[test]
        fn should_parse_copies_relationship() {
            let input = "requirementDiagram\n\na - copies -> b";
            let result = parse(input);
            assert!(result.is_ok(), "Parse error: {:?}", result.err());
            let db = result.unwrap();
            let rel = &db.get_relationships()[0];
            assert_eq!(rel.rel_type, RelationshipType::Copies);
        }

        #[test]
        fn should_parse_derives_relationship() {
            let input = "requirementDiagram\n\na - derives -> b";
            let result = parse(input);
            assert!(result.is_ok(), "Parse error: {:?}", result.err());
            let db = result.unwrap();
            let rel = &db.get_relationships()[0];
            assert_eq!(rel.rel_type, RelationshipType::Derives);
        }

        #[test]
        fn should_parse_satisfies_relationship() {
            let input = "requirementDiagram\n\na - satisfies -> b";
            let result = parse(input);
            assert!(result.is_ok(), "Parse error: {:?}", result.err());
            let db = result.unwrap();
            let rel = &db.get_relationships()[0];
            assert_eq!(rel.rel_type, RelationshipType::Satisfies);
        }

        #[test]
        fn should_parse_verifies_relationship() {
            let input = "requirementDiagram\n\na - verifies -> b";
            let result = parse(input);
            assert!(result.is_ok(), "Parse error: {:?}", result.err());
            let db = result.unwrap();
            let rel = &db.get_relationships()[0];
            assert_eq!(rel.rel_type, RelationshipType::Verifies);
        }

        #[test]
        fn should_parse_refines_relationship() {
            let input = "requirementDiagram\n\na - refines -> b";
            let result = parse(input);
            assert!(result.is_ok(), "Parse error: {:?}", result.err());
            let db = result.unwrap();
            let rel = &db.get_relationships()[0];
            assert_eq!(rel.rel_type, RelationshipType::Refines);
        }

        #[test]
        fn should_parse_traces_relationship() {
            let input = "requirementDiagram\n\na - traces -> b";
            let result = parse(input);
            assert!(result.is_ok(), "Parse error: {:?}", result.err());
            let db = result.unwrap();
            let rel = &db.get_relationships()[0];
            assert_eq!(rel.rel_type, RelationshipType::Traces);
        }
    }

    mod styling_parsing {
        use super::*;

        #[test]
        fn should_parse_style_requirement() {
            let input = r#"requirementDiagram

requirement test_req {
}
style test_req fill:#f9f,stroke:#333,stroke-width:4px
"#;
            let result = parse(input);
            assert!(result.is_ok(), "Parse error: {:?}", result.err());
            let db = result.unwrap();

            let req = db.get_requirements().get("test_req").unwrap();
            assert_eq!(
                req.css_styles,
                vec!["fill:#f9f", "stroke:#333", "stroke-width:4px"]
            );
        }

        #[test]
        fn should_parse_style_element() {
            let input = r#"requirementDiagram

element test_element {
}
style test_element fill:#f9f,stroke:#333,stroke-width:4px
"#;
            let result = parse(input);
            assert!(result.is_ok(), "Parse error: {:?}", result.err());
            let db = result.unwrap();

            let elem = db.get_elements().get("test_element").unwrap();
            assert_eq!(
                elem.css_styles,
                vec!["fill:#f9f", "stroke:#333", "stroke-width:4px"]
            );
        }

        #[test]
        fn should_parse_style_multiple() {
            let input = r#"requirementDiagram

requirement test_requirement {
}
element test_element {
}
style test_requirement,test_element fill:#f9f,stroke:#333,stroke-width:4px
"#;
            let result = parse(input);
            assert!(result.is_ok(), "Parse error: {:?}", result.err());
            let db = result.unwrap();

            let req = db.get_requirements().get("test_requirement").unwrap();
            assert_eq!(
                req.css_styles,
                vec!["fill:#f9f", "stroke:#333", "stroke-width:4px"]
            );

            let elem = db.get_elements().get("test_element").unwrap();
            assert_eq!(
                elem.css_styles,
                vec!["fill:#f9f", "stroke:#333", "stroke-width:4px"]
            );
        }
    }

    mod class_parsing {
        use super::*;

        #[test]
        fn should_parse_class_def() {
            let input = r#"requirementDiagram

classDef myClass fill:#f9f,stroke:#333,stroke-width:4px
"#;
            let result = parse(input);
            assert!(result.is_ok(), "Parse error: {:?}", result.err());
            let db = result.unwrap();

            let class = db.get_classes().get("myClass").unwrap();
            assert_eq!(
                class.styles,
                vec!["fill:#f9f", "stroke:#333", "stroke-width:4px"]
            );
        }

        #[test]
        fn should_parse_class_def_multiple() {
            let input = r#"requirementDiagram

classDef firstClass,secondClass fill:#f9f,stroke:#333,stroke-width:4px
"#;
            let result = parse(input);
            assert!(result.is_ok(), "Parse error: {:?}", result.err());
            let db = result.unwrap();

            let first = db.get_classes().get("firstClass").unwrap();
            assert_eq!(
                first.styles,
                vec!["fill:#f9f", "stroke:#333", "stroke-width:4px"]
            );

            let second = db.get_classes().get("secondClass").unwrap();
            assert_eq!(
                second.styles,
                vec!["fill:#f9f", "stroke:#333", "stroke-width:4px"]
            );
        }

        #[test]
        fn should_parse_class_assignment() {
            let input = r#"requirementDiagram

requirement myReq {
}
classDef myClass fill:#f9f,stroke:#333,stroke-width:4px
class myReq myClass
"#;
            let result = parse(input);
            assert!(result.is_ok(), "Parse error: {:?}", result.err());
            let db = result.unwrap();

            let req = db.get_requirements().get("myReq").unwrap();
            assert!(req.classes.contains(&"myClass".to_string()));
        }

        #[test]
        fn should_parse_class_shorthand() {
            let input = r#"requirementDiagram

requirement myReq {
}
classDef myClass fill:#f9f,stroke:#333,stroke-width:4px
myReq:::myClass
"#;
            let result = parse(input);
            assert!(result.is_ok(), "Parse error: {:?}", result.err());
            let db = result.unwrap();

            let req = db.get_requirements().get("myReq").unwrap();
            assert!(req.classes.contains(&"myClass".to_string()));
        }

        #[test]
        fn should_parse_class_shorthand_multiple() {
            let input = r#"requirementDiagram

requirement myReq {
}
classDef class1 fill:#f9f
classDef class2 color:blue
myReq:::class1,class2
"#;
            let result = parse(input);
            assert!(result.is_ok(), "Parse error: {:?}", result.err());
            let db = result.unwrap();

            let req = db.get_requirements().get("myReq").unwrap();
            assert!(req.classes.contains(&"class1".to_string()));
            assert!(req.classes.contains(&"class2".to_string()));
        }

        #[test]
        fn should_parse_class_in_definition() {
            let input = r#"requirementDiagram

requirement myReq:::class1 {
}
classDef class1 fill:#f9f,stroke:#333,stroke-width:4px
"#;
            let result = parse(input);
            assert!(result.is_ok(), "Parse error: {:?}", result.err());
            let db = result.unwrap();

            let req = db.get_requirements().get("myReq").unwrap();
            assert!(req.classes.contains(&"class1".to_string()));
        }

        #[test]
        fn should_parse_element_class_in_definition() {
            let input = r#"requirementDiagram

element myElem:::class1,class2 {
}
classDef class1 fill:#f9f,stroke:#333,stroke-width:4px
classDef class2 color:blue
"#;
            let result = parse(input);
            assert!(result.is_ok(), "Parse error: {:?}", result.err());
            let db = result.unwrap();

            let elem = db.get_elements().get("myElem").unwrap();
            assert!(elem.classes.contains(&"class1".to_string()));
            assert!(elem.classes.contains(&"class2".to_string()));
        }
    }

    mod direction_parsing {
        use super::*;

        #[test]
        fn should_parse_direction_tb() {
            let input = "requirementDiagram\n\ndirection TB";
            let result = parse(input);
            assert!(result.is_ok(), "Parse error: {:?}", result.err());
            let db = result.unwrap();
            assert_eq!(db.get_direction(), "TB");
        }

        #[test]
        fn should_parse_direction_bt() {
            let input = "requirementDiagram\n\ndirection BT";
            let result = parse(input);
            assert!(result.is_ok(), "Parse error: {:?}", result.err());
            let db = result.unwrap();
            assert_eq!(db.get_direction(), "BT");
        }

        #[test]
        fn should_parse_direction_lr() {
            let input = "requirementDiagram\n\ndirection LR";
            let result = parse(input);
            assert!(result.is_ok(), "Parse error: {:?}", result.err());
            let db = result.unwrap();
            assert_eq!(db.get_direction(), "LR");
        }

        #[test]
        fn should_parse_direction_rl() {
            let input = "requirementDiagram\n\ndirection RL";
            let result = parse(input);
            assert!(result.is_ok(), "Parse error: {:?}", result.err());
            let db = result.unwrap();
            assert_eq!(db.get_direction(), "RL");
        }
    }

    mod special_cases {
        use super::*;

        #[test]
        fn should_parse_proto_as_requirement_id() {
            let input = r#"requirementDiagram
requirement __proto__ {
  id: 1
  text: the test text.
  risk: high
  verifymethod: test
}"#;
            let result = parse(input);
            assert!(result.is_ok(), "Parse error: {:?}", result.err());
            let db = result.unwrap();
            assert_eq!(db.get_requirements().len(), 1);
        }

        #[test]
        fn should_parse_constructor_as_element_id() {
            let input = r#"requirementDiagram
element constructor {
  type: simulation
}"#;
            let result = parse(input);
            assert!(result.is_ok(), "Parse error: {:?}", result.err());
            let db = result.unwrap();
            assert_eq!(db.get_elements().len(), 1);
        }
    }

    // Tests ported from mermaid Cypress tests (requirement.spec.js)
    mod cypress_tests {
        use super::*;

        #[test]
        fn test_cypress_sample() {
            // From Cypress: sample
            let input = r#"requirementDiagram

    requirement test_req {
    id: 1
    text: the test text.
    risk: high
    verifymethod: test
    }

    functionalRequirement test_req2 {
    id: 1.1
    text: the second test text.
    risk: low
    verifymethod: inspection
    }

    performanceRequirement test_req3 {
    id: 1.2
    text: the third test text.
    risk: medium
    verifymethod: demonstration
    }

    element test_entity {
    type: simulation
    }

    element test_entity2 {
    type: word doc
    docRef: reqs/test_entity
    }


    test_entity - satisfies -> test_req2
    test_req - traces -> test_req2
    test_req - contains -> test_req3
    test_req <- copies - test_entity2"#;
            let result = parse(input);
            assert!(result.is_ok(), "Failed to parse: {:?}", result);
            let db = result.unwrap();
            assert_eq!(db.get_requirements().len(), 3);
            assert_eq!(db.get_elements().len(), 2);
            assert!(!db.get_relationships().is_empty());
        }
    }
}
