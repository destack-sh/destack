use super::Heap;
use crate::managed::ReferenceMap;
use crate::value::{ManagedReference, RawPointer, Value};
use crate::{HeapResult, StorageLayoutId};

impl Heap {
    /// Allocate one managed byte allocation.
    pub fn allocate_managed_bytes(
        &mut self,
        bytes: &[u8],
        reference_map: ReferenceMap,
        layout_id: Option<StorageLayoutId>,
    ) -> HeapResult<ManagedReference> {
        // check the projected managed mapped-byte delta first
        self.check_managed_allocation_mapped_delta(bytes.len(), &reference_map, layout_id)?;

        // then allocate through managed space
        self.managed.allocate_bytes(bytes, reference_map, layout_id)
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
        reference_map: ReferenceMap,
        layout_id: Option<StorageLayoutId>,
    ) -> HeapResult<ManagedReference> {
        // check the projected managed mapped-byte delta first
        self.check_managed_allocation_mapped_delta(byte_len, &reference_map, layout_id)?;

        // then allocate through managed space
        self.managed
            .allocate_zeroed(byte_len, reference_map, layout_id)
    }

    /// Allocate one zeroed managed byte allocation with one borrowed reference map.
    pub fn allocate_managed_zeroed_borrowed(
        &mut self,
        byte_len: usize,
        reference_map: &ReferenceMap,
        layout_id: Option<StorageLayoutId>,
    ) -> HeapResult<ManagedReference> {
        self.allocate_managed_zeroed(byte_len, reference_map.clone(), layout_id)
    }

    /// Allocate one zeroed managed byte allocation using repeated reference offsets.
    pub fn allocate_managed_zeroed_repeated_reference_offsets(
        &mut self,
        byte_len: usize,
        count: u32,
        element_size: u32,
        offsets: &[u32],
        layout_id: Option<StorageLayoutId>,
    ) -> HeapResult<ManagedReference> {
        let reference_map = ReferenceMap::RepeatedReferenceOffsets {
            count,
            element_size,
            offsets: offsets.to_vec().into_boxed_slice(),
        };

        self.allocate_managed_zeroed(byte_len, reference_map, layout_id)
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
    pub fn managed_layout_id(
        &self,
        reference: ManagedReference,
    ) -> HeapResult<Option<StorageLayoutId>> {
        self.managed.layout_id(reference)
    }

    /// Return the managed reference map for one managed allocation.
    pub fn reference_map(&self, reference: ManagedReference) -> HeapResult<&ReferenceMap> {
        self.managed.reference_map(reference)
    }

    /// Overwrite one managed byte range.
    pub fn set_managed_bytes(
        &mut self,
        reference: ManagedReference,
        start: usize,
        bytes: &[u8],
    ) -> HeapResult<()> {
        let budget = self.budget();
        let mapped_delta = self
            .managed
            .write_mapped_delta(reference, start, bytes.len())?;

        budget.check_mapped_delta(mapped_delta, 0)?;

        self.managed.set_bytes(reference, start, bytes)
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
        self.allocate_raw_zeroed(slot_count.saturating_mul(Value::BYTE_LEN))
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
        let budget = self.budget();
        let mapped_delta = self.raw.write_mapped_delta(pointer, start, bytes.len())?;
        budget.check_mapped_delta(0, mapped_delta)?;
        self.raw.set_bytes(pointer, start, bytes)
    }

    /// Overwrite one raw value slot.
    pub fn set_raw_value(
        &mut self,
        pointer: RawPointer,
        index: usize,
        value: Value,
    ) -> HeapResult<()> {
        self.set_raw_bytes(
            pointer,
            index.saturating_mul(Value::BYTE_LEN),
            &value.to_byte_array(),
        )
    }

    /// Overwrite one raw byte.
    pub fn set_raw_byte(&mut self, pointer: RawPointer, index: usize, byte: u8) -> HeapResult<()> {
        self.set_raw_bytes(pointer, index, &[byte])
    }

    /// Free one raw allocation.
    pub fn free_raw(&mut self, pointer: RawPointer) -> crate::HeapResult<bool> {
        self.raw.free(pointer)
    }

    /// Check the projected mapped-byte delta for one managed allocation.
    fn check_managed_allocation_mapped_delta(
        &self,
        byte_len: usize,
        reference_map: &ReferenceMap,
        layout_id: Option<StorageLayoutId>,
    ) -> HeapResult<()> {
        let budget = self.budget();
        let managed_mapped_delta =
            self.managed
                .alloc_mapped_delta(byte_len, reference_map, layout_id);

        budget.check_mapped_delta(managed_mapped_delta, 0)
    }

    /// Check the projected mapped-byte delta for one raw allocation.
    fn check_raw_allocation_mapped_delta(&self, byte_len: usize) -> HeapResult<()> {
        let budget = self.budget();
        let raw_mapped_delta = self.raw.alloc_mapped_delta(byte_len);

        budget.check_mapped_delta(0, raw_mapped_delta)
    }

    /// Check the projected mapped-byte delta for one raw replacement.
    fn check_raw_replace_mapped_delta(
        &self,
        pointer: RawPointer,
        next_len: usize,
    ) -> HeapResult<()> {
        let budget = self.budget();
        let mapped_delta = self.raw.replace_mapped_delta(pointer, next_len)?;

        budget.check_mapped_delta(0, mapped_delta)
    }

    /// Flatten one iterator of values into one raw byte buffer.
    fn raw_value_bytes(values: impl IntoIterator<Item = Value>) -> Vec<u8> {
        values.into_iter().flat_map(Value::to_byte_array).collect()
    }
}
