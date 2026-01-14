//! Resolve conflicts between barycenters and constraint graph
//!
//! When barycenters would violate ordering constraints between sibling subgraphs,
//! this module coalesces conflicting nodes into groups that respect the constraints.
//!
//! Based on Forster's algorithm: "A Fast and Simple Heuristic for Constrained
//! Two-Level Crossing Reduction"

use std::collections::{HashMap, HashSet};

/// A simple directed graph for tracking ordering constraints between sibling subgraphs
#[derive(Debug, Clone, Default)]
pub struct ConstraintGraph {
    edges: HashSet<(String, String)>,
}

impl ConstraintGraph {
    pub fn new() -> Self {
        Self {
            edges: HashSet::new(),
        }
    }

    /// Add an ordering constraint: v must come before w
    pub fn set_edge(&mut self, v: &str, w: &str) {
        self.edges.insert((v.to_string(), w.to_string()));
    }

    /// Get all edges in the constraint graph
    pub fn edges(&self) -> impl Iterator<Item = (&str, &str)> {
        self.edges.iter().map(|(v, w)| (v.as_str(), w.as_str()))
    }
}

/// Entry with additional tracking information for the resolve algorithm
#[derive(Debug)]
struct MappedEntry {
    indegree: usize,
    incoming: Vec<String>, // Nodes pointing to this entry
    outgoing: Vec<String>, // Nodes this entry points to
    vs: Vec<String>,       // Nodes in this entry
    i: usize,              // Original index
    barycenter: Option<f64>,
    weight: f64,
    merged: bool,
}

/// Result of resolving conflicts
#[derive(Debug, Clone)]
pub struct ResolvedEntry {
    pub vs: Vec<String>,
    /// Original index for stable sorting tie-breaking
    pub i: usize,
    pub barycenter: Option<f64>,
    pub weight: f64,
}

/// Resolve conflicts between barycenter ordering and constraint graph
///
/// If barycenters would place nodes in an order that violates the constraint graph,
/// those nodes are coalesced into a single entry that respects the constraint.
pub fn resolve_conflicts(
    entries: Vec<super::BarycenterEntry>,
    cg: &ConstraintGraph,
) -> Vec<ResolvedEntry> {
    // Create mapped entries with tracking information
    let mut mapped: HashMap<String, MappedEntry> = HashMap::new();

    for (i, entry) in entries.iter().enumerate() {
        mapped.insert(
            entry.v.clone(),
            MappedEntry {
                indegree: 0,
                incoming: Vec::new(),
                outgoing: Vec::new(),
                vs: vec![entry.v.clone()],
                i,
                barycenter: entry.barycenter,
                weight: entry.weight,
                merged: false,
            },
        );
    }

    // Process constraint graph edges
    for (v, w) in cg.edges() {
        if mapped.contains_key(v) && mapped.contains_key(w) {
            // Increment indegree for w
            if let Some(entry_w) = mapped.get_mut(w) {
                entry_w.indegree += 1;
                entry_w.incoming.push(v.to_string());
            }
            // Add to outgoing for v
            if let Some(entry_v) = mapped.get_mut(v) {
                entry_v.outgoing.push(w.to_string());
            }
        }
    }

    // Find source set (entries with indegree 0)
    let source_set: Vec<String> = mapped
        .iter()
        .filter(|(_, entry)| entry.indegree == 0)
        .map(|(v, _)| v.clone())
        .collect();

    do_resolve_conflicts(&mut mapped, source_set)
}

fn do_resolve_conflicts(
    mapped: &mut HashMap<String, MappedEntry>,
    mut source_set: Vec<String>,
) -> Vec<ResolvedEntry> {
    let mut results: Vec<String> = Vec::new();

    while let Some(v) = source_set.pop() {
        results.push(v.clone());

        // Handle incoming edges - merge if needed
        let incoming: Vec<String> = mapped
            .get(&v)
            .map(|e| e.incoming.clone())
            .unwrap_or_default();

        for u in incoming.into_iter().rev() {
            if mapped.get(&u).map(|e| e.merged).unwrap_or(true) {
                continue;
            }

            let u_bc = mapped.get(&u).and_then(|e| e.barycenter);
            let v_bc = mapped.get(&v).and_then(|e| e.barycenter);

            // Merge if u has no barycenter, v has no barycenter, or u's barycenter >= v's
            let should_merge = u_bc.is_none() || v_bc.is_none() || u_bc.unwrap() >= v_bc.unwrap();

            if should_merge {
                merge_entries(mapped, &v, &u);
            }
        }

        // Handle outgoing edges - decrement indegree and add to source set if 0
        let outgoing: Vec<String> = mapped
            .get(&v)
            .map(|e| e.outgoing.clone())
            .unwrap_or_default();

        for w in outgoing {
            // Add v to w's incoming for potential future merging
            if let Some(entry_w) = mapped.get_mut(&w) {
                entry_w.incoming.push(v.clone());
                entry_w.indegree = entry_w.indegree.saturating_sub(1);
                if entry_w.indegree == 0 {
                    source_set.push(w);
                }
            }
        }
    }

    // Build results from non-merged entries
    results
        .into_iter()
        .filter_map(|v| {
            let entry = mapped.get(&v)?;
            if entry.merged {
                return None;
            }
            Some(ResolvedEntry {
                vs: entry.vs.clone(),
                i: entry.i,
                barycenter: entry.barycenter,
                weight: entry.weight,
            })
        })
        .collect()
}

fn merge_entries(mapped: &mut HashMap<String, MappedEntry>, target: &str, source: &str) {
    let source_data = match mapped.get(source) {
        Some(entry) => (entry.vs.clone(), entry.barycenter, entry.weight, entry.i),
        None => return,
    };

    let (source_vs, source_bc, source_weight, source_i) = source_data;

    if let Some(target_entry) = mapped.get_mut(target) {
        // Merge barycenters
        let mut sum = 0.0;
        let mut weight = 0.0;

        if target_entry.weight > 0.0 {
            if let Some(bc) = target_entry.barycenter {
                sum += bc * target_entry.weight;
                weight += target_entry.weight;
            }
        }

        if source_weight > 0.0 {
            if let Some(bc) = source_bc {
                sum += bc * source_weight;
                weight += source_weight;
            }
        }

        // Prepend source vs to target vs (source comes before target)
        let mut new_vs = source_vs;
        new_vs.extend(target_entry.vs.clone());
        target_entry.vs = new_vs;

        target_entry.barycenter = if weight > 0.0 {
            Some(sum / weight)
        } else {
            None
        };
        target_entry.weight = weight;
        target_entry.i = target_entry.i.min(source_i);
    }

    // Mark source as merged
    if let Some(source_entry) = mapped.get_mut(source) {
        source_entry.merged = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::dagre::order::BarycenterEntry;

    #[test]
    fn test_resolve_no_conflicts() {
        let entries = vec![
            BarycenterEntry {
                v: "a".to_string(),
                barycenter: Some(1.0),
                weight: 1.0,
                i: 0,
            },
            BarycenterEntry {
                v: "b".to_string(),
                barycenter: Some(2.0),
                weight: 1.0,
                i: 1,
            },
        ];

        let cg = ConstraintGraph::new();
        let result = resolve_conflicts(entries, &cg);

        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_resolve_with_constraint() {
        // a and b where constraint says a < b but barycenters say b < a
        let entries = vec![
            BarycenterEntry {
                v: "a".to_string(),
                barycenter: Some(2.0), // Would sort after b
                weight: 1.0,
                i: 0,
            },
            BarycenterEntry {
                v: "b".to_string(),
                barycenter: Some(1.0), // Would sort before a
                weight: 1.0,
                i: 1,
            },
        ];

        let mut cg = ConstraintGraph::new();
        cg.set_edge("a", "b"); // a must come before b

        let result = resolve_conflicts(entries, &cg);

        // They should be merged into one entry
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].vs.len(), 2);
        // a should come before b in the merged result
        let a_pos = result[0].vs.iter().position(|v| v == "a");
        let b_pos = result[0].vs.iter().position(|v| v == "b");
        assert!(a_pos < b_pos);
    }

    #[test]
    fn test_constraint_graph_basic() {
        let mut cg = ConstraintGraph::new();
        cg.set_edge("a", "b");
        cg.set_edge("b", "c");

        let edges: Vec<_> = cg.edges().collect();
        assert_eq!(edges.len(), 2);
    }
}
