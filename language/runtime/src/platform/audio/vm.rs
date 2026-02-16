#![allow(dead_code)]
#![allow(unused_imports)]
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::audio::{
    AudioDeviceDirection, AudioDeviceInfoVm, AudioSampleFormat, AudioStreamConfigVm,
    AudioStreamStateVm,
};
use crate::platform::{PlatformError, VmSlice, resource};
use crate::runtime::RuntimeCallContext;
use destack_vm as vm;

/// Close one audio device endpoint.
///
/// Close one opened audio endpoint and release host stream resources.
/// Close semantics follow host backend teardown behavior.
///
/// # Platform
/// Unix and Windows.
/// Uses backend-specific stream close operations on both Unix-like hosts and Windows.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `audio.device`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_audio_device_close(
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

/// List available audio devices.
///
/// Enumerate host audio endpoints and return stable identifiers for later open operations.
/// Device visibility and ordering follow host audio backend semantics.
///
/// # Platform
/// Unix and Windows.
/// Uses ALSA or PulseAudio or CoreAudio enumeration on Unix-like hosts and WASAPI or MMDevice on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, notSupported.
///
/// # Security
/// Requires `audio.device`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_audio_device_list(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
) -> RuntimeResult<VmSlice<AudioDeviceInfoVm>> {
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.audio.device.list is not available in the VM yet",
    ))
    .boxed())
}

/// Open one audio device endpoint.
///
/// Open one host audio endpoint for playback, capture, or duplex operation.
/// Handle lifetime and exclusivity semantics follow host backend rules.
///
/// # Platform
/// Unix and Windows.
/// Uses ALSA or CoreAudio or PulseAudio device open on Unix-like hosts and WASAPI endpoint open on Windows.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `audio.device`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_audio_device_open(
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

/// Close one audio stream.
///
/// Close one host audio stream and release backend buffers and synchronization state.
/// Stream handle becomes invalid after close completes.
///
/// # Platform
/// Unix and Windows.
/// Uses backend-specific stream close operations.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `audio.playback`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_audio_stream_close(
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

/// Open one audio stream on a device.
///
/// Create one host audio stream with explicit sample format, channel, and period configuration.
/// Buffering and latency behavior follow host backend contracts.
///
/// # Platform
/// Unix and Windows.
/// Uses ALSA or PulseAudio or CoreAudio stream creation on Unix-like hosts and WASAPI stream creation on Windows.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `audio.playback`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_audio_stream_open(
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

/// Read one packet of captured audio frames.
///
/// Read one packet of captured interleaved audio frames from the capture stream.
/// Packet sizing and buffering follow host backend capture contracts.
///
/// # Platform
/// Unix and Windows.
/// Uses backend stream read or capture client operations.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `audio.capture`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_audio_stream_read(
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

/// Start one audio stream.
///
/// Transition one opened stream to running state and begin host callback or DMA processing.
/// Start timing follows host backend scheduling semantics.
///
/// # Platform
/// Unix and Windows.
/// Uses backend-specific stream start operations.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `audio.playback`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_audio_stream_start(
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

/// Read one stream state snapshot.
///
/// Read one point-in-time snapshot of stream run state and backend buffering metrics.
/// Snapshot values are advisory and can change immediately after read.
///
/// # Platform
/// Unix and Windows.
/// Uses backend-specific stream query primitives.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioInvalidData, notSupported.
///
/// # Security
/// Requires `audio.control`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_audio_stream_state(
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

/// Stop one audio stream.
///
/// Transition one running stream to stopped state and flush host backend scheduling.
/// Buffered frames may be discarded based on host backend semantics.
///
/// # Platform
/// Unix and Windows.
/// Uses backend-specific stream stop operations.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `audio.playback`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_audio_stream_stop(
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

/// Write one packet of audio frames.
///
/// Submit one packet of interleaved audio frames to the playback stream.
/// Short writes can occur when host buffers are near capacity.
///
/// # Platform
/// Unix and Windows.
/// Uses backend stream write or render client operations.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `audio.playback`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_audio_stream_write(
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
