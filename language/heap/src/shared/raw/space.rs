use std::sync::Arc;

use parking_lot::{Mutex, RwLock};

use super::{SharedRawAllocation, SharedRawLocation, SharedRawPageMapEntry};
use crate::allocator::{PageRunCache, PageView};
use crate::{
    AccountingRegion, AllocationUsage, Allocator, HeapError, HeapResult, Payload, SharedRawPointer,
    SharedRawSpaceUsage,
};

/// Control state for one shared raw space.
#[derive(Debug, Default)]
pub(crate) struct SharedRawState {
    /// The shared front-end cache of reusable page runs.
    pub(crate) page_run_cache: PageRunCache,
    /// The exact live shared raw-space usage.
    pub(crate) usage: AllocationUsage,
}

/// One live shared raw-space store rooted in one allocator.
#[derive(Debug)]
pub struct SharedRawSpace {
    /// The shared raw-space allocator for every allocation.
    pub(crate) allocator: Arc<Allocator>,
    /// The shared raw-space state.
    pub(crate) state: Mutex<SharedRawState>,
    /// The owning allocation location for each visible allocator page.
    pub(crate) page_map: RwLock<Vec<Option<SharedRawPageMapEntry>>>,
    /// The live shared raw-space allocations.
    pub(crate) allocations: RwLock<Vec<Arc<RwLock<SharedRawAllocation>>>>,
}

impl SharedRawSpace {
    /// Create a new empty shared raw-space store over one shared allocator.
    pub fn with_allocator(allocator: Arc<Allocator>) -> Self {
        let page_run_cache = PageRunCache::new(allocator.pages_per_arena());

        Self {
            allocator,
            state: Mutex::new(SharedRawState {
                page_run_cache,
                usage: AllocationUsage::default(),
            }),
            page_map: RwLock::new(Vec::new()),
            allocations: RwLock::new(Vec::new()),
        }
    }

    /// Return the configured shared page size.
    pub fn page_bytes(&self) -> usize {
        self.allocator.page_bytes()
    }

    /// Return the exact active shared raw-space bytes.
    pub fn active_bytes(&self) -> u64 {
        self.mapped_bytes()
    }

    /// Return the exact mapped shared page bytes.
    pub fn mapped_bytes(&self) -> u64 {
        let state = self.state.lock();

        self.live_mapped_bytes(&state)
    }

    /// Return the exact borrowed shared bytes.
    pub fn borrowed_bytes(&self) -> u64 {
        let allocations = self.allocations.read();
        let pages = self.live_page_views(&allocations);

        self.allocator.borrowed_bytes_for_page_views(pages.iter())
    }

    /// Return the exact usage for this live shared raw-space store.
    pub fn usage(&self) -> SharedRawSpaceUsage {
        let state = self.state.lock();
        let allocation_count = state.usage.allocation_count();
        let allocated_bytes = state.usage.allocated_bytes();
        let mapped_bytes = self.live_mapped_bytes(&state);
        let active_bytes = mapped_bytes;

        drop(state);

        let borrowed_bytes = self.borrowed_bytes();

        SharedRawSpaceUsage {
            allocation_count,
            allocated_bytes,
            active_bytes,
            mapped_bytes,
            borrowed_bytes,
        }
    }

    /// Return the mapped live bytes for the current shared raw state.
    fn live_mapped_bytes(&self, state: &SharedRawState) -> u64 {
        let allocations = self.allocations.read();
        let pages = self.live_page_views(&allocations);
        let cached_bytes = state
            .page_run_cache
            .cached_bytes(self.allocator.page_bytes());

        self.allocator
            .mapped_bytes_for_page_views(pages.iter())
            .saturating_add(cached_bytes)
    }

    /// Return whether one shared raw pointer currently refers to one live allocation slot.
    pub fn is_live(&self, pointer: SharedRawPointer) -> bool {
        self.resolve_location(pointer).is_ok()
    }

    /// Allocate one shared raw allocation.
    pub fn allocate(
        &self,
        byte_len: usize,
        allocation: Payload<'_>,
    ) -> HeapResult<SharedRawPointer> {
        if let Some(actual) = allocation.byte_len()
            && actual != byte_len
        {
            return Err(HeapError::InvalidAllocationBytes {
                expected: byte_len,
                actual,
            });
        }

        let pages = self.allocate_pages(byte_len, allocation)?;
        let base_address = match self.allocator.page_view_ptr(&pages, 0) {
            Ok(base_address) => base_address as usize,
            Err(error) => {
                self.release_pages(pages)?;

                return Err(error);
            }
        };

        let mut allocations = self.allocations.write();
        let allocation_index = allocations.len();
        if let Err(error) = self.map_page_view(&pages, allocation_index) {
            drop(allocations);
            self.release_pages(pages)?;

            return Err(error);
        }

        let allocation = Arc::new(RwLock::new(SharedRawAllocation::new(
            byte_len,
            pages.clone(),
        )));
        allocations.push(allocation);
        drop(allocations);

        let mut state = self.state.lock();
        state.usage.allocate(byte_len, AccountingRegion::SharedRaw);

        Ok(SharedRawPointer::new(base_address))
    }

    /// Allocate raw pages and initialize payload bytes outside the shared state lock.
    fn allocate_pages(&self, byte_len: usize, allocation: Payload<'_>) -> HeapResult<PageView> {
        let allocated_byte_len = byte_len.max(1);
        let mut pages = {
            let mut state = self.state.lock();

            state
                .page_run_cache
                .allocate_zeroed(&self.allocator, allocated_byte_len)?
        };

        if matches!(allocation, Payload::Zeroed) {
            return Ok(pages);
        }

        if let Err(error) = self.allocator.write_payload(&mut pages, 0, allocation) {
            self.release_pages(pages)?;

            return Err(error);
        }

        Ok(pages)
    }

    /// Return unpublished raw pages to the shared page-run cache.
    fn release_pages(&self, pages: PageView) -> HeapResult<()> {
        let mut state = self.state.lock();

        state
            .page_run_cache
            .release_page_view(&self.allocator, pages)
    }

    /// Return the projected mapped-byte delta for one shared allocation.
    pub fn alloc_mapped_byte_delta(&self, byte_len: usize) -> i64 {
        self.round_up_allocation_bytes(byte_len) as i64
    }

    /// Return the remaining byte length for one shared raw pointer.
    pub fn byte_len(&self, pointer: SharedRawPointer) -> HeapResult<usize> {
        let location = self.resolve_location(pointer)?;

        checked_remaining_byte_len(pointer, location.byte_offset, location.byte_len)
    }

    /// Return the bytes for one shared raw pointer.
    pub fn read_bytes(&self, pointer: SharedRawPointer) -> HeapResult<Vec<u8>> {
        let location = self.resolve_location(pointer)?;
        let byte_len =
            checked_remaining_byte_len(pointer, location.byte_offset, location.byte_len)?;
        let allocation = self.allocation(pointer, location.allocation_index)?;
        let allocation = allocation.read();

        self.allocator
            .bytes_to_vec_from(&allocation.pages, location.byte_offset, byte_len)
    }

    /// Fill one caller-provided buffer from one shared raw pointer at one offset.
    pub fn read_bytes_into(
        &self,
        pointer: SharedRawPointer,
        start: usize,
        target: &mut [u8],
    ) -> HeapResult<()> {
        let location = self.resolve_location(pointer)?;
        let byte_offset = location
            .byte_offset
            .checked_add(start)
            .ok_or(HeapError::InvalidSharedRawPointer { pointer })?;
        checked_remaining_byte_len(pointer, byte_offset, location.byte_len)?;
        let end = byte_offset
            .checked_add(target.len())
            .ok_or(HeapError::InvalidSharedRawPointer { pointer })?;
        if end > location.byte_len {
            return Err(HeapError::InvalidSharedRawPointer { pointer });
        }

        let allocation = self.allocation(pointer, location.allocation_index)?;
        let allocation = allocation.read();
        if allocation.is_vacant() {
            return Err(HeapError::InvalidSharedRawPointer { pointer });
        }

        self.allocator
            .fill_bytes_from(&allocation.pages, byte_offset, target)
    }

    /// Overwrite one byte range for one live shared raw pointer.
    pub fn write_bytes(
        &self,
        pointer: SharedRawPointer,
        start: usize,
        bytes: &[u8],
    ) -> HeapResult<()> {
        let location = self.resolve_location(pointer)?;
        let byte_offset = location
            .byte_offset
            .checked_add(start)
            .ok_or(HeapError::InvalidSharedRawPointer { pointer })?;
        checked_remaining_byte_len(pointer, byte_offset, location.byte_len)?;
        let end = byte_offset
            .checked_add(bytes.len())
            .ok_or(HeapError::InvalidSharedRawPointer { pointer })?;
        if end > location.byte_len {
            return Err(HeapError::InvalidSharedRawPointer { pointer });
        }

        let allocation = self.allocation(pointer, location.allocation_index)?;
        let mut allocation = allocation.write();
        if allocation.is_vacant() {
            return Err(HeapError::InvalidSharedRawPointer { pointer });
        }

        let previous_pages = allocation.pages.clone();
        self.allocator
            .set_bytes(&mut allocation.pages, byte_offset, bytes)?;
        if allocation.pages != previous_pages {
            self.unmap_page_view(&previous_pages)?;
            self.map_page_view(&allocation.pages, location.allocation_index)?;
        }

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
        let next_pages = if bytes.is_empty() {
            state
                .page_run_cache
                .allocate_zeroed(&self.allocator, bytes.len().max(1))?
        } else {
            state
                .page_run_cache
                .allocate_bytes(&self.allocator, bytes)?
        };
        let mut allocation = allocation.write();

        if allocation.is_vacant() {
            state
                .page_run_cache
                .release_page_view(&self.allocator, next_pages)?;

            return Err(HeapError::InvalidSharedRawPointer { pointer });
        }

        let previous_pages = allocation.pages.clone();
        let previous_len = allocation.len;

        allocation.pages = next_pages.clone();
        allocation.len = bytes.len();
        let base_address = self.allocator.page_view_ptr(&allocation.pages, 0)? as usize;

        drop(allocation);

        self.unmap_page_view(&previous_pages)?;
        self.map_page_view(&next_pages, location.allocation_index)?;

        state
            .usage
            .resize(previous_len, bytes.len(), AccountingRegion::SharedRaw);
        state
            .page_run_cache
            .release_page_view(&self.allocator, previous_pages)?;

        Ok(SharedRawPointer::new(base_address))
    }

    /// Free one shared raw-space allocation.
    pub fn free(&self, pointer: SharedRawPointer) -> HeapResult<bool> {
        let location = self.resolve_location(pointer)?;
        let allocation = self.allocation(pointer, location.allocation_index)?;
        let mut state = self.state.lock();
        let mut allocation = allocation.write();

        if allocation.is_vacant() {
            return Err(HeapError::InvalidSharedRawPointer { pointer });
        }

        let pages = allocation.pages.clone();
        let previous_len = allocation.len as u64;

        state
            .usage
            .check_free(previous_len, AccountingRegion::SharedRaw);

        allocation.retire();
        drop(allocation);

        self.unmap_page_view(&pages)?;
        state.usage.free(previous_len, AccountingRegion::SharedRaw);
        state
            .page_run_cache
            .release_page_view(&self.allocator, pages)?;

        Ok(true)
    }

    /// Return the projected mapped-byte delta for one shared replacement.
    pub fn replace_mapped_byte_delta(
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

        let previous_mapped_bytes = self.round_up_allocation_bytes(allocation.len);
        let next_mapped_bytes = self.round_up_allocation_bytes(next_byte_len);

        Ok(next_mapped_bytes as i64 - previous_mapped_bytes as i64)
    }

    /// Return the page-rounded mapped bytes for one shared allocation.
    fn round_up_allocation_bytes(&self, byte_len: usize) -> u64 {
        let page_bytes = self.page_bytes() as u64;
        let byte_len = byte_len.max(1) as u64;

        byte_len.div_ceil(page_bytes) * page_bytes
    }

    /// Return the current live raw allocation page views.
    fn live_page_views(
        &self,
        allocations: &[Arc<RwLock<SharedRawAllocation>>],
    ) -> Vec<crate::PageView> {
        let mut pages = Vec::with_capacity(allocations.len());

        for allocation in allocations {
            let allocation = allocation.read();

            if allocation.is_vacant() {
                continue;
            }

            pages.push(allocation.pages.clone());
        }

        pages
    }

    /// Release allocator roots owned by this shared raw space.
    fn close(&mut self) -> HeapResult<()> {
        let allocations = self.allocations.read();
        let page_views = self.live_page_views(&allocations);
        drop(allocations);

        let mut state = self.state.lock();
        for page_view in page_views {
            state
                .page_run_cache
                .release_page_view(&self.allocator, page_view)?;
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
        let Some((page_id, page_offset)) = self.allocator.address_page_position(pointer.address())
        else {
            return Err(HeapError::InvalidSharedRawPointer { pointer });
        };
        let Some(entry) = self.page_map.read().get(page_id.index()).copied().flatten() else {
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

        let byte_offset = entry
            .logical_page_index
            .checked_mul(self.page_bytes())
            .and_then(|byte_offset| byte_offset.checked_add(page_offset))
            .ok_or(HeapError::InvalidSharedRawPointer { pointer })?;

        if allocation.len == 0 {
            if byte_offset != 0 {
                return Err(HeapError::InvalidSharedRawPointer { pointer });
            }
        } else if byte_offset >= allocation.len {
            return Err(HeapError::InvalidSharedRawPointer { pointer });
        }

        let base_address =
            self.allocator
                .page_view_ptr(&allocation.pages, 0)
                .map_err(|_| HeapError::InvalidSharedRawPointer { pointer })? as usize;

        Ok(SharedRawLocation {
            allocation_index: entry.allocation_index,
            base: SharedRawPointer::new(base_address),
            byte_offset,
            byte_len: allocation.len,
        })
    }

    /// Record one page map entry for every page in one page view.
    pub(crate) fn map_page_view(
        &self,
        page_view: &PageView,
        allocation_index: usize,
    ) -> HeapResult<()> {
        let mut page_map = self.page_map.write();

        for logical_page_index in 0..page_view.len() {
            let Some(page_id) = page_view.page(logical_page_index) else {
                return Err(HeapError::MissingLogicalPage {
                    page_index: logical_page_index,
                });
            };
            let page_index = page_id.index();

            if page_map.len() <= page_index {
                page_map.resize(page_index + 1, None);
            }

            page_map[page_index] = Some(SharedRawPageMapEntry {
                allocation_index,
                logical_page_index,
            });
        }

        Ok(())
    }

    /// Clear every page map entry for one page view.
    pub(crate) fn unmap_page_view(&self, page_view: &PageView) -> HeapResult<()> {
        let mut page_map = self.page_map.write();

        for logical_page_index in 0..page_view.len() {
            let Some(page_id) = page_view.page(logical_page_index) else {
                return Err(HeapError::MissingLogicalPage {
                    page_index: logical_page_index,
                });
            };

            if let Some(entry) = page_map.get_mut(page_id.index()) {
                *entry = None;
            }
        }

        Ok(())
    }
}

impl Drop for SharedRawSpace {
    fn drop(&mut self) {
        let _ = self.close();
    }
}

/// Return the visible byte length for one shared raw pointer.
fn checked_remaining_byte_len(
    pointer: SharedRawPointer,
    byte_offset: usize,
    byte_len: usize,
) -> HeapResult<usize> {
    if byte_offset > byte_len {
        return Err(HeapError::InvalidSharedRawPointer { pointer });
    }

    Ok(byte_len - byte_offset)
}
