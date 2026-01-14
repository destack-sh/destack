use std::fmt;
use std::ptr::NonNull;

use crate::diagnostic::Error;
use crate::memory::{RawPointer, Value};

use super::IsolateState;

/// Handler invoked by the VM when calling an external function.
pub trait ExternalHandler:
    for<'ctx> Fn(&mut ExternalContext<'ctx>, &[Value]) -> Result<Value, Error> + Send + Sync
{
}

impl<T> ExternalHandler for T where
    T: for<'ctx> Fn(&mut ExternalContext<'ctx>, &[Value]) -> Result<Value, Error> + Send + Sync
{
}

/// Boxed external handler type.
pub type ExternalFn = Box<dyn ExternalHandler>;

/// Cached external handler pointer.
pub(crate) type ExternalFnPtr = NonNull<dyn ExternalHandler>;

/// External call context with restricted access to isolate state.
pub struct ExternalContext<'ctx> {
    /// The isolate state backing this external call.
    state: &'ctx mut IsolateState,
}

impl<'ctx> ExternalContext<'ctx> {
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

    /// Allocate an aggregate on the heap and return it as a Value.
    pub fn allocate_aggregate(&mut self, values: Vec<Value>) -> Value {
        self.state.allocate_aggregate(values)
    }

    /// Allocate a raw heap cell with value slots and return its pointer.
    pub fn allocate_raw_values(&mut self, values: Vec<Value>) -> RawPointer {
        self.state.allocate_raw_values(values)
    }
}

impl fmt::Debug for ExternalContext<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ExternalContext")
            .field("state", &"<isolate>")
            .finish()
    }
}
