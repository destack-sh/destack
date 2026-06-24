use destack_serde::Reflect;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::FunctionId;
use destack_mir as mir;

/// VM resume states keyed by execution frame state.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, Reflect)]
pub struct ResumeTable {
    /// Resume states by dense frame state id.
    states: Vec<ResumeState>,
    /// Frame state id by lowered program point.
    state_by_point: HashMap<ProgramPoint, mir::FrameStateId>,
}

impl ResumeTable {
    /// Return the number of resume states.
    pub fn len(&self) -> usize {
        self.states.len()
    }

    /// Return whether there are no resume states.
    pub fn is_empty(&self) -> bool {
        self.states.is_empty()
    }

    /// Return the next resume state id.
    pub fn next_id(&self) -> mir::FrameStateId {
        (self.states.len() as u32).into()
    }

    /// Add one resume state.
    pub fn push(&mut self, id: mir::FrameStateId, state: ResumeState) {
        debug_assert_eq!(id.0 as usize, self.states.len());
        self.state_by_point.insert(state.point, id);
        self.states.push(state);
    }

    /// Return one resume state.
    pub fn state(&self, id: mir::FrameStateId) -> Option<&ResumeState> {
        self.states.get(id.0 as usize)
    }

    /// Return one resume state id by lowered program point.
    pub fn state_id_at(&self, point: ProgramPoint) -> Option<mir::FrameStateId> {
        self.state_by_point.get(&point).copied()
    }
}

/// VM state for one resumable frame.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Reflect)]
pub struct ResumeState {
    /// The lowered VM program point.
    pub point: ProgramPoint,
    /// The source MIR point within the lowered block.
    pub mir_point: u32,
    /// Entry bindings for block-entry states.
    pub entry: Option<FrameEntry>,
    /// Caller return destination for post-call states.
    pub return_destination: Option<mir::Value>,
}

/// VM entry bindings for one resume state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct FrameEntry {
    /// The slot bindings applied on entry.
    pub bindings: Vec<FrameBinding>,
    /// The implicit received value slot.
    pub received_value: Option<mir::FrameSlotId>,
}

/// One frame slot binding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct FrameBinding {
    /// The source frame slot.
    pub source: mir::FrameSlotId,
    /// The destination frame slot.
    pub destination: mir::FrameSlotId,
}

/// One lowered VM program point.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct ProgramPoint {
    /// The owning function.
    pub function: FunctionId,
    /// The lowered block index.
    pub block: u32,
    /// The lowered program counter inside the block.
    pub pc: u32,
}

impl ProgramPoint {
    /// Create one lowered VM program point.
    pub const fn new(function: FunctionId, block: u32, pc: u32) -> Self {
        Self {
            function,
            block,
            pc,
        }
    }
}
