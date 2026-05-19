use crate::check::FlowState;

/// Local state while walking one DIR control-flow region.
#[derive(Debug, Clone)]
pub(crate) struct WalkState {
    /// Current runtime flow state.
    pub(crate) flow: FlowState,
}

impl WalkState {
    /// Create walk state with reachable flow.
    pub(crate) fn new() -> Self {
        Self {
            flow: FlowState::default(),
        }
    }
}
