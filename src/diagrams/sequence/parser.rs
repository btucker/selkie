//! Sequence diagram parser using pest grammar

use pest::Parser;
use pest_derive::Parser;

use super::types::{LineType, ParticipantType, Placement, SequenceDb};

#[derive(Parser)]
#[grammar = "diagrams/sequence/sequence.pest"]
pub struct SequenceParser;

/// Types of blocks that can be open
#[derive(Debug, Clone, Copy, PartialEq)]
enum BlockType {
    Box,
    Loop,
    Alt,
    Opt,
    Par,
    Critical,
    Break,
    Rect,
}

/// Parse a sequence diagram and return the populated database
pub fn parse(input: &str) -> Result<SequenceDb, String> {
    let mut db = SequenceDb::new();
    let mut block_stack: Vec<BlockType> = Vec::new();

    let pairs =
        SequenceParser::parse(Rule::diagram, input).map_err(|e| format!("Parse error: {}", e))?;

    for pair in pairs {
        if pair.as_rule() == Rule::diagram {
            for inner in pair.into_inner() {
                if inner.as_rule() == Rule::document {
                    process_document(&mut db, inner, &mut block_stack)?;
                }
            }
        }
    }

    Ok(db)
}

fn process_document(
    db: &mut SequenceDb,
    pair: pest::iterators::Pair<Rule>,
    block_stack: &mut Vec<BlockType>,
) -> Result<(), String> {
    for stmt in pair.into_inner() {
        process_statement(db, stmt, block_stack)?;
    }
    Ok(())
}

fn process_statement(
    db: &mut SequenceDb,
    pair: pest::iterators::Pair<Rule>,
    block_stack: &mut Vec<BlockType>,
) -> Result<(), String> {
    match pair.as_rule() {
        Rule::statement => {
            for inner in pair.into_inner() {
                process_statement(db, inner, block_stack)?;
            }
        }
        Rule::comment_stmt => {
            // Ignore comments
        }
        Rule::title_stmt => {
            for inner in pair.into_inner() {
                if inner.as_rule() == Rule::line_content {
                    db.diagram_title = inner.as_str().trim().to_string();
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
        Rule::participant_stmt => {
            process_participant_stmt(db, pair)?;
        }
        Rule::box_start => {
            let mut color = String::new();
            let mut title = String::new();
            for inner in pair.into_inner() {
                match inner.as_rule() {
                    Rule::box_color => {
                        color = inner.as_str().to_string();
                    }
                    Rule::box_title => {
                        title = inner.as_str().trim().to_string();
                    }
                    _ => {}
                }
            }
            block_stack.push(BlockType::Box);
            db.add_box(&title, &color);
        }
        Rule::autonumber_stmt => {
            let mut start: Option<i32> = None;
            let mut step: Option<i32> = None;
            let mut enabled = true;

            for inner in pair.into_inner() {
                if inner.as_rule() == Rule::autonumber_args {
                    let args_str = inner.as_str().trim();
                    if args_str == "off" {
                        enabled = false;
                    } else {
                        let parts: Vec<&str> = args_str.split_whitespace().collect();
                        if !parts.is_empty() {
                            start = parts[0].parse().ok();
                        }
                        if parts.len() > 1 {
                            step = parts[1].parse().ok();
                        }
                    }
                }
            }
            db.set_autonumber(enabled, start, step);
        }
        Rule::activate_stmt => {
            for inner in pair.into_inner() {
                if inner.as_rule() == Rule::actor_name {
                    let name = inner.as_str();
                    db.add_signal(LineType::ActiveStart, Some(name));
                }
            }
        }
        Rule::deactivate_stmt => {
            for inner in pair.into_inner() {
                if inner.as_rule() == Rule::actor_name {
                    let name = inner.as_str();
                    db.add_signal(LineType::ActiveEnd, Some(name));
                }
            }
        }
        Rule::loop_start => {
            let mut text = String::new();
            for inner in pair.into_inner() {
                if inner.as_rule() == Rule::loop_text {
                    text = inner.as_str().trim().to_string();
                }
            }
            block_stack.push(BlockType::Loop);
            db.add_signal(LineType::LoopStart, Some(&text));
        }
        Rule::alt_start => {
            let mut text = String::new();
            for inner in pair.into_inner() {
                if inner.as_rule() == Rule::alt_text {
                    text = inner.as_str().trim().to_string();
                }
            }
            block_stack.push(BlockType::Alt);
            db.add_signal(LineType::AltStart, Some(&text));
        }
        Rule::alt_else => {
            let mut text = String::new();
            for inner in pair.into_inner() {
                if inner.as_rule() == Rule::alt_text {
                    text = inner.as_str().trim().to_string();
                }
            }
            db.add_signal(LineType::AltElse, Some(&text));
        }
        Rule::opt_start => {
            let mut text = String::new();
            for inner in pair.into_inner() {
                if inner.as_rule() == Rule::opt_text {
                    text = inner.as_str().trim().to_string();
                }
            }
            block_stack.push(BlockType::Opt);
            db.add_signal(LineType::OptStart, Some(&text));
        }
        Rule::par_start => {
            let mut text = String::new();
            for inner in pair.into_inner() {
                if inner.as_rule() == Rule::par_text {
                    text = inner.as_str().trim().to_string();
                }
            }
            block_stack.push(BlockType::Par);
            db.add_signal(LineType::ParStart, Some(&text));
        }
        Rule::par_and => {
            let mut text = String::new();
            for inner in pair.into_inner() {
                if inner.as_rule() == Rule::par_text {
                    text = inner.as_str().trim().to_string();
                }
            }
            db.add_signal(LineType::ParAnd, Some(&text));
        }
        Rule::critical_start => {
            let mut text = String::new();
            for inner in pair.into_inner() {
                if inner.as_rule() == Rule::critical_text {
                    text = inner.as_str().trim().to_string();
                }
            }
            block_stack.push(BlockType::Critical);
            db.add_signal(LineType::CriticalStart, Some(&text));
        }
        Rule::critical_option => {
            let mut text = String::new();
            for inner in pair.into_inner() {
                if inner.as_rule() == Rule::critical_text {
                    text = inner.as_str().trim().to_string();
                }
            }
            db.add_signal(LineType::CriticalOption, Some(&text));
        }
        Rule::break_start => {
            let mut text = String::new();
            for inner in pair.into_inner() {
                if inner.as_rule() == Rule::break_text {
                    text = inner.as_str().trim().to_string();
                }
            }
            block_stack.push(BlockType::Break);
            db.add_signal(LineType::BreakStart, Some(&text));
        }
        Rule::rect_start => {
            let mut color = String::new();
            for inner in pair.into_inner() {
                if inner.as_rule() == Rule::rect_color {
                    color = inner.as_str().to_string();
                }
            }
            block_stack.push(BlockType::Rect);
            db.add_signal(LineType::RectStart, Some(&color));
        }
        Rule::end_stmt => {
            // Pop the block stack and emit the appropriate end signal
            if let Some(block_type) = block_stack.pop() {
                match block_type {
                    BlockType::Box => db.end_box(),
                    BlockType::Loop => db.add_signal(LineType::LoopEnd, None),
                    BlockType::Alt => db.add_signal(LineType::AltEnd, None),
                    BlockType::Opt => db.add_signal(LineType::OptEnd, None),
                    BlockType::Par => db.add_signal(LineType::ParEnd, None),
                    BlockType::Critical => db.add_signal(LineType::CriticalEnd, None),
                    BlockType::Break => db.add_signal(LineType::BreakEnd, None),
                    BlockType::Rect => db.add_signal(LineType::RectEnd, None),
                }
            }
        }
        Rule::create_stmt => {
            process_create_stmt(db, pair)?;
        }
        Rule::destroy_stmt => {
            for inner in pair.into_inner() {
                if inner.as_rule() == Rule::actor_name {
                    db.destroy_actor(inner.as_str());
                }
            }
        }
        Rule::note_stmt => {
            process_note_stmt(db, pair)?;
        }
        Rule::message_stmt => {
            process_message_stmt(db, pair)?;
        }
        _ => {}
    }
    Ok(())
}

fn process_participant_stmt(
    db: &mut SequenceDb,
    pair: pest::iterators::Pair<Rule>,
) -> Result<(), String> {
    let mut participant_type = ParticipantType::Participant;
    let mut name = String::new();
    let mut description: Option<String> = None;

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::participant_type => {
                participant_type = ParticipantType::from_str(inner.as_str());
            }
            Rule::actor_name => {
                name = inner.as_str().to_string();
            }
            Rule::alias_def => {
                for alias in inner.into_inner() {
                    if alias.as_rule() == Rule::actor_description {
                        description = Some(alias.as_str().trim().to_string());
                    }
                }
            }
            _ => {}
        }
    }

    if !name.is_empty() {
        db.add_actor(&name, description.as_deref(), participant_type);
    }

    Ok(())
}

fn process_create_stmt(
    db: &mut SequenceDb,
    pair: pest::iterators::Pair<Rule>,
) -> Result<(), String> {
    let mut participant_type = ParticipantType::Participant;
    let mut name = String::new();
    let mut description: Option<String> = None;

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::participant_type => {
                participant_type = ParticipantType::from_str(inner.as_str());
            }
            Rule::actor_name => {
                name = inner.as_str().to_string();
            }
            Rule::alias_def => {
                for alias in inner.into_inner() {
                    if alias.as_rule() == Rule::actor_description {
                        description = Some(alias.as_str().trim().to_string());
                    }
                }
            }
            _ => {}
        }
    }

    if !name.is_empty() {
        db.create_actor(&name, description.as_deref(), participant_type);
    }

    Ok(())
}

fn process_note_stmt(db: &mut SequenceDb, pair: pest::iterators::Pair<Rule>) -> Result<(), String> {
    let mut placement = Placement::RightOf;
    let mut actors: Vec<String> = Vec::new();
    let mut text = String::new();

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::note_placement => {
                let placement_str = inner.as_str().to_lowercase();
                if placement_str.contains("left") {
                    placement = Placement::LeftOf;
                } else if placement_str.contains("over") {
                    placement = Placement::Over;
                } else {
                    placement = Placement::RightOf;
                }
                // Extract actors from note_placement (for left/right of)
                for actor in inner.into_inner() {
                    match actor.as_rule() {
                        Rule::actor_name => {
                            actors.push(actor.as_str().to_string());
                        }
                        Rule::actor_list => {
                            // Handle comma-separated actor list for "over" notes
                            for list_item in actor.into_inner() {
                                if list_item.as_rule() == Rule::actor_name {
                                    actors.push(list_item.as_str().to_string());
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
            Rule::note_text => {
                text = inner.as_str().trim().to_string();
            }
            _ => {}
        }
    }

    // Add note for the first actor (most common case)
    if let Some(actor) = actors.first() {
        db.add_note(actor, placement, &text);
    }

    Ok(())
}

fn process_message_stmt(
    db: &mut SequenceDb,
    pair: pest::iterators::Pair<Rule>,
) -> Result<(), String> {
    let mut from = String::new();
    let mut to = String::new();
    let mut message = String::new();
    let mut arrow = String::new();
    let mut activate = false;
    let mut deactivate = false;
    let mut actor_count = 0;

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::actor_ref => {
                actor_count += 1;
                for actor_inner in inner.into_inner() {
                    match actor_inner.as_rule() {
                        Rule::actor_name => {
                            if actor_count == 1 {
                                from = actor_inner.as_str().to_string();
                            } else {
                                to = actor_inner.as_str().to_string();
                            }
                        }
                        Rule::activation_marker => {
                            let marker = actor_inner.as_str();
                            if actor_count == 2 {
                                // Target actor activation
                                if marker == "+" {
                                    activate = true;
                                } else if marker == "-" {
                                    deactivate = true;
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
            Rule::arrow_type => {
                arrow = inner.as_str().to_string();
            }
            Rule::message_text => {
                message = inner.as_str().trim().to_string();
            }
            _ => {}
        }
    }

    // Parse arrow to determine line type and activation
    let (line_type, arrow_activate, arrow_deactivate) = parse_arrow_type(&arrow);

    // Combine activation from arrow and actor markers
    let activate = activate || arrow_activate;
    let deactivate = deactivate || arrow_deactivate;

    // Add the message
    db.add_message(&from, &to, &message, line_type, activate);

    // Handle activation signals
    if activate {
        db.add_signal(LineType::ActiveStart, Some(&to));
    }
    if deactivate {
        db.add_signal(LineType::ActiveEnd, Some(&to));
    }

    Ok(())
}

fn parse_arrow_type(arrow: &str) -> (LineType, bool, bool) {
    // Check for activation markers at end of arrow
    let activate = arrow.ends_with('+');
    let deactivate = arrow.ends_with('-') && !arrow.ends_with("->") && !arrow.ends_with(">>");

    // Handle bidirectional arrows
    if arrow.contains("<<->>") {
        return (LineType::BidirectionalSolid, activate, deactivate);
    }
    if arrow.contains("<<-->>") {
        return (LineType::BidirectionalDotted, activate, deactivate);
    }

    // Handle dotted vs solid
    let is_dotted = arrow.contains("--");

    // Handle different arrow head types
    let line_type = if arrow.contains("-x") || arrow.contains("--x") {
        if is_dotted {
            LineType::DottedCross
        } else {
            LineType::SolidCross
        }
    } else if arrow.contains("-)") || arrow.contains("--)") {
        if is_dotted {
            LineType::DottedPoint
        } else {
            LineType::SolidPoint
        }
    } else if arrow.contains("->>") || arrow.contains("-->>") {
        if is_dotted {
            LineType::Dotted
        } else {
            LineType::Solid
        }
    } else if arrow.contains("->") || arrow.contains("-->") {
        if is_dotted {
            LineType::DottedOpen
        } else {
            LineType::SolidOpen
        }
    } else if is_dotted {
        LineType::DottedOpen
    } else {
        LineType::SolidOpen
    };

    (line_type, activate, deactivate)
}

#[cfg(test)]
mod tests {
    use super::*;

    mod basic_parsing {
        use super::*;

        #[test]
        fn should_parse_empty_diagram() {
            let result = parse("sequenceDiagram\n");
            assert!(result.is_ok());
        }

        #[test]
        fn should_parse_simple_message() {
            let result = parse("sequenceDiagram\nAlice->Bob:Hello Bob, how are you?");
            assert!(result.is_ok());
            let db = result.unwrap();
            let messages = db.get_messages();
            assert_eq!(messages.len(), 1);
            assert_eq!(messages[0].from, Some("Alice".to_string()));
            assert_eq!(messages[0].to, Some("Bob".to_string()));
            assert_eq!(messages[0].message, "Hello Bob, how are you?");
        }

        #[test]
        fn should_handle_dashes_in_actor_names() {
            let result = parse("sequenceDiagram\nAlice-in-Wonderland->Bob:Hello");
            assert!(result.is_ok());
            let db = result.unwrap();
            assert!(db.get_actors().contains_key("Alice-in-Wonderland"));
        }

        #[test]
        fn should_handle_equals_in_actor_names() {
            let result = parse("sequenceDiagram\nAlice=Wonderland->Bob:Hello");
            assert!(result.is_ok());
            let db = result.unwrap();
            assert!(db.get_actors().contains_key("Alice=Wonderland"));
        }
    }

    mod participants {
        use super::*;

        #[test]
        fn should_parse_participant() {
            let result = parse("sequenceDiagram\nparticipant Alice");
            assert!(result.is_ok());
            let db = result.unwrap();
            assert!(db.get_actors().contains_key("Alice"));
        }

        #[test]
        fn should_parse_actor() {
            let result = parse("sequenceDiagram\nactor Alice");
            assert!(result.is_ok());
            let db = result.unwrap();
            let alice = db.get_actors().get("Alice").unwrap();
            assert_eq!(alice.actor_type, ParticipantType::Actor);
        }

        #[test]
        fn should_alias_participants() {
            let result = parse("sequenceDiagram\nparticipant A as Alice");
            assert!(result.is_ok());
            let db = result.unwrap();
            let actor = db.get_actors().get("A").unwrap();
            assert_eq!(actor.description, "Alice");
        }

        #[test]
        fn should_handle_numeric_participant_name() {
            // Test numeric participant name
            let result = parse("sequenceDiagram\nparticipant 1");
            assert!(
                result.is_ok(),
                "Failed to parse numeric participant: {:?}",
                result.err()
            );
        }

        #[test]
        fn should_handle_numeric_participant_with_alias() {
            // Test numeric participant name with alias
            let result = parse("sequenceDiagram\nparticipant 1 as One");
            assert!(result.is_ok(), "Failed to parse: {:?}", result.err());
            let db = result.unwrap();
            let actor = db.get_actors().get("1").unwrap();
            assert_eq!(actor.description, "One");
        }

        #[test]
        fn should_handle_participant_with_html_entities() {
            // Test participant with HTML entity encoding (#lt; and #gt;)
            let result = parse("sequenceDiagram\nparticipant 1 as multiline<br>using #lt;br#gt;");
            assert!(result.is_ok(), "Failed to parse: {:?}", result.err());
            let db = result.unwrap();
            let actor = db.get_actors().get("1").unwrap();
            assert_eq!(actor.description, "multiline<br>using #lt;br#gt;");
        }

        #[test]
        fn should_parse_all_participant_types() {
            let result = parse(
                "sequenceDiagram
participant P
actor A
boundary B
control C
entity E
database D
collections Col
queue Q",
            );
            assert!(result.is_ok());
            let db = result.unwrap();
            assert_eq!(db.get_actors().len(), 8);
        }

        #[test]
        fn should_parse_participant_with_metadata() {
            let result = parse(
                r#"sequenceDiagram
participant User@{ "type": "actor" }
participant AuthService@{ "type": "control" }
User->>AuthService: Login"#,
            );
            assert!(result.is_ok());
            let db = result.unwrap();
            assert!(db.get_actors().contains_key("User"));
            assert!(db.get_actors().contains_key("AuthService"));
        }
    }

    mod arrow_types {
        use super::*;

        #[test]
        fn should_handle_solid_arrow() {
            let result = parse("sequenceDiagram\nAlice->>Bob:Hello");
            assert!(result.is_ok());
            let db = result.unwrap();
            assert_eq!(db.get_messages()[0].message_type, LineType::Solid);
        }

        #[test]
        fn should_handle_dotted_arrow() {
            let result = parse("sequenceDiagram\nAlice-->>Bob:Hello");
            assert!(result.is_ok());
            let db = result.unwrap();
            assert_eq!(db.get_messages()[0].message_type, LineType::Dotted);
        }

        #[test]
        fn should_handle_solid_open_arrow() {
            let result = parse("sequenceDiagram\nAlice->Bob:Hello");
            assert!(result.is_ok());
            let db = result.unwrap();
            assert_eq!(db.get_messages()[0].message_type, LineType::SolidOpen);
        }

        #[test]
        fn should_handle_dotted_open_arrow() {
            let result = parse("sequenceDiagram\nAlice-->Bob:Hello");
            assert!(result.is_ok());
            let db = result.unwrap();
            assert_eq!(db.get_messages()[0].message_type, LineType::DottedOpen);
        }

        #[test]
        fn should_handle_solid_cross() {
            let result = parse("sequenceDiagram\nAlice-xBob:Hello");
            assert!(result.is_ok());
            let db = result.unwrap();
            assert_eq!(db.get_messages()[0].message_type, LineType::SolidCross);
        }

        #[test]
        fn should_handle_dotted_cross() {
            let result = parse("sequenceDiagram\nAlice--xBob:Hello");
            assert!(result.is_ok());
            let db = result.unwrap();
            assert_eq!(db.get_messages()[0].message_type, LineType::DottedCross);
        }

        #[test]
        fn should_handle_solid_point() {
            let result = parse("sequenceDiagram\nAlice-)Bob:Hello");
            assert!(result.is_ok());
            let db = result.unwrap();
            assert_eq!(db.get_messages()[0].message_type, LineType::SolidPoint);
        }

        #[test]
        fn should_handle_dotted_point() {
            let result = parse("sequenceDiagram\nAlice--)Bob:Hello");
            assert!(result.is_ok());
            let db = result.unwrap();
            assert_eq!(db.get_messages()[0].message_type, LineType::DottedPoint);
        }

        #[test]
        fn should_handle_bidirectional_solid() {
            let result = parse("sequenceDiagram\nAlice<<->>Bob:Hello");
            assert!(result.is_ok());
            let db = result.unwrap();
            assert_eq!(
                db.get_messages()[0].message_type,
                LineType::BidirectionalSolid
            );
        }

        #[test]
        fn should_handle_bidirectional_dotted() {
            let result = parse("sequenceDiagram\nAlice<<-->>Bob:Hello");
            assert!(result.is_ok());
            let db = result.unwrap();
            assert_eq!(
                db.get_messages()[0].message_type,
                LineType::BidirectionalDotted
            );
        }

        #[test]
        fn should_handle_half_arrow_solid_bottom() {
            // Half arrow: -|/ (solid half arrow bottom)
            let result = parse("sequenceDiagram\nAlice-|/Bob:Hello");
            assert!(result.is_ok(), "Failed to parse: {:?}", result.err());
        }

        #[test]
        fn should_handle_half_arrow_solid_top() {
            // Half arrow: -|\ (solid half arrow top)
            let result = parse(
                r"sequenceDiagram
Alice-|\Bob:Hello",
            );
            assert!(result.is_ok(), "Failed to parse: {:?}", result.err());
        }

        #[test]
        fn should_handle_half_arrow_stick_bottom() {
            // Half arrow: -// (stick half arrow bottom)
            let result = parse("sequenceDiagram\nAlice-//Bob:Hello");
            assert!(result.is_ok(), "Failed to parse: {:?}", result.err());
        }

        #[test]
        fn should_handle_half_arrow_stick_top() {
            // Half arrow: -\\ (stick half arrow top - double backslash)
            let result = parse(
                r"sequenceDiagram
Alice-\\Bob:Hello",
            );
            assert!(result.is_ok(), "Failed to parse: {:?}", result.err());
        }

        #[test]
        fn should_handle_half_arrow_with_spaces() {
            // Half arrow with spaces around it - like in mermaid tests
            let result = parse(
                r"sequenceDiagram
      Alice -|\  John: Hello John, how are you?",
            );
            assert!(result.is_ok(), "Failed to parse: {:?}", result.err());
        }

        #[test]
        fn should_handle_half_arrow_reverse() {
            // Half arrow reverse: \|- (solid half arrow bottom reverse)
            let result = parse(
                r"sequenceDiagram
Alice\|-Bob:Hello",
            );
            assert!(result.is_ok(), "Failed to parse: {:?}", result.err());
        }

        #[test]
        fn should_handle_half_arrow_reverse_with_spaces() {
            // Half arrow reverse with spaces
            let result = parse(
                r"sequenceDiagram
        Alice \|- John: Hello",
            );
            assert!(result.is_ok(), "Failed to parse: {:?}", result.err());
        }
    }

    mod notes {
        use super::*;

        #[test]
        fn should_parse_note_right_of() {
            let result = parse("sequenceDiagram\nnote right of Alice: This is a note");
            assert!(result.is_ok());
            let db = result.unwrap();
            let notes = db.get_notes();
            assert_eq!(notes.len(), 1);
            assert_eq!(notes[0].placement, Placement::RightOf);
            assert_eq!(notes[0].message, "This is a note");
        }

        #[test]
        fn should_parse_note_left_of() {
            let result = parse("sequenceDiagram\nnote left of Bob: Note text");
            assert!(result.is_ok());
            let db = result.unwrap();
            let notes = db.get_notes();
            assert_eq!(notes[0].placement, Placement::LeftOf);
        }

        #[test]
        fn should_parse_note_over() {
            let result = parse("sequenceDiagram\nnote over Alice: Over note");
            assert!(result.is_ok());
            let db = result.unwrap();
            let notes = db.get_notes();
            assert_eq!(notes[0].placement, Placement::Over);
        }

        #[test]
        fn should_parse_note_over_multiple_actors() {
            let result = parse("sequenceDiagram\nNote over A,B: Spanning note");
            assert!(result.is_ok());
            let db = result.unwrap();
            let notes = db.get_notes();
            assert_eq!(notes.len(), 1);
            assert_eq!(notes[0].placement, Placement::Over);
            assert_eq!(notes[0].message, "Spanning note");
        }

        #[test]
        fn should_parse_note_with_capital_n() {
            let result = parse("sequenceDiagram\nNote right of Alice: Capital note");
            assert!(result.is_ok());
            let db = result.unwrap();
            let notes = db.get_notes();
            assert_eq!(notes[0].placement, Placement::RightOf);
        }
    }

    mod control_structures {
        use super::*;

        #[test]
        fn should_parse_loop() {
            let result = parse(
                "sequenceDiagram
loop Every minute
Alice->Bob: Ping
end",
            );
            assert!(result.is_ok());
            let db = result.unwrap();
            let messages = db.get_messages();
            assert_eq!(messages[0].message_type, LineType::LoopStart);
            assert_eq!(messages[0].message, "Every minute");
            assert_eq!(messages[2].message_type, LineType::LoopEnd);
        }

        #[test]
        fn should_parse_alt() {
            let result = parse(
                "sequenceDiagram
alt is true
Alice->Bob: Yes
else is false
Alice->Bob: No
end",
            );
            assert!(result.is_ok());
            let db = result.unwrap();
            let messages = db.get_messages();
            assert_eq!(messages[0].message_type, LineType::AltStart);
            assert_eq!(messages[2].message_type, LineType::AltElse);
            assert_eq!(messages[4].message_type, LineType::AltEnd);
        }

        #[test]
        fn should_parse_opt() {
            let result = parse(
                "sequenceDiagram
opt Extra
Alice->Bob: Maybe
end",
            );
            assert!(result.is_ok());
            let db = result.unwrap();
            let messages = db.get_messages();
            assert_eq!(messages[0].message_type, LineType::OptStart);
            assert_eq!(messages[2].message_type, LineType::OptEnd);
        }

        #[test]
        fn should_parse_par() {
            let result = parse(
                "sequenceDiagram
par Action 1
Alice->Bob: Hello
and Action 2
Alice->Carol: Hi
end",
            );
            assert!(result.is_ok());
            let db = result.unwrap();
            let messages = db.get_messages();
            assert_eq!(messages[0].message_type, LineType::ParStart);
            assert_eq!(messages[2].message_type, LineType::ParAnd);
            assert_eq!(messages[4].message_type, LineType::ParEnd);
        }

        #[test]
        fn should_parse_critical() {
            let result = parse(
                "sequenceDiagram
critical Do something
Alice->Bob: Action
option Handle error
Alice->Bob: Error
end",
            );
            assert!(result.is_ok());
            let db = result.unwrap();
            let messages = db.get_messages();
            assert_eq!(messages[0].message_type, LineType::CriticalStart);
            assert_eq!(messages[2].message_type, LineType::CriticalOption);
            assert_eq!(messages[4].message_type, LineType::CriticalEnd);
        }

        #[test]
        fn should_parse_break() {
            let result = parse(
                "sequenceDiagram
break when condition fails
Alice->Bob: Stop
end",
            );
            assert!(result.is_ok());
            let db = result.unwrap();
            let messages = db.get_messages();
            assert_eq!(messages[0].message_type, LineType::BreakStart);
            assert_eq!(messages[2].message_type, LineType::BreakEnd);
        }

        #[test]
        fn should_parse_rect() {
            let result = parse(
                "sequenceDiagram
rect rgb(200,200,200)
Alice->Bob: Hello
end",
            );
            assert!(result.is_ok());
        }
    }

    mod activation {
        use super::*;

        #[test]
        fn should_parse_explicit_activation() {
            let result = parse(
                "sequenceDiagram
Alice->>Bob: Hello
activate Bob
Bob-->>Alice: Hi
deactivate Bob",
            );
            assert!(result.is_ok());
            let db = result.unwrap();
            let messages = db.get_messages();
            assert!(messages.len() >= 4);
            assert_eq!(messages[1].message_type, LineType::ActiveStart);
            assert_eq!(messages[3].message_type, LineType::ActiveEnd);
        }

        #[test]
        fn should_parse_shorthand_activation() {
            let result = parse("sequenceDiagram\nAlice-->>+Bob: Hello");
            assert!(result.is_ok());
            let db = result.unwrap();
            let messages = db.get_messages();
            assert!(messages[0].activate);
        }
    }

    mod autonumber {
        use super::*;

        #[test]
        fn should_enable_autonumber() {
            let result = parse(
                "sequenceDiagram
autonumber
Alice->Bob: Hello",
            );
            assert!(result.is_ok());
            let db = result.unwrap();
            assert!(db.sequence_numbers_enabled());
        }

        #[test]
        fn should_disable_autonumber() {
            let result = parse(
                "sequenceDiagram
autonumber off
Alice->Bob: Hello",
            );
            assert!(result.is_ok());
            let db = result.unwrap();
            assert!(!db.sequence_numbers_enabled());
        }
    }

    mod accessibility {
        use super::*;

        #[test]
        fn should_parse_title() {
            let result = parse(
                "sequenceDiagram
title: Diagram Title
Alice->Bob: Hello",
            );
            assert!(result.is_ok());
            let db = result.unwrap();
            assert_eq!(db.diagram_title, "Diagram Title");
        }

        #[test]
        fn should_parse_title_without_colon() {
            let result = parse(
                "sequenceDiagram
title Diagram Title
Alice->Bob: Hello",
            );
            assert!(result.is_ok());
            let db = result.unwrap();
            assert_eq!(db.diagram_title, "Diagram Title");
        }

        #[test]
        fn should_parse_acc_title_and_descr() {
            let result = parse(
                "sequenceDiagram
accTitle: My Title
accDescr: My Description
Alice->Bob: Hello",
            );
            assert!(result.is_ok());
            let db = result.unwrap();
            assert_eq!(db.acc_title, "My Title");
            assert_eq!(db.acc_descr, "My Description");
        }

        #[test]
        fn should_parse_multiline_acc_descr() {
            let result = parse(
                "sequenceDiagram
accDescr {
This is multi
line description
}
Alice->Bob: Hello",
            );
            assert!(result.is_ok());
            let db = result.unwrap();
            assert_eq!(db.acc_descr, "This is multi\nline description");
        }
    }

    mod comments {
        use super::*;

        #[test]
        fn should_handle_comments() {
            let result = parse(
                "sequenceDiagram
Alice->Bob: Hello
%% This is a comment
Bob-->Alice: Hi",
            );
            assert!(result.is_ok());
            let db = result.unwrap();
            assert_eq!(db.get_messages().len(), 2);
        }

        #[test]
        fn should_handle_comments_before_diagram() {
            let result = parse(
                "%% Header comment
sequenceDiagram
Alice->Bob: Hello",
            );
            assert!(result.is_ok());
        }
    }

    mod create_destroy {
        use super::*;

        #[test]
        fn should_parse_create() {
            let result = parse(
                "sequenceDiagram
create participant Alice
Bob->>Alice: Hello",
            );
            assert!(result.is_ok());
            let db = result.unwrap();
            assert!(db.is_created("Alice"));
        }

        #[test]
        fn should_parse_destroy() {
            let result = parse(
                "sequenceDiagram
participant Alice
destroy Alice",
            );
            assert!(result.is_ok());
            let db = result.unwrap();
            assert!(db.is_destroyed("Alice"));
        }
    }

    mod semicolons {
        use super::*;

        #[test]
        fn should_handle_semicolons() {
            // Note: We no longer treat ; as a statement separator in message text
            // to allow entities like #lt; and #gt;. Use newlines to separate statements.
            let result = parse("sequenceDiagram\nAlice->Bob: Hello\nBob-->Alice: Hi");
            assert!(result.is_ok());
            let db = result.unwrap();
            assert_eq!(db.get_messages().len(), 2);
        }

        #[test]
        fn should_allow_semicolons_in_message_text() {
            // Semicolons in message text are allowed (for entities like #lt;)
            let result = parse("sequenceDiagram\nAlice->Bob: Hello #lt;World#gt;");
            assert!(result.is_ok());
            let db = result.unwrap();
            assert_eq!(db.get_messages().len(), 1);
            assert_eq!(db.get_messages()[0].message, "Hello #lt;World#gt;");
        }
    }
}
