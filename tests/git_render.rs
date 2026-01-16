//! Rendering tests for git graph diagrams

use selkie::{parse, render};

#[test]
fn render_git_graph_basic() {
    let input = r#"gitGraph
        commit
        commit
        commit"#;

    let diagram = parse(input).expect("Failed to parse gitGraph");
    let svg = render(&diagram).expect("Failed to render gitGraph");

    assert!(svg.contains("<svg"));
    assert!(svg.contains("commit-bullets"));
    assert!(svg.contains("commit-labels"));
    assert!(svg.contains("text-anchor=\"middle\""));
}

#[test]
fn render_git_graph_merge_commit() {
    let input = r#"gitGraph
        commit id:"A"
        branch feature
        checkout feature
        commit
        checkout main
        merge feature"#;

    let diagram = parse(input).expect("Failed to parse gitGraph");
    let svg = render(&diagram).expect("Failed to render gitGraph");

    assert!(svg.contains("commit-merge"));
    assert!(svg.contains("branch"));
}
