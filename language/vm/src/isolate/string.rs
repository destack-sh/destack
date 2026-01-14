use std::collections::HashMap;
use std::fmt;

use crate::diagnostic::Error;
use crate::memory::{
    HeapHandle, ManagedHeap, RawCellStorage, RawHeap, RawPointer, STRING_FLAG_IS_ASCII,
    STRING_FLAG_IS_INTERNED, STRING_FLAG_IS_STATIC, StringLayout, Value, ValueTag,
};

/// Managed string interner for literal storage.
pub(crate) struct StringInterner {
    /// Interned string literals mapped to heap handles.
    literals: HashMap<String, HeapHandle>,
    /// Raw heap buffers for string payloads.
    buffers: HashMap<HeapHandle, RawPointer>,
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

    /// Intern a string literal and return its managed value.
    pub(crate) fn intern_string_literal(
        &mut self,
        managed_heap: &mut ManagedHeap,
        raw_heap: &mut RawHeap,
        value: &str,
    ) -> Value {
        // reuse existing interned handle
        if let Some(handle) = self.literals.get(value).copied() {
            return Value::string(handle);
        }

        // compute UTF-8 byte length
        let length_bytes = value.len();

        // compute UTF-16 code unit length
        let length_utf16 = value.encode_utf16().count();

        // validate length bounds for string metadata
        if length_bytes > u32::MAX as usize || length_utf16 > u32::MAX as usize {
            panic!("string literal exceeds u32 length limits");
        }

        // materialize length fields
        let length_bytes = length_bytes as u32;
        let length_utf16 = length_utf16 as u32;

        // compute string flags for literal storage
        let mut flags = STRING_FLAG_IS_INTERNED | STRING_FLAG_IS_STATIC;
        if value.is_ascii() {
            flags |= STRING_FLAG_IS_ASCII;
        }

        // allocate raw UTF-8 payload
        let data = Self::allocate_string_bytes(raw_heap, value.as_bytes());

        // allocate the managed string header
        let handle = Self::allocate_string_cell(
            managed_heap,
            length_utf16,
            length_bytes,
            0,
            length_bytes,
            flags,
            data,
        );

        // record interned handle and payload buffer
        self.literals.insert(value.to_string(), handle);
        if !data.is_null() {
            self.buffers.insert(handle, data);
        }

        Value::string(handle)
    }

    /// Read a UTF-8 string value from the heap.
    pub(crate) fn string_value(
        &self,
        managed_heap: &ManagedHeap,
        raw_heap: &RawHeap,
        value: Value,
    ) -> Result<String, Error> {
        // ensure the value is a string handle
        let handle = match value.tag() {
            ValueTag::String => value.as_heap_handle().unwrap(),
            _ => {
                return Err(Error::TypeMismatch {
                    expected: "string".to_string(),
                    actual: format!("{value:?}"),
                });
            }
        };

        // load string from the managed handle
        self.string_value_for_handle(managed_heap, raw_heap, handle)
    }

    /// Read a UTF-8 string from a managed handle.
    pub(crate) fn string_value_for_handle(
        &self,
        managed_heap: &ManagedHeap,
        raw_heap: &RawHeap,
        handle: HeapHandle,
    ) -> Result<String, Error> {
        // reject null handles
        if handle.is_null() {
            return Err(Error::NullPointerDereference);
        }

        // load the managed string cell
        let cell = managed_heap.get(handle).ok_or(Error::InvalidHeapHandle)?;
        let length_value = cell
            .slots
            .get(StringLayout::LENGTH_BYTES)
            .copied()
            .ok_or(Error::InvalidHeapHandle)?;
        let length = length_value.as_uint().ok_or_else(|| Error::TypeMismatch {
            expected: "u32".to_string(),
            actual: format!("{length_value:?}"),
        })? as usize;
        if length == 0 {
            return Ok(String::new());
        }

        // load the raw payload buffer
        let data_value = cell
            .slots
            .get(StringLayout::DATA)
            .copied()
            .ok_or(Error::InvalidHeapHandle)?;
        let data_ptr = data_value
            .as_raw_pointer()
            .ok_or(Error::InvalidHeapHandle)?;
        let raw_cell = raw_heap.get(data_ptr).ok_or(Error::InvalidHeapHandle)?;
        let bytes = match &raw_cell.storage {
            RawCellStorage::Bytes(bytes) => bytes,
            _ => return Err(Error::InvalidHeapHandle),
        };
        let value = String::from_utf8(bytes.clone()).map_err(|_| Error::TypeMismatch {
            expected: "string".to_string(),
            actual: "bytes".to_string(),
        })?;

        Ok(value)
    }

    /// Collect string literal handles as GC roots.
    pub(crate) fn collect_roots(&self, roots: &mut Vec<HeapHandle>) {
        // extend roots with literal handles
        for handle in self.literals.values() {
            roots.push(*handle);
        }
    }

    /// Sweep raw string payloads for freed managed string headers.
    pub(crate) fn sweep_buffers(&mut self, managed_heap: &ManagedHeap, raw_heap: &mut RawHeap) {
        // collect handles to free without mutating during iteration
        let mut freed_buffers = Vec::new();
        for (&handle, &raw_ptr) in &self.buffers {
            if !managed_heap.is_allocated(handle) {
                freed_buffers.push((handle, raw_ptr));
            }
        }

        // release raw payloads for freed strings
        for (handle, raw_ptr) in freed_buffers {
            self.buffers.remove(&handle);
            if !raw_ptr.is_null() {
                raw_heap.free(raw_ptr);
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
    fn allocate_string_bytes(raw_heap: &mut RawHeap, bytes: &[u8]) -> RawPointer {
        // treat empty payloads as null pointers
        if bytes.is_empty() {
            return RawPointer::NULL;
        }

        // allocate raw heap buffer for payload
        raw_heap.allocate_with_bytes(bytes)
    }

    /// Allocate a managed string header cell.
    fn allocate_string_cell(
        managed_heap: &mut ManagedHeap,
        length_utf16: u32,
        length_bytes: u32,
        hash: u64,
        capacity: u32,
        flags: u32,
        data: RawPointer,
    ) -> HeapHandle {
        // assemble header slots for the string layout
        let mut slots = vec![Value::VOID; StringLayout::SLOT_COUNT];
        slots[StringLayout::LENGTH_UTF16] = Value::uint(length_utf16 as u64, 32);
        slots[StringLayout::LENGTH_BYTES] = Value::uint(length_bytes as u64, 32);
        slots[StringLayout::HASH] = Value::uint(hash, 64);
        slots[StringLayout::CAPACITY] = Value::uint(capacity as u64, 32);
        slots[StringLayout::FLAGS] = Value::uint(flags as u64, 32);
        slots[StringLayout::DATA] = Value::raw_pointer(data);

        // allocate managed heap cell for the header
        managed_heap.allocate_with_values(slots)
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
