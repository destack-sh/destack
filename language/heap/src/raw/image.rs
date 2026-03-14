use std::sync::Arc;

use serde::{Deserialize, Serialize};

use super::{RawExtentImage, RawLocation, RawRunImage, RawSpace};
use crate::heap::{SizeClassTable, TreeVector};

/// One immutable raw-space image.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RawImage {
    /// The configured size-class table.
    pub(crate) size_classes: SizeClassTable,
    /// The configured run width.
    pub(crate) run_bytes: usize,
    /// The configured extent chunk width.
    pub(crate) chunk_bytes: usize,
    /// The captured raw runs.
    pub(crate) runs: TreeVector<RawRunImage>,
    /// The captured raw extents.
    pub(crate) extents: TreeVector<RawExtentImage>,
    /// Stable raw locations keyed by allocation id minus one.
    pub(crate) locations: Arc<[RawLocation]>,
    /// The captured free raw ids.
    pub(crate) free_ids: Arc<[u64]>,
    /// The captured free raw extent ids.
    pub(crate) free_extent_ids: Arc<[u64]>,
    /// The next raw allocation id to allocate.
    pub(crate) next_unused_id: u64,
    /// The next raw extent id to allocate.
    pub(crate) next_unused_extent_id: u64,
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
    /// The configured run width.
    pub run_bytes: usize,
    /// The configured extent chunk width.
    pub chunk_bytes: usize,
    /// The flattened raw runs.
    pub runs: Vec<RawRunImage>,
    /// The flattened raw extents.
    pub extents: Vec<RawExtentImage>,
    /// Stable raw locations keyed by allocation id minus one.
    pub locations: Vec<RawLocation>,
    /// The flattened free raw ids.
    pub free_ids: Vec<u64>,
    /// The flattened free raw extent ids.
    pub free_extent_ids: Vec<u64>,
    /// The next raw allocation id to allocate.
    pub next_unused_id: u64,
    /// The next raw extent id to allocate.
    pub next_unused_extent_id: u64,
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
            run_bytes: snapshot.run_bytes,
            chunk_bytes: snapshot.chunk_bytes,
            runs: TreeVector::from_values_by(
                &snapshot.runs,
                None,
                RawRunImage::shares_storage_with,
            ),
            extents: TreeVector::from_values_by(
                &snapshot.extents,
                None,
                RawExtentImage::shares_storage_with,
            ),
            locations: Arc::from(snapshot.locations.as_slice()),
            free_ids: Arc::from(snapshot.free_ids.as_slice()),
            free_extent_ids: Arc::from(snapshot.free_extent_ids.as_slice()),
            next_unused_id: snapshot.next_unused_id,
            next_unused_extent_id: snapshot.next_unused_extent_id,
            allocated_count: snapshot.allocated_count,
            allocated_bytes: snapshot.allocated_bytes,
        }
    }

    /// Flatten one raw-space image into one serialized snapshot.
    pub(crate) fn snapshot(&self) -> RawSpaceSnapshot {
        RawSpaceSnapshot {
            size_classes: self.size_classes.clone(),
            run_bytes: self.run_bytes,
            chunk_bytes: self.chunk_bytes,
            runs: self.runs.iter().cloned().collect(),
            extents: self.extents.iter().cloned().collect(),
            locations: self.locations.iter().copied().collect(),
            free_ids: self.free_ids.iter().copied().collect(),
            free_extent_ids: self.free_extent_ids.iter().copied().collect(),
            next_unused_id: self.next_unused_id,
            next_unused_extent_id: self.next_unused_extent_id,
            allocated_count: self.allocated_count,
            allocated_bytes: self.allocated_bytes,
        }
    }
}

impl RawSpace {
    /// Restore one raw space from one serialized snapshot.
    pub fn from_snapshot(snapshot: &RawSpaceSnapshot) -> Self {
        Self::from_image(&RawImage::from_snapshot(snapshot))
    }
}
