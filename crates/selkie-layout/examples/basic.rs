use selkie_layout::{layout, LayoutDirection, LayoutEdge, LayoutGraph, LayoutNode, LayoutOptions};

fn main() -> selkie_layout::LayoutResult<()> {
    let mut graph = LayoutGraph::new("example").with_options(LayoutOptions {
        direction: LayoutDirection::TopToBottom,
        node_spacing: 50.0,
        layer_spacing: 50.0,
        ..Default::default()
    });

    graph.add_node(LayoutNode::new("A", 80.0, 40.0));
    graph.add_node(LayoutNode::new("B", 80.0, 40.0));
    graph.add_edge(LayoutEdge::new("A_to_B", "A", "B"));

    let result = layout(graph)?;

    for node in result.nodes() {
        println!("{}: {:?}", node.id(), node.position());
    }

    for edge in result.edges() {
        println!("{} routed through {:?}", edge.id(), edge.points());
    }

    Ok(())
}
