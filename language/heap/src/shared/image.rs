use serde::{Deserialize, Serialize};
use std::sync::Arc;

use super::SharedSpace;
use super::region::SharedRegion;
use crate::alloc::{Arena, PageId, PageMap};

/// One frozen shared-memory region root.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SharedRegionImage {
    /// Whether this region id is currently allocated.
    pub is_allocated: bool,
    /// The logical byte length of this region.
    pub len: usize,
    /// The page run for this region.
    pub pages: PageMap,
}

/// One frozen shared-memory root.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SharedSpaceImage {
    /// Captured shared-memory regions keyed by region id minus one.
    regions: Box<[SharedRegionImage]>,
    /// The next shared-memory region id to allocate.
    next_unused_id: u64,
    /// The captured free shared-memory region ids.
    free_ids: Box<[u64]>,
    /// The number of allocated shared-memory regions.
    allocated_count: usize,
    /// The number of allocated shared-memory bytes.
    allocated_bytes: u64,
    /// The configured shared page width.
    page_bytes: usize,
}

impl SharedSpaceImage {
    /// Create one frozen shared-memory root.
    pub fn new(
        regions: Box<[SharedRegionImage]>,
        next_unused_id: u64,
        free_ids: Box<[u64]>,
        allocated_count: usize,
        allocated_bytes: u64,
        page_bytes: usize,
    ) -> Self {
        Self {
            regions,
            next_unused_id,
            free_ids,
            allocated_count,
            allocated_bytes,
            page_bytes,
        }
    }

    /// Return one shared region image by index.
    pub fn region(&self, index: usize) -> Option<&SharedRegionImage> {
        self.regions.get(index)
    }

    /// Build one shared-memory image from one serialized snapshot.
    pub fn from_snapshot(snapshot: &SharedSpaceSnapshot) -> Self {
        // restore the frozen region roots first
        let regions = snapshot
            .regions
            .iter()
            .map(Self::restore_region_image)
            .collect::<Vec<_>>()
            .into_boxed_slice();

        Self {
            regions,
            next_unused_id: snapshot.next_unused_id,
            free_ids: snapshot.free_ids.clone(),
            allocated_count: snapshot.allocated_count,
            allocated_bytes: snapshot.allocated_bytes,
            page_bytes: snapshot.page_bytes,
        }
    }

    /// Flatten one shared-memory image into one shared-memory snapshot.
    pub fn snapshot(&self) -> SharedSpaceSnapshot {
        SharedSpaceSnapshot {
            regions: self
                .regions
                .iter()
                .map(Self::capture_region_snapshot)
                .collect::<Vec<_>>()
                .into_boxed_slice(),
            next_unused_id: self.next_unused_id,
            free_ids: self.free_ids.clone(),
            allocated_count: self.allocated_count,
            allocated_bytes: self.allocated_bytes,
            page_bytes: self.page_bytes,
        }
    }

    /// Return the frozen shared-memory regions.
    pub fn regions(&self) -> &[SharedRegionImage] {
        &self.regions
    }

    /// Return the next shared-memory region id.
    pub const fn next_unused_id(&self) -> u64 {
        self.next_unused_id
    }

    /// Return the frozen free region ids.
    pub fn free_ids(&self) -> &[u64] {
        &self.free_ids
    }

    /// Return the number of allocated regions.
    pub const fn allocated_count(&self) -> usize {
        self.allocated_count
    }

    /// Return the allocated shared bytes.
    pub const fn allocated_bytes(&self) -> u64 {
        self.allocated_bytes
    }

    /// Return the shared page width.
    pub const fn page_bytes(&self) -> usize {
        self.page_bytes
    }

    /// Return every arena page reachable from this shared-memory image.
    pub fn page_ids(&self) -> Vec<PageId> {
        let mut pages = Vec::new();

        // collect every frozen region page
        for region in &*self.regions {
            pages.extend(region.pages.page_ids());
        }

        pages
    }

    /// Restore one frozen shared region from one serialized snapshot.
    fn restore_region_image(region: &SharedRegionSnapshot) -> SharedRegionImage {
        SharedRegionImage {
            is_allocated: region.is_allocated,
            len: region.len,
            pages: region.pages.clone(),
        }
    }

    /// Capture one serialized shared region snapshot.
    fn capture_region_snapshot(region: &SharedRegionImage) -> SharedRegionSnapshot {
        SharedRegionSnapshot {
            is_allocated: region.is_allocated,
            len: region.len,
            pages: region.pages.clone(),
        }
    }
}

/// One serialized shared-memory region snapshot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SharedRegionSnapshot {
    /// Whether this region id is currently allocated.
    pub is_allocated: bool,
    /// The logical byte length of this region.
    pub len: usize,
    /// The page run for this region.
    pub pages: PageMap,
}

/// One serialized shared-memory snapshot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SharedSpaceSnapshot {
    /// Serialized shared-memory regions keyed by region id minus one.
    pub regions: Box<[SharedRegionSnapshot]>,
    /// The next shared-memory region id to allocate.
    pub next_unused_id: u64,
    /// Serialized free shared-memory region ids.
    pub free_ids: Box<[u64]>,
    /// The number of allocated shared-memory regions.
    pub allocated_count: usize,
    /// The number of allocated shared-memory bytes.
    pub allocated_bytes: u64,
    /// The configured shared page width.
    pub page_bytes: usize,
}

impl SharedSpace {
    /// Create one shared-memory space from one frozen shared-memory root.
    pub fn from_image(image: &SharedSpaceImage) -> Self {
        let arena = Arc::new(Arena::with_page_bytes(image.page_bytes()));

        Self::from_image_with_arena(arena, image)
    }

    /// Create one shared-memory space from one frozen shared-memory root over one shared arena.
    pub fn from_image_with_arena(arena: Arc<Arena>, image: &SharedSpaceImage) -> Self {
        // retain the shared backing first
        Self::retain_image_pages(&arena, image);

        // rebuild the live root over the retained pages
        let mut space = Self::with_arena(arena);
        space.regions = image
            .regions()
            .iter()
            .map(SharedRegion::from_image)
            .collect();
        space.next_unused_id = image.next_unused_id();
        space.free_ids = image.free_ids().to_vec();
        space.allocated_count = image.allocated_count();
        space.allocated_bytes = image.allocated_bytes();
        space
    }

    /// Return one frozen shared-memory root.
    pub fn image(&self) -> SharedSpaceImage {
        // capture the live regions directly
        let regions = self.capture_region_images();

        SharedSpaceImage::new(
            regions,
            self.next_unused_id,
            self.free_ids.clone().into_boxed_slice(),
            self.allocated_count,
            self.allocated_bytes,
            self.page_bytes(),
        )
    }

    /// Retain every arena page reachable from one frozen shared-space root.
    fn retain_image_pages(arena: &Arc<Arena>, image: &SharedSpaceImage) {
        for region in image.regions() {
            arena.retain_pages(&region.pages);
        }
    }

    /// Capture every live shared region image.
    fn capture_region_images(&self) -> Box<[SharedRegionImage]> {
        self.regions
            .iter()
            .map(Self::capture_region_image)
            .collect::<Vec<_>>()
            .into_boxed_slice()
    }

    /// Capture one live shared region image.
    fn capture_region_image(region: &SharedRegion) -> SharedRegionImage {
        SharedRegionImage {
            is_allocated: region.is_allocated,
            len: region.len,
            pages: region.pages.clone(),
        }
    }
}
