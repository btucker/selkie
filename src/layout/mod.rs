//! Layout engine for positioning diagram elements
//!
//! This module provides a graph layout engine using the dagre algorithm
//! (a port of dagre.js) for visual parity with mermaid.js.

mod adapter;
mod graph;
pub(crate) mod size;
mod types;

pub mod dagre;

pub use adapter::{NodeSizeConfig, SizeEstimator, ToLayoutGraph};
pub use graph::LayoutGraph;
pub use size::{
    create_size_estimator, CharacterSizeEstimator, FontdueSizeEstimator, TrebuchetSizeEstimator,
};
pub use types::{
    geometric_midpoint, LayoutDirection, LayoutEdge, LayoutNode, LayoutOptions, LayoutRanker,
    NodeShape, Padding, Point,
};

use crate::error::Result;
use dagre::graph::{DagreGraph, EdgeLabel, NodeLabel};
use dagre::{DagreConfig, RankDir, Ranker};
use std::collections::{HashMap, HashSet};

/// Padding between cluster contents and the cluster border, matching the
/// flowchart SVG renderer's subgraph padding.
const CLUSTER_PADDING: f64 = 20.0;
/// Vertical space reserved for the cluster title, matching the renderer.
const CLUSTER_TITLE_HEIGHT: f64 = 25.0;
/// Extra rank separation applied to extracted cluster graphs, mirroring
/// mermaid's dagre wrapper (`ranksep: ranksep + 25` in
/// rendering-util/layout-algorithms/dagre/index.js).
const CLUSTER_EXTRA_RANKSEP: f64 = 25.0;
/// Maximum cluster extraction recursion depth (mermaid-graphlib.js extractor
/// bails out at depth > 10).
const MAX_EXTRACTION_DEPTH: usize = 10;

/// An extracted cluster: a subgraph without external connections that was laid
/// out recursively as its own graph (mermaid's `extractor` in
/// mermaid-graphlib.js). The cluster is represented in the outer graph as a
/// single fixed-size node; its contents are translated into place afterwards.
#[derive(Debug)]
struct ClusterExtraction {
    /// The cluster (subgraph) node id in the outer graph
    cluster_id: String,
    /// Ids of all descendant nodes that were moved into the sub-graph
    descendant_ids: HashSet<String>,
    /// Indices into the outer graph's edge list of the internal edges
    internal_edge_indices: Vec<usize>,
    /// The recursively laid-out sub-graph
    sub: LayoutGraph,
}

/// Perform layout on a graph using dagre algorithm
pub fn layout(graph: LayoutGraph) -> Result<LayoutGraph> {
    layout_with_depth(graph, 0)
}

fn layout_with_depth(mut graph: LayoutGraph, depth: usize) -> Result<LayoutGraph> {
    // Phase 1: Extract clusters WITHOUT external connections into their own
    // graphs and lay them out recursively (mermaid's adjustClustersAndEdges +
    // extractor). Each extracted cluster becomes a fixed-size node here.
    let extractions = extract_isolated_clusters(&graph, depth)?;

    let mut skip_nodes: HashSet<String> = HashSet::new();
    let mut skip_edges: HashSet<usize> = HashSet::new();
    for extraction in &extractions {
        skip_nodes.extend(extraction.descendant_ids.iter().cloned());
        skip_edges.extend(extraction.internal_edge_indices.iter().copied());

        // Size the collapsed cluster node from the sub-layout bounds plus the
        // cluster decoration (padding + title), mirroring mermaid's
        // updateNodeBounds after recursiveRender.
        if let Some(node) = graph.get_node_mut(&extraction.cluster_id) {
            node.width = extraction.sub.width.unwrap_or(0.0) + 2.0 * CLUSTER_PADDING;
            node.height =
                extraction.sub.height.unwrap_or(0.0) + 2.0 * CLUSTER_PADDING + CLUSTER_TITLE_HEIGHT;
        }
    }

    // Phase 2: Run dagre layout on the outer graph (extracted cluster
    // contents are hidden from it).
    let mut dagre_graph = to_dagre_graph_filtered(&graph, &skip_nodes, &skip_edges);
    let config = to_dagre_config(&graph.options);
    dagre::layout(&mut dagre_graph, &config);

    // Phase 3: Copy results back to LayoutGraph
    apply_dagre_results(&mut graph, &dagre_graph);

    // Phase 4: Translate extracted cluster contents (node positions, edge
    // bend points and edge label positions) by the cluster's final origin.
    for extraction in &extractions {
        place_extracted_cluster(&mut graph, extraction);
    }

    // Compute graph bounds
    graph.compute_bounds();

    Ok(graph)
}

/// Mermaid's default direction for extracted clusters is the FLIP of the
/// parent direction: TB -> LR, anything else -> TB
/// (mermaid-graphlib.js: `dir = graphSettings.rankdir === 'TB' ? 'LR' : 'TB'`).
fn flip_direction(dir: LayoutDirection) -> LayoutDirection {
    match dir {
        LayoutDirection::TopToBottom => LayoutDirection::LeftToRight,
        _ => LayoutDirection::TopToBottom,
    }
}

/// Find clusters without external connections and lay each out recursively as
/// its own graph. Ports mermaid's externalConnections marking
/// (mermaid-graphlib.js adjustClustersAndEdges) and cluster extraction
/// (mermaid-graphlib.js extractor).
fn extract_isolated_clusters(graph: &LayoutGraph, depth: usize) -> Result<Vec<ClusterExtraction>> {
    if depth > MAX_EXTRACTION_DEPTH {
        return Ok(Vec::new());
    }

    // Map parent id -> direct child node ids
    let mut children_of: HashMap<&str, Vec<&str>> = HashMap::new();
    for node in &graph.nodes {
        if let Some(parent_id) = &node.parent_id {
            children_of
                .entry(parent_id.as_str())
                .or_default()
                .push(node.id.as_str());
        }
    }

    // Clusters are group nodes that have children
    let cluster_ids: Vec<&str> = graph
        .nodes
        .iter()
        .filter(|n| {
            n.metadata.get("is_group") == Some(&"true".to_string())
                && children_of.contains_key(n.id.as_str())
        })
        .map(|n| n.id.as_str())
        .collect();

    if cluster_ids.is_empty() {
        return Ok(Vec::new());
    }

    // Transitive descendants of each cluster
    fn collect_descendants(
        id: &str,
        children_of: &HashMap<&str, Vec<&str>>,
        out: &mut HashSet<String>,
    ) {
        if let Some(children) = children_of.get(id) {
            for child in children {
                if out.insert((*child).to_string()) {
                    collect_descendants(child, children_of, out);
                }
            }
        }
    }

    let mut descendants: HashMap<&str, HashSet<String>> = HashMap::new();
    for cluster_id in &cluster_ids {
        let mut set = HashSet::new();
        collect_descendants(cluster_id, &children_of, &mut set);
        descendants.insert(cluster_id, set);
    }

    // Mark external connections: an edge with exactly one endpoint among the
    // cluster's DESCENDANTS (mermaid-graphlib.js adjustClustersAndEdges sets
    // externalConnections via `d1 XOR d2` over extractDescendants; the cluster
    // id is not among its own descendants, so an edge whose endpoint IS the
    // cluster id does not mark it external).
    let mut external: HashSet<&str> = HashSet::new();
    for cluster_id in &cluster_ids {
        let desc = &descendants[cluster_id];
        for edge in &graph.edges {
            let d1 = edge.sources.iter().any(|s| desc.contains(s));
            let d2 = edge.targets.iter().any(|t| desc.contains(t));
            if d1 != d2 {
                external.insert(cluster_id);
                break;
            }
        }
    }

    let mut extractions = Vec::new();
    let mut already_extracted: HashSet<String> = HashSet::new();

    for cluster_id in &cluster_ids {
        // Clusters nested inside an already-extracted cluster are handled by
        // the recursive layout of that cluster's sub-graph.
        if external.contains(cluster_id) || already_extracted.contains(*cluster_id) {
            continue;
        }

        let desc = &descendants[cluster_id];

        // Sub-graph direction: explicit cluster dir if present, otherwise the
        // flip of the parent graph's direction.
        let cluster_node = graph
            .get_node(cluster_id)
            .expect("cluster node must exist in graph");
        let dir = match cluster_node.metadata.get("dir") {
            Some(dir_str) => parse_direction(dir_str),
            None => flip_direction(graph.options.direction),
        };

        let mut sub = LayoutGraph::new(format!("{}_extracted", cluster_id));
        sub.options = graph.options.clone();
        sub.options.direction = dir;
        // Mermaid applies the parent's nodesep and ranksep + 25 to extracted
        // cluster graphs (dagre/index.js recursiveRender).
        sub.options.layer_spacing = graph.options.layer_spacing + CLUSTER_EXTRA_RANKSEP;

        // Copy descendant nodes in original order. Direct children lose their
        // parent link (the cluster is the sub-graph root); deeper descendants
        // keep theirs so nested clusters are handled recursively.
        for node in &graph.nodes {
            if desc.contains(&node.id) {
                let mut cloned = node.clone();
                if cloned.parent_id.as_deref() == Some(*cluster_id) {
                    cloned.parent_id = None;
                }
                sub.add_node(cloned);
            }
        }

        // Copy internal edges (both endpoints inside the cluster)
        let mut internal_edge_indices = Vec::new();
        for (index, edge) in graph.edges.iter().enumerate() {
            let all_inside = !edge.sources.is_empty()
                && !edge.targets.is_empty()
                && edge.sources.iter().all(|s| desc.contains(s))
                && edge.targets.iter().all(|t| desc.contains(t));
            if all_inside {
                sub.add_edge(edge.clone());
                internal_edge_indices.push(index);
            }
        }

        // Lay out the extracted cluster recursively
        let sub = layout_with_depth(sub, depth + 1)?;

        already_extracted.extend(desc.iter().cloned());
        extractions.push(ClusterExtraction {
            cluster_id: (*cluster_id).to_string(),
            descendant_ids: desc.clone(),
            internal_edge_indices,
            sub,
        });
    }

    Ok(extractions)
}

/// Translate an extracted cluster's contents into their final positions based
/// on where the collapsed cluster node ended up in the outer layout.
fn place_extracted_cluster(graph: &mut LayoutGraph, extraction: &ClusterExtraction) {
    let (cluster_x, cluster_y) = match graph.get_node(&extraction.cluster_id) {
        Some(node) => match (node.x, node.y) {
            (Some(x), Some(y)) => (x, y),
            _ => return,
        },
        None => return,
    };

    let origin_x = extraction.sub.bounds_x.unwrap_or(0.0);
    let origin_y = extraction.sub.bounds_y.unwrap_or(0.0);
    let dx = cluster_x + CLUSTER_PADDING - origin_x;
    let dy = cluster_y + CLUSTER_PADDING + CLUSTER_TITLE_HEIGHT - origin_y;

    for sub_node in &extraction.sub.nodes {
        if let Some(node) = graph.get_node_mut(&sub_node.id) {
            node.x = sub_node.x.map(|x| x + dx);
            node.y = sub_node.y.map(|y| y + dy);
            node.width = sub_node.width;
            node.height = sub_node.height;
            node.layer = sub_node.layer;
            node.order = sub_node.order;
        }
    }

    for (sub_index, &outer_index) in extraction.internal_edge_indices.iter().enumerate() {
        let sub_edge = &extraction.sub.edges[sub_index];
        if let Some(edge) = graph.edges.get_mut(outer_index) {
            edge.bend_points = sub_edge
                .bend_points
                .iter()
                .map(|p| Point::new(p.x + dx, p.y + dy))
                .collect();
            edge.label_position = sub_edge
                .label_position
                .map(|p| Point::new(p.x + dx, p.y + dy));
        }
    }
}

/// Parse direction from string
fn parse_direction(dir: &str) -> LayoutDirection {
    match dir.to_uppercase().as_str() {
        "LR" => LayoutDirection::LeftToRight,
        "RL" => LayoutDirection::RightToLeft,
        "BT" => LayoutDirection::BottomToTop,
        _ => LayoutDirection::TopToBottom, // Default TB
    }
}

/// Convert LayoutGraph to DagreGraph for dagre processing
#[cfg(test)]
fn to_dagre_graph(graph: &LayoutGraph) -> DagreGraph {
    to_dagre_graph_filtered(graph, &HashSet::new(), &HashSet::new())
}

/// Convert LayoutGraph to DagreGraph, skipping the given node ids and edge
/// indices (used to hide extracted cluster contents from the outer layout).
fn to_dagre_graph_filtered(
    graph: &LayoutGraph,
    skip_nodes: &HashSet<String>,
    skip_edges: &HashSet<usize>,
) -> DagreGraph {
    let mut dg = DagreGraph::new();

    // Set graph-level options
    dg.graph_mut().nodesep = graph.options.node_spacing;
    dg.graph_mut().ranksep = graph.options.layer_spacing;
    dg.graph_mut().rankdir = match graph.options.direction {
        LayoutDirection::TopToBottom => "TB".to_string(),
        LayoutDirection::BottomToTop => "BT".to_string(),
        LayoutDirection::LeftToRight => "LR".to_string(),
        LayoutDirection::RightToLeft => "RL".to_string(),
    };

    // Add nodes (flatten the tree, handling children separately)
    add_nodes_recursive(&mut dg, &graph.nodes, None, skip_nodes);

    // Add edges
    for (index, edge) in graph.edges.iter().enumerate() {
        if skip_edges.contains(&index) {
            continue;
        }
        if let (Some(source), Some(target)) = (edge.source(), edge.target()) {
            if skip_nodes.contains(source) || skip_nodes.contains(target) {
                continue;
            }
            // Use the label dimensions measured before layout (mirroring
            // mermaid's insertEdgeLabel, which sets edge.width/height from
            // the label bbox before running dagre).
            let (label_width, label_height) = if edge.label.is_some() {
                (edge.label_width, edge.label_height)
            } else {
                (0.0, 0.0)
            };

            let label = EdgeLabel {
                weight: edge.weight as i32,
                minlen: edge.minlen as i32,
                width: label_width,
                height: label_height,
                // Mermaid renders flowchart/state/class/requirement edges with
                // labelpos "c" (see flowDb.ts / stateCommon.ts G_EDGE_LABELPOS),
                // which centers the label on the edge path. Dagre's default is
                // "r", which shifts the label right by width/2 + labeloffset and
                // pulls it off the polyline. Use "c" so the dagre-computed
                // edge.x/edge.y coincide with the edge-label dummy bend point.
                labelpos: "c".to_string(),
                ..Default::default()
            };
            dg.set_edge(source, target, label);
        }
    }

    dg
}

/// Recursively add nodes to DagreGraph, handling compound nodes
fn add_nodes_recursive(
    dg: &mut DagreGraph,
    nodes: &[LayoutNode],
    parent: Option<&str>,
    skip_nodes: &HashSet<String>,
) {
    for node in nodes {
        if skip_nodes.contains(&node.id) {
            continue;
        }
        let label = NodeLabel {
            width: node.width,
            height: node.height,
            shape: node.shape,
            ..Default::default()
        };
        dg.set_node(&node.id, label);

        // Set parent relationship for compound graphs
        // Priority: explicit parent_id field, then parent parameter (from nested children)
        if let Some(ref parent_id) = node.parent_id {
            if !skip_nodes.contains(parent_id) {
                dg.set_parent(&node.id, parent_id);
            }
        } else if let Some(parent_id) = parent {
            dg.set_parent(&node.id, parent_id);
        }

        // Recursively add children
        if !node.children.is_empty() {
            add_nodes_recursive(dg, &node.children, Some(&node.id), skip_nodes);
        }
    }
}

/// Convert LayoutOptions to DagreConfig
fn to_dagre_config(options: &LayoutOptions) -> DagreConfig {
    use types::LayoutRanker;

    DagreConfig {
        rankdir: match options.direction {
            LayoutDirection::TopToBottom => RankDir::TB,
            LayoutDirection::BottomToTop => RankDir::BT,
            LayoutDirection::LeftToRight => RankDir::LR,
            LayoutDirection::RightToLeft => RankDir::RL,
        },
        nodesep: options.node_spacing,
        ranksep: options.layer_spacing,
        ranker: match options.ranker {
            LayoutRanker::NetworkSimplex => Ranker::NetworkSimplex,
            LayoutRanker::LongestPath => Ranker::LongestPath,
        },
        // Use DFS-based cycle detection instead of greedy
        // Greedy can incorrectly reverse forward edges in graphs with back edges
        acyclicer: dagre::Acyclicer::Dfs,
        ..Default::default()
    }
}

/// Copy position results from DagreGraph back to LayoutGraph
fn apply_dagre_results(graph: &mut LayoutGraph, dg: &DagreGraph) {
    apply_results_recursive(&mut graph.nodes, dg);

    // Copy edge bend points
    for edge in &mut graph.edges {
        if let (Some(source), Some(target)) = (edge.source(), edge.target()) {
            if let Some(edge_label) = dg.edge(source, target) {
                // Convert dagre points to layout points
                edge.bend_points = edge_label
                    .points
                    .iter()
                    .map(|p| Point::new(p.x, p.y))
                    .collect();

                // Position the edge label. Mirror mermaid's positionEdgeLabel
                // (dagre-wrapper/edges.js): prefer dagre's computed edge label
                // coordinate (edge.x/edge.y), which comes from the edge-label
                // dummy node's placement. Only fall back to the geometric
                // midpoint of the polyline when dagre did not compute one (e.g.
                // paths cut/rerouted around clusters, matching mermaid's
                // paths.updatedPath branch).
                if edge.label.is_some() {
                    if let (Some(x), Some(y)) = (edge_label.x, edge_label.y) {
                        edge.label_position = Some(Point::new(x, y));
                    } else if !edge.bend_points.is_empty() {
                        edge.label_position = types::geometric_midpoint(&edge.bend_points);
                    }
                }
            }
        }
    }
}

/// Recursively apply results to nodes
fn apply_results_recursive(nodes: &mut [LayoutNode], dg: &DagreGraph) {
    for node in nodes {
        if let Some(dagre_node) = dg.node(&node.id) {
            // For compound nodes (subgraphs), dagre calculates width/height from border positions
            // Copy these calculated dimensions back to the LayoutNode
            if dagre_node.width > 0.0 && node.width == 0.0 {
                node.width = dagre_node.width;
            }
            if dagre_node.height > 0.0 && node.height == 0.0 {
                node.height = dagre_node.height;
            }

            // Dagre returns center coordinates, convert to top-left
            if let (Some(cx), Some(cy)) = (dagre_node.x, dagre_node.y) {
                node.x = Some(cx - node.width / 2.0);
                node.y = Some(cy - node.height / 2.0);
            }

            // Copy layer/order info
            if let Some(rank) = dagre_node.rank {
                node.layer = Some(rank as usize);
            }
            if let Some(order) = dagre_node.order {
                node.order = Some(order);
            }
        }

        // Recursively apply to children
        if !node.children.is_empty() {
            apply_results_recursive(&mut node.children, dg);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_subgraph_with_different_direction() {
        // Test that an ISOLATED subgraph (no external connections) with its own
        // direction lays out nodes with that direction.
        // Main graph is TB (top-to-bottom), subgraph is LR (left-to-right)
        //
        // Mermaid extracts clusters without external connections into their own
        // graph and honors the explicit direction there.

        let mut graph = LayoutGraph::new("test_subgraph_dir");
        graph.options.direction = LayoutDirection::TopToBottom;

        // Add a subgraph node with LR direction
        let mut subgraph = LayoutNode::new("sub1", 0.0, 0.0);
        subgraph
            .metadata
            .insert("is_group".to_string(), "true".to_string());
        subgraph
            .metadata
            .insert("dir".to_string(), "LR".to_string());
        graph.add_node(subgraph);

        // Add child nodes belonging to the subgraph
        graph.add_node(LayoutNode::new("A", 50.0, 30.0).with_parent("sub1"));
        graph.add_node(LayoutNode::new("B", 50.0, 30.0).with_parent("sub1"));

        // Add edge within subgraph
        graph.add_edge(LayoutEdge::new("e1", "A", "B"));

        // Add nodes outside the subgraph (not connected to the subgraph)
        graph.add_node(LayoutNode::new("C", 50.0, 30.0));
        graph.add_node(LayoutNode::new("D", 50.0, 30.0));
        graph.add_edge(LayoutEdge::new("e2", "C", "D"));

        let result = layout(graph).unwrap();

        let a = result.get_node("A").unwrap();
        let b = result.get_node("B").unwrap();

        eprintln!("Node A: x={:?}, y={:?}", a.x, a.y);
        eprintln!("Node B: x={:?}, y={:?}", b.x, b.y);

        // Within the LR subgraph, A and B should be side-by-side (B to the right of A)
        // They should have similar y-coordinates
        let a_center_y = a.y.unwrap() + a.height / 2.0;
        let b_center_y = b.y.unwrap() + b.height / 2.0;

        assert!(
            (a_center_y - b_center_y).abs() < 10.0,
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

    #[test]
    fn test_isolated_subgraph_flips_direction_in_tb_parent() {
        // Mermaid extracts clusters WITHOUT external connections into their own
        // graph and lays them out with the FLIPPED default direction:
        // TB parent -> LR subgraph (mermaid-graphlib.js extractor:
        // `dir = graphSettings.rankdir === 'TB' ? 'LR' : 'TB'`).
        let mut graph = LayoutGraph::new("test_flip_tb");
        graph.options.direction = LayoutDirection::TopToBottom;

        // Subgraph with NO explicit direction
        let mut subgraph = LayoutNode::new("sub1", 0.0, 0.0);
        subgraph
            .metadata
            .insert("is_group".to_string(), "true".to_string());
        graph.add_node(subgraph);

        graph.add_node(LayoutNode::new("A", 50.0, 30.0).with_parent("sub1"));
        graph.add_node(LayoutNode::new("B", 50.0, 30.0).with_parent("sub1"));
        graph.add_node(LayoutNode::new("C", 50.0, 30.0).with_parent("sub1"));
        graph.add_edge(LayoutEdge::new("e1", "A", "B"));
        graph.add_edge(LayoutEdge::new("e2", "B", "C"));

        // Unrelated nodes outside the subgraph
        graph.add_node(LayoutNode::new("X", 50.0, 30.0));
        graph.add_node(LayoutNode::new("Y", 50.0, 30.0));
        graph.add_edge(LayoutEdge::new("e3", "X", "Y"));

        let result = layout(graph).unwrap();

        let a = result.get_node("A").unwrap();
        let b = result.get_node("B").unwrap();
        let c = result.get_node("C").unwrap();

        // Chain should be laid out horizontally (LR): same y, increasing x
        let a_cy = a.y.unwrap() + a.height / 2.0;
        let b_cy = b.y.unwrap() + b.height / 2.0;
        let c_cy = c.y.unwrap() + c.height / 2.0;
        assert!(
            (a_cy - b_cy).abs() < 10.0 && (b_cy - c_cy).abs() < 10.0,
            "Isolated subgraph chain in TB parent should be horizontal (LR). \
             A.cy={:.1}, B.cy={:.1}, C.cy={:.1}",
            a_cy,
            b_cy,
            c_cy
        );
        assert!(
            a.x.unwrap() < b.x.unwrap() && b.x.unwrap() < c.x.unwrap(),
            "Isolated subgraph chain should flow left-to-right. A.x={:.1}, B.x={:.1}, C.x={:.1}",
            a.x.unwrap(),
            b.x.unwrap(),
            c.x.unwrap()
        );

        // Children should sit inside the collapsed cluster node bounds
        let sub = result.get_node("sub1").unwrap();
        let (sx, sy) = (sub.x.unwrap(), sub.y.unwrap());
        for id in ["A", "B", "C"] {
            let n = result.get_node(id).unwrap();
            assert!(
                n.x.unwrap() >= sx
                    && n.y.unwrap() >= sy
                    && n.x.unwrap() + n.width <= sx + sub.width + 0.01
                    && n.y.unwrap() + n.height <= sy + sub.height + 0.01,
                "Child {} should be inside cluster bounds. child=({:.1},{:.1},{:.1},{:.1}) \
                 cluster=({:.1},{:.1},{:.1},{:.1})",
                id,
                n.x.unwrap(),
                n.y.unwrap(),
                n.width,
                n.height,
                sx,
                sy,
                sub.width,
                sub.height
            );
        }
    }

    #[test]
    fn test_isolated_subgraph_in_lr_parent_lays_out_tb() {
        // Flip default: any non-TB parent direction flips extracted clusters to TB.
        let mut graph = LayoutGraph::new("test_flip_lr");
        graph.options.direction = LayoutDirection::LeftToRight;

        let mut subgraph = LayoutNode::new("sub1", 0.0, 0.0);
        subgraph
            .metadata
            .insert("is_group".to_string(), "true".to_string());
        graph.add_node(subgraph);

        graph.add_node(LayoutNode::new("A", 50.0, 30.0).with_parent("sub1"));
        graph.add_node(LayoutNode::new("B", 50.0, 30.0).with_parent("sub1"));
        graph.add_edge(LayoutEdge::new("e1", "A", "B"));

        graph.add_node(LayoutNode::new("X", 50.0, 30.0));
        graph.add_node(LayoutNode::new("Y", 50.0, 30.0));
        graph.add_edge(LayoutEdge::new("e2", "X", "Y"));

        let result = layout(graph).unwrap();

        let a = result.get_node("A").unwrap();
        let b = result.get_node("B").unwrap();

        let a_cx = a.x.unwrap() + a.width / 2.0;
        let b_cx = b.x.unwrap() + b.width / 2.0;
        assert!(
            (a_cx - b_cx).abs() < 10.0,
            "Isolated subgraph in LR parent should lay out TB (same x). A.cx={:.1}, B.cx={:.1}",
            a_cx,
            b_cx
        );
        assert!(
            b.y.unwrap() > a.y.unwrap(),
            "B should be below A in TB sub-layout. A.y={:.1}, B.y={:.1}",
            a.y.unwrap(),
            b.y.unwrap()
        );
    }

    #[test]
    fn test_cluster_id_edges_do_not_block_extraction() {
        // Mermaid marks externalConnections only when an edge has exactly one
        // endpoint among a cluster's DESCENDANTS (d1 XOR d2); the cluster id is
        // not among its own descendants, so an edge whose endpoint IS the
        // cluster id (e.g. `Terminal -.-> Problem`) does not block extraction.
        // The extracted cluster must still honor its explicit `direction`.
        // Mirrors channel_flowchart_terminal_layers.
        let mut graph = LayoutGraph::new("test_cluster_id_edges");
        graph.options.direction = LayoutDirection::TopToBottom;

        // Cluster sub1 with explicit LR direction, children A -> B
        let mut sub1 = LayoutNode::new("sub1", 0.0, 0.0);
        sub1.metadata
            .insert("is_group".to_string(), "true".to_string());
        sub1.metadata.insert("dir".to_string(), "LR".to_string());
        graph.add_node(sub1);
        graph.add_node(LayoutNode::new("A", 50.0, 30.0).with_parent("sub1"));
        graph.add_node(LayoutNode::new("B", 50.0, 30.0).with_parent("sub1"));
        graph.add_edge(LayoutEdge::new("e1", "A", "B"));

        // Cluster sub2, children X -> Y
        let mut sub2 = LayoutNode::new("sub2", 0.0, 0.0);
        sub2.metadata
            .insert("is_group".to_string(), "true".to_string());
        graph.add_node(sub2);
        graph.add_node(LayoutNode::new("X", 50.0, 30.0).with_parent("sub2"));
        graph.add_node(LayoutNode::new("Y", 50.0, 30.0).with_parent("sub2"));
        graph.add_edge(LayoutEdge::new("e2", "X", "Y"));

        // Edge between the CLUSTER IDS themselves (no descendant endpoints)
        graph.add_edge(LayoutEdge::new("e3", "sub1", "sub2"));

        let result = layout(graph).unwrap();

        let a = result.get_node("A").unwrap();
        let b = result.get_node("B").unwrap();

        // sub1's explicit LR direction must be honored: A and B side-by-side
        let a_cy = a.y.unwrap() + a.height / 2.0;
        let b_cy = b.y.unwrap() + b.height / 2.0;
        assert!(
            (a_cy - b_cy).abs() < 10.0,
            "Cluster-id edges must not block extraction: sub1's LR direction \
             should be honored. A.cy={:.1}, B.cy={:.1}",
            a_cy,
            b_cy
        );
        assert!(
            b.x.unwrap() > a.x.unwrap(),
            "B should be right of A in LR cluster. A.x={:.1}, B.x={:.1}",
            a.x.unwrap(),
            b.x.unwrap()
        );

        // The cluster-id edge keeps the clusters connected in the outer TB
        // graph: sub2 should be laid out below sub1.
        let s1 = result.get_node("sub1").unwrap();
        let s2 = result.get_node("sub2").unwrap();
        assert!(
            s2.y.unwrap() > s1.y.unwrap() + s1.height / 2.0,
            "sub2 should be below sub1 in outer TB layout. sub1.y={:.1} sub2.y={:.1}",
            s1.y.unwrap(),
            s2.y.unwrap()
        );
    }

    #[test]
    fn test_connected_subgraph_not_extracted_ignores_explicit_direction() {
        // Mermaid NEVER extracts clusters with external connections, and thus
        // ignores their explicit direction: children follow the parent direction.
        let mut graph = LayoutGraph::new("test_connected_dir_ignored");
        graph.options.direction = LayoutDirection::TopToBottom;

        let mut subgraph = LayoutNode::new("sub1", 0.0, 0.0);
        subgraph
            .metadata
            .insert("is_group".to_string(), "true".to_string());
        subgraph
            .metadata
            .insert("dir".to_string(), "LR".to_string());
        graph.add_node(subgraph);

        graph.add_node(LayoutNode::new("A", 50.0, 30.0).with_parent("sub1"));
        graph.add_node(LayoutNode::new("B", 50.0, 30.0).with_parent("sub1"));
        graph.add_edge(LayoutEdge::new("e1", "A", "B"));

        // External connection: B -> C where C is outside the subgraph
        graph.add_node(LayoutNode::new("C", 50.0, 30.0));
        graph.add_edge(LayoutEdge::new("e2", "B", "C"));

        let result = layout(graph).unwrap();

        let a = result.get_node("A").unwrap();
        let b = result.get_node("B").unwrap();

        // With external connections the cluster stays in the parent layout:
        // TB direction applies, so B is below A.
        assert!(
            b.y.unwrap() > a.y.unwrap() + a.height / 2.0,
            "Connected subgraph should follow parent TB direction (dir ignored). \
             A.y={:.1}, B.y={:.1}",
            a.y.unwrap(),
            b.y.unwrap()
        );
    }

    #[test]
    fn test_layout_simple_graph() {
        let mut graph = LayoutGraph::new("test");
        graph.add_node(LayoutNode::new("A", 50.0, 30.0));
        graph.add_node(LayoutNode::new("B", 50.0, 30.0));
        graph.add_edge(LayoutEdge::new("e1", "A", "B"));

        let result = layout(graph).unwrap();

        // Both nodes should have positions assigned
        let a = result.get_node("A").unwrap();
        let b = result.get_node("B").unwrap();

        assert!(a.x.is_some(), "Node A should have x position");
        assert!(a.y.is_some(), "Node A should have y position");
        assert!(b.x.is_some(), "Node B should have x position");
        assert!(b.y.is_some(), "Node B should have y position");

        // B should be below A (in TB layout)
        assert!(
            b.y.unwrap() > a.y.unwrap(),
            "B should be below A in top-to-bottom layout"
        );
    }

    #[test]
    fn test_layout_diamond() {
        // A -> B, A -> C, B -> D, C -> D
        let mut graph = LayoutGraph::new("diamond");
        graph.add_node(LayoutNode::new("A", 50.0, 30.0));
        graph.add_node(LayoutNode::new("B", 50.0, 30.0));
        graph.add_node(LayoutNode::new("C", 50.0, 30.0));
        graph.add_node(LayoutNode::new("D", 50.0, 30.0));
        graph.add_edge(LayoutEdge::new("e1", "A", "B"));
        graph.add_edge(LayoutEdge::new("e2", "A", "C"));
        graph.add_edge(LayoutEdge::new("e3", "B", "D"));
        graph.add_edge(LayoutEdge::new("e4", "C", "D"));

        let result = layout(graph).unwrap();

        let a = result.get_node("A").unwrap();
        let b = result.get_node("B").unwrap();
        let c = result.get_node("C").unwrap();
        let d = result.get_node("D").unwrap();

        // All nodes should have positions
        assert!(a.x.is_some() && a.y.is_some());
        assert!(b.x.is_some() && b.y.is_some());
        assert!(c.x.is_some() && c.y.is_some());
        assert!(d.x.is_some() && d.y.is_some());

        // B and C should be on the same layer (same y)
        assert!(
            (b.y.unwrap() - c.y.unwrap()).abs() < 1.0,
            "B and C should be on the same layer"
        );

        // D should be below B and C
        assert!(d.y.unwrap() > b.y.unwrap());
    }

    #[test]
    fn test_edge_points_generated() {
        let mut graph = LayoutGraph::new("test");
        graph.add_node(LayoutNode::new("A", 50.0, 30.0));
        graph.add_node(LayoutNode::new("B", 50.0, 30.0));
        graph.add_edge(LayoutEdge::new("e1", "A", "B"));

        let result = layout(graph).unwrap();

        // Edge should have bend points after layout
        let edge = result.edges.first().expect("Should have an edge");
        assert!(
            !edge.bend_points.is_empty(),
            "Edge should have bend points after layout, got {} points",
            edge.bend_points.len()
        );

        // Should have at least 2 points (start and end)
        assert!(
            edge.bend_points.len() >= 2,
            "Edge should have at least start and end points, got {} points",
            edge.bend_points.len()
        );
    }

    #[test]
    fn test_edge_points_lr_direction() {
        // Test LR (left-to-right) layout which flowcharts use
        let mut graph = LayoutGraph::new("test_lr");
        graph.options.direction = LayoutDirection::LeftToRight;
        graph.add_node(LayoutNode::new("A", 50.0, 30.0));
        graph.add_node(LayoutNode::new("B", 50.0, 30.0));
        graph.add_edge(LayoutEdge::new("L-A-B-0", "A", "B"));

        let result = layout(graph).unwrap();

        // Check that edge points exist for LR layout
        let edge = result.edges.first().expect("Should have an edge");
        eprintln!(
            "LR Edge {} has {} bend points:",
            edge.id,
            edge.bend_points.len()
        );
        for (i, p) in edge.bend_points.iter().enumerate() {
            eprintln!("  Point {}: ({:.1}, {:.1})", i, p.x, p.y);
        }

        assert!(
            !edge.bend_points.is_empty(),
            "LR Edge should have bend points, got {} points",
            edge.bend_points.len()
        );
    }

    #[test]
    fn test_layout_left_to_right() {
        let mut graph = LayoutGraph::new("test");
        graph.options.direction = LayoutDirection::LeftToRight;
        graph.add_node(LayoutNode::new("A", 50.0, 30.0));
        graph.add_node(LayoutNode::new("B", 50.0, 30.0));
        graph.add_edge(LayoutEdge::new("e1", "A", "B"));

        let result = layout(graph).unwrap();

        let a = result.get_node("A").unwrap();
        let b = result.get_node("B").unwrap();

        // B should be to the right of A (in LR layout)
        assert!(
            b.x.unwrap() > a.x.unwrap(),
            "B should be to the right of A in left-to-right layout"
        );
    }

    #[test]
    fn test_edge_label_gets_position() {
        let mut graph = LayoutGraph::new("test_label");
        graph.options.direction = LayoutDirection::LeftToRight;
        graph.add_node(LayoutNode::new("A", 50.0, 30.0));
        graph.add_node(LayoutNode::new("B", 50.0, 30.0));

        // Add edge with label
        let edge = LayoutEdge::new("e1", "A", "B").with_label("Yes");
        graph.add_edge(edge);

        let result = layout(graph).unwrap();

        // Edge label should have a position
        let edge = result.edges.first().expect("Should have an edge");
        assert!(
            edge.label_position.is_some(),
            "Edge with label should have label_position set. Label: {:?}, Position: {:?}",
            edge.label,
            edge.label_position
        );

        // Label position should be between the nodes
        let a = result.get_node("A").unwrap();
        let b = result.get_node("B").unwrap();
        let label_pos = edge.label_position.unwrap();

        // For LR layout, label x should be between A and B
        let a_right = a.x.unwrap() + a.width;
        let b_left = b.x.unwrap();
        assert!(
            label_pos.x > a_right && label_pos.x < b_left,
            "Label x ({}) should be between A right edge ({}) and B left edge ({})",
            label_pos.x,
            a_right,
            b_left
        );
    }

    #[test]
    fn test_edge_label_y_position_diagonal_edge() {
        // Test that edge labels are positioned at the midpoint for diagonal edges
        // This reproduces the bug where LR flowchart edge labels were positioned
        // at the source y-coordinate instead of the edge midpoint
        let mut graph = LayoutGraph::new("test_diagonal_label");
        graph.options.direction = LayoutDirection::LeftToRight;

        // Create a "decision" pattern: B has edges going to C (above) and D (below)
        graph.add_node(LayoutNode::new("A", 50.0, 30.0));
        graph.add_node(LayoutNode::new("B", 80.0, 80.0)); // Larger node like a diamond
        graph.add_node(LayoutNode::new("C", 50.0, 30.0));
        graph.add_node(LayoutNode::new("D", 50.0, 30.0));

        // A -> B (no label)
        graph.add_edge(LayoutEdge::new("e_ab", "A", "B"));
        // B -> C with "Yes" label
        graph.add_edge(LayoutEdge::new("e_bc", "B", "C").with_label("Yes"));
        // B -> D with "No" label
        graph.add_edge(LayoutEdge::new("e_bd", "B", "D").with_label("No"));

        let result = layout(graph).unwrap();

        // Get node positions
        let b = result.get_node("B").unwrap();
        let c = result.get_node("C").unwrap();
        let d = result.get_node("D").unwrap();

        let b_center_y = b.y.unwrap() + b.height / 2.0;
        let c_center_y = c.y.unwrap() + c.height / 2.0;
        let d_center_y = d.y.unwrap() + d.height / 2.0;

        eprintln!("Node B center y: {}", b_center_y);
        eprintln!("Node C center y: {}", c_center_y);
        eprintln!("Node D center y: {}", d_center_y);

        // Find the B->C edge
        let edge_bc = result
            .edges
            .iter()
            .find(|e| e.id == "e_bc")
            .expect("Should have edge B->C");
        let label_pos_bc = edge_bc
            .label_position
            .expect("Edge B->C should have label position");

        eprintln!("Edge B->C label y: {}", label_pos_bc.y);
        eprintln!("Edge B->C bend points: {:?}", edge_bc.bend_points);

        // The label y should be between B and C's y-coordinates, not at B's y
        // For a diagonal edge going from B to C, label should be near midpoint
        let min_y = b_center_y.min(c_center_y);
        let max_y = b_center_y.max(c_center_y);
        let midpoint_y = (b_center_y + c_center_y) / 2.0;

        // Allow some tolerance - label should be within the range, closer to midpoint
        // The bug was that labels were at source y, not midpoint
        assert!(
            label_pos_bc.y >= min_y - 10.0 && label_pos_bc.y <= max_y + 10.0,
            "Label y ({}) should be between B ({}) and C ({}) y-coordinates (with tolerance)",
            label_pos_bc.y,
            b_center_y,
            c_center_y
        );

        // More strict check: label should be reasonably close to the midpoint
        // If B and C are at the same y (range=0), this is valid - label at their y is correct
        let distance_from_midpoint = (label_pos_bc.y - midpoint_y).abs();
        let total_range = (max_y - min_y).abs();
        if total_range > 0.0 {
            assert!(
                distance_from_midpoint < total_range * 0.6,
                "Label y ({}) should be close to midpoint ({}), not at an extreme. Distance: {}, Range: {}",
                label_pos_bc.y,
                midpoint_y,
                distance_from_midpoint,
                total_range
            );
        } else {
            // When range is 0 (B and C at same y), label at that y is correct
            assert!(
                distance_from_midpoint < 5.0,
                "Label y ({}) should be at midpoint ({}) when nodes are at same y",
                label_pos_bc.y,
                midpoint_y
            );
        }
    }

    #[test]
    fn test_edge_label_uses_dagre_position_not_midpoint() {
        // Mermaid's positionEdgeLabel keeps the dagre-computed edge label
        // coordinate (edge.x/edge.y) unless the path was cut by a cluster.
        // Dagre computes that coordinate from the edge-label dummy node, which
        // is also emitted as a bend point. So the label position must coincide
        // with one of the edge's bend points -- NOT the geometric midpoint of
        // the polyline, which for a diagonal decision edge falls between
        // vertices and can be ~50px away.
        let mut graph = LayoutGraph::new("test_dagre_label_pos");
        graph.options.direction = LayoutDirection::TopToBottom;

        // Decision diamond fanning out to two children (message_indent shape).
        graph.add_node(LayoutNode::new("Start", 200.0, 54.0));
        graph.add_node(LayoutNode::new("IsAction", 124.0, 124.0));
        graph.add_node(LayoutNode::new("ActionPath", 178.0, 54.0));
        graph.add_node(LayoutNode::new("NormalPath", 178.0, 54.0));

        graph.add_edge(LayoutEdge::new("e0", "Start", "IsAction"));
        graph.add_edge(
            LayoutEdge::new("e1", "IsAction", "ActionPath").with_label("Yes\nMessageType::Action"),
        );
        graph.add_edge(
            LayoutEdge::new("e2", "IsAction", "NormalPath").with_label("No\nNormal text"),
        );

        let result = layout(graph).unwrap();

        let edge = result
            .edges
            .iter()
            .find(|e| e.id == "e1")
            .expect("edge e1 present");
        let label_pos = edge.label_position.expect("label position set");

        // The label must coincide with one of the bend points (the edge-label
        // dummy vertex that dagre also uses as edge.x/edge.y).
        let min_dist = edge
            .bend_points
            .iter()
            .map(|p| ((p.x - label_pos.x).powi(2) + (p.y - label_pos.y).powi(2)).sqrt())
            .fold(f64::INFINITY, f64::min);

        assert!(
            min_dist < 1.0,
            "Edge label at ({:.2},{:.2}) should coincide with a dagre bend point, \
             but nearest bend point is {:.2}px away. Bend points: {:?}",
            label_pos.x,
            label_pos.y,
            min_dist,
            edge.bend_points
        );
    }

    #[test]
    fn test_simple_chain_tb_alignment() {
        // Simple chain without back edges should be perfectly vertically aligned
        let mut graph = LayoutGraph::new("simple_chain");
        graph.options.direction = LayoutDirection::TopToBottom;
        graph.options.node_spacing = 50.0;
        graph.options.layer_spacing = 60.0;

        graph.add_node(LayoutNode::new("A", 80.0, 40.0));
        graph.add_node(LayoutNode::new("B", 80.0, 40.0));
        graph.add_node(LayoutNode::new("C", 80.0, 40.0));
        graph.add_node(LayoutNode::new("D", 80.0, 40.0));

        graph.add_edge(LayoutEdge::new("e1", "A", "B"));
        graph.add_edge(LayoutEdge::new("e2", "B", "C"));
        graph.add_edge(LayoutEdge::new("e3", "C", "D"));

        let result = layout(graph).unwrap();

        eprintln!("Simple chain layout:");
        for node in &result.nodes {
            eprintln!(
                "  {}: x={:.1}, y={:.1}",
                node.id,
                node.x.unwrap_or(0.0),
                node.y.unwrap_or(0.0)
            );
        }

        // All nodes should have the same x (within 1 pixel)
        let a_x = result.get_node("A").unwrap().x.unwrap();
        let b_x = result.get_node("B").unwrap().x.unwrap();
        let c_x = result.get_node("C").unwrap().x.unwrap();
        let d_x = result.get_node("D").unwrap().x.unwrap();

        assert!(
            (a_x - b_x).abs() < 1.0,
            "A ({:.1}) and B ({:.1}) should have same x",
            a_x,
            b_x
        );
        assert!(
            (b_x - c_x).abs() < 1.0,
            "B ({:.1}) and C ({:.1}) should have same x",
            b_x,
            c_x
        );
        assert!(
            (c_x - d_x).abs() < 1.0,
            "C ({:.1}) and D ({:.1}) should have same x",
            c_x,
            d_x
        );
    }

    #[test]
    fn test_state_diagram_pattern_tb_alignment() {
        // This test mimics the state diagram pattern:
        // Start -> Idle -> Running -> Error -> End
        // With back edges: Running -> Idle, Error -> Idle
        //
        // In TB layout, all nodes should be roughly vertically aligned
        // Back edges create dummy nodes but shouldn't significantly spread the layout
        let mut graph = LayoutGraph::new("state_pattern");
        graph.options.direction = LayoutDirection::TopToBottom;
        graph.options.node_spacing = 50.0;
        graph.options.layer_spacing = 60.0;

        // Add nodes (small circles for start/end, rectangles for states)
        graph.add_node(LayoutNode::new("Start", 24.0, 24.0).with_shape(NodeShape::Circle));
        graph.add_node(LayoutNode::new("Idle", 80.0, 40.0).with_shape(NodeShape::RoundedRect));
        graph.add_node(LayoutNode::new("Running", 80.0, 40.0).with_shape(NodeShape::RoundedRect));
        graph.add_node(LayoutNode::new("Error", 80.0, 40.0).with_shape(NodeShape::RoundedRect));
        graph.add_node(LayoutNode::new("End", 24.0, 24.0).with_shape(NodeShape::DoubleCircle));

        // Forward edges (main flow)
        graph.add_edge(LayoutEdge::new("e1", "Start", "Idle"));
        graph.add_edge(LayoutEdge::new("e2", "Idle", "Running").with_label("start"));
        graph.add_edge(LayoutEdge::new("e3", "Running", "Error").with_label("error"));
        graph.add_edge(LayoutEdge::new("e4", "Error", "End"));

        // Back edges (cycles)
        graph.add_edge(LayoutEdge::new("e5", "Running", "Idle").with_label("stop"));
        graph.add_edge(LayoutEdge::new("e6", "Error", "Idle").with_label("reset"));

        let result = layout(graph).unwrap();

        // Get x coordinates for main states (excluding start/end circles)
        let idle_x = result.get_node("Idle").unwrap().x.unwrap();
        let running_x = result.get_node("Running").unwrap().x.unwrap();
        let error_x = result.get_node("Error").unwrap().x.unwrap();

        // In TB layout with this structure, all states should be roughly aligned
        // Back edges create dummy nodes which can cause some horizontal offset
        // Allow up to 50 pixels tolerance (less than a full node width)
        let mean_x = (idle_x + running_x + error_x) / 3.0;
        let max_deviation = 50.0;

        assert!(
            (idle_x - mean_x).abs() < max_deviation,
            "Idle x ({:.1}) should be near mean ({:.1}). States should be vertically aligned in TB layout.",
            idle_x, mean_x
        );
        assert!(
            (running_x - mean_x).abs() < max_deviation,
            "Running x ({:.1}) should be near mean ({:.1}). States should be vertically aligned in TB layout.",
            running_x, mean_x
        );
        assert!(
            (error_x - mean_x).abs() < max_deviation,
            "Error x ({:.1}) should be near mean ({:.1}). States should be vertically aligned in TB layout.",
            error_x, mean_x
        );
    }

    #[test]
    fn test_bidirectional_edges_both_have_points() {
        // Test that edges A→B and B→A both get bend points after layout
        // This is important for state diagrams with transitions in both directions
        let mut graph = LayoutGraph::new("bidirectional");
        graph.options.direction = LayoutDirection::TopToBottom;
        graph.options.node_spacing = 50.0;
        graph.options.layer_spacing = 60.0;

        // Create nodes
        graph.add_node(LayoutNode::new("Idle", 60.0, 40.0));
        graph.add_node(LayoutNode::new("Running", 80.0, 40.0));

        // Create bidirectional edges
        graph.add_edge(LayoutEdge::new("forward", "Idle", "Running").with_label("start"));
        graph.add_edge(LayoutEdge::new("backward", "Running", "Idle").with_label("stop"));

        let result = layout(graph).unwrap();

        // Find both edges
        let forward_edge = result
            .edges
            .iter()
            .find(|e| e.id == "forward")
            .expect("Should have forward edge");
        let backward_edge = result
            .edges
            .iter()
            .find(|e| e.id == "backward")
            .expect("Should have backward edge");

        eprintln!(
            "Forward edge (Idle→Running) has {} bend points",
            forward_edge.bend_points.len()
        );
        eprintln!(
            "Backward edge (Running→Idle) has {} bend points",
            backward_edge.bend_points.len()
        );

        // Both edges should have at least 2 points (start and end)
        assert!(
            forward_edge.bend_points.len() >= 2,
            "Forward edge should have at least 2 bend points, got {}",
            forward_edge.bend_points.len()
        );
        assert!(
            backward_edge.bend_points.len() >= 2,
            "Backward edge should have at least 2 bend points, got {}",
            backward_edge.bend_points.len()
        );
    }

    #[test]
    fn test_dagre_graph_preserves_edge_order() {
        // Test that edge order is preserved when converting LayoutGraph to DagreGraph.
        // This is critical for fork/join ordering.
        let mut graph = LayoutGraph::new("test_edge_order");
        graph.options.direction = LayoutDirection::TopToBottom;

        // Create fork pattern
        graph.add_node(LayoutNode::new("fork", 70.0, 10.0)); // Fork bar
        graph.add_node(LayoutNode::new("first_target", 100.0, 40.0));
        graph.add_node(LayoutNode::new("second_target", 100.0, 40.0));

        // Add edges in specific order
        graph.add_edge(LayoutEdge::new("e1", "fork", "first_target")); // First
        graph.add_edge(LayoutEdge::new("e2", "fork", "second_target")); // Second

        // Convert to DagreGraph
        let dg = to_dagre_graph(&graph);

        // Check successors order
        let successors = dg.successors("fork");
        eprintln!("DagreGraph successors of fork: {:?}", successors);

        assert_eq!(successors.len(), 2, "Should have 2 successors");
        assert_eq!(
            successors[0], "first_target",
            "First successor should be first_target"
        );
        assert_eq!(
            successors[1], "second_target",
            "Second successor should be second_target"
        );
    }

    #[test]
    fn test_fork_layout_position_order() {
        // Test that fork targets are positioned in edge definition order.
        // First defined target should be on the LEFT (smaller x).
        let mut graph = LayoutGraph::new("test_fork_positions");
        graph.options.direction = LayoutDirection::TopToBottom;

        // Create fork pattern
        graph.add_node(LayoutNode::new("start", 50.0, 30.0));
        graph.add_node(LayoutNode::new("fork", 70.0, 10.0)); // Fork bar
        graph.add_node(LayoutNode::new("first_target", 100.0, 40.0));
        graph.add_node(LayoutNode::new("second_target", 100.0, 40.0));
        graph.add_node(LayoutNode::new("join", 70.0, 10.0)); // Join bar

        // Add edges in specific order
        graph.add_edge(LayoutEdge::new("e0", "start", "fork"));
        graph.add_edge(LayoutEdge::new("e1", "fork", "first_target")); // First fork edge
        graph.add_edge(LayoutEdge::new("e2", "fork", "second_target")); // Second fork edge
        graph.add_edge(LayoutEdge::new("e3", "first_target", "join"));
        graph.add_edge(LayoutEdge::new("e4", "second_target", "join"));

        // Run layout
        let result = layout(graph).expect("Layout should succeed");

        let first = result
            .get_node("first_target")
            .expect("Should have first_target");
        let second = result
            .get_node("second_target")
            .expect("Should have second_target");

        let first_x = first.x.expect("first_target should have x position");
        let second_x = second.x.expect("second_target should have x position");

        eprintln!(
            "Fork layout: first_target.x={}, second_target.x={}",
            first_x, second_x
        );

        // First defined edge target should be on the left (smaller x)
        assert!(
            first_x < second_x,
            "first_target (first edge) should be LEFT of second_target. \
             first_target.x={}, second_target.x={}",
            first_x,
            second_x
        );
    }

    #[test]
    fn test_fork_layout_alphabetical_order_reversed() {
        // Test fork layout when alphabetical order is OPPOSITE to edge definition order.
        // This matches the state diagram case where:
        // - Edge 1: fork_state -> Validation (first edge)
        // - Edge 2: fork_state -> ResourceAlloc (second edge)
        // Alphabetically: "ResourceAlloc" < "Validation" (R < V)
        // So if alphabetical sorting happens, ResourceAlloc would be placed first.
        //
        // We want edge definition order, so Validation should be on the LEFT.
        let mut graph = LayoutGraph::new("test_alphabetical");
        graph.options.direction = LayoutDirection::TopToBottom;

        // Use names where alphabetical order is opposite to edge order
        // ZZZ should be FIRST (edge order) but comes LAST alphabetically
        // AAA should be SECOND (edge order) but comes FIRST alphabetically
        graph.add_node(LayoutNode::new("start", 50.0, 30.0));
        graph.add_node(LayoutNode::new("fork", 70.0, 10.0));
        graph.add_node(LayoutNode::new("ZZZ", 100.0, 40.0)); // First edge target
        graph.add_node(LayoutNode::new("AAA", 100.0, 40.0)); // Second edge target
        graph.add_node(LayoutNode::new("join", 70.0, 10.0));

        // Add edges in specific order - ZZZ first, AAA second
        graph.add_edge(LayoutEdge::new("e0", "start", "fork"));
        graph.add_edge(LayoutEdge::new("e1", "fork", "ZZZ")); // First fork edge
        graph.add_edge(LayoutEdge::new("e2", "fork", "AAA")); // Second fork edge
        graph.add_edge(LayoutEdge::new("e3", "ZZZ", "join"));
        graph.add_edge(LayoutEdge::new("e4", "AAA", "join"));

        // Convert to DagreGraph and check intermediate state
        let dg = to_dagre_graph(&graph);
        eprintln!("DagreGraph successors of fork: {:?}", dg.successors("fork"));

        // Check init_order
        use crate::layout::dagre::order::{assign_order, init_order};
        use crate::layout::dagre::rank;
        let mut dg = to_dagre_graph(&graph);
        let config = to_dagre_config(&graph.options);
        rank::assign_ranks(&mut dg, config.ranker);

        let layering = init_order(&dg);
        eprintln!("init_order layer 2: {:?}", layering.get(2));

        assign_order(&mut dg, &layering);
        eprintln!(
            "After assign_order: ZZZ.order={:?}, AAA.order={:?}",
            dg.node("ZZZ").and_then(|n| n.order),
            dg.node("AAA").and_then(|n| n.order)
        );

        // Run layout step by step to trace where order gets lost
        let mut dg2 = to_dagre_graph(&graph);
        let config2 = to_dagre_config(&graph.options);
        crate::layout::dagre::layout(&mut dg2, &config2);

        eprintln!(
            "After dagre::layout: ZZZ.order={:?}, AAA.order={:?}",
            dg2.node("ZZZ").and_then(|n| n.order),
            dg2.node("AAA").and_then(|n| n.order)
        );
        eprintln!(
            "After dagre::layout: ZZZ.x={:?}, AAA.x={:?}",
            dg2.node("ZZZ").and_then(|n| n.x),
            dg2.node("AAA").and_then(|n| n.x)
        );

        // Run full layout
        let result = layout(graph).expect("Layout should succeed");

        let zzz = result.get_node("ZZZ").expect("Should have ZZZ");
        let aaa = result.get_node("AAA").expect("Should have AAA");

        let zzz_x = zzz.x.expect("ZZZ should have x position");
        let aaa_x = aaa.x.expect("AAA should have x position");

        eprintln!(
            "Fork layout (reversed alpha): ZZZ.x={}, AAA.x={}",
            zzz_x, aaa_x
        );

        // ZZZ (first defined edge target) should be on the LEFT (smaller x)
        // even though "AAA" < "ZZZ" alphabetically
        assert!(
            zzz_x < aaa_x,
            "ZZZ (first edge) should be LEFT of AAA even though A < Z alphabetically. \
             ZZZ.x={}, AAA.x={}",
            zzz_x,
            aaa_x
        );
    }
}
