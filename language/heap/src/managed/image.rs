use std::sync::Arc;

use serde::{Deserialize, Serialize};

use super::{
    GcState, ManagedHandleEntry, ManagedLargeAllocationImage, ManagedSpace, ManagedSpanImage,
    ReferenceMap,
};
use crate::alloc::SizeClassTable;
use crate::heap::validate_managed_reference_bytes;

/// One immutable managed-space image.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManagedImage {
    /// The configured size-class table.
    pub(crate) size_classes: SizeClassTable,
    /// The encoded byte width for managed references inside traced payloads.
    pub(crate) managed_reference_bytes: u8,
    /// The configured young-space byte width.
    pub(crate) young_bytes: usize,
    /// The configured small-space span width.
    pub(crate) small_bytes: usize,
    /// The configured local page width.
    pub(crate) page_bytes: usize,
    /// The captured managed spans.
    pub(crate) spans: Vec<ManagedSpanImage>,
    /// The captured managed large allocations.
    pub(crate) large_allocations: Vec<ManagedLargeAllocationImage>,
    /// Dense managed handle metadata keyed by reference id minus one.
    pub(crate) handles: Arc<[ManagedHandleEntry]>,
    /// The free managed handle id at the head of the intrusive free list.
    pub(crate) free_handle_head: u64,
    /// The captured free large-allocation ids.
    pub(crate) free_large_allocation_ids: Arc<[u64]>,
    /// The next managed reference id to allocate.
    pub(crate) next_unused_id: u64,
    /// The next managed large-allocation id to allocate.
    pub(crate) next_unused_large_allocation_id: u64,
    /// The number of allocated managed references.
    pub(crate) allocated_count: usize,
    /// The number of allocated managed bytes.
    pub(crate) allocated_bytes: u64,
    /// The captured reference maps.
    pub(crate) reference_maps: Vec<ReferenceMap>,
    /// The captured GC state.
    pub(crate) gc_state: GcState,
}

/// One serialized managed-space snapshot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManagedSpaceSnapshot {
    /// The configured size-class table.
    pub size_classes: SizeClassTable,
    /// The encoded byte width for managed references inside traced payloads.
    pub managed_reference_bytes: u8,
    /// The configured young-space byte width.
    pub young_bytes: usize,
    /// The configured small-space span width.
    pub small_bytes: usize,
    /// The configured local page width.
    pub page_bytes: usize,
    /// The flattened managed spans.
    pub spans: Vec<ManagedSpanImage>,
    /// The flattened managed large allocations.
    pub large_allocations: Vec<ManagedLargeAllocationImage>,
    /// Dense managed handle metadata keyed by reference id minus one.
    pub(crate) handles: Vec<ManagedHandleEntry>,
    /// The free managed handle id at the head of the intrusive free list.
    pub free_handle_head: u64,
    /// The flattened free large-allocation ids.
    pub free_large_allocation_ids: Vec<u64>,
    /// The next managed reference id to allocate.
    pub next_unused_id: u64,
    /// The next managed large-allocation id to allocate.
    pub next_unused_large_allocation_id: u64,
    /// The number of allocated managed references.
    pub allocated_count: usize,
    /// The number of allocated managed bytes.
    pub allocated_bytes: u64,
    /// The flattened reference maps.
    pub reference_maps: Vec<ReferenceMap>,
    /// The captured GC state.
    pub gc_state: GcState,
}

impl ManagedImage {
    /// Build one managed-space image from one serialized snapshot.
    pub(crate) fn from_snapshot(snapshot: &ManagedSpaceSnapshot) -> Self {
        validate_managed_reference_bytes(snapshot.managed_reference_bytes);

        Self {
            size_classes: snapshot.size_classes.clone(),
            managed_reference_bytes: snapshot.managed_reference_bytes,
            young_bytes: snapshot.young_bytes,
            small_bytes: snapshot.small_bytes,
            page_bytes: snapshot.page_bytes,
            spans: snapshot.spans.clone(),
            large_allocations: snapshot.large_allocations.clone(),
            handles: Arc::from(snapshot.handles.as_slice()),
            free_handle_head: snapshot.free_handle_head,
            free_large_allocation_ids: Arc::from(snapshot.free_large_allocation_ids.as_slice()),
            next_unused_id: snapshot.next_unused_id,
            next_unused_large_allocation_id: snapshot.next_unused_large_allocation_id,
            allocated_count: snapshot.allocated_count,
            allocated_bytes: snapshot.allocated_bytes,
            reference_maps: snapshot.reference_maps.clone(),
            gc_state: snapshot.gc_state.clone(),
        }
    }

    /// Flatten one managed-space image into one serialized snapshot.
    pub(crate) fn snapshot(&self) -> ManagedSpaceSnapshot {
        ManagedSpaceSnapshot {
            size_classes: self.size_classes.clone(),
            managed_reference_bytes: self.managed_reference_bytes,
            young_bytes: self.young_bytes,
            small_bytes: self.small_bytes,
            page_bytes: self.page_bytes,
            spans: self.spans.to_vec(),
            large_allocations: self.large_allocations.to_vec(),
            handles: self.handles.iter().copied().collect(),
            free_handle_head: self.free_handle_head,
            free_large_allocation_ids: self.free_large_allocation_ids.iter().copied().collect(),
            next_unused_id: self.next_unused_id,
            next_unused_large_allocation_id: self.next_unused_large_allocation_id,
            allocated_count: self.allocated_count,
            allocated_bytes: self.allocated_bytes,
            reference_maps: self.reference_maps.clone(),
            gc_state: self.gc_state.clone(),
        }
    }
}

impl ManagedSpace {
    /// Restore one managed space from one serialized snapshot.
    pub fn from_snapshot(snapshot: &ManagedSpaceSnapshot) -> Self {
        Self::from_image(&ManagedImage::from_snapshot(snapshot))
    }
}
