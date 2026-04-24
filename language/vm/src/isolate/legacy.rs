use std::borrow::Cow;
use std::fmt;
use std::marker::PhantomData;
use std::sync::Arc;

use crate::Value;
use crate::diagnostic::Error;
use crate::execute::storage::{decode_raw_value, encode_raw_value};
use crate::module::{Layout, repr_type};
use destack_heap::{HeapReference, Payload};
use destack_mir as mir;

use super::{ExternalCallContext, ExternalReadContext, ExternalWriteContext};

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

    /// Return the semantic field count for this value.
    pub fn field_count(&self) -> usize {
        if let Ok(members) = self.members() {
            return members.len();
        }

        self.byte_len / Value::BYTE_LEN
    }

    /// Return one nested VM value view for one field.
    pub fn field_ref(&self, index: u32) -> Result<Self, Error> {
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

    /// Decode one field value from this view.
    pub fn field_value(&self, index: u32) -> Result<Value, Error> {
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
    fields: Vec<Option<Value>>,
    /// The lifetime marker for the call context.
    _marker: PhantomData<&'ctx ExternalCallContext<'ctx>>,
}

impl VmValueBuilder<'_> {
    /// Return the expected field count for this builder.
    pub fn field_count(&self) -> usize {
        self.fields.len()
    }

    /// Write one field value.
    pub fn write_field(&mut self, index: u32, value: Value) -> Result<(), Error> {
        let Some(field) = self.fields.get_mut(index as usize) else {
            return Err(Error::InvalidHeapReference);
        };

        *field = Some(value);

        Ok(())
    }

    /// Finish the value and allocate its typed heap payload.
    pub fn finish(self) -> Result<Value, Error> {
        let context = unsafe { &mut *self.context };
        let mut values = Vec::with_capacity(self.fields.len());

        for field in self.fields {
            let Some(value) = field else {
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

impl<'ctx> ExternalCallContext<'ctx> {
    /// Return one named MIR type from module metadata.
    fn named_type(&self, name: &str) -> Result<mir::LocalNodeId<mir::Type>, Error> {
        self.module()
            .type_by_display_name(name)
            .ok_or_else(|| Error::TypeMismatch {
                expected: format!("MIR type named {name}"),
                actual: "missing type".to_string(),
            })
    }

    /// Materialize one typed value from one storage byte slice.
    fn materialize_value_from_bytes(
        &mut self,
        ty: mir::LocalNodeId<mir::Type>,
        bytes: &[u8],
    ) -> Result<Value, Error> {
        if self.layout(ty)?.is_scalar() {
            return decode_raw_value(&self.module().tree, ty, bytes);
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
    fn begin_named_aggregate_builder(
        &mut self,
        type_name: &str,
    ) -> Result<VmValueBuilder<'ctx>, Error> {
        let ty = self.named_type(type_name)?;
        let field_count = aggregate_members(self.layout(ty)?)?.len();

        Ok(VmValueBuilder {
            context: self as *mut Self,
            ty,
            fields: vec![None; field_count],
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
            let bytes = encode_raw_value(&self.module().tree, ty, value)?;
            self.write_heap_bytes(handle, start, &bytes)?;

            return Ok(());
        }

        let source = value
            .as_heap_reference()
            .ok_or_else(|| Error::TypeMismatch {
                expected: "heap aggregate".to_string(),
                actual: format!("{value:?}"),
            })?;
        let byte_len = self.layout(ty)?.byte_len;
        let mut bytes = vec![0u8; byte_len];
        self.heap_ref()
            .read_heap_bytes_into(source, 0, &mut bytes)
            .map_err(Error::from)?;

        self.write_heap_bytes(handle, start, &bytes)
    }

    /// Return the callable box payload layout.
    fn callable_payload_layout(&self) -> (usize, usize, usize) {
        let pointer_bytes = self.module().tree.pointer_bytes() as usize;
        let function_offset = 0usize;
        let environment_offset = align_offset(pointer_bytes, pointer_bytes);
        let byte_len = environment_offset + pointer_bytes;

        (function_offset, environment_offset, byte_len)
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
        let pointer_bytes = self.module().tree.pointer_bytes() as usize;
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
            .module()
            .layout_id_for_type(ty)
            .ok_or(Error::InvalidInstruction)?;
        let bytes = self.callable_payload(values)?;
        let handle = self.allocate_heap_layout(layout_id, Payload::Bytes(&bytes))?;
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
            self.module().tree.get(repr_type(&self.module().tree, ty)),
            mir::Type::Callable { .. }
        ) {
            return self.materialize_callable(ty, values);
        }

        let layout_id = self
            .module()
            .layout_id_for_type(ty)
            .ok_or(Error::InvalidInstruction)?;
        let handle = self.allocate_heap_layout(layout_id, Payload::Zeroed)?;
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
}

impl<'call, 'ctx> ExternalReadContext<'call, 'ctx> {
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
}

impl<'call, 'ctx> ExternalWriteContext<'call, 'ctx> {
    /// Return one managed value builder by aggregate type name.
    pub fn begin_named_aggregate_builder(
        &mut self,
        aggregate_type: &str,
    ) -> Result<VmValueBuilder<'ctx>, Error> {
        self.context_mut()
            .begin_named_aggregate_builder(aggregate_type)
    }

    /// Materialize one builtin slice value.
    pub fn materialize_builtin_slice_value(
        &mut self,
        data: Value,
        len: usize,
    ) -> Result<Value, Error> {
        let len = Value::uint(len as u64, usize::BITS as u8);
        self.materialize_named_aggregate("Slice", vec![data, len])
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

        self.materialize_named_aggregate("Array", vec![data, len, capacity])
    }

    /// Materialize one named runtime aggregate value.
    pub fn materialize_named_aggregate(
        &mut self,
        aggregate_type: &str,
        values: Vec<Value>,
    ) -> Result<Value, Error> {
        let ty = self.context_mut().named_type(aggregate_type)?;

        self.context_mut().materialize_heap_value(ty, values)
    }

    /// Materialize one named runtime aggregate value from one fixed-size array.
    pub fn materialize_named_aggregate_array<const N: usize>(
        &mut self,
        aggregate_type: &str,
        values: [Value; N],
    ) -> Result<Value, Error> {
        self.materialize_named_aggregate(aggregate_type, values.to_vec())
    }

    /// Intern one string into VM ABI storage.
    pub fn intern_string(&mut self, value: &str) -> Result<Value, Error> {
        self.context_mut().intern_string(value)
    }

    /// Return one VM string handle for one string.
    pub fn string_handle(&mut self, value: &str) -> Result<StringHandle, Error> {
        self.context_mut().string_handle(value)
    }
}
