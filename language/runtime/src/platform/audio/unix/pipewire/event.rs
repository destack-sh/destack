#[cfg(not(target_os = "linux"))]
use super::super::backend::backend_not_supported;
#[cfg(target_os = "linux")]
use super::super::pactl;
use crate::diagnostic::RuntimeResult;
#[cfg(target_os = "linux")]
use crate::platform::audio::AudioBackend;
use crate::runtime::BindingCallContext;

/// Return whether PipeWire native device-event monitoring is available.
pub(crate) fn native_device_events_supported() -> bool {
    #[cfg(target_os = "linux")]
    {
        return pactl::native_device_events_supported(AudioBackend::PipeWire);
    }

    #[cfg(not(target_os = "linux"))]
    {
        false
    }
}

/// Start PipeWire native device-event monitoring.
pub(crate) fn start_native_device_event_monitor(
    _context: &BindingCallContext,
) -> RuntimeResult<()> {
    #[cfg(target_os = "linux")]
    {
        return pactl::start_native_device_event_monitor(
            _context,
            AudioBackend::PipeWire,
            "pipewire",
        );
    }

    #[cfg(not(target_os = "linux"))]
    {
        Err(backend_not_supported(
            "destack.audio.event.open",
            "pipewire",
        ))
    }
}

/// Stop PipeWire native device-event monitoring.
pub(crate) fn stop_native_device_event_monitor(_context: &BindingCallContext) {
    #[cfg(target_os = "linux")]
    {
        pactl::stop_native_device_event_monitor(_context, AudioBackend::PipeWire);
    }
}
