use crate::isolate::IsolateState;

/// Compiled engine state for native execution.
#[derive(Debug, Default)]
pub(crate) struct CompiledState;

/// Compiled execution engine for native code.
#[derive(Debug)]
pub struct CompiledEngine {
    /// Compiled engine state for execution.
    #[allow(dead_code)]
    state: CompiledState,
}

impl CompiledEngine {
    /// Create a new compiled engine for the given isolate state.
    pub(crate) fn new(_isolate: &IsolateState) -> Self {
        Self {
            state: CompiledState,
        }
    }
}
