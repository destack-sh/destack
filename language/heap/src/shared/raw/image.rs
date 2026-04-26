use std::sync::Arc;

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

use super::{SharedRawAllocation, SharedRawSpace};
use crate::allocator::PageRunCache;
use crate::{AllocationUsage, Allocator, HeapResult, PageId, PageView};

/// One frozen shared raw-space allocation root.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SharedRawAllocationImage {
    /// Whether this allocation is live.
    pub is_live: bool,
    /// The logical byte length of this allocation.
    pub len: usize,
    /// The page view for this allocation.
    pub pages: PageView,
}

/// One frozen shared raw-space root.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SharedRawSpaceImage {
    /// Captured shared raw-space allocations keyed by allocation index.
    allocations: Box<[SharedRawAllocationImage]>,

    /// The number of allocated shared raw-space allocations.
    allocated_count: usize,
    /// The number of allocated shared raw-space bytes.
    allocated_bytes: u64,
}

impl SharedRawSpaceImage {
    /// Create one frozen shared raw-space root.
    pub fn new(
        allocations: Box<[SharedRawAllocationImage]>,
        allocated_count: usize,
        allocated_bytes: u64,
    ) -> Self {
        Self {
            allocations,
            allocated_count,
            allocated_bytes,
        }
    }

    /// Return one shared allocation image by index.
    pub fn allocation(&self, index: usize) -> Option<&SharedRawAllocationImage> {
        self.allocations.get(index)
    }

    /// Return the frozen shared raw-space allocations.
    pub fn allocations(&self) -> &[SharedRawAllocationImage] {
        &self.allocations
    }

    /// Return the number of allocated allocations.
    pub const fn allocated_count(&self) -> usize {
        self.allocated_count
    }

    /// Return the allocated shared bytes.
    pub const fn allocated_bytes(&self) -> u64 {
        self.allocated_bytes
    }

    /// Return every allocator page reachable from this shared raw-space image.
    pub fn page_ids(&self) -> Vec<PageId> {
        let mut pages = Vec::new();

        // collect every frozen allocation page
        for allocation in &*self.allocations {
            pages.extend(allocation.pages.page_ids());
        }

        pages
    }

    /// Return every live allocator page view captured by this image.
    pub fn page_views(&self) -> Vec<PageView> {
        self.allocations
            .iter()
            .filter_map(|allocation| allocation.is_live.then_some(allocation.pages.clone()))
            .collect()
    }
}

impl SharedRawSpace {
    /// Fork one shared raw-space root over the same shared allocator.
    pub fn fork(&self) -> HeapResult<Self> {
        let allocations = self.allocations.read();
        let _retained_page_views = self.allocator.retain_page_views(
            allocations
                .iter()
                .map(|allocation| allocation.read().pages.clone()),
        )?;

        let state = self.state.lock();
        let forked = Self {
            allocator: self.allocator.clone(),
            state: parking_lot::Mutex::new(super::space::SharedRawState {
                page_run_cache: PageRunCache::new(self.allocator.pages_per_arena()),
                usage: state.usage,
            }),
            page_map: parking_lot::RwLock::new(Vec::new()),
            allocations: parking_lot::RwLock::new(
                allocations
                    .iter()
                    .map(|allocation: &Arc<RwLock<SharedRawAllocation>>| {
                        Arc::new(RwLock::new(allocation.read().clone()))
                    })
                    .collect(),
            ),
        };

        drop(state);
        drop(allocations);

        rebuild_page_map(&forked)?;

        Ok(forked)
    }

    /// Create one shared raw-space root from one frozen shared raw-space image over one shared allocator.
    pub fn from_image_with_allocator(
        allocator: Arc<Allocator>,
        image: &SharedRawSpaceImage,
    ) -> HeapResult<Self> {
        let _retained_page_views = allocator.retain_page_views(
            image
                .allocations()
                .iter()
                .map(|allocation| allocation.pages.clone()),
        )?;

        let allocations = image
            .allocations()
            .iter()
            .map(|allocation| {
                if allocation.is_live {
                    Arc::new(RwLock::new(SharedRawAllocation::new(
                        allocation.len,
                        allocation.pages.clone(),
                    )))
                } else {
                    Arc::new(RwLock::new(SharedRawAllocation::vacant()))
                }
            })
            .collect();
        let restored = Self {
            allocator: allocator.clone(),
            state: parking_lot::Mutex::new(super::space::SharedRawState {
                page_run_cache: PageRunCache::new(allocator.pages_per_arena()),
                usage: AllocationUsage::new(image.allocated_count(), image.allocated_bytes()),
            }),
            page_map: parking_lot::RwLock::new(Vec::new()),
            allocations: parking_lot::RwLock::new(allocations),
        };

        rebuild_page_map(&restored)?;

        Ok(restored)
    }

    /// Return one frozen shared raw-space root.
    pub fn image(&self) -> SharedRawSpaceImage {
        let allocations = self.allocations.read();
        let state = self.state.lock();

        SharedRawSpaceImage::new(
            allocations
                .iter()
                .map(|allocation: &Arc<RwLock<SharedRawAllocation>>| {
                    let allocation = allocation.read();

                    SharedRawAllocationImage {
                        is_live: !allocation.is_vacant(),
                        len: allocation.len,
                        pages: allocation.pages.clone(),
                    }
                })
                .collect::<Vec<_>>()
                .into_boxed_slice(),
            state.usage.allocation_count(),
            state.usage.allocated_bytes(),
        )
    }

    /// Return every allocator page reachable from this live shared raw space.
    pub fn page_ids(&self) -> Vec<PageId> {
        let allocations = self.allocations.read();
        let mut pages = Vec::new();

        // collect every live allocation page
        for allocation in &*allocations {
            let allocation = allocation.read();
            pages.extend(allocation.pages.page_ids());
        }

        pages
    }
}

/// Rebuild the page map entries for one shared raw-space root.
fn rebuild_page_map(raw: &SharedRawSpace) -> HeapResult<()> {
    let allocations = raw.allocations.read();

    for (allocation_index, allocation) in allocations.iter().enumerate() {
        let allocation = allocation.read();

        if allocation.is_vacant() {
            continue;
        }

        raw.map_page_view(&allocation.pages, allocation_index)?;
    }

    Ok(())
}
