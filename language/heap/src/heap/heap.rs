use destack_core::{Capture, CaptureMode, SnapshotCodec};
use std::error::Error;
use std::fmt;
use std::sync::Arc;

use crate::value::ManagedReference;
use crate::{
    GcStats, HeapImage, HeapLimitError, HeapLimits, HeapSnapshot, HeapUsage, ManagedAllocation,
    ManagedHeap, RawAllocation, RawHeap, RawPointer, Value,
};

/// Heap image capture failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeapCaptureError {
    /// The managed heap gc has in flight work.
    GcActive,
    /// One managed allocation still has active raw-address exposure.
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

/// Heap for managed and raw allocations.
#[derive(Debug)]
pub struct Heap {
    /// Managed heap used for GC tracked allocations.
    managed: ManagedHeap,
    /// Raw heap used for manual allocations.
    raw: RawHeap,
    /// Exact hard limits for this heap.
    limits: HeapLimits,
    /// The last captured image used to reuse unchanged image chunks.
    cached_image: Option<Arc<HeapImage>>,
}

impl Heap {
    /// Create a new heap with default managed and raw spaces.
    pub fn new() -> Self {
        Self {
            managed: ManagedHeap::new(),
            raw: RawHeap::new(),
            limits: HeapLimits::default(),
            cached_image: None,
        }
    }

    /// Create a heap with explicit limits and large-span thresholds.
    pub fn with_limits_and_large_span_thresholds(
        limits: HeapLimits,
        managed_large_span_values: usize,
        raw_large_span_bytes: usize,
    ) -> Self {
        let managed = ManagedHeap::with_large_span_values(managed_large_span_values);
        let raw = RawHeap::with_large_span_bytes(raw_large_span_bytes);

        Self {
            managed,
            raw,
            limits,
            cached_image: None,
        }
    }

    /// Create a heap from an immutable image.
    pub fn from_image(image: &HeapImage) -> Self {
        let managed = ManagedHeap::from_image(&image.managed);
        let raw = RawHeap::from_image(&image.raw);
        Self {
            managed,
            raw,
            limits: HeapLimits::default(),
            cached_image: Some(Arc::new(image.clone())),
        }
    }

    /// Return the dedicated managed large-span threshold in values.
    pub fn managed_large_span_value_threshold(&self) -> usize {
        self.managed.large_span_values()
    }

    /// Return the dedicated raw large-span threshold in bytes.
    pub fn raw_large_span_byte_threshold(&self) -> usize {
        self.raw.large_span_bytes()
    }

    /// Replace the heap hard limits.
    pub fn set_limits(&mut self, limits: HeapLimits) -> Result<(), HeapLimitError> {
        limits.check(self.managed.retained_bytes(), self.raw.retained_bytes())?;
        self.limits = limits;

        Ok(())
    }

    /// Check the configured heap hard limits against exact current usage.
    pub fn check_limits(&self) -> Result<(), HeapLimitError> {
        self.limits
            .check(self.managed.retained_bytes(), self.raw.retained_bytes())
    }

    /// Allocate one managed allocation with the given slot values.
    pub fn allocate_managed_values(
        &mut self,
        values: Vec<Value>,
    ) -> Result<ManagedReference, HeapLimitError> {
        let limits = self.limits;
        let managed_bytes = self.managed.retained_bytes();
        let raw_bytes = self.raw.retained_bytes();
        let mut admitted_delta = 0i64;
        self.cached_image = None;
        self.managed
            .allocate_with_values_checked(values, |managed_delta| {
                Self::admit_managed_delta(
                    limits,
                    managed_bytes,
                    raw_bytes,
                    &mut admitted_delta,
                    managed_delta,
                )
            })
    }

    /// Allocate one empty managed allocation.
    pub fn allocate_managed(&mut self) -> Result<ManagedReference, HeapLimitError> {
        let limits = self.limits;
        let managed_bytes = self.managed.retained_bytes();
        let raw_bytes = self.raw.retained_bytes();
        let mut admitted_delta = 0i64;
        self.cached_image = None;
        self.managed.allocate_checked(|managed_delta| {
            Self::admit_managed_delta(
                limits,
                managed_bytes,
                raw_bytes,
                &mut admitted_delta,
                managed_delta,
            )
        })
    }

    /// Allocate one managed allocation with the given slot count.
    pub fn allocate_managed_slots(
        &mut self,
        slot_count: usize,
    ) -> Result<ManagedReference, HeapLimitError> {
        let limits = self.limits;
        let managed_bytes = self.managed.retained_bytes();
        let raw_bytes = self.raw.retained_bytes();
        let mut admitted_delta = 0i64;
        self.cached_image = None;
        self.managed
            .allocate_with_slots_checked(slot_count, |managed_delta| {
                Self::admit_managed_delta(
                    limits,
                    managed_bytes,
                    raw_bytes,
                    &mut admitted_delta,
                    managed_delta,
                )
            })
    }

    /// Allocate one 2-slot managed allocation.
    pub fn allocate_managed_pair(
        &mut self,
        first: Value,
        second: Value,
    ) -> Result<ManagedReference, HeapLimitError> {
        let limits = self.limits;
        let managed_bytes = self.managed.retained_bytes();
        let raw_bytes = self.raw.retained_bytes();
        let mut admitted_delta = 0i64;
        self.cached_image = None;
        self.managed
            .allocate_pair_checked(first, second, |managed_delta| {
                Self::admit_managed_delta(
                    limits,
                    managed_bytes,
                    raw_bytes,
                    &mut admitted_delta,
                    managed_delta,
                )
            })
    }

    /// Allocate one 1-slot managed allocation.
    pub fn allocate_managed_single(
        &mut self,
        value: Value,
    ) -> Result<ManagedReference, HeapLimitError> {
        let limits = self.limits;
        let managed_bytes = self.managed.retained_bytes();
        let raw_bytes = self.raw.retained_bytes();
        let mut admitted_delta = 0i64;
        self.cached_image = None;
        self.managed
            .allocate_single_checked(value, |managed_delta| {
                Self::admit_managed_delta(
                    limits,
                    managed_bytes,
                    raw_bytes,
                    &mut admitted_delta,
                    managed_delta,
                )
            })
    }

    /// Return one managed allocation by reference.
    pub fn managed_allocation(&self, handle: ManagedReference) -> Option<&ManagedAllocation> {
        self.managed.get(handle)
    }

    /// Return one managed allocation without bounds checks.
    ///
    /// # Safety
    /// Caller must ensure the managed reference is valid and allocated.
    pub unsafe fn managed_allocation_unchecked(
        &self,
        handle: ManagedReference,
    ) -> &ManagedAllocation {
        unsafe { self.managed.get_unchecked(handle) }
    }

    /// Return one managed slot value by reference and slot index.
    pub fn managed_slot(&self, handle: ManagedReference, index: usize) -> Option<&Value> {
        self.managed.get_slot(handle, index)
    }

    /// Return one owned copy of the managed slot values.
    pub fn managed_slots_to_vec(&self, handle: ManagedReference) -> Option<Vec<Value>> {
        self.managed.copy_slots(handle)
    }

    /// Return the inline managed slot values when one allocation is inline-backed.
    pub fn managed_inline_slots(&self, handle: ManagedReference) -> Option<&[Value]> {
        self.managed.inline_slots(handle)
    }

    /// Set one managed slot value.
    pub fn set_managed_slot(
        &mut self,
        handle: ManagedReference,
        index: usize,
        value: Value,
    ) -> bool {
        self.cached_image = None;
        self.managed.set_slot(handle, index, value)
    }

    /// Return one managed slot value without bounds checks.
    ///
    /// # Safety
    /// Caller must ensure the managed reference and slot index are valid.
    pub unsafe fn managed_slot_unchecked(&self, handle: ManagedReference, index: usize) -> &Value {
        unsafe { self.managed.get_slot_unchecked(handle, index) }
    }

    /// Set one managed slot without bounds checks.
    ///
    /// # Safety
    /// Caller must ensure the managed reference and slot index are valid.
    pub unsafe fn set_managed_slot_unchecked(
        &mut self,
        handle: ManagedReference,
        slot_index: usize,
        value: Value,
    ) {
        self.cached_image = None;
        unsafe { *self.managed.get_slot_unchecked_mut(handle, slot_index) = value };
    }

    /// Resize the slot storage for one managed allocation.
    pub fn resize_managed_slots(
        &mut self,
        handle: ManagedReference,
        len: usize,
    ) -> Result<bool, HeapLimitError> {
        let limits = self.limits;
        let managed_bytes = self.managed.retained_bytes();
        let raw_bytes = self.raw.retained_bytes();
        let mut admitted_delta = 0i64;
        self.cached_image = None;
        self.managed
            .resize_slots_checked(handle, len, |managed_delta| {
                Self::admit_managed_delta(
                    limits,
                    managed_bytes,
                    raw_bytes,
                    &mut admitted_delta,
                    managed_delta,
                )
            })
    }

    /// Report whether one managed reference is currently allocated.
    pub fn is_managed_allocated(&self, handle: ManagedReference) -> bool {
        self.managed.is_allocated(handle)
    }

    /// Return the current managed GC state.
    pub fn managed_gc_state(&self) -> &crate::GcState {
        self.managed.gc_state()
    }

    /// Return the exact retained managed heap bytes.
    pub fn managed_heap_bytes(&self) -> u64 {
        self.managed.heap_bytes()
    }

    /// Return the number of allocated managed allocations.
    pub fn managed_allocation_count(&self) -> usize {
        self.managed.allocation_count()
    }

    /// Run managed garbage collection from one set of roots.
    pub fn collect_managed_handles<I>(&mut self, handles: I) -> GcStats
    where
        I: IntoIterator<Item = ManagedReference>,
    {
        self.cached_image = None;
        self.managed.collect_handles(handles)
    }

    /// Allocate one raw allocation with the given slot values.
    pub fn allocate_raw_values(
        &mut self,
        values: Vec<Value>,
    ) -> Result<RawPointer, HeapLimitError> {
        let limits = self.limits;
        let managed_bytes = self.managed.retained_bytes();
        let raw_bytes = self.raw.retained_bytes();
        let mut admitted_delta = 0i64;
        self.cached_image = None;
        self.raw.allocate_with_values_checked(values, |raw_delta| {
            Self::admit_raw_delta(
                limits,
                managed_bytes,
                raw_bytes,
                &mut admitted_delta,
                raw_delta,
            )
        })
    }

    /// Allocate one empty raw allocation.
    pub fn allocate_raw(&mut self) -> Result<RawPointer, HeapLimitError> {
        let limits = self.limits;
        let managed_bytes = self.managed.retained_bytes();
        let raw_bytes = self.raw.retained_bytes();
        let mut admitted_delta = 0i64;
        self.cached_image = None;
        self.raw.allocate_checked(|raw_delta| {
            Self::admit_raw_delta(
                limits,
                managed_bytes,
                raw_bytes,
                &mut admitted_delta,
                raw_delta,
            )
        })
    }

    /// Allocate one raw allocation with the given slot count.
    pub fn allocate_raw_slots(&mut self, slot_count: usize) -> Result<RawPointer, HeapLimitError> {
        let limits = self.limits;
        let managed_bytes = self.managed.retained_bytes();
        let raw_bytes = self.raw.retained_bytes();
        let mut admitted_delta = 0i64;
        self.cached_image = None;
        self.raw
            .allocate_with_slots_checked(slot_count, |raw_delta| {
                Self::admit_raw_delta(
                    limits,
                    managed_bytes,
                    raw_bytes,
                    &mut admitted_delta,
                    raw_delta,
                )
            })
    }

    /// Allocate one raw allocation with the given byte payload.
    pub fn allocate_raw_bytes(&mut self, bytes: &[u8]) -> Result<RawPointer, HeapLimitError> {
        let limits = self.limits;
        let managed_bytes = self.managed.retained_bytes();
        let raw_bytes = self.raw.retained_bytes();
        let mut admitted_delta = 0i64;
        self.cached_image = None;
        self.raw.allocate_with_bytes_checked(bytes, |raw_delta| {
            Self::admit_raw_delta(
                limits,
                managed_bytes,
                raw_bytes,
                &mut admitted_delta,
                raw_delta,
            )
        })
    }

    /// Return one raw allocation by pointer.
    pub fn raw_allocation(&self, pointer: RawPointer) -> Option<&RawAllocation> {
        self.raw.get(pointer)
    }

    /// Report whether one raw allocation stores bytes.
    pub fn raw_is_bytes(&self, pointer: RawPointer) -> Option<bool> {
        Some(self.raw.get(pointer)?.is_bytes())
    }

    /// Return the raw value slots when one allocation stores values.
    pub fn raw_values(&self, pointer: RawPointer) -> Option<&[Value]> {
        Some(self.raw.get(pointer)?.values()?.as_slice())
    }

    /// Return the raw byte payload length for one pointer.
    pub fn raw_byte_len(&self, pointer: RawPointer) -> Option<usize> {
        self.raw.byte_len(pointer)
    }

    /// Return the raw byte payload for one pointer.
    pub fn raw_bytes(&self, pointer: RawPointer) -> Option<&[u8]> {
        self.raw.bytes(pointer)
    }

    /// Return one raw byte by offset.
    pub fn raw_byte_at(&self, pointer: RawPointer, index: usize) -> Option<u8> {
        self.raw.byte_at(pointer, index)
    }

    /// Return one owned copy of the raw byte payload.
    pub fn raw_bytes_to_vec(&self, pointer: RawPointer) -> Option<Vec<u8>> {
        self.raw.bytes_to_vec(pointer)
    }

    /// Free one raw allocation.
    pub fn free_raw(&mut self, pointer: RawPointer) -> bool {
        self.cached_image = None;
        self.raw.free(pointer)
    }

    /// Write one byte into one raw byte allocation.
    pub fn set_raw_byte(&mut self, pointer: RawPointer, index: usize, byte: u8) -> bool {
        self.cached_image = None;
        self.raw.set_byte(pointer, index, byte)
    }

    /// Resize one raw value allocation.
    pub fn resize_raw_values(
        &mut self,
        pointer: RawPointer,
        len: usize,
    ) -> Result<bool, HeapLimitError> {
        let limits = self.limits;
        let managed_bytes = self.managed.retained_bytes();
        let raw_bytes = self.raw.retained_bytes();
        let mut admitted_delta = 0i64;
        self.cached_image = None;
        self.raw.resize_values_checked(pointer, len, |raw_delta| {
            Self::admit_raw_delta(
                limits,
                managed_bytes,
                raw_bytes,
                &mut admitted_delta,
                raw_delta,
            )
        })
    }

    /// Set one raw value slot.
    pub fn set_raw_value(&mut self, pointer: RawPointer, index: usize, value: Value) -> bool {
        self.cached_image = None;
        self.raw.set_value(pointer, index, value)
    }

    /// Set one raw value slot without bounds checks.
    ///
    /// # Safety
    /// Caller must ensure the raw pointer resolves to one value allocation and
    /// the slot index is within bounds.
    pub unsafe fn set_raw_value_unchecked(
        &mut self,
        pointer: RawPointer,
        index: usize,
        value: Value,
    ) {
        self.cached_image = None;
        unsafe { self.raw.set_value_unchecked(pointer, index, value) };
    }

    /// Replace one raw byte allocation payload.
    pub fn replace_raw_bytes(
        &mut self,
        pointer: RawPointer,
        bytes: &[u8],
    ) -> Result<bool, HeapLimitError> {
        let limits = self.limits;
        let managed_bytes = self.managed.retained_bytes();
        let raw_bytes = self.raw.retained_bytes();
        let mut admitted_delta = 0i64;
        self.cached_image = None;
        self.raw.replace_bytes_checked(pointer, bytes, |raw_delta| {
            Self::admit_raw_delta(
                limits,
                managed_bytes,
                raw_bytes,
                &mut admitted_delta,
                raw_delta,
            )
        })
    }

    /// Replace one raw value allocation payload.
    pub fn replace_raw_values(
        &mut self,
        pointer: RawPointer,
        values: &[Value],
    ) -> Result<bool, HeapLimitError> {
        let limits = self.limits;
        let managed_bytes = self.managed.retained_bytes();
        let raw_bytes = self.raw.retained_bytes();
        let mut admitted_delta = 0i64;
        self.cached_image = None;
        self.raw
            .replace_values_checked(pointer, values, |raw_delta| {
                Self::admit_raw_delta(
                    limits,
                    managed_bytes,
                    raw_bytes,
                    &mut admitted_delta,
                    raw_delta,
                )
            })
    }

    /// Return the number of allocated raw allocations.
    pub fn raw_allocation_count(&self) -> usize {
        self.raw.allocation_count()
    }

    /// Return the exact retained bytes for this live heap.
    pub fn usage(&self) -> HeapUsage {
        HeapUsage {
            managed: self.managed.usage(),
            raw: self.raw.usage(),
        }
    }

    /// Capture one immutable heap image.
    pub fn image(&mut self) -> Result<HeapImage, HeapCaptureError> {
        let base = self.cached_image.as_deref();
        let managed = self.managed.image(base.map(|image| &image.managed))?;
        let raw = self.raw.image(base.map(|image| &image.raw));

        let image = HeapImage { managed, raw };

        self.cached_image = Some(Arc::new(image.clone()));

        Ok(image)
    }

    /// Restore this heap from one immutable image.
    pub fn restore_image(&mut self, image: &HeapImage) {
        *self = Self::from_image(image);
    }

    /// Create a heap from one serialized snapshot.
    pub fn from_snapshot(snapshot: &HeapSnapshot) -> Self {
        let image = HeapImage::from_snapshot(snapshot);

        Self::from_image(&image)
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

    /// Admit one managed retained-byte delta against the current limits.
    fn admit_managed_delta(
        limits: HeapLimits,
        managed_bytes: u64,
        raw_bytes: u64,
        admitted_delta: &mut i64,
        managed_delta: i64,
    ) -> Result<(), HeapLimitError> {
        let next_delta = admitted_delta.saturating_add(managed_delta);
        limits.check_delta(managed_bytes, raw_bytes, next_delta, 0)?;
        *admitted_delta = next_delta;

        Ok(())
    }

    /// Admit one raw retained-byte delta against the current limits.
    fn admit_raw_delta(
        limits: HeapLimits,
        managed_bytes: u64,
        raw_bytes: u64,
        admitted_delta: &mut i64,
        raw_delta: i64,
    ) -> Result<(), HeapLimitError> {
        let next_delta = admitted_delta.saturating_add(raw_delta);
        limits.check_delta(managed_bytes, raw_bytes, 0, next_delta)?;
        *admitted_delta = next_delta;

        Ok(())
    }
}

impl Default for Heap {
    fn default() -> Self {
        Self::new()
    }
}

impl Capture for Heap {
    type Image = HeapImage;
    type Error = HeapCaptureError;
    type CaptureContext<'a> = ();
    type RestoreContext<'a> = ();

    /// Capture one heap image for the given mode.
    fn capture_image(
        &mut self,
        _mode: CaptureMode,
        _context: Self::CaptureContext<'_>,
    ) -> Result<Self::Image, Self::Error> {
        self.image()
    }

    /// Restore one heap image.
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

    /// Encode one heap image as one flat snapshot.
    fn encode_snapshot(image: &Self::Image) -> Result<Self::Snapshot, Self::Error> {
        Ok(image.snapshot())
    }

    /// Decode one heap snapshot back into one in-memory image.
    fn decode_snapshot(snapshot: &Self::Snapshot) -> Result<Self::Image, Self::Error> {
        Ok(HeapImage::from_snapshot(snapshot))
    }
}
