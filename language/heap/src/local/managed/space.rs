use std::sync::Arc;

use super::{
    GcState, LargeEntry, LargeEntryId, ManagedLocation, ManagedReferenceEntry, ManagedYoungId,
    PinSet, SmallSpan, YoungEntry, YoungSpace,
};
use crate::arena::{Arena, PageRunCache, PageView, SizeClassTable};
use crate::{
    AllocationUsage, CowTable, HeapError, HeapOptions, HeapResult, ManagedReference,
    ManagedSpaceUsage, MarkSet, ShapeId, ShapeTable, SharedManagedReference, TraceQueue,
    overlaps_managed_range, overlaps_shared_range,
};

/// Collector marks for local managed references.
type ManagedMarkSet = MarkSet<ManagedReference>;

/// Collector queue for local managed references.
type ManagedTraceQueue = TraceQueue<ManagedReference>;

/// The first non-null managed reference id.
pub(crate) const FIRST_ALLOCATED_REFERENCE_ID: u64 = 1;

/// The first non-null managed large-entry id.
const FIRST_ALLOCATED_LARGE_ENTRY_ID: u64 = 1;

/// Convert a stable managed reference id into its packed representation.
pub(crate) fn checked_reference_id(reference_id: u64) -> HeapResult<u32> {
    if reference_id == 0 || reference_id > u32::MAX as u64 {
        return Err(HeapError::InvalidManagedReferenceId { id: reference_id });
    }

    Ok(reference_id as u32)
}

/// One managed small space.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SmallSpace {
    /// The configured size-class table.
    pub(crate) size_classes: SizeClassTable,
    /// The configured span width.
    pub(crate) span_bytes: usize,
    /// The live managed spans.
    pub(crate) spans: CowTable<SmallSpan>,
    /// The reusable non-full spans per size class.
    pub(crate) available_spans: Vec<Vec<usize>>,
}

/// One managed large space.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LargeSpace {
    /// The configured page width for entries in large space.
    pub(crate) page_bytes: usize,
    /// The live managed entries.
    pub(crate) entries: CowTable<LargeEntry>,
    /// The free managed entry ids available for reuse.
    pub(crate) free_large_entry_ids: Vec<u64>,
    /// The next managed entry id to allocate.
    pub(crate) next_unused_large_entry_id: u64,
}

/// One live managed entry space rooted in one arena.
#[derive(Debug)]
pub struct ManagedSpace {
    /// The shared page arena for every managed payload.
    pub(super) arena: Arc<Arena>,
    /// The local front-end cache of reusable page runs.
    pub(crate) page_run_cache: PageRunCache,

    /// The managed young space.
    pub(crate) young: YoungSpace,
    /// The managed small space.
    pub(crate) small: SmallSpace,
    /// The managed large space.
    pub(crate) large: LargeSpace,

    /// Dense managed reference metadata keyed by reference id minus one.
    pub(crate) references: CowTable<ManagedReferenceEntry>,
    /// The interned managed entry shapes.
    pub(crate) shape_table: ShapeTable,

    /// The byte width for managed references inside traced payloads.
    pub(crate) managed_reference_bytes: usize,
    /// The maximum payload size admitted into young space.
    pub(crate) max_young_allocation_bytes: usize,
    /// The byte width for one remembered card.
    pub(crate) card_bytes: usize,
    /// The entry count per metadata table chunk.
    pub(crate) table_chunk_len: usize,

    /// The free managed reference ids available for reuse.
    pub(crate) free_reference_ids: Vec<u64>,
    /// The next managed reference id to allocate.
    pub(crate) next_unused_reference_id: u64,
    /// The exact live managed usage.
    pub(crate) usage: AllocationUsage,

    /// The live GC state.
    pub(crate) gc_state: GcState,
    /// The reusable collector mark set.
    pub(crate) marks: ManagedMarkSet,
    /// The reusable collector trace queue.
    pub(crate) trace_queue: ManagedTraceQueue,
    /// Whether a managed collection is currently running.
    pub(crate) is_collecting: bool,
    /// The scoped managed pins that block branch boundaries and movement.
    pub(crate) pins: PinSet,
    /// Mature spans queued for dirty-card scanning.
    pub(crate) dirty_spans: Vec<usize>,
    /// Mature large entries queued for dirty-card scanning.
    pub(crate) dirty_large_entries: Vec<LargeEntryId>,
    /// Whether one local-to-shared root scan is currently active.
    pub(crate) is_scanning_shared_roots: bool,
    /// The next dense reference slot to scan for shared roots.
    pub(crate) shared_root_cursor: usize,
    /// The pending local references whose shared edges need rescanning.
    pub(crate) shared_root_queue: ManagedTraceQueue,
    /// Queue-membership bits for pending shared rescans.
    pub(crate) shared_root_pending: Vec<bool>,
}

impl ManagedSpace {
    /// Create one managed space with the default options.
    pub fn new() -> Result<Self, HeapError> {
        let options = HeapOptions::local();
        let arena = Arc::new(Arena::try_new(
            options.page_bytes,
            options.arena_segment_bytes,
        )?);

        Self::with_options(arena, &options)
    }

    /// Create one managed space with explicit options.
    pub fn with_options(arena: Arc<Arena>, options: &HeapOptions) -> Result<Self, HeapError> {
        options.validate()?;
        options.validate_arena(&arena)?;

        Self::build_with_options(arena, options)
    }

    /// Create one managed space from one checked options set.
    pub(crate) fn build_with_options(
        arena: Arc<Arena>,
        options: &HeapOptions,
    ) -> Result<Self, HeapError> {
        let managed_reference_bytes =
            HeapOptions::validate_managed_reference_bytes(options.managed_reference_bytes)?;
        let mut page_run_cache = PageRunCache::new(arena.pages_per_segment());

        // reserve one fixed young-space page run up front
        let young = YoungSpace::new(
            &arena,
            options.managed_young_bytes,
            options.page_bytes,
            &mut page_run_cache,
        )?;
        let max_young_allocation_bytes = if options.managed_young_bytes == 0 {
            0
        } else {
            options.max_managed_young_allocation_bytes
        };

        // build the live root over the shared arena
        Ok(Self {
            arena,
            page_run_cache,
            managed_reference_bytes,
            max_young_allocation_bytes,
            card_bytes: options.card_bytes,
            table_chunk_len: options.table_chunk_len,
            shape_table: ShapeTable::new(),
            young,
            small: SmallSpace {
                size_classes: options.size_classes.clone(),
                span_bytes: options.managed_small_bytes,
                spans: CowTable::with_chunk_len(options.table_chunk_len)?,
                available_spans: vec![Vec::new(); options.size_classes.classes.len()],
            },
            large: LargeSpace {
                page_bytes: options.page_bytes,
                entries: CowTable::with_chunk_len(options.table_chunk_len)?,
                free_large_entry_ids: Vec::new(),
                next_unused_large_entry_id: FIRST_ALLOCATED_LARGE_ENTRY_ID,
            },
            references: CowTable::with_chunk_len(options.table_chunk_len)?,
            free_reference_ids: Vec::new(),
            next_unused_reference_id: FIRST_ALLOCATED_REFERENCE_ID,
            usage: AllocationUsage::default(),
            gc_state: GcState::default(),
            marks: MarkSet::default(),
            trace_queue: TraceQueue::default(),
            is_collecting: false,
            pins: PinSet::default(),
            dirty_spans: Vec::new(),
            dirty_large_entries: Vec::new(),
            is_scanning_shared_roots: false,
            shared_root_cursor: 0,
            shared_root_queue: TraceQueue::default(),
            shared_root_pending: Vec::new(),
        })
    }

    /// Return the shared page arena.
    pub fn arena(&self) -> &Arc<Arena> {
        &self.arena
    }

    /// Return the exact retained managed bytes.
    pub fn active_bytes(&self) -> u64 {
        self.mapped_bytes()
    }

    /// Return the exact mapped managed page bytes.
    pub fn mapped_bytes(&self) -> u64 {
        self.arena.mapped_bytes_for_page_views(
            std::iter::once(&self.young.pages)
                .chain(self.small.spans.iter().map(|span| &span.pages))
                .chain(self.large.entries.iter().map(|entry| &entry.pages)),
        ) + self.page_run_cache.cached_bytes(self.arena.page_bytes())
    }

    /// Return the exact borrowed managed image bytes.
    pub fn borrowed_bytes(&self) -> HeapResult<u64> {
        self.arena.borrowed_bytes_for_page_views(
            std::iter::once(&self.young.pages)
                .chain(self.small.spans.iter().map(|span| &span.pages))
                .chain(self.large.entries.iter().map(|entry| &entry.pages)),
        )
    }

    /// Return the current GC state.
    pub fn gc_state(&self) -> &GcState {
        &self.gc_state
    }

    /// Pin one local managed reference against movement.
    pub fn pin(&mut self, reference: ManagedReference) -> HeapResult<()> {
        self.pins.pin(reference)
    }

    /// Release one local managed pin.
    pub fn unpin(&mut self, reference: ManagedReference) -> HeapResult<()> {
        self.pins.unpin(reference)
    }

    /// Return the number of live managed entries.
    pub fn allocation_count(&self) -> usize {
        self.usage.allocation_count()
    }

    /// Return the number of live managed bytes.
    pub fn allocated_bytes(&self) -> u64 {
        self.usage.allocated_bytes()
    }

    /// Return the exact live usage for this managed space.
    pub fn usage(&self) -> HeapResult<ManagedSpaceUsage> {
        Ok(ManagedSpaceUsage {
            allocation_count: self.usage.allocation_count(),
            allocated_bytes: self.usage.allocated_bytes(),
            active_bytes: self.active_bytes(),
            mapped_bytes: self.mapped_bytes(),
            borrowed_bytes: self.borrowed_bytes()?,
        })
    }

    /// Flush the local page-run cache back into the arena page-run pool.
    pub(crate) fn flush_page_run_cache(&mut self) {
        self.page_run_cache.flush(&self.arena);
    }

    /// Flush transient cache state before one exact branch boundary.
    pub(crate) fn flush_branch_boundary(&mut self) {
        self.flush_page_run_cache();
    }

    /// Allocate one zeroed page view through the local page-run cache.
    pub(crate) fn allocate_page_view_zeroed(&mut self, byte_len: usize) -> HeapResult<PageView> {
        self.page_run_cache.allocate_zeroed(&self.arena, byte_len)
    }

    /// Allocate one initialized page view through the local page-run cache.
    pub(crate) fn allocate_page_view_bytes(&mut self, bytes: &[u8]) -> HeapResult<PageView> {
        self.page_run_cache.allocate_bytes(&self.arena, bytes)
    }

    /// Release one page view through the local page-run cache.
    pub(crate) fn release_page_view(&mut self, page_view: PageView) -> HeapResult<()> {
        self.page_run_cache
            .release_page_view(&self.arena, page_view)
    }

    /// Return one live reference record by managed reference.
    pub(crate) fn reference(&self, reference: ManagedReference) -> Option<&ManagedReferenceEntry> {
        // resolve the dense table slot first
        let index = reference.id().checked_sub(1)? as usize;
        let record = self.references.get(index)?;

        // skip vacant records
        (!record.is_vacant()).then_some(record)
    }

    /// Return one live reference record mutably by managed reference.
    pub(crate) fn reference_mut(
        &mut self,
        reference: ManagedReference,
    ) -> Option<&mut ManagedReferenceEntry> {
        // resolve the dense table slot first
        let index = reference.id().checked_sub(1)? as usize;
        let record = self.references.get_mut(index)?;

        // skip vacant records
        (!record.is_vacant()).then_some(record)
    }

    /// Return the dense table index for one managed reference id.
    pub(crate) fn reference_index(reference_id: u32) -> HeapResult<usize> {
        let Some(index) = reference_id.checked_sub(1) else {
            return Err(HeapError::InvalidManagedReferenceId {
                id: reference_id.into(),
            });
        };

        Ok(index as usize)
    }

    /// Store one dense managed reference record by stable reference id.
    pub(crate) fn set_reference_entry(
        &mut self,
        reference_id: u32,
        record: ManagedReferenceEntry,
    ) -> HeapResult<()> {
        self.references
            .set_or_push(Self::reference_index(reference_id)?, record)
    }

    /// Retire one stable managed reference slot.
    pub(crate) fn retire_reference(&mut self, reference_id: u32) -> HeapResult<()> {
        self.references.set(
            Self::reference_index(reference_id)?,
            ManagedReferenceEntry::vacant(),
        )?;
        self.free_reference_ids.push(reference_id.into());

        Ok(())
    }

    /// Return one live large entry by id.
    pub(crate) fn large_entry(&self, entry_id: LargeEntryId) -> Option<&LargeEntry> {
        // resolve the dense table slot first
        let index = entry_id.index().ok()?;
        let entry = self.large.entries.get(index)?;

        // skip free entries
        entry.is_live.then_some(entry)
    }

    /// Return one live large entry mutably by id.
    pub(crate) fn large_entry_mut(&mut self, entry_id: LargeEntryId) -> Option<&mut LargeEntry> {
        // resolve the dense table slot first
        let index = entry_id.index().ok()?;
        let entry = self.large.entries.get_mut(index)?;

        // skip free entries
        entry.is_live.then_some(entry)
    }

    /// Return one live young entry by id.
    pub(crate) fn young_entry(&self, young_id: ManagedYoungId) -> Option<&YoungEntry> {
        // reject stale generation ids first
        self.check_young_generation(young_id)?;

        let entry = self.young.entries.get(young_id.index() as usize)?;

        // skip free young-entry slots
        entry.is_live.then_some(entry)
    }

    /// Return one live young entry mutably by id.
    pub(crate) fn young_entry_mut(&mut self, young_id: ManagedYoungId) -> Option<&mut YoungEntry> {
        // reject stale generation ids first
        self.check_young_generation(young_id)?;

        let entry = self.young.entries.get_mut(young_id.index() as usize)?;

        // skip free young-entry slots
        entry.is_live.then_some(entry)
    }

    /// Return one live managed span by index.
    pub(crate) fn span(&self, span_index: usize) -> Option<&SmallSpan> {
        self.small.spans.get(span_index)
    }

    /// Return one live managed span mutably by index.
    pub(crate) fn span_mut(&mut self, span_index: usize) -> Option<&mut SmallSpan> {
        self.small.spans.get_mut(span_index)
    }

    /// Return the byte offset for one young entry.
    pub(crate) fn young_entry_offset(&self, entry: &YoungEntry) -> usize {
        entry.first_page as usize * self.young.page_bytes + entry.first_offset as usize
    }

    /// Check that one young id belongs to the current young-space generation.
    fn check_young_generation(&self, young_id: ManagedYoungId) -> Option<()> {
        (young_id.generation() == self.young.generation).then_some(())
    }

    /// Return the shape id for one managed location.
    pub(crate) fn location_shape_id(&self, location: ManagedLocation) -> Option<ShapeId> {
        match location {
            ManagedLocation::Young(young_id) => Some(self.young_entry(young_id)?.shape_id),
            ManagedLocation::Small(slot) => {
                let span = self.span(slot.span_index())?;
                let slot_index = slot.slot_index();

                span.shape_ids.get(slot_index).copied().flatten()
            }
            ManagedLocation::Large(entry_id) => Some(self.large_entry(entry_id)?.shape_id),
        }
    }

    /// Rebuild the mature remembered set conservatively.
    pub(crate) fn rebuild_remembered_set(&mut self) -> HeapResult<()> {
        self.dirty_spans.clear();
        self.dirty_large_entries.clear();

        // conservatively dirty every mature span slot with managed edges
        for span_index in 0..self.small.spans.len() {
            let Some(span) = self.span(span_index) else {
                continue;
            };
            let occupied = span.occupied.clone();
            let shape_ids = span.shape_ids.clone();
            let size_class = span.size_class;

            for slot_index in 0..span.slot_count {
                if !occupied.contains(slot_index) {
                    continue;
                }

                let Some(shape_id) = shape_ids.get(slot_index).copied().flatten() else {
                    return Err(HeapError::MissingSmallSlot {
                        span_index,
                        slot_index,
                    });
                };
                let Some(shape) = self.shape_table.shape(shape_id) else {
                    return Err(HeapError::MissingSmallSlot {
                        span_index,
                        slot_index,
                    });
                };

                if !shape.scan.has_reference() {
                    continue;
                }

                self.mark_span_slot_dirty(span_index, slot_index, 0, size_class)?;
            }
        }

        // conservatively dirty every mature large entry with managed edges
        for entry_index in 0..self.large.entries.len() {
            let entry_id = LargeEntryId::new(entry_index as u64 + 1);
            let Some(entry) = self.large_entry(entry_id) else {
                continue;
            };
            let Some(shape) = self.shape_table.shape(entry.shape_id) else {
                return Err(HeapError::MissingLargeEntry {
                    entry_id: entry_id.id(),
                });
            };

            if !shape.scan.has_reference() {
                continue;
            }

            self.mark_large_entry_dirty(entry_id, 0, entry.len)?;
        }

        Ok(())
    }

    /// Remember one mature managed span write if it may touch references.
    pub(crate) fn mark_span_slot_dirty(
        &mut self,
        span_index: usize,
        slot_index: usize,
        byte_offset: usize,
        byte_len: usize,
    ) -> HeapResult<()> {
        let Some(span) = self.span(span_index) else {
            return Err(HeapError::MissingSpan { span_index });
        };
        let Some(shape_id) = span.shape_ids.get(slot_index).copied().flatten() else {
            return Err(HeapError::MissingSmallSlot {
                span_index,
                slot_index,
            });
        };
        let Some(shape) = self.shape_table.shape(shape_id) else {
            return Err(HeapError::MissingSmallSlot {
                span_index,
                slot_index,
            });
        };

        let is_overlapping = overlaps_managed_range(
            &shape.scan,
            byte_offset,
            byte_len,
            self.managed_reference_bytes,
        )?;

        if !is_overlapping {
            return Ok(());
        }

        let slot_offset =
            span.size_class
                .checked_mul(slot_index)
                .ok_or(HeapError::InvariantOverflow {
                    context: "managed dirty slot offset",
                })?;
        let mut should_queue = false;

        // mark the overlapping card range on the owning span
        if let Some(span) = self.span_mut(span_index) {
            let dirty_start =
                slot_offset
                    .checked_add(byte_offset)
                    .ok_or(HeapError::InvariantOverflow {
                        context: "managed dirty card start",
                    })?;
            span.dirty_cards.mark_range(dirty_start, byte_len);

            if !span.is_dirty_queued {
                span.is_dirty_queued = true;
                should_queue = true;
            }
        }

        // queue the owning span once for the next minor collection
        if should_queue {
            self.dirty_spans.push(span_index);
        }

        Ok(())
    }

    /// Remember one mature managed large-entry write if it may touch references.
    pub(crate) fn mark_large_entry_dirty(
        &mut self,
        entry_id: LargeEntryId,
        byte_offset: usize,
        byte_len: usize,
    ) -> HeapResult<()> {
        let Some(entry) = self.large_entry(entry_id) else {
            return Err(HeapError::MissingLargeEntry {
                entry_id: entry_id.id(),
            });
        };
        let Some(shape) = self.shape_table.shape(entry.shape_id) else {
            return Err(HeapError::MissingLargeEntry {
                entry_id: entry_id.id(),
            });
        };

        let is_overlapping = overlaps_managed_range(
            &shape.scan,
            byte_offset,
            byte_len,
            self.managed_reference_bytes,
        )?;
        if !is_overlapping {
            return Ok(());
        }

        // mark the overlapping card range on the owning entry
        let mut should_queue = false;
        if let Some(entry) = self.large_entry_mut(entry_id) {
            entry.dirty_cards.mark_range(byte_offset, byte_len);

            if !entry.is_dirty_queued {
                entry.is_dirty_queued = true;
                should_queue = true;
            }
        }

        // queue the owning entry once for the next minor collection
        if should_queue {
            self.dirty_large_entries.push(entry_id);
        }

        Ok(())
    }

    /// Return whether one local write range may overlap shared managed roots.
    pub(crate) fn overlaps_shared_roots(
        &self,
        shape_id: ShapeId,
        byte_offset: usize,
        byte_len: usize,
    ) -> HeapResult<bool> {
        let Some(shape) = self.shape_table.shape(shape_id) else {
            return Err(HeapError::InvalidShapeId {
                index: shape_id.index(),
            });
        };

        overlaps_shared_range(
            &shape.scan,
            byte_offset,
            byte_len,
            SharedManagedReference::BYTE_LEN,
        )
    }
}

impl Drop for ManagedSpace {
    fn drop(&mut self) {
        self.flush_page_run_cache();
    }
}
