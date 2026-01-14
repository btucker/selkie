//! Utility functions for dagre layout

use super::graph::DagreGraph;

/// Remove empty ranks from the graph
///
/// After nesting graph processing, edge minlen values are multiplied by nodeRankFactor
/// to ensure space for border nodes. This creates many empty intermediate ranks.
/// This function collapses those empty ranks, except at positions divisible by
/// nodeRankFactor (which are reserved for border nodes).
///
/// Reference: dagre.js util.removeEmptyRanks
pub fn remove_empty_ranks(g: &mut DagreGraph) {
    // Get all ranks and find the minimum
    let node_ranks: Vec<i32> = g
        .nodes()
        .iter()
        .filter_map(|v| g.node(v).and_then(|n| n.rank))
        .collect();

    if node_ranks.is_empty() {
        return;
    }

    let offset = *node_ranks.iter().min().unwrap();

    // Build layers (sparse - some indices may be empty)
    let max_rank = node_ranks.iter().max().unwrap() - offset;
    let mut layers: Vec<Vec<String>> = vec![Vec::new(); (max_rank + 1) as usize];

    let all_nodes = g.nodes().to_vec();
    for v in &all_nodes {
        if let Some(node) = g.node(v) {
            if let Some(rank) = node.rank {
                let adjusted_rank = (rank - offset) as usize;
                layers[adjusted_rank].push(v.to_string());
            }
        }
    }

    // Remove empty ranks, but keep those at positions divisible by nodeRankFactor
    let node_rank_factor = g.graph().node_rank_factor.unwrap_or(1) as usize;
    let mut delta: i32 = 0;

    for (i, layer) in layers.iter().enumerate() {
        if layer.is_empty() && (node_rank_factor == 0 || i % node_rank_factor != 0) {
            // Empty rank not at a border position - remove it
            delta -= 1;
        } else if !layer.is_empty() && delta != 0 {
            // Non-empty rank - adjust by delta
            for v in layer {
                if let Some(node) = g.node_mut(v) {
                    if let Some(rank) = node.rank.as_mut() {
                        *rank += delta;
                    }
                }
            }
        }
    }
}

/// Build a 2D matrix of nodes organized by layer (rank) and order.
///
/// Returns a Vec where each element is a Vec of node IDs at that rank,
/// sorted by their order within the rank.
pub fn build_layer_matrix(g: &DagreGraph) -> Vec<Vec<String>> {
    // Find the maximum rank
    let max_rank = g
        .nodes()
        .iter()
        .filter_map(|v| g.node(v).and_then(|n| n.rank))
        .max()
        .unwrap_or(-1);

    if max_rank < 0 {
        return vec![];
    }

    // Initialize layers
    let mut layers: Vec<Vec<(usize, String)>> =
        (0..=(max_rank as usize)).map(|_| Vec::new()).collect();

    // Place nodes in their layers
    for v in g.nodes() {
        if let Some(node) = g.node(v) {
            if let Some(rank) = node.rank {
                let order = node.order.unwrap_or(0);
                layers[rank as usize].push((order, v.clone()));
            }
        }
    }

    // Sort each layer by order and extract just the node IDs
    layers
        .into_iter()
        .map(|mut layer| {
            layer.sort_by_key(|(order, _)| *order);
            layer.into_iter().map(|(_, v)| v).collect()
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::dagre::graph::NodeLabel;

    #[test]
    fn test_build_layer_matrix_empty_graph() {
        let g = DagreGraph::new();
        let layers = build_layer_matrix(&g);
        assert!(layers.is_empty());
    }

    #[test]
    fn test_build_layer_matrix_single_layer() {
        let mut g = DagreGraph::new();
        g.set_node(
            "a",
            NodeLabel {
                rank: Some(0),
                order: Some(0),
                ..Default::default()
            },
        );
        g.set_node(
            "b",
            NodeLabel {
                rank: Some(0),
                order: Some(1),
                ..Default::default()
            },
        );

        let layers = build_layer_matrix(&g);

        assert_eq!(layers.len(), 1);
        assert_eq!(layers[0], vec!["a", "b"]);
    }

    #[test]
    fn test_build_layer_matrix_multiple_layers() {
        let mut g = DagreGraph::new();
        g.set_node(
            "a",
            NodeLabel {
                rank: Some(0),
                order: Some(0),
                ..Default::default()
            },
        );
        g.set_node(
            "b",
            NodeLabel {
                rank: Some(1),
                order: Some(0),
                ..Default::default()
            },
        );
        g.set_node(
            "c",
            NodeLabel {
                rank: Some(1),
                order: Some(1),
                ..Default::default()
            },
        );

        let layers = build_layer_matrix(&g);

        assert_eq!(layers.len(), 2);
        assert_eq!(layers[0], vec!["a"]);
        assert_eq!(layers[1], vec!["b", "c"]);
    }

    #[test]
    fn test_build_layer_matrix_sorts_by_order() {
        let mut g = DagreGraph::new();
        // Add in non-sorted order
        g.set_node(
            "c",
            NodeLabel {
                rank: Some(0),
                order: Some(2),
                ..Default::default()
            },
        );
        g.set_node(
            "a",
            NodeLabel {
                rank: Some(0),
                order: Some(0),
                ..Default::default()
            },
        );
        g.set_node(
            "b",
            NodeLabel {
                rank: Some(0),
                order: Some(1),
                ..Default::default()
            },
        );

        let layers = build_layer_matrix(&g);

        assert_eq!(layers[0], vec!["a", "b", "c"]);
    }
}
