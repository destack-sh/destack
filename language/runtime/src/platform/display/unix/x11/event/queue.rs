use std::sync::Arc;
use std::time::Duration;

use crate::diagnostic::RuntimeResult;
use crate::platform::core as core_platform;

use super::super::super::core;
use super::core::{DisplayEventRecord, MonitorEventBinding, WindowEventBinding, WindowEventRecord};

/// Push one monitor-event record into one stream queue.
pub(super) fn push_monitor_record(
    binding: &Arc<MonitorEventBinding>,
    mut record: DisplayEventRecord,
) {
    // skip records filtered out for this stream
    if !binding.filter.matches(&record) {
        return;
    }

    // queue the record under overflow policy and wake waiting readers
    let mut state = binding
        .state
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    record.sequence = state.next_sequence;
    record.dropped_count = state.dropped_count;
    state.next_sequence = state.next_sequence.saturating_add(1);

    let queue_capacity = state.queue_capacity;
    let overflow_policy = state.overflow_policy;
    let mut overflow_error_pending = state.overflow_error_pending;
    let mut dropped_count = state.dropped_count;
    core::push_with_overflow(
        &mut state.pending,
        queue_capacity,
        overflow_policy,
        &mut overflow_error_pending,
        &mut dropped_count,
        record,
    );
    state.overflow_error_pending = overflow_error_pending;
    state.dropped_count = dropped_count;
    drop(state);
    binding.signal.notify_all();
}

/// Push one window-event record into one stream queue.
pub(super) fn push_window_record(binding: &Arc<WindowEventBinding>, mut record: WindowEventRecord) {
    // skip records filtered out for this stream
    if !binding.filter.matches(&record) {
        return;
    }

    // queue the record under overflow policy and wake waiting readers
    let mut state = binding
        .state
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    record.sequence = state.next_sequence;
    record.dropped_count = state.dropped_count;
    state.next_sequence = state.next_sequence.saturating_add(1);

    let queue_capacity = state.queue_capacity;
    let overflow_policy = state.overflow_policy;
    let mut overflow_error_pending = state.overflow_error_pending;
    let mut dropped_count = state.dropped_count;
    core::push_with_overflow(
        &mut state.pending,
        queue_capacity,
        overflow_policy,
        &mut overflow_error_pending,
        &mut dropped_count,
        record,
    );
    state.overflow_error_pending = overflow_error_pending;
    state.dropped_count = dropped_count;
    drop(state);
    binding.signal.notify_all();
}

/// Publish one monitor event to all live subscribers.
pub(super) fn publish_monitor_event(
    runtime_state: &Arc<core::X11RuntimeState>,
    record: DisplayEventRecord,
) {
    let subscribers = {
        let mut registry = runtime_state
            .monitor_event_registry
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let mut subscribers = Vec::new();
        registry.retain(|weak| {
            let Some(strong) = weak.upgrade() else {
                return false;
            };
            subscribers.push(strong);
            true
        });
        subscribers
    };

    // iterate this sequence
    for binding in subscribers {
        push_monitor_record(&binding, record.clone());
    }
}

/// Publish one window event to all live subscribers.
pub(super) fn publish_window_event(
    runtime_state: &Arc<core::X11RuntimeState>,
    record: WindowEventRecord,
) {
    let subscribers = {
        let mut registry = runtime_state
            .window_event_registry
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let mut subscribers = Vec::new();
        registry.retain(|weak| {
            let Some(strong) = weak.upgrade() else {
                return false;
            };
            subscribers.push(strong);
            true
        });
        subscribers
    };

    // iterate this sequence
    for binding in subscribers {
        push_window_record(&binding, record.clone());
    }
}
/// Enforce owner-thread affinity for one window-event stream.
pub(super) fn ensure_window_event_thread(
    binding: &Arc<WindowEventBinding>,
    operation: &'static str,
) -> RuntimeResult<()> {
    let current_thread_id = std::thread::current().id();
    // evaluate this condition
    if binding.owner_thread_id == current_thread_id {
        return Ok(());
    }

    Err(core_platform::invalid_argument(
        "handle",
        format!("{operation} must run on the owner thread of this window event stream"),
    ))
}

/// Build one bounded wait duration from one timeout budget and one wait-slice limit.
pub(super) fn wait_duration(remaining_ns: u64, wait_slice_ns: u64) -> Duration {
    Duration::from_nanos(remaining_ns.min(wait_slice_ns))
}
