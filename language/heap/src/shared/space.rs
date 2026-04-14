use std::sync::Arc;

use crate::alloc::Arena;
use crate::heap::{DEFAULT_PAGE_BYTES, SharedSpaceUsage};

use super::super::SharedPointer;
use super::region::SharedRegion;

/// The first allocated shared-memory region id.
const FIRST_SHARED_REGION_ID: u64 = 1;

/// One live shared-memory space rooted in one arena.
#[derive(Debug, Clone)]
pub struct SharedSpace {
    /// The shared page arena for every region.
    pub(crate) arena: Arc<Arena>,
    /// Stable shared-memory regions keyed by region id minus one.
    pub(crate) regions: Vec<SharedRegion>,
    /// Free shared-memory region ids available for reuse.
    pub(crate) free_ids: Vec<u64>,
    /// The next shared-memory region id to allocate.
    pub(crate) next_unused_id: u64,
    /// The number of allocated shared-memory regions.
    pub(crate) allocated_count: usize,
    /// The number of allocated shared-memory bytes.
    pub(crate) allocated_bytes: u64,
}

impl Default for SharedSpace {
    fn default() -> Self {
        Self::new()
    }
}

impl SharedSpace {
    /// Create a new empty shared-memory space.
    pub fn new() -> Self {
        Self::with_page_bytes(DEFAULT_PAGE_BYTES)
    }

    /// Create a new empty shared-memory space over one shared arena.
    pub fn with_arena(arena: Arc<Arena>) -> Self {
        Self {
            arena,
            regions: Vec::new(),
            free_ids: Vec::new(),
            next_unused_id: FIRST_SHARED_REGION_ID,
            allocated_count: 0,
            allocated_bytes: 0,
        }
    }

    /// Create a new empty shared-memory space with one explicit page width.
    pub fn with_page_bytes(page_bytes: usize) -> Self {
        Self::with_arena(Arc::new(Arena::with_page_bytes(page_bytes)))
    }

    /// Return the configured shared page width.
    pub fn page_bytes(&self) -> usize {
        self.arena.page_bytes()
    }

    /// Return the exact active shared-memory bytes.
    pub fn active_bytes(&self) -> u64 {
        self.allocated_bytes
    }

    /// Return the exact mapped shared page bytes.
    pub fn mapped_bytes(&self) -> u64 {
        0
    }

    /// Return the exact borrowed shared bytes.
    pub fn borrowed_bytes(&self) -> u64 {
        self.allocated_bytes
    }

    /// Return the exact usage for this live shared-memory space.
    pub fn usage(&self) -> SharedSpaceUsage {
        SharedSpaceUsage {
            allocation_count: self.allocated_count,
            allocated_bytes: self.allocated_bytes,
            active_bytes: self.active_bytes(),
            mapped_bytes: self.mapped_bytes(),
            borrowed_bytes: self.borrowed_bytes(),
        }
    }

    /// Return whether one shared pointer currently refers to one live region slot.
    pub fn contains(&self, region: SharedPointer) -> bool {
        let index = region.id().saturating_sub(1) as usize;
        self.regions
            .get(index)
            .map(|region| region.is_allocated)
            .unwrap_or(false)
    }

    /// Allocate one shared byte region.
    pub fn allocate_bytes(&mut self, bytes: &[u8]) -> SharedPointer {
        let region_id = if let Some(region_id) = self.free_ids.pop() {
            region_id
        } else {
            let region_id = self.next_unused_id;
            self.next_unused_id = self.next_unused_id.saturating_add(1);
            region_id
        };

        let pages = self.arena.allocate_bytes(bytes);
        let region = SharedRegion {
            is_allocated: true,
            len: bytes.len(),
            pages,
        };
        let index = region_id.saturating_sub(1) as usize;

        if index == self.regions.len() {
            self.regions.push(region);
        } else if let Some(existing) = self.regions.get_mut(index) {
            if existing.is_allocated {
                self.arena.release_pages(&existing.pages);
            }
            *existing = region;
        }

        self.allocated_count = self.allocated_count.saturating_add(1);
        self.allocated_bytes = self.allocated_bytes.saturating_add(bytes.len() as u64);

        SharedPointer::new(region_id)
    }
}
