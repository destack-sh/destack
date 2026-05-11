#![allow(dead_code)]
#![allow(unused_imports)]

use destack_vm as vm;

use super::super::{
    AudioBackend, AudioBackendDescriptorVm, AudioBackendSelectionPolicy, AudioClockDomain,
    AudioClockSnapshotVm, AudioDeviceDescriptorVm, AudioDeviceDirection, AudioDeviceListRequestVm,
    AudioDeviceOpenOptionsVm, AudioEventSubscriptionOptionsVm, AudioEventVm,
    AudioStreamAvailabilityVm, AudioStreamClockDomain, AudioStreamConfigVm,
    AudioStreamDescriptorVm, AudioStreamOpenOptionsVm, AudioStreamStateVm, AudioStreamSupportVm,
    AudioStreamTimingVm,
};
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::{PlatformError, VmSlice, resource};
use crate::runtime::BindingCallContext;

fn unsupported(operation: &'static str) -> Box<RuntimeError> {
    RuntimeError::from(PlatformError::not_supported(operation)).boxed()
}

/// List host audio backends.
pub(crate) fn destack_audio_backend_list(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
) -> RuntimeResult<VmSlice<AudioBackendDescriptorVm>> {
    Err(unsupported("destack.audio.backend.list"))
}

/// Read one timestamp in one selected clock domain.
pub(crate) fn destack_audio_clock_now(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    domain: AudioClockDomain,
) -> RuntimeResult<u64> {
    let _ = domain;
    Err(unsupported("destack.audio.clock.now"))
}

/// Read one stream clock snapshot.
pub(crate) fn destack_audio_stream_clock(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::AudioStreamHandle,
    domain: AudioStreamClockDomain,
) -> RuntimeResult<AudioClockSnapshotVm> {
    let _ = (handle, domain);
    Err(unsupported("destack.audio.clock.stream"))
}

/// Close one audio device endpoint.
pub(crate) fn destack_audio_device_close(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::AudioDeviceHandle,
) -> RuntimeResult<()> {
    let _ = handle;
    Err(unsupported("destack.audio.device.close"))
}

/// Read one default device identifier for the selected direction.
pub(crate) fn destack_audio_device_default(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    direction: AudioDeviceDirection,
    backend: AudioBackend,
    backend_policy: AudioBackendSelectionPolicy,
) -> RuntimeResult<vm::StringHandle> {
    let _ = (direction, backend, backend_policy);
    Err(unsupported("destack.audio.device.default"))
}

/// Read metadata for one opened device endpoint.
pub(crate) fn destack_audio_device_descriptor(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::AudioDeviceHandle,
) -> RuntimeResult<AudioDeviceDescriptorVm> {
    let _ = handle;
    Err(unsupported("destack.audio.device.descriptor"))
}

/// List available audio devices.
pub(crate) fn destack_audio_device_list(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    request: AudioDeviceListRequestVm,
) -> RuntimeResult<VmSlice<AudioDeviceDescriptorVm>> {
    let _ = request;
    Err(unsupported("destack.audio.device.list"))
}

/// Open one audio device endpoint.
pub(crate) fn destack_audio_device_open(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    id: vm::StringHandle,
    options: AudioDeviceOpenOptionsVm,
) -> RuntimeResult<resource::AudioDeviceHandle> {
    let _ = (id, options);
    Err(unsupported("destack.audio.device.open"))
}

/// Trigger one backend rescan.
pub(crate) fn destack_audio_device_rescan(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    backend: AudioBackend,
    backend_policy: AudioBackendSelectionPolicy,
) -> RuntimeResult<()> {
    let _ = (backend, backend_policy);
    Err(unsupported("destack.audio.device.rescan"))
}

/// Close one audio event subscription.
pub(crate) fn destack_audio_event_close(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::AudioEventHandle,
) -> RuntimeResult<()> {
    let _ = handle;
    Err(unsupported("destack.audio.event.close"))
}

/// Open one audio event subscription.
pub(crate) fn destack_audio_event_open(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    options: AudioEventSubscriptionOptionsVm,
) -> RuntimeResult<resource::AudioEventHandle> {
    let _ = options;
    Err(unsupported("destack.audio.event.open"))
}

/// Wait for one audio event.
pub(crate) fn destack_audio_event_read(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::AudioEventHandle,
    timeoutns: u64,
) -> RuntimeResult<AudioEventVm> {
    let _ = (handle, timeoutns);
    Err(unsupported("destack.audio.event.read"))
}

/// Wait for one batch of audio events.
pub(crate) fn destack_audio_event_read_batch(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::AudioEventHandle,
    maxevents: u32,
    timeoutns: u64,
) -> RuntimeResult<VmSlice<AudioEventVm>> {
    let _ = (handle, maxevents, timeoutns);
    Err(unsupported("destack.audio.event.readBatch"))
}

/// Poll one audio event without blocking.
pub(crate) fn destack_audio_event_try_read(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::AudioEventHandle,
) -> RuntimeResult<AudioEventVm> {
    let _ = handle;
    Err(unsupported("destack.audio.event.tryRead"))
}

/// Poll one batch of audio events without blocking.
pub(crate) fn destack_audio_event_try_read_batch(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::AudioEventHandle,
    maxevents: u32,
) -> RuntimeResult<VmSlice<AudioEventVm>> {
    let _ = (handle, maxevents);
    Err(unsupported("destack.audio.event.tryReadBatch"))
}

/// Abort one audio stream immediately.
pub(crate) fn destack_audio_stream_abort(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::AudioStreamHandle,
) -> RuntimeResult<()> {
    let _ = handle;
    Err(unsupported("destack.audio.stream.abort"))
}

/// Read one stream immediate availability sample.
pub(crate) fn destack_audio_stream_availability(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::AudioStreamHandle,
) -> RuntimeResult<AudioStreamAvailabilityVm> {
    let _ = handle;
    Err(unsupported("destack.audio.stream.availability"))
}

/// Close one audio stream.
pub(crate) fn destack_audio_stream_close(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::AudioStreamHandle,
) -> RuntimeResult<()> {
    let _ = handle;
    Err(unsupported("destack.audio.stream.close"))
}

/// Drain one playback stream.
pub(crate) fn destack_audio_stream_drain(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::AudioStreamHandle,
    timeoutns: u64,
) -> RuntimeResult<()> {
    let _ = (handle, timeoutns);
    Err(unsupported("destack.audio.stream.drain"))
}

/// Flush buffered stream data.
pub(crate) fn destack_audio_stream_flush(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::AudioStreamHandle,
) -> RuntimeResult<()> {
    let _ = handle;
    Err(unsupported("destack.audio.stream.flush"))
}

/// Open one audio stream on one device.
pub(crate) fn destack_audio_stream_open(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    device: resource::AudioDeviceHandle,
    config: AudioStreamConfigVm,
    options: AudioStreamOpenOptionsVm,
) -> RuntimeResult<resource::AudioStreamHandle> {
    let _ = (device, config, options);
    Err(unsupported("destack.audio.stream.open"))
}

/// Check one audio stream configuration for backend support.
pub(crate) fn destack_audio_stream_support(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    device: resource::AudioDeviceHandle,
    config: AudioStreamConfigVm,
    options: AudioStreamOpenOptionsVm,
) -> RuntimeResult<AudioStreamSupportVm> {
    let _ = (device, config, options);
    Err(unsupported("destack.audio.stream.support"))
}

/// Pause or resume one audio stream.
pub(crate) fn destack_audio_stream_pause(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::AudioStreamHandle,
    pause: bool,
) -> RuntimeResult<()> {
    let _ = (handle, pause);
    Err(unsupported("destack.audio.stream.pause"))
}

/// Read one packet of captured audio frames.
pub(crate) fn destack_audio_stream_read(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::AudioStreamHandle,
    maxbytes: u32,
) -> RuntimeResult<VmSlice<u8>> {
    let _ = (handle, maxbytes);
    Err(unsupported("destack.audio.stream.read"))
}

/// Read one packet into vectorized buffers.
pub(crate) fn destack_audio_stream_readv(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::AudioStreamHandle,
    buffers: VmSlice<VmSlice<u8>>,
) -> RuntimeResult<u64> {
    let _ = (handle, buffers);
    Err(unsupported("destack.audio.stream.readv"))
}

/// Set one stream mute state.
pub(crate) fn destack_audio_stream_set_mute(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::AudioStreamHandle,
    muted: bool,
) -> RuntimeResult<()> {
    let _ = (handle, muted);
    Err(unsupported("destack.audio.stream.setMute"))
}

/// Set one stream name.
pub(crate) fn destack_audio_stream_set_name(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::AudioStreamHandle,
    name: vm::StringHandle,
) -> RuntimeResult<()> {
    let _ = (handle, name);
    Err(unsupported("destack.audio.stream.setName"))
}

/// Set one stream gain multiplier.
pub(crate) fn destack_audio_stream_set_volume(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::AudioStreamHandle,
    lineargain: f64,
) -> RuntimeResult<()> {
    let _ = (handle, lineargain);
    Err(unsupported("destack.audio.stream.setVolume"))
}

/// Read one stream negotiated configuration descriptor.
pub(crate) fn destack_audio_stream_descriptor(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::AudioStreamHandle,
) -> RuntimeResult<AudioStreamDescriptorVm> {
    let _ = handle;
    Err(unsupported("destack.audio.stream.descriptor"))
}

/// Start one audio stream.
pub(crate) fn destack_audio_stream_start(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::AudioStreamHandle,
) -> RuntimeResult<()> {
    let _ = handle;
    Err(unsupported("destack.audio.stream.start"))
}

/// Read one stream state.
pub(crate) fn destack_audio_stream_state(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::AudioStreamHandle,
) -> RuntimeResult<AudioStreamStateVm> {
    let _ = handle;
    Err(unsupported("destack.audio.stream.state"))
}

/// Stop one audio stream.
pub(crate) fn destack_audio_stream_stop(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::AudioStreamHandle,
) -> RuntimeResult<()> {
    let _ = handle;
    Err(unsupported("destack.audio.stream.stop"))
}

/// Read one stream timing sample.
pub(crate) fn destack_audio_stream_timing(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::AudioStreamHandle,
) -> RuntimeResult<AudioStreamTimingVm> {
    let _ = handle;
    Err(unsupported("destack.audio.stream.timing"))
}

/// Try to read one packet of captured audio frames without blocking.
pub(crate) fn destack_audio_stream_try_read(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::AudioStreamHandle,
    maxbytes: u32,
) -> RuntimeResult<VmSlice<u8>> {
    let _ = (handle, maxbytes);
    Err(unsupported("destack.audio.stream.tryRead"))
}

/// Try to read one packet into vectorized buffers without blocking.
pub(crate) fn destack_audio_stream_try_readv(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::AudioStreamHandle,
    buffers: VmSlice<VmSlice<u8>>,
) -> RuntimeResult<u64> {
    let _ = (handle, buffers);
    Err(unsupported("destack.audio.stream.tryReadv"))
}

/// Try to write one packet of audio frames without blocking.
pub(crate) fn destack_audio_stream_try_write(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::AudioStreamHandle,
    data: VmSlice<u8>,
) -> RuntimeResult<u64> {
    let _ = (handle, data);
    Err(unsupported("destack.audio.stream.tryWrite"))
}

/// Try to write one packet from vectorized buffers without blocking.
pub(crate) fn destack_audio_stream_try_writev(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::AudioStreamHandle,
    buffers: VmSlice<VmSlice<u8>>,
) -> RuntimeResult<u64> {
    let _ = (handle, buffers);
    Err(unsupported("destack.audio.stream.tryWritev"))
}

/// Write one packet of audio frames.
pub(crate) fn destack_audio_stream_write(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::AudioStreamHandle,
    data: VmSlice<u8>,
) -> RuntimeResult<u64> {
    let _ = (handle, data);
    Err(unsupported("destack.audio.stream.write"))
}

/// Write one packet for one target presentation time.
pub(crate) fn destack_audio_stream_write_at(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::AudioStreamHandle,
    data: VmSlice<u8>,
    presentationtimens: u64,
) -> RuntimeResult<u64> {
    let _ = (handle, data, presentationtimens);
    Err(unsupported("destack.audio.stream.writeAt"))
}

/// Write one vectorized packet for one target presentation time.
pub(crate) fn destack_audio_stream_write_atv(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::AudioStreamHandle,
    buffers: VmSlice<VmSlice<u8>>,
    presentationtimens: u64,
) -> RuntimeResult<u64> {
    let _ = (handle, buffers, presentationtimens);
    Err(unsupported("destack.audio.stream.writeAtv"))
}

/// Write one packet from vectorized buffers.
pub(crate) fn destack_audio_stream_writev(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::AudioStreamHandle,
    buffers: VmSlice<VmSlice<u8>>,
) -> RuntimeResult<u64> {
    let _ = (handle, buffers);
    Err(unsupported("destack.audio.stream.writev"))
}
