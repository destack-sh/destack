use super::{Allocator, PageRun, PageView};
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

    /// Allocate one zeroed page view through this cache.
    pub(crate) fn allocate_zeroed(
        &mut self,
        allocator: &Allocator,
        byte_len: usize,
    ) -> HeapResult<PageView> {
        let page_count = allocator.page_count(byte_len);
        if page_count == 0 {
            return Ok(PageView::empty());
        }

        // reuse one cached run when possible
        if let Some(run) = self.take(page_count) {
            allocator.zero_run(run)?;

            return Ok(PageView::from_run(run));
        }

        allocator.allocate_zeroed(byte_len)
    }

    /// Allocate one initialized page view through this cache.
    pub(crate) fn allocate_bytes(
        &mut self,
        allocator: &Allocator,
        bytes: &[u8],
    ) -> HeapResult<PageView> {
        let mut page_view = self.allocate_zeroed(allocator, bytes.len())?;

        // initialize the new logical page range
        allocator.set_bytes(&mut page_view, 0, bytes)?;

        Ok(page_view)
    }

    /// Release one page view through this cache.
    pub(crate) fn release_page_view(
        &mut self,
        allocator: &Allocator,
        page_view: PageView,
    ) -> HeapResult<()> {
        let Some(run) = page_view.as_run() else {
            return allocator.release_page_view(&page_view);
        };

        if !allocator.run_is_unique(run)? {
            return allocator.release_page_view(&page_view);
        }

        if self.insert(run) {
            return Ok(());
        }

        allocator.free_cached_run(run);

        Ok(())
    }

    /// Flush this cache back into the allocator free runs.
    pub(crate) fn flush(&mut self, allocator: &Allocator) {
        for run in self.drain() {
            allocator.free_cached_run(run);
        }
    }

    /// Return the currently cached byte count.
    pub(crate) fn cached_bytes(&self, page_bytes: usize) -> u64 {
        self.cached_pages as u64 * page_bytes as u64
    }

    /// Return one cached run that satisfies the requested page count.
    pub(crate) fn take(&mut self, page_count: usize) -> Option<PageRun> {
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

        let (allocation, remainder) = run.split_prefix(page_count)?;
        if !remainder.is_empty() {
            self.insert(remainder);
        }

        Some(allocation)
    }

    /// Insert one contiguous run when it fits this cache.
    pub(crate) fn insert(&mut self, run: PageRun) -> bool {
        if run.is_empty() {
            return true;
        }

        if self.page_capacity == 0 {
            return false;
        }

        let Some(next_cached_pages) = self.cached_pages.checked_add(run.len()) else {
            return false;
        };

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
        let page_count = left.len().checked_add(right.len()).unwrap_or_else(|| {
            panic!(
                "adjacent cached runs should not overflow: first_page={}, left_len={}, right_len={}",
                left.first_page.index(),
                left.len(),
                right.len()
            )
        });

        PageRun::new(left.first_page, page_count)
            .unwrap_or_else(|error| panic!("adjacent cached runs should stay valid: {error}"))
    }
}
