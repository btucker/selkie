//! XY Chart rendering tests
//!
//! Test cases ported from Mermaid.js Cypress tests (xyChart.spec.js)

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
// Basic Chart Rendering Tests (from Cypress xyChart.spec.js)
// ============================================================================

#[test]
fn test_simplest_xy_beta_chart() {
    // From Cypress: should render the simplest possible xy-beta chart
    let input = r#"xychart-beta
        line [10, 30, 20]"#;

    let diagram = parse(input).expect("Failed to parse xychart-beta");
    let svg = render(&diagram).expect("Failed to render xychart-beta");

    assert_valid_svg(&svg);
}

#[test]
fn test_simplest_xy_chart() {
    // From Cypress: should render the simplest possible xy chart
    let input = r#"xychart
        line [10, 30, 20]"#;

    let diagram = parse(input).expect("Failed to parse xychart");
    let svg = render(&diagram).expect("Failed to render xychart");

    assert_valid_svg(&svg);
}

#[test]
fn test_complete_chart() {
    // From Cypress: Should render a complete chart
    let input = r#"xychart
        title "Sales Revenue"
        x-axis Months [jan, feb, mar, apr, may, jun, jul, aug, sep, oct, nov, dec]
        y-axis "Revenue (in $)" 4000 --> 11000
        bar [5000, 6000, 7500, 8200, 9500, 10500, 11000, 10200, 9200, 8500, 7000, 6000]
        line [5000, 6000, 7500, 8200, 9500, 10500, 11000, 10200, 9200, 8500, 7000, 6000]"#;

    let diagram = parse(input).expect("Failed to parse complete chart");
    let svg = render(&diagram).expect("Failed to render complete chart");

    assert_valid_svg(&svg);
    assert!(svg.contains("Sales Revenue"), "Should contain title");
}

#[test]
fn test_chart_without_title() {
    // From Cypress: Should render a chart without title
    let input = r#"xychart
        x-axis Months [jan, feb, mar, apr, may, jun, jul, aug, sep, oct, nov, dec]
        y-axis "Revenue (in $)" 4000 --> 11000
        bar [5000, 6000, 7500, 8200, 9500, 10500, 11000, 10200, 9200, 8500, 7000, 6000]
        line [5000, 6000, 7500, 8200, 9500, 10500, 11000, 10200, 9200, 8500, 7000, 6000]"#;

    let diagram = parse(input).expect("Failed to parse chart without title");
    let svg = render(&diagram).expect("Failed to render chart without title");

    assert_valid_svg(&svg);
}

#[test]
fn test_y_axis_title_not_required() {
    // From Cypress: y-axis title not required
    let input = r#"xychart
        x-axis Months [jan, feb, mar, apr, may, jun, jul, aug, sep, oct, nov, dec]
        y-axis 4000 --> 11000
        bar [5000, 6000, 7500, 8200, 9500, 10500, 11000, 10200, 9200, 8500, 7000, 6000]
        line [5000, 6000, 7500, 8200, 9500, 10500, 11000, 10200, 9200, 8500, 7000, 6000]"#;

    let diagram = parse(input).expect("Failed to parse chart");
    let svg = render(&diagram).expect("Failed to render chart");

    assert_valid_svg(&svg);
}

#[test]
fn test_chart_without_y_axis_different_range() {
    // From Cypress: Should render a chart without y-axis with different range
    let input = r#"xychart
        x-axis Months [jan, feb, mar, apr, may, jun, jul, aug, sep, oct, nov, dec]
        bar [5000, 6000, 7500, 8200, 9500, 10500, 14000, 3200, 9200, 9900, 3400, 6000]
        line [2000, 7000, 6500, 9200, 9500, 7500, 11000, 10200, 3200, 8500, 7000, 8800]"#;

    let diagram = parse(input).expect("Failed to parse chart");
    let svg = render(&diagram).expect("Failed to render chart");

    assert_valid_svg(&svg);
}

#[test]
fn test_x_axis_title_not_required() {
    // From Cypress: x axis title not required
    let input = r#"xychart
        x-axis [jan, feb, mar, apr, may, jun, jul, aug, sep, oct, nov, dec]
        bar [5000, 6000, 7500, 8200, 9500, 10500, 14000, 3200, 9200, 9900, 3400, 6000]
        line [2000, 7000, 6500, 9200, 9500, 7500, 11000, 10200, 3200, 8500, 7000, 8800]"#;

    let diagram = parse(input).expect("Failed to parse chart");
    let svg = render(&diagram).expect("Failed to render chart");

    assert_valid_svg(&svg);
}

#[test]
fn test_multiple_plots() {
    // From Cypress: Multiple plots can be rendered
    let input = r#"xychart
        line [23, 46, 77, 34]
        line [45, 32, 33, 12]
        bar [87, 54, 99, 85]
        line [78, 88, 22, 4]
        line [22, 29, 75, 33]
        bar [52, 96, 35, 10]"#;

    let diagram = parse(input).expect("Failed to parse chart with multiple plots");
    let svg = render(&diagram).expect("Failed to render chart with multiple plots");

    assert_valid_svg(&svg);
}

#[test]
fn test_decimals_and_negatives() {
    // From Cypress: Decimals and negative numbers are supported
    let input = r#"xychart
        y-axis -2.4 --> 3.5
        line [+1.3, 0.6, 2.4, -0.34]"#;

    let diagram = parse(input).expect("Failed to parse chart with decimals");
    let svg = render(&diagram).expect("Failed to render chart with decimals");

    assert_valid_svg(&svg);
}

#[test]
fn test_correct_distances_between_data_points() {
    // From Cypress: should use the correct distances between data points
    let input = r#"xychart
        x-axis 0 --> 2
        line [0, 1, 0, 1]
        bar [1, 0, 1, 0]"#;

    let diagram = parse(input).expect("Failed to parse chart");
    let svg = render(&diagram).expect("Failed to render chart");

    assert_valid_svg(&svg);
}

// ============================================================================
// Orientation Tests
// ============================================================================

#[test]
fn test_horizontal_orientation() {
    // From Cypress demo: XY Charts horizontal
    let input = r#"xychart horizontal
        title "Basic xychart"
        x-axis "this is x axis" [category1, "category 2", category3, category4]
        y-axis yaxisText 10 --> 150
        bar "sample bar" [52, 96, 35, 10]
        line [23, 46, 75, 43]"#;

    let diagram = parse(input).expect("Failed to parse horizontal chart");
    let svg = render(&diagram).expect("Failed to render horizontal chart");

    assert_valid_svg(&svg);
    assert!(svg.contains("Basic xychart"), "Should contain title");
}

#[test]
fn test_vertical_bar_chart_with_labels() {
    // From Cypress: should render vertical bar chart with labels
    let input = r#"xychart
        title "Sales Revenue"
        x-axis Months [jan, feb, mar, apr, may, jun, jul, aug, sep, oct, nov, dec]
        y-axis "Revenue (in $)" 4000 --> 11000
        bar [5000, 6000, 7500, 8200, 9500, 10500, 11000, 10200, 9200, 8500, 7000, 6000]"#;

    let diagram = parse(input).expect("Failed to parse chart");
    let svg = render(&diagram).expect("Failed to render chart");

    assert_valid_svg(&svg);
}

#[test]
fn test_horizontal_bar_chart_without_labels_default() {
    // From Cypress: should render horizontal bar chart without labels by default
    let input = r#"xychart horizontal
        title "Sales Revenue"
        x-axis Months [jan, feb, mar, apr, may, jun, jul, aug, sep, oct, nov, dec]
        y-axis "Revenue (in $)" 4000 --> 11000
        bar [5000, 6000, 7500, 8200, 9500, 10500, 11000, 10200, 9200, 8500, 7000, 6000]"#;

    let diagram = parse(input).expect("Failed to parse chart");
    let svg = render(&diagram).expect("Failed to render chart");

    assert_valid_svg(&svg);
}

// ============================================================================
// Multiple Bar Plots Tests
// ============================================================================

#[test]
fn test_multiple_bar_plots_vertical() {
    // From Cypress: should render multiple bar plots vertically with labels correctly
    let input = r#"xychart
        title "Multiple Bar Plots"
        x-axis Categories [A, B, C]
        y-axis "Values" 0 --> 100
        bar [10, 50, 90]"#;

    let diagram = parse(input).expect("Failed to parse chart");
    let svg = render(&diagram).expect("Failed to render chart");

    assert_valid_svg(&svg);
}

#[test]
fn test_multiple_bar_plots_horizontal() {
    // From Cypress: should render multiple bar plots horizontally with labels correctly
    let input = r#"xychart horizontal
        title "Multiple Bar Plots"
        x-axis Categories [A, B, C]
        y-axis "Values" 0 --> 100
        bar [10, 50, 90]"#;

    let diagram = parse(input).expect("Failed to parse chart");
    let svg = render(&diagram).expect("Failed to render chart");

    assert_valid_svg(&svg);
}

#[test]
fn test_single_bar_vertical() {
    // From Cypress: should render a single bar with label for a vertical xy-chart
    let input = r#"xychart
        title "Single Bar Chart"
        x-axis Categories [A]
        y-axis "Value" 0 --> 100
        bar [75]"#;

    let diagram = parse(input).expect("Failed to parse chart");
    let svg = render(&diagram).expect("Failed to render chart");

    assert_valid_svg(&svg);
}

#[test]
fn test_single_bar_horizontal() {
    // From Cypress: should render a single bar with label for a horizontal xy-chart
    let input = r#"xychart horizontal
        title "Single Bar Chart"
        x-axis Categories [A]
        y-axis "Value" 0 --> 100
        bar [75]"#;

    let diagram = parse(input).expect("Failed to parse chart");
    let svg = render(&diagram).expect("Failed to render chart");

    assert_valid_svg(&svg);
}

#[test]
fn test_negative_decimal_values_vertical() {
    // From Cypress: should render negative and decimal values with correct labels for vertical xy-chart
    let input = r#"xychart
        title "Decimal and Negative Values"
        x-axis Categories [A, B, C]
        y-axis -10 --> 10
        bar [ -2.5, 0.75, 5.1 ]"#;

    let diagram = parse(input).expect("Failed to parse chart");
    let svg = render(&diagram).expect("Failed to render chart");

    assert_valid_svg(&svg);
}

#[test]
fn test_negative_decimal_values_horizontal() {
    // From Cypress: should render negative and decimal values with correct labels for horizontal xy-chart
    let input = r#"xychart horizontal
        title "Decimal and Negative Values"
        x-axis Categories [A, B, C]
        y-axis -10 --> 10
        bar [ -2.5, 0.75, 5.1 ]"#;

    let diagram = parse(input).expect("Failed to parse chart");
    let svg = render(&diagram).expect("Failed to render chart");

    assert_valid_svg(&svg);
}

// ============================================================================
// Many Bars Tests
// ============================================================================

#[test]
fn test_many_bars_vertical() {
    // From Cypress: should render data labels within each bar in the vertical xy-chart with a lot of bars
    let input = r#"xychart
        title "Sales Revenue"
        x-axis Months [jan, feb, mar, apr, may, jun, jul, aug, sep, oct, nov, dec]
        y-axis "Revenue (in $)" 4000 --> 12000
        bar [5000, 6000, 7500, 8200, 9500, 10500, 11000, 10200, 9200, 8500, 7000, 6000]"#;

    let diagram = parse(input).expect("Failed to parse chart");
    let svg = render(&diagram).expect("Failed to render chart");

    assert_valid_svg(&svg);
    // Verify bar elements exist
    assert!(
        svg.contains("<rect"),
        "Should contain rect elements for bars"
    );
}

#[test]
fn test_many_bars_horizontal() {
    // From Cypress: should render data labels within each bar in the horizontal xy-chart with a lot of bars
    let input = r#"xychart horizontal
        title "Sales Revenue"
        x-axis Months [jan, feb, mar, apr, may, jun, jul, aug, sep, oct, nov, dec]
        y-axis "Revenue (in $)" 4000 --> 12000
        bar [5000, 6000, 7500, 8200, 9500, 10500, 11000, 10200, 9200, 8500, 7000, 6000]"#;

    let diagram = parse(input).expect("Failed to parse chart");
    let svg = render(&diagram).expect("Failed to render chart");

    assert_valid_svg(&svg);
}

#[test]
fn test_many_different_size_bars_vertical() {
    // From Cypress: should render data labels within each bar in the vertical xy-chart with a lot of bars of different sizes
    let input = r#"xychart
        title "Sales Revenue"
        x-axis Months [jan,a,b,c,d,e,f,g,h,i,j,k,l,m,n,o,p,q,r,s]
        y-axis "Revenue (in $)" 4000 --> 12000
        bar [5000, 6000, 7500, 8200, 9500, 10500, 11000, 10200, 9200, 8500, 7000, 6000, 8000, 10000, 5000, 7600, 4999, 11000, 5000, 6000]"#;

    let diagram = parse(input).expect("Failed to parse chart");
    let svg = render(&diagram).expect("Failed to render chart");

    assert_valid_svg(&svg);
}

#[test]
fn test_many_different_size_bars_horizontal() {
    // From Cypress: should render data labels within each bar in the horizontal xy-chart with a lot of bars of different sizes
    let input = r#"xychart horizontal
        title "Sales Revenue"
        x-axis Months [jan,a,b,c,d,e,f,g,h,i,j,k,l,m,n,o,p,q,r,s]
        y-axis "Revenue (in $)" 4000 --> 12000
        bar [5000, 6000, 7500, 8200, 9500, 10500, 11000, 10200, 9200, 8500, 7000, 6000, 8000, 10000, 5000, 7600, 4999, 11000, 5000, 6000]"#;

    let diagram = parse(input).expect("Failed to parse chart");
    let svg = render(&diagram).expect("Failed to render chart");

    assert_valid_svg(&svg);
}

// ============================================================================
// Line Chart Tests
// ============================================================================

#[test]
fn test_line_chart_only() {
    // Line chart without bars
    let input = r#"xychart
        title "Line Chart Only"
        x-axis [jan, feb, mar, apr]
        y-axis 0 --> 100
        line [10, 50, 30, 80]"#;

    let diagram = parse(input).expect("Failed to parse chart");
    let svg = render(&diagram).expect("Failed to render chart");

    assert_valid_svg(&svg);
    // Verify path elements exist for lines
    assert!(
        svg.contains("<path"),
        "Should contain path elements for lines"
    );
}

#[test]
fn test_multiple_lines() {
    // Multiple line plots
    let input = r#"xychart
        title "Multiple Lines"
        x-axis [A, B, C, D]
        y-axis 0 --> 100
        line [10, 30, 20, 40]
        line [50, 40, 60, 30]
        line [25, 55, 35, 65]"#;

    let diagram = parse(input).expect("Failed to parse chart");
    let svg = render(&diagram).expect("Failed to render chart");

    assert_valid_svg(&svg);
}

// ============================================================================
// Combined Bar and Line Tests
// ============================================================================

#[test]
fn test_bar_and_line_combined() {
    // From mermaid demo
    let input = r#"xychart
        title "Sales Revenue"
        x-axis [jan, feb, mar, apr, may, jun, jul, aug, sep, oct, nov, dec]
        y-axis "Revenue (in $)" 4000 --> 11000
        bar [5000, 6000, 7500, 8200, 9500, 10500, 11000, 10200, 9200, 8500, 7000, 6000]
        line [5000, 6000, 7500, 8200, 9500, 10500, 11000, 10200, 9200, 8500, 7000, 6000]"#;

    let diagram = parse(input).expect("Failed to parse chart");
    let svg = render(&diagram).expect("Failed to render chart");

    assert_valid_svg(&svg);
    assert!(
        svg.contains("<rect"),
        "Should contain rect elements for bars"
    );
    assert!(
        svg.contains("<path"),
        "Should contain path elements for lines"
    );
}

// ============================================================================
// Theme Tests
// ============================================================================

#[test]
fn test_with_dark_theme() {
    let input = r#"xychart
        title "Dark Theme Chart"
        x-axis [A, B, C]
        y-axis 0 --> 100
        bar [30, 60, 90]"#;

    let diagram = parse(input).expect("Failed to parse chart");
    let config = RenderConfig {
        theme: Theme::dark(),
        ..Default::default()
    };
    let svg =
        render_with_config(&diagram, &config).expect("Failed to render chart with dark theme");

    assert_valid_svg(&svg);
}

#[test]
fn test_with_forest_theme() {
    let input = r#"xychart
        title "Forest Theme Chart"
        x-axis [A, B, C]
        y-axis 0 --> 100
        bar [30, 60, 90]"#;

    let diagram = parse(input).expect("Failed to parse chart");
    let config = RenderConfig {
        theme: Theme::forest(),
        ..Default::default()
    };
    let svg =
        render_with_config(&diagram, &config).expect("Failed to render chart with forest theme");

    assert_valid_svg(&svg);
}

#[test]
fn test_with_neutral_theme() {
    let input = r#"xychart
        title "Neutral Theme Chart"
        x-axis [A, B, C]
        y-axis 0 --> 100
        bar [30, 60, 90]"#;

    let diagram = parse(input).expect("Failed to parse chart");
    let config = RenderConfig {
        theme: Theme::neutral(),
        ..Default::default()
    };
    let svg =
        render_with_config(&diagram, &config).expect("Failed to render chart with neutral theme");

    assert_valid_svg(&svg);
}

// ============================================================================
// Edge Cases
// ============================================================================

#[test]
fn test_minimal_data() {
    // Minimal data with single point
    let input = r#"xychart
        line [10]"#;

    let diagram = parse(input).expect("Failed to parse chart");
    let svg = render(&diagram).expect("Failed to render chart");

    assert_valid_svg(&svg);
}

#[test]
fn test_large_values() {
    // Large values
    let input = r#"xychart
        x-axis [Q1, Q2, Q3, Q4]
        y-axis 0 --> 1000000
        bar [250000, 500000, 750000, 1000000]"#;

    let diagram = parse(input).expect("Failed to parse chart");
    let svg = render(&diagram).expect("Failed to render chart");

    assert_valid_svg(&svg);
}

#[test]
fn test_zero_values() {
    // Zero values
    let input = r#"xychart
        x-axis [A, B, C, D]
        y-axis 0 --> 100
        bar [0, 50, 0, 75]"#;

    let diagram = parse(input).expect("Failed to parse chart");
    let svg = render(&diagram).expect("Failed to render chart");

    assert_valid_svg(&svg);
}

#[test]
fn test_all_same_values() {
    // All same values
    let input = r#"xychart
        x-axis [A, B, C, D]
        y-axis 0 --> 100
        bar [50, 50, 50, 50]"#;

    let diagram = parse(input).expect("Failed to parse chart");
    let svg = render(&diagram).expect("Failed to render chart");

    assert_valid_svg(&svg);
}

// ============================================================================
// Axis Label Tests
// ============================================================================

#[test]
fn test_long_category_names() {
    // Long category names
    let input = r#"xychart
        x-axis ["Very Long Category Name", "Another Long One", "Short"]
        y-axis 0 --> 100
        bar [30, 60, 90]"#;

    let diagram = parse(input).expect("Failed to parse chart");
    let svg = render(&diagram).expect("Failed to render chart");

    assert_valid_svg(&svg);
}

#[test]
fn test_numeric_x_axis_range() {
    // Numeric x-axis range (linear axis)
    let input = r#"xychart
        x-axis 0 --> 100
        y-axis 0 --> 50
        line [10, 20, 30, 40, 50]"#;

    let diagram = parse(input).expect("Failed to parse chart");
    let svg = render(&diagram).expect("Failed to render chart");

    assert_valid_svg(&svg);
}
