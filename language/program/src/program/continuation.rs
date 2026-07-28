use std::sync::Arc;

use destack_heap::{HeapResult, RootSlot};
use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use crate::Result;

use super::{FrameStateId, Program, Word};

/// One canonical coroutine call chain.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct Continuation {
    /// Root completion mode for the call chain.
    completion: Completion,
    /// Canonical frame states in caller to callee order.
    states: Arc<[FrameStateId]>,
    /// Canonical live frame bytes in the same order.
    bytes: Arc<[u8]>,
}

/// Root completion mode preserved by one coroutine call chain.
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub enum Completion {
    /// Publish the function's returned value.
    Return,
    /// Discard the terminal value after cancellation cleanup.
    Cancel,
}

/// Runtime continuations addressed by generation-checked handles.
#[derive(Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContinuationTable {
    /// Stable slots addressed by continuation ids.
    slots: Vec<ContinuationSlot>,
    /// Vacant slot indices available for reuse.
    vacant: Vec<u32>,
}

/// One generation-checked continuation identity.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct ContinuationId(u64);

/// One reusable continuation table slot.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
struct ContinuationSlot {
    /// Generation required by the current id.
    generation: u32,
    /// Live continuation when this slot is occupied.
    continuation: Option<Continuation>,
}

impl Continuation {
    /// Create one canonical coroutine call chain.
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
        Self {
            completion: self.completion,
            states: self.states.clone(),
            bytes: self.bytes.clone(),
        }
    }

    /// Return canonical frame states in caller to callee order.
    pub fn states(&self) -> &[FrameStateId] {
        &self.states
    }

    /// Return the innermost frame state.
    pub fn innermost(&self) -> Option<FrameStateId> {
        self.states.last().copied()
    }

    /// Return canonical live frame bytes in caller to callee order.
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Return frame states and mutable canonical bytes.
    pub(crate) fn state_bytes_mut(&mut self) -> (&[FrameStateId], &mut [u8]) {
        (&self.states, Arc::make_mut(&mut self.bytes))
    }
}

impl ContinuationTable {
    /// Fork this continuation table through copy-on-write continuation storage.
    pub fn fork(&self) -> Self {
        Self {
            slots: self.slots.iter().map(ContinuationSlot::fork).collect(),
            vacant: self.vacant.clone(),
        }
    }

    /// Insert one continuation and return its runtime identity.
    pub fn insert(&mut self, continuation: Continuation) -> ContinuationId {
        let Some(index) = self.vacant.pop() else {
            let index = self.slots.len() as u32;
            self.slots.push(ContinuationSlot {
                generation: 1,
                continuation: Some(continuation),
            });

            return ContinuationId::new(index, 1);
        };
        let slot = &mut self.slots[index as usize];
        slot.continuation = Some(continuation);

        ContinuationId::new(index, slot.generation)
    }

    /// Consume one continuation selected by its exact generation.
    pub fn take(&mut self, id: ContinuationId) -> Option<Continuation> {
        let index = id.index();
        let slot = self.slots.get_mut(index as usize)?;
        if slot.generation != id.generation() {
            return None;
        }
        let continuation = slot.continuation.take()?;
        slot.generation = slot.generation.wrapping_add(1).max(1);
        self.vacant.push(index);

        Some(continuation)
    }

    /// Iterate over every live continuation and its exact runtime identity.
    pub fn iter(&self) -> impl Iterator<Item = (ContinuationId, &Continuation)> {
        self.slots.iter().enumerate().filter_map(|(index, slot)| {
            let continuation = slot.continuation.as_ref()?;
            let id = ContinuationId::new(index as u32, slot.generation);

            Some((id, continuation))
        })
    }

    /// Visit mutable heap roots retained by every live continuation.
    pub fn visit_root_slots(
        &mut self,
        program: &Program,
        visit: &mut dyn FnMut(RootSlot<'_>) -> HeapResult<()>,
    ) -> Result<()> {
        for slot in &mut self.slots {
            let Some(continuation) = &mut slot.continuation else {
                continue;
            };

            program.visit_continuation_root_slots(continuation, visit)?;
        }

        Ok(())
    }
}

impl ContinuationSlot {
    /// Fork this occupied or vacant continuation slot.
    fn fork(&self) -> Self {
        Self {
            generation: self.generation,
            continuation: self.continuation.as_ref().map(Continuation::fork),
        }
    }
}

impl ContinuationId {
    /// Decode one continuation identity word.
    pub const fn from_word(word: Word) -> Self {
        Self(word.bits())
    }

    /// Encode this continuation identity as one execution word.
    pub const fn into_word(self) -> Word {
        Word::from_bits(self.0)
    }

    /// Create one identity from its slot index and generation.
    const fn new(index: u32, generation: u32) -> Self {
        Self((generation as u64) << u32::BITS | index as u64)
    }

    /// Return the dense slot index.
    const fn index(self) -> u32 {
        self.0 as u32
    }

    /// Return the required slot generation.
    const fn generation(self) -> u32 {
        (self.0 >> u32::BITS) as u32
    }
}
