use std::sync::Arc;

use crate::diagnostic::RuntimeResult;
use crate::platform::display::host::unix::appkit::{
    core as appkit_core, resource as display_resource,
};
use crate::platform::display::{WindowEvent, WindowEventOpenOptions};
use crate::platform::resource::{ResourceEntry, ResourceKind};
use crate::platform::{NativeArray, core as core_platform, resource};
use crate::runtime::BindingCallContext;

use super::super::codec::window_event_from_record;
use super::super::queue::{
    ensure_window_event_thread, pop_live_window_record, pop_seeded_window_record,
    take_window_overflow_error, trim_window_events, wait_duration,
};
use super::super::{WindowEventBinding, WindowEventFilterState, WindowEventState};

/// Return the next window event available to one stream.
fn next_visible_window_event(
    runtime_state: &Arc<appkit_core::AppKitRuntimeState>,
    resolved_binding: &Arc<WindowEventBinding>,
    operation: &'static str,
) -> RuntimeResult<Option<super::super::WindowEventRecord>> {
    // surface queued overflow immediately
    if take_window_overflow_error(resolved_binding) {
        return Err(appkit_core::overflow_error(operation));
    }

    // deliver seeded records before the live event log
    if let Some(record) = pop_seeded_window_record(resolved_binding) {
        return Ok(Some(record));
    }

    // deliver one live event when the frontier can advance
    Ok(pop_live_window_record(runtime_state, resolved_binding))
}

/// Return the next window event available to one stream.
fn next_window_event(
    binding: &BindingCallContext,
    runtime_state: &Arc<appkit_core::AppKitRuntimeState>,
    resolved_binding: &Arc<WindowEventBinding>,
    operation: &'static str,
) -> RuntimeResult<Option<WindowEvent>> {
    // prefer already published records before reporting emptiness
    if let Some(record) = next_visible_window_event(runtime_state, resolved_binding, operation)? {
        return Ok(Some(window_event_from_record(binding, record)));
    }

    Ok(None)
}

/// Drain up to `maxevents` window events for one stream.
fn drain_window_events(
    binding: &BindingCallContext,
    runtime_state: &Arc<appkit_core::AppKitRuntimeState>,
    resolved_binding: &Arc<WindowEventBinding>,
    maxevents: usize,
    operation: &'static str,
) -> RuntimeResult<Vec<WindowEvent>> {
    let mut events = Vec::new();

    // drain seeded and live events until the batch is full or the stream is empty
    while events.len() < maxevents {
        // consume already published records before touching the host
        if let Some(record) = next_visible_window_event(runtime_state, resolved_binding, operation)?
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
    core_platform::ensure_out(out, "out")?;
    let filter = WindowEventFilterState::from_open_options(options)?;

    // capture the current live frontier for this new subscription
    let runtime_state = appkit_core::runtime_state(binding);
    let next_live_sequence = {
        let window_events = runtime_state
            .window_events
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        window_events.next_sequence
    };

    // allocate stream state for seeded and live event delivery
    let resolved_binding = Arc::new(WindowEventBinding {
        state: std::sync::Mutex::new(WindowEventState {
            queue_capacity: appkit_core::resolved_queue_capacity(
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
        owner_thread_id: std::thread::current().id(),
    });

    // register the resource and subscriber entry
    let resource_id = binding.agent().resources.insert(
        ResourceEntry::new(ResourceKind::Window)
            .with_label(appkit_core::WINDOW_EVENT_RESOURCE_LABEL)
            .with_payload(Arc::clone(&resolved_binding)),
        Some(binding.engine()),
    );
    runtime_state
        .window_event_registry
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .push(Arc::downgrade(&resolved_binding));

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
    let resolved_binding = display_resource::resolve_window_event_binding(
        binding,
        handle,
        "destack.display.window.eventClose",
    )?;
    let identity = Arc::as_ptr(&resolved_binding) as usize;
    let runtime_state = appkit_core::runtime_state(binding);

    // remove the subscriber entry and trim any now-unreachable live records
    {
        let mut registry = runtime_state
            .window_event_registry
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        appkit_core::retain_live_without_identity(&mut registry, identity);
    }
    trim_window_events(&runtime_state);

    // remove the resource entry itself
    let removed = binding
        .agent()
        .resources
        .remove(handle.0, Some(binding.engine()))
        .is_some();

    // reject unknown handles loudly
    if !removed {
        return Err(core_platform::io_not_found(
            "destack.display.window.eventClose",
            format!("window event handle {} was not found", handle.0.0),
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
    // validate out pointer and resolve the stream binding
    core_platform::ensure_out(out, "out")?;
    let resolved_binding = display_resource::resolve_window_event_binding(
        binding,
        handle,
        "destack.display.window.eventRead",
    )?;
    ensure_window_event_thread(&resolved_binding, "destack.display.window.eventRead")?;

    let runtime_state = appkit_core::runtime_state(binding);
    let deadline = core_platform::monotonic_now_ns().saturating_add(timeoutns);
    let wait_slice_ns = appkit_core::window_event_wait_slice_ns(binding);

    // wait until one seeded or live record becomes visible
    loop {
        if let Some(event) = next_window_event(
            binding,
            &runtime_state,
            &resolved_binding,
            "destack.display.window.eventRead",
        )? {
            unsafe {
                *out = event;
            }
            return Ok(());
        }

        let now = core_platform::monotonic_now_ns();

        // stop once the timeout budget is exhausted
        if now >= deadline {
            return Err(core_platform::io_would_block(
                "destack.display.window.eventRead",
                "event read timed out",
            ));
        }

        // wait on the shared window-event signal for the next publication
        let remaining = deadline.saturating_sub(now);
        let duration = wait_duration(remaining, wait_slice_ns);
        let window_events = runtime_state
            .window_events
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let (window_events, _) = runtime_state
            .window_event_signal
            .wait_timeout(window_events, duration)
            .unwrap_or_else(|error| error.into_inner());
        drop(window_events);
    }
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
    core_platform::ensure_out(out, "out")?;
    let maxevents = core_platform::u32_to_nonzero_usize("maxevents", maxevents)?;
    let resolved_binding = display_resource::resolve_window_event_binding(
        binding,
        handle,
        "destack.display.window.eventReadBatch",
    )?;
    ensure_window_event_thread(&resolved_binding, "destack.display.window.eventReadBatch")?;

    let runtime_state = appkit_core::runtime_state(binding);
    let deadline = core_platform::monotonic_now_ns().saturating_add(timeoutns);
    let wait_slice_ns = appkit_core::window_event_wait_slice_ns(binding);

    // wait until at least one event is available, then drain a bounded batch
    loop {
        let events = drain_window_events(
            binding,
            &runtime_state,
            &resolved_binding,
            maxevents,
            "destack.display.window.eventReadBatch",
        )?;

        if !events.is_empty() {
            unsafe {
                *out = binding.store_array(events);
            }
            return Ok(());
        }

        let now = core_platform::monotonic_now_ns();

        // stop once the timeout budget is exhausted
        if now >= deadline {
            return Err(core_platform::io_would_block(
                "destack.display.window.eventReadBatch",
                "event read timed out",
            ));
        }

        // wait on the shared window-event signal for another publication
        let remaining = deadline.saturating_sub(now);
        let duration = wait_duration(remaining, wait_slice_ns);
        let window_events = runtime_state
            .window_events
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let (window_events, _) = runtime_state
            .window_event_signal
            .wait_timeout(window_events, duration)
            .unwrap_or_else(|error| error.into_inner());
        drop(window_events);
    }
}

/// Poll one window event without blocking.
pub(crate) unsafe fn window_event_try_read(
    binding: &BindingCallContext,
    out: *mut WindowEvent,
    handle: resource::WindowEventHandle,
) -> RuntimeResult<()> {
    // validate out pointer and resolve the stream binding
    core_platform::ensure_out(out, "out")?;
    let resolved_binding = display_resource::resolve_window_event_binding(
        binding,
        handle,
        "destack.display.window.eventTryRead",
    )?;
    ensure_window_event_thread(&resolved_binding, "destack.display.window.eventTryRead")?;

    // return one event when the stream already has work visible
    let runtime_state = appkit_core::runtime_state(binding);
    let Some(event) = next_window_event(
        binding,
        &runtime_state,
        &resolved_binding,
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
    // validate out pointer and batch size
    core_platform::ensure_out(out, "out")?;
    let maxevents = core_platform::u32_to_nonzero_usize("maxevents", maxevents)?;
    let resolved_binding = display_resource::resolve_window_event_binding(
        binding,
        handle,
        "destack.display.window.eventTryReadBatch",
    )?;
    ensure_window_event_thread(
        &resolved_binding,
        "destack.display.window.eventTryReadBatch",
    )?;

    // drain one bounded batch immediately
    let runtime_state = appkit_core::runtime_state(binding);
    let events = drain_window_events(
        binding,
        &runtime_state,
        &resolved_binding,
        maxevents,
        "destack.display.window.eventTryReadBatch",
    )?;

    // report would-block when the stream is currently empty
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
