use std::collections::BTreeMap;
use std::sync::Arc;

use destack_memory::AddressSpace;

use super::{
    LargeBlock, LargeBlockId, RawExtent, RawPageMapEntry, RawSmallSpanClass, RawStorage, SmallSpan,
};
use crate::allocator::{Allocator, PageSpan, PageSpanCache, SizeClassTable, Slot};
use crate::{
    AllocationUsage, CowTable, HeapError, HeapOptions, HeapResult, RawPointer, RawSpaceUsage,
};

/// The first non-null raw large-block id.
const FIRST_ALLOCATED_LARGE_ALLOCATION_ID: u64 = 1;

/// One live raw block space over one allocator.
#[derive(Debug)]
pub struct RawSpace {
    /// The shared page allocator for every raw payload.
    pub(super) allocator: Arc<Allocator>,
    /// The local cache of reusable page spans.
    pub(crate) page_span_cache: PageSpanCache,

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
    /// Return the base native address for direct raw access.
    #[inline(always)]
    pub fn base_address(&self) -> usize {
        self.mapping.base_address()
    }

    /// Create one raw space with explicit options.
    pub fn with_options(
        allocator: Arc<Allocator>,
        options: &HeapOptions,
    ) -> Result<Self, HeapError> {
        options.validate_local()?;
        options.validate_allocator(&allocator)?;
        let page_span_cache = PageSpanCache::new(allocator.pages_per_chunk());
        let next_offset = allocator.page_size_bytes();
        let mapping = AddressSpace::reserve(options.raw_space_size_bytes, options.page_size_bytes)?;

        Ok(Self {
            allocator,
            page_span_cache,
            small: SmallSpace {
                size_classes: options.size_classes.clone(),
                span_size_bytes: options.raw_small_size_bytes,
                spans: CowTable::new(),
                partial_spans: BTreeMap::new(),
            },
            large: LargeSpace {
                blocks: CowTable::new(),
                free_large_block_ids: Vec::new(),
                next_unused_large_block_id: FIRST_ALLOCATED_LARGE_ALLOCATION_ID,
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

    /// Return the number of live raw blocks.
    pub fn allocation_count(&self) -> usize {
        self.usage.allocation_count()
    }

    /// Return the exact retained raw allocator-page bytes.
    pub fn retained_bytes(&self) -> u64 {
        self.allocator.retained_bytes_for_page_spans(
            self.small
                .spans
                .iter()
                .map(|span| &span.pages)
                .chain(self.large.blocks.iter().map(|block| &block.pages)),
        ) + self
            .page_span_cache
            .cached_bytes(self.allocator.page_size_bytes())
    }

    /// Return the exact live usage for this raw space.
    pub fn usage(&self) -> RawSpaceUsage {
        RawSpaceUsage {
            allocation_count: self.usage.allocation_count(),
            allocated_bytes: self.usage.allocated_bytes(),
            retained_bytes: self.retained_bytes(),
        }
    }

    /// Allocate one page span through the local page-span cache.
    pub(crate) fn allocate_page_span(&mut self, byte_len: usize) -> HeapResult<PageSpan> {
        self.page_span_cache
            .allocate_pages(&self.allocator, byte_len)
    }

    /// Release one page span through the local page-span cache.
    pub(crate) fn release_page_span(&mut self, page_span: PageSpan) -> HeapResult<()> {
        self.page_span_cache
            .release_page_span(&self.allocator, page_span)
    }

    /// Flush transient cache state before one exact branch boundary.
    pub(crate) fn flush_branch_boundary(&mut self) -> HeapResult<()> {
        self.page_span_cache.flush(&self.allocator)
    }

    /// Return one live raw large block by id.
    pub(super) fn large_block(&self, block_id: LargeBlockId) -> Option<&LargeBlock> {
        let index = block_id.index().ok()?;
        let block = self.large.blocks.get(index)?;

        if block.is_live { Some(block) } else { None }
    }

    /// Return one live raw large block mutably by id.
    pub(super) fn large_block_mut(&mut self, block_id: LargeBlockId) -> Option<&mut LargeBlock> {
        let index = block_id.index().ok()?;
        let block = self.large.blocks.get_mut(index)?;

        if block.is_live { Some(block) } else { None }
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

    /// Record one page-map entry for every page in one logical page span.
    pub(crate) fn map_page_span(
        &mut self,
        first_offset: usize,
        page_span: &PageSpan,
        mut entry: impl FnMut(usize) -> RawPageMapEntry,
    ) {
        let first_page_index = first_offset / self.allocator.page_size_bytes();

        for logical_page_index in 0..page_span.len() {
            let page_index = first_page_index + logical_page_index;

            if self.page_map.len() <= page_index {
                self.page_map.resize(page_index + 1, None);
            }

            self.page_map[page_index] = Some(entry(logical_page_index));
        }
    }

    /// Clear every page-map entry for one logical page span.
    pub(crate) fn unmap_page_span(&mut self, first_offset: usize, page_span: &PageSpan) {
        let first_page_index = first_offset / self.allocator.page_size_bytes();

        for logical_page_index in 0..page_span.len() {
            let page_index = first_page_index + logical_page_index;

            if let Some(entry) = self.page_map.get_mut(page_index) {
                *entry = None;
            }
        }
    }

    /// Return the resolved extent for one live raw pointer.
    pub(crate) fn resolve_extent(&self, pointer: RawPointer) -> Option<RawExtent> {
        let page_size_bytes = self.allocator.page_size_bytes();
        let page_index = pointer.offset() / page_size_bytes;
        let page_offset = pointer.offset() % page_size_bytes;
        let entry = self.page_entry(page_index)?;

        match entry {
            RawPageMapEntry::SmallSpan {
                span_index,
                logical_page_index,
            } => {
                let span = self.span(span_index)?;
                let logical_byte_offset =
                    logical_page_index * self.allocator.page_size_bytes() + page_offset;
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
                let slot = Slot::new(span_index, slot_index).ok()?;

                Some(RawExtent {
                    storage: RawStorage::SmallSlot(slot),
                    base: RawPointer::new(base_offset),
                    byte_offset: slot_offset,
                    byte_len,
                })
            }
            RawPageMapEntry::LargeBlock {
                block_id,
                logical_page_index,
            } => {
                let block = self.large_block(block_id)?;
                let logical_byte_offset =
                    logical_page_index * self.allocator.page_size_bytes() + page_offset;
                if block.byte_len == 0 {
                    if logical_byte_offset != 0 {
                        return None;
                    }
                } else if logical_byte_offset >= block.byte_len {
                    return None;
                }

                Some(RawExtent {
                    storage: RawStorage::LargeBlock(block_id),
                    base: RawPointer::new(block.first_offset),
                    byte_offset: logical_byte_offset,
                    byte_len: block.byte_len,
                })
            }
        }
    }

    /// Return the base pointer for one raw storage.
    pub(crate) fn base_pointer(&self, storage: RawStorage) -> HeapResult<RawPointer> {
        let base_offset = match storage {
            RawStorage::SmallSlot(slot) => {
                let Some(span) = self.span(slot.span_index()) else {
                    return Err(HeapError::internal("missing span"));
                };
                let slot_offset = span.class.size_class * slot.slot_index();

                span.first_offset + slot_offset
            }
            RawStorage::LargeBlock(block_id) => {
                let Some(block) = self.large_block(block_id) else {
                    return Err(HeapError::internal("missing large block"));
                };

                block.first_offset
            }
        };

        Ok(RawPointer::new(base_offset))
    }

    /// Reserve one logical raw-space byte range with the given alignment.
    pub(crate) fn reserve_space_range_aligned(
        &mut self,
        byte_len: usize,
        alignment: usize,
    ) -> HeapResult<usize> {
        debug_assert!(self.next_offset <= self.mapping.byte_len());

        let alignment = alignment.max(self.allocator.page_size_bytes());
        let first_offset = align_up(self.next_offset, alignment);
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

    /// Return the page spans reachable from this live raw space.
    pub(crate) fn live_page_spans(&self) -> Vec<PageSpan> {
        let mut page_spans = Vec::new();

        // collect raw span page spans first
        page_spans.extend(self.small.spans.iter().map(|span| span.pages));

        // collect live large-block page spans next
        page_spans.extend(
            self.large
                .blocks
                .iter()
                .filter(|block| block.is_live)
                .map(|block| block.pages),
        );

        page_spans
    }

    /// Release allocator page spans owned by this raw space.
    fn close(&mut self) -> HeapResult<()> {
        for page_span in self.live_page_spans() {
            self.release_page_span(page_span)?;
        }

        self.page_span_cache.flush(&self.allocator)
    }
}

impl Drop for RawSpace {
    fn drop(&mut self) {
        let _ = self.close();
    }
}

/// One raw small space.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SmallSpace {
    /// The configured size-class table.
    pub(crate) size_classes: SizeClassTable,
    /// The configured span width.
    pub(crate) span_size_bytes: usize,
    /// The live raw spans.
    pub(crate) spans: CowTable<SmallSpan>,
    /// The reusable non-full spans per raw small-span class.
    pub(crate) partial_spans: BTreeMap<RawSmallSpanClass, Vec<usize>>,
}

/// One raw large space.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LargeSpace {
    /// The live raw blocks.
    pub(crate) blocks: CowTable<LargeBlock>,
    /// The free raw block ids available for reuse.
    pub(crate) free_large_block_ids: Vec<u64>,
    /// The next raw block id to allocate.
    pub(crate) next_unused_large_block_id: u64,
}

/// Return the offset rounded up to one block boundary.
fn align_up(byte_len: usize, alignment_bytes: usize) -> usize {
    let alignment_bytes = alignment_bytes.max(1);

    byte_len.div_ceil(alignment_bytes) * alignment_bytes
}
