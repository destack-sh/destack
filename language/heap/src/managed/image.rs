use std::sync::Arc;

use serde::{Deserialize, Serialize};

use super::{
    GcState, ManagedExtentImage, ManagedLocation, ManagedRunImage, ManagedSpace, ReferenceMap,
};
use crate::heap::{SizeClassTable, TreeVector};

/// One immutable managed-space image.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManagedImage {
    /// The configured size-class table.
    pub(crate) size_classes: SizeClassTable,
    /// The configured run width.
    pub(crate) run_bytes: usize,
    /// The configured extent chunk width.
    pub(crate) chunk_bytes: usize,
    /// The captured managed runs.
    pub(crate) runs: TreeVector<ManagedRunImage>,
    /// The captured managed extents.
    pub(crate) extents: TreeVector<ManagedExtentImage>,
    /// Stable allocation locations keyed by reference id minus one.
    pub(crate) locations: Arc<[ManagedLocation]>,
    /// The captured free managed reference ids.
    pub(crate) free_ids: Arc<[u64]>,
    /// The captured free extent ids.
    pub(crate) free_extent_ids: Arc<[u64]>,
    /// The next managed reference id to allocate.
    pub(crate) next_unused_id: u64,
    /// The next managed extent id to allocate.
    pub(crate) next_unused_extent_id: u64,
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
    /// The configured run width.
    pub run_bytes: usize,
    /// The configured extent chunk width.
    pub chunk_bytes: usize,
    /// The flattened managed runs.
    pub runs: Vec<ManagedRunImage>,
    /// The flattened managed extents.
    pub extents: Vec<ManagedExtentImage>,
    /// Stable allocation locations keyed by reference id minus one.
    pub(crate) locations: Vec<ManagedLocation>,
    /// The flattened free managed reference ids.
    pub free_ids: Vec<u64>,
    /// The flattened free extent ids.
    pub free_extent_ids: Vec<u64>,
    /// The next managed reference id to allocate.
    pub next_unused_id: u64,
    /// The next managed extent id to allocate.
    pub next_unused_extent_id: u64,
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
        Self {
            size_classes: snapshot.size_classes.clone(),
            run_bytes: snapshot.run_bytes,
            chunk_bytes: snapshot.chunk_bytes,
            runs: TreeVector::from_values_by(
                &snapshot.runs,
                None,
                ManagedRunImage::shares_storage_with,
            ),
            extents: TreeVector::from_values_by(
                &snapshot.extents,
                None,
                ManagedExtentImage::shares_storage_with,
            ),
            locations: Arc::from(snapshot.locations.as_slice()),
            free_ids: Arc::from(snapshot.free_ids.as_slice()),
            free_extent_ids: Arc::from(snapshot.free_extent_ids.as_slice()),
            next_unused_id: snapshot.next_unused_id,
            next_unused_extent_id: snapshot.next_unused_extent_id,
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
