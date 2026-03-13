use crate::heap::{DEFAULT_LARGE_SPAN_TARGET_BYTES, HeapLimitError};
use crate::page::{PageSlot, RAW_PAGE_CAPACITY, RawPage};
use crate::value::{RawPointer, Value};
use crate::{RawAllocation, RawAllocationStorage, RawSpan, RawSpanClass, RawSpanReference};
use std::mem::size_of;

/// The first non-null raw allocation id.
const FIRST_ALLOCATED_ALLOCATION_ID: u64 = 1;
/// The first non-null ordinary raw span id.
const FIRST_ALLOCATED_RAW_SPAN_ID: u64 = 1;
/// The first non-null dedicated large raw span id.
const FIRST_ALLOCATED_LARGE_RAW_SPAN_ID: u64 = 1;

/// The retained payload bytes for one empty raw page.
const RAW_PAGE_PAYLOAD_BYTES: usize = RAW_PAGE_CAPACITY * size_of::<RawAllocation>();

/// A raw heap for manual memory management.
#[derive(Debug)]
pub struct RawHeap {
    /// Raw pages keyed by global allocation id.
    pub(super) pages: Vec<RawPage>,
    /// Stable raw spans keyed by span id minus one.
    pub(super) spans: Vec<RawSpan>,
    /// Stable large raw spans keyed by large span id minus one.
    pub(super) large_spans: Vec<RawSpan>,
    /// Stable allocation locations keyed by raw allocation id minus one.
    pub(super) locations: Vec<PageSlot>,

    /// Free global raw allocation ids available for reuse.
    pub(super) free_ids: Vec<u64>,
    /// Free raw span ids available for reuse.
    pub(super) free_span_ids: Vec<u64>,
    /// Free large raw span ids available for reuse.
    pub(super) free_large_span_ids: Vec<u64>,

    /// The next global raw allocation id to allocate.
    pub(super) next_unused_id: u64,
    /// The next global raw span id to allocate.
    pub(super) next_unused_span_id: u64,
    /// The next global large raw span id to allocate.
    pub(super) next_unused_large_span_id: u64,
    /// The next raw page slot to allocate for one new id.
    pub(super) next_unused_slot: u64,

    /// Count of live raw allocations, excluding the null slot.
    pub(super) allocated_count: usize,
    /// Approximate live raw allocation bytes.
    pub(super) allocated_bytes: u64,
    /// Exact retained raw heap bytes.
    pub(super) retained_bytes: u64,
    /// The dedicated large-span threshold in bytes.
    pub(super) large_span_bytes: usize,
}

impl Default for RawHeap {
    fn default() -> Self {
        Self::new()
    }
}

impl RawHeap {
    /// Create a new empty raw heap.
    pub fn new() -> Self {
        Self::with_large_span_bytes(DEFAULT_LARGE_SPAN_TARGET_BYTES)
    }

    /// Create a new empty raw heap with one explicit large-span threshold.
    pub(crate) fn with_large_span_bytes(large_span_bytes: usize) -> Self {
        let pages = vec![RawPage::new()];

        Self {
            pages,
            spans: Vec::new(),
            large_spans: Vec::new(),
            locations: Vec::new(),
            free_ids: Vec::new(),
            free_span_ids: Vec::new(),
            free_large_span_ids: Vec::new(),
            next_unused_id: FIRST_ALLOCATED_ALLOCATION_ID,
            next_unused_span_id: FIRST_ALLOCATED_RAW_SPAN_ID,
            next_unused_large_span_id: FIRST_ALLOCATED_LARGE_RAW_SPAN_ID,
            next_unused_slot: 0,
            allocated_count: 0,
            allocated_bytes: 0,
            retained_bytes: 0,
            large_span_bytes,
        }
        .with_retained_bytes()
    }

    /// Return the dedicated raw large-span threshold in bytes.
    pub(crate) fn large_span_bytes(&self) -> usize {
        self.large_span_bytes
    }

    /// Return the raw span class for one byte length.
    pub(crate) fn span_class(&self, len: usize) -> RawSpanClass {
        if len >= self.large_span_bytes {
            RawSpanClass::Large
        } else {
            RawSpanClass::Regular
        }
    }

    /// Allocate one new raw allocation and return its pointer.
    pub(crate) fn allocate_checked(
        &mut self,
        admit: impl FnMut(i64) -> Result<(), HeapLimitError>,
    ) -> Result<RawPointer, HeapLimitError> {
        self.allocate_empty_checked(admit)
    }

    /// Allocate one raw value allocation with a given slot count.
    pub(crate) fn allocate_with_slots_checked(
        &mut self,
        slot_count: usize,
        admit: impl FnMut(i64) -> Result<(), HeapLimitError>,
    ) -> Result<RawPointer, HeapLimitError> {
        self.allocate_checked_with(RawAllocation::with_slots(slot_count), admit)
    }

    /// Allocate one raw value allocation with the given slot values.
    pub(crate) fn allocate_with_values_checked(
        &mut self,
        slots: Vec<Value>,
        admit: impl FnMut(i64) -> Result<(), HeapLimitError>,
    ) -> Result<RawPointer, HeapLimitError> {
        self.allocate_checked_with(RawAllocation::with_values(slots), admit)
    }

    /// Allocate one raw byte allocation with the given payload.
    pub(crate) fn allocate_with_bytes_checked(
        &mut self,
        bytes: &[u8],
        mut admit: impl FnMut(i64) -> Result<(), HeapLimitError>,
    ) -> Result<RawPointer, HeapLimitError> {
        let retained_delta =
            self.location_allocate_delta() + self.span_allocate_delta_for_bytes(bytes.len());
        admit(retained_delta)?;

        // choose the target span class
        let span = match self.span_class(bytes.len()) {
            RawSpanClass::Regular => RawSpanReference::Regular(self.allocate_span(bytes)),
            RawSpanClass::Large => RawSpanReference::Large(self.allocate_large_span(bytes)),
        };

        // install the byte allocation
        let pointer = self.allocate_with(RawAllocation {
            storage: RawAllocationStorage::Bytes(span),
        });
        self.apply_retained_delta(retained_delta);

        Ok(pointer)
    }

    /// Allocate one empty raw allocation and return its pointer.
    fn allocate_empty_checked(
        &mut self,
        admit: impl FnMut(i64) -> Result<(), HeapLimitError>,
    ) -> Result<RawPointer, HeapLimitError> {
        self.allocate_checked_with(RawAllocation::new(), admit)
    }

    /// Allocate one raw allocation and return its pointer.
    fn allocate_checked_with(
        &mut self,
        allocation: RawAllocation,
        mut admit: impl FnMut(i64) -> Result<(), HeapLimitError>,
    ) -> Result<RawPointer, HeapLimitError> {
        let retained_delta = self.location_allocate_delta() + allocation.retained_bytes() as i64;
        admit(retained_delta)?;

        let pointer = self.allocate_with(allocation);
        self.apply_retained_delta(retained_delta);

        Ok(pointer)
    }

    /// Allocate one raw allocation and return its pointer.
    fn allocate_with(&mut self, allocation: RawAllocation) -> RawPointer {
        // reuse or allocate one stable location
        let (allocation_id, address) = if let Some(allocation_id) = self.free_ids.pop() {
            let address = self
                .location_for_id(allocation_id)
                .expect("reused raw allocation id must keep one stable page slot");
            (allocation_id, address)
        } else {
            let allocation_id = self.next_unused_id;
            self.next_unused_id = self.next_unused_id.saturating_add(1);
            let address = self.allocate_location();
            self.locations.push(address);
            (allocation_id, address)
        };

        let pointer = RawPointer::new(allocation_id);
        let (target_page, target_offset) = address.position();
        let allocation_bytes = self.allocation_size_for(&allocation);

        // install the allocation
        {
            let page = self.page_mut(target_page);
            page.set(target_offset, allocation);
        }

        // update usage
        self.allocated_count += 1;
        self.allocated_bytes = self.allocated_bytes.saturating_add(allocation_bytes);

        pointer
    }

    /// Initialize exact retained-byte accounting after one bulk construction path.
    pub(super) fn with_retained_bytes(mut self) -> Self {
        self.recompute_retained_bytes();
        self
    }

    /// Return the exact retained-byte delta for allocating one stable raw location.
    fn location_allocate_delta(&self) -> i64 {
        // reusing one free id shrinks the free-id list
        if !self.free_ids.is_empty() {
            return -(size_of::<u64>() as i64);
        }

        // otherwise account for the new location entry and maybe one page
        let mut delta = size_of::<PageSlot>() as i64;
        let slot = self.next_unused_slot as usize;
        let page_index = slot / RAW_PAGE_CAPACITY;

        if self.pages.len() <= page_index {
            delta += (size_of::<RawPage>() + RAW_PAGE_PAYLOAD_BYTES) as i64;
        }

        delta
    }

    /// Return the exact retained-byte delta for allocating one raw byte span payload.
    pub(crate) fn span_allocate_delta_for_bytes(&self, len: usize) -> i64 {
        let payload_bytes = len as i64;

        // regular spans
        match self.span_class(len) {
            RawSpanClass::Regular => {
                if self.free_span_ids.is_empty() {
                    payload_bytes + size_of::<RawSpan>() as i64
                } else {
                    payload_bytes - size_of::<u64>() as i64
                }
            }

            // dedicated large spans
            RawSpanClass::Large => {
                if self.free_large_span_ids.is_empty() {
                    payload_bytes + size_of::<RawSpan>() as i64
                } else {
                    payload_bytes - size_of::<u64>() as i64
                }
            }
        }
    }

    /// Get one raw allocation by pointer.
    #[inline]
    pub fn get(&self, pointer: RawPointer) -> Option<&RawAllocation> {
        let allocation_id = pointer.id();

        // reject null or never-allocated ids
        if allocation_id == 0 || allocation_id >= self.next_unused_id {
            return None;
        }

        // resolve one stable page slot
        let address = self.location(pointer)?;
        let (target_page, target_offset) = address.position();
        let page = self.pages.get(target_page)?;

        page.get(target_offset)
    }

    /// Get one raw allocation mutably by pointer.
    #[inline]
    pub fn get_mut(&mut self, pointer: RawPointer) -> Option<&mut RawAllocation> {
        let allocation_id = pointer.id();

        // reject null or never-allocated ids
        if allocation_id == 0 || allocation_id >= self.next_unused_id {
            return None;
        }

        // resolve one stable page slot
        let address = self.location(pointer)?;
        let (target_page, target_offset) = address.position();
        let page = self.pages.get_mut(target_page)?;

        page.get_mut(target_offset)
    }

    /// Resize one raw value allocation.
    #[inline]
    pub(crate) fn resize_values_checked(
        &mut self,
        pointer: RawPointer,
        len: usize,
        mut admit: impl FnMut(i64) -> Result<(), HeapLimitError>,
    ) -> Result<bool, HeapLimitError> {
        let Some(delta) = self.resize_values_delta(pointer, len) else {
            return Ok(false);
        };
        admit(delta)?;

        let resized = self.resize_values(pointer, len);

        if resized {
            self.apply_retained_delta(delta);
        }

        Ok(resized)
    }

    /// Resize one raw value allocation.
    #[inline]
    pub fn resize_values(&mut self, pointer: RawPointer, len: usize) -> bool {
        // resolve one value allocation
        let Some(allocation) = self.get_mut(pointer) else {
            return false;
        };
        let Some(values) = allocation.values_mut() else {
            return false;
        };

        values.resize(len, Value::VOID);
        true
    }

    /// Set one raw value slot.
    #[inline]
    pub fn set_value(&mut self, pointer: RawPointer, index: usize, value: Value) -> bool {
        // resolve one value allocation
        let Some(allocation) = self.get_mut(pointer) else {
            return false;
        };
        let Some(values) = allocation.values_mut() else {
            return false;
        };
        let Some(slot) = values.get_mut(index) else {
            return false;
        };

        *slot = value;
        true
    }

    /// Replace one raw value allocation payload.
    #[inline]
    pub(crate) fn replace_values_checked(
        &mut self,
        pointer: RawPointer,
        values: &[Value],
        mut admit: impl FnMut(i64) -> Result<(), HeapLimitError>,
    ) -> Result<bool, HeapLimitError> {
        let Some(delta) = self.replace_values_delta(pointer, values) else {
            return Ok(false);
        };
        admit(delta)?;

        let replaced = self.replace_values(pointer, values);

        if replaced {
            self.apply_retained_delta(delta);
        }

        Ok(replaced)
    }

    /// Replace one raw value allocation payload.
    #[inline]
    pub fn replace_values(&mut self, pointer: RawPointer, values: &[Value]) -> bool {
        // resolve one value allocation
        let Some(allocation) = self.get_mut(pointer) else {
            return false;
        };
        let Some(storage) = allocation.values_mut() else {
            return false;
        };

        *storage = crate::ValueCell::with_values(values.to_vec());
        true
    }

    /// Set one raw value slot without bounds checks.
    ///
    /// # Safety
    /// Caller must ensure the raw pointer resolves to one value allocation and
    /// the slot index is within bounds.
    #[inline]
    pub unsafe fn set_value_unchecked(&mut self, pointer: RawPointer, index: usize, value: Value) {
        let allocation = self
            .get_mut(pointer)
            .expect("raw pointer must resolve to one value allocation");
        let values = allocation
            .values_mut()
            .expect("raw pointer must resolve to one value allocation");

        unsafe { *values.get_unchecked_mut(index) = value };
    }

    /// Free one raw allocation by pointer.
    #[inline]
    pub fn free(&mut self, pointer: RawPointer) -> bool {
        let allocation_id = pointer.id();
        if allocation_id == 0 || allocation_id >= self.next_unused_id {
            return false;
        }

        let Some(address) = self.location(pointer) else {
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
        let mut retained_delta = std::mem::size_of::<u64>() as i64;
        if let RawAllocationStorage::Bytes(span) = allocation.storage {
            retained_delta += self.free_span_delta(span);
            self.free_span(span);
        }

        // recycle allocation id and usage
        self.free_ids.push(allocation_id);
        debug_assert!(self.allocated_count > 0, "heap allocation count underflow");
        self.allocated_count -= 1;
        self.allocated_bytes = self
            .allocated_bytes
            .saturating_sub(self.allocation_size_for(&allocation));
        self.apply_retained_delta(retained_delta);
        true
    }

    /// Return the number of allocated raw allocations.
    #[inline]
    pub fn allocation_count(&self) -> usize {
        self.allocated_count
    }

    /// Check if the heap is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.allocation_count() == 0
    }

    /// Clear all allocations.
    #[inline]
    pub fn clear(&mut self) {
        // reset page and span storage
        self.pages.clear();
        self.pages.push(RawPage::new());
        self.spans.clear();
        self.large_spans.clear();
        self.locations.clear();

        // reset id reuse state
        self.free_ids.clear();
        self.free_span_ids.clear();
        self.free_large_span_ids.clear();
        self.next_unused_id = FIRST_ALLOCATED_ALLOCATION_ID;
        self.next_unused_span_id = FIRST_ALLOCATED_RAW_SPAN_ID;
        self.next_unused_large_span_id = FIRST_ALLOCATED_LARGE_RAW_SPAN_ID;
        self.next_unused_slot = 0;

        // reset usage
        self.allocated_count = 0;
        self.allocated_bytes = 0;
        self.recompute_retained_bytes();
    }

    /// Return one live raw page by index.
    #[inline(always)]
    fn page_mut(&mut self, index: usize) -> &mut RawPage {
        debug_assert!(index < self.pages.len(), "raw page out of bounds");

        unsafe { self.pages.get_unchecked_mut(index) }
    }

    /// Return the exact retained-byte delta for resizing one raw value allocation.
    fn resize_values_delta(&self, pointer: RawPointer, len: usize) -> Option<i64> {
        let allocation = self.get(pointer)?;
        let values = allocation.values()?;
        let old_bytes = values.retained_bytes() as i64;
        let new_bytes = crate::ValueCell::with_values_len(len).retained_bytes() as i64;

        Some(new_bytes - old_bytes)
    }

    /// Return the exact retained-byte delta for replacing one raw value allocation payload.
    fn replace_values_delta(&self, pointer: RawPointer, values: &[Value]) -> Option<i64> {
        let allocation = self.get(pointer)?;
        let previous_values = allocation.values()?;
        let old_bytes = previous_values.retained_bytes() as i64;
        let new_bytes = crate::ValueCell::with_values(values.to_vec()).retained_bytes() as i64;

        Some(new_bytes - old_bytes)
    }
}
