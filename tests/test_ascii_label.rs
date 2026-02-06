#[test]
fn show_subgraph_ascii() {
    use selkie::layout::{CharacterSizeEstimator, ToLayoutGraph};
    use selkie::render::ascii::render_flowchart_ascii;

    let input =
        "flowchart TD\n    subgraph sg[TestLabel]\n        A[NodeA]\n        B[NodeB]\n    end";
    let diagram = selkie::parse(input).unwrap();
    let db = match diagram {
        selkie::diagrams::Diagram::Flowchart(db) => db,
        _ => panic!("Expected flowchart"),
    };
    let estimator = CharacterSizeEstimator::default();
    let graph = db.to_layout_graph(&estimator).unwrap();
    let graph = selkie::layout::layout(graph).unwrap();

    let output = render_flowchart_ascii(&db, &graph).unwrap();
    println!("\n=== ASCII OUTPUT ===\n{}\n=== END ===", output);
    assert!(output.contains("TestLabel"), "Should contain label");
}
