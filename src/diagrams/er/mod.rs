//! Entity Relationship diagram support
//!
//! ER diagrams model database schemas with entities, attributes,
//! and relationships with cardinality and identification.

pub mod parser;
mod types;

pub use parser::parse;
pub use types::*;

#[cfg(test)]
mod tests {
    use super::*;

    mod entity_tests {
        use super::*;

        #[test]
        fn should_initialize_empty() {
            let db = ErDb::new();
            assert!(db.get_entities().is_empty());
            assert!(db.get_relationships().is_empty());
            assert!(db.get_classes().is_empty());
            assert_eq!(db.get_direction(), Direction::TopToBottom);
        }

        #[test]
        fn should_add_entity() {
            let mut db = ErDb::new();
            db.add_entity("Customer", None);

            assert_eq!(db.get_entities().len(), 1);
            assert!(db.get_entities().contains_key("Customer"));
        }

        #[test]
        fn should_create_entity_with_unique_id() {
            let mut db = ErDb::new();
            db.add_entity("Customer", None);
            db.add_entity("Order", None);

            let customer = db.get_entity("Customer").unwrap();
            let order = db.get_entity("Order").unwrap();

            assert!(customer.id.contains("Customer"));
            assert!(order.id.contains("Order"));
            assert_ne!(customer.id, order.id);
        }

        #[test]
        fn should_not_duplicate_entities() {
            let mut db = ErDb::new();
            db.add_entity("Customer", None);
            db.add_entity("Customer", None);

            assert_eq!(db.get_entities().len(), 1);
        }

        #[test]
        fn should_add_entity_with_alias() {
            let mut db = ErDb::new();
            db.add_entity("c", Some("Customer Account"));

            let entity = db.get_entity("c").unwrap();
            assert_eq!(entity.alias, "Customer Account");
        }

        #[test]
        fn should_update_alias_if_empty() {
            let mut db = ErDb::new();
            db.add_entity("c", None);
            db.add_entity("c", Some("Customer"));

            let entity = db.get_entity("c").unwrap();
            assert_eq!(entity.alias, "Customer");
        }

        #[test]
        fn should_not_update_alias_if_already_set() {
            let mut db = ErDb::new();
            db.add_entity("c", Some("Customer"));
            db.add_entity("c", Some("Different"));

            let entity = db.get_entity("c").unwrap();
            assert_eq!(entity.alias, "Customer");
        }

        #[test]
        fn should_get_mutable_entity() {
            let mut db = ErDb::new();
            db.add_entity("Customer", None);

            if let Some(entity) = db.get_entity_mut("Customer") {
                entity.css_classes = "highlight".to_string();
            }

            assert_eq!(db.get_entity("Customer").unwrap().css_classes, "highlight");
        }
    }

    mod attribute_tests {
        use super::*;

        #[test]
        fn should_add_attributes_to_entity() {
            let mut db = ErDb::new();
            db.add_entity("Book", None);
            db.add_attributes(
                "Book",
                vec![
                    Attribute::new("string".to_string(), "title".to_string()),
                    Attribute::new("float".to_string(), "price".to_string()),
                ],
            );

            let entity = db.get_entity("Book").unwrap();
            assert_eq!(entity.attributes.len(), 2);
        }

        #[test]
        fn should_create_entity_when_adding_attributes() {
            let mut db = ErDb::new();
            db.add_attributes(
                "Book",
                vec![Attribute::new("string".to_string(), "title".to_string())],
            );

            assert!(db.get_entities().contains_key("Book"));
        }

        #[test]
        fn should_add_attributes_with_keys() {
            let attr = Attribute::new("string".to_string(), "id".to_string())
                .with_keys(vec![AttributeKey::PrimaryKey]);

            assert_eq!(attr.keys.len(), 1);
            assert_eq!(attr.keys[0], AttributeKey::PrimaryKey);
        }

        #[test]
        fn should_add_attributes_with_comment() {
            let attr = Attribute::new("string".to_string(), "name".to_string())
                .with_comment("User's full name".to_string());

            assert_eq!(attr.comment, "User's full name");
        }

        #[test]
        fn should_add_attributes_with_keys_and_comment() {
            let attr = Attribute::new("string".to_string(), "author".to_string())
                .with_keys(vec![AttributeKey::ForeignKey])
                .with_comment("author comment".to_string());

            assert_eq!(attr.keys[0], AttributeKey::ForeignKey);
            assert_eq!(attr.comment, "author comment");
        }
    }

    mod relationship_tests {
        use super::*;

        #[test]
        fn should_add_relationship() {
            let mut db = ErDb::new();
            db.add_entity("Customer", None);
            db.add_entity("Order", None);
            db.add_relationship(
                "Customer",
                "places",
                "Order",
                RelSpec::new(
                    Cardinality::OnlyOne,
                    Cardinality::ZeroOrMore,
                    Identification::Identifying,
                ),
            );

            assert_eq!(db.get_relationships().len(), 1);
            assert_eq!(db.get_relationships()[0].role_a, "places");
        }

        #[test]
        fn should_not_add_relationship_if_entities_missing() {
            let mut db = ErDb::new();
            db.add_entity("Customer", None);
            // Order not added
            db.add_relationship(
                "Customer",
                "places",
                "Order",
                RelSpec::new(
                    Cardinality::OnlyOne,
                    Cardinality::ZeroOrMore,
                    Identification::Identifying,
                ),
            );

            assert_eq!(db.get_relationships().len(), 0);
        }

        #[test]
        fn should_store_entity_ids_in_relationship() {
            let mut db = ErDb::new();
            db.add_entity("Customer", None);
            db.add_entity("Order", None);

            let customer_id = db.get_entity("Customer").unwrap().id.clone();
            let order_id = db.get_entity("Order").unwrap().id.clone();

            db.add_relationship(
                "Customer",
                "places",
                "Order",
                RelSpec::new(
                    Cardinality::OnlyOne,
                    Cardinality::ZeroOrMore,
                    Identification::Identifying,
                ),
            );

            let rel = &db.get_relationships()[0];
            assert_eq!(rel.entity_a, customer_id);
            assert_eq!(rel.entity_b, order_id);
        }
    }

    mod cardinality_tests {
        use super::*;

        #[test]
        fn should_parse_cardinality() {
            assert_eq!(Cardinality::from_str("ZERO_OR_ONE"), Cardinality::ZeroOrOne);
            assert_eq!(
                Cardinality::from_str("ZERO_OR_MORE"),
                Cardinality::ZeroOrMore
            );
            assert_eq!(Cardinality::from_str("ONE_OR_MORE"), Cardinality::OneOrMore);
            assert_eq!(Cardinality::from_str("ONLY_ONE"), Cardinality::OnlyOne);
        }

        #[test]
        fn should_output_cardinality() {
            assert_eq!(Cardinality::ZeroOrOne.as_str(), "ZERO_OR_ONE");
            assert_eq!(Cardinality::ZeroOrMore.as_str(), "ZERO_OR_MORE");
            assert_eq!(Cardinality::OneOrMore.as_str(), "ONE_OR_MORE");
            assert_eq!(Cardinality::OnlyOne.as_str(), "ONLY_ONE");
        }
    }

    mod identification_tests {
        use super::*;

        #[test]
        fn should_parse_identification() {
            assert_eq!(
                Identification::from_str("IDENTIFYING"),
                Identification::Identifying
            );
            assert_eq!(
                Identification::from_str("NON_IDENTIFYING"),
                Identification::NonIdentifying
            );
            assert_eq!(Identification::from_str("--"), Identification::Identifying);
            assert_eq!(
                Identification::from_str(".."),
                Identification::NonIdentifying
            );
        }

        #[test]
        fn should_output_identification() {
            assert_eq!(Identification::Identifying.as_str(), "IDENTIFYING");
            assert_eq!(Identification::NonIdentifying.as_str(), "NON_IDENTIFYING");
        }
    }

    mod attribute_key_tests {
        use super::*;

        #[test]
        fn should_parse_attribute_keys() {
            assert_eq!(AttributeKey::from_str("PK"), Some(AttributeKey::PrimaryKey));
            assert_eq!(AttributeKey::from_str("FK"), Some(AttributeKey::ForeignKey));
            assert_eq!(AttributeKey::from_str("UK"), Some(AttributeKey::UniqueKey));
            assert_eq!(AttributeKey::from_str("invalid"), None);
        }

        #[test]
        fn should_output_attribute_keys() {
            assert_eq!(AttributeKey::PrimaryKey.as_str(), "PK");
            assert_eq!(AttributeKey::ForeignKey.as_str(), "FK");
            assert_eq!(AttributeKey::UniqueKey.as_str(), "UK");
        }
    }

    mod direction_tests {
        use super::*;

        #[test]
        fn should_set_direction() {
            let mut db = ErDb::new();
            db.set_direction(Direction::LeftToRight);

            assert_eq!(db.get_direction(), Direction::LeftToRight);
        }

        #[test]
        fn should_parse_direction() {
            assert_eq!(Direction::from_str("TB"), Direction::TopToBottom);
            assert_eq!(Direction::from_str("BT"), Direction::BottomToTop);
            assert_eq!(Direction::from_str("LR"), Direction::LeftToRight);
            assert_eq!(Direction::from_str("RL"), Direction::RightToLeft);
        }

        #[test]
        fn should_output_direction() {
            assert_eq!(Direction::TopToBottom.as_str(), "TB");
            assert_eq!(Direction::BottomToTop.as_str(), "BT");
            assert_eq!(Direction::LeftToRight.as_str(), "LR");
            assert_eq!(Direction::RightToLeft.as_str(), "RL");
        }
    }

    mod style_tests {
        use super::*;

        #[test]
        fn should_add_css_styles() {
            let mut db = ErDb::new();
            db.add_entity("Customer", None);
            db.add_css_styles(&["Customer"], &["fill:#f00", "stroke:#000"]);

            let entity = db.get_entity("Customer").unwrap();
            assert_eq!(entity.css_styles.len(), 2);
        }

        #[test]
        fn should_add_class_definition() {
            let mut db = ErDb::new();
            db.add_class(&["highlight"], &["fill:#ff0", "stroke:#000"]);

            let classes = db.get_classes();
            assert!(classes.contains_key("highlight"));
            assert_eq!(classes.get("highlight").unwrap().styles.len(), 2);
        }

        #[test]
        fn should_add_color_to_text_styles() {
            let mut db = ErDb::new();
            db.add_class(&["highlight"], &["color:#fff", "fill:#000"]);

            let class = db.get_classes().get("highlight").unwrap();
            assert_eq!(class.text_styles.len(), 1);
            assert!(class.text_styles[0].contains("color"));
        }

        #[test]
        fn should_set_class_on_entity() {
            let mut db = ErDb::new();
            db.add_entity("Customer", None);
            db.set_class(&["Customer"], &["highlight", "important"]);

            let entity = db.get_entity("Customer").unwrap();
            assert!(entity.css_classes.contains("highlight"));
            assert!(entity.css_classes.contains("important"));
        }

        #[test]
        fn should_get_compiled_styles() {
            let mut db = ErDb::new();
            db.add_class(&["myClass"], &["fill:#f00", "stroke:#000"]);

            let compiled = db.get_compiled_styles(&["myClass"]);
            assert_eq!(compiled.len(), 2);
            assert!(compiled.contains(&"fill:#f00".to_string()));
        }
    }

    mod clear_tests {
        use super::*;

        #[test]
        fn should_clear_all_state() {
            let mut db = ErDb::new();
            db.add_entity("Customer", None);
            db.add_entity("Order", None);
            db.add_relationship(
                "Customer",
                "places",
                "Order",
                RelSpec::new(
                    Cardinality::OnlyOne,
                    Cardinality::ZeroOrMore,
                    Identification::Identifying,
                ),
            );
            db.add_class(&["highlight"], &["fill:#ff0"]);
            db.set_direction(Direction::LeftToRight);
            db.acc_title = "Title".to_string();

            db.clear();

            assert!(db.get_entities().is_empty());
            assert!(db.get_relationships().is_empty());
            assert!(db.get_classes().is_empty());
            assert_eq!(db.get_direction(), Direction::TopToBottom);
            assert!(db.acc_title.is_empty());
        }
    }

    mod rel_spec_tests {
        use super::*;

        #[test]
        fn should_create_rel_spec() {
            let spec = RelSpec::new(
                Cardinality::OnlyOne,
                Cardinality::ZeroOrMore,
                Identification::Identifying,
            );

            assert_eq!(spec.card_a, Cardinality::OnlyOne);
            assert_eq!(spec.card_b, Cardinality::ZeroOrMore);
            assert_eq!(spec.rel_type, Identification::Identifying);
        }
    }
}
