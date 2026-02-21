use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use super::super::resolve_requested_backend;
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::audio::{AudioEvent, AudioEventSubscriptionOptions, core as audio_core};
use crate::platform::resource::{ResourceEntry, ResourceKind};
use crate::platform::{PlatformError, resource};
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
    context: &BindingCallContext,
    handle: resource::AudioEventHandle,
) -> RuntimeResult<()> {
    let removed = context.runtime().resources.remove(handle.0);
    if removed.is_none() {
        return Err(audio_core::audio_not_found(
            "destack.audio.event.close",
            format!("unknown audio event handle {}", handle.0.0),
        ));
    }

    Ok(())
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
    context: &BindingCallContext,
    out: *mut resource::AudioEventHandle,
    options: AudioEventSubscriptionOptions,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    let backend = resolve_requested_backend(
        options.backend,
        options.backend_policy,
        "destack.audio.event.open",
    )?;

    let mut options = options;
    options.backend = backend;

    let payload = Arc::new(Mutex::new(audio_core::AudioEventBinding {
        options,
        previous_signatures: HashMap::new(),
        previous_default_playback: None,
        previous_default_capture: None,
        previous_default_loopback: None,
        previous_stream_state: None,
        previous_stream_device_id: None,
        previous_stream_xrun_count: 0,
        pending: VecDeque::new(),
    }));

    let handle = context.runtime().resources.insert(
        ResourceEntry::new(ResourceKind::AudioEvent)
            .with_label(audio_core::AUDIO_EVENT_RESOURCE_LABEL)
            .with_payload(payload),
    );

    unsafe {
        *out = resource::AudioEventHandle(handle);
    }

    Ok(())
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
    context: &BindingCallContext,
    out: *mut AudioEvent,
    handle: resource::AudioEventHandle,
    timeoutns: u64,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    let binding = audio_core::resolve_event_binding(context, handle, "destack.audio.event.read")?;
    let deadline = audio_core::host_monotonic_nanos().saturating_add(timeoutns);

    loop {
        let mut guard = binding.lock().unwrap_or_else(|error| error.into_inner());
        audio_core::refresh_event_queue(context, &mut guard)?;
        if let Some(event) = guard.pending.pop_front() {
            unsafe {
                *out = audio_core::abi_event(context, event);
            }
            return Ok(());
        }
        drop(guard);

        if timeoutns == 0 || audio_core::host_monotonic_nanos() >= deadline {
            return Err(audio_core::audio_would_block(
                "destack.audio.event.read",
                "event read timed out",
            ));
        }

        let remaining = deadline.saturating_sub(audio_core::host_monotonic_nanos());
        let sleep_ns = remaining.min(audio_core::EVENT_POLL_INTERVAL_NS);
        thread::sleep(Duration::from_nanos(sleep_ns));
    }
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
    context: &BindingCallContext,
    out: *mut AudioEvent,
    handle: resource::AudioEventHandle,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    let binding =
        audio_core::resolve_event_binding(context, handle, "destack.audio.event.tryRead")?;
    let mut guard = binding.lock().unwrap_or_else(|error| error.into_inner());
    audio_core::refresh_event_queue(context, &mut guard)?;

    let event = guard.pending.pop_front().ok_or_else(|| {
        audio_core::audio_would_block(
            "destack.audio.event.tryRead",
            "no audio event is currently queued",
        )
    })?;

    unsafe {
        *out = audio_core::abi_event(context, event);
    }
    Ok(())
}
