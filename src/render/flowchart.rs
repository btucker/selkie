//! Flowchart adapter for layout

use std::collections::{HashMap, HashSet};

use crate::diagrams::flowchart::{Direction, FlowTextType, FlowVertexType, FlowchartDb};
use crate::error::Result;
use crate::layout::{
    LayoutDirection, LayoutEdge, LayoutGraph, LayoutNode, LayoutOptions, NodeShape, NodeSizeConfig,
    Padding, SizeEstimator, ToLayoutGraph,
};

impl ToLayoutGraph for FlowchartDb {
    fn to_layout_graph(&self, size_estimator: &dyn SizeEstimator) -> Result<LayoutGraph> {
        let config = NodeSizeConfig::default();
        let mut graph = LayoutGraph::new("flowchart");

        // Set layout options from diagram direction
        // Use dagre defaults (50/50) - compound graph support handles horizontal spread
        graph.options = LayoutOptions {
            direction: self.preferred_direction(),
            node_spacing: 50.0,
            layer_spacing: 50.0,
            padding: Padding::uniform(20.0),
            ..Default::default()
        };

        // Build map of node_id -> subgraph_id for setting parent relationships
        let mut node_to_subgraph: HashMap<&str, &str> = HashMap::new();
        let subgraph_ids: HashSet<&str> =
            self.subgraphs().iter().map(|sg| sg.id.as_str()).collect();
        for subgraph in self.subgraphs() {
            for node_id in &subgraph.nodes {
                node_to_subgraph.insert(node_id.as_str(), subgraph.id.as_str());
            }
        }

        // Add subgraph nodes first (compound parent nodes) in REVERSE
        // definition order, matching mermaid's flowDb.getData() which pushes
        // subgraphs with `for (let i = subGraphs.length - 1; i >= 0; i--)`.
        // These have zero dimensions initially - they're calculated from children by layout
        for subgraph in self.subgraphs().iter().rev() {
            let mut sg_node =
                LayoutNode::new(&subgraph.id, 0.0, 0.0).with_shape(NodeShape::Rectangle);

            // Use subgraph title as label if available
            if !subgraph.title.is_empty() {
                sg_node = sg_node.with_label(&subgraph.title);
            }

            // Mark as a subgraph/group in metadata
            sg_node
                .metadata
                .insert("is_group".to_string(), "true".to_string());

            // Nest this subgraph inside its parent subgraph when it is a member
            // of one. Mermaid lists a nested subgraph as a node in its parent's
            // node list, so the parent cluster encloses it; without this the
            // inner cluster detaches and the outer cluster collapses to only its
            // direct leaf nodes.
            if let Some(&parent_id) = node_to_subgraph.get(subgraph.id.as_str()) {
                sg_node = sg_node.with_parent(parent_id);
            }

            // Store subgraph direction if specified. Like mermaid, the
            // direction only takes effect when the subgraph has no external
            // connections (it is then extracted and laid out recursively).
            if let Some(ref dir) = subgraph.dir {
                sg_node.metadata.insert("dir".to_string(), dir.clone());
            }

            graph.add_node(sg_node);
        }

        // Convert vertices to layout nodes in document insertion order.
        // mermaid adds nodes to dagre in data4Layout.nodes order and dagre's
        // ordering phase is insertion-order sensitive, so we must not sort.
        for id in self.vertex_ids_in_order() {
            if subgraph_ids.contains(id.as_str()) {
                continue;
            }

            let vertex = self.vertices().get(id).unwrap();
            let shape = vertex
                .vertex_type
                .as_ref()
                .map(vertex_type_to_shape)
                .unwrap_or(NodeShape::Rectangle);

            // Markdown labels are measured (and stored) as their rendered
            // visible text: mermaid strips the `**`/`_` emphasis markers before
            // measuring the label bbox, so the node is sized from "bold and em",
            // not the source "**bold** and _em_".
            let stripped_label = match vertex.label_type {
                FlowTextType::Markdown => vertex
                    .text
                    .as_deref()
                    .map(crate::render::text_utils::strip_markdown_markers),
                FlowTextType::Text => None,
            };
            let label = stripped_label.as_deref().or(vertex.text.as_deref());
            let (width, height) = size_estimator.estimate_node_size(label, shape, &config);

            let mut node = LayoutNode::new(id, width, height).with_shape(shape);

            if let Some(label) = label {
                node = node.with_label(label);
            }

            // Set parent for compound graph if this node belongs to a subgraph
            if let Some(&subgraph_id) = node_to_subgraph.get(id.as_str()) {
                node = node.with_parent(subgraph_id);
            }

            // Store original metadata
            node.metadata
                .insert("dom_id".to_string(), vertex.dom_id.clone());
            if let Some(vt) = &vertex.vertex_type {
                node.metadata
                    .insert("vertex_type".to_string(), format!("{:?}", vt));
            }

            graph.add_node(node);
        }

        // Convert edges
        for edge in self.edges() {
            let edge_id = edge
                .id
                .clone()
                .unwrap_or_else(|| format!("{}-{}", edge.start, edge.end));

            let mut layout_edge = LayoutEdge::new(&edge_id, &edge.start, &edge.end);

            if !edge.text.is_empty() {
                // Measure the label before layout (mermaid's insertEdgeLabel):
                // the raw text bbox, with no extra padding. Markdown edge labels
                // are measured/stored as their marker-stripped visible text.
                let edge_label = match edge.label_type {
                    FlowTextType::Markdown => {
                        crate::render::text_utils::strip_markdown_markers(&edge.text)
                    }
                    FlowTextType::Text => edge.text.clone(),
                };
                let (label_w, label_h) =
                    size_estimator.estimate_text_size(&edge_label, config.font_size);
                layout_edge = layout_edge
                    .with_label(&edge_label)
                    .with_label_size(label_w, label_h);
            }

            // Map the edge's dash count (`----`) to the dagre minlen so extra
            // dashes add rank separation and lengthen the edge, mirroring
            // mermaid (flowDb.ts: `minlen: rawEdge.length`).
            if let Some(length) = edge.length {
                layout_edge = layout_edge.with_minlen(length);
            }

            // Store edge type for rendering
            if let Some(et) = &edge.edge_type {
                layout_edge
                    .metadata
                    .insert("edge_type".to_string(), et.clone());
            }
            layout_edge
                .metadata
                .insert("stroke".to_string(), format!("{:?}", edge.stroke));

            graph.add_edge(layout_edge);
        }

        Ok(graph)
    }

    fn preferred_direction(&self) -> LayoutDirection {
        Direction::parse(self.get_direction()).into()
    }
}

/// Convert FlowVertexType to NodeShape
fn vertex_type_to_shape(vt: &FlowVertexType) -> NodeShape {
    match vt {
        FlowVertexType::Square | FlowVertexType::Rect => NodeShape::Rectangle,
        FlowVertexType::Round => NodeShape::RoundedRect,
        FlowVertexType::Circle => NodeShape::Circle,
        FlowVertexType::DoubleCircle => NodeShape::DoubleCircle,
        FlowVertexType::Ellipse => NodeShape::Ellipse,
        FlowVertexType::Stadium => NodeShape::Stadium,
        FlowVertexType::Diamond => NodeShape::Diamond,
        FlowVertexType::Hexagon => NodeShape::Hexagon,
        FlowVertexType::Cylinder => NodeShape::Cylinder,
        FlowVertexType::Subroutine => NodeShape::Subroutine,
        FlowVertexType::Trapezoid => NodeShape::Trapezoid,
        FlowVertexType::InvTrapezoid => NodeShape::InvTrapezoid,
        FlowVertexType::LeanRight => NodeShape::LeanRight,
        FlowVertexType::LeanLeft => NodeShape::LeanLeft,
        FlowVertexType::Odd => NodeShape::Odd,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::CharacterSizeEstimator;

    #[test]
    fn test_simple_flowchart_to_layout() {
        let mut db = FlowchartDb::new();
        db.set_direction("LR");
        db.add_vertex_simple("A", Some("Start"), Some(FlowVertexType::Round));
        db.add_vertex_simple("B", Some("Process"), Some(FlowVertexType::Rect));
        db.add_vertex_simple("C", Some("Decision"), Some(FlowVertexType::Diamond));
        db.add_edge("A", "B", "-->", "-->", None, None);
        db.add_edge("B", "C", "-->", "-->", None, None);

        let estimator = CharacterSizeEstimator::default();
        let graph = db.to_layout_graph(&estimator).unwrap();

        assert_eq!(graph.nodes.len(), 3);
        assert_eq!(graph.edges.len(), 2);
        assert_eq!(graph.options.direction, LayoutDirection::LeftToRight);

        // Check shapes
        let node_a = graph.get_node("A").unwrap();
        assert_eq!(node_a.shape, NodeShape::RoundedRect);

        let node_c = graph.get_node("C").unwrap();
        assert_eq!(node_c.shape, NodeShape::Diamond);
    }

    #[test]
    fn test_markdown_label_measured_with_markers_stripped() {
        // mermaid measures the rendered markdown label ("bold and em"), not the
        // source markers ("**bold** and _em_"). Reference node A renders as
        // 149.953 x 54 (bbox 89.953 + 4*15 padding); measuring the raw markers
        // would inflate the node to ~205px wide.
        use crate::diagrams::flowchart::FlowText;
        use crate::layout::TrebuchetSizeEstimator;

        let mut db = FlowchartDb::new();
        db.set_direction("TD");
        db.add_vertex(
            "A",
            Some(FlowText::markdown("**bold** and _em_")),
            Some(FlowVertexType::Square),
            vec![],
            vec![],
            None,
            None,
        );
        db.add_vertex_simple("B", Some("Next"), Some(FlowVertexType::Square));
        db.add_edge("A", "B", "-->", "-->", None, None);

        let estimator = TrebuchetSizeEstimator::new();
        let graph = db.to_layout_graph(&estimator).unwrap();

        let node_a = graph.get_node("A").unwrap();
        assert!(
            (node_a.width - 149.953125).abs() <= 2.0,
            "markdown node A should be sized from stripped text (~149.95), got {}",
            node_a.width
        );
        // The stored label is the visible text, not the source markers.
        assert_eq!(node_a.label.as_deref(), Some("bold and em"));
    }

    #[test]
    fn test_layout_graph_preserves_document_order() {
        // mermaid feeds nodes to dagre in document order (data4Layout.nodes
        // insertion order); dagre's ordering phase is insertion-order
        // sensitive, so the adapter must not alphabetize vertices.
        let mut db = FlowchartDb::new();
        db.add_vertex_simple("B", None, None);
        db.add_vertex_simple("A", None, None);
        db.add_vertex_simple("C", None, None);
        db.add_edge("B", "A", "-->", "-->", None, None);
        db.add_edge("B", "C", "-->", "-->", None, None);

        let estimator = CharacterSizeEstimator::default();
        let graph = db.to_layout_graph(&estimator).unwrap();

        let ids: Vec<&str> = graph.nodes.iter().map(|n| n.id.as_str()).collect();
        assert_eq!(ids, vec!["B", "A", "C"]);
    }

    #[test]
    fn test_layout_graph_subgraphs_in_reverse_definition_order() {
        // mermaid's flowDb.getData() pushes subgraph nodes in REVERSE
        // definition order (for i = subGraphs.length - 1; i >= 0; i--),
        // then vertices in document order.
        let mut db = FlowchartDb::new();
        db.add_vertex_simple("A", None, None);
        db.add_vertex_simple("B", None, None);
        db.add_subgraph_with_nodes("First", "First", vec!["A".to_string()]);
        db.add_subgraph_with_nodes("Second", "Second", vec!["B".to_string()]);

        let estimator = CharacterSizeEstimator::default();
        let graph = db.to_layout_graph(&estimator).unwrap();

        let ids: Vec<&str> = graph.nodes.iter().map(|n| n.id.as_str()).collect();
        assert_eq!(ids, vec!["Second", "First", "A", "B"]);
    }

    #[test]
    fn test_edge_labels_measured_with_estimator() {
        // Edge labels must be measured pre-layout via the size estimator
        // (mirroring mermaid's insertEdgeLabel), not via char-count heuristics.
        // Reference bbox for 'Link text' at 16px trebuchet: 63.734375 x 24.
        use crate::layout::TrebuchetSizeEstimator;

        let mut db = FlowchartDb::new();
        db.set_direction("TB");
        db.add_vertex_simple("A", Some("Start"), Some(FlowVertexType::Rect));
        db.add_vertex_simple("B", Some("End"), Some(FlowVertexType::Rect));
        db.add_edge("A", "B", "-->", "-->", Some("Link text"), None);

        let estimator = TrebuchetSizeEstimator::new();
        let graph = db.to_layout_graph(&estimator).unwrap();

        let edge = &graph.edges[0];
        assert_eq!(edge.label.as_deref(), Some("Link text"));
        assert!(
            (edge.label_width - 63.734375).abs() <= 2.0,
            "edge label width should be ~63.73, got {}",
            edge.label_width
        );
        assert!(
            (edge.label_height - 24.0).abs() <= 0.001,
            "edge label height should be 24, got {}",
            edge.label_height
        );
    }

    #[test]
    /// @spec FLOW-1.3: When a flowchart contains subgraph member nodes, the application shall preserve parent-child relationships in the layout graph.
    fn test_compound_graph_structure() {
        use crate::diagrams::flowchart::parse;
        use crate::layout;
        use crate::layout::dagre::graph::{DagreGraph, NodeLabel};

        // Parse a flowchart with subgraphs
        let input = r#"flowchart TB
    subgraph Frontend[Frontend Layer]
        A[React App]
        B[Vue App]
    end
    subgraph API[API Layer]
        C[REST API]
        D[GraphQL]
    end
    A --> C
    B --> D
"#;

        let db = parse(input).unwrap();
        eprintln!(
            "Subgraphs: {:?}",
            db.subgraphs().iter().map(|s| &s.id).collect::<Vec<_>>()
        );

        let estimator = CharacterSizeEstimator::default();
        let graph = db.to_layout_graph(&estimator).unwrap();

        eprintln!("\nLayout nodes:");
        for node in &graph.nodes {
            eprintln!(
                "  {} - parent: {:?}, size: {}x{}",
                node.id, node.parent_id, node.width, node.height
            );
        }

        // Check that parent_id is set on child nodes
        let node_a = graph.get_node("A").unwrap();
        assert!(node_a.parent_id.is_some(), "Node A should have a parent_id");
        eprintln!("Node A parent: {:?}", node_a.parent_id);

        // Check subgraph nodes exist
        let frontend = graph.get_node("Frontend").unwrap();
        assert!(
            frontend.parent_id.is_none(),
            "Frontend subgraph should have no parent"
        );
        eprintln!("Frontend size: {}x{}", frontend.width, frontend.height);

        // Create DagreGraph manually to test is_compound
        let mut dg = DagreGraph::new();
        dg.set_node("sg", NodeLabel::default());
        dg.set_node("a", NodeLabel::default());
        dg.set_parent("a", "sg");
        eprintln!("\nManual DagreGraph is_compound: {}", dg.is_compound());
        eprintln!("Children of sg: {:?}", dg.children("sg"));

        // Run layout
        let laid_out = layout::layout(graph).unwrap();

        eprintln!("\nAfter layout:");
        for node in &laid_out.nodes {
            eprintln!(
                "  {} - pos: ({:?}, {:?}), size: {}x{}",
                node.id, node.x, node.y, node.width, node.height
            );
        }
        eprintln!("Graph bounds: {:?}x{:?}", laid_out.width, laid_out.height);
    }

    #[test]
    /// @spec FLOW-2.1: When a flowchart edge has a Mermaid label, the application shall preserve that label in the layout graph edge model.
    fn test_edge_labels() {
        let mut db = FlowchartDb::new();
        db.add_vertex_simple("A", Some("Start"), None);
        db.add_vertex_simple("B", Some("End"), None);
        db.add_edge("A", "B", "-->", "-->", Some("Yes"), None);

        let estimator = CharacterSizeEstimator::default();
        let graph = db.to_layout_graph(&estimator).unwrap();

        let edge = &graph.edges[0];
        assert_eq!(edge.label.as_deref(), Some("Yes"));
    }

    #[test]
    fn test_flowchart_edge_points_after_layout() {
        use crate::layout;

        let mut db = FlowchartDb::new();
        db.set_direction("LR");
        db.add_vertex_simple("A", Some("Start"), Some(FlowVertexType::Round));
        db.add_vertex_simple("B", Some("End"), Some(FlowVertexType::Rect));
        db.add_edge("A", "B", "-->", "-->", None, None);

        let estimator = CharacterSizeEstimator::default();
        let graph = db.to_layout_graph(&estimator).unwrap();

        eprintln!("Before layout:");
        eprintln!(
            "  Nodes: {:?}",
            graph.nodes.iter().map(|n| &n.id).collect::<Vec<_>>()
        );
        eprintln!(
            "  Edges: {:?}",
            graph
                .edges
                .iter()
                .map(|e| (&e.id, &e.sources, &e.targets))
                .collect::<Vec<_>>()
        );

        // Run layout
        let graph = layout::layout(graph).unwrap();

        eprintln!("\nAfter layout:");
        for edge in &graph.edges {
            eprintln!(
                "  Edge {} ({:?} -> {:?}):",
                edge.id, edge.sources, edge.targets
            );
            eprintln!("    bend_points: {:?}", edge.bend_points);
            eprintln!("    label_position: {:?}", edge.label_position);
        }

        // Check that edges have bend points
        let edge = &graph.edges[0];
        assert!(
            !edge.bend_points.is_empty(),
            "Flowchart edge should have bend points after layout, got: {:?}",
            edge
        );
    }

    #[test]
    fn test_edge_extra_length_increases_edge_span() {
        use crate::layout;

        // Mermaid maps an edge's dash count (`length`) to the dagre edge
        // minlen (flowDb.ts: `minlen: rawEdge.length`), so extra dashes add
        // rank separation and lengthen the edge. A short `A --> B` edge
        // (length 1) must produce a smaller horizontal span than a long
        // `A ----> B` edge (length 3) in an LR layout.
        let span = |arrow: &str| -> f64 {
            let mut db = FlowchartDb::new();
            db.set_direction("LR");
            db.add_vertex_simple("A", Some("Start"), Some(FlowVertexType::Rect));
            db.add_vertex_simple("B", Some("End"), Some(FlowVertexType::Rect));
            db.add_edge("A", "B", arrow, arrow, None, None);

            let estimator = CharacterSizeEstimator::default();
            let graph = db.to_layout_graph(&estimator).unwrap();
            let graph = layout::layout(graph).unwrap();

            let a = graph.get_node("A").unwrap();
            let b = graph.get_node("B").unwrap();
            b.x.unwrap() - a.x.unwrap()
        };

        let short = span("-->");
        let long = span("---->");
        assert!(
            long > short + 50.0,
            "A ----> B (length 3) should span noticeably wider than A --> B (length 1). short={short:.1}, long={long:.1}"
        );
    }

    #[test]
    fn test_decision_branch_ordering_from_parsed_flowchart() {
        use crate::diagrams::flowchart::parse;
        use crate::layout;

        // Parse the flowchart with decision branches
        let input = "flowchart LR\n    B{Decision} -->|Yes| C[Action 1]\n    B -->|No| D[Action 2]";
        let db = parse(input).unwrap();

        // Convert to layout graph
        let estimator = CharacterSizeEstimator::default();
        let graph = db.to_layout_graph(&estimator).unwrap();

        // Run layout
        let graph = layout::layout(graph).unwrap();

        // Get positions of C and D
        let c = graph.get_node("C").unwrap();
        let d = graph.get_node("D").unwrap();

        // In LR layout, C (first branch, "Yes") should be ABOVE D (second branch, "No")
        // That means C should have LOWER y coordinate
        assert!(
            c.y.unwrap() < d.y.unwrap(),
            "C (Action 1, first branch) should be above D (Action 2, second branch) in LR layout. C.y={:?}, D.y={:?}",
            c.y, d.y
        );
    }

    #[test]
    /// @spec FLOW-3.1: When a flowchart edge is rendered to SVG, the application shall emit an SVG path for the edge route.
    fn test_flowchart_svg_has_edge_path() {
        use crate::diagrams::Diagram;
        use crate::render;

        let mut db = FlowchartDb::new();
        db.set_direction("LR");
        db.add_vertex_simple("A", Some("Start"), Some(FlowVertexType::Round));
        db.add_vertex_simple("B", Some("End"), Some(FlowVertexType::Rect));
        db.add_edge("A", "B", "-->", "-->", None, None);

        // Render to SVG
        let diagram = Diagram::Flowchart(db);
        let svg = render::render(&diagram).unwrap();

        eprintln!("Generated SVG:\n{}", svg);

        // Edge should have a path element
        assert!(
            svg.contains("<path"),
            "SVG should contain path element for edge. SVG:\n{}",
            svg
        );

        // Check for mermaid's edge classes
        assert!(
            svg.contains("flowchart-link"),
            "SVG should contain flowchart-link class. SVG:\n{}",
            svg
        );

        // Path should have actual coordinates (M command followed by numbers)
        assert!(
            svg.contains("M "),
            "Path should have M (move) command. SVG:\n{}",
            svg
        );
    }

    #[test]
    /// @spec FLOW-1.2: When a subgraph declares its own direction, the application shall preserve that direction without changing the parent flowchart direction.
    fn test_subgraph_with_different_direction_end_to_end() {
        use crate::diagrams::flowchart::parse;
        use crate::layout;

        // Parse a flowchart with TB direction but a subgraph with LR direction.
        // The subgraph has no external connections, so (like mermaid) it is
        // extracted and laid out with its declared direction.
        let input = r#"flowchart TB
    subgraph sub1[LR Subgraph]
        direction LR
        A[Node A] --> B[Node B]
    end
    C[External] --> D[Other]"#;

        let db = parse(input).unwrap();

        // Verify parsing captured the direction
        let subgraphs = db.subgraphs();
        assert_eq!(subgraphs.len(), 1);
        assert_eq!(
            subgraphs[0].dir,
            Some("LR".to_string()),
            "Subgraph should have LR direction"
        );

        // Convert to layout graph
        let estimator = CharacterSizeEstimator::default();
        let graph = db.to_layout_graph(&estimator).unwrap();

        // Verify the direction is in metadata
        let sub_node = graph.get_node("sub1").unwrap();
        assert_eq!(
            sub_node.metadata.get("dir"),
            Some(&"LR".to_string()),
            "Subgraph node should have dir in metadata"
        );

        // Run layout
        let graph = layout::layout(graph).unwrap();

        // Get positions
        let a = graph.get_node("A").unwrap();
        let b = graph.get_node("B").unwrap();

        eprintln!("A: x={:?}, y={:?}", a.x, a.y);
        eprintln!("B: x={:?}, y={:?}", b.x, b.y);

        // A and B are in the LR subgraph, so they should be side-by-side
        // (B to the right of A, similar y)
        let a_center_y = a.y.unwrap() + a.height / 2.0;
        let b_center_y = b.y.unwrap() + b.height / 2.0;

        assert!(
            (a_center_y - b_center_y).abs() < 15.0,
            "A and B in LR subgraph should have similar y. A.y={:.1}, B.y={:.1}",
            a_center_y,
            b_center_y
        );

        assert!(
            b.x.unwrap() > a.x.unwrap(),
            "B should be to the right of A in LR subgraph. A.x={:.1}, B.x={:.1}",
            a.x.unwrap(),
            b.x.unwrap()
        );
    }
}
