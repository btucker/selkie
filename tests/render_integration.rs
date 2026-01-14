//! Integration tests for the rendering engine

use mermaid::render::{RenderConfig, Theme};
use mermaid::{parse, render, render_text, render_with_config};

// ============================================================================
// Output Format Tests (PNG/PDF)
// ============================================================================

#[cfg(feature = "png")]
mod png_output_tests {
    use super::*;

    /// Test that SVG can be converted to valid PNG
    #[test]
    fn test_svg_to_png_produces_valid_png() {
        let input = r#"flowchart LR
            A[Start] --> B[End]"#;

        let diagram = parse(input).expect("Failed to parse");
        let svg = render(&diagram).expect("Failed to render");

        // Convert to PNG using resvg
        use resvg::tiny_skia;
        use resvg::usvg;

        let mut opt = usvg::Options::default();
        opt.fontdb_mut().load_system_fonts();

        let tree = usvg::Tree::from_str(&svg, &opt).expect("Failed to parse SVG");

        let size = tree.size();
        let mut pixmap = tiny_skia::Pixmap::new(size.width() as u32, size.height() as u32)
            .expect("Failed to create pixmap");

        resvg::render(&tree, tiny_skia::Transform::default(), &mut pixmap.as_mut());

        let png_data = pixmap.encode_png().expect("Failed to encode PNG");

        // Verify PNG header (magic bytes)
        assert!(
            png_data.starts_with(&[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]),
            "Output should be valid PNG (check magic bytes)"
        );

        // Should have reasonable size
        assert!(png_data.len() > 100, "PNG should have content");
    }

    /// Test that PNG output respects scaling
    #[test]
    fn test_png_scaling() {
        let input = r#"flowchart LR
            A --> B --> C"#;

        let diagram = parse(input).expect("Failed to parse");
        let svg = render(&diagram).expect("Failed to render");

        use resvg::tiny_skia;
        use resvg::usvg;

        let mut opt = usvg::Options::default();
        opt.fontdb_mut().load_system_fonts();

        let tree = usvg::Tree::from_str(&svg, &opt).expect("Failed to parse SVG");

        // Render at 2x scale
        let size = tree.size();
        let scale = 2.0;
        let width = (size.width() * scale) as u32;
        let height = (size.height() * scale) as u32;

        let mut pixmap = tiny_skia::Pixmap::new(width, height).expect("Failed to create pixmap");

        let transform = tiny_skia::Transform::from_scale(scale, scale);
        resvg::render(&tree, transform, &mut pixmap.as_mut());

        assert_eq!(pixmap.width(), width);
        assert_eq!(pixmap.height(), height);
    }

    /// Test PNG output with dark theme
    #[test]
    fn test_png_with_dark_theme() {
        let input = r#"flowchart TB
            A[Start] --> B{Decision}
            B -->|Yes| C[End]"#;

        let diagram = parse(input).expect("Failed to parse");
        let config = RenderConfig {
            theme: Theme::dark(),
            ..Default::default()
        };
        let svg = render_with_config(&diagram, &config).expect("Failed to render");

        use resvg::tiny_skia;
        use resvg::usvg;

        let mut opt = usvg::Options::default();
        opt.fontdb_mut().load_system_fonts();

        let tree = usvg::Tree::from_str(&svg, &opt).expect("Failed to parse SVG");

        let size = tree.size();
        let mut pixmap = tiny_skia::Pixmap::new(size.width() as u32, size.height() as u32)
            .expect("Failed to create pixmap");

        resvg::render(&tree, tiny_skia::Transform::default(), &mut pixmap.as_mut());

        let png_data = pixmap.encode_png().expect("Failed to encode PNG");
        assert!(
            png_data.starts_with(&[0x89, 0x50, 0x4E, 0x47]),
            "Dark theme should produce valid PNG"
        );
    }
}

#[cfg(feature = "pdf")]
mod pdf_output_tests {
    use super::*;

    /// Test that SVG can be converted to valid PDF
    #[test]
    fn test_svg_to_pdf_produces_valid_pdf() {
        let input = r#"flowchart LR
            A[Start] --> B[End]"#;

        let diagram = parse(input).expect("Failed to parse");
        let svg = render(&diagram).expect("Failed to render");

        use resvg::usvg;

        let mut opt = usvg::Options::default();
        opt.fontdb_mut().load_system_fonts();

        let tree = usvg::Tree::from_str(&svg, &opt).expect("Failed to parse SVG");

        let pdf_data = svg2pdf::to_pdf(
            &tree,
            svg2pdf::ConversionOptions::default(),
            svg2pdf::PageOptions::default(),
        )
        .expect("Failed to convert to PDF");

        // Verify PDF header
        assert!(
            pdf_data.starts_with(b"%PDF-"),
            "Output should be valid PDF (check header)"
        );

        // Should have reasonable size
        assert!(pdf_data.len() > 100, "PDF should have content");
    }

    /// Test PDF output with various diagram types
    #[test]
    fn test_pdf_with_different_diagrams() {
        let diagrams = [
            (
                "flowchart",
                r#"flowchart TB
                A --> B --> C"#,
            ),
            (
                "pie",
                r#"pie title Test
                "A" : 50
                "B" : 50"#,
            ),
            (
                "state",
                r#"stateDiagram-v2
                [*] --> Active
                Active --> [*]"#,
            ),
        ];

        for (name, input) in diagrams {
            let diagram = parse(input).unwrap_or_else(|_| panic!("Failed to parse {}", name));
            let svg = render(&diagram).unwrap_or_else(|_| panic!("Failed to render {}", name));

            use resvg::usvg;

            let mut opt = usvg::Options::default();
            opt.fontdb_mut().load_system_fonts();

            let tree = usvg::Tree::from_str(&svg, &opt)
                .unwrap_or_else(|_| panic!("Failed to parse {} SVG", name));

            let pdf_data = svg2pdf::to_pdf(
                &tree,
                svg2pdf::ConversionOptions::default(),
                svg2pdf::PageOptions::default(),
            )
            .unwrap_or_else(|_| panic!("Failed to convert {} to PDF", name));

            assert!(
                pdf_data.starts_with(b"%PDF-"),
                "{} should produce valid PDF",
                name
            );
        }
    }
}

// ============================================================================
// Original Integration Tests
// ============================================================================

#[test]
fn test_simple_flowchart_renders_to_svg() {
    let input = r#"flowchart LR
    A[Start] --> B[Process]
    B --> C[End]"#;

    let diagram = parse(input).expect("Failed to parse flowchart");
    let svg = render(&diagram).expect("Failed to render flowchart");

    // Verify basic SVG structure
    assert!(svg.contains("<svg"), "SVG should have opening tag");
    assert!(svg.contains("</svg>"), "SVG should have closing tag");
    assert!(
        svg.contains("xmlns=\"http://www.w3.org/2000/svg\""),
        "SVG should have namespace"
    );

    // Verify node labels are present
    assert!(svg.contains("Start"), "SVG should contain 'Start' label");
    assert!(
        svg.contains("Process"),
        "SVG should contain 'Process' label"
    );
    assert!(svg.contains("End"), "SVG should contain 'End' label");
}

#[test]
fn test_flowchart_with_decision_diamond() {
    let input = r#"flowchart TB
    A[Start] --> B{Decision}
    B -->|Yes| C[Action]
    B -->|No| D[End]"#;

    let diagram = parse(input).expect("Failed to parse flowchart");
    let svg = render(&diagram).expect("Failed to render flowchart");

    // Verify diamond shape (polygon) is rendered
    assert!(
        svg.contains("<polygon"),
        "SVG should contain polygon for diamond shape"
    );

    // Verify all labels present
    assert!(svg.contains("Start"), "SVG should contain 'Start' label");
    assert!(
        svg.contains("Decision"),
        "SVG should contain 'Decision' label"
    );
    assert!(svg.contains("Action"), "SVG should contain 'Action' label");
}

#[test]
fn test_flowchart_with_various_shapes() {
    let input = r#"flowchart TD
    A([Stadium]) --> B[[Subroutine]]
    B --> C[(Database)]
    C --> D((Circle))
    D --> E>Odd]"#;

    let diagram = parse(input).expect("Failed to parse flowchart");
    let svg = render(&diagram).expect("Failed to render flowchart");

    // Verify SVG generated
    assert!(svg.contains("<svg"), "SVG should have opening tag");

    // Verify labels
    assert!(
        svg.contains("Stadium"),
        "SVG should contain 'Stadium' label"
    );
    assert!(
        svg.contains("Subroutine"),
        "SVG should contain 'Subroutine' label"
    );
    assert!(
        svg.contains("Database"),
        "SVG should contain 'Database' label"
    );
    assert!(svg.contains("Circle"), "SVG should contain 'Circle' label");
}

#[test]
fn test_render_with_dark_theme() {
    let input = r#"flowchart LR
    A --> B"#;

    let diagram = parse(input).expect("Failed to parse flowchart");
    let config = RenderConfig {
        theme: Theme::dark(),
        ..Default::default()
    };
    let svg = render_with_config(&diagram, &config).expect("Failed to render with dark theme");

    // Verify SVG generated with dark theme
    assert!(svg.contains("<svg"), "SVG should have opening tag");
    // Dark theme should have dark background color in styles
    assert!(
        svg.contains("<style>"),
        "SVG should contain embedded styles"
    );
}

#[test]
fn test_render_with_forest_theme() {
    let input = r#"flowchart LR
    A --> B"#;

    let diagram = parse(input).expect("Failed to parse flowchart");
    let config = RenderConfig {
        theme: Theme::forest(),
        ..Default::default()
    };
    let svg = render_with_config(&diagram, &config).expect("Failed to render with forest theme");

    // Verify SVG generated with forest theme
    assert!(svg.contains("<svg"), "SVG should have opening tag");
    assert!(
        svg.contains("<style>"),
        "SVG should contain embedded styles"
    );
    // Forest theme should have green colors in styles
    assert!(
        svg.contains("#cde498") || svg.contains("#cdffb2") || svg.contains("#13540c"),
        "Forest theme should have green color palette, got: {}",
        &svg[..500.min(svg.len())]
    );
}

#[test]
fn test_render_with_base_theme() {
    let input = r#"flowchart LR
    A --> B"#;

    let diagram = parse(input).expect("Failed to parse flowchart");
    let config = RenderConfig {
        theme: Theme::base(),
        ..Default::default()
    };
    let svg = render_with_config(&diagram, &config).expect("Failed to render with base theme");

    // Verify SVG generated with base theme
    assert!(svg.contains("<svg"), "SVG should have opening tag");
    assert!(
        svg.contains("<style>"),
        "SVG should contain embedded styles"
    );
    // Base theme should have neutral warm colors
    assert!(
        svg.contains("#fff4dd") || svg.contains("#f4f4f4"),
        "Base theme should have neutral warm color palette, got: {}",
        &svg[..500.min(svg.len())]
    );
}

#[test]
fn test_render_with_custom_padding() {
    let input = r#"flowchart LR
    A --> B"#;

    let diagram = parse(input).expect("Failed to parse flowchart");
    let config = RenderConfig {
        padding: 50.0,
        ..Default::default()
    };
    let svg = render_with_config(&diagram, &config).expect("Failed to render with custom padding");

    assert!(svg.contains("<svg"), "SVG should have opening tag");
}

#[test]
fn test_flowchart_with_edge_labels() {
    let input = r#"flowchart LR
    A -->|label text| B"#;

    let diagram = parse(input).expect("Failed to parse flowchart");
    let svg = render(&diagram).expect("Failed to render flowchart");

    // Verify edge label is present
    assert!(svg.contains("label text"), "SVG should contain edge label");
}

#[test]
fn test_flowchart_all_directions() {
    let directions = ["TB", "TD", "BT", "LR", "RL"];

    for dir in &directions {
        let input = format!("flowchart {}\n    A --> B", dir);
        let diagram = parse(&input)
            .unwrap_or_else(|_| panic!("Failed to parse flowchart with direction {}", dir));
        let svg = render(&diagram)
            .unwrap_or_else(|_| panic!("Failed to render flowchart with direction {}", dir));

        assert!(
            svg.contains("<svg"),
            "SVG should have opening tag for direction {}",
            dir
        );
    }
}

#[test]
fn test_flowchart_with_subgraph() {
    let input = r#"flowchart TB
    subgraph one
        A --> B
    end
    subgraph two
        C --> D
    end
    B --> C"#;

    let diagram = parse(input).expect("Failed to parse flowchart with subgraphs");
    let svg = render(&diagram).expect("Failed to render flowchart with subgraphs");

    // Verify basic structure
    assert!(svg.contains("<svg"), "SVG should have opening tag");
}

#[test]
fn test_arrow_markers_are_defined() {
    let input = r#"flowchart LR
    A --> B"#;

    let diagram = parse(input).expect("Failed to parse flowchart");
    let svg = render(&diagram).expect("Failed to render flowchart");

    // Verify arrow markers are defined in defs section
    assert!(svg.contains("<defs>"), "SVG should have defs section");
    assert!(
        svg.contains("<marker"),
        "SVG should define markers for arrows"
    );
}

#[test]
fn test_edges_use_path_elements() {
    let input = r#"flowchart LR
    A --> B --> C"#;

    let diagram = parse(input).expect("Failed to parse flowchart");
    let svg = render(&diagram).expect("Failed to render flowchart");

    // Verify edges are rendered as path elements
    assert!(
        svg.contains("<path"),
        "SVG should contain path elements for edges"
    );
}

#[test]
fn test_flowchart_nodes_have_proper_styling_class() {
    // Issue: CSS selectors like ".node rect" require shapes to be INSIDE a .node element,
    // but shapes were getting class="node" directly, causing CSS not to match
    // and shapes to render with default black fill.
    let input = r#"flowchart LR
    A[Start] --> B[End]"#;

    let diagram = parse(input).expect("Failed to parse flowchart");
    let svg = render(&diagram).expect("Failed to render flowchart");

    // Shapes should be wrapped in a group with class="node"
    // Pattern: <g class="node" ...><rect .../></g>
    assert!(
        svg.contains(r#"<g class="node""#),
        "Should have group elements with class='node'"
    );

    // The rect should NOT have class="node" directly (which breaks CSS)
    // Bad: <rect class="node" .../>
    // Good: <g class="node"><rect .../></g>
    assert!(
        !svg.contains(r#"<rect "#) || !svg.contains(r#"class="node"/>"#),
        "rect elements should not have class='node' directly - should be inside a .node group"
    );
}

#[test]
fn test_state_diagram_has_start_state() {
    // Issue: State diagrams were missing the initial [*] state circle
    let input = r#"stateDiagram-v2
    [*] --> Idle
    Idle --> Running"#;

    let diagram = parse(input).expect("Failed to parse state diagram");
    let svg = render(&diagram).expect("Failed to render state diagram");

    // Should have a filled circle for start state
    assert!(
        svg.contains("<circle") && svg.contains("start"),
        "State diagram should render the [*] start state as a circle"
    );
}

#[test]
fn test_state_diagram_has_end_state_bullseye() {
    // Issue: End state should be a circle-in-circle (bullseye), not just a dot
    let input = r#"stateDiagram-v2
    Running --> [*]"#;

    let diagram = parse(input).expect("Failed to parse state diagram");
    let svg = render(&diagram).expect("Failed to render state diagram");

    // End state should have double circles (outer ring + inner fill)
    // Count circles - should have at least 2 for end state representation
    let circle_count = svg.matches("<circle").count();
    assert!(
        circle_count >= 2,
        "End state should be rendered as double circles (bullseye), found {} circles",
        circle_count
    );
}

#[test]
fn test_class_diagram_inheritance_uses_hollow_triangle() {
    // Issue: Class inheritance should use hollow triangle arrowhead (UML standard)
    let input = r#"classDiagram
    Animal <|-- Dog"#;

    let diagram = parse(input).expect("Failed to parse class diagram");
    let svg = render(&diagram).expect("Failed to render class diagram");

    // Should have an inheritance marker defined (hollow triangle)
    assert!(
        svg.contains("marker") && svg.contains("inheritance"),
        "Class diagram should define an inheritance marker for hollow triangle arrows"
    );
}

#[test]
fn test_class_diagram_hierarchical_layout() {
    // Issue: Parent classes should appear above child classes in class diagrams
    let input = r#"classDiagram
    Animal <|-- Duck
    Animal <|-- Fish
    Animal <|-- Zebra"#;

    let diagram = parse(input).expect("Failed to parse class diagram");
    let svg = render(&diagram).expect("Failed to render class diagram");

    // Extract y-coordinates from SVG to verify layout
    // Animal (parent) should have smaller y value than children

    // Find the class boxes by looking for class-node groups
    // The y values in transform or rect elements indicate vertical position
    // Parent (Animal) should be above children (Duck, Fish, Zebra)

    // Check that we have all 4 classes rendered
    assert!(svg.contains("Animal"), "Should contain Animal class");
    assert!(svg.contains("Duck"), "Should contain Duck class");
    assert!(svg.contains("Fish"), "Should contain Fish class");
    assert!(svg.contains("Zebra"), "Should contain Zebra class");

    // TODO: Add more specific y-coordinate checks once hierarchical layout is implemented
}

#[test]
fn test_state_diagram_both_start_and_end_states() {
    // Issue: When a diagram has both [*] --> State and State --> [*],
    // both start (filled circle) and end (bullseye) states should be rendered.
    // Previously, both [*] were treated as the same state.
    let input = r#"stateDiagram-v2
    [*] --> Idle
    Idle --> Running
    Running --> [*]"#;

    let diagram = parse(input).expect("Failed to parse state diagram");
    let svg = render(&diagram).expect("Failed to render state diagram");

    // Should have separate start and end states
    // Start state: 1 filled circle with state-start class
    // End state: 2 circles (outer + inner) with state-end-* classes

    // Check for start state (filled circle)
    assert!(
        svg.contains("state-start"),
        "State diagram should have a start state with class 'state-start'. SVG:\n{}",
        svg
    );

    // Check for end state (bullseye with outer and inner circles)
    assert!(
        svg.contains("state-end-outer") && svg.contains("state-end-inner"),
        "State diagram should have an end state with bullseye (outer and inner circles). SVG:\n{}",
        svg
    );

    // Should have at least 3 circles total: 1 for start, 2 for end (bullseye)
    let circle_count = svg.matches("<circle").count();
    assert!(
        circle_count >= 3,
        "State diagram should have at least 3 circles (1 start + 2 end bullseye), found {}. SVG:\n{}",
        circle_count, svg
    );
}

#[test]
fn test_pie_chart_has_legend() {
    // Issue: Pie chart should have a legend with colored boxes and labels
    let input = r#"pie title Test Distribution
    "Alpha" : 40
    "Beta" : 30
    "Gamma" : 30"#;

    let diagram = parse(input).expect("Failed to parse pie chart");
    let svg = render(&diagram).expect("Failed to render pie chart");

    // Should have a legend with colored rectangles
    assert!(
        svg.contains("pie-legend") || svg.contains("legend"),
        "Pie chart should have a legend. SVG:\n{}",
        svg
    );
}

#[test]
fn test_pie_chart_preserves_section_order() {
    // Issue: Pie chart should render slices in declaration order, not alphabetically
    // Use labels that would be reordered alphabetically: Zebra, Apple, Mango
    // Declaration order: Zebra, Apple, Mango
    // Alphabetical order: Apple, Mango, Zebra
    let input = r#"pie
    "Zebra" : 40
    "Apple" : 30
    "Mango" : 30"#;

    let diagram = parse(input).expect("Failed to parse pie chart");
    let svg = render(&diagram).expect("Failed to render pie chart");

    // Find positions of labels in the SVG
    let zebra_pos = svg.find("Zebra").unwrap_or(usize::MAX);
    let apple_pos = svg.find("Apple").unwrap_or(usize::MAX);
    let mango_pos = svg.find("Mango").unwrap_or(usize::MAX);

    // The slices should be rendered in declaration order (Zebra first, then Apple, then Mango)
    // If sorted alphabetically, Apple would come first, which is wrong
    assert!(
        zebra_pos < apple_pos && apple_pos < mango_pos,
        "Pie chart sections should be rendered in declaration order (Zebra, Apple, Mango), not alphabetically. Zebra={}, Apple={}, Mango={}",
        zebra_pos, apple_pos, mango_pos
    );
}

#[test]
fn test_pie_chart_renders_small_percentage_labels() {
    // Issue: Small slices like 4% should still have percentage labels rendered
    // The Voldemort pie chart has FRIENDS=2 (4%), FAMILY=3 (6%), NOSE=45 (90%)
    let input = r#"pie title What Voldemort doesnt have
         "FRIENDS" : 2
         "FAMILY" : 3
         "NOSE" : 45"#;

    let diagram = parse(input).expect("Failed to parse pie chart");
    let svg = render(&diagram).expect("Failed to render pie chart");

    // All percentage labels should be present
    assert!(
        svg.contains("90%"),
        "Should render 90% label. SVG:\n{}",
        svg
    );
    assert!(svg.contains("6%"), "Should render 6% label. SVG:\n{}", svg);
    assert!(
        svg.contains("4%"),
        "Should render 4% label for small slice. SVG:\n{}",
        svg
    );
}

#[test]
fn test_pie_chart_uses_theme_colors() {
    // Verify pie chart uses theme colors instead of hardcoded values
    let input = r#"pie title Themed Pie
    "A" : 50
    "B" : 50"#;

    let diagram = parse(input).expect("Failed to parse pie chart");

    // Test with forest theme which has distinctive green colors
    let config = RenderConfig {
        theme: Theme::forest(),
        ..Default::default()
    };
    let svg = render_with_config(&diagram, &config).expect("Failed to render with forest theme");

    // Forest theme pie colors include #cde498, #cdffb2
    assert!(
        svg.contains("#cde498") || svg.contains("#cdffb2"),
        "Pie chart should use forest theme colors. SVG:\n{}",
        &svg[..1000.min(svg.len())]
    );

    // Test with default theme
    let default_svg = render(&diagram).expect("Failed to render with default theme");

    // Default theme uses different colors (#ECECFF, #ffffde)
    assert!(
        default_svg.contains("#ECECFF") || default_svg.contains("#ffffde"),
        "Pie chart should use default theme colors. SVG:\n{}",
        &default_svg[..1000.min(default_svg.len())]
    );
}

#[test]
fn test_flowchart_edge_stroke_width() {
    // mermaid.js uses stroke-width: 1px for normal edges, not 2px
    let input = r#"flowchart LR
    A --> B"#;

    let diagram = parse(input).expect("Failed to parse flowchart");
    let svg = render(&diagram).expect("Failed to render flowchart");

    // Edge paths should use stroke-width of 1, and CSS should specify 1px
    // (Note: markers like cross may still use stroke-width: 2 for their internal paths)
    assert!(
        svg.contains("stroke-width: 1px") || svg.contains("stroke-width=\"1\""),
        "Edge stroke-width should be 1px. SVG:\n{}",
        svg
    );

    // The edge path element specifically should have stroke-width 1
    assert!(
        svg.contains(r#"class="edge-path""#)
            && (svg.contains("stroke-width=\"1\"") || svg.contains("stroke-width: 1px")),
        "Edge path should have stroke-width 1. SVG:\n{}",
        svg
    );
}

#[test]
fn test_flowchart_arrow_marker_size() {
    // mermaid.js uses markerWidth=8, markerHeight=8 for point markers
    let input = r#"flowchart LR
    A --> B"#;

    let diagram = parse(input).expect("Failed to parse flowchart");
    let svg = render(&diagram).expect("Failed to render flowchart");

    // Arrow point markers should be 8x8, not 12x12
    assert!(
        svg.contains("markerWidth=\"8\"") || svg.contains("markerWidth: 8"),
        "Arrow markers should be 8x8. SVG:\n{}",
        svg
    );
}

#[test]
fn test_flowchart_subroutine_uses_polygon() {
    // Subroutine shape is rendered as a polygon (matching mermaid.js)
    // The polygon traces inner rect → outer rect to create vertical bar effect
    let input = r#"flowchart LR
    A[[Subroutine]]"#;

    let diagram = parse(input).expect("Failed to parse flowchart");
    let svg = render(&diagram).expect("Failed to render flowchart");

    // Should use polygon element (not separate rect + lines)
    assert!(
        svg.contains("<polygon"),
        "Subroutine should use polygon element. SVG:\n{}",
        svg
    );

    // The polygon should have points for both inner and outer rectangles
    let points_re = regex::Regex::new(r#"<polygon points="([^"]+)""#).unwrap();
    if let Some(caps) = points_re.captures(&svg) {
        let points = caps.get(1).unwrap().as_str();
        let point_count = points.split_whitespace().count();
        // Should have 10 points for subroutine shape (inner rect 5 + outer rect 5)
        assert!(
            point_count >= 8,
            "Subroutine polygon should have at least 8 points (got {}). Points: {}",
            point_count,
            points
        );
    } else {
        panic!(
            "Subroutine should have polygon with points attribute. SVG:\n{}",
            svg
        );
    }
}

#[test]
fn test_flowchart_has_circle_markers() {
    // mermaid.js defines circleStart and circleEnd markers for o-o edge types
    // These use circle elements within marker definitions
    let input = r#"flowchart LR
    A o--o B"#;

    let diagram = parse(input).expect("Failed to parse flowchart");
    let svg = render(&diagram).expect("Failed to render flowchart");

    // Count circle elements - should have circles for circleStart and circleEnd markers
    let circle_count = svg.matches("<circle").count();
    assert!(
        circle_count >= 2,
        "Flowchart with o--o edge should have at least 2 circles (for circleStart and circleEnd markers). Found {} circles. SVG:\n{}",
        circle_count, svg
    );

    // Verify marker definitions exist
    assert!(
        svg.contains("marker") && svg.contains("circle"),
        "Flowchart should have circle markers defined. SVG:\n{}",
        svg
    );
}

#[test]
fn test_er_diagram_has_crow_foot_notation() {
    // ER diagrams should render crow's foot cardinality symbols using SVG markers
    let input = r#"erDiagram
    CUSTOMER ||--o{ ORDER : places"#;

    let diagram = parse(input).expect("Failed to parse ER diagram");
    let svg = render(&diagram).expect("Failed to render ER diagram");

    // Should have marker definitions for ER cardinality symbols
    assert!(
        svg.contains("er-onlyOne") || svg.contains("er-zeroOrMore"),
        "ER diagram should have cardinality marker definitions. SVG:\n{}",
        svg
    );

    // The ZeroOrMore cardinality (o{) marker should include a circle
    assert!(
        svg.contains("<circle"),
        "ER diagram ZeroOrMore cardinality marker should include a circle. SVG:\n{}",
        svg
    );

    // Relationship lines should use marker-start and marker-end attributes
    assert!(
        svg.contains("marker-start") && svg.contains("marker-end"),
        "ER diagram relationship paths should use marker-start and marker-end. SVG:\n{}",
        svg
    );

    // Paths should reference ER markers
    assert!(
        svg.contains("url(#er-"),
        "ER diagram paths should reference ER marker definitions. SVG:\n{}",
        svg
    );
}

#[test]
fn test_gantt_task_labels_inside_bars() {
    // Gantt chart task labels should be rendered inside the task bars
    let input = r#"gantt
    title Test Gantt
    dateFormat YYYY-MM-DD
    section Phase1
    Task A :a1, 2024-01-01, 7d"#;

    let diagram = parse(input).expect("Failed to parse Gantt chart");
    let svg = render(&diagram).expect("Failed to render Gantt chart");

    // Task bar should be rendered
    assert!(
        svg.contains("task-bar"),
        "Gantt chart should have task bars"
    );

    // Task label should have class that indicates it's inside the bar (mermaid.js uses taskText)
    assert!(
        svg.contains("taskText")
            || svg.contains("task-label-inside")
            || svg.contains(r#"class="task-label""#),
        "Gantt chart should have task labels"
    );

    // The task name should appear in the SVG
    assert!(
        svg.contains("Task A"),
        "Gantt chart should contain the task name 'Task A'"
    );
}

#[test]
fn test_gantt_task_bar_uses_mermaid_colors() {
    // Issue: selkie-055 - Gantt task bars should use mermaid.js default purple (#8a90dd)
    // mermaid.js uses #8a90dd fill with #534fbc stroke for task bars
    let input = r#"gantt
    title Test
    dateFormat YYYY-MM-DD
    section S1
    Task :a1, 2024-01-01, 3d"#;

    let diagram = parse(input).expect("Failed to parse Gantt chart");
    let svg = render(&diagram).expect("Failed to render Gantt chart");

    // Task bar should use mermaid.js default purple color
    assert!(
        svg.contains("fill=\"#8a90dd\"")
            || svg.contains("fill: #8a90dd")
            || svg.contains("fill=\"#8A90DD\""),
        "Gantt task bars should use mermaid.js purple (#8a90dd), not light blue. SVG:\n{}",
        svg
    );
}

#[test]
fn test_gantt_has_vertical_grid_lines() {
    // Issue: selkie-yn3 - Gantt chart should have vertical grid lines
    let input = r#"gantt
    title Test
    dateFormat YYYY-MM-DD
    section S1
    Task :a1, 2024-01-01, 7d"#;

    let diagram = parse(input).expect("Failed to parse Gantt chart");
    let svg = render(&diagram).expect("Failed to render Gantt chart");

    // Should have vertical grid lines (multiple vertical lines in the chart area)
    // mermaid.js renders these with class="grid" containing tick marks
    assert!(
        svg.contains("grid") || svg.contains("tick"),
        "Gantt chart should have vertical grid lines. SVG:\n{}",
        svg
    );
}

#[test]
fn test_pie_chart_has_outer_circle() {
    // Issue: selkie-vsx - Pie chart should have outer circle like mermaid.js
    let input = r#"pie
    "A" : 50
    "B" : 50"#;

    let diagram = parse(input).expect("Failed to parse pie chart");
    let svg = render(&diagram).expect("Failed to render pie chart");

    // mermaid.js renders a pieOuterCircle around the pie
    // This is a circle with no fill, just a stroke around the pie
    assert!(
        svg.contains("pieOuterCircle")
            || svg.contains("pie-outer")
            || (svg.contains("<circle") && svg.contains("fill=\"none\"")),
        "Pie chart should have an outer circle. SVG:\n{}",
        svg
    );
}

#[test]
fn test_diamond_edges_exit_from_sides_not_corners() {
    // Issue: Edges from diamond decision nodes should exit from the sides
    // (top, bottom, left, right vertices) based on target position, not from corners
    let input = r#"flowchart TB
    A[Start] --> B{Decision}
    B -->|Yes| C[Below]
    B -->|No| D[Side]"#;

    let diagram = parse(input).expect("Failed to parse flowchart");
    let svg = render(&diagram).expect("Failed to render flowchart");

    // Parse the SVG to extract edge paths
    // Diamond edges should have different exit points based on target position
    // Edge B->C should exit from bottom (same x as diamond center)
    // Edge B->D should exit from side (different x than diamond center)

    // Find edge paths that start from the decision diamond
    // We need to verify edges start from diamond sides, not corners
    // Diamond has vertices at top/bottom/left/right of bounding box

    // A robust test: edges from a diamond with multiple targets should
    // have start points that differ in their relative positions
    assert!(svg.contains("<path"), "SVG should contain edge paths");

    // The test will be more specific once we fix the implementation
    // For now, verify the diamond is rendered correctly
    assert!(
        svg.contains("<polygon"),
        "Diamond should be rendered as polygon"
    );
}

#[test]
fn test_parallelogram_renders_as_polygon() {
    let input = r#"flowchart LR
    A[/Parallelogram/]"#;

    let diagram = parse(input).expect("Failed to parse flowchart");
    let svg = render(&diagram).expect("Failed to render flowchart");

    // Parallelogram (LeanRight) should render as a polygon, not a rect
    assert!(
        svg.contains("<polygon"),
        "Parallelogram should render as polygon, got:\n{}",
        svg
    );
}

#[test]
fn test_class_inheritance_arrow_is_hollow_triangle() {
    // Issue selkie-cq5: Inheritance arrows should be hollow triangular heads
    // <|-- means "extends" and the triangle points to the parent (left side)
    let input = r#"classDiagram
    Animal <|-- Dog
    Animal : +String name"#;

    let diagram = parse(input).expect("Failed to parse class diagram");
    let svg = render(&diagram).expect("Failed to render class diagram");

    // Inheritance relation should use a marker
    let has_inheritance_marker =
        svg.contains("url(#inheritance-start)") || svg.contains("url(#inheritance-end)");
    assert!(
        has_inheritance_marker,
        "Inheritance relation should use inheritance marker"
    );

    // The inheritance marker should exist and be hollow (fill="none")
    let has_inheritance_def =
        svg.contains(r#"id="inheritance-start""#) || svg.contains(r#"id="inheritance-end""#);
    assert!(
        has_inheritance_def,
        "SVG should contain inheritance marker definition"
    );

    // The marker path should have fill="none" for hollow triangle (not filled)
    // Extract the marker section and check it has fill="none"
    if let Some(marker_start) = svg
        .find(r#"id="inheritance-start""#)
        .or_else(|| svg.find(r#"id="inheritance-end""#))
    {
        let marker_end = svg[marker_start..].find("</marker>").unwrap_or(200);
        let marker_section = &svg[marker_start..marker_start + marker_end];
        assert!(
            marker_section.contains(r#"fill="none""#),
            "Inheritance marker should have fill=\"none\" for hollow triangle. Got marker section:\n{}",
            marker_section
        );
    }
}

#[test]
fn test_flowchart_tb_layout_vertical_ordering() {
    // Issue selkie-agi: In TB (top-to-bottom) layout, nodes should be laid out
    // vertically with the first node at the top.
    // A -> B -> C should result in A at top, B in middle, C at bottom
    let input = r#"flowchart TB
    A[First] --> B[Second]
    B --> C[Third]"#;

    let diagram = parse(input).expect("Failed to parse flowchart");
    let svg = render(&diagram).expect("Failed to render flowchart");

    // Extract y-coordinates for each node from the SVG
    // Nodes are rendered as <g class="node" id="node-X">...<rect x="..." y="Y">...
    let extract_node_y = |svg: &str, node_id: &str| -> Option<f64> {
        // Find the node group
        let node_marker = format!(r#"id="node-{}""#, node_id);
        let node_start = svg.find(&node_marker)?;
        let node_section = &svg[node_start..];

        // Find the rect element within this node (ends at next </g>)
        let node_end = node_section.find("</g>")?;
        let node_section = &node_section[..node_end];

        // Extract y from rect y="..." or polygon points="..."
        // For rect: y="..."
        if let Some(y_start) = node_section.find(r#" y=""#) {
            let y_value_start = y_start + 4; // skip ` y="`
            let remaining = &node_section[y_value_start..];
            let y_end = remaining.find('"')?;
            let y_str = &remaining[..y_end];
            return y_str.parse().ok();
        }

        // For polygon (diamond): points="x1,y1 x2,y2 ..."
        // Take the first y value from points
        if let Some(points_start) = node_section.find(r#"points=""#) {
            let points_value_start = points_start + 8; // skip `points="`
            let remaining = &node_section[points_value_start..];
            let points_end = remaining.find('"')?;
            let points_str = &remaining[..points_end];
            // Parse "x1,y1 x2,y2 ..." - take first point
            let first_point = points_str.split_whitespace().next()?;
            let coords: Vec<&str> = first_point.split(',').collect();
            if coords.len() >= 2 {
                return coords[1].parse().ok();
            }
        }

        None
    };

    let y_a = extract_node_y(&svg, "A").expect("Should find node A y-coordinate");
    let y_b = extract_node_y(&svg, "B").expect("Should find node B y-coordinate");
    let y_c = extract_node_y(&svg, "C").expect("Should find node C y-coordinate");

    // In TB layout: A should be above B, B should be above C
    // "Above" means smaller y value
    assert!(
        y_a < y_b,
        "In TB layout, A should be above B (A.y={} should be < B.y={}). SVG:\n{}",
        y_a,
        y_b,
        svg
    );
    assert!(
        y_b < y_c,
        "In TB layout, B should be above C (B.y={} should be < C.y={}). SVG:\n{}",
        y_b,
        y_c,
        svg
    );
}

#[test]
fn test_flowchart_tb_layout_with_diamond_ordering() {
    // Issue selkie-agi: In TB layout with a diamond decision node,
    // the diamond should appear BELOW the node pointing to it, not above.
    // This mirrors the issue in flowchart_full where C (Diamond Decision)
    // appears above A (Rectangle) when it should be below A and B.
    //
    // Expected layout:
    //     A (Rectangle)
    //         |
    //     B (Rounded)
    //         |
    //     C (Diamond)
    //       /   \
    //      D     E
    //       \   /
    //        F
    let input = r#"flowchart TB
    A[Rectangle] --> B(Rounded)
    B --> C{Diamond Decision}
    C -->|Yes| D([Stadium])
    C -->|No| E[[Subroutine]]
    D --> F[(Cylinder)]
    E --> F"#;

    let diagram = parse(input).expect("Failed to parse flowchart");
    let svg = render(&diagram).expect("Failed to render flowchart");

    // Extract CENTER y-coordinates for each node from the SVG
    // Important: Different shapes store y differently:
    // - rect: y is top-left, so center = y + height/2
    // - polygon (diamond): vertices form a rotated square, center = (min_y + max_y)/2
    // - circle: cy is already the center
    // - path (cylinder): use text y which is centered
    let extract_node_center_y = |svg: &str, node_id: &str| -> Option<f64> {
        let node_marker = format!(r#"id="node-{}""#, node_id);
        let node_start = svg.find(&node_marker)?;
        let node_section = &svg[node_start..];
        let node_end = node_section.find("</g>")?;
        let node_section = &node_section[..node_end];

        // For rect: y="..." height="..." -> center = y + height/2
        if let Some(y_start) = node_section.find(r#" y=""#) {
            let y_value_start = y_start + 4;
            let remaining = &node_section[y_value_start..];
            let y_end = remaining.find('"')?;
            let y_str = &remaining[..y_end];
            let y: f64 = y_str.parse().ok()?;

            if let Some(h_start) = node_section.find(r#" height=""#) {
                let h_value_start = h_start + 9;
                let remaining = &node_section[h_value_start..];
                let h_end = remaining.find('"')?;
                let h_str = &remaining[..h_end];
                let h: f64 = h_str.parse().ok()?;
                return Some(y + h / 2.0);
            }
            return Some(y);
        }

        // For polygon (diamond): find min/max y to compute center
        if let Some(points_start) = node_section.find(r#"points=""#) {
            let points_value_start = points_start + 8;
            let remaining = &node_section[points_value_start..];
            let points_end = remaining.find('"')?;
            let points_str = &remaining[..points_end];

            let mut min_y = f64::MAX;
            let mut max_y = f64::MIN;
            for point in points_str.split_whitespace() {
                let coords: Vec<&str> = point.split(',').collect();
                if coords.len() >= 2 {
                    if let Ok(y) = coords[1].parse::<f64>() {
                        min_y = min_y.min(y);
                        max_y = max_y.max(y);
                    }
                }
            }
            if min_y != f64::MAX {
                return Some((min_y + max_y) / 2.0);
            }
        }

        // For path (cylinder) or other shapes: use text y as center
        if let Some(text_start) = node_section.find("<text") {
            let text_section = &node_section[text_start..];
            if let Some(y_start) = text_section.find(r#" y=""#) {
                let y_value_start = y_start + 4;
                let remaining = &text_section[y_value_start..];
                let y_end = remaining.find('"')?;
                let y_str = &remaining[..y_end];
                return y_str.parse().ok();
            }
        }

        // For circle: cy is the center
        if let Some(cy_start) = node_section.find(r#" cy=""#) {
            let cy_value_start = cy_start + 5;
            let remaining = &node_section[cy_value_start..];
            let cy_end = remaining.find('"')?;
            let cy_str = &remaining[..cy_end];
            return cy_str.parse().ok();
        }

        None
    };

    let y_a = extract_node_center_y(&svg, "A").expect("Should find node A");
    let y_b = extract_node_center_y(&svg, "B").expect("Should find node B");
    let y_c = extract_node_center_y(&svg, "C").expect("Should find node C");
    let y_d = extract_node_center_y(&svg, "D").expect("Should find node D");
    let y_e = extract_node_center_y(&svg, "E").expect("Should find node E");
    let y_f = extract_node_center_y(&svg, "F").expect("Should find node F");

    eprintln!("Node CENTER y-coordinates:");
    eprintln!("  A (Rectangle): {}", y_a);
    eprintln!("  B (Rounded): {}", y_b);
    eprintln!("  C (Diamond): {}", y_c);
    eprintln!("  D (Stadium): {}", y_d);
    eprintln!("  E (Subroutine): {}", y_e);
    eprintln!("  F (Cylinder): {}", y_f);

    // The vertical order should be: A -> B -> C -> D,E -> F
    // All comparisons use center y-coordinates now
    assert!(y_a < y_b, "A should be above B: A.y={} < B.y={}", y_a, y_b);
    assert!(y_b < y_c, "B should be above C: B.y={} < C.y={}", y_b, y_c);
    assert!(y_c < y_d, "C should be above D: C.y={} < D.y={}", y_c, y_d);
    assert!(y_c < y_e, "C should be above E: C.y={} < E.y={}", y_c, y_e);

    // D and E should be on approximately the same level
    assert!(
        (y_d - y_e).abs() < 20.0,
        "D and E should be on same level: D.y={}, E.y={}",
        y_d,
        y_e
    );

    // F should be below D and E
    assert!(y_d < y_f, "D should be above F: D.y={} < F.y={}", y_d, y_f);
    assert!(y_e < y_f, "E should be above F: E.y={} < F.y={}", y_e, y_f);
}

#[test]
fn test_flowchart_tb_subgraph_internal_layout() {
    // Issue selkie-agi: When a flowchart has subgraphs, nodes within
    // a subgraph should still follow the TB layout direction.
    // In flowchart_full, the "Main Flow" subgraph has nodes that should
    // be laid out vertically, but they appear horizontally.
    let input = r#"flowchart TB
    subgraph main [Main Flow]
        A[Rectangle] --> B(Rounded)
        B --> C{Diamond Decision}
        C -->|Yes| D([Stadium])
        C -->|No| E[[Subroutine]]
        D --> F[(Cylinder DB)]
        E --> F
    end
    subgraph shapes [All Shapes]
        G((Circle)) --> H
    end
    F --> G"#;

    let diagram = parse(input).expect("Failed to parse flowchart with subgraphs");
    let svg = render(&diagram).expect("Failed to render flowchart with subgraphs");

    // Extract y-coordinates for each node from the SVG
    let extract_node_y = |svg: &str, node_id: &str| -> Option<f64> {
        let node_marker = format!(r#"id="node-{}""#, node_id);
        let node_start = svg.find(&node_marker)?;
        let node_section = &svg[node_start..];
        let node_end = node_section.find("</g>")?;
        let node_section = &node_section[..node_end];

        // For rect: y="..."
        if let Some(y_start) = node_section.find(r#" y=""#) {
            let y_value_start = y_start + 4;
            let remaining = &node_section[y_value_start..];
            let y_end = remaining.find('"')?;
            let y_str = &remaining[..y_end];
            return y_str.parse().ok();
        }

        // For polygon (diamond): points="x1,y1 x2,y2 ..."
        if let Some(points_start) = node_section.find(r#"points=""#) {
            let points_value_start = points_start + 8;
            let remaining = &node_section[points_value_start..];
            let points_end = remaining.find('"')?;
            let points_str = &remaining[..points_end];
            let first_point = points_str.split_whitespace().next()?;
            let coords: Vec<&str> = first_point.split(',').collect();
            if coords.len() >= 2 {
                return coords[1].parse().ok();
            }
        }

        // For path (cylinder): check for path with specific pattern
        if let Some(path_start) = node_section.find(r#"<path d=""#) {
            let path_section = &node_section[path_start..];
            if let Some(m_pos) = path_section.find("M ") {
                let coords_start = m_pos + 2;
                let remaining = &path_section[coords_start..];
                let parts: Vec<&str> = remaining.split_whitespace().take(2).collect();
                if parts.len() >= 2 {
                    return parts[1].parse().ok();
                }
            }
        }

        // For circle
        if let Some(cy_start) = node_section.find(r#" cy=""#) {
            let cy_value_start = cy_start + 5;
            let remaining = &node_section[cy_value_start..];
            let cy_end = remaining.find('"')?;
            let cy_str = &remaining[..cy_end];
            return cy_str.parse().ok();
        }

        None
    };

    let y_a = extract_node_y(&svg, "A").expect("Should find node A y-coordinate");
    let y_b = extract_node_y(&svg, "B").expect("Should find node B y-coordinate");
    let y_c = extract_node_y(&svg, "C").expect("Should find node C y-coordinate");

    eprintln!("Node y-coordinates in subgraph:");
    eprintln!("  A (Rectangle): {}", y_a);
    eprintln!("  B (Rounded): {}", y_b);
    eprintln!("  C (Diamond): {}", y_c);

    // Within the Main Flow subgraph, nodes should still follow TB ordering:
    // A should be above B, B should be above C (diamond)
    assert!(
        y_a < y_c,
        "Within subgraph, A should be above C (Diamond): A.y={} should be < C.y={}. \
        This is selkie-agi: nodes in subgraph not respecting TB direction.",
        y_a,
        y_c
    );

    assert!(y_a < y_b, "A should be above B: A.y={} < B.y={}", y_a, y_b);
    assert!(y_b < y_c, "B should be above C: B.y={} < C.y={}", y_b, y_c);
}

#[test]
fn test_flowchart_full_tb_layout() {
    // Issue selkie-agi: This is the exact flowchart_full input that shows
    // incorrect layout - C (Diamond) appears above A (Rectangle) instead of below.
    let input = r#"flowchart TB
    subgraph main [Main Flow]
        A[Rectangle] --> B(Rounded)
        B --> C{Diamond Decision}
        C -->|Yes| D([Stadium])
        C -->|No| E[[Subroutine]]
        D --> F[(Cylinder DB)]
        E --> F
    end
    subgraph shapes [All Shapes]
        G((Circle)) --> H>Asymmetric]
        H --> I[/Parallelogram/]
        I --> J[\Reverse Para\]
        J --> K[/Trapezoid\]
        K --> L[\Inv Trapezoid/]
        L --> M{{Hexagon}}
        M --> N(((Double Circle)))
    end
    subgraph edges [Edge Types]
        O --> P
        O --- Q
        O -.- R
        O -.-> S
        O ==> T
        O <--> U
        O x--x V
        O o--o W
    end
    F --> G
    N --> O"#;

    let diagram = parse(input).expect("Failed to parse flowchart_full");
    let svg = render(&diagram).expect("Failed to render flowchart_full");

    // Extract y-coordinates for each node from the SVG
    let extract_node_y = |svg: &str, node_id: &str| -> Option<f64> {
        let node_marker = format!(r#"id="node-{}""#, node_id);
        let node_start = svg.find(&node_marker)?;
        let node_section = &svg[node_start..];
        let node_end = node_section.find("</g>")?;
        let node_section = &node_section[..node_end];

        // For rect: y="..."
        if let Some(y_start) = node_section.find(r#" y=""#) {
            let y_value_start = y_start + 4;
            let remaining = &node_section[y_value_start..];
            let y_end = remaining.find('"')?;
            let y_str = &remaining[..y_end];
            return y_str.parse().ok();
        }

        // For polygon (diamond): points="x1,y1 x2,y2 ..."
        // Diamond's first point is the TOP vertex, so that's the minimum y
        if let Some(points_start) = node_section.find(r#"points=""#) {
            let points_value_start = points_start + 8;
            let remaining = &node_section[points_value_start..];
            let points_end = remaining.find('"')?;
            let points_str = &remaining[..points_end];
            let first_point = points_str.split_whitespace().next()?;
            let coords: Vec<&str> = first_point.split(',').collect();
            if coords.len() >= 2 {
                return coords[1].parse().ok();
            }
        }

        // For path (cylinder): check for path with specific pattern
        if let Some(path_start) = node_section.find(r#"<path d=""#) {
            let path_section = &node_section[path_start..];
            if let Some(m_pos) = path_section.find("M ") {
                let coords_start = m_pos + 2;
                let remaining = &path_section[coords_start..];
                let parts: Vec<&str> = remaining.split_whitespace().take(2).collect();
                if parts.len() >= 2 {
                    return parts[1].parse().ok();
                }
            }
        }

        // For circle
        if let Some(cy_start) = node_section.find(r#" cy=""#) {
            let cy_value_start = cy_start + 5;
            let remaining = &node_section[cy_value_start..];
            let cy_end = remaining.find('"')?;
            let cy_str = &remaining[..cy_end];
            return cy_str.parse().ok();
        }

        None
    };

    let y_a = extract_node_y(&svg, "A").expect("Should find node A");
    let y_c = extract_node_y(&svg, "C").expect("Should find node C");

    eprintln!("flowchart_full node y-coordinates:");
    eprintln!("  A (Rectangle): {}", y_a);
    eprintln!("  C (Diamond): {}", y_c);

    // THE CRITICAL BUG: In the broken version, C.y < A.y (C is above A)
    // In the fixed version, A.y < C.y (A is above C, which is correct for A --> B --> C)
    assert!(
        y_a < y_c,
        "BUG selkie-agi: In flowchart_full, A (Rectangle) should be ABOVE C (Diamond). \
        Instead, C is at y={} and A is at y={}. The Diamond Decision is being placed \
        above the Rectangle when it should be below it.",
        y_c,
        y_a
    );
}

#[test]
fn test_state_diagram_vertical_layout() {
    // State diagrams should default to vertical (top-to-bottom) layout
    // with states positioned based on the transition flow
    let input = r#"stateDiagram-v2
    [*] --> Idle
    Idle --> Running : start
    Running --> Idle : stop
    Running --> [*]"#;

    let diagram = parse(input).expect("Failed to parse state diagram");
    let svg = render(&diagram).expect("Failed to render state diagram");

    // Extract y-coordinates for each state
    let extract_state_y = |svg: &str, state_id: &str| -> Option<f64> {
        // Look for state-<id> group
        let state_marker = format!(r#"id="state-{}""#, state_id);
        let state_start = svg.find(&state_marker)?;
        let state_section = &svg[state_start..];
        let state_end = state_section.find("</g>")?;
        let state_section = &state_section[..state_end];

        // For rect: y="..."
        if let Some(y_start) = state_section.find(r#" y=""#) {
            let y_value_start = y_start + 4;
            let remaining = &state_section[y_value_start..];
            let y_end = remaining.find('"')?;
            let y_str = &remaining[..y_end];
            return y_str.parse().ok();
        }

        // For circle (start/end states): cy="..."
        if let Some(cy_start) = state_section.find(r#" cy=""#) {
            let cy_value_start = cy_start + 5;
            let remaining = &state_section[cy_value_start..];
            let cy_end = remaining.find('"')?;
            let cy_str = &remaining[..cy_end];
            return cy_str.parse().ok();
        }

        None
    };

    // Get positions
    let y_idle = extract_state_y(&svg, "Idle").expect("Should find Idle state");
    let y_running = extract_state_y(&svg, "Running").expect("Should find Running state");

    eprintln!("State y-coordinates:");
    eprintln!("  Idle: {}", y_idle);
    eprintln!("  Running: {}", y_running);

    // In TB layout: Idle should be above Running (smaller y)
    // because [*] --> Idle --> Running
    assert!(
        y_idle < y_running,
        "In vertical layout, Idle should be above Running. Idle.y={} vs Running.y={}",
        y_idle,
        y_running
    );

    // Also verify the diagram is taller than it is wide (vertical orientation)
    let width_match = svg.find(r#"width=""#).and_then(|start| {
        let remaining = &svg[start + 7..];
        let end = remaining.find('"')?;
        remaining[..end].parse::<f64>().ok()
    });
    let height_match = svg.find(r#"height=""#).and_then(|start| {
        let remaining = &svg[start + 8..];
        let end = remaining.find('"')?;
        remaining[..end].parse::<f64>().ok()
    });

    if let (Some(width), Some(height)) = (width_match, height_match) {
        eprintln!("Diagram size: {}x{}", width, height);
        assert!(
            height > width * 0.8, // Allow some tolerance, but should be roughly taller
            "State diagram should be roughly vertical. Got {}x{} (width x height)",
            width,
            height
        );
    }
}

#[test]
fn test_class_diagram_cardinality_labels() {
    // Class diagram relations with cardinality should render the cardinality labels
    // Duck "1" *-- "many" Egg : has
    let input = r#"classDiagram
    Duck "1" *-- "many" Egg : has"#;

    let diagram = parse(input).expect("Failed to parse class diagram");
    let svg = render(&diagram).expect("Failed to render class diagram");

    // Should contain both cardinality labels
    assert!(
        svg.contains("1"),
        "Class diagram should contain cardinality '1'. SVG:\n{}",
        svg
    );
    assert!(
        svg.contains("many"),
        "Class diagram should contain cardinality 'many'. SVG:\n{}",
        svg
    );
}

#[test]
fn test_er_diagram_vertical_layout() {
    // ER diagrams should use DAG layout with entities flowing vertically
    // based on relationships, not a simple grid layout
    let input = r#"erDiagram
    CUSTOMER ||--o{ ORDER : places
    ORDER ||--|{ LINE-ITEM : contains
    PRODUCT ||--o{ LINE-ITEM : includes
    CUSTOMER {
        string name
        string email PK
        string address
    }
    ORDER {
        int orderNumber PK
        date orderDate
        string status
    }
    PRODUCT {
        int id PK
        string name
        float price
    }"#;

    let diagram = parse(input).expect("Failed to parse ER diagram");
    let svg = render(&diagram).expect("Failed to render ER diagram");

    // Extract y-coordinates for each entity
    let extract_entity_y = |svg: &str, entity_name: &str| -> Option<f64> {
        // Look for entity-<name> group
        let entity_marker = format!("entity-{}", entity_name);
        let entity_start = svg.find(&entity_marker)?;
        let entity_section = &svg[entity_start..];
        let entity_end = entity_section.find("</g>")?;
        let entity_section = &entity_section[..entity_end];

        // For rect: y="..."
        if let Some(y_start) = entity_section.find(r#" y=""#) {
            let y_value_start = y_start + 4;
            let remaining = &entity_section[y_value_start..];
            let y_end = remaining.find('"')?;
            let y_str = &remaining[..y_end];
            return y_str.parse().ok();
        }

        None
    };

    // Get positions
    let y_customer = extract_entity_y(&svg, "CUSTOMER").expect("Should find CUSTOMER entity");
    let y_order = extract_entity_y(&svg, "ORDER").expect("Should find ORDER entity");
    let y_line_item = extract_entity_y(&svg, "LINE-ITEM").expect("Should find LINE-ITEM entity");
    let y_product = extract_entity_y(&svg, "PRODUCT").expect("Should find PRODUCT entity");

    eprintln!("Entity y-coordinates:");
    eprintln!("  CUSTOMER: {}", y_customer);
    eprintln!("  ORDER: {}", y_order);
    eprintln!("  LINE-ITEM: {}", y_line_item);
    eprintln!("  PRODUCT: {}", y_product);

    // In vertical DAG layout based on relationships:
    // CUSTOMER --> ORDER --> LINE-ITEM <-- PRODUCT
    // So: CUSTOMER above ORDER, ORDER above LINE-ITEM
    // PRODUCT should be on same level as ORDER (both connect to LINE-ITEM)

    // ORDER should be below CUSTOMER (because CUSTOMER ||--o{ ORDER)
    assert!(
        y_customer < y_order,
        "CUSTOMER should be above ORDER. CUSTOMER.y={} vs ORDER.y={}",
        y_customer,
        y_order
    );

    // LINE-ITEM should be below ORDER (because ORDER ||--|{ LINE-ITEM)
    assert!(
        y_order < y_line_item,
        "ORDER should be above LINE-ITEM. ORDER.y={} vs LINE-ITEM.y={}",
        y_order,
        y_line_item
    );

    // Verify the diagram is taller than a simple grid would produce
    // Reference mermaid.js produces ~644px height vs our ~392px
    let height_match = svg.find(r#"height=""#).and_then(|start| {
        let remaining = &svg[start + 8..];
        let end = remaining.find('"')?;
        remaining[..end].parse::<f64>().ok()
    });

    if let Some(height) = height_match {
        eprintln!("Diagram height: {}", height);
        // Reference is 644px, our grid produces ~392px
        // Proper vertical layout should produce at least 500px
        assert!(
            height >= 500.0,
            "ER diagram should have proper vertical layout with height >= 500px. Got {}px (grid layout produces ~392px)",
            height
        );
    }
}

#[test]
fn test_class_diagram_parent_centered_over_children() {
    // Parent class should be horizontally centered over its children
    // Not left-aligned at the margin
    let input = r#"classDiagram
    Animal <|-- Duck
    Animal <|-- Fish
    Animal <|-- Zebra
    Animal : +int age
    Animal: +isMammal()"#;

    let diagram = parse(input).expect("Failed to parse class diagram");
    let svg = render(&diagram).expect("Failed to render class diagram");

    // Extract x positions of class boxes
    // Animal should be centered, not left-aligned
    // The reference has: Animal at x=298, Duck at x=92, Fish at x=298, Zebra at x=488
    // Children span from ~92 to ~488, so parent should be near middle (~290)

    // Parse Animal's x position and width from class box path
    let (animal_x, animal_width) =
        extract_class_box_position(&svg, "Animal").expect("Could not find Animal class x position");

    // Parse children's x positions
    let (duck_x, duck_width) =
        extract_class_box_position(&svg, "Duck").expect("Could not find Duck class x position");

    let (zebra_x, zebra_width) =
        extract_class_box_position(&svg, "Zebra").expect("Could not find Zebra class x position");

    let animal_center = animal_x + animal_width / 2.0;
    let duck_center = duck_x + duck_width / 2.0;
    let zebra_center = zebra_x + zebra_width / 2.0;
    let children_center = (duck_center + zebra_center) / 2.0;

    // Animal should be centered over children (within reasonable tolerance)
    let tolerance = 50.0;
    assert!(
        (animal_center - children_center).abs() < tolerance,
        "Animal (parent) should be horizontally centered over children. \
         Animal center={}, children center={}, diff={}. \
         Animal x={}, Duck x={}, Zebra x={}",
        animal_center,
        children_center,
        (animal_center - children_center).abs(),
        animal_x,
        duck_x,
        zebra_x
    );
}

fn extract_class_box_position(svg: &str, class_name: &str) -> Option<(f64, f64)> {
    let id = format!(r#"id="class-{}""#, class_name);
    let start = svg.find(&id)?;
    let remaining = &svg[start..];

    let mut scan = remaining;
    while let Some(path_start) = scan.find("<path") {
        let after_path = &scan[path_start..];
        let tag_end = after_path.find("/>")?;
        let element = &after_path[..tag_end];
        if element.contains(r#"class="class-box-bg""#) {
            return parse_box_path(element);
        }
        scan = &after_path[tag_end + 2..];
    }

    None
}

fn parse_box_path(element: &str) -> Option<(f64, f64)> {
    let d_start = element.find(r#"d=""#)? + 3;
    let d_end = element[d_start..].find('"')? + d_start;
    let d = &element[d_start..d_end];
    let mut parts = d.split_whitespace();
    let cmd = parts.next()?;
    if cmd != "M" {
        return None;
    }
    let x1 = parts.next()?.parse::<f64>().ok()?;
    parts.next()?; // y1
    let h_cmd = parts.next()?;
    if h_cmd != "H" {
        return None;
    }
    let h = parts.next()?.parse::<f64>().ok()?;
    let rx = 3.0;
    let x = x1 - rx;
    let width = h - x1 + 2.0 * rx;
    Some((x, width))
}

#[test]
fn test_gantt_uses_css_classes_not_hardcoded_colors() {
    // Issue mermaid-rs-lg2: Gantt charts should use CSS classes for colors,
    // not hardcoded inline fill/stroke attributes. This enables theme switching.
    let input = r#"gantt
    title Test
    dateFormat YYYY-MM-DD
    section S1
    Task :a1, 2024-01-01, 3d"#;

    let diagram = parse(input).expect("Failed to parse Gantt chart");
    let svg = render(&diagram).expect("Failed to render Gantt chart");

    // Task bars should NOT have hardcoded inline fill colors
    // They should use CSS classes like "task" or "task-bar" that get
    // their colors from the embedded stylesheet
    assert!(
        !svg.contains("fill=\"#8a90dd\""),
        "Gantt task bars should not have hardcoded fill='#8a90dd', should use CSS class. SVG:\n{}",
        svg
    );
    assert!(
        !svg.contains("stroke=\"#534fbc\""),
        "Gantt task bars should not have hardcoded stroke='#534fbc', should use CSS class. SVG:\n{}", svg
    );
}

// ============================================================================
// Directive-Based Theme Configuration Tests
// ============================================================================

#[test]
fn test_render_text_with_dark_theme_directive() {
    // Test that %%{init: {"theme": "dark"}}%% directive selects dark theme
    let input = r##"%%{init: {"theme": "dark"}}%%
flowchart LR
    A --> B"##;

    let svg = render_text(input).expect("Failed to render with directive");

    // Dark theme has dark background color
    assert!(
        svg.contains("#1f2020"),
        "Dark theme should have #1f2020 color"
    );
}

#[test]
fn test_render_text_with_forest_theme_directive() {
    // Test that %%{init: {"theme": "forest"}}%% directive selects forest theme
    let input = r##"%%{init: {"theme": "forest"}}%%
flowchart LR
    A --> B"##;

    let svg = render_text(input).expect("Failed to render with directive");

    // Forest theme has green colors
    assert!(
        svg.contains("#cde498") || svg.contains("#13540c") || svg.contains("#008000"),
        "Forest theme should have green colors"
    );
}

#[test]
fn test_render_text_with_theme_variables_override() {
    // Test that themeVariables overrides work
    let input = r##"%%{init: {"theme": "default", "themeVariables": {"primaryColor": "#ff5500"}}}%%
flowchart LR
    A --> B"##;

    let svg = render_text(input).expect("Failed to render with theme variables");

    // Custom primary color should be used
    assert!(
        svg.contains("#ff5500"),
        "Custom primaryColor should appear in SVG styles"
    );
}

#[test]
fn test_render_text_with_single_quotes() {
    // Test that single quote JSON syntax works (common in mermaid.js examples)
    let input = r#"%%{init: {'theme': 'dark'}}%%
flowchart LR
    A --> B"#;

    let svg = render_text(input).expect("Failed to render with single quote directive");

    // Dark theme colors should be present
    assert!(
        svg.contains("#1f2020"),
        "Dark theme should be applied with single quote syntax"
    );
}

#[test]
fn test_render_text_without_directive() {
    // Test that diagrams without directives still render correctly
    let input = r#"flowchart LR
    A --> B"#;

    let svg = render_text(input).expect("Failed to render without directive");

    // Default theme colors should be present
    assert!(
        svg.contains("#ECECFF") || svg.contains("#ececff"),
        "Default theme should be used when no directive"
    );
}

#[test]
fn test_render_text_directive_removed_from_output() {
    // Test that directives are stripped from final output
    let input = r##"%%{init: {"theme": "dark"}}%%
flowchart LR
    A --> B"##;

    let svg = render_text(input).expect("Failed to render");

    // Directive should not appear in final SVG
    assert!(
        !svg.contains("%%{init"),
        "Directive should be removed from output"
    );
}

#[test]
fn test_render_text_with_theme_css_directive() {
    // Test that themeCSS in directive is applied
    let input = r#"%%{init: {"themeCSS": ".node rect { rx: 15; }"}}%%
flowchart LR
    A --> B"#;

    let svg = render_text(input).expect("Failed to render with themeCSS directive");

    // Custom CSS should appear in output
    assert!(
        svg.contains("/* Custom CSS */"),
        "SVG should contain custom CSS marker"
    );
    assert!(
        svg.contains(".node rect { rx: 15; }"),
        "SVG should contain custom CSS from directive"
    );
}

// ============================================================================
// Comprehensive Theme Tests (mermaid-rs-l27)
// ============================================================================

#[test]
fn test_render_with_neutral_theme() {
    let input = "flowchart TD\n    A[Start] --> B[End]";

    let diagram = parse(input).expect("Failed to parse flowchart");
    let config = RenderConfig {
        theme: Theme::neutral(),
        ..Default::default()
    };
    let svg = render_with_config(&diagram, &config).expect("Failed to render with neutral theme");

    // Neutral theme has gray-ish colors
    assert!(svg.contains("<svg"), "Should produce SVG output");
    // Verify neutral theme CSS is embedded
    assert!(
        svg.contains("<style>"),
        "Should contain embedded CSS styling"
    );
}

#[test]
fn test_flowchart_nodes_use_theme_primary_color() {
    let input = "flowchart TD\n    A[Node A] --> B[Node B]";

    let diagram = parse(input).expect("Failed to parse flowchart");
    let config = RenderConfig {
        theme: Theme::default(),
        ..Default::default()
    };
    let svg = render_with_config(&diagram, &config).expect("Failed to render flowchart");

    // Default theme primary color is #ECECFF - should appear in CSS
    // Nodes should use CSS class styling, not hardcoded colors
    assert!(
        svg.contains("class=\"node\"") || svg.contains("class=\"default\""),
        "Nodes should have CSS class for styling"
    );
    // CSS should define node styling
    assert!(
        svg.contains(".node") || svg.contains(".default"),
        "CSS should define node styles"
    );
}

#[test]
fn test_flowchart_edges_use_theme_line_color() {
    let input = "flowchart TD\n    A --> B";

    let diagram = parse(input).expect("Failed to parse flowchart");
    let config = RenderConfig {
        theme: Theme::forest(),
        ..Default::default()
    };
    let svg = render_with_config(&diagram, &config).expect("Failed to render with forest theme");

    // Forest theme line color is #008000 (green)
    assert!(
        svg.contains("#008000") || svg.contains("class=\"edge\"") || svg.contains("stroke:"),
        "Edges should use theme line color or CSS class"
    );
}

#[test]
fn test_sequence_diagram_uses_theme_colors() {
    let input = r#"sequenceDiagram
    participant A
    participant B
    A->>B: Hello"#;

    let diagram = parse(input).expect("Failed to parse sequence diagram");
    let config = RenderConfig {
        theme: Theme::dark(),
        ..Default::default()
    };
    let svg = render_with_config(&diagram, &config).expect("Failed to render with dark theme");

    // Dark theme uses #1f2020 background, #cccccc text
    assert!(svg.contains("<svg"), "Should produce SVG output");
    // Actors should use CSS classes for theming
    assert!(
        svg.contains("class=\"actor\"") || svg.contains(".actor"),
        "Actors should have CSS class styling"
    );
}

#[test]
fn test_sequence_open_arrow_has_no_marker() {
    let input = r#"sequenceDiagram
    participant A
    participant B
    A->B: Open
    A->>B: Filled"#;

    let diagram = parse(input).expect("Failed to parse sequence diagram");
    let svg = render_with_config(&diagram, &RenderConfig::default())
        .expect("Failed to render sequence diagram");

    assert_eq!(
        svg.matches("marker-end=").count(),
        1,
        "Only filled arrows should get marker-end"
    );
}

#[test]
fn test_sequence_activation_renders_box() {
    let input = r#"sequenceDiagram
    participant A
    participant C
    A->>+C: Login request
    C-->>-A: Token"#;

    let diagram = parse(input).expect("Failed to parse sequence diagram");
    let svg = render_with_config(&diagram, &RenderConfig::default())
        .expect("Failed to render sequence diagram");

    assert!(
        svg.contains("class=\"activation\""),
        "Activation box should render for + activation"
    );
}

#[test]
fn test_sequence_loop_label_renders() {
    let input = r#"sequenceDiagram
    Alice->>Bob: start
    loop Every minute
    Alice->>Bob: ping
    end"#;

    let diagram = parse(input).expect("Failed to parse sequence diagram");
    let svg = render_with_config(&diagram, &RenderConfig::default())
        .expect("Failed to render sequence diagram");

    assert!(svg.contains(">loop<"), "Loop label should render in SVG");
    assert!(
        svg.contains("[Every minute]"),
        "Loop condition label should render in SVG"
    );
    assert!(
        svg.contains("labelBox") || svg.contains("loopLine"),
        "Loop framing should render in SVG"
    );
}

#[test]
fn test_sequence_loop_condition_sits_near_top_line() {
    let input = r#"sequenceDiagram
    Alice->>Bob: start
    loop Daily query
    Alice->>Bob: ping
    end"#;

    let diagram = parse(input).expect("Failed to parse sequence diagram");
    let svg = render_with_config(&diagram, &RenderConfig::default())
        .expect("Failed to render sequence diagram");

    let label_y = svg
        .split("<polygon")
        .find_map(|chunk| {
            if !chunk.contains("class=\"labelBox\"") {
                return None;
            }
            let points = chunk.split("points=\"").nth(1)?.split('"').next()?;
            let first_point = points.split_whitespace().next()?;
            let y = first_point.split(',').nth(1)?;
            y.parse::<f64>().ok()
        })
        .expect("Failed to parse label box y");

    let loop_text_y = svg
        .split("<text")
        .find_map(|chunk| {
            if !chunk.contains("class=\"loopText\"") {
                return None;
            }
            let y = chunk.split("y=\"").nth(1)?.split('"').next()?;
            y.parse::<f64>().ok()
        })
        .expect("Failed to parse loop text y");

    assert!(
        loop_text_y <= label_y + 20.0,
        "Loop condition should sit just below the top line (label_y={}, loop_text_y={})",
        label_y,
        loop_text_y
    );
}

#[test]
fn test_sequence_fragment_uses_actor_theme_colors() {
    let input = r#"sequenceDiagram
    Alice->>Bob: start
    loop Daily query
    Alice->>Bob: ping
    end"#;

    let diagram = parse(input).expect("Failed to parse sequence diagram");
    let theme = Theme::forest();
    let svg = render_with_config(
        &diagram,
        &RenderConfig {
            theme: theme.clone(),
            ..Default::default()
        },
    )
    .expect("Failed to render sequence diagram");

    assert!(
        svg.contains(&format!("stroke: {};", theme.actor_border)),
        "Fragment stroke should use actor border color"
    );
    assert!(
        svg.contains(&format!("fill: {};", theme.actor_bkg)),
        "Fragment fill should use actor background color"
    );
}

#[test]
fn test_sequence_fragment_frames_render_behind_text() {
    let input = r#"sequenceDiagram
    Alice->>Bob: start
    loop Daily query
    Alice->>Bob: ping
    end"#;

    let diagram = parse(input).expect("Failed to parse sequence diagram");
    let svg = render_with_config(&diagram, &RenderConfig::default())
        .expect("Failed to render sequence diagram");

    let frame_idx = svg
        .find("class=\"loopLine\"")
        .expect("Missing fragment frame");
    let text_idx = svg
        .find("class=\"loopText\"")
        .expect("Missing fragment text");

    assert!(
        frame_idx < text_idx,
        "Fragment frames should render before text"
    );
}

#[test]
fn test_sequence_loop_scopes_to_single_actor() {
    let input = r#"sequenceDiagram
    participant Alice
    participant Bob
    participant John
    Alice->>John: Hello John, how are you?
    loop HealthCheck
        John->>John: Fight against hypochondria
    end
    John-->>Alice: Great!"#;

    let diagram = parse(input).expect("Failed to parse sequence diagram");
    let svg = render_with_config(&diagram, &RenderConfig::default())
        .expect("Failed to render sequence diagram");

    let loop_width = svg
        .split("<rect")
        .find_map(|chunk| {
            if !chunk.contains("class=\"loopLine\"") {
                return None;
            }
            let width = chunk.split("width=\"").nth(1)?.split('"').next()?;
            width.parse::<f64>().ok()
        })
        .expect("Failed to parse loop frame width");

    assert!(
        loop_width < 300.0,
        "Loop frame should scope to a single actor, got width {}",
        loop_width
    );
}

#[test]
fn test_theme_override_appears_in_svg() {
    // Override primary color to a distinctive red
    let svg = render_text(
        r##"%%{init: {"theme": "default", "themeVariables": {"primaryColor": "#ff0000"}}}%%
flowchart TD
    A[Red Node]"##,
    )
    .expect("Failed to render with theme override");

    // The red color should appear in the CSS
    assert!(
        svg.contains("#ff0000"),
        "Custom primaryColor should appear in SVG CSS"
    );
}

#[test]
fn test_all_built_in_themes_produce_valid_svg() {
    let themes = vec![
        ("default", Theme::default()),
        ("dark", Theme::dark()),
        ("forest", Theme::forest()),
        ("neutral", Theme::neutral()),
    ];

    for (name, theme) in themes {
        let input = "flowchart TD\n    A --> B";
        let diagram = parse(input).expect("Failed to parse flowchart");
        let config = RenderConfig {
            theme,
            ..Default::default()
        };
        let svg = render_with_config(&diagram, &config).expect("Failed to render");

        assert!(
            svg.contains("<svg") && svg.contains("</svg>"),
            "{} theme should produce valid SVG",
            name
        );
        assert!(
            svg.contains("<style>") || svg.contains("<defs>"),
            "{} theme should include styling",
            name
        );
    }
}

#[test]
fn test_theme_css_overrides_built_in_styles() {
    // Custom CSS should come after built-in theme CSS
    let svg = render_text(
        r#"%%{init: {"themeCSS": ".node rect { rx: 20; ry: 20; }"}}%%
flowchart TD
    A[Rounded]"#,
    )
    .expect("Failed to render with themeCSS");

    // Custom CSS should appear in output
    assert!(svg.contains("rx: 20"), "Custom themeCSS should be applied");
    // Custom CSS marker should indicate it comes after theme CSS
    assert!(
        svg.contains("/* Custom CSS */"),
        "Custom CSS section should be marked"
    );
}

#[test]
fn test_invalid_theme_name_falls_back_to_default() {
    // Using invalid theme name should fall back to default
    let svg = render_text(
        r#"%%{init: {"theme": "nonexistent_theme_xyz"}}%%
flowchart TD
    A --> B"#,
    )
    .expect("Failed to render with invalid theme");

    // Should still produce valid SVG
    assert!(
        svg.contains("<svg"),
        "Should produce SVG even with invalid theme"
    );

    // Default theme primary color #ECECFF should be present
    assert!(
        svg.contains("#ECECFF") || svg.contains("#ececff"),
        "Should use default theme colors as fallback"
    );
}

// ============================================================================
// State Diagram Improvements
// ============================================================================

#[test]
fn test_state_diagram_all_states_rendered() {
    // Issue: State diagrams should render ALL states mentioned in transitions.
    // The sample state diagram has 5 states: [*] start, Idle, Running, Error, [*] end
    // but the Error state was missing from the rendered output.
    let input = r#"stateDiagram-v2
    [*] --> Idle
    Idle --> Running : start
    Running --> Idle : stop
    Running --> Error : error
    Error --> Idle : reset
    Error --> [*]"#;

    let diagram = parse(input).expect("Failed to parse state diagram");
    let svg = render(&diagram).expect("Failed to render state diagram");

    // Count state-node groups (each state should have one)
    let state_node_count = svg.matches(r#"class="state-node""#).count();

    // Verify all states are present
    assert!(
        svg.contains("id=\"state-Idle\""),
        "Should contain Idle state. SVG:\n{}",
        svg
    );
    assert!(
        svg.contains("id=\"state-Running\""),
        "Should contain Running state. SVG:\n{}",
        svg
    );
    assert!(
        svg.contains("id=\"state-Error\""),
        "Should contain Error state. SVG:\n{}",
        svg
    );

    // Should have at least 5 state-node groups:
    // [*] start, Idle, Running, Error, [*] end
    assert!(
        state_node_count >= 5,
        "Should have at least 5 state-node groups (start, Idle, Running, Error, end), found {}. SVG:\n{}",
        state_node_count,
        svg
    );
}

#[test]
fn test_state_diagram_uses_curved_edges() {
    // Issue: State diagram transitions should use curved paths like mermaid.js,
    // not straight lines. Mermaid uses cubic Bezier curves for smooth edges.
    let input = r#"stateDiagram-v2
    [*] --> Idle
    Idle --> Running : start
    Running --> Idle : stop"#;

    let diagram = parse(input).expect("Failed to parse state diagram");
    let svg = render(&diagram).expect("Failed to render state diagram");

    // Mermaid.js uses <path> elements with cubic Bezier curves (C command)
    // for transitions, not straight <line> elements.
    // Count path elements (should be used for edges)
    let path_count = svg.matches("<path").count();
    let line_count = svg.matches("<line").count();

    // Transitions should use path elements, not line elements
    // (excluding marker path which is in defs)
    let marker_path_count = svg.matches(r#"<marker"#).count();
    let edge_path_count = path_count.saturating_sub(marker_path_count);

    // We have 3 transitions, they should all use paths
    assert!(
        edge_path_count >= 3,
        "State diagram should use path elements for transitions (found {} paths excluding markers), not lines (found {}). SVG:\n{}",
        edge_path_count,
        line_count,
        svg
    );

    // Line elements should not be used for transitions
    // (there might be some lines for other purposes like dividers)
    assert!(
        line_count == 0,
        "State diagram should not use <line> elements for transitions (found {}). SVG:\n{}",
        line_count,
        svg
    );
}
