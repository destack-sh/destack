use std::sync::Arc;

use parking_lot::{Mutex, RwLock};

use super::{SharedRawEntry, SharedRawLocation, SharedRawPageOwner};
use crate::allocator::{PageRunCache, PageView};
use crate::{
    AccountingRegion, Allocation, AllocationUsage, Allocator, HeapError, HeapResult,
    SharedRawPointer, SharedRawSpaceUsage,
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
    /// The shared raw-space allocator for every entry.
    pub(crate) allocator: Arc<Allocator>,
    /// The shared raw-space state.
    pub(crate) state: Mutex<SharedRawState>,
    /// The owning entry location for each visible allocator page.
    pub(crate) page_owners: RwLock<Vec<Option<SharedRawPageOwner>>>,
    /// The live shared raw-space entries.
    pub(crate) entries: RwLock<Vec<Arc<RwLock<SharedRawEntry>>>>,
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
            page_owners: RwLock::new(Vec::new()),
            entries: RwLock::new(Vec::new()),
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
        let entries = self.entries.read();
        let pages = self.live_pages(&entries);

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
        let entries = self.entries.read();
        let pages = self.live_pages(&entries);
        let cached_bytes = state
            .page_run_cache
            .cached_bytes(self.allocator.page_bytes());

        self.allocator
            .mapped_bytes_for_page_views(pages.iter())
            .saturating_add(cached_bytes)
    }

    /// Return whether one shared raw pointer currently refers to one live entry slot.
    pub fn is_live(&self, pointer: SharedRawPointer) -> bool {
        self.resolve_location(pointer).is_ok()
    }

    /// Allocate one shared raw entry.
    pub fn allocate(
        &self,
        byte_len: usize,
        allocation: Allocation<'_>,
    ) -> HeapResult<SharedRawPointer> {
        if let Some(bytes) = allocation.bytes()
            && bytes.len() != byte_len
        {
            return Err(HeapError::InvalidAllocationBytes {
                expected: byte_len,
                actual: bytes.len(),
            });
        }

        let mut state = self.state.lock();
        let pages = match allocation.bytes() {
            Some(bytes) => state
                .page_run_cache
                .allocate_bytes(&self.allocator, bytes)?,
            None => state
                .page_run_cache
                .allocate_zeroed(&self.allocator, byte_len)?,
        };
        let entry = Arc::new(RwLock::new(SharedRawEntry::new(byte_len, pages.clone())));
        let base_address = self.allocator.page_view_ptr(&pages, 0)? as *mut u8 as usize;

        let mut entries = self.entries.write();
        let entry_index = entries.len();
        entries.push(entry);
        drop(entries);

        self.map_page_view(&pages, entry_index)?;
        state
            .usage
            .allocate(byte_len, AccountingRegion::SharedRaw)?;

        Ok(SharedRawPointer::new(base_address))
    }

    /// Return the projected mapped-byte delta for one shared entry.
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
        let entry = self.entry(pointer, location.entry_index)?;
        let entry = entry.read();

        self.allocator
            .bytes_to_vec_from(&entry.pages, location.byte_offset, byte_len)
    }

    /// Replace the bytes for one shared raw pointer.
    pub fn replace_bytes(
        &self,
        pointer: SharedRawPointer,
        bytes: &[u8],
    ) -> HeapResult<SharedRawPointer> {
        let location = self.resolve_location(pointer)?;
        let entry = self.entry(pointer, location.entry_index)?;
        let mut state = self.state.lock();
        let next_pages = state
            .page_run_cache
            .allocate_bytes(&self.allocator, bytes)?;
        let mut entry = entry.write();

        if entry.is_vacant() {
            state
                .page_run_cache
                .release_page_view(&self.allocator, next_pages)?;

            return Err(HeapError::InvalidSharedRawPointer { pointer });
        }

        let previous_pages = entry.pages.clone();
        let previous_len = entry.len;

        entry.pages = next_pages.clone();
        entry.len = bytes.len();
        let base_address = self.allocator.page_view_ptr(&entry.pages, 0)? as *mut u8 as usize;

        drop(entry);

        self.unmap_page_view(&previous_pages)?;
        self.map_page_view(&next_pages, location.entry_index)?;

        state
            .usage
            .resize(previous_len, bytes.len(), AccountingRegion::SharedRaw)?;
        state
            .page_run_cache
            .release_page_view(&self.allocator, previous_pages)?;

        Ok(SharedRawPointer::new(base_address))
    }

    /// Free one shared raw-space entry.
    pub fn free(&self, pointer: SharedRawPointer) -> HeapResult<bool> {
        let location = self.resolve_location(pointer)?;
        let entry = self.entry(pointer, location.entry_index)?;
        let mut state = self.state.lock();
        let mut entry = entry.write();

        if entry.is_vacant() {
            return Err(HeapError::InvalidSharedRawPointer { pointer });
        }

        let pages = entry.pages.clone();
        let previous_len = entry.len as u64;

        state
            .usage
            .check_free(previous_len, AccountingRegion::SharedRaw)?;

        entry.retire();
        drop(entry);

        self.unmap_page_view(&pages)?;
        state
            .usage
            .free(previous_len, AccountingRegion::SharedRaw)?;
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
        let entry = self.entry(pointer, location.entry_index)?;
        let entry = entry.read();

        if entry.is_vacant() {
            return Err(HeapError::InvalidSharedRawPointer { pointer });
        }

        let previous_mapped_bytes = self.round_up_allocation_bytes(entry.len);
        let next_mapped_bytes = self.round_up_allocation_bytes(next_byte_len);

        Ok(next_mapped_bytes as i64 - previous_mapped_bytes as i64)
    }

    /// Return the page-rounded mapped bytes for one shared entry.
    fn round_up_allocation_bytes(&self, byte_len: usize) -> u64 {
        let page_bytes = self.page_bytes() as u64;
        let byte_len = byte_len as u64;

        byte_len.div_ceil(page_bytes) * page_bytes
    }

    /// Return the current live raw entry page views.
    fn live_pages(&self, entries: &[Arc<RwLock<SharedRawEntry>>]) -> Vec<crate::PageView> {
        let mut pages = Vec::with_capacity(entries.len());

        for entry in entries {
            let entry = entry.read();

            if entry.is_vacant() {
                continue;
            }

            pages.push(entry.pages.clone());
        }

        pages
    }

    /// Return one live shared raw entry by index.
    fn entry(
        &self,
        pointer: SharedRawPointer,
        entry_index: usize,
    ) -> HeapResult<Arc<RwLock<SharedRawEntry>>> {
        self.entries
            .read()
            .get(entry_index)
            .cloned()
            .ok_or(HeapError::InvalidSharedRawPointer { pointer })
    }

    /// Return the resolved live location for one shared raw pointer.
    fn resolve_location(&self, pointer: SharedRawPointer) -> HeapResult<SharedRawLocation> {
        let Some((page_id, page_offset)) = self.allocator.address_page_position(pointer.address())
        else {
            return Err(HeapError::InvalidSharedRawPointer { pointer });
        };
        let Some(owner) = self
            .page_owners
            .read()
            .get(page_id.index())
            .copied()
            .flatten()
        else {
            return Err(HeapError::InvalidSharedRawPointer { pointer });
        };
        let entry = self
            .entries
            .read()
            .get(owner.entry_index)
            .cloned()
            .ok_or(HeapError::InvalidSharedRawPointer { pointer })?;
        let entry = entry.read();

        if entry.is_vacant() {
            return Err(HeapError::InvalidSharedRawPointer { pointer });
        }

        let byte_offset = owner
            .logical_page_index
            .checked_mul(self.page_bytes())
            .and_then(|byte_offset| byte_offset.checked_add(page_offset))
            .ok_or(HeapError::InvalidSharedRawPointer { pointer })?;

        if byte_offset >= entry.len {
            return Err(HeapError::InvalidSharedRawPointer { pointer });
        }

        let base_address = self
            .allocator
            .page_view_ptr(&entry.pages, 0)
            .map_err(|_| HeapError::InvalidSharedRawPointer { pointer })?
            as *mut u8 as usize;

        Ok(SharedRawLocation {
            entry_index: owner.entry_index,
            base: SharedRawPointer::new(base_address),
            byte_offset,
            byte_len: entry.len,
        })
    }

    /// Record one visible owner for every page in one page view.
    pub(crate) fn map_page_view(&self, page_view: &PageView, entry_index: usize) -> HeapResult<()> {
        let mut page_owners = self.page_owners.write();

        for logical_page_index in 0..page_view.len() {
            let Some(page_id) = page_view.page(logical_page_index) else {
                return Err(HeapError::MissingLogicalPage {
                    page_index: logical_page_index,
                });
            };
            let page_index = page_id.index();

            if page_owners.len() <= page_index {
                page_owners.resize(page_index + 1, None);
            }

            page_owners[page_index] = Some(SharedRawPageOwner {
                entry_index,
                logical_page_index,
            });
        }

        Ok(())
    }

    /// Clear every visible owner for one page view.
    pub(crate) fn unmap_page_view(&self, page_view: &PageView) -> HeapResult<()> {
        let mut page_owners = self.page_owners.write();

        for logical_page_index in 0..page_view.len() {
            let Some(page_id) = page_view.page(logical_page_index) else {
                return Err(HeapError::MissingLogicalPage {
                    page_index: logical_page_index,
                });
            };

            if let Some(owner) = page_owners.get_mut(page_id.index()) {
                *owner = None;
            }
        }

        Ok(())
    }
}

impl Drop for SharedRawSpace {
    fn drop(&mut self) {
        let mut state = self.state.lock();

        state.page_run_cache.flush(&self.allocator);
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
