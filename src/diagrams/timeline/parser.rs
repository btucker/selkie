//! Timeline diagram parser using pest grammar

use pest::Parser;
use pest_derive::Parser;

use super::types::TimelineDb;

#[derive(Parser)]
#[grammar = "diagrams/timeline/timeline.pest"]
pub struct TimelineParser;

/// Parse a timeline diagram and return the populated database
pub fn parse(input: &str) -> Result<TimelineDb, String> {
    let mut db = TimelineDb::new();

    let pairs =
        TimelineParser::parse(Rule::diagram, input).map_err(|e| format!("Parse error: {}", e))?;

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

fn process_document(db: &mut TimelineDb, pair: pest::iterators::Pair<Rule>) -> Result<(), String> {
    for stmt in pair.into_inner() {
        process_statement(db, stmt)?;
    }
    Ok(())
}

fn process_statement(db: &mut TimelineDb, pair: pest::iterators::Pair<Rule>) -> Result<(), String> {
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
                    db.set_title(inner.as_str().trim());
                }
            }
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
        Rule::event_continuation => {
            // Add events to the most recent task
            for inner in pair.into_inner() {
                if inner.as_rule() == Rule::events {
                    let events_str = inner.as_str();
                    db.add_event(events_str);
                }
            }
        }
        _ => {}
    }
    Ok(())
}

fn process_task(db: &mut TimelineDb, pair: pest::iterators::Pair<Rule>) -> Result<(), String> {
    let mut task_name = String::new();
    let mut events_str = String::new();

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::task_name => {
                task_name = inner.as_str().trim().to_string();
            }
            Rule::events => {
                events_str = inner.as_str().to_string();
            }
            _ => {}
        }
    }

    if !task_name.is_empty() {
        // Build task string with events
        let task_str = if events_str.is_empty() {
            task_name
        } else {
            format!("{}: {}", task_name, events_str)
        };
        db.add_task(&task_str, &[]);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    mod basic_parsing {
        use super::*;

        #[test]
        fn should_handle_simple_section_definition() {
            let result = parse("timeline\n    section abc-123");
            assert!(result.is_ok(), "Parse error: {:?}", result.err());
            let db = result.unwrap();
            assert_eq!(db.get_sections(), &["abc-123"]);
        }

        #[test]
        fn should_handle_section_and_two_tasks() {
            let result = parse("timeline\n    section abc-123\n    task1\n    task2");
            assert!(result.is_ok(), "Parse error: {:?}", result.err());
            let db = result.unwrap();

            let tasks = db.get_tasks();
            assert_eq!(tasks.len(), 2);
            for task in tasks {
                assert_eq!(task.section, "abc-123");
                assert!(task.task == "task1" || task.task == "task2");
            }
        }

        #[test]
        fn should_handle_two_sections_and_tasks() {
            let result = parse("timeline\n    section abc-123\n    task1\n    task2\n    section abc-456\n    task3\n    task4");
            assert!(result.is_ok(), "Parse error: {:?}", result.err());
            let db = result.unwrap();

            assert_eq!(db.get_sections(), &["abc-123", "abc-456"]);

            let tasks = db.get_tasks();
            assert_eq!(tasks.len(), 4);

            for task in tasks {
                if task.section == "abc-123" {
                    assert!(task.task == "task1" || task.task == "task2");
                } else {
                    assert_eq!(task.section, "abc-456");
                    assert!(task.task == "task3" || task.task == "task4");
                }
            }
        }
    }

    mod event_parsing {
        use super::*;

        #[test]
        fn should_handle_task_with_events() {
            let input =
                "timeline\n    section abc-123\n      task1: event1\n      task2: event2: event3";
            let result = parse(input);
            assert!(result.is_ok(), "Parse error: {:?}", result.err());
            let db = result.unwrap();

            assert_eq!(db.get_sections()[0], "abc-123");

            let tasks = db.get_tasks();
            for task in tasks {
                match task.task.trim() {
                    "task1" => assert_eq!(task.events, vec!["event1"]),
                    "task2" => assert_eq!(task.events, vec!["event2", "event3"]),
                    _ => {}
                }
            }
        }

        #[test]
        fn should_handle_markdown_link_in_event() {
            let input = "timeline\n    section abc-123\n      task1: [event1](http://example.com)\n      task2: event2: event3";
            let result = parse(input);
            assert!(result.is_ok(), "Parse error: {:?}", result.err());
            let db = result.unwrap();

            let tasks = db.get_tasks();
            for task in tasks {
                match task.task.trim() {
                    "task1" => assert_eq!(task.events, vec!["[event1](http://example.com)"]),
                    "task2" => assert_eq!(task.events, vec!["event2", "event3"]),
                    _ => {}
                }
            }
        }

        #[test]
        fn should_handle_multiline_events() {
            let input = "timeline\n    section abc-123\n      task1: event1\n      task2: event2: event3\n           : event4: event5";
            let result = parse(input);
            assert!(result.is_ok(), "Parse error: {:?}", result.err());
            let db = result.unwrap();

            let tasks = db.get_tasks();
            for task in tasks {
                match task.task.trim() {
                    "task1" => assert_eq!(task.events, vec!["event1"]),
                    "task2" => {
                        assert_eq!(task.events, vec!["event2", "event3", "event4", "event5"])
                    }
                    _ => {}
                }
            }
        }
    }

    mod special_characters {
        use super::*;

        #[test]
        fn should_handle_semicolons_in_title() {
            let input = "timeline\n      title ;my;title;\n      section ;a;bc-123;\n      ;ta;sk1;: ;ev;ent1; : ;ev;ent2; : ;ev;ent3;";
            let result = parse(input);
            assert!(result.is_ok(), "Parse error: {:?}", result.err());
            let db = result.unwrap();

            assert_eq!(db.get_title(), ";my;title;");
            assert_eq!(db.get_sections(), &[";a;bc-123;"]);
        }

        #[test]
        fn should_handle_hashtags_in_content() {
            let input = "timeline\n      title #my#title#\n      section #a#bc-123#\n      task1: #ev#ent1# : #ev#ent2# : #ev#ent3#";
            let result = parse(input);
            assert!(result.is_ok(), "Parse error: {:?}", result.err());
            let db = result.unwrap();

            assert_eq!(db.get_title(), "#my#title#");
            assert_eq!(db.get_sections(), &["#a#bc-123#"]);

            let tasks = db.get_tasks();
            assert_eq!(tasks[0].task, "task1");
            // Events contain hashtags
            assert_eq!(tasks[0].events.len(), 3);
        }
    }

    mod comments {
        use super::*;

        #[test]
        fn should_handle_comments() {
            let result = parse("timeline\n%% This is a comment\n    section Test");
            assert!(result.is_ok());
            let db = result.unwrap();
            assert_eq!(db.get_sections(), &["Test"]);
        }

        #[test]
        fn should_handle_comments_before_timeline() {
            let result = parse("%% Comment\ntimeline\nsection Test");
            assert!(result.is_ok());
        }
    }

    mod complex_diagrams {
        use super::*;

        #[test]
        fn should_parse_full_timeline() {
            let input = r#"timeline
    title History of Social Media Platform
    2002 : LinkedIn
    2004 : Facebook : Google
    2005 : YouTube
    2006 : Twitter"#;

            let result = parse(input);
            assert!(result.is_ok(), "Parse error: {:?}", result.err());
            let db = result.unwrap();

            assert_eq!(db.get_title(), "History of Social Media Platform");

            let tasks = db.get_tasks();
            assert_eq!(tasks.len(), 4);
        }
    }
}
