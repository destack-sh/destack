use std::collections::HashMap;

use destack_mir as mir;
use serde::{Deserialize, Serialize};

use crate::Value;

/// Storage for global variables.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GlobalStorage {
    values: HashMap<mir::LocalNodeId<mir::Global>, Value>,
}

impl GlobalStorage {
    /// Create empty global storage.
    pub fn new() -> Self {
        Self {
            values: HashMap::new(),
        }
    }

    /// Get a global value by id.
    pub fn get(&self, id: mir::LocalNodeId<mir::Global>) -> Option<&Value> {
        self.values.get(&id)
    }

    /// Set a global value.
    pub fn set(&mut self, id: mir::LocalNodeId<mir::Global>, value: Value) {
        self.values.insert(id, value);
    }

    /// Return an iterator over global values.
    pub fn values(&self) -> impl Iterator<Item = &Value> {
        self.values.values()
    }

    /// Return an iterator over mutable global values.
    pub fn values_mut(&mut self) -> impl Iterator<Item = &mut Value> {
        self.values.values_mut()
    }

    /// Get the number of globals.
    pub fn len(&self) -> usize {
        self.values.len()
    }

    /// Check if empty.
    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }
}
