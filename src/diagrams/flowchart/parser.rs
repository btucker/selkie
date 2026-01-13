//! Flowchart parser
//!
//! This module provides parsing for flowchart diagrams using pest.

use pest::Parser;
use pest_derive::Parser;

use super::types::{FlowText, FlowVertexType, FlowchartDb};
use crate::error::{MermaidError, Result};

#[derive(Parser)]
#[grammar = "diagrams/flowchart/flowchart.pest"]
struct FlowchartParser;

/// Parse a flowchart diagram
pub fn parse(input: &str) -> Result<FlowchartDb> {
    let mut db = FlowchartDb::new();
    parse_into(input, &mut db)?;
    Ok(db)
}

/// Parse into an existing database
pub fn parse_into(input: &str, db: &mut FlowchartDb) -> Result<()> {
    let pairs = FlowchartParser::parse(Rule::diagram, input)
        .map_err(|e| MermaidError::ParseError(e.to_string()))?;

    for pair in pairs {
        match pair.as_rule() {
            Rule::diagram => {
                for inner in pair.into_inner() {
                    process_rule(inner, db)?;
                }
            }
            _ => {}
        }
    }

    Ok(())
}

fn process_rule(pair: pest::iterators::Pair<Rule>, db: &mut FlowchartDb) -> Result<()> {
    match pair.as_rule() {
        Rule::graph_config => {
            for inner in pair.into_inner() {
                if inner.as_rule() == Rule::direction {
                    db.set_direction(inner.as_str());
                }
            }
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
                    db.set_direction(inner.as_str());
                }
            }
        }
        Rule::vertex_statement => {
            process_vertex_statement(pair, db)?;
        }
        Rule::acc_title_stmt => {
            for inner in pair.into_inner() {
                if inner.as_rule() == Rule::line_content {
                    db.set_acc_title(inner.as_str().trim());
                }
            }
        }
        Rule::acc_descr_stmt => {
            process_acc_descr(pair, db)?;
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
        Rule::link_style_stmt => {
            process_link_style_stmt(pair, db)?;
        }
        Rule::click_stmt => {
            process_click_stmt(pair, db)?;
        }
        Rule::subgraph_stmt => {
            process_subgraph(pair, db)?;
        }
        _ => {}
    }
    Ok(())
}

fn process_vertex_statement(pair: pest::iterators::Pair<Rule>, db: &mut FlowchartDb) -> Result<()> {
    let mut nodes: Vec<String> = Vec::new();
    let mut pending_link: Option<(String, Option<String>, Option<String>)> = None; // (arrow, text, id)

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::node => {
                let node_ids = process_node(inner, db)?;

                // If we have a pending link, connect previous nodes to current nodes
                if let Some((arrow, text, link_id)) = pending_link.take() {
                    for from in &nodes {
                        for to in &node_ids {
                            db.add_edge(from, to, &arrow, text.as_deref(), link_id.as_deref());
                        }
                    }
                }

                nodes = node_ids;
            }
            Rule::link => {
                pending_link = Some(process_link(inner)?);
            }
            _ => {}
        }
    }

    Ok(())
}

fn process_node(pair: pest::iterators::Pair<Rule>, db: &mut FlowchartDb) -> Result<Vec<String>> {
    let mut node_ids = Vec::new();

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::styled_vertex => {
                let (id, class) = process_styled_vertex(inner, db)?;
                if let Some(class_name) = class {
                    db.set_class(&id, &class_name);
                }
                node_ids.push(id);
            }
            Rule::node_group => {
                for styled in inner.into_inner() {
                    if styled.as_rule() == Rule::styled_vertex {
                        let (id, class) = process_styled_vertex(styled, db)?;
                        if let Some(class_name) = class {
                            db.set_class(&id, &class_name);
                        }
                        node_ids.push(id);
                    }
                }
            }
            _ => {}
        }
    }

    Ok(node_ids)
}

fn process_styled_vertex(
    pair: pest::iterators::Pair<Rule>,
    db: &mut FlowchartDb,
) -> Result<(String, Option<String>)> {
    let mut vertex_id = String::new();
    let mut class_name = None;

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::vertex => {
                vertex_id = process_vertex(inner, db)?;
            }
            Rule::class_name => {
                class_name = Some(inner.as_str().to_string());
            }
            _ => {}
        }
    }

    Ok((vertex_id, class_name))
}

fn process_vertex(pair: pest::iterators::Pair<Rule>, db: &mut FlowchartDb) -> Result<String> {
    let mut id = String::new();
    let mut text: Option<FlowText> = None;
    let mut vertex_type: Option<FlowVertexType> = None;

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::identifier => {
                id = inner.as_str().to_string();
            }
            Rule::vertex_shape => {
                let (shape, shape_text) = process_vertex_shape(inner)?;
                vertex_type = Some(shape);
                text = Some(shape_text);
            }
            _ => {}
        }
    }

    db.add_vertex_simple(&id, text.as_ref().map(|t| t.text.as_str()), vertex_type);
    Ok(id)
}

fn process_vertex_shape(pair: pest::iterators::Pair<Rule>) -> Result<(FlowVertexType, FlowText)> {
    let inner = pair.into_inner().next().unwrap();

    let (shape_type, text) = match inner.as_rule() {
        Rule::shape_square => (FlowVertexType::Square, extract_text(inner)?),
        Rule::shape_round => (FlowVertexType::Round, extract_text(inner)?),
        Rule::shape_circle => (FlowVertexType::Circle, extract_text(inner)?),
        Rule::shape_double_circle => (FlowVertexType::DoubleCircle, extract_text(inner)?),
        Rule::shape_stadium => (FlowVertexType::Stadium, extract_text(inner)?),
        Rule::shape_subroutine => (FlowVertexType::Subroutine, extract_text(inner)?),
        Rule::shape_cylinder => (FlowVertexType::Cylinder, extract_text(inner)?),
        Rule::shape_diamond => (FlowVertexType::Diamond, extract_text(inner)?),
        Rule::shape_hexagon => (FlowVertexType::Hexagon, extract_text(inner)?),
        Rule::shape_ellipse => (FlowVertexType::Ellipse, extract_text(inner)?),
        Rule::shape_odd => (FlowVertexType::Odd, extract_text(inner)?),
        Rule::shape_trapezoid => (FlowVertexType::Trapezoid, extract_text(inner)?),
        Rule::shape_inv_trapezoid => (FlowVertexType::InvTrapezoid, extract_text(inner)?),
        Rule::shape_lean_right => (FlowVertexType::LeanRight, extract_text(inner)?),
        Rule::shape_lean_left => (FlowVertexType::LeanLeft, extract_text(inner)?),
        _ => return Err(MermaidError::ParseError("Unknown shape type".to_string())),
    };

    Ok((shape_type, text))
}

fn extract_text(pair: pest::iterators::Pair<Rule>) -> Result<FlowText> {
    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::text | Rule::text_to_slash_bracket | Rule::text_to_backslash_bracket => {
                return process_text(inner);
            }
            _ => {}
        }
    }
    Ok(FlowText::new(""))
}

fn process_text(pair: pest::iterators::Pair<Rule>) -> Result<FlowText> {
    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::quoted_string => {
                let s = inner.as_str();
                // Remove surrounding quotes
                let text = &s[1..s.len() - 1];
                return Ok(FlowText::new(text));
            }
            Rule::md_string => {
                let s = inner.as_str();
                // Remove surrounding "`...`"
                let text = &s[2..s.len() - 2];
                return Ok(FlowText::markdown(text));
            }
            Rule::plain_text
            | Rule::plain_text_to_slash_bracket
            | Rule::plain_text_to_backslash_bracket => {
                return Ok(FlowText::new(inner.as_str().trim()));
            }
            _ => {}
        }
    }
    Ok(FlowText::new(""))
}

fn process_link(
    pair: pest::iterators::Pair<Rule>,
) -> Result<(String, Option<String>, Option<String>)> {
    let mut arrow = String::new();
    let mut text = None;
    let mut link_id = None;

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::simple_link => {
                for link_inner in inner.into_inner() {
                    match link_inner.as_rule() {
                        Rule::link_arrow => {
                            arrow = link_inner.as_str().to_string();
                        }
                        Rule::link_id => {
                            let id_str = link_inner.as_str();
                            link_id = Some(id_str[..id_str.len() - 1].to_string());
                            // Remove @
                        }
                        _ => {}
                    }
                }
            }
            Rule::link_with_text => {
                for link_inner in inner.into_inner() {
                    match link_inner.as_rule() {
                        Rule::link_start => {
                            arrow = link_inner.as_str().to_string();
                        }
                        Rule::link_end => {
                            arrow.push_str(link_inner.as_str());
                        }
                        Rule::link_arrow => {
                            arrow = link_inner.as_str().to_string();
                        }
                        Rule::edge_text => {
                            text = Some(link_inner.as_str().trim().to_string());
                        }
                        Rule::text => {
                            let flow_text = process_text(link_inner)?;
                            text = Some(flow_text.text);
                        }
                        Rule::link_id => {
                            let id_str = link_inner.as_str();
                            link_id = Some(id_str[..id_str.len() - 1].to_string());
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }

    Ok((arrow, text, link_id))
}

fn process_acc_descr(pair: pest::iterators::Pair<Rule>, db: &mut FlowchartDb) -> Result<()> {
    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::acc_descr_single => {
                for i in inner.into_inner() {
                    if i.as_rule() == Rule::line_content {
                        db.set_acc_description(i.as_str().trim());
                    }
                }
            }
            Rule::acc_descr_multi => {
                for i in inner.into_inner() {
                    if i.as_rule() == Rule::multiline_content {
                        db.set_acc_description(i.as_str().trim());
                    }
                }
            }
            _ => {}
        }
    }
    Ok(())
}

fn process_class_def(pair: pest::iterators::Pair<Rule>, db: &mut FlowchartDb) -> Result<()> {
    let mut identifier = String::new();
    let mut styles_str = String::new();

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::class_id => {
                identifier = inner.as_str().to_string();
            }
            Rule::styles => {
                styles_str = inner.as_str().trim().to_string();
            }
            _ => {}
        }
    }

    // Split styles by comma
    let styles: Vec<String> = styles_str
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    db.add_class(&identifier, &styles);
    Ok(())
}

fn process_class_stmt(pair: pest::iterators::Pair<Rule>, db: &mut FlowchartDb) -> Result<()> {
    let mut identifiers = Vec::new();
    let mut class_name = String::new();

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::identifier_list => {
                for id in inner.into_inner() {
                    if id.as_rule() == Rule::identifier {
                        identifiers.push(id.as_str().to_string());
                    }
                }
            }
            Rule::identifier => {
                class_name = inner.as_str().to_string();
            }
            _ => {}
        }
    }

    for id in identifiers {
        db.set_class(&id, &class_name);
    }
    Ok(())
}

fn process_style_stmt(pair: pest::iterators::Pair<Rule>, db: &mut FlowchartDb) -> Result<()> {
    let mut identifier = String::new();
    let mut styles_str = String::new();

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::identifier => {
                identifier = inner.as_str().to_string();
            }
            Rule::styles => {
                styles_str = inner.as_str().trim().to_string();
            }
            _ => {}
        }
    }

    let styles: Vec<String> = styles_str
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    if let Some(vertex) = db.get_vertex_mut(&identifier) {
        vertex.styles = styles;
    }
    Ok(())
}

fn process_link_style_stmt(pair: pest::iterators::Pair<Rule>, db: &mut FlowchartDb) -> Result<()> {
    let mut indices = Vec::new();
    let mut styles_str = String::new();
    let mut interpolate = None;
    let mut is_default = false;

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::num_list => {
                for num in inner.into_inner() {
                    if num.as_rule() == Rule::NUMBER {
                        if let Ok(n) = num.as_str().parse::<usize>() {
                            indices.push(n);
                        }
                    }
                }
            }
            Rule::identifier => {
                // Could be "default" or interpolate value
                let id = inner.as_str();
                if id == "default" {
                    is_default = true;
                } else {
                    interpolate = Some(id.to_string());
                }
            }
            Rule::styles => {
                styles_str = inner.as_str().trim().to_string();
            }
            _ => {}
        }
    }

    let styles: Vec<String> = styles_str
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    if is_default {
        db.set_default_link_style(&styles);
    } else {
        for idx in indices {
            db.set_link_style(idx, &styles);
        }
    }

    if let Some(interp) = interpolate {
        db.set_default_link_interpolate(&interp);
    }

    Ok(())
}

fn process_click_stmt(pair: pest::iterators::Pair<Rule>, db: &mut FlowchartDb) -> Result<()> {
    let mut node_id = String::new();
    let mut callback = None;
    let mut href = None;
    let mut tooltip = None;
    let mut link_target = None;

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::identifier => {
                node_id = inner.as_str().to_string();
            }
            Rule::click_action => {
                for action in inner.into_inner() {
                    match action.as_rule() {
                        Rule::callback_action | Rule::simple_callback => {
                            for a in action.into_inner() {
                                match a.as_rule() {
                                    Rule::identifier => {
                                        callback = Some(a.as_str().to_string());
                                    }
                                    Rule::tooltip => {
                                        for t in a.into_inner() {
                                            if t.as_rule() == Rule::quoted_string {
                                                let s = t.as_str();
                                                tooltip = Some(s[1..s.len() - 1].to_string());
                                            }
                                        }
                                    }
                                    _ => {}
                                }
                            }
                        }
                        Rule::href_action => {
                            for a in action.into_inner() {
                                match a.as_rule() {
                                    Rule::quoted_string => {
                                        let s = a.as_str();
                                        href = Some(s[1..s.len() - 1].to_string());
                                    }
                                    Rule::link_target => {
                                        for t in a.into_inner() {
                                            link_target = Some(t.as_str().to_string());
                                        }
                                    }
                                    Rule::tooltip => {
                                        for t in a.into_inner() {
                                            if t.as_rule() == Rule::quoted_string {
                                                let s = t.as_str();
                                                tooltip = Some(s[1..s.len() - 1].to_string());
                                            }
                                        }
                                    }
                                    _ => {}
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }

    if let Some(cb) = callback {
        db.set_click_event(&node_id, &cb);
    }
    if let Some(h) = href {
        db.set_link(&node_id, &h, link_target.as_deref());
    }
    if let Some(t) = tooltip {
        db.set_tooltip(&node_id, &t);
    }

    Ok(())
}

fn process_subgraph(pair: pest::iterators::Pair<Rule>, db: &mut FlowchartDb) -> Result<()> {
    let mut id = String::new();
    let mut title = None;

    // Track existing vertices before processing subgraph content
    let existing_vertices: std::collections::HashSet<String> =
        db.vertices().keys().cloned().collect();

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::subgraph_id => {
                for i in inner.into_inner() {
                    match i.as_rule() {
                        Rule::identifier => {
                            id = i.as_str().to_string();
                        }
                        Rule::text => {
                            let flow_text = process_text(i)?;
                            title = Some(flow_text.text);
                        }
                        _ => {}
                    }
                }
            }
            Rule::document => {
                // Recursively process subgraph content
                for doc_inner in inner.into_inner() {
                    process_rule(doc_inner, db)?;
                }
            }
            _ => {}
        }
    }

    // Find vertices that were added during subgraph processing
    let new_vertices: Vec<String> = db
        .vertices()
        .keys()
        .filter(|k| !existing_vertices.contains(*k))
        .cloned()
        .collect();

    db.add_subgraph_with_nodes(&id, title.as_deref().unwrap_or(&id), new_vertices);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_flowchart() {
        let input = "flowchart LR\n    A --> B";
        let result = parse(input);
        assert!(result.is_ok(), "Failed to parse: {:?}", result);
        let db = result.unwrap();
        assert!(db.get_vertices().contains_key("A"));
        assert!(db.get_vertices().contains_key("B"));
        assert_eq!(db.get_edges().len(), 1);
    }

    #[test]
    fn test_parse_with_labels() {
        let input = "flowchart TD\n    A[Start] --> B[End]";
        let result = parse(input);
        assert!(result.is_ok(), "Failed to parse: {:?}", result);
        let db = result.unwrap();
        let a = db.get_vertices().get("A").unwrap();
        assert_eq!(a.text, Some("Start".to_string()));
    }

    #[test]
    fn test_parse_different_shapes() {
        let input = r#"flowchart LR
    A[Square]
    B(Round)
    C((Circle))
    D{Diamond}
    E[(Cylinder)]
    F([Stadium])"#;
        let result = parse(input);
        assert!(result.is_ok(), "Failed to parse: {:?}", result);
        let db = result.unwrap();
        assert_eq!(db.get_vertices().len(), 6);
    }

    #[test]
    fn test_parse_edge_with_text() {
        let input = "flowchart LR\n    A -->|label| B";
        let result = parse(input);
        assert!(result.is_ok(), "Failed to parse: {:?}", result);
        let db = result.unwrap();
        let edges = db.get_edges();
        assert_eq!(edges.len(), 1);
        assert_eq!(edges[0].text, "label");
    }

    #[test]
    fn test_parse_style_stmt() {
        // Style statements need the vertex to exist first
        let input = "flowchart LR\nA[Start]\nstyle A fill:#f9f";
        let result = parse(input);
        assert!(result.is_ok(), "Failed to parse: {:?}", result);
        let db = result.unwrap();
        let vertex = db.get_vertices().get("A").unwrap();
        assert!(
            vertex.styles.len() > 0,
            "Vertex A should have styles: {:?}",
            vertex
        );
    }

    #[test]
    fn test_parse_class_def() {
        let input = "flowchart LR\nclassDef myClass fill:#f9f";
        let result = parse(input);
        assert!(result.is_ok(), "Failed to parse: {:?}", result);
        let db = result.unwrap();
        assert!(
            db.get_classes().contains_key("myClass"),
            "myClass should be defined"
        );
    }

    #[test]
    fn test_parse_subgraph() {
        let input = r#"flowchart LR
subgraph sub1[Title]
    A --> B
end"#;
        let result = parse(input);
        assert!(result.is_ok(), "Failed to parse: {:?}", result);
        let db = result.unwrap();
        assert!(!db.subgraphs().is_empty(), "Should have subgraphs");
    }

    #[test]
    fn test_parse_direction_stmt() {
        let input = "flowchart LR\ndirection TB";
        let result = parse(input);
        assert!(result.is_ok(), "Failed to parse: {:?}", result);
        let db = result.unwrap();
        assert_eq!(
            db.get_direction(),
            "TB",
            "Direction should be changed to TB"
        );
    }

    #[test]
    fn test_parse_click_stmt() {
        let input = r#"flowchart LR
A[Node]
click A myCallback"#;
        let result = parse(input);
        assert!(result.is_ok(), "Failed to parse: {:?}", result);
    }

    #[test]
    fn test_parse_link_style() {
        let input = "flowchart LR\nA --> B\nlinkStyle 0 stroke:#ff0";
        let result = parse(input);
        assert!(result.is_ok(), "Failed to parse: {:?}", result);
    }

    #[test]
    fn test_parse_node_with_class() {
        let input = "flowchart LR\nA:::myClass --> B";
        let result = parse(input);
        assert!(result.is_ok(), "Failed to parse: {:?}", result);
        let db = result.unwrap();
        let vertex = db.get_vertices().get("A").expect("Vertex A should exist");
        assert!(vertex.classes.contains(&"myClass".to_string()));
    }

    #[test]
    fn test_parse_multiple_edges() {
        let input = "flowchart LR\nA --> B --> C";
        let result = parse(input);
        assert!(result.is_ok(), "Failed to parse: {:?}", result);
        let db = result.unwrap();
        assert_eq!(db.get_edges().len(), 2);
    }

    #[test]
    fn test_parse_graph_keyword() {
        let input = "graph TD\nA --> B";
        let result = parse(input);
        assert!(
            result.is_ok(),
            "Failed to parse with 'graph' keyword: {:?}",
            result
        );
    }

    #[test]
    fn test_parse_thick_arrow() {
        let input = "flowchart LR\nA ==> B";
        let result = parse(input);
        assert!(result.is_ok(), "Failed to parse thick arrow: {:?}", result);
        let db = result.unwrap();
        assert_eq!(db.get_edges().len(), 1);
    }

    #[test]
    fn test_parse_dotted_arrow() {
        let input = "flowchart LR\nA -.-> B";
        let result = parse(input);
        assert!(result.is_ok(), "Failed to parse dotted arrow: {:?}", result);
        let db = result.unwrap();
        assert_eq!(db.get_edges().len(), 1);
    }

    #[test]
    fn test_parse_hexagon_shape() {
        let input = "flowchart LR\nA{{Hexagon}}";
        let result = parse(input);
        assert!(result.is_ok(), "Failed to parse hexagon: {:?}", result);
        let db = result.unwrap();
        let vertex = db.get_vertices().get("A").unwrap();
        assert_eq!(vertex.vertex_type, Some(FlowVertexType::Hexagon));
    }

    #[test]
    fn test_parse_acc_title() {
        let input = "flowchart LR\naccTitle: My Chart Title\nA --> B";
        let result = parse(input);
        assert!(result.is_ok(), "Failed to parse: {:?}", result);
        let db = result.unwrap();
        assert_eq!(db.get_acc_title(), Some("My Chart Title"));
    }

    #[test]
    fn test_parse_quoted_text() {
        let input = r#"flowchart LR
A["Node with spaces"]"#;
        let result = parse(input);
        assert!(result.is_ok(), "Failed to parse quoted text: {:?}", result);
        let db = result.unwrap();
        let vertex = db.get_vertices().get("A").unwrap();
        assert_eq!(vertex.text, Some("Node with spaces".to_string()));
    }

    #[test]
    fn test_parse_semicolon_newline() {
        let input = "flowchart LR;A --> B;C --> D";
        let result = parse(input);
        assert!(
            result.is_ok(),
            "Failed to parse semicolon-separated: {:?}",
            result
        );
        let db = result.unwrap();
        assert_eq!(db.get_edges().len(), 2);
    }

    #[test]
    fn test_parse_bidirectional_arrow() {
        let input = "flowchart LR\nA <--> B";
        let result = parse(input);
        assert!(
            result.is_ok(),
            "Failed to parse bidirectional: {:?}",
            result
        );
    }

    #[test]
    fn test_parse_node_group() {
        let input = "flowchart LR\nA & B --> C";
        let result = parse(input);
        assert!(result.is_ok(), "Failed to parse node group: {:?}", result);
        let db = result.unwrap();
        // Should have edges from both A and B to C
        assert_eq!(db.get_edges().len(), 2);
    }

    #[test]
    fn test_parse_nested_subgraph() {
        let input = r#"flowchart LR
subgraph outer
    subgraph inner
        A --> B
    end
end"#;
        let result = parse(input);
        assert!(
            result.is_ok(),
            "Failed to parse nested subgraph: {:?}",
            result
        );
        let db = result.unwrap();
        assert_eq!(db.subgraphs().len(), 2);
    }

    // Tests from flow.spec.js
    mod flow_spec_tests {
        use super::*;

        #[test]
        #[ignore = "TODO: Add comment support (%%) to grammar"]
        fn should_handle_trailing_whitespaces_after_statements() {
            let input = "graph TD;\n\n\n %% Comment\n A-->B; \n B-->C;";
            let result = parse(input);
            assert!(result.is_ok(), "Failed to parse: {:?}", result);
            let db = result.unwrap();
            let vert = db.get_vertices();
            let edges = db.get_edges();

            assert!(vert.contains_key("A"));
            assert!(vert.contains_key("B"));
            assert_eq!(edges.len(), 2);
            assert_eq!(edges[0].start, "A");
            assert_eq!(edges[0].end, "B");
        }

        #[test]
        #[ignore = "TODO: Fix identifier parsing to allow 'end' prefix"]
        fn should_handle_node_names_with_end_substring() {
            let input = "graph TD\nendpoint --> sender";
            let result = parse(input);
            assert!(result.is_ok(), "Failed to parse: {:?}", result);
            let db = result.unwrap();
            let vert = db.get_vertices();
            let edges = db.get_edges();

            assert!(vert.contains_key("endpoint"));
            assert!(vert.contains_key("sender"));
            assert_eq!(edges[0].start, "endpoint");
            assert_eq!(edges[0].end, "sender");
        }

        #[test]
        fn should_handle_node_names_ending_with_keywords() {
            let input = "graph TD\nblend --> monograph";
            let result = parse(input);
            assert!(result.is_ok(), "Failed to parse: {:?}", result);
            let db = result.unwrap();
            let vert = db.get_vertices();

            assert!(vert.contains_key("blend"));
            assert!(vert.contains_key("monograph"));
        }

        #[test]
        fn should_allow_default_in_node_name() {
            let input = "graph TD\ndefault --> monograph";
            let result = parse(input);
            assert!(result.is_ok(), "Failed to parse: {:?}", result);
            let db = result.unwrap();
            let vert = db.get_vertices();

            assert!(vert.contains_key("default"));
            assert!(vert.contains_key("monograph"));
        }

        #[test]
        fn should_parse_special_char_period() {
            let input = "graph TD;A(.)-->B;";
            let result = parse(input);
            assert!(result.is_ok(), "Failed to parse period: {:?}", result);
            let db = result.unwrap();
            let vert = db.get_vertices();
            assert!(vert.contains_key("A"));
            assert_eq!(vert.get("A").unwrap().text, Some(".".to_string()));
        }

        #[test]
        fn should_parse_special_char_colon() {
            let input = "graph TD;A(:)-->B;";
            let result = parse(input);
            assert!(result.is_ok(), "Failed to parse colon: {:?}", result);
            let db = result.unwrap();
            let vert = db.get_vertices();
            assert!(vert.contains_key("A"));
            assert_eq!(vert.get("A").unwrap().text, Some(":".to_string()));
        }

        #[test]
        fn should_parse_special_char_comma() {
            let input = "graph TD;A(,)-->B;";
            let result = parse(input);
            assert!(result.is_ok(), "Failed to parse comma: {:?}", result);
            let db = result.unwrap();
            let vert = db.get_vertices();
            assert!(vert.contains_key("A"));
        }

        #[test]
        fn should_parse_text_with_dash() {
            let input = "graph TD;A(a-b)-->B;";
            let result = parse(input);
            assert!(result.is_ok(), "Failed to parse dash: {:?}", result);
            let db = result.unwrap();
            let vert = db.get_vertices();
            assert_eq!(vert.get("A").unwrap().text, Some("a-b".to_string()));
        }

        #[test]
        fn should_parse_special_char_plus() {
            let input = "graph TD;A(+)-->B;";
            let result = parse(input);
            assert!(result.is_ok(), "Failed to parse plus: {:?}", result);
        }

        #[test]
        fn should_parse_special_char_asterisk() {
            let input = "graph TD;A(*)-->B;";
            let result = parse(input);
            assert!(result.is_ok(), "Failed to parse asterisk: {:?}", result);
        }

        #[test]
        fn should_parse_special_char_ampersand() {
            let input = "graph TD;A(&)-->B;";
            let result = parse(input);
            assert!(result.is_ok(), "Failed to parse ampersand: {:?}", result);
        }

        #[test]
        fn should_use_direction_in_node_ids() {
            let input = "graph TD;\n  node1TB\n";
            let result = parse(input);
            assert!(result.is_ok(), "Failed to parse: {:?}", result);
            let db = result.unwrap();
            assert!(db.get_vertices().contains_key("node1TB"));
        }

        #[test]
        #[ignore = "TODO: Add quoted subgraph title support"]
        fn should_allow_numbers_as_labels() {
            let input = r#"graph TB;subgraph "number as labels";1;end;"#;
            let result = parse(input);
            assert!(result.is_ok(), "Failed to parse: {:?}", result);
            let db = result.unwrap();
            assert!(db.get_vertices().contains_key("1"));
        }

        #[test]
        fn should_add_acc_title_and_acc_descr() {
            let input = r#"graph LR
accTitle: Big decisions
accDescr: Flow chart of the decision making process
A[Hard] -->|Text| B(Round)"#;
            let result = parse(input);
            assert!(result.is_ok(), "Failed to parse: {:?}", result);
            let db = result.unwrap();
            assert_eq!(db.get_acc_title(), Some("Big decisions"));
            assert_eq!(
                db.get_acc_description(),
                Some("Flow chart of the decision making process")
            );
        }

        #[test]
        fn should_add_multiline_acc_descr() {
            let input = r#"graph LR
accTitle: Big decisions
accDescr {
    Flow chart of the decision making process
    with a second line
}
A[Hard] -->|Text| B(Round)"#;
            let result = parse(input);
            assert!(result.is_ok(), "Failed to parse: {:?}", result);
            let db = result.unwrap();
            assert_eq!(db.get_acc_title(), Some("Big decisions"));
            // The multiline description should contain the text
            let descr = db.get_acc_description().unwrap();
            assert!(descr.contains("Flow chart"));
            assert!(descr.contains("second line"));
        }
    }

    // Tests from flow-edges.spec.js
    mod flow_edges_tests {
        use super::*;
        use crate::diagrams::flowchart::types::EdgeStroke;

        #[test]
        #[ignore = "TODO: Add open-ended edge (---) support to grammar"]
        fn should_handle_open_ended_edges() {
            let input = "graph TD;A---B;";
            let result = parse(input);
            assert!(result.is_ok(), "Failed to parse open edge: {:?}", result);
            let db = result.unwrap();
            let edges = db.get_edges();
            assert_eq!(edges.len(), 1);
            assert_eq!(edges[0].start, "A");
            assert_eq!(edges[0].end, "B");
        }

        #[test]
        #[ignore = "TODO: Add cross arrow (--x) support to grammar"]
        fn should_handle_cross_ended_edges() {
            let input = "graph TD;A--xB;";
            let result = parse(input);
            assert!(result.is_ok(), "Failed to parse cross edge: {:?}", result);
            let db = result.unwrap();
            assert_eq!(db.get_edges().len(), 1);
        }

        #[test]
        #[ignore = "TODO: Add circle arrow (--o) support to grammar"]
        fn should_handle_circle_ended_edges() {
            let input = "graph TD;A--oB;";
            let result = parse(input);
            assert!(result.is_ok(), "Failed to parse circle edge: {:?}", result);
            let db = result.unwrap();
            assert_eq!(db.get_edges().len(), 1);
        }

        #[test]
        #[ignore = "TODO: Add pipe text (|text|) edge support to grammar"]
        fn should_handle_multiple_edges_with_text() {
            let input = "graph TD;A---|This is the 123 s text|B;\nA---|This is the second edge|B;";
            let result = parse(input);
            assert!(result.is_ok(), "Failed to parse: {:?}", result);
            let db = result.unwrap();
            let edges = db.get_edges();

            assert_eq!(edges.len(), 2);
            assert_eq!(edges[0].start, "A");
            assert_eq!(edges[0].end, "B");
            assert_eq!(edges[0].text, "This is the 123 s text");
            assert_eq!(edges[1].text, "This is the second edge");
        }

        #[test]
        fn should_handle_normal_edges_length_1() {
            let input = "graph TD;\nA --- B;";
            let result = parse(input);
            assert!(result.is_ok(), "Failed to parse: {:?}", result);
            let db = result.unwrap();
            assert_eq!(db.get_edges().len(), 1);
        }

        #[test]
        fn should_handle_normal_edges_length_2() {
            let input = "graph TD;\nA ---- B;";
            let result = parse(input);
            assert!(result.is_ok(), "Failed to parse: {:?}", result);
            let db = result.unwrap();
            assert_eq!(db.get_edges().len(), 1);
        }

        #[test]
        fn should_handle_normal_edges_length_3() {
            let input = "graph TD;\nA ----- B;";
            let result = parse(input);
            assert!(result.is_ok(), "Failed to parse: {:?}", result);
            let db = result.unwrap();
            assert_eq!(db.get_edges().len(), 1);
        }

        #[test]
        fn should_handle_arrow_edges_length_1() {
            let input = "graph TD;\nA --> B;";
            let result = parse(input);
            assert!(result.is_ok(), "Failed to parse: {:?}", result);
            let db = result.unwrap();
            assert_eq!(db.get_edges().len(), 1);
        }

        #[test]
        fn should_handle_arrow_edges_length_2() {
            let input = "graph TD;\nA ---> B;";
            let result = parse(input);
            assert!(result.is_ok(), "Failed to parse: {:?}", result);
            let db = result.unwrap();
            assert_eq!(db.get_edges().len(), 1);
        }

        #[test]
        fn should_handle_arrow_edges_length_3() {
            let input = "graph TD;\nA ----> B;";
            let result = parse(input);
            assert!(result.is_ok(), "Failed to parse: {:?}", result);
            let db = result.unwrap();
            assert_eq!(db.get_edges().len(), 1);
        }

        #[test]
        fn should_handle_thick_edges() {
            let input = "graph TD;\nA ==> B;";
            let result = parse(input);
            assert!(result.is_ok(), "Failed to parse thick edge: {:?}", result);
            let db = result.unwrap();
            assert_eq!(db.get_edges().len(), 1);
        }

        #[test]
        fn should_handle_dotted_edges() {
            let input = "graph TD;\nA -.-> B;";
            let result = parse(input);
            assert!(result.is_ok(), "Failed to parse dotted edge: {:?}", result);
            let db = result.unwrap();
            assert_eq!(db.get_edges().len(), 1);
        }

        #[test]
        fn should_handle_double_arrow_point() {
            let input = "graph TD;\nA <--> B;";
            let result = parse(input);
            assert!(result.is_ok(), "Failed to parse double arrow: {:?}", result);
            let db = result.unwrap();
            assert_eq!(db.get_edges().len(), 1);
        }

        #[test]
        fn should_handle_double_arrow_cross() {
            let input = "graph TD;\nA x--x B;";
            let result = parse(input);
            assert!(result.is_ok(), "Failed to parse double cross: {:?}", result);
        }

        #[test]
        fn should_handle_double_arrow_circle() {
            let input = "graph TD;\nA o--o B;";
            let result = parse(input);
            assert!(
                result.is_ok(),
                "Failed to parse double circle: {:?}",
                result
            );
        }

        #[test]
        fn should_handle_thick_double_arrow_point() {
            let input = "graph TD;\nA <==> B;";
            let result = parse(input);
            assert!(
                result.is_ok(),
                "Failed to parse thick double arrow: {:?}",
                result
            );
            let db = result.unwrap();
            assert_eq!(db.get_edges().len(), 1);
        }

        #[test]
        fn should_handle_dotted_double_arrow_point() {
            let input = "graph TD;\nA <-.-> B;";
            let result = parse(input);
            assert!(
                result.is_ok(),
                "Failed to parse dotted double arrow: {:?}",
                result
            );
            let db = result.unwrap();
            assert_eq!(db.get_edges().len(), 1);
        }

        #[test]
        fn should_handle_edge_with_text_normal() {
            let input = "graph TD;\nA -- text --> B;";
            let result = parse(input);
            assert!(
                result.is_ok(),
                "Failed to parse edge with text: {:?}",
                result
            );
            let db = result.unwrap();
            let edges = db.get_edges();
            assert_eq!(edges.len(), 1);
            assert_eq!(edges[0].text, "text");
        }

        #[test]
        fn should_handle_edge_with_text_thick() {
            let input = "graph TD;\nA == text ==> B;";
            let result = parse(input);
            assert!(
                result.is_ok(),
                "Failed to parse thick edge with text: {:?}",
                result
            );
            let db = result.unwrap();
            let edges = db.get_edges();
            assert_eq!(edges.len(), 1);
            assert_eq!(edges[0].text, "text");
        }

        #[test]
        fn should_handle_edge_with_text_dotted() {
            let input = "graph TD;\nA -. text .-> B;";
            let result = parse(input);
            assert!(
                result.is_ok(),
                "Failed to parse dotted edge with text: {:?}",
                result
            );
            let db = result.unwrap();
            let edges = db.get_edges();
            assert_eq!(edges.len(), 1);
            assert_eq!(edges[0].text, "text");
        }

        #[test]
        fn should_handle_pipe_text_edge_labels() {
            // This is the mermaid.js syntax: -->|Yes|
            let input = "flowchart LR\n    B -->|Yes| C\n    B -->|No| D";
            let result = parse(input);
            assert!(
                result.is_ok(),
                "Failed to parse pipe text edge: {:?}",
                result
            );
            let db = result.unwrap();
            let edges = db.get_edges();
            assert_eq!(edges.len(), 2, "Should have 2 edges");
            assert_eq!(edges[0].text, "Yes", "First edge should have 'Yes' label");
            assert_eq!(edges[1].text, "No", "Second edge should have 'No' label");
        }

        // Edge type and stroke verification tests
        #[test]
        fn should_set_arrow_point_type_for_normal_arrow() {
            let input = "graph TD;\nA --> B;";
            let result = parse(input);
            assert!(result.is_ok());
            let db = result.unwrap();
            let edges = db.get_edges();
            assert_eq!(edges[0].edge_type, Some("arrow_point".to_string()));
            assert_eq!(edges[0].stroke, EdgeStroke::Normal);
        }

        #[test]
        fn should_set_arrow_point_type_for_thick_arrow() {
            let input = "graph TD;\nA ==> B;";
            let result = parse(input);
            assert!(result.is_ok());
            let db = result.unwrap();
            let edges = db.get_edges();
            assert_eq!(edges[0].edge_type, Some("arrow_point".to_string()));
            assert_eq!(edges[0].stroke, EdgeStroke::Thick);
        }

        #[test]
        fn should_set_arrow_point_type_for_dotted_arrow() {
            let input = "graph TD;\nA -.-> B;";
            let result = parse(input);
            assert!(result.is_ok());
            let db = result.unwrap();
            let edges = db.get_edges();
            assert_eq!(edges[0].edge_type, Some("arrow_point".to_string()));
            assert_eq!(edges[0].stroke, EdgeStroke::Dotted);
        }

        #[test]
        fn should_set_double_arrow_point_type() {
            let input = "graph TD;\nA <--> B;";
            let result = parse(input);
            assert!(result.is_ok());
            let db = result.unwrap();
            let edges = db.get_edges();
            assert_eq!(edges[0].edge_type, Some("double_arrow_point".to_string()));
        }

        #[test]
        fn should_set_double_arrow_cross_type() {
            let input = "graph TD;\nA x--x B;";
            let result = parse(input);
            assert!(result.is_ok());
            let db = result.unwrap();
            let edges = db.get_edges();
            assert_eq!(edges[0].edge_type, Some("double_arrow_cross".to_string()));
        }

        #[test]
        fn should_set_double_arrow_circle_type() {
            let input = "graph TD;\nA o--o B;";
            let result = parse(input);
            assert!(result.is_ok());
            let db = result.unwrap();
            let edges = db.get_edges();
            assert_eq!(edges[0].edge_type, Some("double_arrow_circle".to_string()));
        }

        #[test]
        fn should_set_thick_double_arrow_point_type() {
            let input = "graph TD;\nA <==> B;";
            let result = parse(input);
            assert!(result.is_ok());
            let db = result.unwrap();
            let edges = db.get_edges();
            assert_eq!(edges[0].edge_type, Some("double_arrow_point".to_string()));
            assert_eq!(edges[0].stroke, EdgeStroke::Thick);
        }

        #[test]
        fn should_set_dotted_double_arrow_point_type() {
            let input = "graph TD;\nA <-.-> B;";
            let result = parse(input);
            assert!(result.is_ok());
            let db = result.unwrap();
            let edges = db.get_edges();
            assert_eq!(edges[0].edge_type, Some("double_arrow_point".to_string()));
            assert_eq!(edges[0].stroke, EdgeStroke::Dotted);
        }

        #[test]
        fn should_set_edge_length_for_normal_arrows() {
            // Length 1: -->
            let input = "graph TD;\nA --> B;";
            let db = parse(input).unwrap();
            assert_eq!(db.get_edges()[0].length, Some(1));

            // Length 2: --->
            let input = "graph TD;\nA ---> B;";
            let db = parse(input).unwrap();
            assert_eq!(db.get_edges()[0].length, Some(2));

            // Length 3: ---->
            let input = "graph TD;\nA ----> B;";
            let db = parse(input).unwrap();
            assert_eq!(db.get_edges()[0].length, Some(3));
        }

        #[test]
        fn should_set_edge_length_for_thick_arrows() {
            // Length 1: ==>
            let input = "graph TD;\nA ==> B;";
            let db = parse(input).unwrap();
            assert_eq!(db.get_edges()[0].length, Some(1));

            // Length 2: ===>
            let input = "graph TD;\nA ===> B;";
            let db = parse(input).unwrap();
            assert_eq!(db.get_edges()[0].length, Some(2));

            // Length 3: ====>
            let input = "graph TD;\nA ====> B;";
            let db = parse(input).unwrap();
            assert_eq!(db.get_edges()[0].length, Some(3));
        }

        #[test]
        fn should_preserve_edge_text_with_type_and_stroke() {
            let input = "graph TD;\nA -- Label --> B;";
            let result = parse(input);
            assert!(result.is_ok());
            let db = result.unwrap();
            let edges = db.get_edges();
            assert_eq!(edges[0].text, "Label");
            assert_eq!(edges[0].edge_type, Some("arrow_point".to_string()));
            assert_eq!(edges[0].stroke, EdgeStroke::Normal);
        }
    }

    // Tests for various shape types
    mod shape_tests {
        use super::*;

        #[test]
        fn should_parse_all_shape_types() {
            let inputs = vec![
                ("flowchart LR\nA[Square]", FlowVertexType::Square),
                ("flowchart LR\nA(Round)", FlowVertexType::Round),
                ("flowchart LR\nA((Circle))", FlowVertexType::Circle),
                ("flowchart LR\nA{Diamond}", FlowVertexType::Diamond),
                ("flowchart LR\nA[(Cylinder)]", FlowVertexType::Cylinder),
                ("flowchart LR\nA([Stadium])", FlowVertexType::Stadium),
                ("flowchart LR\nA{{Hexagon}}", FlowVertexType::Hexagon),
                ("flowchart LR\nA[[Subroutine]]", FlowVertexType::Subroutine),
                (
                    "flowchart LR\nA(((DoubleCircle)))",
                    FlowVertexType::DoubleCircle,
                ),
            ];

            for (input, expected_type) in inputs {
                let result = parse(input);
                assert!(
                    result.is_ok(),
                    "Failed to parse shape: {:?} for input: {}",
                    result,
                    input
                );
                let db = result.unwrap();
                let vertex = db.get_vertices().get("A").unwrap();
                assert_eq!(
                    vertex.vertex_type,
                    Some(expected_type),
                    "Wrong shape type for input: {}",
                    input
                );
            }
        }

        #[test]
        fn should_parse_trapezoid() {
            // [/...\] = trapezoid per mermaid.js jison grammar
            let input = r#"flowchart LR
A[/Trapezoid\]"#;
            let result = parse(input);
            assert!(result.is_ok(), "Failed to parse trapezoid: {:?}", result);
            let db = result.unwrap();
            let vertex = db.get_vertices().get("A").unwrap();
            assert_eq!(vertex.vertex_type, Some(FlowVertexType::Trapezoid));
        }

        #[test]
        fn should_parse_inv_trapezoid() {
            // [\../] = inv_trapezoid per mermaid.js jison grammar
            let input = r#"flowchart LR
A[\InvTrapezoid/]"#;
            let result = parse(input);
            assert!(
                result.is_ok(),
                "Failed to parse inv trapezoid: {:?}",
                result
            );
            let db = result.unwrap();
            let vertex = db.get_vertices().get("A").unwrap();
            assert_eq!(vertex.vertex_type, Some(FlowVertexType::InvTrapezoid));
        }

        #[test]
        fn should_parse_lean_right() {
            // [/../] = lean_right (parallelogram) per mermaid.js jison grammar
            let input = r#"flowchart LR
A[/LeanRight/]"#;
            let result = parse(input);
            assert!(result.is_ok(), "Failed to parse: {:?}", result);
            let db = result.unwrap();
            let vertex = db.get_vertices().get("A").unwrap();
            assert_eq!(vertex.vertex_type, Some(FlowVertexType::LeanRight));
        }

        #[test]
        fn should_parse_lean_left() {
            // [\...\] = lean_left (parallelogram) per mermaid.js jison grammar
            let input = r#"flowchart LR
A[\LeanLeft\]"#;
            let result = parse(input);
            assert!(result.is_ok(), "Failed to parse: {:?}", result);
            let db = result.unwrap();
            let vertex = db.get_vertices().get("A").unwrap();
            assert_eq!(vertex.vertex_type, Some(FlowVertexType::LeanLeft));
        }
    }

    // Tests for edge/link parsing
    mod edge_tests {
        use super::*;

        #[test]
        fn should_parse_link_arrow_directly() {
            // Test parsing just the link_arrow rule
            let test_cases = [
                "-->", "---", "-.->", "===", "--->", "-.", "--", ".->", "<.->", ".-",
            ];
            for input in &test_cases {
                let result = FlowchartParser::parse(Rule::link_arrow, input);
                assert!(
                    result.is_ok(),
                    "Failed to parse link_arrow '{}': {:?}",
                    input,
                    result.err()
                );
            }
        }

        #[test]
        fn should_parse_simple_link_directly() {
            // Test parsing just the simple_link rule
            let test_cases = ["-->", "---", "-.->", "==="];
            for input in &test_cases {
                let result = FlowchartParser::parse(Rule::simple_link, input);
                assert!(
                    result.is_ok(),
                    "Failed to parse simple_link '{}': {:?}",
                    input,
                    result.err()
                );
            }
        }

        #[test]
        fn should_parse_link_directly() {
            // Test parsing just the link rule
            let test_cases = ["-->", "---", "-.->", "==="];
            for input in &test_cases {
                let result = FlowchartParser::parse(Rule::link, input);
                assert!(
                    result.is_ok(),
                    "Failed to parse link '{}': {:?}",
                    input,
                    result.err()
                );
            }
        }

        #[test]
        fn should_parse_vertex_statement_with_triple_dash() {
            let test_cases = ["L1 --- L2", "L2 --- C", "A --- B"];
            for input in &test_cases {
                let result = FlowchartParser::parse(Rule::vertex_statement, input);
                assert!(
                    result.is_ok(),
                    "Failed to parse vertex_statement '{}': {:?}",
                    input,
                    result.err()
                );
            }
        }

        #[test]
        fn should_parse_two_statements() {
            let input = "flowchart TD\nL1 --- L2";
            let result = parse(input);
            assert!(result.is_ok(), "Failed to parse: {:?}", result);
        }

        #[test]
        fn should_parse_three_statements() {
            // This is the minimal failing case
            let input = "flowchart TD\nL1 --- L2\nL2 --- C";
            let result = parse(input);
            assert!(result.is_ok(), "Failed to parse: {:?}", result);
        }

        #[test]
        fn should_parse_two_arrow_edges() {
            let input = "flowchart TD\nL1 --> L2\nL2 --> C";
            let result = parse(input);
            assert!(result.is_ok(), "Failed to parse: {:?}", result);
        }

        #[test]
        fn should_parse_mixed_edges() {
            let input = "flowchart TD\nL1 --- L2\nL2 --> C";
            let result = parse(input);
            assert!(result.is_ok(), "Failed to parse: {:?}", result);
        }

        #[test]
        fn should_parse_mixed_edges_reverse() {
            let input = "flowchart TD\nL1 --> L2\nL2 --- C";
            let result = parse(input);
            assert!(result.is_ok(), "Failed to parse: {:?}", result);
        }

        #[test]
        fn should_parse_double_dash_edges() {
            let input = "flowchart TD\nL1 -- L2\nL2 -- C";
            let result = parse(input);
            assert!(result.is_ok(), "Failed to parse: {:?}", result);
        }

        #[test]
        fn should_parse_four_dash_edges() {
            let input = "flowchart TD\nL1 ---- L2\nL2 ---- C";
            let result = parse(input);
            assert!(result.is_ok(), "Failed to parse: {:?}", result);
        }

        #[test]
        fn should_parse_swapped_three_statements() {
            // Try different node names to see if it's name-related
            let input = "flowchart TD\nA --- B\nC --- D";
            let result = parse(input);
            assert!(result.is_ok(), "Failed to parse: {:?}", result);
        }

        #[test]
        fn should_parse_three_dash_single_line() {
            let input = "flowchart TD\nA --- B";
            let result = parse(input);
            assert!(result.is_ok(), "Failed to parse: {:?}", result);
        }

        #[test]
        fn should_parse_two_statements_with_indent() {
            let input = "flowchart TD\n      L1 --- L2";
            let result = parse(input);
            assert!(result.is_ok(), "Failed to parse: {:?}", result);
        }

        #[test]
        fn should_parse_triple_dash_edge() {
            let input = r#"flowchart TD
      L1 --- L2
      L2 --- C"#;
            let result = parse(input);
            assert!(result.is_ok(), "Failed to parse: {:?}", result);
        }
    }

    // Tests for prototype pollution protection
    mod security_tests {
        use super::*;

        #[test]
        fn should_work_with_proto_node_id() {
            let input = "graph LR\n__proto__ --> A;";
            let result = parse(input);
            assert!(result.is_ok(), "Should handle __proto__ node: {:?}", result);
        }

        #[test]
        fn should_work_with_constructor_node_id() {
            let input = "graph LR\nconstructor --> A;";
            let result = parse(input);
            assert!(
                result.is_ok(),
                "Should handle constructor node: {:?}",
                result
            );
        }

        #[test]
        fn should_work_with_proto_subgraph_id() {
            let input = "graph LR\n__proto__ --> A;\nsubgraph __proto__\n    C --> D;\nend;";
            let result = parse(input);
            assert!(
                result.is_ok(),
                "Should handle __proto__ subgraph: {:?}",
                result
            );
        }
    }
}
