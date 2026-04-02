use std::borrow::Cow;
use std::error::Error;
use std::fmt;
use std::rc::Rc;

use destack_core::{Capture, CaptureMode, SnapshotCodec};

use crate::managed::{GcState, GcStats, ManagedSpace, ReferenceMap};
use crate::raw::RawSpace;
use crate::shared::SharedSpace;
use crate::value::{ManagedReference, RawPointer, Value};
use crate::{
    HeapBudget, HeapImage, HeapLayoutOptions, HeapLimitError, HeapLimits, HeapSnapshot, HeapUsage,
    LayoutId, ManagedSpaceUsage, MemoryBudget, MemoryLimits, MemoryUsage, RawSpaceUsage,
    SharedBudget, SharedLimitError, SharedLimits, SharedPointer,
};

/// Heap image capture failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeapCaptureError {
    /// The managed GC has in-flight work.
    GcActive,
    /// One managed allocation is still pinned for raw exposure.
    PinnedManagedReferences,
}

impl fmt::Display for HeapCaptureError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::GcActive => write!(f, "heap capture requires idle gc state"),
            Self::PinnedManagedReferences => {
                write!(f, "heap capture requires all managed pins to be released")
            }
        }
    }
}

impl Error for HeapCaptureError {}

/// One live local heap with managed and raw spaces.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Heap {
    /// The configured heap layout options.
    layout: HeapLayoutOptions,
    /// The managed local allocation space.
    managed: ManagedSpace,
    /// The raw local allocation space.
    raw: RawSpace,
    /// Exact hard limits for this heap.
    limits: HeapLimits,
    /// The previous immutable heap image reused as the next capture base.
    base_image: Option<Rc<HeapImage>>,
}

/// One agent-local execution memory view across local and shared memory.
#[derive(Debug)]
pub struct MemoryContext<'a> {
    /// The agent-local heap.
    heap: &'a mut Heap,
    /// The world-shared memory space.
    shared: &'a mut SharedSpace,
    /// The live shared-memory admission budget.
    shared_budget: SharedBudget,
}

impl Heap {
    /// Create one heap with default limits and layout options.
    pub fn new() -> Self {
        Self::with_limits_and_layout(HeapLimits::default(), HeapLayoutOptions::default())
    }

    /// Create one heap with explicit limits and layout options.
    pub fn with_limits_and_layout(limits: HeapLimits, layout: HeapLayoutOptions) -> Self {
        layout.validate();

        let managed = ManagedSpace::with_layout(&layout);
        let raw = RawSpace::with_layout(&layout);

        Self {
            layout,
            managed,
            raw,
            limits,
            base_image: None,
        }
    }

    /// Create one heap from one immutable image.
    pub fn from_image(image: &HeapImage) -> Self {
        crate::validate_managed_reference_bytes(image.managed.managed_reference_bytes);

        let layout = HeapLayoutOptions {
            size_classes: image.managed.size_classes.clone(),
            managed_reference_bytes: image.managed.managed_reference_bytes,
            managed_young_bytes: image.managed.young_bytes,
            managed_small_bytes: image.managed.small_bytes,
            raw_small_bytes: image.raw.small_bytes,
            page_bytes: image.managed.page_bytes,
        };

        debug_assert_eq!(image.managed.page_bytes, image.raw.page_bytes);

        Self {
            managed: ManagedSpace::from_image(&image.managed),
            raw: RawSpace::from_image(&image.raw),
            layout,
            limits: HeapLimits::default(),
            base_image: Some(Rc::new(image.clone())),
        }
    }

    /// Create one heap from one serialized snapshot.
    pub fn from_snapshot(snapshot: &HeapSnapshot) -> Self {
        Self::from_image(&HeapImage::from_snapshot(snapshot))
    }

    /// Return the heap layout options.
    pub fn layout(&self) -> &HeapLayoutOptions {
        &self.layout
    }

    /// Return the managed space.
    pub fn managed(&self) -> &ManagedSpace {
        &self.managed
    }

    /// Return the raw space.
    pub fn raw(&self) -> &RawSpace {
        &self.raw
    }

    /// Replace the heap hard limits.
    pub fn set_limits(&mut self, limits: HeapLimits) -> Result<(), HeapLimitError> {
        limits.check(self.managed.active_bytes(), self.raw.active_bytes())?;
        self.limits = limits;
        Ok(())
    }

    /// Check the configured heap hard limits against current usage.
    pub fn check_limits(&self) -> Result<(), HeapLimitError> {
        self.budget().check_active_reservation(0, 0)
    }

    /// Return the current live heap budget.
    #[inline]
    pub fn budget(&self) -> HeapBudget {
        HeapBudget::new(
            self.limits,
            self.managed.active_bytes(),
            self.raw.active_bytes(),
        )
    }

    /// Clear the current capture base after one local heap mutation.
    #[inline]
    fn invalidate_base_image(&mut self) {
        self.base_image = None;
    }

    /// Allocate one managed byte allocation.
    pub fn allocate_managed_bytes(
        &mut self,
        bytes: &[u8],
        reference_map: ReferenceMap,
        layout_id: Option<LayoutId>,
    ) -> Result<ManagedReference, HeapLimitError> {
        self.invalidate_base_image();
        let budget = self.budget();
        let managed_reservation =
            self.managed
                .allocate_bytes_active_reservation(bytes.len(), &reference_map, layout_id);
        budget.check_active_reservation(managed_reservation, 0)?;
        let handle = self.managed.allocate_bytes(bytes, reference_map, layout_id);

        Ok(handle)
    }

    /// Allocate one managed byte allocation using one borrowed reference map.
    pub fn allocate_managed_bytes_borrowed(
        &mut self,
        bytes: &[u8],
        reference_map: &ReferenceMap,
        layout_id: Option<LayoutId>,
    ) -> Result<ManagedReference, HeapLimitError> {
        self.invalidate_base_image();
        let budget = self.budget();
        let managed_reservation =
            self.managed
                .allocate_bytes_active_reservation(bytes.len(), reference_map, layout_id);
        budget.check_active_reservation(managed_reservation, 0)?;
        let handle = self
            .managed
            .allocate_bytes_borrowed(bytes, reference_map, layout_id);

        Ok(handle)
    }

    /// Allocate one managed byte allocation and stamp one nominal type id.
    pub fn allocate_managed_bytes_typed(
        &mut self,
        bytes: &[u8],
        reference_map: ReferenceMap,
        layout_id: Option<LayoutId>,
        type_id: u32,
    ) -> Result<ManagedReference, HeapLimitError> {
        self.invalidate_base_image();
        let budget = self.budget();
        let managed_reservation =
            self.managed
                .allocate_bytes_active_reservation(bytes.len(), &reference_map, layout_id);
        budget.check_active_reservation(managed_reservation, 0)?;
        let handle = self
            .managed
            .allocate_bytes_typed(bytes, reference_map, layout_id, type_id);

        Ok(handle)
    }

    /// Allocate one borrowed managed byte allocation and stamp one nominal type id.
    pub fn allocate_managed_bytes_borrowed_typed(
        &mut self,
        bytes: &[u8],
        reference_map: &ReferenceMap,
        layout_id: Option<LayoutId>,
        type_id: u32,
    ) -> Result<ManagedReference, HeapLimitError> {
        self.invalidate_base_image();
        let budget = self.budget();
        let managed_reservation =
            self.managed
                .allocate_bytes_active_reservation(bytes.len(), reference_map, layout_id);
        budget.check_active_reservation(managed_reservation, 0)?;
        let handle =
            self.managed
                .allocate_bytes_borrowed_typed(bytes, reference_map, layout_id, type_id);

        Ok(handle)
    }

    /// Allocate one zeroed managed byte allocation.
    pub fn allocate_managed_zeroed(
        &mut self,
        byte_len: usize,
        reference_map: ReferenceMap,
        layout_id: Option<LayoutId>,
    ) -> Result<ManagedReference, HeapLimitError> {
        self.invalidate_base_image();
        let budget = self.budget();
        let managed_reservation =
            self.managed
                .allocate_bytes_active_reservation(byte_len, &reference_map, layout_id);
        budget.check_active_reservation(managed_reservation, 0)?;
        let handle = self
            .managed
            .allocate_zeroed(byte_len, reference_map, layout_id);

        Ok(handle)
    }

    /// Allocate one zeroed managed byte allocation using one borrowed reference map.
    pub fn allocate_managed_zeroed_borrowed(
        &mut self,
        byte_len: usize,
        reference_map: &ReferenceMap,
        layout_id: Option<LayoutId>,
    ) -> Result<ManagedReference, HeapLimitError> {
        self.invalidate_base_image();
        let budget = self.budget();
        let managed_reservation =
            self.managed
                .allocate_bytes_active_reservation(byte_len, reference_map, layout_id);
        budget.check_active_reservation(managed_reservation, 0)?;
        let handle = self
            .managed
            .allocate_zeroed_borrowed(byte_len, reference_map, layout_id);

        Ok(handle)
    }

    /// Allocate one zeroed borrowed managed byte allocation and stamp one nominal type id.
    pub fn allocate_managed_zeroed_borrowed_typed(
        &mut self,
        byte_len: usize,
        reference_map: &ReferenceMap,
        layout_id: Option<LayoutId>,
        type_id: u32,
    ) -> Result<ManagedReference, HeapLimitError> {
        self.invalidate_base_image();
        let budget = self.budget();
        let managed_reservation =
            self.managed
                .allocate_bytes_active_reservation(byte_len, reference_map, layout_id);
        budget.check_active_reservation(managed_reservation, 0)?;
        let handle = self.managed.allocate_zeroed_borrowed_typed(
            byte_len,
            reference_map,
            layout_id,
            type_id,
        );

        Ok(handle)
    }

    /// Allocate one zeroed managed byte allocation using borrowed repeated offsets.
    pub fn allocate_managed_zeroed_repeated_reference_offsets(
        &mut self,
        byte_len: usize,
        count: u32,
        element_size: u32,
        offsets: &[u32],
        layout_id: Option<LayoutId>,
    ) -> Result<ManagedReference, HeapLimitError> {
        self.invalidate_base_image();
        let budget = self.budget();
        let managed_reservation = self.managed.allocate_repeated_offsets_active_reservation(
            byte_len,
            count,
            element_size,
            offsets,
            layout_id,
        );
        budget.check_active_reservation(managed_reservation, 0)?;
        let handle = self.managed.allocate_zeroed_repeated_reference_offsets(
            byte_len,
            count,
            element_size,
            offsets,
            layout_id,
        );

        Ok(handle)
    }

    /// Allocate one managed byte allocation with repeated reference offsets.
    pub fn allocate_managed_bytes_repeated_reference_offsets(
        &mut self,
        bytes: &[u8],
        count: u32,
        element_size: u32,
        offsets: &[u32],
        layout_id: Option<LayoutId>,
    ) -> Result<ManagedReference, HeapLimitError> {
        self.invalidate_base_image();
        let budget = self.budget();
        let managed_reservation = self.managed.allocate_repeated_offsets_active_reservation(
            bytes.len(),
            count,
            element_size,
            offsets,
            layout_id,
        );
        budget.check_active_reservation(managed_reservation, 0)?;
        let handle = self.managed.allocate_bytes_repeated_reference_offsets(
            bytes,
            count,
            element_size,
            offsets,
            layout_id,
        );

        Ok(handle)
    }

    /// Return the managed bytes for this handle.
    pub fn managed_bytes(&self, handle: ManagedReference) -> Option<Cow<'_, [u8]>> {
        self.managed.bytes(handle)
    }

    /// Return one owned copy of the managed bytes for this handle.
    pub fn managed_bytes_to_vec(&self, handle: ManagedReference) -> Option<Vec<u8>> {
        self.managed.bytes_to_vec(handle)
    }

    /// Return the managed byte length for this handle.
    pub fn managed_byte_len(&self, handle: ManagedReference) -> Option<usize> {
        self.managed.byte_len(handle)
    }

    /// Return the managed reference map for this handle.
    pub fn reference_map(&self, handle: ManagedReference) -> Option<&ReferenceMap> {
        self.managed.reference_map(handle)
    }

    /// Return the nominal type id for this managed allocation.
    pub fn managed_type_id(&self, handle: ManagedReference) -> Option<u32> {
        self.managed.type_id(handle)
    }

    /// Return the layout id for this managed allocation.
    pub fn managed_layout_id(&self, handle: ManagedReference) -> Option<LayoutId> {
        self.managed.layout_id(handle)
    }

    /// Set one managed byte.
    pub fn set_managed_byte(&mut self, handle: ManagedReference, index: usize, byte: u8) -> bool {
        self.invalidate_base_image();
        let budget = self.budget();
        let managed_reservation = self.managed.write_active_reservation(handle, index, 1);
        if budget
            .check_active_reservation(managed_reservation, 0)
            .is_err()
        {
            return false;
        }

        self.managed.set_byte(handle, index, byte)
    }

    /// Set one managed byte slice.
    pub fn set_managed_bytes(
        &mut self,
        handle: ManagedReference,
        start: usize,
        bytes: &[u8],
    ) -> bool {
        self.invalidate_base_image();
        let budget = self.budget();
        let managed_reservation = self
            .managed
            .write_active_reservation(handle, start, bytes.len());
        if budget
            .check_active_reservation(managed_reservation, 0)
            .is_err()
        {
            return false;
        }

        self.managed.set_bytes(handle, start, bytes)
    }

    /// Set the nominal type id for one managed allocation.
    pub fn set_managed_type_id(&mut self, handle: ManagedReference, type_id: u32) -> bool {
        self.invalidate_base_image();
        self.managed.set_type_id(handle, type_id)
    }

    /// Report whether one managed reference is currently allocated.
    pub fn is_managed_allocated(&self, handle: ManagedReference) -> bool {
        self.managed.is_allocated(handle)
    }

    /// Return the current managed GC state.
    pub fn managed_gc_state(&self) -> &GcState {
        self.managed.gc_state()
    }

    /// Return the exact live allocated managed bytes.
    pub fn managed_allocated_bytes(&self) -> u64 {
        self.managed.allocated_bytes()
    }

    /// Return the exact active managed bytes.
    pub fn managed_active_bytes(&self) -> u64 {
        self.managed.active_bytes()
    }

    /// Return the number of live managed allocations.
    pub fn managed_allocation_count(&self) -> usize {
        self.managed.allocation_count()
    }

    /// Run one full managed GC cycle.
    pub fn collect_managed_handles<I>(&mut self, handles: I) -> GcStats
    where
        I: IntoIterator<Item = ManagedReference>,
    {
        self.invalidate_base_image();
        self.managed.collect_handles(handles)
    }

    /// Run one staged managed GC cycle with a young prepass and one conditional full sweep.
    pub fn collect_managed_handles_staged<I>(&mut self, handles: I) -> GcStats
    where
        I: IntoIterator<Item = ManagedReference>,
    {
        self.invalidate_base_image();

        // root set
        let handles = handles.into_iter().collect::<Vec<_>>();
        let min_small_bytes = self.managed.small.size_classes.min_small_allocation_bytes();

        // young prepass
        let young_stats = self.managed.collect_young_handles(handles.iter().copied());

        // escalate only when minor collection still leaves hard pressure
        let should_collect_full = !self.managed.young.can_fit(min_small_bytes)
            || self.budget().check_active_reservation(0, 0).is_err();

        if !should_collect_full {
            return young_stats;
        }

        // full sweep
        self.managed.collect_handles(handles)
    }

    /// Run one young-generation managed GC cycle.
    pub fn collect_young_managed_handles<I>(&mut self, handles: I) -> GcStats
    where
        I: IntoIterator<Item = ManagedReference>,
    {
        self.invalidate_base_image();
        self.managed.collect_young_handles(handles)
    }

    /// Allocate one raw packed-value allocation.
    pub fn allocate_raw_values(
        &mut self,
        values: Vec<Value>,
    ) -> Result<RawPointer, HeapLimitError> {
        self.invalidate_base_image();
        let budget = self.budget();
        let raw_reservation = self
            .raw
            .allocate_bytes_active_reservation(values.len() * Value::BYTE_LEN);
        budget.check_active_reservation(0, raw_reservation)?;
        let pointer = self.raw.allocate_packed_values(values);

        Ok(pointer)
    }

    /// Allocate one empty raw allocation.
    pub fn allocate_raw(&mut self) -> Result<RawPointer, HeapLimitError> {
        self.allocate_raw_bytes(&[])
    }

    /// Allocate one zeroed raw packed-value buffer.
    pub fn allocate_raw_slots(&mut self, slot_count: usize) -> Result<RawPointer, HeapLimitError> {
        self.invalidate_base_image();
        let budget = self.budget();
        let raw_reservation = self
            .raw
            .allocate_zeroed_active_reservation(slot_count * Value::BYTE_LEN);
        budget.check_active_reservation(0, raw_reservation)?;
        let pointer = self.raw.allocate_packed_value_slots(slot_count);

        Ok(pointer)
    }

    /// Allocate one raw byte allocation.
    pub fn allocate_raw_bytes(&mut self, bytes: &[u8]) -> Result<RawPointer, HeapLimitError> {
        self.invalidate_base_image();
        let budget = self.budget();
        let raw_reservation = self.raw.allocate_bytes_active_reservation(bytes.len());
        budget.check_active_reservation(0, raw_reservation)?;
        let pointer = self.raw.allocate_bytes(bytes);

        Ok(pointer)
    }

    /// Allocate one zeroed raw byte allocation.
    pub fn allocate_raw_zeroed(&mut self, byte_len: usize) -> Result<RawPointer, HeapLimitError> {
        self.invalidate_base_image();
        let budget = self.budget();
        let raw_reservation = self.raw.allocate_zeroed_active_reservation(byte_len);
        budget.check_active_reservation(0, raw_reservation)?;
        let pointer = self.raw.allocate_zeroed(byte_len);

        Ok(pointer)
    }

    /// Return the raw values for one raw allocation.
    pub fn raw_values(&self, pointer: RawPointer) -> Option<Vec<Value>> {
        self.raw.values_to_vec(pointer)
    }

    /// Return the raw byte length for one pointer.
    pub fn raw_byte_len(&self, pointer: RawPointer) -> Option<usize> {
        self.raw.byte_len(pointer)
    }

    /// Return the raw bytes for one pointer.
    pub fn raw_bytes(&self, pointer: RawPointer) -> Option<Cow<'_, [u8]>> {
        self.raw.bytes(pointer)
    }

    /// Return one raw byte by offset.
    pub fn raw_byte_at(&self, pointer: RawPointer, index: usize) -> Option<u8> {
        self.raw.byte_at(pointer, index)
    }

    /// Return one owned copy of the raw bytes.
    pub fn raw_bytes_to_vec(&self, pointer: RawPointer) -> Option<Vec<u8>> {
        self.raw.bytes_to_vec(pointer)
    }

    /// Free one raw allocation.
    pub fn free_raw(&mut self, pointer: RawPointer) -> bool {
        self.invalidate_base_image();
        self.raw.free(pointer)
    }

    /// Free one managed allocation.
    pub fn free_managed(&mut self, handle: ManagedReference) -> bool {
        self.invalidate_base_image();
        self.managed.free(handle)
    }

    /// Set one raw byte.
    pub fn set_raw_byte(&mut self, pointer: RawPointer, index: usize, byte: u8) -> bool {
        self.invalidate_base_image();
        let budget = self.budget();
        let raw_reservation = self.raw.write_active_reservation(pointer, index, 1);
        if budget.check_active_reservation(0, raw_reservation).is_err() {
            return false;
        }

        self.raw.set_byte(pointer, index, byte)
    }

    /// Set one raw byte slice.
    pub fn set_raw_bytes(&mut self, pointer: RawPointer, start: usize, bytes: &[u8]) -> bool {
        self.invalidate_base_image();
        let budget = self.budget();
        let raw_reservation = self
            .raw
            .write_active_reservation(pointer, start, bytes.len());
        if budget.check_active_reservation(0, raw_reservation).is_err() {
            return false;
        }

        self.raw.set_bytes(pointer, start, bytes)
    }

    /// Resize one raw packed-value payload.
    pub fn resize_raw_values(
        &mut self,
        pointer: RawPointer,
        len: usize,
    ) -> Result<bool, HeapLimitError> {
        self.invalidate_base_image();
        let budget = self.budget();
        let raw_reservation = self
            .raw
            .replace_bytes_active_reservation(pointer, len * Value::BYTE_LEN);
        budget.check_active_reservation(0, raw_reservation)?;
        let replaced = self.raw.resize_values(pointer, len);

        if !replaced {
            return Ok(false);
        }

        Ok(true)
    }

    /// Set one raw packed value.
    pub fn set_raw_value(&mut self, pointer: RawPointer, index: usize, value: Value) -> bool {
        self.invalidate_base_image();
        let budget = self.budget();
        let raw_reservation =
            self.raw
                .write_active_reservation(pointer, index * Value::BYTE_LEN, Value::BYTE_LEN);
        if budget.check_active_reservation(0, raw_reservation).is_err() {
            return false;
        }

        self.raw.set_value(pointer, index, value)
    }

    /// Set one raw packed value without bounds checks.
    ///
    /// # Safety
    ///
    /// The caller must ensure that `pointer` refers to a live packed-value raw allocation
    /// and that `index` is within that allocation's packed-value bounds.
    pub unsafe fn set_raw_value_unchecked(
        &mut self,
        pointer: RawPointer,
        index: usize,
        value: Value,
    ) {
        let _ = self.set_raw_value(pointer, index, value);
    }

    /// Replace one raw byte allocation payload.
    pub fn replace_raw_bytes(
        &mut self,
        pointer: RawPointer,
        bytes: &[u8],
    ) -> Result<bool, HeapLimitError> {
        self.invalidate_base_image();
        let budget = self.budget();
        let raw_reservation = self
            .raw
            .replace_bytes_active_reservation(pointer, bytes.len());
        budget.check_active_reservation(0, raw_reservation)?;
        let replaced = self.raw.replace_bytes(pointer, bytes);

        if !replaced {
            return Ok(false);
        }

        Ok(true)
    }

    /// Replace one raw packed-value allocation payload.
    pub fn replace_raw_values(
        &mut self,
        pointer: RawPointer,
        values: &[Value],
    ) -> Result<bool, HeapLimitError> {
        self.replace_raw_bytes(pointer, &encode_values(values))
    }

    /// Return the number of live raw allocations.
    pub fn raw_allocation_count(&self) -> usize {
        self.raw.allocation_count()
    }

    /// Return the exact usage for this live heap.
    pub fn usage(&self) -> HeapUsage {
        HeapUsage {
            managed: ManagedSpaceUsage {
                allocation_count: self.managed.allocation_count(),
                allocated_bytes: self.managed.allocated_bytes(),
                active_bytes: self.managed.active_bytes(),
                mapped_bytes: self.managed.mapped_bytes(),
                borrowed_bytes: self.managed.borrowed_bytes(),
            },
            raw: RawSpaceUsage {
                allocation_count: self.raw.allocation_count(),
                allocated_bytes: self.raw.usage().allocated_bytes,
                active_bytes: self.raw.active_bytes(),
                mapped_bytes: self.raw.mapped_bytes(),
                borrowed_bytes: self.raw.borrowed_bytes(),
            },
        }
    }

    /// Capture one immutable heap image.
    pub fn image(&mut self) -> Result<HeapImage, HeapCaptureError> {
        let base_managed = self.base_image.as_deref().map(|image| &image.managed);
        let base_raw = self.base_image.as_deref().map(|image| &image.raw);
        let managed = self.managed.image(base_managed)?;
        let raw = self.raw.image(base_raw);
        let image = HeapImage { managed, raw };
        self.base_image = Some(Rc::new(image.clone()));
        Ok(image)
    }

    /// Restore this heap from one immutable image.
    pub fn restore_image(&mut self, image: &HeapImage) {
        *self = Self::from_image(image);
    }

    /// Capture one serialized heap snapshot.
    pub fn snapshot(&mut self) -> Result<HeapSnapshot, HeapCaptureError> {
        let image = self.image()?;
        Self::encode_snapshot(&image)
    }

    /// Restore this heap from one serialized snapshot.
    pub fn restore_snapshot(&mut self, snapshot: &HeapSnapshot) {
        *self = Self::from_snapshot(snapshot);
    }

    /// Return the currently allocated managed references.
    pub fn allocated_references(&self) -> Vec<ManagedReference> {
        self.managed.allocated_references()
    }
}

impl Default for Heap {
    fn default() -> Self {
        Self::new()
    }
}

impl MemoryContext<'_> {
    /// Create one agent memory view.
    pub fn new<'a>(heap: &'a mut Heap, shared: &'a mut SharedSpace) -> MemoryContext<'a> {
        Self::with_shared_limits(heap, shared, SharedLimits::default())
    }

    /// Create one agent memory view with explicit shared-memory limits.
    pub fn with_shared_limits<'a>(
        heap: &'a mut Heap,
        shared: &'a mut SharedSpace,
        shared_limits: SharedLimits,
    ) -> MemoryContext<'a> {
        let shared_budget = SharedBudget::new(shared_limits, shared.active_bytes());

        MemoryContext {
            heap,
            shared,
            shared_budget,
        }
    }

    /// Reborrow this agent memory view for one nested operation.
    pub fn reborrow(&mut self) -> MemoryContext<'_> {
        MemoryContext {
            heap: self.heap,
            shared: self.shared,
            shared_budget: self.shared_budget,
        }
    }

    /// Return the mutable local heap.
    pub fn heap(&mut self) -> &mut Heap {
        self.heap
    }

    /// Return the shared local heap reference.
    pub fn heap_ref(&self) -> &Heap {
        self.heap
    }

    /// Return the shared-memory space by shared reference.
    pub fn shared_ref(&self) -> &SharedSpace {
        self.shared
    }

    /// Allocate one shared-memory byte region.
    pub fn allocate_shared_bytes(
        &mut self,
        bytes: &[u8],
    ) -> Result<SharedPointer, SharedLimitError> {
        self.shared.allocate_bytes(bytes, &mut self.shared_budget)
    }

    /// Replace one shared-memory byte region.
    pub fn replace_shared_bytes(
        &mut self,
        pointer: SharedPointer,
        bytes: &[u8],
    ) -> Result<bool, SharedLimitError> {
        self.shared
            .replace_bytes(pointer, bytes, &mut self.shared_budget)
    }

    /// Return combined local and shared memory usage.
    pub fn usage(&self) -> MemoryUsage {
        MemoryUsage {
            heap: self.heap.usage(),
            shared: self.shared.usage(),
        }
    }

    /// Return the current combined memory budget.
    pub fn budget(&self) -> MemoryBudget {
        MemoryBudget::from_usage(
            MemoryLimits {
                heap: self.heap.limits,
                shared: self.shared_budget.limits(),
            },
            self.usage(),
        )
    }
}

impl Capture for Heap {
    type Image = HeapImage;
    type Error = HeapCaptureError;
    type CaptureContext<'a> = ();
    type RestoreContext<'a> = ();

    fn capture_image(
        &mut self,
        _mode: CaptureMode,
        _context: Self::CaptureContext<'_>,
    ) -> Result<Self::Image, Self::Error> {
        self.image()
    }

    fn restore_image(
        &mut self,
        image: &Self::Image,
        _context: Self::RestoreContext<'_>,
    ) -> Result<(), Self::Error> {
        self.restore_image(image);
        Ok(())
    }
}

impl SnapshotCodec for Heap {
    type Snapshot = HeapSnapshot;

    fn encode_snapshot(image: &Self::Image) -> Result<Self::Snapshot, Self::Error> {
        Ok(image.snapshot())
    }

    fn decode_snapshot(snapshot: &Self::Snapshot) -> Result<Self::Image, Self::Error> {
        Ok(HeapImage::from_snapshot(snapshot))
    }
}

// encode one packed value vector into bytes
fn encode_values(values: &[Value]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(values.len() * Value::BYTE_LEN);

    for value in values {
        bytes.extend_from_slice(&value.to_byte_array());
    }

    bytes
}
