//! Tests for sequence diagram rendering to match mermaid.js reference output

use selkie::{parse, render};

fn render_sequence(input: &str) -> String {
    let diagram = parse(input).expect("Failed to parse");
    render(&diagram).expect("Failed to render")
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
