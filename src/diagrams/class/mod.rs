//! Class diagram support
//!
//! The class diagram shows the structure of classes, their attributes,
//! methods, and relationships between them.

pub mod parser;
mod types;

pub use parser::parse;
pub use types::*;

#[cfg(test)]
mod tests {
    use super::*;

    const STATIC_CSS_STYLE: &str = "text-decoration:underline;";
    const ABSTRACT_CSS_STYLE: &str = "font-style:italic;";

    mod method_no_params {
        use super::*;

        #[test]
        fn should_parse_correctly() {
            let member = ClassMember::new("getTime()", MemberType::Method);
            assert_eq!(member.get_display_details().display_text, "getTime()");
        }

        #[test]
        fn should_handle_public_visibility() {
            let member = ClassMember::new("+getTime()", MemberType::Method);
            assert_eq!(member.get_display_details().display_text, "+getTime()");
        }

        #[test]
        fn should_handle_private_visibility() {
            let member = ClassMember::new("-getTime()", MemberType::Method);
            assert_eq!(member.get_display_details().display_text, "-getTime()");
        }

        #[test]
        fn should_handle_protected_visibility() {
            let member = ClassMember::new("#getTime()", MemberType::Method);
            assert_eq!(member.get_display_details().display_text, "#getTime()");
        }

        #[test]
        fn should_handle_internal_visibility() {
            let member = ClassMember::new("~getTime()", MemberType::Method);
            assert_eq!(member.get_display_details().display_text, "~getTime()");
        }

        #[test]
        fn should_return_correct_css_for_static_classifier() {
            let member = ClassMember::new("getTime()$", MemberType::Method);
            let details = member.get_display_details();
            assert_eq!(details.display_text, "getTime()");
            assert_eq!(details.css_style, STATIC_CSS_STYLE);
        }

        #[test]
        fn should_return_correct_css_for_abstract_classifier() {
            let member = ClassMember::new("getTime()*", MemberType::Method);
            let details = member.get_display_details();
            assert_eq!(details.display_text, "getTime()");
            assert_eq!(details.css_style, ABSTRACT_CSS_STYLE);
        }
    }

    mod method_single_param {
        use super::*;

        #[test]
        fn should_parse_correctly() {
            let member = ClassMember::new("getTime(int)", MemberType::Method);
            assert_eq!(member.get_display_details().display_text, "getTime(int)");
        }

        #[test]
        fn should_handle_public_visibility() {
            let member = ClassMember::new("+getTime(int)", MemberType::Method);
            assert_eq!(member.get_display_details().display_text, "+getTime(int)");
        }

        #[test]
        fn should_handle_private_visibility() {
            let member = ClassMember::new("-getTime(int)", MemberType::Method);
            assert_eq!(member.get_display_details().display_text, "-getTime(int)");
        }

        #[test]
        fn should_handle_protected_visibility() {
            let member = ClassMember::new("#getTime(int)", MemberType::Method);
            assert_eq!(member.get_display_details().display_text, "#getTime(int)");
        }

        #[test]
        fn should_handle_internal_visibility() {
            let member = ClassMember::new("~getTime(int)", MemberType::Method);
            assert_eq!(member.get_display_details().display_text, "~getTime(int)");
        }

        #[test]
        fn should_return_correct_css_for_static_classifier() {
            let member = ClassMember::new("getTime(int)$", MemberType::Method);
            let details = member.get_display_details();
            assert_eq!(details.display_text, "getTime(int)");
            assert_eq!(details.css_style, STATIC_CSS_STYLE);
        }

        #[test]
        fn should_return_correct_css_for_abstract_classifier() {
            let member = ClassMember::new("getTime(int)*", MemberType::Method);
            let details = member.get_display_details();
            assert_eq!(details.display_text, "getTime(int)");
            assert_eq!(details.css_style, ABSTRACT_CSS_STYLE);
        }
    }

    mod method_param_type_first {
        use super::*;

        #[test]
        fn should_parse_correctly() {
            let member = ClassMember::new("getTime(int count)", MemberType::Method);
            assert_eq!(
                member.get_display_details().display_text,
                "getTime(int count)"
            );
        }

        #[test]
        fn should_handle_public_visibility() {
            let member = ClassMember::new("+getTime(int count)", MemberType::Method);
            assert_eq!(
                member.get_display_details().display_text,
                "+getTime(int count)"
            );
        }

        #[test]
        fn should_handle_private_visibility() {
            let member = ClassMember::new("-getTime(int count)", MemberType::Method);
            assert_eq!(
                member.get_display_details().display_text,
                "-getTime(int count)"
            );
        }

        #[test]
        fn should_handle_protected_visibility() {
            let member = ClassMember::new("#getTime(int count)", MemberType::Method);
            assert_eq!(
                member.get_display_details().display_text,
                "#getTime(int count)"
            );
        }

        #[test]
        fn should_handle_internal_visibility() {
            let member = ClassMember::new("~getTime(int count)", MemberType::Method);
            assert_eq!(
                member.get_display_details().display_text,
                "~getTime(int count)"
            );
        }

        #[test]
        fn should_return_correct_css_for_static_classifier() {
            let member = ClassMember::new("getTime(int count)$", MemberType::Method);
            let details = member.get_display_details();
            assert_eq!(details.display_text, "getTime(int count)");
            assert_eq!(details.css_style, STATIC_CSS_STYLE);
        }

        #[test]
        fn should_return_correct_css_for_abstract_classifier() {
            let member = ClassMember::new("getTime(int count)*", MemberType::Method);
            let details = member.get_display_details();
            assert_eq!(details.display_text, "getTime(int count)");
            assert_eq!(details.css_style, ABSTRACT_CSS_STYLE);
        }
    }

    mod method_param_name_first {
        use super::*;

        #[test]
        fn should_parse_correctly() {
            let member = ClassMember::new("getTime(count int)", MemberType::Method);
            assert_eq!(
                member.get_display_details().display_text,
                "getTime(count int)"
            );
        }

        #[test]
        fn should_handle_public_visibility() {
            let member = ClassMember::new("+getTime(count int)", MemberType::Method);
            assert_eq!(
                member.get_display_details().display_text,
                "+getTime(count int)"
            );
        }

        #[test]
        fn should_handle_private_visibility() {
            let member = ClassMember::new("-getTime(count int)", MemberType::Method);
            assert_eq!(
                member.get_display_details().display_text,
                "-getTime(count int)"
            );
        }

        #[test]
        fn should_handle_protected_visibility() {
            let member = ClassMember::new("#getTime(count int)", MemberType::Method);
            assert_eq!(
                member.get_display_details().display_text,
                "#getTime(count int)"
            );
        }

        #[test]
        fn should_handle_internal_visibility() {
            let member = ClassMember::new("~getTime(count int)", MemberType::Method);
            assert_eq!(
                member.get_display_details().display_text,
                "~getTime(count int)"
            );
        }

        #[test]
        fn should_return_correct_css_for_static_classifier() {
            let member = ClassMember::new("getTime(count int)$", MemberType::Method);
            let details = member.get_display_details();
            assert_eq!(details.display_text, "getTime(count int)");
            assert_eq!(details.css_style, STATIC_CSS_STYLE);
        }

        #[test]
        fn should_return_correct_css_for_abstract_classifier() {
            let member = ClassMember::new("getTime(count int)*", MemberType::Method);
            let details = member.get_display_details();
            assert_eq!(details.display_text, "getTime(count int)");
            assert_eq!(details.css_style, ABSTRACT_CSS_STYLE);
        }
    }

    mod method_multiple_params {
        use super::*;

        #[test]
        fn should_parse_correctly() {
            let member = ClassMember::new("getTime(string text, int count)", MemberType::Method);
            assert_eq!(
                member.get_display_details().display_text,
                "getTime(string text, int count)"
            );
        }

        #[test]
        fn should_handle_public_visibility() {
            let member = ClassMember::new("+getTime(string text, int count)", MemberType::Method);
            assert_eq!(
                member.get_display_details().display_text,
                "+getTime(string text, int count)"
            );
        }

        #[test]
        fn should_handle_private_visibility() {
            let member = ClassMember::new("-getTime(string text, int count)", MemberType::Method);
            assert_eq!(
                member.get_display_details().display_text,
                "-getTime(string text, int count)"
            );
        }

        #[test]
        fn should_handle_protected_visibility() {
            let member = ClassMember::new("#getTime(string text, int count)", MemberType::Method);
            assert_eq!(
                member.get_display_details().display_text,
                "#getTime(string text, int count)"
            );
        }

        #[test]
        fn should_handle_internal_visibility() {
            let member = ClassMember::new("~getTime(string text, int count)", MemberType::Method);
            assert_eq!(
                member.get_display_details().display_text,
                "~getTime(string text, int count)"
            );
        }

        #[test]
        fn should_return_correct_css_for_static_classifier() {
            let member = ClassMember::new("getTime(string text, int count)$", MemberType::Method);
            let details = member.get_display_details();
            assert_eq!(details.display_text, "getTime(string text, int count)");
            assert_eq!(details.css_style, STATIC_CSS_STYLE);
        }

        #[test]
        fn should_return_correct_css_for_abstract_classifier() {
            let member = ClassMember::new("getTime(string text, int count)*", MemberType::Method);
            let details = member.get_display_details();
            assert_eq!(details.display_text, "getTime(string text, int count)");
            assert_eq!(details.css_style, ABSTRACT_CSS_STYLE);
        }
    }

    mod method_with_return_type {
        use super::*;

        #[test]
        fn should_parse_correctly() {
            let member = ClassMember::new("getTime() DateTime", MemberType::Method);
            assert_eq!(
                member.get_display_details().display_text,
                "getTime() : DateTime"
            );
        }

        #[test]
        fn should_handle_public_visibility() {
            let member = ClassMember::new("+getTime() DateTime", MemberType::Method);
            assert_eq!(
                member.get_display_details().display_text,
                "+getTime() : DateTime"
            );
        }

        #[test]
        fn should_handle_private_visibility() {
            let member = ClassMember::new("-getTime() DateTime", MemberType::Method);
            assert_eq!(
                member.get_display_details().display_text,
                "-getTime() : DateTime"
            );
        }

        #[test]
        fn should_handle_protected_visibility() {
            let member = ClassMember::new("#getTime() DateTime", MemberType::Method);
            assert_eq!(
                member.get_display_details().display_text,
                "#getTime() : DateTime"
            );
        }

        #[test]
        fn should_handle_internal_visibility() {
            let member = ClassMember::new("~getTime() DateTime", MemberType::Method);
            assert_eq!(
                member.get_display_details().display_text,
                "~getTime() : DateTime"
            );
        }

        #[test]
        fn should_return_correct_css_for_static_classifier() {
            let member = ClassMember::new("getTime() DateTime$", MemberType::Method);
            let details = member.get_display_details();
            assert_eq!(details.display_text, "getTime() : DateTime");
            assert_eq!(details.css_style, STATIC_CSS_STYLE);
        }

        #[test]
        fn should_return_correct_css_for_abstract_classifier() {
            let member = ClassMember::new("getTime()  DateTime*", MemberType::Method);
            let details = member.get_display_details();
            assert_eq!(details.display_text, "getTime() : DateTime");
            assert_eq!(details.css_style, ABSTRACT_CSS_STYLE);
        }
    }

    mod method_generic_param {
        use super::*;

        #[test]
        fn should_parse_correctly() {
            let member = ClassMember::new("getTimes(List~T~)", MemberType::Method);
            assert_eq!(
                member.get_display_details().display_text,
                "getTimes(List<T>)"
            );
        }

        #[test]
        fn should_handle_public_visibility() {
            let member = ClassMember::new("+getTimes(List~T~)", MemberType::Method);
            assert_eq!(
                member.get_display_details().display_text,
                "+getTimes(List<T>)"
            );
        }

        #[test]
        fn should_handle_private_visibility() {
            let member = ClassMember::new("-getTimes(List~T~)", MemberType::Method);
            assert_eq!(
                member.get_display_details().display_text,
                "-getTimes(List<T>)"
            );
        }

        #[test]
        fn should_handle_protected_visibility() {
            let member = ClassMember::new("#getTimes(List~T~)", MemberType::Method);
            assert_eq!(
                member.get_display_details().display_text,
                "#getTimes(List<T>)"
            );
        }

        #[test]
        fn should_handle_internal_visibility() {
            let member = ClassMember::new("~getTimes(List~T~)", MemberType::Method);
            assert_eq!(
                member.get_display_details().display_text,
                "~getTimes(List<T>)"
            );
        }

        #[test]
        fn should_return_correct_css_for_static_classifier() {
            let member = ClassMember::new("getTimes(List~T~)$", MemberType::Method);
            let details = member.get_display_details();
            assert_eq!(details.display_text, "getTimes(List<T>)");
            assert_eq!(details.css_style, STATIC_CSS_STYLE);
        }

        #[test]
        fn should_return_correct_css_for_abstract_classifier() {
            let member = ClassMember::new("getTimes(List~T~)*", MemberType::Method);
            let details = member.get_display_details();
            assert_eq!(details.display_text, "getTimes(List<T>)");
            assert_eq!(details.css_style, ABSTRACT_CSS_STYLE);
        }
    }

    mod method_two_generics {
        use super::*;

        #[test]
        fn should_parse_correctly() {
            let member = ClassMember::new("getTimes(List~T~, List~OT~)", MemberType::Method);
            assert_eq!(
                member.get_display_details().display_text,
                "getTimes(List<T>, List<OT>)"
            );
        }

        #[test]
        fn should_handle_public_visibility() {
            let member = ClassMember::new("+getTimes(List~T~, List~OT~)", MemberType::Method);
            assert_eq!(
                member.get_display_details().display_text,
                "+getTimes(List<T>, List<OT>)"
            );
        }

        #[test]
        fn should_handle_private_visibility() {
            let member = ClassMember::new("-getTimes(List~T~, List~OT~)", MemberType::Method);
            assert_eq!(
                member.get_display_details().display_text,
                "-getTimes(List<T>, List<OT>)"
            );
        }

        #[test]
        fn should_handle_protected_visibility() {
            let member = ClassMember::new("#getTimes(List~T~, List~OT~)", MemberType::Method);
            assert_eq!(
                member.get_display_details().display_text,
                "#getTimes(List<T>, List<OT>)"
            );
        }

        #[test]
        fn should_handle_internal_visibility() {
            let member = ClassMember::new("~getTimes(List~T~, List~OT~)", MemberType::Method);
            assert_eq!(
                member.get_display_details().display_text,
                "~getTimes(List<T>, List<OT>)"
            );
        }

        #[test]
        fn should_return_correct_css_for_static_classifier() {
            let member = ClassMember::new("getTimes(List~T~, List~OT~)$", MemberType::Method);
            let details = member.get_display_details();
            assert_eq!(details.display_text, "getTimes(List<T>, List<OT>)");
            assert_eq!(details.css_style, STATIC_CSS_STYLE);
        }

        #[test]
        fn should_return_correct_css_for_abstract_classifier() {
            let member = ClassMember::new("getTimes(List~T~, List~OT~)*", MemberType::Method);
            let details = member.get_display_details();
            assert_eq!(details.display_text, "getTimes(List<T>, List<OT>)");
            assert_eq!(details.css_style, ABSTRACT_CSS_STYLE);
        }
    }

    mod method_nested_generic_param {
        use super::*;

        #[test]
        fn should_parse_correctly() {
            let member = ClassMember::new("getTimetableList(List~List~T~~)", MemberType::Method);
            assert_eq!(
                member.get_display_details().display_text,
                "getTimetableList(List<List<T>>)"
            );
        }

        #[test]
        fn should_handle_public_visibility() {
            let member = ClassMember::new("+getTimetableList(List~List~T~~)", MemberType::Method);
            assert_eq!(
                member.get_display_details().display_text,
                "+getTimetableList(List<List<T>>)"
            );
        }

        #[test]
        fn should_handle_private_visibility() {
            let member = ClassMember::new("-getTimetableList(List~List~T~~)", MemberType::Method);
            assert_eq!(
                member.get_display_details().display_text,
                "-getTimetableList(List<List<T>>)"
            );
        }

        #[test]
        fn should_handle_protected_visibility() {
            let member = ClassMember::new("#getTimetableList(List~List~T~~)", MemberType::Method);
            assert_eq!(
                member.get_display_details().display_text,
                "#getTimetableList(List<List<T>>)"
            );
        }

        #[test]
        fn should_handle_internal_visibility() {
            let member = ClassMember::new("~getTimetableList(List~List~T~~)", MemberType::Method);
            assert_eq!(
                member.get_display_details().display_text,
                "~getTimetableList(List<List<T>>)"
            );
        }

        #[test]
        fn should_return_correct_css_for_static_classifier() {
            let member = ClassMember::new("getTimetableList(List~List~T~~)$", MemberType::Method);
            let details = member.get_display_details();
            assert_eq!(details.display_text, "getTimetableList(List<List<T>>)");
            assert_eq!(details.css_style, STATIC_CSS_STYLE);
        }

        #[test]
        fn should_return_correct_css_for_abstract_classifier() {
            let member = ClassMember::new("getTimetableList(List~List~T~~)*", MemberType::Method);
            let details = member.get_display_details();
            assert_eq!(details.display_text, "getTimetableList(List<List<T>>)");
            assert_eq!(details.css_style, ABSTRACT_CSS_STYLE);
        }
    }

    mod method_composite_generic_param {
        use super::*;

        #[test]
        fn should_parse_correctly() {
            let member = ClassMember::new("getTimes(List~K, V~)", MemberType::Method);
            assert_eq!(
                member.get_display_details().display_text,
                "getTimes(List<K, V>)"
            );
        }

        #[test]
        fn should_handle_public_visibility() {
            let member = ClassMember::new("+getTimes(List~K, V~)", MemberType::Method);
            assert_eq!(
                member.get_display_details().display_text,
                "+getTimes(List<K, V>)"
            );
        }

        #[test]
        fn should_handle_private_visibility() {
            let member = ClassMember::new("-getTimes(List~K, V~)", MemberType::Method);
            assert_eq!(
                member.get_display_details().display_text,
                "-getTimes(List<K, V>)"
            );
        }

        #[test]
        fn should_handle_protected_visibility() {
            let member = ClassMember::new("#getTimes(List~K, V~)", MemberType::Method);
            assert_eq!(
                member.get_display_details().display_text,
                "#getTimes(List<K, V>)"
            );
        }

        #[test]
        fn should_handle_internal_visibility() {
            let member = ClassMember::new("~getTimes(List~K, V~)", MemberType::Method);
            assert_eq!(
                member.get_display_details().display_text,
                "~getTimes(List<K, V>)"
            );
        }

        #[test]
        fn should_return_correct_css_for_static_classifier() {
            let member = ClassMember::new("getTimes(List~K, V~)$", MemberType::Method);
            let details = member.get_display_details();
            assert_eq!(details.display_text, "getTimes(List<K, V>)");
            assert_eq!(details.css_style, STATIC_CSS_STYLE);
        }

        #[test]
        fn should_return_correct_css_for_abstract_classifier() {
            let member = ClassMember::new("getTimes(List~K, V~)*", MemberType::Method);
            let details = member.get_display_details();
            assert_eq!(details.display_text, "getTimes(List<K, V>)");
            assert_eq!(details.css_style, ABSTRACT_CSS_STYLE);
        }
    }

    mod method_generic_return_type {
        use super::*;

        #[test]
        fn should_parse_correctly() {
            let member = ClassMember::new("getTimes() List~T~", MemberType::Method);
            assert_eq!(
                member.get_display_details().display_text,
                "getTimes() : List<T>"
            );
        }

        #[test]
        fn should_handle_public_visibility() {
            let member = ClassMember::new("+getTimes() List~T~", MemberType::Method);
            assert_eq!(
                member.get_display_details().display_text,
                "+getTimes() : List<T>"
            );
        }

        #[test]
        fn should_handle_private_visibility() {
            let member = ClassMember::new("-getTimes() List~T~", MemberType::Method);
            assert_eq!(
                member.get_display_details().display_text,
                "-getTimes() : List<T>"
            );
        }

        #[test]
        fn should_handle_protected_visibility() {
            let member = ClassMember::new("#getTimes() List~T~", MemberType::Method);
            assert_eq!(
                member.get_display_details().display_text,
                "#getTimes() : List<T>"
            );
        }

        #[test]
        fn should_handle_internal_visibility() {
            let member = ClassMember::new("~getTimes() List~T~", MemberType::Method);
            assert_eq!(
                member.get_display_details().display_text,
                "~getTimes() : List<T>"
            );
        }

        #[test]
        fn should_return_correct_css_for_static_classifier() {
            let member = ClassMember::new("getTimes() List~T~$", MemberType::Method);
            let details = member.get_display_details();
            assert_eq!(details.display_text, "getTimes() : List<T>");
            assert_eq!(details.css_style, STATIC_CSS_STYLE);
        }

        #[test]
        fn should_return_correct_css_for_abstract_classifier() {
            let member = ClassMember::new("getTimes() List~T~*", MemberType::Method);
            let details = member.get_display_details();
            assert_eq!(details.display_text, "getTimes() : List<T>");
            assert_eq!(details.css_style, ABSTRACT_CSS_STYLE);
        }
    }

    mod method_nested_generic_return {
        use super::*;

        #[test]
        fn should_parse_correctly() {
            let member = ClassMember::new("getTimetableList() List~List~T~~", MemberType::Method);
            assert_eq!(
                member.get_display_details().display_text,
                "getTimetableList() : List<List<T>>"
            );
        }

        #[test]
        fn should_handle_public_visibility() {
            let member = ClassMember::new("+getTimetableList() List~List~T~~", MemberType::Method);
            assert_eq!(
                member.get_display_details().display_text,
                "+getTimetableList() : List<List<T>>"
            );
        }

        #[test]
        fn should_handle_private_visibility() {
            let member = ClassMember::new("-getTimetableList() List~List~T~~", MemberType::Method);
            assert_eq!(
                member.get_display_details().display_text,
                "-getTimetableList() : List<List<T>>"
            );
        }

        #[test]
        fn should_handle_protected_visibility() {
            let member = ClassMember::new("#getTimetableList() List~List~T~~", MemberType::Method);
            assert_eq!(
                member.get_display_details().display_text,
                "#getTimetableList() : List<List<T>>"
            );
        }

        #[test]
        fn should_handle_internal_visibility() {
            let member = ClassMember::new("~getTimetableList() List~List~T~~", MemberType::Method);
            assert_eq!(
                member.get_display_details().display_text,
                "~getTimetableList() : List<List<T>>"
            );
        }

        #[test]
        fn should_return_correct_css_for_static_classifier() {
            let member = ClassMember::new("getTimetableList() List~List~T~~$", MemberType::Method);
            let details = member.get_display_details();
            assert_eq!(details.display_text, "getTimetableList() : List<List<T>>");
            assert_eq!(details.css_style, STATIC_CSS_STYLE);
        }

        #[test]
        fn should_return_correct_css_for_abstract_classifier() {
            let member = ClassMember::new("getTimetableList() List~List~T~~*", MemberType::Method);
            let details = member.get_display_details();
            assert_eq!(details.display_text, "getTimetableList() : List<List<T>>");
            assert_eq!(details.css_style, ABSTRACT_CSS_STYLE);
        }
    }

    mod uncategorized_method_tests {
        use super::*;

        #[test]
        fn member_name_should_handle_double_colons() {
            let member = ClassMember::new("std::map ~int,string~ pMap;", MemberType::Attribute);
            assert_eq!(
                member.get_display_details().display_text,
                "std::map <int,string> pMap;"
            );
        }

        #[test]
        fn member_name_should_handle_generic_type() {
            let member = ClassMember::new(
                "getTime~T~(this T, int seconds)$ DateTime",
                MemberType::Method,
            );
            let details = member.get_display_details();
            assert_eq!(
                details.display_text,
                "getTime<T>(this T, int seconds) : DateTime"
            );
            assert_eq!(details.css_style, STATIC_CSS_STYLE);
        }
    }

    mod attribute_tests {
        use super::*;

        #[test]
        fn should_parse_no_modifiers() {
            let member = ClassMember::new("name String", MemberType::Attribute);
            let details = member.get_display_details();
            assert_eq!(details.display_text, "name String");
            assert_eq!(details.css_style, "");
        }

        #[test]
        fn should_handle_public_modifier() {
            let member = ClassMember::new("+name String", MemberType::Attribute);
            let details = member.get_display_details();
            assert_eq!(details.display_text, "+name String");
            assert_eq!(details.css_style, "");
        }

        #[test]
        fn should_handle_protected_modifier() {
            let member = ClassMember::new("#name String", MemberType::Attribute);
            let details = member.get_display_details();
            assert_eq!(details.display_text, "#name String");
            assert_eq!(details.css_style, "");
        }

        #[test]
        fn should_handle_private_modifier() {
            let member = ClassMember::new("-name String", MemberType::Attribute);
            let details = member.get_display_details();
            assert_eq!(details.display_text, "-name String");
            assert_eq!(details.css_style, "");
        }

        #[test]
        fn should_handle_internal_modifier() {
            let member = ClassMember::new("~name String", MemberType::Attribute);
            let details = member.get_display_details();
            assert_eq!(details.display_text, "~name String");
            assert_eq!(details.css_style, "");
        }

        #[test]
        fn should_handle_static_modifier() {
            let member = ClassMember::new("name String$", MemberType::Attribute);
            let details = member.get_display_details();
            assert_eq!(details.display_text, "name String");
            assert_eq!(details.css_style, STATIC_CSS_STYLE);
        }

        #[test]
        fn should_handle_abstract_modifier() {
            let member = ClassMember::new("name String*", MemberType::Attribute);
            let details = member.get_display_details();
            assert_eq!(details.display_text, "name String");
            assert_eq!(details.css_style, ABSTRACT_CSS_STYLE);
        }
    }

    mod class_db_tests {
        use super::*;

        #[test]
        fn should_add_class() {
            let mut db = ClassDb::new();
            db.add_class("TestClass");
            assert!(db.get_class("TestClass").is_some());
        }

        #[test]
        fn should_add_member_to_class() {
            let mut db = ClassDb::new();
            db.add_member("TestClass", "name String");
            let class = db.get_class("TestClass").unwrap();
            assert_eq!(class.members.len(), 1);
        }

        #[test]
        fn should_add_method_to_class() {
            let mut db = ClassDb::new();
            db.add_member("TestClass", "getName()");
            let class = db.get_class("TestClass").unwrap();
            assert_eq!(class.methods.len(), 1);
        }

        #[test]
        fn should_add_relation() {
            let mut db = ClassDb::new();
            let relation = ClassRelation {
                id1: "Class1".to_string(),
                id2: "Class2".to_string(),
                relation_title1: String::new(),
                relation_title2: String::new(),
                relation_type: "extension".to_string(),
                title: String::new(),
                text: String::new(),
                style: Vec::new(),
                relation: RelationDetails {
                    type1: 0,
                    type2: 1,
                    line_type: LineType::Solid,
                },
            };
            db.add_relation(relation);
            assert_eq!(db.relations.len(), 1);
        }

        #[test]
        fn should_set_direction() {
            let mut db = ClassDb::new();
            db.set_direction("LR");
            assert_eq!(db.direction, "LR");
        }

        #[test]
        fn should_add_annotation() {
            let mut db = ClassDb::new();
            db.add_annotation("TestClass", "interface");
            let class = db.get_class("TestClass").unwrap();
            assert_eq!(class.annotations.len(), 1);
            assert_eq!(class.annotations[0], "interface");
        }

        #[test]
        fn should_add_note() {
            let mut db = ClassDb::new();
            let id = db.add_note("This is a note", "TestClass");
            assert!(db.notes.contains_key(&id));
        }

        #[test]
        fn should_set_css_class() {
            let mut db = ClassDb::new();
            db.set_css_class("TestClass", "highlight");
            let class = db.get_class("TestClass").unwrap();
            assert_eq!(class.css_classes, "highlight");
        }

        #[test]
        fn should_set_link() {
            let mut db = ClassDb::new();
            db.set_link("TestClass", "https://example.com", "_blank");
            let class = db.get_class("TestClass").unwrap();
            assert_eq!(class.link, Some("https://example.com".to_string()));
            assert_eq!(class.link_target, Some("_blank".to_string()));
        }

        #[test]
        fn should_clear() {
            let mut db = ClassDb::new();
            db.add_class("TestClass");
            db.set_direction("LR");
            db.clear();
            assert!(db.classes.is_empty());
            assert_eq!(db.direction, "TB");
        }
    }
}
