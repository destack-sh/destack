#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(clippy::missing_safety_doc)]
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::{NativeSlice, NativeStringRef, PlatformError};

use crate::runtime::BindingCallContext;

use crate::platform::audio::{
    AudioClockDomain, AudioClockSnapshot, AudioDeviceDescriptor, AudioDeviceDirection,
    AudioDeviceEvent, AudioDeviceListRequest, AudioDeviceOpenOptions, AudioStreamAvailability,
    AudioStreamConfig, AudioStreamSnapshot, AudioStreamState, AudioStreamTiming,
};
use crate::platform::resource;

/// Read one timestamp in one selected clock domain.
///
/// Read one clock timestamp for the selected domain.
/// Domain availability and precision follow host backend behavior.
///
/// # Platform
/// Unix and Windows.
/// Uses ALSA, PulseAudio, PipeWire, CoreAudio, WASAPI, AAudio, and OpenSL ES stream-clock or runtime-clock query APIs.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioInvalidData, notSupported.
///
/// # Security
/// Requires `audio.control`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_audio_clock_now(
    _context: &BindingCallContext,
    out: *mut u64,
    domain: AudioClockDomain,
) -> RuntimeResult<()> {
    let _ = (out, domain);

    Err(RuntimeError::from(PlatformError::not_supported("destack.audio.clock.now")).boxed())
}

/// Read one stream clock snapshot.
///
/// Read one synchronized stream-position and clock timestamp snapshot.
/// Snapshot values are advisory and can change immediately after read.
///
/// # Platform
/// Unix and Windows.
/// Uses ALSA, PulseAudio, PipeWire, CoreAudio, WASAPI, AAudio, and OpenSL ES stream-position and clock correlation APIs.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioInvalidData, notSupported.
///
/// # Security
/// Requires `audio.control`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_audio_stream_clock(
    _context: &BindingCallContext,
    out: *mut AudioClockSnapshot,
    handle: resource::AudioStreamHandle,
    domain: AudioClockDomain,
) -> RuntimeResult<()> {
    let _ = (out, handle, domain);

    Err(RuntimeError::from(PlatformError::not_supported("destack.audio.clock.stream")).boxed())
}

/// Close one audio device endpoint.
///
/// Close one opened audio endpoint and release host resources.
/// Close semantics follow host backend teardown behavior.
///
/// # Platform
/// Unix and Windows.
/// Uses ALSA, PulseAudio, PipeWire, CoreAudio, WASAPI, AAudio, and OpenSL ES endpoint close operations.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `audio.device`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_audio_device_close(
    _context: &BindingCallContext,
    handle: resource::AudioDeviceHandle,
) -> RuntimeResult<()> {
    let _ = handle;

    Err(RuntimeError::from(PlatformError::not_supported("destack.audio.device.close")).boxed())
}

/// Read one default device identifier for the selected direction.
///
/// Resolve one default host audio endpoint for the selected direction.
/// Default selection can change asynchronously as host policy changes.
///
/// # Platform
/// Unix and Windows.
/// Uses ALSA, PulseAudio, PipeWire, CoreAudio, WASAPI, AAudio, and OpenSL ES default-endpoint query APIs.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioInvalidData, notSupported.
///
/// # Security
/// Requires `audio.device`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_audio_device_default(
    _context: &BindingCallContext,
    out: *mut NativeStringRef,
    direction: AudioDeviceDirection,
) -> RuntimeResult<()> {
    let _ = (out, direction);

    Err(RuntimeError::from(PlatformError::not_supported("destack.audio.device.default")).boxed())
}

/// Read metadata for one opened device endpoint.
///
/// Read one normalized snapshot for one opened device handle.
/// Snapshot values are advisory and can change as host routes are updated.
///
/// # Platform
/// Unix and Windows.
/// Uses ALSA, PulseAudio, PipeWire, CoreAudio, WASAPI, AAudio, and OpenSL ES endpoint information query APIs.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioInvalidData, notSupported.
///
/// # Security
/// Requires `audio.device`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_audio_device_descriptor(
    _context: &BindingCallContext,
    out: *mut AudioDeviceDescriptor,
    handle: resource::AudioDeviceHandle,
) -> RuntimeResult<()> {
    let _ = (out, handle);

    Err(RuntimeError::from(PlatformError::not_supported("destack.audio.device.info")).boxed())
}

/// List available audio devices.
///
/// Enumerate host audio endpoints and return stable identifiers for later open operations.
/// Device visibility and ordering follow host backend semantics.
///
/// # Platform
/// Unix and Windows.
/// Uses ALSA, PulseAudio, PipeWire, CoreAudio, WASAPI, AAudio, and OpenSL ES device enumeration.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, notSupported.
///
/// # Security
/// Requires `audio.device`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_audio_device_list(
    _context: &BindingCallContext,
    out: *mut NativeSlice<AudioDeviceDescriptor>,
    request: AudioDeviceListRequest,
) -> RuntimeResult<()> {
    let _ = (out, request);

    Err(RuntimeError::from(PlatformError::not_supported("destack.audio.device.list")).boxed())
}

/// Open one audio device endpoint.
///
/// Open one host audio endpoint for playback, capture, or duplex operation.
/// Handle lifetime and exclusivity semantics follow host backend rules.
///
/// # Platform
/// Unix and Windows.
/// Uses ALSA, PulseAudio, PipeWire, CoreAudio, WASAPI, AAudio, and OpenSL ES endpoint open operations.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `audio.device`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_audio_device_open(
    _context: &BindingCallContext,
    out: *mut resource::AudioDeviceHandle,
    id: NativeStringRef,
    options: AudioDeviceOpenOptions,
) -> RuntimeResult<()> {
    let _ = (out, id, options);

    Err(RuntimeError::from(PlatformError::not_supported("destack.audio.device.open")).boxed())
}

/// Close one audio device event subscription.
///
/// Close one event subscription and release backend notification resources.
/// Pending events are discarded.
///
/// # Platform
/// Unix and Windows.
/// Uses ALSA, PulseAudio, PipeWire, CoreAudio, WASAPI, AAudio, and OpenSL ES notification unregistration APIs.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `audio.device.monitor`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_audio_event_close(
    _context: &BindingCallContext,
    handle: resource::AudioEventHandle,
) -> RuntimeResult<()> {
    let _ = handle;

    Err(RuntimeError::from(PlatformError::not_supported("destack.audio.event.close")).boxed())
}

/// Open one audio device event subscription.
///
/// Open one backend event subscription for hotplug and default-route changes.
/// Subscription routing and queue depth follow host backend behavior.
///
/// # Platform
/// Unix and Windows.
/// Uses ALSA, PulseAudio, PipeWire, CoreAudio, WASAPI, AAudio, and OpenSL ES device-notification registration APIs.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `audio.device.monitor`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_audio_event_open(
    _context: &BindingCallContext,
    out: *mut resource::AudioEventHandle,
) -> RuntimeResult<()> {
    let _ = out;

    Err(RuntimeError::from(PlatformError::not_supported("destack.audio.event.open")).boxed())
}

/// Wait for one audio device event.
///
/// Wait for one pending device event from one subscription queue.
/// Timeout uses nanoseconds in the runtime monotonic domain.
///
/// # Platform
/// Unix and Windows.
/// Uses ALSA, PulseAudio, PipeWire, CoreAudio, WASAPI, AAudio, and OpenSL ES event wait or callback-queue drain operations.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioInterrupted, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `audio.device.monitor`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_audio_event_read(
    _context: &BindingCallContext,
    out: *mut AudioDeviceEvent,
    handle: resource::AudioEventHandle,
    timeoutns: u64,
) -> RuntimeResult<()> {
    let _ = (out, handle, timeoutns);

    Err(RuntimeError::from(PlatformError::not_supported("destack.audio.event.read")).boxed())
}

/// Poll one audio device event without blocking.
///
/// Poll one pending device event from one subscription queue.
/// Empty queue state is reported through ioWouldBlock.
///
/// # Platform
/// Unix and Windows.
/// Uses ALSA, PulseAudio, PipeWire, CoreAudio, WASAPI, AAudio, and OpenSL ES nonblocking event queue reads.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `audio.device.monitor`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_audio_event_try_read(
    _context: &BindingCallContext,
    out: *mut AudioDeviceEvent,
    handle: resource::AudioEventHandle,
) -> RuntimeResult<()> {
    let _ = (out, handle);

    Err(RuntimeError::from(PlatformError::not_supported("destack.audio.event.tryRead")).boxed())
}

/// Read one stream immediate availability snapshot.
///
/// Read one point-in-time snapshot of immediately readable and writable frame counts.
/// Values are advisory and can change immediately after read.
///
/// # Platform
/// Unix and Windows.
/// Uses ALSA, PulseAudio, PipeWire, CoreAudio, WASAPI, AAudio, and OpenSL ES stream-space query operations.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioInvalidData, notSupported.
///
/// # Security
/// Requires `audio.control`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_audio_stream_availability(
    _context: &BindingCallContext,
    out: *mut AudioStreamAvailability,
    handle: resource::AudioStreamHandle,
) -> RuntimeResult<()> {
    let _ = (out, handle);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.audio.stream.availability",
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
/// Uses ALSA, PulseAudio, PipeWire, CoreAudio, WASAPI, AAudio, and OpenSL ES stream close operations.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `audio.stream`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_audio_stream_close(
    _context: &BindingCallContext,
    handle: resource::AudioStreamHandle,
) -> RuntimeResult<()> {
    let _ = handle;

    Err(RuntimeError::from(PlatformError::not_supported("destack.audio.stream.close")).boxed())
}

/// Drain one playback stream.
///
/// Wait for one playback stream to consume currently queued samples.
/// Drain timeout is expressed in nanoseconds in the runtime monotonic domain.
///
/// # Platform
/// Unix and Windows.
/// Uses ALSA, PulseAudio, PipeWire, CoreAudio, WASAPI, AAudio, and OpenSL ES drain or synchronized-stop operations.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioInterrupted, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `audio.playback`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_audio_stream_drain(
    _context: &BindingCallContext,
    handle: resource::AudioStreamHandle,
    timeoutns: u64,
) -> RuntimeResult<()> {
    let _ = (handle, timeoutns);

    Err(RuntimeError::from(PlatformError::not_supported("destack.audio.stream.drain")).boxed())
}

/// Flush buffered stream data.
///
/// Drop pending buffered data for one stream without closing it.
/// Flushing semantics are backend-defined for capture and duplex streams.
///
/// # Platform
/// Unix and Windows.
/// Uses ALSA, PulseAudio, PipeWire, CoreAudio, WASAPI, AAudio, and OpenSL ES stream flush or reset operations.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `audio.control`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_audio_stream_flush(
    _context: &BindingCallContext,
    handle: resource::AudioStreamHandle,
) -> RuntimeResult<()> {
    let _ = handle;

    Err(RuntimeError::from(PlatformError::not_supported("destack.audio.stream.flush")).boxed())
}

/// Read one stream negotiated configuration snapshot.
///
/// Read one normalized snapshot of negotiated stream parameters and backend mode.
/// Values reflect backend negotiation outcomes and can differ from open-time requests.
///
/// # Platform
/// Unix and Windows.
/// Uses ALSA, PulseAudio, PipeWire, CoreAudio, WASAPI, AAudio, and OpenSL ES stream-parameter query operations.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioInvalidData, notSupported.
///
/// # Security
/// Requires `audio.control`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_audio_stream_snapshot(
    _context: &BindingCallContext,
    out: *mut AudioStreamSnapshot,
    handle: resource::AudioStreamHandle,
) -> RuntimeResult<()> {
    let _ = (out, handle);

    Err(RuntimeError::from(PlatformError::not_supported("destack.audio.stream.info")).boxed())
}

/// Open one audio stream on one device.
///
/// Create one host audio stream with explicit sample format, channel, and period configuration.
/// Buffering and latency behavior follow host backend contracts.
///
/// # Platform
/// Unix and Windows.
/// Uses ALSA, PulseAudio, PipeWire, CoreAudio, WASAPI, AAudio, and OpenSL ES stream creation APIs.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `audio.stream`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_audio_stream_open(
    _context: &BindingCallContext,
    out: *mut resource::AudioStreamHandle,
    device: resource::AudioDeviceHandle,
    config: AudioStreamConfig,
) -> RuntimeResult<()> {
    let _ = (out, device, config);

    Err(RuntimeError::from(PlatformError::not_supported("destack.audio.stream.open")).boxed())
}

/// Read one packet of captured audio frames.
///
/// Read one packet of captured interleaved audio frames from the capture stream.
/// Packet sizing and buffering follow host backend capture contracts.
///
/// # Platform
/// Unix and Windows.
/// Uses ALSA, PulseAudio, PipeWire, CoreAudio, WASAPI, AAudio, and OpenSL ES stream read or capture-client operations.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `audio.capture`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_audio_stream_read(
    _context: &BindingCallContext,
    out: *mut NativeSlice<u8>,
    handle: resource::AudioStreamHandle,
    maxbytes: u32,
) -> RuntimeResult<()> {
    let _ = (out, handle, maxbytes);

    Err(RuntimeError::from(PlatformError::not_supported("destack.audio.stream.read")).boxed())
}

/// Set one stream mute state.
///
/// Apply one mute state for one stream where backend controls are available.
/// Mute behavior can be backend-local and independent of global endpoint mute.
///
/// # Platform
/// Unix and Windows.
/// Uses ALSA, PulseAudio, PipeWire, CoreAudio, WASAPI, AAudio, and OpenSL ES stream mute controls when available.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, notSupported.
///
/// # Security
/// Requires `audio.control`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_audio_stream_set_mute(
    _context: &BindingCallContext,
    handle: resource::AudioStreamHandle,
    muted: bool,
) -> RuntimeResult<()> {
    let _ = (handle, muted);

    Err(RuntimeError::from(PlatformError::not_supported("destack.audio.stream.setMute")).boxed())
}

/// Set one stream gain multiplier.
///
/// Apply one linear gain multiplier for one stream where backend controls are available.
/// Gain handling can be backend-local and independent of global mixer volume.
///
/// # Platform
/// Unix and Windows.
/// Uses ALSA, PulseAudio, PipeWire, CoreAudio, WASAPI, AAudio, and OpenSL ES stream volume controls when available.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, notSupported.
///
/// # Security
/// Requires `audio.control`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_audio_stream_set_volume(
    _context: &BindingCallContext,
    handle: resource::AudioStreamHandle,
    lineargain: f64,
) -> RuntimeResult<()> {
    let _ = (handle, lineargain);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.audio.stream.setVolume",
    ))
    .boxed())
}

/// Start one audio stream.
///
/// Transition one opened stream to running state and begin host DMA or scheduler processing.
/// Start timing follows host backend scheduling semantics.
///
/// # Platform
/// Unix and Windows.
/// Uses ALSA, PulseAudio, PipeWire, CoreAudio, WASAPI, AAudio, and OpenSL ES stream start operations.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `audio.stream`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_audio_stream_start(
    _context: &BindingCallContext,
    handle: resource::AudioStreamHandle,
) -> RuntimeResult<()> {
    let _ = handle;

    Err(RuntimeError::from(PlatformError::not_supported("destack.audio.stream.start")).boxed())
}

/// Read one stream state snapshot.
///
/// Read one point-in-time snapshot of stream run state and backend buffering metrics.
/// Snapshot values are advisory and can change immediately after read.
///
/// # Platform
/// Unix and Windows.
/// Uses ALSA, PulseAudio, PipeWire, CoreAudio, WASAPI, AAudio, and OpenSL ES stream query primitives.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioInvalidData, notSupported.
///
/// # Security
/// Requires `audio.control`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_audio_stream_state(
    _context: &BindingCallContext,
    out: *mut AudioStreamState,
    handle: resource::AudioStreamHandle,
) -> RuntimeResult<()> {
    let _ = (out, handle);

    Err(RuntimeError::from(PlatformError::not_supported("destack.audio.stream.state")).boxed())
}

/// Stop one audio stream.
///
/// Transition one running stream to stopped state and flush host backend scheduling.
/// Buffered frames can be discarded based on host backend semantics.
///
/// # Platform
/// Unix and Windows.
/// Uses ALSA, PulseAudio, PipeWire, CoreAudio, WASAPI, AAudio, and OpenSL ES stream stop operations.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `audio.stream`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_audio_stream_stop(
    _context: &BindingCallContext,
    handle: resource::AudioStreamHandle,
) -> RuntimeResult<()> {
    let _ = handle;

    Err(RuntimeError::from(PlatformError::not_supported("destack.audio.stream.stop")).boxed())
}

/// Read one stream timing snapshot.
///
/// Read one timing snapshot that correlates stream position and host device time.
/// Timing values are intended for drift correction and synchronization.
///
/// # Platform
/// Unix and Windows.
/// Uses ALSA, PulseAudio, PipeWire, CoreAudio, WASAPI, AAudio, and OpenSL ES stream-clock query operations.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioInvalidData, notSupported.
///
/// # Security
/// Requires `audio.control`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_audio_stream_timing(
    _context: &BindingCallContext,
    out: *mut AudioStreamTiming,
    handle: resource::AudioStreamHandle,
) -> RuntimeResult<()> {
    let _ = (out, handle);

    Err(RuntimeError::from(PlatformError::not_supported("destack.audio.stream.timing")).boxed())
}

/// Try to read one packet of captured audio frames without blocking.
///
/// Read one packet of captured interleaved audio frames without waiting.
/// Empty input state is reported through ioWouldBlock.
///
/// # Platform
/// Unix and Windows.
/// Uses ALSA, PulseAudio, PipeWire, CoreAudio, WASAPI, AAudio, and OpenSL ES nonblocking stream read operations.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `audio.capture`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_audio_stream_try_read(
    _context: &BindingCallContext,
    out: *mut NativeSlice<u8>,
    handle: resource::AudioStreamHandle,
    maxbytes: u32,
) -> RuntimeResult<()> {
    let _ = (out, handle, maxbytes);

    Err(RuntimeError::from(PlatformError::not_supported("destack.audio.stream.tryRead")).boxed())
}

/// Try to write one packet of audio frames without blocking.
///
/// Submit one packet of interleaved audio frames to the playback stream without waiting.
/// Empty output space is reported through ioWouldBlock.
///
/// # Platform
/// Unix and Windows.
/// Uses ALSA, PulseAudio, PipeWire, CoreAudio, WASAPI, AAudio, and OpenSL ES nonblocking stream write operations.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `audio.playback`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_audio_stream_try_write(
    _context: &BindingCallContext,
    out: *mut u64,
    handle: resource::AudioStreamHandle,
    data: NativeSlice<u8>,
) -> RuntimeResult<()> {
    let _ = (out, handle, data);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.audio.stream.tryWrite",
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
/// Uses ALSA, PulseAudio, PipeWire, CoreAudio, WASAPI, AAudio, and OpenSL ES stream write or render-client operations.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `audio.playback`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_audio_stream_write(
    _context: &BindingCallContext,
    out: *mut u64,
    handle: resource::AudioStreamHandle,
    data: NativeSlice<u8>,
) -> RuntimeResult<()> {
    let _ = (out, handle, data);

    Err(RuntimeError::from(PlatformError::not_supported("destack.audio.stream.write")).boxed())
}

/// Write one packet for one target presentation time.
///
/// Submit one packet of interleaved audio frames for one target presentation timestamp.
/// Scheduling precision depends on host backend timing guarantees.
/// This is one optional scheduling lane and can return `notSupported` when unavailable.
///
/// # Platform
/// Unix and Windows.
/// Uses ALSA, PulseAudio, PipeWire, CoreAudio, WASAPI, AAudio, and OpenSL ES scheduled-render operations when available.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `audio.playback.schedule`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_audio_stream_write_at(
    _context: &BindingCallContext,
    out: *mut u64,
    handle: resource::AudioStreamHandle,
    data: NativeSlice<u8>,
    presentationtimens: u64,
) -> RuntimeResult<()> {
    let _ = (out, handle, data, presentationtimens);

    Err(RuntimeError::from(PlatformError::not_supported("destack.audio.stream.writeAt")).boxed())
}
