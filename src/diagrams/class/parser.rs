//! Class diagram parser using pest grammar

use pest::Parser;
use pest_derive::Parser;

use super::types::{ClassDb, ClassRelation, LineType, RelationDetails, RelationType};

#[derive(Parser)]
#[grammar = "diagrams/class/class.pest"]
pub struct ClassParser;

/// Parse a class diagram and return the populated database
pub fn parse(input: &str) -> Result<ClassDb, String> {
    let mut db = ClassDb::new();

    let pairs =
        ClassParser::parse(Rule::diagram, input).map_err(|e| format!("Parse error: {}", e))?;

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
    db: &mut ClassDb,
    pair: pest::iterators::Pair<Rule>,
    current_namespace: Option<&str>,
) -> Result<(), String> {
    for stmt in pair.into_inner() {
        process_statement(db, stmt, current_namespace)?;
    }
    Ok(())
}

fn process_statement(
    db: &mut ClassDb,
    pair: pest::iterators::Pair<Rule>,
    current_namespace: Option<&str>,
) -> Result<(), String> {
    match pair.as_rule() {
        Rule::statement | Rule::namespace_statement => {
            for inner in pair.into_inner() {
                process_statement(db, inner, current_namespace)?;
            }
        }
        Rule::comment_stmt => {
            // Ignore comments
        }
        Rule::direction_stmt => {
            for inner in pair.into_inner() {
                if inner.as_rule() == Rule::direction {
                    db.set_direction(inner.as_str());
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
            for inner in pair.into_inner() {
                match inner.as_rule() {
                    Rule::acc_descr_single => {
                        for content in inner.into_inner() {
                            if content.as_rule() == Rule::line_content {
                                db.acc_descr = content.as_str().trim().to_string();
                            }
                        }
                    }
                    Rule::acc_descr_multi => {
                        for content in inner.into_inner() {
                            if content.as_rule() == Rule::multiline_content {
                                // Normalize line endings and trim
                                db.acc_descr = content
                                    .as_str()
                                    .trim()
                                    .lines()
                                    .map(|l| l.trim())
                                    .collect::<Vec<_>>()
                                    .join("\n");
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
        Rule::namespace_stmt => {
            let mut ns_name = String::new();
            for inner in pair.into_inner() {
                match inner.as_rule() {
                    Rule::namespace_name => {
                        ns_name = inner.as_str().to_string();
                        db.add_namespace(&ns_name);
                    }
                    Rule::namespace_body => {
                        process_document(db, inner, Some(&ns_name))?;
                    }
                    _ => {}
                }
            }
        }
        Rule::class_stmt => {
            process_class_stmt(db, pair, current_namespace)?;
        }
        Rule::member_stmt => {
            process_member_stmt(db, pair)?;
        }
        Rule::relationship_stmt => {
            process_relationship_stmt(db, pair, current_namespace)?;
        }
        Rule::annotation_stmt => {
            let mut annotation = String::new();
            let mut class_name = String::new();
            for inner in pair.into_inner() {
                match inner.as_rule() {
                    Rule::annotation_text => {
                        annotation = inner.as_str().to_string();
                    }
                    Rule::class_name => {
                        class_name = cleanup_class_name(inner.as_str());
                    }
                    _ => {}
                }
            }
            if !class_name.is_empty() {
                db.add_annotation(&class_name, &annotation);
            }
        }
        Rule::note_stmt => {
            process_note_stmt(db, pair)?;
        }
        Rule::click_stmt => {
            process_click_stmt(db, pair)?;
        }
        Rule::link_stmt => {
            process_link_stmt(db, pair)?;
        }
        Rule::callback_stmt => {
            process_callback_stmt(db, pair)?;
        }
        Rule::class_def_stmt => {
            process_class_def_stmt(db, pair)?;
        }
        Rule::css_class_stmt => {
            process_css_class_stmt(db, pair)?;
        }
        Rule::style_stmt => {
            process_style_stmt(db, pair)?;
        }
        _ => {}
    }
    Ok(())
}

fn process_class_stmt(
    db: &mut ClassDb,
    pair: pest::iterators::Pair<Rule>,
    namespace: Option<&str>,
) -> Result<(), String> {
    let mut class_name = String::new();
    let mut generic_type = String::new();
    let mut text_label = String::new();
    let mut css_class = String::new();
    let mut members: Vec<String> = Vec::new();
    let mut annotations: Vec<String> = Vec::new();

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::class_name => {
                class_name = cleanup_class_name(inner.as_str());
            }
            Rule::generic_type => {
                // Extract content between ~ markers
                let s = inner.as_str();
                generic_type = s[1..s.len() - 1].to_string();
            }
            Rule::text_label => {
                for label in inner.into_inner() {
                    if label.as_rule() == Rule::quoted_text {
                        let s = label.as_str();
                        text_label = s[1..s.len() - 1].to_string();
                    }
                }
            }
            Rule::css_shorthand => {
                for id in inner.into_inner() {
                    if id.as_rule() == Rule::identifier {
                        css_class = id.as_str().to_string();
                    }
                }
            }
            Rule::class_body => {
                for line in inner.into_inner() {
                    if line.as_rule() == Rule::class_body_line {
                        for content in line.into_inner() {
                            match content.as_rule() {
                                Rule::annotation_line => {
                                    for ann in content.into_inner() {
                                        if ann.as_rule() == Rule::annotation_text {
                                            annotations.push(ann.as_str().to_string());
                                        }
                                    }
                                }
                                Rule::member_line => {
                                    let member = content.as_str().trim();
                                    if !member.is_empty() {
                                        members.push(member.to_string());
                                    }
                                }
                                Rule::separator_line | Rule::bracket_comment => {
                                    // Ignore separators and comments
                                }
                                _ => {}
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }

    if !class_name.is_empty() {
        db.add_class(&class_name);

        if let Some(class) = db.get_class_mut(&class_name) {
            if !generic_type.is_empty() {
                class.type_param = generic_type;
            }
            if !text_label.is_empty() {
                class.label = text_label;
            } else {
                class.label = class_name.clone();
            }
            if !css_class.is_empty() {
                if class.css_classes.is_empty() {
                    class.css_classes = "default".to_string();
                }
                class.css_classes.push(' ');
                class.css_classes.push_str(&css_class);
            } else if class.css_classes.is_empty() {
                class.css_classes = "default".to_string();
            }
            if let Some(ns) = namespace {
                class.parent = Some(ns.to_string());
            }
            for ann in annotations {
                class.annotations.push(ann);
            }
        }

        // Add members after class is set up
        for member in members {
            db.add_member(&class_name, &member);
        }
    }

    Ok(())
}

fn process_member_stmt(db: &mut ClassDb, pair: pest::iterators::Pair<Rule>) -> Result<(), String> {
    let mut class_name = String::new();
    let mut member = String::new();
    let mut generic_type = String::new();

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::class_name => {
                class_name = cleanup_class_name(inner.as_str());
            }
            Rule::generic_type => {
                let s = inner.as_str();
                generic_type = s[1..s.len() - 1].to_string();
            }
            Rule::member_text => {
                member = inner.as_str().trim().to_string();
            }
            _ => {}
        }
    }

    if !class_name.is_empty() {
        db.add_class(&class_name);

        // Set generic type if present
        if !generic_type.is_empty() {
            if let Some(class) = db.get_class_mut(&class_name) {
                if class.type_param.is_empty() {
                    class.type_param = generic_type;
                }
            }
        }

        if !member.is_empty() {
            db.add_member(&class_name, &member);
        }
    }

    Ok(())
}

fn process_relationship_stmt(
    db: &mut ClassDb,
    pair: pest::iterators::Pair<Rule>,
    namespace: Option<&str>,
) -> Result<(), String> {
    let mut id1 = String::new();
    let mut id2 = String::new();
    let mut card1 = String::new();
    let mut card2 = String::new();
    let mut rel_type_str = String::new();
    let mut label = String::new();
    let mut generic1 = String::new();
    let mut generic2 = String::new();
    let mut css1 = String::new();
    let mut css2 = String::new();
    let mut class_ref_count = 0;

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::class_ref => {
                class_ref_count += 1;
                for ref_inner in inner.into_inner() {
                    match ref_inner.as_rule() {
                        Rule::class_name => {
                            let name = cleanup_class_name(ref_inner.as_str());
                            if class_ref_count == 1 {
                                id1 = name;
                            } else {
                                id2 = name;
                            }
                        }
                        Rule::generic_type => {
                            let s = ref_inner.as_str();
                            let gt = s[1..s.len() - 1].to_string();
                            if class_ref_count == 1 {
                                generic1 = gt;
                            } else {
                                generic2 = gt;
                            }
                        }
                        Rule::css_shorthand => {
                            for id in ref_inner.into_inner() {
                                if id.as_rule() == Rule::identifier {
                                    if class_ref_count == 1 {
                                        css1 = id.as_str().to_string();
                                    } else {
                                        css2 = id.as_str().to_string();
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
            Rule::cardinality_left => {
                let s = inner.as_str();
                card1 = s[1..s.len() - 1].to_string();
            }
            Rule::cardinality_right => {
                let s = inner.as_str();
                card2 = s[1..s.len() - 1].to_string();
            }
            Rule::rel_type => {
                rel_type_str = inner.as_str().to_string();
            }
            Rule::relation_label => {
                label = inner.as_str().trim().to_string();
            }
            _ => {}
        }
    }

    // Add classes if they don't exist
    if !id1.is_empty() {
        db.add_class(&id1);
        if let Some(class) = db.get_class_mut(&id1) {
            if !generic1.is_empty() && class.type_param.is_empty() {
                class.type_param = generic1;
            }
            if class.label.is_empty() {
                class.label = id1.clone();
            }
            if class.css_classes.is_empty() {
                class.css_classes = "default".to_string();
            }
            if !css1.is_empty() {
                class.css_classes.push(' ');
                class.css_classes.push_str(&css1);
            }
            if namespace.is_some() && class.parent.is_none() {
                class.parent = namespace.map(|s| s.to_string());
            }
        }
    }

    if !id2.is_empty() {
        db.add_class(&id2);
        if let Some(class) = db.get_class_mut(&id2) {
            if !generic2.is_empty() && class.type_param.is_empty() {
                class.type_param = generic2;
            }
            if class.label.is_empty() {
                class.label = id2.clone();
            }
            if class.css_classes.is_empty() {
                class.css_classes = "default".to_string();
            }
            if !css2.is_empty() {
                class.css_classes.push(' ');
                class.css_classes.push_str(&css2);
            }
            if namespace.is_some() && class.parent.is_none() {
                class.parent = namespace.map(|s| s.to_string());
            }
        }
    }

    // Parse the relationship type
    let (type1, type2, line_type) = parse_relation_type(&rel_type_str);

    let relation = ClassRelation {
        id1,
        id2,
        relation_title1: card1,
        relation_title2: card2,
        relation_type: rel_type_str,
        title: label,
        text: String::new(),
        style: Vec::new(),
        relation: RelationDetails {
            type1,
            type2,
            line_type,
        },
    };

    db.add_relation(relation);

    Ok(())
}

fn parse_relation_type(rel: &str) -> (i32, i32, LineType) {
    let line_type = if rel.contains("..") {
        LineType::Dotted
    } else {
        LineType::Solid
    };

    // Left side (type1) - reading from the left end
    let type1 = if rel.starts_with("<|") {
        RelationType::Extension as i32
    } else if rel.starts_with("o") {
        RelationType::Aggregation as i32
    } else if rel.starts_with("*") {
        RelationType::Composition as i32
    } else if rel.starts_with("<") {
        RelationType::Dependency as i32
    } else {
        -1 // "none"
    };

    // Right side (type2) - reading from the right end
    let type2 = if rel.ends_with("|>") {
        RelationType::Extension as i32
    } else if rel.ends_with("o)") || rel.ends_with("o") && !rel.starts_with("o") {
        if rel.contains(")") {
            RelationType::Lollipop as i32
        } else {
            RelationType::Aggregation as i32
        }
    } else if rel.ends_with("*") {
        RelationType::Composition as i32
    } else if rel.ends_with(">") {
        RelationType::Dependency as i32
    } else {
        -1 // "none"
    };

    (type1, type2, line_type)
}

fn process_note_stmt(db: &mut ClassDb, pair: pest::iterators::Pair<Rule>) -> Result<(), String> {
    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::note_for_stmt => {
                let mut class_name = String::new();
                let mut text = String::new();
                for note_inner in inner.into_inner() {
                    match note_inner.as_rule() {
                        Rule::class_name => {
                            class_name = cleanup_class_name(note_inner.as_str());
                        }
                        Rule::note_content => {
                            let s = note_inner.as_str();
                            // Remove surrounding quotes
                            text = s[1..s.len() - 1].to_string();
                        }
                        _ => {}
                    }
                }
                db.add_note(&text, &class_name);
            }
            Rule::note_general_stmt => {
                let mut text = String::new();
                for note_inner in inner.into_inner() {
                    if note_inner.as_rule() == Rule::note_content {
                        let s = note_inner.as_str();
                        // Remove surrounding quotes
                        text = s[1..s.len() - 1].to_string();
                    }
                }
                db.add_note(&text, "");
            }
            _ => {}
        }
    }
    Ok(())
}

fn process_click_stmt(db: &mut ClassDb, pair: pest::iterators::Pair<Rule>) -> Result<(), String> {
    let mut class_name = String::new();
    let mut href = String::new();
    let mut tooltip = String::new();
    let mut target = String::new();
    let mut callback = String::new();
    let mut quotes_seen = 0;

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::class_name => {
                class_name = cleanup_class_name(inner.as_str());
            }
            Rule::quoted_text => {
                let s = inner.as_str();
                let unquoted = s[1..s.len() - 1].to_string();
                quotes_seen += 1;
                if quotes_seen == 1 {
                    href = unquoted;
                } else if quotes_seen == 2 {
                    tooltip = unquoted;
                }
            }
            Rule::target => {
                target = inner.as_str().to_string();
            }
            Rule::callback_def => {
                callback = inner.as_str().to_string();
                // Extract function name from "funcName(args)"
                if let Some(paren_pos) = callback.find('(') {
                    callback = callback[..paren_pos].to_string();
                }
            }
            _ => {}
        }
    }

    if !class_name.is_empty() {
        if !href.is_empty() {
            db.set_link(&class_name, &href, &target);
            if !tooltip.is_empty() {
                db.set_tooltip(&class_name, Some(&tooltip));
            }
            // Add clickable css class
            if let Some(class) = db.get_class_mut(&class_name) {
                if !class.css_classes.contains("clickable") {
                    class.css_classes.push_str(" clickable");
                }
            }
        } else if !callback.is_empty() {
            if let Some(class) = db.get_class_mut(&class_name) {
                class.have_callback = true;
            }
        }
    }

    Ok(())
}

fn process_link_stmt(db: &mut ClassDb, pair: pest::iterators::Pair<Rule>) -> Result<(), String> {
    let mut class_name = String::new();
    let mut href = String::new();
    let mut tooltip = String::new();
    let mut target = String::new();
    let mut quotes_seen = 0;

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::class_name => {
                class_name = cleanup_class_name(inner.as_str());
            }
            Rule::quoted_text => {
                let s = inner.as_str();
                let unquoted = s[1..s.len() - 1].to_string();
                quotes_seen += 1;
                if quotes_seen == 1 {
                    href = unquoted;
                } else if quotes_seen == 2 {
                    tooltip = unquoted;
                }
            }
            Rule::target => {
                target = inner.as_str().to_string();
            }
            _ => {}
        }
    }

    if !class_name.is_empty() && !href.is_empty() {
        db.set_link(&class_name, &href, &target);
        if !tooltip.is_empty() {
            db.set_tooltip(&class_name, Some(&tooltip));
        }
        // Add clickable css class
        if let Some(class) = db.get_class_mut(&class_name) {
            if !class.css_classes.contains("clickable") {
                class.css_classes.push_str(" clickable");
            }
        }
    }

    Ok(())
}

fn process_callback_stmt(
    db: &mut ClassDb,
    pair: pest::iterators::Pair<Rule>,
) -> Result<(), String> {
    let mut class_name = String::new();

    for inner in pair.into_inner() {
        if inner.as_rule() == Rule::class_name {
            class_name = cleanup_class_name(inner.as_str());
        }
    }

    if !class_name.is_empty() {
        db.add_class(&class_name);
        if let Some(class) = db.get_class_mut(&class_name) {
            class.have_callback = true;
        }
    }

    Ok(())
}

fn process_class_def_stmt(
    db: &mut ClassDb,
    pair: pest::iterators::Pair<Rule>,
) -> Result<(), String> {
    let mut class_name = String::new();
    let mut styles = String::new();

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::identifier => {
                class_name = inner.as_str().to_string();
            }
            Rule::style_list => {
                styles = inner.as_str().trim().to_string();
            }
            _ => {}
        }
    }

    if !class_name.is_empty() && !styles.is_empty() {
        let style_class = super::types::StyleClass {
            id: class_name.clone(),
            styles: styles.split(',').map(|s| s.trim().to_string()).collect(),
            text_styles: Vec::new(),
        };
        db.style_classes.insert(class_name, style_class);
    }

    Ok(())
}

fn process_css_class_stmt(
    db: &mut ClassDb,
    pair: pest::iterators::Pair<Rule>,
) -> Result<(), String> {
    let mut class_ids = String::new();
    let mut css_class = String::new();

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::quoted_class_list => {
                let s = inner.as_str();
                class_ids = s[1..s.len() - 1].to_string();
            }
            Rule::identifier => {
                css_class = inner.as_str().to_string();
            }
            _ => {}
        }
    }

    if !class_ids.is_empty() && !css_class.is_empty() {
        db.set_css_class(&class_ids, &css_class);
    }

    Ok(())
}

fn process_style_stmt(db: &mut ClassDb, pair: pest::iterators::Pair<Rule>) -> Result<(), String> {
    let mut ids: Vec<String> = Vec::new();
    let mut styles = String::new();

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::id_list => {
                for id in inner.into_inner() {
                    if id.as_rule() == Rule::identifier {
                        ids.push(id.as_str().to_string());
                    }
                }
            }
            Rule::style_list => {
                styles = inner.as_str().trim().to_string();
            }
            _ => {}
        }
    }

    let style_vec: Vec<String> = styles.split(',').map(|s| s.trim().to_string()).collect();
    for id in ids {
        db.set_css_style(&id, style_vec.clone());
    }

    Ok(())
}

/// Clean up a class name by removing backticks
fn cleanup_class_name(name: &str) -> String {
    let s = name.trim();
    if s.starts_with('`') && s.ends_with('`') && s.len() > 2 {
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
        fn should_parse_empty_diagram() {
            let result = parse("classDiagram\n");
            assert!(result.is_ok());
        }

        #[test]
        fn should_parse_simple_class() {
            let result = parse("classDiagram\nclass Car");
            assert!(result.is_ok());
            let db = result.unwrap();
            assert!(db.get_class("Car").is_some());
        }

        #[test]
        fn should_handle_class_names_with_dash() {
            let result = parse("classDiagram\nclass Ca-r");
            assert!(result.is_ok());
            let db = result.unwrap();
            let class = db.get_class("Ca-r").unwrap();
            assert_eq!(class.label, "Ca-r");
        }

        #[test]
        fn should_handle_backticked_class_name() {
            let result = parse("classDiagram\nclass `Car`");
            assert!(result.is_ok());
            let db = result.unwrap();
            assert!(db.get_class("Car").is_some());
        }

        #[test]
        fn should_handle_class_with_underscore() {
            let result = parse("classDiagram\nclass `A_Car`");
            assert!(result.is_ok());
            let db = result.unwrap();
            assert!(db.get_class("A_Car").is_some());
        }
    }

    mod accessibility {
        use super::*;

        #[test]
        fn should_handle_acc_title_and_acc_descr() {
            let result = parse(
                "classDiagram
            accTitle: My Title
            accDescr: My Description",
            );
            assert!(result.is_ok());
            let db = result.unwrap();
            assert_eq!(db.acc_title, "My Title");
            assert_eq!(db.acc_descr, "My Description");
        }

        #[test]
        fn should_handle_multiline_acc_descr() {
            let result = parse(
                "classDiagram
            accTitle: My Title
            accDescr {
              This is my multi
              line description
            }",
            );
            assert!(result.is_ok());
            let db = result.unwrap();
            assert_eq!(db.acc_title, "My Title");
            assert_eq!(db.acc_descr, "This is my multi\nline description");
        }
    }

    mod text_labels {
        use super::*;

        #[test]
        fn should_parse_class_with_text_label() {
            let result = parse("classDiagram\nclass C1[\"Class 1 with text label\"]");
            assert!(result.is_ok());
            let db = result.unwrap();
            let c1 = db.get_class("C1").unwrap();
            assert_eq!(c1.label, "Class 1 with text label");
        }

        #[test]
        fn should_parse_two_classes_with_text_labels() {
            let result = parse(
                "classDiagram
class C1[\"Class 1 with text label\"]
class C2[\"Class 2 with chars @?\"]",
            );
            assert!(result.is_ok());
            let db = result.unwrap();
            let c1 = db.get_class("C1").unwrap();
            assert_eq!(c1.label, "Class 1 with text label");
            let c2 = db.get_class("C2").unwrap();
            assert_eq!(c2.label, "Class 2 with chars @?");
        }

        #[test]
        fn should_parse_class_with_text_label_and_css_shorthand() {
            let result = parse("classDiagram\nclass C1[\"Class 1 with text label\"]:::styleClass");
            assert!(result.is_ok());
            let db = result.unwrap();
            let c1 = db.get_class("C1").unwrap();
            assert_eq!(c1.label, "Class 1 with text label");
            assert!(c1.css_classes.contains("styleClass"));
        }
    }

    mod class_body {
        use super::*;

        #[test]
        fn should_handle_member_definitions() {
            let result = parse(
                "classDiagram
class Car{
+int wheels
}",
            );
            assert!(result.is_ok());
            let db = result.unwrap();
            let class = db.get_class("Car").unwrap();
            assert_eq!(class.members.len(), 1);
        }

        #[test]
        fn should_handle_method_definitions() {
            let result = parse(
                "classDiagram
class Car{
+size()
}",
            );
            assert!(result.is_ok());
            let db = result.unwrap();
            let class = db.get_class("Car").unwrap();
            assert_eq!(class.methods.len(), 1);
        }

        #[test]
        fn should_add_bracket_members_in_right_order() {
            let result = parse(
                "classDiagram
class Class1 {
int testMember
test()
string fooMember
foo()
}",
            );
            assert!(result.is_ok());
            let db = result.unwrap();
            let class = db.get_class("Class1").unwrap();
            assert_eq!(class.members.len(), 2);
            assert_eq!(class.methods.len(), 2);
            assert_eq!(
                class.members[0].get_display_details().display_text,
                "int testMember"
            );
            assert_eq!(
                class.members[1].get_display_details().display_text,
                "string fooMember"
            );
            assert_eq!(
                class.methods[0].get_display_details().display_text,
                "test()"
            );
            assert_eq!(class.methods[1].get_display_details().display_text, "foo()");
        }

        #[test]
        fn should_handle_annotation_in_brackets() {
            let result = parse(
                "classDiagram
class Class1 {
<<interface>>
}",
            );
            assert!(result.is_ok());
            let db = result.unwrap();
            let class = db.get_class("Class1").unwrap();
            assert_eq!(class.annotations.len(), 1);
            assert_eq!(class.annotations[0], "interface");
        }

        #[test]
        fn should_handle_empty_class_body() {
            let result = parse("classDiagram\nclass EmptyClass {}");
            assert!(result.is_ok());
            let db = result.unwrap();
            let class = db.get_class("EmptyClass").unwrap();
            assert_eq!(class.label, "EmptyClass");
            assert_eq!(class.members.len(), 0);
            assert_eq!(class.methods.len(), 0);
        }

        #[test]
        fn should_handle_text_label_with_members() {
            let result = parse(
                "classDiagram
class C1[\"Class 1 with text label\"] {
+member1
}",
            );
            assert!(result.is_ok());
            let db = result.unwrap();
            let c1 = db.get_class("C1").unwrap();
            assert_eq!(c1.label, "Class 1 with text label");
            assert_eq!(c1.members.len(), 1);
        }
    }

    mod member_statements {
        use super::*;

        #[test]
        fn should_handle_simple_member() {
            let result = parse(
                "classDiagram
class Car
Car : wheels",
            );
            assert!(result.is_ok());
        }

        #[test]
        fn should_handle_direct_member_declaration() {
            let result = parse("classDiagram\nCar : wheels");
            assert!(result.is_ok());
            let db = result.unwrap();
            let car = db.get_class("Car").unwrap();
            assert_eq!(car.members.len(), 1);
            assert_eq!(car.members[0].id, "wheels");
        }

        #[test]
        fn should_handle_member_with_type() {
            let result = parse("classDiagram\nCar : int wheels");
            assert!(result.is_ok());
            let db = result.unwrap();
            let car = db.get_class("Car").unwrap();
            assert_eq!(car.members.len(), 1);
            assert_eq!(car.members[0].id, "int wheels");
        }

        #[test]
        fn should_handle_visibility() {
            let result = parse(
                "classDiagram
class actual
actual : -int privateMember
actual : +int publicMember
actual : #int protectedMember
actual : ~int privatePackage",
            );
            assert!(result.is_ok());
            let db = result.unwrap();
            let class = db.get_class("actual").unwrap();
            assert_eq!(class.members.len(), 4);
        }
    }

    mod relationships {
        use super::*;

        #[test]
        fn should_handle_basic_relationships() {
            let result = parse(
                "classDiagram
Class1 <|-- Class02
Class03 *-- Class04
Class05 o-- Class06
Class07 .. Class08
Class09 -- Class1",
            );
            assert!(result.is_ok());
            let db = result.unwrap();
            assert_eq!(db.relations.len(), 5);
        }

        #[test]
        fn should_handle_extension() {
            let result = parse("classDiagram\nClass1 <|-- Class02");
            assert!(result.is_ok());
            let db = result.unwrap();
            assert_eq!(db.relations.len(), 1);
            assert_eq!(
                db.relations[0].relation.type1,
                RelationType::Extension as i32
            );
            assert_eq!(db.relations[0].relation.type2, -1);
            assert_eq!(db.relations[0].relation.line_type, LineType::Solid);
        }

        #[test]
        fn should_handle_aggregation_dotted() {
            let result = parse("classDiagram\nClass1 o.. Class02");
            assert!(result.is_ok());
            let db = result.unwrap();
            assert_eq!(
                db.relations[0].relation.type1,
                RelationType::Aggregation as i32
            );
            assert_eq!(db.relations[0].relation.line_type, LineType::Dotted);
        }

        #[test]
        fn should_handle_composition_both_sides() {
            let result = parse("classDiagram\nClass1 *--* Class02");
            assert!(result.is_ok());
            let db = result.unwrap();
            assert_eq!(
                db.relations[0].relation.type1,
                RelationType::Composition as i32
            );
            assert_eq!(
                db.relations[0].relation.type2,
                RelationType::Composition as i32
            );
        }

        #[test]
        fn should_handle_cardinality_and_labels() {
            let result = parse(
                "classDiagram
Class1 \"1\" *-- \"many\" Class02 : contains",
            );
            assert!(result.is_ok());
            let db = result.unwrap();
            assert_eq!(db.relations[0].relation_title1, "1");
            assert_eq!(db.relations[0].relation_title2, "many");
            assert_eq!(db.relations[0].title, "contains");
        }

        #[test]
        fn should_handle_generics_in_relations() {
            let result = parse("classDiagram\nClass1~T~ <|-- Class02");
            assert!(result.is_ok());
            let db = result.unwrap();
            let class1 = db.get_class("Class1").unwrap();
            assert_eq!(class1.type_param, "T");
        }
    }

    mod namespaces {
        use super::*;

        #[test]
        fn should_handle_namespace() {
            let result = parse(
                "classDiagram
namespace Namespace1 { class Class1 }",
            );
            assert!(result.is_ok());
            let db = result.unwrap();
            assert!(db.namespaces.contains_key("Namespace1"));
        }

        #[test]
        fn should_handle_classes_within_namespaces() {
            let result = parse(
                "classDiagram
namespace Company.Project {
  class User {
    +login(username: String, password: String)
    +logout()
  }
}",
            );
            assert!(result.is_ok());
            let db = result.unwrap();
            let user = db.get_class("User").unwrap();
            assert_eq!(user.parent, Some("Company.Project".to_string()));
            assert_eq!(user.methods.len(), 2);
        }

        #[test]
        fn should_handle_nested_namespaces_and_relationships() {
            let result = parse(
                "classDiagram
namespace Company.Project.Module.SubModule {
  class Report {
    +generatePDF(data: List)
  }
}
namespace Company.Project.Module {
  class Admin {
    +generateReport()
  }
}
Admin --> Report : generates",
            );
            assert!(result.is_ok());
            let db = result.unwrap();
            let report = db.get_class("Report").unwrap();
            assert_eq!(
                report.parent,
                Some("Company.Project.Module.SubModule".to_string())
            );
            let admin = db.get_class("Admin").unwrap();
            assert_eq!(admin.parent, Some("Company.Project.Module".to_string()));
            assert_eq!(db.relations[0].title, "generates");
        }
    }

    mod annotations {
        use super::*;

        #[test]
        fn should_handle_class_annotation() {
            let result = parse(
                "classDiagram
class Class1
<<interface>> Class1",
            );
            assert!(result.is_ok());
            let db = result.unwrap();
            let class = db.get_class("Class1").unwrap();
            assert_eq!(class.annotations.len(), 1);
            assert_eq!(class.annotations[0], "interface");
        }
    }

    mod notes {
        use super::*;

        #[test]
        fn should_handle_note_for() {
            let result = parse(
                "classDiagram
Class11 <|.. Class12
note for Class11 \"test\"",
            );
            assert!(result.is_ok());
            let db = result.unwrap();
            assert_eq!(db.notes.len(), 1);
            let note = db.notes.get("note0").unwrap();
            assert_eq!(note.text, "test");
            assert_eq!(note.class, "Class11");
        }

        #[test]
        fn should_handle_general_note() {
            let result = parse(
                "classDiagram
note \"test\"",
            );
            assert!(result.is_ok());
            let db = result.unwrap();
            assert_eq!(db.notes.len(), 1);
        }
    }

    mod direction {
        use super::*;

        #[test]
        fn should_parse_direction() {
            let result = parse(
                "classDiagram
direction TB
class Student",
            );
            assert!(result.is_ok());
            let db = result.unwrap();
            assert_eq!(db.direction, "TB");
        }

        #[test]
        fn should_default_to_tb() {
            let result = parse("classDiagram\nclass A");
            assert!(result.is_ok());
            let db = result.unwrap();
            assert_eq!(db.direction, "TB");
        }
    }

    mod comments {
        use super::*;

        #[test]
        fn should_handle_comments_at_start() {
            let result = parse(
                "%% Comment
classDiagram
class Class1",
            );
            assert!(result.is_ok());
        }

        #[test]
        fn should_handle_comments_at_end() {
            let result = parse(
                "classDiagram
class Class1
%% Comment",
            );
            assert!(result.is_ok());
        }

        #[test]
        fn should_handle_comments_inside_brackets() {
            let result = parse(
                "classDiagram
class Class1 {
%% Comment Class1 <|-- Class02
int : test
}",
            );
            assert!(result.is_ok());
        }
    }

    mod click_and_links {
        use super::*;

        #[test]
        fn should_handle_href_link() {
            let result = parse(
                "classDiagram
class Class1
click Class1 href \"google.com\"",
            );
            assert!(result.is_ok());
            let db = result.unwrap();
            let class = db.get_class("Class1").unwrap();
            assert_eq!(class.link, Some("google.com".to_string()));
            assert!(class.css_classes.contains("clickable"));
        }

        #[test]
        fn should_handle_href_with_tooltip() {
            let result = parse(
                "classDiagram
class Class1
click Class1 href \"google.com\" \"A Tooltip\"",
            );
            assert!(result.is_ok());
            let db = result.unwrap();
            let class = db.get_class("Class1").unwrap();
            assert_eq!(class.link, Some("google.com".to_string()));
            assert_eq!(class.tooltip, Some("A Tooltip".to_string()));
        }

        #[test]
        fn should_handle_href_with_target() {
            let result = parse(
                "classDiagram
class Class1
click Class1 href \"google.com\" \"A tooltip\" _self",
            );
            assert!(result.is_ok());
            let db = result.unwrap();
            let class = db.get_class("Class1").unwrap();
            assert_eq!(class.link, Some("google.com".to_string()));
            assert_eq!(class.link_target, Some("_self".to_string()));
        }

        #[test]
        fn should_handle_link_statement() {
            let result = parse(
                "classDiagram
class Class1
link Class1 \"google.com\"",
            );
            assert!(result.is_ok());
            let db = result.unwrap();
            let class = db.get_class("Class1").unwrap();
            assert_eq!(class.link, Some("google.com".to_string()));
            assert!(class.css_classes.contains("clickable"));
        }
    }

    mod generics {
        use super::*;

        #[test]
        fn should_handle_generic_class() {
            let result = parse("classDiagram\nclass Car~T~");
            assert!(result.is_ok());
            let db = result.unwrap();
            let class = db.get_class("Car").unwrap();
            assert_eq!(class.type_param, "T");
        }

        #[test]
        fn should_handle_generic_with_relationships() {
            let result = parse(
                "classDiagram
class Car~T~
Driver -- Car : drives >",
            );
            assert!(result.is_ok());
        }

        #[test]
        fn should_handle_generic_class_with_brackets() {
            let result = parse(
                "classDiagram
class Dummy_Class~T~ {
String data
void methods()
}",
            );
            assert!(result.is_ok());
            let db = result.unwrap();
            let class = db.get_class("Dummy_Class").unwrap();
            assert_eq!(class.type_param, "T");
        }
    }

    mod css_styling {
        use super::*;

        #[test]
        fn should_handle_css_class_statement() {
            let result = parse(
                "classDiagram
class C1
cssClass \"C1\" styleClass",
            );
            assert!(result.is_ok());
            let db = result.unwrap();
            let class = db.get_class("C1").unwrap();
            assert!(class.css_classes.contains("styleClass"));
        }

        #[test]
        fn should_handle_multiple_classes_with_css() {
            let result = parse(
                "classDiagram
class C1
class C2
cssClass \"C1,C2\" styleClass",
            );
            assert!(result.is_ok());
            let db = result.unwrap();
            assert!(db
                .get_class("C1")
                .unwrap()
                .css_classes
                .contains("styleClass"));
            assert!(db
                .get_class("C2")
                .unwrap()
                .css_classes
                .contains("styleClass"));
        }
    }

    mod advanced_relationships {
        use super::*;

        #[test]
        fn should_handle_dashed_relations() {
            let result = parse(
                "classDiagram
Class11 <|.. Class12
Class13 <.. Class14
Class15 ..|> Class16
Class17 ..> Class18
Class19 .. Class20",
            );
            assert!(result.is_ok());
            let db = result.unwrap();
            assert_eq!(db.relations.len(), 5);
            // All should be dotted
            for rel in &db.relations {
                assert_eq!(rel.relation.line_type, LineType::Dotted);
            }
        }

        #[test]
        fn should_handle_no_types() {
            let result = parse("classDiagram\nClass1 -- Class02");
            assert!(result.is_ok());
            let db = result.unwrap();
            assert_eq!(db.relations[0].relation.type1, -1); // none
            assert_eq!(db.relations[0].relation.type2, -1); // none
            assert_eq!(db.relations[0].relation.line_type, LineType::Solid);
        }

        #[test]
        fn should_handle_type_only_on_right_side() {
            let result = parse("classDiagram\nClass1 --|> Class02");
            assert!(result.is_ok());
            let db = result.unwrap();
            assert_eq!(db.relations[0].relation.type1, -1); // none
            assert_eq!(
                db.relations[0].relation.type2,
                RelationType::Extension as i32
            );
        }

        #[test]
        fn should_handle_multiple_relations() {
            let result = parse(
                "classDiagram
Class1 <|-- Class02
Class03 *-- Class04
Class05 o-- Class06
Class07 .. Class08
Class09 -- Class10",
            );
            assert!(result.is_ok());
            let db = result.unwrap();
            assert_eq!(db.relations.len(), 5);
            // First: extension
            assert_eq!(
                db.relations[0].relation.type1,
                RelationType::Extension as i32
            );
            // Fourth: dotted line
            assert_eq!(db.relations[3].relation.line_type, LineType::Dotted);
        }

        #[test]
        fn should_handle_backticked_class_in_relation() {
            let result = parse(
                "classDiagram
`Class1` <|-- Class02",
            );
            assert!(result.is_ok());
            let db = result.unwrap();
            assert!(db.get_class("Class1").is_some());
        }
    }

    mod separators {
        use super::*;

        #[test]
        fn should_handle_separators() {
            let result = parse(
                "classDiagram
class Foo1 {
  You can use
  several lines
..
as you want
==
things together.
__
You can have as many groups
--
End of class
}",
            );
            assert!(result.is_ok());
        }

        #[test]
        fn should_handle_separator_with_text() {
            let result = parse(
                "classDiagram
class User {
.. Simple Getter ..
+ getName()
__ private data __
int age
}",
            );
            assert!(result.is_ok());
            let db = result.unwrap();
            let class = db.get_class("User").unwrap();
            assert_eq!(class.methods.len(), 1);
            assert_eq!(class.members.len(), 1);
        }
    }

    mod return_types {
        use super::*;

        #[test]
        fn should_handle_return_types() {
            let result = parse(
                "classDiagram
class Flight {
int flightNumber
datetime departureTime
getDepartureTime() datetime
}",
            );
            assert!(result.is_ok());
            let db = result.unwrap();
            let class = db.get_class("Flight").unwrap();
            assert_eq!(class.members.len(), 2);
            assert_eq!(class.methods.len(), 1);
        }

        #[test]
        fn should_handle_array_return_types() {
            let result = parse(
                "classDiagram
class Object
Object : getObjects() Object[]",
            );
            assert!(result.is_ok());
        }

        #[test]
        fn should_handle_generic_return_types() {
            let result = parse(
                "classDiagram
class Car
Car : +getWheels() List~Wheel~",
            );
            assert!(result.is_ok());
        }

        #[test]
        fn should_handle_generic_types_in_members() {
            let result = parse(
                "classDiagram
class Car {
List~Wheel~ wheels
setWheels(List~Wheel~ wheels)
+getWheels() List~Wheel~
}",
            );
            assert!(result.is_ok());
        }
    }

    mod abstract_static {
        use super::*;

        #[test]
        fn should_handle_abstract_methods() {
            let result = parse(
                "classDiagram
class Class1
Class1 : someMethod()*",
            );
            assert!(result.is_ok());
            let db = result.unwrap();
            let class = db.get_class("Class1").unwrap();
            assert_eq!(class.methods.len(), 1);
            let method = &class.methods[0];
            assert_eq!(method.get_display_details().display_text, "someMethod()");
            assert_eq!(method.get_display_details().css_style, "font-style:italic;");
        }

        #[test]
        fn should_handle_static_methods() {
            let result = parse(
                "classDiagram
class Class1
Class1 : someMethod()$",
            );
            assert!(result.is_ok());
            let db = result.unwrap();
            let class = db.get_class("Class1").unwrap();
            assert_eq!(class.methods.len(), 1);
            let method = &class.methods[0];
            assert_eq!(method.get_display_details().display_text, "someMethod()");
            assert_eq!(
                method.get_display_details().css_style,
                "text-decoration:underline;"
            );
        }
    }

    mod text_labels_advanced {
        use super::*;

        #[test]
        fn should_parse_classes_with_different_labels() {
            let result = parse(
                r#"classDiagram
class C1["OneWord"]
class C2["With, Comma"]
class C3["With (Brackets)"]
class C7["With 1 number"]
class C8["With . period..."]
class C9["With - dash"]
class C10["With _ underscore"]"#,
            );
            assert!(result.is_ok());
            let db = result.unwrap();
            assert_eq!(db.get_class("C1").unwrap().label, "OneWord");
            assert_eq!(db.get_class("C2").unwrap().label, "With, Comma");
            assert_eq!(db.get_class("C3").unwrap().label, "With (Brackets)");
            assert_eq!(db.get_class("C7").unwrap().label, "With 1 number");
            assert_eq!(db.get_class("C8").unwrap().label, "With . period...");
            assert_eq!(db.get_class("C9").unwrap().label, "With - dash");
            assert_eq!(db.get_class("C10").unwrap().label, "With _ underscore");
        }

        #[test]
        fn should_handle_text_label_with_member_and_annotation() {
            let result = parse(
                "classDiagram
class C1[\"Class 1 with text label\"]
<<interface>> C1
C1 : int member1",
            );
            assert!(result.is_ok());
            let db = result.unwrap();
            let c1 = db.get_class("C1").unwrap();
            assert_eq!(c1.label, "Class 1 with text label");
            assert_eq!(c1.members.len(), 1);
            assert_eq!(c1.annotations.len(), 1);
            assert_eq!(c1.annotations[0], "interface");
        }
    }

    mod namespace_advanced {
        use super::*;

        #[test]
        fn should_handle_generic_class_within_namespaces() {
            let result = parse(
                "classDiagram
namespace Company.Project.Module {
    class GenericClass~T~ {
        +addItem(item: T)
        +getItem() T
    }
}",
            );
            assert!(result.is_ok());
            let db = result.unwrap();
            let class = db.get_class("GenericClass").unwrap();
            assert_eq!(class.type_param, "T");
            assert_eq!(class.methods.len(), 2);
        }

        #[test]
        fn should_handle_namespace_with_generic_types() {
            let result = parse(
                "classDiagram
namespace space {
    class Square~Shape~{
        int id
        List~int~ position
        setPoints(List~int~ points)
        getPoints() List~int~
    }
}",
            );
            assert!(result.is_ok());
            let db = result.unwrap();
            let class = db.get_class("Square").unwrap();
            assert_eq!(class.type_param, "Shape");
        }

        #[test]
        fn should_add_relations_between_different_namespaces() {
            let result = parse(
                "classDiagram
A1 --> B1
namespace A {
  class A1 {
    +foo : string
  }
  class A2 {
    +bar : int
  }
}
namespace B {
  class B1 {
    +foo : bool
  }
  class B2 {
    +bar : float
  }
}
A2 --> B2",
            );
            assert!(result.is_ok());
            let db = result.unwrap();
            assert_eq!(db.relations.len(), 2);
            assert_eq!(db.relations[0].id1, "A1");
            assert_eq!(db.relations[0].id2, "B1");
            assert_eq!(db.relations[1].id1, "A2");
            assert_eq!(db.relations[1].id2, "B2");
        }
    }

    mod visibility_tests {
        use super::*;

        #[test]
        fn should_handle_all_visibility_modifiers() {
            let result = parse(
                "classDiagram
class actual
actual : -int privateMember
actual : +int publicMember
actual : #int protectedMember
actual : ~int privatePackage",
            );
            assert!(result.is_ok());
            let db = result.unwrap();
            let class = db.get_class("actual").unwrap();
            assert_eq!(class.members.len(), 4);
            assert_eq!(
                class.members[0].get_display_details().display_text,
                "-int privateMember"
            );
            assert_eq!(
                class.members[1].get_display_details().display_text,
                "+int publicMember"
            );
            assert_eq!(
                class.members[2].get_display_details().display_text,
                "#int protectedMember"
            );
            assert_eq!(
                class.members[3].get_display_details().display_text,
                "~int privatePackage"
            );
        }

        #[test]
        fn should_handle_method_visibility() {
            let result = parse(
                "classDiagram
class actual
actual : -privateMethod()
actual : +publicMethod()
actual : #protectedMethod()",
            );
            assert!(result.is_ok());
            let db = result.unwrap();
            let class = db.get_class("actual").unwrap();
            assert_eq!(class.methods.len(), 3);
        }
    }

    mod backtick_tests {
        use super::*;

        #[test]
        fn should_handle_newline_in_backticked_class_name() {
            // Mermaid allows newlines inside backticked class names
            let result = parse(
                "classDiagram
  Animal <|-- `Du
ck`
  class `Du
ck` {
    +swim()
  }",
            );
            assert!(result.is_ok(), "Parse failed: {:?}", result);
            let db = result.unwrap();
            // The class name contains an actual newline
            let class = db.get_class("Du\nck");
            assert!(class.is_some(), "Class 'Du\\nck' not found");
        }
    }

    mod callback_tests {
        use super::*;

        #[test]
        fn should_handle_callback() {
            let result = parse(
                "classDiagram
class Class1
callback Class1 \"functionCall\"",
            );
            assert!(result.is_ok());
            let db = result.unwrap();
            let class = db.get_class("Class1").unwrap();
            assert!(class.have_callback);
        }

        #[test]
        fn should_handle_click_call() {
            let result = parse(
                "classDiagram
class Class1
click Class1 call functionCall()",
            );
            assert!(result.is_ok());
            let db = result.unwrap();
            let class = db.get_class("Class1").unwrap();
            assert!(class.have_callback);
        }
    }
}
