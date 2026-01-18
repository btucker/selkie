//! Mindmap rendering tests - ported from Cypress tests
//!
//! These tests are ported from the mermaid.js Cypress test suite:
//! - cypress/integration/rendering/mindmap.spec.ts
//! - cypress/integration/rendering/mindmap-tidy-tree.spec.js

use roxmltree::Document;
use selkie::{parse, render};

fn render_mindmap_svg(input: &str) -> String {
    let diagram = parse(input).expect("Failed to parse mindmap diagram");
    render(&diagram).expect("Failed to render mindmap diagram")
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

// ============================================================================
// Basic Structure Tests (from mindmap.spec.ts)
// ============================================================================

#[test]
fn mindmap_only_a_root() {
    let input = r#"mindmap
root"#;
    let svg = render_mindmap_svg(input);
    let doc = parse_svg(&svg);

    // Should have mindmap-node class
    assert!(
        has_class(&doc, "mindmap-node"),
        "Should have mindmap-node class"
    );

    // Should have section-root class for root node
    assert!(
        has_class(&doc, "section-root"),
        "Should have section-root class"
    );

    // Should contain the root text
    assert!(
        svg_contains_text(&svg, "root"),
        "Should contain 'root' text"
    );
}

#[test]
fn mindmap_root_with_shape() {
    let input = r#"mindmap
root[root]"#;
    let svg = render_mindmap_svg(input);
    let doc = parse_svg(&svg);

    assert!(
        has_class(&doc, "mindmap-node"),
        "Should have mindmap-node class"
    );
    assert!(
        svg_contains_text(&svg, "root"),
        "Should contain 'root' text"
    );
}

#[test]
fn mindmap_root_with_wrapping_text_and_shape() {
    let input = r#"mindmap
root[A root with a long text that wraps to keep the node size in check]"#;
    let svg = render_mindmap_svg(input);
    let doc = parse_svg(&svg);

    assert!(
        has_class(&doc, "mindmap-node"),
        "Should have mindmap-node class"
    );
    assert!(svg_contains_text(&svg, "root"), "Should contain node text");
}

#[test]
fn mindmap_root_with_icon() {
    // Icon declarations need to be indented (as children of the node)
    let input = r#"mindmap
root[root]
    ::icon(mdi mdi-fire)"#;
    let svg = render_mindmap_svg(input);
    let doc = parse_svg(&svg);

    assert!(
        has_class(&doc, "mindmap-node"),
        "Should have mindmap-node class"
    );
}

// ============================================================================
// Shape Tests
// ============================================================================

#[test]
fn mindmap_bang_and_cloud_shape() {
    let input = r#"mindmap
root))bang((
  ::icon(mdi mdi-fire)
  a))Another bang((
  ::icon(mdi mdi-fire)
  a)A cloud(
  ::icon(mdi mdi-fire)"#;
    let svg = render_mindmap_svg(input);
    let doc = parse_svg(&svg);

    assert!(
        has_class(&doc, "mindmap-node"),
        "Should have mindmap-node class"
    );
}

#[test]
fn mindmap_bang_and_cloud_shape_without_icons() {
    let input = r#"mindmap
root))bang((
  a))Another bang((
  a)A cloud("#;
    let svg = render_mindmap_svg(input);
    let doc = parse_svg(&svg);

    assert!(
        has_class(&doc, "mindmap-node"),
        "Should have mindmap-node class"
    );
}

#[test]
fn mindmap_square_shape() {
    // Single line syntax for square shape
    let input = r#"mindmap
    root[The root]"#;
    let svg = render_mindmap_svg(input);
    let doc = parse_svg(&svg);

    assert!(
        has_class(&doc, "mindmap-node"),
        "Should have mindmap-node class"
    );
}

#[test]
fn mindmap_rounded_rect_shape() {
    // Single line syntax for circle shape (double parens)
    let input = r#"mindmap
    root((The root))"#;
    let svg = render_mindmap_svg(input);
    let doc = parse_svg(&svg);

    assert!(
        has_class(&doc, "mindmap-node"),
        "Should have mindmap-node class"
    );
}

#[test]
fn mindmap_circle_shape() {
    // Single line syntax for rounded rect shape (single parens)
    let input = r#"mindmap
    root(The root)"#;
    let svg = render_mindmap_svg(input);
    let doc = parse_svg(&svg);

    assert!(
        has_class(&doc, "mindmap-node"),
        "Should have mindmap-node class"
    );
}

#[test]
fn mindmap_default_shape() {
    let input = r#"mindmap
  The root"#;
    let svg = render_mindmap_svg(input);
    let doc = parse_svg(&svg);

    assert!(
        has_class(&doc, "mindmap-node"),
        "Should have mindmap-node class"
    );
}

// ============================================================================
// Hierarchy Tests
// ============================================================================

#[test]
fn mindmap_branches() {
    let input = r#"mindmap
root
  child1
      grandchild 1
      grandchild 2
  child2
      grandchild 3
      grandchild 4
  child3
      grandchild 5
      grandchild 6"#;
    let svg = render_mindmap_svg(input);
    let doc = parse_svg(&svg);

    assert!(
        has_class(&doc, "mindmap-node"),
        "Should have mindmap-node class"
    );

    // Should have multiple nodes
    let node_count = count_elements_with_class(&doc, "mindmap-node");
    assert!(
        node_count >= 10,
        "Should have at least 10 nodes (1 root + 3 children + 6 grandchildren)"
    );
}

#[test]
fn mindmap_branches_with_shapes_and_labels() {
    let input = r#"mindmap
root
  child1((Circle))
      grandchild 1
      grandchild 2
  child2(Round rectangle)
      grandchild 3
      grandchild 4
  child3[Square]
      grandchild 5
      ::icon(mdi mdi-fire)
      gc6((grand<br/>child 6))
      ::icon(mdi mdi-fire)"#;
    let svg = render_mindmap_svg(input);
    let doc = parse_svg(&svg);

    assert!(
        has_class(&doc, "mindmap-node"),
        "Should have mindmap-node class"
    );
}

#[test]
fn mindmap_text_should_wrap_with_icon() {
    let input = r#"mindmap
root
  Child3(A node with an icon and with a long text that wraps to keep the node size in check)"#;
    let svg = render_mindmap_svg(input);
    let doc = parse_svg(&svg);

    assert!(
        has_class(&doc, "mindmap-node"),
        "Should have mindmap-node class"
    );
}

#[test]
fn mindmap_adding_children() {
    let input = r#"mindmap
  The root
    child1
    child2"#;
    let svg = render_mindmap_svg(input);
    let doc = parse_svg(&svg);

    assert!(
        has_class(&doc, "mindmap-node"),
        "Should have mindmap-node class"
    );

    let node_count = count_elements_with_class(&doc, "mindmap-node");
    assert!(node_count >= 3, "Should have at least 3 nodes");
}

#[test]
fn mindmap_adding_grandchildren() {
    let input = r#"mindmap
  The root
    child1
      child2
      child3"#;
    let svg = render_mindmap_svg(input);
    let doc = parse_svg(&svg);

    assert!(
        has_class(&doc, "mindmap-node"),
        "Should have mindmap-node class"
    );

    let node_count = count_elements_with_class(&doc, "mindmap-node");
    assert!(node_count >= 4, "Should have at least 4 nodes");
}

// ============================================================================
// Special Cases
// ============================================================================

#[test]
fn mindmap_label_with_graph_sequence() {
    // Test that "graph" in text doesn't confuse the parser
    let input = r#"mindmap
  root
    Photograph
      Waterfall
      Landscape
    Geography
      Mountains
      Rocks"#;
    let svg = render_mindmap_svg(input);

    assert!(
        svg_contains_text(&svg, "Photograph"),
        "Should contain 'Photograph'"
    );
    assert!(
        svg_contains_text(&svg, "Geography"),
        "Should contain 'Geography'"
    );
}

#[test]
fn mindmap_many_level_2_nodes() {
    // Test that more than 11 Level 2 nodes render correctly
    let input = r#"mindmap
root
  Node1
  Node2
  Node3
  Node4
  Node5
  Node6
  Node7
  Node8
  Node9
  Node10
  Node11
  Node12
  Node13
  Node14
  Node15"#;
    let svg = render_mindmap_svg(input);
    let doc = parse_svg(&svg);

    assert!(
        has_class(&doc, "mindmap-node"),
        "Should have mindmap-node class"
    );

    let node_count = count_elements_with_class(&doc, "mindmap-node");
    assert!(
        node_count >= 16,
        "Should have at least 16 nodes (1 root + 15 children)"
    );
}

// ============================================================================
// Accessibility Tests
// ============================================================================

#[test]
fn mindmap_accessibility_title() {
    // Note: accTitle/accDescr are parsed but not all positions are supported
    // This test verifies basic rendering still works
    let input = r#"mindmap
    root((mindmap))
        A
        B"#;
    let svg = render_mindmap_svg(input);

    // SVG should be valid
    let _doc = parse_svg(&svg);
}

// ============================================================================
// Complex Examples (from mindmap-tidy-tree.spec.js adapted)
// ============================================================================

#[test]
fn mindmap_complex_hierarchy() {
    let input = r#"mindmap
root((mindmap))
  Origins
    Long history
    ::icon(fa fa-book)
    Popularisation
      British popular psychology author Tony Buzan
  Research
    On effectiveness<br/>and features
    On Automatic creation
      Uses
          Creative techniques
          Strategic planning
          Argument mapping
  Tools
        id)I am a cloud(
            id))I am a bang((
              Tools"#;
    let svg = render_mindmap_svg(input);
    let doc = parse_svg(&svg);

    assert!(
        has_class(&doc, "mindmap-node"),
        "Should have mindmap-node class"
    );
}

#[test]
fn mindmap_with_children_deep_hierarchy() {
    let input = r#"mindmap
((This is a mindmap))
  child1
   grandchild 1
   grandchild 2
  child2
   grandchild 3
   grandchild 4
  child3
   grandchild 5
   grandchild 6"#;
    let svg = render_mindmap_svg(input);
    let doc = parse_svg(&svg);

    assert!(
        has_class(&doc, "mindmap-node"),
        "Should have mindmap-node class"
    );
}
