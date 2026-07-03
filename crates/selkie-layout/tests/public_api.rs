use selkie_layout::{
    layout, LayoutDirection, LayoutEdge, LayoutError, LayoutGraph, LayoutNode, LayoutOptions,
};

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

    assert_eq!(a.size(), (80.0, 40.0));

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
    assert_eq!(
        error,
        LayoutError::MissingEdgeEndpoint {
            edge: "missing".to_string(),
            endpoint: "B".to_string()
        }
    );
}

#[test]
fn rejects_edge_with_empty_sources() {
    let mut graph = LayoutGraph::new("invalid");
    graph.add_node(LayoutNode::new("A", 80.0, 40.0));

    let mut edge = LayoutEdge::new("empty_source", "A", "A");
    edge.sources.clear();
    graph.add_edge(edge);

    let error = layout(graph).expect_err("empty sources should fail");
    assert_eq!(
        error,
        LayoutError::MissingEdgeEndpoint {
            edge: "empty_source".to_string(),
            endpoint: "<source>".to_string()
        }
    );
}

#[test]
fn rejects_edge_with_empty_targets() {
    let mut graph = LayoutGraph::new("invalid");
    graph.add_node(LayoutNode::new("A", 80.0, 40.0));

    let mut edge = LayoutEdge::new("empty_target", "A", "A");
    edge.targets.clear();
    graph.add_edge(edge);

    let error = layout(graph).expect_err("empty targets should fail");
    assert_eq!(
        error,
        LayoutError::MissingEdgeEndpoint {
            edge: "empty_target".to_string(),
            endpoint: "<target>".to_string()
        }
    );
}

#[test]
fn rejects_duplicate_node_ids() {
    let mut graph = LayoutGraph::new("invalid");
    graph.add_node(LayoutNode::new("A", 80.0, 40.0));
    graph.add_node(LayoutNode::new("A", 80.0, 40.0));

    let error = layout(graph).expect_err("duplicate node should fail");
    assert_eq!(error, LayoutError::DuplicateNodeId("A".to_string()));
}

#[test]
fn rejects_invalid_node_dimensions() {
    let mut graph = LayoutGraph::new("invalid");
    graph.add_node(LayoutNode::new("A", -1.0, 40.0));

    let error = layout(graph).expect_err("negative width should fail");
    assert!(matches!(error, LayoutError::InvalidValue(message) if message.contains("node width")));
}

#[test]
fn rejects_invalid_spacing() {
    let mut graph = LayoutGraph::new("invalid").with_options(LayoutOptions {
        node_spacing: f64::NAN,
        ..Default::default()
    });
    graph.add_node(LayoutNode::new("A", 80.0, 40.0));

    let error = layout(graph).expect_err("NaN spacing should fail");
    assert!(
        matches!(error, LayoutError::InvalidValue(message) if message.contains("node_spacing"))
    );
}

#[test]
fn rejects_missing_parent() {
    let mut graph = LayoutGraph::new("invalid");
    graph.add_node(LayoutNode::new("A", 80.0, 40.0).with_parent("missing"));

    let error = layout(graph).expect_err("missing parent should fail");
    assert!(
        matches!(error, LayoutError::InvalidParent(message) if message.contains("missing parent"))
    );
}

#[test]
fn rejects_parent_cycle() {
    let mut graph = LayoutGraph::new("invalid");
    graph.add_node(LayoutNode::new("A", 80.0, 40.0).with_parent("B"));
    graph.add_node(LayoutNode::new("B", 80.0, 40.0).with_parent("A"));

    let error = layout(graph).expect_err("parent cycle should fail");
    assert!(
        matches!(error, LayoutError::InvalidParent(message) if message.contains("parent cycle"))
    );
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
    assert!(b.y().expect("node B has y") > a.y().expect("node A has y"));

    let edge = graph.edge("A", "B").expect("edge A -> B exists");
    assert!(edge.points().len() >= 2);
}

#[test]
fn curated_dagre_api_supports_longest_path_ranker() {
    use selkie_layout::dagre::{layout as dagre_layout, DagreConfig, DagreGraph, Ranker};

    let mut graph = DagreGraph::new();
    graph.set_node("A", 80.0, 40.0);
    graph.set_node("B", 80.0, 40.0);
    graph.set_edge("A", "B");

    dagre_layout(
        &mut graph,
        &DagreConfig {
            ranker: Ranker::LongestPath,
            ..Default::default()
        },
    );

    let a = graph.node("A").expect("node A exists");
    let b = graph.node("B").expect("node B exists");
    assert!(b.y().expect("node B has y") > a.y().expect("node A has y"));
}

#[test]
fn public_dagre_ranker_does_not_expose_tight_tree() {
    let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let target_dir = std::env::var_os("CARGO_TARGET_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| {
            manifest_dir
                .parent()
                .expect("crate has workspace parent")
                .parent()
                .expect("workspace parent has target directory")
                .join("target")
        });
    let deps_dir = target_dir.join("debug").join("deps");
    let layout_rlib = std::fs::read_dir(&deps_dir)
        .expect("target deps directory exists")
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with("libselkie_layout") && name.ends_with(".rlib"))
        })
        .max_by_key(|path| {
            path.metadata()
                .and_then(|metadata| metadata.modified())
                .ok()
        })
        .expect("selkie_layout rlib exists");

    let scratch_dir =
        std::env::temp_dir().join(format!("selkie-layout-public-api-{}", std::process::id()));
    std::fs::create_dir_all(&scratch_dir).expect("scratch directory can be created");
    let source_path = scratch_dir.join("uses_tight_tree.rs");
    std::fs::write(
        &source_path,
        r#"
use selkie_layout::dagre::Ranker;

fn main() {
    let _ = Ranker::TightTree;
}
"#,
    )
    .expect("can write compile probe");

    let output = std::process::Command::new("rustc")
        .arg("--edition=2021")
        .arg(&source_path)
        .arg("-L")
        .arg(format!("dependency={}", deps_dir.display()))
        .arg("--extern")
        .arg(format!("selkie_layout={}", layout_rlib.display()))
        .arg("-o")
        .arg(scratch_dir.join("uses_tight_tree"))
        .output()
        .expect("rustc can run");

    assert!(
        !output.status.success(),
        "Ranker::TightTree should not compile as public API"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("TightTree"), "rustc stderr was: {stderr}");
}
