use serde::{Deserialize, Serialize};

use super::{ReferenceMapId, StoredLayoutId};
use crate::alloc::{Arena, PageMap};

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
    pub(crate) const fn new(generation: u32, index: u32) -> Self {
        Self { generation, index }
    }

    /// Return the owning young-space generation.
    pub(crate) const fn generation(self) -> u32 {
        self.generation
    }

    /// Return the zero-based young-allocation index.
    pub(crate) const fn index(self) -> u32 {
        self.index
    }
}

/// One live young-allocation metadata entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct YoungAllocation {
    /// The first page touched by this allocation.
    pub(crate) first_page: u32,
    /// The first byte offset inside the first page.
    pub(crate) first_offset: u32,
    /// The logical byte length for this allocation.
    pub(crate) byte_len: usize,
    /// The interned reference map for this allocation.
    pub(crate) trace_id: ReferenceMapId,
    /// The durable layout id for this allocation, if any.
    pub(crate) layout_id: StoredLayoutId,
    /// The survivor age for this allocation.
    pub(crate) age: u8,
    /// Whether this young allocation is still live.
    pub(crate) is_allocated: bool,
    /// Whether this young allocation is marked in the current collection.
    pub(crate) marked: bool,
}

/// One frozen young-space root.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct YoungImage {
    /// The generation number for this young space.
    generation: u32,
    /// The configured byte capacity for the young space.
    capacity_bytes: usize,
    /// The fixed page width for young storage.
    page_bytes: usize,
    /// The bump-allocation cursor inside the logical young byte space.
    next_offset: usize,
    /// The arena pages backing this young space.
    pages: PageMap,
    /// The captured young-allocation metadata entries.
    allocations: Box<[YoungAllocation]>,
    /// The reusable metadata entry ids.
    free_ids: Box<[u32]>,
}

/// One serialized young-space snapshot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct YoungSnapshot {
    /// The generation number for this young space.
    pub generation: u32,
    /// The configured byte capacity for the young space.
    pub capacity_bytes: usize,
    /// The fixed page width for young storage.
    pub page_bytes: usize,
    /// The bump-allocation cursor inside the logical young byte space.
    pub next_offset: usize,
    /// The arena pages backing this young space.
    pub pages: PageMap,
    /// The serialized young-allocation metadata entries.
    pub allocations: Box<[YoungAllocation]>,
    /// The reusable metadata entry ids.
    pub free_ids: Box<[u32]>,
}

/// One live managed young-allocation space.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct YoungSpace {
    /// The generation number for this young space.
    pub(crate) generation: u32,
    /// The configured byte capacity for the young space.
    pub(crate) capacity_bytes: usize,
    /// The fixed page width for young storage.
    pub(crate) page_bytes: usize,
    /// The bump-allocation cursor inside the logical young byte space.
    pub(crate) next_offset: usize,
    /// The arena pages backing this young space.
    pub(crate) pages: PageMap,
    /// The live young-allocation metadata entries.
    pub(crate) allocations: Box<[YoungAllocation]>,
    /// The reusable metadata entry ids.
    pub(crate) free_ids: Box<[u32]>,
}

impl YoungSpace {
    /// Create one empty young space with its full nursery reservation.
    pub(crate) fn new(arena: &Arena, capacity_bytes: usize, page_bytes: usize) -> Self {
        Self {
            generation: 0,
            capacity_bytes,
            page_bytes,
            next_offset: 0,
            pages: arena.allocate_zeroed(capacity_bytes),
            allocations: Box::new([]),
            free_ids: Box::new([]),
        }
    }

    /// Reset this young space with a fresh nursery reservation.
    pub(crate) fn reset(&mut self, arena: &Arena) {
        arena.release_pages(&self.pages);
        self.generation = self.generation.saturating_add(1);
        self.next_offset = 0;
        self.pages = arena.allocate_zeroed(self.capacity_bytes);
        self.allocations = Box::new([]);
        self.free_ids = Box::new([]);
    }
}

impl YoungImage {
    /// Create one frozen young-space root.
    pub(crate) fn new(
        generation: u32,
        capacity_bytes: usize,
        page_bytes: usize,
        next_offset: usize,
        pages: PageMap,
        allocations: Box<[YoungAllocation]>,
        free_ids: Box<[u32]>,
    ) -> Self {
        Self {
            generation,
            capacity_bytes,
            page_bytes,
            next_offset,
            pages,
            allocations,
            free_ids,
        }
    }

    /// Build one frozen young-space root from one serialized snapshot.
    pub(crate) fn from_snapshot(snapshot: &YoungSnapshot) -> Self {
        Self {
            generation: snapshot.generation,
            capacity_bytes: snapshot.capacity_bytes,
            page_bytes: snapshot.page_bytes,
            next_offset: snapshot.next_offset,
            pages: snapshot.pages.clone(),
            allocations: snapshot.allocations.clone(),
            free_ids: snapshot.free_ids.clone(),
        }
    }

    /// Flatten one frozen young-space root into one serialized snapshot.
    pub(crate) fn snapshot(&self) -> YoungSnapshot {
        YoungSnapshot {
            generation: self.generation,
            capacity_bytes: self.capacity_bytes,
            page_bytes: self.page_bytes,
            next_offset: self.next_offset,
            pages: self.pages.clone(),
            allocations: self.allocations.clone(),
            free_ids: self.free_ids.clone(),
        }
    }

    /// Return the generation number.
    pub(crate) const fn generation(&self) -> u32 {
        self.generation
    }

    /// Return the young-space byte capacity.
    pub(crate) const fn capacity_bytes(&self) -> usize {
        self.capacity_bytes
    }

    /// Return the young page width.
    pub(crate) const fn page_bytes(&self) -> usize {
        self.page_bytes
    }

    /// Return the next bump offset.
    pub(crate) const fn next_offset(&self) -> usize {
        self.next_offset
    }

    /// Return the arena pages for this young root.
    pub(crate) fn pages(&self) -> &PageMap {
        &self.pages
    }

    /// Return the allocation table.
    pub(crate) fn allocations(&self) -> &[YoungAllocation] {
        &self.allocations
    }

    /// Return the reusable young ids.
    pub(crate) fn free_ids(&self) -> &[u32] {
        &self.free_ids
    }
}
