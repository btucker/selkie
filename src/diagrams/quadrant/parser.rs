//! Quadrant chart diagram parser using pest grammar

use pest::Parser;
use pest_derive::Parser;

use super::types::QuadrantDb;

#[derive(Parser)]
#[grammar = "diagrams/quadrant/quadrant.pest"]
pub struct QuadrantParser;

/// Parse a quadrant chart diagram and return the populated database
pub fn parse(input: &str) -> Result<QuadrantDb, String> {
    let mut db = QuadrantDb::new();

    let pairs =
        QuadrantParser::parse(Rule::diagram, input).map_err(|e| format!("Parse error: {}", e))?;

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

fn process_document(db: &mut QuadrantDb, pair: pest::iterators::Pair<Rule>) -> Result<(), String> {
    for stmt in pair.into_inner() {
        process_statement(db, stmt)?;
    }
    Ok(())
}

fn process_statement(db: &mut QuadrantDb, pair: pest::iterators::Pair<Rule>) -> Result<(), String> {
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
                if inner.as_rule() == Rule::line_content {
                    db.set_diagram_title(inner.as_str().trim());
                }
            }
        }
        Rule::x_axis_stmt => {
            process_x_axis(db, pair)?;
        }
        Rule::y_axis_stmt => {
            process_y_axis(db, pair)?;
        }
        Rule::quadrant1_stmt => {
            for inner in pair.into_inner() {
                if inner.as_rule() == Rule::label_text {
                    let text = extract_label_text(inner);
                    db.set_quadrant1_text(&text);
                }
            }
        }
        Rule::quadrant2_stmt => {
            for inner in pair.into_inner() {
                if inner.as_rule() == Rule::label_text {
                    let text = extract_label_text(inner);
                    db.set_quadrant2_text(&text);
                }
            }
        }
        Rule::quadrant3_stmt => {
            for inner in pair.into_inner() {
                if inner.as_rule() == Rule::label_text {
                    let text = extract_label_text(inner);
                    db.set_quadrant3_text(&text);
                }
            }
        }
        Rule::quadrant4_stmt => {
            for inner in pair.into_inner() {
                if inner.as_rule() == Rule::label_text {
                    let text = extract_label_text(inner);
                    db.set_quadrant4_text(&text);
                }
            }
        }
        Rule::class_def_stmt => {
            process_class_def(db, pair)?;
        }
        Rule::point_stmt => {
            process_point(db, pair)?;
        }
        _ => {}
    }
    Ok(())
}

fn process_x_axis(db: &mut QuadrantDb, pair: pest::iterators::Pair<Rule>) -> Result<(), String> {
    let mut texts: Vec<String> = Vec::new();
    let mut has_arrow = false;

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::axis_text => {
                let text = extract_axis_text(inner);
                texts.push(text);
            }
            Rule::arrow => {
                has_arrow = true;
            }
            _ => {}
        }
    }

    if !texts.is_empty() {
        db.set_x_axis_left_text(&texts[0]);
    }
    if texts.len() > 1 && has_arrow {
        db.set_x_axis_right_text(&texts[1]);
    }
    Ok(())
}

fn process_y_axis(db: &mut QuadrantDb, pair: pest::iterators::Pair<Rule>) -> Result<(), String> {
    let mut texts: Vec<String> = Vec::new();
    let mut has_arrow = false;

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::axis_text => {
                let text = extract_axis_text(inner);
                texts.push(text);
            }
            Rule::arrow => {
                has_arrow = true;
            }
            _ => {}
        }
    }

    if !texts.is_empty() {
        db.set_y_axis_bottom_text(&texts[0]);
    }
    if texts.len() > 1 && has_arrow {
        db.set_y_axis_top_text(&texts[1]);
    }
    Ok(())
}

fn extract_axis_text(pair: pest::iterators::Pair<Rule>) -> String {
    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::quoted_string => {
                return unquote(inner.as_str());
            }
            Rule::unquoted_axis_text => {
                return inner.as_str().trim().to_string();
            }
            _ => {}
        }
    }
    String::new()
}

fn extract_label_text(pair: pest::iterators::Pair<Rule>) -> String {
    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::quoted_string => {
                return unquote(inner.as_str());
            }
            Rule::unquoted_label_text => {
                return inner.as_str().trim().to_string();
            }
            _ => {}
        }
    }
    String::new()
}

fn process_class_def(db: &mut QuadrantDb, pair: pest::iterators::Pair<Rule>) -> Result<(), String> {
    let mut class_name = String::new();
    let mut styles: Vec<String> = Vec::new();

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::class_name => {
                class_name = inner.as_str().to_string();
            }
            Rule::style_list => {
                styles.push(inner.as_str().trim().to_string());
            }
            _ => {}
        }
    }

    if !class_name.is_empty() {
        let style_refs: Vec<&str> = styles.iter().map(|s| s.as_str()).collect();
        db.add_class(&class_name, &style_refs);
    }
    Ok(())
}

fn process_point(db: &mut QuadrantDb, pair: pest::iterators::Pair<Rule>) -> Result<(), String> {
    let mut point_name = String::new();
    let mut class_name = String::new();
    let mut x = String::new();
    let mut y = String::new();
    let mut styles: Vec<String> = Vec::new();

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::point_name => {
                point_name = extract_point_name(inner);
            }
            Rule::class_ref => {
                for class_inner in inner.into_inner() {
                    if class_inner.as_rule() == Rule::class_name {
                        class_name = class_inner.as_str().to_string();
                    }
                }
            }
            Rule::coordinates => {
                let mut nums: Vec<String> = Vec::new();
                for coord_inner in inner.into_inner() {
                    if coord_inner.as_rule() == Rule::number {
                        nums.push(coord_inner.as_str().to_string());
                    }
                }
                if nums.len() >= 2 {
                    x = nums[0].clone();
                    y = nums[1].clone();
                }
            }
            Rule::point_styles => {
                for style_inner in inner.into_inner() {
                    if style_inner.as_rule() == Rule::style_param {
                        styles.push(style_inner.as_str().trim().to_string());
                    }
                }
            }
            _ => {}
        }
    }

    if !point_name.is_empty() {
        let style_refs: Vec<&str> = styles.iter().map(|s| s.as_str()).collect();
        db.add_point(&point_name, &class_name, &x, &y, &style_refs);
    }
    Ok(())
}

fn extract_point_name(pair: pest::iterators::Pair<Rule>) -> String {
    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::quoted_string => {
                return unquote(inner.as_str());
            }
            Rule::unquoted_point_name => {
                return inner.as_str().trim().to_string();
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
        fn should_parse_empty_quadrant_chart() {
            let result = parse("quadrantChart");
            assert!(result.is_ok(), "Parse error: {:?}", result.err());
        }

        #[test]
        fn should_reject_quadrant_without_keyword() {
            let result = parse("quadrant-1 do");
            assert!(result.is_err());
        }
    }

    mod axis_parsing {
        use super::*;

        #[test]
        fn should_parse_x_axis_with_arrow() {
            let result = parse("quadrantChart\nx-axis urgent --> not urgent");
            assert!(result.is_ok(), "Parse error: {:?}", result.err());
            let db = result.unwrap();
            assert_eq!(db.x_axis_left, "urgent");
            assert_eq!(db.x_axis_right, "not urgent");
        }

        #[test]
        fn should_parse_x_axis_quoted() {
            let result = parse("quadrantChart\nx-axis \"Urgent(* +=[\" --> \"Not Urgent (* +=[\"");
            assert!(result.is_ok(), "Parse error: {:?}", result.err());
            let db = result.unwrap();
            assert_eq!(db.x_axis_left, "Urgent(* +=[");
            assert_eq!(db.x_axis_right, "Not Urgent (* +=[");
        }

        #[test]
        fn should_parse_y_axis_with_arrow() {
            let result = parse("quadrantChart\ny-axis urgent --> not urgent");
            assert!(result.is_ok(), "Parse error: {:?}", result.err());
            let db = result.unwrap();
            assert_eq!(db.y_axis_bottom, "urgent");
            assert_eq!(db.y_axis_top, "not urgent");
        }
    }

    mod quadrant_parsing {
        use super::*;

        #[test]
        fn should_parse_quadrant_labels() {
            let result = parse("quadrantChart\nquadrant-1 Plan\nquadrant-2 do\nquadrant-3 delegate\nquadrant-4 delete");
            assert!(result.is_ok(), "Parse error: {:?}", result.err());
            let db = result.unwrap();
            assert_eq!(db.quadrant1, "Plan");
            assert_eq!(db.quadrant2, "do");
            assert_eq!(db.quadrant3, "delegate");
            assert_eq!(db.quadrant4, "delete");
        }

        #[test]
        fn should_parse_quoted_quadrant_labels() {
            let result = parse("quadrantChart\nquadrant-1 \"Plan(* +=[\"");
            assert!(result.is_ok(), "Parse error: {:?}", result.err());
            let db = result.unwrap();
            assert_eq!(db.quadrant1, "Plan(* +=[");
        }
    }

    mod title_parsing {
        use super::*;

        #[test]
        fn should_parse_title() {
            let result = parse("quadrantChart\ntitle this is title");
            assert!(result.is_ok(), "Parse error: {:?}", result.err());
            let db = result.unwrap();
            assert_eq!(db.title, "this is title");
        }
    }

    mod point_parsing {
        use super::*;

        #[test]
        fn should_parse_basic_point() {
            let result = parse("quadrantChart\npoint1: [0.1, 0.4]");
            assert!(result.is_ok(), "Parse error: {:?}", result.err());
            let db = result.unwrap();
            let points = db.get_points();
            assert_eq!(points.len(), 1);
            assert_eq!(points[0].text, "point1");
            assert_eq!(points[0].x, 0.1);
            assert_eq!(points[0].y, 0.4);
        }

        #[test]
        fn should_parse_point_with_quoted_name() {
            let result = parse("quadrantChart\n\"Point1 : (* +=[\" : [0.75, 0.5]");
            assert!(result.is_ok(), "Parse error: {:?}", result.err());
            let db = result.unwrap();
            let points = db.get_points();
            assert_eq!(points[0].text, "Point1 : (* +=[");
        }

        #[test]
        fn should_parse_point_with_styling() {
            let result = parse("quadrantChart\nMicrosoft: [0.75, 0.75] radius: 10");
            assert!(result.is_ok(), "Parse error: {:?}", result.err());
            let db = result.unwrap();
            let points = db.get_points();
            assert_eq!(points[0].text, "Microsoft");
            assert_eq!(points[0].style.radius, Some(10.0));
        }

        #[test]
        fn should_parse_point_with_all_styles() {
            let result = parse("quadrantChart\nIncorta: [0.20, 0.30] radius: 10, color: #ff0000, stroke-color: #ff00ff, stroke-width: 10px");
            assert!(result.is_ok(), "Parse error: {:?}", result.err());
            let db = result.unwrap();
            let points = db.get_points();
            assert_eq!(points[0].style.radius, Some(10.0));
            assert_eq!(points[0].style.color, Some("#ff0000".to_string()));
            assert_eq!(points[0].style.stroke_color, Some("#ff00ff".to_string()));
            assert_eq!(points[0].style.stroke_width, Some("10px".to_string()));
        }

        #[test]
        fn should_parse_point_with_class() {
            let result = parse("quadrantChart\nSalesforce:::class1: [0.55, 0.60]");
            assert!(result.is_ok(), "Parse error: {:?}", result.err());
            let db = result.unwrap();
            let points = db.get_points();
            assert_eq!(points[0].text, "Salesforce");
            assert_eq!(points[0].class_name, Some("class1".to_string()));
        }
    }

    mod class_def_parsing {
        use super::*;

        #[test]
        fn should_parse_class_def() {
            let result = parse("quadrantChart\nclassDef constructor fill:#ff0000");
            assert!(result.is_ok(), "Parse error: {:?}", result.err());
            let db = result.unwrap();
            let class = db.get_class("constructor");
            assert!(class.is_some());
            assert_eq!(class.unwrap().styles, vec!["fill:#ff0000"]);
        }
    }

    mod complex_charts {
        use super::*;

        #[test]
        fn should_parse_full_chart() {
            let input = r#"quadrantChart
      title Analytics and Business Intelligence Platforms
      x-axis "Completeness of Vision" --> "x-axis-2"
      y-axis Ability to Execute --> "y-axis-2"
      quadrant-1 Leaders
      quadrant-2 Challengers
      quadrant-3 Niche
      quadrant-4 Visionaries
      Microsoft: [0.75, 0.75]
      Salesforce: [0.55, 0.60]
      IBM: [0.51, 0.40]
      Incorta: [0.20, 0.30]"#;

            let result = parse(input);
            assert!(result.is_ok(), "Parse error: {:?}", result.err());
            let db = result.unwrap();

            assert_eq!(db.title, "Analytics and Business Intelligence Platforms");
            assert_eq!(db.x_axis_left, "Completeness of Vision");
            assert_eq!(db.x_axis_right, "x-axis-2");
            assert_eq!(db.y_axis_bottom, "Ability to Execute");
            assert_eq!(db.y_axis_top, "y-axis-2");
            assert_eq!(db.quadrant1, "Leaders");
            assert_eq!(db.quadrant2, "Challengers");
            assert_eq!(db.quadrant3, "Niche");
            assert_eq!(db.quadrant4, "Visionaries");

            let points = db.get_points();
            assert_eq!(points.len(), 4);
            assert_eq!(points[0].text, "Microsoft");
            assert_eq!(points[1].text, "Salesforce");
            assert_eq!(points[2].text, "IBM");
            assert_eq!(points[3].text, "Incorta");
        }
    }
}
