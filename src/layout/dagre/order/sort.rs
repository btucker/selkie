//! Sorting nodes by barycenter
//!
//! Sorts nodes within a layer by their barycenter values, with special
//! handling for nodes without barycenters.

use super::barycenter::BarycenterEntry;

/// Result of sorting operation
#[derive(Debug, Clone)]
pub struct SortResult {
    pub vs: Vec<String>,
    pub barycenter: Option<f64>,
    pub weight: f64,
}

/// Sort entries by barycenter value
///
/// Entries with a barycenter are sorted by that value.
/// Entries without a barycenter maintain their relative order.
pub fn sort(entries: Vec<BarycenterEntry>, bias_right: bool) -> SortResult {
    // Partition into sortable (has barycenter) and unsortable (no barycenter)
    let mut sortable: Vec<BarycenterEntry> = Vec::new();
    let mut unsortable: Vec<BarycenterEntry> = Vec::new();

    for entry in entries {
        if entry.barycenter.is_some() {
            sortable.push(entry);
        } else {
            unsortable.push(entry);
        }
    }

    // Sort sortable entries by barycenter, using original index (entry.i) for tie-breaking
    sortable.sort_by(|a, b| {
        let bc_a = a.barycenter.unwrap();
        let bc_b = b.barycenter.unwrap();

        match bc_a.partial_cmp(&bc_b) {
            Some(std::cmp::Ordering::Equal) | None => {
                if bias_right {
                    b.i.cmp(&a.i)
                } else {
                    a.i.cmp(&b.i)
                }
            }
            Some(ord) => ord,
        }
    });

    // Sort unsortable by original index (descending for consumption)
    unsortable.sort_by(|a, b| b.i.cmp(&a.i));

    // Merge the two lists
    let mut vs = Vec::new();
    let mut sum = 0.0;
    let mut weight = 0.0;
    let mut vs_index = 0;

    // Consume unsortable entries that come before any sortable
    consume_unsortable(&mut vs, &mut unsortable, &mut vs_index);

    for entry in sortable {
        vs_index += 1;
        vs.push(entry.v.clone());
        if let Some(bc) = entry.barycenter {
            sum += bc * entry.weight;
            weight += entry.weight;
        }
        consume_unsortable(&mut vs, &mut unsortable, &mut vs_index);
    }

    SortResult {
        vs,
        barycenter: if weight > 0.0 {
            Some(sum / weight)
        } else {
            None
        },
        weight,
    }
}

fn consume_unsortable(
    vs: &mut Vec<String>,
    unsortable: &mut Vec<BarycenterEntry>,
    index: &mut usize,
) {
    while let Some(entry) = unsortable.last() {
        if entry.i <= *index {
            let entry = unsortable.pop().unwrap();
            vs.push(entry.v);
            *index += 1;
        } else {
            break;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sort_by_barycenter() {
        let entries = vec![
            BarycenterEntry {
                v: "a".to_string(),
                barycenter: Some(2.0),
                weight: 1.0,
                i: 0,
            },
            BarycenterEntry {
                v: "b".to_string(),
                barycenter: Some(1.0),
                weight: 1.0,
                i: 1,
            },
            BarycenterEntry {
                v: "c".to_string(),
                barycenter: Some(3.0),
                weight: 1.0,
                i: 2,
            },
        ];

        let result = sort(entries, false);

        assert_eq!(result.vs, vec!["b", "a", "c"]);
    }

    #[test]
    fn test_sort_preserves_order_for_equal_barycenter() {
        let entries = vec![
            BarycenterEntry {
                v: "a".to_string(),
                barycenter: Some(1.0),
                weight: 1.0,
                i: 0,
            },
            BarycenterEntry {
                v: "b".to_string(),
                barycenter: Some(1.0),
                weight: 1.0,
                i: 1,
            },
            BarycenterEntry {
                v: "c".to_string(),
                barycenter: Some(1.0),
                weight: 1.0,
                i: 2,
            },
        ];

        let result = sort(entries, false);

        assert_eq!(result.vs, vec!["a", "b", "c"]);
    }

    #[test]
    fn test_sort_bias_right() {
        let entries = vec![
            BarycenterEntry {
                v: "a".to_string(),
                barycenter: Some(1.0),
                weight: 1.0,
                i: 0,
            },
            BarycenterEntry {
                v: "b".to_string(),
                barycenter: Some(1.0),
                weight: 1.0,
                i: 1,
            },
        ];

        let result = sort(entries, true);

        assert_eq!(result.vs, vec!["b", "a"]);
    }

    #[test]
    fn test_sort_handles_no_barycenter() {
        let entries = vec![
            BarycenterEntry {
                v: "a".to_string(),
                barycenter: Some(2.0),
                weight: 1.0,
                i: 0,
            },
            BarycenterEntry {
                v: "b".to_string(),
                barycenter: None,
                weight: 0.0,
                i: 1,
            },
            BarycenterEntry {
                v: "c".to_string(),
                barycenter: Some(1.0),
                weight: 1.0,
                i: 2,
            },
        ];

        let result = sort(entries, false);

        // b has no barycenter, should maintain position relative to its original index
        assert_eq!(result.vs.len(), 3);
        assert!(result.vs.contains(&"a".to_string()));
        assert!(result.vs.contains(&"b".to_string()));
        assert!(result.vs.contains(&"c".to_string()));
    }
}
