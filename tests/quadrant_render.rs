//! Quadrant chart rendering tests
//!
//! Test cases ported from Mermaid.js Cypress tests (quadrantChart.spec.ts)

use selkie::render::{RenderConfig, Theme};
use selkie::{parse, render, render_with_config};

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
// Basic Chart Rendering Tests (from Cypress quadrantChart.spec.ts)
// ============================================================================

#[test]
fn test_minimal_quadrant_chart() {
    // From Cypress: should render a minimal quadrant chart
    let input = r#"quadrantChart
        Campaign A: [0.3, 0.6]"#;

    let diagram = parse(input).expect("Failed to parse minimal quadrant chart");
    let svg = render(&diagram).expect("Failed to render minimal quadrant chart");

    assert_valid_svg(&svg);
    assert!(svg.contains("Campaign A"), "Should contain point label");
    assert!(
        svg.contains("<circle"),
        "Should render data point as circle"
    );
}

#[test]
fn test_complete_quadrant_chart() {
    // From Cypress: should render a complete quadrant chart
    let input = r#"quadrantChart
        title Reach and engagement of campaigns
        x-axis Low Reach --> High Reach
        y-axis Low Engagement --> High Engagement
        quadrant-1 We should expand
        quadrant-2 Need to promote
        quadrant-3 Re-evaluate
        quadrant-4 May be improved
        Campaign A: [0.3, 0.6]
        Campaign B: [0.45, 0.23]
        Campaign C: [0.57, 0.69]
        Campaign D: [0.78, 0.34]
        Campaign E: [0.40, 0.34]
        Campaign F: [0.35, 0.78]"#;

    let diagram = parse(input).expect("Failed to parse complete quadrant chart");
    let svg = render(&diagram).expect("Failed to render complete quadrant chart");

    assert_valid_svg(&svg);
    assert!(
        svg.contains("Reach and engagement of campaigns"),
        "Should contain title"
    );
    assert!(
        svg.contains("Low Reach"),
        "Should contain x-axis left label"
    );
    assert!(
        svg.contains("High Reach"),
        "Should contain x-axis right label"
    );
    assert!(
        svg.contains("Low Engagement"),
        "Should contain y-axis bottom label"
    );
    assert!(
        svg.contains("High Engagement"),
        "Should contain y-axis top label"
    );
    assert!(
        svg.contains("We should expand"),
        "Should contain quadrant-1 label"
    );
    assert!(
        svg.contains("Need to promote"),
        "Should contain quadrant-2 label"
    );
    assert!(
        svg.contains("Re-evaluate"),
        "Should contain quadrant-3 label"
    );
    assert!(
        svg.contains("May be improved"),
        "Should contain quadrant-4 label"
    );

    // Check all campaign points are rendered
    assert!(svg.contains("Campaign A"), "Should contain Campaign A");
    assert!(svg.contains("Campaign B"), "Should contain Campaign B");
    assert!(svg.contains("Campaign C"), "Should contain Campaign C");
    assert!(svg.contains("Campaign D"), "Should contain Campaign D");
    assert!(svg.contains("Campaign E"), "Should contain Campaign E");
    assert!(svg.contains("Campaign F"), "Should contain Campaign F");
}

#[test]
fn test_quadrant_chart_without_title() {
    // From Cypress: should render without title
    let input = r#"quadrantChart
        x-axis Low Reach --> High Reach
        y-axis Low Engagement --> High Engagement
        quadrant-1 We should expand
        quadrant-2 Need to promote
        quadrant-3 Re-evaluate
        quadrant-4 May be improved
        Campaign A: [0.3, 0.6]"#;

    let diagram = parse(input).expect("Failed to parse quadrant chart without title");
    let svg = render(&diagram).expect("Failed to render quadrant chart without title");

    assert_valid_svg(&svg);
    // Should still render structure
    assert!(svg.contains("Low Reach"), "Should contain axis labels");
    assert!(svg.contains("Campaign A"), "Should contain data point");
}

#[test]
fn test_quadrant_chart_without_points() {
    // From Cypress: should render without points
    let input = r#"quadrantChart
        title Reach and engagement of campaigns
        x-axis Low Reach --> High Reach
        y-axis Low Engagement --> High Engagement
        quadrant-1 We should expand
        quadrant-2 Need to promote
        quadrant-3 Re-evaluate
        quadrant-4 May be improved"#;

    let diagram = parse(input).expect("Failed to parse quadrant chart without points");
    let svg = render(&diagram).expect("Failed to render quadrant chart without points");

    assert_valid_svg(&svg);
    assert!(
        svg.contains("Reach and engagement of campaigns"),
        "Should contain title"
    );
    // Quadrant structure should still render
    assert!(svg.contains("quadrant-1"), "Should render quadrant 1");
    assert!(svg.contains("quadrant-2"), "Should render quadrant 2");
    assert!(svg.contains("quadrant-3"), "Should render quadrant 3");
    assert!(svg.contains("quadrant-4"), "Should render quadrant 4");
}

#[test]
fn test_quadrant_chart_with_styled_points() {
    // From Cypress: should render with styled points
    let input = r#"quadrantChart
        title Reach and engagement of campaigns
        x-axis Low Reach --> High Reach
        y-axis Low Engagement --> High Engagement
        quadrant-1 We should expand
        quadrant-2 Need to promote
        quadrant-3 Re-evaluate
        quadrant-4 May be improved
        Campaign A: [0.3, 0.6] radius: 10
        Campaign B: [0.45, 0.23] color: #ff0000
        Campaign C: [0.57, 0.69] stroke-color: #ff00ff, stroke-width: 10px
        Campaign D: [0.78, 0.34] radius: 12, color: #ff0000, stroke-color: #ff00ff, stroke-width: 10px"#;

    let diagram = parse(input).expect("Failed to parse styled quadrant chart");
    let svg = render(&diagram).expect("Failed to render styled quadrant chart");

    assert_valid_svg(&svg);
    assert!(svg.contains("r=\"10\""), "Should contain custom radius");
    assert!(
        svg.contains("fill=\"#ff0000\""),
        "Should contain custom fill color"
    );
    assert!(
        svg.contains("stroke=\"#ff00ff\""),
        "Should contain custom stroke color"
    );
}

#[test]
fn test_quadrant_chart_with_class_styles() {
    // From Cypress: should render with class styles
    // Note: class_ref comes BEFORE the colon in the grammar: point_name ~ class_ref? ~ ":" ~ coordinates
    let input = r#"quadrantChart
        title Reach and engagement of campaigns
        x-axis Low Reach --> High Reach
        y-axis Low Engagement --> High Engagement
        quadrant-1 We should expand
        quadrant-2 Need to promote
        quadrant-3 Re-evaluate
        quadrant-4 May be improved
        Campaign A:::classA: [0.3, 0.6]
        classDef classA color: #ff0000"#;

    let diagram = parse(input).expect("Failed to parse quadrant chart with class styles");
    let svg = render(&diagram).expect("Failed to render quadrant chart with class styles");

    assert_valid_svg(&svg);
    assert!(svg.contains("Campaign A"), "Should contain data point");
}

// ============================================================================
// Point Position Tests
// ============================================================================

#[test]
fn test_point_in_quadrant_1() {
    // Point at (0.75, 0.75) should be in quadrant 1 (top-right)
    let input = r#"quadrantChart
        Q1 Point: [0.75, 0.75]"#;

    let diagram = parse(input).expect("Failed to parse");
    let svg = render(&diagram).expect("Failed to render");

    assert_valid_svg(&svg);
    assert!(svg.contains("Q1 Point"), "Should contain point label");
}

#[test]
fn test_point_in_quadrant_3() {
    // Point at (0.25, 0.25) should be in quadrant 3 (bottom-left)
    let input = r#"quadrantChart
        Q3 Point: [0.25, 0.25]"#;

    let diagram = parse(input).expect("Failed to parse");
    let svg = render(&diagram).expect("Failed to render");

    assert_valid_svg(&svg);
    assert!(svg.contains("Q3 Point"), "Should contain point label");
}

#[test]
fn test_point_at_center() {
    // Point at (0.5, 0.5) should be at the center crosshair
    let input = r#"quadrantChart
        Center: [0.5, 0.5]"#;

    let diagram = parse(input).expect("Failed to parse");
    let svg = render(&diagram).expect("Failed to render");

    assert_valid_svg(&svg);
    assert!(svg.contains("Center"), "Should contain point label");
}

// ============================================================================
// Edge Cases
// ============================================================================

#[test]
fn test_empty_quadrant_chart() {
    let input = r#"quadrantChart"#;

    let diagram = parse(input).expect("Failed to parse empty quadrant chart");
    let svg = render(&diagram).expect("Failed to render empty quadrant chart");

    assert_valid_svg(&svg);
    // Should still render basic structure with 4 quadrants
    assert!(svg.contains("quadrant-1"), "Should render quadrant 1");
    assert!(svg.contains("quadrant-2"), "Should render quadrant 2");
    assert!(svg.contains("quadrant-3"), "Should render quadrant 3");
    assert!(svg.contains("quadrant-4"), "Should render quadrant 4");
}

#[test]
fn test_point_at_extreme_positions() {
    // Points at corners
    let input = r#"quadrantChart
        TopRight: [1.0, 1.0]
        TopLeft: [0.0, 1.0]
        BottomLeft: [0.0, 0.0]
        BottomRight: [1.0, 0.0]"#;

    let diagram = parse(input).expect("Failed to parse");
    let svg = render(&diagram).expect("Failed to render");

    assert_valid_svg(&svg);
    assert!(svg.contains("TopRight"), "Should contain corner points");
    assert!(svg.contains("TopLeft"), "Should contain corner points");
    assert!(svg.contains("BottomLeft"), "Should contain corner points");
    assert!(svg.contains("BottomRight"), "Should contain corner points");
}

#[test]
fn test_quoted_labels() {
    let input = r#"quadrantChart
        title "Analytics Platform Evaluation"
        x-axis "Low Value" --> "High Value"
        y-axis "Low Cost" --> "High Cost"
        quadrant-1 "Premium Solutions"
        quadrant-2 "Budget Options"
        quadrant-3 "Avoid"
        quadrant-4 "Consider""#;

    let diagram = parse(input).expect("Failed to parse quoted labels");
    let svg = render(&diagram).expect("Failed to render quoted labels");

    assert_valid_svg(&svg);
    assert!(
        svg.contains("Analytics Platform Evaluation"),
        "Should contain quoted title"
    );
    assert!(
        svg.contains("Low Value"),
        "Should contain quoted axis label"
    );
    assert!(
        svg.contains("Premium Solutions"),
        "Should contain quoted quadrant label"
    );
}

// ============================================================================
// Gartner Magic Quadrant Style Tests
// ============================================================================

#[test]
fn test_gartner_style_chart() {
    // A complete Gartner Magic Quadrant style diagram
    // Note: axis syntax requires text after arrow if arrow is used
    let input = r#"quadrantChart
        title Analytics and Business Intelligence Platforms
        x-axis Completeness of Vision --> High Vision
        y-axis Ability to Execute --> High Execution
        quadrant-1 Leaders
        quadrant-2 Challengers
        quadrant-3 Niche Players
        quadrant-4 Visionaries
        Microsoft: [0.75, 0.75]
        Tableau: [0.68, 0.72]
        Salesforce: [0.55, 0.60]
        SAP: [0.70, 0.65]
        Qlik: [0.60, 0.45]
        Oracle: [0.65, 0.55]
        IBM: [0.51, 0.40]
        SAS: [0.45, 0.58]
        MicroStrategy: [0.50, 0.50]
        Alteryx: [0.35, 0.42]
        Sisense: [0.30, 0.35]
        ThoughtSpot: [0.25, 0.45]
        Domo: [0.20, 0.30]"#;

    let diagram = parse(input).expect("Failed to parse Gartner style chart");
    let svg = render(&diagram).expect("Failed to render Gartner style chart");

    assert_valid_svg(&svg);
    assert!(
        svg.contains("Analytics and Business Intelligence Platforms"),
        "Should contain title"
    );
    assert!(svg.contains("Leaders"), "Should contain quadrant labels");
    assert!(
        svg.contains("Microsoft"),
        "Should contain company data points"
    );
    assert!(
        svg.contains("Tableau"),
        "Should contain company data points"
    );
    assert!(
        svg.contains("ThoughtSpot"),
        "Should contain company data points"
    );
}

// ============================================================================
// Theme Tests
// ============================================================================

#[test]
fn test_quadrant_with_dark_theme() {
    let input = r#"quadrantChart
        title Dark Theme Test
        x-axis Low --> High
        y-axis Low --> High
        quadrant-1 Q1
        quadrant-2 Q2
        quadrant-3 Q3
        quadrant-4 Q4
        Point A: [0.5, 0.5]"#;

    let diagram = parse(input).expect("Failed to parse");
    let config = RenderConfig {
        theme: Theme::dark(),
        ..Default::default()
    };
    let svg = render_with_config(&diagram, &config).expect("Failed to render with dark theme");

    assert_valid_svg(&svg);
    // Dark theme should use dark quadrant colors
    assert!(
        svg.contains("#2a2a2a") || svg.contains("#3a3a3a"),
        "Should use dark quadrant fill colors"
    );
    // Dark theme should use light text colors
    assert!(
        svg.contains("#ccc"),
        "Should use light text color for dark theme"
    );
}

#[test]
fn test_quadrant_with_forest_theme() {
    let input = r#"quadrantChart
        title Forest Theme Test
        x-axis Low --> High
        y-axis Low --> High
        quadrant-1 Q1
        Point A: [0.75, 0.75]"#;

    let diagram = parse(input).expect("Failed to parse");
    let config = RenderConfig {
        theme: Theme::forest(),
        ..Default::default()
    };
    let svg = render_with_config(&diagram, &config).expect("Failed to render with forest theme");

    assert_valid_svg(&svg);
    // Forest theme should use green-ish colors
    assert!(
        svg.contains("#cde498") || svg.contains("#cdffb2"),
        "Should use forest green quadrant fill colors"
    );
}

#[test]
fn test_quadrant_with_neutral_theme() {
    let input = r#"quadrantChart
        title Neutral Theme Test
        quadrant-1 Q1
        Point A: [0.5, 0.5]"#;

    let diagram = parse(input).expect("Failed to parse");
    let config = RenderConfig {
        theme: Theme::neutral(),
        ..Default::default()
    };
    let svg = render_with_config(&diagram, &config).expect("Failed to render with neutral theme");

    assert_valid_svg(&svg);
    // Neutral theme should use grayscale colors
    assert!(
        svg.contains("#f0f0f0") || svg.contains("#e0e0e0"),
        "Should use neutral grayscale quadrant fill colors"
    );
}

#[test]
fn test_quadrant_with_base_theme() {
    let input = r#"quadrantChart
        title Base Theme Test
        quadrant-1 Q1
        Point A: [0.5, 0.5]"#;

    let diagram = parse(input).expect("Failed to parse");
    let config = RenderConfig {
        theme: Theme::base(),
        ..Default::default()
    };
    let svg = render_with_config(&diagram, &config).expect("Failed to render with base theme");

    assert_valid_svg(&svg);
    // Base theme should use warm pastel colors
    assert!(
        svg.contains("#fff4dd") || svg.contains("#dde4ff"),
        "Should use base theme warm pastel quadrant fill colors"
    );
}
