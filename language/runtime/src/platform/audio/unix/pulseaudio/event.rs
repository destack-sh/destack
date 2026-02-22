#[cfg(not(target_os = "linux"))]
use super::super::backend::backend_not_supported;
#[cfg(target_os = "linux")]
use super::super::pactl;
use crate::diagnostic::RuntimeResult;
#[cfg(target_os = "linux")]
use crate::platform::audio::AudioBackend;

/// Return whether PulseAudio native device-event monitoring is available.
pub(crate) fn native_device_events_supported() -> bool {
    #[cfg(target_os = "linux")]
    {
        return pactl::native_device_events_supported(AudioBackend::PulseAudio);
    }

    #[cfg(not(target_os = "linux"))]
    {
        false
    }
}

/// Start PulseAudio native device-event monitoring.
pub(crate) fn start_native_device_event_monitor() -> RuntimeResult<()> {
    #[cfg(target_os = "linux")]
    {
        return pactl::start_native_device_event_monitor(AudioBackend::PulseAudio, "pulseaudio");
    }

    #[cfg(not(target_os = "linux"))]
    {
        Err(backend_not_supported(
            "destack.audio.event.open",
            "pulseaudio",
        ))
    }
}

/// Stop PulseAudio native device-event monitoring.
pub(crate) fn stop_native_device_event_monitor() {
    #[cfg(target_os = "linux")]
    {
        pactl::stop_native_device_event_monitor(AudioBackend::PulseAudio);
    }
}
