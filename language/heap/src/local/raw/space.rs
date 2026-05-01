use std::collections::BTreeMap;
use std::sync::Arc;

use super::{
    LargeAllocation, LargeAllocationId, RawLocation, RawPageMapEntry, RawPlace, SmallSpan,
};
use crate::allocator::{AddressSpace, Allocator, PageRun, PageRunCache, SizeClassTable};
use crate::{AllocationUsage, CowTable, HeapError, HeapOptions, HeapResult, RawSpaceUsage};

/// The first non-null raw large-allocation id.
const FIRST_ALLOCATED_LARGE_ALLOCATION_ID: u64 = 1;

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
    pub(crate) partial_spans: BTreeMap<usize, Vec<usize>>,
}

/// One raw large space.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LargeSpace {
    /// The configured page width for allocations in large space.
    pub(crate) page_bytes: usize,
    /// The live raw allocations.
    pub(crate) allocations: CowTable<LargeAllocation>,
    /// The free raw allocation ids available for reuse.
    pub(crate) free_large_allocation_ids: Vec<u64>,
    /// The next raw allocation id to allocate.
    pub(crate) next_unused_large_allocation_id: u64,
}

/// One live raw allocation space over one allocator.
#[derive(Debug)]
pub struct RawSpace {
    /// The shared page allocator for every raw payload.
    pub(super) allocator: Arc<Allocator>,
    /// The local cache of reusable page runs.
    pub(crate) page_run_cache: PageRunCache,

    /// The raw small space.
    pub(crate) small: SmallSpace,
    /// The raw large space.
    pub(crate) large: LargeSpace,
    /// The owning raw metadata for each visible allocator page.
    pub(crate) page_map: Vec<Option<RawPageMapEntry>>,
    /// The next unused byte offset in raw space.
    pub(crate) next_offset: usize,
    /// The fixed virtual mapping for live raw bytes.
    pub(crate) mapping: AddressSpace,

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
        let page_run_cache = PageRunCache::new(allocator.pages_per_chunk());
        let next_offset = allocator.page_bytes();
        let mapping = AddressSpace::reserve(options.raw_space_bytes, options.page_bytes)?;

        Ok(Self {
            allocator,
            page_run_cache,
            small: SmallSpace {
                size_classes: options.size_classes.clone(),
                span_bytes: options.raw_small_bytes,
                spans: CowTable::new(),
                partial_spans: BTreeMap::new(),
            },
            large: LargeSpace {
                page_bytes: options.page_bytes,
                allocations: CowTable::new(),
                free_large_allocation_ids: Vec::new(),
                next_unused_large_allocation_id: FIRST_ALLOCATED_LARGE_ALLOCATION_ID,
            },
            page_map: Vec::new(),
            next_offset,
            mapping,
            usage: AllocationUsage::default(),
        })
    }

    /// Return the shared page allocator.
    pub fn allocator(&self) -> &Arc<Allocator> {
        &self.allocator
    }

    /// Return the number of live raw allocations.
    pub fn allocation_count(&self) -> usize {
        self.usage.allocation_count()
    }

    /// Return the exact retained raw allocator-page bytes.
    pub fn retained_bytes(&self) -> u64 {
        self.allocator.retained_bytes_for_page_runs(
            self.small.spans.iter().map(|span| &span.pages).chain(
                self.large
                    .allocations
                    .iter()
                    .map(|allocation| &allocation.pages),
            ),
        ) + self
            .page_run_cache
            .cached_bytes(self.allocator.page_bytes())
    }

    /// Return the exact live usage for this raw space.
    pub fn usage(&self) -> RawSpaceUsage {
        RawSpaceUsage {
            allocation_count: self.usage.allocation_count(),
            allocated_bytes: self.usage.allocated_bytes(),
            retained_bytes: self.retained_bytes(),
        }
    }

    /// Allocate one zeroed page run through the local page-run cache.
    pub(crate) fn allocate_page_run_zeroed(&mut self, byte_len: usize) -> HeapResult<PageRun> {
        self.page_run_cache
            .allocate_pages(&self.allocator, byte_len)
    }

    /// Release one page run through the local page-run cache.
    pub(crate) fn release_page_run(&mut self, page_run: PageRun) -> HeapResult<()> {
        self.page_run_cache
            .release_page_run(&self.allocator, page_run)
    }

    /// Flush transient cache state before one exact branch boundary.
    pub(crate) fn flush_branch_boundary(&mut self) -> HeapResult<()> {
        self.page_run_cache.flush(&self.allocator)
    }

    /// Return one live raw large allocation by id.
    pub(super) fn large_allocation(
        &self,
        allocation_id: LargeAllocationId,
    ) -> Option<&LargeAllocation> {
        let index = allocation_id.index().ok()?;
        let allocation = self.large.allocations.get(index)?;

        if allocation.is_live {
            Some(allocation)
        } else {
            None
        }
    }

    /// Return one live raw large allocation mutably by id.
    pub(super) fn large_allocation_mut(
        &mut self,
        allocation_id: LargeAllocationId,
    ) -> Option<&mut LargeAllocation> {
        let index = allocation_id.index().ok()?;
        let allocation = self.large.allocations.get_mut(index)?;

        if allocation.is_live {
            Some(allocation)
        } else {
            None
        }
    }

    /// Return one live raw span by index.
    pub(super) fn span(&self, span_index: usize) -> Option<&SmallSpan> {
        self.small.spans.get(span_index)
    }

    /// Return one live raw span mutably by index.
    pub(super) fn span_mut(&mut self, span_index: usize) -> Option<&mut SmallSpan> {
        self.small.spans.get_mut(span_index)
    }

    /// Return the page-map entry for one logical page.
    pub(crate) fn page_entry(&self, page_index: usize) -> Option<RawPageMapEntry> {
        self.page_map.get(page_index).copied().flatten()
    }

    /// Record one page-map entry for every page in one logical page run.
    pub(crate) fn map_page_run(
        &mut self,
        first_offset: usize,
        page_run: &PageRun,
        mut entry: impl FnMut(usize) -> RawPageMapEntry,
    ) {
        let first_page_index = first_offset / self.allocator.page_bytes();

        for logical_page_index in 0..page_run.len() {
            let page_index = first_page_index + logical_page_index;

            if self.page_map.len() <= page_index {
                self.page_map.resize(page_index + 1, None);
            }

            self.page_map[page_index] = Some(entry(logical_page_index));
        }
    }

    /// Clear every page-map entry for one logical page run.
    pub(crate) fn unmap_page_run(&mut self, first_offset: usize, page_run: &PageRun) {
        let first_page_index = first_offset / self.allocator.page_bytes();

        for logical_page_index in 0..page_run.len() {
            let page_index = first_page_index + logical_page_index;

            if let Some(entry) = self.page_map.get_mut(page_index) {
                *entry = None;
            }
        }
    }

    /// Return the resolved location for one live raw pointer.
    pub(crate) fn resolve_location(&self, pointer: crate::RawPointer) -> Option<RawLocation> {
        let page_bytes = self.allocator.page_bytes();
        let page_index = pointer.offset() / page_bytes;
        let page_offset = pointer.offset() % page_bytes;
        let entry = self.page_entry(page_index)?;

        match entry {
            RawPageMapEntry::Small {
                span_index,
                logical_page_index,
            } => {
                let span = self.span(span_index)?;
                let logical_byte_offset =
                    logical_page_index * self.allocator.page_bytes() + page_offset;
                let slot_index = logical_byte_offset / span.class.size_class;
                let slot_offset = logical_byte_offset % span.class.size_class;
                if slot_index >= span.slot_count || !span.occupied.contains(slot_index) {
                    return None;
                }

                let byte_len = span.class.byte_len;
                if byte_len == 0 {
                    if slot_offset != 0 {
                        return None;
                    }
                } else if slot_offset >= byte_len {
                    return None;
                }

                let slot_base_offset = slot_index * span.class.size_class;
                let base_offset = span.first_offset + slot_base_offset;
                let slot = crate::allocator::SpanSlot::new(span_index, slot_index).ok()?;

                Some(RawLocation {
                    place: RawPlace::Small(slot),
                    base: crate::RawPointer::new(base_offset),
                    byte_offset: slot_offset,
                    byte_len,
                })
            }
            RawPageMapEntry::Large {
                allocation_id,
                logical_page_index,
            } => {
                let allocation = self.large_allocation(allocation_id)?;
                let logical_byte_offset =
                    logical_page_index * self.allocator.page_bytes() + page_offset;
                if allocation.len == 0 {
                    if logical_byte_offset != 0 {
                        return None;
                    }
                } else if logical_byte_offset >= allocation.len {
                    return None;
                }

                Some(RawLocation {
                    place: RawPlace::Large(allocation_id),
                    base: crate::RawPointer::new(allocation.first_offset),
                    byte_offset: logical_byte_offset,
                    byte_len: allocation.len,
                })
            }
        }
    }

    /// Return the base pointer for one raw place.
    pub(crate) fn base_pointer(&self, place: RawPlace) -> HeapResult<crate::RawPointer> {
        let base_offset = match place {
            RawPlace::Small(slot) => {
                let Some(span) = self.span(slot.span_index()) else {
                    return Err(HeapError::MissingSpan {
                        span_index: slot.span_index(),
                    });
                };
                let slot_offset = span.class.size_class * slot.slot_index();

                span.first_offset + slot_offset
            }
            RawPlace::Large(allocation_id) => {
                let Some(allocation) = self.large_allocation(allocation_id) else {
                    return Err(HeapError::MissingLargeAllocation {
                        allocation_id: allocation_id.id(),
                    });
                };

                allocation.first_offset
            }
        };

        Ok(crate::RawPointer::new(base_offset))
    }

    /// Reserve one logical raw-space byte range.
    pub(crate) fn reserve_space_range(&mut self, byte_len: usize) -> HeapResult<usize> {
        debug_assert!(self.next_offset <= self.mapping.byte_len());

        let first_offset = align_up(self.next_offset, self.allocator.page_bytes());
        let next_offset = first_offset + byte_len;
        if next_offset > self.mapping.byte_len() {
            return Err(HeapError::InvalidByteRange {
                start: first_offset,
                len: byte_len,
                capacity: self.mapping.byte_len(),
            });
        }

        self.next_offset = next_offset;

        Ok(first_offset)
    }

    /// Return the page runs reachable from this live raw space.
    pub(crate) fn live_page_runs(&self) -> Vec<PageRun> {
        let mut page_runs = Vec::new();

        // collect raw span page runs first
        page_runs.extend(self.small.spans.iter().map(|span| span.pages));

        // collect live large-allocation page runs next
        page_runs.extend(
            self.large
                .allocations
                .iter()
                .filter(|allocation| allocation.is_live)
                .map(|allocation| allocation.pages),
        );

        page_runs
    }

    /// Release allocator page runs owned by this raw space.
    fn close(&mut self) -> HeapResult<()> {
        for page_run in self.live_page_runs() {
            self.release_page_run(page_run)?;
        }

        self.page_run_cache.flush(&self.allocator)
    }
}

impl Drop for RawSpace {
    fn drop(&mut self) {
        let _ = self.close();
    }
}

/// Return the offset rounded up to one allocation boundary.
fn align_up(byte_len: usize, alignment_bytes: usize) -> usize {
    let alignment_bytes = alignment_bytes.max(1);

    byte_len.div_ceil(alignment_bytes) * alignment_bytes
}
