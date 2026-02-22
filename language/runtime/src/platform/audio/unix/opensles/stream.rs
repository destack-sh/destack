use std::sync::Arc;

#[cfg(not(target_os = "android"))]
use super::super::backend::backend_not_supported;
#[cfg(target_os = "android")]
use super::runtime::open_stream;
use crate::diagnostic::RuntimeResult;
use crate::platform::audio::core as audio_core;

/// Open one OpenSL ES stream.
pub(crate) fn open_host_stream(
    device_info: &audio_core::HostDeviceDescriptor,
    config: audio_core::AudioStreamConfig,
    share_mode: audio_core::AudioShareMode,
    backend_flags: audio_core::AudioBackendOpenFlags,
) -> RuntimeResult<Arc<audio_core::AudioStreamBinding>> {
    #[cfg(target_os = "android")]
    {
        let _ = backend_flags;
        open_stream(device_info, config, share_mode)
    }

    #[cfg(not(target_os = "android"))]
    {
        let _ = (device_info, config, share_mode, backend_flags);
        Err(backend_not_supported(
            "destack.audio.stream.open",
            "opensles",
        ))
    }
}
