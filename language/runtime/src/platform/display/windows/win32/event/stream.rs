use super::*;

/// Open one global monitor-event stream.
pub(crate) unsafe fn monitor_event_open(
    context: &BindingCallContext,
    out: *mut resource::DisplayEventHandle,
    options: DisplayMonitorEventOpenOptions,
) -> RuntimeResult<()> {
    // validate out pointer and parse open filter
    core_platform::ensure_out(out, "out")?;
    let filter = unsafe { MonitorEventFilterState::from_open_options(options)? };

    // allocate stream binding with configured queue state
    let binding = Arc::new(MonitorEventBinding {
        state: Mutex::new(MonitorEventState {
            queue_capacity: core::resolved_queue_capacity(context, options.queue.queue_capacity),
            overflow_policy: options.queue.overflow_policy,
            overflow_error_pending: false,
            next_sequence: 1,
            dropped_count: 0,
            pending: VecDeque::new(),
        }),
        filter,
        signal: Condvar::new(),
    });

    // seed stream with current monitor topology snapshot
    let seeded_snapshots = {
        let mut state = binding
            .state
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        seed_monitor_event_stream(&mut state, &binding.filter)?
    };

    // register resource and subscriber entry
    let resource_id = context.runtime().resources.insert(
        ResourceEntry::new(ResourceKind::Display)
            .with_label(display_resource::DISPLAY_EVENT_RESOURCE_LABEL)
            .with_payload(Arc::clone(&binding)),
        Some(context.engine()),
    );
    let runtime_state = display_event_runtime_state(context);
    {
        let mut topology_snapshot = runtime_state
            .monitor_topology_snapshot
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        // evaluate this condition
        if topology_snapshot.is_none() {
            *topology_snapshot = Some(seeded_snapshots);
        }
    }
    runtime_state
        .monitor_event_registry
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .push(Arc::downgrade(&binding));

    // write stream handle
    unsafe {
        *out = resource::DisplayEventHandle(resource_id);
    }

    Ok(())
}

/// Close one global monitor-event stream.
pub(crate) unsafe fn monitor_event_close(
    context: &BindingCallContext,
    handle: resource::DisplayEventHandle,
) -> RuntimeResult<()> {
    // resolve stream binding and remove it from subscriber registry
    let binding = display_resource::resolve_monitor_event_binding(
        context,
        handle,
        "destack.display.monitor.eventClose",
    )?;

    let identity = Arc::as_ptr(&binding) as usize;
    let runtime_state = display_event_runtime_state(context);
    {
        let mut registry = runtime_state
            .monitor_event_registry
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        retain_live_without_identity(&mut registry, identity);
    }

    // remove stream resource entry
    let removed = context
        .runtime()
        .resources
        .remove(handle.0, Some(context.engine()))
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
pub(crate) unsafe fn monitor_event_read(
    context: &BindingCallContext,
    out: *mut DisplayMonitorEvent,
    handle: resource::DisplayEventHandle,
    timeoutns: u64,
) -> RuntimeResult<()> {
    // validate out pointer and resolve stream binding
    core_platform::ensure_out(out, "out")?;

    let binding = display_resource::resolve_monitor_event_binding(
        context,
        handle,
        "destack.display.monitor.eventRead",
    )?;

    // wait until one event is available or timeout expires
    let deadline = core_platform::monotonic_now_ns().saturating_add(timeoutns);
    let mut state = binding
        .state
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    // loop until one branch exits
    loop {
        // surface overflow policy as explicit runtime error
        consume_overflow_error(&mut *state, "destack.display.monitor.eventRead")?;

        // return next pending event when available
        if let Some(record) = state.pending.pop_front() {
            unsafe {
                *out = display_event_from_record(context, record);
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
        let wait_duration = wait_duration(remaining);
        let (next_state, _) = binding
            .signal
            .wait_timeout(state, wait_duration)
            .unwrap_or_else(|error| error.into_inner());
        state = next_state;
    }
}

/// Wait for one batch of monitor events.
pub(crate) unsafe fn monitor_event_read_batch(
    context: &BindingCallContext,
    out: *mut NativeArray<DisplayMonitorEvent>,
    handle: resource::DisplayEventHandle,
    maxevents: u32,
    timeoutns: u64,
) -> RuntimeResult<()> {
    // validate out pointer and batch size
    core_platform::ensure_out(out, "out")?;
    let maxevents = core::validate_max_events(maxevents, "maxevents")?;

    // resolve stream binding
    let binding = display_resource::resolve_monitor_event_binding(
        context,
        handle,
        "destack.display.monitor.eventReadBatch",
    )?;

    // wait until one or more events are available or timeout expires
    let deadline = core_platform::monotonic_now_ns().saturating_add(timeoutns);
    let mut state = binding
        .state
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    // loop until one branch exits
    loop {
        // surface overflow policy as explicit runtime error
        consume_overflow_error(&mut *state, "destack.display.monitor.eventReadBatch")?;

        // drain up to maxevents records when queue is non-empty
        if !state.pending.is_empty() {
            let take = maxevents.min(state.pending.len());
            let mut events = Vec::with_capacity(take);
            // iterate this sequence
            for _ in 0..take {
                // evaluate this condition
                if let Some(record) = state.pending.pop_front() {
                    events.push(display_event_from_record(context, record));
                }
            }

            unsafe {
                *out = context.store_array(events);
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
        let wait_duration = wait_duration(remaining);
        let (next_state, _) = binding
            .signal
            .wait_timeout(state, wait_duration)
            .unwrap_or_else(|error| error.into_inner());
        state = next_state;
    }
}

/// Poll one monitor event without blocking.
pub(crate) unsafe fn monitor_event_try_read(
    context: &BindingCallContext,
    out: *mut DisplayMonitorEvent,
    handle: resource::DisplayEventHandle,
) -> RuntimeResult<()> {
    // validate out pointer and resolve stream binding
    core_platform::ensure_out(out, "out")?;

    let binding = display_resource::resolve_monitor_event_binding(
        context,
        handle,
        "destack.display.monitor.eventTryRead",
    )?;
    let mut state = binding
        .state
        .lock()
        .unwrap_or_else(|error| error.into_inner());

    // pop one event without blocking
    let record = pop_pending_record(
        &mut *state,
        "destack.display.monitor.eventTryRead",
        "no monitor event is currently queued",
    )?;

    // write decoded event payload
    unsafe {
        *out = display_event_from_record(context, record);
    }

    Ok(())
}

/// Poll one batch of monitor events without blocking.
pub(crate) unsafe fn monitor_event_try_read_batch(
    context: &BindingCallContext,
    out: *mut NativeArray<DisplayMonitorEvent>,
    handle: resource::DisplayEventHandle,
    maxevents: u32,
) -> RuntimeResult<()> {
    // validate out pointer and batch size
    core_platform::ensure_out(out, "out")?;
    let maxevents = core::validate_max_events(maxevents, "maxevents")?;

    // resolve stream binding and pop pending batch without blocking
    let binding = display_resource::resolve_monitor_event_binding(
        context,
        handle,
        "destack.display.monitor.eventTryReadBatch",
    )?;
    let mut state = binding
        .state
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let records = pop_pending_batch(
        &mut *state,
        maxevents,
        "destack.display.monitor.eventTryReadBatch",
        "no monitor event is currently queued",
    )?;

    // encode drained records
    let mut events = Vec::with_capacity(records.len());
    // iterate this sequence
    for record in records {
        events.push(display_event_from_record(context, record));
    }

    // write event batch payload
    unsafe {
        *out = context.store_array(events);
    }

    Ok(())
}

/// Open one global window-event stream.
pub(crate) unsafe fn window_event_open(
    context: &BindingCallContext,
    out: *mut resource::WindowEventHandle,
    options: WindowEventOpenOptions,
) -> RuntimeResult<()> {
    // validate out pointer and parse open filter
    core_platform::ensure_out(out, "out")?;
    let filter = WindowEventFilterState::from_open_options(options)?;

    // allocate stream binding with configured queue state
    let binding = Arc::new(WindowEventBinding {
        state: Mutex::new(WindowEventState {
            queue_capacity: core::resolved_queue_capacity(context, options.queue.queue_capacity),
            overflow_policy: options.queue.overflow_policy,
            overflow_error_pending: false,
            next_sequence: 1,
            dropped_count: 0,
            pending: VecDeque::new(),
        }),
        filter,
        signal: Condvar::new(),
        owner_thread_id: current_thread_id(),
    });

    // register resource and subscriber entry
    let resource_id = context.runtime().resources.insert(
        ResourceEntry::new(ResourceKind::Window)
            .with_label(display_resource::WINDOW_EVENT_RESOURCE_LABEL)
            .with_payload(Arc::clone(&binding)),
        Some(context.engine()),
    );
    let runtime_state = display_event_runtime_state(context);
    runtime_state
        .window_event_registry
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .push(Arc::downgrade(&binding));

    // write stream handle
    unsafe {
        *out = resource::WindowEventHandle(resource_id);
    }

    Ok(())
}

/// Close one global window-event stream.
pub(crate) unsafe fn window_event_close(
    context: &BindingCallContext,
    handle: resource::WindowEventHandle,
) -> RuntimeResult<()> {
    // resolve stream binding and remove it from subscriber registry
    let binding = display_resource::resolve_window_event_binding(
        context,
        handle,
        "destack.display.window.eventClose",
    )?;

    let identity = Arc::as_ptr(&binding) as usize;
    let runtime_state = display_event_runtime_state(context);
    {
        let mut registry = runtime_state
            .window_event_registry
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        retain_live_without_identity(&mut registry, identity);
    }

    // remove stream resource entry
    let removed = context
        .runtime()
        .resources
        .remove(handle.0, Some(context.engine()))
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
pub(crate) unsafe fn window_event_read(
    context: &BindingCallContext,
    out: *mut WindowEvent,
    handle: resource::WindowEventHandle,
    timeoutns: u64,
) -> RuntimeResult<()> {
    // validate out pointer and resolve stream binding
    core_platform::ensure_out(out, "out")?;

    let binding = display_resource::resolve_window_event_binding(
        context,
        handle,
        "destack.display.window.eventRead",
    )?;

    // enforce owner-thread affinity for window message pumping
    ensure_window_event_thread(&binding, "destack.display.window.eventRead")?;

    // wait until one event is available or timeout expires
    let deadline = core_platform::monotonic_now_ns().saturating_add(timeoutns);
    // loop until one branch exits
    loop {
        // pump native window messages before reading queue state
        window::pump_window_messages(context)?;

        let mut state = binding
            .state
            .lock()
            .unwrap_or_else(|error| error.into_inner());

        // surface overflow policy as explicit runtime error
        consume_overflow_error(&mut *state, "destack.display.window.eventRead")?;

        // return next pending event when available
        if let Some(record) = state.pending.pop_front() {
            unsafe {
                *out = window_event_from_record(record, context);
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

        // wait for one bounded timeout slice
        let wait_slice_ns = core::window_event_wait_slice_ns(context);
        let remaining_ns = deadline.saturating_sub(now).min(wait_slice_ns);
        let wait_duration = wait_duration(remaining_ns);
        let state_guard = binding
            .signal
            .wait_timeout(state, wait_duration)
            .unwrap_or_else(|error| error.into_inner());
        drop(state_guard);
    }
}

/// Wait for one batch of window events.
pub(crate) unsafe fn window_event_read_batch(
    context: &BindingCallContext,
    out: *mut NativeArray<WindowEvent>,
    handle: resource::WindowEventHandle,
    maxevents: u32,
    timeoutns: u64,
) -> RuntimeResult<()> {
    // validate out pointer and batch size
    core_platform::ensure_out(out, "out")?;
    let maxevents = core::validate_max_events(maxevents, "maxevents")?;

    // resolve stream binding and enforce owner-thread affinity
    let binding = display_resource::resolve_window_event_binding(
        context,
        handle,
        "destack.display.window.eventReadBatch",
    )?;
    ensure_window_event_thread(&binding, "destack.display.window.eventReadBatch")?;

    // wait until one or more events are available or timeout expires
    let deadline = core_platform::monotonic_now_ns().saturating_add(timeoutns);
    // loop until one branch exits
    loop {
        // pump native window messages before reading queue state
        window::pump_window_messages(context)?;

        let mut state = binding
            .state
            .lock()
            .unwrap_or_else(|error| error.into_inner());

        // surface overflow policy as explicit runtime error
        consume_overflow_error(&mut *state, "destack.display.window.eventReadBatch")?;

        // drain up to maxevents records when queue is non-empty
        if !state.pending.is_empty() {
            let take = maxevents.min(state.pending.len());
            let mut events = Vec::with_capacity(take);
            // iterate this sequence
            for _ in 0..take {
                // evaluate this condition
                if let Some(record) = state.pending.pop_front() {
                    events.push(window_event_from_record(record, context));
                }
            }

            unsafe {
                *out = context.store_array(events);
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

        // wait for one bounded timeout slice
        let wait_slice_ns = core::window_event_wait_slice_ns(context);
        let remaining_ns = deadline.saturating_sub(now).min(wait_slice_ns);
        let wait_duration = wait_duration(remaining_ns);
        let state_guard = binding
            .signal
            .wait_timeout(state, wait_duration)
            .unwrap_or_else(|error| error.into_inner());
        drop(state_guard);
    }
}

/// Poll one window event without blocking.
pub(crate) unsafe fn window_event_try_read(
    context: &BindingCallContext,
    out: *mut WindowEvent,
    handle: resource::WindowEventHandle,
) -> RuntimeResult<()> {
    // validate out pointer and resolve stream binding
    core_platform::ensure_out(out, "out")?;

    let binding = display_resource::resolve_window_event_binding(
        context,
        handle,
        "destack.display.window.eventTryRead",
    )?;

    // enforce owner-thread affinity for window message pumping
    ensure_window_event_thread(&binding, "destack.display.window.eventTryRead")?;

    // pump native window messages before non-blocking read
    window::pump_window_messages(context)?;
    let mut state = binding
        .state
        .lock()
        .unwrap_or_else(|error| error.into_inner());

    // pop one event without blocking
    let record = pop_pending_record(
        &mut *state,
        "destack.display.window.eventTryRead",
        "no window event is currently queued",
    )?;

    // write decoded event payload
    unsafe {
        *out = window_event_from_record(record, context);
    }

    Ok(())
}

/// Poll one batch of window events without blocking.
pub(crate) unsafe fn window_event_try_read_batch(
    context: &BindingCallContext,
    out: *mut NativeArray<WindowEvent>,
    handle: resource::WindowEventHandle,
    maxevents: u32,
) -> RuntimeResult<()> {
    // validate out pointer and batch size
    core_platform::ensure_out(out, "out")?;
    let maxevents = core::validate_max_events(maxevents, "maxevents")?;

    // resolve stream binding and enforce owner-thread affinity
    let binding = display_resource::resolve_window_event_binding(
        context,
        handle,
        "destack.display.window.eventTryReadBatch",
    )?;
    ensure_window_event_thread(&binding, "destack.display.window.eventTryReadBatch")?;

    // pump native window messages before non-blocking read
    window::pump_window_messages(context)?;
    let mut state = binding
        .state
        .lock()
        .unwrap_or_else(|error| error.into_inner());

    // pop one batch without blocking
    let records = pop_pending_batch(
        &mut *state,
        maxevents,
        "destack.display.window.eventTryReadBatch",
        "no window event is currently queued",
    )?;

    // encode drained records
    let mut events = Vec::with_capacity(records.len());
    // iterate this sequence
    for record in records {
        events.push(window_event_from_record(record, context));
    }

    // write event batch payload
    unsafe {
        *out = context.store_array(events);
    }

    Ok(())
}
