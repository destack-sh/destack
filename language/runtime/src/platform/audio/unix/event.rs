use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::audio::{AudioEvent, AudioEventSubscriptionOptions, core as audio_core};
use crate::platform::resource::{ResourceEntry, ResourceKind};
use crate::platform::{PlatformError, core as core_platform, resource};
use crate::runtime::{BindingCallContext, NativeSlice};

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
    handle: resource::AudioEventHandle,
) -> RuntimeResult<()> {
    let resolved_binding =
        audio_core::resolve_event_binding(binding, handle, "destack.audio.event.close")?;
    let backend = {
        let resolved_binding = resolved_binding
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        resolved_binding.options.backend
    };
    audio_core::unregister_event_binding(binding, &resolved_binding);

    let removed =
        binding
            .agent()
            .resources
            .remove(binding.world(), handle.0, Some(binding.engine()));
    if removed.is_none() {
        return Err(core_platform::io_not_found(
            "destack.audio.event.close",
            format!("unknown audio event handle {}", handle.0.0),
        ));
    }

    let _ = audio_core::refresh_backend_device_monitor(binding, backend);

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
    binding: &BindingCallContext,
    out: *mut resource::AudioEventHandle,
    options: AudioEventSubscriptionOptions,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    let options = audio_core::normalize_event_subscription_options(
        binding,
        options,
        "destack.audio.event.open",
    )?;
    let resolved_binding = audio_core::build_event_binding(binding, options)?;
    let payload = Arc::new(Mutex::new(resolved_binding));

    let handle = binding.agent().resources.insert(
        binding.world(),
        ResourceEntry::new(ResourceKind::AudioEvent)
            .with_label(audio_core::AUDIO_EVENT_RESOURCE_LABEL)
            .with_payload(payload.clone()),
        Some(binding.engine()),
    );
    audio_core::register_event_binding(binding, &payload);
    if let Err(error) = audio_core::refresh_backend_device_monitor(binding, options.backend) {
        audio_core::unregister_event_binding(binding, &payload);
        let _ = binding
            .agent()
            .resources
            .remove(binding.world(), handle, Some(binding.engine()));
        return Err(error);
    }

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
    binding: &BindingCallContext,
    out: *mut AudioEvent,
    handle: resource::AudioEventHandle,
    timeoutns: u64,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    let resolved_binding =
        audio_core::resolve_event_binding(binding, handle, "destack.audio.event.read")?;
    let deadline = audio_core::host_monotonic_nanos().saturating_add(timeoutns);

    loop {
        let mut guard = resolved_binding
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let poll_interval_ns = guard.options.poll_interval_ns.max(1);
        audio_core::refresh_event_queue_for_delivery_mode(binding, &mut guard)?;
        audio_core::take_event_overflow_error(&mut guard, "destack.audio.event.read")?;
        if let Some(event) = guard.pending.pop_front() {
            unsafe {
                *out = audio_core::abi_event(binding, event);
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
        let sleep_ns = remaining.min(poll_interval_ns);
        thread::sleep(Duration::from_nanos(sleep_ns));
    }
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
    handle: resource::AudioEventHandle,
    maxevents: u32,
    timeoutns: u64,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    if maxevents == 0 {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "maxevents",
            "maxevents must be greater than zero",
        ))
        .boxed());
    }

    let resolved_binding =
        audio_core::resolve_event_binding(binding, handle, "destack.audio.event.readBatch")?;
    let deadline = audio_core::host_monotonic_nanos().saturating_add(timeoutns);

    loop {
        let mut guard = resolved_binding
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let poll_interval_ns = guard.options.poll_interval_ns.max(1);
        audio_core::refresh_event_queue_for_delivery_mode(binding, &mut guard)?;
        audio_core::take_event_overflow_error(&mut guard, "destack.audio.event.readBatch")?;
        if !guard.pending.is_empty() {
            let take = (maxevents as usize).min(guard.pending.len());
            let mut events = Vec::with_capacity(take);
            for _ in 0..take {
                if let Some(event) = guard.pending.pop_front() {
                    events.push(audio_core::abi_event(binding, event));
                }
            }

            unsafe {
                *out = binding.store_slice(events);
            }
            return Ok(());
        }
        drop(guard);

        if timeoutns == 0 || audio_core::host_monotonic_nanos() >= deadline {
            return Err(audio_core::audio_would_block(
                "destack.audio.event.readBatch",
                "event read timed out",
            ));
        }

        let remaining = deadline.saturating_sub(audio_core::host_monotonic_nanos());
        let sleep_ns = remaining.min(poll_interval_ns);
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
    binding: &BindingCallContext,
    out: *mut AudioEvent,
    handle: resource::AudioEventHandle,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    let resolved_binding =
        audio_core::resolve_event_binding(binding, handle, "destack.audio.event.tryRead")?;
    let mut guard = resolved_binding
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    audio_core::refresh_event_queue_for_delivery_mode(binding, &mut guard)?;
    audio_core::take_event_overflow_error(&mut guard, "destack.audio.event.tryRead")?;

    let event = guard.pending.pop_front().ok_or_else(|| {
        audio_core::audio_would_block(
            "destack.audio.event.tryRead",
            "no audio event is currently queued",
        )
    })?;

    unsafe {
        *out = audio_core::abi_event(binding, event);
    }
    Ok(())
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
    handle: resource::AudioEventHandle,
    maxevents: u32,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    if maxevents == 0 {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "maxevents",
            "maxevents must be greater than zero",
        ))
        .boxed());
    }

    let resolved_binding =
        audio_core::resolve_event_binding(binding, handle, "destack.audio.event.tryReadBatch")?;
    let mut guard = resolved_binding
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    audio_core::refresh_event_queue_for_delivery_mode(binding, &mut guard)?;
    audio_core::take_event_overflow_error(&mut guard, "destack.audio.event.tryReadBatch")?;

    if guard.pending.is_empty() {
        return Err(audio_core::audio_would_block(
            "destack.audio.event.tryReadBatch",
            "no audio event is currently queued",
        ));
    }

    let take = (maxevents as usize).min(guard.pending.len());
    let mut events = Vec::with_capacity(take);
    for _ in 0..take {
        if let Some(event) = guard.pending.pop_front() {
            events.push(audio_core::abi_event(binding, event));
        }
    }

    unsafe {
        *out = binding.store_slice(events);
    }

    Ok(())
}
