use std::borrow::Cow;
use std::mem::size_of;
use std::sync::Arc;

use destack_mir::LayoutId;
use serde::{Deserialize, Serialize};

use super::{ReferenceMapId, StoredLayoutId};
use crate::alloc::{Bitmap, CardSet, PageArena, PageId, projected_vec_capacity};
use crate::heap::ImageAccounting;

/// One immutable managed span trace layout.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
enum SpanTraceImage {
    /// One span-wide trace id shared by every occupied slot.
    Monomorphic(u32),
    /// One per-slot trace id table.
    Polymorphic(Arc<[u32]>),
}

/// One immutable managed span layout metadata shape.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
enum SpanLayoutImage {
    /// One span-wide layout id shared by every occupied slot.
    Monomorphic(StoredLayoutId),
    /// One per-slot layout id table.
    Polymorphic(Arc<[StoredLayoutId]>),
}

/// One immutable managed span image.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManagedSpanImage {
    /// The size class for this span in bytes.
    pub size_class: usize,
    /// The number of slots in this span.
    pub slot_count: usize,
    /// The fixed byte width for every page in this span image.
    pub page_bytes: usize,
    /// The packed slot payload pages.
    pub pages: Arc<[Arc<[u8]>]>,
    /// The occupied slots in this span.
    pub occupied: Bitmap,
    /// The trace metadata for this span.
    trace_metadata: SpanTraceImage,
    /// The layout metadata for this span.
    layout_metadata: SpanLayoutImage,
}

impl ManagedSpanImage {
    /// Report whether this span image is one vacant directory slot.
    pub(crate) fn is_vacant(&self) -> bool {
        self.slot_count == 0
    }

    /// Report whether this span image shares durable backing with another span image.
    pub fn shares_storage_with(&self, other: &Self) -> bool {
        self.size_class == other.size_class
            && self.slot_count == other.slot_count
            && self.page_bytes == other.page_bytes
            && self.pages.len() == other.pages.len()
            && self
                .pages
                .iter()
                .zip(other.pages.iter())
                .all(|(left, right)| Arc::ptr_eq(left, right))
            && self.occupied == other.occupied
            && Self::shares_trace_storage(&self.trace_metadata, &other.trace_metadata)
            && Self::shares_layout_storage(&self.layout_metadata, &other.layout_metadata)
    }

    /// Report whether two trace metadata images share backing.
    fn shares_trace_storage(left: &SpanTraceImage, right: &SpanTraceImage) -> bool {
        match (left, right) {
            (SpanTraceImage::Monomorphic(left), SpanTraceImage::Monomorphic(right)) => {
                left == right
            }
            (SpanTraceImage::Polymorphic(left), SpanTraceImage::Polymorphic(right)) => {
                Arc::ptr_eq(left, right)
            }
            _ => false,
        }
    }

    /// Report whether two layout metadata images share backing.
    fn shares_layout_storage(left: &SpanLayoutImage, right: &SpanLayoutImage) -> bool {
        match (left, right) {
            (SpanLayoutImage::Monomorphic(left), SpanLayoutImage::Monomorphic(right)) => {
                left == right
            }
            (SpanLayoutImage::Polymorphic(left), SpanLayoutImage::Polymorphic(right)) => {
                Arc::ptr_eq(left, right)
            }
            _ => false,
        }
    }

    /// Return the exact owned bytes for this durable span image.
    pub fn image_bytes(&self) -> usize {
        let mut image_bytes = size_of::<Self>();
        image_bytes += self.pages.len() * size_of::<Arc<[u8]>>();

        for page in self.pages.iter() {
            image_bytes += page.len();
        }

        image_bytes += self.occupied.retained_bytes();
        image_bytes += match &self.trace_metadata {
            SpanTraceImage::Monomorphic(_) => 0,
            SpanTraceImage::Polymorphic(trace_ids) => trace_ids.len() * size_of::<u32>(),
        };
        image_bytes += match &self.layout_metadata {
            SpanLayoutImage::Monomorphic(_) => 0,
            SpanLayoutImage::Polymorphic(layout_ids) => {
                layout_ids.len() * size_of::<StoredLayoutId>()
            }
        };

        image_bytes
    }

    /// Account this span image into deduplicated retained-image bytes.
    pub fn retained_image_bytes(&self, accounting: &mut ImageAccounting) -> usize {
        let mut image_bytes = size_of::<Self>();
        image_bytes += accounting.account_arc_page_table(&self.pages);

        for page in self.pages.iter() {
            image_bytes += accounting.account_arc_bytes(page);
        }

        image_bytes += self.occupied.retained_bytes();
        image_bytes += match &self.trace_metadata {
            SpanTraceImage::Monomorphic(_) => 0,
            SpanTraceImage::Polymorphic(trace_ids) => accounting.account_arc_u32_slice(trace_ids),
        };
        image_bytes += match &self.layout_metadata {
            SpanLayoutImage::Monomorphic(_) => 0,
            SpanLayoutImage::Polymorphic(layout_ids) => {
                accounting.account_arc_layout_slice(layout_ids)
            }
        };

        image_bytes
    }
}

/// One owned managed span trace layout.
#[derive(Debug, Clone, PartialEq, Eq)]
enum SpanTraceData {
    /// One span-wide trace id shared by every occupied slot.
    Monomorphic(u32),
    /// One per-slot trace id table.
    Polymorphic(Box<[u32]>),
}

/// One owned managed span layout metadata shape.
#[derive(Debug, Clone, PartialEq, Eq)]
enum SpanLayoutData {
    /// One span-wide layout id shared by every occupied slot.
    Monomorphic(StoredLayoutId),
    /// One per-slot layout id table.
    Polymorphic(Box<[StoredLayoutId]>),
}

/// One owned managed span payload.
#[derive(Debug, Clone, PartialEq, Eq)]
struct SpanData {
    /// The packed slot payload pages.
    pages: Vec<PageId>,
    /// The occupied slots in this span.
    occupied: Bitmap,
    /// The trace metadata for this span.
    trace_metadata: SpanTraceData,
    /// The layout metadata for this span.
    layout_metadata: SpanLayoutData,
}

/// One live managed span storage state.
#[derive(Debug, Clone, PartialEq, Eq)]
enum SpanState {
    /// Owned mutable span storage.
    Local(SpanData),
    /// Shared immutable span storage.
    Shared(ManagedSpanImage),
}

/// One live managed span for one size class.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ManagedSpan {
    /// The slot payload size in bytes.
    size_class: usize,
    /// The number of slots in this span.
    slot_count: usize,
    /// The number of occupied slots in this span.
    occupied_count: usize,
    /// The next likely free slot.
    next_free_slot: usize,
    /// The live span storage.
    state: SpanState,
    /// The live mark bitmap keyed by slot.
    marked: Bitmap,
    /// The live pinned slots keyed by slot.
    pinned: Bitmap,
    /// Additional pin counts for slots pinned more than once.
    extra_pin_counts: Vec<(usize, u16)>,
    /// The number of active pins in this span.
    active_pins: usize,
    /// The dirty cards remembered for young tracing.
    dirty_cards: CardSet,
    /// Whether this span is already queued for dirty-card scanning.
    is_dirty_queued: bool,
}

impl ManagedSpan {
    /// Create one vacant managed span slot with no backing storage.
    pub(crate) fn vacant() -> Self {
        Self {
            size_class: 0,
            slot_count: 0,
            occupied_count: 0,
            next_free_slot: 0,
            state: SpanState::Local(SpanData {
                pages: Vec::new(),
                occupied: Bitmap::with_capacity(0),
                trace_metadata: SpanTraceData::Monomorphic(0),
                layout_metadata: SpanLayoutData::Monomorphic(StoredLayoutId::none()),
            }),
            marked: Bitmap::with_capacity(0),
            pinned: Bitmap::with_capacity(0),
            extra_pin_counts: Vec::new(),
            active_pins: 0,
            dirty_cards: CardSet::with_len(0),
            is_dirty_queued: false,
        }
    }

    /// Create one empty managed span for the given size class and span byte width.
    pub(crate) fn new(size_class: usize, span_bytes: usize, page_arena: &mut PageArena) -> Self {
        let slot_count = (span_bytes / size_class).max(1);
        let pages = page_arena.allocate_pages(&[], slot_count * size_class);

        Self {
            size_class,
            slot_count,
            occupied_count: 0,
            next_free_slot: 0,
            state: SpanState::Local(SpanData {
                pages,
                occupied: Bitmap::with_capacity(slot_count),
                trace_metadata: SpanTraceData::Monomorphic(0),
                layout_metadata: SpanLayoutData::Monomorphic(StoredLayoutId::none()),
            }),
            marked: Bitmap::with_capacity(slot_count),
            pinned: Bitmap::with_capacity(slot_count),
            extra_pin_counts: Vec::new(),
            active_pins: 0,
            dirty_cards: CardSet::with_len(slot_count * size_class),
            is_dirty_queued: false,
        }
    }

    /// Restore one managed span from one immutable image.
    pub(crate) fn from_image(image: ManagedSpanImage) -> Self {
        if image.is_vacant() {
            return Self::vacant();
        }

        let occupied_count = image.occupied.count_ones();
        let slot_count = image.slot_count;
        let size_class = image.size_class;
        let next_free_slot = image.occupied.first_clear_from(0).unwrap_or(slot_count);

        Self {
            size_class,
            slot_count,
            occupied_count,
            next_free_slot,
            state: SpanState::Shared(image),
            marked: Bitmap::with_capacity(slot_count),
            pinned: Bitmap::with_capacity(slot_count),
            extra_pin_counts: Vec::new(),
            active_pins: 0,
            dirty_cards: CardSet::with_len(slot_count * size_class),
            is_dirty_queued: false,
        }
    }

    /// Report whether this span is one vacant directory slot.
    pub(crate) fn is_vacant(&self) -> bool {
        self.slot_count == 0
    }

    /// Report whether this span has no live allocations.
    pub(crate) fn is_empty(&self) -> bool {
        self.occupied_count == 0
    }

    /// Report whether this span has at least one free slot.
    pub(crate) fn has_free_slot(&self) -> bool {
        !self.is_vacant() && self.occupied_count < self.slot_count
    }

    /// Return the size class for this span in bytes.
    pub(crate) fn size_class(&self) -> usize {
        self.size_class
    }

    /// Return the slot count for this span.
    pub(crate) fn slot_count(&self) -> usize {
        self.slot_count
    }

    /// Return the first free slot in this span.
    pub(crate) fn first_free_slot(&self) -> Option<usize> {
        if !self.has_free_slot() {
            return None;
        }

        let occupied = self.occupied();
        occupied
            .first_clear_from(self.next_free_slot)
            .or_else(|| occupied.first_clear_from(0))
    }

    /// Report whether one slot is occupied.
    pub(crate) fn is_occupied(&self, slot: usize) -> bool {
        self.occupied().contains(slot)
    }

    /// Report whether one slot is marked.
    pub(crate) fn is_marked(&self, slot: usize) -> bool {
        self.marked.contains(slot)
    }

    /// Mark one slot.
    pub(crate) fn mark(&mut self, slot: usize) {
        self.marked.set(slot);
    }

    /// Clear all marks.
    pub(crate) fn clear_marks(&mut self) {
        self.marked.clear_all();
    }

    /// Return the bytes for one occupied slot.
    pub(crate) fn bytes<'a>(
        &'a self,
        page_arena: &'a PageArena,
        slot: usize,
        byte_len: usize,
    ) -> Option<Cow<'a, [u8]>> {
        if slot >= self.slot_count || !self.is_occupied(slot) || byte_len > self.size_class {
            return None;
        }

        let start = slot.checked_mul(self.size_class)?;
        match &self.state {
            SpanState::Local(storage) => {
                if let Some(bytes) = page_arena.borrow_window(&storage.pages, start, byte_len) {
                    Some(Cow::Borrowed(bytes))
                } else {
                    let mut bytes = vec![0; byte_len];
                    let read = page_arena.read_window(&storage.pages, start, &mut bytes);
                    if !read {
                        return None;
                    }

                    Some(Cow::Owned(bytes))
                }
            }
            SpanState::Shared(image) => Self::shared_bytes(image, start, byte_len),
        }
    }

    /// Return the trace id for one occupied slot.
    pub(crate) fn trace_id(&self, slot: usize) -> Option<ReferenceMapId> {
        if slot >= self.slot_count || !self.is_occupied(slot) {
            return None;
        }

        let trace_id = match &self.state {
            SpanState::Local(storage) => match &storage.trace_metadata {
                SpanTraceData::Monomorphic(trace_id) => *trace_id,
                SpanTraceData::Polymorphic(trace_ids) => trace_ids.get(slot).copied()?,
            },
            SpanState::Shared(image) => match &image.trace_metadata {
                SpanTraceImage::Monomorphic(trace_id) => *trace_id,
                SpanTraceImage::Polymorphic(trace_ids) => trace_ids.get(slot).copied()?,
            },
        };

        Some(ReferenceMapId::new(trace_id as usize))
    }

    /// Return the layout id for one occupied slot.
    pub(crate) fn layout_id(&self, slot: usize) -> Option<LayoutId> {
        if slot >= self.slot_count || !self.is_occupied(slot) {
            return None;
        }

        let layout_id = match &self.state {
            SpanState::Local(storage) => match &storage.layout_metadata {
                SpanLayoutData::Monomorphic(layout_id) => *layout_id,
                SpanLayoutData::Polymorphic(layout_ids) => layout_ids.get(slot).copied()?,
            },
            SpanState::Shared(image) => match &image.layout_metadata {
                SpanLayoutImage::Monomorphic(layout_id) => *layout_id,
                SpanLayoutImage::Polymorphic(layout_ids) => layout_ids.get(slot).copied()?,
            },
        };

        layout_id.to_option()
    }

    /// Set the layout id for one occupied slot.
    #[cfg(test)]
    pub(crate) fn set_layout_id(
        &mut self,
        page_arena: &mut PageArena,
        slot: usize,
        layout_id: LayoutId,
    ) -> bool {
        if slot >= self.slot_count || !self.is_occupied(slot) {
            return false;
        }

        let occupied = self.occupied_count;
        let slot_count = self.slot_count;
        let storage = self.state_mut(page_arena);
        Self::set_layout_metadata(
            &storage.occupied,
            &mut storage.layout_metadata,
            occupied,
            slot_count,
            slot,
            StoredLayoutId::from_option(Some(layout_id)),
        );
        true
    }

    /// Allocate one free slot in this span.
    pub(crate) fn allocate_slot(
        &mut self,
        page_arena: &mut PageArena,
        slot: usize,
        bytes: &[u8],
        trace_id: ReferenceMapId,
        layout_id: Option<LayoutId>,
    ) -> bool {
        if slot >= self.slot_count || bytes.len() > self.size_class || self.is_occupied(slot) {
            return false;
        }

        let size_class = self.size_class;
        let occupied = self.occupied_count;
        let slot_count = self.slot_count;
        let storage = self.state_mut(page_arena);
        let start = slot * size_class;
        let zeroed = page_arena.fill_window(&storage.pages, start, size_class, 0);
        debug_assert!(zeroed, "managed span slot window must stay writable");
        let copied = page_arena.write_window(&storage.pages, start, bytes);
        debug_assert!(copied, "managed span slot bytes must stay writable");
        Self::set_trace_metadata(
            &storage.occupied,
            &mut storage.trace_metadata,
            occupied,
            slot_count,
            slot,
            trace_id.index() as u32,
        );
        Self::set_layout_metadata(
            &storage.occupied,
            &mut storage.layout_metadata,
            occupied,
            slot_count,
            slot,
            StoredLayoutId::from_option(layout_id),
        );
        storage.occupied.set(slot);

        self.occupied_count += 1;
        self.next_free_slot = self
            .occupied()
            .first_clear_from(slot.saturating_add(1))
            .or_else(|| self.occupied().first_clear_from(0))
            .unwrap_or(self.slot_count);
        true
    }

    /// Write one byte inside one occupied slot.
    pub(crate) fn set_byte(
        &mut self,
        page_arena: &mut PageArena,
        slot: usize,
        byte_len: usize,
        offset: usize,
        byte: u8,
    ) -> bool {
        if slot >= self.slot_count
            || !self.is_occupied(slot)
            || offset >= byte_len
            || byte_len > self.size_class
        {
            return false;
        }

        let size_class = self.size_class;
        let storage = self.state_mut(page_arena);
        let index = slot * size_class + offset;
        page_arena.write_window(&storage.pages, index, &[byte])
    }

    /// Write one byte slice inside one occupied slot.
    pub(crate) fn set_bytes(
        &mut self,
        page_arena: &mut PageArena,
        slot: usize,
        byte_len: usize,
        offset: usize,
        bytes: &[u8],
    ) -> bool {
        if slot >= self.slot_count || !self.is_occupied(slot) || byte_len > self.size_class {
            return false;
        }
        let Some(end) = offset.checked_add(bytes.len()) else {
            return false;
        };

        if end > byte_len {
            return false;
        }

        let size_class = self.size_class;
        let storage = self.state_mut(page_arena);
        let start = slot * size_class + offset;
        page_arena.write_window(&storage.pages, start, bytes)
    }

    /// Copy one byte window out of this span payload.
    pub(crate) fn read_window(
        &self,
        page_arena: &PageArena,
        start: usize,
        dest: &mut [u8],
    ) -> bool {
        match &self.state {
            SpanState::Local(storage) => page_arena.read_window(&storage.pages, start, dest),
            SpanState::Shared(image) => Self::read_shared_window(image, start, dest),
        }
    }

    /// Free one occupied slot.
    pub(crate) fn free_slot(&mut self, page_arena: &mut PageArena, slot: usize) -> bool {
        if slot >= self.slot_count || !self.is_occupied(slot) {
            return false;
        }

        let size_class = self.size_class;
        {
            let storage = self.state_mut(page_arena);
            let start = slot * size_class;
            let zeroed = page_arena.fill_window(&storage.pages, start, size_class, 0);
            debug_assert!(zeroed, "managed span slot window must stay writable");
            Self::clear_trace_metadata(&mut storage.trace_metadata, slot);
            Self::clear_layout_metadata(&mut storage.layout_metadata, slot);
            storage.occupied.clear(slot);
        }

        self.marked.clear(slot);
        self.clear_pin_state(slot);
        self.occupied_count = self.occupied_count.saturating_sub(1);

        if self.occupied_count == 0 {
            let storage = self.state_mut(page_arena);
            storage.trace_metadata = SpanTraceData::Monomorphic(0);
            storage.layout_metadata = SpanLayoutData::Monomorphic(StoredLayoutId::none());
            self.next_free_slot = 0;
            self.dirty_cards.clear_all();
            self.is_dirty_queued = false;
        } else {
            self.next_free_slot = self.next_free_slot.min(slot);
        }

        true
    }

    /// Allocate one free slot in this span with zeroed payload bytes.
    pub(crate) fn allocate_zeroed_slot(
        &mut self,
        page_arena: &mut PageArena,
        slot: usize,
        byte_len: usize,
        trace_id: ReferenceMapId,
        layout_id: Option<LayoutId>,
    ) -> bool {
        if slot >= self.slot_count || byte_len > self.size_class || self.is_occupied(slot) {
            return false;
        }

        let size_class = self.size_class;
        let occupied = self.occupied_count;
        let slot_count = self.slot_count;
        let storage = self.state_mut(page_arena);
        let start = slot * size_class;
        let zeroed = page_arena.fill_window(&storage.pages, start, size_class, 0);
        debug_assert!(zeroed, "managed span slot window must stay writable");
        Self::set_trace_metadata(
            &storage.occupied,
            &mut storage.trace_metadata,
            occupied,
            slot_count,
            slot,
            trace_id.index() as u32,
        );
        Self::set_layout_metadata(
            &storage.occupied,
            &mut storage.layout_metadata,
            occupied,
            slot_count,
            slot,
            StoredLayoutId::from_option(layout_id),
        );
        storage.occupied.set(slot);

        self.occupied_count += 1;
        self.next_free_slot = self
            .occupied()
            .first_clear_from(slot.saturating_add(1))
            .or_else(|| self.occupied().first_clear_from(0))
            .unwrap_or(self.slot_count);
        true
    }

    /// Return the active pin count for this span.
    pub(crate) fn active_pins(&self) -> usize {
        self.active_pins
    }

    /// Mark one slot-relative byte range dirty for young tracking.
    pub(crate) fn mark_dirty_slot_range(&mut self, slot: usize, start: usize, len: usize) -> bool {
        if slot >= self.slot_count || !self.is_occupied(slot) {
            return false;
        }

        let slot_start = slot.saturating_mul(self.size_class);
        self.dirty_cards
            .mark_range(slot_start.saturating_add(start), len)
    }

    /// Mark one card dirty.
    pub(crate) fn mark_dirty_card(&mut self, card_index: usize) {
        self.dirty_cards.mark_card(card_index);
    }

    /// Clear one dirty card.
    pub(crate) fn clear_dirty_card(&mut self, card_index: usize) {
        self.dirty_cards.clear_card(card_index);
    }

    /// Return the first dirty card from the given index.
    pub(crate) fn first_dirty_card_from(&self, start: usize) -> Option<usize> {
        self.dirty_cards.first_dirty_from(start)
    }

    /// Return the byte start for the given card.
    pub(crate) fn dirty_card_start(&self, card_index: usize) -> usize {
        self.dirty_cards.card_start(card_index)
    }

    /// Return the byte length for the given card.
    pub(crate) fn dirty_card_len(&self, card_index: usize) -> usize {
        self.dirty_cards.card_len(card_index)
    }

    /// Report whether this span currently has dirty cards.
    pub(crate) fn has_dirty_cards(&self) -> bool {
        self.dirty_cards.has_dirty_cards()
    }

    /// Report whether this span is already queued for dirty-card scanning.
    pub(crate) fn is_dirty_queued(&self) -> bool {
        self.is_dirty_queued
    }

    /// Set whether this span is queued for dirty-card scanning.
    pub(crate) fn set_dirty_queued(&mut self, is_dirty_queued: bool) {
        self.is_dirty_queued = is_dirty_queued;
    }

    /// Increment the pin count for one occupied slot.
    #[cfg(test)]
    pub(crate) fn pin(&mut self, slot: usize) -> bool {
        if !self.is_occupied(slot) {
            return false;
        }

        let key = slot;
        if self.pinned.contains(slot) {
            match self
                .extra_pin_counts
                .iter_mut()
                .find(|(span_slot, _)| *span_slot == key)
            {
                Some((_, pin_count)) => *pin_count = pin_count.saturating_add(1),
                None => self.extra_pin_counts.push((key, 2)),
            }
        } else {
            self.pinned.set(slot);
        }

        self.active_pins = self.active_pins.saturating_add(1);
        true
    }

    /// Decrement the pin count for one occupied slot.
    #[cfg(test)]
    pub(crate) fn unpin(&mut self, slot: usize) -> bool {
        if !self.is_occupied(slot) || !self.pinned.contains(slot) {
            return false;
        }

        let key = slot;
        match self
            .extra_pin_counts
            .iter_mut()
            .find(|(span_slot, _)| *span_slot == key)
        {
            Some((_, pin_count)) if *pin_count > 2 => {
                *pin_count -= 1;
            }
            Some((_, pin_count)) if *pin_count == 2 => {
                self.extra_pin_counts
                    .retain(|(span_slot, _)| *span_slot != key);
            }
            _ => {
                self.pinned.clear(slot);
            }
        }

        self.active_pins = self.active_pins.saturating_sub(1);
        true
    }

    /// Capture one immutable span image.
    pub(crate) fn image(&mut self, page_arena: &mut PageArena) -> ManagedSpanImage {
        if self.is_vacant() {
            return ManagedSpanImage {
                size_class: 0,
                slot_count: 0,
                page_bytes: page_arena.page_bytes(),
                pages: Arc::from(Vec::<Arc<[u8]>>::new()),
                occupied: Bitmap::with_capacity(0),
                trace_metadata: SpanTraceImage::Monomorphic(0),
                layout_metadata: SpanLayoutImage::Monomorphic(StoredLayoutId::none()),
            };
        }

        match &mut self.state {
            SpanState::Shared(image) => image.clone(),
            SpanState::Local(storage) => {
                let image = ManagedSpanImage {
                    size_class: self.size_class,
                    slot_count: self.slot_count,
                    page_bytes: page_arena.page_bytes(),
                    pages: Arc::from(
                        storage
                            .pages
                            .iter()
                            .map(|&page_id| {
                                page_arena
                                    .image(page_id)
                                    .expect("managed span image pages must stay readable")
                            })
                            .collect::<Vec<_>>(),
                    ),
                    occupied: storage.occupied.clone(),
                    trace_metadata: Self::trace_image(&mut storage.trace_metadata),
                    layout_metadata: Self::layout_image(&mut storage.layout_metadata),
                };
                page_arena.free_pages(&storage.pages);
                storage.pages.clear();
                self.state = SpanState::Shared(image.clone());
                image
            }
        }
    }

    /// Return the active local bytes for this span.
    pub(crate) fn active_bytes(&self, page_bytes: usize) -> usize {
        if self.is_vacant() {
            return self.marked.retained_bytes()
                + self.pinned.retained_bytes()
                + self.extra_pin_counts.capacity() * std::mem::size_of::<(usize, u16)>()
                + self.dirty_cards.retained_bytes();
        }

        let side_bytes = self.marked.retained_bytes()
            + self.pinned.retained_bytes()
            + self.extra_pin_counts.capacity() * std::mem::size_of::<(usize, u16)>();

        match &self.state {
            SpanState::Local(storage) => {
                let _ = page_bytes;

                storage.pages.capacity() * size_of::<PageId>()
                    + storage.occupied.retained_bytes()
                    + Self::trace_metadata_retained_bytes(&storage.trace_metadata)
                    + Self::layout_metadata_retained_bytes(&storage.layout_metadata)
                    + self.dirty_cards.retained_bytes()
                    + side_bytes
            }
            SpanState::Shared(_) => self.dirty_cards.retained_bytes() + side_bytes,
        }
    }

    /// Return the active local bytes for one fresh local span.
    pub(crate) fn active_bytes_for_new(
        size_class: usize,
        span_bytes: usize,
        page_bytes: usize,
    ) -> usize {
        let slot_count = (span_bytes / size_class).max(1);
        let page_count = span_bytes.div_ceil(page_bytes).max(1);

        page_count * size_of::<PageId>()
            + Bitmap::with_capacity(slot_count).retained_bytes()
            + CardSet::with_len(slot_count * size_class).retained_bytes()
            + Bitmap::with_capacity(slot_count).retained_bytes()
            + Bitmap::with_capacity(slot_count).retained_bytes()
    }

    /// Return the active-byte reservation for allocating one slot.
    pub(crate) fn allocate_active_reservation(
        &self,
        page_bytes: usize,
        trace_id: ReferenceMapId,
        layout_id: Option<LayoutId>,
    ) -> i64 {
        match &self.state {
            SpanState::Local(storage) => {
                let trace_delta = Self::trace_metadata_insert_delta(
                    &storage.trace_metadata,
                    &storage.occupied,
                    self.occupied_count,
                    self.slot_count,
                    trace_id.index() as u32,
                );
                let layout_delta = Self::layout_metadata_insert_delta(
                    &storage.layout_metadata,
                    &storage.occupied,
                    self.occupied_count,
                    self.slot_count,
                    StoredLayoutId::from_option(layout_id),
                );

                trace_delta + layout_delta
            }
            SpanState::Shared(image) => {
                let page_count = image.pages.len();
                let local_pages_capacity = projected_vec_capacity::<PageId>(0, 0, page_count);
                let local_active = local_pages_capacity * size_of::<PageId>()
                    + image.occupied.retained_bytes()
                    + Self::trace_image_retained_bytes_after_insert(
                        &image.trace_metadata,
                        &image.occupied,
                        self.occupied_count,
                        self.slot_count,
                        trace_id.index() as u32,
                    )
                    + Self::layout_image_retained_bytes_after_insert(
                        &image.layout_metadata,
                        &image.occupied,
                        self.occupied_count,
                        self.slot_count,
                        StoredLayoutId::from_option(layout_id),
                    )
                    + self.dirty_cards.retained_bytes()
                    + self.marked.retained_bytes()
                    + self.pinned.retained_bytes()
                    + self.extra_pin_counts.capacity() * size_of::<(usize, u16)>();

                let _ = page_bytes;

                local_active as i64
            }
        }
    }

    /// Return the detached-page count and active-byte reservation for one write.
    pub(crate) fn write_active_reservation(&self, page_bytes: usize) -> (usize, i64) {
        match &self.state {
            SpanState::Local(_) => (0, 0),
            SpanState::Shared(image) => {
                let page_count = image.pages.len();
                let local_pages_capacity = projected_vec_capacity::<PageId>(0, 0, page_count);
                let local_active = local_pages_capacity * size_of::<PageId>()
                    + image.occupied.retained_bytes()
                    + Self::trace_image_retained_bytes(&image.trace_metadata)
                    + Self::layout_image_retained_bytes(&image.layout_metadata)
                    + self.dirty_cards.retained_bytes()
                    + self.marked.retained_bytes()
                    + self.pinned.retained_bytes()
                    + self.extra_pin_counts.capacity() * size_of::<(usize, u16)>();

                let _ = page_bytes;

                (page_count, local_active as i64)
            }
        }
    }

    /// Return the borrowed image bytes referenced by this span.
    pub(crate) fn borrowed_bytes(&self, page_bytes: usize) -> usize {
        if self.is_vacant() {
            return 0;
        }

        match &self.state {
            SpanState::Local(_) => 0,
            SpanState::Shared(image) => {
                let _ = page_bytes;

                image.pages.iter().map(|page| page.len()).sum::<usize>()
                    + image.occupied.retained_bytes()
                    + Self::trace_image_retained_bytes(&image.trace_metadata)
                    + Self::layout_image_retained_bytes(&image.layout_metadata)
            }
        }
    }

    /// Return the occupied bitmap for this span.
    fn occupied(&self) -> &Bitmap {
        match &self.state {
            SpanState::Local(storage) => &storage.occupied,
            SpanState::Shared(image) => &image.occupied,
        }
    }

    /// Return one shared span byte window, borrowing one page when possible.
    fn shared_bytes<'a>(
        image: &'a ManagedSpanImage,
        start: usize,
        byte_len: usize,
    ) -> Option<Cow<'a, [u8]>> {
        if byte_len == 0 {
            return Some(Cow::Borrowed(&[]));
        }

        let end = start.checked_add(byte_len)?;
        let start_page = start / image.page_bytes;
        let end_page = (end - 1) / image.page_bytes;

        if start_page == end_page {
            let page = image.pages.get(start_page)?;
            let page_offset = start % image.page_bytes;

            return page
                .get(page_offset..page_offset + byte_len)
                .map(Cow::Borrowed);
        }

        let mut bytes = vec![0; byte_len];
        let mut copied = 0usize;
        let mut offset = start;

        while copied < byte_len {
            let page_index = offset / image.page_bytes;
            let page_offset = offset % image.page_bytes;
            let page = image.pages.get(page_index)?;
            let remaining = byte_len - copied;
            let available = page.len().saturating_sub(page_offset);
            let count = remaining.min(available);
            if count == 0 {
                return None;
            }

            bytes[copied..copied + count].copy_from_slice(&page[page_offset..page_offset + count]);
            copied += count;
            offset += count;
        }

        Some(Cow::Owned(bytes))
    }

    /// Read one shared span byte window into the given destination.
    fn read_shared_window(image: &ManagedSpanImage, start: usize, dest: &mut [u8]) -> bool {
        if dest.is_empty() {
            return true;
        }

        let mut copied = 0usize;
        let mut offset = start;

        while copied < dest.len() {
            let page_index = offset / image.page_bytes;
            let page_offset = offset % image.page_bytes;
            let Some(page) = image.pages.get(page_index) else {
                return false;
            };

            let remaining = dest.len() - copied;
            let available = page.len().saturating_sub(page_offset);
            let count = remaining.min(available);
            if count == 0 {
                return false;
            }

            dest[copied..copied + count].copy_from_slice(&page[page_offset..page_offset + count]);
            copied += count;
            offset += count;
        }

        true
    }

    /// Clear one slot's pin state.
    fn clear_pin_state(&mut self, slot: usize) {
        if self.pinned.contains(slot) {
            self.pinned.clear(slot);
        }

        let key = slot;
        self.extra_pin_counts
            .retain(|(span_slot, _)| *span_slot != key);
    }

    /// Return mutable owned span storage, detaching images on first write.
    fn state_mut(&mut self, page_arena: &mut PageArena) -> &mut SpanData {
        if let SpanState::Shared(image) = &self.state {
            self.state = SpanState::Local(SpanData {
                pages: image
                    .pages
                    .iter()
                    .map(|page| page_arena.allocate(page.as_ref()))
                    .collect(),
                occupied: image.occupied.clone(),
                trace_metadata: Self::trace_owned(&image.trace_metadata),
                layout_metadata: Self::layout_owned(&image.layout_metadata),
            });
        }

        match &mut self.state {
            SpanState::Local(storage) => storage,
            SpanState::Shared(_) => unreachable!(),
        }
    }

    /// Release one owned page run before vacating this span slot.
    pub(crate) fn release(&mut self, page_arena: &mut PageArena) {
        if let SpanState::Local(storage) = &mut self.state {
            page_arena.free_pages(&storage.pages);
            storage.pages.clear();
        }
    }

    /// Convert one shared trace image into owned storage.
    fn trace_owned(trace_metadata: &SpanTraceImage) -> SpanTraceData {
        match trace_metadata {
            SpanTraceImage::Monomorphic(trace_id) => SpanTraceData::Monomorphic(*trace_id),
            SpanTraceImage::Polymorphic(trace_ids) => {
                SpanTraceData::Polymorphic(trace_ids.as_ref().to_vec().into_boxed_slice())
            }
        }
    }

    /// Convert one shared layout image into owned storage.
    fn layout_owned(layout_metadata: &SpanLayoutImage) -> SpanLayoutData {
        match layout_metadata {
            SpanLayoutImage::Monomorphic(layout_id) => SpanLayoutData::Monomorphic(*layout_id),
            SpanLayoutImage::Polymorphic(layout_ids) => {
                SpanLayoutData::Polymorphic(layout_ids.as_ref().to_vec().into_boxed_slice())
            }
        }
    }

    /// Convert one owned trace storage into one shared image.
    fn trace_image(trace_metadata: &mut SpanTraceData) -> SpanTraceImage {
        match trace_metadata {
            SpanTraceData::Monomorphic(trace_id) => SpanTraceImage::Monomorphic(*trace_id),
            SpanTraceData::Polymorphic(trace_ids) => {
                SpanTraceImage::Polymorphic(Arc::from(std::mem::take(trace_ids)))
            }
        }
    }

    /// Convert one owned layout storage into one shared image.
    fn layout_image(layout_metadata: &mut SpanLayoutData) -> SpanLayoutImage {
        match layout_metadata {
            SpanLayoutData::Monomorphic(layout_id) => SpanLayoutImage::Monomorphic(*layout_id),
            SpanLayoutData::Polymorphic(layout_ids) => {
                SpanLayoutImage::Polymorphic(Arc::from(std::mem::take(layout_ids)))
            }
        }
    }

    /// Return the retained bytes owned by one trace metadata shape.
    fn trace_metadata_retained_bytes(trace_metadata: &SpanTraceData) -> usize {
        match trace_metadata {
            SpanTraceData::Monomorphic(_) => 0,
            SpanTraceData::Polymorphic(trace_ids) => trace_ids.len() * std::mem::size_of::<u32>(),
        }
    }

    /// Return the retained bytes owned by one layout metadata shape.
    fn layout_metadata_retained_bytes(layout_metadata: &SpanLayoutData) -> usize {
        match layout_metadata {
            SpanLayoutData::Monomorphic(_) => 0,
            SpanLayoutData::Polymorphic(layout_ids) => {
                layout_ids.len() * std::mem::size_of::<StoredLayoutId>()
            }
        }
    }

    /// Return the retained bytes owned by one trace metadata image.
    fn trace_image_retained_bytes(trace_metadata: &SpanTraceImage) -> usize {
        match trace_metadata {
            SpanTraceImage::Monomorphic(_) => 0,
            SpanTraceImage::Polymorphic(trace_ids) => trace_ids.len() * std::mem::size_of::<u32>(),
        }
    }

    /// Return the retained bytes owned by one layout metadata image.
    fn layout_image_retained_bytes(layout_metadata: &SpanLayoutImage) -> usize {
        match layout_metadata {
            SpanLayoutImage::Monomorphic(_) => 0,
            SpanLayoutImage::Polymorphic(layout_ids) => {
                layout_ids.len() * std::mem::size_of::<StoredLayoutId>()
            }
        }
    }

    /// Return the retained-byte delta for inserting one trace id into owned metadata.
    fn trace_metadata_insert_delta(
        trace_metadata: &SpanTraceData,
        occupied: &Bitmap,
        occupied_count: usize,
        slot_count: usize,
        trace_id: u32,
    ) -> i64 {
        let before = Self::trace_metadata_retained_bytes(trace_metadata) as i64;
        let after = Self::trace_metadata_retained_bytes_after_insert(
            trace_metadata,
            occupied,
            occupied_count,
            slot_count,
            trace_id,
        ) as i64;

        after - before
    }

    /// Return the retained bytes for owned trace metadata after one insert.
    fn trace_metadata_retained_bytes_after_insert(
        trace_metadata: &SpanTraceData,
        occupied: &Bitmap,
        occupied_count: usize,
        slot_count: usize,
        trace_id: u32,
    ) -> usize {
        let _ = occupied;

        match trace_metadata {
            SpanTraceData::Monomorphic(current) if occupied_count == 0 || *current == trace_id => 0,
            SpanTraceData::Monomorphic(_) => slot_count * size_of::<u32>(),
            SpanTraceData::Polymorphic(trace_ids) => trace_ids.len() * size_of::<u32>(),
        }
    }

    /// Return the retained-byte delta for inserting one layout id into owned metadata.
    fn layout_metadata_insert_delta(
        layout_metadata: &SpanLayoutData,
        occupied: &Bitmap,
        occupied_count: usize,
        slot_count: usize,
        layout_id: StoredLayoutId,
    ) -> i64 {
        let before = Self::layout_metadata_retained_bytes(layout_metadata) as i64;
        let after = Self::layout_metadata_retained_bytes_after_insert(
            layout_metadata,
            occupied,
            occupied_count,
            slot_count,
            layout_id,
        ) as i64;

        after - before
    }

    /// Return the retained bytes for owned layout metadata after one insert.
    fn layout_metadata_retained_bytes_after_insert(
        layout_metadata: &SpanLayoutData,
        occupied: &Bitmap,
        occupied_count: usize,
        slot_count: usize,
        layout_id: StoredLayoutId,
    ) -> usize {
        let _ = occupied;

        match layout_metadata {
            SpanLayoutData::Monomorphic(current)
                if occupied_count == 0 || *current == layout_id =>
            {
                0
            }
            SpanLayoutData::Monomorphic(_) => slot_count * size_of::<StoredLayoutId>(),
            SpanLayoutData::Polymorphic(layout_ids) => {
                layout_ids.len() * size_of::<StoredLayoutId>()
            }
        }
    }

    /// Return the retained bytes for shared trace metadata after one insert.
    fn trace_image_retained_bytes_after_insert(
        trace_metadata: &SpanTraceImage,
        occupied: &Bitmap,
        occupied_count: usize,
        slot_count: usize,
        trace_id: u32,
    ) -> usize {
        let _ = occupied;

        match trace_metadata {
            SpanTraceImage::Monomorphic(current) if occupied_count == 0 || *current == trace_id => {
                0
            }
            SpanTraceImage::Monomorphic(_) => slot_count * size_of::<u32>(),
            SpanTraceImage::Polymorphic(trace_ids) => trace_ids.len() * size_of::<u32>(),
        }
    }

    /// Return the retained bytes for shared layout metadata after one insert.
    fn layout_image_retained_bytes_after_insert(
        layout_metadata: &SpanLayoutImage,
        occupied: &Bitmap,
        occupied_count: usize,
        slot_count: usize,
        layout_id: StoredLayoutId,
    ) -> usize {
        let _ = occupied;

        match layout_metadata {
            SpanLayoutImage::Monomorphic(current)
                if occupied_count == 0 || *current == layout_id =>
            {
                0
            }
            SpanLayoutImage::Monomorphic(_) => slot_count * size_of::<StoredLayoutId>(),
            SpanLayoutImage::Polymorphic(layout_ids) => {
                layout_ids.len() * size_of::<StoredLayoutId>()
            }
        }
    }

    /// Write one trace id into the span metadata.
    fn set_trace_metadata(
        occupied: &Bitmap,
        trace_metadata: &mut SpanTraceData,
        occupied_count: usize,
        slot_count: usize,
        slot: usize,
        trace_id: u32,
    ) {
        match trace_metadata {
            SpanTraceData::Monomorphic(current) if occupied_count == 0 || *current == trace_id => {
                *current = trace_id;
            }
            SpanTraceData::Monomorphic(current) => {
                let mut trace_ids = vec![0; slot_count].into_boxed_slice();

                for occupied_slot in 0..slot_count {
                    if occupied.contains(occupied_slot) {
                        trace_ids[occupied_slot] = *current;
                    }
                }

                trace_ids[slot] = trace_id;
                *trace_metadata = SpanTraceData::Polymorphic(trace_ids);
            }
            SpanTraceData::Polymorphic(trace_ids) => {
                trace_ids[slot] = trace_id;
            }
        }
    }

    /// Write one layout id into the span metadata.
    fn set_layout_metadata(
        occupied: &Bitmap,
        layout_metadata: &mut SpanLayoutData,
        occupied_count: usize,
        slot_count: usize,
        slot: usize,
        layout_id: StoredLayoutId,
    ) {
        match layout_metadata {
            SpanLayoutData::Monomorphic(current)
                if occupied_count == 0 || *current == layout_id =>
            {
                *current = layout_id;
            }
            SpanLayoutData::Monomorphic(current) => {
                let mut layout_ids = vec![StoredLayoutId::none(); slot_count].into_boxed_slice();

                for occupied_slot in 0..slot_count {
                    if occupied.contains(occupied_slot) {
                        layout_ids[occupied_slot] = *current;
                    }
                }

                layout_ids[slot] = layout_id;
                *layout_metadata = SpanLayoutData::Polymorphic(layout_ids);
            }
            SpanLayoutData::Polymorphic(layout_ids) => {
                layout_ids[slot] = layout_id;
            }
        }
    }

    /// Clear one slot's trace metadata.
    fn clear_trace_metadata(trace_metadata: &mut SpanTraceData, slot: usize) {
        if let SpanTraceData::Polymorphic(trace_ids) = trace_metadata {
            trace_ids[slot] = 0;
        }
    }

    /// Clear one slot's layout metadata.
    fn clear_layout_metadata(layout_metadata: &mut SpanLayoutData, slot: usize) {
        if let SpanLayoutData::Polymorphic(layout_ids) = layout_metadata {
            layout_ids[slot] = StoredLayoutId::none();
        }
    }
}
