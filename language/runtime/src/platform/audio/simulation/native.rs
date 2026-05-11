#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(clippy::missing_safety_doc)]

use super::super::{
    AudioBackend, AudioBackendDescriptor, AudioBackendSelectionPolicy, AudioClockDomain,
    AudioClockSnapshot, AudioDeviceDescriptor, AudioDeviceDirection, AudioDeviceListRequest,
    AudioDeviceOpenOptions, AudioEvent, AudioEventSubscriptionOptions, AudioStreamAvailability,
    AudioStreamClockDomain, AudioStreamConfig, AudioStreamDescriptor, AudioStreamOpenOptions,
    AudioStreamState, AudioStreamSupport, AudioStreamTiming,
};
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::abi::{NativeSlice, NativeStringRef};
use crate::platform::{PlatformError, resource};
use crate::runtime::BindingCallContext;

fn unsupported(operation: &'static str) -> Box<RuntimeError> {
    RuntimeError::from(PlatformError::not_supported(operation)).boxed()
}

/// List host audio backends.
pub(crate) unsafe fn destack_audio_backend_list(
    _binding: &BindingCallContext,
    out: *mut NativeSlice<AudioBackendDescriptor>,
) -> RuntimeResult<()> {
    let _ = out;
    Err(unsupported("destack.audio.backend.list"))
}

/// Read one timestamp in one selected clock domain.
pub(crate) unsafe fn destack_audio_clock_now(
    _binding: &BindingCallContext,
    out: *mut u64,
    domain: AudioClockDomain,
) -> RuntimeResult<()> {
    let _ = (out, domain);
    Err(unsupported("destack.audio.clock.now"))
}

/// Read one stream clock snapshot.
pub(crate) unsafe fn destack_audio_stream_clock(
    _binding: &BindingCallContext,
    out: *mut AudioClockSnapshot,
    handle: resource::AudioStreamHandle,
    domain: AudioStreamClockDomain,
) -> RuntimeResult<()> {
    let _ = (out, handle, domain);
    Err(unsupported("destack.audio.clock.stream"))
}

/// Close one audio device endpoint.
pub(crate) unsafe fn destack_audio_device_close(
    _binding: &BindingCallContext,
    handle: resource::AudioDeviceHandle,
) -> RuntimeResult<()> {
    let _ = handle;
    Err(unsupported("destack.audio.device.close"))
}

/// Read one default device identifier for the selected direction.
pub(crate) unsafe fn destack_audio_device_default(
    _binding: &BindingCallContext,
    out: *mut NativeStringRef,
    direction: AudioDeviceDirection,
    backend: AudioBackend,
    backend_policy: AudioBackendSelectionPolicy,
) -> RuntimeResult<()> {
    let _ = (out, direction, backend, backend_policy);
    Err(unsupported("destack.audio.device.default"))
}

/// Read metadata for one opened device endpoint.
pub(crate) unsafe fn destack_audio_device_descriptor(
    _binding: &BindingCallContext,
    out: *mut AudioDeviceDescriptor,
    handle: resource::AudioDeviceHandle,
) -> RuntimeResult<()> {
    let _ = (out, handle);
    Err(unsupported("destack.audio.device.descriptor"))
}

/// List available audio devices.
pub(crate) unsafe fn destack_audio_device_list(
    _binding: &BindingCallContext,
    out: *mut NativeSlice<AudioDeviceDescriptor>,
    request: AudioDeviceListRequest,
) -> RuntimeResult<()> {
    let _ = (out, request);
    Err(unsupported("destack.audio.device.list"))
}

/// Open one audio device endpoint.
pub(crate) unsafe fn destack_audio_device_open(
    _binding: &BindingCallContext,
    out: *mut resource::AudioDeviceHandle,
    id: NativeStringRef,
    options: AudioDeviceOpenOptions,
) -> RuntimeResult<()> {
    let _ = (out, id, options);
    Err(unsupported("destack.audio.device.open"))
}

/// Trigger one backend rescan.
pub(crate) unsafe fn destack_audio_device_rescan(
    _binding: &BindingCallContext,
    backend: AudioBackend,
    backend_policy: AudioBackendSelectionPolicy,
) -> RuntimeResult<()> {
    let _ = (backend, backend_policy);
    Err(unsupported("destack.audio.device.rescan"))
}

/// Close one audio event subscription.
pub(crate) unsafe fn destack_audio_event_close(
    _binding: &BindingCallContext,
    handle: resource::AudioEventHandle,
) -> RuntimeResult<()> {
    let _ = handle;
    Err(unsupported("destack.audio.event.close"))
}

/// Open one audio event subscription.
pub(crate) unsafe fn destack_audio_event_open(
    _binding: &BindingCallContext,
    out: *mut resource::AudioEventHandle,
    options: AudioEventSubscriptionOptions,
) -> RuntimeResult<()> {
    let _ = (out, options);
    Err(unsupported("destack.audio.event.open"))
}

/// Wait for one audio event.
pub(crate) unsafe fn destack_audio_event_read(
    _binding: &BindingCallContext,
    out: *mut AudioEvent,
    handle: resource::AudioEventHandle,
    timeoutns: u64,
) -> RuntimeResult<()> {
    let _ = (out, handle, timeoutns);
    Err(unsupported("destack.audio.event.read"))
}

/// Wait for one batch of audio events.
pub(crate) unsafe fn destack_audio_event_read_batch(
    _binding: &BindingCallContext,
    out: *mut NativeSlice<AudioEvent>,
    handle: resource::AudioEventHandle,
    maxevents: u32,
    timeoutns: u64,
) -> RuntimeResult<()> {
    let _ = (out, handle, maxevents, timeoutns);
    Err(unsupported("destack.audio.event.readBatch"))
}

/// Poll one audio event without blocking.
pub(crate) unsafe fn destack_audio_event_try_read(
    _binding: &BindingCallContext,
    out: *mut AudioEvent,
    handle: resource::AudioEventHandle,
) -> RuntimeResult<()> {
    let _ = (out, handle);
    Err(unsupported("destack.audio.event.tryRead"))
}

/// Poll one batch of audio events without blocking.
pub(crate) unsafe fn destack_audio_event_try_read_batch(
    _binding: &BindingCallContext,
    out: *mut NativeSlice<AudioEvent>,
    handle: resource::AudioEventHandle,
    maxevents: u32,
) -> RuntimeResult<()> {
    let _ = (out, handle, maxevents);
    Err(unsupported("destack.audio.event.tryReadBatch"))
}

/// Abort one audio stream immediately.
pub(crate) unsafe fn destack_audio_stream_abort(
    _binding: &BindingCallContext,
    handle: resource::AudioStreamHandle,
) -> RuntimeResult<()> {
    let _ = handle;
    Err(unsupported("destack.audio.stream.abort"))
}

/// Read one stream immediate availability sample.
pub(crate) unsafe fn destack_audio_stream_availability(
    _binding: &BindingCallContext,
    out: *mut AudioStreamAvailability,
    handle: resource::AudioStreamHandle,
) -> RuntimeResult<()> {
    let _ = (out, handle);
    Err(unsupported("destack.audio.stream.availability"))
}

/// Close one audio stream.
pub(crate) unsafe fn destack_audio_stream_close(
    _binding: &BindingCallContext,
    handle: resource::AudioStreamHandle,
) -> RuntimeResult<()> {
    let _ = handle;
    Err(unsupported("destack.audio.stream.close"))
}

/// Drain one playback stream.
pub(crate) unsafe fn destack_audio_stream_drain(
    _binding: &BindingCallContext,
    handle: resource::AudioStreamHandle,
    timeoutns: u64,
) -> RuntimeResult<()> {
    let _ = (handle, timeoutns);
    Err(unsupported("destack.audio.stream.drain"))
}

/// Flush buffered stream data.
pub(crate) unsafe fn destack_audio_stream_flush(
    _binding: &BindingCallContext,
    handle: resource::AudioStreamHandle,
) -> RuntimeResult<()> {
    let _ = handle;
    Err(unsupported("destack.audio.stream.flush"))
}

/// Open one audio stream on one device.
pub(crate) unsafe fn destack_audio_stream_open(
    _binding: &BindingCallContext,
    out: *mut resource::AudioStreamHandle,
    device: resource::AudioDeviceHandle,
    config: AudioStreamConfig,
    options: AudioStreamOpenOptions,
) -> RuntimeResult<()> {
    let _ = (out, device, config, options);
    Err(unsupported("destack.audio.stream.open"))
}

/// Check one audio stream configuration for backend support.
pub(crate) unsafe fn destack_audio_stream_support(
    _binding: &BindingCallContext,
    out: *mut AudioStreamSupport,
    device: resource::AudioDeviceHandle,
    config: AudioStreamConfig,
    options: AudioStreamOpenOptions,
) -> RuntimeResult<()> {
    let _ = (out, device, config, options);
    Err(unsupported("destack.audio.stream.support"))
}

/// Pause or resume one audio stream.
pub(crate) unsafe fn destack_audio_stream_pause(
    _binding: &BindingCallContext,
    handle: resource::AudioStreamHandle,
    pause: bool,
) -> RuntimeResult<()> {
    let _ = (handle, pause);
    Err(unsupported("destack.audio.stream.pause"))
}

/// Read one packet of captured audio frames.
pub(crate) unsafe fn destack_audio_stream_read(
    _binding: &BindingCallContext,
    out: *mut NativeSlice<u8>,
    handle: resource::AudioStreamHandle,
    maxbytes: u32,
) -> RuntimeResult<()> {
    let _ = (out, handle, maxbytes);
    Err(unsupported("destack.audio.stream.read"))
}

/// Read one packet into vectorized buffers.
pub(crate) unsafe fn destack_audio_stream_readv(
    _binding: &BindingCallContext,
    out: *mut u64,
    handle: resource::AudioStreamHandle,
    buffers: NativeSlice<NativeSlice<u8>>,
) -> RuntimeResult<()> {
    let _ = (out, handle, buffers);
    Err(unsupported("destack.audio.stream.readv"))
}

/// Set one stream mute state.
pub(crate) unsafe fn destack_audio_stream_set_mute(
    _binding: &BindingCallContext,
    handle: resource::AudioStreamHandle,
    muted: bool,
) -> RuntimeResult<()> {
    let _ = (handle, muted);
    Err(unsupported("destack.audio.stream.setMute"))
}

/// Set one stream name.
pub(crate) unsafe fn destack_audio_stream_set_name(
    _binding: &BindingCallContext,
    handle: resource::AudioStreamHandle,
    name: NativeStringRef,
) -> RuntimeResult<()> {
    let _ = (handle, name);
    Err(unsupported("destack.audio.stream.setName"))
}

/// Set one stream gain multiplier.
pub(crate) unsafe fn destack_audio_stream_set_volume(
    _binding: &BindingCallContext,
    handle: resource::AudioStreamHandle,
    lineargain: f64,
) -> RuntimeResult<()> {
    let _ = (handle, lineargain);
    Err(unsupported("destack.audio.stream.setVolume"))
}

/// Read one stream negotiated configuration descriptor.
pub(crate) unsafe fn destack_audio_stream_descriptor(
    _binding: &BindingCallContext,
    out: *mut AudioStreamDescriptor,
    handle: resource::AudioStreamHandle,
) -> RuntimeResult<()> {
    let _ = (out, handle);
    Err(unsupported("destack.audio.stream.descriptor"))
}

/// Start one audio stream.
pub(crate) unsafe fn destack_audio_stream_start(
    _binding: &BindingCallContext,
    handle: resource::AudioStreamHandle,
) -> RuntimeResult<()> {
    let _ = handle;
    Err(unsupported("destack.audio.stream.start"))
}

/// Read one stream state.
pub(crate) unsafe fn destack_audio_stream_state(
    _binding: &BindingCallContext,
    out: *mut AudioStreamState,
    handle: resource::AudioStreamHandle,
) -> RuntimeResult<()> {
    let _ = (out, handle);
    Err(unsupported("destack.audio.stream.state"))
}

/// Stop one audio stream.
pub(crate) unsafe fn destack_audio_stream_stop(
    _binding: &BindingCallContext,
    handle: resource::AudioStreamHandle,
) -> RuntimeResult<()> {
    let _ = handle;
    Err(unsupported("destack.audio.stream.stop"))
}

/// Read one stream timing sample.
pub(crate) unsafe fn destack_audio_stream_timing(
    _binding: &BindingCallContext,
    out: *mut AudioStreamTiming,
    handle: resource::AudioStreamHandle,
) -> RuntimeResult<()> {
    let _ = (out, handle);
    Err(unsupported("destack.audio.stream.timing"))
}

/// Try to read one packet of captured audio frames without blocking.
pub(crate) unsafe fn destack_audio_stream_try_read(
    _binding: &BindingCallContext,
    out: *mut NativeSlice<u8>,
    handle: resource::AudioStreamHandle,
    maxbytes: u32,
) -> RuntimeResult<()> {
    let _ = (out, handle, maxbytes);
    Err(unsupported("destack.audio.stream.tryRead"))
}

/// Try to read one packet into vectorized buffers without blocking.
pub(crate) unsafe fn destack_audio_stream_try_readv(
    _binding: &BindingCallContext,
    out: *mut u64,
    handle: resource::AudioStreamHandle,
    buffers: NativeSlice<NativeSlice<u8>>,
) -> RuntimeResult<()> {
    let _ = (out, handle, buffers);
    Err(unsupported("destack.audio.stream.tryReadv"))
}

/// Try to write one packet of audio frames without blocking.
pub(crate) unsafe fn destack_audio_stream_try_write(
    _binding: &BindingCallContext,
    out: *mut u64,
    handle: resource::AudioStreamHandle,
    data: NativeSlice<u8>,
) -> RuntimeResult<()> {
    let _ = (out, handle, data);
    Err(unsupported("destack.audio.stream.tryWrite"))
}

/// Try to write one packet from vectorized buffers without blocking.
pub(crate) unsafe fn destack_audio_stream_try_writev(
    _binding: &BindingCallContext,
    out: *mut u64,
    handle: resource::AudioStreamHandle,
    buffers: NativeSlice<NativeSlice<u8>>,
) -> RuntimeResult<()> {
    let _ = (out, handle, buffers);
    Err(unsupported("destack.audio.stream.tryWritev"))
}

/// Write one packet of audio frames.
pub(crate) unsafe fn destack_audio_stream_write(
    _binding: &BindingCallContext,
    out: *mut u64,
    handle: resource::AudioStreamHandle,
    data: NativeSlice<u8>,
) -> RuntimeResult<()> {
    let _ = (out, handle, data);
    Err(unsupported("destack.audio.stream.write"))
}

/// Write one packet for one target presentation time.
pub(crate) unsafe fn destack_audio_stream_write_at(
    _binding: &BindingCallContext,
    out: *mut u64,
    handle: resource::AudioStreamHandle,
    data: NativeSlice<u8>,
    presentationtimens: u64,
) -> RuntimeResult<()> {
    let _ = (out, handle, data, presentationtimens);
    Err(unsupported("destack.audio.stream.writeAt"))
}

/// Write one vectorized packet for one target presentation time.
pub(crate) unsafe fn destack_audio_stream_write_atv(
    _binding: &BindingCallContext,
    out: *mut u64,
    handle: resource::AudioStreamHandle,
    buffers: NativeSlice<NativeSlice<u8>>,
    presentationtimens: u64,
) -> RuntimeResult<()> {
    let _ = (out, handle, buffers, presentationtimens);
    Err(unsupported("destack.audio.stream.writeAtv"))
}

/// Write one packet from vectorized buffers.
pub(crate) unsafe fn destack_audio_stream_writev(
    _binding: &BindingCallContext,
    out: *mut u64,
    handle: resource::AudioStreamHandle,
    buffers: NativeSlice<NativeSlice<u8>>,
) -> RuntimeResult<()> {
    let _ = (out, handle, buffers);
    Err(unsupported("destack.audio.stream.writev"))
}
