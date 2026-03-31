use std::borrow::Cow;
use std::rc::Rc;
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::alloc::{Bitmap, PageArena, PageId};

/// One immutable raw span image.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RawSpanImage {
    /// The size class for this span in bytes.
    pub size_class: usize,
    /// The number of slots in this span.
    pub slot_count: usize,
    /// The fixed byte width for every page in this span image.
    pub page_bytes: usize,
    /// The packed slot payload pages.
    pub pages: Arc<[Rc<[u8]>]>,
    /// The occupied slots in this span.
    pub occupied: Bitmap,
    /// The logical byte length for each slot.
    pub lengths: Arc<[u16]>,
}

impl RawSpanImage {
    /// Report whether this span image is one vacant directory slot.
    pub(crate) fn is_vacant(&self) -> bool {
        self.slot_count == 0
    }

    /// Report whether this span image shares durable backing with another span image.
    pub(crate) fn shares_storage_with(&self, other: &Self) -> bool {
        self.size_class == other.size_class
            && self.slot_count == other.slot_count
            && self.page_bytes == other.page_bytes
            && self.pages.len() == other.pages.len()
            && self
                .pages
                .iter()
                .zip(other.pages.iter())
                .all(|(left, right)| Rc::ptr_eq(left, right))
            && self.occupied == other.occupied
            && Arc::ptr_eq(&self.lengths, &other.lengths)
    }
}

/// One owned raw span payload.
#[derive(Debug, Clone, PartialEq, Eq)]
struct SpanData {
    /// The packed slot payload pages.
    pages: Vec<PageId>,
    /// The occupied slots in this span.
    occupied: Bitmap,
    /// The logical byte length for each slot.
    lengths: Box<[u16]>,
}

/// One live raw span storage state.
#[derive(Debug, Clone, PartialEq, Eq)]
enum SpanState {
    /// Owned mutable span storage.
    Local(SpanData),
    /// Shared immutable span storage.
    Shared(RawSpanImage),
}

/// One live raw span for one size class.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RawSpan {
    /// The slot payload size in bytes.
    size_class: usize,
    /// The number of slots in this span.
    slot_count: usize,
    /// The number of occupied slots in this span.
    occupied_count: usize,
    /// The next likely free slot.
    next_free_slot: usize,
    /// The live storage for this span.
    state: SpanState,
}

impl RawSpan {
    /// Create one vacant raw span slot with no backing storage.
    pub(crate) fn vacant() -> Self {
        Self {
            size_class: 0,
            slot_count: 0,
            occupied_count: 0,
            next_free_slot: 0,
            state: SpanState::Local(SpanData {
                pages: Vec::new(),
                occupied: Bitmap::with_capacity(0),
                lengths: Vec::new().into_boxed_slice(),
            }),
        }
    }

    /// Create one empty raw span for the given size class and span byte width.
    pub(crate) fn new(size_class: usize, span_bytes: usize, page_arena: &mut PageArena) -> Self {
        let slot_count = (span_bytes / size_class).max(1);
        let pages = page_arena.allocate_pages(&[], slot_count * size_class);
        let lengths = vec![0; slot_count].into_boxed_slice();

        Self {
            size_class,
            slot_count,
            occupied_count: 0,
            next_free_slot: 0,
            state: SpanState::Local(SpanData {
                pages,
                occupied: Bitmap::with_capacity(slot_count),
                lengths,
            }),
        }
    }

    /// Restore one raw span from one immutable image.
    pub(crate) fn from_image(image: RawSpanImage) -> Self {
        if image.is_vacant() {
            return Self::vacant();
        }

        let occupied_count = image.occupied.count_ones();
        let slot_count = image.slot_count;
        let next_free_slot = image.occupied.first_clear_from(0).unwrap_or(slot_count);

        Self {
            size_class: image.size_class,
            slot_count,
            occupied_count,
            next_free_slot,
            state: SpanState::Shared(image),
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

    /// Return whether one slot is occupied.
    pub(crate) fn is_occupied(&self, slot: usize) -> bool {
        self.occupied().contains(slot)
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

    /// Return the logical byte length for one occupied slot.
    pub(crate) fn len(&self, slot: usize) -> Option<usize> {
        if slot >= self.slot_count || !self.is_occupied(slot) {
            return None;
        }

        match &self.state {
            SpanState::Local(storage) => storage.lengths.get(slot).copied(),
            SpanState::Shared(image) => image.lengths.get(slot).copied(),
        }
        .map(|len| len as usize)
    }

    /// Return the bytes for one occupied slot.
    pub(crate) fn bytes<'a>(
        &'a self,
        page_arena: &'a PageArena,
        slot: usize,
    ) -> Option<Cow<'a, [u8]>> {
        let len = self.len(slot)?;
        let start = slot.checked_mul(self.size_class)?;
        match &self.state {
            SpanState::Local(storage) => {
                if let Some(bytes) = page_arena.borrow_window(&storage.pages, start, len) {
                    Some(Cow::Borrowed(bytes))
                } else {
                    let mut bytes = vec![0; len];
                    let read = page_arena.read_window(&storage.pages, start, &mut bytes);
                    if !read {
                        return None;
                    }

                    Some(Cow::Owned(bytes))
                }
            }
            SpanState::Shared(image) => Self::shared_bytes(image, start, len),
        }
    }

    /// Allocate one free slot in this span.
    pub(crate) fn allocate_slot(
        &mut self,
        page_arena: &mut PageArena,
        slot: usize,
        bytes: &[u8],
    ) -> bool {
        if slot >= self.slot_count || bytes.len() > self.size_class || self.is_occupied(slot) {
            return false;
        }

        let size_class = self.size_class;
        let storage = self.state_mut(page_arena);
        let start = slot * size_class;
        let zeroed = page_arena.fill_window(&storage.pages, start, size_class, 0);
        debug_assert!(zeroed, "raw span slot window must stay writable");
        let copied = page_arena.write_window(&storage.pages, start, bytes);
        debug_assert!(copied, "raw span slot bytes must stay writable");
        storage.lengths[slot] = bytes.len() as u16;
        storage.occupied.set(slot);
        self.occupied_count += 1;
        self.next_free_slot = self
            .occupied()
            .first_clear_from(slot.saturating_add(1))
            .or_else(|| self.occupied().first_clear_from(0))
            .unwrap_or(self.slot_count);
        true
    }

    /// Allocate one free zeroed slot in this span.
    pub(crate) fn allocate_zeroed_slot(
        &mut self,
        page_arena: &mut PageArena,
        slot: usize,
        byte_len: usize,
    ) -> bool {
        if slot >= self.slot_count || byte_len > self.size_class || self.is_occupied(slot) {
            return false;
        }

        let size_class = self.size_class;
        let storage = self.state_mut(page_arena);
        let start = slot * size_class;
        let zeroed = page_arena.fill_window(&storage.pages, start, size_class, 0);
        debug_assert!(zeroed, "raw span slot window must stay writable");
        storage.lengths[slot] = byte_len as u16;
        storage.occupied.set(slot);
        self.occupied_count += 1;
        self.next_free_slot = self
            .occupied()
            .first_clear_from(slot.saturating_add(1))
            .or_else(|| self.occupied().first_clear_from(0))
            .unwrap_or(self.slot_count);
        true
    }

    /// Replace one occupied slot.
    pub(crate) fn replace_slot(
        &mut self,
        page_arena: &mut PageArena,
        slot: usize,
        bytes: &[u8],
    ) -> bool {
        if slot >= self.slot_count || !self.is_occupied(slot) || bytes.len() > self.size_class {
            return false;
        }

        let size_class = self.size_class;
        let storage = self.state_mut(page_arena);
        let start = slot * size_class;
        let zeroed = page_arena.fill_window(&storage.pages, start, size_class, 0);
        debug_assert!(zeroed, "raw span slot window must stay writable");
        let copied = page_arena.write_window(&storage.pages, start, bytes);
        debug_assert!(copied, "raw span slot bytes must stay writable");
        storage.lengths[slot] = bytes.len() as u16;
        true
    }

    /// Write one byte inside one occupied slot.
    pub(crate) fn set_byte(
        &mut self,
        page_arena: &mut PageArena,
        slot: usize,
        offset: usize,
        byte: u8,
    ) -> bool {
        let Some(len) = self.len(slot) else {
            return false;
        };

        if offset >= len {
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
        offset: usize,
        bytes: &[u8],
    ) -> bool {
        let Some(len) = self.len(slot) else {
            return false;
        };
        let Some(end) = offset.checked_add(bytes.len()) else {
            return false;
        };

        if end > len {
            return false;
        }

        let size_class = self.size_class;
        let storage = self.state_mut(page_arena);
        let start = slot * size_class + offset;
        page_arena.write_window(&storage.pages, start, bytes)
    }

    /// Free one occupied slot.
    pub(crate) fn free_slot(&mut self, page_arena: &mut PageArena, slot: usize) -> bool {
        if slot >= self.slot_count || !self.is_occupied(slot) {
            return false;
        }

        let size_class = self.size_class;
        let storage = self.state_mut(page_arena);
        let start = slot * size_class;
        let zeroed = page_arena.fill_window(&storage.pages, start, size_class, 0);
        debug_assert!(zeroed, "raw span slot window must stay writable");
        storage.lengths[slot] = 0;
        storage.occupied.clear(slot);
        self.occupied_count = self.occupied_count.saturating_sub(1);

        if self.occupied_count == 0 {
            self.next_free_slot = 0;
        } else {
            self.next_free_slot = self.next_free_slot.min(slot);
        }

        true
    }

    /// Capture one immutable span image.
    pub(crate) fn image(&mut self, page_arena: &mut PageArena) -> RawSpanImage {
        if self.is_vacant() {
            return RawSpanImage {
                size_class: 0,
                slot_count: 0,
                page_bytes: page_arena.page_bytes(),
                pages: Arc::from(Vec::<Rc<[u8]>>::new()),
                occupied: Bitmap::with_capacity(0),
                lengths: Arc::from(Vec::<u16>::new().into_boxed_slice()),
            };
        }

        match &mut self.state {
            SpanState::Shared(image) => image.clone(),
            SpanState::Local(storage) => {
                let image = RawSpanImage {
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
                                    .expect("raw span image pages must stay readable")
                            })
                            .collect::<Vec<_>>(),
                    ),
                    occupied: storage.occupied.clone(),
                    lengths: Arc::from(std::mem::take(&mut storage.lengths)),
                };
                page_arena.free_pages(&storage.pages);
                storage.pages.clear();
                self.state = SpanState::Shared(image.clone());
                image
            }
        }
    }

    /// Return the retained bytes for this span.
    pub(crate) fn retained_bytes(&self, page_bytes: usize) -> usize {
        match &self.state {
            SpanState::Local(storage) => {
                storage.pages.len() * page_bytes
                    + storage.lengths.len() * std::mem::size_of::<u16>()
                    + storage.occupied.retained_bytes()
            }
            SpanState::Shared(image) => {
                image.pages.iter().map(|page| page.len()).sum::<usize>()
                    + image.lengths.len() * std::mem::size_of::<u16>()
                    + image.occupied.retained_bytes()
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
                lengths: image.lengths.as_ref().to_vec().into_boxed_slice(),
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

    /// Return one shared span byte window, borrowing one page when possible.
    fn shared_bytes<'a>(
        image: &'a RawSpanImage,
        start: usize,
        len: usize,
    ) -> Option<Cow<'a, [u8]>> {
        if len == 0 {
            return Some(Cow::Borrowed(&[]));
        }

        let end = start.checked_add(len)?;
        let start_page = start / image.page_bytes;
        let end_page = (end - 1) / image.page_bytes;

        if start_page == end_page {
            let page = image.pages.get(start_page)?;
            let page_offset = start % image.page_bytes;

            return page.get(page_offset..page_offset + len).map(Cow::Borrowed);
        }

        let mut bytes = vec![0; len];
        let mut copied = 0usize;
        let mut offset = start;

        while copied < len {
            let page_index = offset / image.page_bytes;
            let page_offset = offset % image.page_bytes;
            let page = image.pages.get(page_index)?;
            let remaining = len - copied;
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
}
