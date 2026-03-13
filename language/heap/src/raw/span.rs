use super::heap::RawHeap;
use super::{
    RawAllocation, RawAllocationStorage, RawLargeSpanId, RawSpan, RawSpanId, RawSpanReference,
};
use crate::heap::HeapLimitError;
use crate::page::{PageSlot, RAW_PAGE_CAPACITY};
use crate::value::RawPointer;

impl RawHeap {
    /// Replace the entire raw byte payload with exact retained-byte admission.
    pub(crate) fn replace_bytes_checked(
        &mut self,
        pointer: RawPointer,
        bytes: &[u8],
        mut admit: impl FnMut(i64) -> Result<(), HeapLimitError>,
    ) -> Result<bool, HeapLimitError> {
        let Some(delta) = self.replace_bytes_delta(pointer, bytes.len()) else {
            return Ok(false);
        };

        // admit retained-byte growth before mutating the raw span
        admit(delta)?;

        let replaced = self.replace_bytes(pointer, bytes);

        if replaced {
            self.apply_retained_delta(delta);
        }

        Ok(replaced)
    }

    /// Return one raw byte slice when this pointer refers to byte storage.
    pub fn bytes(&self, pointer: RawPointer) -> Option<&[u8]> {
        let RawAllocationStorage::Bytes(span) = &self.get(pointer)?.storage else {
            return None;
        };

        Some(self.span(*span)?.as_slice())
    }

    /// Return one owned copy of the raw bytes for this pointer.
    pub fn bytes_to_vec(&self, pointer: RawPointer) -> Option<Vec<u8>> {
        Some(self.bytes(pointer)?.to_vec())
    }

    /// Return one byte by slot offset when this pointer refers to byte storage.
    pub fn byte_at(&self, pointer: RawPointer, index: usize) -> Option<u8> {
        let RawAllocationStorage::Bytes(span) = &self.get(pointer)?.storage else {
            return None;
        };

        self.span(*span)?.get(index)
    }

    /// Return the raw byte length for this pointer.
    pub fn byte_len(&self, pointer: RawPointer) -> Option<usize> {
        let RawAllocationStorage::Bytes(span) = &self.get(pointer)?.storage else {
            return None;
        };

        Some(self.span(*span)?.len())
    }

    /// Write one raw byte by slot offset.
    pub fn set_byte(&mut self, pointer: RawPointer, index: usize, byte: u8) -> bool {
        let Some(span) = self.span_id(pointer) else {
            return false;
        };

        let Some(span) = self.span_mut(span) else {
            return false;
        };

        span.set(index, byte)
    }

    /// Replace the entire raw byte payload.
    pub fn replace_bytes(&mut self, pointer: RawPointer, bytes: &[u8]) -> bool {
        let Some(span) = self.span_id(pointer) else {
            return false;
        };

        let Some(span) = self.span_mut(span) else {
            return false;
        };

        span.replace(bytes);
        true
    }

    /// Return the exact retained-byte delta for replacing one raw byte payload.
    fn replace_bytes_delta(&self, pointer: RawPointer, new_len: usize) -> Option<i64> {
        let span = self.span_id(pointer)?;
        let old_len = self.span(span)?.len();
        let old_class = span.class();
        let new_class = self.span_class(new_len);

        // in class replacements only change payload length
        if old_class == new_class {
            return Some(new_len as i64 - old_len as i64);
        }

        // class changes free the old span and allocate one new classed span
        Some(self.free_span_delta(span) + self.span_allocate_delta_for_bytes(new_len))
    }

    /// Return the approximate byte size for one allocation.
    pub fn allocation_size_for(&self, allocation: &RawAllocation) -> u64 {
        match &allocation.storage {
            RawAllocationStorage::Values(values) => values.retained_bytes() as u64,
            RawAllocationStorage::Bytes(reference) => self
                .span(*reference)
                .map(|span| span.len() as u64)
                .unwrap_or_default(),
        }
    }

    /// Allocate one fresh stable page slot for one new raw allocation id.
    pub(super) fn allocate_location(&mut self) -> PageSlot {
        let slot = self.next_unused_slot as usize;
        self.next_unused_slot = self.next_unused_slot.saturating_add(1);

        let page_slot = PageSlot::new(slot / RAW_PAGE_CAPACITY, slot % RAW_PAGE_CAPACITY);
        self.ensure_location(page_slot);
        page_slot
    }

    /// Ensure the selected raw allocation location has page storage allocated.
    pub(super) fn ensure_location(&mut self, page_slot: PageSlot) {
        let required_page = page_slot.page_index();

        // extend raw pages until the requested slot becomes addressable
        while self.pages.len() <= required_page {
            self.pages.push(crate::page::RawPage::new());
        }
    }

    /// Return the stable page slot for one raw allocation id.
    pub(super) fn location_for_id(&self, allocation_id: u64) -> Option<PageSlot> {
        // allocation id zero is the null raw pointer
        if allocation_id == 0 {
            return None;
        }

        self.locations.get((allocation_id - 1) as usize).copied()
    }

    /// Return the stable page slot for one raw pointer.
    pub(super) fn location(&self, pointer: RawPointer) -> Option<PageSlot> {
        self.location_for_id(pointer.id())
    }

    /// Allocate or reuse one raw span slot.
    pub(super) fn allocate_span(&mut self, bytes: &[u8]) -> RawSpanId {
        // reuse one free ordinary span slot when possible
        if let Some(id) = self.free_span_ids.pop() {
            let span = self
                .span_mut(RawSpanReference::Regular(RawSpanId::new(id)))
                .expect("reused raw span id must stay addressable");
            span.replace(bytes);
            return RawSpanId::new(id);
        }

        // otherwise append one new ordinary span
        let id = self.next_unused_span_id;
        self.next_unused_span_id = self.next_unused_span_id.saturating_add(1);
        self.spans.push(RawSpan::new(bytes));
        RawSpanId::new(id)
    }

    /// Allocate or reuse one dedicated large raw span slot.
    pub(super) fn allocate_large_span(&mut self, bytes: &[u8]) -> RawLargeSpanId {
        // reuse one free dedicated large span slot when possible
        if let Some(id) = self.free_large_span_ids.pop() {
            let span = self
                .span_mut(RawSpanReference::Large(RawLargeSpanId::new(id)))
                .expect("reused large raw span id must stay addressable");
            span.replace(bytes);
            return RawLargeSpanId::new(id);
        }

        // otherwise append one new dedicated large span
        let id = self.next_unused_large_span_id;
        self.next_unused_large_span_id = self.next_unused_large_span_id.saturating_add(1);
        self.large_spans.push(RawSpan::new(bytes));
        RawLargeSpanId::new(id)
    }

    /// Recycle one raw span slot.
    pub(super) fn free_span(&mut self, span: RawSpanReference) {
        let Some(storage) = self.span_mut(span) else {
            return;
        };

        // clear the payload before the span id returns to the free list
        storage.replace(&[]);

        // recycle the span id in the right class free list
        match span {
            RawSpanReference::Regular(id) => self.free_span_ids.push(id.id()),
            RawSpanReference::Large(id) => self.free_large_span_ids.push(id.id()),
        }
    }

    /// Return one raw span identifier for one raw pointer.
    pub(super) fn span_id(&self, pointer: RawPointer) -> Option<RawSpanReference> {
        let RawAllocationStorage::Bytes(span) = &self.get(pointer)?.storage else {
            return None;
        };

        Some(*span)
    }

    /// Return one shared raw span.
    pub(super) fn span(&self, span: RawSpanReference) -> Option<&RawSpan> {
        // ordinary and large spans live in separate stable vectors
        match span {
            RawSpanReference::Regular(id) => {
                if id.id() == 0 {
                    return None;
                }

                self.spans.get((id.id() - 1) as usize)
            }
            RawSpanReference::Large(id) => {
                if id.id() == 0 {
                    return None;
                }

                self.large_spans.get((id.id() - 1) as usize)
            }
        }
    }

    /// Return one mutable raw span.
    pub(super) fn span_mut(&mut self, span: RawSpanReference) -> Option<&mut RawSpan> {
        // ordinary and large spans live in separate stable vectors
        match span {
            RawSpanReference::Regular(id) => {
                if id.id() == 0 {
                    return None;
                }

                self.spans.get_mut((id.id() - 1) as usize)
            }
            RawSpanReference::Large(id) => {
                if id.id() == 0 {
                    return None;
                }

                self.large_spans.get_mut((id.id() - 1) as usize)
            }
        }
    }

    /// Return the exact retained-byte delta for freeing one raw span.
    pub(super) fn free_span_delta(&self, span: RawSpanReference) -> i64 {
        let Some(storage) = self.span(span) else {
            return 0;
        };

        let payload_bytes = storage.len() as i64;
        let id_bytes = std::mem::size_of::<u64>() as i64;

        match span {
            RawSpanReference::Regular(_) | RawSpanReference::Large(_) => id_bytes - payload_bytes,
        }
    }
}
