use std::collections::HashMap;

use {destack_engine as engine, destack_mir as mir};

/// Dense frame-state index for one VM program.
#[derive(Default)]
pub(crate) struct FrameStateTable {
    /// Frame states by dense frame state id.
    states: Vec<FrameState>,
    /// Frame state id by lowered program point.
    state_by_point: HashMap<ProgramPoint, engine::FrameStateId>,
}

impl FrameStateTable {
    /// Return the number of frame states.
    pub(crate) fn len(&self) -> usize {
        self.states.len()
    }

    /// Return the next frame state id.
    pub(crate) fn next_id(&self) -> engine::FrameStateId {
        engine::FrameStateId(self.states.len() as u32)
    }

    /// Add one frame state.
    pub(crate) fn push(&mut self, id: engine::FrameStateId, state: FrameState) {
        debug_assert_eq!(id.0 as usize, self.states.len());
        self.state_by_point.insert(state.point, id);
        self.states.push(state);
    }

    /// Return one frame state.
    pub(crate) fn state(&self, id: engine::FrameStateId) -> Option<&FrameState> {
        self.states.get(id.0 as usize)
    }

    /// Return one frame state id by lowered program point.
    pub(crate) fn state_id_at(&self, point: ProgramPoint) -> Option<engine::FrameStateId> {
        self.state_by_point.get(&point).copied()
    }
}

/// One resumable VM frame state.
pub(crate) struct FrameState {
    /// The lowered VM program point.
    pub(crate) point: ProgramPoint,
    /// Entry recipe for block-entry states.
    pub(crate) entry: Option<FrameEntry>,
    /// Caller return destination for post-call states.
    pub(crate) return_destination: Option<mir::Value>,
    /// Live slot materialization at this state.
    pub(crate) materialization: engine::FrameMaterialization,
}

/// VM entry recipe for one frame state.
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

/// One lowered instruction boundary inside the VM program.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct ProgramPoint {
    /// The owning function.
    pub(crate) function: mir::LocalNodeId<mir::Function>,
    /// The owning block.
    pub(crate) block: mir::LocalNodeId<mir::Block>,
    /// The lowered instruction index within the block.
    pub(crate) instruction_index: u32,
}

impl ProgramPoint {
    /// Create one lowered instruction boundary.
    pub(crate) const fn new(
        function: mir::LocalNodeId<mir::Function>,
        block: mir::LocalNodeId<mir::Block>,
        instruction_index: u32,
    ) -> Self {
        Self {
            function,
            block,
            instruction_index,
        }
    }
}
