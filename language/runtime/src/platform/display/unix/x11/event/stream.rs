use std::collections::VecDeque;
use std::sync::{Arc, Condvar, Mutex};

use crate::diagnostic::RuntimeResult;
use crate::platform::display::{
    DisplayMonitorEvent, DisplayMonitorEventOpenOptions, WindowEvent, WindowEventOpenOptions,
};
use crate::platform::resource::{ResourceEntry, ResourceKind};
use crate::platform::{NativeArray, core as core_platform, resource};
use crate::runtime::BindingCallContext;

use super::super::super::{core, resource as display_resource, window};
use super::{
    MonitorEventBinding, MonitorEventFilterState, MonitorEventState, WindowEventBinding,
    WindowEventFilterState, WindowEventState, display_event_from_record,
    ensure_window_event_thread, publish_monitor_topology_deltas, seed_monitor_event_stream,
    wait_duration, window_event_from_record,
};

/// Open one global monitor-event stream.
pub(in crate::platform::display::host::unix) unsafe fn monitor_event_open(
    binding: &BindingCallContext,
    out: *mut resource::DisplayEventHandle,
    options: DisplayMonitorEventOpenOptions,
) -> RuntimeResult<()> {
    // validate out pointer and parse open filter
    core_platform::ensure_out(out, "out")?;
    let filter = MonitorEventFilterState::from_open_options(options)?;

    // allocate stream resolved_binding with configured queue state
    let resolved_binding = Arc::new(MonitorEventBinding {
        state: Mutex::new(MonitorEventState {
            queue_capacity: core::resolved_queue_capacity(binding, options.queue.queue_capacity),
            overflow_policy: options.queue.overflow_policy,
            overflow_error_pending: false,
            next_sequence: 1,
            dropped_count: 0,
            pending: VecDeque::new(),
        }),
        filter,
        signal: Condvar::new(),
    });

    // seed stream with current monitor snapshot events and cache topology snapshot
    let snapshots = seed_monitor_event_stream(binding, &resolved_binding)?;
    let runtime_state = core::runtime_state(binding);
    {
        let mut topology_snapshot = runtime_state
            .monitor_topology_snapshot
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        *topology_snapshot = Some(snapshots);
    }

    // register resource and subscriber entry
    let resource_id = binding.agent().resources.insert(
        binding.world(),
        ResourceEntry::new(ResourceKind::Display)
            .with_label(core::DISPLAY_EVENT_RESOURCE_LABEL)
            .with_payload(Arc::clone(&resolved_binding)),
        Some(binding.engine()),
    );
    runtime_state
        .monitor_event_registry
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .push(Arc::downgrade(&resolved_binding));

    // write stream handle
    unsafe {
        *out = resource::DisplayEventHandle(resource_id);
    }

    Ok(())
}

/// Close one global monitor-event stream.
pub(in crate::platform::display::host::unix) unsafe fn monitor_event_close(
    binding: &BindingCallContext,
    handle: resource::DisplayEventHandle,
) -> RuntimeResult<()> {
    // resolve stream resolved_binding and remove it from subscriber registry
    let resolved_binding = display_resource::resolve_monitor_event_binding(
        binding,
        handle,
        "destack.display.monitor.eventClose",
    )?;
    let identity = Arc::as_ptr(&resolved_binding) as usize;
    let runtime_state = core::runtime_state(binding);
    {
        let mut registry = runtime_state
            .monitor_event_registry
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        core::retain_live_without_identity(&mut registry, identity);
    }

    // remove stream resource entry
    let removed = binding
        .agent()
        .resources
        .remove(binding.world(), handle.0, Some(binding.engine()))
        .is_some();
    // evaluate this condition
    if !removed {
        return Err(core_platform::io_not_found(
            "destack.display.monitor.eventClose",
            format!("display event handle {} was not found", handle.0.0),
        ));
    }

    Ok(())
}

/// Wait for one monitor event.
pub(in crate::platform::display::host::unix) unsafe fn monitor_event_read(
    binding: &BindingCallContext,
    out: *mut DisplayMonitorEvent,
    handle: resource::DisplayEventHandle,
    timeoutns: u64,
) -> RuntimeResult<()> {
    // validate out pointer and resolve stream resolved_binding
    core_platform::ensure_out(out, "out")?;
    let resolved_binding = display_resource::resolve_monitor_event_binding(
        binding,
        handle,
        "destack.display.monitor.eventRead",
    )?;

    // resolve one bounded monitor wait-slice interval
    let wait_slice_ns = core::DEFAULT_EVENT_WAIT_SLICE_NS;

    // wait until one event is available or timeout expires
    let deadline = core_platform::monotonic_now_ns().saturating_add(timeoutns);
    // loop until one branch exits
    loop {
        // refresh monitor topology state before reading one queue snapshot
        publish_monitor_topology_deltas(binding)?;

        let mut state = resolved_binding
            .state
            .lock()
            .unwrap_or_else(|error| error.into_inner());

        // surface overflow policy as explicit runtime error
        if state.overflow_error_pending {
            state.overflow_error_pending = false;
            return Err(core::overflow_error("destack.display.monitor.eventRead"));
        }

        // return next pending event when available
        if let Some(record) = state.pending.pop_front() {
            unsafe {
                *out = display_event_from_record(binding, record);
            }
            return Ok(());
        }

        // abort on timeout
        let now = core_platform::monotonic_now_ns();
        // evaluate this condition
        if now >= deadline {
            return Err(core_platform::io_would_block(
                "destack.display.monitor.eventRead",
                "event read timed out",
            ));
        }

        // wait for the remaining timeout slice
        let remaining = deadline.saturating_sub(now);
        let wait_duration = wait_duration(remaining, wait_slice_ns);
        let (state, _) = resolved_binding
            .signal
            .wait_timeout(state, wait_duration)
            .unwrap_or_else(|error| error.into_inner());
        drop(state);
    }
}

/// Wait for one batch of monitor events.
pub(in crate::platform::display::host::unix) unsafe fn monitor_event_read_batch(
    binding: &BindingCallContext,
    out: *mut NativeArray<DisplayMonitorEvent>,
    handle: resource::DisplayEventHandle,
    maxevents: u32,
    timeoutns: u64,
) -> RuntimeResult<()> {
    // validate out pointer and batch size
    core_platform::ensure_out(out, "out")?;
    let maxevents = core_platform::u32_to_nonzero_usize("maxevents", maxevents)?;

    // resolve stream resolved_binding
    let resolved_binding = display_resource::resolve_monitor_event_binding(
        binding,
        handle,
        "destack.display.monitor.eventReadBatch",
    )?;

    // resolve one bounded monitor wait-slice interval
    let wait_slice_ns = core::DEFAULT_EVENT_WAIT_SLICE_NS;

    // wait until one or more events are available or timeout expires
    let deadline = core_platform::monotonic_now_ns().saturating_add(timeoutns);
    // loop until one branch exits
    loop {
        // refresh monitor topology state before reading one queue snapshot
        publish_monitor_topology_deltas(binding)?;

        let mut state = resolved_binding
            .state
            .lock()
            .unwrap_or_else(|error| error.into_inner());

        // surface overflow policy as explicit runtime error
        if state.overflow_error_pending {
            state.overflow_error_pending = false;
            return Err(core::overflow_error(
                "destack.display.monitor.eventReadBatch",
            ));
        }

        // drain up to maxevents records when queue is non-empty
        if !state.pending.is_empty() {
            let take = maxevents.min(state.pending.len());
            let mut events = Vec::with_capacity(take);
            // iterate this sequence
            for _ in 0..take {
                // evaluate this condition
                if let Some(record) = state.pending.pop_front() {
                    events.push(display_event_from_record(binding, record));
                }
            }

            unsafe {
                *out = binding.store_array(events);
            }
            return Ok(());
        }

        // abort on timeout
        let now = core_platform::monotonic_now_ns();
        // evaluate this condition
        if now >= deadline {
            return Err(core_platform::io_would_block(
                "destack.display.monitor.eventReadBatch",
                "event read timed out",
            ));
        }

        // wait for the remaining timeout slice
        let remaining = deadline.saturating_sub(now);
        let wait_duration = wait_duration(remaining, wait_slice_ns);
        let (state, _) = resolved_binding
            .signal
            .wait_timeout(state, wait_duration)
            .unwrap_or_else(|error| error.into_inner());
        drop(state);
    }
}

/// Poll one monitor event without blocking.
pub(in crate::platform::display::host::unix) unsafe fn monitor_event_try_read(
    binding: &BindingCallContext,
    out: *mut DisplayMonitorEvent,
    handle: resource::DisplayEventHandle,
) -> RuntimeResult<()> {
    // validate out pointer and resolve stream resolved_binding
    core_platform::ensure_out(out, "out")?;
    publish_monitor_topology_deltas(binding)?;
    let resolved_binding = display_resource::resolve_monitor_event_binding(
        binding,
        handle,
        "destack.display.monitor.eventTryRead",
    )?;
    let mut state = resolved_binding
        .state
        .lock()
        .unwrap_or_else(|error| error.into_inner());

    // surface overflow policy as explicit runtime error
    if state.overflow_error_pending {
        state.overflow_error_pending = false;
        return Err(core::overflow_error("destack.display.monitor.eventTryRead"));
    }

    // pop one event without blocking
    let Some(record) = state.pending.pop_front() else {
        return Err(core_platform::io_would_block(
            "destack.display.monitor.eventTryRead",
            "no monitor event is currently queued",
        ));
    };

    // write decoded event payload
    unsafe {
        *out = display_event_from_record(binding, record);
    }

    Ok(())
}

/// Poll one batch of monitor events without blocking.
pub(in crate::platform::display::host::unix) unsafe fn monitor_event_try_read_batch(
    binding: &BindingCallContext,
    out: *mut NativeArray<DisplayMonitorEvent>,
    handle: resource::DisplayEventHandle,
    maxevents: u32,
) -> RuntimeResult<()> {
    // validate out pointer and batch size
    core_platform::ensure_out(out, "out")?;
    let maxevents = core_platform::u32_to_nonzero_usize("maxevents", maxevents)?;
    publish_monitor_topology_deltas(binding)?;

    // resolve stream resolved_binding and pop pending batch without blocking
    let resolved_binding = display_resource::resolve_monitor_event_binding(
        binding,
        handle,
        "destack.display.monitor.eventTryReadBatch",
    )?;
    let mut state = resolved_binding
        .state
        .lock()
        .unwrap_or_else(|error| error.into_inner());

    // surface overflow policy as explicit runtime error
    if state.overflow_error_pending {
        state.overflow_error_pending = false;
        return Err(core::overflow_error(
            "destack.display.monitor.eventTryReadBatch",
        ));
    }

    // reject empty queues with would-block semantics
    if state.pending.is_empty() {
        return Err(core_platform::io_would_block(
            "destack.display.monitor.eventTryReadBatch",
            "no monitor event is currently queued",
        ));
    }

    // drain up to maxevents records
    let take = maxevents.min(state.pending.len());
    let mut events = Vec::with_capacity(take);
    // iterate this sequence
    for _ in 0..take {
        // evaluate this condition
        if let Some(record) = state.pending.pop_front() {
            events.push(display_event_from_record(binding, record));
        }
    }

    unsafe {
        *out = binding.store_array(events);
    }

    Ok(())
}

/// Open one global window-event stream.
pub(in crate::platform::display::host::unix) unsafe fn window_event_open(
    binding: &BindingCallContext,
    out: *mut resource::WindowEventHandle,
    options: WindowEventOpenOptions,
) -> RuntimeResult<()> {
    // validate out pointer and parse open filter
    core_platform::ensure_out(out, "out")?;
    let filter = WindowEventFilterState::from_open_options(options)?;

    // allocate stream resolved_binding with configured queue state
    let resolved_binding = Arc::new(WindowEventBinding {
        state: Mutex::new(WindowEventState {
            queue_capacity: core::resolved_queue_capacity(binding, options.queue.queue_capacity),
            overflow_policy: options.queue.overflow_policy,
            overflow_error_pending: false,
            next_sequence: 1,
            dropped_count: 0,
            pending: VecDeque::new(),
        }),
        filter,
        signal: Condvar::new(),
        owner_thread_id: std::thread::current().id(),
    });

    // register resource and subscriber entry
    let resource_id = binding.agent().resources.insert(
        binding.world(),
        ResourceEntry::new(ResourceKind::Window)
            .with_label(core::WINDOW_EVENT_RESOURCE_LABEL)
            .with_payload(Arc::clone(&resolved_binding)),
        Some(binding.engine()),
    );
    let runtime_state = core::runtime_state(binding);
    runtime_state
        .window_event_registry
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .push(Arc::downgrade(&resolved_binding));

    // write stream handle
    unsafe {
        *out = resource::WindowEventHandle(resource_id);
    }

    Ok(())
}

/// Close one global window-event stream.
pub(in crate::platform::display::host::unix) unsafe fn window_event_close(
    binding: &BindingCallContext,
    handle: resource::WindowEventHandle,
) -> RuntimeResult<()> {
    // resolve stream resolved_binding and remove it from subscriber registry
    let resolved_binding = display_resource::resolve_window_event_binding(
        binding,
        handle,
        "destack.display.window.eventClose",
    )?;
    let identity = Arc::as_ptr(&resolved_binding) as usize;
    let runtime_state = core::runtime_state(binding);
    {
        let mut registry = runtime_state
            .window_event_registry
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        core::retain_live_without_identity(&mut registry, identity);
    }

    // remove stream resource entry
    let removed = binding
        .agent()
        .resources
        .remove(binding.world(), handle.0, Some(binding.engine()))
        .is_some();
    // evaluate this condition
    if !removed {
        return Err(core_platform::io_not_found(
            "destack.display.window.eventClose",
            format!("window event handle {} was not found", handle.0.0),
        ));
    }

    Ok(())
}

/// Wait for one window event.
pub(in crate::platform::display::host::unix) unsafe fn window_event_read(
    binding: &BindingCallContext,
    out: *mut WindowEvent,
    handle: resource::WindowEventHandle,
    timeoutns: u64,
) -> RuntimeResult<()> {
    // validate out pointer and resolve stream resolved_binding
    core_platform::ensure_out(out, "out")?;
    let resolved_binding = display_resource::resolve_window_event_binding(
        binding,
        handle,
        "destack.display.window.eventRead",
    )?;

    // enforce owner-thread affinity for x11 event pumping
    ensure_window_event_thread(&resolved_binding, "destack.display.window.eventRead")?;

    // wait until one event is available or timeout expires
    let deadline = core_platform::monotonic_now_ns().saturating_add(timeoutns);
    let wait_slice_ns = core::window_event_wait_slice_ns(binding);
    // loop until one branch exits
    loop {
        // pump x11 events before reading queue state
        window::pump_window_messages(binding)?;

        let mut state = resolved_binding
            .state
            .lock()
            .unwrap_or_else(|error| error.into_inner());

        // surface overflow policy as explicit runtime error
        if state.overflow_error_pending {
            state.overflow_error_pending = false;
            return Err(core::overflow_error("destack.display.window.eventRead"));
        }

        // return next pending event when available
        if let Some(record) = state.pending.pop_front() {
            unsafe {
                *out = window_event_from_record(binding, record);
            }
            return Ok(());
        }

        // abort on timeout
        let now = core_platform::monotonic_now_ns();
        // evaluate this condition
        if now >= deadline {
            return Err(core_platform::io_would_block(
                "destack.display.window.eventRead",
                "event read timed out",
            ));
        }

        // wait for the remaining timeout slice
        let remaining = deadline.saturating_sub(now);
        let wait_duration = wait_duration(remaining, wait_slice_ns);
        let (next_state, _) = resolved_binding
            .signal
            .wait_timeout(state, wait_duration)
            .unwrap_or_else(|error| error.into_inner());
        drop(next_state);
    }
}

/// Wait for one batch of window events.
pub(in crate::platform::display::host::unix) unsafe fn window_event_read_batch(
    binding: &BindingCallContext,
    out: *mut NativeArray<WindowEvent>,
    handle: resource::WindowEventHandle,
    maxevents: u32,
    timeoutns: u64,
) -> RuntimeResult<()> {
    // validate out pointer and batch size
    core_platform::ensure_out(out, "out")?;
    let maxevents = core_platform::u32_to_nonzero_usize("maxevents", maxevents)?;

    // resolve stream resolved_binding and enforce owner-thread affinity
    let resolved_binding = display_resource::resolve_window_event_binding(
        binding,
        handle,
        "destack.display.window.eventReadBatch",
    )?;
    ensure_window_event_thread(&resolved_binding, "destack.display.window.eventReadBatch")?;

    // wait until one or more events are available or timeout expires
    let deadline = core_platform::monotonic_now_ns().saturating_add(timeoutns);
    let wait_slice_ns = core::window_event_wait_slice_ns(binding);
    // loop until one branch exits
    loop {
        // pump x11 events before reading queue state
        window::pump_window_messages(binding)?;

        let mut state = resolved_binding
            .state
            .lock()
            .unwrap_or_else(|error| error.into_inner());

        // surface overflow policy as explicit runtime error
        if state.overflow_error_pending {
            state.overflow_error_pending = false;
            return Err(core::overflow_error(
                "destack.display.window.eventReadBatch",
            ));
        }

        // drain up to maxevents records when queue is non-empty
        if !state.pending.is_empty() {
            let take = maxevents.min(state.pending.len());
            let mut events = Vec::with_capacity(take);
            // iterate this sequence
            for _ in 0..take {
                // evaluate this condition
                if let Some(record) = state.pending.pop_front() {
                    events.push(window_event_from_record(binding, record));
                }
            }

            unsafe {
                *out = binding.store_array(events);
            }
            return Ok(());
        }

        // abort on timeout
        let now = core_platform::monotonic_now_ns();
        // evaluate this condition
        if now >= deadline {
            return Err(core_platform::io_would_block(
                "destack.display.window.eventReadBatch",
                "event read timed out",
            ));
        }

        // wait for the remaining timeout slice
        let remaining = deadline.saturating_sub(now);
        let wait_duration = wait_duration(remaining, wait_slice_ns);
        let (next_state, _) = resolved_binding
            .signal
            .wait_timeout(state, wait_duration)
            .unwrap_or_else(|error| error.into_inner());
        drop(next_state);
    }
}

/// Poll one window event without blocking.
pub(in crate::platform::display::host::unix) unsafe fn window_event_try_read(
    binding: &BindingCallContext,
    out: *mut WindowEvent,
    handle: resource::WindowEventHandle,
) -> RuntimeResult<()> {
    // validate out pointer and resolve stream resolved_binding
    core_platform::ensure_out(out, "out")?;
    let resolved_binding = display_resource::resolve_window_event_binding(
        binding,
        handle,
        "destack.display.window.eventTryRead",
    )?;
    ensure_window_event_thread(&resolved_binding, "destack.display.window.eventTryRead")?;

    // pump x11 events and pop one queued event
    window::pump_window_messages(binding)?;
    let mut state = resolved_binding
        .state
        .lock()
        .unwrap_or_else(|error| error.into_inner());

    // surface overflow policy as explicit runtime error
    if state.overflow_error_pending {
        state.overflow_error_pending = false;
        return Err(core::overflow_error("destack.display.window.eventTryRead"));
    }

    let Some(record) = state.pending.pop_front() else {
        return Err(core_platform::io_would_block(
            "destack.display.window.eventTryRead",
            "no window event is currently queued",
        ));
    };
    unsafe {
        *out = window_event_from_record(binding, record);
    }

    Ok(())
}

/// Poll one batch of window events without blocking.
pub(in crate::platform::display::host::unix) unsafe fn window_event_try_read_batch(
    binding: &BindingCallContext,
    out: *mut NativeArray<WindowEvent>,
    handle: resource::WindowEventHandle,
    maxevents: u32,
) -> RuntimeResult<()> {
    // validate out pointer and batch size
    core_platform::ensure_out(out, "out")?;
    let maxevents = core_platform::u32_to_nonzero_usize("maxevents", maxevents)?;

    // resolve stream resolved_binding and enforce owner-thread affinity
    let resolved_binding = display_resource::resolve_window_event_binding(
        binding,
        handle,
        "destack.display.window.eventTryReadBatch",
    )?;
    ensure_window_event_thread(
        &resolved_binding,
        "destack.display.window.eventTryReadBatch",
    )?;

    // pump x11 events and pop queued batch
    window::pump_window_messages(binding)?;
    let mut state = resolved_binding
        .state
        .lock()
        .unwrap_or_else(|error| error.into_inner());

    // surface overflow policy as explicit runtime error
    if state.overflow_error_pending {
        state.overflow_error_pending = false;
        return Err(core::overflow_error(
            "destack.display.window.eventTryReadBatch",
        ));
    }

    // evaluate this condition
    if state.pending.is_empty() {
        return Err(core_platform::io_would_block(
            "destack.display.window.eventTryReadBatch",
            "no window event is currently queued",
        ));
    }

    // drain up to maxevents records
    let take = maxevents.min(state.pending.len());
    let mut events = Vec::with_capacity(take);
    // iterate this sequence
    for _ in 0..take {
        // evaluate this condition
        if let Some(record) = state.pending.pop_front() {
            events.push(window_event_from_record(binding, record));
        }
    }

    unsafe {
        *out = binding.store_array(events);
    }

    Ok(())
}
