use std::sync::Arc;

use crate::diagnostic::RuntimeResult;
use crate::platform::audio::core as audio_core;

use super::core::is_stream_supported as host_stream_supported;
use super::runtime::open_stream;

/// Return whether ASIO stream support is implemented for this build.
pub(crate) fn is_stream_supported() -> bool {
    host_stream_supported()
}

/// Open one ASIO stream.
pub(crate) fn open_host_stream(
    device_info: &audio_core::HostDeviceDescriptor,
    config: audio_core::AudioStreamConfig,
    share_mode: audio_core::AudioShareMode,
    backend_flags: audio_core::AudioBackendOpenFlags,
) -> RuntimeResult<Arc<audio_core::AudioStreamBinding>> {
    let _ = backend_flags;
    open_stream(device_info, config, share_mode)
}
