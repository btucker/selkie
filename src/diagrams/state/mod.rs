//! State diagram support
//!
//! State diagrams model finite state machines with states, transitions,
//! notes, and nested composite states.

pub mod parser;
mod types;

pub use parser::parse;
pub use types::*;

#[cfg(test)]
mod tests {
    use super::*;

    mod state_tests {
        use super::*;

        #[test]
        fn should_initialize_empty() {
            let db = StateDb::new();
            assert!(db.get_states().is_empty());
            assert!(db.get_relations().is_empty());
            assert!(db.get_classes().is_empty());
            assert_eq!(db.get_direction(), Direction::TopToBottom);
        }

        #[test]
        fn should_add_state() {
            let mut db = StateDb::new();
            db.add_state("state1");

            assert_eq!(db.get_states().len(), 1);
            assert!(db.get_states().contains_key("state1"));
        }

        #[test]
        fn should_not_duplicate_states() {
            let mut db = StateDb::new();
            db.add_state("state1");
            db.add_state("state1");

            assert_eq!(db.get_states().len(), 1);
        }

        #[test]
        fn should_add_state_with_type() {
            let mut db = StateDb::new();
            db.add_state_with_type("s1", StateType::Start);
            db.add_state_with_type("s2", StateType::End);
            db.add_state_with_type("s3", StateType::Fork);

            assert_eq!(db.get_state("s1").unwrap().state_type, StateType::Start);
            assert_eq!(db.get_state("s2").unwrap().state_type, StateType::End);
            assert_eq!(db.get_state("s3").unwrap().state_type, StateType::Fork);
        }

        #[test]
        fn should_get_mutable_state() {
            let mut db = StateDb::new();
            db.add_state("state1");

            if let Some(state) = db.get_state_mut("state1") {
                state.state_type = StateType::Choice;
            }

            assert_eq!(
                db.get_state("state1").unwrap().state_type,
                StateType::Choice
            );
        }
    }

    mod description_tests {
        use super::*;

        #[test]
        fn should_add_description() {
            let mut db = StateDb::new();
            db.add_description("state1", "This is a description");

            let state = db.get_state("state1").unwrap();
            assert_eq!(state.descriptions.len(), 1);
            assert_eq!(state.descriptions[0], "This is a description");
        }

        #[test]
        fn should_add_multiple_descriptions() {
            let mut db = StateDb::new();
            db.add_description("state1", "First description");
            db.add_description("state1", "Second description");

            let state = db.get_state("state1").unwrap();
            assert_eq!(state.descriptions.len(), 2);
        }

        #[test]
        fn should_trim_colon_from_description() {
            let mut db = StateDb::new();
            db.add_description("state1", ":Description with colon");

            let state = db.get_state("state1").unwrap();
            assert_eq!(state.descriptions[0], "Description with colon");
        }

        #[test]
        fn should_sanitize_script_in_description() {
            let mut db = StateDb::new();
            db.add_description("state1", "Normal <script>alert('xss')</script> text");

            let state = db.get_state("state1").unwrap();
            assert!(!state.descriptions[0].contains("<script>"));
        }

        #[test]
        fn should_create_state_when_adding_description() {
            let mut db = StateDb::new();
            db.add_description("newState", "description");

            assert!(db.get_states().contains_key("newState"));
        }
    }

    mod relation_tests {
        use super::*;

        #[test]
        fn should_add_relation() {
            let mut db = StateDb::new();
            db.add_relation("state1", "state2", None);

            let relations = db.get_relations();
            assert_eq!(relations.len(), 1);
            assert_eq!(relations[0].state1, "state1");
            assert_eq!(relations[0].state2, "state2");
        }

        #[test]
        fn should_add_relation_with_description() {
            let mut db = StateDb::new();
            db.add_relation("state1", "state2", Some("transition label"));

            let relations = db.get_relations();
            assert_eq!(
                relations[0].description,
                Some("transition label".to_string())
            );
        }

        #[test]
        fn should_auto_create_states_from_relation() {
            let mut db = StateDb::new();
            db.add_relation("s1", "s2", None);

            assert!(db.get_states().contains_key("s1"));
            assert!(db.get_states().contains_key("s2"));
        }
    }

    mod style_class_tests {
        use super::*;

        #[test]
        fn should_add_style_class() {
            let mut db = StateDb::new();
            db.add_style_class("myClass", "fill:#f00,stroke:#000");

            let classes = db.get_classes();
            assert!(classes.contains_key("myClass"));
        }

        #[test]
        fn should_parse_comma_separated_styles() {
            let mut db = StateDb::new();
            db.add_style_class("myClass", "fill:#f00,stroke:#000,stroke-width:2px");

            let class = db.get_classes().get("myClass").unwrap();
            assert_eq!(class.styles.len(), 3);
            assert!(class.styles.contains(&"fill:#f00".to_string()));
            assert!(class.styles.contains(&"stroke:#000".to_string()));
        }

        #[test]
        fn should_parse_semicolon_separated_styles() {
            let mut db = StateDb::new();
            db.add_style_class("myClass", "fill:#f00;stroke:#000");

            let class = db.get_classes().get("myClass").unwrap();
            assert_eq!(class.styles.len(), 2);
        }

        #[test]
        fn should_apply_class_to_state() {
            let mut db = StateDb::new();
            db.add_style_class("highlight", "fill:#ff0");
            db.apply_class("state1", "highlight");

            let state = db.get_state("state1").unwrap();
            assert!(state.classes.contains(&"highlight".to_string()));
        }
    }

    mod direction_tests {
        use super::*;

        #[test]
        fn should_set_direction() {
            let mut db = StateDb::new();
            db.set_direction(Direction::LeftToRight);

            assert_eq!(db.get_direction(), Direction::LeftToRight);
        }

        #[test]
        fn should_parse_direction_from_string() {
            assert_eq!(Direction::from_str("TB"), Direction::TopToBottom);
            assert_eq!(Direction::from_str("BT"), Direction::BottomToTop);
            assert_eq!(Direction::from_str("LR"), Direction::LeftToRight);
            assert_eq!(Direction::from_str("RL"), Direction::RightToLeft);
        }

        #[test]
        fn should_output_direction_as_string() {
            assert_eq!(Direction::TopToBottom.as_str(), "TB");
            assert_eq!(Direction::BottomToTop.as_str(), "BT");
            assert_eq!(Direction::LeftToRight.as_str(), "LR");
            assert_eq!(Direction::RightToLeft.as_str(), "RL");
        }
    }

    mod state_type_tests {
        use super::*;

        #[test]
        fn should_parse_state_type_from_string() {
            assert_eq!(StateType::from_str("start"), StateType::Start);
            assert_eq!(StateType::from_str("end"), StateType::End);
            assert_eq!(StateType::from_str("fork"), StateType::Fork);
            assert_eq!(StateType::from_str("join"), StateType::Join);
            assert_eq!(StateType::from_str("choice"), StateType::Choice);
            assert_eq!(StateType::from_str("divider"), StateType::Divider);
            assert_eq!(StateType::from_str("unknown"), StateType::Default);
        }

        #[test]
        fn should_output_state_type_as_string() {
            assert_eq!(StateType::Start.as_str(), "start");
            assert_eq!(StateType::End.as_str(), "end");
            assert_eq!(StateType::Fork.as_str(), "fork");
            assert_eq!(StateType::Join.as_str(), "join");
        }

        #[test]
        fn should_be_case_insensitive() {
            assert_eq!(StateType::from_str("START"), StateType::Start);
            assert_eq!(StateType::from_str("Fork"), StateType::Fork);
            assert_eq!(StateType::from_str("CHOICE"), StateType::Choice);
        }
    }

    mod note_tests {
        use super::*;

        #[test]
        fn should_add_note_to_state() {
            let mut db = StateDb::new();
            db.add_note("state1", NotePosition::RightOf, "This is a note");

            let state = db.get_state("state1").unwrap();
            assert!(state.note.is_some());
            let note = state.note.as_ref().unwrap();
            assert_eq!(note.text, "This is a note");
            assert_eq!(note.position, NotePosition::RightOf);
        }

        #[test]
        fn should_parse_note_position() {
            assert_eq!(NotePosition::from_str("left of"), NotePosition::LeftOf);
            assert_eq!(NotePosition::from_str("leftof"), NotePosition::LeftOf);
            assert_eq!(NotePosition::from_str("right of"), NotePosition::RightOf);
        }
    }

    mod divider_tests {
        use super::*;

        #[test]
        fn should_generate_unique_divider_ids() {
            let mut db = StateDb::new();
            let id1 = db.get_divider_id();
            let id2 = db.get_divider_id();
            let id3 = db.get_divider_id();

            assert_eq!(id1, "divider-id-0");
            assert_eq!(id2, "divider-id-1");
            assert_eq!(id3, "divider-id-2");
        }
    }

    mod root_doc_tests {
        use super::*;

        #[test]
        fn should_set_and_get_root_doc() {
            let mut db = StateDb::new();
            let doc = vec![
                Statement::State(State::new("s1".to_string())),
                Statement::Direction(Direction::LeftToRight),
            ];
            db.set_root_doc(doc);

            assert_eq!(db.get_root_doc().len(), 2);
        }
    }

    mod clear_tests {
        use super::*;

        #[test]
        fn should_clear_all_state() {
            let mut db = StateDb::new();
            db.add_state("state1");
            db.add_relation("s1", "s2", None);
            db.add_style_class("cls", "fill:#f00");
            db.set_direction(Direction::LeftToRight);
            db.acc_title = "Title".to_string();

            db.clear();

            assert!(db.get_states().is_empty());
            assert!(db.get_relations().is_empty());
            assert!(db.get_classes().is_empty());
            assert_eq!(db.get_direction(), Direction::TopToBottom);
            assert!(db.acc_title.is_empty());
        }
    }

    mod trim_colon_tests {
        use super::*;

        #[test]
        fn should_trim_leading_colon() {
            assert_eq!(StateDb::trim_colon(":text"), "text");
        }

        #[test]
        fn should_not_trim_when_no_colon() {
            assert_eq!(StateDb::trim_colon("text"), "text");
        }

        #[test]
        fn should_only_trim_first_colon() {
            assert_eq!(StateDb::trim_colon(":text:with:colons"), "text:with:colons");
        }
    }
}
