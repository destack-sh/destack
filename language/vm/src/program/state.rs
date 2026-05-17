use std::collections::HashMap;

use {destack_engine as engine, destack_mir as mir};

/// VM resume recipes keyed by engine frame state.
#[derive(Default)]
pub(crate) struct ResumeTable {
    /// Resume states by dense frame state id.
    states: Vec<ResumeState>,
    /// Frame state id by lowered program point.
    state_by_point: HashMap<ProgramPoint, engine::FrameStateId>,
}

impl ResumeTable {
    /// Return the number of resume states.
    pub(crate) fn len(&self) -> usize {
        self.states.len()
    }

    /// Return the next resume state id.
    pub(crate) fn next_id(&self) -> engine::FrameStateId {
        engine::FrameStateId(self.states.len() as u32)
    }

    /// Add one resume state.
    pub(crate) fn push(&mut self, id: engine::FrameStateId, state: ResumeState) {
        debug_assert_eq!(id.0 as usize, self.states.len());
        self.state_by_point.insert(state.point, id);
        self.states.push(state);
    }

    /// Return one resume state.
    pub(crate) fn state(&self, id: engine::FrameStateId) -> Option<&ResumeState> {
        self.states.get(id.0 as usize)
    }

    /// Return one resume state id by lowered program point.
    pub(crate) fn state_id_at(&self, point: ProgramPoint) -> Option<engine::FrameStateId> {
        self.state_by_point.get(&point).copied()
    }
}

/// VM recipe for one resumable frame state.
pub(crate) struct ResumeState {
    /// The lowered VM program point.
    pub(crate) point: ProgramPoint,
    /// The source MIR point within the lowered block.
    pub(crate) mir_point: u32,
    /// Entry recipe for block-entry states.
    pub(crate) entry: Option<FrameEntry>,
    /// Caller return destination for post-call states.
    pub(crate) return_destination: Option<mir::Value>,
}

/// VM entry recipe for one resume state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FrameEntry {
    /// The slot bindings applied on entry.
    pub(crate) bindings: Vec<FrameBinding>,
    /// The implicit received value slot.
    pub(crate) received_value: Option<engine::FrameSlotId>,
}

/// One frame slot binding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct FrameBinding {
    /// The source frame slot.
    pub(crate) source: engine::FrameSlotId,
    /// The destination frame slot.
    pub(crate) destination: engine::FrameSlotId,
}

/// One lowered VM program point.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct ProgramPoint {
    /// The owning function.
    pub(crate) function: mir::LocalNodeId<mir::Function>,
    /// The owning block.
    pub(crate) block: mir::LocalNodeId<mir::Block>,
    /// The lowered program counter inside the block.
    pub(crate) pc: u32,
}

impl ProgramPoint {
    /// Create one lowered VM program point.
    pub(crate) const fn new(
        function: mir::LocalNodeId<mir::Function>,
        block: mir::LocalNodeId<mir::Block>,
        pc: u32,
    ) -> Self {
        Self {
            function,
            block,
            pc,
        }
    }
}
