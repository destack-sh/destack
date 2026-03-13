use destack_mir::LayoutId;
use std::mem::size_of;

use crate::heap::{DEFAULT_LARGE_SPAN_TARGET_BYTES, HeapLimitError};
use crate::page::{MANAGED_PAGE_CAPACITY, ManagedPage, PageSlot, VALUE_PAGE_CAPACITY, ValuePage};
use crate::value::{ManagedReference, Value};
use crate::{GcState, ManagedAllocation, ManagedLargeSpan, ManagedSpan, ManagedSpanClass};

/// The first non-null managed reference id.
pub(super) const FIRST_ALLOCATED_REFERENCE_ID: u64 = 1;
/// The first page-backed managed span index.
pub(super) const FIRST_MANAGED_SPAN_INDEX: u64 = 0;
/// The first non-null dedicated managed large-span id.
pub(super) const FIRST_ALLOCATED_LARGE_MANAGED_SPAN_ID: u64 = 1;
/// The default value count that spills into one dedicated managed large span.
pub const DEFAULT_MANAGED_LARGE_SPAN_VALUES: usize =
    DEFAULT_LARGE_SPAN_TARGET_BYTES / size_of::<Value>();

/// The retained payload bytes for one empty managed page.
pub(super) const MANAGED_PAGE_PAYLOAD_BYTES: usize = MANAGED_PAGE_CAPACITY
    * size_of::<ManagedAllocation>()
    + MANAGED_PAGE_CAPACITY * size_of::<u32>()
    + MANAGED_PAGE_CAPACITY * size_of::<Option<LayoutId>>();

/// The retained payload bytes for one empty managed value page.
pub(super) const VALUE_PAGE_PAYLOAD_BYTES: usize = VALUE_PAGE_CAPACITY * size_of::<Value>();

/// A managed heap for interpreter allocations.
#[derive(Debug)]
pub struct ManagedHeap {
    /// Managed pages keyed by global reference id.
    pub(super) pages: Vec<ManagedPage>,
    /// Value pages keyed by global span index.
    pub(super) value_pages: Vec<ValuePage>,
    /// Dedicated large managed spans keyed by large span id minus one.
    pub(super) large_spans: Vec<ManagedLargeSpan>,
    /// Stable allocation locations keyed by managed reference id minus one.
    pub(super) locations: Vec<PageSlot>,

    /// Free global managed reference ids available for reuse.
    pub(super) free_ids: Vec<u64>,
    /// Free managed spans available for reuse.
    pub(super) free_spans: Vec<ManagedSpan>,
    /// Free large managed span ids available for reuse.
    pub(super) free_large_span_ids: Vec<u64>,

    /// The next global managed reference id to allocate.
    pub(super) next_unused_id: u64,
    /// The next managed page slot to allocate for one new id.
    pub(super) next_unused_slot: u64,
    /// The next managed span index to allocate.
    pub(super) next_unused_span_index: u64,
    /// The next large managed span id to allocate.
    pub(super) next_unused_large_span_id: u64,

    /// Count of live managed allocations, excluding the null slot.
    pub(super) allocated_count: usize,
    /// Approximate heap bytes in use.
    pub(super) allocated_bytes: u64,
    /// Exact retained managed heap bytes.
    pub(super) retained_bytes: u64,
    /// GC phase and cycle state.
    pub(super) gc_state: GcState,
    /// Pending mark worklist.
    pub(super) mark_queue: Vec<ManagedReference>,
    /// The dedicated large-span threshold in values.
    pub(super) large_span_values: usize,
}

impl Default for ManagedHeap {
    fn default() -> Self {
        Self::new()
    }
}

impl ManagedHeap {
    /// Create a new empty heap.
    pub fn new() -> Self {
        Self::with_large_span_values(DEFAULT_MANAGED_LARGE_SPAN_VALUES)
    }

    /// Create a new empty heap with one explicit large-span threshold.
    pub(crate) fn with_large_span_values(large_span_values: usize) -> Self {
        let pages = vec![ManagedPage::new()];
        let value_pages = vec![ValuePage::new()];

        Self {
            pages,
            value_pages,
            large_spans: Vec::new(),
            locations: Vec::new(),
            free_ids: Vec::new(),
            free_spans: Vec::new(),
            free_large_span_ids: Vec::new(),
            next_unused_id: FIRST_ALLOCATED_REFERENCE_ID,
            next_unused_slot: 0,
            next_unused_span_index: FIRST_MANAGED_SPAN_INDEX,
            next_unused_large_span_id: FIRST_ALLOCATED_LARGE_MANAGED_SPAN_ID,
            allocated_count: 0,
            allocated_bytes: 0,
            retained_bytes: 0,
            gc_state: GcState::default(),
            mark_queue: Vec::new(),
            large_span_values,
        }
        .with_retained_bytes()
    }

    /// Check whether a handle points to one live managed allocation.
    pub fn is_allocated(&self, handle: ManagedReference) -> bool {
        self.get(handle).is_some()
    }

    /// Check if the heap is empty.
    pub fn is_empty(&self) -> bool {
        self.allocation_count() == 0
    }

    /// Allocate one new managed allocation and return its reference.
    pub(crate) fn allocate_checked(
        &mut self,
        admit: impl FnMut(i64) -> Result<(), HeapLimitError>,
    ) -> Result<ManagedReference, HeapLimitError> {
        self.allocate_empty_checked(admit)
    }

    /// Allocate one managed allocation with a given slot count.
    pub(crate) fn allocate_with_slots_checked(
        &mut self,
        slot_count: usize,
        mut admit: impl FnMut(i64) -> Result<(), HeapLimitError>,
    ) -> Result<ManagedReference, HeapLimitError> {
        // inline allocation
        if ManagedAllocation::can_inline(slot_count) {
            let allocation = ManagedAllocation::with_values_len(slot_count);
            return self.allocate_checked_with(allocation, admit);
        }

        // external allocation
        let span_delta = self.span_allocate_delta(slot_count);
        let allocation = self.allocate_external_with_slots(slot_count, &mut admit, span_delta)?;
        self.allocate_checked_with(allocation, admit)
    }

    /// Allocate one managed allocation with the given slot values.
    pub(crate) fn allocate_with_values_checked(
        &mut self,
        slots: Vec<Value>,
        mut admit: impl FnMut(i64) -> Result<(), HeapLimitError>,
    ) -> Result<ManagedReference, HeapLimitError> {
        // inline allocation
        if ManagedAllocation::can_inline(slots.len()) {
            let allocation = ManagedAllocation::with_values(&slots);
            return self.allocate_checked_with(allocation, admit);
        }

        // external allocation
        let span_delta = self.span_allocate_delta(slots.len());
        let allocation = self.allocate_external_with_values(&slots, &mut admit, span_delta)?;
        self.allocate_checked_with(allocation, admit)
    }

    /// Allocate one managed allocation with exactly 2 slot values.
    #[inline]
    pub(crate) fn allocate_pair_checked(
        &mut self,
        first: Value,
        second: Value,
        admit: impl FnMut(i64) -> Result<(), HeapLimitError>,
    ) -> Result<ManagedReference, HeapLimitError> {
        self.allocate_checked_with(ManagedAllocation::with_pair(first, second), admit)
    }

    /// Allocate one managed allocation with exactly 1 slot value.
    #[inline]
    pub(crate) fn allocate_single_checked(
        &mut self,
        value: Value,
        admit: impl FnMut(i64) -> Result<(), HeapLimitError>,
    ) -> Result<ManagedReference, HeapLimitError> {
        self.allocate_checked_with(ManagedAllocation::with_single(value), admit)
    }

    /// Allocate one empty managed allocation and return its reference.
    #[inline]
    fn allocate_empty_checked(
        &mut self,
        admit: impl FnMut(i64) -> Result<(), HeapLimitError>,
    ) -> Result<ManagedReference, HeapLimitError> {
        self.allocate_checked_with(ManagedAllocation::new(), admit)
    }

    /// Allocate one managed allocation and return its reference.
    #[inline]
    fn allocate_checked_with(
        &mut self,
        allocation: ManagedAllocation,
        mut admit: impl FnMut(i64) -> Result<(), HeapLimitError>,
    ) -> Result<ManagedReference, HeapLimitError> {
        let allocation_bytes = Self::allocation_size_for(&allocation);
        let retained_delta = self.location_allocate_delta();
        let mark_queue_delta = self.mark_queue_allocate_delta();

        admit(retained_delta + mark_queue_delta)?;

        let (reference_id, address) = if let Some(reference_id) = self.free_ids.pop() {
            let address = self
                .location_for_id(reference_id)
                .expect("reused managed reference id must keep one stable page slot");
            (reference_id, address)
        } else {
            let reference_id = self.next_unused_id;
            self.next_unused_id = self.next_unused_id.saturating_add(1);
            let address = self.allocate_location();
            self.locations.push(address);
            (reference_id, address)
        };

        let handle = ManagedReference::new(reference_id);
        let (target_page, target_offset) = address.position();
        let page = self.page_mut(target_page);
        page.set(target_offset, allocation);

        self.allocated_count += 1;
        self.allocated_bytes = self.allocated_bytes.saturating_add(allocation_bytes);
        self.apply_retained_delta(retained_delta);

        // keep new allocations visible to the current mark phase
        if self.gc_state.phase.is_marking() {
            self.mark_reference(handle);
        }

        Ok(handle)
    }

    /// Initialize exact retained-byte accounting after one bulk construction path.
    pub(super) fn with_retained_bytes(mut self) -> Self {
        self.recompute_retained_bytes();
        self
    }

    /// Allocate one external managed allocation with zero-filled slots.
    fn allocate_external_with_slots(
        &mut self,
        slot_count: usize,
        admit: &mut impl FnMut(i64) -> Result<(), HeapLimitError>,
        retained_delta: i64,
    ) -> Result<ManagedAllocation, HeapLimitError> {
        let values = vec![Value::VOID; slot_count];

        admit(retained_delta)?;

        let span = self.allocate_span(&values);
        self.apply_retained_delta(retained_delta);
        Ok(ManagedAllocation::with_span(span))
    }

    /// Allocate one external managed allocation with the given values.
    fn allocate_external_with_values(
        &mut self,
        values: &[Value],
        admit: &mut impl FnMut(i64) -> Result<(), HeapLimitError>,
        retained_delta: i64,
    ) -> Result<ManagedAllocation, HeapLimitError> {
        admit(retained_delta)?;

        let span = self.allocate_span(values);
        self.apply_retained_delta(retained_delta);
        Ok(ManagedAllocation::with_span(span))
    }

    /// Return the exact retained-byte delta for allocating one stable managed location.
    fn location_allocate_delta(&self) -> i64 {
        // reuse one free id
        if !self.free_ids.is_empty() {
            return -(size_of::<u64>() as i64);
        }

        let mut delta = size_of::<PageSlot>() as i64;
        let slot = self.next_unused_slot as usize;
        let page_index = slot / MANAGED_PAGE_CAPACITY;

        // allocate one new managed page
        if self.pages.len() <= page_index {
            delta += (size_of::<ManagedPage>() + MANAGED_PAGE_PAYLOAD_BYTES) as i64;
        }

        delta
    }

    /// Return the exact retained-byte delta for exposing one new allocation to the active mark queue.
    fn mark_queue_allocate_delta(&self) -> i64 {
        if self.gc_state.phase.is_marking() {
            size_of::<ManagedReference>() as i64
        } else {
            0
        }
    }

    /// Return the exact retained-byte delta for allocating one managed span of the given length.
    pub(crate) fn span_allocate_delta(&self, len: usize) -> i64 {
        if len == 0 {
            return 0;
        }

        match self.span_class(len) {
            ManagedSpanClass::Large => self.large_span_allocate_delta(len),
            ManagedSpanClass::Paged => self.paged_span_allocate_delta(len),
        }
    }

    /// Return the exact retained-byte delta for allocating one page-backed managed span.
    fn paged_span_allocate_delta(&self, len: usize) -> i64 {
        if len == 0 {
            return 0;
        }

        if let Some(index) = self.free_spans.iter().position(|span| span.len() >= len) {
            let mut delta = -(size_of::<ManagedSpan>() as i64);
            let remaining = self.free_spans[index].len().saturating_sub(len);

            if remaining > 0 {
                delta += size_of::<ManagedSpan>() as i64;
            }

            return delta;
        }

        let start_index = self.next_unused_span_index;
        let last_index = start_index.saturating_add(len as u64).saturating_sub(1);
        let required_page = (last_index as usize) / VALUE_PAGE_CAPACITY;
        let added_pages = required_page
            .saturating_add(1)
            .saturating_sub(self.value_pages.len());

        (added_pages * (size_of::<ValuePage>() + VALUE_PAGE_PAYLOAD_BYTES)) as i64
    }

    /// Return the exact retained-byte delta for allocating one large managed span.
    fn large_span_allocate_delta(&self, len: usize) -> i64 {
        let payload_bytes = len.saturating_mul(size_of::<Value>()) as i64;

        if self.free_large_span_ids.is_empty() {
            payload_bytes + size_of::<ManagedLargeSpan>() as i64
        } else {
            payload_bytes - size_of::<u64>() as i64
        }
    }

    /// Get one managed allocation by handle.
    #[inline(always)]
    pub fn get(&self, handle: ManagedReference) -> Option<&ManagedAllocation> {
        let reference_id = handle.id();
        if reference_id == 0 || reference_id >= self.next_unused_id {
            return None;
        }

        let (target_page, target_offset) = self.location(handle)?.position();
        let page = self.pages.get(target_page)?;

        page.get(target_offset)
    }

    /// Get one managed allocation mutably by handle.
    #[inline(always)]
    pub fn get_mut(&mut self, handle: ManagedReference) -> Option<&mut ManagedAllocation> {
        let reference_id = handle.id();
        if reference_id == 0 || reference_id >= self.next_unused_id {
            return None;
        }

        let (target_page, target_offset) = self.location(handle)?.position();
        let page = self.pages.get_mut(target_page)?;

        page.get_mut(target_offset)
    }

    /// Return the durable layout id for one managed allocation.
    pub fn layout_id(&self, handle: ManagedReference) -> Option<LayoutId> {
        let reference_id = handle.id();
        if reference_id == 0 || reference_id >= self.next_unused_id {
            return None;
        }

        let (target_page, target_offset) = self.location(handle)?.position();
        let page = self.pages.get(target_page)?;

        if !page.is_occupied(target_offset) {
            return None;
        }

        page.layout_id(target_offset)
    }

    /// Set the durable layout id for one managed allocation.
    pub fn set_layout_id(&mut self, handle: ManagedReference, layout_id: LayoutId) -> bool {
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

        if !page.is_occupied(target_offset) {
            return false;
        }

        page.set_layout_id(target_offset, layout_id);
        true
    }

    /// Report whether one managed allocation is pinned.
    pub fn is_pinned(&self, handle: ManagedReference) -> bool {
        let reference_id = handle.id();
        if reference_id == 0 || reference_id >= self.next_unused_id {
            return false;
        }

        let Some(address) = self.location(handle) else {
            return false;
        };
        let (target_page, target_offset) = address.position();
        let Some(page) = self.pages.get(target_page) else {
            return false;
        };

        page.is_pinned(target_offset)
    }

    /// Pin one managed allocation for raw-address exposure.
    pub fn pin(&mut self, handle: ManagedReference) -> bool {
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

        page.pin(target_offset)
    }

    /// Release one managed pin.
    pub fn unpin(&mut self, handle: ManagedReference) -> bool {
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

        page.unpin(target_offset)
    }

    /// Get one managed allocation by handle without bounds checks.
    ///
    /// # Safety
    /// Caller must ensure the handle points to a valid allocated allocation.
    #[inline(always)]
    pub unsafe fn get_unchecked(&self, handle: ManagedReference) -> &ManagedAllocation {
        let reference_id = handle.id();
        let address = self
            .location_for_id(reference_id)
            .expect("managed reference must resolve to one stable allocation location");
        let (target_page, target_offset) = address.position();

        debug_assert!(reference_id != 0, "managed reference is null");
        debug_assert!(
            reference_id < self.next_unused_id,
            "managed reference is out of bounds"
        );
        debug_assert!(target_page < self.pages.len(), "heap page out of bounds");
        debug_assert!(
            self.page(target_page).is_occupied(target_offset),
            "managed reference points to freed allocation"
        );

        unsafe {
            self.pages
                .get_unchecked(target_page)
                .get_unchecked(target_offset)
        }
    }

    /// Get one managed allocation mutably by handle without bounds checks.
    ///
    /// # Safety
    /// Caller must ensure the handle points to a valid allocated allocation.
    #[inline(always)]
    pub unsafe fn get_unchecked_mut(&mut self, handle: ManagedReference) -> &mut ManagedAllocation {
        let reference_id = handle.id();
        let address = self
            .location_for_id(reference_id)
            .expect("managed reference must resolve to one stable allocation location");
        let (target_page, target_offset) = address.position();

        debug_assert!(reference_id != 0, "managed reference is null");
        debug_assert!(
            reference_id < self.next_unused_id,
            "managed reference is out of bounds"
        );
        debug_assert!(target_page < self.pages.len(), "heap page out of bounds");
        debug_assert!(
            self.page(target_page).is_occupied(target_offset),
            "managed reference points to freed allocation"
        );

        unsafe {
            self.pages
                .get_unchecked_mut(target_page)
                .get_unchecked_mut(target_offset)
        }
    }

    /// Return one live managed page by index.
    #[inline(always)]
    pub(crate) fn page(&self, index: usize) -> &ManagedPage {
        debug_assert!(index < self.pages.len(), "managed page out of bounds");

        unsafe { self.pages.get_unchecked(index) }
    }

    /// Return one mutable live managed page by index.
    #[inline(always)]
    pub(crate) fn page_mut(&mut self, index: usize) -> &mut ManagedPage {
        debug_assert!(index < self.pages.len(), "managed page out of bounds");

        unsafe { self.pages.get_unchecked_mut(index) }
    }

    /// Return the stable allocation location for one managed reference id.
    pub(super) fn location_for_id(&self, reference_id: u64) -> Option<PageSlot> {
        if reference_id == 0 {
            return None;
        }

        self.locations.get((reference_id - 1) as usize).copied()
    }

    /// Return the stable allocation location for one managed reference.
    pub(super) fn location(&self, handle: ManagedReference) -> Option<PageSlot> {
        Self::location_in(&self.locations, handle)
    }

    /// Return the stable allocation location for one managed reference from one location table.
    fn location_in(locations: &[PageSlot], handle: ManagedReference) -> Option<PageSlot> {
        let reference_id = handle.id();
        if reference_id == 0 {
            return None;
        }

        locations.get((reference_id - 1) as usize).copied()
    }
}
