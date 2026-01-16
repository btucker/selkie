//! Barycenter heuristic for crossing minimization
//!
//! Calculates the barycenter (weighted average position) for nodes based on
//! the positions of their neighbors.

use crate::layout::dagre::graph::DagreGraph;

/// Result of barycenter calculation for a node
#[derive(Debug, Clone)]
pub struct BarycenterEntry {
    pub v: String,
    pub barycenter: Option<f64>,
    pub weight: f64,
    /// Original index for stable sorting tie-breaking
    pub i: usize,
}

/// Calculate barycenters for movable nodes based on in-edges
///
/// For each node, the barycenter is the weighted average of the order positions
/// of its predecessors. A tiny offset based on array position is added to ensure
/// stable ordering when barycenters are equal (e.g., fork targets sharing one predecessor).
pub fn barycenter(g: &DagreGraph, movable: &[String]) -> Vec<BarycenterEntry> {
    // Use a small epsilon to create tie-breaking without affecting actual ordering
    // The epsilon is small enough that it won't change the ordering of nodes
    // with genuinely different barycenters (which differ by at least 1.0)
    let epsilon = 1e-6;

    movable
        .iter()
        .enumerate()
        .map(|(i, v)| {
            let in_edges = g.in_edges(v);
            if in_edges.is_empty() {
                return BarycenterEntry {
                    v: v.clone(),
                    barycenter: None,
                    weight: 0.0,
                    i,
                };
            }

            let mut sum = 0.0;
            let mut weight = 0.0;

            for e in &in_edges {
                let edge_weight = g
                    .edge_by_key(e)
                    .map(|edge| edge.weight as f64)
                    .unwrap_or(1.0);

                if let Some(node_u) = g.node(&e.v) {
                    if let Some(order) = node_u.order {
                        sum += edge_weight * (order as f64);
                        weight += edge_weight;
                    }
                }
            }

            if weight > 0.0 {
                // Add tiny offset based on array position to ensure stable ordering
                // This preserves edge definition order when barycenters are equal
                let bc = sum / weight + (i as f64) * epsilon;
                BarycenterEntry {
                    v: v.clone(),
                    barycenter: Some(bc),
                    weight,
                    i,
                }
            } else {
                BarycenterEntry {
                    v: v.clone(),
                    barycenter: None,
                    weight: 0.0,
                    i,
                }
            }
        })
        .collect()
}

/// Calculate barycenters for movable nodes based on out-edges
///
/// For each node, the barycenter is the weighted average of the order positions
/// of its successors. A tiny offset based on array position is added to ensure
/// stable ordering when barycenters are equal.
pub fn barycenter_down(g: &DagreGraph, movable: &[String]) -> Vec<BarycenterEntry> {
    let epsilon = 1e-6;

    movable
        .iter()
        .enumerate()
        .map(|(i, v)| {
            let out_edges = g.out_edges(v);
            if out_edges.is_empty() {
                return BarycenterEntry {
                    v: v.clone(),
                    barycenter: None,
                    weight: 0.0,
                    i,
                };
            }

            let mut sum = 0.0;
            let mut weight = 0.0;

            for e in &out_edges {
                let edge_weight = g
                    .edge_by_key(e)
                    .map(|edge| edge.weight as f64)
                    .unwrap_or(1.0);

                if let Some(node_w) = g.node(&e.w) {
                    if let Some(order) = node_w.order {
                        sum += edge_weight * (order as f64);
                        weight += edge_weight;
                    }
                }
            }

            if weight > 0.0 {
                // Add tiny offset based on array position to ensure stable ordering
                let bc = sum / weight + (i as f64) * epsilon;
                BarycenterEntry {
                    v: v.clone(),
                    barycenter: Some(bc),
                    weight,
                    i,
                }
            } else {
                BarycenterEntry {
                    v: v.clone(),
                    barycenter: None,
                    weight: 0.0,
                    i,
                }
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::dagre::graph::{EdgeLabel, NodeLabel};

    #[test]
    fn test_barycenter_single_predecessor() {
        let mut g = DagreGraph::new();
        g.set_node(
            "a",
            NodeLabel {
                order: Some(0),
                ..Default::default()
            },
        );
        g.set_node(
            "b",
            NodeLabel {
                order: Some(2),
                ..Default::default()
            },
        );
        g.set_node("c", NodeLabel::default());
        g.set_edge("a", "c", EdgeLabel::default());

        let result = barycenter(&g, &["c".to_string()]);

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].v, "c");
        assert_eq!(result[0].barycenter, Some(0.0));
    }

    #[test]
    fn test_barycenter_multiple_predecessors() {
        let mut g = DagreGraph::new();
        g.set_node(
            "a",
            NodeLabel {
                order: Some(0),
                ..Default::default()
            },
        );
        g.set_node(
            "b",
            NodeLabel {
                order: Some(2),
                ..Default::default()
            },
        );
        g.set_node("c", NodeLabel::default());
        g.set_edge("a", "c", EdgeLabel::default());
        g.set_edge("b", "c", EdgeLabel::default());

        let result = barycenter(&g, &["c".to_string()]);

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].v, "c");
        // (0 * 1 + 2 * 1) / (1 + 1) = 1.0
        assert_eq!(result[0].barycenter, Some(1.0));
    }

    #[test]
    fn test_barycenter_weighted() {
        let mut g = DagreGraph::new();
        g.set_node(
            "a",
            NodeLabel {
                order: Some(0),
                ..Default::default()
            },
        );
        g.set_node(
            "b",
            NodeLabel {
                order: Some(2),
                ..Default::default()
            },
        );
        g.set_node("c", NodeLabel::default());
        g.set_edge(
            "a",
            "c",
            EdgeLabel {
                weight: 3,
                ..Default::default()
            },
        );
        g.set_edge(
            "b",
            "c",
            EdgeLabel {
                weight: 1,
                ..Default::default()
            },
        );

        let result = barycenter(&g, &["c".to_string()]);

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].v, "c");
        // (0 * 3 + 2 * 1) / (3 + 1) = 0.5
        assert_eq!(result[0].barycenter, Some(0.5));
    }

    #[test]
    fn test_barycenter_no_predecessors() {
        let mut g = DagreGraph::new();
        g.set_node("a", NodeLabel::default());

        let result = barycenter(&g, &["a".to_string()]);

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].v, "a");
        assert_eq!(result[0].barycenter, None);
    }
}
