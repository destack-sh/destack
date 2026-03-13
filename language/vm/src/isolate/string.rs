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

/// Borrowed string view metadata.
#[derive(Debug, Clone, Copy)]
pub(crate) struct StringView {
    /// Pointer to UTF-8 payload.
    pub ptr: *const u8,
    /// Length of payload in bytes.
    pub len: usize,
}

impl StringView {
    /// Return an empty string view.
    pub(crate) fn empty() -> Self {
        Self {
            ptr: std::ptr::null(),
            len: 0,
        }
    }
}

/// Borrowed string view tied to one heap borrow.
#[derive(Debug)]
pub struct StringRef<'a> {
    /// Heap borrow that keeps the string payload alive.
    _heap: &'a Heap,
    /// Borrowed string payload.
    data_ptr: *const u8,
    /// Length of the borrowed string payload.
    data_len: usize,
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
    pub(crate) fn new(heap: &'a Heap, data_ptr: *const u8, data_len: usize) -> Self {
        Self {
            _heap: heap,
            data_ptr,
            data_len,
        }
    }

    /// Return the borrowed string slice.
    pub fn as_str(&self) -> &str {
        if self.data_len == 0 {
            return "";
        }

        // safety: payload is validated as UTF-8 on creation
        unsafe {
            std::str::from_utf8_unchecked(std::slice::from_raw_parts(self.data_ptr, self.data_len))
        }
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
    ) -> Result<&'a str, Error> {
        let view = self.string_value_view(heap, value)?;
        if view.len == 0 {
            return Ok("");
        }

        // safety: payload is validated by string_value_view
        Ok(
            unsafe {
                std::str::from_utf8_unchecked(std::slice::from_raw_parts(view.ptr, view.len))
            },
        )
    }

    /// Read a UTF-8 string view from a managed handle.
    pub(crate) fn string_value_ref_for_handle<'a>(
        &self,
        heap: &'a Heap,
        handle: ManagedReference,
    ) -> Result<&'a str, Error> {
        let view = self.string_value_view_for_handle(heap, handle)?;
        if view.len == 0 {
            return Ok("");
        }

        // safety: payload is validated by string_value_view_for_handle
        Ok(
            unsafe {
                std::str::from_utf8_unchecked(std::slice::from_raw_parts(view.ptr, view.len))
            },
        )
    }

    /// Read a UTF-8 string view from the heap.
    pub(crate) fn string_value_view(&self, heap: &Heap, value: Value) -> Result<StringView, Error> {
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

        // load the borrowed view
        self.string_value_view_for_handle(heap, handle)
    }

    /// Read a UTF-8 string view from a managed handle.
    pub(crate) fn string_value_view_for_handle(
        &self,
        heap: &Heap,
        handle: ManagedReference,
    ) -> Result<StringView, Error> {
        // reject null handles
        if handle.is_null() {
            return Err(Error::NullPointerDereference);
        }

        // load the managed string cell
        let cell = heap
            .managed_allocation(handle)
            .ok_or(Error::InvalidManagedReference)?;
        let length_value = cell
            .get(StringLayout::LENGTH_BYTES)
            .copied()
            .ok_or(Error::InvalidManagedReference)?;
        let length = length_value.as_uint().ok_or_else(|| Error::TypeMismatch {
            expected: "u32".to_string(),
            actual: format!("{length_value:?}"),
        })? as usize;
        if length == 0 {
            return Ok(StringView::empty());
        }

        // load the raw payload buffer
        let data_value = cell
            .get(StringLayout::DATA)
            .copied()
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

        let slice = &bytes[..length];
        if std::str::from_utf8(slice).is_err() {
            return Err(Error::TypeMismatch {
                expected: "string".to_string(),
                actual: "bytes".to_string(),
            });
        }

        Ok(StringView {
            ptr: slice.as_ptr(),
            len: slice.len(),
        })
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

    /// Allocate a managed string header cell.
    fn allocate_string_cell(
        heap: &mut Heap,
        length_utf16: u32,
        length_bytes: u32,
        hash: u64,
        capacity: u32,
        flags: u32,
        data: RawPointer,
    ) -> Result<ManagedReference, Error> {
        // assemble header slots for the string layout
        let mut slots = vec![Value::VOID; StringLayout::SLOT_COUNT];
        slots[StringLayout::LENGTH_UTF16] = Value::uint(length_utf16 as u64, 32);
        slots[StringLayout::LENGTH_BYTES] = Value::uint(length_bytes as u64, 32);
        slots[StringLayout::HASH] = Value::uint(hash, 64);
        slots[StringLayout::CAPACITY] = Value::uint(capacity as u64, 32);
        slots[StringLayout::FLAGS] = Value::uint(flags as u64, 32);
        slots[StringLayout::DATA] = Value::raw_pointer(data);

        // allocate managed heap cell for the header
        heap.allocate_managed_values(slots).map_err(Error::from)
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
