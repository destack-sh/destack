use std::sync::Arc;

use super::SharedRawEntry;
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

/// One live shared raw-space store rooted in one arena.
#[derive(Debug)]
pub struct SharedRawSpace {
    /// The shared raw-space arena for every entry.
    pub(crate) arena: Arc<Arena>,

    /// Stable shared raw-space entries keyed by entry id minus one.
    pub(crate) entries: Vec<SharedRawEntry>,
    /// Free shared raw-space entry ids available for reuse.
    pub(crate) free_ids: Vec<u64>,
    /// The next shared raw-space entry id to allocate.
    pub(crate) next_unused_id: u64,

    /// The exact live shared raw-space usage.
    pub(crate) usage: AllocationUsage,
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
        Self {
            arena,
            entries: Vec::new(),
            free_ids: Vec::new(),
            next_unused_id: FIRST_SHARED_ENTRY_ID,
            usage: AllocationUsage::default(),
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
        self.arena
            .mapped_bytes_for_page_views(self.entries.iter().map(|entry| &entry.pages))
    }

    /// Return the exact borrowed shared bytes.
    pub fn borrowed_bytes(&self) -> HeapResult<u64> {
        self.arena
            .borrowed_bytes_for_page_views(self.entries.iter().map(|entry| &entry.pages))
    }

    /// Return the exact usage for this live shared raw-space store.
    pub fn usage(&self) -> HeapResult<SharedRawSpaceUsage> {
        Ok(SharedRawSpaceUsage {
            allocation_count: self.usage.allocation_count(),
            allocated_bytes: self.usage.allocated_bytes(),
            active_bytes: self.active_bytes(),
            mapped_bytes: self.mapped_bytes(),
            borrowed_bytes: self.borrowed_bytes()?,
        })
    }

    /// Return whether one shared raw pointer currently refers to one live entry slot.
    pub fn is_live(&self, pointer: SharedRawPointer) -> bool {
        self.entry(pointer).is_some()
    }

    /// Allocate one shared raw byte entry.
    pub fn allocate_bytes(&mut self, bytes: &[u8]) -> HeapResult<SharedRawPointer> {
        let entry_id = self.allocate_entry_id()?;
        let pages = self.arena.allocate_bytes(bytes)?;
        let entry = SharedRawEntry::new(bytes.len(), pages);

        // install the live entry slot and usage
        self.set_entry(entry_id, entry)?;
        self.usage.allocate(bytes.len(), HeapSpace::SharedRaw)?;

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

        checked_remaining_byte_len(pointer, entry.len)
    }

    /// Return the bytes for one shared raw pointer.
    pub fn read_bytes(&self, pointer: SharedRawPointer) -> HeapResult<Vec<u8>> {
        // resolve the live entry first
        let Some(entry) = self.entry(pointer) else {
            return Err(HeapError::InvalidSharedRawPointer { pointer });
        };
        let byte_offset = pointer.byte_offset();
        let byte_len = checked_remaining_byte_len(pointer, entry.len)?;

        // then materialize the requested logical range
        self.arena
            .bytes_to_vec_from(&entry.pages, byte_offset, byte_len)
    }

    /// Replace the bytes for one shared raw pointer.
    pub fn replace_bytes(&mut self, pointer: SharedRawPointer, bytes: &[u8]) -> HeapResult<()> {
        // resolve the live entry and shared arena first
        let arena = self.arena.clone();
        let Some(previous_entry) = self.entry(pointer) else {
            return Err(HeapError::InvalidSharedRawPointer { pointer });
        };
        let previous_pages = previous_entry.pages;
        let next_pages = arena.allocate_bytes(bytes)?;
        let previous_len = previous_entry.len;

        let Some(entry) = self.entry_mut(pointer) else {
            return Err(HeapError::InvalidSharedRawPointer { pointer });
        };

        // commit the replacement before releasing the previous pages
        entry.pages = next_pages;

        // update the live byte count
        entry.len = bytes.len();
        self.usage
            .resize(previous_len, bytes.len(), HeapSpace::SharedRaw)?;

        // release the previous page view after commit
        arena.release_page_view(&previous_pages)?;

        Ok(())
    }

    /// Free one shared raw-space entry.
    pub fn free(&mut self, pointer: SharedRawPointer) -> HeapResult<bool> {
        // resolve the live entry first
        let entry_id = pointer.id();
        let Some(entry) = self.entry(pointer) else {
            return Err(HeapError::InvalidSharedRawPointer { pointer });
        };
        let pages = entry.pages;
        let previous_len = entry.len as u64;

        self.usage.check_free(previous_len, HeapSpace::SharedRaw)?;

        // retire the live entry slot before releasing its pages
        self.retire_entry(entry_id)?;

        // update shared usage before releasing the old pages
        self.usage.free(previous_len, HeapSpace::SharedRaw)?;

        // release the old physical pages after the live slot is gone
        self.arena.release_page_view(&pages)?;

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

        let previous_mapped_bytes = self.round_up_allocation_bytes(entry.len);
        let next_mapped_bytes = self.round_up_allocation_bytes(next_byte_len);

        Ok(next_mapped_bytes as i64 - previous_mapped_bytes as i64)
    }

    /// Return one allocated shared entry by pointer.
    fn entry(&self, pointer: SharedRawPointer) -> Option<&SharedRawEntry> {
        // resolve the dense entry slot first
        let index = pointer.id().checked_sub(1)? as usize;
        let entry = self.entries.get(index)?;

        // skip free entry entries
        (!entry.is_vacant()).then_some(entry)
    }

    /// Return one live shared entry mutably by pointer.
    fn entry_mut(&mut self, pointer: SharedRawPointer) -> Option<&mut SharedRawEntry> {
        // resolve the dense entry slot first
        let index = pointer.id().checked_sub(1)? as usize;
        let entry = self.entries.get_mut(index)?;

        // skip free entry entries
        (!entry.is_vacant()).then_some(entry)
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
    fn allocate_entry_id(&mut self) -> HeapResult<u32> {
        // reuse one freed entry id when possible
        if let Some(entry_id) = self.free_ids.pop() {
            let entry_id = checked_shared_entry_id(entry_id)?;
            let index = Self::entry_index(entry_id)?;

            if index > self.entries.len() {
                self.free_ids.push(entry_id.into());

                return Err(HeapError::InvalidSharedRawPointerId {
                    id: entry_id.into(),
                });
            }

            if self
                .entries
                .get(index)
                .is_some_and(|entry| !entry.is_vacant())
            {
                self.free_ids.push(entry_id.into());

                return Err(HeapError::InvalidSharedRawPointerId {
                    id: entry_id.into(),
                });
            }

            Ok(entry_id)
        }
        // otherwise allocate from the unused tail
        else {
            let entry_id = self.next_unused_id;
            let entry_id = checked_shared_entry_id(entry_id)?;

            self.next_unused_id =
                self.next_unused_id
                    .checked_add(1)
                    .ok_or(HeapError::InvalidSharedRawPointerId {
                        id: self.next_unused_id,
                    })?;

            Ok(entry_id)
        }
    }

    /// Store one dense shared entry by stable entry id.
    fn set_entry(&mut self, entry_id: u32, entry: SharedRawEntry) -> HeapResult<()> {
        let index = Self::entry_index(entry_id)?;

        if index > self.entries.len() {
            return Err(HeapError::InvalidSharedRawPointerId {
                id: entry_id.into(),
            });
        }

        if index == self.entries.len() {
            self.entries.push(entry);
        } else {
            self.entries[index] = entry;
        }

        Ok(())
    }

    /// Retire one stable shared entry slot.
    fn retire_entry(&mut self, entry_id: u32) -> HeapResult<()> {
        let index = Self::entry_index(entry_id)?;
        let Some(entry) = self.entries.get_mut(index) else {
            return Err(HeapError::InvalidSharedRawPointerId {
                id: entry_id.into(),
            });
        };

        entry.retire();
        self.free_ids.push(entry_id.into());

        Ok(())
    }

    /// Return the page-rounded mapped bytes for one shared entry.
    fn round_up_allocation_bytes(&self, byte_len: usize) -> u64 {
        let page_bytes = self.page_bytes() as u64;
        let byte_len = byte_len as u64;

        byte_len.div_ceil(page_bytes) * page_bytes
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
