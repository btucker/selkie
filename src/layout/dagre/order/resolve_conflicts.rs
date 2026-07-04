//! Resolve conflicts between barycenters and constraint graph
//!
//! Port of dagre's lib/order/resolve-conflicts.js.
//!
//! Given a list of entries of the form {v, barycenter, weight} and a
//! constraint graph this function will resolve any conflicts between the
//! constraint graph and the barycenters for the entries. If the barycenters
//! for an entry would violate a constraint in the constraint graph then we
//! coalesce the nodes in the conflict into a new node that respects the
//! constraint and aggregates barycenter and weight information.
//!
//! This implementation is based on the description in Forster, "A Fast and
//! Simple Heuristic for Constrained Two-Level Crossing Reduction," though it
//! differs in some specific details.

use std::collections::HashMap;

/// A simple directed graph for tracking ordering constraints between sibling
/// subgraphs. Edges are kept in insertion order (graphlib object key order).
#[derive(Debug, Clone, Default)]
pub struct ConstraintGraph {
    edges: Vec<(String, String)>,
}

impl ConstraintGraph {
    pub fn new() -> Self {
        Self { edges: Vec::new() }
    }

    /// Add an ordering constraint: v must come before w
    pub fn set_edge(&mut self, v: &str, w: &str) {
        if !self.edges.iter().any(|(ev, ew)| ev == v && ew == w) {
            self.edges.push((v.to_string(), w.to_string()));
        }
    }

    /// Get all edges in the constraint graph in insertion order
    pub fn edges(&self) -> impl Iterator<Item = (&str, &str)> {
        self.edges.iter().map(|(v, w)| (v.as_str(), w.as_str()))
    }
}

/// Entry with additional tracking information for the resolve algorithm
#[derive(Debug)]
struct MappedEntry {
    indegree: usize,
    /// Indices of entries pointing to this entry that were already processed
    incoming: Vec<usize>,
    /// Indices of entries this entry points to
    outgoing: Vec<usize>,
    vs: Vec<String>,
    i: usize,
    barycenter: Option<f64>,
    weight: f64,
    merged: bool,
}

/// Result of resolving conflicts
#[derive(Debug, Clone)]
pub struct ResolvedEntry {
    pub vs: Vec<String>,
    /// Lowest original index of any of the elements in `vs`
    pub i: usize,
    pub barycenter: Option<f64>,
    pub weight: f64,
}

/// Resolve conflicts between barycenter ordering and constraint graph
///
/// If barycenters would place nodes in an order that violates the constraint
/// graph, those nodes are coalesced into a single entry that respects the
/// constraint.
pub fn resolve_conflicts(
    entries: Vec<super::BarycenterEntry>,
    cg: &ConstraintGraph,
) -> Vec<ResolvedEntry> {
    let mut index_of: HashMap<String, usize> = HashMap::new();
    let mut mapped: Vec<MappedEntry> = Vec::with_capacity(entries.len());

    for (i, entry) in entries.into_iter().enumerate() {
        index_of.insert(entry.v.clone(), i);
        mapped.push(MappedEntry {
            indegree: 0,
            incoming: Vec::new(),
            outgoing: Vec::new(),
            vs: vec![entry.v],
            i,
            barycenter: entry.barycenter,
            weight: if entry.barycenter.is_some() {
                entry.weight
            } else {
                0.0
            },
            merged: false,
        });
    }

    // Process constraint graph edges (only those between mapped entries)
    for (v, w) in cg.edges() {
        if let (Some(&vi), Some(&wi)) = (index_of.get(v), index_of.get(w)) {
            mapped[wi].indegree += 1;
            mapped[vi].outgoing.push(wi);
        }
    }

    // Source set: entries with indegree 0, in original entry order
    let source_set: Vec<usize> = (0..mapped.len())
        .filter(|&i| mapped[i].indegree == 0)
        .collect();

    do_resolve_conflicts(mapped, source_set)
}

fn do_resolve_conflicts(
    mut mapped: Vec<MappedEntry>,
    mut source_set: Vec<usize>,
) -> Vec<ResolvedEntry> {
    let mut results: Vec<usize> = Vec::new();

    while let Some(idx) = source_set.pop() {
        results.push(idx);

        // handleIn: merge already-processed predecessors whose barycenter
        // would violate the constraint (iterated in reverse order)
        let mut incoming = mapped[idx].incoming.clone();
        incoming.reverse();
        for u in incoming {
            if mapped[u].merged {
                continue;
            }
            let u_bc = mapped[u].barycenter;
            let v_bc = mapped[idx].barycenter;
            let should_merge = match (u_bc, v_bc) {
                (Some(ub), Some(vb)) => ub >= vb,
                _ => true,
            };
            if should_merge {
                merge_entries(&mut mapped, idx, u);
            }
        }

        // handleOut: record this entry on successors and release those whose
        // indegree drops to zero
        let outgoing = mapped[idx].outgoing.clone();
        for w in outgoing {
            mapped[w].incoming.push(idx);
            mapped[w].indegree -= 1;
            if mapped[w].indegree == 0 {
                source_set.push(w);
            }
        }
    }

    results
        .into_iter()
        .filter(|&idx| !mapped[idx].merged)
        .map(|idx| {
            let entry = &mapped[idx];
            ResolvedEntry {
                vs: entry.vs.clone(),
                i: entry.i,
                barycenter: entry.barycenter,
                weight: entry.weight,
            }
        })
        .collect()
}

fn merge_entries(mapped: &mut [MappedEntry], target: usize, source: usize) {
    let mut sum = 0.0;
    let mut weight = 0.0;

    if mapped[target].weight > 0.0 {
        if let Some(bc) = mapped[target].barycenter {
            sum += bc * mapped[target].weight;
            weight += mapped[target].weight;
        }
    }

    if mapped[source].weight > 0.0 {
        if let Some(bc) = mapped[source].barycenter {
            sum += bc * mapped[source].weight;
            weight += mapped[source].weight;
        }
    }

    // Source nodes come before target nodes in the merged entry
    let mut new_vs = mapped[source].vs.clone();
    new_vs.extend(mapped[target].vs.clone());
    mapped[target].vs = new_vs;

    mapped[target].barycenter = if weight > 0.0 {
        Some(sum / weight)
    } else {
        None
    };
    mapped[target].weight = weight;
    mapped[target].i = mapped[target].i.min(mapped[source].i);
    mapped[source].merged = true;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::dagre::order::BarycenterEntry;

    fn entry(v: &str, barycenter: Option<f64>, weight: f64, i: usize) -> BarycenterEntry {
        BarycenterEntry {
            v: v.to_string(),
            barycenter,
            weight,
            i,
        }
    }

    #[test]
    fn test_returns_back_singleton_lists_with_no_constraints() {
        // dagre: "returns back nodes unchanged when no constraints exist"
        let entries = vec![entry("a", Some(2.0), 3.0, 0), entry("b", Some(1.0), 2.0, 1)];
        let cg = ConstraintGraph::new();
        let mut result = resolve_conflicts(entries, &cg);
        result.sort_by_key(|e| e.vs[0].clone());

        assert_eq!(result.len(), 2);
        assert_eq!(result[0].vs, vec!["a"]);
        assert_eq!(result[0].barycenter, Some(2.0));
        assert_eq!(result[0].weight, 3.0);
        assert_eq!(result[1].vs, vec!["b"]);
        assert_eq!(result[1].barycenter, Some(1.0));
        assert_eq!(result[1].weight, 2.0);
    }

    #[test]
    fn test_returns_nodes_unchanged_when_constraint_satisfied() {
        // dagre: constraint a -> b already satisfied by barycenters
        let entries = vec![entry("a", Some(1.0), 1.0, 0), entry("b", Some(2.0), 1.0, 1)];
        let mut cg = ConstraintGraph::new();
        cg.set_edge("a", "b");
        let result = resolve_conflicts(entries, &cg);

        assert_eq!(result.len(), 2);
        let a = result.iter().find(|e| e.vs == vec!["a"]).unwrap();
        let b = result.iter().find(|e| e.vs == vec!["b"]).unwrap();
        assert_eq!(a.barycenter, Some(1.0));
        assert_eq!(b.barycenter, Some(2.0));
    }

    #[test]
    fn test_coalesces_nodes_when_constraint_violated() {
        // dagre: "coalesces nodes when there is a conflict"
        let entries = vec![entry("a", Some(2.0), 1.0, 0), entry("b", Some(1.0), 1.0, 1)];
        let mut cg = ConstraintGraph::new();
        cg.set_edge("a", "b");
        let result = resolve_conflicts(entries, &cg);

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].vs, vec!["a", "b"]);
        assert_eq!(result[0].barycenter, Some(1.5));
        assert_eq!(result[0].weight, 2.0);
        assert_eq!(result[0].i, 0);
    }

    #[test]
    fn test_coalesces_nodes_weighted() {
        // dagre: "coalesces nodes when there is a conflict #2"
        let entries = vec![entry("a", Some(4.0), 1.0, 0), entry("b", Some(3.0), 2.0, 1)];
        let mut cg = ConstraintGraph::new();
        cg.set_edge("a", "b");
        let result = resolve_conflicts(entries, &cg);

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].vs, vec!["a", "b"]);
        // (4*1 + 3*2) / 3 = 10/3
        assert!((result[0].barycenter.unwrap() - 10.0 / 3.0).abs() < 1e-9);
        assert_eq!(result[0].weight, 3.0);
    }

    #[test]
    fn test_works_with_multiple_constraints_for_same_target() {
        // dagre: "works with multiple constraints for the same target #1"
        let entries = vec![
            entry("a", Some(4.0), 1.0, 0),
            entry("b", Some(3.0), 1.0, 1),
            entry("c", Some(2.0), 1.0, 2),
        ];
        let mut cg = ConstraintGraph::new();
        cg.set_edge("a", "c");
        cg.set_edge("b", "c");
        let result = resolve_conflicts(entries, &cg);

        assert_eq!(result.len(), 1);
        let vs = &result[0].vs;
        assert!(
            vs.iter().position(|v| v == "a").unwrap() < vs.iter().position(|v| v == "c").unwrap()
        );
        assert!(
            vs.iter().position(|v| v == "b").unwrap() < vs.iter().position(|v| v == "c").unwrap()
        );
        assert_eq!(result[0].barycenter, Some(3.0));
        assert_eq!(result[0].weight, 3.0);
    }

    #[test]
    fn test_applies_constraints_transitively() {
        // dagre: entries a(bc=1), b(bc=2), c(bc=3) with c->b and b->a
        let entries = vec![
            entry("a", Some(1.0), 1.0, 0),
            entry("b", Some(2.0), 1.0, 1),
            entry("c", Some(3.0), 1.0, 2),
        ];
        let mut cg = ConstraintGraph::new();
        cg.set_edge("c", "b");
        cg.set_edge("b", "a");
        let result = resolve_conflicts(entries, &cg);

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].vs, vec!["c", "b", "a"]);
        assert_eq!(result[0].barycenter, Some(2.0));
        assert_eq!(result[0].weight, 3.0);
    }

    #[test]
    fn test_entry_without_barycenter_merges_on_constraint() {
        // Constraint from an entry with no barycenter always merges
        let entries = vec![entry("a", None, 0.0, 0), entry("b", Some(1.0), 2.0, 1)];
        let mut cg = ConstraintGraph::new();
        cg.set_edge("a", "b");
        let result = resolve_conflicts(entries, &cg);

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].vs, vec!["a", "b"]);
        assert_eq!(result[0].barycenter, Some(1.0));
        assert_eq!(result[0].weight, 2.0);
    }
}
