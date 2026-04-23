use std::sync::Arc;

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

use super::{SharedRawEntry, SharedRawSpace};
use crate::allocator::PageRunCache;
use crate::{AllocationUsage, Allocator, HeapResult, PageId, PageView};

/// One frozen shared raw-space entry root.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SharedRawEntryImage {
    /// Whether this entry is live.
    pub is_live: bool,
    /// The logical byte length of this entry.
    pub len: usize,
    /// The page view for this entry.
    pub pages: PageView,
}

/// One frozen shared raw-space root.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SharedRawSpaceImage {
    /// Captured shared raw-space entries keyed by entry index.
    entries: Box<[SharedRawEntryImage]>,

    /// The number of allocated shared raw-space entries.
    allocated_count: usize,
    /// The number of allocated shared raw-space bytes.
    allocated_bytes: u64,
}

impl SharedRawSpaceImage {
    /// Create one frozen shared raw-space root.
    pub fn new(
        entries: Box<[SharedRawEntryImage]>,
        allocated_count: usize,
        allocated_bytes: u64,
    ) -> Self {
        Self {
            entries,
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

    /// Return the number of allocated entries.
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

        // collect every frozen entry page
        for entry in &*self.entries {
            pages.extend(entry.pages.page_ids());
        }

        pages
    }
}

impl SharedRawSpace {
    /// Fork one shared raw-space root over the same shared allocator.
    pub fn fork(&self) -> HeapResult<Self> {
        let entries = self.entries.read();
        let mut retained = Vec::new();

        // retain the shared backing before cloning metadata
        for entry in &*entries {
            let entry = entry.read();

            if let Err(error) = self.allocator.retain_page_view(&entry.pages) {
                for page_view in retained.into_iter().rev() {
                    self.allocator.release_page_view(&page_view)?;
                }

                return Err(error);
            }

            retained.push(entry.pages.clone());
        }

        let state = self.state.lock();
        let forked = Self {
            allocator: self.allocator.clone(),
            state: parking_lot::Mutex::new(super::space::SharedRawState {
                page_run_cache: PageRunCache::new(self.allocator.pages_per_segment()),
                usage: state.usage,
            }),
            page_owners: parking_lot::RwLock::new(Vec::new()),
            entries: parking_lot::RwLock::new(
                entries
                    .iter()
                    .map(|entry: &Arc<RwLock<SharedRawEntry>>| {
                        Arc::new(RwLock::new(entry.read().clone()))
                    })
                    .collect(),
            ),
        };

        drop(state);
        drop(entries);

        rebuild_page_owners(&forked)?;

        Ok(forked)
    }

    /// Create one shared raw-space root from one frozen shared raw-space image over one shared allocator.
    pub fn from_image_with_allocator(
        allocator: Arc<Allocator>,
        image: &SharedRawSpaceImage,
    ) -> HeapResult<Self> {
        let mut retained = Vec::new();

        // retain the shared backing first
        for entry in image.entries() {
            if let Err(error) = allocator.retain_page_view(&entry.pages) {
                for page_view in retained.into_iter().rev() {
                    allocator.release_page_view(&page_view)?;
                }

                return Err(error);
            }

            retained.push(entry.pages.clone());
        }

        // rebuild the live root over the retained pages
        let entries = image
            .entries()
            .iter()
            .map(|entry| {
                if entry.is_live {
                    Arc::new(RwLock::new(SharedRawEntry::new(
                        entry.len,
                        entry.pages.clone(),
                    )))
                } else {
                    Arc::new(RwLock::new(SharedRawEntry::vacant()))
                }
            })
            .collect();
        let restored = Self {
            allocator: allocator.clone(),
            state: parking_lot::Mutex::new(super::space::SharedRawState {
                page_run_cache: PageRunCache::new(allocator.pages_per_segment()),
                usage: AllocationUsage::new(image.allocated_count(), image.allocated_bytes()),
            }),
            page_owners: parking_lot::RwLock::new(Vec::new()),
            entries: parking_lot::RwLock::new(entries),
        };

        rebuild_page_owners(&restored)?;

        Ok(restored)
    }

    /// Return one frozen shared raw-space root.
    pub fn image(&self) -> SharedRawSpaceImage {
        let entries = self.entries.read();
        let state = self.state.lock();

        SharedRawSpaceImage::new(
            entries
                .iter()
                .map(|entry: &Arc<RwLock<SharedRawEntry>>| {
                    let entry = entry.read();

                    SharedRawEntryImage {
                        is_live: !entry.is_vacant(),
                        len: entry.len,
                        pages: entry.pages.clone(),
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

/// Rebuild the visible page owners for one shared raw-space root.
fn rebuild_page_owners(raw: &SharedRawSpace) -> HeapResult<()> {
    let entries = raw.entries.read();

    for (entry_index, entry) in entries.iter().enumerate() {
        let entry = entry.read();

        if entry.is_vacant() {
            continue;
        }

        raw.map_page_view(&entry.pages, entry_index)?;
    }

    Ok(())
}
