//! Call frame for the interpreter.

use std::collections::HashMap;

use destack_mir as mir;

use crate::diagnostic::{Error, Result};
use crate::memory::Value;

/// A call frame in the interpreter.
///
/// Each function call creates a new frame that holds the local values
/// and tracks the current execution position.
#[derive(Debug)]
pub struct Frame {
    /// The function being executed.
    pub function: mir::LocalNodeId<mir::Function>,
    /// The current block being executed.
    pub current_block: mir::LocalNodeId<mir::Block>,
    /// Values in this frame (SSA values).
    values: HashMap<mir::Value, Value>,
    /// Local variables (stack slots).
    locals: HashMap<mir::LocalNodeId<mir::Local>, Value>,
}

impl Frame {
    /// Create a new frame for a function.
    pub fn new(
        function: mir::LocalNodeId<mir::Function>,
        entry_block: mir::LocalNodeId<mir::Block>,
    ) -> Self {
        Self {
            function,
            current_block: entry_block,
            values: HashMap::new(),
            locals: HashMap::new(),
        }
    }

    /// Get a value from this frame.
    pub fn get_value(&self, value: mir::Value) -> Result<Value> {
        self.values
            .get(&value)
            .cloned()
            .ok_or(Error::UndefinedValue { value })
    }

    /// Set a value in this frame.
    pub fn set_value(&mut self, value: mir::Value, val: Value) {
        self.values.insert(value, val);
    }

    /// Get a local variable.
    pub fn get_local(&self, local: mir::LocalNodeId<mir::Local>) -> Result<Value> {
        self.locals
            .get(&local)
            .cloned()
            .ok_or_else(|| Error::UndefinedValue {
                value: mir::Value::new(local.id),
            })
    }

    /// Set a local variable.
    pub fn set_local(&mut self, local: mir::LocalNodeId<mir::Local>, value: Value) {
        self.locals.insert(local, value);
    }

    /// Check if a value is defined in this frame.
    pub fn has_value(&self, value: mir::Value) -> bool {
        self.values.contains_key(&value)
    }

    /// Clear all values (but keep locals).
    pub fn clear_values(&mut self) {
        self.values.clear();
    }
}
