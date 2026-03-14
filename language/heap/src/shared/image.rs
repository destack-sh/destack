use std::sync::Arc;

use serde::{Deserialize, Serialize};

use super::super::{SharedSpace, TreeVector};
use super::region::SharedRegion;

/// One serialized shared-memory region snapshot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SharedRegionSnapshot {
    /// Whether this region id is currently allocated.
    pub is_allocated: bool,
    /// The flattened shared-memory bytes for this region id.
    pub bytes: Vec<u8>,
}

/// Serialized shared-memory snapshot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SharedSpaceSnapshot {
    /// Flattened shared-memory regions keyed by region id minus one.
    pub regions: Vec<SharedRegionSnapshot>,
    /// The next shared-memory region id to allocate.
    pub next_unused_id: u64,
    /// Flattened free shared-memory region ids.
    pub free_ids: Vec<u64>,
    /// The number of allocated shared-memory regions.
    pub allocated_count: usize,
    /// The number of allocated shared-memory bytes.
    pub allocated_bytes: u64,
    /// The configured shared chunk width.
    pub chunk_bytes: usize,
}

/// One immutable shared-memory region image.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SharedRegionImage {
    /// Whether this region id is currently allocated.
    pub is_allocated: bool,
    /// The logical byte length of this region.
    pub len: usize,
    /// The chunk width used by this region.
    pub chunk_bytes: usize,
    /// The immutable chunk leaves captured for this region id.
    pub(crate) chunks: TreeVector<Arc<[u8]>>,
}

impl SharedRegionImage {
    /// Report whether this region image shares immutable chunk storage with another image.
    pub fn shares_storage_with(&self, other: &Self) -> bool {
        self.is_allocated == other.is_allocated
            && self.len == other.len
            && self.chunk_bytes == other.chunk_bytes
            && self.chunks.len() == other.chunks.len()
            && self
                .chunks
                .iter()
                .zip(other.chunks.iter())
                .all(|(left, right)| Arc::ptr_eq(left, right))
    }

    /// Flatten this region image into one contiguous byte vector.
    pub fn to_vec(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(self.len);

        for chunk in self.chunks.iter() {
            bytes.extend_from_slice(chunk.as_ref());
        }

        bytes.truncate(self.len);
        bytes
    }
}

/// Immutable shared-memory image.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SharedImage {
    /// Captured shared-memory regions keyed by region id minus one.
    regions: TreeVector<SharedRegionImage>,
    /// The next shared-memory region id to allocate.
    next_unused_id: u64,
    /// The captured free shared-memory region ids.
    free_ids: Arc<[u64]>,
    /// The number of allocated shared-memory regions.
    allocated_count: usize,
    /// The number of allocated shared-memory bytes.
    allocated_bytes: u64,
    /// The configured shared chunk width.
    chunk_bytes: usize,
}

impl SharedImage {
    /// Return one shared region image by index.
    pub fn region(&self, index: usize) -> Option<&SharedRegionImage> {
        self.regions.get(index)
    }

    /// Build one shared-memory image from one shared-memory snapshot.
    pub fn from_snapshot(snapshot: &SharedSpaceSnapshot) -> Self {
        let regions = snapshot
            .regions
            .iter()
            .map(|region| SharedRegionImage {
                is_allocated: region.is_allocated,
                len: region.bytes.len(),
                chunk_bytes: snapshot.chunk_bytes,
                chunks: TreeVector::from_shared(
                    &region
                        .bytes
                        .chunks(snapshot.chunk_bytes)
                        .map(|chunk| Arc::from(chunk.to_vec().into_boxed_slice()))
                        .collect::<Vec<_>>(),
                    None,
                ),
            })
            .collect::<Vec<_>>();

        Self {
            regions: TreeVector::from_values_by(
                &regions,
                None,
                SharedRegionImage::shares_storage_with,
            ),
            next_unused_id: snapshot.next_unused_id,
            free_ids: Arc::from(snapshot.free_ids.as_slice()),
            allocated_count: snapshot.allocated_count,
            allocated_bytes: snapshot.allocated_bytes,
            chunk_bytes: snapshot.chunk_bytes,
        }
    }

    /// Flatten one shared-memory image into one shared-memory snapshot.
    pub fn snapshot(&self) -> SharedSpaceSnapshot {
        SharedSpaceSnapshot {
            regions: self
                .regions
                .iter()
                .map(|region| SharedRegionSnapshot {
                    is_allocated: region.is_allocated,
                    bytes: region.to_vec(),
                })
                .collect(),
            next_unused_id: self.next_unused_id,
            free_ids: self.free_ids.iter().copied().collect(),
            allocated_count: self.allocated_count,
            allocated_bytes: self.allocated_bytes,
            chunk_bytes: self.chunk_bytes,
        }
    }
}

impl SharedSpace {
    /// Create one shared-memory space directly from one shared-memory image.
    pub fn from_image(image: &SharedImage) -> Self {
        let regions = image
            .regions
            .iter()
            .map(SharedRegion::from_image)
            .collect::<Vec<_>>();
        let free_ids = image.free_ids.iter().copied().collect();

        let mut space = Self {
            chunk_bytes: image.chunk_bytes,
            regions,
            free_ids,
            next_unused_id: image.next_unused_id,
            allocated_count: image.allocated_count,
            allocated_bytes: image.allocated_bytes,
            retained_bytes: 0,
        };

        // exact retained bytes
        space.recompute_retained_bytes();

        space
    }

    /// Capture one immutable shared-memory section.
    pub fn image(&mut self, base: Option<&SharedImage>) -> SharedImage {
        let regions = self
            .regions
            .iter_mut()
            .enumerate()
            .map(|(index, region)| region.image(base.and_then(|image| image.regions.get(index))))
            .collect::<Vec<_>>();
        let regions = TreeVector::from_values_by(
            &regions,
            base.map(|image| &image.regions),
            SharedRegionImage::shares_storage_with,
        );
        let free_ids = Arc::from(self.free_ids.as_slice());

        SharedImage {
            regions,
            next_unused_id: self.next_unused_id,
            free_ids,
            allocated_count: self.allocated_count,
            allocated_bytes: self.allocated_bytes,
            chunk_bytes: self.chunk_bytes,
        }
    }
}
