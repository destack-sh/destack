use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::marker::PhantomData;
use std::sync::Arc;

use crate::diagnostic::Error;
use crate::program::{Layout, Program};
use crate::{SharedHeap, Word};
use destack_heap::{
    Heap, HeapReference, Payload, RawPointer, SharedRawBudget, SharedRawLimits, SharedRawPointer,
};
use destack_mir::{self as mir, ReferenceMap};

/// Handler invoked by the VM when calling an external function.
pub trait ExternalHandler:
    for<'ctx> Fn(&mut ExternalCallContext<'ctx>, &[Word]) -> Result<Word, Error> + Send + Sync
{
}

impl<T> ExternalHandler for T where
    T: for<'ctx> Fn(&mut ExternalCallContext<'ctx>, &[Word]) -> Result<Word, Error> + Send + Sync
{
}

/// Boxed external handler type.
pub type ExternalFn = Arc<dyn ExternalHandler>;

/// Runtime call context with restricted access to isolate state.
pub struct ExternalCallContext<'ctx> {
    /// The immutable program metadata for this isolate.
    program: &'ctx Program,
    /// The worker-local heap.
    heap: *mut Heap,
    /// The world-shared heap.
    shared: *const SharedHeap,
    /// The configured shared raw-space limits.
    shared_raw_limits: SharedRawLimits,
    /// The external pin scope for this call.
    pin_scope: PinScope,
    /// MIR types keyed by heap references allocated through this call context.
    heap_type_by_reference: BTreeMap<HeapReference, mir::LocalNodeId<mir::Type>>,
}

/// External VM call read capability.
#[derive(Debug)]
pub struct ExternalReadContext<'call, 'ctx> {
    /// The owning external call context.
    pub(super) context: *mut ExternalCallContext<'ctx>,
    /// The lifetime marker for the call context.
    _marker: PhantomData<&'call mut ExternalCallContext<'ctx>>,
}

/// Mutable external VM call capability.
#[derive(Debug)]
pub struct ExternalWriteContext<'call, 'ctx> {
    /// The owning external call context.
    pub(super) context: *mut ExternalCallContext<'ctx>,
    /// The lifetime marker for the call context.
    _marker: PhantomData<&'call mut ExternalCallContext<'ctx>>,
}

/// One external call pin scope.
#[derive(Debug, Default)]
struct PinScope {
    /// The local heap references pinned for the call lifetime.
    heap_references: Vec<HeapReference>,
    /// The unique pinned heap references for the call lifetime.
    pinned_heap_references: BTreeSet<HeapReference>,
    /// The per-call rewrites from original to pinned heap references.
    heap_reference_rewrites: BTreeMap<HeapReference, HeapReference>,
}

impl PinScope {
    /// Return one rewritten pinned heap reference when this scope already captured it.
    fn rewritten_heap_reference(&self, reference: HeapReference) -> Option<HeapReference> {
        self.heap_reference_rewrites.get(&reference).copied()
    }

    /// Record one pinned heap reference rewrite in this scope.
    fn rewrite_heap_reference(&mut self, original: HeapReference, pinned: HeapReference) {
        self.heap_reference_rewrites.insert(original, pinned);
        self.heap_reference_rewrites.entry(pinned).or_insert(pinned);
    }

    /// Record one pinned local heap reference in this scope.
    fn push_heap_reference(&mut self, reference: HeapReference) {
        if !self.pinned_heap_references.insert(reference) {
            return;
        }

        self.heap_references.push(reference);
    }
}

impl<'ctx> ExternalCallContext<'ctx> {
    /// Return the external call read capability.
    pub fn read(&self) -> ExternalReadContext<'_, 'ctx> {
        ExternalReadContext {
            context: self as *const Self as *mut Self,
            _marker: PhantomData,
        }
    }

    /// Return the mutable external call capability.
    pub fn write(&mut self) -> ExternalWriteContext<'_, 'ctx> {
        ExternalWriteContext {
            context: self as *mut Self,
            _marker: PhantomData,
        }
    }

    /// Create one external call context.
    pub(crate) fn new(
        program: &'ctx Program,
        heap: &'ctx mut Heap,
        shared: &'ctx SharedHeap,
        shared_raw_limits: SharedRawLimits,
    ) -> Self {
        Self {
            program,
            heap: heap as *mut Heap,
            shared: shared as *const SharedHeap,
            shared_raw_limits,
            pin_scope: PinScope::default(),
            heap_type_by_reference: BTreeMap::new(),
        }
    }

    /// Return the immutable program metadata.
    pub(super) fn program(&self) -> &Program {
        self.program
    }

    /// Return one compiled layout by type.
    pub(super) fn layout(&self, ty: mir::LocalNodeId<mir::Type>) -> Result<&Layout, Error> {
        self.program.layout(ty).ok_or(Error::InvalidInstruction)
    }

    /// Track the MIR type for one heap reference allocated through this context.
    pub(super) fn record_heap_type(
        &mut self,
        reference: HeapReference,
        ty: mir::LocalNodeId<mir::Type>,
    ) {
        if reference.is_null() {
            return;
        }

        self.heap_type_by_reference.insert(reference, ty);
    }

    /// Return the tracked MIR type for one heap reference.
    pub(super) fn heap_type(
        &self,
        reference: HeapReference,
    ) -> Option<mir::LocalNodeId<mir::Type>> {
        self.heap_type_by_reference.get(&reference).copied()
    }

    /// Borrow the local heap.
    pub(super) fn heap(&mut self) -> &mut Heap {
        unsafe { &mut *self.heap }
    }

    /// Allocate one local heap payload from one program layout id.
    pub(super) fn allocate_heap_layout(
        &mut self,
        layout_id: mir::LayoutId,
        payload: Payload<'_>,
    ) -> Result<HeapReference, Error> {
        let layout = self.program.allocation_layout(layout_id)?;

        unsafe { &mut *self.heap }
            .allocate(layout, payload)
            .map_err(Error::from)
    }

    /// Write one managed byte range through the external mutator path.
    pub(super) fn write_heap_bytes(
        &mut self,
        handle: HeapReference,
        start: usize,
        bytes: &[u8],
    ) -> Result<(), Error> {
        // publish the completed store to the collector
        self.heap().write_heap_bytes(handle, start, bytes)?;
        self.heap()
            .write_barrier(handle, start, bytes.len())
            .map_err(Error::from)
    }

    /// Borrow the local heap immutably.
    pub(super) fn heap_ref(&self) -> &Heap {
        unsafe { &*self.heap }
    }

    /// Borrow the world shared heap immutably.
    fn shared_ref(&self) -> &SharedHeap {
        unsafe { &*self.shared }
    }

    /// Borrow the world shared heap.
    fn shared(&self) -> &SharedHeap {
        unsafe { &*self.shared }
    }

    /// Return the current shared raw-space budget.
    fn shared_raw_budget(&self) -> SharedRawBudget {
        SharedRawBudget::new(self.shared_raw_limits, self.shared_ref().raw_active_bytes())
    }

    /// Allocate a raw heap byte buffer and return its pointer.
    pub fn allocate_raw_bytes(&mut self, bytes: &[u8]) -> Result<RawPointer, Error> {
        let heap = self.heap();
        heap.allocate_raw(bytes.len(), Payload::Bytes(bytes))
            .map_err(Error::from)
    }

    /// Allocate one zeroed raw heap byte buffer and return its pointer.
    pub fn allocate_zeroed_raw_bytes(&mut self, byte_len: usize) -> Result<RawPointer, Error> {
        let heap = self.heap();
        heap.allocate_raw(byte_len, Payload::Zeroed)
            .map_err(Error::from)
    }

    /// Allocate one raw packed-value buffer and return its pointer.
    pub fn allocate_raw_values(&mut self, values: Vec<Word>) -> Result<RawPointer, Error> {
        let bytes = values
            .into_iter()
            .flat_map(Word::to_byte_array)
            .collect::<Vec<_>>();

        self.allocate_raw_bytes(&bytes)
    }

    /// Allocate one zeroed raw packed-value buffer and return its pointer.
    pub fn allocate_raw_value_slots(&mut self, slot_count: usize) -> Result<RawPointer, Error> {
        let byte_len = slot_count
            .checked_mul(Word::BYTE_LEN)
            .ok_or(Error::InvariantViolation {
                context: "raw value buffer byte length".to_string(),
            })?;

        self.allocate_zeroed_raw_bytes(byte_len)
    }

    /// Allocate a heap packed-value buffer.
    pub fn allocate_heap_value_slots(&mut self, slot_count: usize) -> Result<HeapReference, Error> {
        let byte_len = slot_count
            .checked_mul(Word::BYTE_LEN)
            .ok_or(Error::InvariantViolation {
                context: "heap value buffer byte length".to_string(),
            })?;
        let reference_map = ReferenceMap::None;
        let layout = destack_heap::AllocationLayout::new(byte_len, &reference_map);

        self.heap()
            .allocate(layout, Payload::Zeroed)
            .map_err(Error::from)
    }

    /// Read heap packed values from one heap reference.
    pub fn heap_values(&mut self, reference: HeapReference) -> Result<Vec<Word>, Error> {
        let bytes = self.heap_ref().read_heap_bytes(reference)?;
        if bytes.len() % Word::BYTE_LEN != 0 {
            return Err(Error::InvalidHeapReference);
        }

        let mut values = Vec::with_capacity(bytes.len() / Word::BYTE_LEN);

        for bytes in bytes.chunks_exact(Word::BYTE_LEN) {
            let value = Word::from_byte_slice(bytes).ok_or(Error::InvalidHeapReference)?;
            let value = self.capture_value(value)?;
            values.push(value);
        }

        Ok(values)
    }

    /// Read one heap packed value by index.
    pub fn heap_value_at(&mut self, reference: HeapReference, index: usize) -> Result<Word, Error> {
        let start = index
            .checked_mul(Word::BYTE_LEN)
            .ok_or(Error::InvalidHeapReference)?;
        let mut bytes = [0u8; Word::BYTE_LEN];

        self.heap_ref()
            .read_heap_bytes_into(reference, start, &mut bytes)?;
        let value = Word::from_byte_slice(&bytes).ok_or(Error::InvalidHeapReference)?;

        self.capture_value(value)
    }

    /// Write one heap packed value by index.
    pub fn write_heap_value(
        &mut self,
        reference: HeapReference,
        index: usize,
        value: Word,
    ) -> Result<(), Error> {
        let start = index
            .checked_mul(Word::BYTE_LEN)
            .ok_or(Error::InvalidHeapReference)?;

        self.write_heap_bytes(reference, start, &value.to_byte_array())
    }

    /// Allocate a shared heap byte region and return its pointer.
    pub fn allocate_shared_bytes(&mut self, bytes: &[u8]) -> Result<SharedRawPointer, Error> {
        let mapped_delta = self.shared().raw_alloc_mapped_byte_delta(bytes.len());
        self.shared_raw_budget()
            .check_mapped_byte_delta(mapped_delta)?;

        self.shared()
            .allocate_raw(bytes.len(), Payload::Bytes(bytes))
            .map_err(Error::from)
    }

    /// Read raw bytes from a pointer to a bytes cell.
    pub fn raw_bytes_ref(&self, pointer: RawPointer) -> Result<Cow<'_, [u8]>, Error> {
        let bytes = self.heap_ref().read_raw_bytes(pointer)?;

        Ok(Cow::Owned(bytes))
    }

    /// Read raw bytes from a pointer to a bytes cell as one owned vector.
    pub fn read_raw_bytes(&self, pointer: RawPointer) -> Result<Vec<u8>, Error> {
        Ok(self.raw_bytes_ref(pointer)?.into_owned())
    }

    /// Read the raw byte length for one pointer.
    pub fn raw_byte_len(&self, pointer: RawPointer) -> Result<usize, Error> {
        self.heap_ref().raw_byte_len(pointer).map_err(Error::from)
    }

    /// Read raw packed values from one pointer.
    pub fn raw_values(&mut self, pointer: RawPointer) -> Result<Vec<Word>, Error> {
        let bytes = self
            .heap_ref()
            .read_raw_bytes(pointer)
            .map_err(Error::from)?;
        if bytes.len() % Word::BYTE_LEN != 0 {
            return Err(Error::InvalidHeapReference);
        }

        let mut values = Vec::with_capacity(bytes.len() / Word::BYTE_LEN);

        // decode each packed lane from the raw payload
        for window in bytes.chunks_exact(Word::BYTE_LEN) {
            let value = Word::from_byte_slice(window).ok_or(Error::InvalidHeapReference)?;
            let value = self.capture_value(value)?;

            values.push(value);
        }

        Ok(values)
    }

    /// Read one raw packed value by slot index.
    pub fn raw_value_at(&mut self, pointer: RawPointer, index: usize) -> Result<Word, Error> {
        let start = index
            .checked_mul(Word::BYTE_LEN)
            .ok_or(Error::InvalidHeapReference)?;
        let mut bytes = [0u8; Word::BYTE_LEN];

        // read the packed value lane without materializing the whole raw payload
        self.heap_ref()
            .read_raw_bytes_into(pointer, start, &mut bytes)
            .map_err(Error::from)?;

        let value = Word::from_byte_slice(&bytes).ok_or(Error::InvalidHeapReference)?;

        self.capture_value(value)
    }

    /// Read shared bytes from a pointer to one shared allocation as one owned vector.
    pub fn read_shared_bytes(&self, pointer: SharedRawPointer) -> Result<Vec<u8>, Error> {
        self.shared_ref()
            .read_raw_bytes(pointer)
            .map_err(Error::from)
    }

    /// Write raw bytes into a pointer to a bytes cell.
    pub fn write_raw_bytes(
        &mut self,
        pointer: RawPointer,
        bytes: &[u8],
    ) -> Result<RawPointer, Error> {
        self.heap()
            .replace_raw_bytes(pointer, bytes)
            .map_err(Error::from)
    }

    /// Write raw packed values into one pointer.
    pub fn write_raw_values(
        &mut self,
        pointer: RawPointer,
        values: &[Word],
    ) -> Result<RawPointer, Error> {
        let bytes = values
            .iter()
            .copied()
            .flat_map(Word::to_byte_array)
            .collect::<Vec<_>>();

        self.write_raw_bytes(pointer, &bytes)
    }

    /// Write one raw packed value into one pointer slot.
    pub fn write_raw_value(
        &mut self,
        pointer: RawPointer,
        index: usize,
        value: Word,
    ) -> Result<(), Error> {
        let start = index
            .checked_mul(Word::BYTE_LEN)
            .ok_or(Error::InvariantViolation {
                context: "raw value byte offset".to_string(),
            })?;

        self.heap()
            .set_raw_bytes(pointer, start, &value.to_byte_array())
            .map_err(Error::from)
    }

    /// Write one raw byte into one pointer slot.
    pub fn write_raw_byte(
        &mut self,
        pointer: RawPointer,
        index: usize,
        byte: u8,
    ) -> Result<(), Error> {
        self.heap()
            .set_raw_byte(pointer, index, byte)
            .map_err(Error::from)
    }

    /// Write shared bytes into a pointer to one shared allocation.
    pub fn write_shared_bytes(
        &mut self,
        pointer: SharedRawPointer,
        bytes: &[u8],
    ) -> Result<SharedRawPointer, Error> {
        let mapped_delta = self
            .shared()
            .raw_replace_mapped_byte_delta(pointer, bytes.len())?;
        self.shared_raw_budget()
            .check_mapped_byte_delta(mapped_delta)?;

        self.shared()
            .replace_raw_bytes(pointer, bytes)
            .map_err(Error::from)
    }

    /// Pin one local heap reference for the external call lifetime.
    pub(super) fn capture_heap_reference(
        &mut self,
        reference: HeapReference,
    ) -> Result<HeapReference, Error> {
        // empty payloads may carry one null heap reference
        if reference.is_null() {
            return Ok(reference);
        }

        // the same payload may still carry the original young reference bits
        if let Some(reference) = self.pin_scope.rewritten_heap_reference(reference) {
            return Ok(reference);
        }

        let pinned_reference = self.heap().pin_heap(reference).map_err(Error::from)?;

        // repeated captures in the same call should reuse the first pinned reference
        self.pin_scope
            .rewrite_heap_reference(reference, pinned_reference);
        self.pin_scope.push_heap_reference(pinned_reference);
        if let Some(ty) = self.heap_type(reference) {
            self.record_heap_type(pinned_reference, ty);
        }

        Ok(pinned_reference)
    }

    /// Capture one VM value in the external handle scope.
    pub(super) fn capture_value(&mut self, value: Word) -> Result<Word, Error> {
        Ok(value)
    }

    /// Release all heap pins captured for this call.
    pub(crate) fn release_pins(&mut self) -> Result<(), Error> {
        let heap_references = std::mem::take(&mut self.pin_scope.heap_references);

        // release every call scoped local heap pin
        for reference in heap_references.into_iter().rev() {
            self.heap().unpin_heap(reference).map_err(Error::from)?;
        }

        Ok(())
    }
}

impl Drop for ExternalCallContext<'_> {
    fn drop(&mut self) {
        let _ = self.release_pins();
    }
}

#[allow(clippy::mut_from_ref)]
impl<'call, 'ctx> ExternalReadContext<'call, 'ctx> {
    /// Return the external call context immutably.
    pub(super) fn context(&self) -> &ExternalCallContext<'ctx> {
        unsafe { &*self.context }
    }

    /// Return the external call context mutably.
    pub(super) fn context_mut(&self) -> &mut ExternalCallContext<'ctx> {
        unsafe { &mut *self.context }
    }

    /// Return one raw byte payload borrow or copy.
    pub fn raw_bytes_ref(&self, pointer: RawPointer) -> Result<Cow<'_, [u8]>, Error> {
        self.context().raw_bytes_ref(pointer)
    }

    /// Return one owned raw byte payload.
    pub fn read_raw_bytes(&self, pointer: RawPointer) -> Result<Vec<u8>, Error> {
        self.context().read_raw_bytes(pointer)
    }

    /// Return the raw byte length for one pointer.
    pub fn raw_byte_len(&self, pointer: RawPointer) -> Result<usize, Error> {
        self.context().raw_byte_len(pointer)
    }

    /// Return one raw packed value by slot index.
    pub fn raw_value_at(&self, pointer: RawPointer, index: usize) -> Result<Word, Error> {
        self.context_mut().raw_value_at(pointer, index)
    }

    /// Return one raw packed-value payload copy.
    pub fn raw_values(&self, pointer: RawPointer) -> Result<Vec<Word>, Error> {
        self.context_mut().raw_values(pointer)
    }

    /// Return one shared byte payload copy.
    pub fn read_shared_bytes(&self, pointer: SharedRawPointer) -> Result<Vec<u8>, Error> {
        self.context().read_shared_bytes(pointer)
    }

    /// Return one heap packed-value payload copy.
    pub fn heap_values(&self, reference: HeapReference) -> Result<Vec<Word>, Error> {
        self.context_mut().heap_values(reference)
    }

    /// Return one heap packed value by index.
    pub fn heap_value_at(&self, reference: HeapReference, index: usize) -> Result<Word, Error> {
        self.context_mut().heap_value_at(reference, index)
    }
}

#[allow(clippy::mut_from_ref)]
impl<'call, 'ctx> ExternalWriteContext<'call, 'ctx> {
    /// Return the external call context mutably.
    pub(super) fn context_mut(&self) -> &mut ExternalCallContext<'ctx> {
        unsafe { &mut *self.context }
    }

    /// Return the raw byte length for one pointer.
    pub fn raw_byte_len(&self, pointer: RawPointer) -> Result<usize, Error> {
        self.context_mut().raw_byte_len(pointer)
    }

    /// Allocate one raw byte buffer.
    pub fn allocate_raw_bytes(&mut self, bytes: &[u8]) -> Result<RawPointer, Error> {
        self.context_mut().allocate_raw_bytes(bytes)
    }

    /// Allocate one zeroed raw byte buffer.
    pub fn allocate_zeroed_raw_bytes(&mut self, byte_len: usize) -> Result<RawPointer, Error> {
        self.context_mut().allocate_zeroed_raw_bytes(byte_len)
    }

    /// Allocate one raw packed-value buffer.
    pub fn allocate_raw_values(&mut self, values: Vec<Word>) -> Result<RawPointer, Error> {
        self.context_mut().allocate_raw_values(values)
    }

    /// Allocate one zeroed raw packed-value buffer.
    pub fn allocate_raw_value_slots(&mut self, slot_count: usize) -> Result<RawPointer, Error> {
        self.context_mut().allocate_raw_value_slots(slot_count)
    }

    /// Allocate one shared byte region.
    pub fn allocate_shared_bytes(&mut self, bytes: &[u8]) -> Result<SharedRawPointer, Error> {
        self.context_mut().allocate_shared_bytes(bytes)
    }

    /// Write one raw byte payload.
    pub fn write_raw_bytes(
        &mut self,
        pointer: RawPointer,
        bytes: &[u8],
    ) -> Result<RawPointer, Error> {
        self.context_mut().write_raw_bytes(pointer, bytes)
    }

    /// Write one raw packed-value payload.
    pub fn write_raw_values(
        &mut self,
        pointer: RawPointer,
        values: &[Word],
    ) -> Result<RawPointer, Error> {
        self.context_mut().write_raw_values(pointer, values)
    }

    /// Write one raw packed value.
    pub fn write_raw_value(
        &mut self,
        pointer: RawPointer,
        index: usize,
        value: Word,
    ) -> Result<(), Error> {
        self.context_mut().write_raw_value(pointer, index, value)
    }

    /// Write one raw byte into one pointer slot.
    pub fn write_raw_byte(
        &mut self,
        pointer: RawPointer,
        index: usize,
        byte: u8,
    ) -> Result<(), Error> {
        self.context_mut().write_raw_byte(pointer, index, byte)
    }

    /// Write one shared byte payload.
    pub fn write_shared_bytes(
        &mut self,
        pointer: SharedRawPointer,
        bytes: &[u8],
    ) -> Result<SharedRawPointer, Error> {
        self.context_mut().write_shared_bytes(pointer, bytes)
    }

    /// Allocate one heap packed-value buffer.
    pub fn allocate_heap_value_slots(&mut self, slot_count: usize) -> Result<HeapReference, Error> {
        self.context_mut().allocate_heap_value_slots(slot_count)
    }

    /// Write one heap packed value.
    pub fn write_heap_value(
        &mut self,
        reference: HeapReference,
        index: usize,
        value: Word,
    ) -> Result<(), Error> {
        self.context_mut().write_heap_value(reference, index, value)
    }
}

impl fmt::Debug for ExternalCallContext<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ExternalCallContext").finish()
    }
}
