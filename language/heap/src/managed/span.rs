use super::heap::ManagedHeap;
use super::{
    INLINE_ALLOCATION_VALUES, ManagedLargeSpan, ManagedLargeSpanId, ManagedSpan, ManagedSpanClass,
};
use crate::MANAGED_PAGE_CAPACITY;
use crate::page::{ManagedPage, PageSlot, VALUE_PAGE_CAPACITY, ValuePage};
use crate::value::{ManagedReference, Value};
use std::mem::size_of;

impl ManagedHeap {
    /// Free one managed allocation by handle.
    pub fn free(&mut self, handle: ManagedReference) -> bool {
        let reference_id = handle.id();
        if reference_id == 0 || reference_id >= self.next_unused_id {
            return false;
        }

        let Some(address) = self.location(handle) else {
            return false;
        };
        let (target_page, target_offset) = address.position();

        let Some(page) = self.pages.get_mut(target_page) else {
            return false;
        };
        let Some(allocation) = page.take(target_offset) else {
            return false;
        };

        // recycle span storage and the free id entry
        let mut retained_delta = size_of::<u64>() as i64;
        if let Some(span) = allocation.span() {
            retained_delta += self.free_span_delta(span);
            self.free_span(span);
        }

        self.free_ids.push(reference_id);
        debug_assert!(self.allocated_count > 0, "heap allocation count underflow");
        self.allocated_count -= 1;
        self.allocated_bytes = self
            .allocated_bytes
            .saturating_sub(Self::allocation_size_for(&allocation));
        self.apply_retained_delta(retained_delta);

        true
    }

    /// Clear all managed allocations.
    pub fn clear(&mut self) {
        self.pages.clear();
        self.pages.push(ManagedPage::new());
        self.value_pages.clear();
        self.value_pages.push(ValuePage::new());
        self.large_spans.clear();
        self.locations.clear();
        self.free_ids.clear();
        self.free_spans.clear();
        self.free_large_span_ids.clear();
        self.next_unused_id = 1;
        self.next_unused_slot = 0;
        self.next_unused_span_index = 0;
        self.next_unused_large_span_id = 1;
        self.allocated_count = 0;
        self.allocated_bytes = 0;
        self.gc_state = crate::GcState::default();
        self.mark_queue.clear();
        self.recompute_retained_bytes();
    }

    /// Allocate one managed span and initialize it from values.
    pub(super) fn allocate_span(&mut self, values: &[Value]) -> ManagedSpan {
        // empty spans
        if values.is_empty() {
            return ManagedSpan::new(PageSlot::ZERO, 0);
        }

        // dedicated large spans
        if self.span_class(values.len()) == ManagedSpanClass::Large {
            return self.allocate_large_span(values);
        }

        // page-backed spans
        self.allocate_paged_span(values)
    }

    /// Allocate one page-backed managed span and initialize it from values.
    fn allocate_paged_span(&mut self, values: &[Value]) -> ManagedSpan {
        // empty spans
        if values.is_empty() {
            return ManagedSpan::new(PageSlot::ZERO, 0);
        }

        // reuse one free span when possible
        if let Some(index) = self
            .free_spans
            .iter()
            .position(|span| span.len() >= values.len())
        {
            let free_span = self.free_spans.remove(index);
            let allocated = ManagedSpan::new(
                free_span
                    .start()
                    .expect("page-backed managed span must keep one start location"),
                values.len(),
            );
            let remaining = free_span.len().saturating_sub(values.len());

            if remaining > 0 {
                let remaining_start = Self::span_advance(
                    free_span
                        .start()
                        .expect("page-backed managed span must keep one start location"),
                    values.len(),
                );
                self.insert_free_span(ManagedSpan::new(remaining_start, remaining));
            }

            self.write_span(allocated, values);
            return allocated;
        }

        // otherwise extend the managed value page space
        let start_index = self.next_unused_span_index;
        self.next_unused_span_index = self
            .next_unused_span_index
            .saturating_add(values.len() as u64);
        self.ensure_span_storage(start_index, values.len());

        let span = ManagedSpan::new(Self::span_location_from_index(start_index), values.len());
        self.write_span(span, values);
        span
    }

    /// Allocate one fresh stable page slot for one new managed reference id.
    pub(super) fn allocate_location(&mut self) -> PageSlot {
        let slot = self.next_unused_slot as usize;
        self.next_unused_slot = self.next_unused_slot.saturating_add(1);

        let page_slot = PageSlot::new(slot / MANAGED_PAGE_CAPACITY, slot % MANAGED_PAGE_CAPACITY);
        self.ensure_location(page_slot);
        page_slot
    }

    /// Ensure the selected managed location has page storage allocated.
    fn ensure_location(&mut self, page_slot: PageSlot) {
        let required_page = page_slot.page_index();

        // grow the page set
        while self.pages.len() <= required_page {
            self.pages.push(ManagedPage::new());
        }
    }

    /// Ensure the selected managed span range has page storage allocated.
    fn ensure_span_storage(&mut self, start_index: u64, len: usize) {
        // empty spans
        if len == 0 {
            return;
        }

        let last_index = start_index.saturating_add(len as u64).saturating_sub(1);
        let required_page = Self::span_page_index(last_index);

        // grow the value page set
        while self.value_pages.len() <= required_page {
            self.value_pages.push(ValuePage::new());
        }
    }

    /// Allocate one managed span with the requested length.
    pub(super) fn allocate_span_len(&mut self, len: usize) -> ManagedSpan {
        // empty spans
        if len == 0 {
            return ManagedSpan::new(PageSlot::ZERO, 0);
        }

        // dedicated large spans
        if self.span_class(len) == ManagedSpanClass::Large {
            return self.allocate_large_span_len(len);
        }

        // page-backed spans
        self.allocate_paged_span_len(len)
    }

    /// Allocate one page-backed managed span with the requested length.
    fn allocate_paged_span_len(&mut self, len: usize) -> ManagedSpan {
        // empty spans
        if len == 0 {
            return ManagedSpan::new(PageSlot::ZERO, 0);
        }

        // reuse one free span when possible
        if let Some(index) = self.free_spans.iter().position(|span| span.len() >= len) {
            let free_span = self.free_spans.remove(index);
            let allocated = ManagedSpan::new(
                free_span
                    .start()
                    .expect("page-backed managed span must keep one start location"),
                len,
            );
            let remaining = free_span.len().saturating_sub(len);

            if remaining > 0 {
                let remaining_start = Self::span_advance(
                    free_span
                        .start()
                        .expect("page-backed managed span must keep one start location"),
                    len,
                );
                self.insert_free_span(ManagedSpan::new(remaining_start, remaining));
            }

            return allocated;
        }

        // otherwise extend the managed value page space
        let start_index = self.next_unused_span_index;
        self.next_unused_span_index = self.next_unused_span_index.saturating_add(len as u64);
        self.ensure_span_storage(start_index, len);

        ManagedSpan::new(Self::span_location_from_index(start_index), len)
    }

    /// Allocate one dedicated large managed span and initialize it from values.
    fn allocate_large_span(&mut self, values: &[Value]) -> ManagedSpan {
        // empty spans
        if values.is_empty() {
            return ManagedSpan::new(PageSlot::ZERO, 0);
        }

        // reuse one free large-span id
        if let Some(id) = self.free_large_span_ids.pop() {
            let large_span_id = ManagedLargeSpanId::new(id);
            let large_span = self
                .large_span_mut(large_span_id)
                .expect("reused large managed span id must stay addressable");
            large_span.replace(values);
            return ManagedSpan::large(large_span_id, values.len());
        }

        // allocate one fresh large span
        let id = self.next_unused_large_span_id;
        self.next_unused_large_span_id = self.next_unused_large_span_id.saturating_add(1);
        self.large_spans.push(ManagedLargeSpan::new(values));
        ManagedSpan::large(ManagedLargeSpanId::new(id), values.len())
    }

    /// Allocate one dedicated large managed span with the requested length.
    fn allocate_large_span_len(&mut self, len: usize) -> ManagedSpan {
        // empty spans
        if len == 0 {
            return ManagedSpan::new(PageSlot::ZERO, 0);
        }

        let values = vec![Value::VOID; len];
        self.allocate_large_span(&values)
    }

    /// Write one contiguous managed span.
    pub(super) fn write_span(&mut self, span: ManagedSpan, values: &[Value]) {
        // dedicated large span
        if let Some(id) = span.large_span_id() {
            let large_span = self
                .large_span_mut(id)
                .expect("allocated large managed span must stay addressable");
            large_span.replace(values);
            return;
        }

        // page-backed span
        for (offset, value) in values.iter().copied().enumerate() {
            let (page_index, page_offset) = Self::span_location(span, offset)
                .expect("allocated managed span offset should be in bounds")
                .position();
            let page = self.value_page_mut(page_index);
            page.set(page_offset, value);
        }
    }

    /// Write one managed span value by local offset.
    fn write_span_value(&mut self, span: ManagedSpan, offset: usize, value: Value) -> Option<()> {
        // dedicated large span
        if let Some(id) = span.large_span_id() {
            let slot = self.large_span_mut(id)?.get_mut(offset)?;
            *slot = value;
            return Some(());
        }

        // page-backed span
        let (page_index, page_offset) = Self::span_location(span, offset)?.position();
        self.value_pages
            .get_mut(page_index)?
            .set(page_offset, value);
        Some(())
    }

    /// Clear one contiguous managed span.
    pub(super) fn clear_span(&mut self, span: ManagedSpan) {
        // dedicated large span
        if let Some(id) = span.large_span_id() {
            if let Some(large_span) = self.large_span_mut(id) {
                large_span.replace(&[]);
            }
            return;
        }

        // page-backed span
        for offset in 0..span.len() {
            let (page_index, page_offset) = Self::span_location(span, offset)
                .expect("cleared managed span offset should be in bounds")
                .position();
            let page = self.value_page_mut(page_index);
            let _ = page.take(page_offset);
        }
    }

    /// Fill one managed span range with one value.
    pub(super) fn fill_span_range(
        &mut self,
        span: ManagedSpan,
        start_offset: usize,
        len: usize,
        value: Value,
    ) {
        // dedicated large span
        if let Some(id) = span.large_span_id() {
            let Some(large_span) = self.large_span_mut(id) else {
                return;
            };
            let values = large_span.values_mut();
            let end_offset = start_offset.saturating_add(len);

            // reject out-of-bounds fill
            if end_offset > values.len() {
                return;
            }

            // fill one contiguous large-span range
            values[start_offset..end_offset].fill(value);
            return;
        }

        // fill one page-backed span range
        for offset in start_offset..start_offset.saturating_add(len) {
            let (page_index, page_offset) = Self::span_location(span, offset)
                .expect("filled managed span offset should be in bounds")
                .position();
            let page = self.value_page_mut(page_index);
            page.set(page_offset, value);
        }
    }

    /// Read one managed span value by local offset.
    pub(super) fn read_span_value(&self, span: ManagedSpan, offset: usize) -> Option<Value> {
        // dedicated large span
        if let Some(id) = span.large_span_id() {
            return Some(*self.large_span(id)?.get(offset)?);
        }

        // page-backed span
        let (page_index, page_offset) = Self::span_location(span, offset)?.position();
        Some(*self.value_pages.get(page_index)?.get(page_offset)?)
    }

    /// Copy one managed allocation prefix into one inline buffer.
    pub(super) fn copy_slots_into_inline(
        &self,
        handle: ManagedReference,
        inline_values: &mut [Value; INLINE_ALLOCATION_VALUES],
        len: usize,
    ) -> Option<()> {
        let allocation = self.get(handle)?;

        // copy inline values directly
        if let Some(slots) = allocation.inline_values() {
            inline_values[..len].copy_from_slice(&slots[..len]);
            return Some(());
        }

        // copy values out of one span
        let span = allocation.span()?;
        for (offset, slot) in inline_values.iter_mut().take(len).enumerate() {
            *slot = self.read_span_value(span, offset)?;
        }

        Some(())
    }

    /// Copy one managed allocation prefix into one managed span.
    pub(super) fn copy_slots_into_span(
        &mut self,
        handle: ManagedReference,
        target_span: ManagedSpan,
        len: usize,
    ) -> Option<()> {
        let allocation = self.get(handle)?;

        // copy inline values into one span
        if let Some(slots) = allocation.inline_values() {
            let mut copied = [Value::VOID; INLINE_ALLOCATION_VALUES];
            copied[..len].copy_from_slice(&slots[..len]);

            for (offset, value) in copied.into_iter().take(len).enumerate() {
                self.write_span_value(target_span, offset, value)?;
            }

            return Some(());
        }

        // copy one span into another span
        let span = allocation.span()?;
        for offset in 0..len {
            let value = self.read_span_value(span, offset)?;
            self.write_span_value(target_span, offset, value)?;
        }

        Some(())
    }

    /// Try to extend one managed span in place.
    pub(super) fn try_extend_span(
        &mut self,
        span: ManagedSpan,
        additional_len: usize,
    ) -> Option<ManagedSpan> {
        // large spans do not extend in place here
        if span.is_large() {
            return None;
        }

        // unchanged span
        if additional_len == 0 {
            return Some(span);
        }

        let next_start = Self::span_advance(
            span.start()
                .expect("page-backed managed span must keep one start location"),
            span.len(),
        );
        let next_start_index = Self::span_index(next_start);

        // extend directly into never-before-used managed span space
        if next_start_index == self.next_unused_span_index {
            self.next_unused_span_index = self
                .next_unused_span_index
                .saturating_add(additional_len as u64);
            self.ensure_span_storage(next_start_index, additional_len);
            return Some(ManagedSpan::new(
                span.start()
                    .expect("page-backed managed span must keep one start location"),
                span.len().saturating_add(additional_len),
            ));
        }

        // extend into one adjacent free span
        let free_index = self.free_spans.iter().position(|free_span| {
            free_span.start() == Some(next_start) && free_span.len() >= additional_len
        })?;
        let free_span = self.free_spans.remove(free_index);
        let remaining = free_span.len().saturating_sub(additional_len);

        if remaining > 0 {
            let remaining_start = Self::span_advance(
                free_span
                    .start()
                    .expect("free span must keep one start location"),
                additional_len,
            );
            self.insert_free_span(ManagedSpan::new(remaining_start, remaining));
        }

        Some(ManagedSpan::new(
            span.start()
                .expect("page-backed managed span must keep one start location"),
            span.len().saturating_add(additional_len),
        ))
    }

    /// Return one managed span to the free list.
    pub(super) fn free_span(&mut self, span: ManagedSpan) {
        // empty spans
        if span.is_empty() {
            return;
        }

        // dedicated large spans
        if let Some(id) = span.large_span_id() {
            self.free_large_span(id);
            return;
        }

        // page-backed spans
        self.clear_span(span);
        self.insert_free_span(span);
    }

    /// Update the tracked allocated bytes after one resize.
    pub(super) fn update_allocated_bytes(&mut self, old_bytes: u64, new_bytes: u64) {
        // growing allocations
        if new_bytes >= old_bytes {
            self.allocated_bytes = self
                .allocated_bytes
                .saturating_add(new_bytes.saturating_sub(old_bytes));
        }
        // shrinking allocations
        else {
            self.allocated_bytes = self
                .allocated_bytes
                .saturating_sub(old_bytes.saturating_sub(new_bytes));
        }
    }

    /// Insert one free managed span and coalesce adjacent neighbors.
    pub(super) fn insert_free_span(&mut self, span: ManagedSpan) {
        self.free_spans.push(span);
        self.free_spans
            .sort_by_key(|span| Self::span_index(span.start().expect("free span must be paged")));

        let mut merged: Vec<ManagedSpan> = Vec::with_capacity(self.free_spans.len());

        // merge adjacent spans
        for span in self.free_spans.drain(..) {
            if let Some(previous) = merged.last_mut() {
                let previous_start = previous.start().expect("free span must be paged");
                let span_start = span.start().expect("free span must be paged");
                let previous_end = Self::span_advance(previous_start, previous.len());
                if previous_end == span_start {
                    let merged_len = previous.len().saturating_add(span.len());
                    *previous = ManagedSpan::new(previous_start, merged_len);
                    continue;
                }
            }

            merged.push(span);
        }

        // install the merged free list
        self.free_spans = merged;
    }

    /// Return one immutable span value reference.
    pub(super) fn span_value(&self, span: ManagedSpan, index: usize) -> Option<&Value> {
        // dedicated large span
        if let Some(id) = span.large_span_id() {
            return self.large_span(id)?.get(index);
        }

        // page-backed span
        let (page_index, page_offset) = Self::span_location(span, index)?.position();
        self.value_pages.get(page_index)?.get(page_offset)
    }

    /// Return one mutable span value reference.
    pub(super) fn span_value_mut(&mut self, span: ManagedSpan, index: usize) -> Option<&mut Value> {
        // dedicated large span
        if let Some(id) = span.large_span_id() {
            return self.large_span_mut(id)?.get_mut(index);
        }

        // page-backed span
        let (page_index, page_offset) = Self::span_location(span, index)?.position();
        self.value_pages.get_mut(page_index)?.get_mut(page_offset)
    }

    /// Return one immutable span value reference without bounds checks.
    pub(super) unsafe fn span_value_unchecked(&self, span: ManagedSpan, index: usize) -> &Value {
        // dedicated large span
        if let Some(id) = span.large_span_id() {
            return self
                .large_span(id)
                .expect("large managed span must stay addressable")
                .get(index)
                .expect("large managed span value out of bounds");
        }

        // page-backed span
        let (page_index, page_offset) = Self::span_location(span, index)
            .expect("managed span value out of bounds")
            .position();
        let page = self.value_page(page_index);

        unsafe { page.get_unchecked(page_offset) }
    }

    /// Return one mutable span value reference without bounds checks.
    pub(super) unsafe fn span_value_unchecked_mut(
        &mut self,
        span: ManagedSpan,
        index: usize,
    ) -> &mut Value {
        // dedicated large span
        if let Some(id) = span.large_span_id() {
            return self
                .large_span_mut(id)
                .expect("large managed span must stay addressable")
                .get_mut(index)
                .expect("large managed span value out of bounds");
        }

        // page-backed span
        let (page_index, page_offset) = Self::span_location(span, index)
            .expect("managed span value out of bounds")
            .position();
        let page = self.value_page_mut(page_index);

        unsafe { page.get_unchecked_mut(page_offset) }
    }

    /// Recycle one dedicated large managed span slot.
    fn free_large_span(&mut self, large_span_id: ManagedLargeSpanId) {
        let Some(large_span) = self.large_span_mut(large_span_id) else {
            return;
        };

        large_span.replace(&[]);
        self.free_large_span_ids.push(large_span_id.id());
    }

    /// Return one live value page by index.
    #[inline(always)]
    fn value_page(&self, index: usize) -> &ValuePage {
        debug_assert!(index < self.value_pages.len(), "value page out of bounds");

        unsafe { self.value_pages.get_unchecked(index) }
    }

    /// Return one mutable live value page by index.
    #[inline(always)]
    fn value_page_mut(&mut self, index: usize) -> &mut ValuePage {
        debug_assert!(index < self.value_pages.len(), "value page out of bounds");

        unsafe { self.value_pages.get_unchecked_mut(index) }
    }

    /// Return one shared large managed span.
    fn large_span(&self, large_span_id: ManagedLargeSpanId) -> Option<&ManagedLargeSpan> {
        // reject null ids
        if large_span_id.id() == 0 {
            return None;
        }

        self.large_spans.get((large_span_id.id() - 1) as usize)
    }

    /// Return one mutable large managed span.
    pub(super) fn large_span_mut(
        &mut self,
        large_span_id: ManagedLargeSpanId,
    ) -> Option<&mut ManagedLargeSpan> {
        // reject null ids
        if large_span_id.id() == 0 {
            return None;
        }

        self.large_spans.get_mut((large_span_id.id() - 1) as usize)
    }

    /// Map one global span index to one value page position.
    fn span_position(index: u64) -> (usize, usize) {
        let index = index as usize;
        (index / VALUE_PAGE_CAPACITY, index % VALUE_PAGE_CAPACITY)
    }

    /// Return the value page index for one global span index.
    pub(super) fn span_page_index(index: u64) -> usize {
        let (page_index, _) = Self::span_position(index);
        page_index
    }

    /// Return the allocation location for one linear span index.
    fn span_location_from_index(index: u64) -> PageSlot {
        let (page_index, page_offset) = Self::span_position(index);
        PageSlot::new(page_index, page_offset)
    }

    /// Return the linear span index for one allocation location.
    pub(super) fn span_index(page_slot: PageSlot) -> u64 {
        (page_slot.page_index() as u64)
            .saturating_mul(VALUE_PAGE_CAPACITY as u64)
            .saturating_add(page_slot.slot_index() as u64)
    }

    /// Return one allocation location advanced by one local span offset.
    pub(super) fn span_advance(page_slot: PageSlot, offset: usize) -> PageSlot {
        Self::span_location_from_index(Self::span_index(page_slot).saturating_add(offset as u64))
    }

    /// Return one allocation location inside one managed span.
    pub(super) fn span_location(span: ManagedSpan, offset: usize) -> Option<PageSlot> {
        // reject out-of-bounds offsets
        if offset >= span.len() {
            return None;
        }

        Some(Self::span_advance(span.start()?, offset))
    }
}
