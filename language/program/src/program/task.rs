use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use crate::Value;

/// Runtime identity for one eager asynchronous task.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct Task(u64);

impl Task {
    /// Create one task identity from its slot and generation.
    pub const fn new(index: u32, generation: u32) -> Self {
        Self(((generation as u64) << u32::BITS) | index as u64)
    }

    /// Return the task table slot.
    pub const fn index(self) -> u32 {
        self.0 as u32
    }

    /// Return the task slot generation.
    pub const fn generation(self) -> u32 {
        (self.0 >> u32::BITS) as u32
    }

    /// Return the scalar handle passed through program code.
    pub const fn bits(self) -> u64 {
        self.0
    }

    /// Decode one scalar handle passed through program code.
    pub const fn from_bits(bits: u64) -> Self {
        Self(bits)
    }
}

/// The terminal outcome of one asynchronous task.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub enum TaskOutcome {
    /// The task completed with a value.
    Completed(Value),
    /// The task completed through cancellation.
    Cancelled,
}
