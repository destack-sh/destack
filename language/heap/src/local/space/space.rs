use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use destack_mir::{Layout, LayoutId, LayoutTable, ReferenceMap};

use super::{
    GcState, HeapLocation, HeapPageOwner, HeapStorage, HeapYoungId, LargeEntry, LargeEntryId,
    PinSet, SmallSpan, YoungEntry, YoungSpace,
};
use crate::allocator::{Allocator, PageId, PageRunCache, PageView, SizeClassTable, SpanSlot};
use crate::{
    AllocationUsage, CowTable, HeapError, HeapOptions, HeapReference, HeapResult, HeapSpaceUsage,
    SmallSpanClass, TraceQueue, overlaps_heap_range, overlaps_shared_range, slot_reference_map,
};

/// Collector queue for local heap references.
type HeapTraceQueue = TraceQueue<HeapReference>;

/// The first non-null heap large-entry id.
const FIRST_ALLOCATED_LARGE_ENTRY_ID: u64 = 1;

/// One heap small space.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SmallSpace {
    /// The configured size-class table.
    pub(crate) size_classes: SizeClassTable,
    /// The configured span width.
    pub(crate) span_bytes: usize,
    /// The live heap spans.
    pub(crate) spans: CowTable<SmallSpan>,
    /// The reusable non-full spans per layout id.
    pub(crate) available_spans: Vec<Vec<usize>>,
}

/// One heap large space.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LargeSpace {
    /// The configured page width for entries in large space.
    pub(crate) page_bytes: usize,
    /// The live heap entries.
    pub(crate) entries: CowTable<LargeEntry>,
    /// The free heap entry ids available for reuse.
    pub(crate) free_large_entry_ids: Vec<u64>,
    /// The next heap entry id to allocate.
    pub(crate) next_unused_large_entry_id: u64,
}

/// One heap space over a shared allocator.
#[derive(Debug)]
pub struct HeapSpace {
    /// The shared page allocator for every heap payload.
    pub(super) allocator: Arc<Allocator>,
    /// The managed layout table visible to this heap space.
    pub(super) layouts: Arc<LayoutTable>,
    /// The local front-end cache of reusable page runs.
    pub(crate) page_run_cache: PageRunCache,

    /// The heap young space.
    pub(crate) young: YoungSpace,
    /// The heap small space.
    pub(crate) small: SmallSpace,
    /// The heap large space.
    pub(crate) large: LargeSpace,
    /// The owning heap metadata for each visible allocator page.
    pub(crate) page_owners: Vec<Option<HeapPageOwner>>,

    /// The maximum payload size admitted into young space.
    pub(crate) max_young_allocation_bytes: usize,
    /// The exact live heap usage.
    pub(crate) usage: AllocationUsage,

    /// The live GC state.
    pub(crate) gc: GcState,
    /// The reusable collector trace queue.
    pub(crate) trace_queue: HeapTraceQueue,
    /// Whether a heap collection is currently running.
    pub(crate) is_collecting: bool,
    /// The scoped heap pins that keep stable addresses and block branch boundaries.
    pub(crate) pins: PinSet,
    /// Mature spans queued for dirty-card scanning.
    pub(crate) dirty_spans: Vec<usize>,
    /// Mature large entries queued for dirty-card scanning.
    pub(crate) dirty_large_entries: Vec<LargeEntryId>,
    /// Live local references whose shapes may contain shared heap edges.
    pub(crate) shared_edge_roots: Vec<HeapReference>,
    /// Reverse index into tracked shared-edge roots.
    pub(crate) shared_edge_index: BTreeMap<HeapReference, usize>,
    /// Whether one local-to-shared edge scan is currently active.
    pub(crate) is_scanning_shared_edges: bool,
    /// The next dense reference slot to scan for shared edges.
    pub(crate) shared_edge_cursor: usize,
    /// The pending local references whose shared edges need rescanning.
    pub(crate) shared_edge_queue: HeapTraceQueue,
    /// Queue-membership for pending shared-edge rescans.
    pub(crate) shared_edge_pending: BTreeSet<HeapReference>,
}

impl HeapSpace {
    /// Register one managed layout in this heap space and return its stable id.
    pub(crate) fn register_layout(&mut self, layout: Layout) -> LayoutId {
        Arc::make_mut(&mut self.layouts).insert(layout)
    }

    /// Create one heap space with explicit options.
    pub fn with_options(
        allocator: Arc<Allocator>,
        options: &HeapOptions,
    ) -> Result<Self, HeapError> {
        Self::with_layouts_and_options(allocator, Arc::new(LayoutTable::new()), options)
    }

    /// Create one heap space with explicit layouts and options.
    pub fn with_layouts_and_options(
        allocator: Arc<Allocator>,
        layouts: Arc<LayoutTable>,
        options: &HeapOptions,
    ) -> Result<Self, HeapError> {
        options.validate_local()?;
        options.validate_allocator(&allocator)?;

        Self::build_with_options(allocator, layouts, options)
    }

    /// Create one heap space from one checked options set.
    pub(crate) fn build_with_options(
        allocator: Arc<Allocator>,
        layouts: Arc<LayoutTable>,
        options: &HeapOptions,
    ) -> Result<Self, HeapError> {
        let mut page_run_cache = PageRunCache::new(allocator.pages_per_arena());

        // reserve one fixed young-space page run up front
        let young = YoungSpace::new(
            &allocator,
            options.heap_young_bytes,
            options.page_bytes,
            &mut page_run_cache,
        )?;
        let max_young_allocation_bytes = if options.heap_young_bytes == 0 {
            0
        } else {
            options.max_heap_young_allocation_bytes
        };

        // build the live root over the shared allocator
        let young_pages = young.pages.clone();
        let mut space = Self {
            allocator,
            layouts,
            page_run_cache,
            max_young_allocation_bytes,
            young,
            small: SmallSpace {
                size_classes: options.size_classes.clone(),
                span_bytes: options.heap_small_bytes,
                spans: CowTable::new(),
                available_spans: vec![
                    Vec::new();
                    SmallSpanClass::bucket_count(&options.size_classes)
                ],
            },
            large: LargeSpace {
                page_bytes: options.page_bytes,
                entries: CowTable::new(),
                free_large_entry_ids: Vec::new(),
                next_unused_large_entry_id: FIRST_ALLOCATED_LARGE_ENTRY_ID,
            },
            page_owners: Vec::new(),
            usage: AllocationUsage::default(),
            gc: GcState::default(),
            trace_queue: TraceQueue::default(),
            is_collecting: false,
            pins: PinSet::default(),
            dirty_spans: Vec::new(),
            dirty_large_entries: Vec::new(),
            shared_edge_roots: Vec::new(),
            shared_edge_index: BTreeMap::new(),
            is_scanning_shared_edges: false,
            shared_edge_cursor: 0,
            shared_edge_queue: TraceQueue::default(),
            shared_edge_pending: BTreeSet::new(),
        };

        space.map_page_view(&young_pages, |logical_page_index| HeapPageOwner::Young {
            logical_page_index,
        })?;

        Ok(space)
    }

    /// Return the shared page allocator.
    pub fn allocator(&self) -> &Arc<Allocator> {
        &self.allocator
    }

    /// Return the exact retained heap bytes.
    pub fn active_bytes(&self) -> u64 {
        self.mapped_bytes()
    }

    /// Return the exact mapped heap page bytes.
    pub fn mapped_bytes(&self) -> u64 {
        self.allocator.mapped_bytes_for_page_views(
            std::iter::once(&self.young.pages)
                .chain(self.small.spans.iter().map(|span| &span.pages))
                .chain(self.large.entries.iter().map(|entry| &entry.pages)),
        ) + self
            .page_run_cache
            .cached_bytes(self.allocator.page_bytes())
    }

    /// Return the exact borrowed heap image bytes.
    pub fn borrowed_bytes(&self) -> u64 {
        self.allocator.borrowed_bytes_for_page_views(
            std::iter::once(&self.young.pages)
                .chain(self.small.spans.iter().map(|span| &span.pages))
                .chain(self.large.entries.iter().map(|entry| &entry.pages)),
        )
    }

    /// Return the current GC state.
    pub fn gc_state(&self) -> &GcState {
        &self.gc
    }

    /// Stabilize one local heap reference in mature storage.
    pub fn stabilize(&mut self, reference: HeapReference) -> HeapResult<HeapReference> {
        self.promote_reference(reference)
    }

    /// Pin one local heap reference against movement.
    pub fn pin(&mut self, reference: HeapReference) -> HeapResult<HeapReference> {
        // first ensure the reference already points at stable mature storage
        let reference = self.stabilize(reference)?;

        // then record the active pin count
        self.pins.pin(reference)?;

        Ok(reference)
    }

    /// Release one local heap pin.
    pub fn unpin(&mut self, reference: HeapReference) -> HeapResult<()> {
        self.pins.unpin(reference)
    }

    /// Return the number of live heap entries.
    pub fn allocation_count(&self) -> usize {
        self.usage.allocation_count()
    }

    /// Return the number of live heap bytes.
    pub fn allocated_bytes(&self) -> u64 {
        self.usage.allocated_bytes()
    }

    /// Return the exact live usage for this heap space.
    pub fn usage(&self) -> HeapSpaceUsage {
        HeapSpaceUsage {
            allocation_count: self.usage.allocation_count(),
            allocated_bytes: self.usage.allocated_bytes(),
            active_bytes: self.active_bytes(),
            mapped_bytes: self.mapped_bytes(),
            borrowed_bytes: self.borrowed_bytes(),
        }
    }

    /// Flush transient cache state before one exact branch boundary.
    pub(crate) fn flush_branch_boundary(&mut self) {
        self.page_run_cache.flush(&self.allocator);
    }

    /// Allocate one zeroed page view through the local page-run cache.
    pub(crate) fn allocate_page_view_zeroed(&mut self, byte_len: usize) -> HeapResult<PageView> {
        self.page_run_cache
            .allocate_zeroed(&self.allocator, byte_len)
    }

    /// Allocate one initialized page view through the local page-run cache.
    pub(crate) fn allocate_page_view_bytes(&mut self, bytes: &[u8]) -> HeapResult<PageView> {
        self.page_run_cache.allocate_bytes(&self.allocator, bytes)
    }

    /// Release one page view through the local page-run cache.
    pub(crate) fn release_page_view(&mut self, page_view: PageView) -> HeapResult<()> {
        self.page_run_cache
            .release_page_view(&self.allocator, page_view)
    }

    /// Return the visible owner for one physical page.
    pub(crate) fn page_owner(&self, page_id: PageId) -> Option<HeapPageOwner> {
        self.page_owners.get(page_id.index()).copied().flatten()
    }

    /// Record one visible owner for every page in one logical page view.
    pub(crate) fn map_page_view(
        &mut self,
        page_view: &PageView,
        mut owner: impl FnMut(usize) -> HeapPageOwner,
    ) -> HeapResult<()> {
        for logical_page_index in 0..page_view.len() {
            let Some(page_id) = page_view.page(logical_page_index) else {
                return Err(HeapError::MissingLogicalPage {
                    page_index: logical_page_index,
                });
            };
            let page_index = page_id.index();

            if self.page_owners.len() <= page_index {
                self.page_owners.resize(page_index + 1, None);
            }

            self.page_owners[page_index] = Some(owner(logical_page_index));
        }

        Ok(())
    }

    /// Clear every visible owner for one logical page view.
    pub(crate) fn unmap_page_view(&mut self, page_view: &PageView) -> HeapResult<()> {
        for logical_page_index in 0..page_view.len() {
            let Some(page_id) = page_view.page(logical_page_index) else {
                return Err(HeapError::MissingLogicalPage {
                    page_index: logical_page_index,
                });
            };

            if let Some(owner) = self.page_owners.get_mut(page_id.index()) {
                *owner = None;
            }
        }

        Ok(())
    }

    /// Return the resolved location for one live heap reference.
    pub(crate) fn resolve_location(&self, reference: HeapReference) -> Option<HeapLocation> {
        let (page_id, page_offset) = self.allocator.address_page_position(reference.address())?;
        let owner = self.page_owner(page_id)?;

        match owner {
            HeapPageOwner::Young { logical_page_index } => {
                self.resolve_young_location(logical_page_index, page_offset)
            }
            HeapPageOwner::Small {
                span_index,
                logical_page_index,
            } => {
                self.resolve_small_location(reference, span_index, logical_page_index, page_offset)
            }
            HeapPageOwner::Large {
                entry_id,
                logical_page_index,
            } => self.resolve_large_location(reference, entry_id, logical_page_index, page_offset),
        }
    }

    /// Return the resolved young-space location for one live heap reference.
    fn resolve_young_location(
        &self,
        logical_page_index: usize,
        page_offset: usize,
    ) -> Option<HeapLocation> {
        let logical_byte_offset = logical_page_index
            .checked_mul(self.young.page_bytes)?
            .checked_add(page_offset)?;

        for (entry_index, entry) in self.young.entries.iter().enumerate() {
            if !entry.is_live {
                continue;
            }

            let entry_offset = self.young_entry_offset(entry);
            if entry.byte_len == 0 {
                if logical_byte_offset != entry_offset {
                    continue;
                }

                let base_address = self
                    .allocator
                    .page_view_ptr(&self.young.pages, entry_offset)
                    .ok()? as *mut u8 as usize;

                return Some(HeapLocation {
                    storage: HeapStorage::Young(HeapYoungId::new(
                        self.young.generation,
                        entry_index as u32,
                    )),
                    base: HeapReference::new(base_address),
                    byte_offset: 0,
                    byte_len: 0,
                });
            }

            let entry_end = entry_offset.checked_add(entry.byte_len)?;
            if logical_byte_offset < entry_offset || logical_byte_offset >= entry_end {
                continue;
            }

            let base_address = self
                .allocator
                .page_view_ptr(&self.young.pages, entry_offset)
                .ok()? as *mut u8 as usize;
            let byte_offset = logical_byte_offset.checked_sub(entry_offset)?;

            return Some(HeapLocation {
                storage: HeapStorage::Young(HeapYoungId::new(
                    self.young.generation,
                    entry_index as u32,
                )),
                base: HeapReference::new(base_address),
                byte_offset,
                byte_len: entry.byte_len,
            });
        }

        None
    }

    /// Return the resolved small-span location for one live heap reference.
    fn resolve_small_location(
        &self,
        reference: HeapReference,
        span_index: usize,
        logical_page_index: usize,
        page_offset: usize,
    ) -> Option<HeapLocation> {
        let span = self.span(span_index)?;
        let logical_byte_offset = logical_page_index
            .checked_mul(self.allocator.page_bytes())?
            .checked_add(page_offset)?;
        let slot_index = logical_byte_offset / span.class.size_class;
        let slot_offset = logical_byte_offset % span.class.size_class;
        if slot_index >= span.slot_count || !span.occupied.contains(slot_index) {
            return None;
        }

        let byte_len = *span.byte_lens.get(slot_index)?;
        if byte_len == 0 {
            if slot_offset != 0 {
                return None;
            }
        } else if slot_offset >= byte_len {
            return None;
        }

        let slot_base_offset = slot_index.checked_mul(span.class.size_class)?;
        let base_address = self
            .allocator
            .page_view_ptr(&span.pages, slot_base_offset)
            .ok()? as *mut u8 as usize;
        let slot = SpanSlot::new(span_index, slot_index).ok()?;

        debug_assert_eq!(reference.address(), base_address.checked_add(slot_offset)?);

        Some(HeapLocation {
            storage: HeapStorage::Small(slot),
            base: HeapReference::new(base_address),
            byte_offset: slot_offset,
            byte_len,
        })
    }

    /// Return the resolved large-entry location for one live heap reference.
    fn resolve_large_location(
        &self,
        reference: HeapReference,
        entry_id: LargeEntryId,
        logical_page_index: usize,
        page_offset: usize,
    ) -> Option<HeapLocation> {
        let entry = self.large_entry(entry_id)?;
        let logical_byte_offset = logical_page_index
            .checked_mul(self.allocator.page_bytes())?
            .checked_add(page_offset)?;
        if entry.len == 0 {
            if logical_byte_offset != 0 {
                return None;
            }
        } else if logical_byte_offset >= entry.len {
            return None;
        }

        let base_address = self.allocator.page_view_ptr(&entry.pages, 0).ok()? as *mut u8 as usize;

        debug_assert_eq!(
            reference.address(),
            base_address.checked_add(logical_byte_offset)?
        );

        Some(HeapLocation {
            storage: HeapStorage::Large(entry_id),
            base: HeapReference::new(base_address),
            byte_offset: logical_byte_offset,
            byte_len: entry.len,
        })
    }

    /// Rebuild the tracked local references that may contain shared edges.
    pub(crate) fn rebuild_shared_edge_roots(&mut self) -> HeapResult<()> {
        self.shared_edge_roots.clear();
        self.shared_edge_index.clear();

        for reference in self.live_references()? {
            if !self.reference_has_shared_roots(reference)? {
                continue;
            }

            self.track_shared_edge_root(reference)?;
        }

        Ok(())
    }

    /// Record one live reference whose shape may contain shared edges.
    pub(crate) fn track_shared_edge_root(&mut self, reference: HeapReference) -> HeapResult<()> {
        if self.shared_edge_index.contains_key(&reference) {
            return Ok(());
        }

        let tracked_index = self.shared_edge_roots.len();
        self.shared_edge_roots.push(reference);
        self.shared_edge_index.insert(reference, tracked_index);

        Ok(())
    }

    /// Remove one live reference from the tracked shared-edge set.
    pub(crate) fn remove_shared_edge_root(&mut self, reference: HeapReference) -> HeapResult<()> {
        let Some(tracked_index) = self.shared_edge_index.remove(&reference) else {
            return Ok(());
        };

        let moved_reference = self.shared_edge_roots.pop();

        let Some(moved_reference) = moved_reference else {
            return Ok(());
        };

        if tracked_index == self.shared_edge_roots.len() {
            return Ok(());
        }

        self.shared_edge_roots[tracked_index] = moved_reference;
        self.shared_edge_index
            .insert(moved_reference, tracked_index);

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
    pub(crate) fn young_entry(&self, young_id: HeapYoungId) -> Option<&YoungEntry> {
        // reject stale generation ids first
        self.check_young_generation(young_id)?;

        let entry = self.young.entries.get(young_id.index() as usize)?;

        // skip free young-entry slots
        entry.is_live.then_some(entry)
    }

    /// Return one live young entry mutably by id.
    pub(crate) fn young_entry_mut(&mut self, young_id: HeapYoungId) -> Option<&mut YoungEntry> {
        // reject stale generation ids first
        self.check_young_generation(young_id)?;

        let entry = self.young.entries.get_mut(young_id.index() as usize)?;

        // skip free young-entry slots
        entry.is_live.then_some(entry)
    }

    /// Return one live heap span by index.
    pub(crate) fn span(&self, span_index: usize) -> Option<&SmallSpan> {
        self.small.spans.get(span_index)
    }

    /// Return one live heap span mutably by index.
    pub(crate) fn span_mut(&mut self, span_index: usize) -> Option<&mut SmallSpan> {
        self.small.spans.get_mut(span_index)
    }

    /// Return the byte offset for one young entry.
    pub(crate) fn young_entry_offset(&self, entry: &YoungEntry) -> usize {
        entry.first_page as usize * self.young.page_bytes + entry.first_offset as usize
    }

    /// Return one managed layout by id.
    pub(crate) fn layout(&self, layout_id: LayoutId) -> HeapResult<&Layout> {
        self.layouts
            .layouts
            .get(layout_id.index())
            .ok_or(HeapError::InvalidLayoutId {
                index: layout_id.index(),
            })
    }

    /// Return the byte length for one managed layout.
    pub(crate) fn layout_byte_len(&self, layout_id: LayoutId) -> HeapResult<usize> {
        Ok(self.layout(layout_id)?.size as usize)
    }

    /// Return the reference map for one managed layout.
    pub(crate) fn reference_map(&self, layout_id: LayoutId) -> HeapResult<&ReferenceMap> {
        Ok(&self.layout(layout_id)?.reference_map)
    }

    /// Check that one young id belongs to the current young-space generation.
    fn check_young_generation(&self, young_id: HeapYoungId) -> Option<()> {
        (young_id.generation() == self.young.generation).then_some(())
    }

    /// Return the reference map for one heap storage partition.
    pub(crate) fn location_reference_map(&self, storage: HeapStorage) -> HeapResult<ReferenceMap> {
        match storage {
            HeapStorage::Young(young_id) => {
                let layout_id = self
                    .young_entry(young_id)
                    .ok_or(HeapError::MissingYoungEntry {
                        generation: young_id.generation(),
                        entry_index: young_id.index(),
                    })?
                    .layout_id;

                self.reference_map(layout_id).cloned()
            }
            HeapStorage::Small(slot) => {
                self.small_slot_reference_map(slot.span_index(), slot.slot_index())
            }
            HeapStorage::Large(entry_id) => {
                let layout_id = self
                    .large_entry(entry_id)
                    .ok_or(HeapError::MissingLargeEntry {
                        entry_id: entry_id.id(),
                    })?
                    .layout_id;

                self.reference_map(layout_id).cloned()
            }
        }
    }

    /// Return the base reference for one heap storage partition.
    pub(crate) fn base_reference(&self, storage: HeapStorage) -> HeapResult<HeapReference> {
        let base_address = match storage {
            HeapStorage::Young(young_id) => {
                let Some(entry) = self.young_entry(young_id) else {
                    return Err(HeapError::MissingYoungEntry {
                        generation: young_id.generation(),
                        entry_index: young_id.index(),
                    });
                };
                let entry_offset = self.young_entry_offset(entry);

                self.allocator
                    .page_view_ptr(&self.young.pages, entry_offset)? as *mut u8
                    as usize
            }
            HeapStorage::Small(slot) => {
                let Some(span) = self.span(slot.span_index()) else {
                    return Err(HeapError::MissingSpan {
                        span_index: slot.span_index(),
                    });
                };
                let slot_offset = span.class.size_class.checked_mul(slot.slot_index()).ok_or(
                    HeapError::InvariantOverflow {
                        context: "heap slot base offset",
                    },
                )?;

                self.allocator.page_view_ptr(&span.pages, slot_offset)? as *mut u8 as usize
            }
            HeapStorage::Large(entry_id) => {
                let Some(entry) = self.large_entry(entry_id) else {
                    return Err(HeapError::MissingLargeEntry {
                        entry_id: entry_id.id(),
                    });
                };

                self.allocator.page_view_ptr(&entry.pages, 0)? as *mut u8 as usize
            }
        };

        Ok(HeapReference::new(base_address))
    }

    /// Rebuild the mature remembered set conservatively.
    pub(crate) fn rebuild_remembered_set(&mut self) -> HeapResult<()> {
        self.dirty_spans.clear();
        self.dirty_large_entries.clear();

        // conservatively dirty every mature span slot with heap edges
        for span_index in 0..self.small.spans.len() {
            let Some(span) = self.span(span_index) else {
                continue;
            };
            let occupied = span.occupied.clone();
            let size_class = span.class.size_class;

            for slot_index in 0..span.slot_count {
                if !occupied.contains(slot_index) {
                    continue;
                }

                let reference_map = self.small_slot_reference_map(span_index, slot_index)?;
                if !reference_map.has_reference() {
                    continue;
                }

                self.mark_span_slot_dirty(span_index, slot_index, 0, size_class)?;
            }
        }

        // conservatively dirty every mature large entry with heap edges
        for entry_index in 0..self.large.entries.len() {
            let entry_id = LargeEntryId::new(entry_index as u64 + 1);
            let Some(entry) = self.large_entry(entry_id) else {
                continue;
            };
            if !self.reference_map(entry.layout_id)?.has_reference() {
                continue;
            }

            self.mark_large_entry_dirty(entry_id, 0, entry.len)?;
        }

        Ok(())
    }

    /// Remember one mature heap span write if it may touch references.
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
        if !span.occupied.contains(slot_index) {
            return Err(HeapError::MissingSmallSlot {
                span_index,
                slot_index,
            });
        }

        let reference_map = self.small_slot_reference_map(span_index, slot_index)?;
        let is_overlapping = overlaps_heap_range(&reference_map, byte_offset, byte_len)?;
        if !is_overlapping {
            return Ok(());
        }

        let slot_offset =
            span.class
                .size_class
                .checked_mul(slot_index)
                .ok_or(HeapError::InvariantOverflow {
                    context: "heap dirty slot offset",
                })?;
        let mut should_queue = false;

        // mark the overlapping card range on the owning span
        if let Some(span) = self.span_mut(span_index) {
            let dirty_start =
                slot_offset
                    .checked_add(byte_offset)
                    .ok_or(HeapError::InvariantOverflow {
                        context: "heap dirty card start",
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

    /// Remember one mature heap large-entry write if it may touch references.
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
        let is_overlapping =
            overlaps_heap_range(self.reference_map(entry.layout_id)?, byte_offset, byte_len)?;
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

    /// Return whether one local write range may overlap shared heap roots.
    pub(crate) fn overlaps_shared_roots(
        &self,
        reference_map: &ReferenceMap,
        byte_offset: usize,
        byte_len: usize,
    ) -> HeapResult<bool> {
        overlaps_shared_range(reference_map, byte_offset, byte_len)
    }

    /// Return the exact reference map stored for one small slot.
    pub(crate) fn small_slot_reference_map(
        &self,
        span_index: usize,
        slot_index: usize,
    ) -> HeapResult<ReferenceMap> {
        let Some(span) = self.span(span_index) else {
            return Err(HeapError::MissingSpan { span_index });
        };
        let Some(&byte_len) = span.byte_lens.get(slot_index) else {
            return Err(HeapError::MissingSmallSlot {
                span_index,
                slot_index,
            });
        };

        Ok(slot_reference_map(
            &span.local_reference_bits,
            &span.shared_reference_bits,
            slot_index,
            span.class.size_class,
            byte_len,
        ))
    }

    /// Return the reusable-span bucket index for one small-span class.
    pub(crate) fn small_span_bucket(&self, class: &SmallSpanClass) -> HeapResult<usize> {
        class.bucket_index(&self.small.size_classes)
    }
}

impl Drop for HeapSpace {
    fn drop(&mut self) {
        self.page_run_cache.flush(&self.allocator);
    }
}
