use std::sync::{Arc, OnceLock};

use destack_base::{Capture, CaptureMode};
use serde::{Deserialize, Serialize};

use crate::diagnostic::RuntimeError;
use crate::runtime::{AgentId, BindingCallContext};

/// Runtime-owned MIDI module state for one agent.
pub(crate) struct MidiRuntimeState {
    /// Owning agent identifier.
    pub(crate) _agent_id: AgentId,
}

/// Runtime-owned MIDI module state.
#[derive(Default)]
pub(crate) struct PlatformMidiState {
    /// Runtime-owned shared MIDI state.
    runtime_state: OnceLock<Arc<MidiRuntimeState>>,
}

impl std::fmt::Debug for PlatformMidiState {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("PlatformMidiState")
            .finish_non_exhaustive()
    }
}

impl PlatformMidiState {
    /// Return whether any runtime-owned MIDI state is active.
    fn has_runtime_state(&self) -> bool {
        self.runtime_state.get().is_some()
    }

    /// Capture one MIDI-state image.
    fn image(&self, mode: CaptureMode) -> Result<PlatformMidiImage, Box<RuntimeError>> {
        if !self.has_runtime_state() {
            return Ok(PlatformMidiImage);
        }

        Err(RuntimeError::CaptureBarrier {
            component: "platform.midi".to_string(),
            mode: format!("{mode:?}"),
            detail: "runtime state is active".to_string(),
        }
        .boxed())
    }

    /// Return runtime-owned shared MIDI state.
    pub(crate) fn runtime_state(&self, ctx: &BindingCallContext) -> Arc<MidiRuntimeState> {
        Arc::clone(self.runtime_state.get_or_init(|| {
            Arc::new(MidiRuntimeState {
                _agent_id: ctx.agent().id,
            })
        }))
    }
}

/// Materialized MIDI platform-state image.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct PlatformMidiImage;

impl Capture for PlatformMidiState {
    type Image = PlatformMidiImage;
    type Error = Box<RuntimeError>;
    type CaptureContext<'a> = ();
    type RestoreContext<'a> = ();

    /// Capture one MIDI platform-state image.
    fn capture_image(
        &mut self,
        mode: CaptureMode,
        _context: Self::CaptureContext<'_>,
    ) -> Result<Self::Image, Self::Error> {
        self.image(mode)
    }

    /// Restore one MIDI platform-state image.
    fn restore_image(
        &mut self,
        _image: &Self::Image,
        _context: Self::RestoreContext<'_>,
    ) -> Result<(), Self::Error> {
        *self = Self::default();

        Ok(())
    }
}
