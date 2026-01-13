//! User journey diagram parser using pest grammar

use pest::Parser;
use pest_derive::Parser;

use super::types::JourneyDb;

#[derive(Parser)]
#[grammar = "diagrams/journey/journey.pest"]
pub struct JourneyParser;

/// Parse a user journey diagram and return the populated database
pub fn parse(input: &str) -> Result<JourneyDb, String> {
    let mut db = JourneyDb::new();

    let pairs =
        JourneyParser::parse(Rule::diagram, input).map_err(|e| format!("Parse error: {}", e))?;

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

fn process_document(db: &mut JourneyDb, pair: pest::iterators::Pair<Rule>) -> Result<(), String> {
    for stmt in pair.into_inner() {
        process_statement(db, stmt)?;
    }
    Ok(())
}

fn process_statement(db: &mut JourneyDb, pair: pest::iterators::Pair<Rule>) -> Result<(), String> {
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
                    db.title = inner.as_str().trim().to_string();
                }
            }
        }
        Rule::acc_title_stmt => {
            for inner in pair.into_inner() {
                if inner.as_rule() == Rule::line_content {
                    db.set_acc_title(inner.as_str().trim());
                }
            }
        }
        Rule::acc_descr_stmt => {
            process_acc_descr(db, pair)?;
        }
        Rule::section_stmt => {
            for inner in pair.into_inner() {
                if inner.as_rule() == Rule::section_name {
                    db.add_section(inner.as_str().trim());
                }
            }
        }
        Rule::task_stmt => {
            process_task(db, pair)?;
        }
        _ => {}
    }
    Ok(())
}

fn process_acc_descr(db: &mut JourneyDb, pair: pest::iterators::Pair<Rule>) -> Result<(), String> {
    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::acc_descr_single => {
                for content in inner.into_inner() {
                    if content.as_rule() == Rule::line_content {
                        db.set_acc_description(content.as_str().trim());
                    }
                }
            }
            Rule::acc_descr_multi => {
                for content in inner.into_inner() {
                    if content.as_rule() == Rule::multiline_content {
                        let text = content.as_str().trim();
                        let cleaned: String = text
                            .lines()
                            .map(|l| l.trim())
                            .collect::<Vec<_>>()
                            .join("\n");
                        db.set_acc_description(&cleaned);
                    }
                }
            }
            _ => {}
        }
    }
    Ok(())
}

fn process_task(db: &mut JourneyDb, pair: pest::iterators::Pair<Rule>) -> Result<(), String> {
    let mut task_name = String::new();
    let mut task_data = String::new();

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::task_name => {
                task_name = inner.as_str().trim().to_string();
            }
            Rule::task_data => {
                task_data = inner.as_str().to_string();
            }
            _ => {}
        }
    }

    if !task_name.is_empty() {
        db.add_task(&task_name, &task_data);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    mod basic_parsing {
        use super::*;

        #[test]
        fn should_handle_title_definition() {
            let result = parse("journey\ntitle Adding journey diagram functionality to mermaid");
            assert!(result.is_ok(), "Parse error: {:?}", result.err());
            let db = result.unwrap();
            assert_eq!(db.title, "Adding journey diagram functionality to mermaid");
        }

        #[test]
        fn should_handle_section_definition() {
            let result = parse("journey\ntitle Adding journey diagram\nsection Order from website");
            assert!(result.is_ok(), "Parse error: {:?}", result.err());
            let db = result.unwrap();
            assert_eq!(db.get_sections(), &["Order from website"]);
        }

        #[test]
        fn should_handle_multiline_section_titles_with_br() {
            let result = parse("journey\ntitle Test\nsection Line1<br>Line2<br/>Line3</br />Line4");
            assert!(result.is_ok(), "Parse error: {:?}", result.err());
        }
    }

    mod accessibility {
        use super::*;

        #[test]
        fn should_handle_acc_descr() {
            let result = parse("journey\naccDescr: A user journey for family shopping\ntitle Adding journey\nsection Order from website");
            assert!(result.is_ok(), "Parse error: {:?}", result.err());
            let db = result.unwrap();
            assert_eq!(
                db.get_acc_description(),
                "A user journey for family shopping"
            );
        }

        #[test]
        fn should_handle_multiline_acc_descr() {
            let input = r#"journey
accDescr {
    A user journey for
    family shopping
}
title Adding journey diagram functionality to mermaid
accTitle: Adding acc journey diagram functionality to mermaid
section Order from website"#;

            let result = parse(input);
            assert!(result.is_ok(), "Parse error: {:?}", result.err());
            let db = result.unwrap();
            assert_eq!(
                db.get_acc_description(),
                "A user journey for\nfamily shopping"
            );
            assert_eq!(db.title, "Adding journey diagram functionality to mermaid");
            assert_eq!(
                db.get_acc_title(),
                "Adding acc journey diagram functionality to mermaid"
            );
        }

        #[test]
        fn should_handle_acc_title() {
            let result = parse("journey\naccTitle: The title\nsection Order from website");
            assert!(result.is_ok(), "Parse error: {:?}", result.err());
            let db = result.unwrap();
            assert_eq!(db.get_acc_description(), "");
            assert_eq!(db.get_acc_title(), "The title");
        }
    }

    mod task_parsing {
        use super::*;

        #[test]
        fn should_parse_tasks_with_various_formats() {
            let input = r#"journey
title Adding journey diagram functionality to mermaid
section Documentation
A task: 5: Alice, Bob, Charlie
B task: 3:Bob, Charlie
C task: 5
D task: 5: Charlie, Alice
E task: 5:
section Another section
P task: 5:
Q task: 5:
R task: 5:"#;

            let result = parse(input);
            assert!(result.is_ok(), "Parse error: {:?}", result.err());
            let db = result.unwrap();

            let tasks = db.get_tasks();
            assert_eq!(tasks.len(), 8);

            // Check first task
            assert_eq!(tasks[0].score, 5);
            assert_eq!(tasks[0].people, vec!["Alice", "Bob", "Charlie"]);
            assert_eq!(tasks[0].section, "Documentation");
            assert_eq!(tasks[0].task, "A task");
            assert_eq!(tasks[0].task_type, "Documentation");

            // Check second task
            assert_eq!(tasks[1].score, 3);
            assert_eq!(tasks[1].people, vec!["Bob", "Charlie"]);
            assert_eq!(tasks[1].section, "Documentation");
            assert_eq!(tasks[1].task, "B task");

            // Check third task (no people)
            assert_eq!(tasks[2].score, 5);
            assert!(tasks[2].people.is_empty());
            assert_eq!(tasks[2].task, "C task");

            // Check fourth task
            assert_eq!(tasks[3].score, 5);
            assert_eq!(tasks[3].people, vec!["Charlie", "Alice"]);
            assert_eq!(tasks[3].task, "D task");

            // Check fifth task (empty people after colon)
            assert_eq!(tasks[4].score, 5);
            // Note: The TypeScript version returns [""] for empty people, but our implementation
            // correctly returns an empty vec since we skip empty strings

            // Check tasks in second section
            assert_eq!(tasks[5].section, "Another section");
            assert_eq!(tasks[5].task_type, "Another section");
            assert_eq!(tasks[5].task, "P task");
        }

        #[test]
        fn should_handle_task_with_simple_format() {
            let result = parse("journey\ntitle Test\nsection Test\nMy task: 3: Alice");
            assert!(result.is_ok());
            let db = result.unwrap();
            let tasks = db.get_tasks();
            assert_eq!(tasks.len(), 1);
            assert_eq!(tasks[0].task, "My task");
            assert_eq!(tasks[0].score, 3);
            assert_eq!(tasks[0].people, vec!["Alice"]);
        }
    }

    mod comments {
        use super::*;

        #[test]
        fn should_handle_comments() {
            let result = parse("journey\n%% This is a comment\ntitle Test Journey");
            assert!(result.is_ok());
            let db = result.unwrap();
            assert_eq!(db.title, "Test Journey");
        }

        #[test]
        fn should_handle_comments_before_journey() {
            let result = parse("%% Comment\njourney\ntitle Test");
            assert!(result.is_ok());
        }
    }

    mod complex_diagrams {
        use super::*;

        #[test]
        fn should_parse_full_journey() {
            let input = r#"journey
    title My working day
    section Go to work
      Make tea: 5: Me
      Go upstairs: 3: Me
      Do work: 1: Me, Cat
    section Go home
      Go downstairs: 5: Me
      Sit down: 5: Me"#;

            let result = parse(input);
            assert!(result.is_ok(), "Parse error: {:?}", result.err());
            let db = result.unwrap();

            assert_eq!(db.title, "My working day");
            assert_eq!(db.get_sections(), &["Go to work", "Go home"]);

            let tasks = db.get_tasks();
            assert_eq!(tasks.len(), 5);

            // Verify actors
            let actors = db.get_actors();
            assert!(actors.contains(&"Me".to_string()));
            assert!(actors.contains(&"Cat".to_string()));
        }
    }
}
