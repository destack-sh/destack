use std::sync::Arc;

use super::{
    SharedLargeEntry, SharedLargeEntryId, SharedManagedLocation, SharedManagedReferenceEntry,
    SharedSmallSpan,
};
use crate::arena::{Arena, PageView, SizeClassTable, SpanSlot};
use crate::shared::gc::SharedGcPhase;
use crate::{
    AllocationTotals, EdgeMap, GcState, HeapDomain, HeapError, HeapOptions, HeapResult, LayoutId,
    SharedManagedReference, SharedManagedSpaceUsage,
};

/// The first allocated shared managed reference id.
const FIRST_SHARED_MANAGED_REFERENCE_ID: u64 = 1;

/// The first allocated shared managed large-entry id.
const FIRST_SHARED_MANAGED_LARGE_ENTRY_ID: u64 = 1;

/// One shared managed small space.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SharedSmallSpace {
    /// The configured size-class table.
    pub(crate) size_classes: SizeClassTable,
    /// The configured span width.
    pub(crate) span_bytes: usize,
    /// The live shared managed spans.
    pub(crate) spans: Vec<SharedSmallSpan>,
    /// The reusable non-full spans per size class.
    pub(crate) available_spans: Vec<Vec<usize>>,
}

/// One shared managed large space.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SharedLargeSpace {
    /// The configured page width for entries in large space.
    pub(crate) page_bytes: usize,
    /// The live shared managed entries.
    pub(crate) entries: Vec<SharedLargeEntry>,
    /// The free shared managed entry ids available for reuse.
    pub(crate) free_large_entry_ids: Vec<u64>,
    /// The next shared managed entry id to allocate.
    pub(crate) next_unused_large_entry_id: u64,
}

/// Convert a stable shared managed reference id into its packed representation.
fn checked_shared_managed_reference_id(reference_id: u64) -> HeapResult<u32> {
    if reference_id == 0 || reference_id > u32::MAX as u64 {
        return Err(HeapError::InvalidSharedManagedReferenceId { id: reference_id });
    }

    Ok(reference_id as u32)
}

/// One live shared managed-space store rooted in one arena.
#[derive(Debug)]
pub struct SharedManagedSpace {
    /// The shared managed-space arena for every entry.
    pub(crate) arena: Arc<Arena>,

    /// The shared managed small space.
    pub(crate) small: SharedSmallSpace,
    /// The shared managed large space.
    pub(crate) large: SharedLargeSpace,

    /// Stable shared managed reference metadata keyed by reference id minus one.
    pub(crate) references: Vec<SharedManagedReferenceEntry>,
    /// Free shared managed reference ids available for reuse.
    pub(crate) free_reference_ids: Vec<u64>,
    /// The next shared managed reference id to allocate.
    pub(crate) next_unused_reference_id: u64,

    /// The exact live shared managed-space totals.
    pub(crate) totals: AllocationTotals,
    /// The live shared managed collector state.
    pub(crate) gc_state: GcState,
    /// Reused mark bits for shared collection.
    pub(crate) marks: Vec<bool>,
    /// Reused collector trace queue for shared tracing.
    pub(crate) trace_queue: Vec<SharedManagedReference>,
    /// Reused edge scratch buffer for shared tracing.
    pub(crate) trace_buffer: Vec<SharedManagedReference>,
    /// The current shared collection phase.
    pub(crate) phase: SharedGcPhase,
    /// The next reference index to sweep.
    pub(crate) sweep_cursor: usize,
    /// The allocations freed so far in the active cycle.
    pub(crate) cycle_freed_allocations: usize,
    /// The bytes freed so far in the active cycle.
    pub(crate) cycle_freed_bytes: u64,
}

impl Default for SharedManagedSpace {
    fn default() -> Self {
        Self::new()
    }
}

impl SharedManagedSpace {
    /// Create a new empty shared managed-space store.
    pub fn new() -> Self {
        Self::with_arena(Arc::new(Arena::new()))
    }

    /// Create a new empty shared managed-space store over one shared arena.
    pub fn with_arena(arena: Arc<Arena>) -> Self {
        let options = HeapOptions::default();

        Self {
            arena: arena.clone(),
            small: SharedSmallSpace {
                size_classes: options.size_classes.clone(),
                span_bytes: options.managed_small_bytes,
                spans: Vec::new(),
                available_spans: vec![Vec::new(); options.size_classes.classes.len()],
            },
            large: SharedLargeSpace {
                page_bytes: arena.page_bytes(),
                entries: Vec::new(),
                free_large_entry_ids: Vec::new(),
                next_unused_large_entry_id: FIRST_SHARED_MANAGED_LARGE_ENTRY_ID,
            },
            references: Vec::new(),
            free_reference_ids: Vec::new(),
            next_unused_reference_id: FIRST_SHARED_MANAGED_REFERENCE_ID,
            totals: AllocationTotals::default(),
            gc_state: GcState::default(),
            marks: Vec::new(),
            trace_queue: Vec::new(),
            trace_buffer: Vec::new(),
            phase: SharedGcPhase::Idle,
            sweep_cursor: 0,
            cycle_freed_allocations: 0,
            cycle_freed_bytes: 0,
        }
    }

    /// Return the configured shared page width.
    pub fn page_bytes(&self) -> usize {
        self.arena.page_bytes()
    }

    /// Return the exact active shared managed-space bytes.
    pub fn active_bytes(&self) -> u64 {
        self.mapped_bytes()
    }

    /// Return the exact mapped shared page bytes.
    pub fn mapped_bytes(&self) -> u64 {
        self.arena.mapped_bytes_for_page_views(
            self.small
                .spans
                .iter()
                .map(|span| &span.pages)
                .chain(self.large.entries.iter().map(|entry| &entry.pages)),
        )
    }

    /// Return the exact borrowed shared image bytes.
    pub fn borrowed_bytes(&self) -> HeapResult<u64> {
        self.arena.borrowed_bytes_for_page_views(
            self.small
                .spans
                .iter()
                .map(|span| &span.pages)
                .chain(self.large.entries.iter().map(|entry| &entry.pages)),
        )
    }

    /// Return the exact usage for this live shared managed-space store.
    pub fn usage(&self) -> HeapResult<SharedManagedSpaceUsage> {
        Ok(SharedManagedSpaceUsage {
            allocation_count: self.totals.allocation_count(),
            allocated_bytes: self.totals.allocated_bytes(),
            active_bytes: self.active_bytes(),
            mapped_bytes: self.mapped_bytes(),
            borrowed_bytes: self.borrowed_bytes()?,
        })
    }

    /// Return the current shared managed collector state.
    pub fn gc_state(&self) -> &GcState {
        &self.gc_state
    }

    /// Return the current shared managed collector phase.
    pub fn phase(&self) -> SharedGcPhase {
        self.phase
    }

    /// Return whether one shared managed reference currently refers to one live entry.
    pub fn is_live(&self, reference: SharedManagedReference) -> bool {
        self.reference(reference).is_some()
    }

    /// Allocate one shared managed byte entry.
    pub fn allocate_bytes(
        &mut self,
        bytes: &[u8],
        edge_map: EdgeMap,
        layout_id: Option<LayoutId>,
    ) -> HeapResult<SharedManagedReference> {
        let byte_len = bytes.len();
        let reference_id = self.allocate_reference_id()?;
        let reference = SharedManagedReference::new(reference_id);

        // allocate the backing location before installing the live reference
        let location = self.allocate_location(byte_len, edge_map, layout_id, Some(bytes))?;

        self.set_reference(
            reference_id,
            SharedManagedReferenceEntry::new(location, byte_len),
        )?;
        self.totals.allocate(byte_len, HeapDomain::Shared)?;

        Ok(reference)
    }

    /// Allocate one zeroed shared managed-space entry.
    pub fn allocate_zeroed(
        &mut self,
        byte_len: usize,
        edge_map: EdgeMap,
        layout_id: Option<LayoutId>,
    ) -> HeapResult<SharedManagedReference> {
        let reference_id = self.allocate_reference_id()?;
        let reference = SharedManagedReference::new(reference_id);

        // allocate the zeroed backing location before installing the live reference
        let location = self.allocate_location(byte_len, edge_map, layout_id, None)?;

        self.set_reference(
            reference_id,
            SharedManagedReferenceEntry::new(location, byte_len),
        )?;
        self.totals.allocate(byte_len, HeapDomain::Shared)?;

        Ok(reference)
    }

    /// Return the projected mapped-byte delta for one shared managed-space entry.
    pub fn alloc_mapped_delta(&self, byte_len: usize) -> i64 {
        if let Some(class_index) = self.small.size_classes.class_index_for(byte_len) {
            if self.has_available_small_slot(class_index) {
                return 0;
            }

            return self.small.span_bytes as i64;
        }

        self.round_up_allocation_bytes(byte_len) as i64
    }

    /// Return the projected mapped-byte delta for one shared managed write.
    pub fn write_mapped_delta(
        &self,
        reference: SharedManagedReference,
        start: usize,
        byte_len: usize,
    ) -> HeapResult<i64> {
        let Some(record) = self.reference(reference) else {
            return Err(HeapError::InvalidSharedManagedReference { reference });
        };
        checked_byte_range(reference.slot_offset(), start, byte_len, record.byte_len())?;

        Ok(0)
    }

    /// Return the remaining byte length for one shared managed reference.
    pub fn byte_len(&self, reference: SharedManagedReference) -> HeapResult<usize> {
        let Some(record) = self.reference(reference) else {
            return Err(HeapError::InvalidSharedManagedReference { reference });
        };

        checked_remaining_byte_len(reference, record.byte_len())
    }

    /// Return the bytes for one shared managed reference.
    pub fn read_bytes(&self, reference: SharedManagedReference) -> HeapResult<Vec<u8>> {
        let Some(record) = self.reference(reference) else {
            return Err(HeapError::InvalidSharedManagedReference { reference });
        };
        let Some(location) = record.location() else {
            return Err(HeapError::InvalidSharedManagedReference { reference });
        };

        let byte_offset = reference.slot_offset();
        let byte_len = checked_remaining_byte_len(reference, record.byte_len())?;

        self.location_bytes(location, byte_offset, byte_len)
    }

    /// Fill one caller-provided buffer from one shared managed entry at one offset.
    pub fn read_bytes_into(
        &self,
        reference: SharedManagedReference,
        start: usize,
        target: &mut [u8],
    ) -> HeapResult<()> {
        let Some(record) = self.reference(reference) else {
            return Err(HeapError::InvalidSharedManagedReference { reference });
        };
        let Some(location) = record.location() else {
            return Err(HeapError::InvalidSharedManagedReference { reference });
        };

        let byte_offset = checked_byte_range(
            reference.slot_offset(),
            start,
            target.len(),
            record.byte_len(),
        )?;

        self.fill_location_bytes(location, byte_offset, target)
    }

    /// Return the traced edge map for one shared managed reference.
    pub fn edge_map(&self, reference: SharedManagedReference) -> HeapResult<&EdgeMap> {
        let Some(record) = self.reference(reference) else {
            return Err(HeapError::InvalidSharedManagedReference { reference });
        };
        let Some(location) = record.location() else {
            return Err(HeapError::InvalidSharedManagedReference { reference });
        };

        self.location_edge_map(location)
    }

    /// Return the storage layout id for one shared managed reference.
    pub fn layout_id(&self, reference: SharedManagedReference) -> HeapResult<Option<LayoutId>> {
        let Some(record) = self.reference(reference) else {
            return Err(HeapError::InvalidSharedManagedReference { reference });
        };
        let Some(location) = record.location() else {
            return Err(HeapError::InvalidSharedManagedReference { reference });
        };

        self.location_layout_id(location)
    }

    /// Set the storage layout id for one shared managed reference.
    pub fn set_layout_id(
        &mut self,
        reference: SharedManagedReference,
        layout_id: LayoutId,
    ) -> HeapResult<()> {
        let Some(record) = self.reference(reference).copied() else {
            return Err(HeapError::InvalidSharedManagedReference { reference });
        };
        let Some(location) = record.location() else {
            return Err(HeapError::InvalidSharedManagedReference { reference });
        };

        self.set_location_layout_id(location, layout_id)
    }

    /// Overwrite one shared managed byte range.
    pub fn write_bytes(
        &mut self,
        reference: SharedManagedReference,
        start: usize,
        bytes: &[u8],
    ) -> HeapResult<()> {
        let Some(record) = self.reference(reference).copied() else {
            return Err(HeapError::InvalidSharedManagedReference { reference });
        };
        let Some(location) = record.location() else {
            return Err(HeapError::InvalidSharedManagedReference { reference });
        };

        let byte_offset = checked_byte_range(
            reference.slot_offset(),
            start,
            bytes.len(),
            record.byte_len(),
        )?;

        self.write_location_bytes(location, byte_offset, bytes)
    }

    /// Record one shared managed write barrier.
    pub fn write_barrier(
        &mut self,
        reference: SharedManagedReference,
        start: usize,
        byte_len: usize,
    ) -> HeapResult<()> {
        let Some(record) = self.reference(reference) else {
            return Err(HeapError::InvalidSharedManagedReference { reference });
        };

        let byte_offset =
            checked_byte_range(reference.slot_offset(), start, byte_len, record.byte_len())?;

        self.write_shared_barrier(reference, byte_offset, byte_len)
    }

    /// Return one allocated shared managed entry by reference.
    pub(crate) fn reference(
        &self,
        reference: SharedManagedReference,
    ) -> Option<&SharedManagedReferenceEntry> {
        let index = reference.id().checked_sub(1)? as usize;
        let record = self.references.get(index)?;

        (!record.is_vacant()).then_some(record)
    }

    /// Return one shared managed span by index.
    pub(crate) fn span(&self, span_index: usize) -> Option<&SharedSmallSpan> {
        self.small.spans.get(span_index)
    }

    /// Return one shared managed span mutably by index.
    pub(crate) fn span_mut(&mut self, span_index: usize) -> Option<&mut SharedSmallSpan> {
        self.small.spans.get_mut(span_index)
    }

    /// Return one shared managed large entry by stable id.
    pub(crate) fn large_entry(&self, entry_id: SharedLargeEntryId) -> Option<&SharedLargeEntry> {
        let index = entry_id.index().ok()?;
        let entry = self.large.entries.get(index)?;

        (entry.is_live).then_some(entry)
    }

    /// Return one shared managed large entry mutably by stable id.
    pub(crate) fn large_entry_mut(
        &mut self,
        entry_id: SharedLargeEntryId,
    ) -> Option<&mut SharedLargeEntry> {
        let index = entry_id.index().ok()?;
        let entry = self.large.entries.get_mut(index)?;

        (entry.is_live).then_some(entry)
    }

    /// Allocate one shared managed location for the given payload.
    fn allocate_location(
        &mut self,
        byte_len: usize,
        edge_map: EdgeMap,
        layout_id: Option<LayoutId>,
        bytes: Option<&[u8]>,
    ) -> HeapResult<SharedManagedLocation> {
        if let Some(slot) = self.allocate_small(byte_len, edge_map.clone(), layout_id, bytes)? {
            return Ok(SharedManagedLocation::Small(slot));
        }

        let pages = match bytes {
            Some(bytes) => self.arena.allocate_bytes(bytes)?,
            None => self.arena.allocate_zeroed(byte_len)?,
        };
        let entry_id = self.store_large_entry(byte_len, pages, edge_map, layout_id)?;

        Ok(SharedManagedLocation::Large(entry_id))
    }

    /// Allocate one shared managed reference id from the intrusive free list or the unused tail.
    fn allocate_reference_id(&mut self) -> HeapResult<u32> {
        if let Some(reference_id) = self.free_reference_ids.pop() {
            checked_shared_managed_reference_id(reference_id)
        } else {
            let reference_id = self.next_unused_reference_id;
            let reference_id = checked_shared_managed_reference_id(reference_id)?;
            self.next_unused_reference_id = self.next_unused_reference_id.checked_add(1).ok_or(
                HeapError::InvalidSharedManagedReferenceId {
                    id: self.next_unused_reference_id,
                },
            )?;

            Ok(reference_id)
        }
    }

    /// Store one dense shared managed reference record by stable id.
    fn set_reference(
        &mut self,
        reference_id: u32,
        record: SharedManagedReferenceEntry,
    ) -> HeapResult<()> {
        let index = shared_reference_index(reference_id)?;

        if index > self.references.len() {
            return Err(HeapError::InvalidSharedManagedReferenceId {
                id: reference_id.into(),
            });
        }

        if index == self.references.len() {
            self.references.push(record);
        } else {
            self.references[index] = record;
        }

        Ok(())
    }

    /// Retire one shared managed reference record and queue its id for reuse.
    pub(crate) fn retire_reference(&mut self, reference_id: u32) -> HeapResult<()> {
        let index = shared_reference_index(reference_id)?;
        let Some(record) = self.references.get_mut(index) else {
            return Err(HeapError::InvalidSharedManagedReferenceId {
                id: reference_id.into(),
            });
        };

        *record = SharedManagedReferenceEntry::vacant();
        self.free_reference_ids.push(reference_id.into());

        Ok(())
    }

    /// Return whether one size class still has one live reusable slot.
    fn has_available_small_slot(&self, class_index: usize) -> bool {
        self.small.available_spans[class_index]
            .iter()
            .copied()
            .any(|span_index| {
                self.small
                    .spans
                    .get(span_index)
                    .map(|span| span.occupied_count < span.slot_count)
                    .unwrap_or(false)
            })
    }

    /// Allocate or reuse one non-full shared managed span for the given size class.
    fn allocate_small_span(&mut self, class_index: usize, size_class: usize) -> HeapResult<usize> {
        while let Some(span_index) = self.small.available_spans[class_index].pop() {
            let Some(span) = self.small.spans.get(span_index) else {
                return Err(HeapError::MissingSpan { span_index });
            };

            if span.occupied_count < span.slot_count {
                if span.occupied_count == 0 && span.pages.is_empty() {
                    let pages = self.arena.allocate_zeroed(self.small.span_bytes)?;
                    let Some(span) = self.small.spans.get_mut(span_index) else {
                        self.arena.release_page_view(&pages)?;

                        return Err(HeapError::MissingSpan { span_index });
                    };

                    span.pages = pages;
                }

                return Ok(span_index);
            }
        }

        let slot_count = (self.small.span_bytes / size_class).max(1);
        let pages = self.arena.allocate_zeroed(self.small.span_bytes)?;
        let span = SharedSmallSpan {
            size_class,
            slot_count,
            occupied_count: 0,
            next_free_slot: 0,
            occupied: crate::Bitmap::with_capacity(slot_count),
            edge_maps: vec![EdgeMap::empty(); slot_count].into_boxed_slice(),
            layout_ids: vec![None; slot_count].into_boxed_slice(),
            pages,
        };
        let span_index = self.small.spans.len();
        self.small.spans.push(span);

        Ok(span_index)
    }

    /// Allocate one shared managed small slot from one explicit initialization source.
    fn allocate_small(
        &mut self,
        byte_len: usize,
        edge_map: EdgeMap,
        layout_id: Option<LayoutId>,
        bytes: Option<&[u8]>,
    ) -> HeapResult<Option<SpanSlot>> {
        let Some(class_index) = self.small.size_classes.class_index_for(byte_len) else {
            return Ok(None);
        };
        let size_class = self.small.size_classes.classes[class_index].bytes;
        let span_index = self.allocate_small_span(class_index, size_class)?;
        let Some(span) = self.small.spans.get(span_index) else {
            return Err(HeapError::MissingSpan { span_index });
        };
        let slot_index = span.next_free_slot;

        // slot initialization
        let slot = SpanSlot::new(span_index, slot_index)?;
        let slot_offset = checked_slot_offset(span.size_class, slot_index)?;

        if let Some(bytes) = bytes {
            let arena = self.arena.clone();
            let Some(span) = self.small.spans.get_mut(span_index) else {
                return Err(HeapError::MissingSpan { span_index });
            };

            arena.set_bytes(&mut span.pages, slot_offset, bytes)?;
        }

        // slot metadata
        let Some(span) = self.small.spans.get_mut(span_index) else {
            return Err(HeapError::MissingSpan { span_index });
        };

        span.occupied.set(slot_index);
        span.occupied_count += 1;
        span.next_free_slot = find_next_free_slot(&span.occupied, slot_index + 1, span.slot_count);
        span.set_edge_map(slot_index, edge_map);
        span.set_layout_id(slot_index, layout_id);

        if span.occupied_count < span.slot_count {
            self.small.available_spans[class_index].push(span_index);
        }

        Ok(Some(slot))
    }

    /// Store one large entry in shared managed large space and return its stable id.
    fn store_large_entry(
        &mut self,
        len: usize,
        pages: PageView,
        edge_map: EdgeMap,
        layout_id: Option<LayoutId>,
    ) -> HeapResult<SharedLargeEntryId> {
        let (entry_id, reused_entry_id) = if let Some(entry_id) =
            self.large.free_large_entry_ids.pop()
        {
            (entry_id, true)
        } else {
            let entry_id = self.large.next_unused_large_entry_id;
            let Some(next_entry_id) = self.large.next_unused_large_entry_id.checked_add(1) else {
                self.arena.release_page_view(&pages)?;

                return Err(HeapError::InvalidLargeEntryId { id: entry_id });
            };

            self.large.next_unused_large_entry_id = next_entry_id;
            (entry_id, false)
        };

        if entry_id == 0 {
            if reused_entry_id {
                self.large.free_large_entry_ids.push(entry_id);
            }

            self.arena.release_page_view(&pages)?;

            return Err(HeapError::InvalidLargeEntryId { id: entry_id });
        }

        let entry = SharedLargeEntry {
            is_live: true,
            len,
            pages,
            edge_map,
            layout_id,
        };
        let index = SharedLargeEntryId::new(entry_id).index()?;

        if index > self.large.entries.len() {
            if reused_entry_id {
                self.large.free_large_entry_ids.push(entry_id);
            }

            self.arena.release_page_view(&pages)?;

            return Err(HeapError::InvalidLargeEntryId { id: entry_id });
        }

        if index == self.large.entries.len() {
            self.large.entries.push(entry);
        } else {
            self.large.entries[index] = entry;
        }

        Ok(SharedLargeEntryId::new(entry_id))
    }

    /// Release one shared managed small slot without retiring its stable reference id.
    pub(crate) fn release_small_slot(&mut self, slot: SpanSlot) -> HeapResult<()> {
        let pages = {
            let Some(span) = self.span_mut(slot.span_index()) else {
                return Err(HeapError::MissingSpan {
                    span_index: slot.span_index(),
                });
            };

            let slot_index = slot.slot_index();
            let was_full = span.occupied_count == span.slot_count;
            let size_class = span.size_class;

            if !span.occupied.contains(slot_index) || span.occupied_count == 0 {
                return Err(HeapError::MissingSmallSlot {
                    span_index: slot.span_index(),
                    slot_index,
                });
            }

            span.occupied.clear(slot_index);
            span.occupied_count -= 1;
            span.next_free_slot = span.next_free_slot.min(slot_index);
            span.set_edge_map(slot_index, EdgeMap::empty());
            span.set_layout_id(slot_index, None);

            if span.occupied_count == 0 {
                span.next_free_slot = 0;

                let pages = span.pages;
                span.pages = PageView::empty();

                Some(pages)
            } else {
                let should_requeue = was_full && span.occupied_count < span.slot_count;

                if should_requeue
                    && let Some(class_index) = self.small.size_classes.class_index_for(size_class)
                {
                    self.small.available_spans[class_index].push(slot.span_index());
                }

                None
            }
        };

        if let Some(pages) = pages {
            self.arena.release_page_view(&pages)?;
        }

        Ok(())
    }

    /// Return the traced edge map for one shared managed location.
    fn location_edge_map(&self, location: SharedManagedLocation) -> HeapResult<&EdgeMap> {
        match location {
            SharedManagedLocation::Small(slot) => {
                let Some(span) = self.span(slot.span_index()) else {
                    return Err(HeapError::MissingSpan {
                        span_index: slot.span_index(),
                    });
                };
                let Some(edge_map) = span.edge_map(slot.slot_index()) else {
                    return Err(HeapError::MissingSmallSlot {
                        span_index: slot.span_index(),
                        slot_index: slot.slot_index(),
                    });
                };

                Ok(edge_map)
            }
            SharedManagedLocation::Large(entry_id) => {
                let Some(entry) = self.large_entry(entry_id) else {
                    return Err(HeapError::MissingLargeEntry {
                        entry_id: entry_id.id(),
                    });
                };

                Ok(&entry.edge_map)
            }
        }
    }

    /// Return the storage layout id for one shared managed location.
    fn location_layout_id(&self, location: SharedManagedLocation) -> HeapResult<Option<LayoutId>> {
        match location {
            SharedManagedLocation::Small(slot) => {
                let Some(span) = self.span(slot.span_index()) else {
                    return Err(HeapError::MissingSpan {
                        span_index: slot.span_index(),
                    });
                };

                Ok(span.layout_id(slot.slot_index()))
            }
            SharedManagedLocation::Large(entry_id) => {
                let Some(entry) = self.large_entry(entry_id) else {
                    return Err(HeapError::MissingLargeEntry {
                        entry_id: entry_id.id(),
                    });
                };

                Ok(entry.layout_id)
            }
        }
    }

    /// Set the storage layout id for one shared managed location.
    fn set_location_layout_id(
        &mut self,
        location: SharedManagedLocation,
        layout_id: LayoutId,
    ) -> HeapResult<()> {
        match location {
            SharedManagedLocation::Small(slot) => {
                let Some(span) = self.span_mut(slot.span_index()) else {
                    return Err(HeapError::MissingSpan {
                        span_index: slot.span_index(),
                    });
                };

                span.set_layout_id(slot.slot_index(), Some(layout_id));

                Ok(())
            }
            SharedManagedLocation::Large(entry_id) => {
                let Some(entry) = self.large_entry_mut(entry_id) else {
                    return Err(HeapError::MissingLargeEntry {
                        entry_id: entry_id.id(),
                    });
                };

                entry.layout_id = Some(layout_id);

                Ok(())
            }
        }
    }

    /// Return the bytes for one shared managed location as one owned vector.
    fn location_bytes(
        &self,
        location: SharedManagedLocation,
        byte_offset: usize,
        byte_len: usize,
    ) -> HeapResult<Vec<u8>> {
        match location {
            SharedManagedLocation::Small(slot) => {
                let Some(span) = self.span(slot.span_index()) else {
                    return Err(HeapError::MissingSpan {
                        span_index: slot.span_index(),
                    });
                };

                let slot_offset = checked_slot_offset(span.size_class, slot.slot_index())?;
                let read_offset =
                    checked_storage_offset(slot_offset, byte_offset, self.small.span_bytes)?;

                self.arena
                    .bytes_to_vec_from(&span.pages, read_offset, byte_len)
            }
            SharedManagedLocation::Large(entry_id) => {
                let Some(entry) = self.large_entry(entry_id) else {
                    return Err(HeapError::MissingLargeEntry {
                        entry_id: entry_id.id(),
                    });
                };

                self.arena
                    .bytes_to_vec_from(&entry.pages, byte_offset, byte_len)
            }
        }
    }

    /// Fill one caller-provided buffer from one shared managed location.
    fn fill_location_bytes(
        &self,
        location: SharedManagedLocation,
        byte_offset: usize,
        target: &mut [u8],
    ) -> HeapResult<()> {
        match location {
            SharedManagedLocation::Small(slot) => {
                let Some(span) = self.span(slot.span_index()) else {
                    return Err(HeapError::MissingSpan {
                        span_index: slot.span_index(),
                    });
                };

                let slot_offset = checked_slot_offset(span.size_class, slot.slot_index())?;
                let read_offset =
                    checked_storage_offset(slot_offset, byte_offset, self.small.span_bytes)?;

                self.arena.fill_bytes_from(&span.pages, read_offset, target)
            }
            SharedManagedLocation::Large(entry_id) => {
                let Some(entry) = self.large_entry(entry_id) else {
                    return Err(HeapError::MissingLargeEntry {
                        entry_id: entry_id.id(),
                    });
                };

                self.arena
                    .fill_bytes_from(&entry.pages, byte_offset, target)
            }
        }
    }

    /// Overwrite one byte range for one shared managed location.
    fn write_location_bytes(
        &mut self,
        location: SharedManagedLocation,
        byte_offset: usize,
        bytes: &[u8],
    ) -> HeapResult<()> {
        match location {
            SharedManagedLocation::Small(slot) => {
                let arena = self.arena.clone();
                let span_bytes = self.small.span_bytes;
                let Some(span) = self.span_mut(slot.span_index()) else {
                    return Err(HeapError::MissingSpan {
                        span_index: slot.span_index(),
                    });
                };

                let slot_offset = checked_slot_offset(span.size_class, slot.slot_index())?;
                let write_offset = checked_storage_offset(slot_offset, byte_offset, span_bytes)?;

                arena.set_bytes(&mut span.pages, write_offset, bytes)
            }
            SharedManagedLocation::Large(entry_id) => {
                let arena = self.arena.clone();
                let Some(entry) = self.large_entry_mut(entry_id) else {
                    return Err(HeapError::MissingLargeEntry {
                        entry_id: entry_id.id(),
                    });
                };

                arena.set_bytes(&mut entry.pages, byte_offset, bytes)
            }
        }
    }

    /// Return the page-rounded mapped bytes for one shared managed-space entry.
    fn round_up_allocation_bytes(&self, byte_len: usize) -> u64 {
        let page_bytes = self.page_bytes() as u64;
        let byte_len = byte_len as u64;

        byte_len.div_ceil(page_bytes) * page_bytes
    }
}

/// Return the dense shared managed reference table index for one stable id.
fn shared_reference_index(reference_id: u32) -> HeapResult<usize> {
    let Some(index) = reference_id.checked_sub(1) else {
        return Err(HeapError::InvalidSharedManagedReferenceId {
            id: reference_id.into(),
        });
    };

    Ok(index as usize)
}

/// Return the visible byte length for one shared managed reference.
fn checked_remaining_byte_len(
    reference: SharedManagedReference,
    byte_len: usize,
) -> HeapResult<usize> {
    let byte_offset = reference.slot_offset();

    if byte_offset > byte_len {
        return Err(HeapError::InvalidSharedManagedReference { reference });
    }

    Ok(byte_len - byte_offset)
}

/// Validate one byte range inside one shared managed entry.
fn checked_byte_range(
    pointer_offset: usize,
    start: usize,
    len: usize,
    capacity: usize,
) -> HeapResult<usize> {
    let byte_offset = pointer_offset
        .checked_add(start)
        .ok_or(HeapError::InvalidByteRange {
            start,
            len,
            capacity,
        })?;
    let byte_end = byte_offset
        .checked_add(len)
        .ok_or(HeapError::InvalidByteRange {
            start,
            len,
            capacity,
        })?;

    if byte_end > capacity {
        return Err(HeapError::InvalidByteRange {
            start: byte_offset,
            len,
            capacity,
        });
    }

    Ok(byte_offset)
}

/// Return the byte offset for one slot payload inside one span.
fn checked_slot_offset(size_class: usize, slot_index: usize) -> HeapResult<usize> {
    size_class
        .checked_mul(slot_index)
        .ok_or(HeapError::InvariantOverflow {
            context: "shared small-slot byte offset",
        })
}

/// Return one nested storage offset inside one bounded page view.
fn checked_storage_offset(base: usize, byte_offset: usize, capacity: usize) -> HeapResult<usize> {
    let storage_offset = base
        .checked_add(byte_offset)
        .ok_or(HeapError::InvalidByteRange {
            start: base,
            len: byte_offset,
            capacity,
        })?;

    if storage_offset > capacity {
        return Err(HeapError::InvalidByteRange {
            start: storage_offset,
            len: 0,
            capacity,
        });
    }

    Ok(storage_offset)
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
