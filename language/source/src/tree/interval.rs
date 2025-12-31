/// Fast interval queries for properly nested AST spans.
///
/// Exploits the nesting property: AST spans never partially overlap,
/// they either contain each other or are disjoint. This allows O(log n + k)
/// queries by building a containment tree.

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

        // sort by (start ASC, end DESC) - parents come before children at same start
        // check if already sorted by (start ASC, end DESC) (common case from parsing)
        let needs_sort = intervals.windows(2).any(|w| {
            let (s1, e1, _) = w[0];
            let (s2, e2, _) = w[1];
            s1 > s2 || (s1 == s2 && e1 < e2)
        });
        if needs_sort {
            intervals.sort_unstable_by(|a, b| a.0.cmp(&b.0).then(b.1.cmp(&a.1)));
        }

        // build entries with parent pointers using a stack
        // the stack contains indices of potential parents (intervals that haven't ended yet)
        let mut entries: Vec<IntervalEntry> = Vec::with_capacity(n);
        let mut stack: Vec<usize> = Vec::with_capacity(32); // typical nesting depth
        for (start, end, node_id) in intervals {
            // pop intervals that have ended before this one starts
            while let Some(&top_idx) = stack.last() {
                if entries[top_idx].end <= start {
                    stack.pop();
                } else {
                    break;
                }
            }

            // parent is the top of stack (or none if stack is empty)
            let parent = stack.last().map(|&i| i as u32).unwrap_or(u32::MAX);

            let idx = entries.len();
            entries.push(IntervalEntry {
                start,
                end,
                node_id,
                parent,
            });

            // push this interval as potential parent for future intervals
            stack.push(idx);
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

        // binary search to find the rightmost interval where interval.start <= start
        let search_idx = self.entries.partition_point(|e| e.start <= start);

        if search_idx == 0 {
            return Vec::new();
        }

        // start from the interval just before partition_point
        let mut idx = search_idx - 1;
        let mut results = Vec::new();

        // find the smallest containing interval using parent pointers
        loop {
            let entry = &self.entries[idx];

            if entry.end > end_inclusive {
                // this interval contains the query - found the smallest one
                // now walk up parent chain to collect all containing intervals
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

            // this interval doesn't contain the query, try its parent
            if entry.parent == u32::MAX {
                // no parent, no containing interval found
                break;
            }
            idx = entry.parent as usize;
        }

        results
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
