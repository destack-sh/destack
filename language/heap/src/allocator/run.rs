use std::collections::BTreeMap;

use super::PageRun;

/// The free page-run set owned by one allocator.
#[derive(Debug)]
pub(crate) struct PageRunSet {
    /// Small free runs keyed directly by page count.
    small_by_len: Vec<Vec<PageRun>>,
    /// Large free runs keyed by page count.
    large_by_len: BTreeMap<usize, Vec<PageRun>>,
    /// Free runs keyed by first page index.
    by_start: BTreeMap<usize, PageRun>,
}

impl PageRunSet {
    /// Create one free-run set with small buckets up to one arena.
    pub(crate) fn new(pages_per_arena: usize) -> Self {
        let mut small_by_len = Vec::with_capacity(pages_per_arena + 1);
        small_by_len.resize_with(pages_per_arena + 1, Vec::new);

        Self {
            small_by_len,
            large_by_len: BTreeMap::new(),
            by_start: BTreeMap::new(),
        }
    }

    /// Return one free run large enough for the requested size.
    pub(crate) fn take(&mut self, page_count: usize) -> Option<PageRun> {
        let small_limit = self.small_by_len.len().saturating_sub(1);

        // prefer small buckets first
        if page_count <= small_limit {
            for run_len in page_count..=small_limit {
                let runs = &mut self.small_by_len[run_len];
                if let Some(run) = runs.pop() {
                    self.by_start.remove(&run.first_page.index());

                    let (allocation, remainder) = run.split_prefix(page_count)?;

                    if !remainder.is_empty() {
                        self.insert(remainder);
                    }

                    return Some(allocation);
                }
            }
        }

        // then fall back to larger ordered runs
        let (run_len, run) = self
            .large_by_len
            .range_mut(page_count..)
            .find_map(|(&run_len, runs)| runs.pop().map(|run| (run_len, run)))?;

        self.by_start.remove(&run.first_page.index());

        if self.large_by_len.get(&run_len).is_some_and(Vec::is_empty) {
            self.large_by_len.remove(&run_len);
        }

        let (allocation, remainder) = run.split_prefix(page_count)?;

        if !remainder.is_empty() {
            self.insert(remainder);
        }

        Some(allocation)
    }

    /// Return one free run to both free-run indexes.
    pub(crate) fn insert(&mut self, run: PageRun) {
        let mut run = run;

        // merge the immediate predecessor when it touches this run
        if let Some(previous_run) = self.previous(run)
            && previous_run.is_immediately_before(run)
        {
            self.remove(previous_run);
            run = Self::merged(previous_run, run);
        }

        // merge the immediate successor when it touches this run
        if let Some(next_run) = self.next(run)
            && run.is_immediately_before(next_run)
        {
            self.remove(next_run);
            run = Self::merged(run, next_run);
        }

        if run.len() < self.small_by_len.len() {
            self.small_by_len[run.len()].push(run);
        } else {
            self.large_by_len.entry(run.len()).or_default().push(run);
        }

        self.by_start.insert(run.first_page.index(), run);
    }

    /// Remove one free run from both free-run indexes.
    fn remove(&mut self, run: PageRun) {
        if run.len() < self.small_by_len.len() {
            let runs = &mut self.small_by_len[run.len()];
            if let Some(run_index) = runs.iter().position(|candidate| *candidate == run) {
                runs.swap_remove(run_index);
            }
        } else if let Some(runs) = self.large_by_len.get_mut(&run.len()) {
            if let Some(run_index) = runs.iter().position(|candidate| *candidate == run) {
                runs.swap_remove(run_index);
            }

            if runs.is_empty() {
                self.large_by_len.remove(&run.len());
            }
        }

        self.by_start.remove(&run.first_page.index());
    }

    /// Return the immediately preceding free run when one exists.
    fn previous(&self, run: PageRun) -> Option<PageRun> {
        self.by_start
            .range(..run.start_page_index())
            .next_back()
            .map(|(_, run)| *run)
    }

    /// Return the immediately following free run when one exists.
    fn next(&self, run: PageRun) -> Option<PageRun> {
        self.by_start
            .range(run.end_page_index()..)
            .next()
            .map(|(_, run)| *run)
    }

    /// Merge two adjacent free runs.
    fn merged(left: PageRun, right: PageRun) -> PageRun {
        let page_count = left.len().checked_add(right.len()).unwrap_or_else(|| {
            panic!(
                "adjacent free runs should not overflow: first_page={}, left_len={}, right_len={}",
                left.first_page.index(),
                left.len(),
                right.len()
            )
        });

        PageRun::new(left.first_page, page_count)
            .unwrap_or_else(|error| panic!("adjacent free runs should stay valid: {error}"))
    }
}
