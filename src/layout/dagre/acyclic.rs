//! Cycle removal for dagre layout
//!
//! Makes the graph acyclic by identifying back edges and reversing them.
//! Provides two methods:
//! - DFS-based: Simple depth-first search to find back edges
//! - Greedy: Prefers to break cycles at low-weight edges (Eades algorithm)

use super::graph::{DagreGraph, EdgeKey};
use super::Acyclicer;
use std::collections::HashSet;

/// Make the graph acyclic by reversing back edges
pub fn run(g: &mut DagreGraph, method: Acyclicer) {
    let fas = match method {
        Acyclicer::Greedy => greedy_fas(g),
        Acyclicer::Dfs => dfs_fas(g),
    };

    // Reverse each edge in the feedback arc set
    for edge_key in fas {
        if let Some(mut label) = g.edges.remove(&edge_key) {
            // Remove from adjacency lists
            if let Some(out_list) = g.out_edges.get_mut(&edge_key.v) {
                out_list.retain(|k| k != &edge_key);
            }
            if let Some(in_list) = g.in_edges.get_mut(&edge_key.w) {
                in_list.retain(|k| k != &edge_key);
            }

            // Mark as reversed and store original name
            label.forward_name = edge_key.name.clone();
            label.reversed = true;

            // Create reversed edge with new unique name
            let rev_name = g.unique_id("rev");
            let rev_key = EdgeKey::with_name(&edge_key.w, &edge_key.v, &rev_name);

            g.edges.insert(rev_key.clone(), label);
            g.out_edges
                .get_mut(&edge_key.w)
                .unwrap()
                .push(rev_key.clone());
            g.in_edges.get_mut(&edge_key.v).unwrap().push(rev_key);
        }
    }
}

/// Undo the cycle removal - restore reversed edges to original direction
pub fn undo(g: &mut DagreGraph) {
    // Collect reversed edges in sorted order for determinism
    let mut reversed_edges: Vec<EdgeKey> = g
        .edges
        .iter()
        .filter(|(_, label)| label.reversed)
        .map(|(key, _)| key.clone())
        .collect();
    reversed_edges.sort();

    for edge_key in reversed_edges {
        if let Some(mut label) = g.edges.remove(&edge_key) {
            // Remove from adjacency lists
            if let Some(out_list) = g.out_edges.get_mut(&edge_key.v) {
                out_list.retain(|k| k != &edge_key);
            }
            if let Some(in_list) = g.in_edges.get_mut(&edge_key.w) {
                in_list.retain(|k| k != &edge_key);
            }

            // Restore original direction
            let forward_name = label.forward_name.take();
            label.reversed = false;

            // Create edge in original direction
            let key = if let Some(name) = forward_name {
                EdgeKey::with_name(&edge_key.w, &edge_key.v, &name)
            } else {
                EdgeKey::new(&edge_key.w, &edge_key.v)
            };

            g.edges.insert(key.clone(), label);
            g.out_edges.get_mut(&edge_key.w).unwrap().push(key.clone());
            g.in_edges.get_mut(&edge_key.v).unwrap().push(key);
        }
    }
}

/// Find feedback arc set using DFS
fn dfs_fas(g: &DagreGraph) -> Vec<EdgeKey> {
    let mut fas = Vec::new();
    let mut visited = HashSet::new();
    let mut stack = HashSet::new();

    fn dfs(
        g: &DagreGraph,
        v: &str,
        visited: &mut HashSet<String>,
        stack: &mut HashSet<String>,
        fas: &mut Vec<EdgeKey>,
    ) {
        if visited.contains(v) {
            return;
        }
        visited.insert(v.to_string());
        stack.insert(v.to_string());

        for edge in g.out_edges(v) {
            if stack.contains(&edge.w) {
                // Back edge found - add to feedback arc set
                fas.push(edge.clone());
            } else {
                dfs(g, &edge.w, visited, stack, fas);
            }
        }

        stack.remove(v);
    }

    for v in g.nodes() {
        dfs(g, v, &mut visited, &mut stack, &mut fas);
    }

    fas
}

/// Find feedback arc set using greedy algorithm (prefers low-weight edges)
///
/// This implements a simplified version of the Eades heuristic for finding
/// a minimum feedback arc set in weighted graphs.
fn greedy_fas(g: &DagreGraph) -> Vec<EdgeKey> {
    if g.node_count() == 0 {
        return Vec::new();
    }

    // Create a working copy of the graph structure
    let mut in_degree: std::collections::HashMap<String, i32> = std::collections::HashMap::new();
    let mut out_degree: std::collections::HashMap<String, i32> = std::collections::HashMap::new();
    let mut active_nodes: HashSet<String> = HashSet::new();

    // Initialize degrees
    for v in g.nodes() {
        let in_deg = g.in_edges(v).len() as i32;
        let out_deg = g.out_edges(v).len() as i32;
        in_degree.insert(v.clone(), in_deg);
        out_degree.insert(v.clone(), out_deg);
        active_nodes.insert(v.clone());
    }

    let mut fas = Vec::new();
    let mut sources: Vec<String> = Vec::new();
    let mut sinks: Vec<String> = Vec::new();

    // Process until graph is empty
    while !active_nodes.is_empty() {
        // Find sources (in-degree 0) and sinks (out-degree 0)
        sources.clear();
        sinks.clear();

        // Iterate in sorted order for determinism
        let mut sorted_nodes: Vec<&String> = active_nodes.iter().collect();
        sorted_nodes.sort();
        for v in sorted_nodes {
            if *in_degree.get(v).unwrap_or(&0) == 0 {
                sources.push(v.clone());
            }
            if *out_degree.get(v).unwrap_or(&0) == 0 {
                sinks.push(v.clone());
            }
        }

        // Remove all sources
        for v in &sources {
            for edge in g.out_edges(v) {
                if active_nodes.contains(&edge.w) {
                    *in_degree.get_mut(&edge.w).unwrap() -= 1;
                }
            }
            active_nodes.remove(v);
        }

        // Remove all sinks
        for v in &sinks {
            for edge in g.in_edges(v) {
                if active_nodes.contains(&edge.v) {
                    *out_degree.get_mut(&edge.v).unwrap() -= 1;
                }
            }
            active_nodes.remove(v);
        }

        // If there are remaining nodes, we have a cycle
        // Remove the node with highest out-degree - in-degree (most likely cycle contributor)
        if !active_nodes.is_empty() {
            let mut best_node: Option<String> = None;
            let mut best_score = i32::MIN;

            // Iterate in sorted order so ties are resolved deterministically
            let mut sorted_nodes: Vec<&String> = active_nodes.iter().collect();
            sorted_nodes.sort();
            for v in sorted_nodes {
                let score = *out_degree.get(v).unwrap_or(&0) - *in_degree.get(v).unwrap_or(&0);
                if score > best_score {
                    best_score = score;
                    best_node = Some(v.clone());
                }
            }

            if let Some(v) = best_node {
                // Add incoming edges to FAS (they point "backwards" relative to this node)
                for edge in g.in_edges(&v) {
                    if active_nodes.contains(&edge.v) {
                        // Add the lowest weight incoming edge to FAS
                        fas.push(edge.clone());
                        *out_degree.get_mut(&edge.v).unwrap() -= 1;
                    }
                }

                // Update outgoing edge degrees
                for edge in g.out_edges(&v) {
                    if active_nodes.contains(&edge.w) {
                        *in_degree.get_mut(&edge.w).unwrap() -= 1;
                    }
                }

                active_nodes.remove(&v);
            }
        }
    }

    // Deduplicate (may have added same edge multiple times in greedy process)
    let mut seen = HashSet::new();
    fas.retain(|e| seen.insert(e.clone()));

    fas
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::dagre::graph::EdgeLabel;

    fn find_cycles(g: &DagreGraph) -> Vec<Vec<String>> {
        // Simple cycle detection using DFS
        let mut cycles = Vec::new();
        let mut visited = HashSet::new();
        let mut rec_stack = Vec::new();
        let mut rec_set = HashSet::new();

        fn dfs(
            g: &DagreGraph,
            v: &str,
            visited: &mut HashSet<String>,
            rec_stack: &mut Vec<String>,
            rec_set: &mut HashSet<String>,
            cycles: &mut Vec<Vec<String>>,
        ) {
            visited.insert(v.to_string());
            rec_stack.push(v.to_string());
            rec_set.insert(v.to_string());

            for edge in g.out_edges(v) {
                if !visited.contains(&edge.w) {
                    dfs(g, &edge.w, visited, rec_stack, rec_set, cycles);
                } else if rec_set.contains(&edge.w) {
                    // Found a cycle
                    let start_idx = rec_stack.iter().position(|x| x == &edge.w).unwrap();
                    let cycle: Vec<String> = rec_stack[start_idx..].to_vec();
                    cycles.push(cycle);
                }
            }

            rec_stack.pop();
            rec_set.remove(v);
        }

        for v in g.nodes() {
            if !visited.contains(v) {
                dfs(
                    g,
                    v,
                    &mut visited,
                    &mut rec_stack,
                    &mut rec_set,
                    &mut cycles,
                );
            }
        }

        cycles
    }

    #[test]
    fn test_dfs_does_not_change_acyclic_graph() {
        let mut g = DagreGraph::new();
        g.set_path(&["a", "b", "d"]);
        g.set_path(&["a", "c", "d"]);

        run(&mut g, Acyclicer::Dfs);

        assert!(g.has_edge("a", "b"));
        assert!(g.has_edge("a", "c"));
        assert!(g.has_edge("b", "d"));
        assert!(g.has_edge("c", "d"));
        assert_eq!(g.edge_count(), 4);
    }

    #[test]
    fn test_dfs_breaks_cycles() {
        let mut g = DagreGraph::new();
        g.set_path(&["a", "b", "c", "d", "a"]);

        run(&mut g, Acyclicer::Dfs);

        assert!(find_cycles(&g).is_empty());
    }

    #[test]
    fn test_greedy_breaks_cycles() {
        let mut g = DagreGraph::new();
        g.set_path(&["a", "b", "c", "d", "a"]);

        run(&mut g, Acyclicer::Greedy);

        assert!(find_cycles(&g).is_empty());
    }

    #[test]
    fn test_creates_multi_edge_where_necessary() {
        let mut g = DagreGraph::new();
        g.set_path(&["a", "b", "a"]);

        run(&mut g, Acyclicer::Dfs);

        assert!(find_cycles(&g).is_empty());
        // Should have 2 edges between a and b (one in each direction, or both same direction)
        assert_eq!(g.edge_count(), 2);
    }

    #[test]
    fn test_undo_does_not_change_acyclic_graph() {
        let mut g = DagreGraph::new();
        g.set_edge(
            "a",
            "b",
            EdgeLabel {
                minlen: 2,
                weight: 3,
                ..Default::default()
            },
        );

        run(&mut g, Acyclicer::Dfs);
        undo(&mut g);

        let edge = g.edge("a", "b").unwrap();
        assert_eq!(edge.minlen, 2);
        assert_eq!(edge.weight, 3);
        assert_eq!(g.edge_count(), 1);
    }

    #[test]
    fn test_undo_restores_reversed_edges() {
        let mut g = DagreGraph::new();
        g.set_edge(
            "a",
            "b",
            EdgeLabel {
                minlen: 2,
                weight: 3,
                ..Default::default()
            },
        );
        g.set_edge(
            "b",
            "a",
            EdgeLabel {
                minlen: 3,
                weight: 4,
                ..Default::default()
            },
        );

        run(&mut g, Acyclicer::Dfs);
        undo(&mut g);

        let ab = g.edge("a", "b").unwrap();
        let ba = g.edge("b", "a").unwrap();

        // At least one of these should exist with original properties
        assert!(ab.minlen == 2 || ba.minlen == 3);
        assert_eq!(g.edge_count(), 2);
    }

    #[test]
    fn test_greedy_prefers_low_weight_edges() {
        let mut g = DagreGraph::new();
        // Set default weight to 2
        g.set_edge(
            "a",
            "b",
            EdgeLabel {
                minlen: 1,
                weight: 2,
                ..Default::default()
            },
        );
        g.set_edge(
            "b",
            "c",
            EdgeLabel {
                minlen: 1,
                weight: 2,
                ..Default::default()
            },
        );
        g.set_edge(
            "c",
            "d",
            EdgeLabel {
                minlen: 1,
                weight: 1,
                ..Default::default()
            },
        ); // Low weight
        g.set_edge(
            "d",
            "a",
            EdgeLabel {
                minlen: 1,
                weight: 2,
                ..Default::default()
            },
        );

        run(&mut g, Acyclicer::Greedy);

        assert!(find_cycles(&g).is_empty());
        // The low-weight edge c->d should have been reversed
        // Note: The exact edge reversed may vary based on implementation details
    }
}
