use super::{Allocator, PageRun};
use crate::HeapResult;

/// One bounded cache of contiguous page runs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PageRunCache {
    /// The maximum cached page count.
    page_capacity: usize,
    /// The current cached page count.
    cached_pages: usize,
    /// The cached contiguous page runs.
    runs: Vec<PageRun>,
}

impl Default for PageRunCache {
    fn default() -> Self {
        Self::new(0)
    }
}

impl PageRunCache {
    /// Create one empty page-run cache with the given page capacity.
    pub(crate) const fn new(page_capacity: usize) -> Self {
        Self {
            page_capacity,
            cached_pages: 0,
            runs: Vec::new(),
        }
    }

    /// Allocate one page run through this cache.
    pub(crate) fn allocate_pages(
        &mut self,
        allocator: &Allocator,
        byte_len: usize,
    ) -> HeapResult<PageRun> {
        let page_count = allocator.page_count(byte_len);
        if page_count == 0 {
            return Ok(PageRun::empty());
        }

        // reuse one cached run when possible
        if let Some(run) = self.allocate(page_count) {
            return Ok(run);
        }

        allocator.allocate_pages(byte_len)
    }

    /// Release one page run through this cache.
    pub(crate) fn release_page_run(
        &mut self,
        allocator: &Allocator,
        page_run: PageRun,
    ) -> HeapResult<()> {
        if !allocator.is_run_unique(page_run)? {
            return allocator.release_page_run(&page_run);
        }

        if self.cache(page_run) {
            return Ok(());
        }

        allocator.recycle_cached_run(page_run)?;

        Ok(())
    }

    /// Flush this cache back into the allocator free runs.
    pub(crate) fn flush(&mut self, allocator: &Allocator) -> HeapResult<()> {
        for run in self.drain() {
            allocator.recycle_cached_run(run)?;
        }

        Ok(())
    }

    /// Return the currently cached byte count.
    pub(crate) fn cached_bytes(&self, page_size_bytes: usize) -> u64 {
        self.cached_pages as u64 * page_size_bytes as u64
    }

    /// Return one cached run that satisfies the requested page count.
    pub(crate) fn allocate(&mut self, page_count: usize) -> Option<PageRun> {
        if page_count == 0 {
            return Some(PageRun::empty());
        }

        // prefer the most recently released run first
        let run_index = self.runs.iter().rposition(|run| run.len() >= page_count)?;
        let run = self.runs.swap_remove(run_index);
        self.cached_pages -= run.len();
        if run.len() == page_count {
            return Some(run);
        }

        let (allocation, remainder) = run.split_prefix_unchecked(page_count);
        if !remainder.is_empty() {
            self.cache(remainder);
        }

        Some(allocation)
    }

    /// Cache one contiguous run when it fits this cache.
    pub(crate) fn cache(&mut self, run: PageRun) -> bool {
        if run.is_empty() {
            return true;
        }

        if self.page_capacity == 0 {
            return false;
        }

        let next_cached_pages = self.cached_pages + run.len();
        if next_cached_pages > self.page_capacity {
            return false;
        }

        let run = self.coalesce(run);

        self.cached_pages = next_cached_pages;
        self.runs.push(run);

        true
    }

    /// Drain every cached run.
    pub(crate) fn drain(&mut self) -> impl Iterator<Item = PageRun> + '_ {
        self.cached_pages = 0;

        self.runs.drain(..)
    }

    /// Merge one run with any immediately adjacent cached runs.
    fn coalesce(&mut self, mut run: PageRun) -> PageRun {
        let mut run_index = 0;
        while run_index < self.runs.len() {
            let candidate = self.runs[run_index];
            let merged = if candidate.is_immediately_before(run) {
                Some(Self::merged_run(candidate, run))
            } else if run.is_immediately_before(candidate) {
                Some(Self::merged_run(run, candidate))
            } else {
                None
            };

            if let Some(merged) = merged {
                self.runs.swap_remove(run_index);
                run = merged;
                continue;
            }

            run_index += 1;
        }

        run
    }

    /// Merge two adjacent cached runs.
    fn merged_run(left: PageRun, right: PageRun) -> PageRun {
        debug_assert!(left.is_immediately_before(right));

        PageRun::from_raw(left.first_page, left.page_count + right.page_count)
    }
}
