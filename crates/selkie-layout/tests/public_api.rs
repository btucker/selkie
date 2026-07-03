use selkie_layout::{layout, LayoutDirection, LayoutEdge, LayoutGraph, LayoutNode, LayoutOptions};

#[test]
fn lays_out_simple_graph_with_public_api() {
    let mut graph = LayoutGraph::new("example").with_options(LayoutOptions {
        direction: LayoutDirection::TopToBottom,
        node_spacing: 50.0,
        layer_spacing: 50.0,
        ..Default::default()
    });

    graph.add_node(LayoutNode::new("A", 80.0, 40.0));
    graph.add_node(LayoutNode::new("B", 80.0, 40.0));
    graph.add_edge(LayoutEdge::new("A_to_B", "A", "B"));

    let result = layout(graph).expect("layout should succeed");
    let a = result.get_node("A").expect("node A exists");
    let b = result.get_node("B").expect("node B exists");

    let a_pos = a.position().expect("node A has a position");
    let b_pos = b.position().expect("node B has a position");
    assert!(b_pos.y > a_pos.y);

    let edge = result.edges().first().expect("edge exists");
    assert_eq!(edge.id(), "A_to_B");
    assert!(edge.points().len() >= 2);
}

#[test]
fn rejects_missing_edge_endpoint() {
    let mut graph = LayoutGraph::new("invalid");
    graph.add_node(LayoutNode::new("A", 80.0, 40.0));
    graph.add_edge(LayoutEdge::new("missing", "A", "B"));

    let error = layout(graph).expect_err("missing endpoint should fail");
    assert!(error.to_string().contains("B"));
}

#[test]
fn rejects_duplicate_node_ids() {
    let mut graph = LayoutGraph::new("invalid");
    graph.add_node(LayoutNode::new("A", 80.0, 40.0));
    graph.add_node(LayoutNode::new("A", 80.0, 40.0));

    let error = layout(graph).expect_err("duplicate node should fail");
    assert!(error.to_string().contains("Duplicate node id"));
}

#[test]
fn rejects_invalid_node_dimensions() {
    let mut graph = LayoutGraph::new("invalid");
    graph.add_node(LayoutNode::new("A", -1.0, 40.0));

    let error = layout(graph).expect_err("negative width should fail");
    assert!(error.to_string().contains("Invalid layout value"));
}

#[test]
fn rejects_invalid_spacing() {
    let mut graph = LayoutGraph::new("invalid").with_options(LayoutOptions {
        node_spacing: f64::NAN,
        ..Default::default()
    });
    graph.add_node(LayoutNode::new("A", 80.0, 40.0));

    let error = layout(graph).expect_err("NaN spacing should fail");
    assert!(error.to_string().contains("Invalid layout value"));
}

#[test]
fn rejects_missing_parent() {
    let mut graph = LayoutGraph::new("invalid");
    graph.add_node(LayoutNode::new("A", 80.0, 40.0).with_parent("missing"));

    let error = layout(graph).expect_err("missing parent should fail");
    assert!(error.to_string().contains("Invalid parent relationship"));
}

#[test]
fn rejects_parent_cycle() {
    let mut graph = LayoutGraph::new("invalid");
    graph.add_node(LayoutNode::new("A", 80.0, 40.0).with_parent("B"));
    graph.add_node(LayoutNode::new("B", 80.0, 40.0).with_parent("A"));

    let error = layout(graph).expect_err("parent cycle should fail");
    assert!(error.to_string().contains("Invalid parent relationship"));
}

#[test]
fn curated_dagre_api_lays_out_flat_graph() {
    use selkie_layout::dagre::{layout as dagre_layout, DagreConfig, DagreGraph};

    let mut graph = DagreGraph::new();
    graph.set_node("A", 80.0, 40.0);
    graph.set_node("B", 80.0, 40.0);
    graph.set_edge("A", "B");

    dagre_layout(&mut graph, &DagreConfig::default());

    let a = graph.node("A").expect("node A exists");
    let b = graph.node("B").expect("node B exists");

    assert!(a.x().is_some());
    assert!(a.y().is_some());
    assert!(b.x().is_some());
    assert!(b.y().is_some());
}
