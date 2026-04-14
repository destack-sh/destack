use std::borrow::Cow;

use super::Heap;
use crate::managed::ReferenceMap;
use crate::value::{ManagedReference, RawPointer, Value};
use crate::{HeapLimitError, LayoutId};

impl Heap {
    /// Allocate one managed byte allocation.
    pub fn allocate_managed_bytes(
        &mut self,
        bytes: &[u8],
        reference_map: ReferenceMap,
        layout_id: Option<LayoutId>,
    ) -> Result<ManagedReference, HeapLimitError> {
        // check the projected managed reservation first
        self.check_managed_allocation_reservation(bytes.len(), &reference_map, layout_id)?;

        // then allocate through managed space
        Ok(self.managed.allocate_bytes(bytes, reference_map, layout_id))
    }

    /// Allocate one raw byte allocation.
    pub fn allocate_raw_bytes(&mut self, bytes: &[u8]) -> Result<RawPointer, HeapLimitError> {
        // check the projected raw reservation first
        self.check_raw_allocation_reservation(bytes.len())?;

        // then allocate through raw space
        Ok(self.raw.allocate_bytes(bytes))
    }

    /// Allocate one zeroed managed byte allocation.
    pub fn allocate_managed_zeroed(
        &mut self,
        byte_len: usize,
        reference_map: ReferenceMap,
        layout_id: Option<LayoutId>,
    ) -> Result<ManagedReference, HeapLimitError> {
        self.allocate_managed_bytes(&vec![0; byte_len], reference_map, layout_id)
    }

    /// Allocate one managed byte allocation with one nominal type id.
    pub fn allocate_managed_bytes_typed(
        &mut self,
        bytes: &[u8],
        reference_map: ReferenceMap,
        layout_id: Option<LayoutId>,
        type_id: u32,
    ) -> Result<ManagedReference, HeapLimitError> {
        // allocate the payload first
        let reference = self.allocate_managed_bytes(bytes, reference_map, layout_id)?;
        let is_set = self.managed.set_type_id(reference, type_id);

        debug_assert!(is_set);

        Ok(reference)
    }

    /// Allocate one managed byte allocation with one borrowed reference map and nominal type id.
    pub fn allocate_managed_bytes_borrowed_typed(
        &mut self,
        bytes: &[u8],
        reference_map: &ReferenceMap,
        layout_id: Option<LayoutId>,
        type_id: u32,
    ) -> Result<ManagedReference, HeapLimitError> {
        self.allocate_managed_bytes_typed(bytes, reference_map.clone(), layout_id, type_id)
    }

    /// Allocate one zeroed managed byte allocation with one nominal type id.
    pub fn allocate_managed_zeroed_borrowed_typed(
        &mut self,
        byte_len: usize,
        reference_map: &ReferenceMap,
        layout_id: Option<LayoutId>,
        type_id: u32,
    ) -> Result<ManagedReference, HeapLimitError> {
        self.allocate_managed_bytes_typed(
            &vec![0; byte_len],
            reference_map.clone(),
            layout_id,
            type_id,
        )
    }

    /// Allocate one zeroed managed byte allocation with one borrowed reference map.
    pub fn allocate_managed_zeroed_borrowed(
        &mut self,
        byte_len: usize,
        reference_map: &ReferenceMap,
        layout_id: Option<LayoutId>,
    ) -> Result<ManagedReference, HeapLimitError> {
        self.allocate_managed_zeroed(byte_len, reference_map.clone(), layout_id)
    }

    /// Allocate one zeroed managed byte allocation using repeated reference offsets.
    pub fn allocate_managed_zeroed_repeated_reference_offsets(
        &mut self,
        byte_len: usize,
        count: u32,
        element_size: u32,
        offsets: &[u32],
        layout_id: Option<LayoutId>,
    ) -> Result<ManagedReference, HeapLimitError> {
        let reference_map = ReferenceMap::RepeatedReferenceOffsets {
            count,
            element_size,
            offsets: offsets.to_vec().into_boxed_slice(),
        };

        self.allocate_managed_zeroed(byte_len, reference_map, layout_id)
    }

    /// Return whether one managed reference currently refers to one live allocation.
    pub fn is_managed_allocated(&self, reference: ManagedReference) -> bool {
        self.managed.is_allocated(reference)
    }

    /// Return the bytes for one managed allocation.
    pub fn managed_bytes(&self, reference: ManagedReference) -> Option<Cow<'_, [u8]>> {
        Some(Cow::Owned(self.managed.bytes(reference)?))
    }

    /// Return the bytes for one managed allocation as one owned vector.
    pub fn managed_bytes_to_vec(&self, reference: ManagedReference) -> Option<Vec<u8>> {
        Some(self.managed_bytes(reference)?.into_owned())
    }

    /// Return the remaining byte length for one managed allocation.
    pub fn managed_byte_len(&self, reference: ManagedReference) -> Option<usize> {
        self.managed.byte_len(reference)
    }

    /// Return the nominal managed type id for one managed allocation.
    pub fn managed_type_id(&self, reference: ManagedReference) -> Option<u32> {
        self.managed.type_id(reference)
    }

    /// Return the physical managed layout id for one managed allocation.
    pub fn managed_layout_id(&self, reference: ManagedReference) -> Option<LayoutId> {
        self.managed.layout_id(reference)
    }

    /// Return the managed reference map for one managed allocation.
    pub fn reference_map(&self, reference: ManagedReference) -> Option<&ReferenceMap> {
        self.managed.reference_map(reference)
    }

    /// Set the nominal managed type id for one managed allocation.
    pub fn set_managed_type_id(&mut self, reference: ManagedReference, type_id: u32) -> bool {
        self.managed.set_type_id(reference, type_id)
    }

    /// Overwrite one managed byte range.
    pub fn set_managed_bytes(
        &mut self,
        reference: ManagedReference,
        start: usize,
        bytes: &[u8],
    ) -> bool {
        self.managed.set_bytes(reference, start, bytes)
    }

    /// Allocate one zeroed raw byte allocation.
    pub fn allocate_raw_zeroed(&mut self, byte_len: usize) -> Result<RawPointer, HeapLimitError> {
        self.allocate_raw_bytes(&vec![0; byte_len])
    }

    /// Allocate one raw value buffer.
    pub fn allocate_raw_values(
        &mut self,
        values: Vec<Value>,
    ) -> Result<RawPointer, HeapLimitError> {
        // flatten the values into one raw byte buffer first
        let bytes = Self::raw_value_bytes(values.into_iter());

        self.allocate_raw_bytes(&bytes)
    }

    /// Allocate one zeroed raw value buffer with one explicit slot count.
    pub fn allocate_raw_slots(&mut self, slot_count: usize) -> Result<RawPointer, HeapLimitError> {
        self.allocate_raw_zeroed(slot_count.saturating_mul(Value::BYTE_LEN))
    }

    /// Return the bytes for one raw allocation.
    pub fn raw_bytes(&self, pointer: RawPointer) -> Option<Cow<'_, [u8]>> {
        Some(Cow::Owned(self.raw.bytes(pointer)?))
    }

    /// Return the remaining byte length for one raw allocation.
    pub fn raw_byte_len(&self, pointer: RawPointer) -> Option<usize> {
        self.raw.byte_len(pointer)
    }

    /// Return the values for one raw allocation.
    pub fn raw_values(&self, pointer: RawPointer) -> Option<Vec<Value>> {
        // load the raw byte payload first
        let bytes = self.raw_bytes(pointer)?.into_owned();

        // then decode each full value slot
        Some(
            bytes
                .chunks_exact(Value::BYTE_LEN)
                .filter_map(Value::from_byte_slice)
                .collect(),
        )
    }

    /// Return one raw byte by offset.
    pub fn raw_byte_at(&self, pointer: RawPointer, index: usize) -> Option<u8> {
        self.raw.bytes(pointer)?.get(index).copied()
    }

    /// Replace the bytes for one raw allocation.
    pub fn replace_raw_bytes(
        &mut self,
        pointer: RawPointer,
        bytes: &[u8],
    ) -> Result<bool, HeapLimitError> {
        // resolve the previous live payload length first
        let Some(previous_len) = self.raw.byte_len(pointer) else {
            return Ok(false);
        };

        // check the projected replacement reservation next
        self.check_raw_replace_reservation(previous_len, bytes.len())?;

        // then replace the raw payload
        Ok(self.raw.replace_bytes(pointer, bytes))
    }

    /// Replace the values for one raw allocation.
    pub fn replace_raw_values(
        &mut self,
        pointer: RawPointer,
        values: &[Value],
    ) -> Result<bool, HeapLimitError> {
        // flatten the values into one raw byte buffer first
        let bytes = Self::raw_value_bytes(values.iter().copied());

        self.replace_raw_bytes(pointer, &bytes)
    }

    /// Overwrite one raw byte range.
    pub fn set_raw_bytes(&mut self, pointer: RawPointer, start: usize, bytes: &[u8]) -> bool {
        self.raw.set_bytes(pointer, start, bytes)
    }

    /// Overwrite one raw value slot.
    pub fn set_raw_value(&mut self, pointer: RawPointer, index: usize, value: Value) -> bool {
        self.set_raw_bytes(
            pointer,
            index.saturating_mul(Value::BYTE_LEN),
            &value.to_byte_array(),
        )
    }

    /// Overwrite one raw byte.
    pub fn set_raw_byte(&mut self, pointer: RawPointer, index: usize, byte: u8) -> bool {
        self.set_raw_bytes(pointer, index, &[byte])
    }

    /// Free one raw allocation.
    pub fn free_raw(&mut self, pointer: RawPointer) -> bool {
        self.raw.free(pointer)
    }

    /// Check the projected reservation for one managed allocation.
    fn check_managed_allocation_reservation(
        &self,
        byte_len: usize,
        reference_map: &ReferenceMap,
        layout_id: Option<LayoutId>,
    ) -> Result<(), HeapLimitError> {
        let budget = self.budget();
        let managed_reservation =
            self.managed
                .allocate_bytes_active_reservation(byte_len, reference_map, layout_id);

        budget.check_active_reservation(managed_reservation, 0)
    }

    /// Check the projected reservation for one raw allocation.
    fn check_raw_allocation_reservation(&self, byte_len: usize) -> Result<(), HeapLimitError> {
        let budget = self.budget();
        let raw_reservation = self.raw.allocate_bytes_active_reservation(byte_len);

        budget.check_active_reservation(0, raw_reservation)
    }

    /// Check the projected reservation for one raw replacement.
    fn check_raw_replace_reservation(
        &self,
        previous_len: usize,
        next_len: usize,
    ) -> Result<(), HeapLimitError> {
        let budget = self.budget();
        let reservation = next_len as i64 - previous_len as i64;

        budget.check_active_reservation(0, reservation)
    }

    /// Flatten one iterator of values into one raw byte buffer.
    fn raw_value_bytes(values: impl IntoIterator<Item = Value>) -> Vec<u8> {
        values.into_iter().flat_map(Value::to_byte_array).collect()
    }
}
