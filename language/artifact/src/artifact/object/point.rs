use destack_mir as mir;
use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

/// One logical operation in an emitted object.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Reflect,
)]
pub struct Point {
    /// The object-local function.
    pub function: mir::FunctionId,
    /// The operation index inside the function.
    pub operation: u32,
}

impl Point {
    /// Create one object-local operation point.
    pub const fn new(function: mir::FunctionId, operation: u32) -> Self {
        Self {
            function,
            operation,
        }
    }
}
