use std::sync::{Arc, OnceLock};

use destack_core::{Capture, CaptureMode};
use serde::{Deserialize, Serialize};

use crate::diagnostic::RuntimeError;
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
    /// Return whether any runtime-owned audio state is active.
    fn has_runtime_state(&self) -> bool {
        self.runtime_state.get().is_some()
    }

    /// Capture one audio-state image.
    fn image(&self, mode: CaptureMode) -> Result<PlatformAudioImage, Box<RuntimeError>> {
        if !self.has_runtime_state() {
            return Ok(PlatformAudioImage);
        }

        Err(RuntimeError::CaptureBarrier {
            component: "platform.audio".to_string(),
            mode: format!("{mode:?}"),
            detail: "runtime state is active".to_string(),
        }
        .boxed())
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

/// Materialized audio platform-state image.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct PlatformAudioImage;

impl Capture for PlatformAudioState {
    type Image = PlatformAudioImage;
    type Error = Box<RuntimeError>;
    type CaptureContext<'a> = ();
    type RestoreContext<'a> = ();

    /// Capture one audio platform-state image.
    fn capture_image(
        &mut self,
        mode: CaptureMode,
        _context: Self::CaptureContext<'_>,
    ) -> Result<Self::Image, Self::Error> {
        self.image(mode)
    }

    /// Restore one audio platform-state image.
    fn restore_image(
        &mut self,
        _image: &Self::Image,
        _context: Self::RestoreContext<'_>,
    ) -> Result<(), Self::Error> {
        *self = Self::default();

        Ok(())
    }
}
