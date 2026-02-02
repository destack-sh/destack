use std::fmt;
use std::ptr::NonNull;

use crate::diagnostic::Error;
use crate::memory::{RawPointer, Value, ValueTag};

use super::IsolateState;

/// Handler invoked by the VM when calling an external function.
pub trait ExternalHandler:
    for<'ctx> Fn(&mut RuntimeContext<'ctx>, &[Value]) -> Result<Value, Error> + Send + Sync
{
}

impl<T> ExternalHandler for T where
    T: for<'ctx> Fn(&mut RuntimeContext<'ctx>, &[Value]) -> Result<Value, Error> + Send + Sync
{
}

/// Boxed external handler type.
pub type ExternalFn = Box<dyn ExternalHandler>;

/// Cached external handler pointer.
pub(crate) type ExternalFnPtr = NonNull<dyn ExternalHandler>;

/// Runtime call context with restricted access to isolate state.
pub struct RuntimeContext<'ctx> {
    /// The isolate state backing this external call.
    state: &'ctx mut IsolateState,
}

impl<'ctx> RuntimeContext<'ctx> {
    /// Wrap an isolate state for external calls.
    pub(crate) fn new(state: &'ctx mut IsolateState) -> Self {
        Self { state }
    }

    /// Intern a UTF-8 string and return the managed string value.
    pub fn intern_string(&mut self, value: &str) -> Value {
        self.state.intern_string_literal(value)
    }

    /// Read a UTF-8 string value from the heap.
    pub fn string_value(&self, value: Value) -> Result<String, Error> {
        self.state.string_value(value)
    }

    /// Read a UTF-8 string view from the heap.
    pub fn string_value_ref(&self, value: Value) -> Result<super::StringRef<'_>, Error> {
        self.state.string_value_ref(value)
    }

    /// Read a UTF-8 string view from the heap using a string handle.
    pub fn string_ref(&self, value: super::StringHandle) -> Result<super::StringRef<'_>, Error> {
        self.state.string_value_ref(value.value())
    }

    /// Allocate an aggregate on the heap and return it as a Value.
    pub fn allocate_aggregate(&mut self, values: Vec<Value>) -> Value {
        self.state.allocate_aggregate(values)
    }

    /// Allocate a 2-element aggregate on the heap.
    pub fn allocate_pair(&mut self, first: Value, second: Value) -> Value {
        self.state.allocate_pair(first, second)
    }

    /// Allocate a 1-element aggregate on the heap.
    pub fn allocate_single(&mut self, value: Value) -> Value {
        self.state.allocate_single(value)
    }

    /// Allocate a raw heap cell with value slots and return its pointer.
    pub fn allocate_raw_values(&mut self, values: Vec<Value>) -> RawPointer {
        self.state.allocate_raw_values(values)
    }

    /// Read aggregate slots from the heap.
    pub fn aggregate_slots(&self, value: Value) -> Result<Vec<Value>, Error> {
        if value.tag() != ValueTag::Aggregate {
            return Err(Error::TypeMismatch {
                expected: "aggregate".to_string(),
                actual: format!("{:?}", value.tag()),
            });
        }
        let handle = value.as_heap_handle().ok_or(Error::InvalidHeapHandle)?;
        let heap = self.state.heap_borrow();
        let cell = heap.managed.get(handle).ok_or(Error::InvalidHeapHandle)?;
        Ok(cell.slots.as_slice().to_vec())
    }
}

impl fmt::Debug for RuntimeContext<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RuntimeContext")
            .field("state", &"<isolate>")
            .finish()
    }
}
