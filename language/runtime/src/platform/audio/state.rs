use std::sync::{Arc, OnceLock};

use crate::runtime::BindingCallContext;

use super::core::{AudioMonitorServiceRegistry, AudioRuntimeState};

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
        // shared audio event state
        if self.audio_event_runtime_state.get().is_some() {
            return true;
        }

        // windows monitor state
        #[cfg(all(windows, feature = "audio-asio"))]
        if self.asio_monitor_runtime_state.get().is_some() {
            return true;
        }

        // windows monitor state
        #[cfg(all(windows, feature = "audio-wasapi"))]
        if self.wasapi_monitor_runtime_state.get().is_some() {
            return true;
        }

        // linux monitor state
        #[cfg(all(target_os = "linux", feature = "audio-alsa"))]
        if self.alsa_monitor_runtime_state.get().is_some() {
            return true;
        }

        // linux monitor state
        #[cfg(all(target_os = "linux", feature = "audio-jack"))]
        if self.jack_monitor_runtime_state.get().is_some() {
            return true;
        }

        // macos monitor state
        #[cfg(all(target_os = "macos", feature = "audio-coreaudio"))]
        if self.coreaudio_monitor_runtime_state.get().is_some() {
            return true;
        }

        // linux monitor state
        #[cfg(all(
            target_os = "linux",
            any(feature = "audio-pipewire", feature = "audio-pulseaudio")
        ))]
        if self.pactl_monitor_runtime_state.get().is_some() {
            return true;
        }

        false
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
