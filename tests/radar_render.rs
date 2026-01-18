//! Radar diagram rendering tests
//!
//! Test cases ported from Mermaid.js Cypress tests (radar.spec.js)

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
// Cypress Test Parity: radar.spec.js
// ============================================================================

#[test]
fn test_cypress_simple_radar() {
    // Cypress: should render a simple radar diagram
    let input = r#"radar-beta
                title Best Radar Ever
                axis A, B, C
                curve c1{1, 2, 3}"#;

    let diagram = parse(input).expect("Failed to parse radar diagram");
    let svg = render(&diagram).expect("Failed to render radar diagram");

    assert_valid_svg(&svg);
    // Verify title is rendered
    assert!(svg.contains("Best Radar Ever"), "Should contain title");
    // Verify we have a curve
    assert!(svg.contains("radarCurve"), "Should contain curve");
}

#[test]
fn test_cypress_multiple_curves() {
    // Cypress: should render a radar diagram with multiple curves
    let input = r#"radar-beta
                title Best Radar Ever
                axis A, B, C
                curve c1{1, 2, 3}
                curve c2{2, 3, 1}"#;

    let diagram = parse(input).expect("Failed to parse radar diagram");
    let svg = render(&diagram).expect("Failed to render radar diagram");

    assert_valid_svg(&svg);
    // Should have two curves
    assert!(svg.contains("radarCurve-0"), "Should contain first curve");
    assert!(svg.contains("radarCurve-1"), "Should contain second curve");
}

#[test]
fn test_cypress_complex_radar() {
    // Cypress: should render a complex radar diagram
    let input = r#"radar-beta
                title My favorite ninjas
                axis Agility, Speed, Strength
                axis Stam["Stamina"] , Intel["Intelligence"]

                curve Ninja1["Naruto Uzumaki"]{
                    Agility 2, Speed 2,
                    Strength 3, Stam 5,
                    Intel 0
                }
                curve Ninja2["Sasuke"]{2, 3, 4, 1, 5}
                curve Ninja3 {3, 2, 1, 5, 4}

                showLegend true
                ticks 3
                max 8
                min 0
                graticule polygon"#;

    let diagram = parse(input).expect("Failed to parse complex radar");
    let svg = render(&diagram).expect("Failed to render complex radar");

    assert_valid_svg(&svg);
    assert!(svg.contains("My favorite ninjas"), "Should contain title");
    // Should have polygon graticule
    assert!(
        svg.contains("<polygon") && svg.contains("radarGraticule"),
        "Should have polygon graticule"
    );
    // Should have three curves
    assert!(svg.contains("radarCurve-0"), "Should have curve 0");
    assert!(svg.contains("radarCurve-1"), "Should have curve 1");
    assert!(svg.contains("radarCurve-2"), "Should have curve 2");
    // Should have legend
    assert!(svg.contains("radarLegend"), "Should have legend");
}

#[test]
fn test_cypress_config_override() {
    // Cypress: should render radar diagram with config override
    let input = r#"radar-beta
                title Best Radar Ever
                axis A,B,C
                curve mycurve{1,2,3}"#;

    let diagram = parse(input).expect("Failed to parse radar");
    let svg = render(&diagram).expect("Failed to render radar");

    assert_valid_svg(&svg);
}

#[test]
fn test_cypress_theme_override() {
    // Cypress: should parse radar diagram with theme override
    let input = r#"radar-beta
                axis A,B,C
                curve mycurve{1,2,3}"#;

    let diagram = parse(input).expect("Failed to parse radar");
    let config = RenderConfig {
        theme: Theme::dark(),
        ..RenderConfig::default()
    };
    let svg = render_with_config(&diagram, &config).expect("Failed to render radar");

    assert_valid_svg(&svg);
}

#[test]
fn test_cypress_radar_style_override() {
    // Cypress: should handle radar diagram with radar style override
    let input = r#"radar-beta
                axis A,B,C
                curve mycurve{1,2,3}"#;

    let diagram = parse(input).expect("Failed to parse radar");
    let svg = render(&diagram).expect("Failed to render radar");

    assert_valid_svg(&svg);
}

// ============================================================================
// Additional rendering tests
// ============================================================================

#[test]
fn test_radar_without_title() {
    let input = r#"radar-beta
                axis A,B,C
                curve mycurve{1,2,3}"#;

    let diagram = parse(input).expect("Failed to parse radar");
    let svg = render(&diagram).expect("Failed to render radar");

    assert_valid_svg(&svg);
}

#[test]
fn test_radar_circle_graticule() {
    let input = r#"radar-beta
                axis A,B,C
                curve mycurve{1,2,3}
                graticule circle"#;

    let diagram = parse(input).expect("Failed to parse radar");
    let svg = render(&diagram).expect("Failed to render radar");

    assert_valid_svg(&svg);
    // Circle graticule uses circles
    assert!(
        svg.contains("<circle") && svg.contains("radarGraticule"),
        "Should have circle graticule"
    );
}

#[test]
fn test_radar_polygon_graticule() {
    let input = r#"radar-beta
                axis A,B,C
                curve mycurve{1,2,3}
                graticule polygon"#;

    let diagram = parse(input).expect("Failed to parse radar");
    let svg = render(&diagram).expect("Failed to render radar");

    assert_valid_svg(&svg);
    // Polygon graticule uses polygons
    assert!(
        svg.contains("<polygon") && svg.contains("radarGraticule"),
        "Should have polygon graticule"
    );
}

#[test]
fn test_radar_custom_ticks() {
    let input = r#"radar-beta
                axis A,B,C,D
                curve c{1,2,3,4}
                ticks 3"#;

    let diagram = parse(input).expect("Failed to parse radar");
    let svg = render(&diagram).expect("Failed to render radar");

    assert_valid_svg(&svg);
    // Should have exactly 3 graticule SVG elements (circles with the class)
    // We count actual circle elements with the radarGraticule class
    let count = svg.matches(r#"class="radarGraticule""#).count();
    assert_eq!(count, 3, "Should have 3 graticule elements for 3 ticks");
}

#[test]
fn test_radar_legend_shown() {
    let input = r#"radar-beta
                axis A,B,C
                curve c1{1,2,3}
                curve c2{3,2,1}
                showLegend true"#;

    let diagram = parse(input).expect("Failed to parse radar");
    let svg = render(&diagram).expect("Failed to render radar");

    assert_valid_svg(&svg);
    assert!(svg.contains("radarLegend"), "Should have legend");
}

#[test]
fn test_radar_legend_hidden() {
    let input = r#"radar-beta
                axis A,B,C
                curve c1{1,2,3}
                showLegend false"#;

    let diagram = parse(input).expect("Failed to parse radar");
    let svg = render(&diagram).expect("Failed to render radar");

    assert_valid_svg(&svg);
    // Legend should not be present
    assert!(
        !svg.contains("radarLegendBox"),
        "Should not have legend when showLegend is false"
    );
}

#[test]
fn test_radar_axes_labels() {
    let input = r#"radar-beta
                axis Speed["Top Speed"], Power["Max Power"], Agility
                curve c{1,2,3}"#;

    let diagram = parse(input).expect("Failed to parse radar");
    let svg = render(&diagram).expect("Failed to render radar");

    assert_valid_svg(&svg);
    assert!(
        svg.contains("Top Speed"),
        "Should contain 'Top Speed' label"
    );
    assert!(
        svg.contains("Max Power"),
        "Should contain 'Max Power' label"
    );
    assert!(svg.contains("Agility"), "Should contain 'Agility' label");
}

#[test]
fn test_radar_min_max_values() {
    let input = r#"radar-beta
                axis A,B,C
                curve c{5,6,7}
                min 0
                max 10"#;

    let diagram = parse(input).expect("Failed to parse radar");
    let svg = render(&diagram).expect("Failed to render radar");

    assert_valid_svg(&svg);
}

#[test]
fn test_radar_curve_labels() {
    let input = r#"radar-beta
                axis A,B,C
                curve c1["Curve One"]{1,2,3}
                curve c2["Curve Two"]{3,2,1}
                showLegend true"#;

    let diagram = parse(input).expect("Failed to parse radar");
    let svg = render(&diagram).expect("Failed to render radar");

    assert_valid_svg(&svg);
    // Legend should show curve labels
    assert!(
        svg.contains("Curve One"),
        "Should contain 'Curve One' label"
    );
    assert!(
        svg.contains("Curve Two"),
        "Should contain 'Curve Two' label"
    );
}

#[test]
fn test_radar_dark_theme() {
    let input = r#"radar-beta
                axis A,B,C
                curve c{1,2,3}"#;

    let diagram = parse(input).expect("Failed to parse radar");
    let config = RenderConfig {
        theme: Theme::dark(),
        ..RenderConfig::default()
    };
    let svg = render_with_config(&diagram, &config).expect("Failed to render radar");

    assert_valid_svg(&svg);
}

#[test]
fn test_radar_forest_theme() {
    let input = r#"radar-beta
                axis A,B,C
                curve c{1,2,3}"#;

    let diagram = parse(input).expect("Failed to parse radar");
    let config = RenderConfig {
        theme: Theme::forest(),
        ..RenderConfig::default()
    };
    let svg = render_with_config(&diagram, &config).expect("Failed to render radar");

    assert_valid_svg(&svg);
}

#[test]
fn test_radar_five_axes() {
    let input = r#"radar-beta
                axis A, B, C, D, E
                curve c{1, 2, 3, 4, 5}"#;

    let diagram = parse(input).expect("Failed to parse radar");
    let svg = render(&diagram).expect("Failed to render radar");

    assert_valid_svg(&svg);
    // Should have 5 axis lines and labels (count actual elements, not CSS)
    assert_eq!(
        svg.matches(r#"class="radarAxisLine""#).count(),
        5,
        "Should have 5 axis lines"
    );
    assert_eq!(
        svg.matches(r#"class="radarAxisLabel""#).count(),
        5,
        "Should have 5 axis labels"
    );
}

#[test]
fn test_radar_decimal_values() {
    let input = r#"radar-beta
                axis A,B,C
                curve c{1.5, 2.7, 3.9}"#;

    let diagram = parse(input).expect("Failed to parse radar");
    let svg = render(&diagram).expect("Failed to render radar");

    assert_valid_svg(&svg);
}
