use serde::{Deserialize, Serialize};

use crate::{HeapError, HeapResult};

/// The number of inline page patches per logical page view.
const INLINE_PAGE_PATCH_COUNT: usize = 3;

/// One stable page identifier in an arena (segment).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[repr(transparent)]
pub struct PageId(u32);

impl PageId {
    /// Create one page identifier.
    pub fn new(index: usize) -> HeapResult<Self> {
        let index = u32::try_from(index).map_err(|_| HeapError::InvalidPageId { index })?;

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

/// One contiguous arena page run.
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
        let page_count = u32::try_from(page_count).map_err(|_| HeapError::InvalidPageRun {
            first_page,
            page_count,
        })?;
        let end_page_index = first_page.index().checked_add(page_count as usize).ok_or(
            HeapError::InvalidPageRun {
                first_page,
                page_count: page_count as usize,
            },
        )?;

        if end_page_index > (u32::MAX as usize) + 1 {
            return Err(HeapError::InvalidPageRun {
                first_page,
                page_count: page_count as usize,
            });
        }

        Ok(Self {
            first_page,
            page_count,
        })
    }

    /// Return one trusted page run from encoded parts.
    pub(crate) const fn from_raw_parts(first_page: PageId, page_count: u32) -> Self {
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

        let prefix = Self::from_raw_parts(self.first_page, page_count as u32);
        let suffix = Self::from_raw_parts(
            PageId::from_raw(self.first_page.raw() + page_count as u32),
            self.page_count - page_count as u32,
        );

        Some((prefix, suffix))
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

/// One overridden page inside one logical page view.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PagePatch {
    /// The zero-based page index inside the logical sequence.
    pub page_index: u32,
    /// The single patched page now stored at that index.
    pub page_id: PageId,
}

impl PagePatch {
    /// Report whether this patch slot is empty.
    pub const fn is_empty(self) -> bool {
        self.page_index == u32::MAX
    }

    /// Return one empty page patch sentinel.
    pub const fn empty() -> Self {
        Self {
            page_index: u32::MAX,
            page_id: PageId(0),
        }
    }
}

/// One logical page view for one allocation or span.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PageView {
    /// The physical base run for this view.
    base_run: PageRun,
    /// The inline page patches that override specific indices.
    patches: [PagePatch; INLINE_PAGE_PATCH_COUNT],
}

impl PageView {
    /// Return one empty logical page view.
    pub const fn empty() -> Self {
        Self {
            base_run: PageRun::empty(),
            patches: [PagePatch::empty(); INLINE_PAGE_PATCH_COUNT],
        }
    }

    /// Build one logical page view from one contiguous run.
    pub const fn from_run(run: PageRun) -> Self {
        Self {
            base_run: run,
            patches: [PagePatch::empty(); INLINE_PAGE_PATCH_COUNT],
        }
    }

    /// Return the number of pages in this logical view.
    pub const fn len(&self) -> usize {
        self.base_run.len()
    }

    /// Report whether this view is empty.
    pub const fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Report whether the base run still contributes any live pages.
    pub fn has_base_pages(&self) -> bool {
        self.patch_count() < self.len()
    }

    /// Return the physical base run.
    pub const fn base_run(&self) -> PageRun {
        self.base_run
    }

    /// Return the contiguous base run when this view has no patches.
    pub fn as_run(&self) -> Option<PageRun> {
        if self.patch_count() == 0 {
            return Some(self.base_run);
        }

        None
    }

    /// Return the patches for this view.
    pub fn patches(&self) -> &[PagePatch] {
        &self.patches[..self.patch_count()]
    }

    /// Return the number of active inline patches.
    #[inline]
    pub fn patch_count(&self) -> usize {
        for (patch_index, patch) in self.patches.iter().enumerate() {
            if patch.is_empty() {
                return patch_index;
            }
        }

        INLINE_PAGE_PATCH_COUNT
    }

    /// Return the remaining inline patch room.
    #[inline]
    pub fn patch_room(&self) -> usize {
        INLINE_PAGE_PATCH_COUNT - self.patch_count()
    }

    /// Report whether this view can record one more distinct patch inline.
    pub fn can_patch(&self, page_index: usize) -> bool {
        self.patch_at(page_index).is_some() || self.patch_room() > 0
    }

    /// Return the effective page id by logical page index.
    pub fn page(&self, index: usize) -> Option<PageId> {
        let slot = self.slot(index)?;
        slot.run.page(slot.run_page_index)
    }

    /// Return the effective physical slot by logical page index.
    pub fn slot(&self, index: usize) -> Option<PageSlot> {
        if index >= self.len() {
            return None;
        }

        if let Some(patch) = self.patch_at(index) {
            return Some(PageSlot {
                run: PageRun::single_page(patch.page_id),
                run_page_index: 0,
                is_patched: true,
            });
        }

        Some(PageSlot {
            run: self.base_run,
            run_page_index: index,
            is_patched: false,
        })
    }

    /// Return every effective page id in this logical view.
    pub fn page_ids(&self) -> impl Iterator<Item = PageId> + '_ {
        PageViewIter::new(self)
    }

    /// Install one single-page patch at the given logical page index.
    pub(crate) fn set_patch(&mut self, page_index: usize, page_id: PageId) -> HeapResult<()> {
        // reject patch indices outside the logical page range
        if page_index >= self.len() {
            return Err(HeapError::MissingLogicalPage { page_index });
        }

        let page_index = page_index as u32;
        let active_count = self.patch_count();
        let active = &self.patches[..active_count];
        let result = active.binary_search_by_key(&page_index, |patch| patch.page_index);

        // update one existing patch in place
        if let Ok(entry_index) = result {
            self.patches[entry_index].page_id = page_id;

            return Ok(());
        }

        // reject new patches once the inline patch set is full
        if active_count >= INLINE_PAGE_PATCH_COUNT {
            return Err(HeapError::PagePatchCapacityExceeded {
                page_count: self.len(),
                patch_capacity: INLINE_PAGE_PATCH_COUNT,
            });
        }

        let entry_index = match result {
            Ok(entry_index) | Err(entry_index) => entry_index,
        };

        // make room for the new sorted patch entry
        for slot_index in (entry_index..active_count).rev() {
            self.patches[slot_index + 1] = self.patches[slot_index];
        }

        // install the new patch entry
        self.patches[entry_index] = PagePatch {
            page_index,
            page_id,
        };

        Ok(())
    }

    /// Return one patch by logical page index.
    fn patch_at(&self, page_index: usize) -> Option<PagePatch> {
        let page_index = page_index as u32;
        let patch_index = self
            .patches()
            .binary_search_by_key(&page_index, |patch| patch.page_index);
        let patch_index = patch_index.ok()?;

        self.patches.get(patch_index).copied()
    }
}

/// One resolved physical page slot inside one page view.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PageSlot {
    /// The physical run that backs this logical page.
    pub run: PageRun,
    /// The zero-based page index inside that run.
    pub run_page_index: usize,
    /// Whether this page comes from one patch run.
    pub is_patched: bool,
}

/// One iterator over the effective page ids in one logical page view.
struct PageViewIter<'a> {
    /// The page view being iterated.
    page_view: &'a PageView,
    /// The next logical page index.
    page_index: usize,
    /// The next patch to consider.
    patch_index: usize,
}

impl<'a> PageViewIter<'a> {
    /// Create one page iterator for one logical view.
    fn new(page_view: &'a PageView) -> Self {
        Self {
            page_view,
            page_index: 0,
            patch_index: 0,
        }
    }
}

impl Iterator for PageViewIter<'_> {
    type Item = PageId;

    fn next(&mut self) -> Option<Self::Item> {
        // stop once the logical view is exhausted
        if self.page_index >= self.page_view.len() {
            return None;
        }

        // advance the logical cursor first
        let patches = self.page_view.patches();
        let current_index = self.page_index;
        self.page_index += 1;

        // yield one patched page when present
        if let Some(patch) = patches.get(self.patch_index)
            && patch.page_index as usize == current_index
        {
            self.patch_index += 1;

            return Some(patch.page_id);
        }

        // otherwise fall back to the base run
        self.page_view.base_run.page(current_index)
    }
}
