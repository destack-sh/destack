use std::collections::BTreeMap;
use std::sync::Arc;

use super::{LargeEntry, LargeEntryId, RawLocation, RawPageOwner, RawStorage, SmallSpan};
use crate::allocator::{Allocator, PageId, PageRunCache, PageView, SizeClassTable};
use crate::{AllocationUsage, CowTable, HeapError, HeapOptions, HeapResult, RawSpaceUsage};

/// The first non-null raw large-entry id.
const FIRST_ALLOCATED_LARGE_ENTRY_ID: u64 = 1;

/// One raw small space.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SmallSpace {
    /// The configured size-class table.
    pub(crate) size_classes: SizeClassTable,
    /// The configured span width.
    pub(crate) span_bytes: usize,
    /// The live raw spans.
    pub(crate) spans: CowTable<SmallSpan>,
    /// The reusable non-full spans per logical byte length.
    pub(crate) available_spans: BTreeMap<usize, Vec<usize>>,
}

/// One raw large space.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LargeSpace {
    /// The configured page width for entries in large space.
    pub(crate) page_bytes: usize,
    /// The live raw entries.
    pub(crate) entries: CowTable<LargeEntry>,
    /// The free raw entry ids available for reuse.
    pub(crate) free_large_entry_ids: Vec<u64>,
    /// The next raw entry id to allocate.
    pub(crate) next_unused_large_entry_id: u64,
}

/// One live raw entry space rooted in one allocator.
#[derive(Debug)]
pub struct RawSpace {
    /// The shared page allocator for every raw payload.
    pub(super) allocator: Arc<Allocator>,
    /// The local front-end cache of reusable page runs.
    pub(crate) page_run_cache: PageRunCache,

    /// The raw small space.
    pub(crate) small: SmallSpace,
    /// The raw large space.
    pub(crate) large: LargeSpace,
    /// The owning raw metadata for each visible allocator page.
    pub(crate) page_owners: Vec<Option<RawPageOwner>>,

    /// The exact live raw usage.
    pub(crate) usage: AllocationUsage,
}

impl RawSpace {
    /// Create one raw space with explicit options.
    pub fn with_options(
        allocator: Arc<Allocator>,
        options: &HeapOptions,
    ) -> Result<Self, HeapError> {
        options.validate_local()?;
        options.validate_allocator(&allocator)?;
        let page_run_cache = PageRunCache::new(allocator.pages_per_arena());

        Ok(Self {
            allocator,
            page_run_cache,
            small: SmallSpace {
                size_classes: options.size_classes.clone(),
                span_bytes: options.raw_small_bytes,
                spans: CowTable::new(),
                available_spans: BTreeMap::new(),
            },
            large: LargeSpace {
                page_bytes: options.page_bytes,
                entries: CowTable::new(),
                free_large_entry_ids: Vec::new(),
                next_unused_large_entry_id: FIRST_ALLOCATED_LARGE_ENTRY_ID,
            },
            page_owners: Vec::new(),
            usage: AllocationUsage::default(),
        })
    }

    /// Return the shared page allocator.
    pub fn allocator(&self) -> &Arc<Allocator> {
        &self.allocator
    }

    /// Return the number of live raw entries.
    pub fn allocation_count(&self) -> usize {
        self.usage.allocation_count()
    }

    /// Return the exact retained raw bytes.
    pub fn active_bytes(&self) -> u64 {
        self.mapped_bytes()
    }

    /// Return the exact mapped raw page bytes.
    pub fn mapped_bytes(&self) -> u64 {
        self.allocator.mapped_bytes_for_page_views(
            self.small
                .spans
                .iter()
                .map(|span| &span.pages)
                .chain(self.large.entries.iter().map(|entry| &entry.pages)),
        ) + self
            .page_run_cache
            .cached_bytes(self.allocator.page_bytes())
    }

    /// Return the exact borrowed raw bytes.
    pub fn borrowed_bytes(&self) -> u64 {
        0
    }

    /// Return the exact live usage for this raw space.
    pub fn usage(&self) -> RawSpaceUsage {
        RawSpaceUsage {
            allocation_count: self.usage.allocation_count(),
            allocated_bytes: self.usage.allocated_bytes(),
            active_bytes: self.active_bytes(),
            mapped_bytes: self.mapped_bytes(),
            borrowed_bytes: self.borrowed_bytes(),
        }
    }

    /// Allocate one zeroed page view through the local page-run cache.
    pub(crate) fn allocate_page_view_zeroed(&mut self, byte_len: usize) -> HeapResult<PageView> {
        self.page_run_cache
            .allocate_zeroed(&self.allocator, byte_len)
    }

    /// Allocate one initialized page view through the local page-run cache.
    pub(crate) fn allocate_page_view_bytes(&mut self, bytes: &[u8]) -> HeapResult<PageView> {
        self.page_run_cache.allocate_bytes(&self.allocator, bytes)
    }

    /// Release one page view through the local page-run cache.
    pub(crate) fn release_page_view(&mut self, page_view: PageView) -> HeapResult<()> {
        self.page_run_cache
            .release_page_view(&self.allocator, page_view)
    }

    /// Flush transient cache state before one exact branch boundary.
    pub(crate) fn flush_branch_boundary(&mut self) {
        self.page_run_cache.flush(&self.allocator);
    }

    /// Return one live raw large entry by id.
    pub(super) fn large_entry(&self, entry_id: LargeEntryId) -> Option<&LargeEntry> {
        let index = entry_id.index().ok()?;
        let entry = self.large.entries.get(index)?;

        if entry.is_live { Some(entry) } else { None }
    }

    /// Return one live raw large entry mutably by id.
    pub(super) fn large_entry_mut(&mut self, entry_id: LargeEntryId) -> Option<&mut LargeEntry> {
        let index = entry_id.index().ok()?;
        let entry = self.large.entries.get_mut(index)?;

        if entry.is_live { Some(entry) } else { None }
    }

    /// Return one live raw span by index.
    pub(super) fn span(&self, span_index: usize) -> Option<&SmallSpan> {
        self.small.spans.get(span_index)
    }

    /// Return one live raw span mutably by index.
    pub(super) fn span_mut(&mut self, span_index: usize) -> Option<&mut SmallSpan> {
        self.small.spans.get_mut(span_index)
    }

    /// Return the visible owner for one physical page.
    pub(crate) fn page_owner(&self, page_id: PageId) -> Option<RawPageOwner> {
        self.page_owners.get(page_id.index()).copied().flatten()
    }

    /// Record one visible owner for every page in one logical page view.
    pub(crate) fn map_page_view(
        &mut self,
        page_view: &PageView,
        mut owner: impl FnMut(usize) -> RawPageOwner,
    ) -> HeapResult<()> {
        for logical_page_index in 0..page_view.len() {
            let Some(page_id) = page_view.page(logical_page_index) else {
                return Err(HeapError::MissingLogicalPage {
                    page_index: logical_page_index,
                });
            };
            let page_index = page_id.index();

            if self.page_owners.len() <= page_index {
                self.page_owners.resize(page_index + 1, None);
            }

            self.page_owners[page_index] = Some(owner(logical_page_index));
        }

        Ok(())
    }

    /// Clear every visible owner for one logical page view.
    pub(crate) fn unmap_page_view(&mut self, page_view: &PageView) -> HeapResult<()> {
        for logical_page_index in 0..page_view.len() {
            let Some(page_id) = page_view.page(logical_page_index) else {
                return Err(HeapError::MissingLogicalPage {
                    page_index: logical_page_index,
                });
            };

            if let Some(owner) = self.page_owners.get_mut(page_id.index()) {
                *owner = None;
            }
        }

        Ok(())
    }

    /// Return the resolved location for one live raw pointer.
    pub(crate) fn resolve_location(&self, pointer: crate::RawPointer) -> Option<RawLocation> {
        let (page_id, page_offset) = self.allocator.address_page_position(pointer.address())?;
        let owner = self.page_owner(page_id)?;

        match owner {
            RawPageOwner::Small {
                span_index,
                logical_page_index,
            } => {
                let span = self.span(span_index)?;
                let logical_byte_offset = logical_page_index
                    .checked_mul(self.allocator.page_bytes())?
                    .checked_add(page_offset)?;
                let slot_index = logical_byte_offset / span.class.size_class;
                let slot_offset = logical_byte_offset % span.class.size_class;
                if slot_index >= span.slot_count || !span.occupied.contains(slot_index) {
                    return None;
                }

                let byte_len = span.class.byte_len;
                if slot_offset >= byte_len {
                    return None;
                }

                let slot_base_offset = slot_index.checked_mul(span.class.size_class)?;
                let base_address = self
                    .allocator
                    .page_view_ptr(&span.pages, slot_base_offset)
                    .ok()? as *mut u8 as usize;
                let slot = crate::allocator::SpanSlot::new(span_index, slot_index).ok()?;

                Some(RawLocation {
                    storage: RawStorage::Small(slot),
                    base: crate::RawPointer::new(base_address),
                    byte_offset: slot_offset,
                    byte_len,
                })
            }
            RawPageOwner::Large {
                entry_id,
                logical_page_index,
            } => {
                let entry = self.large_entry(entry_id)?;
                let logical_byte_offset = logical_page_index
                    .checked_mul(self.allocator.page_bytes())?
                    .checked_add(page_offset)?;
                if logical_byte_offset >= entry.len {
                    return None;
                }

                let base_address =
                    self.allocator.page_view_ptr(&entry.pages, 0).ok()? as *mut u8 as usize;

                Some(RawLocation {
                    storage: RawStorage::Large(entry_id),
                    base: crate::RawPointer::new(base_address),
                    byte_offset: logical_byte_offset,
                    byte_len: entry.len,
                })
            }
        }
    }

    /// Return the base pointer for one raw storage partition.
    pub(crate) fn base_pointer(&self, storage: RawStorage) -> HeapResult<crate::RawPointer> {
        let base_address = match storage {
            RawStorage::Small(slot) => {
                let Some(span) = self.span(slot.span_index()) else {
                    return Err(HeapError::MissingSpan {
                        span_index: slot.span_index(),
                    });
                };
                let slot_offset = span.class.size_class.checked_mul(slot.slot_index()).ok_or(
                    HeapError::InvariantOverflow {
                        context: "raw slot base offset",
                    },
                )?;

                self.allocator.page_view_ptr(&span.pages, slot_offset)? as *mut u8 as usize
            }
            RawStorage::Large(entry_id) => {
                let Some(entry) = self.large_entry(entry_id) else {
                    return Err(HeapError::MissingLargeEntry {
                        entry_id: entry_id.id(),
                    });
                };

                self.allocator.page_view_ptr(&entry.pages, 0)? as *mut u8 as usize
            }
        };

        Ok(crate::RawPointer::new(base_address))
    }
}

impl Drop for RawSpace {
    fn drop(&mut self) {
        self.page_run_cache.flush(&self.allocator);
    }
}
