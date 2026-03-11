use std::sync::Arc;

use destack_core::{Capture, CaptureMode};
use serde::{Deserialize, Serialize};

use crate::diagnostic::RuntimeError;
#[cfg(target_os = "macos")]
use crate::diagnostic::RuntimeResult;
#[cfg(target_os = "linux")]
use crate::diagnostic::RuntimeResult;
#[cfg(windows)]
use crate::diagnostic::RuntimeResult;
use crate::platform::service::CachedServiceHandle;
use crate::runtime::{AgentId, BindingCallContext};

#[cfg(target_os = "linux")]
use super::host::{AlsaService, alsa_service};
#[cfg(target_os = "macos")]
use super::host::{CoreMidiService, core_midi_service};
#[cfg(windows)]
use super::host::{WinRtService, winrt_service};

/// Agent-owned MIDI module state.
#[derive(Default)]
pub(crate) struct PlatformMidiState {
    /// Shared ALSA service handle for this agent.
    #[cfg(target_os = "linux")]
    alsa_service: CachedServiceHandle<AlsaService>,
    /// Shared CoreMIDI service handle for this agent.
    #[cfg(target_os = "macos")]
    core_midi_service: CachedServiceHandle<CoreMidiService>,
    /// Shared WinRT service handle for this agent.
    #[cfg(windows)]
    winrt_service: CachedServiceHandle<WinRtService>,
    /// Owning agent identifier once MIDI runtime state becomes active.
    runtime_agent_id: std::sync::OnceLock<AgentId>,
}

impl std::fmt::Debug for PlatformMidiState {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("PlatformMidiState")
            .finish_non_exhaustive()
    }
}

impl PlatformMidiState {
    /// Return one shared ALSA service handle for this agent.
    #[cfg(target_os = "linux")]
    pub(crate) fn alsa_service(&self, operation: &'static str) -> RuntimeResult<Arc<AlsaService>> {
        self.alsa_service
            .get_or_try_init(|| alsa_service(operation))
    }

    /// Ensure the shared CoreMIDI service is initialized for this agent.
    #[cfg(target_os = "macos")]
    pub(crate) fn ensure_core_midi_service(&self, operation: &'static str) -> RuntimeResult<()> {
        let _service = self.core_midi_service(operation)?;

        Ok(())
    }

    /// Return one shared CoreMIDI service handle for this agent.
    #[cfg(target_os = "macos")]
    pub(crate) fn core_midi_service(
        &self,
        operation: &'static str,
    ) -> RuntimeResult<Arc<CoreMidiService>> {
        self.core_midi_service
            .get_or_try_init(|| core_midi_service(operation))
    }

    /// Ensure the shared WinRT service is initialized for this agent.
    #[cfg(windows)]
    pub(crate) fn ensure_winrt_service(&self, operation: &'static str) -> RuntimeResult<()> {
        let _service = self.winrt_service(operation)?;

        Ok(())
    }

    /// Return one shared WinRT service handle for this agent.
    #[cfg(windows)]
    pub(crate) fn winrt_service(
        &self,
        operation: &'static str,
    ) -> RuntimeResult<Arc<WinRtService>> {
        self.winrt_service
            .get_or_try_init(|| winrt_service(operation))
    }

    /// Return whether any agent-owned MIDI runtime state is active.
    fn has_runtime_state(&self) -> bool {
        self.runtime_agent_id.get().is_some()
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

    /// Mark agent-owned MIDI runtime state as active.
    pub(crate) fn mark_runtime_active(&self, ctx: &BindingCallContext) {
        let _ = self.runtime_agent_id.get_or_init(|| ctx.agent().id);
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
