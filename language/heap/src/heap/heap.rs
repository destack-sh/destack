use std::borrow::Cow;
use std::error::Error;
use std::fmt;
use std::sync::Arc;

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
    /// The last captured image used to reuse unchanged image leaves.
    cached_image: Option<Arc<HeapImage>>,
}

/// One agent-local execution memory view across local and shared memory.
#[derive(Debug)]
pub struct AgentMemory<'a> {
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
        let managed = ManagedSpace::with_layout(&layout);
        let raw = RawSpace::with_layout(&layout);

        Self {
            layout,
            managed,
            raw,
            limits,
            cached_image: None,
        }
    }

    /// Create one heap from one immutable image.
    pub fn from_image(image: &HeapImage) -> Self {
        let layout = HeapLayoutOptions {
            size_classes: image.managed.size_classes.clone(),
            managed_run_bytes: image.managed.run_bytes,
            raw_run_bytes: image.raw.run_bytes,
            chunk_bytes: image.managed.chunk_bytes,
        };

        debug_assert_eq!(image.managed.chunk_bytes, image.raw.chunk_bytes);

        Self {
            managed: ManagedSpace::from_image(&image.managed),
            raw: RawSpace::from_image(&image.raw),
            layout,
            limits: HeapLimits::default(),
            cached_image: Some(Arc::new(image.clone())),
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
        limits.check(self.managed.retained_bytes(), self.raw.retained_bytes())?;
        self.limits = limits;
        Ok(())
    }

    /// Check the configured heap hard limits against current usage.
    pub fn check_limits(&self) -> Result<(), HeapLimitError> {
        self.budget().check_delta(0, 0)
    }

    /// Return the current live heap budget.
    pub fn budget(&self) -> HeapBudget {
        HeapBudget::new(
            self.limits,
            self.managed.retained_bytes(),
            self.raw.retained_bytes(),
        )
    }

    /// Allocate one managed byte allocation.
    pub fn allocate_managed_bytes(
        &mut self,
        bytes: &[u8],
        reference_map: ReferenceMap,
        layout_id: Option<LayoutId>,
    ) -> Result<ManagedReference, HeapLimitError> {
        self.cached_image = None;
        let budget = self.budget();
        let old_managed_bytes = self.managed.retained_bytes();
        let handle = self.managed.allocate_bytes(bytes, reference_map, layout_id);

        let managed_delta = retained_delta(old_managed_bytes, self.managed.retained_bytes());
        if let Err(error) = budget.check_delta(managed_delta, 0) {
            let _ = self.managed.free(handle);
            return Err(error);
        }

        Ok(handle)
    }

    /// Allocate one zeroed managed byte allocation.
    pub fn allocate_managed_zeroed(
        &mut self,
        byte_len: usize,
        reference_map: ReferenceMap,
        layout_id: Option<LayoutId>,
    ) -> Result<ManagedReference, HeapLimitError> {
        self.cached_image = None;
        let budget = self.budget();
        let old_managed_bytes = self.managed.retained_bytes();
        let handle = self
            .managed
            .allocate_zeroed(byte_len, reference_map, layout_id);

        let managed_delta = retained_delta(old_managed_bytes, self.managed.retained_bytes());
        if let Err(error) = budget.check_delta(managed_delta, 0) {
            let _ = self.managed.free(handle);
            return Err(error);
        }

        Ok(handle)
    }

    /// Allocate one packed managed value allocation.
    pub fn allocate_packed_values(
        &mut self,
        values: Vec<Value>,
    ) -> Result<ManagedReference, HeapLimitError> {
        let bytes = encode_values(&values);
        self.allocate_managed_bytes(&bytes, ReferenceMap::value_array(values.len()), None)
    }

    /// Allocate one packed managed pair.
    pub fn allocate_packed_pair(
        &mut self,
        first: Value,
        second: Value,
    ) -> Result<ManagedReference, HeapLimitError> {
        self.allocate_packed_values(vec![first, second])
    }

    /// Allocate one packed managed single.
    pub fn allocate_packed_single(
        &mut self,
        value: Value,
    ) -> Result<ManagedReference, HeapLimitError> {
        self.allocate_packed_values(vec![value])
    }

    /// Allocate one empty packed managed value buffer.
    pub fn allocate_zeroed_packed_values(
        &mut self,
        count: usize,
    ) -> Result<ManagedReference, HeapLimitError> {
        self.allocate_managed_zeroed(
            count * Value::BYTE_LEN,
            ReferenceMap::value_array(count),
            None,
        )
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

    /// Return one packed value by index.
    pub fn packed_value_at(&self, handle: ManagedReference, index: usize) -> Option<Value> {
        self.managed.packed_value_at(handle, index)
    }

    /// Return the number of packed values in one managed allocation.
    pub fn packed_value_count(&self, handle: ManagedReference) -> Option<usize> {
        self.managed.packed_value_count(handle)
    }

    /// Return one owned copy of the packed values.
    pub fn packed_values_to_vec(&self, handle: ManagedReference) -> Option<Vec<Value>> {
        self.managed.packed_values_to_vec(handle)
    }

    /// Set one packed value by index.
    pub fn set_packed_value(
        &mut self,
        handle: ManagedReference,
        index: usize,
        value: Value,
    ) -> bool {
        self.cached_image = None;
        self.managed.set_packed_value(handle, index, value)
    }

    /// Resize one packed managed value buffer.
    pub fn resize_packed_values(
        &mut self,
        handle: ManagedReference,
        len: usize,
    ) -> Result<bool, HeapLimitError> {
        self.cached_image = None;
        let budget = self.budget();
        let old_values = self.managed.packed_values_to_vec(handle);
        let old_managed_bytes = self.managed.retained_bytes();
        let replaced = self.managed.resize_packed_values(handle, len);

        if !replaced {
            return Ok(false);
        }

        let managed_delta = retained_delta(old_managed_bytes, self.managed.retained_bytes());
        if let Err(error) = budget.check_delta(managed_delta, 0) {
            if let Some(old_values) = old_values {
                let _ = self.managed.replace_packed_values(handle, &old_values);
            }
            return Err(error);
        }

        Ok(true)
    }

    /// Return the managed reference map for this handle.
    pub fn reference_map(&self, handle: ManagedReference) -> Option<&ReferenceMap> {
        self.managed.reference_map(handle)
    }

    /// Set one managed byte.
    pub fn set_managed_byte(&mut self, handle: ManagedReference, index: usize, byte: u8) -> bool {
        self.cached_image = None;
        self.managed.set_byte(handle, index, byte)
    }

    /// Report whether one managed reference is currently allocated.
    pub fn is_managed_allocated(&self, handle: ManagedReference) -> bool {
        self.managed.is_allocated(handle)
    }

    /// Return the current managed GC state.
    pub fn managed_gc_state(&self) -> &GcState {
        self.managed.gc_state()
    }

    /// Return the exact retained managed bytes.
    pub fn managed_retained_bytes(&self) -> u64 {
        self.managed.retained_bytes()
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
        self.cached_image = None;
        self.managed.collect_handles(handles)
    }

    /// Allocate one raw packed-value allocation.
    pub fn allocate_raw_values(
        &mut self,
        values: Vec<Value>,
    ) -> Result<RawPointer, HeapLimitError> {
        self.cached_image = None;
        let budget = self.budget();
        let old_raw_bytes = self.raw.retained_bytes();
        let pointer = self.raw.allocate_packed_values(values);

        let raw_delta = retained_delta(old_raw_bytes, self.raw.retained_bytes());
        if let Err(error) = budget.check_delta(0, raw_delta) {
            let _ = self.raw.free(pointer);
            return Err(error);
        }

        Ok(pointer)
    }

    /// Allocate one empty raw allocation.
    pub fn allocate_raw(&mut self) -> Result<RawPointer, HeapLimitError> {
        self.allocate_raw_bytes(&[])
    }

    /// Allocate one zeroed raw packed-value buffer.
    pub fn allocate_raw_slots(&mut self, slot_count: usize) -> Result<RawPointer, HeapLimitError> {
        self.cached_image = None;
        let budget = self.budget();
        let old_raw_bytes = self.raw.retained_bytes();
        let pointer = self.raw.allocate_packed_value_slots(slot_count);

        let raw_delta = retained_delta(old_raw_bytes, self.raw.retained_bytes());
        if let Err(error) = budget.check_delta(0, raw_delta) {
            let _ = self.raw.free(pointer);
            return Err(error);
        }

        Ok(pointer)
    }

    /// Allocate one raw byte allocation.
    pub fn allocate_raw_bytes(&mut self, bytes: &[u8]) -> Result<RawPointer, HeapLimitError> {
        self.cached_image = None;
        let budget = self.budget();
        let old_raw_bytes = self.raw.retained_bytes();
        let pointer = self.raw.allocate_bytes(bytes);

        let raw_delta = retained_delta(old_raw_bytes, self.raw.retained_bytes());
        if let Err(error) = budget.check_delta(0, raw_delta) {
            let _ = self.raw.free(pointer);
            return Err(error);
        }

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
        self.cached_image = None;
        self.raw.free(pointer)
    }

    /// Set one raw byte.
    pub fn set_raw_byte(&mut self, pointer: RawPointer, index: usize, byte: u8) -> bool {
        self.cached_image = None;
        self.raw.set_byte(pointer, index, byte)
    }

    /// Resize one raw packed-value payload.
    pub fn resize_raw_values(
        &mut self,
        pointer: RawPointer,
        len: usize,
    ) -> Result<bool, HeapLimitError> {
        self.cached_image = None;
        let budget = self.budget();
        let old_values = self.raw.values_to_vec(pointer);
        let old_raw_bytes = self.raw.retained_bytes();
        let replaced = self.raw.resize_values(pointer, len);

        if !replaced {
            return Ok(false);
        }

        let raw_delta = retained_delta(old_raw_bytes, self.raw.retained_bytes());
        if let Err(error) = budget.check_delta(0, raw_delta) {
            if let Some(old_values) = old_values {
                let _ = self.raw.replace_bytes(pointer, &encode_values(&old_values));
            }
            return Err(error);
        }

        Ok(true)
    }

    /// Set one raw packed value.
    pub fn set_raw_value(&mut self, pointer: RawPointer, index: usize, value: Value) -> bool {
        self.cached_image = None;
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
        self.cached_image = None;
        let budget = self.budget();
        let old_bytes = self.raw.bytes_to_vec(pointer);
        let old_raw_bytes = self.raw.retained_bytes();
        let replaced = self.raw.replace_bytes(pointer, bytes);

        if !replaced {
            return Ok(false);
        }

        let raw_delta = retained_delta(old_raw_bytes, self.raw.retained_bytes());
        if let Err(error) = budget.check_delta(0, raw_delta) {
            if let Some(old_bytes) = old_bytes {
                let _ = self.raw.replace_bytes(pointer, &old_bytes);
            }
            return Err(error);
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
                allocation_bytes: self.managed.allocation_bytes(),
                retained_bytes: self.managed.retained_bytes(),
            },
            raw: RawSpaceUsage {
                allocation_count: self.raw.allocation_count(),
                allocation_bytes: self.raw.usage().allocation_bytes,
                retained_bytes: self.raw.retained_bytes(),
            },
        }
    }

    /// Capture one immutable heap image.
    pub fn image(&mut self) -> Result<HeapImage, HeapCaptureError> {
        let base_managed = self.cached_image.as_deref().map(|image| &image.managed);
        let base_raw = self.cached_image.as_deref().map(|image| &image.raw);
        let managed = self.managed.image(base_managed)?;
        let raw = self.raw.image(base_raw);
        let image = HeapImage { managed, raw };
        self.cached_image = Some(Arc::new(image.clone()));
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

impl AgentMemory<'_> {
    /// Create one agent memory view.
    pub fn new<'a>(heap: &'a mut Heap, shared: &'a mut SharedSpace) -> AgentMemory<'a> {
        Self::with_shared_limits(heap, shared, SharedLimits::default())
    }

    /// Create one agent memory view with explicit shared-memory limits.
    pub fn with_shared_limits<'a>(
        heap: &'a mut Heap,
        shared: &'a mut SharedSpace,
        shared_limits: SharedLimits,
    ) -> AgentMemory<'a> {
        let shared_budget = SharedBudget::new(shared_limits, shared.retained_bytes());

        AgentMemory {
            heap,
            shared,
            shared_budget,
        }
    }

    /// Reborrow this agent memory view for one nested operation.
    pub fn reborrow(&mut self) -> AgentMemory<'_> {
        AgentMemory {
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

// convert one retained-byte before and after pair into a signed delta
fn retained_delta(before: u64, after: u64) -> i64 {
    if after >= before {
        (after - before) as i64
    } else {
        -((before - after) as i64)
    }
}
