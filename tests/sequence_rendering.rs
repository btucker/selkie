//! Tests for sequence diagram rendering to match mermaid.js reference output

use selkie::{parse, render};

const TEST_CHAR_WIDTH: f64 = 8.0;
const TEST_LINE_HEIGHT: f64 = 18.0;

fn render_sequence(input: &str) -> String {
    let diagram = parse(input).expect("Failed to parse");
    render(&diagram).expect("Failed to render")
}

#[derive(Debug, Clone)]
struct TestBox {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
}

impl TestBox {
    fn right(&self) -> f64 {
        self.x + self.width
    }

    fn bottom(&self) -> f64 {
        self.y + self.height
    }

    fn overlaps(&self, other: &Self, tolerance: f64) -> bool {
        self.x + tolerance < other.x + other.width - tolerance
            && self.x + self.width - tolerance > other.x + tolerance
            && self.y + tolerance < other.y + other.height - tolerance
            && self.y + self.height - tolerance > other.y + tolerance
    }
}

fn parse_num(node: roxmltree::Node<'_, '_>, attr: &str) -> f64 {
    node.attribute(attr)
        .unwrap_or_else(|| panic!("missing {attr}"))
        .parse::<f64>()
        .unwrap_or_else(|_| panic!("invalid {attr}"))
}

fn find_text_box(svg: &str, label: &str) -> TestBox {
    let doc = roxmltree::Document::parse(svg).expect("valid svg");
    let text = doc
        .descendants()
        .find(|n| n.tag_name().name() == "text" && node_text(*n).contains(label))
        .unwrap_or_else(|| panic!("missing text {label}"));
    let content = node_text(text);
    let width = content.chars().count() as f64 * TEST_CHAR_WIDTH;
    let mut x = parse_num(text, "x");
    match text.attribute("text-anchor").unwrap_or("start") {
        "middle" => x -= width / 2.0,
        "end" => x -= width,
        _ => {}
    }

    let y = parse_num(text, "y");
    TestBox {
        x,
        y: text_box_y(text, y),
        width,
        height: TEST_LINE_HEIGHT,
    }
}

fn node_text(node: roxmltree::Node<'_, '_>) -> String {
    node.descendants()
        .filter(|n| n.is_text())
        .filter_map(|n| n.text())
        .collect::<Vec<_>>()
        .join("")
}

fn text_box_y(node: roxmltree::Node<'_, '_>, y: f64) -> f64 {
    if node.attribute("class").unwrap_or("").contains("noteText") && has_middle_baseline(node) {
        return y + parse_text_dy(node).unwrap_or(0.0) - (TEST_LINE_HEIGHT / 2.0);
    }

    y - TEST_LINE_HEIGHT
}

fn has_middle_baseline(node: roxmltree::Node<'_, '_>) -> bool {
    matches!(
        node.attribute("dominant-baseline"),
        Some("middle" | "central")
    ) || matches!(
        node.attribute("alignment-baseline"),
        Some("middle" | "central")
    )
}

fn parse_text_dy(node: roxmltree::Node<'_, '_>) -> Option<f64> {
    let dy = node.attribute("dy")?;
    if let Some(em) = dy.strip_suffix("em") {
        let font_size = node
            .attribute("font-size")
            .and_then(|value| value.parse::<f64>().ok())
            .unwrap_or(16.0);
        return Some(em.parse::<f64>().ok()? * font_size);
    }

    dy.parse::<f64>().ok()
}

fn find_note_box_containing(svg: &str, label: &str) -> TestBox {
    let doc = roxmltree::Document::parse(svg).expect("valid svg");
    let note_text = doc
        .descendants()
        .find(|n| n.tag_name().name() == "text" && node_text(*n).contains(label))
        .unwrap_or_else(|| panic!("missing note text {label}"));
    let note_group = note_text.ancestors().find(|n| n.tag_name().name() == "g");
    if let Some(group) = note_group {
        if let Some(rect) = group.descendants().find(|n| {
            n.tag_name().name() == "rect" && n.attribute("class").unwrap_or("").contains("note")
        }) {
            return TestBox {
                x: parse_num(rect, "x"),
                y: parse_num(rect, "y"),
                width: parse_num(rect, "width"),
                height: parse_num(rect, "height"),
            };
        }
    }
    panic!("missing note rect for {label}");
}

fn find_actor_box_containing(svg: &str, label: &str) -> TestBox {
    let doc = roxmltree::Document::parse(svg).expect("valid svg");
    let actor_text = doc
        .descendants()
        .find(|n| n.tag_name().name() == "text" && node_text(*n).contains(label))
        .unwrap_or_else(|| panic!("missing actor text {label}"));
    let actor_group = actor_text.ancestors().find(|n| {
        n.tag_name().name() == "g" && n.attribute("class").unwrap_or("").contains("actor")
    });
    if let Some(group) = actor_group {
        if let Some(rect) = group.descendants().find(|n| {
            n.tag_name().name() == "rect"
                && n.attribute("class").unwrap_or("").contains("actor-box")
        }) {
            return TestBox {
                x: parse_num(rect, "x"),
                y: parse_num(rect, "y"),
                width: parse_num(rect, "width"),
                height: parse_num(rect, "height"),
            };
        }
    }
    panic!("missing actor rect for {label}");
}

fn find_lifeline_x_for_actor(svg: &str, label: &str) -> f64 {
    let actor = find_actor_box_containing(svg, label);
    let center_x = actor.x + actor.width / 2.0;
    let doc = roxmltree::Document::parse(svg).expect("valid svg");
    doc.descendants()
        .find(|n| {
            n.tag_name().name() == "line"
                && n.attribute("class").unwrap_or("").contains("actor-line")
                && (parse_num(*n, "x1") - center_x).abs() < 0.1
                && (parse_num(*n, "x2") - center_x).abs() < 0.1
        })
        .map(|line| parse_num(line, "x1"))
        .unwrap_or_else(|| panic!("missing lifeline for {label}"))
}

fn svg_visible_right(svg: &str) -> f64 {
    let doc = roxmltree::Document::parse(svg).expect("valid svg");
    let root = doc.root_element();
    let width = root.attribute("width").unwrap().parse::<f64>().unwrap();
    let view_box = root.attribute("viewBox").unwrap_or("0 0 0 0");
    let parts: Vec<f64> = view_box
        .split_whitespace()
        .map(|p| p.parse().unwrap())
        .collect();

    parts[0] + width
}

fn svg_width(svg: &str) -> f64 {
    let doc = roxmltree::Document::parse(svg).expect("valid svg");
    doc.root_element()
        .attribute("width")
        .unwrap()
        .parse::<f64>()
        .unwrap()
}

fn find_self_message_path_box(svg: &str) -> TestBox {
    let doc = roxmltree::Document::parse(svg).expect("valid svg");
    let path = doc
        .descendants()
        .find(|n| {
            if n.tag_name().name() != "path" || n.attribute("marker-end").is_none() {
                return false;
            }

            let class = n.attribute("class").unwrap_or("");
            let is_message_path = class.contains("message-line")
                || class.contains("messageLine0")
                || class.contains("messageLine1");
            let is_self_loop_shape = n
                .attribute("d")
                .map(|d| path_points(d).len() >= 3)
                .unwrap_or(false);

            is_message_path && is_self_loop_shape
        })
        .unwrap_or_else(|| panic!("missing self-message path\n{svg}"));
    let points = path_points(path.attribute("d").expect("path d"));
    assert!(
        points.len() >= 3,
        "unsupported self-message path geometry: {:?}",
        path.attribute("d")
    );
    box_from_points(&points)
}

fn path_points(path: &str) -> Vec<(f64, f64)> {
    let normalized: String = path
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphabetic() || ch == ',' {
                ' '
            } else {
                ch
            }
        })
        .collect();
    let nums: Vec<f64> = normalized
        .split_whitespace()
        .filter_map(|part| part.parse::<f64>().ok())
        .collect();

    nums.chunks_exact(2)
        .map(|chunk| (chunk[0], chunk[1]))
        .collect()
}

fn box_from_points(points: &[(f64, f64)]) -> TestBox {
    assert!(!points.is_empty(), "missing path points");
    let mut min_x = f64::INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut max_y = f64::NEG_INFINITY;

    for (x, y) in points {
        min_x = min_x.min(*x);
        min_y = min_y.min(*y);
        max_x = max_x.max(*x);
        max_y = max_y.max(*y);
    }

    TestBox {
        x: min_x,
        y: min_y,
        width: max_x - min_x,
        height: max_y - min_y,
    }
}

fn find_first_fragment_frame(svg: &str) -> TestBox {
    let doc = roxmltree::Document::parse(svg).expect("valid svg");
    let mut min_x = f64::INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut max_y = f64::NEG_INFINITY;

    for line in doc.descendants().filter(|n| {
        n.tag_name().name() == "line" && n.attribute("class").unwrap_or("").contains("loopLine")
    }) {
        let x1 = parse_num(line, "x1");
        let y1 = parse_num(line, "y1");
        let x2 = parse_num(line, "x2");
        let y2 = parse_num(line, "y2");
        min_x = min_x.min(x1).min(x2);
        min_y = min_y.min(y1).min(y2);
        max_x = max_x.max(x1).max(x2);
        max_y = max_y.max(y1).max(y2);
    }

    assert!(min_x.is_finite(), "missing fragment frame\n{svg}");
    TestBox {
        x: min_x,
        y: min_y,
        width: max_x - min_x,
        height: max_y - min_y,
    }
}

#[test]
fn sequence_fragment_frames_use_lines_not_rects() {
    // Mermaid.js renders fragment frames as 4 line elements (top/right/bottom/left)
    // not as a single rect element with loopLine class
    let input = r#"sequenceDiagram
    Alice->>Bob: Hello
    loop Every minute
        Bob->>Alice: Reply
    end"#;

    let svg = render_sequence(input);

    // Should NOT have rect elements with loopLine class (that's selkie's old approach)
    // Instead should have line elements forming the frame border
    let has_rect_loop = svg.contains("<rect") && {
        // Check if any rect has loopLine class
        svg.split("<rect").skip(1).any(|s| {
            s.split('>')
                .next()
                .map_or(false, |attrs| attrs.contains("loopLine"))
        })
    };
    assert!(
        !has_rect_loop,
        "Fragment frames should NOT use rect elements; should use 4 line elements like mermaid.js"
    );
}

#[test]
fn sequence_message_lines_use_mermaid_classes() {
    // Mermaid.js uses class="messageLine0" for solid and class="messageLine1" for dotted
    // on the actual <line> elements, not "message-line"
    let input = r#"sequenceDiagram
    Alice->>Bob: Solid message
    Bob-->>Alice: Dotted message"#;

    let svg = render_sequence(input);

    // Check that line elements use messageLine0/messageLine1 classes
    let lines: Vec<&str> = svg
        .split("<line")
        .skip(1)
        .filter_map(|s| s.split('>').next())
        .collect();

    let has_message_line0 = lines.iter().any(|l| l.contains("messageLine0"));
    let has_message_line1 = lines.iter().any(|l| l.contains("messageLine1"));

    assert!(
        has_message_line0,
        "Solid message lines should have messageLine0 class on line element"
    );
    assert!(
        has_message_line1,
        "Dotted message lines should have messageLine1 class on line element"
    );
}

#[test]
fn sequence_autonumber_uses_marker_not_circles() {
    // Mermaid.js uses zero-length line with marker-start="url(#sequencenumber)"
    // instead of explicit circle + text elements for sequence numbers
    let input = r#"sequenceDiagram
    autonumber
    Alice->>Bob: First
    Bob-->>Alice: Second"#;

    let svg = render_sequence(input);

    // Should use marker-start for sequence numbers
    assert!(
        svg.contains("marker-start=\"url(#sequencenumber)\""),
        "Sequence numbers should use marker-start on a zero-length line"
    );

    // Should NOT have explicit sequenceNumber-circle elements in the body
    // (only in the marker def is fine)
    let body_circles = svg
        .split("<circle")
        .skip(1)
        .filter(|s| {
            s.split('>')
                .next()
                .map_or(false, |a| a.contains("sequenceNumber-circle"))
        })
        .count();
    assert_eq!(
        body_circles, 0,
        "Should not render explicit sequenceNumber-circle elements in body"
    );
}

#[test]
fn sequence_basic_structure() {
    let input = r#"sequenceDiagram
    participant A as Alice
    participant B as Bob
    A->>B: Hello Bob!
    B-->>A: Hi Alice!"#;

    let svg = render_sequence(input);

    // Should have actor boxes (top and bottom)
    assert!(svg.contains("actor-box"), "Should render actor boxes");

    // Should have lifelines
    assert!(svg.contains("actor-line"), "Should render actor lifelines");

    // Should have message labels
    assert!(svg.contains("Hello Bob!"), "Should render message text");
    assert!(svg.contains("Hi Alice!"), "Should render reply text");
}

#[test]
fn sequence_right_note_extends_actor_gap_without_clipping() {
    let input = r#"sequenceDiagram
    participant Alice
    participant Bob
    Alice->>Bob: Hello
    Note right of Alice: This note needs enough horizontal room
    Bob-->>Alice: Reply"#;

    let svg = render_sequence(input);
    let note = find_note_box_containing(&svg, "This note needs enough horizontal room");
    let bob = find_actor_box_containing(&svg, "Bob");
    let bob_lifeline_x = find_lifeline_x_for_actor(&svg, "Bob");
    let visible_right = svg_visible_right(&svg);
    let actor_margin = 50.0;

    assert!(
        note.right() + actor_margin <= bob.x,
        "right-of note should keep actor-margin gutter before following actor box\n{svg}"
    );
    assert!(
        note.right() + actor_margin <= bob_lifeline_x,
        "right-of note should keep actor-margin gutter before following actor lifeline\n{svg}"
    );

    assert!(
        note.right() <= visible_right,
        "note should fit in viewBox\n{svg}"
    );
}

#[test]
fn sequence_last_right_note_uses_rendered_width_for_viewbox() {
    let input = r#"sequenceDiagram
    participant Alice
    Alice->>Alice: Hello
    Note right of Alice: This rendered note is fixed width even with a much longer first line<br/>short"#;

    let svg = render_sequence(input);
    let note = find_note_box_containing(
        &svg,
        "This rendered note is fixed width even with a much longer first line",
    );

    assert_eq!(
        note.width, 150.0,
        "right-of note render width changed\n{svg}"
    );
    assert!(
        note.right() <= svg_visible_right(&svg),
        "last right-of note should fit in viewBox\n{svg}"
    );
    assert!(
        svg_width(&svg) <= 400.0,
        "viewBox should use rendered note width, not raw multiline text width\n{svg}"
    );
}

#[test]
fn sequence_alt_fragment_has_divider() {
    let input = r#"sequenceDiagram
    Alice->>Bob: Request
    alt Success
        Bob-->>Alice: OK
    else Failure
        Bob-->>Alice: Error
    end"#;

    let svg = render_sequence(input);

    // Should have alt label
    assert!(svg.contains(">alt<"), "Should render alt fragment label");
    // Should have divider line with loopLine class
    assert!(
        svg.contains("loopLine"),
        "Should render fragment elements with loopLine class"
    );
}

#[test]
fn sequence_activation_renders() {
    let input = r#"sequenceDiagram
    Alice->>+Bob: Request
    Bob-->>-Alice: Response"#;

    let svg = render_sequence(input);

    assert!(svg.contains("activation"), "Should render activation box");
}

#[test]
fn sequence_self_message_uses_path() {
    // Mermaid.js renders self-messages as path elements
    let input = r#"sequenceDiagram
    Alice->>Alice: Self message"#;

    let svg = render_sequence(input);

    assert!(
        svg.contains("Self message"),
        "Should render self message text"
    );
    // Self messages use a path element (the loop shape)
    assert!(
        svg.contains("<path"),
        "Self messages should use path elements"
    );
}

#[test]
fn sequence_self_message_label_extends_actor_gap_without_overlap() {
    let input = r#"sequenceDiagram
    participant Alice
    participant Bob
    Alice->>Alice: This self message label needs the full right side reserved
    Bob-->>Alice: Reply"#;

    let svg = render_sequence(input);
    let self_label = find_text_box(
        &svg,
        "This self message label needs the full right side reserved",
    );
    let bob = find_actor_box_containing(&svg, "Bob");
    let bob_lifeline_x = find_lifeline_x_for_actor(&svg, "Bob");

    assert!(
        !self_label.overlaps(&bob, 4.0),
        "self-message label should not overlap following actor box\n{svg}"
    );
    assert!(
        self_label.right() + 4.0 <= bob_lifeline_x,
        "self-message label should not overlap following actor lifeline\n{svg}"
    );
}

#[test]
fn sequence_issue_202_loop_self_message_and_note_do_not_overlap() {
    let input = r#"sequenceDiagram
    participant Alice
    participant Bob
    Alice->>John: Hello John, how are you?
    loop Healthcheck
        John->>John: Fight against hypochondria
    end
    Note right of John: Rational thoughts prevail!
    John-->>Alice: Great!
    John->>Bob: How about you?
    Bob-->>John: Jolly good!"#;

    let svg = render_sequence(input);
    let self_label = find_text_box(&svg, "Fight against hypochondria");
    let note_box = find_note_box_containing(&svg, "Rational thoughts prevail!");
    let note_text = find_text_box(&svg, "Rational thoughts prevail!");
    let loop_label = find_text_box(&svg, "Healthcheck");
    let loop_frame = find_first_fragment_frame(&svg);
    let self_path = find_self_message_path_box(&svg);

    assert!(
        !self_label.overlaps(&note_box, 4.0),
        "self-message label should not overlap note box\n{svg}"
    );
    assert!(
        self_label.bottom() + 4.0 <= note_text.y || note_text.bottom() + 4.0 <= self_label.y,
        "self-message label and note text should have vertical separation\n{svg}"
    );
    assert!(
        loop_label.bottom() + 4.0 <= self_label.y,
        "loop header should sit above self-message label\n{svg}"
    );
    assert!(
        loop_frame.bottom() + 4.0 <= note_box.y || note_box.bottom() + 4.0 <= loop_frame.y,
        "note box should not overlap loop frame\n{svg}"
    );
    assert!(
        !self_path.overlaps(&note_box, 4.0),
        "self-message path should not overlap note box\n{svg}"
    );
}
