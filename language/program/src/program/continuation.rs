use std::sync::Arc;

use destack_heap::{HeapResult, RootSlot};
use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use crate::{Error, Result};

use super::{FrameStateId, FunctionId, Program, Word};

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

/// Root completion mode preserved by one suspended continuation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub enum Completion {
    /// Publish the function's returned value.
    Return,
    /// Discard the terminal value after cancellation cleanup.
    Cancel,
}

/// Runtime continuations addressed by generation-checked handles.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContinuationTable {
    /// Stable slots addressed by continuation ids.
    slots: Vec<ContinuationSlot>,
    /// Vacant slot indices available for reuse.
    vacant: Vec<u32>,
}

/// One ready or suspended coroutine execution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContinuationEntry {
    /// A coroutine body that has not entered its first instruction.
    Ready {
        /// The coroutine body function.
        function: FunctionId,
        /// Captured arguments in parameter order.
        arguments: Arc<[Word]>,
    },
    /// A coroutine body suspended at an await or yield operation.
    Suspended(Continuation),
}

/// One generation-checked continuation identity.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct ContinuationId(u64);

/// One reusable continuation table slot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct ContinuationSlot {
    /// Generation required by the current id.
    generation: u32,
    /// Live continuation when this slot is occupied.
    continuation: Option<ContinuationEntry>,
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

    /// Return frame states and mutable canonical bytes.
    pub(crate) fn state_bytes_mut(&mut self) -> (&[FrameStateId], &mut [u8]) {
        (&self.states, Arc::make_mut(&mut self.bytes))
    }
}

impl ContinuationTable {
    /// Insert one continuation and return its runtime identity.
    pub fn insert(&mut self, continuation: ContinuationEntry) -> ContinuationId {
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
    pub fn take(&mut self, id: ContinuationId) -> Option<ContinuationEntry> {
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

            match continuation {
                ContinuationEntry::Ready {
                    function,
                    arguments,
                } => {
                    let arguments = Arc::make_mut(arguments);
                    Self::visit_arguments(program, *function, arguments, visit)?;
                }
                ContinuationEntry::Suspended(continuation) => {
                    program.visit_continuation_root_slots(continuation, visit)?;
                }
            }
        }

        Ok(())
    }

    /// Visit captured arguments through the coroutine function parameters.
    fn visit_arguments(
        program: &Program,
        function: FunctionId,
        arguments: &mut [Word],
        visit: &mut dyn FnMut(RootSlot<'_>) -> HeapResult<()>,
    ) -> Result<()> {
        let parameters = program
            .function_parameters(function)
            .ok_or_else(|| Error::undefined_function(function))?;
        let function_entry = program
            .function(function)
            .ok_or_else(|| Error::undefined_function(function))?;
        let environment = function_entry.environment();
        let mut expected_word_count = 0usize;

        // determine the complete captured argument width
        let types = environment.into_iter().chain(parameters.iter().copied());
        for ty in types {
            let byte_len = program
                .type_byte_len(ty)
                .ok_or_else(|| Error::undefined_type(ty))?;
            expected_word_count += byte_len.div_ceil(Word::BYTE_LEN);
        }
        if arguments.len() != expected_word_count {
            return Err(Error::ContinuationWordCountMismatch {
                function,
                expected: expected_word_count,
                actual: arguments.len(),
            });
        }

        // visit each captured value through its linked program type
        let bytes = Word::bytes_mut(arguments);
        let mut word_offset = 0usize;
        let types = environment.into_iter().chain(parameters.iter().copied());
        for ty in types {
            let byte_len = program
                .type_byte_len(ty)
                .ok_or_else(|| Error::undefined_type(ty))?;
            let word_count = byte_len.div_ceil(Word::BYTE_LEN);
            let byte_offset = word_offset * Word::BYTE_LEN;
            let argument = &mut bytes[byte_offset..byte_offset + byte_len];
            program.visit_byte_root_slots(ty, argument, visit)?;
            word_offset += word_count;
        }

        Ok(())
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
