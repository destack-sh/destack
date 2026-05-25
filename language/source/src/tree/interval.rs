use crate::Span;

/// Interval entry with parent pointer for containment tree.
#[derive(Debug, Clone, Copy)]
struct IntervalEntry {
    start: u32,
    end: u32,
    node_id: u32,
    /// Index of parent in the tree, or u32::MAX if root.
    parent: u32,
}

/// Nested interval tree exploiting proper nesting of AST spans.
#[derive(Debug, Clone)]
pub struct IntervalTree {
    /// Intervals with parent pointers, sorted by start ASC.
    entries: Vec<IntervalEntry>,
}

impl IntervalTree {
    /// Build a nested interval tree from spans.
    /// Takes (start, end, node_id) tuples.
    pub fn build(mut intervals: Vec<(u32, u32, u32)>) -> Self {
        if intervals.is_empty() {
            return Self {
                entries: Vec::new(),
            };
        }

        let n = intervals.len();

        // check whether parse order already has parents before children
        let needs_sort = intervals.windows(2).any(|window| {
            let (left_start, left_end, _) = window[0];
            let (right_start, right_end, _) = window[1];
            left_start > right_start || (left_start == right_start && left_end < right_end)
        });
        if needs_sort {
            intervals
                .sort_unstable_by(|left, right| left.0.cmp(&right.0).then(right.1.cmp(&left.1)));
        }

        // build entries with parent pointers
        let mut entries: Vec<IntervalEntry> = Vec::with_capacity(n);
        let mut stack: Vec<usize> = Vec::with_capacity(32);
        for (start, end, node_id) in intervals {
            push_interval_entry(&mut entries, &mut stack, node_id, start, end);
        }

        Self { entries }
    }

    /// Build a nested interval tree from node spans.
    /// Node ids correspond to span indices.
    pub fn build_from_spans(spans: &[Span]) -> Self {
        Self::build_from_ranges(spans.len(), |index| {
            let span = spans[index];

            (span.start, span.end)
        })
    }

    /// Build a nested interval tree from node span ranges.
    /// Node ids correspond to span indices.
    pub fn build_from_ranges(len: usize, mut range_at: impl FnMut(usize) -> (u32, u32)) -> Self {
        if len == 0 {
            return Self {
                entries: Vec::new(),
            };
        }

        // check whether parse order already has parents before children
        let needs_sort = (1..len).any(|index| {
            let (left_start, left_end) = range_at(index - 1);
            let (right_start, right_end) = range_at(index);

            left_start > right_start || (left_start == right_start && left_end < right_end)
        });

        let mut entries: Vec<IntervalEntry> = Vec::with_capacity(len);
        let mut stack: Vec<usize> = Vec::with_capacity(32);

        if needs_sort {
            let mut node_order: Vec<u32> = (0..len as u32).collect();
            node_order.sort_unstable_by(|&left_id, &right_id| {
                let (left_start, left_end) = range_at(left_id as usize);
                let (right_start, right_end) = range_at(right_id as usize);

                left_start.cmp(&right_start).then(right_end.cmp(&left_end))
            });

            for node_id in node_order {
                let (start, end) = range_at(node_id as usize);
                push_interval_entry(&mut entries, &mut stack, node_id, start, end);
            }
        } else {
            for index in 0..len {
                let (start, end) = range_at(index);
                push_interval_entry(&mut entries, &mut stack, index as u32, start, end);
            }
        }

        Self { entries }
    }

    /// Find all intervals that contain the range [start, end_inclusive].
    /// Returns (start, end, node_id, length) for each containing interval.
    #[inline]
    pub fn query_containing(&self, start: u32, end_inclusive: u32) -> Vec<(u32, u32, u32, u32)> {
        if self.entries.is_empty() {
            return Vec::new();
        }

        // find the last interval that starts before this range
        let search_idx = self.entries.partition_point(|e| e.start <= start);

        if search_idx == 0 {
            return Vec::new();
        }

        // start from the nearest candidate
        let mut idx = search_idx - 1;
        let mut results = Vec::new();

        // find the smallest containing interval
        loop {
            let entry = &self.entries[idx];

            if entry.end > end_inclusive {
                // collect this interval and its parents
                let mut collect_idx = idx;
                loop {
                    let e = &self.entries[collect_idx];
                    results.push((e.start, e.end, e.node_id, e.end - e.start));
                    if e.parent == u32::MAX {
                        break;
                    }
                    collect_idx = e.parent as usize;
                }
                break;
            }

            // try the parent when this interval does not contain the query
            if entry.parent == u32::MAX {
                break;
            }
            idx = entry.parent as usize;
        }

        results
    }

    /// Visit all intervals that contain the range [start, end_inclusive].
    pub fn visit_containing(
        &self,
        start: u32,
        end_inclusive: u32,
        mut visit: impl FnMut(u32, u32, u32, u32),
    ) {
        if self.entries.is_empty() {
            return;
        }

        // find the last interval that starts before this range
        let search_idx = self.entries.partition_point(|entry| entry.start <= start);
        if search_idx == 0 {
            return;
        }

        // start from the nearest candidate
        let mut idx = search_idx - 1;

        // find the smallest containing interval
        loop {
            let entry = &self.entries[idx];

            if entry.end > end_inclusive {
                // visit this interval and its parents
                let mut collect_idx = idx;
                loop {
                    let current = &self.entries[collect_idx];
                    visit(
                        current.start,
                        current.end,
                        current.node_id,
                        current.end - current.start,
                    );
                    if current.parent == u32::MAX {
                        break;
                    }
                    collect_idx = current.parent as usize;
                }
                break;
            }

            // try the parent when this interval does not contain the query
            if entry.parent == u32::MAX {
                break;
            }
            idx = entry.parent as usize;
        }
    }

    /// Check if tree is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Get the number of intervals.
    #[inline]
    pub fn len(&self) -> usize {
        self.entries.len()
    }
}

/// Push one interval entry and wire its parent from the active stack.
fn push_interval_entry(
    entries: &mut Vec<IntervalEntry>,
    stack: &mut Vec<usize>,
    node_id: u32,
    start: u32,
    end: u32,
) {
    // drop parents that ended before this interval
    while let Some(&top_index) = stack.last() {
        if entries[top_index].end <= start {
            stack.pop();
        } else {
            break;
        }
    }

    // append the interval under the current parent
    let parent = stack.last().map(|&index| index as u32).unwrap_or(u32::MAX);
    let index = entries.len();
    entries.push(IntervalEntry {
        start,
        end,
        node_id,
        parent,
    });
    stack.push(index);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_interval_tree_empty() {
        let tree = IntervalTree::build(vec![]);
        assert!(tree.is_empty());
        assert_eq!(tree.query_containing(0, 10), vec![]);
    }

    #[test]
    fn test_interval_tree_single() {
        let tree = IntervalTree::build(vec![(0, 100, 0)]);
        let results = tree.query_containing(10, 20);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].2, 0);
    }

    #[test]
    fn test_interval_tree_nested() {
        let tree = IntervalTree::build(vec![(0, 100, 0), (10, 90, 1), (20, 80, 2)]);
        let results = tree.query_containing(40, 50);
        assert_eq!(results.len(), 3);
    }

    #[test]
    fn test_interval_tree_non_overlapping() {
        let tree = IntervalTree::build(vec![(0, 10, 0), (20, 30, 1), (40, 50, 2)]);
        let results = tree.query_containing(25, 28);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].2, 1);
    }

    #[test]
    fn test_interval_tree_flat_decls() {
        // simulates a flat declaration file with global scope
        let tree = IntervalTree::build(vec![
            (0, 1000, 0), // global scope
            (10, 20, 1),  // decl 1
            (30, 40, 2),  // decl 2
            (50, 60, 3),  // decl 3
        ]);

        // query inside decl 2 should find decl 2 and global scope
        let results = tree.query_containing(35, 38);
        assert_eq!(results.len(), 2);

        // query between decls should only find global scope
        let results = tree.query_containing(22, 25);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].2, 0); // global scope
    }
}
