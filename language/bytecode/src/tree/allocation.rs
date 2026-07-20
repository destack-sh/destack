use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

/// One allocation operation.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub enum AllocationOperation {
    /// One fixed-size value.
    Value = 0,
    /// One variable-length slice.
    Slice = 1,
}

/// One allocation initialization mode.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub enum AllocationInitialization {
    /// Storage initialized with the zero bit pattern.
    Zeroed = 0,
    /// Storage whose bytes are not initialized yet.
    Uninit = 1,
}
