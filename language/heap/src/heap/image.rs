use std::collections::HashSet;
use std::mem::size_of;
use std::rc::Rc;
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::managed::{GcState, ManagedImage, ManagedSpaceSnapshot, ManagedSpanImage};
use crate::raw::{RawImage, RawSpaceSnapshot, RawSpanImage};
/// Approximate control-block bytes for one rc allocation.
const RC_CONTROL_BLOCK_BYTES: usize = size_of::<usize>() * 2;

/// Approximate control-block bytes for one arc allocation.
const ARC_CONTROL_BLOCK_BYTES: usize = size_of::<usize>() * 2;

/// Deduplicated retained bytes for shared immutable heap-image storage.
#[derive(Debug, Default)]
pub struct ImageAccounting {
    /// The unique `Rc<[u8]>` leaves accounted so far.
    rc_bytes: HashSet<(*const u8, usize)>,
    /// The unique `Arc<[Rc<[u8]>]>` page tables accounted so far.
    arc_page_tables: HashSet<(*const Rc<[u8]>, usize)>,
    /// The unique `Arc<[u16]>` slices accounted so far.
    arc_u16_slices: HashSet<(*const u16, usize)>,
    /// The unique `Arc<[u32]>` slices accounted so far.
    arc_u32_slices: HashSet<(*const u32, usize)>,
    /// The unique `Arc<[u64]>` slices accounted so far.
    arc_u64_slices: HashSet<(*const u64, usize)>,
    /// The unique `Arc<[StoredLayoutId]>` slices accounted so far.
    arc_layout_slices: HashSet<(*const crate::managed::StoredLayoutId, usize)>,
    /// The unique `Arc<[ManagedHandleEntry]>` slices accounted so far.
    arc_managed_handles: HashSet<(*const crate::managed::ManagedHandleEntry, usize)>,
    /// The unique `Arc<[RawHandleEntry]>` slices accounted so far.
    arc_raw_handles: HashSet<(*const crate::raw::RawHandleEntry, usize)>,
}

impl ImageAccounting {
    /// Create one empty heap-image accounting state.
    pub fn new() -> Self {
        Self::default()
    }

    /// Account one shared immutable byte leaf once.
    pub(crate) fn account_rc_bytes(&mut self, bytes: &Rc<[u8]>) -> usize {
        let len = bytes.len();
        if len == 0 {
            return 0;
        }

        let key = (bytes.as_ptr(), len);
        if self.rc_bytes.insert(key) {
            RC_CONTROL_BLOCK_BYTES + len
        } else {
            0
        }
    }

    /// Account one shared page-table allocation once.
    pub(crate) fn account_arc_page_table(&mut self, pages: &Arc<[Rc<[u8]>]>) -> usize {
        let len = pages.len();
        if len == 0 {
            return 0;
        }

        let key = (pages.as_ptr(), len);
        if self.arc_page_tables.insert(key) {
            ARC_CONTROL_BLOCK_BYTES + len * size_of::<Rc<[u8]>>()
        } else {
            0
        }
    }

    /// Account one shared `u16` slice allocation once.
    pub(crate) fn account_arc_u16_slice(&mut self, values: &Arc<[u16]>) -> usize {
        let len = values.len();
        if len == 0 {
            return 0;
        }

        let key = (values.as_ptr(), len);
        if self.arc_u16_slices.insert(key) {
            ARC_CONTROL_BLOCK_BYTES + len * size_of::<u16>()
        } else {
            0
        }
    }

    /// Account one shared `u32` slice allocation once.
    pub(crate) fn account_arc_u32_slice(&mut self, values: &Arc<[u32]>) -> usize {
        let len = values.len();
        if len == 0 {
            return 0;
        }

        let key = (values.as_ptr(), len);
        if self.arc_u32_slices.insert(key) {
            ARC_CONTROL_BLOCK_BYTES + len * size_of::<u32>()
        } else {
            0
        }
    }

    /// Account one shared `u64` slice allocation once.
    pub(crate) fn account_arc_u64_slice(&mut self, values: &Arc<[u64]>) -> usize {
        let len = values.len();
        if len == 0 {
            return 0;
        }

        let key = (values.as_ptr(), len);
        if self.arc_u64_slices.insert(key) {
            ARC_CONTROL_BLOCK_BYTES + len * size_of::<u64>()
        } else {
            0
        }
    }

    /// Account one shared managed layout slice allocation once.
    pub(crate) fn account_arc_layout_slice(
        &mut self,
        values: &Arc<[crate::managed::StoredLayoutId]>,
    ) -> usize {
        let len = values.len();
        if len == 0 {
            return 0;
        }

        let key = (values.as_ptr(), len);
        if self.arc_layout_slices.insert(key) {
            ARC_CONTROL_BLOCK_BYTES + len * size_of::<crate::managed::StoredLayoutId>()
        } else {
            0
        }
    }

    /// Account one shared managed-handle table allocation once.
    pub(crate) fn account_managed_handles(
        &mut self,
        handles: &Arc<[crate::managed::ManagedHandleEntry]>,
    ) -> usize {
        let len = handles.len();
        if len == 0 {
            return 0;
        }

        let key = (handles.as_ptr(), len);
        if self.arc_managed_handles.insert(key) {
            ARC_CONTROL_BLOCK_BYTES + len * size_of::<crate::managed::ManagedHandleEntry>()
        } else {
            0
        }
    }

    /// Account one shared raw-handle table allocation once.
    pub(crate) fn account_raw_handles(
        &mut self,
        handles: &Arc<[crate::raw::RawHandleEntry]>,
    ) -> usize {
        let len = handles.len();
        if len == 0 {
            return 0;
        }

        let key = (handles.as_ptr(), len);
        if self.arc_raw_handles.insert(key) {
            ARC_CONTROL_BLOCK_BYTES + len * size_of::<crate::raw::RawHandleEntry>()
        } else {
            0
        }
    }
}

/// Immutable in-memory heap image.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HeapImage {
    /// Captured managed-space state.
    pub(crate) managed: ManagedImage,
    /// Captured raw-space state.
    pub(crate) raw: RawImage,
}

impl HeapImage {
    /// Build one heap image from one serialized snapshot.
    pub(crate) fn from_snapshot(snapshot: &HeapSnapshot) -> Self {
        Self {
            managed: ManagedImage::from_snapshot(&snapshot.managed),
            raw: RawImage::from_snapshot(&snapshot.raw),
        }
    }

    /// Flatten one heap image into one serialized snapshot.
    pub(crate) fn snapshot(&self) -> HeapSnapshot {
        HeapSnapshot {
            managed: self.managed.snapshot(),
            raw: self.raw.snapshot(),
        }
    }

    /// Return one managed span image by index.
    pub fn managed_span(&self, index: usize) -> Option<&ManagedSpanImage> {
        self.managed.spans.get(index)
    }

    /// Return one raw span image by index.
    pub fn raw_span(&self, index: usize) -> Option<&RawSpanImage> {
        self.raw.spans.get(index)
    }

    /// Return whether one managed span leaf shares captured storage at the given index.
    pub fn managed_span_shares_with(&self, other: &Self, index: usize) -> bool {
        match (self.managed_span(index), other.managed_span(index)) {
            (Some(left), Some(right)) => left.shares_storage_with(right),
            _ => false,
        }
    }

    /// Return whether one raw span leaf shares captured storage at the given index.
    pub fn raw_span_shares_with(&self, other: &Self, index: usize) -> bool {
        match (self.raw_span(index), other.raw_span(index)) {
            (Some(left), Some(right)) => left.shares_storage_with(right),
            _ => false,
        }
    }

    /// Return the number of immutable managed leaves in this image.
    pub fn managed_leaf_count(&self) -> usize {
        self.managed.spans.len() + self.managed.large_allocations.len()
    }

    /// Return the number of immutable raw leaves in this image.
    pub fn raw_leaf_count(&self) -> usize {
        self.raw.spans.len() + self.raw.large_allocations.len()
    }

    /// Return the total captured local allocated bytes in this heap image.
    pub fn local_allocated_bytes(&self) -> u64 {
        self.managed
            .allocated_bytes
            .saturating_add(self.raw.allocated_bytes)
    }

    /// Return the total number of immutable leaf images in this heap image.
    pub fn leaf_count(&self) -> usize {
        self.managed_leaf_count() + self.raw_leaf_count()
    }

    /// Return the captured managed GC state.
    pub fn managed_gc_state(&self) -> &GcState {
        &self.managed.gc_state
    }

    /// Return the exact owned bytes for this durable heap image.
    pub fn image_bytes(&self) -> usize {
        size_of::<Self>() + self.managed.image_bytes() + self.raw.image_bytes()
    }

    /// Account one heap image into deduplicated retained-image bytes.
    pub fn retained_image_bytes(&self, accounting: &mut ImageAccounting) -> usize {
        size_of::<Self>()
            + self.managed.retained_image_bytes(accounting)
            + self.raw.retained_image_bytes(accounting)
    }
}

/// Serialized heap snapshot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HeapSnapshot {
    /// Flattened managed-space image.
    pub managed: ManagedSpaceSnapshot,
    /// Flattened raw-space image.
    pub raw: RawSpaceSnapshot,
}
