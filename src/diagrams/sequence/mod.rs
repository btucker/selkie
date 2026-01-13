//! Sequence diagram support
//!
//! Sequence diagrams show interactions between actors/participants
//! with messages, notes, and control structures (loops, alternatives, etc.)

pub mod parser;
mod types;

pub use parser::parse;
pub use types::*;

#[cfg(test)]
mod tests {
    use super::*;

    mod basic_tests {
        use super::*;

        #[test]
        fn should_initialize_empty() {
            let db = SequenceDb::new();
            assert!(db.get_actors().is_empty());
            assert!(db.get_messages().is_empty());
            assert!(db.get_notes().is_empty());
            assert!(!db.sequence_numbers_enabled());
        }

        #[test]
        fn should_add_actor() {
            let mut db = SequenceDb::new();
            db.add_actor("Alice", None, ParticipantType::Participant);

            assert_eq!(db.get_actors().len(), 1);
            assert!(db.get_actors().contains_key("Alice"));
        }

        #[test]
        fn should_add_actor_with_description() {
            let mut db = SequenceDb::new();
            db.add_actor("A", Some("Alice"), ParticipantType::Participant);

            let actor = db.get_actors().get("A").unwrap();
            assert_eq!(actor.description, "Alice");
        }

        #[test]
        fn should_not_duplicate_actors() {
            let mut db = SequenceDb::new();
            db.add_actor("Alice", None, ParticipantType::Participant);
            db.add_actor("Alice", None, ParticipantType::Participant);

            assert_eq!(db.get_actors().len(), 1);
        }

        #[test]
        fn should_link_actors() {
            let mut db = SequenceDb::new();
            db.add_actor("Alice", None, ParticipantType::Participant);
            db.add_actor("Bob", None, ParticipantType::Participant);

            let alice = db.get_actors().get("Alice").unwrap();
            let bob = db.get_actors().get("Bob").unwrap();

            assert_eq!(alice.next_actor, Some("Bob".to_string()));
            assert_eq!(bob.prev_actor, Some("Alice".to_string()));
        }

        #[test]
        fn should_track_actor_order() {
            let mut db = SequenceDb::new();
            db.add_actor("Alice", None, ParticipantType::Participant);
            db.add_actor("Bob", None, ParticipantType::Participant);
            db.add_actor("Charlie", None, ParticipantType::Participant);

            let ordered = db.get_actors_in_order();
            assert_eq!(ordered.len(), 3);
            assert_eq!(ordered[0].name, "Alice");
            assert_eq!(ordered[1].name, "Bob");
            assert_eq!(ordered[2].name, "Charlie");
        }
    }

    mod message_tests {
        use super::*;

        #[test]
        fn should_add_message() {
            let mut db = SequenceDb::new();
            db.add_message("Alice", "Bob", "Hello!", LineType::Solid, false);

            let messages = db.get_messages();
            assert_eq!(messages.len(), 1);
            assert_eq!(messages[0].from, Some("Alice".to_string()));
            assert_eq!(messages[0].to, Some("Bob".to_string()));
            assert_eq!(messages[0].message, "Hello!");
        }

        #[test]
        fn should_auto_create_actors_from_message() {
            let mut db = SequenceDb::new();
            db.add_message("Alice", "Bob", "Hello!", LineType::Solid, false);

            assert_eq!(db.get_actors().len(), 2);
            assert!(db.get_actors().contains_key("Alice"));
            assert!(db.get_actors().contains_key("Bob"));
        }

        #[test]
        fn should_track_message_type() {
            let mut db = SequenceDb::new();
            db.add_message("Alice", "Bob", "Sync", LineType::Solid, false);
            db.add_message("Bob", "Alice", "Async", LineType::Dotted, false);

            let messages = db.get_messages();
            assert_eq!(messages[0].message_type, LineType::Solid);
            assert_eq!(messages[1].message_type, LineType::Dotted);
        }

        #[test]
        fn should_increment_message_ids() {
            let mut db = SequenceDb::new();
            db.add_message("Alice", "Bob", "First", LineType::Solid, false);
            db.add_message("Bob", "Alice", "Second", LineType::Dotted, false);
            db.add_message("Alice", "Bob", "Third", LineType::SolidOpen, false);

            let messages = db.get_messages();
            assert_eq!(messages[0].id, "0");
            assert_eq!(messages[1].id, "1");
            assert_eq!(messages[2].id, "2");
        }

        #[test]
        fn should_handle_activation() {
            let mut db = SequenceDb::new();
            db.add_message("Alice", "Bob", "Activate", LineType::Solid, true);

            let messages = db.get_messages();
            assert!(messages[0].activate);
        }
    }

    mod note_tests {
        use super::*;

        #[test]
        fn should_add_note() {
            let mut db = SequenceDb::new();
            db.add_actor("Alice", None, ParticipantType::Participant);
            db.add_note("Alice", Placement::RightOf, "This is a note", None);

            let notes = db.get_notes();
            assert_eq!(notes.len(), 1);
            assert_eq!(notes[0].actor, "Alice");
            assert_eq!(notes[0].actor_to, None);
            assert_eq!(notes[0].message, "This is a note");
            assert_eq!(notes[0].placement, Placement::RightOf);
        }

        #[test]
        fn should_parse_placement() {
            assert_eq!(Placement::from_str("leftof"), Placement::LeftOf);
            assert_eq!(Placement::from_str("left of"), Placement::LeftOf);
            assert_eq!(Placement::from_str("rightof"), Placement::RightOf);
            assert_eq!(Placement::from_str("right of"), Placement::RightOf);
            assert_eq!(Placement::from_str("over"), Placement::Over);
        }
    }

    mod autonumber_tests {
        use super::*;

        #[test]
        fn should_enable_autonumber() {
            let mut db = SequenceDb::new();
            db.set_autonumber(true, None, None);

            assert!(db.sequence_numbers_enabled());
        }

        #[test]
        fn should_disable_autonumber() {
            let mut db = SequenceDb::new();
            db.set_autonumber(true, None, None);
            db.set_autonumber(false, None, None);

            assert!(!db.sequence_numbers_enabled());
        }

        #[test]
        fn should_set_custom_start_and_step() {
            let mut db = SequenceDb::new();
            db.set_autonumber(true, Some(10), Some(5));

            assert!(db.sequence_numbers_enabled());
        }
    }

    mod participant_type_tests {
        use super::*;

        #[test]
        fn should_parse_participant_types() {
            assert_eq!(
                ParticipantType::from_str("participant"),
                ParticipantType::Participant
            );
            assert_eq!(ParticipantType::from_str("actor"), ParticipantType::Actor);
            assert_eq!(
                ParticipantType::from_str("boundary"),
                ParticipantType::Boundary
            );
            assert_eq!(
                ParticipantType::from_str("control"),
                ParticipantType::Control
            );
            assert_eq!(ParticipantType::from_str("entity"), ParticipantType::Entity);
            assert_eq!(
                ParticipantType::from_str("database"),
                ParticipantType::Database
            );
            assert_eq!(
                ParticipantType::from_str("collections"),
                ParticipantType::Collections
            );
            assert_eq!(ParticipantType::from_str("queue"), ParticipantType::Queue);
        }

        #[test]
        fn should_output_participant_types() {
            assert_eq!(ParticipantType::Participant.as_str(), "participant");
            assert_eq!(ParticipantType::Actor.as_str(), "actor");
            assert_eq!(ParticipantType::Database.as_str(), "database");
        }

        #[test]
        fn should_add_different_actor_types() {
            let mut db = SequenceDb::new();
            db.add_actor("Alice", None, ParticipantType::Actor);
            db.add_actor("DB", None, ParticipantType::Database);

            let alice = db.get_actors().get("Alice").unwrap();
            let db_actor = db.get_actors().get("DB").unwrap();

            assert_eq!(alice.actor_type, ParticipantType::Actor);
            assert_eq!(db_actor.actor_type, ParticipantType::Database);
        }
    }

    mod line_type_tests {
        use super::*;

        #[test]
        fn line_type_values() {
            assert_eq!(LineType::Solid.as_num(), 0);
            assert_eq!(LineType::Dotted.as_num(), 1);
            assert_eq!(LineType::SolidCross.as_num(), 3);
            assert_eq!(LineType::DottedCross.as_num(), 4);
            assert_eq!(LineType::SolidOpen.as_num(), 5);
            assert_eq!(LineType::DottedOpen.as_num(), 6);
            assert_eq!(LineType::BidirectionalSolid.as_num(), 33);
            assert_eq!(LineType::CentralConnectionDual.as_num(), 61);
        }
    }

    mod box_tests {
        use super::*;

        #[test]
        fn should_add_box() {
            let mut db = SequenceDb::new();
            db.add_box("Group 1", "#ffcc00");

            let boxes = db.get_boxes();
            assert_eq!(boxes.len(), 1);
            assert_eq!(boxes[0].name, "Group 1");
            assert_eq!(boxes[0].fill, "#ffcc00");
        }
    }

    mod lifecycle_tests {
        use super::*;

        #[test]
        fn should_create_actor() {
            let mut db = SequenceDb::new();
            db.create_actor("Alice", None, ParticipantType::Participant);

            assert!(db.get_actors().contains_key("Alice"));
            assert!(db.is_created("Alice"));
        }

        #[test]
        fn should_destroy_actor() {
            let mut db = SequenceDb::new();
            db.add_actor("Alice", None, ParticipantType::Participant);
            db.destroy_actor("Alice");

            assert!(db.is_destroyed("Alice"));
        }

        #[test]
        fn should_not_destroy_unknown_actor() {
            let mut db = SequenceDb::new();
            db.destroy_actor("Unknown");

            assert!(!db.is_destroyed("Unknown"));
        }
    }

    mod control_structure_tests {
        use super::*;

        #[test]
        fn should_add_loop() {
            let mut db = SequenceDb::new();
            db.add_signal(LineType::LoopStart, Some("for each item"));
            db.add_message("Alice", "Bob", "Process", LineType::Solid, false);
            db.add_signal(LineType::LoopEnd, None);

            let messages = db.get_messages();
            assert_eq!(messages.len(), 3);
            assert_eq!(messages[0].message_type, LineType::LoopStart);
            assert_eq!(messages[0].message, "for each item");
            assert_eq!(messages[2].message_type, LineType::LoopEnd);
        }

        #[test]
        fn should_add_alt() {
            let mut db = SequenceDb::new();
            db.add_signal(LineType::AltStart, Some("if condition"));
            db.add_message("Alice", "Bob", "Yes", LineType::Solid, false);
            db.add_signal(LineType::AltElse, Some("else"));
            db.add_message("Alice", "Bob", "No", LineType::Solid, false);
            db.add_signal(LineType::AltEnd, None);

            let messages = db.get_messages();
            assert_eq!(messages.len(), 5);
            assert_eq!(messages[0].message_type, LineType::AltStart);
            assert_eq!(messages[2].message_type, LineType::AltElse);
            assert_eq!(messages[4].message_type, LineType::AltEnd);
        }
    }

    mod clear_tests {
        use super::*;

        #[test]
        fn should_clear_state() {
            let mut db = SequenceDb::new();
            db.add_actor("Alice", None, ParticipantType::Participant);
            db.add_message("Alice", "Bob", "Hello", LineType::Solid, false);
            db.add_note("Alice", Placement::RightOf, "Note", None);
            db.set_autonumber(true, None, None);
            db.acc_title = "Title".to_string();

            db.clear();

            assert!(db.get_actors().is_empty());
            assert!(db.get_messages().is_empty());
            assert!(db.get_notes().is_empty());
            assert!(!db.sequence_numbers_enabled());
            assert!(db.acc_title.is_empty());
        }
    }
}
