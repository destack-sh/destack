use std::cmp::Ordering;

use serde::{Deserialize, Serialize};
use tspp_mir as mir;
use tspp_serde::Reflect;

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

    /// Return the following operation in the same function.
    pub const fn next(self) -> Self {
        Self::new(self.function, self.operation + 1)
    }
}

/// One logical frame coordinate in an emitted object.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub enum FramePoint {
    /// State before a coroutine executes its first operation.
    Entry {
        /// The object-local coroutine function.
        function: mir::FunctionId,
    },
    /// State at one executable operation.
    Operation(Point),
}

impl FramePoint {
    /// Create one coroutine entry coordinate.
    pub const fn entry(function: mir::FunctionId) -> Self {
        Self::Entry { function }
    }

    /// Create one executable operation coordinate.
    pub const fn operation(point: Point) -> Self {
        Self::Operation(point)
    }

    /// Return the object-local function containing this coordinate.
    pub const fn function(self) -> mir::FunctionId {
        match self {
            Self::Entry { function } => function,
            Self::Operation(point) => point.function,
        }
    }
}

impl PartialOrd for FramePoint {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for FramePoint {
    fn cmp(&self, other: &Self) -> Ordering {
        // order functions first so every function owns one contiguous frame range
        let function = self.function().cmp(&other.function());
        if function != Ordering::Equal {
            return function;
        }

        // order the entry before executable operations inside one function
        match (self, other) {
            (Self::Entry { .. }, Self::Entry { .. }) => Ordering::Equal,
            (Self::Entry { .. }, Self::Operation(_)) => Ordering::Less,
            (Self::Operation(_), Self::Entry { .. }) => Ordering::Greater,
            (Self::Operation(left), Self::Operation(right)) => left.operation.cmp(&right.operation),
        }
    }
}
