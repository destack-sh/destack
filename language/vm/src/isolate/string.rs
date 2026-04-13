use std::borrow::Cow;
use std::collections::HashMap;
use std::fmt;
use std::ops::Deref;

use crate::diagnostic::Error;
use crate::snapshot::StringInternerImage;
use destack_mir::LayoutId;

use destack_heap::{
    Heap, ManagedReference, RawPointer, StringLayout, Value, ValueTag, string_layout_id,
};

/// Managed string interner for literal storage.
pub(crate) struct StringInterner {
    /// Canonical runtime string layout for this isolate.
    layout: StringLayout,
    /// Canonical runtime string layout id for this isolate.
    layout_id: Option<LayoutId>,
    /// Canonical well known string type id for this isolate.
    type_id: Option<u32>,
    /// Interned string literals mapped to managed references.
    literals: HashMap<String, ManagedReference>,
    /// Raw heap buffers for string payloads.
    buffers: HashMap<ManagedReference, RawPointer>,
}

/// Borrowed string view tied to one heap borrow.
#[derive(Debug)]
pub struct StringRef<'a> {
    /// Borrowed or owned string payload.
    data: Cow<'a, str>,
}

/// Managed string handle for external bindings.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StringHandle {
    /// Raw value backing the string handle.
    value: Value,
}

impl StringHandle {
    /// Create a string handle from a raw value.
    pub const fn new(value: Value) -> Self {
        Self { value }
    }

    /// Return the raw value backing this handle.
    pub const fn value(self) -> Value {
        self.value
    }
}

impl<'a> StringRef<'a> {
    /// Create a new borrowed string view.
    pub(crate) fn borrowed(data: &'a str) -> Self {
        Self {
            data: Cow::Borrowed(data),
        }
    }

    /// Create a new owned string view.
    pub(crate) fn owned(data: String) -> Self {
        Self {
            data: Cow::Owned(data),
        }
    }

    /// Return the borrowed string slice.
    pub fn as_str(&self) -> &str {
        self.data.as_ref()
    }
}

impl Deref for StringRef<'_> {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.as_str()
    }
}

impl StringInterner {
    /// Create an empty string interner.
    pub(crate) fn new(tree: &destack_mir::NodeTree) -> Self {
        // load the canonical string metadata from the executable
        let layout_id = string_layout_id(tree);
        let type_id = tree.string_type().map(|type_id| type_id.id);

        // initialize empty caches
        Self {
            layout: StringLayout::new(tree.metadata.layout.storage.native_pointer_bytes),
            layout_id,
            type_id,
            literals: HashMap::new(),
            buffers: HashMap::new(),
        }
    }

    /// Capture one immutable string interner image.
    pub(crate) fn image(&self) -> StringInternerImage {
        let literals = self
            .literals
            .iter()
            .map(|(literal, pointer)| (literal.clone(), *pointer))
            .collect();
        let buffers = self
            .buffers
            .iter()
            .map(|(pointer, buffer)| (*pointer, *buffer))
            .collect();

        StringInternerImage { literals, buffers }
    }

    /// Restore this interner from one immutable image.
    pub(crate) fn restore_image(&mut self, image: &StringInternerImage) {
        self.literals = image
            .literals
            .iter()
            .map(|(literal, pointer)| (literal.clone(), *pointer))
            .collect();
        self.buffers = image
            .buffers
            .iter()
            .map(|(pointer, buffer)| (*pointer, *buffer))
            .collect();
    }

    /// Intern a string literal and return its managed value.
    pub(crate) fn intern_string_literal(
        &mut self,
        heap: &mut Heap,
        value: &str,
    ) -> Result<Value, Error> {
        self.try_intern_string_literal(heap, value)
    }

    /// Intern a string literal and return its managed value.
    pub(crate) fn try_intern_string_literal(
        &mut self,
        heap: &mut Heap,
        value: &str,
    ) -> Result<Value, Error> {
        // reuse existing interned handle
        if let Some(handle) = self.literals.get(value).copied() {
            return Ok(Value::managed_reference(handle));
        }

        // compute and validate metadata
        let length_bytes = value.len();
        let length_utf16 = value.encode_utf16().count();
        if length_bytes > u32::MAX as usize || length_utf16 > u32::MAX as usize {
            return Err(Error::InvariantViolation {
                context: "string literal exceeds u32 length limits".to_string(),
            });
        }
        let length_bytes = length_bytes as u32;
        let length_utf16 = length_utf16 as u32;
        // allocate
        let data = Self::allocate_string_bytes(heap, value.as_bytes())?;
        let handle = self.allocate_string_cell(heap, length_utf16, length_bytes, data)?;

        // record
        self.literals.insert(value.to_string(), handle);
        self.buffers.insert(handle, data);

        Ok(Value::managed_reference(handle))
    }

    /// Read a UTF-8 string value from the heap.
    pub(crate) fn string_value(&self, heap: &Heap, value: Value) -> Result<String, Error> {
        // load the borrowed view and allocate an owned copy
        let value = self.string_value_ref(heap, value)?;
        Ok(value.to_string())
    }

    /// Read a UTF-8 string from a managed handle.
    pub(crate) fn string_value_for_handle(
        &self,
        heap: &Heap,
        handle: ManagedReference,
    ) -> Result<String, Error> {
        // load the borrowed view and allocate an owned copy
        let value = self.string_value_ref_for_handle(heap, handle)?;
        Ok(value.to_string())
    }

    /// Read a UTF-8 string view from the heap.
    pub(crate) fn string_value_ref<'a>(
        &self,
        heap: &'a Heap,
        value: Value,
    ) -> Result<StringRef<'a>, Error> {
        let handle = self.validate_string_value(heap, value)?;

        self.string_value_ref_for_handle(heap, handle)
    }

    /// Validate one VM value as a runtime string and return its managed handle.
    pub(crate) fn string_handle(&self, heap: &Heap, value: Value) -> Result<StringHandle, Error> {
        let handle = self.validate_string_value(heap, value)?;
        let value = Value::managed_reference(handle);

        Ok(StringHandle::new(value))
    }

    /// Read a UTF-8 string view from a managed handle.
    pub(crate) fn string_value_ref_for_handle<'a>(
        &self,
        heap: &'a Heap,
        handle: ManagedReference,
    ) -> Result<StringRef<'a>, Error> {
        self.validate_string_handle(heap, handle)?;

        // load the string header bytes from the heap
        let header = heap
            .managed_bytes(handle)
            .ok_or(Error::InvalidManagedReference)?;
        let length_value = self
            .layout
            .read_field(header.as_ref(), StringLayout::LENGTH_BYTES_FIELD as u32)
            .ok_or(Error::InvalidManagedReference)?;
        let length = length_value.as_uint().ok_or_else(|| Error::TypeMismatch {
            expected: "u32".to_string(),
            actual: format!("{length_value:?}"),
        })? as usize;
        if length == 0 {
            return Ok(StringRef::borrowed(""));
        }

        // load the raw payload buffer
        let data_value = self
            .layout
            .read_field(header.as_ref(), StringLayout::DATA_FIELD as u32)
            .ok_or(Error::InvalidManagedReference)?;
        let data_ptr = data_value
            .as_raw_pointer()
            .ok_or(Error::InvalidManagedReference)?;
        let bytes = heap
            .raw_bytes(data_ptr)
            .ok_or(Error::InvalidManagedReference)?;
        if length > bytes.len() {
            return Err(Error::InvalidManagedReference);
        }

        // preserve the backing storage lifetime
        match bytes {
            Cow::Borrowed(bytes) => {
                let bytes = &bytes[..length];
                let value = std::str::from_utf8(bytes).map_err(|_| Error::TypeMismatch {
                    expected: "string".to_string(),
                    actual: "bytes".to_string(),
                })?;

                Ok(StringRef::borrowed(value))
            }
            Cow::Owned(bytes) => {
                let mut bytes = bytes;
                bytes.truncate(length);
                let value = String::from_utf8(bytes).map_err(|_| Error::TypeMismatch {
                    expected: "string".to_string(),
                    actual: "bytes".to_string(),
                })?;

                Ok(StringRef::owned(value))
            }
        }
    }

    /// Collect string literal handles as GC roots.
    pub(crate) fn collect_roots(&self, roots: &mut Vec<ManagedReference>) {
        // extend roots with literal handles
        for handle in self.literals.values() {
            roots.push(*handle);
        }
    }

    /// Sweep raw string payloads for freed managed string headers.
    pub(crate) fn sweep_buffers(&mut self, heap: &mut Heap) {
        // collect handles to free without mutating during iteration
        let mut freed_buffers = Vec::new();
        for (&handle, &raw_ptr) in &self.buffers {
            if !heap.is_managed_allocated(handle) {
                freed_buffers.push((handle, raw_ptr));
            }
        }

        // release raw payloads for freed strings
        for (handle, raw_ptr) in freed_buffers {
            self.buffers.remove(&handle);
            if !raw_ptr.is_null() {
                heap.free_raw(raw_ptr);
            }
        }
    }

    /// Return the number of interned literals.
    pub(crate) fn literal_count(&self) -> usize {
        // expose literal cache size
        self.literals.len()
    }

    /// Return the number of payload buffers.
    pub(crate) fn buffer_count(&self) -> usize {
        // expose buffer cache size
        self.buffers.len()
    }

    /// Allocate raw heap storage for string payload bytes.
    fn allocate_string_bytes(heap: &mut Heap, bytes: &[u8]) -> Result<RawPointer, Error> {
        // treat empty payloads as null pointers
        if bytes.is_empty() {
            return Ok(RawPointer::NULL);
        }

        // allocate raw heap buffer for payload
        heap.allocate_raw_bytes(bytes).map_err(Error::from)
    }

    /// Allocate a managed string header allocation.
    fn allocate_string_cell(
        &self,
        heap: &mut Heap,
        length_utf16: u32,
        length_bytes: u32,
        data: RawPointer,
    ) -> Result<ManagedReference, Error> {
        let layout_id = self.layout_id.ok_or_else(|| Error::TypeMismatch {
            expected: "canonical string layout".to_string(),
            actual: "missing".to_string(),
        })?;
        let type_id = self.type_id.ok_or_else(|| Error::TypeMismatch {
            expected: "canonical string type".to_string(),
            actual: "missing".to_string(),
        })?;

        // assemble the canonical string header
        let mut bytes = vec![0u8; self.layout.byte_len()];

        let wrote_length_utf16 = self.layout.write_field(
            &mut bytes,
            StringLayout::LENGTH_UTF16_FIELD as u32,
            Value::uint32(length_utf16),
        );
        let wrote_length_bytes = self.layout.write_field(
            &mut bytes,
            StringLayout::LENGTH_BYTES_FIELD as u32,
            Value::uint32(length_bytes),
        );
        let wrote_data = self.layout.write_field(
            &mut bytes,
            StringLayout::DATA_FIELD as u32,
            Value::raw_pointer(data),
        );

        if !wrote_length_utf16 || !wrote_length_bytes || !wrote_data {
            return Err(Error::InvalidManagedReference);
        }

        // allocate managed heap storage for the header
        let handle = heap
            .allocate_managed_bytes_typed(
                &bytes,
                destack_heap::ReferenceMap::empty(),
                Some(layout_id),
                type_id,
            )
            .map_err(Error::from)?;

        Ok(handle)
    }

    /// Validate one VM value as a runtime string and return its managed handle.
    fn validate_string_value(&self, heap: &Heap, value: Value) -> Result<ManagedReference, Error> {
        // ensure the value is one managed reference
        if value.tag() != ValueTag::ManagedReference {
            return Err(Error::TypeMismatch {
                expected: "string".to_string(),
                actual: format!("{value:?}"),
            });
        }

        let handle = value
            .as_managed_reference()
            .ok_or_else(|| Error::TypeMismatch {
                expected: "string".to_string(),
                actual: format!("{value:?}"),
            })?;
        self.validate_string_handle(heap, handle)?;

        Ok(handle)
    }

    /// Validate one managed handle against the canonical string layout.
    fn validate_string_handle(&self, heap: &Heap, handle: ManagedReference) -> Result<(), Error> {
        let layout_id = self.layout_id.ok_or_else(|| Error::TypeMismatch {
            expected: "canonical string layout".to_string(),
            actual: "missing".to_string(),
        })?;
        let type_id = self.type_id.ok_or_else(|| Error::TypeMismatch {
            expected: "canonical string type".to_string(),
            actual: "missing".to_string(),
        })?;

        // reject null handles
        if handle.is_null() {
            return Err(Error::NullPointerDereference);
        }

        // reject dangling references
        if heap.managed_bytes(handle).is_none() {
            return Err(Error::InvalidManagedReference);
        }

        // reject non-string managed allocations
        if heap.managed_type_id(handle) != Some(type_id) {
            return Err(Error::TypeMismatch {
                expected: "string".to_string(),
                actual: "managed reference".to_string(),
            });
        }

        // reject managed allocations with a different physical string layout
        if heap.managed_layout_id(handle) != Some(layout_id) {
            return Err(Error::TypeMismatch {
                expected: "canonical string layout".to_string(),
                actual: "managed allocation".to_string(),
            });
        }

        Ok(())
    }
}

impl fmt::Debug for StringInterner {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // precompute sizes for debug output
        let literal_count = self.literal_count();
        let buffer_count = self.buffer_count();

        f.debug_struct("StringInterner")
            .field("layout", &self.layout)
            .field("layout_id", &self.layout_id)
            .field("type_id", &self.type_id)
            .field("literals", &format!("<{literal_count} literals>"))
            .field("buffers", &format!("<{buffer_count} buffers>"))
            .finish_non_exhaustive()
    }
}
