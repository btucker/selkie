# selkie-layout

`selkie-layout` is the standalone graph layout engine used by Selkie. It is for callers that want Dagre-style graph layout without Mermaid parsing, SVG rendering, font lookup, or text measurement.

The high-level API takes explicit node dimensions and edges, then returns positioned nodes and routed edge points.

```rust
use selkie_layout::{
    layout, LayoutDirection, LayoutEdge, LayoutGraph, LayoutNode, LayoutOptions,
};

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
```

## API Contract

- Node dimensions must be finite and non-negative. The crate does not measure labels.
- Padding, spacing, edge label sizes, and edge weights are validated before layout.
- `LayoutNode::position()` returns top-left rendering coordinates. The internal algorithm uses center coordinates and converts them before returning the graph.
- Compound nodes can use `children` or `parent_id`; custom-direction groups can set `metadata["is_group"] = "true"` and `metadata["dir"]` to `TB`, `BT`, `LR`, or `RL`.
- `LayoutOptions::padding` describes graph-level padding metadata. Rendering code decides how to use graph bounds and padding.

## Dagre Expert API

`selkie_layout::dagre` exposes a curated expert API for direct Dagre-style layout:

```rust
use selkie_layout::dagre::{layout, DagreConfig, DagreGraph};

let mut graph = DagreGraph::new();
graph.set_node("A", 80.0, 40.0);
graph.set_node("B", 80.0, 40.0);
graph.set_edge("A", "B");

layout(&mut graph, &DagreConfig::default());
```

This module intentionally does not expose every internal Dagre phase or graph implementation detail as public API. If you need Mermaid parsing, diagram adapters, text sizing, or SVG output, use the main `selkie-rs` crate.
