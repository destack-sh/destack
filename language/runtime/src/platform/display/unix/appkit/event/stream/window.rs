use std::sync::Arc;

use crate::diagnostic::RuntimeResult;
use crate::platform;
use crate::platform::display::{WindowEvent, WindowEventOpenOptions};
use crate::platform::resource::{ResourceEntry, ResourceKind};
use crate::platform::{NativeArray, resource};
use crate::runtime::BindingCallContext;
use crate::runtime::binding::BindingAffinity;

use crate::platform::display::unix::appkit::event::codec::window_event_from_record;
use crate::platform::display::unix::appkit::event::queue::{
    pop_live_window_record, pop_seeded_window_record, take_window_overflow_error,
    trim_window_events,
};
use crate::platform::display::unix::appkit::event::{
    WindowEventFilterState, WindowEventRecord, WindowEventState, WindowEventStream,
};

/// Return the next window event available to one stream.
fn next_visible_window_event(
    runtime_state: &Arc<platform::display::unix::appkit::core::AppKitRuntimeState>,
    resolved_stream: &Arc<WindowEventStream>,
    operation: &'static str,
) -> RuntimeResult<Option<WindowEventRecord>> {
    // surface queued overflow immediately
    if take_window_overflow_error(resolved_stream) {
        return Err(platform::display::unix::appkit::core::overflow_error(
            operation,
        ));
    }

    // deliver seeded records before the live event log
    if let Some(record) = pop_seeded_window_record(resolved_stream) {
        return Ok(Some(record));
    }

    // deliver one live event when the frontier can advance
    Ok(pop_live_window_record(runtime_state, resolved_stream))
}

/// Return the next window event available to one stream.
fn next_window_event(
    binding: &BindingCallContext,
    runtime_state: &Arc<platform::display::unix::appkit::core::AppKitRuntimeState>,
    resolved_stream: &Arc<WindowEventStream>,
    operation: &'static str,
) -> RuntimeResult<Option<WindowEvent>> {
    // prefer already published records before reporting emptiness
    if let Some(record) = next_visible_window_event(runtime_state, resolved_stream, operation)? {
        return Ok(Some(window_event_from_record(binding, record)));
    }

    Ok(None)
}

/// Drain up to `maxevents` window events for one stream.
fn drain_window_events(
    binding: &BindingCallContext,
    runtime_state: &Arc<platform::display::unix::appkit::core::AppKitRuntimeState>,
    resolved_stream: &Arc<WindowEventStream>,
    maxevents: usize,
    operation: &'static str,
) -> RuntimeResult<Vec<WindowEvent>> {
    let mut events = Vec::new();

    // drain seeded and live events until the batch is full or the stream is empty
    while events.len() < maxevents {
        // consume already published records before touching the host
        if let Some(record) = next_visible_window_event(runtime_state, resolved_stream, operation)?
        {
            events.push(window_event_from_record(binding, record));
            continue;
        }

        // stop once the local stream frontier is empty
        break;
    }

    Ok(events)
}

/// Open one global window-event stream.
pub(crate) unsafe fn window_event_open(
    binding: &BindingCallContext,
    out: *mut resource::WindowEventHandle,
    options: WindowEventOpenOptions,
) -> RuntimeResult<()> {
    // validate out pointer and parse the filter payload
    platform::core::ensure_out(out, "out")?;
    let filter = WindowEventFilterState::from_open_options(options)?;

    // capture the current live frontier for this new subscription
    let runtime_state = platform::display::unix::appkit::core::runtime_state(binding);
    let next_live_sequence = {
        let window_events = runtime_state
            .window_events
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        window_events.next_sequence
    };

    // allocate stream state for seeded and live event delivery
    let resolved_stream = Arc::new(WindowEventStream {
        stream_id: runtime_state.next_window_stream_id(),
        state: std::sync::Mutex::new(WindowEventState {
            queue_capacity: platform::display::options::resolved_event_queue_capacity(
                binding,
                options.queue.queue_capacity,
            ),
            overflow_policy: options.queue.overflow_policy,
            overflow_error_pending: false,
            next_output_sequence: 1,
            dropped_count: 0,
            next_live_sequence,
            unread_live_count: 0,
            seeded: std::collections::VecDeque::new(),
        }),
        filter,
    });

    // register the resource and stream entry
    let resource_id = binding.worker().resources.insert(
        binding.world(),
        ResourceEntry::new(ResourceKind::Window)
            .with_label(platform::display::unix::appkit::core::WINDOW_EVENT_RESOURCE_LABEL)
            .with_binding_affinity(BindingAffinity::Worker)
            .with_payload(Arc::clone(&resolved_stream)),
        Some(binding.engine()),
    );
    runtime_state.register_window_stream(Arc::clone(&resolved_stream));

    // write the opened stream handle
    unsafe {
        *out = resource::WindowEventHandle(resource_id);
    }

    Ok(())
}

/// Close one global window-event stream.
pub(crate) unsafe fn window_event_close(
    binding: &BindingCallContext,
    handle: resource::WindowEventHandle,
) -> RuntimeResult<()> {
    // resolve the stream before removing it from the registry
    let resolved_stream = platform::display::unix::appkit::resource::resolve_window_event_stream(
        binding,
        handle,
        "destack.display.window.eventClose",
    )?;
    let runtime_state = platform::display::unix::appkit::core::runtime_state(binding);

    // remove the stream entry and trim any now-unreachable live records
    runtime_state.unregister_window_stream(resolved_stream.stream_id);
    trim_window_events(&runtime_state);

    // remove the resource entry itself
    let removed = binding
        .worker()
        .resources
        .remove(binding.world(), handle.0, Some(binding.engine()))
        .is_some();

    // reject unknown handles loudly
    if !removed {
        return Err(platform::core::io_not_found(
            "destack.display.window.eventClose",
            format!("window event handle {} was not found", handle.0.local_id),
        ));
    }

    Ok(())
}

/// Wait for one window event.
pub(crate) unsafe fn window_event_read(
    binding: &BindingCallContext,
    out: *mut WindowEvent,
    handle: resource::WindowEventHandle,
    timeoutns: u64,
) -> RuntimeResult<()> {
    // validate out pointer and resolve the stream payload
    platform::core::ensure_out(out, "out")?;
    let resolved_stream = platform::display::unix::appkit::resource::resolve_window_event_stream(
        binding,
        handle,
        "destack.display.window.eventRead",
    )?;

    let runtime_state = platform::display::unix::appkit::core::runtime_state(binding);
    let deadline = platform::core::monotonic_now_ns().saturating_add(timeoutns);
    // wait until one seeded or live record becomes visible
    let event = binding.wait_for_binding_result(
        "destack.display.window.eventRead",
        "event read timed out",
        deadline,
        || {
            next_window_event(
                binding,
                &runtime_state,
                &resolved_stream,
                "destack.display.window.eventRead",
            )
        },
        |duration| {
            let window_events = runtime_state
                .window_events
                .lock()
                .unwrap_or_else(|error| error.into_inner());
            let (window_events, _) = runtime_state
                .window_event_signal
                .wait_timeout(window_events, duration)
                .unwrap_or_else(|error| error.into_inner());
            drop(window_events);
        },
    )?;

    unsafe {
        *out = event;
    }

    Ok(())
}

/// Wait for one batch of window events.
pub(crate) unsafe fn window_event_read_batch(
    binding: &BindingCallContext,
    out: *mut NativeArray<WindowEvent>,
    handle: resource::WindowEventHandle,
    maxevents: u32,
    timeoutns: u64,
) -> RuntimeResult<()> {
    // validate out pointer and batch size
    platform::core::ensure_out(out, "out")?;
    let maxevents = platform::core::u32_to_nonzero_usize("maxevents", maxevents)?;
    let resolved_stream = platform::display::unix::appkit::resource::resolve_window_event_stream(
        binding,
        handle,
        "destack.display.window.eventReadBatch",
    )?;

    let runtime_state = platform::display::unix::appkit::core::runtime_state(binding);
    let deadline = platform::core::monotonic_now_ns().saturating_add(timeoutns);
    // wait until at least one event is available, then drain a bounded batch
    let events = binding.wait_for_binding_result(
        "destack.display.window.eventReadBatch",
        "event read timed out",
        deadline,
        || {
            let events = drain_window_events(
                binding,
                &runtime_state,
                &resolved_stream,
                maxevents,
                "destack.display.window.eventReadBatch",
            )?;

            if events.is_empty() {
                return Ok(None);
            }

            Ok(Some(events))
        },
        |duration| {
            let window_events = runtime_state
                .window_events
                .lock()
                .unwrap_or_else(|error| error.into_inner());
            let (window_events, _) = runtime_state
                .window_event_signal
                .wait_timeout(window_events, duration)
                .unwrap_or_else(|error| error.into_inner());
            drop(window_events);
        },
    )?;

    unsafe {
        *out = binding.store_array(events);
    }

    Ok(())
}

/// Poll one window event without blocking.
pub(crate) unsafe fn window_event_try_read(
    binding: &BindingCallContext,
    out: *mut WindowEvent,
    handle: resource::WindowEventHandle,
) -> RuntimeResult<()> {
    // validate out pointer and resolve the stream payload
    platform::core::ensure_out(out, "out")?;
    let resolved_stream = platform::display::unix::appkit::resource::resolve_window_event_stream(
        binding,
        handle,
        "destack.display.window.eventTryRead",
    )?;

    // return one event when the stream already has work visible
    let runtime_state = platform::display::unix::appkit::core::runtime_state(binding);
    let Some(event) = next_window_event(
        binding,
        &runtime_state,
        &resolved_stream,
        "destack.display.window.eventTryRead",
    )?
    else {
        return Err(platform::core::io_would_block(
            "destack.display.window.eventTryRead",
            "no window event is currently queued",
        ));
    };

    unsafe {
        *out = event;
    }
    Ok(())
}

/// Poll one batch of window events without blocking.
pub(crate) unsafe fn window_event_try_read_batch(
    binding: &BindingCallContext,
    out: *mut NativeArray<WindowEvent>,
    handle: resource::WindowEventHandle,
    maxevents: u32,
) -> RuntimeResult<()> {
    // validate out pointer and batch size
    platform::core::ensure_out(out, "out")?;
    let maxevents = platform::core::u32_to_nonzero_usize("maxevents", maxevents)?;
    let resolved_stream = platform::display::unix::appkit::resource::resolve_window_event_stream(
        binding,
        handle,
        "destack.display.window.eventTryReadBatch",
    )?;

    // drain one bounded batch immediately
    let runtime_state = platform::display::unix::appkit::core::runtime_state(binding);
    let events = drain_window_events(
        binding,
        &runtime_state,
        &resolved_stream,
        maxevents,
        "destack.display.window.eventTryReadBatch",
    )?;

    // report would-block when the stream is currently empty
    if events.is_empty() {
        return Err(platform::core::io_would_block(
            "destack.display.window.eventTryReadBatch",
            "no window event is currently queued",
        ));
    }

    unsafe {
        *out = binding.store_array(events);
    }
    Ok(())
}
