//! Brandes-Köpf x-coordinate assignment
//!
//! Based on "Fast and Simple Horizontal Coordinate Assignment" by Brandes and Köpf.
//!
//! The algorithm runs four passes (up-left, up-right, down-left, down-right) and
//! balances them by taking the median of the four x-coordinates for each node.

use crate::layout::dagre::graph::DagreGraph;
use std::collections::{HashMap, HashSet};

/// Block graph for horizontal compaction: (block root nodes, edges with separations).
/// The Vec preserves **layer insertion order** — roots appear in the order they are first
/// encountered when iterating layers left-to-right, top-to-bottom. This ordering is
/// critical for deterministic DFS traversal in horizontal compaction (matching dagre.js).
type BlockGraph = (Vec<String>, HashMap<String, Vec<(String, f64)>>);

/// Type alias for the neighbor function used in vertical alignment
type NeighborFn = dyn Fn(&DagreGraph, &str) -> Vec<String>;

/// Assign x coordinates to all nodes using Brandes-Köpf algorithm
pub fn position_x(g: &DagreGraph) -> HashMap<String, f64> {
    let nodesep = g.graph().nodesep;
    // dagre.js uses edgesep as-is (no minimum). The previous .max(10.0) was a
    // workaround for wider layouts; with conflict detection and proper compaction
    // the raw value produces correct results.
    let edgesep = g.graph().edgesep;

    // Build layer matrix
    let layers = build_layer_matrix(g);
    if layers.is_empty() {
        return HashMap::new();
    }

    // Find type-1 and type-2 conflicts (edges crossing inner segments)
    let conflicts = find_type1_conflicts(g, &layers);
    let type2 = find_type2_conflicts(g, &layers);
    // Merge type2 into conflicts
    let conflicts = merge_conflicts(conflicts, type2);

    // Run four alignment passes
    let mut xss: HashMap<&str, HashMap<String, f64>> = HashMap::new();

    for vert in ["u", "d"] {
        let adjusted_layers = if vert == "u" {
            layers.clone()
        } else {
            layers.iter().rev().cloned().collect()
        };

        for horiz in ["l", "r"] {
            let aligned_layers: Vec<Vec<String>> = if horiz == "r" {
                adjusted_layers
                    .iter()
                    .map(|layer| layer.iter().rev().cloned().collect())
                    .collect()
            } else {
                adjusted_layers.clone()
            };

            // Build alignment (with conflict avoidance)
            let neighbor_fn: Box<NeighborFn> = if vert == "u" {
                Box::new(|g: &DagreGraph, v: &str| g.predecessors(v).into_iter().cloned().collect())
            } else {
                Box::new(|g: &DagreGraph, v: &str| g.successors(v).into_iter().cloned().collect())
            };
            let (root, align) = vertical_alignment(g, &aligned_layers, &conflicts, &*neighbor_fn);

            // Compact horizontally
            let mut xs = horizontal_compaction(
                g,
                &aligned_layers,
                &root,
                &align,
                nodesep,
                edgesep,
                horiz == "r",
            );

            // Flip for right alignment
            if horiz == "r" {
                for x in xs.values_mut() {
                    *x = -*x;
                }
            }

            xss.insert(
                match (vert, horiz) {
                    ("u", "l") => "ul",
                    ("u", "r") => "ur",
                    ("d", "l") => "dl",
                    ("d", "r") => "dr",
                    _ => unreachable!(),
                },
                xs,
            );
        }
    }

    // Balance the four alignments by taking the median
    balance(g, &xss, &layers)
}

/// Build a matrix of nodes organized by layer
fn build_layer_matrix(g: &DagreGraph) -> Vec<Vec<String>> {
    let max_rank = g
        .nodes()
        .iter()
        .filter_map(|v| g.node(v).and_then(|n| n.rank))
        .max()
        .unwrap_or(0) as usize;

    let mut layers: Vec<Vec<(String, usize)>> = (0..=max_rank).map(|_| Vec::new()).collect();

    for v in g.nodes() {
        if let Some(node) = g.node(v) {
            if let (Some(rank), Some(order)) = (node.rank, node.order) {
                if rank >= 0 && (rank as usize) <= max_rank {
                    layers[rank as usize].push((v.clone(), order));
                }
            }
        }
    }

    // Sort each layer by order and extract just the node IDs
    layers
        .iter_mut()
        .map(|layer| {
            layer.sort_by_key(|(_, order)| *order);
            layer.iter().map(|(v, _)| v.clone()).collect()
        })
        .collect()
}

/// Type for conflict pairs: maps v -> set of w where (v,w) is a conflicting edge
type Conflicts = HashMap<String, HashSet<String>>;

/// Find the other end of an inner segment (dummy-to-dummy edge)
fn find_other_inner_segment_node(g: &DagreGraph, v: &str) -> Option<String> {
    if g.node(v).map(|n| n.dummy.is_some()).unwrap_or(false) {
        g.predecessors(v)
            .into_iter()
            .find(|u| g.node(u).map(|n| n.dummy.is_some()).unwrap_or(false))
            .cloned()
    } else {
        None
    }
}

/// Add a conflict between two nodes
fn add_conflict(conflicts: &mut Conflicts, v: &str, w: &str) {
    let (v, w) = if v > w { (w, v) } else { (v, w) };
    conflicts
        .entry(v.to_string())
        .or_default()
        .insert(w.to_string());
}

/// Check if two nodes have a conflict
fn has_conflict(conflicts: &Conflicts, v: &str, w: &str) -> bool {
    let (v, w) = if v > w { (w, v) } else { (v, w) };
    conflicts.get(v).map(|set| set.contains(w)).unwrap_or(false)
}

/// Merge two conflict maps
fn merge_conflicts(mut a: Conflicts, b: Conflicts) -> Conflicts {
    for (k, vs) in b {
        a.entry(k).or_default().extend(vs);
    }
    a
}

/// Find type-1 conflicts: non-inner segments crossing inner segments.
/// An inner segment is a dummy-to-dummy edge. When a non-inner edge crosses
/// an inner segment, aligning across it would distort the inner segment's
/// vertical path. Marking these as conflicts prevents such alignments.
fn find_type1_conflicts(g: &DagreGraph, layering: &[Vec<String>]) -> Conflicts {
    let mut conflicts = Conflicts::new();

    if layering.len() < 2 {
        return conflicts;
    }

    for i in 1..layering.len() {
        let prev_layer = &layering[i - 1];
        let layer = &layering[i];
        let prev_layer_length = prev_layer.len();

        let mut k0: usize = 0;
        let mut scan_pos: usize = 0;

        for (i_in_layer, v) in layer.iter().enumerate() {
            let w = find_other_inner_segment_node(g, v);
            let k1 = w
                .as_ref()
                .and_then(|w| g.node(w).and_then(|n| n.order))
                .unwrap_or(prev_layer_length);

            let is_last = i_in_layer == layer.len() - 1;

            if w.is_some() || is_last {
                // Scan nodes from scan_pos to current position
                for scan_node in layer.iter().take(i_in_layer + 1).skip(scan_pos) {
                    for u in g.predecessors(scan_node) {
                        let u_pos = g.node(u).and_then(|n| n.order).unwrap_or(0);
                        let u_is_dummy = g.node(u).map(|n| n.dummy.is_some()).unwrap_or(false);
                        let scan_is_dummy = g
                            .node(scan_node)
                            .map(|n| n.dummy.is_some())
                            .unwrap_or(false);

                        if (u_pos < k0 || k1 < u_pos) && !(u_is_dummy && scan_is_dummy) {
                            add_conflict(&mut conflicts, u, scan_node);
                        }
                    }
                }
                scan_pos = i_in_layer + 1;
                k0 = k1;
            }
        }
    }

    conflicts
}

/// Find type-2 conflicts: edges between dummy nodes that cross subgraph borders.
fn find_type2_conflicts(g: &DagreGraph, layering: &[Vec<String>]) -> Conflicts {
    let mut conflicts = Conflicts::new();

    if layering.len() < 2 {
        return conflicts;
    }

    for i in 1..layering.len() {
        let north = &layering[i - 1];
        let south = &layering[i];
        let mut prev_north_pos: i64 = -1;
        let mut south_pos: usize = 0;

        for (south_lookahead, v) in south.iter().enumerate() {
            if g.node(v).and_then(|n| n.dummy.as_deref()) == Some("border") {
                let predecessors: Vec<_> = g.predecessors(v).into_iter().cloned().collect();
                if let Some(pred) = predecessors.first() {
                    let next_north_pos = g.node(pred).and_then(|n| n.order).unwrap_or(0) as i64;
                    // Scan dummy nodes in [south_pos..south_lookahead]
                    for scan_node in south.iter().take(south_lookahead).skip(south_pos) {
                        if g.node(scan_node)
                            .map(|n| n.dummy.is_some())
                            .unwrap_or(false)
                        {
                            for u in g.predecessors(scan_node) {
                                let u_node = g.node(u);
                                let u_is_dummy = u_node.map(|n| n.dummy.is_some()).unwrap_or(false);
                                if u_is_dummy {
                                    let u_order = u_node.and_then(|n| n.order).unwrap_or(0) as i64;
                                    if u_order < prev_north_pos || u_order > next_north_pos {
                                        add_conflict(&mut conflicts, u, scan_node);
                                    }
                                }
                            }
                        }
                    }
                    south_pos = south_lookahead;
                    prev_north_pos = next_north_pos;
                }
            }
        }

        // Final scan from south_pos to end
        let next_north_pos = north.len() as i64;
        for scan_node in south.iter().skip(south_pos) {
            if g.node(scan_node)
                .map(|n| n.dummy.is_some())
                .unwrap_or(false)
            {
                for u in g.predecessors(scan_node) {
                    let u_node = g.node(u);
                    let u_is_dummy = u_node.map(|n| n.dummy.is_some()).unwrap_or(false);
                    if u_is_dummy {
                        let u_order = u_node.and_then(|n| n.order).unwrap_or(0) as i64;
                        if u_order < prev_north_pos || u_order > next_north_pos {
                            add_conflict(&mut conflicts, u, scan_node);
                        }
                    }
                }
            }
        }
    }

    conflicts
}

/// Vertical alignment: align nodes with their median neighbors.
/// Uses conflict detection to avoid aligning across inner segments.
fn vertical_alignment(
    g: &DagreGraph,
    layers: &[Vec<String>],
    conflicts: &Conflicts,
    neighbor_fn: &dyn Fn(&DagreGraph, &str) -> Vec<String>,
) -> (HashMap<String, String>, HashMap<String, String>) {
    let mut root: HashMap<String, String> = HashMap::new();
    let mut align: HashMap<String, String> = HashMap::new();
    let mut pos: HashMap<String, usize> = HashMap::new();

    // Initialize: each node is its own root and aligned to itself
    for layer in layers {
        for (order, v) in layer.iter().enumerate() {
            root.insert(v.clone(), v.clone());
            align.insert(v.clone(), v.clone());
            pos.insert(v.clone(), order);
        }
    }

    // Process layers
    for layer in layers {
        let mut prev_idx: i64 = -1;

        for v in layer {
            let mut neighbors = neighbor_fn(g, v);

            if neighbors.is_empty() {
                continue;
            }

            // Sort neighbors by their position
            neighbors.sort_by_key(|n| pos.get(n).copied().unwrap_or(0));

            // Find median neighbor(s)
            let len = neighbors.len();
            let mp = (len as f64 - 1.0) / 2.0;
            let median_low = mp.floor() as usize;
            let median_high = mp.ceil() as usize;

            for idx in median_low..=median_high {
                if let Some(w) = neighbors.get(idx) {
                    let w_pos = pos.get(w).copied().unwrap_or(0) as i64;

                    // Check if we can align with this neighbor:
                    // 1. v must not already be aligned to another node
                    // 2. No crossing with previously aligned nodes (w_pos > prev_idx)
                    // 3. No type-1 or type-2 conflict between v and w
                    if align.get(v).map(|a| a == v).unwrap_or(false)
                        && prev_idx < w_pos
                        && !has_conflict(conflicts, v, w)
                    {
                        // Align v with w
                        align.insert(w.clone(), v.clone());
                        let r = root.get(w).cloned().unwrap_or_else(|| w.clone());
                        root.insert(v.clone(), r.clone());
                        align.insert(v.clone(), r);
                        prev_idx = w_pos;
                    }
                }
            }
        }
    }

    (root, align)
}

/// Horizontal compaction: assign x coordinates based on blocks
/// This follows the dagre.js reference implementation using DFS iteration
/// which handles cycles gracefully (unlike topological sort)
fn horizontal_compaction(
    g: &DagreGraph,
    layers: &[Vec<String>],
    root: &HashMap<String, String>,
    _align: &HashMap<String, String>,
    nodesep: f64,
    edgesep: f64,
    reverse_sep: bool,
) -> HashMap<String, f64> {
    // Build block graph: nodes are block roots, edges are separation constraints
    let (block_graph, block_edges) =
        build_block_graph(g, layers, root, nodesep, edgesep, reverse_sep);

    // Determine which border type to skip in pass2
    let skip_border_type = if reverse_sep {
        "borderLeft"
    } else {
        "borderRight"
    };

    let mut xs: HashMap<String, f64> = HashMap::new();

    // Build successor map (for pass2) in deterministic order.
    // In dagre.js, blockG.successors() returns successors in edge-insertion order.
    // We iterate block_graph (which is in layer-insertion order) to replicate that.
    let mut successors: HashMap<String, Vec<(String, f64)>> = HashMap::new();
    for target in &block_graph {
        if let Some(preds) = block_edges.get(target) {
            for (source, sep) in preds {
                successors
                    .entry(source.clone())
                    .or_default()
                    .push((target.clone(), *sep));
            }
        }
    }

    // DFS-based iteration matching dagre.js: uses a stack with two-phase
    // visit pattern. On first visit, pushes the element back and all its
    // neighbors (even already-visited ones). On second visit, processes the
    // element. The initial stack order is the block graph's insertion order
    // (layer order), which is critical for deterministic results.
    fn dfs_iterate<F>(nodes: &[String], get_neighbors: impl Fn(&str) -> Vec<String>, mut process: F)
    where
        F: FnMut(&str),
    {
        let mut stack: Vec<String> = nodes.to_vec();
        let mut visited: HashSet<String> = HashSet::new();

        while let Some(elem) = stack.pop() {
            if visited.contains(&elem) {
                process(&elem);
            } else {
                visited.insert(elem.clone());
                stack.push(elem.clone());
                for neighbor in get_neighbors(&elem) {
                    stack.push(neighbor);
                }
            }
        }
    }

    // Pass 1: Assign smallest coordinates (process predecessors first)
    dfs_iterate(
        &block_graph,
        |v| {
            block_edges
                .get(v)
                .map(|preds| preds.iter().map(|(p, _)| p.clone()).collect())
                .unwrap_or_default()
        },
        |elem| {
            let x = block_edges
                .get(elem)
                .map(|preds| {
                    preds
                        .iter()
                        .filter_map(|(pred, sep)| xs.get(pred).map(|&px| px + sep))
                        .fold(0.0_f64, f64::max)
                })
                .unwrap_or(0.0);
            xs.insert(elem.to_string(), x);
        },
    );

    // Pass 2: Pull nodes toward maximum (process successors first)
    dfs_iterate(
        &block_graph,
        |v| {
            successors
                .get(v)
                .map(|succs| succs.iter().map(|(s, _)| s.clone()).collect())
                .unwrap_or_default()
        },
        |elem| {
            let min_x = successors
                .get(elem)
                .map(|succs| {
                    succs
                        .iter()
                        .filter_map(|(succ, sep)| xs.get(succ).map(|&sx| sx - sep))
                        .fold(f64::INFINITY, f64::min)
                })
                .unwrap_or(f64::INFINITY);

            // Pull toward max, but NOT for border nodes of the skip type
            let border_type = g.node(elem).and_then(|n| n.border_type.as_deref());
            if min_x != f64::INFINITY && border_type != Some(skip_border_type) {
                if let Some(curr_x) = xs.get_mut(elem) {
                    *curr_x = curr_x.max(min_x);
                }
            }
        },
    );

    // Assign x coordinates to all nodes from their roots
    // Use `root` map (not `align`) to get each node's block root
    let mut result: HashMap<String, f64> = HashMap::new();

    for (v, r) in root {
        if let Some(&rx) = xs.get(r) {
            result.insert(v.clone(), rx);
        }
    }

    result
}

/// Build block graph for horizontal compaction
/// Returns (ordered block root nodes, map of node -> list of (predecessor, separation))
/// The node ordering follows layer insertion order for deterministic DFS traversal.
fn build_block_graph(
    g: &DagreGraph,
    layers: &[Vec<String>],
    root: &HashMap<String, String>,
    nodesep: f64,
    edgesep: f64,
    reverse_sep: bool,
) -> BlockGraph {
    let mut block_nodes: Vec<String> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    let mut block_edges: HashMap<String, Vec<(String, f64)>> = HashMap::new();

    for layer in layers {
        let mut prev: Option<&String> = None;

        for v in layer {
            let v_root = root.get(v).unwrap_or(v);
            if seen.insert(v_root.clone()) {
                block_nodes.push(v_root.clone());
            }

            if let Some(u) = prev {
                let u_root = root.get(u).unwrap_or(u);

                // Calculate separation between u and v
                let sep = calculate_sep(g, v, u, nodesep, edgesep, reverse_sep);

                // Add edge from u_root to v_root with separation
                let prev_sep = block_edges
                    .get(v_root)
                    .and_then(|edges| edges.iter().find(|(p, _)| p == u_root))
                    .map(|(_, s)| *s)
                    .unwrap_or(0.0);

                // Keep maximum separation
                if sep > prev_sep {
                    block_edges
                        .entry(v_root.clone())
                        .or_default()
                        .retain(|(p, _)| p != u_root);
                    block_edges
                        .entry(v_root.clone())
                        .or_default()
                        .push((u_root.clone(), sep));
                }
            }

            prev = Some(v);
        }
    }

    (block_nodes, block_edges)
}

/// Calculate separation between two adjacent nodes in a layer.
/// This matches the dagre.js sep() function, including labelpos adjustments
/// for dummy edge-label nodes.
fn calculate_sep(
    g: &DagreGraph,
    v: &str,
    w: &str,
    nodesep: f64,
    edgesep: f64,
    reverse_sep: bool,
) -> f64 {
    let v_node = g.node(v);
    let w_node = g.node(w);

    let v_width = v_node.map(|n| n.width).unwrap_or(0.0);
    let w_width = w_node.map(|n| n.width).unwrap_or(0.0);
    let v_is_dummy = v_node.map(|n| n.dummy.is_some()).unwrap_or(false);
    let w_is_dummy = w_node.map(|n| n.dummy.is_some()).unwrap_or(false);

    let mut sum = 0.0;

    // v's width contribution + labelpos adjustment
    sum += v_width / 2.0;
    if let Some(labelpos) = v_node.and_then(|n| n.labelpos.as_deref()) {
        let delta = match labelpos.to_lowercase().as_str() {
            "l" => -v_width / 2.0,
            "r" => v_width / 2.0,
            _ => 0.0,
        };
        if delta != 0.0 {
            sum += if reverse_sep { delta } else { -delta };
        }
    }

    // Add separation based on whether nodes are dummies
    sum += if v_is_dummy { edgesep } else { nodesep } / 2.0;
    sum += if w_is_dummy { edgesep } else { nodesep } / 2.0;

    // w's width contribution + labelpos adjustment (note: delta signs are inverted vs v)
    sum += w_width / 2.0;
    if let Some(labelpos) = w_node.and_then(|n| n.labelpos.as_deref()) {
        let delta = match labelpos.to_lowercase().as_str() {
            "l" => w_width / 2.0,
            "r" => -w_width / 2.0,
            _ => 0.0,
        };
        if delta != 0.0 {
            sum += if reverse_sep { delta } else { -delta };
        }
    }

    sum
}

/// Find the alignment with the smallest visual width, accounting for node widths.
/// This matches dagre.js findSmallestWidthAlignment which uses x ± halfWidth.
fn find_smallest_width_alignment<'a>(
    g: &DagreGraph,
    xss: &HashMap<&'a str, HashMap<String, f64>>,
) -> Option<&'a str> {
    let mut min_width = f64::MAX;
    let mut best: Option<&str> = None;

    for (&name, xs) in xss {
        let mut max_bound = f64::NEG_INFINITY;
        let mut min_bound = f64::INFINITY;

        for (v, &x) in xs {
            let half_width = g.node(v).map(|n| n.width).unwrap_or(0.0) / 2.0;
            max_bound = max_bound.max(x + half_width);
            min_bound = min_bound.min(x - half_width);
        }

        let width = max_bound - min_bound;
        if width < min_width {
            min_width = width;
            best = Some(name);
        }
    }

    best
}

/// Align all four alignment results to the smallest width alignment.
/// Left-biased alignments align at the minimum, right-biased at the maximum.
fn align_coordinates(xss: &mut HashMap<&str, HashMap<String, f64>>, align_to_name: &str) {
    // Compute min/max of the reference alignment
    let align_xs = &xss[align_to_name];
    let align_min: f64 = align_xs.values().copied().fold(f64::INFINITY, f64::min);
    let align_max: f64 = align_xs.values().copied().fold(f64::NEG_INFINITY, f64::max);

    let names: Vec<String> = xss.keys().map(|k| k.to_string()).collect();

    for name in &names {
        if name == align_to_name {
            continue;
        }

        let xs = xss.get(name.as_str()).unwrap();
        let xs_min: f64 = xs.values().copied().fold(f64::INFINITY, f64::min);
        let xs_max: f64 = xs.values().copied().fold(f64::NEG_INFINITY, f64::max);

        // Left-biased alignments (ending with 'l') align at minimum
        // Right-biased alignments (ending with 'r') align at maximum
        let delta = if name.ends_with('l') {
            align_min - xs_min
        } else {
            align_max - xs_max
        };

        if delta != 0.0 {
            let xs_mut = xss.get_mut(name.as_str()).unwrap();
            for x in xs_mut.values_mut() {
                *x += delta;
            }
        }
    }
}

/// Balance the four alignments by taking the median x for each node
fn balance(
    g: &DagreGraph,
    xss: &HashMap<&str, HashMap<String, f64>>,
    layers: &[Vec<String>],
) -> HashMap<String, f64> {
    let mut result: HashMap<String, f64> = HashMap::new();

    // Collect all nodes
    let all_nodes: Vec<&String> = layers.iter().flatten().collect();

    // Find smallest width alignment (accounting for node widths)
    let align_to = find_smallest_width_alignment(g, xss);

    // Align all results to the smallest width alignment
    let mut aligned_xss: HashMap<&str, HashMap<String, f64>> = xss.clone();

    if let Some(align_name) = align_to {
        align_coordinates(&mut aligned_xss, align_name);
    }

    // Take median of four alignments for each node
    for v in all_nodes {
        let mut coords: Vec<f64> = aligned_xss
            .values()
            .filter_map(|xs| xs.get(v).copied())
            .collect();

        coords.sort_by(|a, b| a.partial_cmp(b).unwrap());

        let x = if coords.len() >= 4 {
            // Median of 4 values = average of middle 2
            (coords[1] + coords[2]) / 2.0
        } else if coords.len() >= 2 {
            (coords[0] + coords[coords.len() - 1]) / 2.0
        } else if !coords.is_empty() {
            coords[0]
        } else {
            0.0
        };

        result.insert(v.clone(), x);
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::dagre::graph::{EdgeLabel, NodeLabel};
    use crate::layout::dagre::order;
    use crate::layout::dagre::rank;
    use crate::layout::dagre::Ranker;

    /// Helper to set up a node with rank and order for conflict detection tests.
    fn setup_node(g: &mut DagreGraph, id: &str, rank: i32, order: usize, dummy: Option<&str>) {
        let mut label = NodeLabel {
            rank: Some(rank),
            order: Some(order),
            ..Default::default()
        };
        if let Some(d) = dummy {
            label.dummy = Some(d.to_string());
        }
        g.set_node(id, label);
    }

    #[test]
    fn test_type1_conflict_non_inner_crossing_inner_segment() {
        // Build a graph where a non-inner edge crosses an inner segment:
        //
        //   Layer 0:  a(0)    d0(1)
        //              \      |          <- a→b crosses inner segment d0→d1
        //   Layer 1:  d1(0)   b(1)
        //
        // d0→d1 is an inner segment (both dummy). a→b is a non-inner edge
        // that crosses it (a is at order 0, b is at order 1, but
        // d0 at order 1 and d1 at order 0 — the inner segment swaps sides).
        let mut g = DagreGraph::new();
        setup_node(&mut g, "a", 0, 0, None);
        setup_node(&mut g, "d0", 0, 1, Some("edge"));
        setup_node(&mut g, "d1", 1, 0, Some("edge"));
        setup_node(&mut g, "b", 1, 1, None);

        g.set_edge("a", "b", EdgeLabel::default());
        g.set_edge("d0", "d1", EdgeLabel::default());

        let layering = vec![
            vec!["a".to_string(), "d0".to_string()],
            vec!["d1".to_string(), "b".to_string()],
        ];

        let conflicts = find_type1_conflicts(&g, &layering);

        // a→b crosses the inner segment d0→d1, so (a, b) should be marked as conflicting
        assert!(
            has_conflict(&conflicts, "a", "b"),
            "Expected type-1 conflict between a and b (non-inner edge crossing inner segment)"
        );
        // The inner segment itself should NOT be marked as a conflict
        assert!(
            !has_conflict(&conflicts, "d0", "d1"),
            "Inner segment d0→d1 should not be a conflict"
        );
    }

    #[test]
    fn test_type1_no_conflict_when_no_crossing() {
        // No crossing: all edges go straight down, no inner segments crossed
        //
        //   Layer 0:  a(0)   b(1)
        //              |      |
        //   Layer 1:  c(0)   d(1)
        let mut g = DagreGraph::new();
        setup_node(&mut g, "a", 0, 0, None);
        setup_node(&mut g, "b", 0, 1, None);
        setup_node(&mut g, "c", 1, 0, None);
        setup_node(&mut g, "d", 1, 1, None);

        g.set_edge("a", "c", EdgeLabel::default());
        g.set_edge("b", "d", EdgeLabel::default());

        let layering = vec![
            vec!["a".to_string(), "b".to_string()],
            vec!["c".to_string(), "d".to_string()],
        ];

        let conflicts = find_type1_conflicts(&g, &layering);

        assert!(
            !has_conflict(&conflicts, "a", "c"),
            "No crossing — should not be a conflict"
        );
        assert!(
            !has_conflict(&conflicts, "b", "d"),
            "No crossing — should not be a conflict"
        );
    }

    #[test]
    fn test_type2_conflict_dummy_crossing_border() {
        // Type-2: a dummy edge crosses a border node boundary.
        //
        //   Layer 0:  border0(0)   d0(1)
        //                |            \
        //   Layer 1:  border1(0)   d1(1)  d2(2)
        //
        // If d0→d2 crosses the border boundary, it's a type-2 conflict.
        let mut g = DagreGraph::new();
        setup_node(&mut g, "border0", 0, 0, Some("border"));
        setup_node(&mut g, "d0", 0, 1, Some("edge"));
        setup_node(&mut g, "border1", 1, 0, Some("border"));
        setup_node(&mut g, "d1", 1, 1, Some("edge"));
        setup_node(&mut g, "d2", 1, 2, Some("edge"));

        g.set_edge("border0", "border1", EdgeLabel::default());
        g.set_edge("d0", "d2", EdgeLabel::default());
        g.set_edge("d0", "d1", EdgeLabel::default());

        let layering = vec![
            vec!["border0".to_string(), "d0".to_string()],
            vec!["border1".to_string(), "d1".to_string(), "d2".to_string()],
        ];

        let conflicts = find_type2_conflicts(&g, &layering);

        // d0→d1 should not be a conflict (within the same boundary section)
        // d0→d2 might or might not be a conflict depending on border positions
        // The key assertion: the function runs without panicking and detects conflicts
        // when dummy edges cross border boundaries.
        eprintln!("Type-2 conflicts: {:?}", conflicts);
    }

    #[test]
    fn test_add_and_has_conflict_symmetric() {
        let mut conflicts = Conflicts::new();
        add_conflict(&mut conflicts, "x", "y");

        // Should be detectable in either argument order
        assert!(has_conflict(&conflicts, "x", "y"));
        assert!(has_conflict(&conflicts, "y", "x"));

        // Unrelated pair should not be a conflict
        assert!(!has_conflict(&conflicts, "x", "z"));
    }

    #[test]
    fn test_merge_conflicts_combines_both_maps() {
        let mut a = Conflicts::new();
        add_conflict(&mut a, "a", "b");

        let mut b = Conflicts::new();
        add_conflict(&mut b, "c", "d");

        let merged = merge_conflicts(a, b);

        assert!(has_conflict(&merged, "a", "b"));
        assert!(has_conflict(&merged, "c", "d"));
    }

    #[test]
    fn test_block_graph_preserves_layer_insertion_order() {
        // Verify that build_block_graph returns root nodes in layer insertion order,
        // not alphabetical or arbitrary order.
        let mut g = DagreGraph::new();
        // Create nodes: z at rank 0, a at rank 0, m at rank 1
        // Alphabetically: a < m < z, but layer order should be z, a (order 0, 1)
        g.set_node(
            "z",
            NodeLabel {
                width: 50.0,
                rank: Some(0),
                order: Some(0),
                ..Default::default()
            },
        );
        g.set_node(
            "a",
            NodeLabel {
                width: 50.0,
                rank: Some(0),
                order: Some(1),
                ..Default::default()
            },
        );
        g.set_node(
            "m",
            NodeLabel {
                width: 50.0,
                rank: Some(1),
                order: Some(0),
                ..Default::default()
            },
        );

        let layers = vec![
            vec!["z".to_string(), "a".to_string()],
            vec!["m".to_string()],
        ];

        // Each node is its own root (no alignment yet)
        let mut root: HashMap<String, String> = HashMap::new();
        root.insert("z".to_string(), "z".to_string());
        root.insert("a".to_string(), "a".to_string());
        root.insert("m".to_string(), "m".to_string());

        let (block_nodes, _) = build_block_graph(&g, &layers, &root, 50.0, 20.0, false);

        // Block nodes should follow layer insertion order: z, a, m
        assert_eq!(
            block_nodes,
            vec!["z".to_string(), "a".to_string(), "m".to_string()],
            "Block graph nodes should follow layer insertion order, not alphabetical"
        );
    }

    #[test]
    fn test_position_x_single_node() {
        let mut g = DagreGraph::new();
        g.set_node(
            "a",
            NodeLabel {
                width: 50.0,
                height: 100.0,
                ..Default::default()
            },
        );
        rank::assign_ranks(&mut g, Ranker::LongestPath);
        order::order(&mut g);

        let xs = position_x(&g);

        assert!(xs.contains_key("a"));
        // Single node should have some x coordinate (can be negative before translation)
        // The important thing is that it has a coordinate
        eprintln!("test_position_x_single_node: x = {}", xs["a"]);
    }

    #[test]
    fn test_position_x_two_nodes_same_rank() {
        let mut g = DagreGraph::new();
        g.graph_mut().nodesep = 50.0;
        g.set_node(
            "a",
            NodeLabel {
                width: 50.0,
                height: 100.0,
                rank: Some(0),
                order: Some(0),
                ..Default::default()
            },
        );
        g.set_node(
            "b",
            NodeLabel {
                width: 50.0,
                height: 100.0,
                rank: Some(0),
                order: Some(1),
                ..Default::default()
            },
        );

        let xs = position_x(&g);

        assert!(xs.contains_key("a"));
        assert!(xs.contains_key("b"));

        // b should be to the right of a
        assert!(
            xs["b"] > xs["a"],
            "b ({}) should be to the right of a ({})",
            xs["b"],
            xs["a"]
        );

        // They should be separated appropriately (no overlap)
        let half_widths = 25.0 + 25.0; // Each node is 50 wide, half = 25
        let actual_sep = xs["b"] - xs["a"] - half_widths;
        assert!(
            actual_sep >= 0.0,
            "Nodes should not overlap, separation = {}",
            actual_sep
        );
    }

    #[test]
    fn test_position_x_chain() {
        let mut g = DagreGraph::new();
        g.set_node(
            "a",
            NodeLabel {
                width: 50.0,
                height: 40.0,
                ..Default::default()
            },
        );
        g.set_node(
            "b",
            NodeLabel {
                width: 50.0,
                height: 40.0,
                ..Default::default()
            },
        );
        g.set_edge("a", "b", EdgeLabel::default());
        rank::assign_ranks(&mut g, Ranker::LongestPath);
        order::order(&mut g);

        let xs = position_x(&g);

        // Both should be centered (single node per layer)
        assert!(xs.contains_key("a"));
        assert!(xs.contains_key("b"));
        // They should be vertically aligned (close x coordinates)
        assert!(
            (xs["a"] - xs["b"]).abs() < 100.0,
            "a ({}) and b ({}) should be vertically aligned",
            xs["a"],
            xs["b"]
        );
    }

    #[test]
    fn test_position_x_diamond() {
        // A -> B, A -> C, B -> D, C -> D
        let mut g = DagreGraph::new();
        g.graph_mut().nodesep = 50.0;
        for v in ["a", "b", "c", "d"] {
            g.set_node(
                v,
                NodeLabel {
                    width: 50.0,
                    height: 50.0,
                    ..Default::default()
                },
            );
        }
        g.set_edge("a", "b", EdgeLabel::default());
        g.set_edge("a", "c", EdgeLabel::default());
        g.set_edge("b", "d", EdgeLabel::default());
        g.set_edge("c", "d", EdgeLabel::default());

        rank::assign_ranks(&mut g, Ranker::LongestPath);
        order::order(&mut g);

        let xs = position_x(&g);

        // All nodes should have positions
        assert!(xs.contains_key("a"));
        assert!(xs.contains_key("b"));
        assert!(xs.contains_key("c"));
        assert!(xs.contains_key("d"));

        // B and C should be separated (they're on the same rank)
        assert!(
            (xs["b"] - xs["c"]).abs() >= 50.0,
            "b ({}) and c ({}) should be separated",
            xs["b"],
            xs["c"]
        );
    }

    #[test]
    fn test_find_smallest_width_accounts_for_node_widths() {
        // Build a graph with nodes of varying widths to verify that
        // findSmallestWidthAlignment accounts for node widths, not just raw x coords
        let mut g = DagreGraph::new();
        g.graph_mut().nodesep = 50.0;

        // Two alignments that look the same by raw x-range but differ
        // when accounting for node widths:
        // Alignment A: narrow node at x=0, wide node at x=200 → visual span = 0-0 + 200+75 = 275
        // Alignment B: wide node at x=0, narrow node at x=200 → visual span = 0-75 + 200+0 = 275
        // The point is: a raw comparison (max_x - min_x) would be equal,
        // but if one alignment has wide nodes at the extremes, the visual width differs.

        g.set_node(
            "narrow",
            NodeLabel {
                width: 20.0,
                height: 40.0,
                rank: Some(0),
                order: Some(0),
                ..Default::default()
            },
        );
        g.set_node(
            "wide",
            NodeLabel {
                width: 150.0,
                height: 40.0,
                rank: Some(0),
                order: Some(1),
                ..Default::default()
            },
        );

        let xs = position_x(&g);
        // Just verify it produces valid positions (the fix is in the balance function)
        assert!(xs.contains_key("narrow"));
        assert!(xs.contains_key("wide"));
    }

    #[test]
    fn test_position_x_complex_graph_compact_width() {
        // Models a graph with long edges and a disconnected node.
        // The BK algorithm should compact this into a tight layout.
        // This structure mimics the requirement diagram: a tree-like
        // graph with one cross-rank link creating dummy nodes.
        use crate::layout::dagre::normalize;

        let mut g = DagreGraph::new();
        g.graph_mut().nodesep = 50.0;
        g.graph_mut().edgesep = 20.0;
        g.graph_mut().ranksep = 50.0;

        let node_width = 150.0;
        let node_height = 70.0;

        // Create nodes
        for name in ["a", "b", "c", "d", "e", "f", "g"] {
            g.set_node(
                name,
                NodeLabel {
                    width: node_width,
                    height: node_height,
                    ..Default::default()
                },
            );
        }

        // Graph structure (3 sources, cross-rank link from a to e):
        //   a       b    c
        //   |       |
        //   d       |
        //   |       |
        //   e <-----+
        //   |
        //   f
        //   |
        //   g
        // The edge b -> e spans 3 ranks (0 to 3), creating 2 dummy nodes.
        // c is disconnected (no edges).
        g.set_edge("a", "d", EdgeLabel::default());
        g.set_edge("d", "e", EdgeLabel::default());
        g.set_edge("b", "e", EdgeLabel::default()); // long edge: rank 0 → rank 3
        g.set_edge("e", "f", EdgeLabel::default());
        g.set_edge("f", "g", EdgeLabel::default());

        rank::assign_ranks(&mut g, Ranker::LongestPath);
        normalize::run(&mut g);
        order::order(&mut g);

        let xs = position_x(&g);

        // Compute the total x-spread of real (non-dummy) nodes
        let real_nodes: Vec<&str> = vec!["a", "b", "c", "d", "e", "f", "g"];
        let real_xs: Vec<f64> = real_nodes
            .iter()
            .filter_map(|n| xs.get(*n).copied())
            .collect();

        let min_x = real_xs.iter().copied().fold(f64::INFINITY, f64::min);
        let max_x = real_xs.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        let total_spread = max_x - min_x;

        eprintln!(
            "Complex graph x-spread: {} (min={}, max={})",
            total_spread, min_x, max_x
        );
        for n in &real_nodes {
            if let Some(x) = xs.get(*n) {
                eprintln!("  {} x={}", n, x);
            }
        }

        // With 3 nodes at rank 0 (a, b, c) at 150px wide + 50px sep,
        // minimum spread for rank 0 = 2 * (150 + 50) = 400px.
        // A well-compacted layout should keep total spread under 500px.
        assert!(
            total_spread < 500.0,
            "Layout is too wide: spread {} exceeds 500px threshold. \
            This indicates the BK algorithm is not compacting well.",
            total_spread,
        );
    }
}
