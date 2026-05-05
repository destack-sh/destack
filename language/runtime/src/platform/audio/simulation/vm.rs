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
///
/// Enumerate available backend implementations and backend-level feature flags.
///
/// # Platform
/// Unix and Windows.
/// Mirrors cubeb `cubeb_get_backend_names`, libsoundio `soundio_backend_count` plus `soundio_get_backend`, and miniaudio `ma_get_enabled_backends`.
/// Mirrors PortAudio host-api enumeration through `PaHostApiTypeId`.
///
/// # Errors
/// Returns ioNotFound, ioInvalidData, notSupported.
///
/// # Security
/// Requires `audio.device`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_audio_backend_list(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
) -> RuntimeResult<VmSlice<AudioBackendDescriptorVm>> {
    Err(unsupported("destack.audio.backend.list"))
}

/// Read one timestamp in one selected clock domain.
///
/// Read one clock timestamp for one process-wide domain.
/// Domain availability and precision follow host platform behavior.
///
/// # Platform
/// Unix and Windows.
/// Mirrors host monotonic and wall clock query semantics.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioInvalidData, notSupported.
///
/// # Security
/// Requires `audio.control`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_audio_clock_now(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    domain: AudioClockDomain,
) -> RuntimeResult<u64> {
    let _ = domain;
    Err(unsupported("destack.audio.clock.now"))
}

/// Read one stream clock snapshot.
///
/// Read one synchronized stream-position and selected clock-domain timestamp snapshot.
/// Snapshot values are advisory and can change immediately after read.
/// This is the strict lane-select API.
/// For one full best-effort snapshot without lane-specific errors use `audio.stream.timing`.
/// Domain-specific lanes like `InputAdc`, `OutputDac`, and `Device` can return `notSupported` when the opened stream does not expose them.
///
/// # Platform
/// Unix and Windows.
/// Mirrors PortAudio `PaStreamCallbackTimeInfo` input and output timestamp correlation.
/// Mirrors ASIO `bufferSwitchTimeInfo` and time-info correlation semantics.
/// Mirrors cubeb `cubeb_stream_get_position` plus latency-correlation snapshots.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioInvalidData, notSupported.
///
/// # Security
/// Requires `audio.control`.
///
/// # Replay
/// External, recordable.
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
///
/// Close one opened audio endpoint and release host resources.
/// Close semantics follow host backend teardown behavior.
///
/// # Platform
/// Unix and Windows.
/// Uses ALSA, PulseAudio, PipeWire, CoreAudio, WASAPI, AAudio, OpenSL ES, JACK, and ASIO where available endpoint close operations.
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
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::AudioDeviceHandle,
) -> RuntimeResult<()> {
    let _ = handle;
    Err(unsupported("destack.audio.device.close"))
}

/// Read one default device identifier for the selected direction.
///
/// Resolve one default host audio endpoint for the selected direction and backend policy.
/// Default selection can change asynchronously as host policy changes.
///
/// # Platform
/// Unix and Windows.
/// Mirrors cubeb and libsoundio default-endpoint query semantics.
/// Mirrors SDL default logical-device routing behavior for playback and recording defaults.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioInvalidData, notSupported.
///
/// # Security
/// Requires `audio.device`.
///
/// # Replay
/// External, recordable.
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
///
/// Read one normalized snapshot for one opened device handle.
/// Snapshot values are advisory and can change as host routes are updated.
///
/// # Platform
/// Unix and Windows.
/// Uses ALSA, PulseAudio, PipeWire, CoreAudio, WASAPI, AAudio, OpenSL ES, JACK, and ASIO where available endpoint information query APIs.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioInvalidData, notSupported.
///
/// # Security
/// Requires `audio.device`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_audio_device_descriptor(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::AudioDeviceHandle,
) -> RuntimeResult<AudioDeviceDescriptorVm> {
    let _ = handle;
    Err(unsupported("destack.audio.device.descriptor"))
}

/// List available audio devices.
///
/// Enumerate host audio endpoints and return stable identifiers for later open operations.
/// Device visibility and ordering follow host backend semantics.
///
/// # Platform
/// Unix and Windows.
/// Uses ALSA, PulseAudio, PipeWire, CoreAudio, WASAPI, AAudio, OpenSL ES, JACK, and ASIO where available device enumeration.
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
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    request: AudioDeviceListRequestVm,
) -> RuntimeResult<VmSlice<AudioDeviceDescriptorVm>> {
    let _ = request;
    Err(unsupported("destack.audio.device.list"))
}

/// Open one audio device endpoint.
///
/// Open one host audio endpoint for playback, capture, duplex, or loopback operation.
/// Handle lifetime and exclusivity semantics follow host backend rules.
///
/// # Platform
/// Unix and Windows.
/// Uses ALSA, PulseAudio, PipeWire, CoreAudio, WASAPI, AAudio, OpenSL ES, JACK, and ASIO where available endpoint open operations.
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
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    id: vm::StringHandle,
    options: AudioDeviceOpenOptionsVm,
) -> RuntimeResult<resource::AudioDeviceHandle> {
    let _ = (id, options);
    Err(unsupported("destack.audio.device.open"))
}

/// Trigger one backend rescan.
///
/// Request one immediate backend device rescan.
/// This allows recovery from stale backend snapshots after hotplug churn.
///
/// # Platform
/// Unix and Windows.
/// Mirrors libsoundio `soundio_force_device_scan` semantics and backend-native refresh flows.
///
/// # Errors
/// Returns ioNotFound, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `audio.device.monitor`.
///
/// # Replay
/// External, recordable.
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
///
/// Close one event subscription and release backend notification resources.
/// Pending events are discarded.
///
/// # Platform
/// Unix and Windows.
/// Uses ALSA, PulseAudio, PipeWire, CoreAudio, WASAPI, AAudio, OpenSL ES, JACK, and ASIO where available notification unregistration APIs.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `audio.device.monitor`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_audio_event_close(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::AudioEventHandle,
) -> RuntimeResult<()> {
    let _ = handle;
    Err(unsupported("destack.audio.event.close"))
}

/// Open one audio event subscription.
///
/// Open one backend event subscription for device and optional stream events.
/// Subscription routing and queue depth follow host backend behavior.
///
/// # Platform
/// Unix and Windows.
/// Mirrors cubeb device and stream change callbacks.
/// Mirrors libsoundio device-change and backend-disconnect callback families.
/// Mirrors miniaudio `ma_device_notification_proc` notification routing.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `audio.device.monitor`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_audio_event_open(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    options: AudioEventSubscriptionOptionsVm,
) -> RuntimeResult<resource::AudioEventHandle> {
    let _ = options;
    Err(unsupported("destack.audio.event.open"))
}

/// Wait for one audio event.
///
/// Wait for one pending event from one subscription queue.
/// Timeout uses nanoseconds in the runtime monotonic domain.
///
/// # Platform
/// Unix and Windows.
/// Uses ALSA, PulseAudio, PipeWire, CoreAudio, WASAPI, AAudio, OpenSL ES, JACK, and ASIO where available event wait or callback-queue drain operations.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioInterrupted, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `audio.device.monitor`.
///
/// # Replay
/// External, recordable.
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
///
/// Wait for pending events from one subscription queue and return up to `maxEvents` events.
/// Timeout uses nanoseconds in the runtime monotonic domain.
/// Empty queue state is reported through ioWouldBlock.
///
/// # Platform
/// Unix and Windows.
/// Uses ALSA, PulseAudio, PipeWire, CoreAudio, WASAPI, AAudio, OpenSL ES, JACK, and ASIO queue-drain operations where available.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioInterrupted, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `audio.device.monitor`.
///
/// # Replay
/// External, recordable.
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
///
/// Poll one pending event from one subscription queue.
/// Empty queue state is reported through ioWouldBlock.
///
/// # Platform
/// Unix and Windows.
/// Uses ALSA, PulseAudio, PipeWire, CoreAudio, WASAPI, AAudio, OpenSL ES, JACK, and ASIO where available nonblocking event queue reads.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `audio.device.monitor`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_audio_event_try_read(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::AudioEventHandle,
) -> RuntimeResult<AudioEventVm> {
    let _ = handle;
    Err(unsupported("destack.audio.event.tryRead"))
}

/// Poll one batch of audio events without blocking.
///
/// Poll pending events from one subscription queue and return up to `maxEvents` events.
/// Empty queue state is reported through ioWouldBlock.
///
/// # Platform
/// Unix and Windows.
/// Uses ALSA, PulseAudio, PipeWire, CoreAudio, WASAPI, AAudio, OpenSL ES, JACK, and ASIO nonblocking queue-drain operations where available.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `audio.device.monitor`.
///
/// # Replay
/// External, recordable.
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
///
/// Request one immediate stream stop without graceful drain.
/// Pending buffered data can be discarded.
///
/// # Platform
/// Unix and Windows.
/// Mirrors PortAudio `Pa_AbortStream` semantics.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioInterrupted, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `audio.stream`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_audio_stream_abort(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::AudioStreamHandle,
) -> RuntimeResult<()> {
    let _ = handle;
    Err(unsupported("destack.audio.stream.abort"))
}

/// Read one stream immediate availability sample.
///
/// Read one point-in-time sample of immediately readable and writable frame counts.
/// Values are advisory and can change immediately after read.
///
/// # Platform
/// Unix and Windows.
/// Uses ALSA, PulseAudio, PipeWire, CoreAudio, WASAPI, AAudio, OpenSL ES, JACK, and ASIO where available stream-space query operations.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioInvalidData, notSupported.
///
/// # Security
/// Requires `audio.control`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_audio_stream_availability(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::AudioStreamHandle,
) -> RuntimeResult<AudioStreamAvailabilityVm> {
    let _ = handle;
    Err(unsupported("destack.audio.stream.availability"))
}

/// Close one audio stream.
///
/// Close one host audio stream and release backend buffers and synchronization state.
/// Stream handle becomes invalid after close completes.
///
/// # Platform
/// Unix and Windows.
/// Uses ALSA, PulseAudio, PipeWire, CoreAudio, WASAPI, AAudio, OpenSL ES, JACK, and ASIO where available stream close operations.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `audio.stream`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_audio_stream_close(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::AudioStreamHandle,
) -> RuntimeResult<()> {
    let _ = handle;
    Err(unsupported("destack.audio.stream.close"))
}

/// Drain one playback stream.
///
/// Wait for one playback stream to consume currently queued samples.
/// Drain timeout is expressed in nanoseconds in the runtime monotonic domain.
///
/// # Platform
/// Unix and Windows.
/// Uses ALSA, PulseAudio, PipeWire, CoreAudio, WASAPI, AAudio, OpenSL ES, JACK, and ASIO where available drain or synchronized-stop operations.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioInterrupted, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `audio.playback`.
///
/// # Replay
/// External, recordable.
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
///
/// Drop pending buffered data for one stream without closing it.
/// Flushing semantics are backend-defined for capture and duplex streams.
///
/// # Platform
/// Unix and Windows.
/// Uses ALSA, PulseAudio, PipeWire, CoreAudio, WASAPI, AAudio, OpenSL ES, JACK, and ASIO where available stream flush or reset operations.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `audio.control`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_audio_stream_flush(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::AudioStreamHandle,
) -> RuntimeResult<()> {
    let _ = handle;
    Err(unsupported("destack.audio.stream.flush"))
}

/// Open one audio stream on one device.
///
/// Create one host audio stream with explicit sample format, channel, and period configuration.
/// Open options carry optional tuning hints and strict requirement lanes.
/// Any unsatisfied requirement must fail open with `notSupported`.
/// Buffering and latency behavior follow host backend contracts.
///
/// # Platform
/// Unix and Windows.
/// Uses ALSA, PulseAudio, PipeWire, CoreAudio, WASAPI, AAudio, OpenSL ES, JACK, and ASIO where available stream creation APIs.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `audio.stream`.
///
/// # Replay
/// External, recordable.
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
///
/// Check one stream configuration and return backend negotiation results without opening one long-lived stream handle.
/// Requirement flags are resolved into `satisfiedRequirements` and `unsatisfiedRequirements`.
///
/// # Platform
/// Unix and Windows.
/// Mirrors PortAudio `Pa_IsFormatSupported` intent and miniaudio native-format probing behavior.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `audio.stream`.
///
/// # Replay
/// External, recordable.
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
///
/// Transition one running stream into paused state and back.
/// Pause support is backend-dependent.
///
/// # Platform
/// Unix and Windows.
/// Mirrors libsoundio `soundio_outstream_pause` and SDL stream-device pause semantics.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioInterrupted, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `audio.stream`.
///
/// # Replay
/// External, recordable.
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
///
/// Read one packet of captured interleaved audio frames from the capture stream.
/// Packet sizing and buffering follow host backend capture contracts.
///
/// # Platform
/// Unix and Windows.
/// Uses ALSA, PulseAudio, PipeWire, CoreAudio, WASAPI, AAudio, OpenSL ES, JACK, and ASIO where available stream read or capture-client operations.
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
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::AudioStreamHandle,
    maxbytes: u32,
) -> RuntimeResult<VmSlice<u8>> {
    let _ = (handle, maxbytes);
    Err(unsupported("destack.audio.stream.read"))
}

/// Read one packet into vectorized buffers.
///
/// Read one packet of captured audio frames into multiple byte slices.
/// Buffers can represent segmented interleaved payloads or channel planes when non-interleaved mode is active.
///
/// # Platform
/// Unix and Windows.
/// Mirrors readv-style capture behavior and backend non-interleaved lanes where available.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `audio.capture`.
///
/// # Replay
/// External, recordable.
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
///
/// Apply one mute state for one stream processing lane.
/// This controls stream-level mute and does not imply global endpoint mute ownership.
///
/// # Platform
/// Unix and Windows.
/// Uses ALSA, PulseAudio, PipeWire, CoreAudio, WASAPI, AAudio, OpenSL ES, JACK, and ASIO stream-level mute paths where available.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, notSupported.
///
/// # Security
/// Requires `audio.control`.
///
/// # Replay
/// External, recordable.
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
///
/// Apply one stream label for host mixers and diagnostics where supported.
/// Backend label visibility and truncation follow host policy.
///
/// # Platform
/// Unix and Windows.
/// Mirrors cubeb `cubeb_stream_set_name` behavior where available.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, notSupported.
///
/// # Security
/// Requires `audio.control`.
///
/// # Replay
/// External, recordable.
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
///
/// Apply one linear gain multiplier for one stream processing lane.
/// This controls stream-level gain and does not imply global endpoint mixer ownership.
///
/// # Platform
/// Unix and Windows.
/// Uses ALSA, PulseAudio, PipeWire, CoreAudio, WASAPI, AAudio, OpenSL ES, JACK, and ASIO stream-level gain paths where available.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, notSupported.
///
/// # Security
/// Requires `audio.control`.
///
/// # Replay
/// External, recordable.
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
///
/// Read one normalized view of negotiated stream parameters and backend mode.
/// Values reflect backend negotiation outcomes and can differ from open-time requests.
///
/// # Platform
/// Unix and Windows.
/// Uses ALSA, PulseAudio, PipeWire, CoreAudio, WASAPI, AAudio, OpenSL ES, JACK, and ASIO where available stream-parameter query operations.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioInvalidData, notSupported.
///
/// # Security
/// Requires `audio.control`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_audio_stream_descriptor(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::AudioStreamHandle,
) -> RuntimeResult<AudioStreamDescriptorVm> {
    let _ = handle;
    Err(unsupported("destack.audio.stream.descriptor"))
}

/// Start one audio stream.
///
/// Transition one opened stream to running state and begin host DMA or scheduler processing.
/// Start timing follows host backend scheduling semantics.
///
/// # Platform
/// Unix and Windows.
/// Uses ALSA, PulseAudio, PipeWire, CoreAudio, WASAPI, AAudio, OpenSL ES, JACK, and ASIO where available stream start operations.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `audio.stream`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_audio_stream_start(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::AudioStreamHandle,
) -> RuntimeResult<()> {
    let _ = handle;
    Err(unsupported("destack.audio.stream.start"))
}

/// Read one stream state.
///
/// Read one point-in-time state sample of stream run state and backend buffering metrics.
/// State values are advisory and can change immediately after read.
///
/// # Platform
/// Unix and Windows.
/// Uses ALSA, PulseAudio, PipeWire, CoreAudio, WASAPI, AAudio, OpenSL ES, JACK, and ASIO where available stream query primitives.
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
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::AudioStreamHandle,
) -> RuntimeResult<AudioStreamStateVm> {
    let _ = handle;
    Err(unsupported("destack.audio.stream.state"))
}

/// Stop one audio stream.
///
/// Transition one running stream to stopped state and flush host backend scheduling.
/// Buffered frames can be discarded based on host backend semantics.
///
/// # Platform
/// Unix and Windows.
/// Uses ALSA, PulseAudio, PipeWire, CoreAudio, WASAPI, AAudio, OpenSL ES, JACK, and ASIO where available stream stop operations.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `audio.stream`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_audio_stream_stop(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::AudioStreamHandle,
) -> RuntimeResult<()> {
    let _ = handle;
    Err(unsupported("destack.audio.stream.stop"))
}

/// Read one stream timing sample.
///
/// Read one full timing sample that correlates stream position and available backend clocks.
/// Missing optional lanes are reported through `has*` fields instead of `notSupported`.
/// Timing values are intended for drift correction and synchronization.
///
/// # Platform
/// Unix and Windows.
/// Uses ALSA, PulseAudio, PipeWire, CoreAudio, WASAPI, AAudio, OpenSL ES, JACK, and ASIO where available stream-clock query operations.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioInvalidData, notSupported.
///
/// # Security
/// Requires `audio.control`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_audio_stream_timing(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::AudioStreamHandle,
) -> RuntimeResult<AudioStreamTimingVm> {
    let _ = handle;
    Err(unsupported("destack.audio.stream.timing"))
}

/// Try to read one packet of captured audio frames without blocking.
///
/// Read one packet of captured interleaved audio frames without waiting.
/// Empty input state is reported through ioWouldBlock.
///
/// # Platform
/// Unix and Windows.
/// Uses ALSA, PulseAudio, PipeWire, CoreAudio, WASAPI, AAudio, OpenSL ES, JACK, and ASIO where available nonblocking stream read operations.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `audio.capture`.
///
/// # Replay
/// External, recordable.
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
///
/// Read one packet of captured audio frames into multiple byte slices without waiting.
/// Empty input state is reported through ioWouldBlock.
///
/// # Platform
/// Unix and Windows.
/// Mirrors nonblocking readv-style capture behavior.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `audio.capture`.
///
/// # Replay
/// External, recordable.
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
///
/// Submit one packet of interleaved audio frames to the playback stream without waiting.
/// Empty output space is reported through ioWouldBlock.
///
/// # Platform
/// Unix and Windows.
/// Uses ALSA, PulseAudio, PipeWire, CoreAudio, WASAPI, AAudio, OpenSL ES, JACK, and ASIO where available nonblocking stream write operations.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `audio.playback`.
///
/// # Replay
/// External, recordable.
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
///
/// Submit one packet of audio frames from multiple byte slices without waiting.
/// Empty output space is reported through ioWouldBlock.
///
/// # Platform
/// Unix and Windows.
/// Mirrors nonblocking writev-style submission behavior.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `audio.playback`.
///
/// # Replay
/// External, recordable.
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
///
/// Submit one packet of interleaved audio frames to the playback stream.
/// Short writes can occur when host buffers are near capacity.
///
/// # Platform
/// Unix and Windows.
/// Uses ALSA, PulseAudio, PipeWire, CoreAudio, WASAPI, AAudio, OpenSL ES, JACK, and ASIO where available stream write or render-client operations.
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
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::AudioStreamHandle,
    data: VmSlice<u8>,
) -> RuntimeResult<u64> {
    let _ = (handle, data);
    Err(unsupported("destack.audio.stream.write"))
}

/// Write one packet for one target presentation time.
///
/// Submit one packet of interleaved audio frames for one target presentation timestamp.
/// Scheduling precision depends on host backend timing guarantees.
/// This is one optional scheduling lane and can return `notSupported` when unavailable.
///
/// # Platform
/// Unix and Windows.
/// Uses ALSA, PulseAudio, PipeWire, CoreAudio, WASAPI, AAudio, OpenSL ES, JACK, and ASIO where available scheduled-render operations when available.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `audio.playback.schedule`.
///
/// # Replay
/// External, recordable.
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
///
/// Submit one packet of audio frames from multiple byte slices for one target presentation timestamp.
/// Buffers can represent segmented interleaved payloads or channel planes when non-interleaved mode is active.
/// Scheduling precision depends on host backend timing guarantees.
///
/// # Platform
/// Unix and Windows.
/// Mirrors scheduled-render operations where available and extends them for writev-style payload submission.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `audio.playback.schedule`.
///
/// # Replay
/// External, recordable.
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
///
/// Submit one packet of audio frames from multiple byte slices.
/// Buffers can represent segmented interleaved payloads or channel planes when non-interleaved mode is active.
///
/// # Platform
/// Unix and Windows.
/// Mirrors writev-style submission behavior and backend non-interleaved lanes where available.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `audio.playback`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_audio_stream_writev(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::AudioStreamHandle,
    buffers: VmSlice<VmSlice<u8>>,
) -> RuntimeResult<u64> {
    let _ = (handle, buffers);
    Err(unsupported("destack.audio.stream.writev"))
}
