//! Tests for sequence diagram rendering to match mermaid.js reference output

use selkie::{parse, render};

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
        .find(|n| n.tag_name().name() == "text" && n.text().unwrap_or("").contains(label))
        .unwrap_or_else(|| panic!("missing text {label}"));
    let x = parse_num(text, "x");
    let y = parse_num(text, "y");
    TestBox {
        x: x - (label.len() as f64 * 4.0),
        y: y - 18.0,
        width: label.len() as f64 * 8.0,
        height: 18.0,
    }
}

fn find_note_box_containing(svg: &str, label: &str) -> TestBox {
    let doc = roxmltree::Document::parse(svg).expect("valid svg");
    let note_text = doc
        .descendants()
        .find(|n| n.tag_name().name() == "text" && n.text().unwrap_or("").contains(label))
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
}
