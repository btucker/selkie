//! Architecture diagram parser
//!
//! Parses architecture diagrams using pest grammar.

use pest::Parser;
use pest_derive::Parser;

use super::{
    ArchitectureDb, ArchitectureDirection, ArchitectureEdge, ArchitectureGroup,
    ArchitectureJunction, ArchitectureService,
};

#[derive(Parser)]
#[grammar = "diagrams/architecture/architecture.pest"]
struct ArchitectureParser;

/// Parse an architecture diagram string into a database
pub fn parse(input: &str) -> Result<ArchitectureDb, Box<dyn std::error::Error>> {
    let pairs = ArchitectureParser::parse(Rule::diagram, input)?;
    let mut db = ArchitectureDb::new();

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
    db: &mut ArchitectureDb,
) -> Result<(), Box<dyn std::error::Error>> {
    for stmt in pair.into_inner() {
        if stmt.as_rule() == Rule::statement {
            process_statement(stmt, db)?;
        }
    }
    Ok(())
}

fn process_statement(
    pair: pest::iterators::Pair<Rule>,
    db: &mut ArchitectureDb,
) -> Result<(), Box<dyn std::error::Error>> {
    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::title_stmt => process_title(inner, db),
            Rule::acc_title_stmt => process_acc_title(inner, db),
            Rule::acc_descr_stmt => process_acc_descr(inner, db),
            Rule::acc_descr_multiline_stmt => process_acc_descr_multiline(inner, db),
            Rule::group_stmt => process_group(inner, db)?,
            Rule::service_stmt => process_service(inner, db)?,
            Rule::junction_stmt => process_junction(inner, db)?,
            Rule::edge_stmt => process_edge(inner, db)?,
            Rule::comment_stmt => {} // Skip comments
            _ => {}
        }
    }
    Ok(())
}

fn process_title(pair: pest::iterators::Pair<Rule>, db: &mut ArchitectureDb) {
    for inner in pair.into_inner() {
        if inner.as_rule() == Rule::title_text {
            db.set_title(inner.as_str().trim());
        }
    }
}

fn process_acc_title(pair: pest::iterators::Pair<Rule>, db: &mut ArchitectureDb) {
    for inner in pair.into_inner() {
        if inner.as_rule() == Rule::acc_title_text {
            db.set_acc_title(inner.as_str().trim());
        }
    }
}

fn process_acc_descr(pair: pest::iterators::Pair<Rule>, db: &mut ArchitectureDb) {
    for inner in pair.into_inner() {
        if inner.as_rule() == Rule::acc_descr_text {
            db.set_acc_description(inner.as_str().trim());
        }
    }
}

fn process_acc_descr_multiline(pair: pest::iterators::Pair<Rule>, db: &mut ArchitectureDb) {
    for inner in pair.into_inner() {
        if inner.as_rule() == Rule::acc_descr_multiline_text {
            db.set_acc_description(inner.as_str().trim());
        }
    }
}

fn process_group(
    pair: pest::iterators::Pair<Rule>,
    db: &mut ArchitectureDb,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut id = String::new();
    let mut icon: Option<String> = None;
    let mut title: Option<String> = None;
    let mut parent: Option<String> = None;

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::arch_id => {
                id = inner.as_str().to_string();
            }
            Rule::arch_icon => {
                // Remove surrounding parentheses
                let s = inner.as_str();
                icon = Some(s[1..s.len() - 1].to_string());
            }
            Rule::arch_title => {
                // Remove surrounding brackets
                let s = inner.as_str();
                title = Some(s[1..s.len() - 1].to_string());
            }
            Rule::in_clause => {
                for clause_inner in inner.into_inner() {
                    if clause_inner.as_rule() == Rule::arch_id {
                        parent = Some(clause_inner.as_str().to_string());
                    }
                }
            }
            _ => {}
        }
    }

    let mut group = ArchitectureGroup::new(id);
    if let Some(i) = icon {
        group = group.with_icon(&i);
    }
    if let Some(t) = title {
        group = group.with_title(&t);
    }
    if let Some(p) = parent {
        group = group.with_parent(&p);
    }

    db.add_group(group)?;
    Ok(())
}

fn process_service(
    pair: pest::iterators::Pair<Rule>,
    db: &mut ArchitectureDb,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut id = String::new();
    let mut icon: Option<String> = None;
    let mut icon_text: Option<String> = None;
    let mut title: Option<String> = None;
    let mut parent: Option<String> = None;

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::arch_id => {
                id = inner.as_str().to_string();
            }
            Rule::arch_icon => {
                // Remove surrounding parentheses
                let s = inner.as_str();
                icon = Some(s[1..s.len() - 1].to_string());
            }
            Rule::icon_text => {
                // Remove surrounding quotes
                let s = inner.as_str();
                icon_text = Some(s[1..s.len() - 1].to_string());
            }
            Rule::arch_title => {
                // Remove surrounding brackets
                let s = inner.as_str();
                title = Some(s[1..s.len() - 1].to_string());
            }
            Rule::in_clause => {
                for clause_inner in inner.into_inner() {
                    if clause_inner.as_rule() == Rule::arch_id {
                        parent = Some(clause_inner.as_str().to_string());
                    }
                }
            }
            _ => {}
        }
    }

    let mut service = ArchitectureService::new(id);
    if let Some(i) = icon {
        service.icon = Some(i);
    }
    if let Some(t) = icon_text {
        service.icon_text = Some(t);
    }
    if let Some(t) = title {
        service = service.with_title(&t);
    }
    if let Some(p) = parent {
        service = service.with_parent(&p);
    }

    db.add_service(service)?;
    Ok(())
}

fn process_junction(
    pair: pest::iterators::Pair<Rule>,
    db: &mut ArchitectureDb,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut id = String::new();
    let mut parent: Option<String> = None;

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::arch_id => {
                id = inner.as_str().to_string();
            }
            Rule::in_clause => {
                for clause_inner in inner.into_inner() {
                    if clause_inner.as_rule() == Rule::arch_id {
                        parent = Some(clause_inner.as_str().to_string());
                    }
                }
            }
            _ => {}
        }
    }

    let mut junction = ArchitectureJunction::new(id);
    if let Some(p) = parent {
        junction = junction.with_parent(&p);
    }

    db.add_junction(junction)?;
    Ok(())
}

fn process_edge(
    pair: pest::iterators::Pair<Rule>,
    db: &mut ArchitectureDb,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut lhs_id = String::new();
    let mut rhs_id = String::new();
    let mut lhs_dir = ArchitectureDirection::Right;
    let mut rhs_dir = ArchitectureDirection::Left;
    let mut lhs_into = false;
    let mut rhs_into = false;
    let mut lhs_group = false;
    let mut rhs_group = false;
    let mut title: Option<String> = None;

    let mut id_count = 0;
    let mut group_count = 0;

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::arch_id => {
                if id_count == 0 {
                    lhs_id = inner.as_str().to_string();
                } else {
                    rhs_id = inner.as_str().to_string();
                }
                id_count += 1;
            }
            Rule::arrow_group => {
                if group_count == 0 {
                    lhs_group = true;
                } else {
                    rhs_group = true;
                }
                group_count += 1;
            }
            Rule::arrow => {
                let (ld, rd, li, ri, t) = process_arrow(inner);
                lhs_dir = ld;
                rhs_dir = rd;
                lhs_into = li;
                rhs_into = ri;
                title = t;
            }
            _ => {}
        }
    }

    let mut edge = ArchitectureEdge::new(lhs_id, lhs_dir, rhs_id, rhs_dir);
    edge.lhs_into = lhs_into;
    edge.rhs_into = rhs_into;
    edge.lhs_group = lhs_group;
    edge.rhs_group = rhs_group;
    if let Some(t) = title {
        edge = edge.with_title(&t);
    }

    db.add_edge(edge)?;
    Ok(())
}

fn process_arrow(
    pair: pest::iterators::Pair<Rule>,
) -> (
    ArchitectureDirection,
    ArchitectureDirection,
    bool,
    bool,
    Option<String>,
) {
    let mut lhs_dir = ArchitectureDirection::Right;
    let mut rhs_dir = ArchitectureDirection::Left;
    let mut _into_count = 0;
    let mut lhs_into = false;
    let mut rhs_into = false;
    let mut title: Option<String> = None;
    let mut saw_arrow_line = false;

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::left_port => {
                for port_inner in inner.into_inner() {
                    if port_inner.as_rule() == Rule::arrow_direction {
                        lhs_dir = parse_direction(port_inner.as_str());
                    }
                }
            }
            Rule::right_port => {
                for port_inner in inner.into_inner() {
                    if port_inner.as_rule() == Rule::arrow_direction {
                        rhs_dir = parse_direction(port_inner.as_str());
                    }
                }
            }
            Rule::arrow_into => {
                if !saw_arrow_line {
                    lhs_into = true;
                } else {
                    rhs_into = true;
                }
                _into_count += 1;
            }
            Rule::arrow_line => {
                saw_arrow_line = true;
                for line_inner in inner.into_inner() {
                    if line_inner.as_rule() == Rule::arrow_line_with_label {
                        for label_inner in line_inner.into_inner() {
                            if label_inner.as_rule() == Rule::arrow_label {
                                title = Some(label_inner.as_str().trim().to_string());
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }

    (lhs_dir, rhs_dir, lhs_into, rhs_into, title)
}

fn parse_direction(s: &str) -> ArchitectureDirection {
    match s.to_uppercase().as_str() {
        "L" => ArchitectureDirection::Left,
        "R" => ArchitectureDirection::Right,
        "T" => ArchitectureDirection::Top,
        "B" => ArchitectureDirection::Bottom,
        _ => ArchitectureDirection::Right,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_architecture_keyword() {
        let input = "architecture-beta";
        let result = parse(input);
        assert!(result.is_ok());
    }

    #[test]
    fn test_simple_service() {
        let input = "architecture-beta
            service db
            ";
        let result = parse(input).unwrap();
        let services = result.get_services();
        assert_eq!(services.len(), 1);
        assert_eq!(services[0].id, "db");
    }

    #[test]
    fn test_title_on_first_line() {
        let input = "architecture-beta title Simple Architecture Diagram";
        let result = parse(input).unwrap();
        assert_eq!(result.get_title(), "Simple Architecture Diagram");
    }

    #[test]
    fn test_title_on_separate_line() {
        let input = "architecture-beta
            title Simple Architecture Diagram
            ";
        let result = parse(input).unwrap();
        assert_eq!(result.get_title(), "Simple Architecture Diagram");
    }

    #[test]
    fn test_accessibility_title() {
        let input = "architecture-beta
            accTitle: Accessibility Title
            ";
        let result = parse(input).unwrap();
        assert_eq!(result.get_acc_title(), "Accessibility Title");
    }

    #[test]
    fn test_accessibility_description() {
        let input = "architecture-beta
            accDescr: Accessibility Description
            ";
        let result = parse(input).unwrap();
        assert_eq!(result.get_acc_description(), "Accessibility Description");
    }

    #[test]
    fn test_accessibility_description_multiline() {
        let input = "architecture-beta
            accDescr {
                Accessibility Description
            }
            ";
        let result = parse(input).unwrap();
        assert_eq!(result.get_acc_description(), "Accessibility Description");
    }

    #[test]
    fn test_service_with_icon() {
        let input = "architecture-beta
            service db(database)
            ";
        let result = parse(input).unwrap();
        let services = result.get_services();
        assert_eq!(services.len(), 1);
        assert_eq!(services[0].id, "db");
        assert_eq!(services[0].icon, Some("database".to_string()));
    }

    #[test]
    fn test_service_with_icon_and_title() {
        let input = "architecture-beta
            service db(database)[Database Service]
            ";
        let result = parse(input).unwrap();
        let services = result.get_services();
        assert_eq!(services.len(), 1);
        assert_eq!(services[0].id, "db");
        assert_eq!(services[0].icon, Some("database".to_string()));
        assert_eq!(services[0].title, Some("Database Service".to_string()));
    }

    #[test]
    fn test_service_with_icon_text() {
        let input = r#"architecture-beta
            service db "DB"
            "#;
        let result = parse(input).unwrap();
        let services = result.get_services();
        assert_eq!(services.len(), 1);
        assert_eq!(services[0].id, "db");
        assert_eq!(services[0].icon_text, Some("DB".to_string()));
    }

    #[test]
    fn test_group() {
        let input = "architecture-beta
            group cloud
            ";
        let result = parse(input).unwrap();
        let groups = result.get_groups();
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].id, "cloud");
    }

    #[test]
    fn test_group_with_icon_and_title() {
        let input = "architecture-beta
            group cloud(aws)[AWS Cloud]
            ";
        let result = parse(input).unwrap();
        let groups = result.get_groups();
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].id, "cloud");
        assert_eq!(groups[0].icon, Some("aws".to_string()));
        assert_eq!(groups[0].title, Some("AWS Cloud".to_string()));
    }

    #[test]
    fn test_service_in_group() {
        let input = "architecture-beta
            group cloud
            service db in cloud
            ";
        let result = parse(input).unwrap();
        let services = result.get_services();
        assert_eq!(services.len(), 1);
        assert_eq!(services[0].id, "db");
        assert_eq!(services[0].parent, Some("cloud".to_string()));
    }

    #[test]
    fn test_junction() {
        let input = "architecture-beta
            junction junc1
            ";
        let result = parse(input).unwrap();
        let junctions = result.get_junctions();
        assert_eq!(junctions.len(), 1);
        assert_eq!(junctions[0].id, "junc1");
    }

    #[test]
    fn test_junction_in_group() {
        let input = "architecture-beta
            group cloud
            junction junc1 in cloud
            ";
        let result = parse(input).unwrap();
        let junctions = result.get_junctions();
        assert_eq!(junctions.len(), 1);
        assert_eq!(junctions[0].id, "junc1");
        assert_eq!(junctions[0].parent, Some("cloud".to_string()));
    }

    #[test]
    fn test_simple_edge() {
        let input = "architecture-beta
            service db
            service api
            db:R -- L:api
            ";
        let result = parse(input).unwrap();
        let edges = result.get_edges();
        assert_eq!(edges.len(), 1);
        assert_eq!(edges[0].lhs_id, "db");
        assert_eq!(edges[0].rhs_id, "api");
        assert_eq!(edges[0].lhs_dir, ArchitectureDirection::Right);
        assert_eq!(edges[0].rhs_dir, ArchitectureDirection::Left);
    }

    #[test]
    fn test_edge_with_label() {
        let input = "architecture-beta
            service db
            service api
            db:R - connects - L:api
            ";
        let result = parse(input).unwrap();
        let edges = result.get_edges();
        assert_eq!(edges.len(), 1);
        assert_eq!(edges[0].title, Some("connects".to_string()));
    }

    #[test]
    fn test_edge_with_into_arrows() {
        let input = "architecture-beta
            service db
            service api
            db:R <-- L:api
            ";
        let result = parse(input).unwrap();
        let edges = result.get_edges();
        assert_eq!(edges.len(), 1);
        assert!(edges[0].lhs_into);
        assert!(!edges[0].rhs_into);
    }

    #[test]
    fn test_edge_with_both_into_arrows() {
        let input = "architecture-beta
            service db
            service api
            db:R <--> L:api
            ";
        let result = parse(input).unwrap();
        let edges = result.get_edges();
        assert_eq!(edges.len(), 1);
        assert!(edges[0].lhs_into);
        assert!(edges[0].rhs_into);
    }

    #[test]
    fn test_edge_with_group_boundary() {
        let input = "architecture-beta
            group cloud1
            group cloud2
            service db in cloud1
            service api in cloud2
            db{group}:R -- L:api{group}
            ";
        let result = parse(input).unwrap();
        let edges = result.get_edges();
        assert_eq!(edges.len(), 1);
        assert!(edges[0].lhs_group);
        assert!(edges[0].rhs_group);
    }

    #[test]
    fn test_multiple_services() {
        let input = "architecture-beta
            service db
            service api
            service web
            ";
        let result = parse(input).unwrap();
        let services = result.get_services();
        assert_eq!(services.len(), 3);
    }

    #[test]
    fn test_nested_groups() {
        let input = "architecture-beta
            group cloud
            group compute in cloud
            service api in compute
            ";
        let result = parse(input).unwrap();
        let groups = result.get_groups();
        assert_eq!(groups.len(), 2);

        let compute = groups.iter().find(|g| g.id == "compute").unwrap();
        assert_eq!(compute.parent, Some("cloud".to_string()));

        let services = result.get_services();
        assert_eq!(services[0].parent, Some("compute".to_string()));
    }

    #[test]
    fn test_all_directions() {
        let input = "architecture-beta
            service a
            service b
            service c
            service d
            service center
            a:R -- L:center
            b:L -- R:center
            c:B -- T:center
            d:T -- B:center
            ";
        let result = parse(input).unwrap();
        let edges = result.get_edges();
        assert_eq!(edges.len(), 4);

        assert_eq!(edges[0].lhs_dir, ArchitectureDirection::Right);
        assert_eq!(edges[0].rhs_dir, ArchitectureDirection::Left);

        assert_eq!(edges[1].lhs_dir, ArchitectureDirection::Left);
        assert_eq!(edges[1].rhs_dir, ArchitectureDirection::Right);

        assert_eq!(edges[2].lhs_dir, ArchitectureDirection::Bottom);
        assert_eq!(edges[2].rhs_dir, ArchitectureDirection::Top);

        assert_eq!(edges[3].lhs_dir, ArchitectureDirection::Top);
        assert_eq!(edges[3].rhs_dir, ArchitectureDirection::Bottom);
    }

    #[test]
    fn test_comment() {
        let input = "architecture-beta
            %% This is a comment
            service db
            ";
        let result = parse(input).unwrap();
        let services = result.get_services();
        assert_eq!(services.len(), 1);
    }

    #[test]
    fn test_icon_with_colon() {
        let input = "architecture-beta
            service db(aws:rds)
            ";
        let result = parse(input).unwrap();
        let services = result.get_services();
        assert_eq!(services[0].icon, Some("aws:rds".to_string()));
    }

    #[test]
    fn test_complex_diagram() {
        let input = "architecture-beta
            title My Architecture
            accTitle: Architecture Diagram
            accDescr: Shows the system architecture

            group cloud(aws)[AWS Cloud]
            service db(aws:rds)[Database] in cloud
            service api(aws:lambda)[API Gateway] in cloud
            service web(aws:cloudfront)[CDN]

            db:R -- L:api
            api:T -- B:web
            ";
        let result = parse(input).unwrap();

        assert_eq!(result.get_title(), "My Architecture");
        assert_eq!(result.get_acc_title(), "Architecture Diagram");
        assert_eq!(
            result.get_acc_description(),
            "Shows the system architecture"
        );

        assert_eq!(result.get_groups().len(), 1);
        assert_eq!(result.get_services().len(), 3);
        assert_eq!(result.get_edges().len(), 2);
    }
}
