use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::{HeapError, HeapRepresentationError, HeapResult};

/// One stable allocator page identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[repr(transparent)]
pub struct PageId(u32);

impl PageId {
    /// Create one page identifier.
    pub fn new(index: usize) -> HeapResult<Self> {
        let index = u32::try_from(index).map_err(|_| {
            HeapError::representation(HeapRepresentationError::InvalidPageId { index })
        })?;

        Ok(Self(index))
    }

    /// Return the zero-based page index.
    pub const fn index(self) -> usize {
        self.0 as usize
    }

    /// Return the raw page identifier value.
    pub const fn raw(self) -> u32 {
        self.0
    }

    /// Return one trusted page identifier from one raw encoded value.
    pub(crate) const fn from_raw(raw: u32) -> Self {
        Self(raw)
    }
}

/// One contiguous allocator page span.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PageSpan {
    /// The first page in the span.
    pub first_page: PageId,
    /// The number of pages in the span.
    pub page_count: u32,
}

impl PageSpan {
    /// Return one empty page span.
    pub const fn empty() -> Self {
        Self {
            first_page: PageId(0),
            page_count: 0,
        }
    }

    /// Create one contiguous page span.
    pub fn new(first_page: PageId, page_count: usize) -> HeapResult<Self> {
        let page_count = u32::try_from(page_count).map_err(|_| {
            HeapError::representation(HeapRepresentationError::InvalidPageSpan {
                first_page,
                page_count,
            })
        })?;
        let end_page_index = u64::from(first_page.raw()) + u64::from(page_count);
        if end_page_index > u64::from(u32::MAX) + 1 {
            return Err(HeapError::representation(
                HeapRepresentationError::InvalidPageSpan {
                    first_page,
                    page_count: page_count as usize,
                },
            ));
        }

        Ok(Self {
            first_page,
            page_count,
        })
    }

    /// Return one trusted page span from encoded values.
    pub(crate) const fn from_raw(first_page: PageId, page_count: u32) -> Self {
        Self {
            first_page,
            page_count,
        }
    }

    /// Return the number of pages in this span.
    pub const fn len(self) -> usize {
        self.page_count as usize
    }

    /// Return the zero-based first page index in this span.
    pub const fn start_page_index(self) -> usize {
        self.first_page.index()
    }

    /// Return the zero-based exclusive end page index in this span.
    pub fn end_page_index(self) -> usize {
        self.start_page_index() + self.len()
    }

    /// Report whether this span is empty.
    pub const fn is_empty(self) -> bool {
        self.page_count == 0
    }

    /// Return one single-page span.
    pub const fn single_page(page_id: PageId) -> Self {
        Self {
            first_page: page_id,
            page_count: 1,
        }
    }

    /// Report whether this span touches the next span exactly.
    pub fn is_immediately_before(self, other: Self) -> bool {
        self.end_page_index() == other.start_page_index()
    }

    /// Split one prefix span from this span.
    pub fn split_prefix(self, page_count: usize) -> Option<(Self, Self)> {
        if page_count > self.len() {
            return None;
        }

        Some(self.split_prefix_unchecked(page_count))
    }

    /// Split one trusted prefix span from this span.
    pub(crate) fn split_prefix_unchecked(self, page_count: usize) -> (Self, Self) {
        debug_assert!(page_count <= self.len());

        let page_count = page_count as u32;
        let suffix_len = self.page_count - page_count;
        let prefix = Self::from_raw(self.first_page, page_count);
        let suffix_first_page = if suffix_len == 0 {
            PageId::from_raw(0)
        } else {
            PageId::from_raw(self.first_page.raw() + page_count)
        };
        let suffix = Self::from_raw(suffix_first_page, suffix_len);

        (prefix, suffix)
    }

    /// Return one page id by span-local index.
    pub fn page(self, index: usize) -> Option<PageId> {
        if index >= self.len() {
            return None;
        }

        Some(PageId::from_raw(self.first_page.raw() + index as u32))
    }

    /// Return every page id in this span.
    pub fn page_ids(self) -> impl Iterator<Item = PageId> {
        let start = self.first_page.index();
        let end = start + self.len();

        (start..end).map(|page_index| PageId::from_raw(page_index as u32))
    }
}

/// The largest free span stored in direct small buckets.
const SMALL_SPAN_BUCKET_LIMIT: usize = 128;

/// The free page-span set owned by one allocator.
#[derive(Debug)]
pub(crate) struct PageSpanSet {
    /// Small free spans keyed directly by page count.
    small_spans: Vec<Vec<PageSpan>>,
    /// Large free spans keyed by page count.
    large_spans: BTreeMap<usize, Vec<PageSpan>>,
    /// Free spans keyed by first page index.
    spans_by_start: BTreeMap<usize, PageSpan>,
}

impl PageSpanSet {
    /// Create one empty free-span set.
    pub(crate) fn new() -> Self {
        let mut small_spans = Vec::with_capacity(SMALL_SPAN_BUCKET_LIMIT + 1);
        small_spans.resize_with(SMALL_SPAN_BUCKET_LIMIT + 1, Vec::new);

        Self {
            small_spans,
            large_spans: BTreeMap::new(),
            spans_by_start: BTreeMap::new(),
        }
    }

    /// Allocate one free span large enough for the requested size.
    pub(crate) fn allocate(&mut self, page_count: usize) -> Option<PageSpan> {
        let small_limit = self.small_spans.len() - 1;

        // prefer small buckets first
        if page_count <= small_limit {
            for span_len in page_count..=small_limit {
                let spans = &mut self.small_spans[span_len];
                if let Some(span) = spans.pop() {
                    self.spans_by_start.remove(&span.first_page.index());

                    let (allocation, remainder) = span.split_prefix_unchecked(page_count);
                    if !remainder.is_empty() {
                        self.free(remainder);
                    }

                    return Some(allocation);
                }
            }
        }

        // search larger ordered spans
        let (span_len, span) = self
            .large_spans
            .range_mut(page_count..)
            .find_map(|(&span_len, spans)| spans.pop().map(|span| (span_len, span)))?;

        self.spans_by_start.remove(&span.first_page.index());

        if self.large_spans.get(&span_len).is_some_and(Vec::is_empty) {
            self.large_spans.remove(&span_len);
        }

        let (allocation, remainder) = span.split_prefix_unchecked(page_count);
        if !remainder.is_empty() {
            self.free(remainder);
        }

        Some(allocation)
    }

    /// Free one span into both free-span indexes.
    pub(crate) fn free(&mut self, span: PageSpan) {
        let mut span = span;

        // merge the immediate predecessor when it touches this span
        if let Some(previous_span) = self.previous(span)
            && previous_span.is_immediately_before(span)
        {
            self.remove(previous_span);
            span = Self::merged(previous_span, span);
        }

        // merge the immediate successor when it touches this span
        if let Some(next_span) = self.next(span)
            && span.is_immediately_before(next_span)
        {
            self.remove(next_span);
            span = Self::merged(span, next_span);
        }

        if span.len() < self.small_spans.len() {
            self.small_spans[span.len()].push(span);
        } else {
            self.large_spans.entry(span.len()).or_default().push(span);
        }

        self.spans_by_start.insert(span.first_page.index(), span);
    }

    /// Remove one free span from both free-span indexes.
    fn remove(&mut self, span: PageSpan) {
        if span.len() < self.small_spans.len() {
            let spans = &mut self.small_spans[span.len()];
            if let Some(span_index) = spans.iter().position(|candidate| *candidate == span) {
                spans.swap_remove(span_index);
            }
        } else if let Some(spans) = self.large_spans.get_mut(&span.len()) {
            if let Some(span_index) = spans.iter().position(|candidate| *candidate == span) {
                spans.swap_remove(span_index);
            }

            if spans.is_empty() {
                self.large_spans.remove(&span.len());
            }
        }

        self.spans_by_start.remove(&span.first_page.index());
    }

    /// Return the immediately preceding free span when one exists.
    fn previous(&self, span: PageSpan) -> Option<PageSpan> {
        self.spans_by_start
            .range(..span.start_page_index())
            .next_back()
            .map(|(_, span)| *span)
    }

    /// Return the immediately following free span when one exists.
    fn next(&self, span: PageSpan) -> Option<PageSpan> {
        self.spans_by_start
            .range(span.end_page_index()..)
            .next()
            .map(|(_, span)| *span)
    }

    /// Merge two adjacent free spans.
    fn merged(left: PageSpan, right: PageSpan) -> PageSpan {
        debug_assert!(left.is_immediately_before(right));

        PageSpan::from_raw(left.first_page, left.page_count + right.page_count)
    }
}
