use destack_heap::{HeapResult, RootSlot};
use destack_memory::{MemoryMap, MemoryRange};
use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use crate::Result;

use super::{ActivationImage, Completion, FrameImage, Program, Word};

/// One suspended coroutine call chain.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct Continuation {
    /// Retained engine-neutral activation.
    image: ActivationImage,
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
    /// Create one suspended coroutine call chain.
    pub const fn new(image: ActivationImage) -> Self {
        Self { image }
    }

    /// Return the root completion mode.
    pub const fn completion(&self) -> Completion {
        self.image.completion()
    }

    /// Inherit this continuation into an already-forked memory map.
    pub fn inherit(&self) -> Self {
        Self::new(self.image.inherit())
    }

    /// Release this continuation's frame bytes.
    pub fn release(self, memory: &MemoryMap) -> Result<()> {
        self.image.release(memory)
    }

    /// Return retained frames in caller to callee order.
    pub fn frames(&self) -> &[FrameImage] {
        self.image.frames()
    }

    /// Return the innermost frame.
    pub fn innermost(&self) -> Option<FrameImage> {
        self.image.frames().last().copied()
    }

    /// Return packed frame bytes inside the owning MemoryMap.
    pub const fn memory(&self) -> MemoryRange {
        self.image.memory()
    }
}

impl ContinuationTable {
    /// Inherit this continuation table into an already-forked memory map.
    pub fn inherit(&self) -> Self {
        Self {
            slots: self.slots.iter().map(ContinuationSlot::inherit).collect(),
            vacant: self.vacant.clone(),
        }
    }

    /// Release every live continuation's frame bytes.
    pub fn release(self, memory: &MemoryMap) -> Result<()> {
        let mut error = None;

        for slot in self.slots {
            let Some(continuation) = slot.continuation else {
                continue;
            };

            if let Err(current) = continuation.release(memory)
                && error.is_none()
            {
                error = Some(current);
            }
        }

        if let Some(error) = error {
            return Err(error);
        }

        Ok(())
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

    /// Return one continuation selected by its exact generation.
    pub fn get(&self, id: ContinuationId) -> Option<&Continuation> {
        let slot = self.slots.get(id.index() as usize)?;
        if slot.generation != id.generation() {
            return None;
        }

        slot.continuation.as_ref()
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
        &self,
        program: &Program,
        memory: &MemoryMap,
        visit: &mut dyn FnMut(RootSlot<'_>) -> HeapResult<()>,
    ) -> Result<()> {
        for slot in &self.slots {
            let Some(continuation) = &slot.continuation else {
                continue;
            };

            program.visit_continuation_root_slots(memory, continuation, visit)?;
        }

        Ok(())
    }
}

impl ContinuationSlot {
    /// Inherit this occupied or vacant continuation slot.
    fn inherit(&self) -> Self {
        Self {
            generation: self.generation,
            continuation: self.continuation.as_ref().map(Continuation::inherit),
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
