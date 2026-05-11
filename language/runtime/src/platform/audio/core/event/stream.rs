use std::sync::{Arc, Mutex};

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::abi::NativeSlice;
use crate::platform::audio::{AudioEvent, AudioEventDeliveryMode, AudioEventSubscriptionOptions};
use crate::platform::resource::{AudioEventHandle, ResourceEntry, ResourceKind};
use crate::platform::{PlatformError, core as core_platform};
use crate::runtime::{BindingCallContext, WorkerCallbackControl};

use super::super::constants::{
    AUDIO_EVENT_RESOURCE_LABEL, EVENT_SUBSCRIBE_BACKEND, EVENT_SUBSCRIBE_INTERRUPTION,
    EVENT_SUBSCRIBE_STREAM, MAX_EVENT_POLL_INTERVAL_NS, MIN_EVENT_POLL_INTERVAL_NS,
    host_monotonic_nanos, resolved_default_event_poll_interval_ns,
    resolved_default_event_queue_capacity,
};
use super::super::device::supported_backend_event_subscription_flags;
use super::super::error::{audio_would_block, resolve_event_stream, resolve_stream_host_state};
use super::super::model::{
    AudioEventStream, audio_stream_monitor_baseline, initial_audio_event_stream_state,
};
use super::super::stream::stream_state_snapshot;
use super::codec::abi_event;
use super::publish::refresh_stream_events_for_stream;
use super::queue::{
    next_audio_event, next_event_stream_id, register_event_stream, trim_event_log,
    unregister_event_stream,
};
use super::snapshot::initial_device_monitor_baseline;
use crate::platform::audio::backend as audio_backend;
use crate::platform::audio::core::runtime::{
    AudioRuntimeState, KNOWN_EVENT_SUBSCRIPTION_FLAGS_MASK, STREAM_EVENT_SUBSCRIPTION_FLAGS_MASK,
    native_only_supported_subscription_flags, runtime_state,
};

/// Normalize one event subscription options payload.
pub(crate) fn normalize_event_subscription_options(
    ctx: &BindingCallContext,
    mut options: AudioEventSubscriptionOptions,
    operation: &'static str,
) -> RuntimeResult<AudioEventSubscriptionOptions> {
    let backend = audio_backend::resolve_requested_backend(
        options.backend,
        options.backend_policy,
        operation,
    )?;
    options.backend = backend;
    let supported_flags = supported_backend_event_subscription_flags(backend);

    // reject unknown subscription flags
    let unknown_flags = options.flags.0 & !KNOWN_EVENT_SUBSCRIPTION_FLAGS_MASK;
    if unknown_flags != 0 {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "options.flags",
            format!("options.flags contains unknown bits: 0x{unknown_flags:08x}"),
        ))
        .boxed());
    }

    // reject subscription flags that the selected backend does not advertise
    let unsupported_flags = options.flags.0 & !supported_flags.0;
    if unsupported_flags != 0 {
        return Err(RuntimeError::from(PlatformError::not_supported(format!(
            "{operation} unsupported subscription flags for backend {backend:?}: 0x{unsupported_flags:08x}",
        )))
        .boxed());
    }

    // require explicit stream target when stream related flags are requested
    let stream_flags = options.flags.0 & STREAM_EVENT_SUBSCRIPTION_FLAGS_MASK;
    if stream_flags != 0 && options.stream.is_none() {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "options.stream",
            "stream subscription flags require options.stream",
        ))
        .boxed());
    }

    // enforce native-only delivery contracts by backend capability
    if options.delivery_mode == AudioEventDeliveryMode::NativeOnly {
        let requested_flags = if options.flags.0 == 0 {
            KNOWN_EVENT_SUBSCRIPTION_FLAGS_MASK
        } else {
            options.flags.0
        };
        let native_supported_flags = native_only_supported_subscription_flags(backend);
        let unsupported_native_flags = requested_flags & !native_supported_flags;
        if unsupported_native_flags != 0 {
            return Err(RuntimeError::from(PlatformError::not_supported(format!(
                "{operation} native-only delivery unsupported subscription flags for backend {backend:?}: 0x{unsupported_native_flags:08x}",
            )))
            .boxed());
        }
    }

    // ensure the stream target exists and matches the selected backend
    if let Some(stream_handle) = options.stream {
        let stream = resolve_stream_host_state(ctx, stream_handle, operation)?;
        if stream.device.backend != backend {
            return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "options.backend",
                format!(
                    "options.backend does not match stream backend: requested {:?}, stream {:?}",
                    backend, stream.device.backend,
                ),
            ))
            .boxed());
        }
    }

    // normalize queue and polling values
    if options.queue_capacity == 0 {
        options.queue_capacity = resolved_default_event_queue_capacity(ctx);
    }

    if options.poll_interval_ns == 0 {
        options.poll_interval_ns = resolved_default_event_poll_interval_ns(ctx);
    }

    options.poll_interval_ns = options
        .poll_interval_ns
        .clamp(MIN_EVENT_POLL_INTERVAL_NS, MAX_EVENT_POLL_INTERVAL_NS);

    Ok(options)
}

/// Ensure monitor and stream baselines exist before the first event read.
fn ensure_event_stream_baselines(
    ctx: &BindingCallContext,
    runtime_state: &Arc<AudioRuntimeState>,
    options: AudioEventSubscriptionOptions,
) -> RuntimeResult<()> {
    let now = host_monotonic_nanos();

    // capture the current device baseline without publishing initial events
    {
        let mut monitor_states = runtime_state
            .device_monitor_baselines
            .lock()
            .unwrap_or_else(|error| error.into_inner());

        if let std::collections::hash_map::Entry::Vacant(entry) =
            monitor_states.entry(options.backend)
        {
            let monitor_state = initial_device_monitor_baseline(options.backend, now)?;
            entry.insert(monitor_state);
        }
    }

    // capture the current stream baseline without publishing initial events
    if let Some(stream_handle) = options.stream {
        let stream = resolve_stream_host_state(ctx, stream_handle, "destack.audio.event.open")?;
        let stream_state = stream_state_snapshot(&stream);
        let mut monitor_states = runtime_state
            .stream_monitor_baselines
            .lock()
            .unwrap_or_else(|error| error.into_inner());

        monitor_states.entry(stream_handle.0).or_insert_with(|| {
            audio_stream_monitor_baseline(
                stream_state.state,
                stream.device.id.clone(),
                stream_state.xrun_count,
                now,
            )
        });
    }

    Ok(())
}

/// Return whether one subscription still needs synthetic stream polling.
fn needs_stream_poll_callback(stream: &AudioEventStream) -> bool {
    let tracks_stream_events = if stream.options.flags.0 == 0 {
        true
    } else {
        (stream.options.flags.0
            & (EVENT_SUBSCRIBE_STREAM.0
                | EVENT_SUBSCRIBE_INTERRUPTION.0
                | EVENT_SUBSCRIBE_BACKEND.0))
            != 0
    };

    stream.options.stream.is_some()
        && tracks_stream_events
        && stream.options.delivery_mode != AudioEventDeliveryMode::NativeOnly
}

/// Register one synthetic stream poll callback on the owning runtime thread.
fn register_stream_poll_callback(
    ctx: &BindingCallContext,
    runtime_state: &Arc<AudioRuntimeState>,
    stream: &Arc<AudioEventStream>,
) -> RuntimeResult<()> {
    if !needs_stream_poll_callback(stream) {
        return Ok(());
    }

    let poll_interval_ns = stream.options.poll_interval_ns.max(1);
    let runtime_state = Arc::clone(runtime_state);
    let live_stream = Arc::clone(stream);
    let weak_stream = Arc::downgrade(stream);
    let callback =
        ctx.schedule_worker_callback(poll_interval_ns, Some(poll_interval_ns), move |binding| {
            let Some(stream) = weak_stream.upgrade() else {
                return Ok(WorkerCallbackControl::Cancel);
            };

            let now = host_monotonic_nanos();
            refresh_stream_events_for_stream(binding, &runtime_state, &stream, now)?;

            Ok(WorkerCallbackControl::Keep)
        })?;

    // store the callback handle after successful registration
    let mut state = live_stream
        .state
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    state.poll_callback = Some(callback);

    Ok(())
}

/// Return one pending live event when available.
fn next_event_for_stream(
    ctx: &BindingCallContext,
    runtime_state: &AudioRuntimeState,
    stream: &Arc<AudioEventStream>,
    operation: &'static str,
) -> RuntimeResult<Option<AudioEvent>> {
    if let Some(event) = next_audio_event(runtime_state, stream, operation)? {
        return Ok(Some(abi_event(ctx, event)));
    }

    Ok(None)
}

/// Drain up to `maxevents` events for one stream.
fn drain_events_for_stream(
    ctx: &BindingCallContext,
    runtime_state: &AudioRuntimeState,
    stream: &Arc<AudioEventStream>,
    maxevents: usize,
    operation: &'static str,
) -> RuntimeResult<Vec<AudioEvent>> {
    let mut events = Vec::new();

    while events.len() < maxevents {
        let Some(event) = next_audio_event(runtime_state, stream, operation)? else {
            break;
        };
        events.push(abi_event(ctx, event));
    }

    Ok(events)
}

/// Open one runtime-owned audio event stream.
pub(crate) unsafe fn open_event_stream(
    ctx: &BindingCallContext,
    out: *mut AudioEventHandle,
    options: AudioEventSubscriptionOptions,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    let options = normalize_event_subscription_options(ctx, options, "destack.audio.event.open")?;
    let runtime_state = runtime_state(ctx);
    ensure_event_stream_baselines(ctx, &runtime_state, options)?;

    let next_live_sequence = {
        let event_log = runtime_state
            .event_log
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        event_log.next_sequence
    };
    let queue_capacity = usize::try_from(options.queue_capacity)
        .unwrap_or(usize::MAX)
        .max(1);
    let stream = Arc::new(AudioEventStream {
        stream_id: next_event_stream_id(&runtime_state),
        options,
        state: Mutex::new(initial_audio_event_stream_state(
            queue_capacity,
            options.overflow_policy,
            next_live_sequence,
        )),
    });

    let handle = ctx.worker().resources.insert(
        ctx.world(),
        ResourceEntry::new(ResourceKind::AudioEvent)
            .with_label(AUDIO_EVENT_RESOURCE_LABEL)
            .with_payload(Arc::clone(&stream)),
        Some(ctx.engine()),
    );
    register_event_stream(&runtime_state, Arc::clone(&stream));

    // register synthetic stream polling after the stream becomes visible
    if let Err(error) = register_stream_poll_callback(ctx, &runtime_state, &stream) {
        unregister_event_stream(&runtime_state, stream.stream_id);
        let _ = ctx
            .worker()
            .resources
            .remove(ctx.world(), handle, Some(ctx.engine()));
        return Err(error);
    }

    // refresh backend monitor demand after the new stream is visible
    let monitor_service = ctx.worker().platform_state.audio.monitor_service();
    if let Err(error) = monitor_service.refresh_runtime(&runtime_state, options.backend) {
        let poll_callback = {
            let mut state = stream
                .state
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            state.poll_callback.take()
        };
        if let Some(poll_callback) = poll_callback {
            let _ = ctx.cancel_worker_callback(poll_callback);
        }

        unregister_event_stream(&runtime_state, stream.stream_id);
        let _ = ctx
            .worker()
            .resources
            .remove(ctx.world(), handle, Some(ctx.engine()));
        return Err(error);
    }

    unsafe {
        *out = AudioEventHandle(handle);
    }

    Ok(())
}

/// Close one runtime-owned audio event stream.
pub(crate) unsafe fn close_event_stream(
    ctx: &BindingCallContext,
    handle: AudioEventHandle,
) -> RuntimeResult<()> {
    let stream = resolve_event_stream(ctx, handle, "destack.audio.event.close")?;
    let runtime_state = runtime_state(ctx);
    let backend = stream.options.backend;

    // cancel one synthetic stream poll callback before dropping the stream
    let poll_callback = {
        let mut state = stream
            .state
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        state.poll_callback.take()
    };
    if let Some(poll_callback) = poll_callback {
        ctx.cancel_worker_callback(poll_callback)?;
    }

    unregister_event_stream(&runtime_state, stream.stream_id);
    trim_event_log(&runtime_state);

    let removed = ctx
        .worker()
        .resources
        .remove(ctx.world(), handle.0, Some(ctx.engine()))
        .is_some();
    if !removed {
        return Err(core_platform::io_not_found(
            "destack.audio.event.close",
            format!("unknown audio event handle {}", handle.0.0),
        ));
    }

    let monitor_service = ctx.worker().platform_state.audio.monitor_service();
    let _ = monitor_service.refresh_runtime(&runtime_state, backend);
    Ok(())
}

/// Wait for one audio event.
pub(crate) unsafe fn read_event(
    ctx: &BindingCallContext,
    out: *mut AudioEvent,
    handle: AudioEventHandle,
    timeoutns: u64,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    let stream = resolve_event_stream(ctx, handle, "destack.audio.event.read")?;
    let runtime_state = runtime_state(ctx);
    let deadline = host_monotonic_nanos().saturating_add(timeoutns);

    let event = ctx.wait_for_binding_result(
        "destack.audio.event.read",
        "event read timed out",
        deadline,
        || next_event_for_stream(ctx, &runtime_state, &stream, "destack.audio.event.read"),
        |duration| {
            let event_log = runtime_state
                .event_log
                .lock()
                .unwrap_or_else(|error| error.into_inner());
            let wait_result = runtime_state.event_signal.wait_timeout(event_log, duration);
            let (event_log, _) = match wait_result {
                Ok(value) => value,
                Err(error) => error.into_inner(),
            };
            drop(event_log);
        },
    )?;

    unsafe {
        *out = event;
    }

    Ok(())
}

/// Wait for one batch of audio events.
pub(crate) unsafe fn read_event_batch(
    ctx: &BindingCallContext,
    out: *mut NativeSlice<AudioEvent>,
    handle: AudioEventHandle,
    maxevents: u32,
    timeoutns: u64,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;
    let maxevents = core_platform::u32_to_nonzero_usize("maxevents", maxevents)?;

    let stream = resolve_event_stream(ctx, handle, "destack.audio.event.readBatch")?;
    let runtime_state = runtime_state(ctx);
    let deadline = host_monotonic_nanos().saturating_add(timeoutns);

    let events = ctx.wait_for_binding_result(
        "destack.audio.event.readBatch",
        "event read timed out",
        deadline,
        || {
            let events = drain_events_for_stream(
                ctx,
                &runtime_state,
                &stream,
                maxevents,
                "destack.audio.event.readBatch",
            )?;
            if events.is_empty() {
                return Ok(None);
            }
            Ok(Some(events))
        },
        |duration| {
            let event_log = runtime_state
                .event_log
                .lock()
                .unwrap_or_else(|error| error.into_inner());
            let wait_result = runtime_state.event_signal.wait_timeout(event_log, duration);
            let (event_log, _) = match wait_result {
                Ok(value) => value,
                Err(error) => error.into_inner(),
            };
            drop(event_log);
        },
    )?;

    unsafe {
        *out = ctx.store_slice(events);
    }

    Ok(())
}

/// Poll one audio event without blocking.
pub(crate) unsafe fn try_read_event(
    ctx: &BindingCallContext,
    out: *mut AudioEvent,
    handle: AudioEventHandle,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    let stream = resolve_event_stream(ctx, handle, "destack.audio.event.tryRead")?;
    let runtime_state = runtime_state(ctx);
    let event = next_event_for_stream(ctx, &runtime_state, &stream, "destack.audio.event.tryRead")?
        .ok_or_else(|| {
            audio_would_block(
                "destack.audio.event.tryRead",
                "no audio event is currently queued",
            )
        })?;

    unsafe {
        *out = event;
    }

    Ok(())
}

/// Poll one batch of audio events without blocking.
pub(crate) unsafe fn try_read_event_batch(
    ctx: &BindingCallContext,
    out: *mut NativeSlice<AudioEvent>,
    handle: AudioEventHandle,
    maxevents: u32,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;
    let maxevents = core_platform::u32_to_nonzero_usize("maxevents", maxevents)?;

    let stream = resolve_event_stream(ctx, handle, "destack.audio.event.tryReadBatch")?;
    let runtime_state = runtime_state(ctx);
    let events = drain_events_for_stream(
        ctx,
        &runtime_state,
        &stream,
        maxevents,
        "destack.audio.event.tryReadBatch",
    )?;

    if events.is_empty() {
        return Err(audio_would_block(
            "destack.audio.event.tryReadBatch",
            "no audio event is currently queued",
        ));
    }

    unsafe {
        *out = ctx.store_slice(events);
    }

    Ok(())
}
