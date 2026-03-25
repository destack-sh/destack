use std::fmt;
use std::ptr::NonNull;

use crate::diagnostic::Error;
use destack_heap::{MemoryContext, RawPointer, SharedPointer, Value, ValueTag};

use super::{StringHandle, StringInterner, StringRef};

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
pub type ExternalFn = Box<dyn ExternalHandler>;

/// Cached external handler pointer.
pub(crate) type ExternalFnPtr = NonNull<dyn ExternalHandler>;

/// Runtime call context with restricted access to isolate state.
pub struct ExternalCallContext<'ctx> {
    /// The string interner backing this external call.
    string_interner: &'ctx mut StringInterner,
    /// The execution memory backing this external call.
    memory: MemoryContext<'ctx>,
}

impl<'ctx> ExternalCallContext<'ctx> {
    /// Wrap the isolate string interner for external calls.
    pub(crate) fn new(
        string_interner: &'ctx mut StringInterner,
        memory: MemoryContext<'ctx>,
    ) -> Self {
        Self {
            string_interner,
            memory,
        }
    }

    /// Borrow the local heap.
    fn heap(&mut self) -> &mut destack_heap::Heap {
        self.memory.heap()
    }

    /// Borrow the local heap immutably.
    fn heap_ref(&self) -> &destack_heap::Heap {
        self.memory.heap_ref()
    }

    /// Borrow the world shared memory immutably.
    fn shared_ref(&self) -> &destack_heap::SharedSpace {
        self.memory.shared_ref()
    }

    /// Intern a UTF-8 string and return the managed string value.
    pub fn intern_string(&mut self, value: &str) -> Result<Value, Error> {
        let heap = self.memory.heap();
        self.string_interner.try_intern_string_literal(heap, value)
    }

    /// Intern a UTF-8 string and return the managed string handle.
    pub fn string_handle(&mut self, value: &str) -> Result<StringHandle, Error> {
        let value = self.intern_string(value)?;

        Ok(StringHandle::new(value))
    }

    /// Read a UTF-8 string value from the heap.
    pub fn string_value(&self, value: Value) -> Result<String, Error> {
        self.string_interner.string_value(self.heap_ref(), value)
    }

    /// Read a UTF-8 string view from the heap.
    pub fn string_value_ref(&self, value: Value) -> Result<StringRef<'_>, Error> {
        self.string_interner
            .string_value_ref(self.heap_ref(), value)
    }

    /// Read a UTF-8 string view from the heap using a string handle.
    pub fn string_ref(&self, value: StringHandle) -> Result<StringRef<'_>, Error> {
        self.string_interner
            .string_value_ref(self.heap_ref(), value.value())
    }

    /// Allocate an aggregate on the heap and return it as a Value.
    pub fn allocate_aggregate(&mut self, values: Vec<Value>) -> Result<Value, Error> {
        let heap = self.memory.heap();
        let handle = heap.allocate_packed_values(values).map_err(Error::from)?;
        Ok(Value::aggregate(handle))
    }

    /// Allocate a 2-element aggregate on the heap.
    pub fn allocate_pair(&mut self, first: Value, second: Value) -> Result<Value, Error> {
        let heap = self.memory.heap();
        let handle = heap
            .allocate_packed_pair(first, second)
            .map_err(Error::from)?;
        Ok(Value::aggregate(handle))
    }

    /// Allocate a 1-element aggregate on the heap.
    pub fn allocate_single(&mut self, value: Value) -> Result<Value, Error> {
        let heap = self.memory.heap();
        let handle = heap.allocate_packed_single(value).map_err(Error::from)?;
        Ok(Value::aggregate(handle))
    }

    /// Allocate a raw heap byte buffer and return its pointer.
    pub fn allocate_raw_bytes(&mut self, bytes: &[u8]) -> Result<RawPointer, Error> {
        let heap = self.memory.heap();
        heap.allocate_raw_bytes(bytes).map_err(Error::from)
    }

    /// Allocate one raw packed-value buffer and return its pointer.
    pub fn allocate_raw_values(&mut self, values: Vec<Value>) -> Result<RawPointer, Error> {
        self.heap().allocate_raw_values(values).map_err(Error::from)
    }

    /// Allocate a shared heap byte region and return its pointer.
    pub fn allocate_shared_bytes(&mut self, bytes: &[u8]) -> Result<SharedPointer, Error> {
        self.memory
            .allocate_shared_bytes(bytes)
            .map_err(Error::from)
    }

    /// Read aggregate slots from the heap.
    pub fn aggregate_slots(&self, value: Value) -> Result<Vec<Value>, Error> {
        if value.tag() != ValueTag::Aggregate {
            return Err(Error::TypeMismatch {
                expected: "aggregate".to_string(),
                actual: format!("{:?}", value.tag()),
            });
        }
        let handle = value
            .as_managed_reference()
            .ok_or(Error::InvalidManagedReference)?;
        let slots = self
            .heap_ref()
            .packed_values_to_vec(handle)
            .ok_or(Error::InvalidManagedReference)?;
        Ok(slots)
    }

    /// Write aggregate slots back into one heap aggregate.
    pub fn write_aggregate_slots(&mut self, value: Value, values: &[Value]) -> Result<(), Error> {
        if value.tag() != ValueTag::Aggregate {
            return Err(Error::TypeMismatch {
                expected: "aggregate".to_string(),
                actual: format!("{:?}", value.tag()),
            });
        }

        let handle = value
            .as_managed_reference()
            .ok_or(Error::InvalidManagedReference)?;

        self.heap()
            .resize_packed_values(handle, values.len())
            .map_err(Error::from)?;

        for (index, value) in values.iter().copied().enumerate() {
            if !self.heap().set_packed_value(handle, index, value) {
                return Err(Error::InvalidManagedReference);
            }
        }

        Ok(())
    }

    /// Read raw bytes from a pointer to a bytes cell.
    pub fn raw_bytes(&self, pointer: RawPointer) -> Result<Vec<u8>, Error> {
        self.heap_ref()
            .raw_bytes_to_vec(pointer)
            .ok_or(Error::InvalidManagedReference)
    }

    /// Read the raw byte length for one pointer.
    pub fn raw_byte_len(&self, pointer: RawPointer) -> Result<usize, Error> {
        self.heap_ref()
            .raw_byte_len(pointer)
            .ok_or(Error::InvalidManagedReference)
    }

    /// Read raw packed values from one pointer.
    pub fn raw_values(&self, pointer: RawPointer) -> Result<Vec<Value>, Error> {
        self.heap_ref()
            .raw_values(pointer)
            .ok_or(Error::InvalidManagedReference)
    }

    /// Read shared bytes from a pointer to one shared region.
    pub fn shared_bytes(&self, pointer: SharedPointer) -> Result<Vec<u8>, Error> {
        self.shared_ref()
            .bytes_to_vec(pointer)
            .ok_or(Error::InvalidManagedReference)
    }

    /// Write raw bytes into a pointer to a bytes cell.
    pub fn write_raw_bytes(&mut self, pointer: RawPointer, bytes: &[u8]) -> Result<(), Error> {
        if self
            .heap()
            .replace_raw_bytes(pointer, bytes)
            .map_err(Error::from)?
        {
            return Ok(());
        }

        Err(Error::InvalidManagedReference)
    }

    /// Write raw packed values into one pointer.
    pub fn write_raw_values(&mut self, pointer: RawPointer, values: &[Value]) -> Result<(), Error> {
        if self
            .heap()
            .replace_raw_values(pointer, values)
            .map_err(Error::from)?
        {
            return Ok(());
        }

        Err(Error::InvalidManagedReference)
    }

    /// Write shared bytes into a pointer to one shared region.
    pub fn write_shared_bytes(
        &mut self,
        pointer: SharedPointer,
        bytes: &[u8],
    ) -> Result<(), Error> {
        if self
            .memory
            .replace_shared_bytes(pointer, bytes)
            .map_err(Error::from)?
        {
            return Ok(());
        }

        Err(Error::InvalidManagedReference)
    }
}

impl fmt::Debug for ExternalCallContext<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ExternalCallContext")
            .field("string_interner", &"<isolate>")
            .finish()
    }
}
