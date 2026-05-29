use std::sync::Arc;

use destack_memory::AddressSpace;
use parking_lot::{Mutex, RwLock};

use super::{SharedRawBlock, SharedRawExtent, SharedRawPageMapEntry};
use crate::allocator::{PageSpan, PageSpanCache};
use crate::{
    AllocationUsage, Allocator, HeapAllocationError, HeapError, HeapResult, Payload,
    RawAllocationShape, SharedHeapOptions, SharedRawPointer, SharedRawSpaceUsage,
};

/// One live shared raw space over one allocator.
#[derive(Debug)]
pub struct SharedRawSpace {
    /// The shared raw-space allocator for every block.
    pub(crate) allocator: Arc<Allocator>,
    /// The fixed base native address for direct raw access.
    pub(crate) base_address: usize,
    /// The shared raw-space state.
    pub(crate) state: Mutex<SharedRawState>,
    /// The owning block extent for each visible allocator page.
    pub(crate) page_map: RwLock<Vec<Option<SharedRawPageMapEntry>>>,
    /// The live shared raw-space blocks.
    pub(crate) blocks: RwLock<Vec<Arc<RwLock<SharedRawBlock>>>>,
    /// The live shared raw-space bytes.
    pub(crate) mapping: RwLock<AddressSpace>,
}

impl SharedRawSpace {
    /// Return the base native address for direct shared raw access.
    #[inline(always)]
    pub fn base_address(&self) -> usize {
        self.base_address
    }

    /// Create a new empty shared raw space over one shared allocator.
    pub fn with_allocator(allocator: Arc<Allocator>) -> HeapResult<Self> {
        let options = SharedHeapOptions {
            page_size_bytes: allocator.page_size_bytes(),
            allocator_chunk_size_bytes: allocator.chunk_size_bytes(),
            ..SharedHeapOptions::default()
        };

        Self::with_options(allocator, &options)
    }

    /// Create a new empty shared raw space over one shared allocator and options.
    pub fn with_options(
        allocator: Arc<Allocator>,
        options: &SharedHeapOptions,
    ) -> HeapResult<Self> {
        options.validate()?;
        options.validate_allocator(&allocator)?;

        let page_span_cache = PageSpanCache::new(allocator.pages_per_chunk());
        let next_offset = allocator.page_size_bytes();
        let mapping = AddressSpace::reserve(options.raw_space_size_bytes, options.page_size_bytes)?;
        let base_address = mapping.base_address();

        Ok(Self {
            allocator,
            base_address,
            state: Mutex::new(SharedRawState {
                page_span_cache,
                usage: AllocationUsage::default(),
                live_retained_bytes: 0,
                next_offset,
            }),
            page_map: RwLock::new(Vec::new()),
            blocks: RwLock::new(Vec::new()),
            mapping: RwLock::new(mapping),
        })
    }

    /// Return the configured shared page size.
    pub fn page_size_bytes(&self) -> usize {
        self.allocator.page_size_bytes()
    }

    /// Return the exact retained shared raw allocator-page bytes.
    pub fn retained_bytes(&self) -> u64 {
        let state = self.state.lock();

        state.retained_bytes(self.allocator.page_size_bytes())
    }

    /// Return the exact usage for this live shared raw space.
    pub fn usage(&self) -> SharedRawSpaceUsage {
        let state = self.state.lock();

        state.usage(self.allocator.page_size_bytes())
    }

    /// Return whether one shared raw pointer currently refers to one live block slot.
    pub fn is_live(&self, pointer: SharedRawPointer) -> bool {
        self.resolve_extent(pointer).is_ok()
    }

    /// Allocate one shared raw block.
    pub fn allocate(
        &self,
        shape: RawAllocationShape,
        block: Payload<'_>,
    ) -> HeapResult<SharedRawPointer> {
        if let Some(actual) = block.byte_len()
            && actual != shape.byte_len
        {
            return Err(HeapError::invalid_allocation(
                HeapAllocationError::ByteLengthMismatch {
                    expected: shape.byte_len,
                    actual,
                },
            ));
        }

        let pages = self.allocate_pages(shape.byte_len)?;
        let first_offset = match self.reserve_space_range(
            pages.len() * self.allocator.page_size_bytes(),
            shape.alignment,
        ) {
            Ok(first_offset) => first_offset,
            Err(error) => {
                self.release_pages(pages)?;

                return Err(error);
            }
        };

        // materialize the full block before publishing it
        if let Err(error) = self
            .mapping
            .write()
            .materialize(first_offset, pages.len() * self.allocator.page_size_bytes())
        {
            self.release_pages(pages)?;

            return Err(error.into());
        }

        let mut blocks = self.blocks.write();
        let block_index = blocks.len();
        self.map_page_span(first_offset, &pages, block_index);

        // initialize bytes before publishing the block record
        self.initialize_mapped_payload(first_offset, shape.byte_len, block);

        let record = Arc::new(RwLock::new(SharedRawBlock::new(
            first_offset,
            shape.byte_len,
            pages,
        )));
        blocks.push(record);
        drop(blocks);

        let mut state = self.state.lock();
        state.usage.allocate(shape.byte_len);
        drop(state);

        Ok(SharedRawPointer::new(first_offset))
    }

    /// Allocate raw pages outside the shared state lock.
    fn allocate_pages(&self, byte_len: usize) -> HeapResult<PageSpan> {
        let allocated_byte_len = byte_len.max(1);
        let pages = {
            let mut state = self.state.lock();

            let pages = state
                .page_span_cache
                .allocate_pages(&self.allocator, allocated_byte_len)?;
            state.retain_pages(pages, self.allocator.page_size_bytes());

            pages
        };

        Ok(pages)
    }

    /// Return unpublished raw pages to the shared page-span cache.
    fn release_pages(&self, pages: PageSpan) -> HeapResult<()> {
        let mut state = self.state.lock();

        state
            .page_span_cache
            .release_page_span(&self.allocator, pages)?;
        state.release_pages(pages, self.allocator.page_size_bytes());

        Ok(())
    }

    /// Return the projected retained-byte delta for one shared block.
    pub fn alloc_retained_byte_delta(&self, shape: RawAllocationShape) -> i64 {
        self.round_up_allocation_bytes(shape.byte_len) as i64
    }

    /// Return the remaining byte length for one shared raw pointer.
    pub fn byte_len(&self, pointer: SharedRawPointer) -> HeapResult<usize> {
        let extent = self.resolve_extent(pointer)?;

        Ok(extent.byte_len - extent.byte_offset)
    }

    /// Return the bytes for one shared raw pointer.
    pub fn read_bytes(&self, pointer: SharedRawPointer) -> HeapResult<Vec<u8>> {
        let extent = self.resolve_extent(pointer)?;
        let byte_len = extent.byte_len - extent.byte_offset;

        self.mapping
            .read()
            .read_bytes(extent.base.offset() + extent.byte_offset, byte_len)
            .map_err(HeapError::from)
    }

    /// Fill one caller-provided buffer from one shared raw pointer at one offset.
    pub fn read_bytes_into(
        &self,
        pointer: SharedRawPointer,
        start: usize,
        target: &mut [u8],
    ) -> HeapResult<()> {
        let (extent, byte_offset) = self.resolve_range(pointer, start, target.len())?;

        self.mapping
            .read()
            .read_bytes_into(extent.base.offset() + byte_offset, target)
            .map_err(HeapError::from)
    }

    /// Return one checked address for a shared raw byte range.
    pub fn address(
        &self,
        pointer: SharedRawPointer,
        start: usize,
        byte_len: usize,
    ) -> HeapResult<*const u8> {
        let (extent, byte_offset) = self.resolve_range(pointer, start, byte_len)?;

        Ok(self
            .mapping
            .read()
            .address(extent.base.offset() + byte_offset, byte_len)
            .map_err(HeapError::from)? as *const u8)
    }

    /// Return one checked mutable address for a shared raw byte range.
    pub fn address_mut(
        &self,
        pointer: SharedRawPointer,
        start: usize,
        byte_len: usize,
    ) -> HeapResult<*mut u8> {
        let (extent, byte_offset) = self.resolve_range(pointer, start, byte_len)?;

        self.mapping
            .write()
            .address(extent.base.offset() + byte_offset, byte_len)
            .map_err(HeapError::from)
    }

    /// Overwrite one byte range for one live shared raw pointer.
    pub fn write_bytes(
        &self,
        pointer: SharedRawPointer,
        start: usize,
        bytes: &[u8],
    ) -> HeapResult<()> {
        let (extent, byte_offset) = self.resolve_range(pointer, start, bytes.len())?;

        self.mapping
            .write()
            .write_bytes(extent.base.offset() + byte_offset, bytes)?;

        Ok(())
    }

    /// Replace the bytes for one shared raw pointer.
    pub fn replace_bytes(
        &self,
        pointer: SharedRawPointer,
        bytes: &[u8],
    ) -> HeapResult<SharedRawPointer> {
        let extent = self.resolve_extent(pointer)?;
        let block = self.block(pointer, extent.block_index)?;
        let mut state = self.state.lock();
        let next_pages = state
            .page_span_cache
            .allocate_pages(&self.allocator, bytes.len().max(1))?;
        let mut block = block.write();

        if block.is_vacant() {
            state
                .page_span_cache
                .release_page_span(&self.allocator, next_pages)?;
            state.release_pages(next_pages, self.allocator.page_size_bytes());

            return Err(HeapError::invalid_shared_raw_pointer(pointer));
        }

        let previous_pages = block.pages;
        let first_offset = block.first_offset;
        let previous_byte_len = block.byte_len;

        // materialize the replacement pages before publishing them
        if let Err(error) = self.mapping.write().materialize(
            first_offset,
            next_pages.len() * self.allocator.page_size_bytes(),
        ) {
            state
                .page_span_cache
                .release_page_span(&self.allocator, next_pages)?;
            state.release_pages(next_pages, self.allocator.page_size_bytes());

            return Err(error.into());
        }

        // write replacement bytes before publishing the new page span
        self.write_mapped_bytes(first_offset, bytes);

        self.replace_page_span(
            first_offset,
            &previous_pages,
            &next_pages,
            extent.block_index,
        );

        block.pages = next_pages;
        block.byte_len = bytes.len();

        drop(block);

        state.usage.resize(previous_byte_len, bytes.len());
        state
            .page_span_cache
            .release_page_span(&self.allocator, previous_pages)?;
        state.release_pages(previous_pages, self.allocator.page_size_bytes());

        Ok(SharedRawPointer::new(first_offset))
    }

    /// Free one shared raw-space block.
    pub fn free(&self, pointer: SharedRawPointer) -> HeapResult<()> {
        let extent = self.resolve_extent(pointer)?;
        let block = self.block(pointer, extent.block_index)?;
        let mut state = self.state.lock();
        let mut block = block.write();

        if block.is_vacant() {
            return Err(HeapError::invalid_shared_raw_pointer(pointer));
        }

        let pages = block.pages;
        let first_offset = block.first_offset;
        let previous_byte_len = block.byte_len as u64;

        state.usage.check_free(previous_byte_len);

        block.retire();
        drop(block);

        self.unmap_page_span(first_offset, &pages);
        state.usage.free(previous_byte_len);
        state
            .page_span_cache
            .release_page_span(&self.allocator, pages)?;
        state.release_pages(pages, self.allocator.page_size_bytes());

        Ok(())
    }

    /// Return the projected retained-byte delta for one shared replacement.
    pub fn replace_retained_byte_delta(
        &self,
        pointer: SharedRawPointer,
        next_byte_len: usize,
    ) -> HeapResult<i64> {
        let extent = self.resolve_extent(pointer)?;
        let block = self.block(pointer, extent.block_index)?;
        let block = block.read();

        if block.is_vacant() {
            return Err(HeapError::invalid_shared_raw_pointer(pointer));
        }

        let previous_retained_bytes = self.round_up_allocation_bytes(block.byte_len);
        let next_retained_bytes = self.round_up_allocation_bytes(next_byte_len);

        Ok(next_retained_bytes as i64 - previous_retained_bytes as i64)
    }

    /// Return the page-rounded retained bytes for one shared block.
    fn round_up_allocation_bytes(&self, byte_len: usize) -> u64 {
        let page_size_bytes = self.page_size_bytes() as u64;
        let byte_len = byte_len.max(1) as u64;

        byte_len.div_ceil(page_size_bytes) * page_size_bytes
    }

    /// Return the current live raw block page spans.
    fn live_page_spans(&self, blocks: &[Arc<RwLock<SharedRawBlock>>]) -> Vec<PageSpan> {
        let mut pages = Vec::with_capacity(blocks.len());

        for block in blocks {
            let block = block.read();

            if block.is_vacant() {
                continue;
            }

            pages.push(block.pages);
        }

        pages
    }

    /// Release allocator page spans owned by this shared raw space.
    fn close(&mut self) -> HeapResult<()> {
        let blocks = self.blocks.read();
        let page_spans = self.live_page_spans(&blocks);
        drop(blocks);

        let mut state = self.state.lock();
        for page_span in page_spans {
            state
                .page_span_cache
                .release_page_span(&self.allocator, page_span)?;
        }

        state.page_span_cache.flush(&self.allocator)
    }

    /// Return one live shared raw block by index.
    fn block(
        &self,
        pointer: SharedRawPointer,
        block_index: usize,
    ) -> HeapResult<Arc<RwLock<SharedRawBlock>>> {
        self.blocks
            .read()
            .get(block_index)
            .cloned()
            .ok_or(HeapError::invalid_shared_raw_pointer(pointer))
    }

    /// Return the resolved live extent for one shared raw pointer.
    fn resolve_extent(&self, pointer: SharedRawPointer) -> HeapResult<SharedRawExtent> {
        let page_index = pointer.offset() / self.page_size_bytes();
        let page_offset = pointer.offset() % self.page_size_bytes();
        let Some(entry) = self.page_map.read().get(page_index).copied().flatten() else {
            return Err(HeapError::invalid_shared_raw_pointer(pointer));
        };
        let block = self
            .blocks
            .read()
            .get(entry.block_index)
            .cloned()
            .ok_or(HeapError::invalid_shared_raw_pointer(pointer))?;
        let block = block.read();

        if block.is_vacant() {
            return Err(HeapError::invalid_shared_raw_pointer(pointer));
        }

        let byte_offset = entry.logical_page_index * self.page_size_bytes() + page_offset;

        if block.byte_len == 0 {
            if byte_offset != 0 {
                return Err(HeapError::invalid_shared_raw_pointer(pointer));
            }
        } else if byte_offset >= block.byte_len {
            return Err(HeapError::invalid_shared_raw_pointer(pointer));
        }

        Ok(SharedRawExtent {
            block_index: entry.block_index,
            base: SharedRawPointer::new(block.first_offset),
            byte_offset,
            byte_len: block.byte_len,
        })
    }

    /// Return one checked shared raw extent range.
    fn resolve_range(
        &self,
        pointer: SharedRawPointer,
        start: usize,
        byte_len: usize,
    ) -> HeapResult<(SharedRawExtent, usize)> {
        let extent = self.resolve_extent(pointer)?;
        debug_assert!(extent.byte_offset <= extent.byte_len);

        let remaining = extent.byte_len - extent.byte_offset;
        if start > remaining {
            return Err(HeapError::invalid_shared_raw_pointer(pointer));
        }

        let byte_offset = extent.byte_offset + start;
        let remaining = extent.byte_len - byte_offset;
        if byte_len > remaining {
            return Err(HeapError::invalid_shared_raw_pointer(pointer));
        }

        Ok((extent, byte_offset))
    }

    /// Record one page-map entry for every page in one page span.
    pub(crate) fn map_page_span(
        &self,
        first_offset: usize,
        page_span: &PageSpan,
        block_index: usize,
    ) {
        let mut page_map = self.page_map.write();
        let first_page_index = first_offset / self.page_size_bytes();

        for logical_page_index in 0..page_span.len() {
            let page_index = first_page_index + logical_page_index;

            if page_map.len() <= page_index {
                page_map.resize(page_index + 1, None);
            }

            page_map[page_index] = Some(SharedRawPageMapEntry {
                block_index,
                logical_page_index,
            });
        }
    }

    /// Clear every page-map entry for one page span.
    pub(crate) fn unmap_page_span(&self, first_offset: usize, page_span: &PageSpan) {
        let mut page_map = self.page_map.write();
        let first_page_index = first_offset / self.page_size_bytes();

        for logical_page_index in 0..page_span.len() {
            let page_index = first_page_index + logical_page_index;

            if let Some(entry) = page_map.get_mut(page_index) {
                *entry = None;
            }
        }
    }

    /// Replace one logical page span entry range.
    pub(crate) fn replace_page_span(
        &self,
        first_offset: usize,
        previous: &PageSpan,
        next: &PageSpan,
        block_index: usize,
    ) {
        let mut page_map = self.page_map.write();
        let first_page_index = first_offset / self.page_size_bytes();

        // publish the next entries over the shared prefix
        for logical_page_index in 0..next.len() {
            let page_index = first_page_index + logical_page_index;

            if page_map.len() <= page_index {
                page_map.resize(page_index + 1, None);
            }

            page_map[page_index] = Some(SharedRawPageMapEntry {
                block_index,
                logical_page_index,
            });
        }

        // clear entries that only belonged to the previous span
        for logical_page_index in next.len()..previous.len() {
            let page_index = first_page_index + logical_page_index;

            if let Some(entry) = page_map.get_mut(page_index) {
                *entry = None;
            }
        }
    }
}

impl SharedRawSpace {
    /// Reserve one logical shared raw-space byte range.
    fn reserve_space_range(&self, byte_len: usize, alignment: usize) -> HeapResult<usize> {
        let mut state = self.state.lock();
        let mapping_byte_len = self.mapping.read().byte_len();
        debug_assert!(state.next_offset <= mapping_byte_len);

        let alignment = alignment.max(self.allocator.page_size_bytes());
        let first_offset = align_up(state.next_offset, alignment);
        let next_offset = first_offset + byte_len;
        if next_offset > mapping_byte_len {
            return Err(HeapError::InvalidByteRange {
                start: first_offset,
                len: byte_len,
                capacity: mapping_byte_len,
            });
        }

        state.next_offset = next_offset;

        Ok(first_offset)
    }

    /// Initialize one mapped payload range.
    #[inline(always)]
    fn initialize_mapped_payload(
        &self,
        offset: usize,
        clear_byte_len: usize,
        payload: Payload<'_>,
    ) {
        match payload {
            Payload::Bytes(bytes) => self.write_mapped_bytes(offset, bytes),
            Payload::Zeroed => self.clear_mapped_bytes(offset, clear_byte_len),
            Payload::Uninit => {}
        }
    }

    /// Write bytes into one mapped payload range.
    #[inline(always)]
    fn write_mapped_bytes(&self, offset: usize, bytes: &[u8]) {
        let mapping = self.mapping.write();

        // SAFETY: block paths materialize the destination before publishing it
        unsafe {
            mapping.write_mapped_bytes(offset, bytes);
        }
    }

    /// Clear one mapped payload range.
    #[inline(always)]
    fn clear_mapped_bytes(&self, offset: usize, byte_len: usize) {
        let mapping = self.mapping.write();
        let address = mapping.base_address() + offset;

        // SAFETY: block paths materialize the destination before publishing it
        unsafe {
            std::ptr::write_bytes(address as *mut u8, 0, byte_len);
        }
    }
}

/// Mutable shared raw-space state.
#[derive(Debug, Default)]
pub(crate) struct SharedRawState {
    /// The shared cache of reusable page spans.
    pub(crate) page_span_cache: PageSpanCache,
    /// The exact live shared raw-space usage.
    pub(crate) usage: AllocationUsage,
    /// The exact retained bytes owned by live raw blocks.
    pub(crate) live_retained_bytes: u64,
    /// The next unused byte offset in shared raw space.
    pub(crate) next_offset: usize,
}

impl SharedRawState {
    /// Return the exact retained bytes owned by live blocks and cached page spans.
    pub(crate) fn retained_bytes(&self, page_size_bytes: usize) -> u64 {
        self.live_retained_bytes + self.page_span_cache.cached_bytes(page_size_bytes)
    }

    /// Return the exact raw-space usage.
    pub(crate) fn usage(&self, page_size_bytes: usize) -> SharedRawSpaceUsage {
        SharedRawSpaceUsage {
            allocation_count: self.usage.allocation_count(),
            allocated_bytes: self.usage.allocated_bytes(),
            retained_bytes: self.retained_bytes(page_size_bytes),
        }
    }

    /// Add one live raw block page span to retained accounting.
    pub(crate) fn retain_pages(&mut self, page_span: PageSpan, page_size_bytes: usize) {
        self.live_retained_bytes += page_span.len() as u64 * page_size_bytes as u64;
    }

    /// Remove one live raw block page span from retained accounting.
    pub(crate) fn release_pages(&mut self, page_span: PageSpan, page_size_bytes: usize) {
        self.live_retained_bytes -= page_span.len() as u64 * page_size_bytes as u64;
    }
}

/// Return the offset rounded up to one block boundary.
fn align_up(byte_len: usize, alignment_bytes: usize) -> usize {
    let alignment_bytes = alignment_bytes.max(1);

    byte_len.div_ceil(alignment_bytes) * alignment_bytes
}

impl Drop for SharedRawSpace {
    fn drop(&mut self) {
        let _ = self.close();
    }
}
