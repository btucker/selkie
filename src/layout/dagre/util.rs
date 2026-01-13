//! Utility functions for dagre layout

use super::graph::DagreGraph;

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
