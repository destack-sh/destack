use super::heap::ManagedHeap;
use super::{INLINE_ALLOCATION_VALUES, ManagedAllocation, ManagedSpan, ManagedSpanClass};
use crate::heap::HeapLimitError;
use crate::page::{ManagedPage, PageSlot};
use crate::value::{ManagedReference, Value};

impl ManagedHeap {
    /// Resize the slot storage for one managed allocation with exact retained-byte admission.
    pub(crate) fn resize_slots_checked(
        &mut self,
        handle: ManagedReference,
        len: usize,
        mut admit: impl FnMut(i64) -> Result<(), HeapLimitError>,
    ) -> Result<bool, HeapLimitError> {
        let Some(retained_delta) = self.resize_slots_delta(handle, len) else {
            return Ok(false);
        };

        admit(retained_delta)?;

        let resized = self.resize_slots(handle, len);

        if resized {
            self.apply_retained_delta(retained_delta);
        }

        Ok(resized)
    }

    /// Get an immutable slot value.
    #[inline(always)]
    pub fn get_slot(&self, handle: ManagedReference, index: usize) -> Option<&Value> {
        let allocation = self.get(handle)?;

        // inline allocations
        if let Some(value) = allocation.get(index) {
            return Some(value);
        }

        // span backed values
        let span = allocation.span()?;
        self.span_value(span, index)
    }

    /// Get a mutable slot value.
    #[inline(always)]
    pub fn get_slot_mut(&mut self, handle: ManagedReference, index: usize) -> Option<&mut Value> {
        // span backed values
        if let Some(span) = self.get(handle)?.span() {
            return self.span_value_mut(span, index);
        }

        // inline allocations
        self.get_mut(handle)?.get_mut(index)
    }

    /// Get an immutable slot value without bounds checks.
    ///
    /// # Safety
    /// Caller must ensure the slot exists for this pointer.
    #[inline(always)]
    pub unsafe fn get_slot_unchecked(&self, handle: ManagedReference, index: usize) -> &Value {
        let address = self
            .location(handle)
            .expect("managed reference must resolve to one stable allocation location");
        let allocation = Self::allocation_from_pages_unchecked(&self.pages, address);

        // inline allocations
        if let Some(slots) = allocation.inline_values() {
            return unsafe { slots.get_unchecked(index) };
        }

        // span-backed allocations
        let span = allocation.span().expect("managed allocation missing span");
        unsafe { self.span_value_unchecked(span, index) }
    }

    /// Get a mutable slot value without bounds checks.
    ///
    /// # Safety
    /// Caller must ensure the slot exists for this pointer.
    #[inline(always)]
    pub unsafe fn get_slot_unchecked_mut(
        &mut self,
        handle: ManagedReference,
        index: usize,
    ) -> &mut Value {
        // span backed values
        if let Some(span) = unsafe { self.get_unchecked(handle) }.span() {
            return unsafe { self.span_value_unchecked_mut(span, index) };
        }

        // inline allocations
        let allocation = unsafe { self.get_unchecked_mut(handle) };
        unsafe { allocation.get_unchecked_mut(index) }
    }

    /// Return the inline slots for one managed allocation when they exist.
    #[inline]
    pub fn inline_slots(&self, handle: ManagedReference) -> Option<&[Value]> {
        let allocation = self.get(handle)?;
        allocation.inline_values()
    }

    /// Return the mutable inline slots for one managed allocation when they exist.
    #[inline]
    pub fn inline_slots_mut(&mut self, handle: ManagedReference) -> Option<&mut [Value]> {
        let allocation = self.get_mut(handle)?;
        allocation.inline_values_mut()
    }

    /// Visit each slot value for one managed allocation.
    pub fn for_each_slot(
        &self,
        handle: ManagedReference,
        mut visit: impl FnMut(Value),
    ) -> Option<()> {
        let allocation = self.get(handle)?;

        // inline values
        if let Some(slots) = allocation.inline_values() {
            for value in slots.iter().copied() {
                visit(value);
            }

            return Some(());
        }

        // span backed values
        let span = allocation.span()?;
        for offset in 0..span.len() {
            let value = self.read_span_value(span, offset)?;
            visit(value);
        }

        Some(())
    }

    /// Clone all slot values for one managed allocation.
    pub fn copy_slots(&self, handle: ManagedReference) -> Option<Vec<Value>> {
        let mut values = Vec::with_capacity(self.get(handle)?.len());
        self.for_each_slot(handle, |value| values.push(value))?;

        Some(values)
    }

    /// Set a slot value.
    #[inline(always)]
    pub fn set_slot(&mut self, handle: ManagedReference, index: usize, value: Value) -> bool {
        let source_marked = self.is_marked(handle);
        self.write_barrier(source_marked, value);

        let Some(slot) = self.get_slot_mut(handle, index) else {
            return false;
        };

        *slot = value;
        true
    }

    /// Resize the slot storage for one managed allocation.
    pub fn resize_slots(&mut self, handle: ManagedReference, len: usize) -> bool {
        let Some(allocation) = self.get(handle) else {
            return false;
        };
        let old_len = allocation.len();
        let old_bytes = Self::allocation_size_for(allocation);
        let previous_span = allocation.span();

        // unchanged length
        if old_len == len {
            return true;
        }

        // resize inline without changing storage class
        if previous_span.is_none() && ManagedAllocation::can_inline(len) {
            let Some(allocation) = self.get_mut(handle) else {
                return false;
            };
            allocation.resize_inline(len, Value::VOID);
            let new_bytes = Self::allocation_size_for(allocation);
            self.update_allocated_bytes(old_bytes, new_bytes);
            return true;
        }

        // shrink one managed span in place
        if let Some(span) = previous_span {
            // resize one dedicated large span in place
            if span.is_large()
                && !ManagedAllocation::can_inline(len)
                && self.span_class(len) == ManagedSpanClass::Large
            {
                let Some(large_span_id) = span.large_span_id() else {
                    return false;
                };
                let Some(large_span) = self.large_span_mut(large_span_id) else {
                    return false;
                };
                large_span.resize(len, Value::VOID);

                let Some(allocation) = self.get_mut(handle) else {
                    return false;
                };
                *allocation = ManagedAllocation::with_span(ManagedSpan::large(large_span_id, len));

                let new_bytes = Self::allocation_size_for(allocation);
                self.update_allocated_bytes(old_bytes, new_bytes);
                return true;
            }

            // shrink one page-backed span in place
            if !span.is_large() && len < old_len && !ManagedAllocation::can_inline(len) {
                let Some(freed_start) = Self::span_location(span, len) else {
                    return false;
                };
                let freed_len = old_len - len;
                let freed_span = ManagedSpan::new(freed_start, freed_len);

                self.clear_span(freed_span);
                self.insert_free_span(freed_span);

                let Some(allocation) = self.get_mut(handle) else {
                    return false;
                };
                *allocation = ManagedAllocation::with_span(ManagedSpan::new(
                    span.start()
                        .expect("page-backed managed span must keep one start location"),
                    len,
                ));

                let new_bytes = Self::allocation_size_for(allocation);
                self.update_allocated_bytes(old_bytes, new_bytes);
                return true;
            }

            // grow one page-backed span in place when the right neighbor is free
            if !span.is_large()
                && len > old_len
                && self.span_class(len) == ManagedSpanClass::Paged
                && let Some(extended_span) = self.try_extend_span(span, len.saturating_sub(old_len))
            {
                let growth_start = old_len;
                let growth_len = len.saturating_sub(old_len);
                self.fill_span_range(extended_span, growth_start, growth_len, Value::VOID);

                let Some(allocation) = self.get_mut(handle) else {
                    return false;
                };
                *allocation = ManagedAllocation::with_span(extended_span);

                let new_bytes = Self::allocation_size_for(allocation);
                self.update_allocated_bytes(old_bytes, new_bytes);
                return true;
            }
        }

        // move between inline and span storage when the storage class changes
        let replacement = if ManagedAllocation::can_inline(len) {
            let mut inline_values = [Value::VOID; INLINE_ALLOCATION_VALUES];
            let copy_len = old_len.min(len);

            // copy into inline storage
            if self
                .copy_slots_into_inline(handle, &mut inline_values, copy_len)
                .is_none()
            {
                return false;
            }

            ManagedAllocation::Inline {
                len: len as u8,
                values: inline_values,
            }
        } else {
            let replacement_span = self.allocate_span_len(len);
            let copy_len = old_len.min(len);

            // copy into span storage
            if copy_len > 0
                && self
                    .copy_slots_into_span(handle, replacement_span, copy_len)
                    .is_none()
            {
                return false;
            }

            // initialize grown tail
            if len > copy_len {
                self.fill_span_range(replacement_span, copy_len, len - copy_len, Value::VOID);
            }

            ManagedAllocation::with_span(replacement_span)
        };
        let new_bytes = Self::allocation_size_for(&replacement);

        // install the replacement allocation
        let Some(allocation) = self.get_mut(handle) else {
            return false;
        };
        *allocation = replacement;

        // recycle old span storage after the replacement is installed
        if let Some(span) = previous_span {
            self.free_span(span);
        }

        self.update_allocated_bytes(old_bytes, new_bytes);

        true
    }

    /// Return the approximate byte size for one allocation.
    pub fn allocation_size(&self, pointer: ManagedReference) -> Option<u64> {
        let allocation = self.get(pointer)?;
        Some(Self::allocation_size_for(allocation))
    }

    /// Return the approximate byte size for one allocation.
    pub fn allocation_size_for(allocation: &ManagedAllocation) -> u64 {
        allocation.logical_bytes()
    }

    /// Return the exact retained-byte delta for resizing one managed allocation.
    fn resize_slots_delta(&self, handle: ManagedReference, len: usize) -> Option<i64> {
        let allocation = self.get(handle)?;
        let old_len = allocation.len();
        let previous_span = allocation.span();

        if old_len == len {
            return Some(0);
        }

        // resize inline without changing storage class
        if previous_span.is_none() && ManagedAllocation::can_inline(len) {
            return Some(0);
        }

        // resize existing span storage
        if let Some(span) = previous_span {
            // resize one dedicated large span in place
            if span.is_large()
                && !ManagedAllocation::can_inline(len)
                && self.span_class(len) == ManagedSpanClass::Large
            {
                let delta = (len as i64 - old_len as i64) * std::mem::size_of::<Value>() as i64;
                return Some(delta);
            }

            // shrink one page-backed span in place
            if !span.is_large() && len < old_len && !ManagedAllocation::can_inline(len) {
                let freed_start = Self::span_location(span, len)?;
                let freed_len = old_len.saturating_sub(len);
                let freed_span = ManagedSpan::new(freed_start, freed_len);

                return Some(self.free_span_delta(freed_span));
            }

            // grow one page-backed span in place when possible
            if !span.is_large()
                && len > old_len
                && self.span_class(len) == ManagedSpanClass::Paged
                && let Some(delta) = self.paged_span_extend_delta(span, len)
            {
                return Some(delta);
            }
        }

        // move between inline and span storage when the storage class changes
        Some(self.storage_change_delta(previous_span, len))
    }

    /// Return the exact retained-byte delta for growing one paged span in place.
    fn paged_span_extend_delta(&self, span: ManagedSpan, len: usize) -> Option<i64> {
        let additional_len = len.saturating_sub(span.len());

        // unchanged or non-paged spans
        if additional_len == 0 || span.is_large() {
            return Some(0);
        }

        let next_start = Self::span_advance(
            span.start()
                .expect("page-backed managed span must keep one start location"),
            span.len(),
        );
        let next_start_index = Self::span_index(next_start);

        // extend into never-before-used value page space
        if next_start_index == self.next_unused_span_index {
            let last_index = next_start_index
                .saturating_add(additional_len as u64)
                .saturating_sub(1);
            let required_page = Self::span_page_index(last_index);
            let added_pages = required_page
                .saturating_add(1)
                .saturating_sub(self.value_pages.len());
            let delta = added_pages
                * (std::mem::size_of::<crate::page::ValuePage>()
                    + super::heap::VALUE_PAGE_PAYLOAD_BYTES);

            return Some(delta as i64);
        }

        // extend into one adjacent free span
        let free_index = self.free_spans.iter().position(|free_span| {
            free_span.start() == Some(next_start) && free_span.len() >= additional_len
        })?;
        let free_span = self.free_spans[free_index];
        let remaining = free_span.len().saturating_sub(additional_len);
        let mut delta = -(std::mem::size_of::<ManagedSpan>() as i64);

        if remaining > 0 {
            delta += std::mem::size_of::<ManagedSpan>() as i64;
        }

        Some(delta)
    }

    /// Return the exact retained-byte delta for freeing one managed span.
    pub(super) fn free_span_delta(&self, span: ManagedSpan) -> i64 {
        if span.is_empty() {
            return 0;
        }

        // free one dedicated large span
        if span.is_large() {
            let payload_bytes = span.len().saturating_mul(std::mem::size_of::<Value>()) as i64;
            return (std::mem::size_of::<u64>() as i64) - payload_bytes;
        }

        // free one page-backed span and merge free-list nodes
        let start = span
            .start()
            .expect("page-backed managed span must keep one start location");
        let end = Self::span_advance(start, span.len());

        let merges_left = self.free_spans.iter().any(|free_span| {
            let free_start = free_span
                .start()
                .expect("free managed span must keep one start location");
            let free_end = Self::span_advance(free_start, free_span.len());
            free_end == start
        });
        let merges_right = self
            .free_spans
            .iter()
            .any(|free_span| free_span.start() == Some(end));

        let delta = 1 - (merges_left as i64) - (merges_right as i64);
        delta * std::mem::size_of::<ManagedSpan>() as i64
    }

    /// Return the exact retained-byte delta for changing one allocation storage class.
    fn storage_change_delta(&self, previous_span: Option<ManagedSpan>, len: usize) -> i64 {
        let removed_delta = previous_span
            .map(|span| self.free_span_delta(span))
            .unwrap_or_default();

        let added_delta = if ManagedAllocation::can_inline(len) {
            0
        } else {
            self.span_allocate_delta(len)
        };

        removed_delta + added_delta
    }

    /// Return one managed allocation payload without bounds checks from one page set.
    ///
    /// # Safety
    /// Caller must ensure the address is valid and allocated.
    pub(super) fn allocation_from_pages_unchecked(
        pages: &[ManagedPage],
        page_slot: PageSlot,
    ) -> &ManagedAllocation {
        let (target_page, target_offset) = page_slot.position();

        unsafe {
            pages
                .get_unchecked(target_page)
                .get_unchecked(target_offset)
        }
    }
}
