use std::sync::Arc;

use parking_lot::{Mutex, RwLock};

use super::SharedRawEntry;
use crate::arena::PageRunCache;
use crate::{
    AllocationUsage, Arena, HeapError, HeapResult, HeapSpace, SharedRawPointer, SharedRawSpaceUsage,
};

/// The first allocated shared raw-space entry id.
const FIRST_SHARED_ENTRY_ID: u64 = 1;

/// Convert a stable shared entry id into its packed representation.
fn checked_shared_entry_id(entry_id: u64) -> HeapResult<u32> {
    if entry_id == 0 || entry_id > u32::MAX as u64 {
        return Err(HeapError::InvalidSharedRawPointerId { id: entry_id });
    }

    Ok(entry_id as u32)
}

/// Allocator metadata for one shared raw space.
#[derive(Debug, Default)]
pub(crate) struct SharedRawAllocator {
    /// The shared front-end cache of reusable page runs.
    pub(crate) page_run_cache: PageRunCache,
    /// Free shared raw-space entry ids available for reuse.
    pub(crate) free_ids: Vec<u64>,
    /// The next shared raw-space entry id to allocate.
    pub(crate) next_unused_id: u64,
    /// The exact live shared raw-space usage.
    pub(crate) usage: AllocationUsage,
}

/// One live shared raw-space store rooted in one arena.
#[derive(Debug)]
pub struct SharedRawSpace {
    /// The shared raw-space arena for every entry.
    pub(crate) arena: Arc<Arena>,
    /// The shared raw-space allocator control state.
    pub(crate) allocator: Mutex<SharedRawAllocator>,
    /// Stable shared raw-space entries keyed by entry id minus one.
    pub(crate) entries: RwLock<Vec<Arc<RwLock<SharedRawEntry>>>>,
}

impl Default for SharedRawSpace {
    fn default() -> Self {
        Self::new()
    }
}

impl SharedRawSpace {
    /// Create a new empty shared raw-space store.
    pub fn new() -> Self {
        Self::with_arena(Arc::new(Arena::new()))
    }

    /// Create a new empty shared raw-space store over one shared arena.
    pub fn with_arena(arena: Arc<Arena>) -> Self {
        let page_run_cache = PageRunCache::new(arena.pages_per_segment());

        Self {
            arena,
            allocator: Mutex::new(SharedRawAllocator {
                page_run_cache,
                free_ids: Vec::new(),
                next_unused_id: FIRST_SHARED_ENTRY_ID,
                usage: AllocationUsage::default(),
            }),
            entries: RwLock::new(Vec::new()),
        }
    }

    /// Return the configured shared page width.
    pub fn page_bytes(&self) -> usize {
        self.arena.page_bytes()
    }

    /// Return the exact active shared raw-space bytes.
    pub fn active_bytes(&self) -> u64 {
        self.mapped_bytes()
    }

    /// Return the exact mapped shared page bytes.
    pub fn mapped_bytes(&self) -> u64 {
        let allocator = self.allocator.lock();

        self.live_mapped_bytes(&allocator)
    }

    /// Return the exact borrowed shared bytes.
    pub fn borrowed_bytes(&self) -> HeapResult<u64> {
        let entries = self.entries.read();
        let pages = self.live_pages(&entries);

        self.arena.borrowed_bytes_for_page_views(pages.iter())
    }

    /// Return the exact usage for this live shared raw-space store.
    pub fn usage(&self) -> HeapResult<SharedRawSpaceUsage> {
        // allocator summary
        let allocator = self.allocator.lock();
        let allocation_count = allocator.usage.allocation_count();
        let allocated_bytes = allocator.usage.allocated_bytes();
        let mapped_bytes = self.live_mapped_bytes(&allocator);
        let active_bytes = mapped_bytes;

        drop(allocator);

        // borrowed bytes
        let borrowed_bytes = self.borrowed_bytes()?;

        Ok(SharedRawSpaceUsage {
            allocation_count,
            allocated_bytes,
            active_bytes,
            mapped_bytes,
            borrowed_bytes,
        })
    }

    /// Return the mapped live bytes for the current shared raw state.
    fn live_mapped_bytes(&self, allocator: &SharedRawAllocator) -> u64 {
        let entries = self.entries.read();
        let pages = self.live_pages(&entries);
        let cached_bytes = allocator
            .page_run_cache
            .cached_bytes(self.arena.page_bytes());

        self.arena
            .mapped_bytes_for_page_views(pages.iter())
            .saturating_add(cached_bytes)
    }

    /// Return whether one shared raw pointer currently refers to one live entry slot.
    pub fn is_live(&self, pointer: SharedRawPointer) -> bool {
        let Some(entry) = self.entry(pointer) else {
            return false;
        };

        !entry.read().is_vacant()
    }

    /// Allocate one shared raw byte entry.
    pub fn allocate_bytes(&self, bytes: &[u8]) -> HeapResult<SharedRawPointer> {
        let mut allocator = self.allocator.lock();
        let entry_id = self.allocate_entry_id(&mut allocator)?;
        let pages = allocator
            .page_run_cache
            .allocate_bytes(&self.arena, bytes)?;
        let entry = Arc::new(RwLock::new(SharedRawEntry::new(bytes.len(), pages)));

        // install the live entry slot and usage
        let index = Self::entry_index(entry_id)?;
        let mut entries = self.entries.write();

        if index > entries.len() {
            return Err(HeapError::InvalidSharedRawPointerId {
                id: entry_id.into(),
            });
        }

        if index == entries.len() {
            entries.push(entry);
        } else {
            entries[index] = entry;
        }

        allocator
            .usage
            .allocate(bytes.len(), HeapSpace::SharedRaw)?;

        Ok(SharedRawPointer::new(entry_id))
    }

    /// Return the projected mapped-byte delta for one shared entry.
    pub fn alloc_mapped_delta(&self, byte_len: usize) -> i64 {
        self.round_up_allocation_bytes(byte_len) as i64
    }

    /// Return the remaining byte length for one shared raw pointer.
    pub fn byte_len(&self, pointer: SharedRawPointer) -> HeapResult<usize> {
        let Some(entry) = self.entry(pointer) else {
            return Err(HeapError::InvalidSharedRawPointer { pointer });
        };
        let entry = entry.read();

        if entry.is_vacant() {
            return Err(HeapError::InvalidSharedRawPointer { pointer });
        }

        checked_remaining_byte_len(pointer, entry.len)
    }

    /// Return the bytes for one shared raw pointer.
    pub fn read_bytes(&self, pointer: SharedRawPointer) -> HeapResult<Vec<u8>> {
        let Some(entry) = self.entry(pointer) else {
            return Err(HeapError::InvalidSharedRawPointer { pointer });
        };
        let entry = entry.read();

        if entry.is_vacant() {
            return Err(HeapError::InvalidSharedRawPointer { pointer });
        }

        let byte_offset = pointer.byte_offset();
        let byte_len = checked_remaining_byte_len(pointer, entry.len)?;

        self.arena
            .bytes_to_vec_from(&entry.pages, byte_offset, byte_len)
    }

    /// Replace the bytes for one shared raw pointer.
    pub fn replace_bytes(&self, pointer: SharedRawPointer, bytes: &[u8]) -> HeapResult<()> {
        let Some(entry) = self.entry(pointer) else {
            return Err(HeapError::InvalidSharedRawPointer { pointer });
        };

        let mut allocator = self.allocator.lock();
        let next_pages = allocator
            .page_run_cache
            .allocate_bytes(&self.arena, bytes)?;
        let mut entry = entry.write();

        if entry.is_vacant() {
            allocator
                .page_run_cache
                .release_page_view(&self.arena, next_pages)?;

            return Err(HeapError::InvalidSharedRawPointer { pointer });
        }

        let previous_pages = entry.pages;
        let previous_len = entry.len;

        // commit the replacement before releasing the previous pages
        entry.pages = next_pages;
        entry.len = bytes.len();
        allocator
            .usage
            .resize(previous_len, bytes.len(), HeapSpace::SharedRaw)?;
        allocator
            .page_run_cache
            .release_page_view(&self.arena, previous_pages)?;

        Ok(())
    }

    /// Free one shared raw-space entry.
    pub fn free(&self, pointer: SharedRawPointer) -> HeapResult<bool> {
        let Some(entry) = self.entry(pointer) else {
            return Err(HeapError::InvalidSharedRawPointer { pointer });
        };

        let mut allocator = self.allocator.lock();
        let mut entry = entry.write();

        if entry.is_vacant() {
            return Err(HeapError::InvalidSharedRawPointer { pointer });
        }

        let pages = entry.pages;
        let previous_len = entry.len as u64;

        allocator
            .usage
            .check_free(previous_len, HeapSpace::SharedRaw)?;

        // retire the live entry slot before releasing its pages
        entry.retire();
        allocator.free_ids.push(pointer.id().into());
        allocator.usage.free(previous_len, HeapSpace::SharedRaw)?;
        allocator
            .page_run_cache
            .release_page_view(&self.arena, pages)?;

        Ok(true)
    }

    /// Return the projected mapped-byte delta for one shared replacement.
    pub fn replace_mapped_delta(
        &self,
        pointer: SharedRawPointer,
        next_byte_len: usize,
    ) -> HeapResult<i64> {
        let Some(entry) = self.entry(pointer) else {
            return Err(HeapError::InvalidSharedRawPointer { pointer });
        };
        let entry = entry.read();

        if entry.is_vacant() {
            return Err(HeapError::InvalidSharedRawPointer { pointer });
        }

        let previous_mapped_bytes = self.round_up_allocation_bytes(entry.len);
        let next_mapped_bytes = self.round_up_allocation_bytes(next_byte_len);

        Ok(next_mapped_bytes as i64 - previous_mapped_bytes as i64)
    }

    /// Return one allocated shared entry by pointer.
    fn entry(&self, pointer: SharedRawPointer) -> Option<Arc<RwLock<SharedRawEntry>>> {
        let index = pointer.id().checked_sub(1)? as usize;
        let entries = self.entries.read();

        entries.get(index).cloned()
    }

    /// Return the dense table index for one shared pointer id.
    fn entry_index(entry_id: u32) -> HeapResult<usize> {
        let Some(index) = entry_id.checked_sub(1) else {
            return Err(HeapError::InvalidSharedRawPointerId {
                id: entry_id.into(),
            });
        };

        Ok(index as usize)
    }

    /// Allocate one stable shared entry id.
    fn allocate_entry_id(&self, allocator: &mut SharedRawAllocator) -> HeapResult<u32> {
        if let Some(entry_id) = allocator.free_ids.pop() {
            let entry_id = checked_shared_entry_id(entry_id)?;
            let index = Self::entry_index(entry_id)?;
            let entries = self.entries.read();

            if index > entries.len() {
                allocator.free_ids.push(entry_id.into());

                return Err(HeapError::InvalidSharedRawPointerId {
                    id: entry_id.into(),
                });
            }

            if entries
                .get(index)
                .is_some_and(|entry| !entry.read().is_vacant())
            {
                allocator.free_ids.push(entry_id.into());

                return Err(HeapError::InvalidSharedRawPointerId {
                    id: entry_id.into(),
                });
            }

            return Ok(entry_id);
        }

        let entry_id = allocator.next_unused_id;
        let entry_id = checked_shared_entry_id(entry_id)?;

        allocator.next_unused_id = allocator.next_unused_id.checked_add(1).ok_or(
            HeapError::InvalidSharedRawPointerId {
                id: allocator.next_unused_id,
            },
        )?;

        Ok(entry_id)
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

            pages.push(entry.pages);
        }

        pages
    }
}

impl Drop for SharedRawSpace {
    fn drop(&mut self) {
        let mut allocator = self.allocator.lock();

        allocator.page_run_cache.flush(&self.arena);
    }
}

/// Return the visible byte length for one shared raw pointer.
fn checked_remaining_byte_len(pointer: SharedRawPointer, byte_len: usize) -> HeapResult<usize> {
    let byte_offset = pointer.byte_offset();

    if byte_offset > byte_len {
        return Err(HeapError::InvalidSharedRawPointer { pointer });
    }

    Ok(byte_len - byte_offset)
}
