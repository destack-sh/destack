use std::sync::Arc;

use crate::diagnostic::RuntimeResult;
use crate::platform::display::unix::appkit::{core as appkit_core, resource as display_resource};
use crate::platform::display::{
    DisplayMonitorEvent, DisplayMonitorEventOpenOptions, options as display_options,
};
use crate::platform::resource::{ResourceEntry, ResourceKind};
use crate::platform::{NativeArray, core as core_platform, resource};
use crate::runtime::BindingCallContext;
use crate::runtime::binding::BindingAffinity;

use crate::platform::display::unix::appkit::event::codec::display_event_from_record;
use crate::platform::display::unix::appkit::event::publish::seed_monitor_event_stream;
use crate::platform::display::unix::appkit::event::queue::{
    pop_live_monitor_record, pop_seeded_monitor_record, take_monitor_overflow_error,
    trim_monitor_events,
};
use crate::platform::display::unix::appkit::event::{
    MonitorEventFilterState, MonitorEventState, MonitorEventStream,
};

/// Return the next monitor event available to one stream.
fn next_monitor_event(
    binding: &BindingCallContext,
    runtime_state: &Arc<appkit_core::AppKitRuntimeState>,
    resolved_stream: &Arc<MonitorEventStream>,
    operation: &'static str,
) -> RuntimeResult<Option<DisplayMonitorEvent>> {
    // surface queued overflow immediately
    if take_monitor_overflow_error(resolved_stream) {
        return Err(appkit_core::overflow_error(operation));
    }

    // deliver seeded snapshot records before live records
    if let Some(record) = pop_seeded_monitor_record(resolved_stream) {
        return Ok(Some(display_event_from_record(binding, record)));
    }

    // deliver one live record when the shared event log has one
    if let Some(record) = pop_live_monitor_record(runtime_state, resolved_stream) {
        return Ok(Some(display_event_from_record(binding, record)));
    }

    Ok(None)
}

/// Drain up to `maxevents` monitor events for one stream.
fn drain_monitor_events(
    binding: &BindingCallContext,
    runtime_state: &Arc<appkit_core::AppKitRuntimeState>,
    resolved_stream: &Arc<MonitorEventStream>,
    maxevents: usize,
    operation: &'static str,
) -> RuntimeResult<Vec<DisplayMonitorEvent>> {
    let mut events = Vec::new();

    // drain seeded and live records until the batch is full or the stream is empty
    while events.len() < maxevents {
        // surface queued overflow before each record pull
        if take_monitor_overflow_error(resolved_stream) {
            return Err(appkit_core::overflow_error(operation));
        }

        // prefer seeded records before the live frontier
        if let Some(record) = pop_seeded_monitor_record(resolved_stream) {
            events.push(display_event_from_record(binding, record));
            continue;
        }

        // stop once the live frontier is exhausted
        let Some(record) = pop_live_monitor_record(runtime_state, resolved_stream) else {
            break;
        };
        events.push(display_event_from_record(binding, record));
    }

    Ok(events)
}

/// Open one global monitor-event stream.
pub(crate) unsafe fn monitor_event_open(
    binding: &BindingCallContext,
    out: *mut resource::DisplayEventHandle,
    options: DisplayMonitorEventOpenOptions,
) -> RuntimeResult<()> {
    // validate out pointer and parse the filter payload
    core_platform::ensure_out(out, "out")?;
    let filter = MonitorEventFilterState::from_open_options(options)?;
    let runtime_state = appkit_core::runtime_state(binding);

    // capture the current live frontier for this new subscription
    let next_live_sequence = {
        let monitor_events = runtime_state
            .monitor_events
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        monitor_events.next_sequence
    };

    // allocate stream state before seeding the initial snapshot
    let resolved_stream = Arc::new(MonitorEventStream {
        stream_id: runtime_state.next_monitor_stream_id(),
        state: std::sync::Mutex::new(MonitorEventState {
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

    // seed the stream from the current topology snapshot
    let snapshots = seed_monitor_event_stream(&resolved_stream)?;
    runtime_state.initialize_monitor_topology_snapshot(snapshots);

    // register the resource and stream entry
    let resource_id = binding.worker().resources.insert(
        binding.world(),
        ResourceEntry::new(ResourceKind::Display)
            .with_label(appkit_core::DISPLAY_EVENT_RESOURCE_LABEL)
            .with_binding_affinity(BindingAffinity::Worker)
            .with_payload(Arc::clone(&resolved_stream)),
        Some(binding.engine()),
    );
    runtime_state.register_monitor_stream(Arc::clone(&resolved_stream));

    // write the opened stream handle
    unsafe {
        *out = resource::DisplayEventHandle(resource_id);
    }

    Ok(())
}

/// Close one global monitor-event stream.
pub(crate) unsafe fn monitor_event_close(
    binding: &BindingCallContext,
    handle: resource::DisplayEventHandle,
) -> RuntimeResult<()> {
    // resolve the stream before removing it from the registry
    let resolved_stream = display_resource::resolve_monitor_event_stream(
        binding,
        handle,
        "destack.display.monitor.eventClose",
    )?;
    let runtime_state = appkit_core::runtime_state(binding);

    // remove the stream entry and trim any now-unreachable live records
    runtime_state.unregister_monitor_stream(resolved_stream.stream_id);
    trim_monitor_events(&runtime_state);

    // remove the resource entry itself
    let removed = binding
        .worker()
        .resources
        .remove(binding.world(), handle.0, Some(binding.engine()))
        .is_some();

    // reject unknown handles loudly
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
    binding: &BindingCallContext,
    out: *mut DisplayMonitorEvent,
    handle: resource::DisplayEventHandle,
    timeoutns: u64,
) -> RuntimeResult<()> {
    // validate out pointer and resolve the stream payload
    core_platform::ensure_out(out, "out")?;
    let resolved_stream = display_resource::resolve_monitor_event_stream(
        binding,
        handle,
        "destack.display.monitor.eventRead",
    )?;
    let runtime_state = appkit_core::runtime_state(binding);
    let deadline = core_platform::monotonic_now_ns().saturating_add(timeoutns);

    // wait until one seeded or live record becomes visible
    let event = binding.wait_for_binding_result(
        "destack.display.monitor.eventRead",
        "event read timed out",
        deadline,
        || {
            next_monitor_event(
                binding,
                &runtime_state,
                &resolved_stream,
                "destack.display.monitor.eventRead",
            )
        },
        |duration| {
            let monitor_events = runtime_state
                .monitor_events
                .lock()
                .unwrap_or_else(|error| error.into_inner());
            let (monitor_events, _) = runtime_state
                .monitor_event_signal
                .wait_timeout(monitor_events, duration)
                .unwrap_or_else(|error| error.into_inner());
            drop(monitor_events);
        },
    )?;

    unsafe {
        *out = event;
    }

    Ok(())
}

/// Wait for one batch of monitor events.
pub(crate) unsafe fn monitor_event_read_batch(
    binding: &BindingCallContext,
    out: *mut NativeArray<DisplayMonitorEvent>,
    handle: resource::DisplayEventHandle,
    maxevents: u32,
    timeoutns: u64,
) -> RuntimeResult<()> {
    // validate out pointer and batch size
    core_platform::ensure_out(out, "out")?;
    let maxevents = core_platform::u32_to_nonzero_usize("maxevents", maxevents)?;
    let resolved_stream = display_resource::resolve_monitor_event_stream(
        binding,
        handle,
        "destack.display.monitor.eventReadBatch",
    )?;
    let runtime_state = appkit_core::runtime_state(binding);
    let deadline = core_platform::monotonic_now_ns().saturating_add(timeoutns);

    // wait until at least one event is available, then drain a bounded batch
    let events = binding.wait_for_binding_result(
        "destack.display.monitor.eventReadBatch",
        "event read timed out",
        deadline,
        || {
            let events = drain_monitor_events(
                binding,
                &runtime_state,
                &resolved_stream,
                maxevents,
                "destack.display.monitor.eventReadBatch",
            )?;

            if events.is_empty() {
                return Ok(None);
            }

            Ok(Some(events))
        },
        |duration| {
            let monitor_events = runtime_state
                .monitor_events
                .lock()
                .unwrap_or_else(|error| error.into_inner());
            let (monitor_events, _) = runtime_state
                .monitor_event_signal
                .wait_timeout(monitor_events, duration)
                .unwrap_or_else(|error| error.into_inner());
            drop(monitor_events);
        },
    )?;

    unsafe {
        *out = binding.store_array(events);
    }

    Ok(())
}

/// Poll one monitor event without blocking.
pub(crate) unsafe fn monitor_event_try_read(
    binding: &BindingCallContext,
    out: *mut DisplayMonitorEvent,
    handle: resource::DisplayEventHandle,
) -> RuntimeResult<()> {
    // validate out pointer and resolve the stream payload
    core_platform::ensure_out(out, "out")?;
    let resolved_stream = display_resource::resolve_monitor_event_stream(
        binding,
        handle,
        "destack.display.monitor.eventTryRead",
    )?;
    let runtime_state = appkit_core::runtime_state(binding);

    // return one event when the stream already has work visible
    let Some(event) = next_monitor_event(
        binding,
        &runtime_state,
        &resolved_stream,
        "destack.display.monitor.eventTryRead",
    )?
    else {
        return Err(core_platform::io_would_block(
            "destack.display.monitor.eventTryRead",
            "no monitor event is currently queued",
        ));
    };

    unsafe {
        *out = event;
    }
    Ok(())
}

/// Poll one batch of monitor events without blocking.
pub(crate) unsafe fn monitor_event_try_read_batch(
    binding: &BindingCallContext,
    out: *mut NativeArray<DisplayMonitorEvent>,
    handle: resource::DisplayEventHandle,
    maxevents: u32,
) -> RuntimeResult<()> {
    // validate out pointer and batch size
    core_platform::ensure_out(out, "out")?;
    let maxevents = core_platform::u32_to_nonzero_usize("maxevents", maxevents)?;

    // resolve the stream and drain one bounded batch immediately
    let resolved_stream = display_resource::resolve_monitor_event_stream(
        binding,
        handle,
        "destack.display.monitor.eventTryReadBatch",
    )?;
    let runtime_state = appkit_core::runtime_state(binding);
    let events = drain_monitor_events(
        binding,
        &runtime_state,
        &resolved_stream,
        maxevents,
        "destack.display.monitor.eventTryReadBatch",
    )?;

    // report would-block when the stream is currently empty
    if events.is_empty() {
        return Err(core_platform::io_would_block(
            "destack.display.monitor.eventTryReadBatch",
            "no monitor event is currently queued",
        ));
    }

    unsafe {
        *out = binding.store_array(events);
    }
    Ok(())
}
