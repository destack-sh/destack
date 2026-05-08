use std::ptr::write_bytes;
use std::sync::Arc;

use destack_memory::AddressSpace;
use parking_lot::{Mutex, RwLock};

use super::{SharedRawAllocation, SharedRawLocation, SharedRawPageMapEntry};
use crate::allocator::{PageRun, PageRunCache};
use crate::{
    AllocationUsage, Allocator, HeapError, HeapOptions, HeapResult, Payload, RawAllocationShape,
    SharedRawPointer, SharedRawSpaceUsage,
};

/// Mutable shared raw-space state.
#[derive(Debug, Default)]
pub(crate) struct SharedRawState {
    /// The shared cache of reusable page runs.
    pub(crate) page_run_cache: PageRunCache,
    /// The exact live shared raw-space usage.
    pub(crate) usage: AllocationUsage,
    /// The next unused byte offset in shared raw space.
    pub(crate) next_offset: usize,
}

/// One live shared raw space over one allocator.
#[derive(Debug)]
pub struct SharedRawSpace {
    /// The shared raw-space allocator for every allocation.
    pub(crate) allocator: Arc<Allocator>,
    /// The fixed base native address for direct raw access.
    pub(crate) base_address: usize,
    /// The shared raw-space state.
    pub(crate) state: Mutex<SharedRawState>,
    /// The owning allocation location for each visible allocator page.
    pub(crate) page_map: RwLock<Vec<Option<SharedRawPageMapEntry>>>,
    /// The live shared raw-space allocations.
    pub(crate) allocations: RwLock<Vec<Arc<RwLock<SharedRawAllocation>>>>,
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
        let options = HeapOptions {
            page_bytes: allocator.page_bytes(),
            allocator_chunk_bytes: allocator.chunk_bytes(),
            ..HeapOptions::shared()
        };

        Self::with_options(allocator, &options)
    }

    /// Create a new empty shared raw space over one shared allocator and options.
    pub fn with_options(allocator: Arc<Allocator>, options: &HeapOptions) -> HeapResult<Self> {
        options.validate_shared()?;
        options.validate_allocator(&allocator)?;

        let page_run_cache = PageRunCache::new(allocator.pages_per_chunk());
        let next_offset = allocator.page_bytes();
        let mapping = AddressSpace::reserve(options.raw_space_bytes, options.page_bytes)?;
        let base_address = mapping.base_address();

        Ok(Self {
            allocator,
            base_address,
            state: Mutex::new(SharedRawState {
                page_run_cache,
                usage: AllocationUsage::default(),
                next_offset,
            }),
            page_map: RwLock::new(Vec::new()),
            allocations: RwLock::new(Vec::new()),
            mapping: RwLock::new(mapping),
        })
    }

    /// Return the configured shared page size.
    pub fn page_bytes(&self) -> usize {
        self.allocator.page_bytes()
    }

    /// Return the exact retained shared raw allocator-page bytes.
    pub fn retained_bytes(&self) -> u64 {
        let state = self.state.lock();

        self.live_retained_bytes(&state)
    }

    /// Return the exact usage for this live shared raw space.
    pub fn usage(&self) -> SharedRawSpaceUsage {
        let state = self.state.lock();
        let allocation_count = state.usage.allocation_count();
        let allocated_bytes = state.usage.allocated_bytes();
        let retained_bytes = self.live_retained_bytes(&state);

        drop(state);

        SharedRawSpaceUsage {
            allocation_count,
            allocated_bytes,
            retained_bytes,
        }
    }

    /// Return the retained live bytes for the current shared raw state.
    fn live_retained_bytes(&self, state: &SharedRawState) -> u64 {
        let allocations = self.allocations.read();
        let pages = self.live_page_runs(&allocations);
        let cached_bytes = state
            .page_run_cache
            .cached_bytes(self.allocator.page_bytes());

        self.allocator.retained_bytes_for_page_runs(pages.iter()) + cached_bytes
    }

    /// Return whether one shared raw pointer currently refers to one live allocation slot.
    pub fn is_live(&self, pointer: SharedRawPointer) -> bool {
        self.resolve_location(pointer).is_ok()
    }

    /// Allocate one shared raw allocation.
    pub fn allocate(
        &self,
        shape: RawAllocationShape,
        allocation: Payload<'_>,
    ) -> HeapResult<SharedRawPointer> {
        if let Some(actual) = allocation.byte_len()
            && actual != shape.byte_len
        {
            return Err(HeapError::InvalidAllocationBytes {
                expected: shape.byte_len,
                actual,
            });
        }

        let pages = self.allocate_pages(shape.byte_len)?;
        let first_offset =
            self.reserve_space_range(pages.len() * self.allocator.page_bytes(), shape.alignment)?;

        // materialize the full allocation before publishing it
        if let Err(error) = self
            .mapping
            .write()
            .materialize(first_offset, pages.len() * self.allocator.page_bytes())
        {
            self.release_pages(pages)?;

            return Err(error.into());
        }

        let mut allocations = self.allocations.write();
        let allocation_index = allocations.len();
        self.map_page_run(first_offset, &pages, allocation_index);

        // initialize bytes before publishing the allocation record
        let mapping = self.mapping.write();
        match allocation {
            Payload::Bytes(bytes) => unsafe {
                mapping.write_mapped_bytes(first_offset, bytes);
            },
            Payload::Zeroed => unsafe {
                write_bytes(
                    (mapping.base_address() + first_offset) as *mut u8,
                    0,
                    shape.byte_len,
                );
            },
        }
        drop(mapping);

        let record = Arc::new(RwLock::new(SharedRawAllocation::new(
            first_offset,
            shape.byte_len,
            pages,
        )));
        allocations.push(record);
        drop(allocations);

        let mut state = self.state.lock();
        state.usage.allocate(shape.byte_len);
        drop(state);

        Ok(SharedRawPointer::new(first_offset))
    }

    /// Allocate raw pages outside the shared state lock.
    fn allocate_pages(&self, byte_len: usize) -> HeapResult<PageRun> {
        let allocated_byte_len = byte_len.max(1);
        let pages = {
            let mut state = self.state.lock();

            state
                .page_run_cache
                .allocate_pages(&self.allocator, allocated_byte_len)?
        };

        Ok(pages)
    }

    /// Return unpublished raw pages to the shared page-run cache.
    fn release_pages(&self, pages: PageRun) -> HeapResult<()> {
        let mut state = self.state.lock();

        state
            .page_run_cache
            .release_page_run(&self.allocator, pages)
    }

    /// Return the projected retained-byte delta for one shared allocation.
    pub fn alloc_retained_byte_delta(&self, shape: RawAllocationShape) -> i64 {
        self.round_up_allocation_bytes(shape.byte_len) as i64
    }

    /// Return the remaining byte length for one shared raw pointer.
    pub fn byte_len(&self, pointer: SharedRawPointer) -> HeapResult<usize> {
        let location = self.resolve_location(pointer)?;

        Ok(location.byte_len - location.byte_offset)
    }

    /// Return the bytes for one shared raw pointer.
    pub fn read_bytes(&self, pointer: SharedRawPointer) -> HeapResult<Vec<u8>> {
        let location = self.resolve_location(pointer)?;
        let byte_len = location.byte_len - location.byte_offset;

        self.mapping
            .read()
            .read_bytes(location.base.offset() + location.byte_offset, byte_len)
            .map_err(HeapError::from)
    }

    /// Fill one caller-provided buffer from one shared raw pointer at one offset.
    pub fn read_bytes_into(
        &self,
        pointer: SharedRawPointer,
        start: usize,
        target: &mut [u8],
    ) -> HeapResult<()> {
        let (location, byte_offset) = self.resolve_range(pointer, start, target.len())?;

        self.mapping
            .read()
            .read_bytes_into(location.base.offset() + byte_offset, target)
            .map_err(HeapError::from)
    }

    /// Return one checked address for a shared raw byte range.
    pub fn address(
        &self,
        pointer: SharedRawPointer,
        start: usize,
        byte_len: usize,
    ) -> HeapResult<*mut u8> {
        let (location, byte_offset) = self.resolve_range(pointer, start, byte_len)?;

        self.mapping
            .read()
            .address(location.base.offset() + byte_offset, byte_len)
            .map_err(HeapError::from)
    }

    /// Return one checked mutable address for a shared raw byte range.
    pub fn address_mut(
        &self,
        pointer: SharedRawPointer,
        start: usize,
        byte_len: usize,
    ) -> HeapResult<*mut u8> {
        let (location, byte_offset) = self.resolve_range(pointer, start, byte_len)?;

        self.mapping
            .write()
            .address(location.base.offset() + byte_offset, byte_len)
            .map_err(HeapError::from)
    }

    /// Overwrite one byte range for one live shared raw pointer.
    pub fn write_bytes(
        &self,
        pointer: SharedRawPointer,
        start: usize,
        bytes: &[u8],
    ) -> HeapResult<()> {
        let (location, byte_offset) = self.resolve_range(pointer, start, bytes.len())?;

        self.mapping
            .write()
            .write_bytes(location.base.offset() + byte_offset, bytes)?;

        Ok(())
    }

    /// Replace the bytes for one shared raw pointer.
    pub fn replace_bytes(
        &self,
        pointer: SharedRawPointer,
        bytes: &[u8],
    ) -> HeapResult<SharedRawPointer> {
        let location = self.resolve_location(pointer)?;
        let allocation = self.allocation(pointer, location.allocation_index)?;
        let mut state = self.state.lock();
        let next_pages = state
            .page_run_cache
            .allocate_pages(&self.allocator, bytes.len().max(1))?;
        let mut allocation = allocation.write();

        if allocation.is_vacant() {
            state
                .page_run_cache
                .release_page_run(&self.allocator, next_pages)?;

            return Err(HeapError::InvalidSharedRawPointer { pointer });
        }

        let previous_pages = allocation.pages;
        let first_offset = allocation.first_offset;
        let previous_len = allocation.len;

        // materialize the replacement pages before publishing them
        if let Err(error) = self
            .mapping
            .write()
            .materialize(first_offset, next_pages.len() * self.allocator.page_bytes())
        {
            state
                .page_run_cache
                .release_page_run(&self.allocator, next_pages)?;

            return Err(error.into());
        }

        // write replacement bytes before publishing the new page run
        {
            let mapping = self.mapping.write();
            unsafe {
                mapping.write_mapped_bytes(first_offset, bytes);
            }
        }

        self.replace_page_run(
            first_offset,
            &previous_pages,
            &next_pages,
            location.allocation_index,
        );

        allocation.pages = next_pages;
        allocation.len = bytes.len();

        drop(allocation);

        state.usage.resize(previous_len, bytes.len());
        state
            .page_run_cache
            .release_page_run(&self.allocator, previous_pages)?;

        Ok(SharedRawPointer::new(first_offset))
    }

    /// Free one shared raw-space allocation.
    pub fn free(&self, pointer: SharedRawPointer) -> HeapResult<()> {
        let location = self.resolve_location(pointer)?;
        let allocation = self.allocation(pointer, location.allocation_index)?;
        let mut state = self.state.lock();
        let mut allocation = allocation.write();

        if allocation.is_vacant() {
            return Err(HeapError::InvalidSharedRawPointer { pointer });
        }

        let pages = allocation.pages;
        let first_offset = allocation.first_offset;
        let previous_len = allocation.len as u64;

        state.usage.check_free(previous_len);

        allocation.retire();
        drop(allocation);

        self.unmap_page_run(first_offset, &pages);
        state.usage.free(previous_len);
        state
            .page_run_cache
            .release_page_run(&self.allocator, pages)?;

        Ok(())
    }

    /// Return the projected retained-byte delta for one shared replacement.
    pub fn replace_retained_byte_delta(
        &self,
        pointer: SharedRawPointer,
        next_byte_len: usize,
    ) -> HeapResult<i64> {
        let location = self.resolve_location(pointer)?;
        let allocation = self.allocation(pointer, location.allocation_index)?;
        let allocation = allocation.read();

        if allocation.is_vacant() {
            return Err(HeapError::InvalidSharedRawPointer { pointer });
        }

        let previous_retained_bytes = self.round_up_allocation_bytes(allocation.len);
        let next_retained_bytes = self.round_up_allocation_bytes(next_byte_len);

        Ok(next_retained_bytes as i64 - previous_retained_bytes as i64)
    }

    /// Return the page-rounded retained bytes for one shared allocation.
    fn round_up_allocation_bytes(&self, byte_len: usize) -> u64 {
        let page_bytes = self.page_bytes() as u64;
        let byte_len = byte_len.max(1) as u64;

        byte_len.div_ceil(page_bytes) * page_bytes
    }

    /// Return the current live raw allocation page runs.
    fn live_page_runs(&self, allocations: &[Arc<RwLock<SharedRawAllocation>>]) -> Vec<PageRun> {
        let mut pages = Vec::with_capacity(allocations.len());

        for allocation in allocations {
            let allocation = allocation.read();

            if allocation.is_vacant() {
                continue;
            }

            pages.push(allocation.pages);
        }

        pages
    }

    /// Release allocator page runs owned by this shared raw space.
    fn close(&mut self) -> HeapResult<()> {
        let allocations = self.allocations.read();
        let page_runs = self.live_page_runs(&allocations);
        drop(allocations);

        let mut state = self.state.lock();
        for page_run in page_runs {
            state
                .page_run_cache
                .release_page_run(&self.allocator, page_run)?;
        }

        state.page_run_cache.flush(&self.allocator)
    }

    /// Return one live shared raw allocation by index.
    fn allocation(
        &self,
        pointer: SharedRawPointer,
        allocation_index: usize,
    ) -> HeapResult<Arc<RwLock<SharedRawAllocation>>> {
        self.allocations
            .read()
            .get(allocation_index)
            .cloned()
            .ok_or(HeapError::InvalidSharedRawPointer { pointer })
    }

    /// Return the resolved live location for one shared raw pointer.
    fn resolve_location(&self, pointer: SharedRawPointer) -> HeapResult<SharedRawLocation> {
        let page_index = pointer.offset() / self.page_bytes();
        let page_offset = pointer.offset() % self.page_bytes();
        let Some(entry) = self.page_map.read().get(page_index).copied().flatten() else {
            return Err(HeapError::InvalidSharedRawPointer { pointer });
        };
        let allocation = self
            .allocations
            .read()
            .get(entry.allocation_index)
            .cloned()
            .ok_or(HeapError::InvalidSharedRawPointer { pointer })?;
        let allocation = allocation.read();

        if allocation.is_vacant() {
            return Err(HeapError::InvalidSharedRawPointer { pointer });
        }

        let byte_offset = entry.logical_page_index * self.page_bytes() + page_offset;

        if allocation.len == 0 {
            if byte_offset != 0 {
                return Err(HeapError::InvalidSharedRawPointer { pointer });
            }
        } else if byte_offset >= allocation.len {
            return Err(HeapError::InvalidSharedRawPointer { pointer });
        }

        Ok(SharedRawLocation {
            allocation_index: entry.allocation_index,
            base: SharedRawPointer::new(allocation.first_offset),
            byte_offset,
            byte_len: allocation.len,
        })
    }

    /// Return one checked shared raw location range.
    fn resolve_range(
        &self,
        pointer: SharedRawPointer,
        start: usize,
        byte_len: usize,
    ) -> HeapResult<(SharedRawLocation, usize)> {
        let location = self.resolve_location(pointer)?;
        debug_assert!(location.byte_offset <= location.byte_len);

        let remaining = location.byte_len - location.byte_offset;
        if start > remaining {
            return Err(HeapError::InvalidSharedRawPointer { pointer });
        }

        let byte_offset = location.byte_offset + start;
        let remaining = location.byte_len - byte_offset;
        if byte_len > remaining {
            return Err(HeapError::InvalidSharedRawPointer { pointer });
        }

        Ok((location, byte_offset))
    }

    /// Record one page-map entry for every page in one page run.
    pub(crate) fn map_page_run(
        &self,
        first_offset: usize,
        page_run: &PageRun,
        allocation_index: usize,
    ) {
        let mut page_map = self.page_map.write();
        let first_page_index = first_offset / self.page_bytes();

        for logical_page_index in 0..page_run.len() {
            let page_index = first_page_index + logical_page_index;

            if page_map.len() <= page_index {
                page_map.resize(page_index + 1, None);
            }

            page_map[page_index] = Some(SharedRawPageMapEntry {
                allocation_index,
                logical_page_index,
            });
        }
    }

    /// Clear every page-map entry for one page run.
    pub(crate) fn unmap_page_run(&self, first_offset: usize, page_run: &PageRun) {
        let mut page_map = self.page_map.write();
        let first_page_index = first_offset / self.page_bytes();

        for logical_page_index in 0..page_run.len() {
            let page_index = first_page_index + logical_page_index;

            if let Some(entry) = page_map.get_mut(page_index) {
                *entry = None;
            }
        }
    }

    /// Replace one logical page run entry range.
    pub(crate) fn replace_page_run(
        &self,
        first_offset: usize,
        previous: &PageRun,
        next: &PageRun,
        allocation_index: usize,
    ) {
        let mut page_map = self.page_map.write();
        let first_page_index = first_offset / self.page_bytes();

        // publish the next entries over the shared prefix
        for logical_page_index in 0..next.len() {
            let page_index = first_page_index + logical_page_index;

            if page_map.len() <= page_index {
                page_map.resize(page_index + 1, None);
            }

            page_map[page_index] = Some(SharedRawPageMapEntry {
                allocation_index,
                logical_page_index,
            });
        }

        // clear entries that only belonged to the previous run
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

        let alignment = alignment.max(self.allocator.page_bytes());
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
}

/// Return the offset rounded up to one allocation boundary.
fn align_up(byte_len: usize, alignment_bytes: usize) -> usize {
    let alignment_bytes = alignment_bytes.max(1);

    byte_len.div_ceil(alignment_bytes) * alignment_bytes
}

impl Drop for SharedRawSpace {
    fn drop(&mut self) {
        let _ = self.close();
    }
}
