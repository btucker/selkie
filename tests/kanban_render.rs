//! Kanban diagram rendering tests - ported from Cypress tests
//!
//! These tests are ported from the mermaid.js Cypress test suite:
//! - cypress/integration/rendering/kanban.spec.ts

use roxmltree::Document;
use selkie::{parse, render};

fn render_kanban_svg(input: &str) -> String {
    let diagram = parse(input).expect("Failed to parse kanban diagram");
    render(&diagram).expect("Failed to render kanban diagram")
}

fn parse_svg(svg: &str) -> Document<'_> {
    Document::parse(svg).expect("Failed to parse SVG")
}

fn has_class(doc: &Document<'_>, class_name: &str) -> bool {
    doc.descendants().any(|node| {
        node.attribute("class")
            .map(|class| class.split_whitespace().any(|c| c == class_name))
            .unwrap_or(false)
    })
}

fn count_elements_with_class(doc: &Document<'_>, class_name: &str) -> usize {
    doc.descendants()
        .filter(|node| {
            node.attribute("class")
                .map(|class| class.split_whitespace().any(|c| c == class_name))
                .unwrap_or(false)
        })
        .count()
}

fn svg_contains_text(svg: &str, text: &str) -> bool {
    svg.contains(text)
}

/// Helper to check basic SVG structure
fn assert_valid_svg(svg: &str) {
    assert!(svg.contains("<svg"), "SVG should have opening tag");
    assert!(svg.contains("</svg>"), "SVG should have closing tag");
    assert!(
        svg.contains("xmlns=\"http://www.w3.org/2000/svg\""),
        "SVG should have namespace"
    );
}

// ============================================================================
// Basic Rendering Tests (from Cypress kanban.spec.ts)
// ============================================================================

#[test]
fn kanban_single_section() {
    // From Cypress: 1: should render a kanban with a single section
    let input = r#"kanban
  id1[Todo]
    docs[Create Documentation]
    docs[Create Blog about the new diagram]"#;

    let svg = render_kanban_svg(input);
    assert_valid_svg(&svg);

    // Should contain the section title
    assert!(
        svg_contains_text(&svg, "Todo"),
        "Should contain section title 'Todo'"
    );

    // Should contain the task labels
    assert!(
        svg_contains_text(&svg, "Create Documentation"),
        "Should contain task 'Create Documentation'"
    );
    assert!(
        svg_contains_text(&svg, "Create Blog about"),
        "Should contain task 'Create Blog...'"
    );

    let doc = parse_svg(&svg);
    // Should have sections class
    assert!(has_class(&doc, "sections"), "Should have sections class");
    // Should have items class
    assert!(has_class(&doc, "items"), "Should have items class");
}

#[test]
fn kanban_multiple_sections() {
    // From Cypress: 2: should render a kanban with multiple sections
    let input = r#"kanban
  id1[Todo]
    docs[Create Documentation]
  id2[Done]
    docs[Create Blog about the new diagram]"#;

    let svg = render_kanban_svg(input);
    assert_valid_svg(&svg);

    // Should contain both section titles
    assert!(
        svg_contains_text(&svg, "Todo"),
        "Should contain section 'Todo'"
    );
    assert!(
        svg_contains_text(&svg, "Done"),
        "Should contain section 'Done'"
    );

    let doc = parse_svg(&svg);
    // Should have two sections
    let section_count = count_elements_with_class(&doc, "section");
    assert!(
        section_count >= 2,
        "Should have at least 2 section elements"
    );
}

#[test]
fn kanban_wrapping_node() {
    // From Cypress: 3: should render a kanban with a single wrapping node
    let input = r#"kanban
  id1[Todo]
    id2[Title of diagram is more than 100 chars when user duplicates diagram with 100 char, wrapping]"#;

    let svg = render_kanban_svg(input);
    assert_valid_svg(&svg);

    // Should contain wrapped text parts
    assert!(
        svg_contains_text(&svg, "Title of diagram"),
        "Should contain start of wrapped text"
    );
}

#[test]
fn kanban_with_assignments() {
    // From Cypress: 6: should handle assignments
    let input = r#"kanban
  id1[Todo]
    docs[Create Documentation]
  id2[In progress]
    docs[Create Blog about the new diagram]@{ assigned: 'knsv' }"#;

    let svg = render_kanban_svg(input);
    assert_valid_svg(&svg);

    // Should contain the assigned person
    assert!(
        svg_contains_text(&svg, "knsv"),
        "Should contain assigned person 'knsv'"
    );

    let doc = parse_svg(&svg);
    assert!(
        has_class(&doc, "kanban-assigned"),
        "Should have kanban-assigned class"
    );
}

#[test]
fn kanban_with_prioritization() {
    // From Cypress: 7: should handle prioritization
    let input = r#"kanban
  id2[In progress]
    vh[Very High]@{ priority: 'Very High' }
    h[High]@{ priority: 'High' }
    m[Default priority]
    l[Low]@{ priority: 'Low' }
    vl[Very Low]@{ priority: 'Very Low' }"#;

    let svg = render_kanban_svg(input);
    assert_valid_svg(&svg);

    // Should contain priority indicators
    let doc = parse_svg(&svg);
    assert!(
        has_class(&doc, "priority-indicator"),
        "Should have priority-indicator class"
    );

    // Should contain all priority labels
    assert!(
        svg_contains_text(&svg, "Very High"),
        "Should contain 'Very High'"
    );
    assert!(svg_contains_text(&svg, "High"), "Should contain 'High'");
    assert!(svg_contains_text(&svg, "Low"), "Should contain 'Low'");
    assert!(
        svg_contains_text(&svg, "Very Low"),
        "Should contain 'Very Low'"
    );
}

#[test]
fn kanban_with_tickets() {
    // From Cypress: 7: should handle external tickets
    let input = r#"kanban
  id1[Todo]
    docs[Create Documentation]
  id2[In progress]
    docs[Create Blog about the new diagram]@{ ticket: MC-2037 }"#;

    let svg = render_kanban_svg(input);
    assert_valid_svg(&svg);

    // Should contain the ticket number
    assert!(
        svg_contains_text(&svg, "MC-2037"),
        "Should contain ticket number 'MC-2037'"
    );

    let doc = parse_svg(&svg);
    assert!(
        has_class(&doc, "kanban-ticket"),
        "Should have kanban-ticket class"
    );
}

#[test]
fn kanban_full_metadata() {
    // From Cypress: 8: should handle assignments, prioritization and tickets ids in the same item
    let input = r#"kanban
  id2[In progress]
    docs[Create Blog about the new diagram]@{ priority: 'Very Low', ticket: MC-2037, assigned: 'knsv' }"#;

    let svg = render_kanban_svg(input);
    assert_valid_svg(&svg);

    // Should contain all metadata
    assert!(svg_contains_text(&svg, "MC-2037"), "Should contain ticket");
    assert!(svg_contains_text(&svg, "knsv"), "Should contain assigned");

    let doc = parse_svg(&svg);
    assert!(
        has_class(&doc, "priority-indicator"),
        "Should have priority indicator"
    );
}

#[test]
fn kanban_full_example() {
    // From Cypress: 10: Full example
    let input = r#"kanban
  id1[Todo]
    docs[Create Documentation]
    blog[Create Blog about the new diagram]
  id7[In progress]
    id6[Create renderer so that it works in all cases]
    id8[Design grammar]@{ assigned: 'knsv' }
  id9[Ready for deploy]
  id10[Ready for test]
  id11[Done]
    id5[define getData]
    id2[Title of diagram]@{ ticket: MC-2036, priority: 'Very High'}
    id3[Update DB function]@{ ticket: MC-2037, assigned: knsv, priority: 'High' }
    id4[Create parsing tests]@{ ticket: MC-2038, assigned: 'K.Sveidqvist', priority: 'High' }
  id12[Can't reproduce]"#;

    let svg = render_kanban_svg(input);
    assert_valid_svg(&svg);

    // Should have multiple sections
    let doc = parse_svg(&svg);
    let section_count = count_elements_with_class(&doc, "section");
    assert!(section_count >= 6, "Should have at least 6 sections");

    // Should contain section titles
    assert!(svg_contains_text(&svg, "Todo"), "Should have Todo section");
    assert!(
        svg_contains_text(&svg, "In progress"),
        "Should have In progress section"
    );
    assert!(svg_contains_text(&svg, "Done"), "Should have Done section");
}

#[test]
fn kanban_empty_section() {
    // Test empty sections render correctly
    let input = r#"kanban
  id1[Todo]
  id2[In progress]
    task1[Working on it]
  id3[Done]"#;

    let svg = render_kanban_svg(input);
    assert_valid_svg(&svg);

    let doc = parse_svg(&svg);
    let section_count = count_elements_with_class(&doc, "section");
    assert_eq!(section_count, 3, "Should have 3 sections");
}

#[test]
fn kanban_section_colors() {
    // Test that sections have color classes
    let input = r#"kanban
  id1[Section 1]
    task1[Task 1]
  id2[Section 2]
    task2[Task 2]
  id3[Section 3]
    task3[Task 3]"#;

    let svg = render_kanban_svg(input);
    assert_valid_svg(&svg);

    let doc = parse_svg(&svg);
    // Check for section color classes
    assert!(has_class(&doc, "section-1"), "Should have section-1 class");
    assert!(has_class(&doc, "section-2"), "Should have section-2 class");
    assert!(has_class(&doc, "section-3"), "Should have section-3 class");
}
