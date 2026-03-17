use std::borrow::Cow;
use std::collections::HashMap;
use std::fmt;
use std::ops::Deref;

use crate::diagnostic::Error;
use crate::snapshot::StringInternerImage;

use destack_heap::{
    Heap, ManagedReference, RawPointer, STRING_FLAG_IS_ASCII, STRING_FLAG_IS_INTERNED,
    STRING_FLAG_IS_STATIC, StringLayout, Value, ValueTag,
};

/// Managed string interner for literal storage.
pub(crate) struct StringInterner {
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
    pub(crate) fn new() -> Self {
        // initialize empty caches
        Self {
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
    pub(crate) fn intern_string_literal(&mut self, heap: &mut Heap, value: &str) -> Value {
        self.try_intern_string_literal(heap, value)
            .unwrap_or_else(|error| panic!("{error}"))
    }

    /// Intern a string literal and return its managed value.
    pub(crate) fn try_intern_string_literal(
        &mut self,
        heap: &mut Heap,
        value: &str,
    ) -> Result<Value, Error> {
        // reuse existing interned handle
        if let Some(handle) = self.literals.get(value).copied() {
            return Ok(Value::string(handle));
        }

        // compute and validate metadata
        let length_bytes = value.len();
        let length_utf16 = value.encode_utf16().count();
        if length_bytes > u32::MAX as usize || length_utf16 > u32::MAX as usize {
            panic!("string literal exceeds u32 length limits");
        }
        let length_bytes = length_bytes as u32;
        let length_utf16 = length_utf16 as u32;
        let mut flags = STRING_FLAG_IS_INTERNED | STRING_FLAG_IS_STATIC;
        if value.is_ascii() {
            flags |= STRING_FLAG_IS_ASCII;
        }

        // allocate
        let data = Self::allocate_string_bytes(heap, value.as_bytes())?;
        let handle = Self::allocate_string_cell(
            heap,
            length_utf16,
            length_bytes,
            0,
            length_bytes,
            flags,
            data,
        )?;

        // record
        self.literals.insert(value.to_string(), handle);
        if !data.is_null() {
            self.buffers.insert(handle, data);
        }

        Ok(Value::string(handle))
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
        // ensure the value is a string
        let handle = match value.tag() {
            ValueTag::String => value.as_managed_reference().unwrap(),
            _ => {
                return Err(Error::TypeMismatch {
                    expected: "string".to_string(),
                    actual: format!("{value:?}"),
                });
            }
        };

        self.string_value_ref_for_handle(heap, handle)
    }

    /// Read a UTF-8 string view from a managed handle.
    pub(crate) fn string_value_ref_for_handle<'a>(
        &self,
        heap: &'a Heap,
        handle: ManagedReference,
    ) -> Result<StringRef<'a>, Error> {
        // reject null handles
        if handle.is_null() {
            return Err(Error::NullPointerDereference);
        }

        // load the string header bytes from the heap
        let header = heap
            .managed_bytes(handle)
            .ok_or(Error::InvalidManagedReference)?;
        let length_value =
            StringLayout::read_field(header.as_ref(), StringLayout::LENGTH_BYTES_FIELD as u32)
                .ok_or(Error::InvalidManagedReference)?;
        let length = length_value.as_uint().ok_or_else(|| Error::TypeMismatch {
            expected: "u32".to_string(),
            actual: format!("{length_value:?}"),
        })? as usize;
        if length == 0 {
            return Ok(StringRef::borrowed(""));
        }

        // load the raw payload buffer
        let data_value = StringLayout::read_field(header.as_ref(), StringLayout::DATA_FIELD as u32)
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
                let bytes = bytes[..length].to_vec();
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
        heap: &mut Heap,
        length_utf16: u32,
        length_bytes: u32,
        hash: u64,
        capacity: u32,
        flags: u32,
        data: RawPointer,
    ) -> Result<ManagedReference, Error> {
        // assemble fixed string header bytes
        let mut bytes = [0u8; StringLayout::BYTE_LEN];
        bytes[StringLayout::LENGTH_UTF16_OFFSET..StringLayout::LENGTH_UTF16_OFFSET + 4]
            .copy_from_slice(&length_utf16.to_le_bytes());
        bytes[StringLayout::LENGTH_BYTES_OFFSET..StringLayout::LENGTH_BYTES_OFFSET + 4]
            .copy_from_slice(&length_bytes.to_le_bytes());
        bytes[StringLayout::HASH_OFFSET..StringLayout::HASH_OFFSET + 8]
            .copy_from_slice(&hash.to_le_bytes());
        bytes[StringLayout::CAPACITY_OFFSET..StringLayout::CAPACITY_OFFSET + 4]
            .copy_from_slice(&capacity.to_le_bytes());
        bytes[StringLayout::FLAGS_OFFSET..StringLayout::FLAGS_OFFSET + 4]
            .copy_from_slice(&flags.to_le_bytes());
        bytes[StringLayout::DATA_OFFSET..StringLayout::DATA_OFFSET + 8]
            .copy_from_slice(&data.bits().to_le_bytes());

        // allocate managed heap storage for the header
        heap.allocate_managed_bytes(&bytes, destack_heap::ReferenceMap::empty(), None)
            .map_err(Error::from)
    }
}

impl fmt::Debug for StringInterner {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // precompute sizes for debug output
        let literal_count = self.literal_count();
        let buffer_count = self.buffer_count();

        f.debug_struct("StringInterner")
            .field("literals", &format!("<{literal_count} literals>"))
            .field("buffers", &format!("<{buffer_count} buffers>"))
            .finish_non_exhaustive()
    }
}
