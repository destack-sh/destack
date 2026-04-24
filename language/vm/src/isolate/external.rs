use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::marker::PhantomData;
use std::sync::Arc;

use crate::diagnostic::Error;
use crate::execute::storage::{decode_raw_value, encode_raw_value};
use crate::module::{Layout, Module, repr_type};
use crate::{SharedHeap, Value};
use destack_heap::{
    Heap, HeapReference, Payload, RawPointer, SharedHeapReference, SharedRawBudget,
    SharedRawLimits, SharedRawPointer,
};
use destack_mir::{self as mir, LayoutKind, ReferenceMap};

// FUGU #Architecture: remove generated ABI shims once VM and native ABI share normal MIR payloads
/// One typed member inside one aggregate payload.
#[derive(Clone, Copy, Debug)]
struct AggregateMember {
    /// The stored value type.
    ty: mir::LocalNodeId<mir::Type>,
    /// The byte offset of the value payload.
    offset: usize,
}

/// One borrowed UTF-8 VM string view.
#[derive(Clone, Debug)]
pub struct StringRef<'a> {
    /// The decoded UTF-8 contents.
    value: Cow<'a, str>,
}

impl<'a> StringRef<'a> {
    /// Return this string as UTF-8 text.
    pub fn as_str(&self) -> &str {
        &self.value
    }
}

impl fmt::Display for StringRef<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// ABI handle for a VM value carrying string bytes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StringHandle {
    /// The VM value for this string payload.
    value: Value,
}

impl StringHandle {
    /// Create a string handle from one VM value.
    pub const fn new(value: Value) -> Self {
        Self { value }
    }

    /// Return the wrapped VM value.
    pub const fn value(self) -> Value {
        self.value
    }
}

/// One VM aggregate value view.
#[derive(Debug)]
pub struct VmValueRef<'call, 'ctx> {
    /// The active external call context.
    context: *mut ExternalCallContext<'ctx>,
    /// The MIR type for this value when known.
    ty: Option<mir::LocalNodeId<mir::Type>>,
    /// The shared payload bytes.
    bytes: Arc<[u8]>,
    /// The first byte of this view inside the payload.
    start: usize,
    /// The byte length of this view.
    byte_len: usize,
    /// The lifetime marker for the call context.
    _marker: PhantomData<&'call ExternalCallContext<'ctx>>,
}

impl<'call, 'ctx> VmValueRef<'call, 'ctx> {
    /// Return the active external call context mutably.
    fn context_mut(&self) -> &'call mut ExternalCallContext<'ctx> {
        unsafe { &mut *self.context }
    }

    /// Return the active external call context immutably.
    fn context_ref(&self) -> &'call ExternalCallContext<'ctx> {
        unsafe { &*self.context }
    }

    /// Return the byte slice for this view.
    fn bytes(&self) -> &[u8] {
        &self.bytes[self.start..self.start + self.byte_len]
    }

    /// Return one checked byte range relative to this view.
    fn byte_window(&self, offset: usize, byte_len: usize) -> Result<&[u8], Error> {
        let end = offset
            .checked_add(byte_len)
            .ok_or(Error::InvalidHeapReference)?;

        self.bytes()
            .get(offset..end)
            .ok_or(Error::InvalidHeapReference)
    }

    /// Return the member layouts for the known MIR type.
    fn members(&self) -> Result<Vec<AggregateMember>, Error> {
        let ty = self.ty.ok_or(Error::InvalidHeapReference)?;
        let layout = self.context_ref().layout(ty)?;

        aggregate_members(layout)
    }

    /// Return the semantic component count for this value.
    pub fn component_count(&self) -> usize {
        if let Ok(members) = self.members() {
            return members.len();
        }

        self.byte_len / Value::BYTE_LEN
    }

    /// Return one nested VM value view for one component.
    pub fn component_ref(&self, index: u32) -> Result<Self, Error> {
        let members = self.members()?;
        let member = members
            .get(index as usize)
            .copied()
            .ok_or(Error::InvalidHeapReference)?;
        let layout = self.context_ref().layout(member.ty)?;
        if layout.is_scalar() {
            return Err(Error::InvalidHeapReference);
        }

        let start = self
            .start
            .checked_add(member.offset)
            .ok_or(Error::InvalidHeapReference)?;
        let end = start
            .checked_add(layout.byte_len)
            .ok_or(Error::InvalidHeapReference)?;
        if end > self.bytes.len() {
            return Err(Error::InvalidHeapReference);
        }

        Ok(Self {
            context: self.context,
            ty: Some(member.ty),
            bytes: self.bytes.clone(),
            start,
            byte_len: layout.byte_len,
            _marker: PhantomData,
        })
    }

    /// Decode one component value from this view.
    pub fn component_value(&self, index: u32) -> Result<Value, Error> {
        if let Ok(members) = self.members() {
            let member = members
                .get(index as usize)
                .copied()
                .ok_or(Error::InvalidHeapReference)?;
            let layout = self.context_ref().layout(member.ty)?;
            let bytes = self.byte_window(member.offset, layout.byte_len)?.to_vec();

            return self
                .context_mut()
                .materialize_value_from_bytes(member.ty, &bytes);
        }

        let start = (index as usize)
            .checked_mul(Value::BYTE_LEN)
            .ok_or(Error::InvalidHeapReference)?;
        let bytes = self.byte_window(start, Value::BYTE_LEN)?;
        let value = Value::from_byte_slice(bytes).ok_or(Error::InvalidHeapReference)?;

        self.context_mut().capture_value(value)
    }
}

/// One deferred VM value builder.
#[derive(Debug)]
pub struct VmValueBuilder<'ctx> {
    /// The active external call context.
    context: *mut ExternalCallContext<'ctx>,
    /// The target MIR type.
    ty: mir::LocalNodeId<mir::Type>,
    /// The staged field values.
    components: Vec<Option<Value>>,
    /// The lifetime marker for the call context.
    _marker: PhantomData<&'ctx ExternalCallContext<'ctx>>,
}

impl VmValueBuilder<'_> {
    /// Return the expected field count for this builder.
    pub fn component_count(&self) -> usize {
        self.components.len()
    }

    /// Write one field value.
    pub fn write_component(&mut self, index: u32, value: Value) -> Result<(), Error> {
        let Some(component) = self.components.get_mut(index as usize) else {
            return Err(Error::InvalidHeapReference);
        };

        *component = Some(value);

        Ok(())
    }

    /// Finish the value and allocate its typed heap payload.
    pub fn finish(self) -> Result<Value, Error> {
        let context = unsafe { &mut *self.context };
        let mut values = Vec::with_capacity(self.components.len());

        for component in self.components {
            let Some(value) = component else {
                return Err(Error::TypeMismatch {
                    expected: "initialized aggregate field".to_string(),
                    actual: "missing aggregate field".to_string(),
                });
            };

            values.push(value);
        }

        context.materialize_heap_value(self.ty, values)
    }
}

/// Align one byte offset up to the requested alignment.
#[inline(always)]
fn align_offset(offset: usize, alignment: usize) -> usize {
    if alignment <= 1 {
        return offset;
    }

    let remainder = offset % alignment;
    if remainder == 0 {
        offset
    } else {
        offset + (alignment - remainder)
    }
}

/// Return the typed value byte ranges for one aggregate layout.
fn aggregate_members(layout: &Layout) -> Result<Vec<AggregateMember>, Error> {
    if let Some(field_count) = layout.field_count() {
        let mut values = Vec::with_capacity(field_count);

        // collect fields in source order
        for index in 0..field_count {
            let field = layout
                .field(index as u32)
                .ok_or(Error::InvalidInstruction)?;
            values.push(AggregateMember {
                ty: field.ty,
                offset: field.offset,
            });
        }

        return Ok(values);
    }

    let element = layout.element().ok_or(Error::InvalidHeapReference)?;
    let element_count = layout.element_count().ok_or(Error::InvalidHeapReference)?;
    let mut values = Vec::with_capacity(element_count);

    // collect repeated elements in payload order
    for index in 0..element_count {
        let offset = element
            .stride
            .checked_mul(index)
            .ok_or(Error::InvalidHeapReference)?;
        values.push(AggregateMember {
            ty: element.ty,
            offset,
        });
    }

    Ok(values)
}

/// Handler invoked by the VM when calling an external function.
pub trait ExternalHandler:
    for<'ctx> Fn(&mut ExternalCallContext<'ctx>, &[Value]) -> Result<Value, Error> + Send + Sync
{
}

impl<T> ExternalHandler for T where
    T: for<'ctx> Fn(&mut ExternalCallContext<'ctx>, &[Value]) -> Result<Value, Error> + Send + Sync
{
}

/// Boxed external handler type.
pub type ExternalFn = Arc<dyn ExternalHandler>;

/// Runtime call context with restricted access to isolate state.
pub struct ExternalCallContext<'ctx> {
    /// The immutable module metadata for this isolate.
    module: &'ctx Module,
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
    context: *mut ExternalCallContext<'ctx>,
    /// The lifetime marker for the call context.
    _marker: PhantomData<&'call mut ExternalCallContext<'ctx>>,
}

/// Mutable external VM call capability.
#[derive(Debug)]
pub struct ExternalWriteContext<'call, 'ctx> {
    /// The owning external call context.
    context: *mut ExternalCallContext<'ctx>,
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

    /// Return the callable box payload layout.
    fn callable_payload_layout(&self) -> (usize, usize, usize) {
        let pointer_bytes = self.module.tree.pointer_bytes() as usize;
        let function_offset = 0usize;
        let environment_offset = align_offset(pointer_bytes, pointer_bytes);
        let byte_len = environment_offset + pointer_bytes;

        (function_offset, environment_offset, byte_len)
    }

    /// Create one external call context.
    pub(crate) fn new(
        module: &'ctx Module,
        heap: &'ctx mut Heap,
        shared: &'ctx SharedHeap,
        shared_raw_limits: SharedRawLimits,
    ) -> Self {
        Self {
            module,
            heap: heap as *mut Heap,
            shared: shared as *const SharedHeap,
            shared_raw_limits,
            pin_scope: PinScope::default(),
            heap_type_by_reference: BTreeMap::new(),
        }
    }

    /// Return one compiled layout by type.
    fn layout(&self, ty: mir::LocalNodeId<mir::Type>) -> Result<&Layout, Error> {
        self.module.layout(ty).ok_or(Error::InvalidInstruction)
    }

    /// Return one named MIR type from module metadata.
    fn named_type(&self, name: &str) -> Result<mir::LocalNodeId<mir::Type>, Error> {
        self.module
            .type_by_display_name(name)
            .ok_or_else(|| Error::TypeMismatch {
                expected: format!("MIR type named {name}"),
                actual: "missing type".to_string(),
            })
    }

    /// Track the MIR type for one heap reference allocated through this context.
    fn record_heap_type(&mut self, reference: HeapReference, ty: mir::LocalNodeId<mir::Type>) {
        if reference.is_null() {
            return;
        }

        self.heap_type_by_reference.insert(reference, ty);
    }

    /// Return the tracked MIR type for one heap reference.
    fn heap_type(&self, reference: HeapReference) -> Option<mir::LocalNodeId<mir::Type>> {
        self.heap_type_by_reference.get(&reference).copied()
    }

    /// Materialize one typed value from one storage byte slice.
    fn materialize_value_from_bytes(
        &mut self,
        ty: mir::LocalNodeId<mir::Type>,
        bytes: &[u8],
    ) -> Result<Value, Error> {
        if self.layout(ty)?.is_scalar() {
            return decode_raw_value(&self.module.tree, ty, bytes);
        }

        let members = aggregate_members(self.layout(ty)?)?;
        let mut values = Vec::with_capacity(members.len());

        for member in members {
            let layout = self.layout(member.ty)?;
            let end = member
                .offset
                .checked_add(layout.byte_len)
                .ok_or(Error::InvalidHeapReference)?;
            let bytes = bytes
                .get(member.offset..end)
                .ok_or(Error::InvalidHeapReference)?;
            let value = self.materialize_value_from_bytes(member.ty, bytes)?;
            values.push(value);
        }

        self.materialize_heap_value(ty, values)
    }

    /// Build one named value builder from the module type table.
    fn begin_named_storage_value_builder(
        &mut self,
        type_name: &str,
    ) -> Result<VmValueBuilder<'ctx>, Error> {
        let ty = self.named_type(type_name)?;
        let field_count = aggregate_members(self.layout(ty)?)?.len();

        Ok(VmValueBuilder {
            context: self as *mut Self,
            ty,
            components: vec![None; field_count],
            _marker: PhantomData,
        })
    }

    /// Write one value directly into one heap payload.
    fn write_heap_value_field(
        &mut self,
        handle: HeapReference,
        start: usize,
        ty: mir::LocalNodeId<mir::Type>,
        value: Value,
    ) -> Result<(), Error> {
        // write scalars directly into the target payload
        if self.layout(ty)?.is_scalar() {
            let bytes = encode_raw_value(&self.module.tree, ty, value)?;
            self.write_heap_bytes(handle, start, &bytes)?;

            return Ok(());
        }

        let source = value
            .as_heap_reference()
            .ok_or_else(|| Error::TypeMismatch {
                expected: "heap aggregate".to_string(),
                actual: format!("{value:?}"),
            })?;
        let bytes = self.heap_ref().read_heap_bytes(source)?;

        if bytes.len() != self.layout(ty)?.byte_len {
            return Err(Error::InvalidHeapReference);
        }

        self.write_heap_bytes(handle, start, &bytes)
    }

    /// Return one encoded callable payload from function and environment values.
    fn callable_payload(&self, values: Vec<Value>) -> Result<Vec<u8>, Error> {
        if values.len() != 2 {
            return Err(Error::TypeMismatch {
                expected: "2 callable values".to_string(),
                actual: format!("{} callable values", values.len()),
            });
        }

        let (function_offset, environment_offset, _) = self.callable_payload_layout();
        let function = values[0]
            .as_function_pointer()
            .ok_or_else(|| Error::TypeMismatch {
                expected: "function pointer".to_string(),
                actual: format!("{:?}", values[0]),
            })?;
        let environment = values[1]
            .as_heap_reference()
            .ok_or_else(|| Error::TypeMismatch {
                expected: "heap reference".to_string(),
                actual: format!("{:?}", values[1]),
            })?;
        let function_bytes = (function.id as u64).to_le_bytes();
        let environment_bytes = (environment.bits() as u64).to_le_bytes();
        let pointer_bytes = self.module.tree.pointer_bytes() as usize;
        let (_, _, byte_len) = self.callable_payload_layout();
        let mut bytes = vec![0; byte_len];

        bytes[function_offset..function_offset + pointer_bytes]
            .copy_from_slice(&function_bytes[..pointer_bytes]);
        bytes[environment_offset..environment_offset + pointer_bytes]
            .copy_from_slice(&environment_bytes[..pointer_bytes]);

        Ok(bytes)
    }

    /// Materialize one callable value on the heap.
    fn materialize_callable(
        &mut self,
        ty: mir::LocalNodeId<mir::Type>,
        values: Vec<Value>,
    ) -> Result<Value, Error> {
        let layout_id = self
            .module
            .layout_id_for_type(ty)
            .ok_or(Error::InvalidInstruction)?;
        let bytes = self.callable_payload(values)?;
        let handle = self
            .heap()
            .allocate(layout_id, Payload::Bytes(&bytes))
            .map_err(Error::from)?;
        let handle = self.capture_heap_reference(handle)?;
        self.record_heap_type(handle, ty);

        Ok(Value::heap_reference(handle))
    }

    /// Materialize one typed aggregate value on the heap.
    pub(crate) fn materialize_heap_value(
        &mut self,
        ty: mir::LocalNodeId<mir::Type>,
        values: Vec<Value>,
    ) -> Result<Value, Error> {
        if matches!(
            self.module.tree.get(repr_type(&self.module.tree, ty)),
            mir::Type::Callable { .. }
        ) {
            return self.materialize_callable(ty, values);
        }

        let layout_id = self
            .module
            .layout_id_for_type(ty)
            .ok_or(Error::InvalidInstruction)?;
        let handle = self
            .heap()
            .allocate(layout_id, Payload::Zeroed)
            .map_err(Error::from)?;
        let handle = self.capture_heap_reference(handle)?;
        self.record_heap_type(handle, ty);
        let value_layouts = aggregate_members(self.layout(ty)?)?;

        if values.len() != value_layouts.len() {
            return Err(Error::TypeMismatch {
                expected: format!("{} aggregate values", value_layouts.len()),
                actual: format!("{} aggregate values", values.len()),
            });
        }

        // write each value directly into the aggregate payload
        for (value_layout, value) in value_layouts.into_iter().zip(values) {
            self.write_heap_value_field(handle, value_layout.offset, value_layout.ty, value)?;
        }

        Ok(Value::heap_reference(handle))
    }

    /// Borrow the local heap.
    fn heap(&mut self) -> &mut Heap {
        unsafe { &mut *self.heap }
    }

    /// Write one managed byte range through the external mutator path.
    fn write_heap_bytes(
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
    fn heap_ref(&self) -> &Heap {
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
    pub fn allocate_raw_values(&mut self, values: Vec<Value>) -> Result<RawPointer, Error> {
        let bytes = values
            .into_iter()
            .flat_map(Value::to_byte_array)
            .collect::<Vec<_>>();

        self.allocate_raw_bytes(&bytes)
    }

    /// Allocate one zeroed raw packed-value buffer and return its pointer.
    pub fn allocate_raw_value_slots(&mut self, slot_count: usize) -> Result<RawPointer, Error> {
        let byte_len =
            slot_count
                .checked_mul(Value::BYTE_LEN)
                .ok_or(Error::InvariantViolation {
                    context: "raw value buffer byte length".to_string(),
                })?;

        self.allocate_zeroed_raw_bytes(byte_len)
    }

    /// Allocate a heap packed-value buffer.
    pub fn allocate_heap_value_slots(&mut self, slot_count: usize) -> Result<HeapReference, Error> {
        let byte_len =
            slot_count
                .checked_mul(Value::BYTE_LEN)
                .ok_or(Error::InvariantViolation {
                    context: "heap value buffer byte length".to_string(),
                })?;
        let layout_id = self.heap().register_layout(mir::Layout {
            kind: LayoutKind::Struct,
            size: byte_len as u32,
            alignment: std::mem::align_of::<Value>() as u32,
            reference_map: ReferenceMap::None,
            fields: Vec::new(),
        });

        self.heap()
            .allocate(layout_id, Payload::Zeroed)
            .map_err(Error::from)
    }

    /// Read heap packed values from one heap reference.
    pub fn heap_values(&mut self, reference: HeapReference) -> Result<Vec<Value>, Error> {
        let bytes = self.heap_ref().read_heap_bytes(reference)?;
        if bytes.len() % Value::BYTE_LEN != 0 {
            return Err(Error::InvalidHeapReference);
        }

        let mut values = Vec::with_capacity(bytes.len() / Value::BYTE_LEN);

        for bytes in bytes.chunks_exact(Value::BYTE_LEN) {
            let value = Value::from_byte_slice(bytes).ok_or(Error::InvalidHeapReference)?;
            let value = self.capture_value(value)?;
            values.push(value);
        }

        Ok(values)
    }

    /// Read one heap packed value by index.
    pub fn heap_value_at(
        &mut self,
        reference: HeapReference,
        index: usize,
    ) -> Result<Value, Error> {
        let start = index
            .checked_mul(Value::BYTE_LEN)
            .ok_or(Error::InvalidHeapReference)?;
        let mut bytes = [0u8; Value::BYTE_LEN];

        self.heap_ref()
            .read_heap_bytes_into(reference, start, &mut bytes)?;
        let value = Value::from_byte_slice(&bytes).ok_or(Error::InvalidHeapReference)?;

        self.capture_value(value)
    }

    /// Write one heap packed value by index.
    pub fn write_heap_value(
        &mut self,
        reference: HeapReference,
        index: usize,
        value: Value,
    ) -> Result<(), Error> {
        let start = index
            .checked_mul(Value::BYTE_LEN)
            .ok_or(Error::InvalidHeapReference)?;

        self.write_heap_bytes(reference, start, &value.to_byte_array())
    }

    /// Store UTF-8 bytes in raw VM storage and return the value.
    pub fn intern_string(&mut self, value: &str) -> Result<Value, Error> {
        let pointer = self.allocate_raw_bytes(value.as_bytes())?;

        Ok(Value::raw_pointer(pointer))
    }

    /// Store UTF-8 bytes in raw VM storage and return a handle.
    pub fn string_handle(&mut self, value: &str) -> Result<StringHandle, Error> {
        let value = self.intern_string(value)?;

        Ok(StringHandle::new(value))
    }

    /// Validate one VM value as a string handle.
    pub fn string_handle_from_value(&self, value: Value) -> Result<StringHandle, Error> {
        if value.as_raw_pointer().is_some() {
            return Ok(StringHandle::new(value));
        }

        Err(Error::TypeMismatch {
            expected: "string byte storage".to_string(),
            actual: format!("{value:?}"),
        })
    }

    /// Read one UTF-8 string view by handle.
    pub fn string_ref(&self, value: StringHandle) -> Result<StringRef<'_>, Error> {
        let value = value.value();
        let pointer = value.as_raw_pointer().ok_or_else(|| Error::TypeMismatch {
            expected: "string byte storage".to_string(),
            actual: format!("{value:?}"),
        })?;
        let bytes = self.raw_bytes_ref(pointer)?.into_owned();
        let value = String::from_utf8(bytes).map_err(|error| Error::TypeMismatch {
            expected: "valid UTF-8 string".to_string(),
            actual: error.to_string(),
        })?;

        Ok(StringRef {
            value: Cow::Owned(value),
        })
    }

    /// Read one UTF-8 string value.
    pub fn string_value(&self, value: Value) -> Result<String, Error> {
        let handle = self.string_handle_from_value(value)?;

        Ok(self.string_ref(handle)?.to_string())
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
    pub fn raw_values(&mut self, pointer: RawPointer) -> Result<Vec<Value>, Error> {
        let bytes = self
            .heap_ref()
            .read_raw_bytes(pointer)
            .map_err(Error::from)?;
        if bytes.len() % Value::BYTE_LEN != 0 {
            return Err(Error::InvalidHeapReference);
        }

        let mut values = Vec::with_capacity(bytes.len() / Value::BYTE_LEN);

        // decode each packed lane from the raw payload
        for window in bytes.chunks_exact(Value::BYTE_LEN) {
            let value = Value::from_byte_slice(window).ok_or(Error::InvalidHeapReference)?;
            let value = self.capture_value(value)?;

            values.push(value);
        }

        Ok(values)
    }

    /// Read one raw packed value by slot index.
    pub fn raw_value_at(&mut self, pointer: RawPointer, index: usize) -> Result<Value, Error> {
        let start = index
            .checked_mul(Value::BYTE_LEN)
            .ok_or(Error::InvalidHeapReference)?;
        let mut bytes = [0u8; Value::BYTE_LEN];

        // read the packed value lane without materializing the whole raw payload
        self.heap_ref()
            .read_raw_bytes_into(pointer, start, &mut bytes)
            .map_err(Error::from)?;

        let value = Value::from_byte_slice(&bytes).ok_or(Error::InvalidHeapReference)?;

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
        values: &[Value],
    ) -> Result<RawPointer, Error> {
        let bytes = values
            .iter()
            .copied()
            .flat_map(Value::to_byte_array)
            .collect::<Vec<_>>();

        self.write_raw_bytes(pointer, &bytes)
    }

    /// Write one raw packed value into one pointer slot.
    pub fn write_raw_value(
        &mut self,
        pointer: RawPointer,
        index: usize,
        value: Value,
    ) -> Result<(), Error> {
        let start = index
            .checked_mul(Value::BYTE_LEN)
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
    fn capture_heap_reference(&mut self, reference: HeapReference) -> Result<HeapReference, Error> {
        // empty aggregate payloads may carry one null heap reference
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

    /// Preserve one shared heap reference across the external call.
    fn capture_shared_heap_reference(
        &mut self,
        reference: SharedHeapReference,
    ) -> Result<SharedHeapReference, Error> {
        if reference.is_null() {
            return Ok(reference);
        }

        Ok(reference)
    }

    /// Capture one VM value in the external handle scope.
    fn capture_value(&mut self, value: Value) -> Result<Value, Error> {
        if let Some(reference) = value.as_heap_reference() {
            let reference = self.capture_heap_reference(reference)?;

            return Ok(Value::heap_reference(reference).with_reference_meta(value.reference_meta()));
        }

        if let Some(reference) = value.as_shared_heap_reference() {
            let reference = self.capture_shared_heap_reference(reference)?;

            return Ok(
                Value::shared_heap_reference(reference).with_reference_meta(value.reference_meta())
            );
        }

        Ok(value)
    }
}

impl Drop for ExternalCallContext<'_> {
    fn drop(&mut self) {
        let heap_references = std::mem::take(&mut self.pin_scope.heap_references);

        // release every call scoped local heap pin
        for reference in heap_references.into_iter().rev() {
            if let Err(error) = self.heap().unpin_heap(reference) {
                panic!("external call pin release failed for {reference:?}: {error:?}");
            }
        }
    }
}

impl<'call, 'ctx> ExternalReadContext<'call, 'ctx> {
    /// Return the external call context immutably.
    fn context(&self) -> &ExternalCallContext<'ctx> {
        unsafe { &*self.context }
    }

    /// Return the external call context mutably.
    fn context_mut(&self) -> &mut ExternalCallContext<'ctx> {
        unsafe { &mut *self.context }
    }

    /// Return one cached VM value view.
    pub fn value_ref(&self, value: Value) -> Result<VmValueRef<'call, 'ctx>, Error> {
        let reference = value
            .as_heap_reference()
            .ok_or_else(|| Error::TypeMismatch {
                expected: "heap aggregate".to_string(),
                actual: format!("{value:?}"),
            })?;
        let bytes = self.context().heap_ref().read_heap_bytes(reference)?;
        let ty = self.context().heap_type(reference);
        let byte_len = bytes.len();

        Ok(VmValueRef {
            context: self.context,
            ty,
            bytes: Arc::<[u8]>::from(bytes),
            start: 0,
            byte_len,
            _marker: PhantomData,
        })
    }

    /// Return one VM string handle from one value.
    pub fn string_handle_from_value(&self, value: Value) -> Result<StringHandle, Error> {
        self.context().string_handle_from_value(value)
    }

    /// Return one borrowed VM string by handle.
    pub fn string_ref(&self, value: StringHandle) -> Result<StringRef<'call>, Error> {
        let value = self.context().string_ref(value)?;

        Ok(StringRef {
            value: Cow::Owned(value.as_str().to_string()),
        })
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
    pub fn raw_value_at(&self, pointer: RawPointer, index: usize) -> Result<Value, Error> {
        self.context_mut().raw_value_at(pointer, index)
    }

    /// Return one raw packed-value payload copy.
    pub fn raw_values(&self, pointer: RawPointer) -> Result<Vec<Value>, Error> {
        self.context_mut().raw_values(pointer)
    }

    /// Return one shared byte payload copy.
    pub fn read_shared_bytes(&self, pointer: SharedRawPointer) -> Result<Vec<u8>, Error> {
        self.context().read_shared_bytes(pointer)
    }

    /// Return one heap packed-value payload copy.
    pub fn heap_values(&self, reference: HeapReference) -> Result<Vec<Value>, Error> {
        self.context_mut().heap_values(reference)
    }

    /// Return one heap packed value by index.
    pub fn heap_value_at(&self, reference: HeapReference, index: usize) -> Result<Value, Error> {
        self.context_mut().heap_value_at(reference, index)
    }
}

impl<'call, 'ctx> ExternalWriteContext<'call, 'ctx> {
    /// Return the external call context mutably.
    fn context_mut(&self) -> &mut ExternalCallContext<'ctx> {
        unsafe { &mut *self.context }
    }

    /// Return the raw byte length for one pointer.
    pub fn raw_byte_len(&self, pointer: RawPointer) -> Result<usize, Error> {
        self.context_mut().raw_byte_len(pointer)
    }

    /// Return one managed value builder by runtime storage name.
    pub fn begin_named_storage_value_builder(
        &mut self,
        storage_type: &str,
    ) -> Result<VmValueBuilder<'ctx>, Error> {
        self.context_mut()
            .begin_named_storage_value_builder(storage_type)
    }

    /// Materialize one builtin slice value.
    pub fn materialize_builtin_slice_value(
        &mut self,
        data: Value,
        len: usize,
    ) -> Result<Value, Error> {
        let len = Value::uint(len as u64, usize::BITS as u8);
        self.materialize_named_storage_value("Slice", vec![data, len])
    }

    /// Materialize one builtin array value.
    pub fn materialize_builtin_array_value(
        &mut self,
        len: usize,
        capacity: usize,
        data: Value,
    ) -> Result<Value, Error> {
        let len = Value::uint(len as u64, usize::BITS as u8);
        let capacity = Value::uint(capacity as u64, usize::BITS as u8);

        self.materialize_named_storage_value("Array", vec![data, len, capacity])
    }

    /// Materialize one named runtime storage value.
    pub fn materialize_named_storage_value(
        &mut self,
        storage_type: &str,
        values: Vec<Value>,
    ) -> Result<Value, Error> {
        let ty = self.context_mut().named_type(storage_type)?;

        self.context_mut().materialize_heap_value(ty, values)
    }

    /// Materialize one named runtime storage value from one fixed-size array.
    pub fn materialize_named_storage_value_array<const N: usize>(
        &mut self,
        storage_type: &str,
        values: [Value; N],
    ) -> Result<Value, Error> {
        self.materialize_named_storage_value(storage_type, values.to_vec())
    }

    /// Intern one string into VM ABI storage.
    pub fn intern_string(&mut self, value: &str) -> Result<Value, Error> {
        self.context_mut().intern_string(value)
    }

    /// Return one VM string handle for one string.
    pub fn string_handle(&mut self, value: &str) -> Result<StringHandle, Error> {
        self.context_mut().string_handle(value)
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
    pub fn allocate_raw_values(&mut self, values: Vec<Value>) -> Result<RawPointer, Error> {
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
        values: &[Value],
    ) -> Result<RawPointer, Error> {
        self.context_mut().write_raw_values(pointer, values)
    }

    /// Write one raw packed value.
    pub fn write_raw_value(
        &mut self,
        pointer: RawPointer,
        index: usize,
        value: Value,
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
        value: Value,
    ) -> Result<(), Error> {
        self.context_mut().write_heap_value(reference, index, value)
    }
}

impl fmt::Debug for ExternalCallContext<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ExternalCallContext").finish()
    }
}
