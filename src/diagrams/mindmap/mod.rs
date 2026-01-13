//! Mindmap diagram support

mod parser;
mod types;

pub use parser::parse;
pub use types::{MindmapDb, MindmapNode, NodeType};

#[cfg(test)]
mod tests {
    use super::*;

    fn setup() -> MindmapDb {
        MindmapDb::new()
    }

    mod hierarchy_tests {
        use super::*;

        #[test]
        fn mmp_1_should_handle_simple_root_definition() {
            let mut db = setup();
            parse_into("mindmap\n    root", &mut db).unwrap();

            let mm = db.get_mindmap().unwrap();
            assert_eq!(mm.descr, "root");
        }

        #[test]
        fn mmp_2_should_handle_hierarchical_mindmap_definition() {
            let mut db = setup();
            parse_into("mindmap\n    root\n      child1\n      child2\n ", &mut db).unwrap();

            let mm = db.get_mindmap().unwrap();
            assert_eq!(mm.descr, "root");
            assert_eq!(mm.children.len(), 2);
            assert_eq!(mm.children[0].descr, "child1");
            assert_eq!(mm.children[1].descr, "child2");
        }

        #[test]
        fn mmp_3_should_handle_root_with_shape_and_no_id() {
            let mut db = setup();
            parse_into("mindmap\n    (root)", &mut db).unwrap();

            let mm = db.get_mindmap().unwrap();
            assert_eq!(mm.descr, "root");
        }

        #[test]
        fn mmp_4_should_handle_deeper_hierarchical_definition() {
            let mut db = setup();
            parse_into(
                "mindmap\n    root\n      child1\n        leaf1\n      child2",
                &mut db,
            )
            .unwrap();

            let mm = db.get_mindmap().unwrap();
            assert_eq!(mm.descr, "root");
            assert_eq!(mm.children.len(), 2);
            assert_eq!(mm.children[0].descr, "child1");
            assert_eq!(mm.children[0].children[0].descr, "leaf1");
            assert_eq!(mm.children[1].descr, "child2");
        }

        #[test]
        fn mmp_5_multiple_roots_are_illegal() {
            let mut db = setup();
            let result = parse_into("mindmap\n    root\n    fakeRoot", &mut db);

            assert!(result.is_err());
            let err = result.unwrap_err().to_string();
            assert!(err.contains("only one root"));
        }

        #[test]
        fn mmp_6_real_root_in_wrong_place() {
            let mut db = setup();
            let result = parse_into(
                "mindmap\n          root\n        fakeRoot\n    realRootWrongPlace",
                &mut db,
            );

            assert!(result.is_err());
        }
    }

    mod nodes_tests {
        use super::*;

        #[test]
        fn mmp_7_should_handle_id_and_type_for_node_definition() {
            let mut db = setup();
            parse_into("mindmap\n    root[The root]\n      ", &mut db).unwrap();

            let mm = db.get_mindmap().unwrap();
            assert_eq!(mm.node_id.as_deref(), Some("root"));
            assert_eq!(mm.descr, "The root");
            assert_eq!(mm.node_type, NodeType::Rect);
        }

        #[test]
        fn mmp_8_should_handle_child_with_id_and_rounded_rect() {
            let mut db = setup();
            parse_into("mindmap\n    root\n      theId(child1)", &mut db).unwrap();

            let mm = db.get_mindmap().unwrap();
            assert_eq!(mm.descr, "root");
            assert_eq!(mm.children.len(), 1);

            let child = &mm.children[0];
            assert_eq!(child.descr, "child1");
            assert_eq!(child.node_id.as_deref(), Some("theId"));
            assert_eq!(child.node_type, NodeType::RoundedRect);
        }

        #[test]
        fn mmp_9_should_handle_node_at_column_zero() {
            let mut db = setup();
            parse_into("mindmap\nroot\n      theId(child1)", &mut db).unwrap();

            let mm = db.get_mindmap().unwrap();
            assert_eq!(mm.descr, "root");
            assert_eq!(mm.children.len(), 1);

            let child = &mm.children[0];
            assert_eq!(child.descr, "child1");
            assert_eq!(child.node_id.as_deref(), Some("theId"));
            assert_eq!(child.node_type, NodeType::RoundedRect);
        }

        #[test]
        fn mmp_10_multiple_types_circle() {
            let mut db = setup();
            parse_into("mindmap\n root((the root))\n ", &mut db).unwrap();

            let mm = db.get_mindmap().unwrap();
            assert_eq!(mm.descr, "the root");
            assert_eq!(mm.children.len(), 0);
            assert_eq!(mm.node_type, NodeType::Circle);
        }

        #[test]
        fn mmp_11_multiple_types_cloud() {
            let mut db = setup();
            parse_into("mindmap\n root)the root(\n", &mut db).unwrap();

            let mm = db.get_mindmap().unwrap();
            assert_eq!(mm.descr, "the root");
            assert_eq!(mm.children.len(), 0);
            assert_eq!(mm.node_type, NodeType::Cloud);
        }

        #[test]
        fn mmp_12_multiple_types_bang() {
            let mut db = setup();
            parse_into("mindmap\n root))the root((\n", &mut db).unwrap();

            let mm = db.get_mindmap().unwrap();
            assert_eq!(mm.descr, "the root");
            assert_eq!(mm.children.len(), 0);
            assert_eq!(mm.node_type, NodeType::Bang);
        }

        #[test]
        fn mmp_12a_multiple_types_hexagon() {
            let mut db = setup();
            parse_into("mindmap\n root{{the root}}\n", &mut db).unwrap();

            let mm = db.get_mindmap().unwrap();
            assert_eq!(mm.node_type, NodeType::Hexagon);
            assert_eq!(mm.descr, "the root");
            assert_eq!(mm.children.len(), 0);
        }
    }

    mod decorations_tests {
        use super::*;

        #[test]
        fn mmp_13_should_set_icon_for_node() {
            let mut db = setup();
            parse_into(
                "mindmap\n    root[The root]\n    ::icon(bomb)\n    ",
                &mut db,
            )
            .unwrap();

            let mm = db.get_mindmap().unwrap();
            assert_eq!(mm.node_id.as_deref(), Some("root"));
            assert_eq!(mm.descr, "The root");
            assert_eq!(mm.node_type, NodeType::Rect);
            assert_eq!(mm.icon.as_deref(), Some("bomb"));
        }

        #[test]
        fn mmp_14_should_set_classes_for_node() {
            let mut db = setup();
            parse_into("mindmap\n    root[The root]\n    :::m-4 p-8\n    ", &mut db).unwrap();

            let mm = db.get_mindmap().unwrap();
            assert_eq!(mm.node_id.as_deref(), Some("root"));
            assert_eq!(mm.descr, "The root");
            assert_eq!(mm.node_type, NodeType::Rect);
            assert_eq!(mm.class.as_deref(), Some("m-4 p-8"));
        }

        #[test]
        fn mmp_15_should_set_both_classes_and_icon() {
            let mut db = setup();
            parse_into(
                "mindmap\n    root[The root]\n    :::m-4 p-8\n    ::icon(bomb)\n    ",
                &mut db,
            )
            .unwrap();

            let mm = db.get_mindmap().unwrap();
            assert_eq!(mm.node_id.as_deref(), Some("root"));
            assert_eq!(mm.descr, "The root");
            assert_eq!(mm.node_type, NodeType::Rect);
            assert_eq!(mm.class.as_deref(), Some("m-4 p-8"));
            assert_eq!(mm.icon.as_deref(), Some("bomb"));
        }

        #[test]
        fn mmp_16_should_set_icon_then_classes() {
            let mut db = setup();
            parse_into(
                "mindmap\n    root[The root]\n    ::icon(bomb)\n    :::m-4 p-8\n    ",
                &mut db,
            )
            .unwrap();

            let mm = db.get_mindmap().unwrap();
            assert_eq!(mm.node_id.as_deref(), Some("root"));
            assert_eq!(mm.descr, "The root");
            assert_eq!(mm.node_type, NodeType::Rect);
            assert_eq!(mm.class.as_deref(), Some("m-4 p-8"));
            assert_eq!(mm.icon.as_deref(), Some("bomb"));
        }
    }

    mod descriptions_tests {
        use super::*;

        #[test]
        fn mmp_17_should_handle_special_chars_in_descriptions() {
            let mut db = setup();
            parse_into("mindmap\n    root[\"String containing []\"]\n", &mut db).unwrap();

            let mm = db.get_mindmap().unwrap();
            assert_eq!(mm.node_id.as_deref(), Some("root"));
            assert_eq!(mm.descr, "String containing []");
        }

        #[test]
        fn mmp_18_should_handle_special_chars_in_children() {
            let mut db = setup();
            parse_into(
                "mindmap\n    root[\"String containing []\"]\n      child1[\"String containing ()\"]\n",
                &mut db,
            )
            .unwrap();

            let mm = db.get_mindmap().unwrap();
            assert_eq!(mm.node_id.as_deref(), Some("root"));
            assert_eq!(mm.descr, "String containing []");
            assert_eq!(mm.children.len(), 1);
            assert_eq!(mm.children[0].descr, "String containing ()");
        }

        #[test]
        fn mmp_19_child_after_class_assignment() {
            let mut db = setup();
            parse_into(
                "mindmap\n  root(Root)\n    Child(Child)\n    :::hot\n      a(a)\n      b[New Stuff]",
                &mut db,
            )
            .unwrap();

            let mm = db.get_mindmap().unwrap();
            assert_eq!(mm.node_id.as_deref(), Some("root"));
            assert_eq!(mm.descr, "Root");
            assert_eq!(mm.children.len(), 1);

            let child = &mm.children[0];
            assert_eq!(child.node_id.as_deref(), Some("Child"));
            assert_eq!(child.children[0].node_id.as_deref(), Some("a"));
            assert_eq!(child.children.len(), 2);
            assert_eq!(child.children[1].node_id.as_deref(), Some("b"));
        }
    }

    mod misc_tests {
        use super::*;

        #[test]
        fn mmp_20_should_handle_empty_rows() {
            let mut db = setup();
            parse_into(
                "mindmap\n  root(Root)\n    Child(Child)\n      a(a)\n\n      b[New Stuff]",
                &mut db,
            )
            .unwrap();

            let mm = db.get_mindmap().unwrap();
            assert_eq!(mm.node_id.as_deref(), Some("root"));
            assert_eq!(mm.descr, "Root");
            assert_eq!(mm.children.len(), 1);

            let child = &mm.children[0];
            assert_eq!(child.node_id.as_deref(), Some("Child"));
            assert_eq!(child.children[0].node_id.as_deref(), Some("a"));
            assert_eq!(child.children.len(), 2);
            assert_eq!(child.children[1].node_id.as_deref(), Some("b"));
        }

        #[test]
        fn mmp_21_should_handle_comments() {
            let mut db = setup();
            parse_into(
                "mindmap\n  root(Root)\n    Child(Child)\n      a(a)\n\n      %% This is a comment\n      b[New Stuff]",
                &mut db,
            )
            .unwrap();

            let mm = db.get_mindmap().unwrap();
            assert_eq!(mm.node_id.as_deref(), Some("root"));
            assert_eq!(mm.children.len(), 1);

            let child = &mm.children[0];
            assert_eq!(child.children.len(), 2);
            assert_eq!(child.children[1].node_id.as_deref(), Some("b"));
        }

        #[test]
        fn mmp_22_should_handle_inline_comments() {
            let mut db = setup();
            parse_into(
                "mindmap\n  root(Root)\n    Child(Child)\n      a(a) %% This is a comment\n      b[New Stuff]",
                &mut db,
            )
            .unwrap();

            let mm = db.get_mindmap().unwrap();
            assert_eq!(mm.children.len(), 1);
            let child = &mm.children[0];
            assert_eq!(child.children.len(), 2);
            assert_eq!(child.children[0].node_id.as_deref(), Some("a"));
            assert_eq!(child.children[1].node_id.as_deref(), Some("b"));
        }

        #[test]
        fn mmp_23_rows_with_only_spaces() {
            let mut db = setup();
            parse_into("mindmap\nroot\n A\n \n\n B", &mut db).unwrap();

            let mm = db.get_mindmap().unwrap();
            assert_eq!(mm.node_id.as_deref(), Some("root"));
            assert_eq!(mm.children.len(), 2);
            assert_eq!(mm.children[0].node_id.as_deref(), Some("A"));
            assert_eq!(mm.children[1].node_id.as_deref(), Some("B"));
        }

        #[test]
        fn mmp_24_handle_rows_above_declarations() {
            let mut db = setup();
            parse_into("\n \nmindmap\nroot\n A\n \n\n B", &mut db).unwrap();

            let mm = db.get_mindmap().unwrap();
            assert_eq!(mm.node_id.as_deref(), Some("root"));
            assert_eq!(mm.children.len(), 2);
            assert_eq!(mm.children[0].node_id.as_deref(), Some("A"));
            assert_eq!(mm.children[1].node_id.as_deref(), Some("B"));
        }

        #[test]
        fn mmp_25_handle_rows_above_declarations_no_space() {
            let mut db = setup();
            parse_into("\n\n\nmindmap\nroot\n A\n \n\n B", &mut db).unwrap();

            let mm = db.get_mindmap().unwrap();
            assert_eq!(mm.node_id.as_deref(), Some("root"));
            assert_eq!(mm.children.len(), 2);
            assert_eq!(mm.children[0].node_id.as_deref(), Some("A"));
            assert_eq!(mm.children[1].node_id.as_deref(), Some("B"));
        }
    }

    fn parse_into(input: &str, db: &mut MindmapDb) -> crate::error::Result<()> {
        parser::parse_into(input, db)
    }
}
