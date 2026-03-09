use std::sync::{Arc, OnceLock};

use crate::runtime::BindingCallContext;

use super::core::monitor::AudioMonitorServiceRegistry;
use super::core::runtime::AudioRuntimeState;

/// Runtime-owned audio module state.
#[derive(Default)]
pub(crate) struct PlatformAudioState {
    /// Runtime-owned shared audio event state.
    runtime_state: OnceLock<Arc<AudioRuntimeState>>,
}

impl std::fmt::Debug for PlatformAudioState {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("PlatformAudioState")
            .finish_non_exhaustive()
    }
}

impl PlatformAudioState {
    /// Return whether any runtime-owned audio state was initialized.
    pub(crate) fn is_initialized(&self) -> bool {
        self.runtime_state.get().is_some()
    }

    /// Return runtime-owned shared audio event state.
    pub(crate) fn runtime_state(&self, ctx: &BindingCallContext) -> Arc<AudioRuntimeState> {
        Arc::clone(self.runtime_state.get_or_init(|| {
            let runtime_state = Arc::new(AudioRuntimeState::new(ctx.agent().id));
            let runtime_state_for_shutdown = Arc::clone(&runtime_state);

            // unregister shared audio services on runtime teardown
            ctx.agent().finalizers.register(move || {
                AudioMonitorServiceRegistry::unregister_runtime(&runtime_state_for_shutdown);
            });

            runtime_state
        }))
    }
}
