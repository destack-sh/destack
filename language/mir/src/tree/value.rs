use serde::{Deserialize, Serialize};
use tspp_serde::Reflect;

use crate::TypeId;

/// Compact reference to a value list stored in the MIR tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, Reflect)]
pub struct ValueSlice {
    /// Start index in the value buffer.
    pub start: u32,
    /// Number of values in the slice.
    pub count: u16,
}

impl ValueSlice {
    /// Create a new value slice.
    #[inline]
    pub const fn new(start: u32, count: u16) -> Self {
        Self { start, count }
    }

    /// Return whether this slice is empty.
    #[inline]
    pub const fn is_empty(&self) -> bool {
        self.count == 0
    }

    /// Return the number of values in this slice.
    #[inline]
    pub const fn len(&self) -> usize {
        self.count as usize
    }
}

/// SSA value (virtual register).
#[repr(transparent)]
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, Reflect,
)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct TypedValue {
    /// The SSA value.
    pub value: Value,
    /// The type of the value.
    pub ty: TypeId,
}

impl TypedValue {
    /// Create a new typed value.
    pub fn new(value: Value, ty: TypeId) -> Self {
        Self { value, ty }
    }
}
