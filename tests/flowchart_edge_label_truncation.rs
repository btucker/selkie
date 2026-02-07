/// Regression test: flowchart edge labels near diamond shapes must not be truncated.
///
/// Bug: the edge label "Invalid" on `Auth -->|Invalid| Reject` was rendered as
/// "Inv" because the diamond's bounding box (marked as occupied) overlapped the
/// label's ideal placement. The `find_clear_label_position` search in edges.rs
/// should relocate the label to an unoccupied row/column so it appears in full.
#[test]
fn flowchart_edge_label_not_truncated_near_diamond() {
    use selkie::layout::{CharacterSizeEstimator, ToLayoutGraph};
    use selkie::render::ascii::render_flowchart_ascii;

    // Minimal reproduction: a diamond with two labeled outgoing edges.
    // The "Invalid" label is the one that was previously truncated to "Inv".
    let input = r#"flowchart TB
    Auth{Authentication}
    Auth -->|Valid| Next[Next Step]
    Auth -->|Invalid| Reject[Reject Request]"#;

    let diagram = selkie::parse(input).unwrap();
    let db = match diagram {
        selkie::diagrams::Diagram::Flowchart(db) => db,
        _ => panic!("Expected flowchart"),
    };
    let estimator = CharacterSizeEstimator::default();
    let graph = db.to_layout_graph(&estimator).unwrap();
    let graph = selkie::layout::layout(graph).unwrap();

    let output = render_flowchart_ascii(&db, &graph).unwrap();
    println!(
        "\n=== FLOWCHART EDGE LABEL OUTPUT ===\n{}\n=== END ===",
        output
    );

    // The full word "Invalid" (7 chars) must appear in the output.
    // Previously it was truncated to "Inv" (3 chars).
    assert!(
        output.contains("Invalid"),
        "Edge label 'Invalid' should not be truncated.\nOutput:\n{}",
        output
    );
    // Also verify "Valid" appears
    assert!(
        output.contains("Valid"),
        "Edge label 'Valid' should appear.\nOutput:\n{}",
        output
    );
}
