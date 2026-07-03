//! Sorting entries by barycenter
//!
//! Port of dagre's lib/order/sort.js. Sorts resolved entries (each possibly
//! holding several nodes) by their barycenter values; entries without a
//! barycenter are re-inserted at their original index.

use super::resolve_conflicts::ResolvedEntry;

/// Result of sorting operation
#[derive(Debug, Clone)]
pub struct SortResult {
    pub vs: Vec<String>,
    pub barycenter: Option<f64>,
    pub weight: f64,
}

/// Sort entries by barycenter value
///
/// Entries with a barycenter are sorted by that value (ties broken by the
/// original index, reversed when `bias_right`). Entries without a barycenter
/// are consumed back into the result at their original index.
pub fn sort(entries: Vec<ResolvedEntry>, bias_right: bool) -> SortResult {
    let mut sortable: Vec<ResolvedEntry> = Vec::new();
    let mut unsortable: Vec<ResolvedEntry> = Vec::new();

    for entry in entries {
        if entry.barycenter.is_some() {
            sortable.push(entry);
        } else {
            unsortable.push(entry);
        }
    }

    // compareWithBias: barycenter ascending, ties by original index
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

    // Unsortable entries are consumed from the highest index down
    unsortable.sort_by_key(|entry| std::cmp::Reverse(entry.i));

    let mut vs: Vec<String> = Vec::new();
    let mut sum = 0.0;
    let mut weight = 0.0;
    let mut vs_index = 0usize;

    consume_unsortable(&mut vs, &mut unsortable, &mut vs_index);

    for entry in sortable {
        vs_index += entry.vs.len();
        vs.extend(entry.vs.iter().cloned());
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
    unsortable: &mut Vec<ResolvedEntry>,
    index: &mut usize,
) {
    while let Some(entry) = unsortable.last() {
        if entry.i <= *index {
            let entry = unsortable.pop().unwrap();
            vs.extend(entry.vs);
            *index += 1;
        } else {
            break;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(vs: &[&str], barycenter: Option<f64>, weight: f64, i: usize) -> ResolvedEntry {
        ResolvedEntry {
            vs: vs.iter().map(|s| s.to_string()).collect(),
            barycenter,
            weight,
            i,
        }
    }

    #[test]
    fn test_sorts_flat_entries_by_barycenter() {
        // dagre sort-test.js "sorts flat subgraphs by barycenter"
        let entries = vec![
            entry(&["a"], Some(2.0), 3.0, 0),
            entry(&["b"], Some(1.0), 2.0, 1),
            entry(&["c"], Some(4.0), 1.0, 2),
        ];

        let result = sort(entries, false);

        assert_eq!(result.vs, vec!["b", "a", "c"]);
        // (2*3 + 1*2 + 4*1) / 6 = 12/6 = 2
        assert_eq!(result.barycenter, Some(2.0));
        assert_eq!(result.weight, 6.0);
    }

    #[test]
    fn test_sorts_nested_entries_by_barycenter() {
        // dagre sort-test.js "can sort super-nodes"
        let entries = vec![
            entry(&["a", "d"], Some(2.0), 3.0, 0),
            entry(&["b"], Some(1.0), 2.0, 1),
            entry(&["c"], Some(4.0), 1.0, 2),
        ];

        let result = sort(entries, false);

        assert_eq!(result.vs, vec!["b", "a", "d", "c"]);
    }

    #[test]
    fn test_bias_right() {
        let entries = vec![
            entry(&["a"], Some(1.0), 1.0, 0),
            entry(&["b"], Some(1.0), 1.0, 1),
        ];

        let result = sort(entries, true);

        assert_eq!(result.vs, vec!["b", "a"]);
    }

    #[test]
    fn test_biases_left_without_bias_right() {
        let entries = vec![
            entry(&["a"], Some(1.0), 1.0, 0),
            entry(&["b"], Some(1.0), 1.0, 1),
        ];

        let result = sort(entries, false);

        assert_eq!(result.vs, vec!["a", "b"]);
    }

    #[test]
    fn test_handles_no_barycenter() {
        // dagre sort-test.js "keeps entries w/o barycenters in their position"
        let entries = vec![
            entry(&["a"], Some(2.0), 1.0, 0),
            entry(&["b"], None, 0.0, 1),
            entry(&["c"], Some(1.0), 1.0, 2),
        ];

        let result = sort(entries, false);

        // c (bc=1) first, then b consumed at index 1, then a
        assert_eq!(result.vs, vec!["c", "b", "a"]);
    }
}
