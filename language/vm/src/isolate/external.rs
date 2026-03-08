use std::fmt;
use std::ptr::NonNull;

use crate::diagnostic::Error;
use destack_heap::{Heap, RawCellStorage, RawPointer, SlotStorage, Value, ValueTag};

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
    pub fn intern_string(&mut self, value: &str) -> Value {
        self.state.intern_string_literal(self.heap, value)
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
    pub fn allocate_aggregate(&mut self, values: Vec<Value>) -> Value {
        self.state.allocate_aggregate(self.heap, values)
    }

    /// Allocate a 2-element aggregate on the heap.
    pub fn allocate_pair(&mut self, first: Value, second: Value) -> Value {
        self.state.allocate_pair(self.heap, first, second)
    }

    /// Allocate a 1-element aggregate on the heap.
    pub fn allocate_single(&mut self, value: Value) -> Value {
        self.state.allocate_single(self.heap, value)
    }

    /// Allocate a raw heap cell with value slots and return its pointer.
    pub fn allocate_raw_values(&mut self, values: Vec<Value>) -> RawPointer {
        self.state.allocate_raw_values(self.heap, values)
    }

    /// Allocate a raw heap byte buffer and return its pointer.
    pub fn allocate_raw_bytes(&mut self, bytes: &[u8]) -> RawPointer {
        self.state.allocate_raw_bytes(self.heap, bytes)
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
            .as_managed_pointer()
            .ok_or(Error::InvalidManagedPointer)?;
        let cell = self
            .heap
            .managed()
            .get(handle)
            .ok_or(Error::InvalidManagedPointer)?;
        Ok(cell.slots.as_slice().to_vec())
    }

    /// Read the raw cell storage for a raw pointer.
    pub fn raw_cell_storage(&self, pointer: RawPointer) -> Result<RawCellStorage, Error> {
        let cell = self
            .heap
            .raw()
            .get(pointer)
            .ok_or(Error::InvalidManagedPointer)?;
        Ok(cell.storage.clone())
    }

    /// Read raw values from a pointer to a values cell.
    pub fn raw_values(&self, pointer: RawPointer) -> Result<Vec<Value>, Error> {
        match self.raw_cell_storage(pointer)? {
            RawCellStorage::Values(storage) => Ok(storage.as_slice().to_vec()),
            RawCellStorage::Bytes(_) => Err(Error::TypeMismatch {
                expected: "values".to_string(),
                actual: "bytes".to_string(),
            }),
        }
    }

    /// Read raw bytes from a pointer to a bytes cell.
    pub fn raw_bytes(&self, pointer: RawPointer) -> Result<Vec<u8>, Error> {
        match self.raw_cell_storage(pointer)? {
            RawCellStorage::Bytes(bytes) => Ok(bytes),
            RawCellStorage::Values(_) => Err(Error::TypeMismatch {
                expected: "bytes".to_string(),
                actual: "values".to_string(),
            }),
        }
    }

    /// Write raw values into a pointer to a values cell.
    pub fn write_raw_values(&mut self, pointer: RawPointer, values: &[Value]) -> Result<(), Error> {
        let cell = self
            .heap
            .raw_mut()
            .get_mut(pointer)
            .ok_or(Error::InvalidManagedPointer)?;
        match &mut cell.storage {
            RawCellStorage::Values(_) => {
                cell.storage = RawCellStorage::Values(SlotStorage::from_values(values.to_vec()));
                Ok(())
            }
            RawCellStorage::Bytes(_) => Err(Error::TypeMismatch {
                expected: "values".to_string(),
                actual: "bytes".to_string(),
            }),
        }
    }

    /// Write raw bytes into a pointer to a bytes cell.
    pub fn write_raw_bytes(&mut self, pointer: RawPointer, bytes: &[u8]) -> Result<(), Error> {
        let cell = self
            .heap
            .raw_mut()
            .get_mut(pointer)
            .ok_or(Error::InvalidManagedPointer)?;
        match &mut cell.storage {
            RawCellStorage::Bytes(storage) => {
                storage.clear();
                storage.extend_from_slice(bytes);
                Ok(())
            }
            RawCellStorage::Values(_) => Err(Error::TypeMismatch {
                expected: "bytes".to_string(),
                actual: "values".to_string(),
            }),
        }
    }
}

impl fmt::Debug for ExternalCallContext<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ExternalCallContext")
            .field("state", &"<isolate>")
            .finish()
    }
}
