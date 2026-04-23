use std::sync::Arc;

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

use super::{
    SharedHeapLargeEntryImage, SharedHeapPageOwner, SharedHeapSmallSpanImage, SharedHeapSpace,
    SharedLargeEntry, SharedLargeEntryId, SharedSmallSpan,
};
use crate::allocator::PageRunCache;
use crate::shared::gc::SharedGcState;
use crate::shared::space::{
    SharedHeapState, SharedLargeSpace, SharedSmallSpace, find_next_free_slot,
};
use crate::{
    AllocationUsage, Allocator, GcSummary, HeapResult, PageId, Shape, ShapeId, ShapeTable,
    SizeClassTable,
};

/// One frozen shared heap-space root.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SharedHeapSpaceImage {
    /// The configured size-class table.
    size_classes: SizeClassTable,
    /// The configured small-space span width.
    small_bytes: usize,
    /// The captured shared heap spans.
    spans: Box<[SharedHeapSmallSpanImage]>,
    /// The configured shared page width.
    page_bytes: usize,
    /// The captured shared heap entries in large space.
    entries: Box<[SharedHeapLargeEntryImage]>,
    /// The captured shared heap entry shapes.
    shapes: Box<[Shape]>,
    /// The captured free shared heap large-entry ids.
    free_large_entry_ids: Box<[u64]>,
    /// The next shared heap large-entry id to allocate.
    next_unused_large_entry_id: u64,
    /// The number of allocated shared heap-space entries.
    allocated_count: usize,
    /// The number of allocated shared heap-space bytes.
    allocated_bytes: u64,
    /// The captured shared heap collector state.
    gc_state: GcSummary,
}

impl SharedHeapSpaceImage {
    /// Create one frozen shared heap-space root.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        size_classes: SizeClassTable,
        small_bytes: usize,
        spans: Box<[SharedHeapSmallSpanImage]>,
        page_bytes: usize,
        entries: Box<[SharedHeapLargeEntryImage]>,
        shapes: Box<[Shape]>,
        free_large_entry_ids: Box<[u64]>,
        next_unused_large_entry_id: u64,
        allocated_count: usize,
        allocated_bytes: u64,
        gc_state: GcSummary,
    ) -> Self {
        Self {
            size_classes,
            small_bytes,
            spans,
            page_bytes,
            entries,
            shapes,
            free_large_entry_ids,
            next_unused_large_entry_id,
            allocated_count,
            allocated_bytes,
            gc_state,
        }
    }

    /// Return the configured size-class table.
    pub fn size_classes(&self) -> &SizeClassTable {
        &self.size_classes
    }

    /// Return the configured small-space span width.
    pub const fn small_bytes(&self) -> usize {
        self.small_bytes
    }

    /// Return the captured shared heap spans.
    pub fn spans(&self) -> &[SharedHeapSmallSpanImage] {
        &self.spans
    }

    /// Return the configured shared page width.
    pub const fn page_bytes(&self) -> usize {
        self.page_bytes
    }

    /// Return the captured shared heap entries in large space.
    pub fn entries(&self) -> &[SharedHeapLargeEntryImage] {
        &self.entries
    }

    /// Return the next shared heap large-entry id.
    pub const fn next_unused_large_entry_id(&self) -> u64 {
        self.next_unused_large_entry_id
    }

    /// Return the number of allocated shared heap-space entries.
    pub const fn allocated_count(&self) -> usize {
        self.allocated_count
    }

    /// Return the allocated shared heap-space bytes.
    pub const fn allocated_bytes(&self) -> u64 {
        self.allocated_bytes
    }

    /// Return the captured shared heap collector state.
    pub fn gc_state(&self) -> &GcSummary {
        &self.gc_state
    }

    /// Return every allocator page reachable from this shared heap-space image.
    pub fn page_ids(&self) -> Vec<PageId> {
        let mut pages = Vec::new();

        // span pages
        for span in &*self.spans {
            pages.extend(span.pages.page_ids());
        }

        // large-entry pages
        for entry in &*self.entries {
            pages.extend(entry.pages.page_ids());
        }

        pages
    }
}

impl SharedHeapSpace {
    /// Fork one shared heap-space root over the same shared allocator.
    pub fn fork(&self) -> HeapResult<Self> {
        let store = self.store.read();
        let mut retained = Vec::new();

        // span pages
        for span in &store.small.spans {
            let span = span.read();

            if let Err(error) = self.allocator.retain_page_view(&span.pages) {
                for page_view in retained.into_iter().rev() {
                    self.allocator.release_page_view(&page_view)?;
                }

                return Err(error);
            }

            retained.push(span.pages.clone());
        }

        // large-entry pages
        for entry in &store.large.entries {
            let entry = entry.read();

            if let Err(error) = self.allocator.retain_page_view(&entry.pages) {
                for page_view in retained.into_iter().rev() {
                    self.allocator.release_page_view(&page_view)?;
                }

                return Err(error);
            }

            retained.push(entry.pages.clone());
        }

        let mut cloned_store = SharedHeapState {
            page_run_cache: PageRunCache::new(self.allocator.pages_per_segment()),
            small: SharedSmallSpace {
                size_classes: store.small.size_classes.clone(),
                span_bytes: store.small.span_bytes,
                spans: store
                    .small
                    .spans
                    .iter()
                    .map(|span| Arc::new(RwLock::new(span.read().clone())))
                    .collect(),
                available_spans: store.small.available_spans.clone(),
            },
            large: SharedLargeSpace {
                page_bytes: store.large.page_bytes,
                entries: store
                    .large
                    .entries
                    .iter()
                    .map(|entry| Arc::new(RwLock::new(entry.read().clone())))
                    .collect(),
                free_large_entry_ids: store.large.free_large_entry_ids.clone(),
                next_unused_large_entry_id: store.large.next_unused_large_entry_id,
            },
            page_owners: Vec::new(),
            shape_table: store.shape_table.clone(),
            usage: store.usage,
            gc_state: store.gc_state.clone(),
        };

        Self::rebuild_page_owners(&mut cloned_store)?;

        Ok(Self {
            allocator: self.allocator.clone(),
            store: RwLock::new(cloned_store),
            gc: SharedGcState::default(),
        })
    }

    /// Create one shared heap-space root from one frozen image over one shared allocator.
    pub fn from_image_with_allocator(
        allocator: Arc<Allocator>,
        image: &SharedHeapSpaceImage,
    ) -> HeapResult<Self> {
        let mut retained = Vec::new();

        // span pages
        for span in image.spans() {
            if let Err(error) = allocator.retain_page_view(&span.pages) {
                for page_view in retained.into_iter().rev() {
                    allocator.release_page_view(&page_view)?;
                }

                return Err(error);
            }

            retained.push(span.pages.clone());
        }

        // large-entry pages
        for entry in image.entries() {
            if let Err(error) = allocator.retain_page_view(&entry.pages) {
                for page_view in retained.into_iter().rev() {
                    allocator.release_page_view(&page_view)?;
                }

                return Err(error);
            }

            retained.push(entry.pages.clone());
        }

        let mut store = SharedHeapState {
            page_run_cache: PageRunCache::new(allocator.pages_per_segment()),
            small: SharedSmallSpace {
                size_classes: image.size_classes().clone(),
                span_bytes: image.small_bytes(),
                spans: image
                    .spans()
                    .iter()
                    .map(|span| -> HeapResult<Arc<RwLock<SharedSmallSpan>>> {
                        Ok(Arc::new(RwLock::new(SharedSmallSpan {
                            size_class: span.size_class,
                            slot_count: span.slot_count,
                            occupied_count: span.occupied.count_ones(),
                            next_free_slot: 0,
                            lengths: span.lengths.clone(),
                            occupied: span.occupied.clone(),
                            marked: crate::Bitmap::with_capacity(span.slot_count),
                            shape_ids: span
                                .shape_ids
                                .iter()
                                .map(|shape_id| shape_id.map(ShapeId::from_raw).transpose())
                                .collect::<HeapResult<Vec<_>>>()?
                                .into_boxed_slice(),
                            pages: span.pages.clone(),
                        })))
                    })
                    .collect::<HeapResult<Vec<_>>>()?,
                available_spans: vec![Vec::new(); image.size_classes().classes.len()],
            },
            large: SharedLargeSpace {
                page_bytes: image.page_bytes(),
                entries: image
                    .entries()
                    .iter()
                    .map(|entry| -> HeapResult<Arc<RwLock<SharedLargeEntry>>> {
                        Ok(Arc::new(RwLock::new(SharedLargeEntry {
                            is_live: entry.is_live,
                            len: entry.len,
                            pages: entry.pages.clone(),
                            shape_id: ShapeId::from_raw(entry.shape_id)?,
                            is_marked: false,
                        })))
                    })
                    .collect::<HeapResult<Vec<_>>>()?,
                free_large_entry_ids: image.free_large_entry_ids.to_vec(),
                next_unused_large_entry_id: image.next_unused_large_entry_id(),
            },
            page_owners: Vec::new(),
            shape_table: ShapeTable::from_shapes(image.shapes.to_vec())?,
            usage: AllocationUsage::new(image.allocated_count(), image.allocated_bytes()),
            gc_state: image.gc_state().clone(),
        };

        Self::rebuild_page_owners(&mut store)?;

        Ok(Self {
            allocator,
            store: RwLock::new(store),
            gc: SharedGcState::default(),
        }
        .with_rebuilt_small_availability())
    }

    /// Return one frozen shared heap-space root.
    pub fn image(&self) -> SharedHeapSpaceImage {
        let store = self.store.read();

        SharedHeapSpaceImage::new(
            store.small.size_classes.clone(),
            store.small.span_bytes,
            store
                .small
                .spans
                .iter()
                .map(|span| {
                    let span = span.read();

                    SharedHeapSmallSpanImage {
                        size_class: span.size_class,
                        slot_count: span.slot_count,
                        lengths: span.lengths.clone(),
                        occupied: span.occupied.clone(),
                        shape_ids: span
                            .shape_ids
                            .iter()
                            .map(|shape_id| shape_id.map(ShapeId::raw))
                            .collect::<Vec<_>>()
                            .into_boxed_slice(),
                        pages: span.pages.clone(),
                    }
                })
                .collect::<Vec<_>>()
                .into_boxed_slice(),
            store.large.page_bytes,
            store
                .large
                .entries
                .iter()
                .map(|entry| {
                    let entry = entry.read();

                    SharedHeapLargeEntryImage {
                        is_live: entry.is_live,
                        len: entry.len,
                        pages: entry.pages.clone(),
                        shape_id: entry.shape_id.raw(),
                    }
                })
                .collect::<Vec<_>>()
                .into_boxed_slice(),
            store.shape_table.shapes().to_vec().into_boxed_slice(),
            store.large.free_large_entry_ids.clone().into_boxed_slice(),
            store.large.next_unused_large_entry_id,
            store.usage.allocation_count(),
            store.usage.allocated_bytes(),
            store.gc_state.clone(),
        )
    }

    /// Return every allocator page reachable from this live shared heap space.
    pub fn page_ids(&self) -> Vec<PageId> {
        let store = self.store.read();
        let mut pages = Vec::new();

        // span pages
        for span in &store.small.spans {
            pages.extend(span.read().pages.page_ids());
        }

        // large-entry pages
        for entry in &store.large.entries {
            pages.extend(entry.read().pages.page_ids());
        }

        pages
    }

    /// Rebuild shared small-span availability after cloning or restore.
    fn with_rebuilt_small_availability(self) -> Self {
        let mut store = self.store.write();

        for span_index in 0..store.small.spans.len() {
            let size_class;
            let occupied_count;
            let slot_count;

            {
                let mut span = store.small.spans[span_index].write();
                span.next_free_slot = find_next_free_slot(&span.occupied, 0, span.slot_count);
                size_class = span.size_class;
                occupied_count = span.occupied_count;
                slot_count = span.slot_count;
            }

            if occupied_count == 0 || occupied_count >= slot_count {
                continue;
            }

            if let Some(class_index) = store.small.size_classes.class_index_for(size_class) {
                store.small.available_spans[class_index].push(span_index);
            }
        }

        drop(store);

        self
    }

    /// Rebuild the page-owner table from live shared storage.
    fn rebuild_page_owners(store: &mut SharedHeapState) -> HeapResult<()> {
        store.page_owners.clear();

        for span_index in 0..store.small.spans.len() {
            let Some(span) = store.small.spans.get(span_index).cloned() else {
                return Err(crate::HeapError::MissingSpan { span_index });
            };
            let span = span.read();

            for logical_page_index in 0..span.pages.len() {
                let Some(page_id) = span.pages.page(logical_page_index) else {
                    return Err(crate::HeapError::MissingLogicalPage {
                        page_index: logical_page_index,
                    });
                };
                let page_index = page_id.index();

                if store.page_owners.len() <= page_index {
                    store.page_owners.resize(page_index + 1, None);
                }

                store.page_owners[page_index] = Some(SharedHeapPageOwner::Small {
                    span_index,
                    logical_page_index,
                });
            }
        }

        for entry_index in 0..store.large.entries.len() {
            let entry_id = SharedLargeEntryId::new(entry_index as u64 + 1);
            let Some(entry) = store.large.entries.get(entry_index).cloned() else {
                continue;
            };
            let entry = entry.read();
            if !entry.is_live {
                continue;
            }

            for logical_page_index in 0..entry.pages.len() {
                let Some(page_id) = entry.pages.page(logical_page_index) else {
                    return Err(crate::HeapError::MissingLogicalPage {
                        page_index: logical_page_index,
                    });
                };
                let page_index = page_id.index();

                if store.page_owners.len() <= page_index {
                    store.page_owners.resize(page_index + 1, None);
                }

                store.page_owners[page_index] = Some(SharedHeapPageOwner::Large {
                    entry_id,
                    logical_page_index,
                });
            }
        }

        Ok(())
    }
}
