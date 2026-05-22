//! Tests for sequence diagram rendering to match mermaid.js reference output

use selkie::render::svg::sequence_geometry::SequenceGeometry;
use selkie::{parse, render};

fn render_sequence(input: &str) -> String {
    let diagram = parse(input).expect("Failed to parse");
    render(&diagram).expect("Failed to render")
}

fn geometry(svg: &str) -> SequenceGeometry {
    SequenceGeometry::parse(svg).expect("valid sequence svg geometry")
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
    let geometry = geometry(&svg);
    let note = geometry
        .note_box_containing("This note needs enough horizontal room")
        .expect("missing note rect for This note needs enough horizontal room");
    let bob = geometry
        .actor_box_containing("Bob")
        .expect("missing actor rect for Bob");
    let bob_lifeline_x = geometry
        .lifeline_x_for_actor("Bob")
        .expect("missing lifeline for Bob");
    let visible_right = geometry
        .svg_visible_right()
        .expect("missing SVG visible right");
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
    let geometry = geometry(&svg);
    let note = geometry
        .note_box_containing("This rendered note is fixed width even with a much longer first line")
        .expect(
            "missing note rect for This rendered note is fixed width even with a much longer first line",
        );
    let note_text = geometry
        .text_box_containing("This rendered note is fixed width even with a much longer first line")
        .expect("missing note text");

    assert!(
        note.contains_with_tolerance(&note_text, 4.0),
        "right-of note should expand to contain the rendered first line\n{svg}"
    );
    assert!(
        note.right()
            <= geometry
                .svg_visible_right()
                .expect("missing SVG visible right"),
        "last right-of note should fit in viewBox\n{svg}"
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
fn sequence_empty_alt_section_stays_inside_fragment_frame() {
    let input = r#"sequenceDiagram
    participant A
    participant B
    alt Success
        A->>B: OK
    else Failure
    end
    A->>B: After"#;

    let svg = render_sequence(input);
    let geometry = geometry(&svg);
    let frame = geometry
        .first_fragment_frame_group()
        .expect("missing fragment frame group");
    let else_label = geometry
        .text_box_containing("Failure")
        .expect("missing text Failure");
    let next_label = geometry
        .text_box_containing("After")
        .expect("missing text After");

    assert!(
        else_label.bottom() + 4.0 <= frame.bottom(),
        "empty else label should stay inside fragment frame\n{svg}"
    );
    assert!(
        else_label.bottom() + 4.0 <= next_label.y,
        "following message should not overlap empty else label\n{svg}"
    );
}

#[test]
fn sequence_fragment_headers_do_not_overlap_first_message_labels() {
    let input = r#"sequenceDiagram
    loop Daily query
        Alice->>Bob: Hello Bob, how are you?
        alt is sick
            Bob->>Alice: Not so good :(
        else is well
            Bob->>Alice: Feeling fresh like a daisy
        end

        opt Extra response
            Bob->>Alice: Thanks for asking
        end
    end"#;

    let svg = render_sequence(input);
    let geometry = geometry(&svg);
    let daily = geometry
        .text_box_containing("Daily query")
        .expect("missing text Daily query");
    let hello = geometry
        .text_box_containing("Hello Bob, how are you?")
        .expect("missing text Hello Bob, how are you?");
    let sick = geometry
        .text_box_containing("is sick")
        .expect("missing text is sick");
    let not_good = geometry
        .text_box_containing("Not so good :(")
        .expect("missing text Not so good :(");
    let well = geometry
        .text_box_containing("is well")
        .expect("missing text is well");
    let fresh = geometry
        .text_box_containing("Feeling fresh like a daisy")
        .expect("missing text Feeling fresh like a daisy");
    let extra = geometry
        .text_box_containing("Extra response")
        .expect("missing text Extra response");
    let thanks = geometry
        .text_box_containing("Thanks for asking")
        .expect("missing text Thanks for asking");

    assert!(
        daily.bottom() + 4.0 <= hello.y,
        "loop header should not overlap first message label\n{svg}"
    );
    assert!(
        sick.bottom() + 4.0 <= not_good.y,
        "alt header should not overlap first branch message label\n{svg}"
    );
    assert!(
        well.bottom() + 4.0 <= fresh.y,
        "else header should not overlap first branch message label\n{svg}"
    );
    assert!(
        extra.bottom() + 4.0 <= thanks.y,
        "opt header should not overlap first message label\n{svg}"
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
fn sequence_self_message_uses_curved_path() {
    let input = r#"sequenceDiagram
    Alice->>Alice: Fight against hypochondria"#;

    let svg = render_sequence(input);
    let geometry = geometry(&svg);
    let path = geometry
        .self_message_paths()
        .first()
        .expect("missing self-message path");

    assert!(
        path.contains('C'),
        "self-message path should use a rounded cubic curve like Mermaid, got {path}\n{svg}"
    );
}

#[test]
fn sequence_self_message_label_sits_above_edge() {
    let input = r#"sequenceDiagram
    Alice->>Alice: Fight against hypochondria"#;

    let svg = render_sequence(input);
    let geometry = geometry(&svg);
    let path = geometry
        .self_message_path_box()
        .expect("missing self-message path");
    let label = geometry
        .text_box_containing("Fight against hypochondria")
        .expect("missing self-message label");

    assert!(
        label.bottom() + 4.0 <= path.y,
        "self-message label should sit above the rounded self-edge\n{svg}"
    );
    assert!(
        label.x < path.right(),
        "self-message label should not be placed to the right of the self-edge\n{svg}"
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
    let geometry = geometry(&svg);
    let self_label = geometry
        .text_box_containing("This self message label needs the full right side reserved")
        .expect("missing text This self message label needs the full right side reserved");
    let bob = geometry
        .actor_box_containing("Bob")
        .expect("missing actor rect for Bob");
    let bob_lifeline_x = geometry
        .lifeline_x_for_actor("Bob")
        .expect("missing lifeline for Bob");

    assert!(
        !self_label.intersects_with_tolerance(&bob, 4.0),
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
    let geometry = geometry(&svg);
    let self_label = geometry
        .text_box_containing("Fight against hypochondria")
        .expect("missing text Fight against hypochondria");
    let note_box = geometry
        .note_box_containing("Rational thoughts prevail!")
        .expect("missing note rect for Rational thoughts prevail!");
    let note_text = geometry
        .text_box_containing("Rational thoughts prevail!")
        .expect("missing text Rational thoughts prevail!");
    let loop_label = geometry
        .text_box_containing("Healthcheck")
        .expect("missing text Healthcheck");
    let fragment_kind_label = geometry
        .text_box_containing("loop")
        .expect("missing fragment kind label loop");
    let loop_frame = geometry
        .first_fragment_frame()
        .expect("missing fragment frame");
    let self_path = geometry
        .self_message_path_box()
        .expect("missing self-message path");

    assert!(
        !self_label.intersects_with_tolerance(&note_box, 4.0),
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
        !fragment_kind_label.intersects_with_tolerance(&loop_label, 4.0),
        "fragment kind label should not overlap loop condition\n{svg}"
    );
    assert!(
        note_box.contains_with_tolerance(&note_text, 4.0),
        "note text should stay inside note box\n{svg}"
    );
    assert!(
        loop_frame.bottom() + 4.0 <= note_box.y || note_box.bottom() + 4.0 <= loop_frame.y,
        "note box should not overlap loop frame\n{svg}"
    );
    assert!(
        !self_path.intersects_with_tolerance(&note_box, 4.0),
        "self-message path should not overlap note box\n{svg}"
    );
}

#[test]
fn sequence_arrowheads_use_user_space_marker_units() {
    let input = r#"sequenceDiagram
    Alice->>Bob: Solid message
    Bob-->>Alice: Dotted message"#;

    let svg = render_sequence(input);
    let geometry = geometry(&svg);
    let marker = geometry
        .marker("arrow-filled")
        .expect("missing arrow-filled marker");

    assert_eq!(
        marker.marker_units.as_deref(),
        Some("userSpaceOnUse"),
        "sequence arrowhead marker should not scale with stroke width\n{svg}"
    );
    assert!(
        marker.marker_width <= 12.0 && marker.marker_height <= 12.0,
        "sequence arrowhead marker dimensions should stay Mermaid-sized\n{svg}"
    );
}

#[test]
fn sequence_fragment_frame_contains_wide_single_actor_note() {
    let input = r#"sequenceDiagram
    participant T1 as Test Thread 1
    participant T2 as Test Thread 2
    participant Lock as Channel File Lock
    par Concurrent Lock Contention
        T2->>Lock: Channel::send()
        Note over T2: Another test tries to write
        Lock-->>T2: Lock acquired by T1
    end"#;

    let svg = render_sequence(input);
    let geometry = geometry(&svg);
    let note = geometry
        .note_box_containing("Another test tries to write")
        .expect("missing note rect");
    let frame = geometry
        .first_fragment_frame()
        .expect("missing fragment frame");

    assert!(
        frame.contains_with_tolerance(&note, 4.0),
        "fragment frame should include a wide single-actor note\n{svg}"
    );
    for border in geometry.fragment_borders() {
        assert!(
            !note.intersects_with_tolerance(border, 4.0),
            "note should not overlap fragment border\n{svg}"
        );
    }
}
