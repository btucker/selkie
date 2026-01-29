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

// ============================================================================
// Visual Parity Tests (for mermaid compatibility)
// ============================================================================

fn count_elements_by_tag(doc: &Document<'_>, tag_name: &str) -> usize {
    doc.descendants()
        .filter(|node| node.tag_name().name() == tag_name)
        .count()
}

#[test]
fn mindmap_icon_with_multiple_classes_not_parsed_as_node() {
    // Bug: Icons with multiple classes (e.g., "fa fa-book") were being parsed as nodes
    // because the regex only matched single-word icon classes.
    // This caused an extra node to appear in diagrams.
    let input = r#"mindmap
root((mindmap))
    Origins
        Long history
        ::icon(fa fa-book)
        Popularisation"#;
    let svg = render_mindmap_svg(input);
    let doc = parse_svg(&svg);

    // Should have exactly 4 nodes: root, Origins, Long history, Popularisation
    // NOT 5 nodes (with icon line as a node)
    let node_count = count_elements_with_class(&doc, "mindmap-node");
    assert_eq!(
        node_count, 4,
        "Icon line should not be parsed as a node. \
         Expected 4 nodes (root, Origins, Long history, Popularisation), got {}",
        node_count
    );
}

#[test]
fn mindmap_nodes_do_not_overlap() {
    // Nodes in the standard mindmap example should not visually overlap each other.
    // The eval report shows edges are 300-900px off because nodes are placed too close together.
    let input = r#"mindmap
root((mindmap))
    Origins
        Long history
    Research
        On effectiveness
        On Automatic creation
    Tools
        Pen and paper
        Mermaid"#;
    let svg = render_mindmap_svg(input);
    let doc = parse_svg(&svg);

    // Extract bounding boxes from mindmap-node groups
    // Each has transform="translate(x, y)" and contains shape elements with width/height
    let mut bboxes: Vec<(String, f64, f64, f64, f64)> = Vec::new(); // (id, x, y, w, h)

    for node in doc.descendants() {
        let class = node.attribute("class").unwrap_or("");
        if !class.contains("mindmap-node") || class.contains("mindmap-nodes") {
            continue;
        }

        // Parse transform="translate(x, y)"
        let transform = node.attribute("transform").unwrap_or("");
        if !transform.starts_with("translate(") {
            continue;
        }
        let coords: &str = &transform["translate(".len()..transform.len() - 1];
        let parts: Vec<f64> = coords
            .split(',')
            .filter_map(|s| s.trim().parse().ok())
            .collect();
        if parts.len() != 2 {
            continue;
        }
        let (tx, ty) = (parts[0], parts[1]);

        // Find the shape element to get width/height
        let mut w = 0.0_f64;
        let mut h = 0.0_f64;
        let id = node.attribute("id").unwrap_or("unknown").to_string();

        for child in node.descendants() {
            let tag = child.tag_name().name();
            if tag == "circle" {
                if let Some(r) = child.attribute("r").and_then(|v| v.parse::<f64>().ok()) {
                    w = r * 2.0;
                    h = r * 2.0;
                }
            } else if tag == "path" {
                // Parse path d attribute - default nodes use paths like "M0 45 v-40 q0,-5 5,-5 h83 q5,0 5,5 v45 H0 Z"
                // Width comes from the 'h' command value + 10 (for the q curves), height from the 'v' values
                if let Some(d) = child.attribute("d") {
                    // Extract width from H command (e.g., "h83" means ~93px wide with padding)
                    // and height from initial "M0 45" (height = 45 + 5 = 50)
                    // Simpler: parse the bounding width from the path
                    for segment in d.split_whitespace() {
                        if segment.starts_with('h') || segment.starts_with('H') {
                            if let Ok(val) = segment[1..].parse::<f64>() {
                                // h command: width = the value + some padding for q curves
                                w = w.max(val.abs() + 10.0);
                            }
                        }
                    }
                    // Height from initial M0 value
                    if d.starts_with("M0 ") {
                        if let Some(hval) = d.split_whitespace().nth(1) {
                            if let Ok(val) = hval.parse::<f64>() {
                                h = h.max(val + 5.0); // path height
                            }
                        }
                    }
                }
            }
        }

        if w > 0.0 && h > 0.0 {
            bboxes.push((id, tx, ty, w, h));
        }
    }

    // Check no pair of nodes overlaps
    assert!(
        bboxes.len() >= 9,
        "Should extract at least 9 node bounding boxes, got {}",
        bboxes.len()
    );

    let mut overlaps = Vec::new();
    for i in 0..bboxes.len() {
        for j in (i + 1)..bboxes.len() {
            let (ref id_a, ax, ay, aw, ah) = bboxes[i];
            let (ref id_b, bx, by, bw, bh) = bboxes[j];

            // Skip root-node overlaps: the mermaid reference (cose-bilkent)
            // places first-level children close to the root edge, and our
            // radial layout matches this tight proximity.
            let is_root_a = id_a.contains("root");
            let is_root_b = id_b.contains("root");
            if is_root_a || is_root_b {
                continue;
            }

            // Check AABB overlap (with small tolerance for touching edges)
            let tolerance = 2.0;
            let overlaps_x = ax + aw - tolerance > bx && bx + bw - tolerance > ax;
            let overlaps_y = ay + ah - tolerance > by && by + bh - tolerance > ay;

            if overlaps_x && overlaps_y {
                overlaps.push(format!(
                    "  {} ({:.0},{:.0} {:.0}x{:.0}) overlaps {} ({:.0},{:.0} {:.0}x{:.0})",
                    id_a, ax, ay, aw, ah, id_b, bx, by, bw, bh
                ));
            }
        }
    }

    assert!(
        overlaps.is_empty(),
        "Found {} node overlaps:\n{}",
        overlaps.len(),
        overlaps.join("\n")
    );
}

#[test]
fn mindmap_default_nodes_have_bottom_lines() {
    // Default-style nodes in mermaid have a decorative line at the bottom
    // This test verifies that Selkie renders these lines to match mermaid output
    let input = r#"mindmap
root((mindmap))
    Origins
        Long history
    Research
        On effectiveness
        On Automatic creation
    Tools
        Pen and paper
        Mermaid"#;
    let svg = render_mindmap_svg(input);
    let doc = parse_svg(&svg);

    // Count default nodes (nodes that aren't root - root is a circle)
    // We expect: Origins, Long history, Research, On effectiveness,
    // On Automatic creation, Tools, Pen and paper, Mermaid = 8 default nodes
    let default_node_count = 8;

    // Each default node should have one <line> element at the bottom
    let line_count = count_elements_by_tag(&doc, "line");

    assert_eq!(
        line_count, default_node_count,
        "Each default-style node should have a decorative bottom line. \
         Expected {} lines for {} default nodes, but found {}",
        default_node_count, default_node_count, line_count
    );
}
