use std::borrow::Cow;
use std::fmt;
use std::marker::PhantomData;
use std::sync::Arc;

use crate::Word;
use crate::diagnostic::Error;
use crate::program::{Layout, decode_word_bytes, encode_word_bytes, repr_type};
use destack_heap::{AllocationPlan, HeapReference, Payload};
use destack_mir as mir;

use super::{ExternalCallContext, ExternalReadContext, ExternalWriteContext};

// FUGU #Architecture: remove generated ABI shims once VM and native ABI share normal MIR payloads
/// One typed field inside one aggregate payload.
#[derive(Clone, Copy, Debug)]
struct AggregateField {
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
    value: Word,
}

impl StringHandle {
    /// Create a string handle from one VM value.
    pub const fn new(value: Word) -> Self {
        Self { value }
    }

    /// Return the wrapped VM value.
    pub const fn value(self) -> Word {
        self.value
    }
}

/// One VM aggregate value view.
#[derive(Debug)]
pub struct VmValueRef<'call, 'ctx> {
    /// The active external call context.
    context: *mut ExternalCallContext<'ctx>,
    /// The typed fields inside this value.
    fields: Box<[AggregateField]>,
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

    /// Return the semantic field count for this value.
    pub fn field_count(&self) -> usize {
        self.fields.len()
    }

    /// Return one nested VM value view for one field.
    pub fn field_ref(&self, index: u32) -> Result<Self, Error> {
        let field = self
            .fields
            .get(index as usize)
            .copied()
            .ok_or(Error::InvalidHeapReference)?;
        let layout = self.context_ref().layout(field.ty)?;
        if layout.is_scalar() {
            return Err(Error::InvalidHeapReference);
        }

        let start = self
            .start
            .checked_add(field.offset)
            .ok_or(Error::InvalidHeapReference)?;
        let end = start
            .checked_add(layout.byte_len)
            .ok_or(Error::InvalidHeapReference)?;
        if end > self.bytes.len() {
            return Err(Error::InvalidHeapReference);
        }

        Ok(Self {
            context: self.context,
            fields: aggregate_fields(layout)?.into_boxed_slice(),
            bytes: self.bytes.clone(),
            start,
            byte_len: layout.byte_len,
            _marker: PhantomData,
        })
    }

    /// Decode one field value from this view.
    pub fn field_value(&self, index: u32) -> Result<Word, Error> {
        let field = self
            .fields
            .get(index as usize)
            .copied()
            .ok_or(Error::InvalidHeapReference)?;
        let layout = self.context_ref().layout(field.ty)?;
        let bytes = self.byte_window(field.offset, layout.byte_len)?.to_vec();

        self.context_mut()
            .materialize_value_from_bytes(field.ty, &bytes)
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
    fields: Vec<Option<Word>>,
    /// The lifetime marker for the call context.
    _marker: PhantomData<&'ctx ExternalCallContext<'ctx>>,
}

impl VmValueBuilder<'_> {
    /// Return the expected field count for this builder.
    pub fn field_count(&self) -> usize {
        self.fields.len()
    }

    /// Write one field value.
    pub fn write_field(&mut self, index: u32, value: Word) -> Result<(), Error> {
        let Some(field) = self.fields.get_mut(index as usize) else {
            return Err(Error::InvalidHeapReference);
        };

        *field = Some(value);

        Ok(())
    }

    /// Finish the value and allocate its typed heap payload.
    pub fn finish(self) -> Result<Word, Error> {
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
fn aggregate_fields(layout: &Layout) -> Result<Vec<AggregateField>, Error> {
    if let Some(field_count) = layout.field_count() {
        let mut values = Vec::with_capacity(field_count);

        // collect fields in source order
        for index in 0..field_count {
            let field = layout
                .field(index as u32)
                .ok_or(Error::InvalidInstruction)?;
            values.push(AggregateField {
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
        values.push(AggregateField {
            ty: element.ty,
            offset,
        });
    }

    Ok(values)
}

impl<'ctx> ExternalCallContext<'ctx> {
    /// Copy one managed payload range into caller storage.
    fn copy_payload_into(
        &self,
        reference: HeapReference,
        start: usize,
        target: &mut [u8],
    ) -> Result<(), Error> {
        if reference.is_null() {
            return Err(Error::InvalidHeapReference);
        }

        let address = self.heap_ref().heap_base_address() + reference.offset() + start;

        // copy from the managed payload address
        unsafe {
            std::ptr::copy_nonoverlapping(address as *const u8, target.as_mut_ptr(), target.len());
        }

        Ok(())
    }

    /// Copy one managed payload range into owned storage.
    fn copy_payload(&self, reference: HeapReference, byte_len: usize) -> Result<Vec<u8>, Error> {
        let mut bytes = vec![0u8; byte_len];
        self.copy_payload_into(reference, 0, &mut bytes)?;

        Ok(bytes)
    }

    /// Write one managed payload range.
    fn write_payload(
        &mut self,
        reference: HeapReference,
        start: usize,
        bytes: &[u8],
    ) -> Result<(), Error> {
        if reference.is_null() {
            return Err(Error::InvalidHeapReference);
        }

        let heap = self.heap();
        heap.write_barrier(reference, start, bytes.len())?;
        let address = heap.heap_base_address() + reference.offset() + start;

        // copy into the managed payload address
        unsafe {
            std::ptr::copy_nonoverlapping(bytes.as_ptr(), address as *mut u8, bytes.len());
        }

        Ok(())
    }

    /// Allocate one heap word buffer.
    pub fn allocate_heap_words(&mut self, count: usize) -> Result<HeapReference, Error> {
        let byte_len = count * Word::BYTE_LEN;
        let reference_map = mir::ReferenceMap::None;
        let plan = AllocationPlan::new(byte_len, Word::BYTE_LEN, &reference_map);
        let layout = self.heap().allocation_layout(plan);

        self.heap()
            .allocate(&layout, Payload::Zeroed)
            .map_err(Into::into)
    }

    /// Read heap words from one heap reference.
    pub fn heap_words(&self, reference: HeapReference, count: usize) -> Result<Vec<Word>, Error> {
        let byte_len = count * Word::BYTE_LEN;
        let bytes = self.copy_payload(reference, byte_len)?;

        let mut values = Vec::with_capacity(count);

        // decode each packed word from the managed payload
        for bytes in bytes.chunks_exact(Word::BYTE_LEN) {
            let value = Word::from_byte_slice(bytes).ok_or(Error::InvalidHeapReference)?;
            values.push(value);
        }

        Ok(values)
    }

    /// Read one heap word by index.
    pub fn heap_word(&self, reference: HeapReference, index: usize) -> Result<Word, Error> {
        let start = index * Word::BYTE_LEN;
        let mut bytes = [0u8; Word::BYTE_LEN];
        self.copy_payload_into(reference, start, &mut bytes)?;
        Word::from_byte_slice(&bytes).ok_or(Error::InvalidHeapReference)
    }

    /// Write one heap word by index.
    pub fn write_heap_word(
        &mut self,
        reference: HeapReference,
        index: usize,
        value: Word,
    ) -> Result<(), Error> {
        let start = index * Word::BYTE_LEN;

        self.write_payload(reference, start, &value.to_byte_array())
    }

    /// Return one named MIR type from program metadata.
    fn named_type(&self, name: &str) -> Result<mir::LocalNodeId<mir::Type>, Error> {
        self.program()
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
    ) -> Result<Word, Error> {
        if self.layout(ty)?.is_scalar() {
            return decode_word_bytes(&self.program().tree, ty, bytes);
        }

        let fields = aggregate_fields(self.layout(ty)?)?;
        let mut values = Vec::with_capacity(fields.len());

        for field in fields {
            let layout = self.layout(field.ty)?;
            let end = field
                .offset
                .checked_add(layout.byte_len)
                .ok_or(Error::InvalidHeapReference)?;
            let bytes = bytes
                .get(field.offset..end)
                .ok_or(Error::InvalidHeapReference)?;
            let value = self.materialize_value_from_bytes(field.ty, bytes)?;
            values.push(value);
        }

        self.materialize_heap_value(ty, values)
    }

    /// Build one named value builder from the program type table.
    fn begin_named_aggregate_builder(
        &mut self,
        type_name: &str,
    ) -> Result<VmValueBuilder<'ctx>, Error> {
        let ty = self.named_type(type_name)?;
        let field_count = aggregate_fields(self.layout(ty)?)?.len();

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
        value: Word,
    ) -> Result<(), Error> {
        // write scalars directly into the target payload
        if self.layout(ty)?.is_scalar() {
            let bytes = encode_word_bytes(&self.program().tree, ty, value)?;
            let bytes = bytes.as_slice().to_vec();
            self.write_payload(handle, start, &bytes)?;

            return Ok(());
        }

        let source = value.as_heap_reference();
        let byte_len = self.layout(ty)?.byte_len;
        let mut bytes = vec![0u8; byte_len];
        self.copy_payload_into(source, 0, &mut bytes)?;

        self.write_payload(handle, start, &bytes)
    }

    /// Return the callable box payload layout.
    fn callable_payload_layout(&self) -> (usize, usize, usize) {
        let pointer_bytes = self.program().tree.pointer_bytes() as usize;
        let function_offset = 0usize;
        let environment_offset = align_offset(pointer_bytes, pointer_bytes);
        let byte_len = environment_offset + pointer_bytes;

        (function_offset, environment_offset, byte_len)
    }

    /// Return one encoded callable payload from function and environment values.
    fn callable_payload(&self, values: Vec<Word>) -> Result<Vec<u8>, Error> {
        if values.len() != 2 {
            return Err(Error::TypeMismatch {
                expected: "2 callable values".to_string(),
                actual: format!("{} callable values", values.len()),
            });
        }

        let (function_offset, environment_offset, _) = self.callable_payload_layout();
        let function = values[0].as_function_pointer();
        let environment = values[1].as_heap_reference();
        let function_bytes = (function.bits() as u64).to_le_bytes();
        let environment_bytes = (environment.bits() as u64).to_le_bytes();
        let pointer_bytes = self.program().tree.pointer_bytes() as usize;
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
        values: Vec<Word>,
    ) -> Result<Word, Error> {
        let layout_id = self
            .program()
            .layout_id_for_type(ty)
            .ok_or(Error::InvalidInstruction)?;
        let bytes = self.callable_payload(values)?;
        let handle = self.allocate_heap_layout_bytes(layout_id, &bytes)?;
        let handle = self.capture_heap_reference(handle)?;

        Ok(Word::heap_reference(handle))
    }

    /// Materialize one typed aggregate value on the heap.
    pub(crate) fn materialize_heap_value(
        &mut self,
        ty: mir::LocalNodeId<mir::Type>,
        values: Vec<Word>,
    ) -> Result<Word, Error> {
        if matches!(
            self.program().tree.get(repr_type(&self.program().tree, ty)),
            mir::Type::Callable { .. }
        ) {
            return self.materialize_callable(ty, values);
        }

        let layout_id = self
            .program()
            .layout_id_for_type(ty)
            .ok_or(Error::InvalidInstruction)?;
        let handle = self.allocate_heap_layout(layout_id, Payload::Zeroed)?;
        let handle = self.capture_heap_reference(handle)?;
        let reprs = aggregate_fields(self.layout(ty)?)?;

        if values.len() != reprs.len() {
            return Err(Error::TypeMismatch {
                expected: format!("{} aggregate values", reprs.len()),
                actual: format!("{} aggregate values", values.len()),
            });
        }

        // write each value directly into the aggregate payload
        for (repr, value) in reprs.into_iter().zip(values) {
            self.write_heap_value_field(handle, repr.offset, repr.ty, value)?;
        }

        Ok(Word::heap_reference(handle))
    }

    /// Store UTF-8 bytes in raw VM storage and return the value.
    pub fn intern_string(&mut self, value: &str) -> Result<Word, Error> {
        let pointer = self.allocate_raw_bytes(value.as_bytes())?;

        Ok(Word::raw_pointer(pointer))
    }

    /// Store UTF-8 bytes in raw VM storage and return a handle.
    pub fn string_handle(&mut self, value: &str) -> Result<StringHandle, Error> {
        let value = self.intern_string(value)?;

        Ok(StringHandle::new(value))
    }

    /// Validate one VM value as a string handle.
    pub fn string_handle_from_value(&self, value: Word) -> Result<StringHandle, Error> {
        if !value.as_raw_pointer().is_null() {
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
        let pointer = value.as_raw_pointer();
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
    pub fn string_value(&self, value: Word) -> Result<String, Error> {
        let handle = self.string_handle_from_value(value)?;

        Ok(self.string_ref(handle)?.to_string())
    }
}

impl<'call, 'ctx> ExternalReadContext<'call, 'ctx> {
    /// Return one cached VM value view.
    pub fn value_ref(
        &self,
        value: Word,
        aggregate_type: &str,
    ) -> Result<VmValueRef<'call, 'ctx>, Error> {
        let reference = value.as_heap_reference();
        let ty = self.context().named_type(aggregate_type)?;
        let layout = self.context().layout(ty)?;
        let fields = aggregate_fields(layout)?.into_boxed_slice();
        let byte_len = layout.byte_len;
        let bytes = self.context().copy_payload(reference, byte_len)?;

        Ok(VmValueRef {
            context: self.context as *mut ExternalCallContext<'ctx>,
            fields,
            bytes: Arc::<[u8]>::from(bytes),
            start: 0,
            byte_len,
            _marker: PhantomData,
        })
    }

    /// Return one VM string handle from one value.
    pub fn string_handle_from_value(&self, value: Word) -> Result<StringHandle, Error> {
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
        data: Word,
        len: usize,
    ) -> Result<Word, Error> {
        let len = Word::uint(len as u64, usize::BITS as u8);
        self.materialize_named_aggregate("Slice", vec![data, len])
    }

    /// Materialize one builtin array value.
    pub fn materialize_builtin_array_value(
        &mut self,
        slice: Word,
        capacity: usize,
    ) -> Result<Word, Error> {
        let capacity = Word::uint(capacity as u64, usize::BITS as u8);

        self.materialize_named_aggregate("Array", vec![slice, capacity])
    }

    /// Materialize one named runtime aggregate value.
    pub fn materialize_named_aggregate(
        &mut self,
        aggregate_type: &str,
        values: Vec<Word>,
    ) -> Result<Word, Error> {
        let ty = self.context_mut().named_type(aggregate_type)?;

        self.context_mut().materialize_heap_value(ty, values)
    }

    /// Materialize one named runtime aggregate value from one fixed-size array.
    pub fn materialize_named_aggregate_array<const N: usize>(
        &mut self,
        aggregate_type: &str,
        values: [Word; N],
    ) -> Result<Word, Error> {
        self.materialize_named_aggregate(aggregate_type, values.to_vec())
    }

    /// Intern one string into VM ABI storage.
    pub fn intern_string(&mut self, value: &str) -> Result<Word, Error> {
        self.context_mut().intern_string(value)
    }

    /// Return one VM string handle for one string.
    pub fn string_handle(&mut self, value: &str) -> Result<StringHandle, Error> {
        self.context_mut().string_handle(value)
    }
}
