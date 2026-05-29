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

/// One contiguous allocator page run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PageRun {
    /// The first page in the run.
    pub first_page: PageId,
    /// The number of pages in the run.
    pub page_count: u32,
}

impl PageRun {
    /// Return one empty page run.
    pub const fn empty() -> Self {
        Self {
            first_page: PageId(0),
            page_count: 0,
        }
    }

    /// Create one contiguous page run.
    pub fn new(first_page: PageId, page_count: usize) -> HeapResult<Self> {
        let page_count = u32::try_from(page_count).map_err(|_| {
            HeapError::representation(HeapRepresentationError::InvalidPageRun {
                first_page,
                page_count,
            })
        })?;
        let end_page_index = u64::from(first_page.raw()) + u64::from(page_count);
        if end_page_index > u64::from(u32::MAX) + 1 {
            return Err(HeapError::representation(
                HeapRepresentationError::InvalidPageRun {
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

    /// Return one trusted page run from encoded values.
    pub(crate) const fn from_raw(first_page: PageId, page_count: u32) -> Self {
        Self {
            first_page,
            page_count,
        }
    }

    /// Return the number of pages in this run.
    pub const fn len(self) -> usize {
        self.page_count as usize
    }

    /// Return the zero-based first page index in this run.
    pub const fn start_page_index(self) -> usize {
        self.first_page.index()
    }

    /// Return the zero-based exclusive end page index in this run.
    pub fn end_page_index(self) -> usize {
        self.start_page_index() + self.len()
    }

    /// Report whether this run is empty.
    pub const fn is_empty(self) -> bool {
        self.page_count == 0
    }

    /// Return one single-page run.
    pub const fn single_page(page_id: PageId) -> Self {
        Self {
            first_page: page_id,
            page_count: 1,
        }
    }

    /// Report whether this run touches the next run exactly.
    pub fn is_immediately_before(self, other: Self) -> bool {
        self.end_page_index() == other.start_page_index()
    }

    /// Split one prefix run from this run.
    pub fn split_prefix(self, page_count: usize) -> Option<(Self, Self)> {
        if page_count > self.len() {
            return None;
        }

        Some(self.split_prefix_unchecked(page_count))
    }

    /// Split one trusted prefix run from this run.
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

    /// Return one page id by run-local index.
    pub fn page(self, index: usize) -> Option<PageId> {
        if index >= self.len() {
            return None;
        }

        Some(PageId::from_raw(self.first_page.raw() + index as u32))
    }

    /// Return every page id in this run.
    pub fn page_ids(self) -> impl Iterator<Item = PageId> {
        let start = self.first_page.index();
        let end = start + self.len();

        (start..end).map(|page_index| PageId::from_raw(page_index as u32))
    }
}
