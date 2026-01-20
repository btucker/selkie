//! State diagram parser using pest grammar

use pest::Parser;
use pest_derive::Parser;

use super::types::{Direction, NotePosition, StateDb, StateType};

#[derive(Parser)]
#[grammar = "diagrams/state/state.pest"]
pub struct StateParser;

/// Parse a state diagram and return the populated database
pub fn parse(input: &str) -> Result<StateDb, String> {
    let mut db = StateDb::new();

    let pairs =
        StateParser::parse(Rule::diagram, input).map_err(|e| format!("Parse error: {}", e))?;

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
    db: &mut StateDb,
    pair: pest::iterators::Pair<Rule>,
    parent: Option<&str>,
) -> Result<(), String> {
    for stmt in pair.into_inner() {
        process_statement(db, stmt, parent)?;
    }
    Ok(())
}

fn process_statement(
    db: &mut StateDb,
    pair: pest::iterators::Pair<Rule>,
    parent: Option<&str>,
) -> Result<(), String> {
    match pair.as_rule() {
        Rule::statement => {
            for inner in pair.into_inner() {
                process_statement(db, inner, parent)?;
            }
        }
        Rule::comment_stmt => {
            // Ignore comments
        }
        Rule::direction_stmt => {
            for inner in pair.into_inner() {
                if inner.as_rule() == Rule::direction {
                    let dir = Direction::from_str(inner.as_str());
                    db.set_direction(dir);
                }
            }
        }
        Rule::hide_empty_stmt => {
            db.set_hide_empty_descriptions(true);
        }
        Rule::scale_stmt => {
            // Scale is mostly for rendering
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
        Rule::class_def_stmt => {
            process_class_def(db, pair)?;
        }
        Rule::class_stmt => {
            process_class_assignment(db, pair)?;
        }
        Rule::style_stmt => {
            // Style statements - for future use
        }
        Rule::state_declaration => {
            process_state_declaration(db, pair, parent)?;
        }
        Rule::note_stmt => {
            process_note(db, pair)?;
        }
        Rule::transition_stmt => {
            process_transition(db, pair, parent)?;
        }
        Rule::state_with_description => {
            process_state_with_description(db, pair)?;
        }
        _ => {}
    }
    Ok(())
}

fn process_acc_descr(db: &mut StateDb, pair: pest::iterators::Pair<Rule>) -> Result<(), String> {
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
                        // Clean up multiline content - trim each line and join with newlines
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

fn process_class_def(db: &mut StateDb, pair: pest::iterators::Pair<Rule>) -> Result<(), String> {
    let mut class_name = String::new();
    let mut styles = String::new();

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::class_name => {
                class_name = inner.as_str().to_string();
            }
            Rule::style_opts => {
                styles = inner.as_str().to_string();
            }
            _ => {}
        }
    }

    db.add_style_class(&class_name, &styles);
    Ok(())
}

fn process_class_assignment(
    db: &mut StateDb,
    pair: pest::iterators::Pair<Rule>,
) -> Result<(), String> {
    let mut state_ids: Vec<String> = Vec::new();
    let mut class_name = String::new();

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::state_id_list => {
                for id in inner.into_inner() {
                    if id.as_rule() == Rule::state_id {
                        state_ids.push(id.as_str().to_string());
                    }
                }
            }
            Rule::class_name => {
                class_name = inner.as_str().to_string();
            }
            _ => {}
        }
    }

    for state_id in state_ids {
        db.apply_class(&state_id, &class_name);
    }
    Ok(())
}

fn process_state_declaration(
    db: &mut StateDb,
    pair: pest::iterators::Pair<Rule>,
    parent: Option<&str>,
) -> Result<(), String> {
    for inner in pair.into_inner() {
        if inner.as_rule() == Rule::state_def {
            process_state_def(db, inner, parent)?;
        }
    }
    Ok(())
}

fn process_state_def(
    db: &mut StateDb,
    pair: pest::iterators::Pair<Rule>,
    parent: Option<&str>,
) -> Result<(), String> {
    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::state_with_body => {
                let mut state_id = String::new();

                for body_inner in inner.into_inner() {
                    match body_inner.as_rule() {
                        Rule::state_id => {
                            state_id = body_inner.as_str().to_string();
                            db.add_state(&state_id);
                            // ALWAYS set parent for explicit state blocks (even if None)
                            // This ensures explicit `state X { }` declarations take precedence
                            // over implicit state creation from transitions
                            db.set_or_clear_parent(&state_id, parent);
                        }
                        Rule::quoted_string => {
                            let s = body_inner.as_str();
                            state_id = s[1..s.len() - 1].to_string(); // Remove quotes
                            db.add_state(&state_id);
                            // ALWAYS set parent for explicit state blocks (even if None)
                            db.set_or_clear_parent(&state_id, parent);
                        }
                        Rule::document => {
                            // Process nested document with this state as parent
                            process_document(db, body_inner, Some(&state_id))?;
                        }
                        _ => {}
                    }
                }
            }
            Rule::state_special => {
                let mut state_id = String::new();
                let mut special_type = StateType::Default;

                for spec_inner in inner.into_inner() {
                    match spec_inner.as_rule() {
                        Rule::state_id => {
                            state_id = spec_inner.as_str().to_string();
                        }
                        Rule::special_type => {
                            special_type = match spec_inner.as_str().to_lowercase().as_str() {
                                "fork" => StateType::Fork,
                                "join" => StateType::Join,
                                "choice" => StateType::Choice,
                                _ => StateType::Default,
                            };
                        }
                        _ => {}
                    }
                }

                db.add_state_with_type(&state_id, special_type);
                if let Some(p) = parent {
                    db.set_parent(&state_id, p);
                }
            }
            Rule::state_alias => {
                let mut state_id = String::new();
                let mut alias = String::new();

                for alias_inner in inner.into_inner() {
                    match alias_inner.as_rule() {
                        Rule::state_id => {
                            state_id = alias_inner.as_str().to_string();
                        }
                        Rule::quoted_string => {
                            let s = alias_inner.as_str();
                            alias = s[1..s.len() - 1].to_string();
                        }
                        _ => {}
                    }
                }

                db.add_state(&state_id);
                if let Some(state) = db.get_state_mut(&state_id) {
                    state.alias = Some(alias);
                }
                if let Some(p) = parent {
                    db.set_parent(&state_id, p);
                }
            }
            Rule::state_simple => {
                let mut state_id = String::new();
                let mut description: Option<String> = None;

                for simple_inner in inner.into_inner() {
                    match simple_inner.as_rule() {
                        Rule::state_id => {
                            state_id = simple_inner.as_str().to_string();
                        }
                        Rule::state_description => {
                            for desc in simple_inner.into_inner() {
                                if desc.as_rule() == Rule::description_text {
                                    description = Some(desc.as_str().trim().to_string());
                                }
                            }
                        }
                        _ => {}
                    }
                }

                db.add_state(&state_id);
                if let Some(desc) = description {
                    db.add_description(&state_id, &desc);
                }
                if let Some(p) = parent {
                    db.set_parent(&state_id, p);
                }
            }
            _ => {}
        }
    }
    Ok(())
}

fn process_note(db: &mut StateDb, pair: pest::iterators::Pair<Rule>) -> Result<(), String> {
    let mut placement = NotePosition::RightOf;
    let mut state_id = String::new();
    let mut note_text = String::new();

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::note_placement => {
                let text = inner.as_str().to_lowercase();
                if text.contains("left") {
                    placement = NotePosition::LeftOf;
                }
            }
            Rule::note_target => {
                for target in inner.into_inner() {
                    if target.as_rule() == Rule::state_id {
                        state_id = target.as_str().to_string();
                    }
                }
            }
            Rule::note_content => {
                for content in inner.into_inner() {
                    match content.as_rule() {
                        Rule::note_inline => {
                            for inline in content.into_inner() {
                                if inline.as_rule() == Rule::line_content {
                                    note_text = inline.as_str().trim().to_string();
                                }
                            }
                        }
                        Rule::note_block => {
                            for block in content.into_inner() {
                                if block.as_rule() == Rule::note_lines {
                                    note_text = block.as_str().trim().to_string();
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

    if !state_id.is_empty() {
        db.add_note(&state_id, placement, &note_text);
    }
    Ok(())
}

fn process_transition(
    db: &mut StateDb,
    pair: pest::iterators::Pair<Rule>,
    parent: Option<&str>,
) -> Result<(), String> {
    let mut from = String::new();
    let mut to = String::new();
    let mut label: Option<String> = None;
    let mut ref_count = 0;

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::state_ref => {
                ref_count += 1;
                for ref_inner in inner.into_inner() {
                    match ref_inner.as_rule() {
                        Rule::start_end_marker => {
                            if ref_count == 1 {
                                from = "[*]".to_string();
                            } else {
                                to = "[*]".to_string();
                            }
                        }
                        Rule::state_id => {
                            let id = ref_inner.as_str().to_string();
                            if ref_count == 1 {
                                from = id;
                            } else {
                                to = id;
                            }
                        }
                        _ => {}
                    }
                }
            }
            Rule::transition_label => {
                for lbl in inner.into_inner() {
                    if lbl.as_rule() == Rule::label_text {
                        label = Some(lbl.as_str().trim().to_string());
                    }
                }
            }
            Rule::arrow => {
                // Arrow type - we just use it to confirm transition
            }
            _ => {}
        }
    }

    // Ensure non-[*] states exist (add_relation handles [*] states specially)
    // Also set parent relationship for states inside composite states
    if from != "[*]" {
        db.add_state(&from);
        if let Some(p) = parent {
            db.set_parent(&from, p);
        }
    }
    if to != "[*]" {
        db.add_state(&to);
        if let Some(p) = parent {
            db.set_parent(&to, p);
        }
    }

    // Add the relation - this handles [*] states by creating unique start/end IDs
    // Pass parent so [*] states inside composites get the correct parent
    db.add_relation(&from, &to, label.as_deref(), parent);
    Ok(())
}

fn process_state_with_description(
    db: &mut StateDb,
    pair: pest::iterators::Pair<Rule>,
) -> Result<(), String> {
    let mut state_id = String::new();
    let mut description = String::new();

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::state_id => {
                state_id = inner.as_str().to_string();
            }
            Rule::description_text => {
                description = inner.as_str().trim().to_string();
            }
            _ => {}
        }
    }

    if !state_id.is_empty() {
        db.add_state(&state_id);
        if !description.is_empty() {
            db.add_description(&state_id, &description);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    mod basic_parsing {
        use super::*;

        #[test]
        fn should_parse_empty_diagram() {
            let result = parse("stateDiagram\n");
            assert!(result.is_ok());
        }

        #[test]
        fn should_parse_v2_diagram() {
            let result = parse("stateDiagram-v2\n");
            assert!(result.is_ok());
        }

        #[test]
        fn should_parse_transition_stmt_directly() {
            // Test parsing just the transition_stmt rule
            let test_cases = ["State1 --> State2", "[*] --> State1", "State1 --> [*]"];
            for input in &test_cases {
                let result = StateParser::parse(Rule::transition_stmt, input);
                assert!(
                    result.is_ok(),
                    "Failed to parse transition_stmt '{}': {:?}",
                    input,
                    result.err()
                );
            }
        }

        #[test]
        fn should_parse_statement_as_transition() {
            // Test parsing statement rule - should match transition_stmt
            let test_cases = ["State1 --> State2", "[*] --> State1", "State1 --> [*]"];
            for input in &test_cases {
                let result = StateParser::parse(Rule::statement, input);
                assert!(
                    result.is_ok(),
                    "Failed to parse statement '{}': {:?}",
                    input,
                    result.err()
                );
                // Verify it was parsed as transition_stmt
                let pairs = result.unwrap();
                let first = pairs.into_iter().next().unwrap();
                let inner = first.into_inner().next();
                if let Some(inner_pair) = inner {
                    println!("Rule: {:?} for input: {}", inner_pair.as_rule(), input);
                }
            }
        }

        #[test]
        fn should_parse_simple_transition() {
            let result = parse("stateDiagram\n[*] --> State1\nState1 --> [*]");
            assert!(result.is_ok(), "Parse error: {:?}", result.err());
            let db = result.unwrap();
            let relations = db.get_relations();
            assert_eq!(relations.len(), 2);
        }

        #[test]
        fn should_parse_state_with_description() {
            let result = parse("stateDiagram\nState1 : this is a description");
            assert!(result.is_ok(), "Parse error: {:?}", result.err());
            let db = result.unwrap();
            let state = db.get_state("State1").unwrap();
            assert!(!state.descriptions.is_empty());
        }

        // Note: Numeric-only state IDs (like "1") are not currently supported
        // because they conflict with the case-insensitive "state" keyword
        // (e.g., "State1" would be parsed as "state" + "1")
    }

    mod direction {
        use super::*;

        #[test]
        fn should_parse_direction_tb() {
            let result = parse("stateDiagram\ndirection TB");
            assert!(result.is_ok());
            let db = result.unwrap();
            assert_eq!(db.get_direction(), Direction::TopToBottom);
        }

        #[test]
        fn should_parse_direction_lr() {
            let result = parse("stateDiagram\ndirection LR");
            assert!(result.is_ok());
            let db = result.unwrap();
            assert_eq!(db.get_direction(), Direction::LeftToRight);
        }
    }

    mod accessibility {
        use super::*;

        #[test]
        fn should_parse_acc_title() {
            let result = parse("stateDiagram\naccTitle: My State Diagram");
            assert!(result.is_ok());
            let db = result.unwrap();
            assert_eq!(db.acc_title, "My State Diagram");
        }

        #[test]
        fn should_parse_acc_descr() {
            let result = parse("stateDiagram\naccDescr: A description of the diagram");
            assert!(result.is_ok());
            let db = result.unwrap();
            assert_eq!(db.acc_descr, "A description of the diagram");
        }

        #[test]
        fn should_parse_multiline_acc_descr() {
            let result =
                parse("stateDiagram\naccDescr {\na simple description\nusing multiple lines\n}");
            assert!(result.is_ok());
            let db = result.unwrap();
            assert!(db.acc_descr.contains("a simple description"));
            assert!(db.acc_descr.contains("using multiple lines"));
        }
    }

    mod special_states {
        use super::*;

        #[test]
        fn should_parse_fork_state() {
            let result = parse("stateDiagram\nstate forkState <<fork>>");
            assert!(result.is_ok());
            let db = result.unwrap();
            let state = db.get_state("forkState").unwrap();
            assert_eq!(state.state_type, StateType::Fork);
        }

        #[test]
        fn should_parse_join_state() {
            let result = parse("stateDiagram\nstate joinState <<join>>");
            assert!(result.is_ok());
            let db = result.unwrap();
            let state = db.get_state("joinState").unwrap();
            assert_eq!(state.state_type, StateType::Join);
        }

        #[test]
        fn should_parse_choice_state() {
            let result = parse("stateDiagram\nstate choiceState <<choice>>");
            assert!(result.is_ok());
            let db = result.unwrap();
            let state = db.get_state("choiceState").unwrap();
            assert_eq!(state.state_type, StateType::Choice);
        }

        #[test]
        fn should_parse_choice_state_with_bracket_syntax() {
            let result = parse("stateDiagram\nstate choiceState [[choice]]");
            assert!(result.is_ok());
            let db = result.unwrap();
            let state = db.get_state("choiceState").unwrap();
            assert_eq!(state.state_type, StateType::Choice);
        }
    }

    mod composite_states {
        use super::*;

        #[test]
        fn should_parse_composite_state() {
            let result =
                parse("stateDiagram\nstate CompositeState {\n[*] --> First\nFirst --> [*]\n}");
            assert!(result.is_ok());
            let db = result.unwrap();
            assert!(db.get_states().contains_key("CompositeState"));
            assert!(db.get_states().contains_key("First"));
        }
    }

    mod classes {
        use super::*;

        #[test]
        fn should_parse_class_def() {
            let result = parse("stateDiagram\nclassDef myClass fill:#f00,color:white");
            assert!(result.is_ok());
            let db = result.unwrap();
            assert!(db.get_classes().contains_key("myClass"));
        }

        #[test]
        fn should_apply_class_to_state() {
            let result =
                parse("stateDiagram\nclassDef myClass fill:#f00\nState1\nclass State1 myClass");
            assert!(result.is_ok(), "Parse error: {:?}", result.err());
            let db = result.unwrap();
            let state = db.get_state("State1").unwrap();
            assert!(state.classes.contains(&"myClass".to_string()));
        }
    }

    mod transition_labels {
        use super::*;

        #[test]
        fn should_parse_transition_with_label() {
            let result = parse("stateDiagram\n[*] --> State1 : Start");
            assert!(result.is_ok());
            let db = result.unwrap();
            let relations = db.get_relations();
            assert_eq!(relations[0].description, Some("Start".to_string()));
        }
    }

    mod hide_empty {
        use super::*;

        #[test]
        fn should_parse_hide_empty_description() {
            let result = parse("stateDiagram\nhide empty description");
            assert!(result.is_ok());
        }
    }

    mod comments {
        use super::*;

        #[test]
        fn should_handle_comments() {
            let result = parse("stateDiagram\n%% This is a comment\n[*] --> State1");
            assert!(result.is_ok());
        }

        #[test]
        fn should_handle_comments_before_diagram() {
            let result = parse("%% Comment before\nstateDiagram\n[*] --> State1");
            assert!(result.is_ok());
        }
    }

    mod notes {
        use super::*;

        #[test]
        fn should_parse_multiline_note() {
            let result = parse(
                "stateDiagram-v2
    State1: The state with a note
    note right of State1
      Important information! You can write
      notes.
    end note",
            );
            assert!(result.is_ok(), "Parse error: {:?}", result.err());
        }

        #[test]
        fn should_parse_inline_note() {
            let result = parse("stateDiagram\nnote right of State1 : Short note");
            assert!(result.is_ok(), "Parse error: {:?}", result.err());
        }
    }

    mod complex_diagrams {
        use super::*;

        #[test]
        fn should_parse_full_diagram() {
            let input = r#"stateDiagram-v2
    direction LR
    accTitle: Order State Machine
    accDescr: Shows order lifecycle

    [*] --> Pending
    Pending --> Processing : Order received
    Processing --> Shipped
    Shipped --> Delivered
    Delivered --> [*]

    state Processing {
        [*] --> Validating
        Validating --> Packing
        Packing --> [*]
    }
"#;

            let result = parse(input);
            assert!(result.is_ok());
            let db = result.unwrap();

            assert_eq!(db.get_direction(), Direction::LeftToRight);
            assert!(db.get_states().contains_key("Pending"));
            assert!(db.get_states().contains_key("Processing"));
            assert!(db.get_states().contains_key("Shipped"));
        }
    }

    // Tests ported from mermaid Cypress tests (stateDiagram.spec.js)
    mod cypress_tests {
        use super::*;

        #[test]
        fn test_cypress_simple_state_diagram() {
            // From Cypress: should render a simple state diagrams
            let input = r#"stateDiagram
    [*] --> State1
    State1 --> [*]"#;
            let result = parse(input);
            assert!(result.is_ok(), "Failed to parse: {:?}", result);
            let db = result.unwrap();
            assert!(db.get_states().contains_key("State1"));
        }

        #[test]
        fn test_cypress_long_descriptions() {
            // From Cypress: should render a long descriptions instead of id when available
            let input = r#"stateDiagram
      [*] --> S1
      state "Some long name" as S1"#;
            let result = parse(input);
            assert!(result.is_ok(), "Failed to parse: {:?}", result);
        }

        #[test]
        fn test_cypress_long_descriptions_with_additional() {
            // From Cypress: should render a long descriptions with additional descriptions
            let input = r#"stateDiagram
      [*] --> S1
      state "Some long name" as S1: The description"#;
            let result = parse(input);
            assert!(result.is_ok(), "Failed to parse: {:?}", result);
        }

        #[test]
        fn test_cypress_transition_descriptions_newlines() {
            // From Cypress: should render a transition descriptions with new lines
            let input = r#"stateDiagram
      [*] --> S1
      S1 --> S2: long line using<br/>should work
      S1 --> S3: long line using <br>should work
      S1 --> S4: long line using \nshould work"#;
            let result = parse(input);
            assert!(result.is_ok(), "Failed to parse: {:?}", result);
        }

        #[test]
        fn test_cypress_state_with_note() {
            // From Cypress: should render a state with a note
            let input = r#"stateDiagram
    State1: The state with a note
    note right of State1
      Important information! You can write
      notes.
    end note"#;
            let result = parse(input);
            assert!(result.is_ok(), "Failed to parse: {:?}", result);
        }

        #[test]
        fn test_cypress_note_left_side() {
            // From Cypress: should render a state with on the left side when so specified
            let input = r#"stateDiagram
    State1: The state with a note with minus - and plus + in it
    note left of State1
      Important information! You can write
      notes with . and  in them.
    end note"#;
            let result = parse(input);
            assert!(result.is_ok(), "Failed to parse: {:?}", result);
        }

        #[test]
        fn test_cypress_multi_description() {
            // From Cypress: should render a states with descriptions including multi-line descriptions
            let input = r#"stateDiagram
    State1: This a single line description
    State2: This a multi line description
    State2: here comes the multi part
    [*] --> State1
    State1 --> State2
    State2 --> [*]"#;
            let result = parse(input);
            assert!(result.is_ok(), "Failed to parse: {:?}", result);
        }

        #[test]
        fn test_cypress_multiple_transitions() {
            // From Cypress: multiple transitions from state
            let input = r#"stateDiagram
    [*] --> State1
    State1 --> State2
    State1 --> State3
    State1 --> [*]"#;
            let result = parse(input);
            assert!(result.is_ok(), "Failed to parse: {:?}", result);
        }

        #[test]
        fn test_cypress_composite_state() {
            // From Cypress: should render a composite state
            let input = r#"stateDiagram
    [*] --> First
    First --> Second
    First --> Third

    state First {
        [*] --> 1st
        1st --> [*]
    }"#;
            let result = parse(input);
            assert!(result.is_ok(), "Failed to parse: {:?}", result);
        }

        #[test]
        fn test_cypress_parallel_states() {
            // From Cypress: should render a state with parallel states
            let input = r#"stateDiagram
    state Active {
        [*] --> NumLock
        --
        [*] --> CapsLock
        --
        [*] --> ScrollLock
    }"#;
            let result = parse(input);
            assert!(result.is_ok(), "Failed to parse: {:?}", result);
        }

        #[test]
        fn test_cypress_fork_join() {
            // From Cypress: should render a state with forks and joins
            let input = r#"stateDiagram-v2
    state fork_state <<fork>>
      [*] --> fork_state
      fork_state --> State2
      fork_state --> State3

      state join_state <<join>>
      State2 --> join_state
      State3 --> join_state
      join_state --> State4
      State4 --> [*]"#;
            let result = parse(input);
            assert!(result.is_ok(), "Failed to parse: {:?}", result);
        }

        #[test]
        fn test_cypress_choice() {
            // From Cypress: should render a state with choice
            let input = r#"stateDiagram-v2
    state if_state <<choice>>
    [*] --> IsPositive
    IsPositive --> if_state
    if_state --> False: if n < 0
    if_state --> True : if n >= 0"#;
            let result = parse(input);
            assert!(result.is_ok(), "Failed to parse: {:?}", result);
        }

        #[test]
        fn test_cypress_direction_lr() {
            // From Cypress: should render a state diagram with direction LR
            let input = r#"stateDiagram
    direction LR
    [*] --> A
    A --> B
    B --> C"#;
            let result = parse(input);
            assert!(result.is_ok(), "Failed to parse: {:?}", result);
            let db = result.unwrap();
            assert_eq!(db.get_direction(), Direction::LeftToRight);
        }

        #[test]
        fn test_cypress_class_def_and_class() {
            // From Cypress: should support classDef and class statements
            let input = r#"stateDiagram-v2
    classDef badBadEvent fill:#f00,color:white,font-weight:bold,stroke-width:2px,stroke:yellow
    [*] --> A
    A --> B: goB
    class A badBadEvent"#;
            let result = parse(input);
            assert!(result.is_ok(), "Failed to parse: {:?}", result);
        }

        #[test]
        fn test_cypress_v2_syntax() {
            // From Cypress: stateDiagram-v2 syntax
            let input = r#"stateDiagram-v2
    [*] --> Still
    Still --> [*]
    Still --> Moving
    Moving --> Still
    Moving --> Crash
    Crash --> [*]"#;
            let result = parse(input);
            assert!(result.is_ok(), "Failed to parse: {:?}", result);
        }

        #[test]
        fn test_complex_nested_composite_states() {
            // Complex diagram with deeply nested composite states
            let input = r#"stateDiagram-v2
[*] --> Idle

state Idle {
    [*] --> Ready
    Ready --> Processing: Start Job
}

state Processing {
    [*] --> Validating
    Validating --> Queued: Valid
    Validating --> Failed: Invalid
    Queued --> Running: Worker Available
    Running --> Completed: Success
    Running --> Failed: Error
    Running --> Paused: Pause Request

    state Running {
        [*] --> Initializing
        Initializing --> Executing
        Executing --> Finalizing
        Finalizing --> [*]
    }
}

state Paused {
    [*] --> WaitingResume
    WaitingResume --> Timeout: 1 hour
}

Paused --> Running: Resume
Paused --> Cancelled: Cancel Request
Timeout --> Cancelled

Completed --> Idle: Reset
Failed --> Idle: Retry
Cancelled --> Idle: Reset

Completed --> [*]
Cancelled --> [*]"#;
            let result = parse(input);
            assert!(result.is_ok(), "Failed to parse: {:?}", result);
            let db = result.unwrap();

            // Verify state count - should have many states
            let states = db.get_states();
            assert!(
                states.len() >= 10,
                "Expected at least 10 states, got {}",
                states.len()
            );

            // Verify parent relationships for key states
            // Ready should be inside Idle
            let ready = states.get("Ready").expect("Ready state should exist");
            assert_eq!(
                ready.parent.as_deref(),
                Some("Idle"),
                "Ready should have parent Idle"
            );

            // Processing is referenced from inside Idle via `Ready --> Processing`.
            // In mermaid, first assignment wins: the transition sets parent=Idle,
            // and the later `state Processing { }` at root level doesn't change it.
            let processing = states
                .get("Processing")
                .expect("Processing state should exist");
            assert_eq!(
                processing.parent.as_deref(),
                Some("Idle"),
                "Processing should have parent Idle (first assignment wins)"
            );

            // Validating should be inside Processing
            let validating = states
                .get("Validating")
                .expect("Validating state should exist");
            assert_eq!(
                validating.parent.as_deref(),
                Some("Processing"),
                "Validating should have parent Processing"
            );

            // Running should be inside Processing
            let running = states.get("Running").expect("Running state should exist");
            assert_eq!(
                running.parent.as_deref(),
                Some("Processing"),
                "Running should have parent Processing"
            );

            // Initializing should be inside Running (nested 3 levels deep)
            let initializing = states
                .get("Initializing")
                .expect("Initializing state should exist");
            assert_eq!(
                initializing.parent.as_deref(),
                Some("Running"),
                "Initializing should have parent Running"
            );

            // Paused is referenced from inside Processing via `Running --> Paused`.
            // In mermaid, first assignment wins: the transition sets parent=Processing,
            // and the later `state Paused { }` at root level doesn't change it.
            let paused = states.get("Paused").expect("Paused state should exist");
            assert_eq!(
                paused.parent.as_deref(),
                Some("Processing"),
                "Paused should have parent Processing (first assignment wins)"
            );

            // WaitingResume should be inside Paused
            let waiting = states
                .get("WaitingResume")
                .expect("WaitingResume state should exist");
            assert_eq!(
                waiting.parent.as_deref(),
                Some("Paused"),
                "WaitingResume should have parent Paused"
            );

            // Cancelled should be at root level (defined in root transitions)
            let cancelled = states
                .get("Cancelled")
                .expect("Cancelled state should exist");
            assert_eq!(
                cancelled.parent.as_deref(),
                None,
                "Cancelled should be at root level"
            );

            // Verify relations count
            let relations = db.get_relations();
            assert!(
                relations.len() >= 15,
                "Expected at least 15 relations, got {}",
                relations.len()
            );
        }
    }
}
