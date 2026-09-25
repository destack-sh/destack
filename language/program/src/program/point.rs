use std::cmp::Ordering;

use serde::{Deserialize, Serialize};
use tspp_core::SectionEntry;
use tspp_serde::Reflect;

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

/// One logical frame coordinate in a linked program.
#[repr(C, u32)]
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect, SectionEntry,
)]
pub enum FramePoint {
    /// State before a coroutine executes its first operation.
    Entry {
        /// The coroutine function.
        function: FunctionId,
    },
    /// State at one executable operation.
    Operation(ProgramPoint),
}

impl FramePoint {
    /// Create one coroutine entry coordinate.
    pub const fn entry(function: FunctionId) -> Self {
        Self::Entry { function }
    }

    /// Create one executable operation coordinate.
    pub const fn operation(point: ProgramPoint) -> Self {
        Self::Operation(point)
    }

    /// Return the function containing this coordinate.
    pub const fn function(self) -> FunctionId {
        match self {
            Self::Entry { function } => function,
            Self::Operation(point) => point.function,
        }
    }

    /// Return the program point when this is executable state.
    pub const fn operation_point(self) -> Option<ProgramPoint> {
        match self {
            Self::Entry { .. } => None,
            Self::Operation(point) => Some(point),
        }
    }

    /// Return the ordering key used by dense frame state tables.
    const fn sort_key(self) -> (FunctionId, u32, u32) {
        match self {
            Self::Entry { function } => (function, 0, 0),
            Self::Operation(point) => (point.function, 1, point.operation),
        }
    }
}

impl PartialOrd for FramePoint {
    /// Compare frame coordinates in function, entry, operation order.
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for FramePoint {
    /// Compare frame coordinates in function, entry, operation order.
    fn cmp(&self, other: &Self) -> Ordering {
        self.sort_key().cmp(&other.sort_key())
    }
}
