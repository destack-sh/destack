use destack_core::{Capture, CaptureMode};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

#[cfg(target_os = "android")]
use super::host::AndroidBackendDescription;
#[cfg(target_os = "linux")]
use super::host::{AlsaService, JackService, alsa_service, jack_service};
#[cfg(any(target_os = "macos", target_os = "ios"))]
use super::host::{CoreMidiService, core_midi_service};
#[cfg(windows)]
use super::host::{
    WinMmService, WinRtService, WindowsMidiService, windows_midi_service, winmm_service,
    winrt_service,
};
use crate::diagnostic::RuntimeError;
#[cfg(any(
    target_os = "android",
    target_os = "linux",
    target_os = "macos",
    target_os = "ios",
    windows
))]
use crate::diagnostic::RuntimeResult;
use crate::platform::resource::ResourceFinalizer;
#[cfg(any(
    target_os = "android",
    target_os = "linux",
    target_os = "macos",
    target_os = "ios",
    windows
))]
use crate::runtime::BindingCallContext;
#[cfg(any(target_os = "linux", target_os = "macos", target_os = "ios", windows))]
use crate::runtime::process::service::ServiceHandle;

/// Agent-owned MIDI module state.
#[derive(Default)]
pub(crate) struct PlatformMidiState {
    /// Number of live MIDI resources that currently hold runtime activity.
    runtime_activity_count: Arc<AtomicUsize>,
    /// Cached Android backend description for this agent.
    #[cfg(target_os = "android")]
    android_backend_description: std::sync::OnceLock<AndroidBackendDescription>,
    /// Shared ALSA service handle for this agent.
    #[cfg(target_os = "linux")]
    alsa_service: ServiceHandle<AlsaService>,
    /// Shared JACK service handle for this agent.
    #[cfg(target_os = "linux")]
    jack_service: ServiceHandle<JackService>,
    /// Shared CoreMIDI service handle for this agent.
    #[cfg(any(target_os = "macos", target_os = "ios"))]
    core_midi_service: ServiceHandle<CoreMidiService>,
    /// Shared WinRT service handle for this agent.
    #[cfg(windows)]
    winrt_service: ServiceHandle<WinRtService>,
    /// Shared Windows MIDI service handle for this agent.
    #[cfg(windows)]
    windows_midi_service: ServiceHandle<WindowsMidiService>,
    /// Shared WinMM service handle for this agent.
    #[cfg(windows)]
    winmm_service: ServiceHandle<WinMmService>,
}

impl std::fmt::Debug for PlatformMidiState {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("PlatformMidiState")
            .finish_non_exhaustive()
    }
}

impl PlatformMidiState {
    /// Return one cached Android backend description for this agent.
    #[cfg(target_os = "android")]
    pub(crate) fn describe_android_backend(
        &self,
        binding: &BindingCallContext,
        operation: &'static str,
    ) -> RuntimeResult<AndroidBackendDescription> {
        if let Some(description) = self.android_backend_description.get() {
            return Ok(*description);
        }

        let description = super::host::describe_backend(binding, operation)?;
        let _ = self.android_backend_description.set(description);

        Ok(*self
            .android_backend_description
            .get()
            .unwrap_or(&description))
    }

    /// Return one shared ALSA service handle for this agent.
    #[cfg(target_os = "linux")]
    pub(crate) fn alsa_service(&self, operation: &'static str) -> RuntimeResult<Arc<AlsaService>> {
        self.alsa_service
            .get_or_try_init(|| alsa_service(operation))
    }

    /// Return one shared JACK service handle for this agent.
    #[cfg(target_os = "linux")]
    pub(crate) fn jack_service(&self, operation: &'static str) -> RuntimeResult<Arc<JackService>> {
        self.jack_service
            .get_or_try_init(|| jack_service(operation))
    }

    /// Return one shared CoreMIDI service handle for this agent.
    #[cfg(any(target_os = "macos", target_os = "ios"))]
    pub(crate) fn core_midi_service(
        &self,
        operation: &'static str,
    ) -> RuntimeResult<Arc<CoreMidiService>> {
        self.core_midi_service
            .get_or_try_init(|| core_midi_service(operation))
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

    /// Return one shared Windows MIDI service handle for this agent.
    #[cfg(windows)]
    pub(crate) fn windows_midi_service(
        &self,
        operation: &'static str,
    ) -> RuntimeResult<Arc<WindowsMidiService>> {
        self.windows_midi_service
            .get_or_try_init(|| windows_midi_service(operation))
    }

    /// Return one shared WinMM service handle for this agent.
    #[cfg(windows)]
    pub(crate) fn winmm_service(
        &self,
        operation: &'static str,
    ) -> RuntimeResult<Arc<WinMmService>> {
        self.winmm_service
            .get_or_try_init(|| winmm_service(operation))
    }

    /// Return whether any agent-owned MIDI runtime state is active.
    fn has_runtime_state(&self) -> bool {
        self.runtime_activity_count.load(Ordering::Acquire) != 0
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

    /// Retain one unit of MIDI runtime activity until the returned finalizer runs.
    #[cfg(any(
        target_os = "android",
        target_os = "linux",
        target_os = "macos",
        target_os = "ios",
        windows
    ))]
    pub(crate) fn retain_runtime_activity(
        &self,
        _ctx: &BindingCallContext,
    ) -> PlatformMidiActivityFinalizer {
        self.runtime_activity_count.fetch_add(1, Ordering::AcqRel);

        PlatformMidiActivityFinalizer {
            activity_count: self.runtime_activity_count.clone(),
        }
    }
}

/// Resource finalizer that releases one retained MIDI runtime activity unit.
pub(crate) struct PlatformMidiActivityFinalizer {
    /// Shared live MIDI activity counter.
    activity_count: Arc<AtomicUsize>,
}

impl ResourceFinalizer for PlatformMidiActivityFinalizer {
    /// Release one retained MIDI runtime activity unit.
    fn finalize(self: Box<Self>, _resource_id: crate::platform::resource::ResourceId) {
        let previous = self.activity_count.fetch_sub(1, Ordering::AcqRel);
        debug_assert!(previous != 0, "midi runtime activity count underflow");
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
