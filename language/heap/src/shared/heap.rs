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
        // reserve active bytes before mutation
        let active_reservation = self.allocate_active_reservation(bytes.len());
        budget.check_active_reservation(active_reservation)?;

        // commit allocation and accounting
        let region = self.commit_allocate_bytes(bytes);
        self.recompute_retained_bytes();
        budget.refresh(self.active_bytes());

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
        let _region_active_bytes = region_metrics.active_bytes();

        // free the live region payload while keeping the stable id slot
        let region_index = (region.id().saturating_sub(1)) as usize;
        let Some(region_slot) = self.regions.get_mut(region_index) else {
            return false;
        };
        if !region_slot.is_allocated() {
            return false;
        }

        region_slot.free(&mut self.page_arena);

        // release usage and active accounting
        self.allocated_count = self.allocated_count.saturating_sub(1);
        self.allocated_bytes = self.allocated_bytes.saturating_sub(region_bytes as u64);
        self.recompute_retained_bytes();

        // publish the reusable stable region id
        self.free_ids.push(region_id);

        true
    }

    /// Write one shared-memory byte by slot offset.
    pub fn set_byte(
        &mut self,
        region: SharedPointer,
        index: usize,
        byte: u8,
        budget: &mut SharedBudget,
    ) -> bool {
        let Some(active_reservation) = self.write_active_reservation(region, index, 1) else {
            return false;
        };
        if budget.check_active_reservation(active_reservation).is_err() {
            return false;
        }

        if region.id() == 0 {
            return false;
        }

        let Some(region) = self.regions.get_mut((region.id() - 1) as usize) else {
            return false;
        };

        if !region.is_allocated() {
            return false;
        }

        let updated = region.set(&mut self.page_arena, index, byte);

        if updated {
            self.recompute_retained_bytes();
            budget.refresh(self.active_bytes());
        }

        updated
    }

    /// Replace the entire shared-memory payload with exact retained-byte admission.
    pub fn replace_bytes(
        &mut self,
        region: SharedPointer,
        bytes: &[u8],
        budget: &mut SharedBudget,
    ) -> Result<bool, SharedLimitError> {
        // reserve active bytes before mutation
        let Some(active_reservation) = self.replace_bytes_active_reservation(region, bytes.len())
        else {
            return Ok(false);
        };

        budget.check_active_reservation(active_reservation)?;

        // commit replacement and accounting
        let replaced = self.commit_replace_bytes(region, bytes);

        if replaced {
            self.recompute_retained_bytes();
            budget.refresh(self.active_bytes());
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

    /// Return the exact active shared-memory bytes.
    pub fn active_bytes(&self) -> u64 {
        self.retained_bytes
    }

    /// Return the exact mapped shared page-arena bytes.
    pub fn mapped_bytes(&self) -> u64 {
        self.page_arena.mapped_bytes() as u64
    }

    /// Return the exact borrowed shared image bytes.
    pub fn borrowed_bytes(&self) -> u64 {
        self.regions
            .iter()
            .map(SharedRegion::borrowed_bytes)
            .sum::<usize>() as u64
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

    /// Return the exact active-byte reservation for allocating one shared-memory region.
    fn allocate_active_reservation(&self, len: usize) -> i64 {
        let page_count = len.div_ceil(self.page_bytes.max(1)).max((len > 0) as usize);
        let page_arena_reservation = self
            .page_arena
            .allocate_pages_active_reservation(page_count);
        let region_bytes = self.region_payload_active_bytes(len) as i64;
        if self.free_ids.is_empty() {
            page_arena_reservation + region_bytes + size_of::<SharedRegion>() as i64
        } else {
            page_arena_reservation + region_bytes - size_of::<u64>() as i64
        }
    }

    /// Return the exact active-byte reservation for writing one shared-memory window.
    fn write_active_reservation(
        &self,
        region: SharedPointer,
        start: usize,
        len: usize,
    ) -> Option<i64> {
        let region = self.region(region)?;
        let (page_count, payload_reservation) = region.write_active_reservation(start, len);

        Some(
            self.page_arena
                .allocate_pages_active_reservation(page_count)
                + payload_reservation,
        )
    }

    /// Return the exact active-byte reservation for replacing one shared-memory payload.
    fn replace_bytes_active_reservation(
        &self,
        region: SharedPointer,
        new_len: usize,
    ) -> Option<i64> {
        let old_region = self.region(region)?;
        let (freed_pages, allocated_pages, payload_reservation) =
            old_region.replace_active_reservation(new_len);

        Some(
            self.page_arena
                .replace_pages_active_reservation(freed_pages, allocated_pages)
                + payload_reservation,
        )
    }

    /// Return the active payload bytes for one region length.
    pub(crate) fn region_payload_active_bytes(&self, len: usize) -> usize {
        SharedRegion::active_bytes_for_len(len, self.page_bytes)
    }

    /// Recompute exact retained bytes from live state.
    pub(crate) fn recompute_retained_bytes(&mut self) {
        self.retained_bytes = self.exact_retained_bytes();
        self.debug_assert_retained_bytes();
    }

    /// Return the exact retained bytes implied by the current live state.
    fn exact_retained_bytes(&self) -> u64 {
        let mut retained_bytes = 0usize;

        // live region descriptors
        retained_bytes += self.regions.capacity() * size_of::<SharedRegion>();

        // live region payload
        for region in &self.regions {
            retained_bytes += region.active_bytes();
        }

        // allocator state
        retained_bytes += self.free_ids.capacity() * size_of::<u64>();
        retained_bytes += self.page_arena.retained_bytes();

        retained_bytes as u64
    }

    /// Assert that the retained-byte cache matches exact state.
    fn debug_assert_retained_bytes(&self) {
        #[cfg(debug_assertions)]
        {
            let retained_bytes = self.exact_retained_bytes();
            debug_assert_eq!(
                self.retained_bytes, retained_bytes,
                "shared-space retained bytes must match exact recomputation"
            );
        }
    }
}
