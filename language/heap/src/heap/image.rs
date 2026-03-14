use serde::{Deserialize, Serialize};

use crate::managed::{GcState, ManagedImage, ManagedRunImage, ManagedSpaceSnapshot};
use crate::raw::{RawImage, RawRunImage, RawSpaceSnapshot};

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

    /// Return one managed run image by index.
    pub fn managed_run(&self, index: usize) -> Option<&ManagedRunImage> {
        self.managed.runs.get(index)
    }

    /// Return one raw run image by index.
    pub fn raw_run(&self, index: usize) -> Option<&RawRunImage> {
        self.raw.runs.get(index)
    }

    /// Return whether one managed run leaf shares captured storage at the given index.
    pub fn managed_run_shares_with(&self, other: &Self, index: usize) -> bool {
        match (self.managed_run(index), other.managed_run(index)) {
            (Some(left), Some(right)) => left.shares_storage_with(right),
            _ => false,
        }
    }

    /// Return whether one raw run leaf shares captured storage at the given index.
    pub fn raw_run_shares_with(&self, other: &Self, index: usize) -> bool {
        match (self.raw_run(index), other.raw_run(index)) {
            (Some(left), Some(right)) => left.shares_storage_with(right),
            _ => false,
        }
    }

    /// Return the number of immutable managed leaves in this image.
    pub fn managed_leaf_count(&self) -> usize {
        self.managed.runs.len() + self.managed.extents.len()
    }

    /// Return the number of immutable raw leaves in this image.
    pub fn raw_leaf_count(&self) -> usize {
        self.raw.runs.len() + self.raw.extents.len()
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
