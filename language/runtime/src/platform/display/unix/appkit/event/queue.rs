use std::sync::Arc;

use crate::platform::display::DisplayEventOverflowPolicy;
use crate::runtime::RuntimeEventLog;

use super::{DisplayEventRecord, MonitorEventStream, WindowEventRecord, WindowEventStream};
use crate::platform::display::unix::appkit::core::AppKitRuntimeState;

/// Publish one monitor event into the runtime-owned live log.
pub(crate) fn publish_monitor_event(
    runtime_state: &Arc<AppKitRuntimeState>,
    mut record: DisplayEventRecord,
) {
    // append one live record to the backend event source
    {
        let mut monitor_events = runtime_state
            .monitor_events
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        record.sequence = monitor_events.next_sequence;
        record.dropped_count = 0;
        monitor_events.next_sequence = monitor_events.next_sequence.saturating_add(1);
        monitor_events.records.push_back(record.clone());
    }

    let streams = runtime_state.monitor_streams_snapshot();

    // update per-stream frontier state for this new live record
    for stream in &streams {
        note_monitor_publication(runtime_state, stream, &record);
    }

    prune_monitor_events(runtime_state, &streams);
    runtime_state.monitor_event_signal.notify_all();
}

/// Push one seeded monitor record into one stream state.
pub(crate) fn push_seeded_monitor_record(
    stream: &Arc<MonitorEventStream>,
    record: DisplayEventRecord,
) {
    // skip seeded records filtered out for this stream
    if !stream.filter.matches(&record) {
        return;
    }

    let mut state = stream
        .state
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let unread_total = state.seeded.len().saturating_add(state.unread_live_count);

    // append one seeded record while capacity remains
    if unread_total < state.queue_capacity {
        state.seeded.push_back(record);
        return;
    }

    // apply overflow policy against the combined seeded and live unread set
    match state.overflow_policy {
        DisplayEventOverflowPolicy::DropNewest => {
            state.dropped_count = state.dropped_count.saturating_add(1);
        }
        DisplayEventOverflowPolicy::Error => {
            state.overflow_error_pending = true;
            state.dropped_count = state.dropped_count.saturating_add(1);
        }
        DisplayEventOverflowPolicy::DropOldest => {
            state.dropped_count = state.dropped_count.saturating_add(1);

            // prefer dropping earlier seeded records before unread live records
            if state.seeded.pop_front().is_some() {
                state.seeded.push_back(record);
                return;
            }

            state.seeded.push_back(record);
        }
    }
}

/// Publish one window event into the runtime-owned live log.
pub(crate) fn publish_window_event(
    runtime_state: &Arc<AppKitRuntimeState>,
    mut record: WindowEventRecord,
) {
    // append one live record to the backend event source
    {
        let mut window_events = runtime_state
            .window_events
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        record.sequence = window_events.next_sequence;
        record.dropped_count = 0;
        window_events.next_sequence = window_events.next_sequence.saturating_add(1);
        window_events.records.push_back(record.clone());
    }

    let streams = runtime_state.window_streams_snapshot();

    // update per-stream frontier state for this new live record
    for stream in &streams {
        note_window_publication(runtime_state, stream, &record);
    }

    prune_window_events(runtime_state, &streams);
    runtime_state.window_event_signal.notify_all();
}

/// Record one newly published monitor event against one stream frontier.
fn note_monitor_publication(
    runtime_state: &Arc<AppKitRuntimeState>,
    stream: &Arc<MonitorEventStream>,
    record: &DisplayEventRecord,
) {
    // ignore live records filtered out for this stream
    if !stream.filter.matches(record) {
        return;
    }

    let mut state = stream
        .state
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let unread_total = state.seeded.len().saturating_add(state.unread_live_count);

    // append one live record while capacity remains
    if unread_total < state.queue_capacity {
        state.unread_live_count = state.unread_live_count.saturating_add(1);
        return;
    }

    // apply overflow policy against the combined seeded and live unread set
    match state.overflow_policy {
        DisplayEventOverflowPolicy::DropNewest => {
            state.dropped_count = state.dropped_count.saturating_add(1);
        }
        DisplayEventOverflowPolicy::Error => {
            state.overflow_error_pending = true;
            state.dropped_count = state.dropped_count.saturating_add(1);
        }
        DisplayEventOverflowPolicy::DropOldest => {
            state.dropped_count = state.dropped_count.saturating_add(1);

            // drop one seeded record before dropping one unread live record
            if state.seeded.pop_front().is_some() {
                state.unread_live_count = state.unread_live_count.saturating_add(1);
                return;
            }

            drop_oldest_live_monitor_record(runtime_state, stream, &mut state);
        }
    }
}

/// Record one newly published window event against one stream frontier.
fn note_window_publication(
    runtime_state: &Arc<AppKitRuntimeState>,
    stream: &Arc<WindowEventStream>,
    record: &WindowEventRecord,
) {
    // ignore live records filtered out for this stream
    if !stream.filter.matches(record) {
        return;
    }

    let mut state = stream
        .state
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let unread_total = state.seeded.len().saturating_add(state.unread_live_count);

    // append one live record while capacity remains
    if unread_total < state.queue_capacity {
        state.unread_live_count = state.unread_live_count.saturating_add(1);
        return;
    }

    // apply overflow policy against the combined seeded and live unread set
    match state.overflow_policy {
        DisplayEventOverflowPolicy::DropNewest => {
            state.dropped_count = state.dropped_count.saturating_add(1);
        }
        DisplayEventOverflowPolicy::Error => {
            state.overflow_error_pending = true;
            state.dropped_count = state.dropped_count.saturating_add(1);
        }
        DisplayEventOverflowPolicy::DropOldest => {
            state.dropped_count = state.dropped_count.saturating_add(1);

            // drop one seeded record before dropping one unread live record
            if state.seeded.pop_front().is_some() {
                state.unread_live_count = state.unread_live_count.saturating_add(1);
                return;
            }

            drop_oldest_live_window_record(runtime_state, stream, &mut state);
        }
    }
}

/// Drop the oldest unread live monitor record for one stream.
fn drop_oldest_live_monitor_record(
    runtime_state: &Arc<AppKitRuntimeState>,
    stream: &Arc<MonitorEventStream>,
    state: &mut super::MonitorEventState,
) {
    // accept the newest record when this stream currently has no unread live backlog
    if state.unread_live_count == 0 {
        state.unread_live_count = 1;
        return;
    }

    let monitor_events = runtime_state
        .monitor_events
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let mut next_live_sequence = state.next_live_sequence.max(monitor_events.first_sequence);

    // skip records until the first unread matching live record is discarded
    while next_live_sequence < monitor_events.next_sequence {
        let Some(record) = monitor_record_at_sequence(&monitor_events, next_live_sequence) else {
            break;
        };
        next_live_sequence = next_live_sequence.saturating_add(1);

        if stream.filter.matches(record) {
            state.next_live_sequence = next_live_sequence;
            return;
        }
    }

    state.next_live_sequence = monitor_events.next_sequence;
    state.unread_live_count = 1;
}

/// Drop the oldest unread live window record for one stream.
fn drop_oldest_live_window_record(
    runtime_state: &Arc<AppKitRuntimeState>,
    stream: &Arc<WindowEventStream>,
    state: &mut super::WindowEventState,
) {
    // accept the newest record when this stream currently has no unread live backlog
    if state.unread_live_count == 0 {
        state.unread_live_count = 1;
        return;
    }

    let window_events = runtime_state
        .window_events
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let mut next_live_sequence = state.next_live_sequence.max(window_events.first_sequence);

    // skip records until the first unread matching live record is discarded
    while next_live_sequence < window_events.next_sequence {
        let Some(record) = window_record_at_sequence(&window_events, next_live_sequence) else {
            break;
        };
        next_live_sequence = next_live_sequence.saturating_add(1);

        if stream.filter.matches(record) {
            state.next_live_sequence = next_live_sequence;
            return;
        }
    }

    state.next_live_sequence = window_events.next_sequence;
    state.unread_live_count = 1;
}

/// Trim retained monitor live records to the earliest active frontier.
fn prune_monitor_events(
    runtime_state: &Arc<AppKitRuntimeState>,
    streams: &[Arc<MonitorEventStream>],
) {
    let mut min_sequence: Option<u64> = None;

    // compute the earliest live frontier across active streams
    for stream in streams {
        let state = stream
            .state
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let next_live_sequence = state.next_live_sequence;
        min_sequence = Some(match min_sequence {
            Some(current) => current.min(next_live_sequence),
            None => next_live_sequence,
        });
    }

    let mut monitor_events = runtime_state
        .monitor_events
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let target_sequence = min_sequence.unwrap_or(monitor_events.next_sequence);

    // discard live records older than the earliest active frontier
    while monitor_events.first_sequence < target_sequence {
        if monitor_events.records.pop_front().is_none() {
            break;
        }

        monitor_events.first_sequence = monitor_events.first_sequence.saturating_add(1);
    }
}

/// Trim retained window live records to the earliest active frontier.
fn prune_window_events(
    runtime_state: &Arc<AppKitRuntimeState>,
    streams: &[Arc<WindowEventStream>],
) {
    let mut min_sequence: Option<u64> = None;

    // compute the earliest live frontier across active streams
    for stream in streams {
        let state = stream
            .state
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let next_live_sequence = state.next_live_sequence;
        min_sequence = Some(match min_sequence {
            Some(current) => current.min(next_live_sequence),
            None => next_live_sequence,
        });
    }

    let mut window_events = runtime_state
        .window_events
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let target_sequence = min_sequence.unwrap_or(window_events.next_sequence);

    // discard live records older than the earliest active frontier
    while window_events.first_sequence < target_sequence {
        if window_events.records.pop_front().is_none() {
            break;
        }

        window_events.first_sequence = window_events.first_sequence.saturating_add(1);
    }
}

/// Trim retained monitor live records after one stream-set change.
pub(crate) fn trim_monitor_events(runtime_state: &Arc<AppKitRuntimeState>) {
    let streams = runtime_state.monitor_streams_snapshot();
    prune_monitor_events(runtime_state, &streams);
}

/// Trim retained window live records after one stream-set change.
pub(crate) fn trim_window_events(runtime_state: &Arc<AppKitRuntimeState>) {
    let streams = runtime_state.window_streams_snapshot();
    prune_window_events(runtime_state, &streams);
}

/// Resolve one retained monitor record by live sequence.
fn monitor_record_at_sequence(
    state: &RuntimeEventLog<DisplayEventRecord>,
    sequence: u64,
) -> Option<&DisplayEventRecord> {
    if sequence < state.first_sequence || sequence >= state.next_sequence {
        return None;
    }

    let index = usize::try_from(sequence.saturating_sub(state.first_sequence)).ok()?;
    state.records.get(index)
}

/// Resolve one retained window record by live sequence.
fn window_record_at_sequence(
    state: &RuntimeEventLog<WindowEventRecord>,
    sequence: u64,
) -> Option<&WindowEventRecord> {
    if sequence < state.first_sequence || sequence >= state.next_sequence {
        return None;
    }

    let index = usize::try_from(sequence.saturating_sub(state.first_sequence)).ok()?;
    state.records.get(index)
}

/// Deliver one seeded monitor record with stream-local metadata.
pub(crate) fn pop_seeded_monitor_record(
    stream: &Arc<MonitorEventStream>,
) -> Option<DisplayEventRecord> {
    let mut state = stream
        .state
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let mut record = state.seeded.pop_front()?;
    record.sequence = state.next_output_sequence;
    record.dropped_count = state.dropped_count;
    state.next_output_sequence = state.next_output_sequence.saturating_add(1);
    Some(record)
}

/// Deliver one seeded window record with stream-local metadata.
pub(crate) fn pop_seeded_window_record(
    stream: &Arc<WindowEventStream>,
) -> Option<WindowEventRecord> {
    let mut state = stream
        .state
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let mut record = state.seeded.pop_front()?;
    record.sequence = state.next_output_sequence;
    record.dropped_count = state.dropped_count;
    state.next_output_sequence = state.next_output_sequence.saturating_add(1);
    Some(record)
}

/// Pop one live monitor record visible to this stream.
pub(crate) fn pop_live_monitor_record(
    runtime_state: &Arc<AppKitRuntimeState>,
    stream: &Arc<MonitorEventStream>,
) -> Option<DisplayEventRecord> {
    let mut state = stream
        .state
        .lock()
        .unwrap_or_else(|error| error.into_inner());

    if state.unread_live_count == 0 {
        return None;
    }

    let monitor_events = runtime_state
        .monitor_events
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let mut next_live_sequence = state.next_live_sequence.max(monitor_events.first_sequence);

    // scan the retained live log for the next record that matches this stream filter
    while next_live_sequence < monitor_events.next_sequence {
        let Some(record) = monitor_record_at_sequence(&monitor_events, next_live_sequence) else {
            break;
        };
        next_live_sequence = next_live_sequence.saturating_add(1);

        if !stream.filter.matches(record) {
            continue;
        }

        let mut record = record.clone();
        state.next_live_sequence = next_live_sequence;
        state.unread_live_count = state.unread_live_count.saturating_sub(1);
        record.sequence = state.next_output_sequence;
        record.dropped_count = state.dropped_count;
        state.next_output_sequence = state.next_output_sequence.saturating_add(1);
        drop(monitor_events);
        drop(state);

        let streams = runtime_state.monitor_streams_snapshot();
        prune_monitor_events(runtime_state, &streams);
        return Some(record);
    }

    state.next_live_sequence = monitor_events.next_sequence;
    state.unread_live_count = 0;
    None
}

/// Pop one live window record visible to this stream.
pub(crate) fn pop_live_window_record(
    runtime_state: &Arc<AppKitRuntimeState>,
    stream: &Arc<WindowEventStream>,
) -> Option<WindowEventRecord> {
    let mut state = stream
        .state
        .lock()
        .unwrap_or_else(|error| error.into_inner());

    if state.unread_live_count == 0 {
        return None;
    }

    let window_events = runtime_state
        .window_events
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let mut next_live_sequence = state.next_live_sequence.max(window_events.first_sequence);

    // scan the retained live log for the next record that matches this stream filter
    while next_live_sequence < window_events.next_sequence {
        let Some(record) = window_record_at_sequence(&window_events, next_live_sequence) else {
            break;
        };
        next_live_sequence = next_live_sequence.saturating_add(1);

        if !stream.filter.matches(record) {
            continue;
        }

        let mut record = record.clone();
        state.next_live_sequence = next_live_sequence;
        state.unread_live_count = state.unread_live_count.saturating_sub(1);
        record.sequence = state.next_output_sequence;
        record.dropped_count = state.dropped_count;
        state.next_output_sequence = state.next_output_sequence.saturating_add(1);
        drop(window_events);
        drop(state);

        let streams = runtime_state.window_streams_snapshot();
        prune_window_events(runtime_state, &streams);
        return Some(record);
    }

    state.next_live_sequence = window_events.next_sequence;
    state.unread_live_count = 0;
    None
}

/// Return whether this monitor stream has a pending overflow error.
pub(crate) fn take_monitor_overflow_error(stream: &Arc<MonitorEventStream>) -> bool {
    let mut state = stream
        .state
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let pending = state.overflow_error_pending;
    state.overflow_error_pending = false;
    pending
}

/// Return whether this window stream has a pending overflow error.
pub(crate) fn take_window_overflow_error(stream: &Arc<WindowEventStream>) -> bool {
    let mut state = stream
        .state
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let pending = state.overflow_error_pending;
    state.overflow_error_pending = false;
    pending
}
