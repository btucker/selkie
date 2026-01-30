//! Requirement diagram rendering tests - ported from Cypress tests
//!
//! These tests are ported from the mermaid.js Cypress test suite:
//! - cypress/integration/rendering/requirement.spec.js
//! - cypress/integration/rendering/requirementDiagram-unified.spec.js

use roxmltree::Document;
use selkie::render::{render_text, render_with_config, RenderConfig, Theme};
use selkie::{parse, render};

fn render_requirement_svg(input: &str) -> String {
    let diagram = parse(input).expect("Failed to parse requirement diagram");
    render(&diagram).expect("Failed to render requirement diagram")
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

fn svg_contains_text(svg: &str, text: &str) -> bool {
    svg.contains(text)
}

// ============================================================================
// Basic Rendering Tests (from requirementDiagram-unified.spec.js)
// ============================================================================

#[test]
fn should_render_a_simple_requirement_diagram() {
    let input = r#"requirementDiagram
    requirement test_req {
    id: 1
    text: the test text.
    risk: high
    verifymethod: test
    }

    element test_entity {
    type: simulation
    }

    test_entity - satisfies -> test_req"#;

    let svg = render_requirement_svg(input);
    let doc = parse_svg(&svg);

    // Should have requirement and element nodes
    assert!(
        has_class(&doc, "requirement-node") || svg_contains_text(&svg, "test_req"),
        "Should render requirement node"
    );
    assert!(
        has_class(&doc, "element-node") || svg_contains_text(&svg, "test_entity"),
        "Should render element node"
    );

    // Should contain the requirement text
    assert!(
        svg_contains_text(&svg, "test_req"),
        "Should contain requirement name"
    );
    assert!(
        svg_contains_text(&svg, "test_entity"),
        "Should contain element name"
    );
}

#[test]
fn should_render_a_not_so_simple_requirement_diagram() {
    let input = r#"requirementDiagram

    requirement test_req {
    id: 1
    text: the test text.
    risk: high
    verifymethod: test
    }

    functionalRequirement test_req2 {
    id: 1.1
    text: the second test text.
    risk: low
    verifymethod: inspection
    }

    performanceRequirement test_req3 {
    id: 1.2
    text: the third test text.
    risk: medium
    verifymethod: demonstration
    }

    interfaceRequirement test_req4 {
    id: 1.2.1
    text: the fourth test text.
    risk: medium
    verifymethod: analysis
    }

    physicalRequirement test_req5 {
    id: 1.2.2
    text: the fifth test text.
    risk: medium
    verifymethod: analysis
    }

    designConstraint test_req6 {
    id: 1.2.3
    text: the sixth test text.
    risk: medium
    verifymethod: analysis
    }

    element test_entity {
    type: simulation
    }

    element test_entity2 {
    type: word doc
    docRef: reqs/test_entity
    }

    element test_entity3 {
    type: "test suite"
    docRef: github.com/all_the_tests
    }


    test_entity - satisfies -> test_req2
    test_req - traces -> test_req2
    test_req - contains -> test_req3
    test_req3 - contains -> test_req4
    test_req4 - derives -> test_req5
    test_req5 - refines -> test_req6
    test_entity3 - verifies -> test_req5"#;

    let svg = render_requirement_svg(input);

    // Should contain all requirement names
    assert!(
        svg_contains_text(&svg, "test_req"),
        "Should contain test_req"
    );
    assert!(
        svg_contains_text(&svg, "test_req2"),
        "Should contain test_req2"
    );
    assert!(
        svg_contains_text(&svg, "test_req3"),
        "Should contain test_req3"
    );
    assert!(
        svg_contains_text(&svg, "test_req4"),
        "Should contain test_req4"
    );
    assert!(
        svg_contains_text(&svg, "test_req5"),
        "Should contain test_req5"
    );
    assert!(
        svg_contains_text(&svg, "test_req6"),
        "Should contain test_req6"
    );

    // Should contain all element names
    assert!(
        svg_contains_text(&svg, "test_entity"),
        "Should contain test_entity"
    );
    assert!(
        svg_contains_text(&svg, "test_entity2"),
        "Should contain test_entity2"
    );
    assert!(
        svg_contains_text(&svg, "test_entity3"),
        "Should contain test_entity3"
    );
}

#[test]
fn should_render_requirement_diagram_with_empty_information() {
    let input = r#"requirementDiagram
    requirement test_req {
    }
    element test_entity {
    }"#;

    let svg = render_requirement_svg(input);

    assert!(
        svg_contains_text(&svg, "test_req"),
        "Should contain empty requirement"
    );
    assert!(
        svg_contains_text(&svg, "test_entity"),
        "Should contain empty element"
    );
}

#[test]
fn should_render_requirements_and_elements_with_and_without_information() {
    let input = r#"requirementDiagram
    requirement test_req {
        id: 1
        text: the test text.
        risk: high
        verifymethod: test
    }
    element test_entity {
    }"#;

    let svg = render_requirement_svg(input);

    assert!(
        svg_contains_text(&svg, "test_req"),
        "Should contain requirement with info"
    );
    assert!(
        svg_contains_text(&svg, "test_entity"),
        "Should contain element without info"
    );
}

#[test]
fn should_render_requirements_and_elements_with_long_and_short_text() {
    let input = r#"requirementDiagram
    requirement test_req {
        id: 1
        text: the test text that is long and takes up a lot of space.
        risk: high
        verifymethod: test
    }
    element test_entity_name_that_is_extra_long {
    }"#;

    let svg = render_requirement_svg(input);

    assert!(
        svg_contains_text(&svg, "test_req"),
        "Should contain requirement"
    );
    assert!(
        svg_contains_text(&svg, "test_entity_name_that_is_extra_long"),
        "Should contain element with long name"
    );
}

#[test]
fn should_render_requirements_and_elements_with_quoted_text_for_spaces() {
    let input = r#"requirementDiagram
    requirement "test req name with spaces" {
        id: 1
        text: the test text that is long and takes up a lot of space.
        risk: high
        verifymethod: test
    }
    element "test entity name that is extra long with spaces" {
    }"#;

    let svg = render_requirement_svg(input);

    assert!(
        svg_contains_text(&svg, "test req name with spaces"),
        "Should contain requirement with spaces in name"
    );
    assert!(
        svg_contains_text(&svg, "test entity name that is extra long with spaces"),
        "Should contain element with spaces in name"
    );
}

// ============================================================================
// Direction Tests
// ============================================================================

#[test]
fn should_render_requirement_diagram_with_tb_direction() {
    let input = r#"requirementDiagram
direction TB

requirement test_req {
id: 1
text: the test text.
risk: high
verifymethod: test
}

element test_entity {
type: simulation
}

test_entity - satisfies -> test_req"#;

    let svg = render_requirement_svg(input);
    let doc = parse_svg(&svg);

    assert!(
        doc.root_element().tag_name().name() == "svg",
        "Should produce valid SVG"
    );
    assert!(
        svg_contains_text(&svg, "test_req"),
        "Should contain requirement"
    );
}

#[test]
fn should_render_requirement_diagram_with_bt_direction() {
    let input = r#"requirementDiagram
direction BT

requirement test_req {
id: 1
text: the test text.
risk: high
verifymethod: test
}

element test_entity {
type: simulation
}

test_entity - satisfies -> test_req"#;

    let svg = render_requirement_svg(input);
    assert!(svg.contains("<svg"), "Should produce valid SVG");
}

#[test]
fn should_render_requirement_diagram_with_lr_direction() {
    let input = r#"requirementDiagram
direction LR

requirement test_req {
id: 1
text: the test text.
risk: high
verifymethod: test
}

element test_entity {
type: simulation
}

test_entity - satisfies -> test_req"#;

    let svg = render_requirement_svg(input);
    assert!(svg.contains("<svg"), "Should produce valid SVG");
}

#[test]
fn should_render_requirement_diagram_with_rl_direction() {
    let input = r#"requirementDiagram
direction RL

requirement test_req {
id: 1
text: the test text.
risk: high
verifymethod: test
}

element test_entity {
type: simulation
}

test_entity - satisfies -> test_req"#;

    let svg = render_requirement_svg(input);
    assert!(svg.contains("<svg"), "Should produce valid SVG");
}

// ============================================================================
// Styling Tests
// ============================================================================

#[test]
fn should_render_requirements_and_elements_with_styles_from_style_statement() {
    let input = r#"requirementDiagram

requirement test_req {
id: 1
text: the test text.
risk: high
verifymethod: test
}

element test_entity {
type: simulation
}

test_entity - satisfies -> test_req

style test_req,test_entity fill:#f9f,stroke:blue, color:grey, font-weight:bold"#;

    let svg = render_requirement_svg(input);

    assert!(
        svg_contains_text(&svg, "test_req"),
        "Should contain requirement"
    );
    assert!(
        svg_contains_text(&svg, "test_entity"),
        "Should contain element"
    );
}

#[test]
fn should_render_requirements_and_elements_with_styles_from_class_statement() {
    let input = r#"requirementDiagram

requirement test_req {
id: 1
text: the test text.
risk: high
verifymethod: test
}

element test_entity {
type: simulation
}

test_entity - satisfies -> test_req
classDef bold font-weight: bold
classDef blue stroke:lightblue, color: #0000FF
class test_entity bold
class test_req blue, bold"#;

    let svg = render_requirement_svg(input);

    assert!(
        svg_contains_text(&svg, "test_req"),
        "Should contain requirement"
    );
    assert!(
        svg_contains_text(&svg, "test_entity"),
        "Should contain element"
    );
}

#[test]
fn should_render_requirements_and_elements_with_classes_shorthand_syntax() {
    let input = r#"requirementDiagram

requirement test_req:::blue {
id: 1
text: the test text.
risk: high
verifymethod: test
}

element test_entity {
type: simulation
}

test_entity - satisfies -> test_req
classDef bold font-weight: bold
classDef blue stroke:lightblue, color: #0000FF
test_entity:::bold"#;

    let svg = render_requirement_svg(input);

    assert!(
        svg_contains_text(&svg, "test_req"),
        "Should contain requirement"
    );
    assert!(
        svg_contains_text(&svg, "test_entity"),
        "Should contain element"
    );
}

#[test]
fn should_render_requirements_with_default_class_and_other_styles() {
    let input = r#"requirementDiagram

requirement test_req:::blue {
id: 1
text: the test text.
risk: high
verifymethod: test
}

element test_entity {
type: simulation
}

test_entity - satisfies -> test_req
classDef blue stroke:lightblue, color:blue
classDef default fill:pink
style test_entity color:green"#;

    let svg = render_requirement_svg(input);

    assert!(
        svg_contains_text(&svg, "test_req"),
        "Should contain requirement"
    );
    assert!(
        svg_contains_text(&svg, "test_entity"),
        "Should contain element"
    );
}

// ============================================================================
// Relationship Type Tests
// ============================================================================

#[test]
fn should_render_all_relationship_types() {
    let input = r#"requirementDiagram
    requirement test_req {
    id: 1
    text: the test text.
    risk: high
    verifymethod: test
    }

    functionalRequirement test_req2 {
    id: 1.1
    text: the second test text.
    risk: low
    verifymethod: inspection
    }

    element test_entity {
    type: simulation
    }

    element test_entity2 {
    type: word doc
    docRef: reqs/test_entity
    }

    test_entity - satisfies -> test_req2
    test_req - traces -> test_req2
    test_req - contains -> test_req2
    test_entity2 - verifies -> test_req"#;

    let svg = render_requirement_svg(input);
    let doc = parse_svg(&svg);

    assert!(
        doc.root_element().tag_name().name() == "svg",
        "Should produce valid SVG"
    );

    // Should have relationship edges
    assert!(
        svg.contains("path") || svg.contains("line"),
        "Should have relationship lines"
    );
}

// ============================================================================
// Demo/Sample Tests (from demos/requirements.html)
// ============================================================================

#[test]
fn should_render_full_demo_diagram() {
    let input = r#"requirementDiagram

requirement test_req {
id: 1
text: the test text.
risk: high
verifymethod: test
}

functionalRequirement test_req2 {
id: 1.1
text: the second test text.
risk: low
verifymethod: inspection
}

performanceRequirement test_req3 {
id: 1.2
text: the third test text.
risk: medium
verifymethod: demonstration
}

interfaceRequirement test_req4 {
id: 1.2.1
text: the fourth test text.
risk: medium
verifymethod: analysis
}

physicalRequirement test_req5 {
id: 1.2.2
text: the fifth test text.
risk: medium
verifymethod: analysis
}

designConstraint test_req6 {
id: 1.2.3
text: the sixth test text.
risk: medium
verifymethod: analysis
}

element test_entity {
type: simulation
}

element test_entity2 {
type: word doc
docRef: reqs/test_entity
}

element test_entity3 {
type: "test suite"
docRef: github.com/all_the_tests
}

test_entity - satisfies -> test_req2
test_req - traces -> test_req2
test_req - contains -> test_req3
test_req3 - contains -> test_req4
test_req4 - derives -> test_req5
test_req5 - refines -> test_req6
test_entity3 - verifies -> test_req5"#;

    let svg = render_requirement_svg(input);

    // Verify all requirement types are rendered
    assert!(
        svg_contains_text(&svg, "test_req"),
        "Should contain requirement"
    );
    assert!(
        svg_contains_text(&svg, "test_req2"),
        "Should contain functionalRequirement"
    );
    assert!(
        svg_contains_text(&svg, "test_req3"),
        "Should contain performanceRequirement"
    );
    assert!(
        svg_contains_text(&svg, "test_req4"),
        "Should contain interfaceRequirement"
    );
    assert!(
        svg_contains_text(&svg, "test_req5"),
        "Should contain physicalRequirement"
    );
    assert!(
        svg_contains_text(&svg, "test_req6"),
        "Should contain designConstraint"
    );

    // Verify all elements are rendered
    assert!(
        svg_contains_text(&svg, "test_entity"),
        "Should contain element 1"
    );
    assert!(
        svg_contains_text(&svg, "test_entity2"),
        "Should contain element 2"
    );
    assert!(
        svg_contains_text(&svg, "test_entity3"),
        "Should contain element 3"
    );
}

#[test]
fn should_render_demo_with_descriptive_names() {
    let input = r#"requirementDiagram

requirement "An Example" {
id: 1
text: the test text.
risk: high
verifymethod: test
}

functionalRequirement "Random Name" {
id: 1.1
text: the second test text.
risk: low
verifymethod: inspection
}

performanceRequirement "Something Else" {
id: 1.2
text: the third test text.
risk: medium
verifymethod: demonstration
}

element test_entity {
type: simulation
}

test_entity - satisfies -> "Random Name"
"An Example" - traces -> "Random Name"
"An Example" - contains -> "Something Else""#;

    let svg = render_requirement_svg(input);

    assert!(
        svg_contains_text(&svg, "An Example"),
        "Should contain 'An Example'"
    );
    assert!(
        svg_contains_text(&svg, "Random Name"),
        "Should contain 'Random Name'"
    );
    assert!(
        svg_contains_text(&svg, "Something Else"),
        "Should contain 'Something Else'"
    );
}

// ============================================================================
// Edge Cases
// ============================================================================

#[test]
fn should_render_only_requirements_no_relationships() {
    let input = r#"requirementDiagram

requirement standalone_req {
id: 1
text: A standalone requirement.
risk: low
verifymethod: test
}

element standalone_element {
type: component
}"#;

    let svg = render_requirement_svg(input);

    assert!(
        svg_contains_text(&svg, "standalone_req"),
        "Should render standalone requirement"
    );
    assert!(
        svg_contains_text(&svg, "standalone_element"),
        "Should render standalone element"
    );
}

#[test]
fn should_render_requirement_with_all_attributes() {
    let input = r#"requirementDiagram

requirement complete_req {
id: REQ-001
text: This is a complete requirement with all attributes specified.
risk: high
verifymethod: analysis
}"#;

    let svg = render_requirement_svg(input);

    assert!(
        svg_contains_text(&svg, "complete_req"),
        "Should render requirement name"
    );
    // The SVG should contain the requirement details
    assert!(svg.contains("<svg"), "Should be valid SVG");
}

#[test]
fn should_render_element_with_docref() {
    let input = r#"requirementDiagram

element documented_element {
type: documentation
docRef: docs/specification.md
}"#;

    let svg = render_requirement_svg(input);

    assert!(
        svg_contains_text(&svg, "documented_element"),
        "Should render element name"
    );
}

// ============================================================================
// Requirement Type Display Tests
// ============================================================================

#[test]
fn should_display_requirement_type_labels() {
    let input = r#"requirementDiagram

requirement basic_req {
id: 1
}

functionalRequirement func_req {
id: 2
}

interfaceRequirement iface_req {
id: 3
}

performanceRequirement perf_req {
id: 4
}

physicalRequirement phys_req {
id: 5
}

designConstraint design_req {
id: 6
}"#;

    let svg = render_requirement_svg(input);
    let doc = parse_svg(&svg);

    // All requirement types should be rendered
    assert!(
        doc.root_element().tag_name().name() == "svg",
        "Should produce valid SVG"
    );

    // Check that each requirement is represented
    assert!(
        svg_contains_text(&svg, "basic_req"),
        "Should render basic requirement"
    );
    assert!(
        svg_contains_text(&svg, "func_req"),
        "Should render functional requirement"
    );
    assert!(
        svg_contains_text(&svg, "iface_req"),
        "Should render interface requirement"
    );
    assert!(
        svg_contains_text(&svg, "perf_req"),
        "Should render performance requirement"
    );
    assert!(
        svg_contains_text(&svg, "phys_req"),
        "Should render physical requirement"
    );
    assert!(
        svg_contains_text(&svg, "design_req"),
        "Should render design constraint"
    );
}

// ============================================================================
// Risk Level Display Tests
// ============================================================================

#[test]
fn should_handle_all_risk_levels() {
    let input = r#"requirementDiagram

requirement low_risk {
id: 1
risk: low
}

requirement medium_risk {
id: 2
risk: medium
}

requirement high_risk {
id: 3
risk: high
}"#;

    let svg = render_requirement_svg(input);

    assert!(
        svg_contains_text(&svg, "low_risk"),
        "Should render low risk requirement"
    );
    assert!(
        svg_contains_text(&svg, "medium_risk"),
        "Should render medium risk requirement"
    );
    assert!(
        svg_contains_text(&svg, "high_risk"),
        "Should render high risk requirement"
    );
}

// ============================================================================
// Verify Method Display Tests
// ============================================================================

#[test]
fn should_handle_all_verify_methods() {
    let input = r#"requirementDiagram

requirement analysis_req {
id: 1
verifymethod: analysis
}

requirement demonstration_req {
id: 2
verifymethod: demonstration
}

requirement inspection_req {
id: 3
verifymethod: inspection
}

requirement test_req {
id: 4
verifymethod: test
}"#;

    let svg = render_requirement_svg(input);

    assert!(
        svg_contains_text(&svg, "analysis_req"),
        "Should render analysis requirement"
    );
    assert!(
        svg_contains_text(&svg, "demonstration_req"),
        "Should render demonstration requirement"
    );
    assert!(
        svg_contains_text(&svg, "inspection_req"),
        "Should render inspection requirement"
    );
    assert!(
        svg_contains_text(&svg, "test_req"),
        "Should render test requirement"
    );
}

// ============================================================================
// Theme Tests
// ============================================================================

#[test]
fn should_render_with_dark_theme() {
    let input = r#"requirementDiagram
    requirement test_req {
    id: 1
    text: the test text.
    risk: high
    verifymethod: test
    }

    element test_entity {
    type: simulation
    }

    test_entity - satisfies -> test_req"#;

    let diagram = parse(input).expect("Failed to parse requirement diagram");
    let config = RenderConfig {
        theme: Theme::dark(),
        ..Default::default()
    };
    let svg = render_with_config(&diagram, &config).expect("Failed to render with dark theme");

    assert!(svg.contains("<svg"), "Should produce valid SVG");
    assert!(svg.contains("<style>"), "Should contain embedded styles");
    // Dark theme uses #1f2020 background
    assert!(svg.contains("#1f2020"), "Should use dark theme colors");
}

#[test]
fn should_render_with_forest_theme() {
    let input = r#"requirementDiagram
    requirement test_req {
    id: 1
    text: the test text.
    risk: high
    verifymethod: test
    }

    element test_entity {
    type: simulation
    }

    test_entity - satisfies -> test_req"#;

    let diagram = parse(input).expect("Failed to parse requirement diagram");
    let config = RenderConfig {
        theme: Theme::forest(),
        ..Default::default()
    };
    let svg = render_with_config(&diagram, &config).expect("Failed to render with forest theme");

    assert!(svg.contains("<svg"), "Should produce valid SVG");
    assert!(svg.contains("<style>"), "Should contain embedded styles");
}

#[test]
fn should_render_with_neutral_theme() {
    let input = r#"requirementDiagram
    requirement test_req {
    id: 1
    text: the test text.
    risk: high
    verifymethod: test
    }

    element test_entity {
    type: simulation
    }

    test_entity - satisfies -> test_req"#;

    let diagram = parse(input).expect("Failed to parse requirement diagram");
    let config = RenderConfig {
        theme: Theme::neutral(),
        ..Default::default()
    };
    let svg = render_with_config(&diagram, &config).expect("Failed to render with neutral theme");

    assert!(svg.contains("<svg"), "Should produce valid SVG");
    assert!(svg.contains("<style>"), "Should contain embedded styles");
}

#[test]
fn should_render_with_theme_directive() {
    let input = r##"%%{init: {"theme": "forest"}}%%
requirementDiagram
    requirement test_req {
    id: 1
    text: the test text.
    risk: high
    verifymethod: test
    }

    element test_entity {
    type: simulation
    }

    test_entity - satisfies -> test_req"##;

    let svg = render_text(input).expect("Failed to render with theme directive");

    assert!(svg.contains("<svg"), "Should produce valid SVG");
    // Forest theme uses green colors
    assert!(
        svg.contains("#cde498") || svg.contains("#cdffb2") || svg.contains("#008000"),
        "Should use forest theme colors"
    );
}

// ============================================================================
// Visual Parity Tests (for mermaid compatibility)
// ============================================================================

/// Test that requirement boxes have inline fill color for better SVG compatibility
/// Mermaid uses inline fill="#ECECFF" on path elements
/// Our eval expects inline fill colors, not just CSS classes
#[test]
fn should_have_inline_fill_color_on_requirement_boxes() {
    let input = r#"requirementDiagram
    requirement test_req {
    id: 1
    text: the test text.
    risk: high
    verifymethod: test
    }"#;

    let svg = render_requirement_svg(input);
    let doc = parse_svg(&svg);

    // Check that rect elements have inline fill attribute
    let has_inline_fill = doc.descendants().any(|node| {
        let tag = node.tag_name().name();
        if tag == "rect" {
            // Should have fill attribute directly on the element
            node.attribute("fill").is_some()
        } else {
            false
        }
    });

    assert!(
        has_inline_fill,
        "Requirement boxes should have inline fill attribute for SVG compatibility. \
         Current SVG relies only on CSS classes which may not be recognized by all viewers."
    );
}

/// Test that requirement boxes use the mermaid default fill color
/// Mermaid default theme uses #ECECFF (light purple/blue) for requirement boxes
#[test]
fn should_use_mermaid_default_fill_color() {
    let input = r#"requirementDiagram
    requirement test_req {
    id: 1
    text: the test text.
    risk: high
    verifymethod: test
    }"#;

    let svg = render_requirement_svg(input);

    // The default fill color should be #ECECFF (case insensitive)
    let has_correct_fill = svg.to_lowercase().contains("fill=\"#ececff\"")
        || svg.to_lowercase().contains("fill: #ececff")
        || svg.contains("fill=\"#ECECFF\"");

    assert!(
        has_correct_fill,
        "Requirement boxes should use mermaid default fill color #ECECFF. \
         SVG content: {}",
        &svg[..svg.len().min(500)]
    );
}

/// Test that top-to-bottom layout produces portrait-oriented diagrams
/// Mermaid TB layout produces diagrams that are taller than wide
#[test]
fn should_produce_portrait_aspect_ratio_for_tb_layout() {
    let input = r#"requirementDiagram

    requirement test_req {
    id: 1
    text: the test text.
    risk: high
    verifymethod: test
    }

    functionalRequirement test_req2 {
    id: 1.1
    text: the second test text.
    risk: low
    verifymethod: inspection
    }

    performanceRequirement test_req3 {
    id: 1.2
    text: the third test text.
    risk: medium
    verifymethod: demonstration
    }

    element test_entity {
    type: simulation
    }

    element test_entity2 {
    type: word doc
    docRef: reqs/test_entity
    }

    test_entity - satisfies -> test_req2
    test_req - traces -> test_req2
    test_req - contains -> test_req3
    test_entity2 - verifies -> test_req"#;

    let svg = render_requirement_svg(input);
    let doc = parse_svg(&svg);

    // Extract dimensions from viewBox or width/height
    let root = doc.root_element();
    let (width, height) = if let Some(viewbox) = root.attribute("viewBox") {
        let parts: Vec<f64> = viewbox
            .split_whitespace()
            .filter_map(|s| s.parse().ok())
            .collect();
        if parts.len() >= 4 {
            (parts[2], parts[3])
        } else {
            (400.0, 300.0)
        }
    } else {
        let w: f64 = root
            .attribute("width")
            .and_then(|s| s.trim_end_matches("px").parse().ok())
            .unwrap_or(400.0);
        let h: f64 = root
            .attribute("height")
            .and_then(|s| s.trim_end_matches("px").parse().ok())
            .unwrap_or(300.0);
        (w, h)
    };

    let aspect_ratio = width / height;

    // Mermaid TB layout produces portrait diagrams (aspect ratio < 1)
    // The reference diagram has aspect ratio ~0.82
    // Allow some tolerance, but should definitely be portrait
    assert!(
        aspect_ratio < 1.0,
        "TB layout should produce portrait diagram (height > width). \
         Current: width={}, height={}, aspect_ratio={}. \
         Expected aspect_ratio < 1.0 (portrait). Mermaid produces ~0.82",
        width,
        height,
        aspect_ratio
    );
}

/// Test that non-contains relationships use dashed lines (matches mermaid.js)
/// Mermaid uses stroke-dasharray: 10,7 for satisfies, traces, verifies, etc.
#[test]
fn should_use_dashed_lines_for_non_contains_relationships() {
    let input = r#"requirementDiagram
    requirement test_req {
    id: 1
    text: the test text.
    risk: high
    verifymethod: test
    }

    element test_entity {
    type: simulation
    }

    test_entity - satisfies -> test_req"#;

    let svg = render_requirement_svg(input);

    // The satisfies relationship should have dashed styling
    assert!(
        svg.contains("stroke-dasharray"),
        "Non-contains relationships (satisfies, traces, verifies) should have dashed lines. \
         Expected stroke-dasharray attribute but not found in SVG."
    );
}

/// Test that contains relationships use solid lines with start marker (matches mermaid.js)
/// Mermaid uses a circle+plus marker at the start of contains relationships
#[test]
fn should_use_solid_line_with_marker_for_contains_relationship() {
    let input = r#"requirementDiagram
    requirement parent_req {
    id: 1
    text: parent requirement
    risk: high
    verifymethod: test
    }

    requirement child_req {
    id: 2
    text: child requirement
    risk: low
    verifymethod: test
    }

    parent_req - contains -> child_req"#;

    let svg = render_requirement_svg(input);

    // The contains relationship should have the start marker
    assert!(
        svg.contains("marker-start=\"url(#requirement-contains-start)\""),
        "Contains relationship should have start marker (circle with +). \
         Expected marker-start attribute but not found."
    );

    // The contains relationship should NOT have dashed styling
    // Count the relationships - if there's only one relationship (contains),
    // there should be no stroke-dasharray on any path
    let contains_paths: Vec<&str> = svg
        .lines()
        .filter(|line| line.contains("marker-start=\"url(#requirement-contains-start)\""))
        .collect();

    for path in &contains_paths {
        assert!(
            !path.contains("stroke-dasharray"),
            "Contains relationship should use solid lines (no stroke-dasharray). \
             Found dashed styling on contains relationship."
        );
    }
}

/// Test that the contains start marker (circle with +) is defined
#[test]
fn should_have_contains_start_marker_definition() {
    let input = r#"requirementDiagram
    requirement test_req {
    id: 1
    }

    element test_entity {
    type: simulation
    }"#;

    let svg = render_requirement_svg(input);

    // Should have the contains-start marker defined
    assert!(
        svg.contains("id=\"requirement-contains-start\""),
        "SVG should define the requirement-contains-start marker for contains relationships."
    );

    // The marker should contain a circle and lines (the + symbol)
    assert!(
        svg.contains("<circle") && svg.contains("<line"),
        "The contains-start marker should contain a circle and lines (for the + symbol)."
    );
}

/// Test that CSS colors match mermaid reference
#[test]
fn should_use_correct_css_colors() {
    let input = r#"requirementDiagram
    requirement test_req {
    id: 1
    text: the test text.
    risk: high
    verifymethod: test
    }

    element test_entity {
    type: simulation
    }

    test_entity - satisfies -> test_req"#;

    let svg = render_requirement_svg(input);

    // Relationship labels should use 'black' fill (matching mermaid's .relationshipLabel)
    assert!(
        svg.contains("fill: black") || svg.contains("fill:black"),
        "Relationship labels should use 'black' fill to match mermaid reference. SVG CSS: {}",
        &svg[..svg.find("</style>").unwrap_or(500)]
    );

    // Relationship line stroke-width should be 1 (matching mermaid's .relationshipLine)
    assert!(
        svg.contains("stroke-width: 1;") || svg.contains("stroke-width:1;"),
        "Relationship lines should use stroke-width: 1 to match mermaid reference"
    );
}

/// Test that relationship line stroke-width is 1px (matching mermaid reference)
#[test]
fn should_use_stroke_width_1_for_relationship_lines() {
    let input = r#"requirementDiagram
    requirement test_req {
    id: 1
    text: the test text.
    risk: high
    verifymethod: test
    }

    element test_entity {
    type: simulation
    }

    test_entity - satisfies -> test_req"#;

    let svg = render_requirement_svg(input);

    // The CSS should define relationship-line with stroke-width: 1
    // NOT stroke-width: 1.5
    assert!(
        !svg.contains("stroke-width: 1.5"),
        "Relationship lines should NOT use stroke-width: 1.5 (mermaid uses 1)"
    );
}
