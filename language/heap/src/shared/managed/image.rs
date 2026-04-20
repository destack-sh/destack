use serde::{Deserialize, Serialize};
use std::sync::Arc;

use super::{
    SharedLargeEntry, SharedManagedLargeEntryImage, SharedManagedReferenceEntry,
    SharedManagedSmallSpanImage, SharedManagedSpace, SharedSmallSpan,
};
use crate::shared::gc::SharedGcPhase;
use crate::shared::managed::space::{SharedLargeSpace, SharedSmallSpace};
use crate::{
    AllocationUsage, Arena, GcState, HeapResult, MarkSet, PageId, Shape, ShapeId, ShapeTable,
    SizeClassTable, TraceQueue,
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
    gc_state: GcState,
}

impl SharedManagedSpaceImage {
    /// Create one frozen shared managed-space root.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        size_classes: crate::SizeClassTable,
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
        gc_state: GcState,
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
    pub fn size_classes(&self) -> &crate::SizeClassTable {
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
    pub fn gc_state(&self) -> &GcState {
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
        let mut retained = Vec::new();

        // span pages
        for span in &self.small.spans {
            if let Err(error) = self.arena.retain_page_view(&span.pages) {
                for page_view in retained.into_iter().rev() {
                    self.arena.release_page_view(&page_view)?;
                }

                return Err(error);
            }

            retained.push(span.pages);
        }

        // large-entry pages
        for entry in &self.large.entries {
            if let Err(error) = self.arena.retain_page_view(&entry.pages) {
                for page_view in retained.into_iter().rev() {
                    self.arena.release_page_view(&page_view)?;
                }

                return Err(error);
            }

            retained.push(entry.pages);
        }

        Ok(Self {
            arena: self.arena.clone(),
            small: self.small.clone(),
            large: self.large.clone(),
            references: self.references.clone(),
            shape_table: self.shape_table.clone(),
            free_reference_ids: self.free_reference_ids.clone(),
            next_unused_reference_id: self.next_unused_reference_id,
            usage: self.usage,
            gc_state: self.gc_state.clone(),
            marks: MarkSet::default(),
            trace_queue: TraceQueue::default(),
            edge_buffer: Vec::new(),
            phase: SharedGcPhase::Idle,
            sweep_cursor: 0,
            cycle_freed_allocations: 0,
            cycle_freed_bytes: 0,
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

        Ok(Self {
            arena: arena.clone(),
            small: SharedSmallSpace {
                size_classes: image.size_classes().clone(),
                span_bytes: image.small_bytes(),
                spans: image
                    .spans()
                    .iter()
                    .map(|span| -> HeapResult<SharedSmallSpan> {
                        Ok(SharedSmallSpan {
                            size_class: span.size_class,
                            slot_count: span.slot_count,
                            occupied_count: span.occupied.count_ones(),
                            next_free_slot: 0,
                            occupied: span.occupied.clone(),
                            shape_ids: span
                                .shape_ids
                                .iter()
                                .map(|shape_id| shape_id.map(ShapeId::from_raw).transpose())
                                .collect::<HeapResult<Vec<_>>>()?
                                .into_boxed_slice(),
                            pages: span.pages,
                        })
                    })
                    .collect::<HeapResult<Vec<_>>>()?,
                available_spans: vec![Vec::new(); image.size_classes().classes.len()],
            },
            large: SharedLargeSpace {
                page_bytes: image.page_bytes(),
                entries: image
                    .entries()
                    .iter()
                    .map(|entry| -> HeapResult<SharedLargeEntry> {
                        Ok(SharedLargeEntry {
                            is_live: entry.is_live,
                            len: entry.len,
                            pages: entry.pages,
                            shape_id: ShapeId::from_raw(entry.shape_id)?,
                        })
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
            marks: MarkSet::default(),
            trace_queue: TraceQueue::default(),
            edge_buffer: Vec::new(),
            phase: SharedGcPhase::Idle,
            sweep_cursor: 0,
            cycle_freed_allocations: 0,
            cycle_freed_bytes: 0,
        }
        .with_rebuilt_small_availability())
    }

    /// Return one frozen shared managed-space root.
    pub fn image(&self) -> SharedManagedSpaceImage {
        SharedManagedSpaceImage::new(
            self.small.size_classes.clone(),
            self.small.span_bytes,
            self.small
                .spans
                .iter()
                .map(|span| SharedManagedSmallSpanImage {
                    size_class: span.size_class,
                    slot_count: span.slot_count,
                    occupied: span.occupied.clone(),
                    shape_ids: span
                        .shape_ids
                        .iter()
                        .map(|shape_id| shape_id.map(ShapeId::raw))
                        .collect::<Vec<_>>()
                        .into_boxed_slice(),
                    pages: span.pages,
                })
                .collect::<Vec<_>>()
                .into_boxed_slice(),
            self.large.page_bytes,
            self.large
                .entries
                .iter()
                .map(|entry| SharedManagedLargeEntryImage {
                    is_live: entry.is_live,
                    len: entry.len,
                    pages: entry.pages,
                    shape_id: entry.shape_id.raw(),
                })
                .collect::<Vec<_>>()
                .into_boxed_slice(),
            self.references.clone().into_boxed_slice(),
            self.shape_table.shapes().to_vec().into_boxed_slice(),
            self.free_reference_ids.clone().into_boxed_slice(),
            self.next_unused_reference_id,
            self.large.free_large_entry_ids.clone().into_boxed_slice(),
            self.large.next_unused_large_entry_id,
            self.usage.allocation_count(),
            self.usage.allocated_bytes(),
            self.gc_state.clone(),
        )
    }

    /// Return every arena page reachable from this live shared managed space.
    pub fn page_ids(&self) -> Vec<PageId> {
        let mut pages = Vec::new();

        // span pages
        for span in &self.small.spans {
            pages.extend(span.pages.page_ids());
        }

        // large-entry pages
        for entry in &self.large.entries {
            pages.extend(entry.pages.page_ids());
        }

        pages
    }

    /// Rebuild shared small-span availability after cloning or restore.
    fn with_rebuilt_small_availability(mut self) -> Self {
        for span_index in 0..self.small.spans.len() {
            let span = &mut self.small.spans[span_index];
            span.next_free_slot = find_next_free_slot(&span.occupied, 0, span.slot_count);

            if span.occupied_count == 0 || span.occupied_count >= span.slot_count {
                continue;
            }

            if let Some(class_index) = self.small.size_classes.class_index_for(span.size_class) {
                self.small.available_spans[class_index].push(span_index);
            }
        }

        self
    }
}

/// Return the next clear slot starting at the given cursor.
fn find_next_free_slot(occupied: &crate::Bitmap, start: usize, slot_count: usize) -> usize {
    for slot_index in start..slot_count {
        if !occupied.contains(slot_index) {
            return slot_index;
        }
    }

    slot_count
}
