use std::sync::Arc;

use crate::diagnostic::RuntimeResult;
use crate::platform::display::unix::wayland::{core as wayland_core, resource as display_resource};
use crate::platform::display::{
    DisplayMonitorEvent, DisplayMonitorEventOpenOptions, options as display_options,
};
use crate::platform::resource::{ResourceEntry, ResourceKind};
use crate::platform::{NativeArray, core as core_platform, resource};
use crate::runtime::BindingCallContext;
use crate::runtime::binding::BindingAffinity;

use crate::platform::display::unix::wayland::event::codec::display_event_from_record;
use crate::platform::display::unix::wayland::event::publish::seed_monitor_event_stream;
use crate::platform::display::unix::wayland::event::queue::{
    pop_live_monitor_record, pop_seeded_monitor_record, take_monitor_overflow_error,
    trim_monitor_events,
};
use crate::platform::display::unix::wayland::event::{
    MonitorEventFilterState, MonitorEventState, MonitorEventStream,
};

/// Return the next monitor event available to one stream.
fn next_monitor_event(
    binding: &BindingCallContext,
    runtime_state: &Arc<wayland_core::WaylandRuntimeState>,
    resolved_stream: &Arc<MonitorEventStream>,
    operation: &'static str,
) -> RuntimeResult<Option<DisplayMonitorEvent>> {
    if take_monitor_overflow_error(resolved_stream) {
        return Err(wayland_core::overflow_error(operation));
    }

    if let Some(record) = pop_seeded_monitor_record(resolved_stream) {
        return Ok(Some(display_event_from_record(binding, record)));
    }

    if let Some(record) = pop_live_monitor_record(runtime_state, resolved_stream) {
        return Ok(Some(display_event_from_record(binding, record)));
    }

    Ok(None)
}

/// Drain up to `maxevents` monitor events for one stream.
fn drain_monitor_events(
    binding: &BindingCallContext,
    runtime_state: &Arc<wayland_core::WaylandRuntimeState>,
    resolved_stream: &Arc<MonitorEventStream>,
    maxevents: usize,
    operation: &'static str,
) -> RuntimeResult<Vec<DisplayMonitorEvent>> {
    let mut events = Vec::new();

    while events.len() < maxevents {
        if take_monitor_overflow_error(resolved_stream) {
            return Err(wayland_core::overflow_error(operation));
        }

        if let Some(record) = pop_seeded_monitor_record(resolved_stream) {
            events.push(display_event_from_record(binding, record));
            continue;
        }

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
    core_platform::ensure_out(out, "out")?;
    let filter = MonitorEventFilterState::from_open_options(options)?;
    let runtime_state = wayland_core::runtime_state(binding);

    let next_live_sequence = {
        let monitor_events = runtime_state
            .monitor_events
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        monitor_events.next_sequence
    };

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

    let snapshots = seed_monitor_event_stream(binding, &resolved_stream)?;
    runtime_state.initialize_monitor_topology_snapshot(snapshots);

    let resource_id = binding.worker().resources.insert(
        binding.world(),
        ResourceEntry::new(ResourceKind::Display)
            .with_label(wayland_core::DISPLAY_EVENT_RESOURCE_LABEL)
            .with_binding_affinity(BindingAffinity::Worker)
            .with_payload(Arc::clone(&resolved_stream)),
        Some(binding.engine()),
    );
    runtime_state.register_monitor_stream(Arc::clone(&resolved_stream));

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
    let resolved_stream = display_resource::resolve_monitor_event_stream(
        binding,
        handle,
        "destack.display.monitor.eventClose",
    )?;
    let runtime_state = wayland_core::runtime_state(binding);

    runtime_state.unregister_monitor_stream(resolved_stream.stream_id);
    trim_monitor_events(&runtime_state);

    let removed = binding
        .worker()
        .resources
        .remove(binding.world(), handle.0, Some(binding.engine()))
        .is_some();

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
    core_platform::ensure_out(out, "out")?;
    let resolved_stream = display_resource::resolve_monitor_event_stream(
        binding,
        handle,
        "destack.display.monitor.eventRead",
    )?;
    let runtime_state = wayland_core::runtime_state(binding);
    let deadline = core_platform::monotonic_now_ns().saturating_add(timeoutns);

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
    core_platform::ensure_out(out, "out")?;
    let maxevents = core_platform::u32_to_nonzero_usize("maxevents", maxevents)?;
    let resolved_stream = display_resource::resolve_monitor_event_stream(
        binding,
        handle,
        "destack.display.monitor.eventReadBatch",
    )?;
    let runtime_state = wayland_core::runtime_state(binding);
    let deadline = core_platform::monotonic_now_ns().saturating_add(timeoutns);

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
    core_platform::ensure_out(out, "out")?;
    let resolved_stream = display_resource::resolve_monitor_event_stream(
        binding,
        handle,
        "destack.display.monitor.eventTryRead",
    )?;
    let runtime_state = wayland_core::runtime_state(binding);

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
    core_platform::ensure_out(out, "out")?;
    let maxevents = core_platform::u32_to_nonzero_usize("maxevents", maxevents)?;
    let resolved_stream = display_resource::resolve_monitor_event_stream(
        binding,
        handle,
        "destack.display.monitor.eventTryReadBatch",
    )?;
    let runtime_state = wayland_core::runtime_state(binding);

    let events = drain_monitor_events(
        binding,
        &runtime_state,
        &resolved_stream,
        maxevents,
        "destack.display.monitor.eventTryReadBatch",
    )?;

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
