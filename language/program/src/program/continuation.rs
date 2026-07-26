use std::sync::Arc;

use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use super::FrameStateId;

/// Runtime identity for one parked asynchronous continuation.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct Waiter(u64);

/// Root completion mode preserved by one suspended continuation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub enum Completion {
    /// Publish the function's returned value.
    Return,
    /// Discard the terminal value after cancellation cleanup.
    Cancel,
}

/// One suspended coroutine call chain.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct Continuation {
    /// Root completion mode for the suspended call chain.
    completion: Completion,
    /// Canonical frame states in caller to callee order.
    states: Arc<[FrameStateId]>,
    /// Canonical live frame bytes in the same order.
    bytes: Arc<[u8]>,
}

impl Continuation {
    /// Create one suspended coroutine call chain.
    pub fn new(
        completion: Completion,
        states: impl Into<Arc<[FrameStateId]>>,
        bytes: impl Into<Arc<[u8]>>,
    ) -> Self {
        Self {
            completion,
            states: states.into(),
            bytes: bytes.into(),
        }
    }

    /// Return the root completion mode.
    pub const fn completion(&self) -> Completion {
        self.completion
    }

    /// Fork this continuation through copy-on-write frame storage.
    pub fn fork(&self) -> Self {
        self.clone()
    }

    /// Return canonical frame states in caller to callee order.
    pub fn states(&self) -> &[FrameStateId] {
        &self.states
    }

    /// Return the innermost suspended frame state.
    pub fn innermost(&self) -> Option<FrameStateId> {
        self.states.last().copied()
    }

    /// Return canonical live frame bytes in caller to callee order.
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Return frame states and mutable bytes through copy-on-write storage.
    pub(crate) fn parts_mut(&mut self) -> (&[FrameStateId], &mut [u8]) {
        (&self.states, Arc::make_mut(&mut self.bytes))
    }
}

impl Waiter {
    /// Create one waiter from its slot and generation.
    pub const fn new(index: u32, generation: u32) -> Self {
        Self(((generation as u64) << u32::BITS) | index as u64)
    }

    /// Return the waiter table slot.
    pub const fn index(self) -> u32 {
        self.0 as u32
    }

    /// Return the waiter slot generation.
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
