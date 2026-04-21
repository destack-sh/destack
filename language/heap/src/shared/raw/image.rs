use std::sync::Arc;

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

use super::{SharedRawEntry, SharedRawSpace};
use crate::arena::PageRunCache;
use crate::{AllocationUsage, Arena, HeapResult, PageId, PageView};

/// One frozen shared raw-space entry root.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SharedRawEntryImage {
    /// Whether this entry id is live.
    pub is_live: bool,
    /// The logical byte length of this entry.
    pub len: usize,
    /// The page view for this entry.
    pub pages: PageView,
}

/// One frozen shared raw-space root.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SharedRawSpaceImage {
    /// Captured shared raw-space entries keyed by entry id minus one.
    entries: Box<[SharedRawEntryImage]>,
    /// The captured free shared raw-space entry ids.
    free_ids: Box<[u64]>,
    /// The next shared raw-space entry id to allocate.
    next_unused_id: u64,

    /// The number of allocated shared raw-space entries.
    allocated_count: usize,
    /// The number of allocated shared raw-space bytes.
    allocated_bytes: u64,
}

impl SharedRawSpaceImage {
    /// Create one frozen shared raw-space root.
    pub fn new(
        entries: Box<[SharedRawEntryImage]>,
        free_ids: Box<[u64]>,
        next_unused_id: u64,
        allocated_count: usize,
        allocated_bytes: u64,
    ) -> Self {
        Self {
            entries,
            free_ids,
            next_unused_id,
            allocated_count,
            allocated_bytes,
        }
    }

    /// Return one shared entry image by index.
    pub fn entry(&self, index: usize) -> Option<&SharedRawEntryImage> {
        self.entries.get(index)
    }

    /// Return the frozen shared raw-space entries.
    pub fn entries(&self) -> &[SharedRawEntryImage] {
        &self.entries
    }

    /// Return the frozen free entry ids.
    pub fn free_ids(&self) -> &[u64] {
        &self.free_ids
    }

    /// Return the next shared raw-space entry id.
    pub const fn next_unused_id(&self) -> u64 {
        self.next_unused_id
    }

    /// Return the number of allocated entries.
    pub const fn allocated_count(&self) -> usize {
        self.allocated_count
    }

    /// Return the allocated shared bytes.
    pub const fn allocated_bytes(&self) -> u64 {
        self.allocated_bytes
    }

    /// Return every arena page reachable from this shared raw-space image.
    pub fn page_ids(&self) -> Vec<PageId> {
        let mut pages = Vec::new();

        // collect every frozen entry page
        for entry in &*self.entries {
            pages.extend(entry.pages.page_ids());
        }

        pages
    }
}

impl SharedRawSpace {
    /// Fork one shared raw-space root over the same shared arena.
    pub fn fork(&self) -> HeapResult<Self> {
        let entries = self.entries.read();
        let mut retained = Vec::new();

        // retain the shared backing before cloning metadata
        for entry in &*entries {
            let entry = entry.read();

            if let Err(error) = self.arena.retain_page_view(&entry.pages) {
                for page_view in retained.into_iter().rev() {
                    self.arena.release_page_view(&page_view)?;
                }

                return Err(error);
            }

            retained.push(entry.pages);
        }

        let allocator = self.allocator.lock();
        let entries = entries
            .iter()
            .map(|entry: &Arc<RwLock<SharedRawEntry>>| Arc::new(RwLock::new(entry.read().clone())))
            .collect();

        Ok(Self {
            arena: self.arena.clone(),
            allocator: parking_lot::Mutex::new(super::space::SharedRawAllocator {
                page_run_cache: PageRunCache::new(self.arena.pages_per_segment()),
                free_ids: allocator.free_ids.clone(),
                next_unused_id: allocator.next_unused_id,
                usage: allocator.usage,
            }),
            entries: parking_lot::RwLock::new(entries),
        })
    }

    /// Create one shared raw-space root from one frozen shared raw-space image over one shared arena.
    pub fn from_image_with_arena(
        arena: Arc<Arena>,
        image: &SharedRawSpaceImage,
    ) -> HeapResult<Self> {
        let mut retained = Vec::new();

        // retain the shared backing first
        for entry in image.entries() {
            if let Err(error) = arena.retain_page_view(&entry.pages) {
                for page_view in retained.into_iter().rev() {
                    arena.release_page_view(&page_view)?;
                }

                return Err(error);
            }

            retained.push(entry.pages);
        }

        // rebuild the live root over the retained pages
        let entries = image
            .entries()
            .iter()
            .map(|entry| {
                if entry.is_live {
                    Arc::new(parking_lot::RwLock::new(SharedRawEntry::new(
                        entry.len,
                        entry.pages,
                    )))
                } else {
                    Arc::new(RwLock::new(SharedRawEntry::vacant()))
                }
            })
            .collect();

        Ok(Self {
            arena: arena.clone(),
            allocator: parking_lot::Mutex::new(super::space::SharedRawAllocator {
                page_run_cache: PageRunCache::new(arena.pages_per_segment()),
                free_ids: image.free_ids().to_vec(),
                next_unused_id: image.next_unused_id(),
                usage: AllocationUsage::new(image.allocated_count(), image.allocated_bytes()),
            }),
            entries: parking_lot::RwLock::new(entries),
        })
    }

    /// Return one frozen shared raw-space root.
    pub fn image(&self) -> SharedRawSpaceImage {
        let entries = self.entries.read();
        let allocator = self.allocator.lock();

        SharedRawSpaceImage::new(
            entries
                .iter()
                .map(|entry: &Arc<RwLock<SharedRawEntry>>| {
                    let entry = entry.read();

                    SharedRawEntryImage {
                        is_live: !entry.is_vacant(),
                        len: entry.len,
                        pages: entry.pages,
                    }
                })
                .collect::<Vec<_>>()
                .into_boxed_slice(),
            allocator.free_ids.clone().into_boxed_slice(),
            allocator.next_unused_id,
            allocator.usage.allocation_count(),
            allocator.usage.allocated_bytes(),
        )
    }

    /// Return every arena page reachable from this live shared raw space.
    pub fn page_ids(&self) -> Vec<PageId> {
        let entries = self.entries.read();
        let mut pages = Vec::new();

        // collect every live entry page
        for entry in &*entries {
            let entry = entry.read();
            pages.extend(entry.pages.page_ids());
        }

        pages
    }
}
