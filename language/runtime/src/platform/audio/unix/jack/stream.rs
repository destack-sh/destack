use std::sync::Arc;

use crate::diagnostic::RuntimeResult;
use crate::platform::audio::core as audio_core;

use super::super::backend::backend_not_supported;

/// Open one JACK stream.
pub(crate) fn open_host_stream(
    _device_info: &audio_core::HostDeviceDescriptor,
    _config: audio_core::AudioStreamConfig,
    _share_mode: audio_core::AudioShareMode,
) -> RuntimeResult<Arc<audio_core::AudioStreamBinding>> {
    Err(backend_not_supported("destack.audio.stream.open", "jack"))
}
