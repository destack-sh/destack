use std::sync::Arc;

use crate::diagnostic::RuntimeResult;
use crate::platform::display::windows::win32::event::codec::window_event_from_record;
use crate::platform::display::windows::win32::event::queue::{
    pop_live_window_record, pop_seeded_window_record, take_window_overflow_error,
    trim_window_events,
};
use crate::platform::display::windows::win32::event::{
    WindowEventFilterState, WindowEventState, WindowEventStream,
};
use crate::platform::display::windows::win32::{core as win32_core, resource as display_resource};
use crate::platform::display::{WindowEvent, WindowEventOpenOptions, options as display_options};
use crate::platform::resource::{ResourceEntry, ResourceKind};
use crate::platform::{NativeArray, core as core_platform, resource};
use crate::runtime::BindingCallContext;
use crate::runtime::binding::BindingAffinity;
/// Return the next window event available to one stream.
fn next_window_event(
    binding: &BindingCallContext,
    runtime_state: &Arc<win32_core::Win32RuntimeState>,
    resolved_stream: &Arc<WindowEventStream>,
    operation: &'static str,
) -> RuntimeResult<Option<WindowEvent>> {
    if take_window_overflow_error(resolved_stream) {
        return Err(win32_core::overflow_error(operation));
    }

    if let Some(record) = pop_seeded_window_record(resolved_stream) {
        return Ok(Some(window_event_from_record(record, binding)));
    }

    if let Some(record) = pop_live_window_record(runtime_state, resolved_stream) {
        return Ok(Some(window_event_from_record(record, binding)));
    }

    Ok(None)
}

/// Drain up to `maxevents` window events for one stream.
fn drain_window_events(
    binding: &BindingCallContext,
    runtime_state: &Arc<win32_core::Win32RuntimeState>,
    resolved_stream: &Arc<WindowEventStream>,
    maxevents: usize,
    operation: &'static str,
) -> RuntimeResult<Vec<WindowEvent>> {
    let mut events = Vec::new();

    while events.len() < maxevents {
        if take_window_overflow_error(resolved_stream) {
            return Err(win32_core::overflow_error(operation));
        }

        if let Some(record) = pop_seeded_window_record(resolved_stream) {
            events.push(window_event_from_record(record, binding));
            continue;
        }

        let Some(record) = pop_live_window_record(runtime_state, resolved_stream) else {
            break;
        };
        events.push(window_event_from_record(record, binding));
    }

    Ok(events)
}

/// Open one global window-event stream.
pub(crate) unsafe fn window_event_open(
    binding: &BindingCallContext,
    out: *mut resource::WindowEventHandle,
    options: WindowEventOpenOptions,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;
    let filter = WindowEventFilterState::from_open_options(options)?;
    let runtime_state = win32_core::runtime_state(binding);

    let next_live_sequence = {
        let window_events = runtime_state
            .window_events
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        window_events.next_sequence
    };

    let resolved_stream = Arc::new(WindowEventStream {
        stream_id: runtime_state.next_window_stream_id(),
        state: std::sync::Mutex::new(WindowEventState {
            queue_capacity: display_options::resolved_event_queue_capacity(
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

    let resource_id = binding.worker().resources.insert(
        binding.world(),
        ResourceEntry::new(ResourceKind::Window)
            .with_label(win32_core::WINDOW_EVENT_RESOURCE_LABEL)
            .with_binding_affinity(BindingAffinity::Worker)
            .with_payload(Arc::clone(&resolved_stream)),
        Some(binding.engine()),
    );
    runtime_state.register_window_stream(Arc::clone(&resolved_stream));

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
    let resolved_stream = display_resource::resolve_window_event_stream(
        binding,
        handle,
        "destack.display.window.eventClose",
    )?;
    let runtime_state = win32_core::runtime_state(binding);

    runtime_state.unregister_window_stream(resolved_stream.stream_id);
    trim_window_events(&runtime_state);

    let removed = binding
        .worker()
        .resources
        .remove(binding.world(), handle.0, Some(binding.engine()))
        .is_some();

    if !removed {
        return Err(core_platform::io_not_found(
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
    core_platform::ensure_out(out, "out")?;
    let resolved_stream = display_resource::resolve_window_event_stream(
        binding,
        handle,
        "destack.display.window.eventRead",
    )?;
    let runtime_state = win32_core::runtime_state(binding);
    let deadline = core_platform::monotonic_now_ns().saturating_add(timeoutns);

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
    core_platform::ensure_out(out, "out")?;
    let maxevents = win32_core::validate_max_events(maxevents, "maxevents")?;
    let resolved_stream = display_resource::resolve_window_event_stream(
        binding,
        handle,
        "destack.display.window.eventReadBatch",
    )?;
    let runtime_state = win32_core::runtime_state(binding);
    let deadline = core_platform::monotonic_now_ns().saturating_add(timeoutns);

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
    core_platform::ensure_out(out, "out")?;
    let resolved_stream = display_resource::resolve_window_event_stream(
        binding,
        handle,
        "destack.display.window.eventTryRead",
    )?;
    let runtime_state = win32_core::runtime_state(binding);

    let Some(event) = next_window_event(
        binding,
        &runtime_state,
        &resolved_stream,
        "destack.display.window.eventTryRead",
    )?
    else {
        return Err(core_platform::io_would_block(
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
    core_platform::ensure_out(out, "out")?;
    let maxevents = win32_core::validate_max_events(maxevents, "maxevents")?;
    let resolved_stream = display_resource::resolve_window_event_stream(
        binding,
        handle,
        "destack.display.window.eventTryReadBatch",
    )?;
    let runtime_state = win32_core::runtime_state(binding);

    let events = drain_window_events(
        binding,
        &runtime_state,
        &resolved_stream,
        maxevents,
        "destack.display.window.eventTryReadBatch",
    )?;

    if events.is_empty() {
        return Err(core_platform::io_would_block(
            "destack.display.window.eventTryReadBatch",
            "no window event is currently queued",
        ));
    }

    unsafe {
        *out = binding.store_array(events);
    }

    Ok(())
}
