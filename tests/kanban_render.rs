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

// ============================================================================
// Visual Parity Tests - Testing mermaid.js compatibility
// ============================================================================

/// Helper to count rects with inline fill
fn count_rects_with_inline_fill(doc: &Document<'_>) -> usize {
    doc.descendants()
        .filter(|node| {
            node.tag_name().name() == "rect"
                && node
                    .attribute("style")
                    .map(|s| s.contains("fill"))
                    .unwrap_or(false)
        })
        .count()
}

#[test]
fn kanban_visual_parity_section_inline_fill() {
    // Mermaid.js uses inline fill styles on section rects for proper color rendering
    // This prevents CSS class conflicts and ensures colors display correctly
    let input = r#"kanban
  id1[Todo]
    task1[Task 1]
  id2[Done]
    task2[Task 2]"#;

    let svg = render_kanban_svg(input);
    let doc = parse_svg(&svg);

    // Section rects should have inline fill style (like mermaid.js)
    let section_rects_with_fill = count_rects_with_inline_fill(&doc);
    assert!(
        section_rects_with_fill >= 2,
        "Section rects should have inline fill style for mermaid visual parity (found {} rects with inline fill)",
        section_rects_with_fill
    );
}

#[test]
fn kanban_visual_parity_section_gap() {
    // Mermaid.js has 5px gap between sections (SECTION_GAP = 5 in kanbanRenderer.ts)
    let input = r#"kanban
  id1[Todo]
    task1[Task 1]
  id2[Done]
    task2[Task 2]"#;

    let svg = render_kanban_svg(input);

    // Section 1 should end at x=200, Section 2 should start at x=205 (5px gap)
    // Check that we have proper 5px gaps between sections
    assert!(
        svg.contains(r#"x="205""#) || svg.contains(r#"x="205.0""#),
        "Second section should start at x=205 (200 width + 5px gap)\nSVG: {}",
        svg
    );
}

#[test]
fn kanban_visual_parity_node_rect_stroke() {
    // Mermaid.js kanban items have stroke="#9370DB" (MediumPurple)
    let input = r#"kanban
  id1[Todo]
    task1[Task 1]"#;

    let svg = render_kanban_svg(input);

    // Items should have MediumPurple stroke color
    assert!(
        svg.contains("#9370DB") || svg.contains("#9370db") || svg.contains("stroke:#9370"),
        "Kanban items should have MediumPurple (#9370DB) stroke\nSVG: {}",
        svg
    );
}

#[test]
fn kanban_visual_parity_section_corner_radius() {
    // Mermaid.js uses rx="5" ry="5" for section and item rectangles
    let input = r#"kanban
  id1[Todo]
    task1[Task 1]"#;

    let svg = render_kanban_svg(input);

    // Should have rx="5" and ry="5" for rounded corners
    assert!(
        svg.contains(r#"rx="5""#),
        "Section/item rects should have rx=5 for rounded corners"
    );
    assert!(
        svg.contains(r#"ry="5""#),
        "Section/item rects should have ry=5 for rounded corners"
    );
}

#[test]
fn kanban_visual_parity_item_width() {
    // Mermaid.js kanban items are 185px wide (SECTION_WIDTH - 2*PADDING = 200 - 15 = 185)
    // Actually mermaid uses 180px items inside 200px sections
    let input = r#"kanban
  id1[Todo]
    task1[Task 1]"#;

    let svg = render_kanban_svg(input);

    // Items should be 180px wide
    assert!(
        svg.contains(r#"width="180""#),
        "Kanban items should be 180px wide for mermaid parity"
    );
}

#[test]
fn kanban_visual_parity_text_anchor() {
    // Mermaid.js uses text-anchor="middle" for centered text in items
    let input = r#"kanban
  id1[Todo]
    task1[Task 1]"#;

    let svg = render_kanban_svg(input);

    // Text should be centered
    assert!(
        svg.contains(r#"text-anchor="middle""#),
        "Kanban text should use text-anchor=middle for centering"
    );
}

#[test]
fn kanban_visual_parity_item_inline_fill() {
    // Mermaid.js kanban items have inline fill="white" for reliable rendering
    let input = r#"kanban
  id1[Todo]
    task1[Task 1]"#;

    let svg = render_kanban_svg(input);
    let doc = parse_svg(&svg);

    // Item rects (kanban-item class) should have inline fill style
    let item_rects_with_fill: Vec<_> = doc
        .descendants()
        .filter(|node| {
            node.tag_name().name() == "rect"
                && node
                    .attribute("class")
                    .map(|c| c.contains("kanban-item"))
                    .unwrap_or(false)
        })
        .filter(|node| {
            node.attribute("style")
                .map(|s| s.contains("fill"))
                .unwrap_or(false)
        })
        .collect();

    assert!(
        !item_rects_with_fill.is_empty(),
        "Kanban item rects should have inline fill style for mermaid visual parity"
    );
}

#[test]
fn kanban_visual_parity_section_label_position() {
    // Mermaid.js section labels are positioned at the top of sections
    // The text should be vertically centered in the header area (around y=22 for default padding)
    let input = r#"kanban
  id1[Todo]
    task1[Task 1]"#;

    let svg = render_kanban_svg(input);

    // Section label should be positioned near the top (y around 20-25)
    assert!(
        svg.contains(r#"y="22""#) || svg.contains(r#"y="22.0""#),
        "Section label should be positioned at y=22 for proper header alignment\nSVG: {}",
        svg
    );
}

#[test]
fn kanban_visual_parity_font_family() {
    // Mermaid.js uses "trebuchet ms, verdana, arial, sans-serif" as default font
    let input = r#"kanban
  id1[Todo]
    task1[Task 1]"#;

    let svg = render_kanban_svg(input);

    // Should include the default mermaid font family
    assert!(
        svg.contains("trebuchet ms") || svg.contains("Trebuchet"),
        "Kanban should use mermaid's default font family (trebuchet ms)"
    );
}

#[test]
fn kanban_visual_parity_priority_indicator() {
    // Test that priority indicator lines are rendered with proper styling
    let input = r#"kanban
  id1[Todo]
    task1[High Priority]@{ priority: 'High' }"#;

    let svg = render_kanban_svg(input);

    // Should have a line element with priority-indicator class
    assert!(
        svg.contains("priority-indicator"),
        "Priority items should have a priority-indicator line"
    );

    // Should have stroke color for priority
    assert!(
        svg.contains("stroke=\"orange\"") || svg.contains("stroke:orange"),
        "High priority should have orange stroke"
    );
}

#[test]
fn kanban_visual_parity_metadata_positioning() {
    // Test that ticket and assigned metadata are positioned correctly
    let input = r#"kanban
  id1[Todo]
    task1[Task]@{ ticket: 'TKT-123', assigned: 'bob' }"#;

    let svg = render_kanban_svg(input);

    // Should contain ticket and assigned
    assert!(svg.contains("TKT-123"), "Should render ticket number");
    assert!(svg.contains("bob"), "Should render assigned person");

    // Ticket should be left-aligned (text-anchor="start")
    assert!(
        svg.contains(r#"class="kanban-ticket"#),
        "Should have kanban-ticket class for ticket styling"
    );
}
