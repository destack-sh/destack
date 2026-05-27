use std::sync::Arc;

use destack_memory::AddressSpace;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

use super::{SharedRawAllocation, SharedRawSpace};
use crate::allocator::PageRunCache;
use crate::{AllocationUsage, Allocator, HeapResult, PageId, PageRun};

/// One frozen shared raw-space image.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SharedRawSpaceImage {
    /// Captured shared raw-space allocations keyed by allocation index.
    allocations: Box<[SharedRawAllocationImage]>,

    /// The number of allocated shared raw-space allocations.
    allocated_count: usize,
    /// The number of allocated shared raw-space bytes.
    allocated_bytes: u64,
    /// The reserved virtual byte capacity for shared raw space.
    space_bytes: usize,
    /// The next unused byte offset in shared raw space.
    next_offset: usize,
}

impl SharedRawSpaceImage {
    /// Create one frozen shared raw-space image.
    pub fn new(
        allocations: Box<[SharedRawAllocationImage]>,
        allocated_count: usize,
        allocated_bytes: u64,
        space_bytes: usize,
        next_offset: usize,
    ) -> Self {
        Self {
            allocations,
            allocated_count,
            allocated_bytes,
            space_bytes,
            next_offset,
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

    /// Return the reserved virtual byte capacity for shared raw space.
    pub const fn space_bytes(&self) -> usize {
        self.space_bytes
    }

    /// Return the next unused byte offset in shared raw space.
    pub const fn next_offset(&self) -> usize {
        self.next_offset
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

    /// Return every live allocator page run captured by this image.
    pub fn page_runs(&self) -> Vec<PageRun> {
        self.allocations
            .iter()
            .filter_map(|allocation| allocation.is_live.then_some(allocation.pages))
            .collect()
    }
}

/// One frozen shared raw-space allocation image.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SharedRawAllocationImage {
    /// Whether this allocation is live.
    pub is_live: bool,
    /// The first byte offset inside shared raw space.
    pub first_offset: usize,
    /// The logical byte length of this allocation.
    pub byte_len: usize,
    /// The page run for this allocation.
    pub pages: PageRun,
}

impl SharedRawSpace {
    /// Fork one shared raw space over the same shared allocator.
    ///
    /// Call this only from a safepoint where shared raw mutators are stopped.
    pub fn fork(&self) -> HeapResult<Self> {
        let allocations = self.allocations.read();
        let state = self.state.lock();
        let mapping = self.mapping.write().fork_lazy()?;
        let base_address = mapping.base_address();

        let forked = Self {
            allocator: self.allocator.clone(),
            base_address,
            state: parking_lot::Mutex::new(super::space::SharedRawState {
                page_run_cache: PageRunCache::new(self.allocator.pages_per_chunk()),
                usage: state.usage,
                live_retained_bytes: state.live_retained_bytes,
                next_offset: state.next_offset,
            }),
            page_map: parking_lot::RwLock::new(Vec::new()),
            allocations: parking_lot::RwLock::new(
                allocations
                    .iter()
                    .map(
                        |allocation: &Arc<RwLock<SharedRawAllocation>>| -> HeapResult<_> {
                            let allocation = allocation.read();
                            let pages = if allocation.is_vacant() {
                                PageRun::empty()
                            } else {
                                self.allocator.share_page_run(allocation.pages)?
                            };
                            let mut allocation = allocation.clone();
                            allocation.pages = pages;

                            Ok(Arc::new(RwLock::new(allocation)))
                        },
                    )
                    .collect::<HeapResult<Vec<_>>>()?,
            ),
            mapping: parking_lot::RwLock::new(mapping),
        };

        drop(state);
        drop(allocations);

        rebuild_page_map(&forked);

        Ok(forked)
    }

    /// Create one shared raw space from one frozen image over one shared allocator.
    pub fn from_image_with_allocator(
        allocator: Arc<Allocator>,
        image: &SharedRawSpaceImage,
    ) -> HeapResult<Self> {
        let mapping = AddressSpace::reserve(image.space_bytes(), allocator.page_bytes())?;
        let base_address = mapping.base_address();

        let allocations = image
            .allocations()
            .iter()
            .map(|allocation| -> HeapResult<_> {
                let allocation = if allocation.is_live {
                    let byte_len = allocation.pages.len() * allocator.page_bytes();
                    let bytes = allocator.read_bytes_from(&allocation.pages, 0, byte_len)?;
                    let pages = allocator.allocate_pages(byte_len)?;

                    mapping.write_bytes(allocation.first_offset, &bytes[..allocation.byte_len])?;

                    Arc::new(RwLock::new(SharedRawAllocation::new(
                        allocation.first_offset,
                        allocation.byte_len,
                        pages,
                    )))
                } else {
                    Arc::new(RwLock::new(SharedRawAllocation::vacant()))
                };

                Ok(allocation)
            })
            .collect::<HeapResult<Vec<_>>>()?;
        let live_retained_bytes = live_retained_bytes(&allocations, allocator.page_bytes());
        let restored = Self {
            allocator: allocator.clone(),
            base_address,
            state: parking_lot::Mutex::new(super::space::SharedRawState {
                page_run_cache: PageRunCache::new(allocator.pages_per_chunk()),
                usage: AllocationUsage::new(image.allocated_count(), image.allocated_bytes()),
                live_retained_bytes,
                next_offset: image.next_offset(),
            }),
            page_map: parking_lot::RwLock::new(Vec::new()),
            allocations: parking_lot::RwLock::new(allocations),
            mapping: parking_lot::RwLock::new(mapping),
        };

        rebuild_page_map(&restored);

        Ok(restored)
    }

    /// Return one frozen shared raw-space image.
    pub fn image(&self) -> HeapResult<SharedRawSpaceImage> {
        let allocations = self.allocations.read();
        let state = self.state.lock();

        Ok(SharedRawSpaceImage::new(
            allocations
                .iter()
                .map(
                    |allocation: &Arc<RwLock<SharedRawAllocation>>| -> HeapResult<_> {
                        let allocation = allocation.read();
                        let pages = if allocation.is_vacant() {
                            PageRun::empty()
                        } else {
                            let byte_len = allocation.pages.len() * self.allocator.page_bytes();
                            let bytes = self
                                .mapping
                                .read()
                                .read_bytes(allocation.first_offset, byte_len)?;

                            // images own the captured bytes
                            self.allocator.allocate_image_bytes(&bytes)?
                        };

                        Ok(SharedRawAllocationImage {
                            is_live: !allocation.is_vacant(),
                            first_offset: allocation.first_offset,
                            byte_len: allocation.byte_len,
                            pages,
                        })
                    },
                )
                .collect::<HeapResult<Vec<_>>>()?
                .into_boxed_slice(),
            state.usage.allocation_count(),
            state.usage.allocated_bytes(),
            self.mapping.read().byte_len(),
            state.next_offset,
        ))
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

/// Rebuild the address space entries for one live shared raw space.
fn rebuild_page_map(raw: &SharedRawSpace) {
    let allocations = raw.allocations.read();

    for (allocation_index, allocation) in allocations.iter().enumerate() {
        let allocation = allocation.read();

        if allocation.is_vacant() {
            continue;
        }

        raw.map_page_run(allocation.first_offset, &allocation.pages, allocation_index);
    }
}

/// Return the live retained bytes for restored raw allocations.
fn live_retained_bytes(allocations: &[Arc<RwLock<SharedRawAllocation>>], page_bytes: usize) -> u64 {
    let mut retained_bytes = 0;

    for allocation in allocations {
        let allocation = allocation.read();

        if allocation.is_vacant() {
            continue;
        }

        retained_bytes += allocation.pages.len() as u64 * page_bytes as u64;
    }

    retained_bytes
}
