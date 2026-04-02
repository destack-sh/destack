use std::mem::size_of;

use destack_mir::LayoutId;
use serde::{Deserialize, Serialize};

use super::{ReferenceMapId, StoredLayoutId};
use crate::alloc::{PageArena, PageId, projected_vec_capacity, vec_capacity_bytes_delta};

/// One stable managed young-allocation identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct ManagedYoungId {
    /// The young-space generation that owns this allocation.
    generation: u32,
    /// The zero-based allocation index inside that generation.
    index: u32,
}

impl ManagedYoungId {
    /// Create one managed young-allocation identifier.
    pub(crate) const fn new(generation: u32, index: usize) -> Self {
        Self {
            generation,
            index: index as u32,
        }
    }

    /// Return the owning young-space generation.
    pub(crate) const fn generation(self) -> u32 {
        self.generation
    }

    /// Return the zero-based young-allocation index.
    pub(crate) const fn index(self) -> usize {
        self.index as usize
    }
}

/// One live young-allocation metadata entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct YoungAllocation {
    /// The byte offset in the logical young-space page slice.
    start: usize,
    /// The logical byte length for this allocation.
    byte_len: usize,
    /// The interned reference map for this allocation.
    trace_id: ReferenceMapId,
    /// The durable layout id for this allocation, if any.
    layout_id: StoredLayoutId,
    /// The survivor age for this allocation.
    age: u8,
    /// Whether this young allocation is still live.
    is_allocated: bool,
    /// Whether this young allocation is marked in the current collection.
    marked: bool,
}

/// One live managed young-allocation space.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct YoungSpace {
    /// The generation number for this young space.
    generation: u32,
    /// The configured byte capacity for the young space.
    capacity_bytes: usize,
    /// The fixed page width for young storage.
    page_bytes: usize,
    /// The bump-allocation cursor inside the logical young byte space.
    next_offset: usize,
    /// The local pages backing this young space.
    pages: Vec<PageId>,
    /// The live young-allocation metadata entries.
    allocations: Vec<YoungAllocation>,
    /// The reusable metadata entry ids.
    free_ids: Vec<u32>,
}

impl YoungSpace {
    /// Create one managed young space with the given byte capacity.
    pub(crate) fn with_capacity(capacity_bytes: usize, page_bytes: usize) -> Self {
        Self::with_generation(capacity_bytes, page_bytes, 0)
    }

    /// Create one managed young space with the given byte capacity and generation.
    pub(crate) fn with_generation(
        capacity_bytes: usize,
        page_bytes: usize,
        generation: u32,
    ) -> Self {
        Self {
            generation,
            capacity_bytes,
            page_bytes: page_bytes.max(1),
            next_offset: 0,
            pages: Vec::new(),
            allocations: Vec::new(),
            free_ids: Vec::new(),
        }
    }

    /// Return the generation number for this young space.
    pub(crate) fn generation(&self) -> u32 {
        self.generation
    }

    /// Create one empty successor young space.
    pub(crate) fn successor(&self) -> Self {
        Self::with_generation(
            self.capacity_bytes,
            self.page_bytes,
            self.generation.saturating_add(1),
        )
    }

    /// Return the configured young-space byte capacity.
    pub(crate) fn capacity_bytes(&self) -> usize {
        self.capacity_bytes
    }

    /// Return the current local-page vector capacity.
    pub(crate) fn pages_capacity(&self) -> usize {
        self.pages.capacity()
    }

    /// Return the current young-page count.
    pub(crate) fn page_count(&self) -> usize {
        self.pages.len()
    }

    /// Return the young-allocation metadata capacity.
    pub(crate) fn allocations_capacity(&self) -> usize {
        self.allocations.capacity()
    }

    /// Return the free-id capacity.
    pub(crate) fn free_ids_capacity(&self) -> usize {
        self.free_ids.capacity()
    }

    /// Report whether the young space can currently fit the given byte length.
    pub(crate) fn can_fit(&self, byte_len: usize) -> bool {
        let Some(start) = self.allocation_start(byte_len) else {
            return false;
        };
        let Some(end) = start.checked_add(byte_len) else {
            return false;
        };

        end <= self.capacity_bytes
    }

    /// Report whether this young space has no live allocations.
    pub(crate) fn is_empty(&self) -> bool {
        self.allocations
            .iter()
            .all(|allocation| !allocation.is_allocated)
    }

    /// Allocate one young payload from the given bytes.
    pub(crate) fn allocate(
        &mut self,
        bytes: &[u8],
        trace_id: ReferenceMapId,
        layout_id: Option<LayoutId>,
        page_arena: &mut PageArena,
    ) -> Option<ManagedYoungId> {
        let (id, start) =
            self.allocate_entry(bytes.len(), trace_id, layout_id, 0, false, page_arena)?;
        if !page_arena.write_window(&self.pages, start, bytes) {
            return None;
        }

        Some(id)
    }

    /// Allocate one zeroed young payload with the given byte length.
    pub(crate) fn allocate_zeroed(
        &mut self,
        byte_len: usize,
        trace_id: ReferenceMapId,
        layout_id: Option<LayoutId>,
        page_arena: &mut PageArena,
    ) -> Option<ManagedYoungId> {
        let (id, _) = self.allocate_entry(byte_len, trace_id, layout_id, 0, false, page_arena)?;
        Some(id)
    }

    /// Allocate one copied survivor with the given age.
    pub(crate) fn allocate_survivor(
        &mut self,
        bytes: &[u8],
        trace_id: ReferenceMapId,
        layout_id: Option<LayoutId>,
        age: u8,
        page_arena: &mut PageArena,
    ) -> Option<ManagedYoungId> {
        let (id, start) =
            self.allocate_entry(bytes.len(), trace_id, layout_id, age, true, page_arena)?;
        if !page_arena.write_window(&self.pages, start, bytes) {
            return None;
        }

        Some(id)
    }

    /// Return the logical byte length for one young allocation.
    pub(crate) fn byte_len(&self, id: ManagedYoungId) -> Option<usize> {
        Some(self.allocation(id)?.byte_len)
    }

    /// Return the bytes for one live young allocation.
    pub(crate) fn bytes<'a>(
        &'a self,
        id: ManagedYoungId,
        page_arena: &'a PageArena,
    ) -> Option<&'a [u8]> {
        let allocation = self.allocation(id)?;
        page_arena.borrow_window(&self.pages, allocation.start, allocation.byte_len)
    }

    /// Return the trace id for one live young allocation.
    pub(crate) fn trace_id(&self, id: ManagedYoungId) -> Option<ReferenceMapId> {
        Some(self.allocation(id)?.trace_id)
    }

    /// Return the survivor age for one live young allocation.
    pub(crate) fn age(&self, id: ManagedYoungId) -> Option<u8> {
        Some(self.allocation(id)?.age)
    }

    /// Return the layout id for one live young allocation, if any.
    pub(crate) fn layout_id(&self, id: ManagedYoungId) -> Option<LayoutId> {
        self.allocation(id)?.layout_id.to_option()
    }

    /// Set the layout id for one live young allocation.
    #[cfg(test)]
    pub(crate) fn set_layout_id(&mut self, id: ManagedYoungId, layout_id: LayoutId) -> bool {
        let Some(allocation) = self.allocation_mut(id) else {
            return false;
        };

        allocation.layout_id = StoredLayoutId::from_option(Some(layout_id));
        true
    }

    /// Report whether one young allocation is still live.
    pub(crate) fn is_allocated(&self, id: ManagedYoungId) -> bool {
        self.allocation(id).is_some()
    }

    /// Clear all young-allocation marks.
    pub(crate) fn clear_marks(&mut self) {
        for allocation in &mut self.allocations {
            allocation.marked = false;
        }
    }

    /// Write one byte inside one live young allocation.
    pub(crate) fn set_byte(
        &mut self,
        id: ManagedYoungId,
        offset: usize,
        byte: u8,
        page_arena: &mut PageArena,
    ) -> bool {
        self.set_bytes(id, offset, &[byte], page_arena)
    }

    /// Write one byte slice inside one live young allocation.
    pub(crate) fn set_bytes(
        &mut self,
        id: ManagedYoungId,
        offset: usize,
        bytes: &[u8],
        page_arena: &mut PageArena,
    ) -> bool {
        let Some((start, byte_len)) = self
            .allocation(id)
            .map(|allocation| (allocation.start, allocation.byte_len))
        else {
            return false;
        };
        let Some(end) = offset.checked_add(bytes.len()) else {
            return false;
        };

        if end > byte_len {
            return false;
        }

        page_arena.write_window(&self.pages, start + offset, bytes)
    }

    /// Free one live young allocation without reclaiming arena space.
    pub(crate) fn free(&mut self, id: ManagedYoungId) -> bool {
        let Some(allocation) = self.allocation_mut(id) else {
            return false;
        };

        allocation.is_allocated = false;
        allocation.age = 0;
        allocation.marked = false;
        self.free_ids.push(id.index);
        true
    }

    /// Reset the young space after all live allocations have been promoted or freed.
    pub(crate) fn reset(&mut self, page_arena: &mut PageArena) {
        page_arena.free_pages(&self.pages);
        self.next_offset = 0;
        self.pages.clear();
        self.allocations.clear();
        self.free_ids.clear();
    }

    /// Return the retained heap bytes for this young space.
    pub(crate) fn retained_bytes(&self) -> usize {
        self.pages.capacity() * size_of::<PageId>()
            + self.allocations.capacity() * size_of::<YoungAllocation>()
            + self.free_ids.capacity() * size_of::<u32>()
    }

    /// Return the retained-byte delta and required page count for one new allocation.
    pub(crate) fn allocate_retained_delta(&self, byte_len: usize) -> Option<(usize, i64)> {
        let start = self.allocation_start(byte_len)?;
        let end = start.checked_add(byte_len)?;
        if end > self.capacity_bytes {
            return None;
        }

        let page_count = end.div_ceil(self.page_bytes);
        let new_page_count = page_count.saturating_sub(self.pages.len());
        let pages_capacity = projected_vec_capacity::<PageId>(
            self.pages.len(),
            self.pages.capacity(),
            new_page_count,
        );
        let allocations_delta = if self.free_ids.is_empty() {
            let allocations_capacity = projected_vec_capacity::<YoungAllocation>(
                self.allocations.len(),
                self.allocations.capacity(),
                1,
            );
            vec_capacity_bytes_delta::<YoungAllocation>(
                self.allocations.capacity(),
                allocations_capacity,
            )
        } else {
            0
        };
        let pages_delta = vec_capacity_bytes_delta::<PageId>(self.pages.capacity(), pages_capacity);

        Some((new_page_count, pages_delta + allocations_delta))
    }

    // allocate one young metadata entry and byte window
    fn allocate_entry(
        &mut self,
        byte_len: usize,
        trace_id: ReferenceMapId,
        layout_id: Option<LayoutId>,
        age: u8,
        marked: bool,
        page_arena: &mut PageArena,
    ) -> Option<(ManagedYoungId, usize)> {
        let start = self.allocation_start(byte_len)?;
        let end = start.checked_add(byte_len)?;
        if end > self.capacity_bytes {
            return None;
        }

        let page_count = end.div_ceil(self.page_bytes);
        while self.pages.len() < page_count {
            self.pages.push(page_arena.allocate_zeroed());
        }
        self.next_offset = end;

        let allocation = YoungAllocation {
            start,
            byte_len,
            trace_id,
            layout_id: StoredLayoutId::from_option(layout_id),
            age,
            is_allocated: true,
            marked,
        };

        let id = if let Some(id) = self.free_ids.pop() {
            let index = id as usize;
            self.allocations[index] = allocation;
            ManagedYoungId::new(self.generation, index)
        } else {
            self.allocations.push(allocation);
            ManagedYoungId::new(self.generation, self.allocations.len() - 1)
        };

        Some((id, start))
    }

    // compute one aligned allocation start
    fn allocation_start(&self, byte_len: usize) -> Option<usize> {
        if byte_len > self.page_bytes {
            return None;
        }

        let page_offset = self.next_offset % self.page_bytes;
        if page_offset == 0 || page_offset + byte_len <= self.page_bytes {
            return Some(self.next_offset);
        }

        let aligned = self
            .next_offset
            .checked_add(self.page_bytes.saturating_sub(page_offset))?;

        Some(aligned)
    }

    // return one live young allocation
    fn allocation(&self, id: ManagedYoungId) -> Option<&YoungAllocation> {
        if id.generation() != self.generation {
            return None;
        }

        let allocation = self.allocations.get(id.index())?;

        if allocation.is_allocated {
            Some(allocation)
        } else {
            None
        }
    }

    // return one mutable live young allocation
    fn allocation_mut(&mut self, id: ManagedYoungId) -> Option<&mut YoungAllocation> {
        if id.generation() != self.generation {
            return None;
        }

        let allocation = self.allocations.get_mut(id.index())?;

        if allocation.is_allocated {
            Some(allocation)
        } else {
            None
        }
    }
}
