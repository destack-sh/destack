use super::{Arena, PageRun, PageView};
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
        arena: &Arena,
        byte_len: usize,
    ) -> HeapResult<PageView> {
        let page_count = arena.page_count(byte_len);
        if page_count == 0 {
            return Ok(PageView::empty());
        }

        // reuse one cached run when possible
        if let Some(run) = self.take(page_count) {
            arena.zero_run(run)?;

            return Ok(PageView::from_run(run));
        }

        arena.allocate_zeroed(byte_len)
    }

    /// Allocate one initialized page view through this cache.
    pub(crate) fn allocate_bytes(&mut self, arena: &Arena, bytes: &[u8]) -> HeapResult<PageView> {
        let mut page_view = self.allocate_zeroed(arena, bytes.len())?;

        // initialize the new logical page range
        arena.set_bytes(&mut page_view, 0, bytes)?;

        Ok(page_view)
    }

    /// Release one page view through this cache.
    pub(crate) fn release_page_view(
        &mut self,
        arena: &Arena,
        page_view: PageView,
    ) -> HeapResult<()> {
        let Some(run) = page_view.as_run() else {
            return arena.release_page_view(&page_view);
        };

        if !arena.run_is_unique(run)? {
            return arena.release_page_view(&page_view);
        }

        if self.insert(run) {
            return Ok(());
        }

        arena.release_cached_run(run)
    }

    /// Flush this cache back into the arena page-run pool.
    pub(crate) fn try_flush(&mut self, arena: &Arena) -> HeapResult<()> {
        for run in self.drain() {
            arena.release_cached_run(run)?;
        }

        Ok(())
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

        let run_index = self.runs.iter().position(|run| run.len() >= page_count)?;
        let run = self.runs.swap_remove(run_index);
        self.cached_pages -= run.len();

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
                PageRun::new(candidate.first_page, candidate.len() + run.len()).ok()
            } else if run.is_immediately_before(candidate) {
                PageRun::new(run.first_page, run.len() + candidate.len()).ok()
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
}
