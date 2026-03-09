use std::sync::Arc;

use crate::diagnostic::RuntimeResult;
use crate::runtime::RuntimeEventLog;

use crate::platform::audio::core::{
    AudioEventRecord, AudioEventStream, AudioEventStreamState, AudioRuntimeState, audio_busy,
    stream_accepts_record,
};

/// Allocate one stable stream identifier.
pub(crate) fn next_event_stream_id(runtime_state: &AudioRuntimeState) -> u64 {
    runtime_state.stream_registry.next_stream_id()
}

/// Register one live event stream.
pub(crate) fn register_event_stream(
    runtime_state: &AudioRuntimeState,
    stream: Arc<AudioEventStream>,
) {
    runtime_state
        .stream_registry
        .register(stream.stream_id, stream);
}

/// Unregister one live event stream.
pub(crate) fn unregister_event_stream(
    runtime_state: &AudioRuntimeState,
    stream_id: u64,
) -> Option<Arc<AudioEventStream>> {
    runtime_state.stream_registry.unregister(stream_id)
}

/// Return one snapshot of the live event streams.
pub(crate) fn event_streams_snapshot(
    runtime_state: &AudioRuntimeState,
) -> Vec<Arc<AudioEventStream>> {
    runtime_state.stream_registry.snapshot()
}

/// Resolve one retained record by shared live sequence.
fn event_record_at_sequence(
    event_log: &RuntimeEventLog<AudioEventRecord>,
    sequence: u64,
) -> Option<&AudioEventRecord> {
    if sequence < event_log.first_sequence || sequence >= event_log.next_sequence {
        return None;
    }

    let index = usize::try_from(sequence.saturating_sub(event_log.first_sequence)).ok()?;
    event_log.records.get(index)
}

/// Drop the oldest unread live record for one stream.
fn drop_oldest_live_event_record(
    runtime_state: &AudioRuntimeState,
    stream: &Arc<AudioEventStream>,
    state: &mut AudioEventStreamState,
) {
    if state.unread_live_count == 0 {
        state.unread_live_count = 1;
        return;
    }

    let event_log = runtime_state
        .event_log
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let mut next_live_sequence = state.next_live_sequence.max(event_log.first_sequence);

    // drop the first unread record that matches this stream
    while next_live_sequence < event_log.next_sequence {
        let Some(record) = event_record_at_sequence(&event_log, next_live_sequence) else {
            break;
        };
        next_live_sequence = next_live_sequence.saturating_add(1);

        if stream_accepts_record(stream, record) {
            state.next_live_sequence = next_live_sequence;
            return;
        }
    }

    state.next_live_sequence = event_log.next_sequence;
    state.unread_live_count = 1;
}

/// Record one live publication against one stream frontier.
fn note_live_publication(
    runtime_state: &AudioRuntimeState,
    stream: &Arc<AudioEventStream>,
    record: &AudioEventRecord,
) {
    if !stream_accepts_record(stream, record) {
        return;
    }

    let mut state = stream
        .state
        .lock()
        .unwrap_or_else(|error| error.into_inner());

    if state.unread_live_count < state.queue_capacity {
        state.unread_live_count = state.unread_live_count.saturating_add(1);
        return;
    }

    match state.overflow_policy {
        crate::platform::audio::AudioEventOverflowPolicy::DropNewest => {
            state.dropped_count = state.dropped_count.saturating_add(1);
        }
        crate::platform::audio::AudioEventOverflowPolicy::Error => {
            state.overflow_error_pending = true;
            state.dropped_count = state.dropped_count.saturating_add(1);
        }
        crate::platform::audio::AudioEventOverflowPolicy::DropOldest => {
            state.dropped_count = state.dropped_count.saturating_add(1);
            drop_oldest_live_event_record(runtime_state, stream, &mut state);
        }
    }
}

/// Trim retained live records after one publication or stream-set change.
pub(crate) fn trim_event_log(runtime_state: &AudioRuntimeState) {
    let streams = event_streams_snapshot(runtime_state);
    let mut event_log = runtime_state
        .event_log
        .lock()
        .unwrap_or_else(|error| error.into_inner());

    if streams.is_empty() {
        event_log.records.clear();
        event_log.first_sequence = event_log.next_sequence;
        return;
    }

    let mut minimum_live_sequence = event_log.next_sequence;
    for stream in &streams {
        let state = stream
            .state
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        minimum_live_sequence = minimum_live_sequence.min(state.next_live_sequence);
    }

    while event_log.first_sequence < minimum_live_sequence {
        if event_log.records.pop_front().is_none() {
            break;
        }

        event_log.first_sequence = event_log.first_sequence.saturating_add(1);
    }
}

/// Publish one shared live record.
pub(crate) fn publish_event_record(
    runtime_state: &AudioRuntimeState,
    mut record: AudioEventRecord,
) {
    {
        let mut event_log = runtime_state
            .event_log
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        record.sequence = event_log.next_sequence;
        record.dropped_count = 0;
        event_log.next_sequence = event_log.next_sequence.saturating_add(1);
        event_log.records.push_back(record.clone());
    }

    let streams = event_streams_snapshot(runtime_state);

    for stream in &streams {
        note_live_publication(runtime_state, stream, &record);
    }

    trim_event_log(runtime_state);
    runtime_state.event_signal.notify_all();
}

/// Return whether one stream has a pending overflow error.
fn take_stream_overflow_error(stream: &Arc<AudioEventStream>) -> bool {
    let mut state = stream
        .state
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let pending = state.overflow_error_pending;
    state.overflow_error_pending = false;
    pending
}

/// Pop one live record visible to this stream.
fn pop_live_event_record(
    runtime_state: &AudioRuntimeState,
    stream: &Arc<AudioEventStream>,
) -> Option<AudioEventRecord> {
    let mut state = stream
        .state
        .lock()
        .unwrap_or_else(|error| error.into_inner());

    if state.unread_live_count == 0 {
        return None;
    }

    let event_log = runtime_state
        .event_log
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let mut next_live_sequence = state.next_live_sequence.max(event_log.first_sequence);

    // scan the retained live log until the next visible record appears
    while next_live_sequence < event_log.next_sequence {
        let Some(record) = event_record_at_sequence(&event_log, next_live_sequence) else {
            break;
        };
        next_live_sequence = next_live_sequence.saturating_add(1);

        if !stream_accepts_record(stream, record) {
            continue;
        }

        let mut record = record.clone();
        state.next_live_sequence = next_live_sequence;
        state.unread_live_count = state.unread_live_count.saturating_sub(1);
        record.sequence = state.next_output_sequence;
        record.dropped_count = state.dropped_count;
        state.next_output_sequence = state.next_output_sequence.saturating_add(1);
        drop(event_log);
        drop(state);

        trim_event_log(runtime_state);
        return Some(record);
    }

    state.next_live_sequence = event_log.next_sequence;
    state.unread_live_count = 0;
    None
}

/// Return one pending event for this stream.
pub(crate) fn next_audio_event(
    runtime_state: &AudioRuntimeState,
    stream: &Arc<AudioEventStream>,
    operation: &'static str,
) -> RuntimeResult<Option<AudioEventRecord>> {
    if take_stream_overflow_error(stream) {
        return Err(audio_busy(
            operation,
            "event queue overflowed with overflow policy error",
        ));
    }

    Ok(pop_live_event_record(runtime_state, stream))
}
