use std::borrow::Cow;
use std::mem::size_of;
use std::rc::Rc;

use destack_mir::LayoutId;
use serde::{Deserialize, Serialize};

use super::{ReferenceMapId, StoredLayoutId};
use crate::alloc::{CardSet, ChunkPayload, PageArena};
use crate::heap::ImageAccounting;

/// One immutable managed large-allocation image.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManagedLargeAllocationImage {
    /// Whether this large-allocation slot is currently allocated.
    pub is_allocated: bool,
    /// The logical byte length of this large allocation.
    pub len: usize,
    /// The page width used by this large allocation.
    pub page_bytes: usize,
    /// The immutable large-allocation chunks.
    pub(crate) chunks: Vec<Rc<[u8]>>,
    /// The interned reference map for this large allocation.
    pub trace_id: ReferenceMapId,
    /// The durable layout id for this large allocation, if any.
    pub layout_id: Option<LayoutId>,
}

impl ManagedLargeAllocationImage {
    /// Report whether this large-allocation image shares durable backing with another large-allocation image.
    pub fn shares_storage_with(&self, other: &Self) -> bool {
        self.is_allocated == other.is_allocated
            && self.len == other.len
            && self.page_bytes == other.page_bytes
            && self.trace_id == other.trace_id
            && self.layout_id == other.layout_id
            && self.chunks.len() == other.chunks.len()
            && self
                .chunks
                .iter()
                .zip(other.chunks.iter())
                .all(|(left, right)| Rc::ptr_eq(left, right))
    }

    /// Return the exact owned bytes for this durable large-allocation image.
    pub fn image_bytes(&self) -> usize {
        let mut image_bytes = size_of::<Self>();
        image_bytes += self.chunks.len() * size_of::<Rc<[u8]>>();

        for chunk in self.chunks.iter() {
            image_bytes += chunk.len();
        }

        image_bytes
    }

    /// Account this large-allocation image into deduplicated retained-image bytes.
    pub fn retained_image_bytes(&self, accounting: &mut ImageAccounting) -> usize {
        let mut image_bytes = size_of::<Self>();
        image_bytes += self.chunks.len() * size_of::<Rc<[u8]>>();

        for chunk in self.chunks.iter() {
            image_bytes += accounting.account_rc_bytes(chunk);
        }

        image_bytes
    }
}

/// One stable managed large-allocation identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct ManagedLargeAllocationId(u64);

impl ManagedLargeAllocationId {
    /// Create one managed large-allocation identifier.
    pub(crate) const fn new(id: u64) -> Self {
        Self(id)
    }

    /// Return the managed large-allocation identifier value.
    pub(crate) const fn id(self) -> u64 {
        self.0
    }
}

/// One managed large allocation for large or dynamic payloads.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ManagedLargeAllocation {
    /// Whether this large-allocation slot is currently allocated.
    is_allocated: bool,
    /// The large-allocation payload.
    payload: ChunkPayload,
    /// The interned reference map for this large allocation.
    trace_id: ReferenceMapId,
    /// The durable layout id for this large allocation, if any.
    layout_id: StoredLayoutId,
    /// The live mark state for this large allocation.
    marked: bool,
    /// The active pin count for this large allocation.
    pin_count: u16,
    /// The dirty cards remembered for young tracing.
    dirty_cards: CardSet,
    /// Whether this large allocation is already queued for dirty-card scanning.
    is_dirty_queued: bool,
}

impl ManagedLargeAllocation {
    /// Create one managed large allocation from the given bytes and metadata.
    pub(crate) fn new(
        bytes: &[u8],
        page_bytes: usize,
        trace_id: ReferenceMapId,
        layout_id: Option<LayoutId>,
        page_arena: &mut PageArena,
    ) -> Self {
        Self {
            is_allocated: true,
            payload: ChunkPayload::new(bytes, page_bytes, page_arena),
            trace_id,
            layout_id: StoredLayoutId::from_option(layout_id),
            marked: false,
            pin_count: 0,
            dirty_cards: CardSet::with_len(bytes.len()),
            is_dirty_queued: false,
        }
    }

    /// Create one zeroed managed large allocation with the given byte length and metadata.
    pub(crate) fn new_zeroed(
        byte_len: usize,
        page_bytes: usize,
        trace_id: ReferenceMapId,
        layout_id: Option<LayoutId>,
        page_arena: &mut PageArena,
    ) -> Self {
        Self {
            is_allocated: true,
            payload: ChunkPayload::new_zeroed(byte_len, page_bytes, page_arena),
            trace_id,
            layout_id: StoredLayoutId::from_option(layout_id),
            marked: false,
            pin_count: 0,
            dirty_cards: CardSet::with_len(byte_len),
            is_dirty_queued: false,
        }
    }

    /// Restore one managed large allocation from one immutable large-allocation image.
    pub(crate) fn from_large_allocation_image(image: &ManagedLargeAllocationImage) -> Self {
        Self {
            is_allocated: image.is_allocated,
            payload: ChunkPayload::from_image(
                image.len,
                image.page_bytes,
                image.chunks.iter().cloned(),
            ),
            trace_id: image.trace_id,
            layout_id: StoredLayoutId::from_option(image.layout_id),
            marked: false,
            pin_count: 0,
            dirty_cards: CardSet::with_len(image.len),
            is_dirty_queued: false,
        }
    }

    /// Return whether this large-allocation slot is currently allocated.
    pub(crate) fn is_allocated(&self) -> bool {
        self.is_allocated
    }

    /// Return the large-allocation bytes as borrowed or materialized storage.
    pub(crate) fn bytes<'a>(&'a self, page_arena: &'a PageArena) -> Cow<'a, [u8]> {
        self.payload.bytes(page_arena)
    }

    /// Copy one byte window out of this large allocation into the provided buffer.
    pub(crate) fn read_window(
        &self,
        page_arena: &PageArena,
        start: usize,
        dest: &mut [u8],
    ) -> bool {
        self.payload.read_window(page_arena, start, dest)
    }

    /// Return the trace id for this large allocation.
    pub(crate) fn trace_id(&self) -> ReferenceMapId {
        self.trace_id
    }

    /// Return the layout id for this large allocation, if any.
    pub(crate) fn layout_id(&self) -> Option<LayoutId> {
        self.layout_id.to_option()
    }

    /// Set the layout id for this large allocation.
    #[cfg(test)]
    pub(crate) fn set_layout_id(&mut self, layout_id: LayoutId) {
        self.layout_id = StoredLayoutId::from_option(Some(layout_id));
    }

    /// Report whether this large allocation is marked.
    pub(crate) fn is_marked(&self) -> bool {
        self.marked
    }

    /// Mark this large allocation.
    pub(crate) fn mark(&mut self) {
        self.marked = true;
    }

    /// Clear the mark state for this large allocation.
    pub(crate) fn clear_mark(&mut self) {
        self.marked = false;
    }

    /// Return the active pin count for this large allocation.
    pub(crate) fn active_pins(&self) -> usize {
        self.pin_count as usize
    }

    /// Increment the pin count for this large allocation.
    #[cfg(test)]
    pub(crate) fn pin(&mut self) -> bool {
        if !self.is_allocated {
            return false;
        }

        self.pin_count = self.pin_count.saturating_add(1);
        true
    }

    /// Decrement the pin count for this large allocation.
    #[cfg(test)]
    pub(crate) fn unpin(&mut self) -> bool {
        if !self.is_allocated || self.pin_count == 0 {
            return false;
        }

        self.pin_count -= 1;
        true
    }

    /// Write one byte by index.
    pub(crate) fn set(&mut self, page_arena: &mut PageArena, index: usize, byte: u8) -> bool {
        self.payload.set(page_arena, index, byte)
    }

    /// Write one byte slice by range.
    pub(crate) fn set_bytes(
        &mut self,
        page_arena: &mut PageArena,
        start: usize,
        bytes: &[u8],
    ) -> bool {
        self.payload.write_window(page_arena, start, bytes)
    }

    /// Mark one byte range dirty for young tracking.
    pub(crate) fn mark_dirty_range(&mut self, start: usize, len: usize) -> bool {
        self.dirty_cards.mark_range(start, len)
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

    /// Report whether this large allocation currently has dirty cards.
    pub(crate) fn has_dirty_cards(&self) -> bool {
        self.dirty_cards.has_dirty_cards()
    }

    /// Report whether this large allocation is queued for dirty-card scanning.
    pub(crate) fn is_dirty_queued(&self) -> bool {
        self.is_dirty_queued
    }

    /// Set whether this large allocation is queued for dirty-card scanning.
    pub(crate) fn set_dirty_queued(&mut self, is_dirty_queued: bool) {
        self.is_dirty_queued = is_dirty_queued;
    }

    /// Replace the entire large-allocation payload and metadata.
    pub(crate) fn replace(
        &mut self,
        bytes: &[u8],
        page_bytes: usize,
        trace_id: ReferenceMapId,
        layout_id: Option<LayoutId>,
        page_arena: &mut PageArena,
    ) {
        self.is_allocated = true;
        self.payload.replace(page_arena, bytes, page_bytes);
        self.trace_id = trace_id;
        self.layout_id = StoredLayoutId::from_option(layout_id);
        self.marked = false;
        self.pin_count = 0;
        self.dirty_cards = CardSet::with_len(bytes.len());
        self.is_dirty_queued = false;
    }

    /// Replace the entire large-allocation payload with zeroed bytes and new metadata.
    pub(crate) fn replace_zeroed(
        &mut self,
        byte_len: usize,
        page_bytes: usize,
        trace_id: ReferenceMapId,
        layout_id: Option<LayoutId>,
        page_arena: &mut PageArena,
    ) {
        self.is_allocated = true;
        self.payload
            .replace_zeroed(page_arena, byte_len, page_bytes);
        self.trace_id = trace_id;
        self.layout_id = StoredLayoutId::from_option(layout_id);
        self.marked = false;
        self.pin_count = 0;
        self.dirty_cards = CardSet::with_len(byte_len);
        self.is_dirty_queued = false;
    }

    /// Free this large-allocation slot while keeping its stable id alive.
    pub(crate) fn free(&mut self, page_arena: &mut PageArena) {
        self.is_allocated = false;
        self.payload.clear(page_arena);
        self.trace_id = ReferenceMapId::new(0);
        self.layout_id = StoredLayoutId::none();
        self.marked = false;
        self.pin_count = 0;
        self.dirty_cards = CardSet::with_len(0);
        self.is_dirty_queued = false;
    }

    /// Capture one immutable large-allocation image.
    pub(crate) fn large_allocation_image(
        &mut self,
        page_arena: &mut PageArena,
    ) -> ManagedLargeAllocationImage {
        ManagedLargeAllocationImage {
            is_allocated: self.is_allocated,
            len: self.payload.len(),
            page_bytes: self.payload.page_bytes(),
            chunks: self.payload.image(page_arena),
            trace_id: self.trace_id,
            layout_id: self.layout_id.to_option(),
        }
    }

    /// Return the active local bytes for this large allocation.
    pub(crate) fn active_bytes(&self) -> usize {
        self.payload.active_bytes() + self.dirty_cards.retained_bytes()
    }

    /// Return the detached-page count and active-byte reservation for one write.
    pub(crate) fn write_active_reservation(&self, start: usize, len: usize) -> (usize, i64) {
        self.payload.write_page_reservation(start, len)
    }

    /// Return the borrowed image bytes referenced by this large allocation.
    pub(crate) fn borrowed_bytes(&self) -> usize {
        self.payload.borrowed_bytes()
    }
}
