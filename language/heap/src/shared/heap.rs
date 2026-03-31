use std::mem::size_of;

use crate::alloc::PageArena;
use crate::heap::{DEFAULT_PAGE_BYTES, SharedBudget, SharedLimitError, SharedSpaceUsage};

use super::super::SharedPointer;
use super::region::SharedRegion;

/// The first allocated shared-memory region id.
const FIRST_SHARED_REGION_ID: u64 = 1;

/// One live shared-memory space.
#[derive(Debug)]
pub struct SharedSpace {
    /// The configured local page width.
    pub(crate) page_bytes: usize,
    /// Stable shared-memory regions keyed by region id minus one.
    pub(crate) regions: Vec<SharedRegion>,
    /// The local page arena for shared-region backing.
    pub(crate) page_arena: PageArena,
    /// Free shared-memory region ids available for reuse.
    pub(crate) free_ids: Vec<u64>,
    /// The next shared-memory region id to allocate.
    pub(crate) next_unused_id: u64,
    /// The number of allocated shared-memory regions.
    pub(crate) allocated_count: usize,
    /// The number of allocated shared-memory bytes.
    pub(crate) allocated_bytes: u64,
    /// The exact retained shared-memory bytes.
    pub(crate) retained_bytes: u64,
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

    /// Create a new empty shared-memory space with one explicit page width.
    pub fn with_page_bytes(page_bytes: usize) -> Self {
        let mut space = Self {
            page_bytes,
            regions: Vec::new(),
            page_arena: PageArena::with_page_bytes(page_bytes),
            free_ids: Vec::new(),
            next_unused_id: FIRST_SHARED_REGION_ID,
            allocated_count: 0,
            allocated_bytes: 0,
            retained_bytes: 0,
        };

        // exact retained bytes
        space.recompute_retained_bytes();

        space
    }

    /// Return the configured shared page width.
    pub fn page_bytes(&self) -> usize {
        self.page_bytes
    }

    /// Allocate one shared-memory region with the given payload.
    pub fn allocate_bytes(
        &mut self,
        bytes: &[u8],
        budget: &mut SharedBudget,
    ) -> Result<SharedPointer, SharedLimitError> {
        // reserve retained bytes before mutation
        let retained_delta = self.allocate_delta(bytes.len());
        budget.check_retained_delta(retained_delta)?;

        // commit allocation and accounting
        let region = self.commit_allocate_bytes(bytes);
        self.apply_retained_delta(retained_delta);
        budget.apply_retained_delta(retained_delta);

        Ok(region)
    }

    /// Return one owned copy of the shared-memory bytes for this region.
    pub fn bytes_to_vec(&self, region: SharedPointer) -> Option<Vec<u8>> {
        Some(self.region(region)?.bytes_to_vec(&self.page_arena))
    }

    /// Return one byte by slot offset.
    pub fn byte_at(&self, region: SharedPointer, index: usize) -> Option<u8> {
        self.region(region)?.get(&self.page_arena, index)
    }

    /// Return the shared-memory byte length for this region.
    pub fn byte_len(&self, region: SharedPointer) -> Option<usize> {
        Some(self.region(region)?.len())
    }

    /// Free one shared-memory region.
    pub fn free(&mut self, region: SharedPointer) -> bool {
        let region_id = region.id();

        // read the current region accounting before taking a mutable borrow
        let Some(region_metrics) = self.region(region) else {
            return false;
        };
        let region_bytes = region_metrics.len();
        let region_retained_bytes = region_metrics.retained_bytes();

        // free the live region payload while keeping the stable id slot
        let region_index = (region.id().saturating_sub(1)) as usize;
        let Some(region_slot) = self.regions.get_mut(region_index) else {
            return false;
        };
        if !region_slot.is_allocated() {
            return false;
        }

        region_slot.free(&mut self.page_arena);

        // release usage and retained accounting
        self.allocated_count = self.allocated_count.saturating_sub(1);
        self.allocated_bytes = self.allocated_bytes.saturating_sub(region_bytes as u64);
        self.apply_retained_delta(size_of::<u64>() as i64 - region_retained_bytes as i64);

        // publish the reusable stable region id
        self.free_ids.push(region_id);

        true
    }

    /// Write one shared-memory byte by slot offset.
    pub fn set_byte(&mut self, region: SharedPointer, index: usize, byte: u8) -> bool {
        if region.id() == 0 {
            return false;
        }

        let Some(region) = self.regions.get_mut((region.id() - 1) as usize) else {
            return false;
        };

        if !region.is_allocated() {
            return false;
        }

        region.set(&mut self.page_arena, index, byte)
    }

    /// Replace the entire shared-memory payload with exact retained-byte admission.
    pub fn replace_bytes(
        &mut self,
        region: SharedPointer,
        bytes: &[u8],
        budget: &mut SharedBudget,
    ) -> Result<bool, SharedLimitError> {
        // reserve retained bytes before mutation
        let Some(delta) = self.replace_bytes_delta(region, bytes.len()) else {
            return Ok(false);
        };

        budget.check_retained_delta(delta)?;

        // commit replacement and accounting
        let replaced = self.commit_replace_bytes(region, bytes);

        if replaced {
            self.apply_retained_delta(delta);
            budget.apply_retained_delta(delta);
        }

        Ok(replaced)
    }

    /// Replace the entire shared-memory payload.
    fn commit_replace_bytes(&mut self, region: SharedPointer, bytes: &[u8]) -> bool {
        let page_bytes = self.page_bytes;
        if region.id() == 0 {
            return false;
        }

        let Some(region) = self.regions.get_mut((region.id() - 1) as usize) else {
            return false;
        };

        if !region.is_allocated() {
            return false;
        }

        let old_len = region.len();
        region.replace(&mut self.page_arena, bytes, page_bytes);
        self.allocated_bytes = self
            .allocated_bytes
            .saturating_sub(old_len as u64)
            .saturating_add(bytes.len() as u64);

        true
    }

    /// Return the number of allocated shared-memory regions.
    pub fn allocation_count(&self) -> usize {
        self.allocated_count
    }

    /// Return the exact retained shared-memory bytes.
    pub fn retained_bytes(&self) -> u64 {
        self.retained_bytes
    }

    /// Return the exact usage for this live shared-memory space.
    pub fn usage(&self) -> SharedSpaceUsage {
        SharedSpaceUsage {
            allocation_count: self.allocated_count,
            allocation_bytes: self.allocated_bytes,
            retained_bytes: self.retained_bytes,
        }
    }

    /// Return one shared-memory region by identifier.
    fn region(&self, region: SharedPointer) -> Option<&SharedRegion> {
        if region.id() == 0 {
            return None;
        }

        let region = self.regions.get((region.id() - 1) as usize)?;
        if !region.is_allocated() {
            return None;
        }

        Some(region)
    }

    /// Allocate one shared-memory region.
    fn commit_allocate_bytes(&mut self, bytes: &[u8]) -> SharedPointer {
        if let Some(id) = self.free_ids.pop() {
            let region = self
                .regions
                .get_mut((id - 1) as usize)
                .expect("reused shared region id must stay addressable");
            region.allocate(&mut self.page_arena, bytes, self.page_bytes);
            self.allocated_count += 1;
            self.allocated_bytes = self.allocated_bytes.saturating_add(bytes.len() as u64);

            return SharedPointer::new(id);
        }

        let id = self.next_unused_id;
        self.next_unused_id = self.next_unused_id.saturating_add(1);
        self.regions.push(SharedRegion::new(
            bytes,
            self.page_bytes,
            &mut self.page_arena,
        ));
        self.allocated_count += 1;
        self.allocated_bytes = self.allocated_bytes.saturating_add(bytes.len() as u64);

        SharedPointer::new(id)
    }

    /// Return the exact retained-byte delta for allocating one shared-memory region.
    fn allocate_delta(&self, len: usize) -> i64 {
        let region_bytes = self.region_payload_retained_bytes(len) as i64;
        if self.free_ids.is_empty() {
            region_bytes + size_of::<SharedRegion>() as i64
        } else {
            region_bytes - size_of::<u64>() as i64
        }
    }

    /// Return the exact retained-byte delta for replacing one shared-memory payload.
    fn replace_bytes_delta(&self, region: SharedPointer, new_len: usize) -> Option<i64> {
        let old_region = self.region(region)?;
        let old_bytes = old_region.retained_bytes() as i64;
        let new_bytes = self.region_payload_retained_bytes(new_len) as i64;

        Some(new_bytes - old_bytes)
    }

    /// Return the retained payload bytes for one region length.
    pub(crate) fn region_payload_retained_bytes(&self, len: usize) -> usize {
        SharedRegion::retained_bytes_for_len(len, self.page_bytes)
    }

    /// Apply one exact retained-byte delta after a committed mutation.
    pub(crate) fn apply_retained_delta(&mut self, delta: i64) {
        if delta >= 0 {
            self.retained_bytes = self.retained_bytes.saturating_add(delta as u64);
        } else {
            self.retained_bytes = self.retained_bytes.saturating_sub((-delta) as u64);
        }
    }

    /// Recompute exact retained bytes from live state.
    pub(crate) fn recompute_retained_bytes(&mut self) {
        let mut retained_bytes = 0usize;

        // live region descriptors
        retained_bytes += self.regions.capacity() * size_of::<SharedRegion>();

        // live region payload
        for region in &self.regions {
            retained_bytes += region.retained_bytes();
        }

        // allocator state
        retained_bytes += self.free_ids.capacity() * size_of::<u64>();
        retained_bytes += self.page_arena.retained_bytes();

        self.retained_bytes = retained_bytes as u64;
    }
}
