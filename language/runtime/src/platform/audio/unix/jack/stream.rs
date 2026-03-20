use std::sync::Arc;

#[cfg(target_os = "linux")]
use super::runtime::open_stream;
use crate::diagnostic::RuntimeResult;
use crate::platform::audio as audio_types;
#[cfg(not(target_os = "linux"))]
use crate::platform::audio::backend::backend_not_supported;
use crate::platform::audio::core as audio_core;

/// Open one JACK stream.
pub(crate) fn open_host_stream(
    device_info: &audio_core::HostDeviceDescriptor,
    config: audio_types::AudioStreamConfig,
    share_mode: audio_types::AudioShareMode,
    backend_flags: audio_core::AudioBackendOpenFlags,
    requested_flags: audio_types::AudioStreamFlags,
    requested_requirements: audio_types::AudioStreamRequirementFlags,
) -> RuntimeResult<Arc<audio_core::AudioStreamHostState>> {
    #[cfg(target_os = "linux")]
    {
        open_stream(
            device_info,
            config,
            share_mode,
            backend_flags,
            requested_flags,
            requested_requirements,
        )
    }

    #[cfg(not(target_os = "linux"))]
    {
        let _ = (
            device_info,
            config,
            share_mode,
            backend_flags,
            requested_flags,
            requested_requirements,
        );
        Err(backend_not_supported("destack.audio.stream.open", "jack"))
    }
}
