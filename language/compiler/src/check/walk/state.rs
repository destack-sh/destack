use destack_source::ModuleId;

use crate::check::{CheckState, FlowState};

/// State used only while walking one module.
pub(in crate::check) struct WalkState<'check, 'state> {
    /// The component check state being populated.
    pub(in crate::check) check: &'check mut CheckState<'state>,
    /// The module being walked.
    pub(in crate::check) module: ModuleId,
    /// Flow state for the current module walk.
    flow: FlowState,
}

impl<'check, 'state> WalkState<'check, 'state> {
    /// Create walk state for one module.
    pub(in crate::check) fn new(module: ModuleId, check: &'check mut CheckState<'state>) -> Self {
        Self {
            check,
            module,
            flow: FlowState::default(),
        }
    }

    /// Return flow state for the active module.
    pub(in crate::check) fn flow(&self) -> &FlowState {
        &self.flow
    }

    /// Return mutable flow state for the active module.
    pub(in crate::check) fn flow_mut(&mut self) -> &mut FlowState {
        &mut self.flow
    }
}
