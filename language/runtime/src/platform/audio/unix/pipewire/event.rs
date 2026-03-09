#[cfg(target_os = "linux")]
use super::super::pactl;
use crate::diagnostic::RuntimeResult;
#[cfg(target_os = "linux")]
use crate::platform::audio::AudioBackend;
#[cfg(not(target_os = "linux"))]
use crate::platform::audio::backend::backend_not_supported;
use crate::platform::audio::core as audio_core;

/// Start PipeWire native device-event monitoring.
pub(crate) fn start_native_device_event_monitor()
-> RuntimeResult<Box<dyn audio_core::AudioMonitorHandle>> {
    #[cfg(target_os = "linux")]
    {
        pactl::start_native_device_event_monitor(AudioBackend::PipeWire, "pipewire")
    }

    #[cfg(not(target_os = "linux"))]
    {
        Err(backend_not_supported(
            "destack.audio.event.open",
            "pipewire",
        ))
    }
}
