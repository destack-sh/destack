use serde::{Deserialize, Serialize};

use crate::INLINE_PAGE_PATCH_COUNT;

/// One stable arena page identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[repr(transparent)]
pub struct PageId(u32);

impl PageId {
    /// Create one page identifier.
    pub const fn new(index: usize) -> Self {
        Self(index as u32)
    }

    /// Return the zero-based page index.
    pub const fn index(self) -> usize {
        self.0 as usize
    }

    /// Return the raw page identifier value.
    pub const fn raw(self) -> u32 {
        self.0
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
    pub const fn new(first_page: PageId, page_count: usize) -> Self {
        Self {
            first_page,
            page_count: page_count as u32,
        }
    }

    /// Return the number of pages in this run.
    pub const fn len(self) -> usize {
        self.page_count as usize
    }

    /// Report whether this run is empty.
    pub const fn is_empty(self) -> bool {
        self.page_count == 0
    }

    /// Return one page id by run-local index.
    pub fn page(self, index: usize) -> Option<PageId> {
        if index >= self.len() {
            return None;
        }

        Some(PageId::new(self.first_page.index().saturating_add(index)))
    }

    /// Return every page id in this run.
    pub fn page_ids(self) -> impl Iterator<Item = PageId> {
        let start = self.first_page.index();
        let end = start.saturating_add(self.len());

        (start..end).map(PageId::new)
    }
}

/// One overridden page inside one logical page map.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PagePatch {
    /// The zero-based page index inside the logical sequence.
    pub page_index: u32,
    /// The single-page run now stored at that index.
    pub run: PageRun,
}

impl PagePatch {
    /// Return one empty page patch sentinel.
    pub const fn empty() -> Self {
        Self {
            page_index: u32::MAX,
            run: PageRun::empty(),
        }
    }
}

/// One logical page map for one allocation or span.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PageMap {
    /// The physical base run for this mapping.
    base_run: PageRun,
    /// The number of active page patches.
    patch_count: u8,
    /// The inline page patches that override specific indices.
    patches: [PagePatch; INLINE_PAGE_PATCH_COUNT],
}

impl PageMap {
    /// Return one empty logical page map.
    pub const fn empty() -> Self {
        Self {
            base_run: PageRun::empty(),
            patch_count: 0,
            patches: [PagePatch::empty(); INLINE_PAGE_PATCH_COUNT],
        }
    }

    /// Build one logical page map from one contiguous run.
    pub const fn from_run(run: PageRun) -> Self {
        Self {
            base_run: run,
            patch_count: 0,
            patches: [PagePatch::empty(); INLINE_PAGE_PATCH_COUNT],
        }
    }

    /// Return the number of pages in this logical map.
    pub const fn len(&self) -> usize {
        self.base_run.len()
    }

    /// Report whether this map is empty.
    pub const fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Report whether the base run still contributes any live pages.
    pub fn has_base_pages(&self) -> bool {
        self.patches().len() < self.len()
    }

    /// Return the physical base run.
    pub const fn base_run(&self) -> PageRun {
        self.base_run
    }

    /// Return the patches for this map.
    pub fn patches(&self) -> &[PagePatch] {
        &self.patches[..self.patch_count as usize]
    }

    /// Report whether this map can record one more distinct patch inline.
    pub fn can_patch(&self, page_index: usize) -> bool {
        patch_at(self.patches(), page_index).is_some()
            || (self.patch_count as usize) < INLINE_PAGE_PATCH_COUNT
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

        let patches = self.patches();

        if let Some(patch) = patch_at(patches, index) {
            return Some(PageSlot {
                run: patch.run,
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

    /// Return every effective page id in this logical map.
    pub fn page_ids(&self) -> impl Iterator<Item = PageId> + '_ {
        PageMapIter::new(self)
    }

    /// Install one single-page patch at the given logical page index.
    pub fn set_patch(&mut self, page_index: usize, run: PageRun) -> bool {
        let page_index = page_index as u32;

        let active_count = self.patch_count as usize;
        let active = &self.patches[..active_count];
        let result = active.binary_search_by_key(&page_index, |patch| patch.page_index);

        if let Ok(entry_index) = result {
            self.patches[entry_index].run = run;
            return true;
        }

        if active_count >= INLINE_PAGE_PATCH_COUNT {
            return false;
        }

        let entry_index = match result {
            Ok(entry_index) | Err(entry_index) => entry_index,
        };

        for slot_index in (entry_index..active_count).rev() {
            self.patches[slot_index + 1] = self.patches[slot_index];
        }

        self.patches[entry_index] = PagePatch { page_index, run };
        self.patch_count = self.patch_count.saturating_add(1);

        true
    }
}

/// One resolved physical page slot inside one page map.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PageSlot {
    /// The physical run that backs this logical page.
    pub run: PageRun,
    /// The zero-based page index inside that run.
    pub run_page_index: usize,
    /// Whether this page comes from one patch run.
    pub is_patched: bool,
}

/// One iterator over the effective page ids in one logical page map.
struct PageMapIter<'a> {
    /// The page map being iterated.
    page_map: &'a PageMap,
    /// The next logical page index.
    page_index: usize,
    /// The next patch to consider.
    patch_index: usize,
}

impl<'a> PageMapIter<'a> {
    /// Create one page iterator for one logical map.
    fn new(page_map: &'a PageMap) -> Self {
        Self {
            page_map,
            page_index: 0,
            patch_index: 0,
        }
    }
}

impl Iterator for PageMapIter<'_> {
    type Item = PageId;

    fn next(&mut self) -> Option<Self::Item> {
        // stop once the logical map is exhausted
        if self.page_index >= self.page_map.len() {
            return None;
        }

        // advance the logical cursor first
        let patches = self.page_map.patches();
        let current_index = self.page_index;
        self.page_index = self.page_index.saturating_add(1);

        // yield one patched page when present
        if let Some(patch) = patches.get(self.patch_index)
            && patch.page_index as usize == current_index
        {
            self.patch_index = self.patch_index.saturating_add(1);

            return patch.run.page(0);
        }

        // otherwise fall back to the base run
        self.page_map.base_run.page(current_index)
    }
}

/// Return one patch by logical page index.
fn patch_at(patches: &[PagePatch], page_index: usize) -> Option<PagePatch> {
    let page_index = page_index as u32;
    let patch_index = patches.binary_search_by_key(&page_index, |patch| patch.page_index);
    let patch_index = patch_index.ok()?;

    patches.get(patch_index).copied()
}
