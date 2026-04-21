use std::sync::Arc;

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

use super::{
    SharedLargeEntry, SharedManagedLargeEntryImage, SharedManagedReferenceEntry,
    SharedManagedSmallSpanImage, SharedManagedSpace, SharedSmallSpan,
};
use crate::arena::PageRunCache;
use crate::shared::gc::SharedGcState;
use crate::shared::managed::space::{
    SharedLargeSpace, SharedManagedState, SharedSmallSpace, find_next_free_slot,
};
use crate::{
    AllocationUsage, Arena, GcSummary, HeapResult, PageId, Shape, ShapeId, ShapeTable,
    SizeClassTable,
};

/// One frozen shared managed-space root.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SharedManagedSpaceImage {
    /// The configured size-class table.
    size_classes: SizeClassTable,
    /// The configured small-space span width.
    small_bytes: usize,
    /// The captured shared managed spans.
    spans: Box<[SharedManagedSmallSpanImage]>,
    /// The configured shared page width.
    page_bytes: usize,
    /// The captured shared managed entries in large space.
    entries: Box<[SharedManagedLargeEntryImage]>,
    /// Captured shared managed references keyed by reference id minus one.
    references: Box<[SharedManagedReferenceEntry]>,
    /// The captured shared managed entry shapes.
    shapes: Box<[Shape]>,
    /// The captured free shared managed reference ids.
    free_reference_ids: Box<[u64]>,
    /// The next shared managed reference id to allocate.
    next_unused_reference_id: u64,
    /// The captured free shared managed large-entry ids.
    free_large_entry_ids: Box<[u64]>,
    /// The next shared managed large-entry id to allocate.
    next_unused_large_entry_id: u64,
    /// The number of allocated shared managed-space entries.
    allocated_count: usize,
    /// The number of allocated shared managed-space bytes.
    allocated_bytes: u64,
    /// The captured shared managed collector state.
    gc_state: GcSummary,
}

impl SharedManagedSpaceImage {
    /// Create one frozen shared managed-space root.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        size_classes: SizeClassTable,
        small_bytes: usize,
        spans: Box<[SharedManagedSmallSpanImage]>,
        page_bytes: usize,
        entries: Box<[SharedManagedLargeEntryImage]>,
        references: Box<[SharedManagedReferenceEntry]>,
        shapes: Box<[Shape]>,
        free_reference_ids: Box<[u64]>,
        next_unused_reference_id: u64,
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
            references,
            shapes,
            free_reference_ids,
            next_unused_reference_id,
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

    /// Return the captured shared managed spans.
    pub fn spans(&self) -> &[SharedManagedSmallSpanImage] {
        &self.spans
    }

    /// Return the configured shared page width.
    pub const fn page_bytes(&self) -> usize {
        self.page_bytes
    }

    /// Return the captured shared managed entries in large space.
    pub fn entries(&self) -> &[SharedManagedLargeEntryImage] {
        &self.entries
    }

    /// Return the next shared managed reference id.
    pub const fn next_unused_reference_id(&self) -> u64 {
        self.next_unused_reference_id
    }

    /// Return the next shared managed large-entry id.
    pub const fn next_unused_large_entry_id(&self) -> u64 {
        self.next_unused_large_entry_id
    }

    /// Return the number of allocated shared managed-space entries.
    pub const fn allocated_count(&self) -> usize {
        self.allocated_count
    }

    /// Return the allocated shared managed-space bytes.
    pub const fn allocated_bytes(&self) -> u64 {
        self.allocated_bytes
    }

    /// Return the captured shared managed collector state.
    pub fn gc_state(&self) -> &GcSummary {
        &self.gc_state
    }

    /// Return every arena page reachable from this shared managed-space image.
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

impl SharedManagedSpace {
    /// Fork one shared managed-space root over the same shared arena.
    pub fn fork(&self) -> HeapResult<Self> {
        let store = self.store.read();
        let mut retained = Vec::new();

        // span pages
        for span in &store.small.spans {
            let span = span.read();

            if let Err(error) = self.arena.retain_page_view(&span.pages) {
                for page_view in retained.into_iter().rev() {
                    self.arena.release_page_view(&page_view)?;
                }

                return Err(error);
            }

            retained.push(span.pages);
        }

        // large-entry pages
        for entry in &store.large.entries {
            let entry = entry.read();

            if let Err(error) = self.arena.retain_page_view(&entry.pages) {
                for page_view in retained.into_iter().rev() {
                    self.arena.release_page_view(&page_view)?;
                }

                return Err(error);
            }

            retained.push(entry.pages);
        }

        let cloned_store = SharedManagedState {
            page_run_cache: PageRunCache::new(self.arena.pages_per_segment()),
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
            references: store.references.clone(),
            shape_table: store.shape_table.clone(),
            free_reference_ids: store.free_reference_ids.clone(),
            next_unused_reference_id: store.next_unused_reference_id,
            usage: store.usage,
            gc_state: store.gc_state.clone(),
        };

        Ok(Self {
            arena: self.arena.clone(),
            store: RwLock::new(cloned_store),
            gc: SharedGcState::default(),
        })
    }

    /// Create one shared managed-space root from one frozen image over one shared arena.
    pub fn from_image_with_arena(
        arena: Arc<Arena>,
        image: &SharedManagedSpaceImage,
    ) -> HeapResult<Self> {
        let mut retained = Vec::new();

        // span pages
        for span in image.spans() {
            if let Err(error) = arena.retain_page_view(&span.pages) {
                for page_view in retained.into_iter().rev() {
                    arena.release_page_view(&page_view)?;
                }

                return Err(error);
            }

            retained.push(span.pages);
        }

        // large-entry pages
        for entry in image.entries() {
            if let Err(error) = arena.retain_page_view(&entry.pages) {
                for page_view in retained.into_iter().rev() {
                    arena.release_page_view(&page_view)?;
                }

                return Err(error);
            }

            retained.push(entry.pages);
        }

        let store = SharedManagedState {
            page_run_cache: PageRunCache::new(arena.pages_per_segment()),
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
                            occupied: span.occupied.clone(),
                            reference_ids: span.reference_ids.clone(),
                            shape_ids: span
                                .shape_ids
                                .iter()
                                .map(|shape_id| shape_id.map(ShapeId::from_raw).transpose())
                                .collect::<HeapResult<Vec<_>>>()?
                                .into_boxed_slice(),
                            pages: span.pages,
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
                            pages: entry.pages,
                            shape_id: ShapeId::from_raw(entry.shape_id)?,
                        })))
                    })
                    .collect::<HeapResult<Vec<_>>>()?,
                free_large_entry_ids: image.free_large_entry_ids.to_vec(),
                next_unused_large_entry_id: image.next_unused_large_entry_id(),
            },
            references: image.references.to_vec(),
            shape_table: ShapeTable::from_shapes(image.shapes.to_vec())?,
            free_reference_ids: image.free_reference_ids.to_vec(),
            next_unused_reference_id: image.next_unused_reference_id(),
            usage: AllocationUsage::new(image.allocated_count(), image.allocated_bytes()),
            gc_state: image.gc_state().clone(),
        };

        Ok(Self {
            arena,
            store: RwLock::new(store),
            gc: SharedGcState::default(),
        }
        .with_rebuilt_small_availability())
    }

    /// Return one frozen shared managed-space root.
    pub fn image(&self) -> SharedManagedSpaceImage {
        let store = self.store.read();

        SharedManagedSpaceImage::new(
            store.small.size_classes.clone(),
            store.small.span_bytes,
            store
                .small
                .spans
                .iter()
                .map(|span| {
                    let span = span.read();

                    SharedManagedSmallSpanImage {
                        size_class: span.size_class,
                        slot_count: span.slot_count,
                        occupied: span.occupied.clone(),
                        reference_ids: span.reference_ids.clone(),
                        shape_ids: span
                            .shape_ids
                            .iter()
                            .map(|shape_id| shape_id.map(ShapeId::raw))
                            .collect::<Vec<_>>()
                            .into_boxed_slice(),
                        pages: span.pages,
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

                    SharedManagedLargeEntryImage {
                        is_live: entry.is_live,
                        len: entry.len,
                        pages: entry.pages,
                        shape_id: entry.shape_id.raw(),
                    }
                })
                .collect::<Vec<_>>()
                .into_boxed_slice(),
            store.references.clone().into_boxed_slice(),
            store.shape_table.shapes().to_vec().into_boxed_slice(),
            store.free_reference_ids.clone().into_boxed_slice(),
            store.next_unused_reference_id,
            store.large.free_large_entry_ids.clone().into_boxed_slice(),
            store.large.next_unused_large_entry_id,
            store.usage.allocation_count(),
            store.usage.allocated_bytes(),
            store.gc_state.clone(),
        )
    }

    /// Return every arena page reachable from this live shared managed space.
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
}
