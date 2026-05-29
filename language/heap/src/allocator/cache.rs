use super::{Allocator, PageSpan};
use crate::HeapResult;

/// One bounded cache of contiguous page spans.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PageSpanCache {
    /// The maximum cached page count.
    page_capacity: usize,
    /// The current cached page count.
    cached_pages: usize,
    /// The cached contiguous page spans.
    spans: Vec<PageSpan>,
}

impl Default for PageSpanCache {
    fn default() -> Self {
        Self::new(0)
    }
}

impl PageSpanCache {
    /// Create one empty page-span cache with the given page capacity.
    pub(crate) const fn new(page_capacity: usize) -> Self {
        Self {
            page_capacity,
            cached_pages: 0,
            spans: Vec::new(),
        }
    }

    /// Allocate one page span through this cache.
    pub(crate) fn allocate_pages(
        &mut self,
        allocator: &Allocator,
        byte_len: usize,
    ) -> HeapResult<PageSpan> {
        let page_count = allocator.page_count(byte_len);
        if page_count == 0 {
            return Ok(PageSpan::empty());
        }

        // reuse one cached span when possible
        if let Some(span) = self.allocate(page_count) {
            return Ok(span);
        }

        allocator.allocate_pages(byte_len)
    }

    /// Release one page span through this cache.
    pub(crate) fn release_page_span(
        &mut self,
        allocator: &Allocator,
        page_span: PageSpan,
    ) -> HeapResult<()> {
        if !allocator.is_span_unique(page_span)? {
            return allocator.release_page_span(&page_span);
        }

        if self.cache(page_span) {
            return Ok(());
        }

        allocator.recycle_cached_span(page_span)?;

        Ok(())
    }

    /// Flush this cache back into the allocator free spans.
    pub(crate) fn flush(&mut self, allocator: &Allocator) -> HeapResult<()> {
        for span in self.drain() {
            allocator.recycle_cached_span(span)?;
        }

        Ok(())
    }

    /// Return the currently cached byte count.
    pub(crate) fn cached_bytes(&self, page_size_bytes: usize) -> u64 {
        self.cached_pages as u64 * page_size_bytes as u64
    }

    /// Return one cached span that satisfies the requested page count.
    pub(crate) fn allocate(&mut self, page_count: usize) -> Option<PageSpan> {
        if page_count == 0 {
            return Some(PageSpan::empty());
        }

        // prefer the most recently released span first
        let span_index = self
            .spans
            .iter()
            .rposition(|span| span.len() >= page_count)?;
        let span = self.spans.swap_remove(span_index);
        self.cached_pages -= span.len();
        if span.len() == page_count {
            return Some(span);
        }

        let (block, remainder) = span.split_prefix_unchecked(page_count);
        if !remainder.is_empty() {
            self.cache(remainder);
        }

        Some(block)
    }

    /// Cache one contiguous span when it fits this cache.
    pub(crate) fn cache(&mut self, span: PageSpan) -> bool {
        if span.is_empty() {
            return true;
        }

        if self.page_capacity == 0 {
            return false;
        }

        let next_cached_pages = self.cached_pages + span.len();
        if next_cached_pages > self.page_capacity {
            return false;
        }

        let span = self.coalesce(span);

        self.cached_pages = next_cached_pages;
        self.spans.push(span);

        true
    }

    /// Drain every cached span.
    pub(crate) fn drain(&mut self) -> impl Iterator<Item = PageSpan> + '_ {
        self.cached_pages = 0;

        self.spans.drain(..)
    }

    /// Merge one span with any immediately adjacent cached spans.
    fn coalesce(&mut self, mut span: PageSpan) -> PageSpan {
        let mut span_index = 0;
        while span_index < self.spans.len() {
            let candidate = self.spans[span_index];
            let merged = if candidate.is_immediately_before(span) {
                Some(Self::merged_span(candidate, span))
            } else if span.is_immediately_before(candidate) {
                Some(Self::merged_span(span, candidate))
            } else {
                None
            };

            if let Some(merged) = merged {
                self.spans.swap_remove(span_index);
                span = merged;
                continue;
            }

            span_index += 1;
        }

        span
    }

    /// Merge two adjacent cached spans.
    fn merged_span(left: PageSpan, right: PageSpan) -> PageSpan {
        debug_assert!(left.is_immediately_before(right));

        PageSpan::from_raw(left.first_page, left.page_count + right.page_count)
    }
}
