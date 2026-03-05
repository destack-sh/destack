use std::sync::{Arc, OnceLock};

use super::core::AudioEventRuntimeState;
#[cfg(all(target_os = "linux", feature = "audio-alsa"))]
use super::host::AlsaMonitorRuntimeState;
#[cfg(all(windows, feature = "audio-asio"))]
use super::host::AsioMonitorRuntimeState;
#[cfg(all(target_os = "macos", feature = "audio-coreaudio"))]
use super::host::CoreAudioMonitorRuntimeState;
#[cfg(all(target_os = "linux", feature = "audio-jack"))]
use super::host::JackMonitorRuntimeState;
#[cfg(all(
    target_os = "linux",
    any(feature = "audio-pipewire", feature = "audio-pulseaudio")
))]
use super::host::PactlMonitorRuntimeState;
#[cfg(all(windows, feature = "audio-wasapi"))]
use super::host::WasapiMonitorRuntimeState;

/// Runtime-owned audio module state.
#[derive(Default)]
pub(crate) struct PlatformAudioState {
    /// Runtime-owned shared audio event state.
    audio_event_runtime_state: OnceLock<Arc<AudioEventRuntimeState>>,
    /// Runtime-owned ASIO monitor state.
    #[cfg(all(windows, feature = "audio-asio"))]
    asio_monitor_runtime_state: OnceLock<Arc<AsioMonitorRuntimeState>>,
    /// Runtime-owned WASAPI monitor state.
    #[cfg(all(windows, feature = "audio-wasapi"))]
    wasapi_monitor_runtime_state: OnceLock<Arc<WasapiMonitorRuntimeState>>,
    /// Runtime-owned ALSA monitor state.
    #[cfg(all(target_os = "linux", feature = "audio-alsa"))]
    alsa_monitor_runtime_state: OnceLock<Arc<AlsaMonitorRuntimeState>>,
    /// Runtime-owned JACK monitor state.
    #[cfg(all(target_os = "linux", feature = "audio-jack"))]
    jack_monitor_runtime_state: OnceLock<Arc<JackMonitorRuntimeState>>,
    /// Runtime-owned CoreAudio monitor state.
    #[cfg(all(target_os = "macos", feature = "audio-coreaudio"))]
    coreaudio_monitor_runtime_state: OnceLock<Arc<CoreAudioMonitorRuntimeState>>,
    /// Runtime-owned pactl monitor state.
    #[cfg(all(
        target_os = "linux",
        any(feature = "audio-pipewire", feature = "audio-pulseaudio")
    ))]
    pactl_monitor_runtime_state: OnceLock<Arc<PactlMonitorRuntimeState>>,
}

impl std::fmt::Debug for PlatformAudioState {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("PlatformAudioState")
            .finish_non_exhaustive()
    }
}

impl PlatformAudioState {
    /// Return runtime-owned shared audio event state.
    pub(crate) fn audio_event_runtime_state(
        &self,
        initialize: impl FnOnce() -> AudioEventRuntimeState,
    ) -> Arc<AudioEventRuntimeState> {
        Arc::clone(
            self.audio_event_runtime_state
                .get_or_init(|| Arc::new(initialize())),
        )
    }

    /// Return runtime-owned ASIO monitor state.
    #[cfg(all(windows, feature = "audio-asio"))]
    pub(crate) fn asio_monitor_runtime_state(
        &self,
        initialize: impl FnOnce() -> AsioMonitorRuntimeState,
    ) -> Arc<AsioMonitorRuntimeState> {
        Arc::clone(
            self.asio_monitor_runtime_state
                .get_or_init(|| Arc::new(initialize())),
        )
    }

    /// Return runtime-owned WASAPI monitor state.
    #[cfg(all(windows, feature = "audio-wasapi"))]
    pub(crate) fn wasapi_monitor_runtime_state(
        &self,
        initialize: impl FnOnce() -> WasapiMonitorRuntimeState,
    ) -> Arc<WasapiMonitorRuntimeState> {
        Arc::clone(
            self.wasapi_monitor_runtime_state
                .get_or_init(|| Arc::new(initialize())),
        )
    }

    /// Return runtime-owned ALSA monitor state.
    #[cfg(all(target_os = "linux", feature = "audio-alsa"))]
    pub(crate) fn alsa_monitor_runtime_state(
        &self,
        initialize: impl FnOnce() -> AlsaMonitorRuntimeState,
    ) -> Arc<AlsaMonitorRuntimeState> {
        Arc::clone(
            self.alsa_monitor_runtime_state
                .get_or_init(|| Arc::new(initialize())),
        )
    }

    /// Return runtime-owned JACK monitor state.
    #[cfg(all(target_os = "linux", feature = "audio-jack"))]
    pub(crate) fn jack_monitor_runtime_state(
        &self,
        initialize: impl FnOnce() -> JackMonitorRuntimeState,
    ) -> Arc<JackMonitorRuntimeState> {
        Arc::clone(
            self.jack_monitor_runtime_state
                .get_or_init(|| Arc::new(initialize())),
        )
    }

    /// Return runtime-owned CoreAudio monitor state.
    #[cfg(all(target_os = "macos", feature = "audio-coreaudio"))]
    pub(crate) fn coreaudio_monitor_runtime_state(
        &self,
        initialize: impl FnOnce() -> CoreAudioMonitorRuntimeState,
    ) -> Arc<CoreAudioMonitorRuntimeState> {
        Arc::clone(
            self.coreaudio_monitor_runtime_state
                .get_or_init(|| Arc::new(initialize())),
        )
    }

    /// Return runtime-owned pactl monitor state.
    #[cfg(all(
        target_os = "linux",
        any(feature = "audio-pipewire", feature = "audio-pulseaudio")
    ))]
    pub(crate) fn pactl_monitor_runtime_state(
        &self,
        initialize: impl FnOnce() -> PactlMonitorRuntimeState,
    ) -> Arc<PactlMonitorRuntimeState> {
        Arc::clone(
            self.pactl_monitor_runtime_state
                .get_or_init(|| Arc::new(initialize())),
        )
    }
}
