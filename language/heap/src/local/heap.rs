use std::sync::Arc;

use crate::arena::Arena;
use crate::local::managed::ManagedSpace;
use crate::local::raw::RawSpace;
use crate::value::{ManagedReference, RawPointer, Value};
use crate::{EdgeMap, GcState, GcStats, HeapError, HeapLimits, HeapOptions, HeapResult, LayoutId};

/// One live local heap rooted in one shared arena.
#[derive(Debug)]
pub struct Heap {
    /// The shared page arena for every local byte payload.
    pub(super) arena: Arc<Arena>,
    /// The configured heap options.
    pub(super) options: HeapOptions,
    /// The managed local allocation space.
    pub(super) managed: ManagedSpace,
    /// The raw local allocation space.
    pub(crate) raw: RawSpace,
    /// Exact hard limits for this heap.
    pub(super) limits: HeapLimits,
}

impl Heap {
    /// Create one heap with the default limits and options.
    pub fn new() -> HeapResult<Self> {
        Self::with_limits_and_options(HeapLimits::default(), HeapOptions::default())
    }

    /// Create one heap with explicit limits and options.
    pub fn with_limits_and_options(limits: HeapLimits, options: HeapOptions) -> HeapResult<Self> {
        let arena = Arc::new(Arena::try_new(
            options.page_bytes,
            options.arena_segment_bytes,
        )?);

        options.validate()?;
        options.validate_arena(&arena)?;

        Self::build_with_options(arena, limits, options)
    }

    /// Create one heap from one checked shared arena, limits, and options.
    fn build_with_options(
        arena: Arc<Arena>,
        limits: HeapLimits,
        options: HeapOptions,
    ) -> HeapResult<Self> {
        Ok(Self {
            managed: ManagedSpace::build_with_options(arena.clone(), &options)?,
            raw: RawSpace::with_options(arena.clone(), &options)?,
            arena,
            options,
            limits,
        })
    }

    /// Return the shared page arena.
    pub(crate) fn arena(&self) -> &Arc<Arena> {
        &self.arena
    }

    /// Return the heap options.
    pub(crate) fn options(&self) -> &HeapOptions {
        &self.options
    }

    /// Return the encoded managed-reference byte width for this heap.
    pub fn managed_reference_bytes(&self) -> u8 {
        self.options.managed_reference_bytes
    }

    /// Return the configured heap limits.
    pub fn limits(&self) -> HeapLimits {
        self.limits
    }

    /// Replace the heap hard limits.
    pub fn set_limits(&mut self, limits: HeapLimits) -> HeapResult<()> {
        self.limits = limits;
        self.check_limits()
    }

    /// Check the configured heap hard limits against current usage.
    pub fn check_limits(&self) -> HeapResult<()> {
        self.check_mapped_delta(0, 0)
    }

    /// Return the currently live managed references.
    pub fn live_references(&self) -> HeapResult<Vec<ManagedReference>> {
        self.managed.live_references()
    }

    /// Return the current collector state.
    pub fn gc_state(&self) -> &GcState {
        self.managed.gc_state()
    }

    /// Perform one young managed collection over explicit roots.
    pub fn collect_young(
        &mut self,
        roots: impl IntoIterator<Item = ManagedReference>,
    ) -> HeapResult<GcStats> {
        self.managed.collect_young(roots)
    }

    /// Perform one full managed collection over explicit roots.
    pub fn collect(
        &mut self,
        roots: impl IntoIterator<Item = ManagedReference>,
    ) -> HeapResult<GcStats> {
        self.managed.collect(roots)
    }

    /// Allocate one managed byte allocation.
    pub fn allocate_managed_bytes(
        &mut self,
        bytes: &[u8],
        edge_map: EdgeMap,
        layout_id: Option<LayoutId>,
    ) -> HeapResult<ManagedReference> {
        // check the projected managed mapped-byte delta first
        self.check_managed_allocation_mapped_delta(bytes.len(), &edge_map, layout_id)?;

        // then allocate through managed space
        self.managed.allocate_bytes(bytes, edge_map, layout_id)
    }

    /// Allocate one raw byte allocation.
    pub fn allocate_raw_bytes(&mut self, bytes: &[u8]) -> HeapResult<RawPointer> {
        // check the projected raw mapped-byte delta first
        self.check_raw_allocation_mapped_delta(bytes.len())?;

        // then allocate through raw space
        self.raw.allocate_bytes(bytes)
    }

    /// Allocate one zeroed managed byte allocation.
    pub fn allocate_managed_zeroed(
        &mut self,
        byte_len: usize,
        edge_map: EdgeMap,
        layout_id: Option<LayoutId>,
    ) -> HeapResult<ManagedReference> {
        // check the projected managed mapped-byte delta first
        self.check_managed_allocation_mapped_delta(byte_len, &edge_map, layout_id)?;

        // then allocate through managed space
        self.managed.allocate_zeroed(byte_len, edge_map, layout_id)
    }

    /// Return whether one managed reference currently refers to one live allocation.
    pub fn is_managed_live(&self, reference: ManagedReference) -> bool {
        self.managed.is_live(reference)
    }

    /// Return the bytes for one managed allocation as one owned vector.
    pub fn read_managed_bytes(&self, reference: ManagedReference) -> HeapResult<Vec<u8>> {
        self.managed.read_bytes(reference)
    }

    /// Return the remaining byte length for one managed allocation.
    pub fn managed_byte_len(&self, reference: ManagedReference) -> HeapResult<usize> {
        self.managed.byte_len(reference)
    }

    /// Fill one caller-provided buffer from one managed allocation at one offset.
    pub fn read_managed_bytes_into(
        &self,
        reference: ManagedReference,
        start: usize,
        target: &mut [u8],
    ) -> HeapResult<()> {
        self.managed.read_bytes_into(reference, start, target)
    }

    /// Return the storage layout id for one managed allocation.
    pub fn managed_layout_id(&self, reference: ManagedReference) -> HeapResult<Option<LayoutId>> {
        self.managed.layout_id(reference)
    }

    /// Return the managed edge map for one managed allocation.
    pub fn edge_map(&self, reference: ManagedReference) -> HeapResult<&EdgeMap> {
        self.managed.edge_map(reference)
    }

    /// Overwrite one managed byte range.
    pub fn write_managed_bytes(
        &mut self,
        reference: ManagedReference,
        start: usize,
        bytes: &[u8],
    ) -> HeapResult<()> {
        let mapped_delta = self
            .managed
            .write_mapped_delta(reference, start, bytes.len())?;

        self.check_mapped_delta(mapped_delta, 0)?;

        self.managed.write_bytes(reference, start, bytes)
    }

    /// Record one managed write barrier over one byte range.
    pub fn write_barrier(
        &mut self,
        reference: ManagedReference,
        start: usize,
        byte_len: usize,
    ) -> HeapResult<()> {
        self.managed.write_barrier(reference, start, byte_len)
    }

    /// Allocate one zeroed raw byte allocation.
    pub fn allocate_raw_zeroed(&mut self, byte_len: usize) -> HeapResult<RawPointer> {
        // check the projected raw mapped-byte delta first
        self.check_raw_allocation_mapped_delta(byte_len)?;

        // then allocate through raw space
        self.raw.allocate_zeroed(byte_len)
    }

    /// Allocate one raw value buffer.
    pub fn allocate_raw_values(&mut self, values: Vec<Value>) -> HeapResult<RawPointer> {
        // flatten the values into one raw byte buffer first
        let bytes = Self::raw_value_bytes(values);

        self.allocate_raw_bytes(&bytes)
    }

    /// Allocate one zeroed raw value buffer with one explicit slot count.
    pub fn allocate_raw_slots(&mut self, slot_count: usize) -> HeapResult<RawPointer> {
        let byte_len =
            slot_count
                .checked_mul(Value::BYTE_LEN)
                .ok_or(HeapError::InvariantOverflow {
                    context: "raw value buffer byte length",
                })?;

        self.allocate_raw_zeroed(byte_len)
    }

    /// Return the bytes for one raw allocation as one owned vector.
    pub fn read_raw_bytes(&self, pointer: RawPointer) -> HeapResult<Vec<u8>> {
        self.raw.read_bytes(pointer)
    }

    /// Return the remaining byte length for one raw allocation.
    pub fn raw_byte_len(&self, pointer: RawPointer) -> HeapResult<usize> {
        self.raw.byte_len(pointer)
    }

    /// Fill one caller-provided buffer from one raw allocation at one offset.
    pub fn read_raw_bytes_into(
        &self,
        pointer: RawPointer,
        start: usize,
        target: &mut [u8],
    ) -> HeapResult<()> {
        self.raw.read_bytes_into(pointer, start, target)
    }

    /// Return the values for one raw allocation.
    pub fn raw_values(&self, pointer: RawPointer) -> HeapResult<Vec<Value>> {
        self.raw.values(pointer)
    }

    /// Return one raw byte by offset.
    pub fn raw_byte_at(&self, pointer: RawPointer, index: usize) -> Option<u8> {
        self.raw.byte_at(pointer, index)
    }

    /// Replace the bytes for one raw allocation.
    pub fn replace_raw_bytes(&mut self, pointer: RawPointer, bytes: &[u8]) -> HeapResult<()> {
        // check the projected replacement mapped-byte delta next
        self.check_raw_replace_mapped_delta(pointer, bytes.len())?;

        // then replace the raw payload
        self.raw.replace_bytes(pointer, bytes)
    }

    /// Replace the values for one raw allocation.
    pub fn replace_raw_values(&mut self, pointer: RawPointer, values: &[Value]) -> HeapResult<()> {
        // flatten the values into one raw byte buffer first
        let bytes = Self::raw_value_bytes(values.iter().copied());

        self.replace_raw_bytes(pointer, &bytes)
    }

    /// Overwrite one raw byte range.
    pub fn set_raw_bytes(
        &mut self,
        pointer: RawPointer,
        start: usize,
        bytes: &[u8],
    ) -> HeapResult<()> {
        let mapped_delta = self.raw.write_mapped_delta(pointer, start, bytes.len())?;
        self.check_mapped_delta(0, mapped_delta)?;
        self.raw.set_bytes(pointer, start, bytes)
    }

    /// Overwrite one raw value slot.
    pub fn set_raw_value(
        &mut self,
        pointer: RawPointer,
        index: usize,
        value: Value,
    ) -> HeapResult<()> {
        let start = index
            .checked_mul(Value::BYTE_LEN)
            .ok_or(HeapError::InvariantOverflow {
                context: "raw value byte offset",
            })?;

        self.set_raw_bytes(pointer, start, &value.to_byte_array())
    }

    /// Overwrite one raw byte.
    pub fn set_raw_byte(&mut self, pointer: RawPointer, index: usize, byte: u8) -> HeapResult<()> {
        self.set_raw_bytes(pointer, index, &[byte])
    }

    /// Free one raw allocation.
    pub fn free_raw(&mut self, pointer: RawPointer) -> HeapResult<bool> {
        self.raw.free(pointer)
    }

    /// Check the projected mapped-byte delta for one managed allocation.
    fn check_managed_allocation_mapped_delta(
        &self,
        byte_len: usize,
        edge_map: &EdgeMap,
        layout_id: Option<LayoutId>,
    ) -> HeapResult<()> {
        let managed_mapped_delta = self
            .managed
            .alloc_mapped_delta(byte_len, edge_map, layout_id);

        self.check_mapped_delta(managed_mapped_delta, 0)
    }

    /// Check the projected mapped-byte delta for one raw allocation.
    fn check_raw_allocation_mapped_delta(&self, byte_len: usize) -> HeapResult<()> {
        let raw_mapped_delta = self.raw.alloc_mapped_delta(byte_len);

        self.check_mapped_delta(0, raw_mapped_delta)
    }

    /// Check the projected mapped-byte delta for one raw replacement.
    fn check_raw_replace_mapped_delta(
        &self,
        pointer: RawPointer,
        next_len: usize,
    ) -> HeapResult<()> {
        let mapped_delta = self.raw.replace_mapped_delta(pointer, next_len)?;

        self.check_mapped_delta(0, mapped_delta)
    }

    /// Check heap limits after one requested mapped-byte delta.
    fn check_mapped_delta(
        &self,
        managed_mapped_delta: i64,
        raw_mapped_delta: i64,
    ) -> HeapResult<()> {
        self.limits.check_mapped_delta(
            self.managed.active_bytes(),
            self.raw.active_bytes(),
            managed_mapped_delta,
            raw_mapped_delta,
        )
    }

    /// Flatten one iterator of values into one raw byte buffer.
    fn raw_value_bytes(values: impl IntoIterator<Item = Value>) -> Vec<u8> {
        values.into_iter().flat_map(Value::to_byte_array).collect()
    }
}
