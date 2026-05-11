use parking_lot::Mutex;

use crate::diagnostic::RuntimeError;
use crate::platform::device::{
    SerialAttachedEvent, SerialDetachedEvent, SerialEvent, SerialOverflowEvent,
    SerialOverflowEventMetadata, SerialPortDescriptor, SerialWatchEvent, SerialWatchEventMetadata,
    SerialWatchOverflowEvent, SerialWatchOverflowEventMetadata,
};
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::resource::ResourceKind;
use crate::platform::{core as core_platform, resource};
use crate::runtime::BindingCallContext;
use crate::runtime::control::queue::BoundedQueue;

/// Resource-table label for one serial session.
pub(super) const SERIAL_PORT_RESOURCE_LABEL: &str = "device.serial.port";

/// Resource-table label for one serial topology watch stream.
pub(super) const SERIAL_WATCH_RESOURCE_LABEL: &str = "device.serial.watch";

/// Monotonic event sequencing for one serial watch queue.
#[derive(Debug, Clone, Copy)]
pub(super) struct SerialWatchEventState {
    /// The next event sequence number to emit.
    pub(super) next_sequence: u64,
    /// Number of dropped events already surfaced to the caller.
    pub(super) reported_dropped_count: u64,
}

/// Build one invalid serial watch-handle error.
pub(super) fn invalid_serial_watch_handle(operation: &'static str) -> Box<RuntimeError> {
    core_platform::io_operation_error(
        operation,
        Some(PlatformErrorCode::InvalidArgumentValue),
        "unknown serial watch handle",
    )
}

/// Resolve one typed serial-watch payload from the resource table.
pub(super) fn serial_watch_payload<T: Clone + 'static>(
    binding: &BindingCallContext,
    handle: resource::SerialWatchHandle,
    operation: &'static str,
) -> Result<T, Box<RuntimeError>> {
    let resolved = binding.worker().resources.with_entry(handle.0, |entry| {
        if entry.kind != ResourceKind::SerialWatch {
            return None;
        }

        entry.payload_cloned::<T>()
    });

    resolved
        .flatten()
        .ok_or_else(|| invalid_serial_watch_handle(operation))
}

/// Close one serial watch resource and run finalization.
pub(super) fn close_serial_watch_resource(
    binding: &BindingCallContext,
    handle: resource::SerialWatchHandle,
    operation: &'static str,
) -> Result<(), Box<RuntimeError>> {
    let kind = binding
        .worker()
        .resources
        .with_entry(handle.0, |entry| entry.kind)
        .ok_or_else(|| invalid_serial_watch_handle(operation))?;
    if kind != ResourceKind::SerialWatch {
        return Err(invalid_serial_watch_handle(operation));
    }

    if !binding.worker().resources.remove_and_finalize(
        binding.world(),
        handle.0,
        Some(binding.engine()),
    ) {
        return Err(invalid_serial_watch_handle(operation));
    }

    Ok(())
}

/// Build one serial-attached watch event.
pub(super) fn attached_watch_event(
    binding: &BindingCallContext,
    state: &Mutex<SerialWatchEventState>,
    port: SerialPortDescriptor,
) -> SerialWatchEvent {
    SerialWatchEvent::SerialAttachedEvent(SerialAttachedEvent {
        kind: binding.store_string("attached"),
        metadata: next_watch_metadata(state, port),
    })
}

/// Build one serial-detached watch event.
pub(super) fn detached_watch_event(
    binding: &BindingCallContext,
    state: &Mutex<SerialWatchEventState>,
    port: SerialPortDescriptor,
) -> SerialWatchEvent {
    SerialWatchEvent::SerialDetachedEvent(SerialDetachedEvent {
        kind: binding.store_string("detached"),
        metadata: next_watch_metadata(state, port),
    })
}

/// Build one serial-watch-overflow event.
pub(super) fn overflow_watch_event(
    binding: &BindingCallContext,
    state: &Mutex<SerialWatchEventState>,
    dropped_count: u64,
) -> SerialWatchEvent {
    let metadata = next_watch_overflow_metadata(state, dropped_count);

    SerialWatchEvent::SerialWatchOverflowEvent(SerialWatchOverflowEvent {
        kind: binding.store_string("overflow"),
        metadata,
    })
}

/// Return one pending serial-watch overflow delta.
pub(super) fn take_watch_overflow_count<T>(
    event_queue: &BoundedQueue<T>,
    state: &Mutex<SerialWatchEventState>,
) -> Option<u64> {
    let dropped_count = event_queue.dropped_count();
    let mut state = state.lock();
    let reported_dropped_count = state.reported_dropped_count;
    if dropped_count <= reported_dropped_count {
        return None;
    }

    let delta = dropped_count - reported_dropped_count;
    state.reported_dropped_count = dropped_count;

    Some(delta)
}

/// Return one fresh serial watch-event metadata payload.
fn next_watch_metadata(
    state: &Mutex<SerialWatchEventState>,
    port: SerialPortDescriptor,
) -> SerialWatchEventMetadata {
    let mut state = state.lock();
    let sequence = state.next_sequence;
    state.next_sequence = state.next_sequence.saturating_add(1);

    SerialWatchEventMetadata {
        timestamp_ns: core_platform::monotonic_now_ns(),
        sequence,
        port,
    }
}

/// Return one fresh serial watch-overflow metadata payload.
fn next_watch_overflow_metadata(
    state: &Mutex<SerialWatchEventState>,
    dropped_count: u64,
) -> SerialWatchOverflowEventMetadata {
    let mut state = state.lock();
    let sequence = state.next_sequence;
    state.next_sequence = state.next_sequence.saturating_add(1);

    SerialWatchOverflowEventMetadata {
        timestamp_ns: core_platform::monotonic_now_ns(),
        sequence,
        dropped_count,
    }
}

/// Build one serial-overflow event.
pub(super) fn overflow_event(
    binding: &BindingCallContext,
    sequence: u64,
    dropped_count: u64,
) -> SerialEvent {
    SerialEvent::SerialOverflowEvent(SerialOverflowEvent {
        kind: binding.store_string("overflow"),
        metadata: SerialOverflowEventMetadata {
            timestamp_ns: core_platform::monotonic_now_ns(),
            sequence,
            dropped_count,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Report one serial-watch overflow delta after queued records are dropped.
    #[test]
    fn test_take_watch_overflow_count_reports_one_delta() {
        let event_queue = BoundedQueue::new(1);
        let event_state = Mutex::new(SerialWatchEventState {
            next_sequence: 1,
            reported_dropped_count: 0,
        });

        event_queue.push_drop_oldest(1u8);
        event_queue.push_drop_oldest(2u8);

        assert_eq!(
            take_watch_overflow_count(&event_queue, &event_state),
            Some(1)
        );

        assert_eq!(take_watch_overflow_count(&event_queue, &event_state), None);
    }
}
