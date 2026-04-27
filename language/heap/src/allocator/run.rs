use std::collections::BTreeMap;

use super::PageRun;

/// The free page-run set owned by one allocator.
#[derive(Debug)]
pub(crate) struct PageRunSet {
    /// Small free runs keyed directly by page count.
    small_runs: Vec<Vec<PageRun>>,
    /// Large free runs keyed by page count.
    large_runs: BTreeMap<usize, Vec<PageRun>>,
    /// Free runs keyed by first page index.
    runs_by_start: BTreeMap<usize, PageRun>,
}

impl PageRunSet {
    /// Create one free-run set with small buckets up to one chunk.
    pub(crate) fn new(pages_per_chunk: usize) -> Self {
        let mut small_runs = Vec::with_capacity(pages_per_chunk + 1);
        small_runs.resize_with(pages_per_chunk + 1, Vec::new);

        Self {
            small_runs,
            large_runs: BTreeMap::new(),
            runs_by_start: BTreeMap::new(),
        }
    }

    /// Allocate one free run large enough for the requested size.
    pub(crate) fn allocate(&mut self, page_count: usize) -> Option<PageRun> {
        let small_limit = self.small_runs.len() - 1;

        // prefer small buckets first
        if page_count <= small_limit {
            for run_len in page_count..=small_limit {
                let runs = &mut self.small_runs[run_len];
                if let Some(run) = runs.pop() {
                    self.runs_by_start.remove(&run.first_page.index());

                    let (allocation, remainder) = run.split_prefix_unchecked(page_count);
                    if !remainder.is_empty() {
                        self.free(remainder);
                    }

                    return Some(allocation);
                }
            }
        }

        // then fall back to larger ordered runs
        let (run_len, run) = self
            .large_runs
            .range_mut(page_count..)
            .find_map(|(&run_len, runs)| runs.pop().map(|run| (run_len, run)))?;

        self.runs_by_start.remove(&run.first_page.index());

        if self.large_runs.get(&run_len).is_some_and(Vec::is_empty) {
            self.large_runs.remove(&run_len);
        }

        let (allocation, remainder) = run.split_prefix_unchecked(page_count);
        if !remainder.is_empty() {
            self.free(remainder);
        }

        Some(allocation)
    }

    /// Free one run into both free-run indexes.
    pub(crate) fn free(&mut self, run: PageRun) {
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

        if run.len() < self.small_runs.len() {
            self.small_runs[run.len()].push(run);
        } else {
            self.large_runs.entry(run.len()).or_default().push(run);
        }

        self.runs_by_start.insert(run.first_page.index(), run);
    }

    /// Remove one free run from both free-run indexes.
    fn remove(&mut self, run: PageRun) {
        if run.len() < self.small_runs.len() {
            let runs = &mut self.small_runs[run.len()];
            if let Some(run_index) = runs.iter().position(|candidate| *candidate == run) {
                runs.swap_remove(run_index);
            }
        } else if let Some(runs) = self.large_runs.get_mut(&run.len()) {
            if let Some(run_index) = runs.iter().position(|candidate| *candidate == run) {
                runs.swap_remove(run_index);
            }

            if runs.is_empty() {
                self.large_runs.remove(&run.len());
            }
        }

        self.runs_by_start.remove(&run.first_page.index());
    }

    /// Return the immediately preceding free run when one exists.
    fn previous(&self, run: PageRun) -> Option<PageRun> {
        self.runs_by_start
            .range(..run.start_page_index())
            .next_back()
            .map(|(_, run)| *run)
    }

    /// Return the immediately following free run when one exists.
    fn next(&self, run: PageRun) -> Option<PageRun> {
        self.runs_by_start
            .range(run.end_page_index()..)
            .next()
            .map(|(_, run)| *run)
    }

    /// Merge two adjacent free runs.
    fn merged(left: PageRun, right: PageRun) -> PageRun {
        debug_assert!(left.is_immediately_before(right));

        PageRun::from_raw(left.first_page, left.page_count + right.page_count)
    }
}
