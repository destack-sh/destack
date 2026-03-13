use std::fmt;
use std::ptr::NonNull;

use crate::diagnostic::Error;
use destack_heap::{Heap, RawPointer, Value, ValueTag};

use super::IsolateState;

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
    /// The isolate state backing this external call.
    state: &'ctx mut IsolateState,
    /// The heap backing this external call.
    heap: &'ctx mut Heap,
}

impl<'ctx> ExternalCallContext<'ctx> {
    /// Wrap an isolate state for external calls.
    pub(crate) fn new(state: &'ctx mut IsolateState, heap: &'ctx mut Heap) -> Self {
        Self { state, heap }
    }

    /// Intern a UTF-8 string and return the managed string value.
    pub fn intern_string(&mut self, value: &str) -> Result<Value, Error> {
        self.state.try_intern_string_literal(self.heap, value)
    }

    /// Intern a UTF-8 string and return the managed string handle.
    pub fn string_handle(&mut self, value: &str) -> Result<super::StringHandle, Error> {
        let value = self.intern_string(value)?;

        Ok(super::StringHandle::new(value))
    }

    /// Read a UTF-8 string value from the heap.
    pub fn string_value(&self, value: Value) -> Result<String, Error> {
        self.state.string_value(self.heap, value)
    }

    /// Read a UTF-8 string view from the heap.
    pub fn string_value_ref(&self, value: Value) -> Result<super::StringRef<'_>, Error> {
        self.state.string_value_ref(self.heap, value)
    }

    /// Read a UTF-8 string view from the heap using a string handle.
    pub fn string_ref(&self, value: super::StringHandle) -> Result<super::StringRef<'_>, Error> {
        self.state.string_value_ref(self.heap, value.value())
    }

    /// Allocate an aggregate on the heap and return it as a Value.
    pub fn allocate_aggregate(&mut self, values: Vec<Value>) -> Result<Value, Error> {
        self.state.try_allocate_aggregate(self.heap, values)
    }

    /// Allocate a 2-element aggregate on the heap.
    pub fn allocate_pair(&mut self, first: Value, second: Value) -> Result<Value, Error> {
        self.state.try_allocate_pair(self.heap, first, second)
    }

    /// Allocate a 1-element aggregate on the heap.
    pub fn allocate_single(&mut self, value: Value) -> Result<Value, Error> {
        self.state.try_allocate_single(self.heap, value)
    }

    /// Allocate a raw heap cell with value slots and return its pointer.
    pub fn allocate_raw_values(&mut self, values: Vec<Value>) -> Result<RawPointer, Error> {
        self.state.try_allocate_raw_values(self.heap, values)
    }

    /// Allocate a raw heap byte buffer and return its pointer.
    pub fn allocate_raw_bytes(&mut self, bytes: &[u8]) -> Result<RawPointer, Error> {
        self.state.try_allocate_raw_bytes(self.heap, bytes)
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
            .heap
            .managed_slots_to_vec(handle)
            .ok_or(Error::InvalidManagedReference)?;
        Ok(slots)
    }

    /// Read raw values from a pointer to a values cell.
    pub fn raw_values(&self, pointer: RawPointer) -> Result<Vec<Value>, Error> {
        self.heap
            .raw_values(pointer)
            .map(|values| values.to_vec())
            .ok_or(Error::TypeMismatch {
                expected: "values".to_string(),
                actual: "bytes".to_string(),
            })
    }

    /// Read raw bytes from a pointer to a bytes cell.
    pub fn raw_bytes(&self, pointer: RawPointer) -> Result<Vec<u8>, Error> {
        self.heap
            .raw_bytes_to_vec(pointer)
            .ok_or(Error::TypeMismatch {
                expected: "bytes".to_string(),
                actual: "values".to_string(),
            })
    }

    /// Write raw values into a pointer to a values cell.
    pub fn write_raw_values(&mut self, pointer: RawPointer, values: &[Value]) -> Result<(), Error> {
        if self
            .heap
            .replace_raw_values(pointer, values)
            .map_err(Error::from)?
        {
            return Ok(());
        }

        Err(Error::TypeMismatch {
            expected: "values".to_string(),
            actual: "bytes".to_string(),
        })
    }

    /// Write raw bytes into a pointer to a bytes cell.
    pub fn write_raw_bytes(&mut self, pointer: RawPointer, bytes: &[u8]) -> Result<(), Error> {
        if self.heap.raw_is_bytes(pointer) != Some(true) {
            return Err(Error::TypeMismatch {
                expected: "bytes".to_string(),
                actual: "values".to_string(),
            });
        }

        if self
            .heap
            .replace_raw_bytes(pointer, bytes)
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
            .field("state", &"<isolate>")
            .finish()
    }
}
