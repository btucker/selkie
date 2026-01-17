//! Gantt diagram parser using pest grammar

use pest::Parser;
use pest_derive::Parser;

use super::types::GanttDb;

#[derive(Parser)]
#[grammar = "diagrams/gantt/gantt.pest"]
pub struct GanttParser;

/// Parse a Gantt diagram and return the populated database
pub fn parse(input: &str) -> Result<GanttDb, String> {
    let mut db = GanttDb::new();

    let pairs =
        GanttParser::parse(Rule::diagram, input).map_err(|e| format!("Parse error: {}", e))?;

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

fn process_document(db: &mut GanttDb, pair: pest::iterators::Pair<Rule>) -> Result<(), String> {
    for stmt in pair.into_inner() {
        process_statement(db, stmt)?;
    }
    Ok(())
}

fn process_statement(db: &mut GanttDb, pair: pest::iterators::Pair<Rule>) -> Result<(), String> {
    match pair.as_rule() {
        Rule::statement => {
            for inner in pair.into_inner() {
                process_statement(db, inner)?;
            }
        }
        Rule::comment_stmt => {
            // Ignore comments
        }
        Rule::date_format_stmt => {
            for inner in pair.into_inner() {
                if inner.as_rule() == Rule::format_text {
                    db.set_date_format(inner.as_str().trim());
                }
            }
        }
        Rule::axis_format_stmt => {
            for inner in pair.into_inner() {
                if inner.as_rule() == Rule::format_text {
                    db.set_axis_format(inner.as_str().trim());
                }
            }
        }
        Rule::tick_interval_stmt => {
            for inner in pair.into_inner() {
                if inner.as_rule() == Rule::format_text {
                    db.set_tick_interval(inner.as_str().trim());
                }
            }
        }
        Rule::inclusive_end_dates_stmt => {
            db.enable_inclusive_end_dates();
        }
        Rule::top_axis_stmt => {
            db.set_top_axis(true);
        }
        Rule::excludes_stmt => {
            for inner in pair.into_inner() {
                if inner.as_rule() == Rule::format_text {
                    db.set_excludes(inner.as_str().trim());
                }
            }
        }
        Rule::includes_stmt => {
            for inner in pair.into_inner() {
                if inner.as_rule() == Rule::format_text {
                    db.set_includes(inner.as_str().trim());
                }
            }
        }
        Rule::today_marker_stmt => {
            for inner in pair.into_inner() {
                if inner.as_rule() == Rule::format_text {
                    db.set_today_marker(inner.as_str().trim());
                }
            }
        }
        Rule::weekday_stmt => {
            for inner in pair.into_inner() {
                if inner.as_rule() == Rule::weekday_value {
                    db.set_weekday(inner.as_str().to_lowercase().as_str());
                }
            }
        }
        Rule::weekend_stmt => {
            for inner in pair.into_inner() {
                if inner.as_rule() == Rule::weekend_value {
                    db.set_weekend(inner.as_str().to_lowercase().as_str());
                }
            }
        }
        Rule::title_stmt => {
            for inner in pair.into_inner() {
                if inner.as_rule() == Rule::title_text {
                    db.set_diagram_title(inner.as_str().trim());
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
                if inner.as_rule() == Rule::section_text {
                    db.add_section(inner.as_str().trim());
                }
            }
        }
        Rule::click_stmt => {
            process_click_stmt(db, pair)?;
        }
        Rule::task_stmt => {
            process_task_stmt(db, pair)?;
        }
        _ => {}
    }
    Ok(())
}

fn process_acc_descr(db: &mut GanttDb, pair: pest::iterators::Pair<Rule>) -> Result<(), String> {
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
                        db.set_acc_description(content.as_str().trim());
                    }
                }
            }
            _ => {}
        }
    }
    Ok(())
}

fn process_click_stmt(db: &mut GanttDb, pair: pest::iterators::Pair<Rule>) -> Result<(), String> {
    let mut task_id = String::new();
    let mut href: Option<String> = None;
    let mut callback: Option<String> = None;
    let mut callback_args: Option<String> = None;

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::task_id => {
                task_id = inner.as_str().to_string();
            }
            Rule::click_action => {
                for action in inner.into_inner() {
                    match action.as_rule() {
                        Rule::click_href => {
                            for href_inner in action.into_inner() {
                                if href_inner.as_rule() == Rule::quoted_string {
                                    // Remove quotes
                                    let s = href_inner.as_str();
                                    href = Some(s[1..s.len() - 1].to_string());
                                }
                            }
                        }
                        Rule::click_call => {
                            for call_inner in action.into_inner() {
                                match call_inner.as_rule() {
                                    Rule::callback_name => {
                                        callback = Some(call_inner.as_str().to_string());
                                    }
                                    Rule::callback_args => {
                                        for args in call_inner.into_inner() {
                                            if args.as_rule() == Rule::args_content {
                                                callback_args = Some(args.as_str().to_string());
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

    if let Some(h) = href {
        db.set_link(&task_id, &h);
    }
    if let Some(cb) = callback {
        db.set_click_event(&task_id, &cb, callback_args.as_deref());
    }

    Ok(())
}

fn process_task_stmt(db: &mut GanttDb, pair: pest::iterators::Pair<Rule>) -> Result<(), String> {
    let mut task_name = String::new();
    let mut task_data = String::new();

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::task_name => {
                task_name = inner.as_str().trim().to_string();
            }
            Rule::task_data => {
                task_data = inner.as_str().trim().to_string();
            }
            _ => {}
        }
    }

    db.add_task(&task_name, &task_data);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    mod basic_parsing {
        use super::*;

        #[test]
        fn should_parse_empty_diagram() {
            let result = parse("gantt\n");
            assert!(result.is_ok());
        }

        #[test]
        fn should_parse_diagram_with_date_format() {
            let result = parse("gantt\ndateFormat YYYY-MM-DD");
            assert!(result.is_ok());
            let db = result.unwrap();
            assert_eq!(db.get_date_format(), "YYYY-MM-DD");
        }

        #[test]
        fn should_parse_diagram_with_axis_format() {
            let result = parse("gantt\naxisFormat %Y-%m-%d");
            assert!(result.is_ok());
            let db = result.unwrap();
            assert_eq!(db.get_axis_format(), "%Y-%m-%d");
        }

        #[test]
        fn should_parse_inclusive_end_dates() {
            let result = parse("gantt\ninclusiveEndDates");
            assert!(result.is_ok());
            let db = result.unwrap();
            assert!(db.end_dates_are_inclusive());
        }

        #[test]
        fn should_parse_top_axis() {
            let result = parse("gantt\ntopAxis");
            assert!(result.is_ok());
        }

        #[test]
        fn should_parse_excludes() {
            let result = parse("gantt\nexcludes weekends 2019-02-06,friday");
            assert!(result.is_ok());
        }

        #[test]
        fn should_parse_includes() {
            let result = parse("gantt\nincludes 2019-02-06");
            assert!(result.is_ok());
        }

        #[test]
        fn should_parse_today_marker() {
            let result = parse("gantt\ntodayMarker off");
            assert!(result.is_ok());
            let db = result.unwrap();
            assert_eq!(db.get_today_marker(), "off");
        }
    }

    mod weekday_weekend {
        use super::*;
        use crate::diagrams::gantt::WeekendStart;

        #[test]
        fn should_parse_weekday_monday() {
            let result = parse("gantt\nweekday monday");
            assert!(result.is_ok());
        }

        #[test]
        fn should_parse_weekend_friday() {
            let result = parse("gantt\nweekend friday");
            assert!(result.is_ok());
            let db = result.unwrap();
            assert_eq!(db.get_weekend(), WeekendStart::Friday);
        }

        #[test]
        fn should_parse_weekend_saturday() {
            let result = parse("gantt\nweekend saturday");
            assert!(result.is_ok());
            let db = result.unwrap();
            assert_eq!(db.get_weekend(), WeekendStart::Saturday);
        }
    }

    mod accessibility {
        use super::*;

        #[test]
        fn should_parse_title() {
            let result = parse("gantt\ntitle My Gantt Chart");
            assert!(result.is_ok());
            let db = result.unwrap();
            assert_eq!(db.get_diagram_title(), "My Gantt Chart");
        }

        #[test]
        fn should_parse_acc_title() {
            let result = parse("gantt\naccTitle: Accessibility Title");
            assert!(result.is_ok());
            let db = result.unwrap();
            assert_eq!(db.get_acc_title(), "Accessibility Title");
        }

        #[test]
        fn should_parse_acc_descr() {
            let result = parse("gantt\naccDescr: Accessibility Description");
            assert!(result.is_ok());
            let db = result.unwrap();
            assert_eq!(db.get_acc_description(), "Accessibility Description");
        }

        #[test]
        fn should_parse_multiline_acc_descr() {
            let result = parse("gantt\naccDescr {\nLine 1\nLine 2\n}");
            assert!(result.is_ok());
            let db = result.unwrap();
            assert!(db.get_acc_description().contains("Line 1"));
        }
    }

    mod sections {
        use super::*;

        #[test]
        fn should_parse_section() {
            let result = parse("gantt\nsection Planning");
            assert!(result.is_ok());
            let db = result.unwrap();
            assert_eq!(db.get_sections(), vec!["Planning"]);
        }

        #[test]
        fn should_parse_multiple_sections() {
            let result = parse("gantt\nsection Planning\nsection Development\nsection Testing");
            assert!(result.is_ok());
            let db = result.unwrap();
            assert_eq!(
                db.get_sections(),
                vec!["Planning", "Development", "Testing"]
            );
        }
    }

    mod tasks {
        use super::*;

        #[test]
        fn should_parse_simple_task() {
            let result =
                parse("gantt\ndateFormat YYYY-MM-DD\nsection Test\nTask 1: 2023-01-01, 2023-01-10");
            assert!(result.is_ok());
            let mut db = result.unwrap();
            let tasks = db.get_tasks();
            assert_eq!(tasks.len(), 1);
            assert_eq!(tasks[0].task, "Task 1");
        }

        #[test]
        fn should_parse_task_with_id() {
            let result = parse(
                "gantt\ndateFormat YYYY-MM-DD\nsection Test\nTask 1: task1, 2023-01-01, 2023-01-10",
            );
            assert!(result.is_ok());
            let mut db = result.unwrap();
            let tasks = db.get_tasks();
            assert_eq!(tasks.len(), 1);
            assert_eq!(tasks[0].id, "task1");
        }

        #[test]
        fn should_parse_task_with_duration() {
            let result =
                parse("gantt\ndateFormat YYYY-MM-DD\nsection Test\nTask 1: task1, 2023-01-01, 5d");
            assert!(result.is_ok());
            let mut db = result.unwrap();
            let tasks = db.get_tasks();
            assert_eq!(tasks.len(), 1);
        }

        #[test]
        fn should_parse_task_with_after_dependency() {
            let result = parse("gantt\ndateFormat YYYY-MM-DD\nsection Test\nTask 1: task1, 2023-01-01, 5d\nTask 2: task2, after task1, 3d");
            assert!(result.is_ok());
            let mut db = result.unwrap();
            let tasks = db.get_tasks();
            assert_eq!(tasks.len(), 2);
        }

        #[test]
        fn should_parse_critical_task() {
            let result = parse(
                "gantt\ndateFormat YYYY-MM-DD\nsection Test\nTask 1: crit, task1, 2023-01-01, 5d",
            );
            assert!(result.is_ok());
            let mut db = result.unwrap();
            let tasks = db.get_tasks();
            assert!(tasks[0].flags.critical);
        }

        #[test]
        fn should_parse_active_task() {
            let result = parse(
                "gantt\ndateFormat YYYY-MM-DD\nsection Test\nTask 1: active, task1, 2023-01-01, 5d",
            );
            assert!(result.is_ok());
            let mut db = result.unwrap();
            let tasks = db.get_tasks();
            assert!(tasks[0].flags.active);
        }

        #[test]
        fn should_parse_done_task() {
            let result = parse(
                "gantt\ndateFormat YYYY-MM-DD\nsection Test\nTask 1: done, task1, 2023-01-01, 5d",
            );
            assert!(result.is_ok());
            let mut db = result.unwrap();
            let tasks = db.get_tasks();
            assert!(tasks[0].flags.done);
        }

        #[test]
        fn should_parse_milestone() {
            let result = parse("gantt\ndateFormat YYYY-MM-DD\nsection Test\nMilestone 1: milestone, m1, 2023-01-15, 0d");
            assert!(result.is_ok());
            let mut db = result.unwrap();
            let tasks = db.get_tasks();
            assert!(tasks[0].flags.milestone);
        }
    }

    mod click {
        use super::*;

        #[test]
        fn should_parse_click_with_href() {
            let result = parse("gantt\nsection Test\nTask 1: task1, 2023-01-01, 5d\nclick task1 href \"http://example.com\"");
            assert!(result.is_ok());
        }

        #[test]
        fn should_parse_click_with_call() {
            let result = parse(
                "gantt\nsection Test\nTask 1: task1, 2023-01-01, 5d\nclick task1 call myFunction()",
            );
            assert!(result.is_ok());
        }

        #[test]
        fn should_parse_click_with_call_and_args() {
            let result = parse("gantt\nsection Test\nTask 1: task1, 2023-01-01, 5d\nclick task1 call myFunction(arg1, arg2)");
            assert!(result.is_ok());
        }
    }

    mod comments {
        use super::*;

        #[test]
        fn should_handle_comments() {
            let result = parse("gantt\n%% This is a comment\nsection Test");
            assert!(result.is_ok());
        }

        #[test]
        fn should_handle_comments_before_diagram() {
            let result = parse("%% Comment before\ngantt\nsection Test");
            assert!(result.is_ok());
        }
    }

    mod complex_diagrams {
        use super::*;

        #[test]
        fn should_parse_full_diagram() {
            let input = r#"gantt
    dateFormat YYYY-MM-DD
    title My Project Schedule
    excludes weekends

    section Planning
    Requirements: done, req, 2023-01-01, 10d
    Design: active, design, after req, 5d

    section Development
    Implementation: crit, impl, after design, 20d
    Testing: test, after impl, 10d

    section Deployment
    Release: milestone, release, after test, 0d"#;

            let result = parse(input);
            assert!(result.is_ok());
            let mut db = result.unwrap();

            assert_eq!(db.get_date_format(), "YYYY-MM-DD");
            assert_eq!(db.get_diagram_title(), "My Project Schedule");
            assert_eq!(db.get_sections().len(), 3);
            assert_eq!(db.get_tasks().len(), 5);
        }
    }

    // Tests ported from mermaid Cypress tests (gantt.spec.js)
    mod cypress_tests {
        use super::*;

        #[test]
        fn test_cypress_basic_gantt() {
            // From Cypress: should render a gantt chart
            let input = r#"gantt
      dateFormat  YYYY-MM-DD
      axisFormat  %d/%m
      title Adding GANTT diagram to mermaid
      excludes weekdays 2014-01-10

      section A section
      Completed task            :done,    des1, 2014-01-06,2014-01-08
      Active task               :active,  des2, 2014-01-09, 3d
      Future task               :         des3, after des2, 5d
      Future task2               :         des4, after des3, 5d"#;
            let result = parse(input);
            assert!(result.is_ok(), "Failed to parse: {:?}", result);
            let mut db = result.unwrap();
            assert!(!db.get_tasks().is_empty());
        }

        #[test]
        fn test_cypress_multiline_section_titles() {
            // From Cypress: Handle multiline section titles with different line breaks
            let input = r#"gantt
      dateFormat  YYYY-MM-DD
      axisFormat  %d/%m
      title       GANTT diagram with multiline section titles

      section A section<br>multiline
      Completed task            : done,    des1, 2014-01-06,2014-01-08

      section Critical tasks<br/>multiline
      Completed task : crit, done, 2014-01-06, 24h"#;
            let result = parse(input);
            assert!(result.is_ok(), "Failed to parse: {:?}", result);
        }

        #[test]
        fn test_cypress_multiple_dependencies() {
            // From Cypress: Multiple dependencies syntax
            let input = r#"gantt
      dateFormat  YYYY-MM-DD
      axisFormat  %d/%m
      title Adding GANTT diagram to mermaid

      apple :a, 2017-07-20, 1w
      banana :crit, b, 2017-07-23, 1d
      cherry :active, c, after b a, 1d"#;
            let result = parse(input);
            assert!(result.is_ok(), "Failed to parse: {:?}", result);
        }

        #[test]
        fn test_cypress_excludes() {
            // From Cypress: excludes specification
            let input = r#"gantt
      dateFormat  YYYY-MM-DD
      excludes weekends
      title Projects Timeline

      section asdf
          specs :done, ps, 2019-05-10, 50d"#;
            let result = parse(input);
            assert!(result.is_ok(), "Failed to parse: {:?}", result);
        }

        #[test]
        fn test_cypress_tick_interval() {
            // From Cypress: tickInterval with week unit
            let input = r#"gantt
    tickInterval 1week
    dateFormat  YYYY-MM-DD
    section Section
    A task : a1, 2024-01-01, 30d"#;
            let result = parse(input);
            assert!(result.is_ok(), "Failed to parse: {:?}", result);
        }

        #[test]
        fn test_cypress_milestones() {
            // From Cypress: milestones
            let input = r#"gantt
      dateFormat  YYYY-MM-DD
      section Release
      Release : milestone, m1, 2024-01-15, 0d"#;
            let result = parse(input);
            assert!(result.is_ok(), "Failed to parse: {:?}", result);
        }

        #[test]
        fn test_cypress_today_marker() {
            // From Cypress: todayMarker setting
            let input = r#"gantt
      dateFormat  YYYY-MM-DD
      todayMarker off
      section Tasks
      Task1 : t1, 2024-01-01, 5d"#;
            let result = parse(input);
            assert!(result.is_ok(), "Failed to parse: {:?}", result);
        }

        #[test]
        fn test_cypress_click_callback() {
            // From Cypress: click callback
            let input = r#"gantt
        dateFormat  YYYY-MM-DD
        section Clickable
        Completed task  :done, cl1, 2014-01-06,2014-01-08
        Active task     :active, cl2, 2014-01-09, 3d
        click cl1 call callback()"#;
            let result = parse(input);
            assert!(result.is_ok(), "Failed to parse: {:?}", result);
        }
    }
}
