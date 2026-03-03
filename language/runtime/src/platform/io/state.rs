use std::sync::{Arc, OnceLock};

use super::core::IoRuntimeState;

/// Runtime-owned I/O module state.
#[derive(Default)]
pub struct PlatformIoState {
    /// Runtime-owned poll attachment state.
    runtime_state: OnceLock<Arc<IoRuntimeState>>,
}

impl std::fmt::Debug for PlatformIoState {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("PlatformIoState")
            .finish_non_exhaustive()
    }
}

impl PlatformIoState {
    /// Return runtime-owned poll attachment state.
    pub(crate) fn runtime_state(
        &self,
        initialize: impl FnOnce() -> IoRuntimeState,
    ) -> Arc<IoRuntimeState> {
        Arc::clone(self.runtime_state.get_or_init(|| Arc::new(initialize())))
    }
}
