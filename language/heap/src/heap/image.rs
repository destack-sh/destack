use serde::{Deserialize, Serialize};

use crate::managed::{GcState, ManagedImage, ManagedSpaceSnapshot, ManagedSpanImage};
use crate::raw::{RawImage, RawSpaceSnapshot, RawSpanImage};

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

    /// Return the total captured local allocation bytes in this heap image.
    pub fn local_allocation_bytes(&self) -> u64 {
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
}

/// Serialized heap snapshot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HeapSnapshot {
    /// Flattened managed-space image.
    pub managed: ManagedSpaceSnapshot,
    /// Flattened raw-space image.
    pub raw: RawSpaceSnapshot,
}
