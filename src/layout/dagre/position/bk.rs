//! Brandes-Köpf x-coordinate assignment
//!
//! Based on "Fast and Simple Horizontal Coordinate Assignment" by Brandes and Köpf.
//!
//! The algorithm runs four passes (up-left, up-right, down-left, down-right) and
//! balances them by taking the median of the four x-coordinates for each node.

use crate::layout::dagre::graph::DagreGraph;
use std::collections::HashMap;

/// Assign x coordinates to all nodes using Brandes-Köpf algorithm
pub fn position_x(g: &DagreGraph) -> HashMap<String, f64> {
    let nodesep = g.graph().nodesep;
    let edgesep = g.graph().edgesep.max(10.0); // Minimum edge separation

    // Build layer matrix
    let layers = build_layer_matrix(g);
    if layers.is_empty() {
        return HashMap::new();
    }

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

            // Build alignment
            let (root, align) = vertical_alignment(g, &aligned_layers, vert == "u");

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
    balance(&xss, &layers)
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

/// Vertical alignment: align nodes with their median neighbors
fn vertical_alignment(
    g: &DagreGraph,
    layers: &[Vec<String>],
    use_predecessors: bool,
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
        let mut prev_idx: Option<usize> = None;

        for v in layer {
            // Get neighbors (predecessors or successors)
            let neighbors: Vec<String> = if use_predecessors {
                g.predecessors(v).into_iter().cloned().collect()
            } else {
                g.successors(v).into_iter().cloned().collect()
            };

            if neighbors.is_empty() {
                continue;
            }

            // Sort neighbors by their position
            let mut sorted_neighbors: Vec<_> = neighbors
                .into_iter()
                .filter_map(|n| pos.get(&n).map(|&p| (n, p)))
                .collect();
            sorted_neighbors.sort_by_key(|(_, p)| *p);

            // Find median neighbor(s)
            let len = sorted_neighbors.len();
            let median_low = (len - 1) / 2;
            let median_high = len / 2;

            for idx in median_low..=median_high {
                if let Some((w, w_pos)) = sorted_neighbors.get(idx) {
                    // Check if we can align with this neighbor
                    if align.get(v) == Some(v) {
                        let can_align = prev_idx.map(|pi| *w_pos > pi).unwrap_or(true);

                        if can_align {
                            // Align v with w
                            align.insert(w.clone(), v.clone());
                            let r = root.get(w).cloned().unwrap_or_else(|| w.clone());
                            root.insert(v.clone(), r.clone());
                            align.insert(v.clone(), r);
                            prev_idx = Some(*w_pos);
                        }
                    }
                }
            }
        }
    }

    (root, align)
}

/// Horizontal compaction: assign x coordinates based on blocks
fn horizontal_compaction(
    g: &DagreGraph,
    layers: &[Vec<String>],
    root: &HashMap<String, String>,
    _align: &HashMap<String, String>,
    nodesep: f64,
    edgesep: f64,
    _reverse_sep: bool,
) -> HashMap<String, f64> {
    let mut xs: HashMap<String, f64> = HashMap::new();

    // First pass: assign initial x coordinates based on position in layer
    for layer in layers {
        let mut x = 0.0;
        for v in layer {
            let width = g.node(v).map(|n| n.width).unwrap_or(0.0);
            let sep = if g.node(v).map(|n| n.dummy.is_some()).unwrap_or(false) {
                edgesep
            } else {
                nodesep
            };

            // Get the root of this node's block
            let r = root.get(v).cloned().unwrap_or_else(|| v.clone());

            // If root hasn't been positioned yet, position it
            if !xs.contains_key(&r) {
                xs.insert(r.clone(), x + width / 2.0);
            }

            // Assign this node the same x as its root
            xs.insert(v.clone(), *xs.get(&r).unwrap());

            x = xs.get(v).copied().unwrap_or(0.0) + width / 2.0 + sep;
        }
    }

    // Second pass: propagate x coordinates from roots to aligned nodes
    for (v, r) in root {
        if let Some(&rx) = xs.get(r) {
            xs.insert(v.clone(), rx);
        }
    }

    // Third pass: resolve overlaps within each layer
    for layer in layers {
        resolve_overlaps_ordered(g, layer, &mut xs, nodesep, edgesep);
    }

    xs
}

/// Resolve overlaps in a layer while maintaining order
fn resolve_overlaps_ordered(
    g: &DagreGraph,
    layer: &[String],
    xs: &mut HashMap<String, f64>,
    nodesep: f64,
    edgesep: f64,
) {
    if layer.len() < 2 {
        return;
    }

    // Get current positions and widths
    let mut positions: Vec<(String, f64, f64)> = layer
        .iter()
        .filter_map(|v| {
            let x = xs.get(v)?;
            let width = g.node(v).map(|n| n.width).unwrap_or(0.0);
            Some((v.clone(), *x, width))
        })
        .collect();

    // Sort by x position
    positions.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());

    // Resolve overlaps left to right
    for i in 1..positions.len() {
        let prev_x = positions[i - 1].1;
        let prev_width = positions[i - 1].2;
        let curr_width = positions[i].2;

        let prev_is_dummy = g
            .node(&positions[i - 1].0)
            .map(|n| n.dummy.is_some())
            .unwrap_or(false);
        let curr_is_dummy = g
            .node(&positions[i].0)
            .map(|n| n.dummy.is_some())
            .unwrap_or(false);

        let sep = if prev_is_dummy || curr_is_dummy {
            edgesep
        } else {
            nodesep
        };
        let min_x = prev_x + prev_width / 2.0 + sep + curr_width / 2.0;

        if positions[i].1 < min_x {
            positions[i].1 = min_x;
            xs.insert(positions[i].0.clone(), min_x);
        }
    }
}

/// Balance the four alignments by taking the median x for each node
fn balance(
    xss: &HashMap<&str, HashMap<String, f64>>,
    layers: &[Vec<String>],
) -> HashMap<String, f64> {
    let mut result: HashMap<String, f64> = HashMap::new();

    // Collect all nodes
    let all_nodes: Vec<&String> = layers.iter().flatten().collect();

    // Find smallest width alignment for centering
    let mut min_width = f64::MAX;
    let mut align_to: Option<&str> = None;

    for (&name, xs) in xss {
        let (min_x, max_x) = xs.values().fold((f64::MAX, f64::MIN), |(min, max), &x| {
            (min.min(x), max.max(x))
        });
        let width = max_x - min_x;
        if width < min_width {
            min_width = width;
            align_to = Some(name);
        }
    }

    // Align all results to the smallest width alignment
    let mut aligned_xss: HashMap<&str, HashMap<String, f64>> = xss.clone();

    if let Some(align_name) = align_to {
        let align_xs = &xss[align_name];
        let align_min: f64 = align_xs.values().copied().fold(f64::MAX, f64::min);
        let align_max: f64 = align_xs.values().copied().fold(f64::MIN, f64::max);

        for (&name, xs) in aligned_xss.iter_mut() {
            if name == align_name {
                continue;
            }

            let xs_min: f64 = xs.values().copied().fold(f64::MAX, f64::min);
            let xs_max: f64 = xs.values().copied().fold(f64::MIN, f64::max);

            // Align based on direction
            let delta = if name.ends_with('l') {
                align_min - xs_min
            } else {
                align_max - xs_max
            };

            for x in xs.values_mut() {
                *x += delta;
            }
        }
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
}
