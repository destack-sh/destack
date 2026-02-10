use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::audio::{
    AudioDeviceDirection, AudioDeviceInfoVm, AudioStreamConfigVm, AudioStreamStateVm,
};
use crate::platform::{PlatformError, VmSlice, resource};
use crate::runtime::RuntimeCallContext;
use destack_vm as vm;

/// Stub for destack.audio.device.close.
pub(super) fn destack_audio_device_close(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::AudioDeviceHandle,
) -> RuntimeResult<()> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.audio.device.close is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.audio.device.list.
pub(super) fn destack_audio_device_list(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
) -> RuntimeResult<VmSlice<AudioDeviceInfoVm>> {
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.audio.device.list is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.audio.device.open.
pub(super) fn destack_audio_device_open(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    id: vm::StringHandle,
    direction: AudioDeviceDirection,
) -> RuntimeResult<resource::AudioDeviceHandle> {
    let _ = (id, direction);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.audio.device.open is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.audio.stream.close.
pub(super) fn destack_audio_stream_close(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::AudioStreamHandle,
) -> RuntimeResult<()> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.audio.stream.close is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.audio.stream.open.
pub(super) fn destack_audio_stream_open(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    device: resource::AudioDeviceHandle,
    config: AudioStreamConfigVm,
) -> RuntimeResult<resource::AudioStreamHandle> {
    let _ = (device, config);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.audio.stream.open is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.audio.stream.read.
pub(super) fn destack_audio_stream_read(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::AudioStreamHandle,
    maxbytes: u32,
) -> RuntimeResult<VmSlice<u8>> {
    let _ = (handle, maxbytes);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.audio.stream.read is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.audio.stream.start.
pub(super) fn destack_audio_stream_start(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::AudioStreamHandle,
) -> RuntimeResult<()> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.audio.stream.start is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.audio.stream.state.
pub(super) fn destack_audio_stream_state(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::AudioStreamHandle,
) -> RuntimeResult<AudioStreamStateVm> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.audio.stream.state is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.audio.stream.stop.
pub(super) fn destack_audio_stream_stop(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::AudioStreamHandle,
) -> RuntimeResult<()> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.audio.stream.stop is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.audio.stream.write.
pub(super) fn destack_audio_stream_write(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::AudioStreamHandle,
    data: VmSlice<u8>,
) -> RuntimeResult<u64> {
    let _ = (handle, data);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.audio.stream.write is not available in the VM yet",
    ))
    .boxed())
}
