//! Flowchart diagram support

mod parser;
mod types;

pub use parser::parse;
pub use types::{
    Direction, EdgeStroke, FlowClass, FlowEdge, FlowSubGraph, FlowText, FlowTextType, FlowVertex,
    FlowVertexType, FlowchartDb,
};

#[cfg(test)]
mod tests {
    use super::*;

    fn setup() -> FlowchartDb {
        FlowchartDb::new()
    }

    mod subgraphs_tests {
        use super::*;

        fn create_subgraphs() -> Vec<FlowSubGraph> {
            vec![
                FlowSubGraph {
                    id: "sg1".to_string(),
                    nodes: vec![
                        "a".to_string(),
                        "b".to_string(),
                        "c".to_string(),
                        "e".to_string(),
                    ],
                    ..Default::default()
                },
                FlowSubGraph {
                    id: "sg2".to_string(),
                    nodes: vec!["f".to_string(), "g".to_string(), "h".to_string()],
                    ..Default::default()
                },
                FlowSubGraph {
                    id: "sg3".to_string(),
                    nodes: vec!["i".to_string(), "j".to_string()],
                    ..Default::default()
                },
                FlowSubGraph {
                    id: "sg4".to_string(),
                    nodes: vec!["k".to_string()],
                    ..Default::default()
                },
            ]
        }

        #[test]
        fn should_return_true_when_node_exists_in_subgraph() {
            let db = setup();
            let subgraphs = create_subgraphs();

            assert!(db.exists(&subgraphs, "a"));
            assert!(db.exists(&subgraphs, "h"));
            assert!(db.exists(&subgraphs, "j"));
            assert!(db.exists(&subgraphs, "k"));
        }

        #[test]
        fn should_return_false_when_node_does_not_exist_in_subgraph() {
            let db = setup();
            let subgraphs = create_subgraphs();

            assert!(!db.exists(&subgraphs, "a2"));
            assert!(!db.exists(&subgraphs, "l"));
        }

        #[test]
        fn should_remove_ids_from_subgraph_that_already_exist_even_if_empty() {
            let db = setup();
            let subgraphs = create_subgraphs();

            let mut subgraph = FlowSubGraph {
                id: "test".to_string(),
                nodes: vec!["i".to_string(), "j".to_string()],
                ..Default::default()
            };

            db.make_uniq(&mut subgraph, &subgraphs);
            assert!(subgraph.nodes.is_empty());
        }

        #[test]
        fn should_remove_ids_from_subgraph_that_already_exist() {
            let db = setup();
            let subgraphs = create_subgraphs();

            let mut subgraph = FlowSubGraph {
                id: "test".to_string(),
                nodes: vec!["i".to_string(), "j".to_string(), "o".to_string()],
                ..Default::default()
            };

            db.make_uniq(&mut subgraph, &subgraphs);
            assert_eq!(subgraph.nodes, vec!["o"]);
        }

        #[test]
        fn should_not_remove_unique_ids() {
            let db = setup();
            let subgraphs = create_subgraphs();

            let mut subgraph = FlowSubGraph {
                id: "test".to_string(),
                nodes: vec!["q".to_string(), "r".to_string(), "s".to_string()],
                ..Default::default()
            };

            db.make_uniq(&mut subgraph, &subgraphs);
            assert_eq!(subgraph.nodes, vec!["q", "r", "s"]);
        }
    }

    mod add_class_tests {
        use super::*;

        #[test]
        fn should_detect_many_classes() {
            let mut db = setup();
            db.add_class("a,b", &["stroke-width: 8px".to_string()]);

            let classes = db.get_classes();

            assert!(classes.contains_key("a"));
            assert!(classes.contains_key("b"));
            assert_eq!(classes.get("a").unwrap().styles, vec!["stroke-width: 8px"]);
            assert_eq!(classes.get("b").unwrap().styles, vec!["stroke-width: 8px"]);
        }

        #[test]
        fn should_detect_single_class() {
            let mut db = setup();
            db.add_class("a", &["stroke-width: 8px".to_string()]);

            let classes = db.get_classes();

            assert!(classes.contains_key("a"));
            assert_eq!(classes.get("a").unwrap().styles, vec!["stroke-width: 8px"]);
        }
    }

    mod class_tests {
        use super::*;

        #[test]
        fn should_have_functions_used_in_flow_jison() {
            // This test verifies that FlowchartDb has all the methods that were used
            // in the original JISON parser. In Rust, we verify this at compile time
            // by ensuring these methods exist.
            let mut db = setup();

            // Test that all methods exist and are callable
            db.set_direction("TB");
            db.add_sub_graph(vec!["a".to_string()], "sg1", "Title", "TB");

            let text = types::FlowText {
                text: "Test".to_string(),
                text_type: types::FlowTextType::Text,
            };
            db.add_vertex("a", Some(text), None, vec![], vec![], None, None);
            db.add_link(&["a"], &["b"], None);
            db.set_class("a", "testClass");
            // destructLink is internal
            db.add_class("a", &[]);
            // setClickEvent requires browser environment - not needed in Rust
            // setTooltip requires browser environment - not needed in Rust
            // setLink requires browser environment - not needed in Rust
            db.update_link(&[0], &[]);
            db.update_link_interpolate(&["default".to_string()], "stepBefore");
        }
    }

    mod get_data_tests {
        use super::*;

        #[test]
        fn should_use_default_interpolate_for_edges() {
            let mut db = setup();

            let text_a = types::FlowText {
                text: "A".to_string(),
                text_type: types::FlowTextType::Text,
            };
            let text_b = types::FlowText {
                text: "B".to_string(),
                text_type: types::FlowTextType::Text,
            };

            db.add_vertex("A", Some(text_a), None, vec![], vec![], None, None);
            db.add_vertex("B", Some(text_b), None, vec![], vec![], None, None);
            db.add_link(&["A"], &["B"], None);
            db.update_link_interpolate(&["default".to_string()], "stepBefore");

            let data = db.get_data();
            assert_eq!(data.edges[0].interpolate.as_deref(), Some("stepBefore"));
        }

        #[test]
        fn should_prioritize_edge_specific_interpolate() {
            let mut db = setup();

            let text_a = types::FlowText {
                text: "A".to_string(),
                text_type: types::FlowTextType::Text,
            };
            let text_b = types::FlowText {
                text: "B".to_string(),
                text_type: types::FlowTextType::Text,
            };

            db.add_vertex("A", Some(text_a), None, vec![], vec![], None, None);
            db.add_vertex("B", Some(text_b), None, vec![], vec![], None, None);
            db.add_link(&["A"], &["B"], None);
            db.update_link_interpolate(&["default".to_string()], "stepBefore");
            db.update_link_interpolate(&["0".to_string()], "basis");

            let data = db.get_data();
            assert_eq!(data.edges[0].interpolate.as_deref(), Some("basis"));
        }
    }

    mod direction_tests {
        use super::*;

        #[test]
        fn should_set_direction_to_tb_when_td_is_set() {
            let mut db = setup();
            db.set_direction("TD");
            assert_eq!(db.get_direction(), "TB");
        }

        #[test]
        fn should_correctly_set_direction_with_leading_spaces() {
            let mut db = setup();
            db.set_direction(" TD");
            assert_eq!(db.get_direction(), "TB");
        }
    }
}
