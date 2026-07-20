use destack_core::SectionEntry;
use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use super::FunctionId;

/// One operation in a linked program.
#[repr(C)]
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Serialize,
    Deserialize,
    Reflect,
    SectionEntry,
)]
pub struct ProgramPoint {
    /// The owning function.
    pub function: FunctionId,
    /// The operation index inside the function.
    pub operation: u32,
}

impl ProgramPoint {
    /// Create one program point.
    pub const fn new(function: FunctionId, operation: u32) -> Self {
        Self {
            function,
            operation,
        }
    }
}
