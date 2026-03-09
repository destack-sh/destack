use crate::diagnostic::RuntimeResult;
use crate::platform::NativeSlice;
use crate::platform::audio::core::event::{
    close_event_stream, open_event_stream, read_event, read_event_batch, try_read_event,
    try_read_event_batch,
};
use crate::platform::audio::{AudioEvent, AudioEventSubscriptionOptions};
use crate::platform::resource::AudioEventHandle;
use crate::runtime::BindingCallContext;

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
pub(crate) unsafe fn destack_audio_event_close(
    binding: &BindingCallContext,
    handle: AudioEventHandle,
) -> RuntimeResult<()> {
    unsafe { close_event_stream(binding, handle) }
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
pub(crate) unsafe fn destack_audio_event_open(
    binding: &BindingCallContext,
    out: *mut AudioEventHandle,
    options: AudioEventSubscriptionOptions,
) -> RuntimeResult<()> {
    unsafe { open_event_stream(binding, out, options) }
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
pub(crate) unsafe fn destack_audio_event_read(
    binding: &BindingCallContext,
    out: *mut AudioEvent,
    handle: AudioEventHandle,
    timeoutns: u64,
) -> RuntimeResult<()> {
    unsafe { read_event(binding, out, handle, timeoutns) }
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
pub(crate) unsafe fn destack_audio_event_read_batch(
    binding: &BindingCallContext,
    out: *mut NativeSlice<AudioEvent>,
    handle: AudioEventHandle,
    maxevents: u32,
    timeoutns: u64,
) -> RuntimeResult<()> {
    unsafe { read_event_batch(binding, out, handle, maxevents, timeoutns) }
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
pub(crate) unsafe fn destack_audio_event_try_read(
    binding: &BindingCallContext,
    out: *mut AudioEvent,
    handle: AudioEventHandle,
) -> RuntimeResult<()> {
    unsafe { try_read_event(binding, out, handle) }
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
pub(crate) unsafe fn destack_audio_event_try_read_batch(
    binding: &BindingCallContext,
    out: *mut NativeSlice<AudioEvent>,
    handle: AudioEventHandle,
    maxevents: u32,
) -> RuntimeResult<()> {
    unsafe { try_read_event_batch(binding, out, handle, maxevents) }
}
