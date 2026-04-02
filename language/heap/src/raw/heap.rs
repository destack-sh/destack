use std::borrow::Cow;
use std::cell::Cell;
use std::mem::size_of;
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use super::{RawImage, RawLargeAllocation, RawLargeAllocationId, RawSpan};
use crate::alloc::{
    ChunkPayload, PageArena, SizeClassTable, projected_vec_capacity, vec_capacity_bytes_delta,
};
use crate::heap::{HeapLayoutOptions, RawSpaceUsage};
use crate::value::{RawPointer, Value};

/// The first non-null raw allocation id.
const FIRST_ALLOCATED_RAW_ID: u64 = 1;

/// The first non-null raw large-allocation id.
const FIRST_ALLOCATED_LARGE_ALLOCATION_ID: u64 = 1;

/// One stable raw span slot location.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RawSpanSlot {
    /// The containing span index.
    span_index: u32,
    /// The slot index inside the span.
    slot_index: u32,
}

impl RawSpanSlot {
    /// Create one raw span slot location.
    pub(crate) const fn new(span_index: usize, slot_index: usize) -> Self {
        Self {
            span_index: span_index as u32,
            slot_index: slot_index as u32,
        }
    }

    /// Return the containing span index.
    pub(crate) const fn span_index(self) -> usize {
        self.span_index as usize
    }

    /// Return the slot index inside the span.
    pub(crate) const fn slot_index(self) -> usize {
        self.slot_index as usize
    }
}

/// One stable raw allocation location.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RawLocation {
    /// One vacant directory slot.
    Vacant,
    /// One small-space allocation stored in one span slot.
    Small(RawSpanSlot),
    /// One large-space allocation stored in one raw large allocation.
    Large(RawLargeAllocationId),
}

/// One live raw handle entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RawHandle {
    /// The storage location for this raw allocation.
    pub(crate) location: RawLocation,
    /// The logical byte length for this raw allocation.
    pub(crate) byte_len: usize,
}

/// One raw handle-table entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RawHandleEntry {
    /// One free handle-table entry linked into the intrusive free list.
    Free { next_free: u64 },
    /// One live raw allocation.
    Live(RawHandle),
}

/// One raw small-allocation space.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SmallSpace {
    /// The configured size-class table.
    pub(crate) size_classes: SizeClassTable,
    /// The configured span width.
    pub(crate) span_bytes: usize,
    /// The live raw spans.
    pub(crate) spans: Vec<RawSpan>,
    /// The reusable non-full spans per size class.
    pub(crate) available_spans: Vec<Vec<usize>>,
    /// The vacant span slots available for reuse.
    pub(crate) free_span_ids: Vec<usize>,
}

/// One raw large-allocation space.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LargeSpace {
    /// The configured page width for large allocations.
    pub(crate) page_bytes: usize,
    /// The live raw large allocations.
    pub(crate) large_allocations: Vec<RawLargeAllocation>,
    /// The free raw large-allocation ids available for reuse.
    pub(crate) free_large_allocation_ids: Vec<u64>,
    /// The next raw large-allocation id to allocate.
    pub(crate) next_unused_large_allocation_id: u64,
}

/// One live raw allocation space.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawSpace {
    /// The raw small-allocation space.
    pub(crate) small: SmallSpace,
    /// The raw large-allocation space.
    pub(crate) large: LargeSpace,
    /// The local page arena for large-allocation backing.
    page_arena: PageArena,
    /// Dense raw handle metadata keyed by allocation id minus one.
    handles: Vec<RawHandleEntry>,
    /// The free raw allocation id at the head of the intrusive free list.
    free_handle_head: u64,
    /// The next raw allocation id to allocate.
    next_unused_id: u64,
    /// The number of live raw allocations.
    allocated_count: usize,
    /// The number of live raw bytes.
    allocated_bytes: u64,
    /// The exact retained raw bytes.
    retained_bytes: Cell<u64>,
    /// Whether the retained-byte cache needs one exact recomputation.
    retained_bytes_dirty: Cell<bool>,
}

impl Default for RawSpace {
    fn default() -> Self {
        Self::new()
    }
}

impl RawSpace {
    /// Create one raw space with the default layout options.
    pub fn new() -> Self {
        Self::with_layout(&HeapLayoutOptions::default())
    }

    /// Create one raw space with explicit layout options.
    pub fn with_layout(options: &HeapLayoutOptions) -> Self {
        let space = Self {
            small: SmallSpace {
                size_classes: options.size_classes.clone(),
                span_bytes: options.raw_small_bytes,
                spans: Vec::new(),
                available_spans: vec![Vec::new(); options.size_classes.classes.len()],
                free_span_ids: Vec::new(),
            },
            large: LargeSpace {
                page_bytes: options.page_bytes,
                large_allocations: Vec::new(),
                free_large_allocation_ids: Vec::new(),
                next_unused_large_allocation_id: FIRST_ALLOCATED_LARGE_ALLOCATION_ID,
            },
            page_arena: PageArena::with_page_bytes(options.page_bytes),
            handles: Vec::new(),
            free_handle_head: 0,
            next_unused_id: FIRST_ALLOCATED_RAW_ID,
            allocated_count: 0,
            allocated_bytes: 0,
            retained_bytes: Cell::new(0),
            retained_bytes_dirty: Cell::new(false),
        };

        space.recompute_retained_bytes();

        space
    }

    /// Restore one raw space from one immutable image.
    pub fn from_image(image: &RawImage) -> Self {
        let mut space = Self {
            small: SmallSpace {
                size_classes: image.size_classes.clone(),
                span_bytes: image.small_bytes,
                spans: image
                    .spans
                    .iter()
                    .cloned()
                    .map(RawSpan::from_image)
                    .collect(),
                available_spans: vec![Vec::new(); image.size_classes.classes.len()],
                free_span_ids: Vec::new(),
            },
            large: LargeSpace {
                page_bytes: image.page_bytes,
                large_allocations: image
                    .large_allocations
                    .iter()
                    .map(RawLargeAllocation::from_large_allocation_image)
                    .collect(),
                free_large_allocation_ids: image
                    .free_large_allocation_ids
                    .iter()
                    .copied()
                    .collect(),
                next_unused_large_allocation_id: image.next_unused_large_allocation_id,
            },
            page_arena: PageArena::with_page_bytes(image.page_bytes),
            handles: image.handles.iter().copied().collect(),
            free_handle_head: image.free_handle_head,
            next_unused_id: image.next_unused_id,
            allocated_count: image.allocated_count,
            allocated_bytes: image.allocated_bytes,
            retained_bytes: Cell::new(0),
            retained_bytes_dirty: Cell::new(false),
        };

        space.rebuild_span_directories();
        space.recompute_retained_bytes();

        space
    }

    /// Return the number of live raw allocations.
    pub fn allocation_count(&self) -> usize {
        self.allocated_count
    }

    /// Return the exact retained raw bytes.
    pub fn active_bytes(&self) -> u64 {
        self.refresh_retained_bytes();
        self.retained_bytes.get()
    }

    /// Return the exact mapped raw page-arena bytes.
    pub fn mapped_bytes(&self) -> u64 {
        self.page_arena.mapped_bytes() as u64
    }

    /// Return the exact borrowed raw image bytes.
    pub fn borrowed_bytes(&self) -> u64 {
        let mut borrowed_bytes = 0usize;

        for span in &self.small.spans {
            borrowed_bytes += span.borrowed_bytes(self.page_arena.page_bytes());
        }

        for large_allocation in &self.large.large_allocations {
            borrowed_bytes += large_allocation.borrowed_bytes();
        }

        borrowed_bytes as u64
    }

    /// Return the exact live usage for this raw space.
    pub fn usage(&self) -> RawSpaceUsage {
        RawSpaceUsage {
            allocation_count: self.allocated_count,
            allocated_bytes: self.allocated_bytes,
            active_bytes: self.active_bytes(),
            mapped_bytes: self.mapped_bytes(),
            borrowed_bytes: self.borrowed_bytes(),
        }
    }

    /// Capture one immutable raw-space image.
    pub fn image(&mut self, base: Option<&RawImage>) -> RawImage {
        let _ = base;
        let spans = self
            .small
            .spans
            .iter_mut()
            .map(|span| span.image(&mut self.page_arena))
            .collect::<Vec<_>>();
        let large_allocations = self
            .large
            .large_allocations
            .iter_mut()
            .map(|large_allocation| large_allocation.large_allocation_image(&mut self.page_arena))
            .collect::<Vec<_>>();

        RawImage {
            size_classes: self.small.size_classes.clone(),
            small_bytes: self.small.span_bytes,
            page_bytes: self.large.page_bytes,
            spans,
            large_allocations,
            handles: Arc::from(self.handles.as_slice()),
            free_handle_head: self.free_handle_head,
            free_large_allocation_ids: Arc::from(self.large.free_large_allocation_ids.as_slice()),
            next_unused_id: self.next_unused_id,
            next_unused_large_allocation_id: self.large.next_unused_large_allocation_id,
            allocated_count: self.allocated_count,
            allocated_bytes: self.allocated_bytes,
        }
    }

    /// Allocate one raw byte allocation.
    pub(crate) fn allocate_bytes(&mut self, bytes: &[u8]) -> RawPointer {
        let max_small_bytes = self.small.size_classes.max_small_allocation_bytes();
        let retained_was_clean = !self.retained_bytes_dirty.get();
        let old_handle_capacity = self.handles.capacity();
        let (location, storage_retained_delta) = if bytes.len() <= max_small_bytes {
            let class_index = self
                .small
                .size_classes
                .class_index_for(bytes.len())
                .expect("small raw payloads must fit one size class");

            let (slot, retained_delta) = self.allocate_span_slot(class_index, bytes);
            (RawLocation::Small(slot), retained_delta)
        } else {
            let (large_allocation_id, retained_delta) = self.allocate_large_allocation(bytes);
            (RawLocation::Large(large_allocation_id), retained_delta)
        };

        let pointer = self.allocate_location(location, bytes.len());
        self.allocated_count += 1;
        self.allocated_bytes = self.allocated_bytes.saturating_add(bytes.len() as u64);

        if retained_was_clean {
            let retained_delta = storage_retained_delta
                + capacity_bytes_delta::<RawHandleEntry>(
                    old_handle_capacity,
                    self.handles.capacity(),
                );
            self.apply_retained_bytes_delta(retained_delta);
        } else {
            self.mark_retained_bytes_dirty();
        }

        pointer
    }

    /// Allocate one zeroed raw allocation.
    pub(crate) fn allocate_zeroed(&mut self, byte_len: usize) -> RawPointer {
        let max_small_bytes = self.small.size_classes.max_small_allocation_bytes();
        let retained_was_clean = !self.retained_bytes_dirty.get();
        let old_handle_capacity = self.handles.capacity();
        let (location, storage_retained_delta) = if byte_len <= max_small_bytes {
            let class_index = self
                .small
                .size_classes
                .class_index_for(byte_len)
                .expect("small raw payloads must fit one size class");

            let (slot, retained_delta) = self.allocate_zeroed_span_slot(class_index, byte_len);
            (RawLocation::Small(slot), retained_delta)
        } else {
            let (large_allocation_id, retained_delta) =
                self.allocate_zeroed_large_allocation(byte_len);
            (RawLocation::Large(large_allocation_id), retained_delta)
        };

        let pointer = self.allocate_location(location, byte_len);
        self.allocated_count += 1;
        self.allocated_bytes = self.allocated_bytes.saturating_add(byte_len as u64);

        if retained_was_clean {
            let retained_delta = storage_retained_delta
                + capacity_bytes_delta::<RawHandleEntry>(
                    old_handle_capacity,
                    self.handles.capacity(),
                );
            self.apply_retained_bytes_delta(retained_delta);
        } else {
            self.mark_retained_bytes_dirty();
        }

        pointer
    }

    /// Return the byte length for one raw allocation.
    pub fn byte_len(&self, pointer: RawPointer) -> Option<usize> {
        let base = RawPointer::new(pointer.id());
        let offset = pointer.byte_offset();
        let handle = self.handle(base)?;
        let len = handle.byte_len;

        len.checked_sub(offset)
    }

    /// Return the bytes for one raw allocation.
    pub fn bytes(&self, pointer: RawPointer) -> Option<Cow<'_, [u8]>> {
        let base = RawPointer::new(pointer.id());
        let offset = pointer.byte_offset();
        let location = self.location(base)?;

        let bytes = match location {
            RawLocation::Vacant => return None,
            RawLocation::Small(slot) => self
                .small
                .spans
                .get(slot.span_index())?
                .bytes(&self.page_arena, slot.slot_index())?,
            RawLocation::Large(large_allocation_id) => self
                .large_allocation(large_allocation_id)?
                .bytes(&self.page_arena),
        };

        match bytes {
            Cow::Borrowed(bytes) => Some(Cow::Borrowed(bytes.get(offset..)?)),
            Cow::Owned(bytes) => Some(Cow::Owned(bytes.get(offset..)?.to_vec())),
        }
    }

    /// Return one owned copy of the bytes for one raw allocation.
    pub fn bytes_to_vec(&self, pointer: RawPointer) -> Option<Vec<u8>> {
        Some(self.bytes(pointer)?.into_owned())
    }

    /// Return one byte by offset within one raw allocation.
    pub fn byte_at(&self, pointer: RawPointer, index: usize) -> Option<u8> {
        self.bytes(pointer)?.as_ref().get(index).copied()
    }

    /// Set one byte inside one raw allocation.
    pub(crate) fn set_byte(&mut self, pointer: RawPointer, index: usize, byte: u8) -> bool {
        let base = RawPointer::new(pointer.id());
        let offset = pointer.byte_offset().saturating_add(index);
        let Some(location) = self.location(base) else {
            return false;
        };

        match location {
            RawLocation::Vacant => false,
            RawLocation::Small(slot) => self
                .small
                .spans
                .get_mut(slot.span_index())
                .map(|span| span.set_byte(&mut self.page_arena, slot.slot_index(), offset, byte))
                .unwrap_or(false),
            RawLocation::Large(large_allocation_id) => self
                .large
                .large_allocations
                .get_mut((large_allocation_id.id().saturating_sub(1)) as usize)
                .map(|large_allocation| large_allocation.set(&mut self.page_arena, offset, byte))
                .unwrap_or(false),
        }
    }

    /// Return the peak active-byte reservation for writing one raw byte window.
    pub(crate) fn write_active_reservation(
        &self,
        pointer: RawPointer,
        start: usize,
        len: usize,
    ) -> i64 {
        let base = RawPointer::new(pointer.id());
        let offset = pointer.byte_offset().saturating_add(start);
        let Some(location) = self.location(base) else {
            return 0;
        };

        match location {
            RawLocation::Vacant => 0,
            RawLocation::Small(slot) => self
                .small
                .spans
                .get(slot.span_index())
                .map(|span| {
                    let (page_count, active_reservation) =
                        span.write_active_reservation(self.page_arena.page_bytes());
                    self.page_arena
                        .allocate_pages_active_reservation(page_count)
                        + active_reservation
                })
                .unwrap_or(0),
            RawLocation::Large(large_allocation_id) => self
                .large_allocation(large_allocation_id)
                .map(|large_allocation| {
                    let (page_count, active_reservation) =
                        large_allocation.write_active_reservation(offset, len);
                    self.page_arena
                        .allocate_pages_active_reservation(page_count)
                        + active_reservation
                })
                .unwrap_or(0),
        }
    }

    /// Set one byte slice inside one raw allocation.
    pub(crate) fn set_bytes(&mut self, pointer: RawPointer, start: usize, bytes: &[u8]) -> bool {
        let base = RawPointer::new(pointer.id());
        let offset = pointer.byte_offset().saturating_add(start);
        let Some(location) = self.location(base) else {
            return false;
        };

        match location {
            RawLocation::Vacant => false,
            RawLocation::Small(slot) => self
                .small
                .spans
                .get_mut(slot.span_index())
                .map(|span| span.set_bytes(&mut self.page_arena, slot.slot_index(), offset, bytes))
                .unwrap_or(false),
            RawLocation::Large(large_allocation_id) => self
                .large
                .large_allocations
                .get_mut((large_allocation_id.id().saturating_sub(1)) as usize)
                .map(|large_allocation| {
                    large_allocation.set_bytes(&mut self.page_arena, offset, bytes)
                })
                .unwrap_or(false),
        }
    }

    /// Free one raw allocation.
    pub(crate) fn free(&mut self, pointer: RawPointer) -> bool {
        let base = RawPointer::new(pointer.id());
        let Some(location) = self.location(base) else {
            return false;
        };
        let Some(old_len) = self.byte_len(base) else {
            return false;
        };

        match location {
            RawLocation::Vacant => return false,
            RawLocation::Small(slot) => {
                let span_index = slot.span_index();
                let Some(size_class) = self.small.spans.get(span_index).map(RawSpan::size_class)
                else {
                    return false;
                };
                let Some(class_index) = self.class_index_for_size_class(size_class) else {
                    return false;
                };
                let Some(span) = self.small.spans.get_mut(span_index) else {
                    return false;
                };

                if !span.free_slot(&mut self.page_arena, slot.slot_index()) {
                    return false;
                }

                if span.is_empty() {
                    span.release(&mut self.page_arena);
                    self.small.spans[span_index] = RawSpan::vacant();
                    self.small.free_span_ids.push(span_index);
                } else if span.has_free_slot() {
                    self.small.available_spans[class_index].push(span_index);
                }
            }
            RawLocation::Large(large_allocation_id) => {
                if large_allocation_id.id() == 0 {
                    return false;
                }

                let Some(large_allocation) = self
                    .large
                    .large_allocations
                    .get_mut((large_allocation_id.id() - 1) as usize)
                else {
                    return false;
                };

                large_allocation.free(&mut self.page_arena);
                self.large
                    .free_large_allocation_ids
                    .push(large_allocation_id.id());
            }
        }

        let next_free = self.free_handle_head;
        let Some(entry) = self.handle_entry_mut(pointer.id()) else {
            return false;
        };
        *entry = RawHandleEntry::Free { next_free };
        self.free_handle_head = pointer.id();
        self.allocated_count = self.allocated_count.saturating_sub(1);
        self.allocated_bytes = self.allocated_bytes.saturating_sub(old_len as u64);
        self.mark_retained_bytes_dirty();

        true
    }

    /// Replace one raw allocation payload.
    pub(crate) fn replace_bytes(&mut self, pointer: RawPointer, bytes: &[u8]) -> bool {
        let old_len = self
            .byte_len(RawPointer::new(pointer.id()))
            .unwrap_or_default() as u64;
        let Some(location) = self.location(RawPointer::new(pointer.id())) else {
            return false;
        };

        let replaced = match location {
            RawLocation::Vacant => false,
            RawLocation::Small(slot) => {
                let Some(class_index) = self.small.size_classes.class_index_for(bytes.len()) else {
                    let (large_allocation_id, _) = self.allocate_large_allocation(bytes);
                    let freed = self
                        .small
                        .spans
                        .get_mut(slot.span_index())
                        .map(|span| span.free_slot(&mut self.page_arena, slot.slot_index()))
                        .unwrap_or(false);
                    self.move_handle(
                        pointer.id(),
                        RawLocation::Large(large_allocation_id),
                        bytes.len(),
                    );
                    return freed;
                };

                let span_size_class = self
                    .small
                    .spans
                    .get(slot.span_index())
                    .map(RawSpan::size_class);
                if span_size_class
                    == self
                        .small
                        .size_classes
                        .classes
                        .get(class_index)
                        .map(|class| class.bytes)
                {
                    self.small
                        .spans
                        .get_mut(slot.span_index())
                        .map(|span| {
                            span.replace_slot(&mut self.page_arena, slot.slot_index(), bytes)
                        })
                        .unwrap_or(false)
                } else {
                    let (new_slot, _) = self.allocate_span_slot(class_index, bytes);
                    let freed = self
                        .small
                        .spans
                        .get_mut(slot.span_index())
                        .map(|span| span.free_slot(&mut self.page_arena, slot.slot_index()))
                        .unwrap_or(false);
                    self.move_handle(pointer.id(), RawLocation::Small(new_slot), bytes.len());
                    freed
                }
            }
            RawLocation::Large(large_allocation_id) => {
                if let Some(class_index) = self.small.size_classes.class_index_for(bytes.len()) {
                    let (new_slot, _) = self.allocate_span_slot(class_index, bytes);
                    if large_allocation_id.id() == 0 {
                        false
                    } else if let Some(large_allocation) = self
                        .large
                        .large_allocations
                        .get_mut((large_allocation_id.id() - 1) as usize)
                    {
                        large_allocation.free(&mut self.page_arena);
                        self.large
                            .free_large_allocation_ids
                            .push(large_allocation_id.id());
                        self.move_handle(pointer.id(), RawLocation::Small(new_slot), bytes.len());
                        true
                    } else {
                        false
                    }
                } else {
                    let page_bytes = self.large.page_bytes;
                    let replaced = self
                        .large
                        .large_allocations
                        .get_mut((large_allocation_id.id().saturating_sub(1)) as usize)
                        .map(|large_allocation| {
                            large_allocation.replace(&mut self.page_arena, bytes, page_bytes);
                            true
                        })
                        .unwrap_or(false);

                    if replaced {
                        self.move_handle(
                            pointer.id(),
                            RawLocation::Large(large_allocation_id),
                            bytes.len(),
                        );
                    }

                    replaced
                }
            }
        };

        if replaced {
            self.allocated_bytes = self
                .allocated_bytes
                .saturating_sub(old_len)
                .saturating_add(bytes.len() as u64);
            self.mark_retained_bytes_dirty();
        }

        replaced
    }

    /// Return the peak active-byte reservation for replacing one raw payload.
    pub(crate) fn replace_bytes_active_reservation(
        &self,
        pointer: RawPointer,
        new_len: usize,
    ) -> i64 {
        let base = RawPointer::new(pointer.id());
        let Some(location) = self.location(base) else {
            return 0;
        };

        match location {
            RawLocation::Vacant => 0,
            RawLocation::Small(slot) => {
                let Some(span) = self.small.spans.get(slot.span_index()) else {
                    return 0;
                };
                let Some(class_index) = self.small.size_classes.class_index_for(new_len) else {
                    return self.allocate_large_allocation_active_reservation(new_len);
                };
                let span_size_class = span.size_class();
                let Some(target_size_class) = self
                    .small
                    .size_classes
                    .classes
                    .get(class_index)
                    .map(|class| class.bytes)
                else {
                    return 0;
                };

                if span_size_class == target_size_class {
                    self.write_active_reservation(pointer, 0, new_len)
                } else {
                    self.allocate_span_slot_active_reservation(class_index)
                }
            }
            RawLocation::Large(large_allocation_id) => {
                if let Some(class_index) = self.small.size_classes.class_index_for(new_len) {
                    self.allocate_span_slot_active_reservation(class_index)
                } else {
                    self.large_allocation(large_allocation_id)
                        .map(|large_allocation| {
                            let (freed_pages, allocated_pages, active_reservation) =
                                large_allocation.replace_active_reservation(new_len);
                            self.page_arena
                                .replace_pages_active_reservation(freed_pages, allocated_pages)
                                + active_reservation
                        })
                        .unwrap_or(0)
                }
            }
        }
    }

    /// Allocate one raw packed-value payload.
    pub(crate) fn allocate_packed_values(&mut self, values: Vec<Value>) -> RawPointer {
        self.allocate_bytes(&encode_values(&values))
    }

    /// Allocate one raw packed-value payload with the given count.
    pub(crate) fn allocate_packed_value_slots(&mut self, slot_count: usize) -> RawPointer {
        self.allocate_zeroed(slot_count * Value::BYTE_LEN)
    }

    /// Return the raw allocation as decoded values.
    pub fn values_to_vec(&self, pointer: RawPointer) -> Option<Vec<Value>> {
        let bytes = self.bytes(pointer)?;
        decode_values(bytes.as_ref())
    }

    /// Set one packed raw value.
    pub(crate) fn set_value(&mut self, pointer: RawPointer, index: usize, value: Value) -> bool {
        let bytes = value.to_byte_array();
        let start = index * Value::BYTE_LEN;

        for (offset, byte) in bytes.into_iter().enumerate() {
            if !self.set_byte(pointer, start + offset, byte) {
                return false;
            }
        }

        true
    }

    /// Resize one raw packed-value payload.
    pub(crate) fn resize_values(&mut self, pointer: RawPointer, len: usize) -> bool {
        let Some(mut values) = self.values_to_vec(RawPointer::new(pointer.id())) else {
            return false;
        };
        values.resize(len, Value::VOID);
        self.replace_bytes(pointer, &encode_values(&values))
    }

    // directories

    fn allocate_location(&mut self, location: RawLocation, byte_len: usize) -> RawPointer {
        let handle = RawHandle { location, byte_len };

        if self.free_handle_head != 0 {
            let id = self.free_handle_head;
            let next_free = match self.handle_entry(id) {
                Some(RawHandleEntry::Free { next_free }) => *next_free,
                Some(RawHandleEntry::Live(_)) => {
                    unreachable!("free raw handle head must point at one free entry")
                }
                None => unreachable!("free raw handle head must remain addressable"),
            };

            self.free_handle_head = next_free;

            let Some(entry) = self.handle_entry_mut(id) else {
                unreachable!("free raw handle head must remain mutable");
            };
            *entry = RawHandleEntry::Live(handle);

            return RawPointer::new(id);
        }

        let id = self.next_unused_id;
        self.next_unused_id = self.next_unused_id.saturating_add(1);
        self.handles.push(RawHandleEntry::Live(handle));

        RawPointer::new(id)
    }

    fn location(&self, pointer: RawPointer) -> Option<RawLocation> {
        Some(self.handle(pointer)?.location)
    }

    fn handle(&self, pointer: RawPointer) -> Option<&RawHandle> {
        match self.handle_entry(pointer.id())? {
            RawHandleEntry::Live(handle) => Some(handle),
            RawHandleEntry::Free { .. } => None,
        }
    }

    fn handle_mut(&mut self, pointer: RawPointer) -> Option<&mut RawHandle> {
        match self.handle_entry_mut(pointer.id())? {
            RawHandleEntry::Live(handle) => Some(handle),
            RawHandleEntry::Free { .. } => None,
        }
    }

    fn handle_entry(&self, id: u64) -> Option<&RawHandleEntry> {
        if id == 0 {
            return None;
        }

        self.handles.get((id - 1) as usize)
    }

    fn handle_entry_mut(&mut self, id: u64) -> Option<&mut RawHandleEntry> {
        if id == 0 {
            return None;
        }

        self.handles.get_mut((id - 1) as usize)
    }

    fn move_handle(&mut self, id: u64, location: RawLocation, byte_len: usize) {
        let pointer = RawPointer::new(id);
        if let Some(handle) = self.handle_mut(pointer) {
            handle.location = location;
            handle.byte_len = byte_len;
        }
    }

    fn allocate_span_slot(&mut self, class_index: usize, bytes: &[u8]) -> (RawSpanSlot, i64) {
        let page_bytes = self.page_arena.page_bytes();

        while let Some(span_index) = self.small.available_spans[class_index].pop() {
            let Some(span) = self.small.spans.get_mut(span_index) else {
                continue;
            };

            if !span.has_free_slot() {
                continue;
            }

            let Some(slot_index) = span.first_free_slot() else {
                continue;
            };

            let retained_before = span.active_bytes(page_bytes) as i64;
            let page_arena_retained = self.page_arena.retained_bytes() as i64;
            if span.allocate_slot(&mut self.page_arena, slot_index, bytes) {
                let retained_after = span.active_bytes(page_bytes) as i64;
                let mut retained_delta = retained_after - retained_before
                    + (self.page_arena.retained_bytes() as i64 - page_arena_retained);

                if span.has_free_slot() {
                    let queue_capacity = self.small.available_spans[class_index].capacity();
                    self.small.available_spans[class_index].push(span_index);
                    retained_delta += capacity_bytes_delta::<usize>(
                        queue_capacity,
                        self.small.available_spans[class_index].capacity(),
                    );
                }

                return (RawSpanSlot::new(span_index, slot_index), retained_delta);
            }
        }

        let size_class = self.small.size_classes.classes[class_index].bytes;
        let spans_capacity = self.small.spans.capacity();
        let queue_capacity = self.small.available_spans[class_index].capacity();
        let page_arena_retained = self.page_arena.retained_bytes() as i64;
        let mut span = RawSpan::new(size_class, self.small.span_bytes, &mut self.page_arena);
        let slot_index = span
            .first_free_slot()
            .unwrap_or_else(|| panic!("fresh raw span must have one free slot"));
        let allocated = span.allocate_slot(&mut self.page_arena, slot_index, bytes);
        debug_assert!(allocated, "fresh raw span must accept its first slot");
        let span_retained = span.active_bytes(page_bytes) as i64;

        let span_index = if let Some(index) = self.small.free_span_ids.pop() {
            self.small.spans[index] = span;
            index
        } else {
            self.small.spans.push(span);
            self.small.spans.len() - 1
        };

        if self.small.spans[span_index].has_free_slot() {
            self.small.available_spans[class_index].push(span_index);
        }

        let retained_delta = span_retained
            + capacity_bytes_delta::<RawSpan>(spans_capacity, self.small.spans.capacity())
            + capacity_bytes_delta::<usize>(
                queue_capacity,
                self.small.available_spans[class_index].capacity(),
            )
            + (self.page_arena.retained_bytes() as i64 - page_arena_retained);

        (RawSpanSlot::new(span_index, slot_index), retained_delta)
    }

    fn allocate_zeroed_span_slot(
        &mut self,
        class_index: usize,
        byte_len: usize,
    ) -> (RawSpanSlot, i64) {
        let page_bytes = self.page_arena.page_bytes();

        while let Some(span_index) = self.small.available_spans[class_index].pop() {
            let Some(span) = self.small.spans.get_mut(span_index) else {
                continue;
            };

            if !span.has_free_slot() {
                continue;
            }

            let Some(slot_index) = span.first_free_slot() else {
                continue;
            };

            let retained_before = span.active_bytes(page_bytes) as i64;
            let page_arena_retained = self.page_arena.retained_bytes() as i64;
            if span.allocate_zeroed_slot(&mut self.page_arena, slot_index, byte_len) {
                let retained_after = span.active_bytes(page_bytes) as i64;
                let mut retained_delta = retained_after - retained_before
                    + (self.page_arena.retained_bytes() as i64 - page_arena_retained);

                if span.has_free_slot() {
                    let queue_capacity = self.small.available_spans[class_index].capacity();
                    self.small.available_spans[class_index].push(span_index);
                    retained_delta += capacity_bytes_delta::<usize>(
                        queue_capacity,
                        self.small.available_spans[class_index].capacity(),
                    );
                }

                return (RawSpanSlot::new(span_index, slot_index), retained_delta);
            }
        }

        let size_class = self.small.size_classes.classes[class_index].bytes;
        let spans_capacity = self.small.spans.capacity();
        let queue_capacity = self.small.available_spans[class_index].capacity();
        let page_arena_retained = self.page_arena.retained_bytes() as i64;
        let mut span = RawSpan::new(size_class, self.small.span_bytes, &mut self.page_arena);
        let slot_index = span
            .first_free_slot()
            .unwrap_or_else(|| panic!("fresh raw span must have one free slot"));
        let allocated = span.allocate_zeroed_slot(&mut self.page_arena, slot_index, byte_len);
        debug_assert!(allocated, "fresh raw span must accept its first slot");
        let span_retained = span.active_bytes(page_bytes) as i64;

        let span_index = if let Some(index) = self.small.free_span_ids.pop() {
            self.small.spans[index] = span;
            index
        } else {
            self.small.spans.push(span);
            self.small.spans.len() - 1
        };

        if self.small.spans[span_index].has_free_slot() {
            self.small.available_spans[class_index].push(span_index);
        }

        let retained_delta = span_retained
            + capacity_bytes_delta::<RawSpan>(spans_capacity, self.small.spans.capacity())
            + capacity_bytes_delta::<usize>(
                queue_capacity,
                self.small.available_spans[class_index].capacity(),
            )
            + (self.page_arena.retained_bytes() as i64 - page_arena_retained);

        (RawSpanSlot::new(span_index, slot_index), retained_delta)
    }

    fn allocate_large_allocation(&mut self, bytes: &[u8]) -> (RawLargeAllocationId, i64) {
        if let Some(id) = self.large.free_large_allocation_ids.pop() {
            let large_allocation_id = RawLargeAllocationId::new(id);
            let page_bytes = self.large.page_bytes;
            let page_arena_retained = self.page_arena.retained_bytes() as i64;
            let retained_before = self
                .large
                .large_allocations
                .get((large_allocation_id.id() - 1) as usize)
                .map(RawLargeAllocation::active_bytes)
                .unwrap_or(0) as i64;
            if let Some(large_allocation) = self
                .large
                .large_allocations
                .get_mut((large_allocation_id.id() - 1) as usize)
            {
                large_allocation.replace(&mut self.page_arena, bytes, page_bytes);
            }

            let retained_after = self
                .large
                .large_allocations
                .get((large_allocation_id.id() - 1) as usize)
                .map(RawLargeAllocation::active_bytes)
                .unwrap_or(0) as i64;
            let retained_delta = retained_after - retained_before
                + (self.page_arena.retained_bytes() as i64 - page_arena_retained);

            return (large_allocation_id, retained_delta);
        }

        let large_allocation_id =
            RawLargeAllocationId::new(self.large.next_unused_large_allocation_id);
        self.large.next_unused_large_allocation_id =
            self.large.next_unused_large_allocation_id.saturating_add(1);
        let large_capacity = self.large.large_allocations.capacity();
        let page_arena_retained = self.page_arena.retained_bytes() as i64;
        self.large.large_allocations.push(RawLargeAllocation::new(
            bytes,
            self.large.page_bytes,
            &mut self.page_arena,
        ));
        let retained_delta = self
            .large
            .large_allocations
            .last()
            .map(RawLargeAllocation::active_bytes)
            .unwrap_or(0) as i64
            + capacity_bytes_delta::<RawLargeAllocation>(
                large_capacity,
                self.large.large_allocations.capacity(),
            )
            + (self.page_arena.retained_bytes() as i64 - page_arena_retained);

        (large_allocation_id, retained_delta)
    }

    fn allocate_zeroed_large_allocation(&mut self, byte_len: usize) -> (RawLargeAllocationId, i64) {
        if let Some(id) = self.large.free_large_allocation_ids.pop() {
            let large_allocation_id = RawLargeAllocationId::new(id);
            let page_bytes = self.large.page_bytes;
            let page_arena_retained = self.page_arena.retained_bytes() as i64;
            let retained_before = self
                .large
                .large_allocations
                .get((large_allocation_id.id() - 1) as usize)
                .map(RawLargeAllocation::active_bytes)
                .unwrap_or(0) as i64;
            if let Some(large_allocation) = self
                .large
                .large_allocations
                .get_mut((large_allocation_id.id() - 1) as usize)
            {
                large_allocation.replace_zeroed(&mut self.page_arena, byte_len, page_bytes);
            }

            let retained_after = self
                .large
                .large_allocations
                .get((large_allocation_id.id() - 1) as usize)
                .map(RawLargeAllocation::active_bytes)
                .unwrap_or(0) as i64;
            let retained_delta = retained_after - retained_before
                + (self.page_arena.retained_bytes() as i64 - page_arena_retained);

            return (large_allocation_id, retained_delta);
        }

        let large_allocation_id =
            RawLargeAllocationId::new(self.large.next_unused_large_allocation_id);
        self.large.next_unused_large_allocation_id =
            self.large.next_unused_large_allocation_id.saturating_add(1);
        let large_capacity = self.large.large_allocations.capacity();
        let page_arena_retained = self.page_arena.retained_bytes() as i64;
        self.large
            .large_allocations
            .push(RawLargeAllocation::new_zeroed(
                byte_len,
                self.large.page_bytes,
                &mut self.page_arena,
            ));
        let retained_delta = self
            .large
            .large_allocations
            .last()
            .map(RawLargeAllocation::active_bytes)
            .unwrap_or(0) as i64
            + capacity_bytes_delta::<RawLargeAllocation>(
                large_capacity,
                self.large.large_allocations.capacity(),
            )
            + (self.page_arena.retained_bytes() as i64 - page_arena_retained);

        (large_allocation_id, retained_delta)
    }

    fn large_allocation(
        &self,
        large_allocation_id: RawLargeAllocationId,
    ) -> Option<&RawLargeAllocation> {
        if large_allocation_id.id() == 0 {
            return None;
        }

        self.large
            .large_allocations
            .get((large_allocation_id.id() - 1) as usize)
    }

    fn class_index_for_size_class(&self, size_class: usize) -> Option<usize> {
        self.small
            .size_classes
            .classes
            .iter()
            .position(|class| class.bytes == size_class)
    }

    fn rebuild_span_directories(&mut self) {
        self.small.available_spans = vec![Vec::new(); self.small.size_classes.classes.len()];
        self.small.free_span_ids.clear();

        for (index, span) in self.small.spans.iter().enumerate() {
            if span.is_vacant() {
                self.small.free_span_ids.push(index);
                continue;
            }

            if !span.has_free_slot() {
                continue;
            }

            if let Some(class_index) = self.class_index_for_size_class(span.size_class()) {
                self.small.available_spans[class_index].push(index);
            }
        }
    }

    fn mark_retained_bytes_dirty(&self) {
        self.retained_bytes_dirty.set(true);
    }

    fn apply_retained_bytes_delta(&self, delta: i64) {
        let retained_bytes = self.retained_bytes.get();
        let retained_bytes = if delta >= 0 {
            retained_bytes.saturating_add(delta as u64)
        } else {
            retained_bytes.saturating_sub(delta.unsigned_abs())
        };

        self.retained_bytes.set(retained_bytes);
        self.retained_bytes_dirty.set(false);

        self.debug_assert_retained_bytes();
    }

    fn refresh_retained_bytes(&self) {
        if !self.retained_bytes_dirty.get() {
            return;
        }

        self.retained_bytes.set(self.exact_retained_bytes());
        self.retained_bytes_dirty.set(false);
    }

    fn recompute_retained_bytes(&self) {
        self.retained_bytes_dirty.set(true);
        self.refresh_retained_bytes();
    }

    /// Return the exact retained bytes implied by the current live state.
    fn exact_retained_bytes(&self) -> u64 {
        let mut retained_bytes = 0usize;

        retained_bytes += self.small.spans.capacity() * size_of::<RawSpan>();
        retained_bytes += self.small.size_classes.retained_bytes();
        retained_bytes += self.small.available_spans.capacity() * size_of::<Vec<usize>>();
        retained_bytes += self.small.free_span_ids.capacity() * size_of::<usize>();
        retained_bytes += self.large.large_allocations.capacity() * size_of::<RawLargeAllocation>();
        retained_bytes += self.large.free_large_allocation_ids.capacity() * size_of::<u64>();
        retained_bytes += self.page_arena.retained_bytes();
        retained_bytes += self.handles.capacity() * size_of::<RawHandleEntry>();

        for queue in &self.small.available_spans {
            retained_bytes += queue.capacity() * size_of::<usize>();
        }

        for span in &self.small.spans {
            retained_bytes += span.active_bytes(self.page_arena.page_bytes());
        }

        for large_allocation in &self.large.large_allocations {
            retained_bytes += large_allocation.active_bytes();
        }

        retained_bytes as u64
    }

    /// Assert that the incremental retained-byte cache matches exact state.
    fn debug_assert_retained_bytes(&self) {
        #[cfg(debug_assertions)]
        {
            debug_assert!(
                !self.retained_bytes_dirty.get(),
                "retained-byte audit requires one clean raw-space cache"
            );

            let retained_bytes = self.exact_retained_bytes();
            debug_assert_eq!(
                self.retained_bytes.get(),
                retained_bytes,
                "raw-space retained bytes must match exact recomputation"
            );
        }
    }

    /// Return the active-byte reservation for allocating one raw byte allocation.
    pub(crate) fn allocate_bytes_active_reservation(&self, byte_len: usize) -> i64 {
        self.allocate_storage_active_reservation(byte_len)
            + self.allocate_handle_active_reservation()
    }

    /// Return the active-byte reservation for allocating one zeroed raw allocation.
    pub(crate) fn allocate_zeroed_active_reservation(&self, byte_len: usize) -> i64 {
        self.allocate_bytes_active_reservation(byte_len)
    }

    /// Return the active-byte reservation for one new raw storage allocation.
    fn allocate_storage_active_reservation(&self, byte_len: usize) -> i64 {
        let max_small_bytes = self.small.size_classes.max_small_allocation_bytes();

        if byte_len <= max_small_bytes {
            let class_index = self
                .small
                .size_classes
                .class_index_for(byte_len)
                .expect("small raw payloads must fit one size class");

            self.allocate_span_slot_active_reservation(class_index)
        } else {
            self.allocate_large_allocation_active_reservation(byte_len)
        }
    }

    /// Return the active-byte reservation for one raw handle publication.
    fn allocate_handle_active_reservation(&self) -> i64 {
        if self.free_handle_head != 0 {
            return 0;
        }

        let capacity = projected_vec_capacity::<RawHandleEntry>(
            self.handles.len(),
            self.handles.capacity(),
            1,
        );

        vec_capacity_bytes_delta::<RawHandleEntry>(self.handles.capacity(), capacity)
    }

    /// Return the active-byte reservation for allocating one raw span slot.
    fn allocate_span_slot_active_reservation(&self, class_index: usize) -> i64 {
        let page_bytes = self.page_arena.page_bytes();

        for &span_index in self.small.available_spans[class_index].iter().rev() {
            let Some(span) = self.small.spans.get(span_index) else {
                continue;
            };

            if !span.has_free_slot() {
                continue;
            }

            let span_delta = span.allocate_active_reservation(page_bytes);
            let page_count = self.small.span_bytes.div_ceil(page_bytes).max(1);
            let page_arena_delta = if span_delta != 0 {
                self.page_arena
                    .allocate_pages_active_reservation(page_count)
            } else {
                0
            };

            return span_delta + page_arena_delta;
        }

        let size_class = self.small.size_classes.classes[class_index].bytes;
        let span_delta =
            RawSpan::active_bytes_for_new(size_class, self.small.span_bytes, page_bytes) as i64;
        let spans_delta = if self.small.free_span_ids.is_empty() {
            let capacity = projected_vec_capacity::<RawSpan>(
                self.small.spans.len(),
                self.small.spans.capacity(),
                1,
            );
            vec_capacity_bytes_delta::<RawSpan>(self.small.spans.capacity(), capacity)
        } else {
            0
        };
        let slot_count = (self.small.span_bytes / size_class).max(1);
        let queue_delta = if slot_count > 1 {
            let queue = &self.small.available_spans[class_index];
            let capacity = projected_vec_capacity::<usize>(queue.len(), queue.capacity(), 1);
            vec_capacity_bytes_delta::<usize>(queue.capacity(), capacity)
        } else {
            0
        };
        let page_count = self.small.span_bytes.div_ceil(page_bytes).max(1);
        let page_arena_delta = self
            .page_arena
            .allocate_pages_active_reservation(page_count);

        span_delta + spans_delta + queue_delta + page_arena_delta
    }

    /// Return the active-byte reservation for allocating one raw large allocation.
    fn allocate_large_allocation_active_reservation(&self, byte_len: usize) -> i64 {
        let page_bytes = self.large.page_bytes;
        let page_count = byte_len.div_ceil(page_bytes).max((byte_len > 0) as usize);
        let page_arena_delta = self
            .page_arena
            .allocate_pages_active_reservation(page_count);
        let payload_retained = ChunkPayload::active_bytes_for_len(byte_len, page_bytes);

        if let Some(id) = self.large.free_large_allocation_ids.last().copied() {
            let retained_before = self
                .large
                .large_allocations
                .get((id - 1) as usize)
                .map(RawLargeAllocation::active_bytes)
                .unwrap_or(0);

            return payload_retained as i64 - retained_before as i64 + page_arena_delta;
        }

        let large_capacity = projected_vec_capacity::<RawLargeAllocation>(
            self.large.large_allocations.len(),
            self.large.large_allocations.capacity(),
            1,
        );
        let large_delta = vec_capacity_bytes_delta::<RawLargeAllocation>(
            self.large.large_allocations.capacity(),
            large_capacity,
        );

        payload_retained as i64 + large_delta + page_arena_delta
    }
}

/// Return the retained-byte delta implied by one vector-capacity change.
fn capacity_bytes_delta<T>(old_capacity: usize, new_capacity: usize) -> i64 {
    vec_capacity_bytes_delta::<T>(old_capacity, new_capacity)
}

// encode one packed value vector into bytes
fn encode_values(values: &[Value]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(values.len() * Value::BYTE_LEN);

    for value in values {
        bytes.extend_from_slice(&value.to_byte_array());
    }

    bytes
}

// decode one byte slice into packed values
fn decode_values(bytes: &[u8]) -> Option<Vec<Value>> {
    if !bytes.len().is_multiple_of(Value::BYTE_LEN) {
        return None;
    }

    let mut values = Vec::with_capacity(bytes.len() / Value::BYTE_LEN);

    for chunk in bytes.chunks(Value::BYTE_LEN) {
        values.push(Value::from_byte_slice(chunk)?);
    }

    Some(values)
}
