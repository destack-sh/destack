use serde::{Deserialize, Serialize};

use crate::{LocalNodeId, Type};

/// SSA value (virtual register).
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Value(pub u32);

impl Value {
    /// Create a new value.
    pub fn new(id: u32) -> Self {
        Self(id)
    }

    /// Get the numeric id.
    pub fn id(&self) -> u32 {
        self.0
    }
}

/// One SSA value with its type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TypedValue {
    /// The SSA value.
    pub value: Value,
    /// The type of the value.
    pub ty: LocalNodeId<Type>,
}

impl TypedValue {
    /// Create a new typed value.
    pub fn new(value: Value, ty: LocalNodeId<Type>) -> Self {
        Self { value, ty }
    }
}
