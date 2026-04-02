use std::mem::size_of;
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use super::{RawHandleEntry, RawLargeAllocationImage, RawSpace, RawSpanImage};
use crate::alloc::SizeClassTable;
use crate::heap::ImageAccounting;

/// Approximate control-block bytes for one arc allocation.
const ARC_CONTROL_BLOCK_BYTES: usize = size_of::<usize>() * 2;

/// One immutable raw-space image.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RawImage {
    /// The configured size-class table.
    pub(crate) size_classes: SizeClassTable,
    /// The configured small-space span width.
    pub(crate) small_bytes: usize,
    /// The configured local page width.
    pub(crate) page_bytes: usize,
    /// The captured raw spans.
    pub(crate) spans: Vec<RawSpanImage>,
    /// The captured raw large allocations.
    pub(crate) large_allocations: Vec<RawLargeAllocationImage>,
    /// Dense raw handle metadata keyed by allocation id minus one.
    pub(crate) handles: Arc<[RawHandleEntry]>,
    /// The free raw allocation id at the head of the intrusive free list.
    pub(crate) free_handle_head: u64,
    /// The captured free raw large-allocation ids.
    pub(crate) free_large_allocation_ids: Arc<[u64]>,
    /// The next raw allocation id to allocate.
    pub(crate) next_unused_id: u64,
    /// The next raw large-allocation id to allocate.
    pub(crate) next_unused_large_allocation_id: u64,
    /// The number of live raw allocations.
    pub(crate) allocated_count: usize,
    /// The number of live raw bytes.
    pub(crate) allocated_bytes: u64,
}

/// One serialized raw-space snapshot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RawSpaceSnapshot {
    /// The configured size-class table.
    pub size_classes: SizeClassTable,
    /// The configured small-space span width.
    pub small_bytes: usize,
    /// The configured local page width.
    pub page_bytes: usize,
    /// The flattened raw spans.
    pub spans: Vec<RawSpanImage>,
    /// The flattened raw large allocations.
    pub large_allocations: Vec<RawLargeAllocationImage>,
    /// Dense raw handle metadata keyed by allocation id minus one.
    pub handles: Vec<RawHandleEntry>,
    /// The free raw allocation id at the head of the intrusive free list.
    pub free_handle_head: u64,
    /// The flattened free raw large-allocation ids.
    pub free_large_allocation_ids: Vec<u64>,
    /// The next raw allocation id to allocate.
    pub next_unused_id: u64,
    /// The next raw large-allocation id to allocate.
    pub next_unused_large_allocation_id: u64,
    /// The number of live raw allocations.
    pub allocated_count: usize,
    /// The number of live raw bytes.
    pub allocated_bytes: u64,
}

impl RawImage {
    /// Build one raw-space image from one serialized snapshot.
    pub(crate) fn from_snapshot(snapshot: &RawSpaceSnapshot) -> Self {
        Self {
            size_classes: snapshot.size_classes.clone(),
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
        }
    }

    /// Flatten one raw-space image into one serialized snapshot.
    pub(crate) fn snapshot(&self) -> RawSpaceSnapshot {
        RawSpaceSnapshot {
            size_classes: self.size_classes.clone(),
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
        }
    }

    /// Return the exact owned bytes for this durable raw-space image.
    pub fn image_bytes(&self) -> usize {
        let mut image_bytes = size_of::<Self>();
        image_bytes += self.spans.capacity() * size_of::<RawSpanImage>();
        image_bytes += self.large_allocations.capacity() * size_of::<RawLargeAllocationImage>();
        image_bytes += ARC_CONTROL_BLOCK_BYTES + self.handles.len() * size_of::<RawHandleEntry>();
        image_bytes +=
            ARC_CONTROL_BLOCK_BYTES + self.free_large_allocation_ids.len() * size_of::<u64>();
        image_bytes += self.size_classes.retained_bytes();

        for span in &self.spans {
            image_bytes += span.image_bytes();
        }

        for large_allocation in &self.large_allocations {
            image_bytes += large_allocation.image_bytes();
        }

        image_bytes
    }

    /// Account this raw image into deduplicated retained-image bytes.
    pub fn retained_image_bytes(&self, accounting: &mut ImageAccounting) -> usize {
        let mut image_bytes = size_of::<Self>();
        image_bytes += self.spans.capacity() * size_of::<RawSpanImage>();
        image_bytes += self.large_allocations.capacity() * size_of::<RawLargeAllocationImage>();
        image_bytes += accounting.account_raw_handles(&self.handles);
        image_bytes += accounting.account_arc_u64_slice(&self.free_large_allocation_ids);
        image_bytes += self.size_classes.retained_bytes();

        for span in &self.spans {
            image_bytes += span.retained_image_bytes(accounting);
        }

        for large_allocation in &self.large_allocations {
            image_bytes += large_allocation.retained_image_bytes(accounting);
        }

        image_bytes
    }
}

impl RawSpace {
    /// Restore one raw space from one serialized snapshot.
    pub fn from_snapshot(snapshot: &RawSpaceSnapshot) -> Self {
        Self::from_image(&RawImage::from_snapshot(snapshot))
    }
}
