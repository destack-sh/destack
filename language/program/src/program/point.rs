use destack_core::SectionEntry;
use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use super::FunctionId;

/// One executable program point.
#[repr(C)]
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Reflect,
)]
pub struct ProgramPoint {
    /// The owning function.
    pub function: FunctionId,
    /// The executable operation index inside the function.
    pub operation: u32,
}

impl ProgramPoint {
    /// Create one executable program point.
    pub const fn new(function: FunctionId, operation: u32) -> Self {
        Self {
            function,
            operation,
        }
    }
}

// SAFETY: program points are fixed-width program section entries.
unsafe impl SectionEntry for ProgramPoint {}
