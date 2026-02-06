#[test]
fn show_edge_case_small_subgraph() {
    use selkie::layout::{CharacterSizeEstimator, ToLayoutGraph};
    use selkie::render::ascii::render_flowchart_ascii;

    // Create a very small subgraph to test edge positioning
    let input = "flowchart TD\n    subgraph sg[A]\n        X[X]\n    end";
    let diagram = selkie::parse(input).unwrap();
    let db = match diagram {
        selkie::diagrams::Diagram::Flowchart(db) => db,
        _ => panic!("Expected flowchart"),
    };
    let estimator = CharacterSizeEstimator::default();
    let graph = db.to_layout_graph(&estimator).unwrap();
    let graph = selkie::layout::layout(graph).unwrap();

    let output = render_flowchart_ascii(&db, &graph).unwrap();
    println!("\n=== SMALL SUBGRAPH ASCII ===\n{}\n=== END ===", output);

    // Check for proper spacing
    let lines: Vec<&str> = output.lines().collect();
    if let Some(first_line) = lines.first() {
        println!(
            "First line chars: {:?}",
            first_line.chars().collect::<Vec<_>>()
        );
        println!("First line: '{}'", first_line);
        // The pattern should be: ┏ + dashes + space + label + space + dashes + ┓
        assert!(
            !first_line.starts_with("┏A"),
            "Label should have leading space"
        );
    }
    assert!(output.contains("A"), "Should contain label");
}
