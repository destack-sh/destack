use std::sync::Arc;

use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use super::FrameStateId;

/// One suspended coroutine call chain.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct Continuation {
    /// Canonical frame states in caller to callee order.
    states: Arc<[FrameStateId]>,
    /// Canonical live frame bytes in the same order.
    bytes: Arc<[u8]>,
}

impl Continuation {
    /// Create one suspended coroutine call chain.
    pub fn new(states: impl Into<Arc<[FrameStateId]>>, bytes: impl Into<Arc<[u8]>>) -> Self {
        Self {
            states: states.into(),
            bytes: bytes.into(),
        }
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
