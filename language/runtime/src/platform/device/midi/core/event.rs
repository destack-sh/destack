use std::collections::BTreeMap;
use std::sync::{Arc, Weak};

use parking_lot::Mutex;

use crate::diagnostic::RuntimeResult;
use crate::platform::core::BoundedQueue;
use crate::platform::device::midi::core::{
    MidiEventMetadataValue, MidiEventValue, MidiPortDescriptorValue,
};
use crate::platform::device::{
    MidiBackend, MidiEventOverflowPolicy, MidiEventSource, MidiPortDirection,
};

use super::{binding_timestamp_now, endpoint_direction_name, push_event_with_overflow_policy};

/// Return one stable snapshot key for one direction and descriptor id.
pub(crate) fn snapshot_key(direction: MidiPortDirection, id: &str) -> String {
    let direction_name = endpoint_direction_name(direction);

    format!("{direction_name}:{id}")
}

/// Push all snapshot delta events from one previous and next topology snapshot.
pub(crate) fn push_snapshot_delta_events<K>(
    queue: &Arc<BoundedQueue<MidiEventValue>>,
    backend: MidiBackend,
    overflow_policy: MidiEventOverflowPolicy,
    next_sequence: &mut u64,
    snapshot: &BTreeMap<K, (MidiPortDirection, MidiPortDescriptorValue)>,
    next_snapshot: &BTreeMap<K, (MidiPortDirection, MidiPortDescriptorValue)>,
    source: MidiEventSource,
) -> RuntimeResult<()>
where
    K: Ord,
{
    // additions and changes
    for (key, (direction, descriptor)) in next_snapshot {
        match snapshot.get(key) {
            None => {
                let metadata = MidiEventMetadataValue {
                    timestamp_ns: binding_timestamp_now(),
                    sequence: *next_sequence,
                    dropped_count: queue.dropped_count(),
                    source,
                    backend,
                };
                *next_sequence = next_sequence.saturating_add(1);

                push_event_with_overflow_policy(
                    queue,
                    MidiEventValue::PortAdded {
                        metadata,
                        direction: *direction,
                        descriptor: descriptor.clone(),
                    },
                    overflow_policy,
                )?;
            }
            Some((_, previous)) if previous != descriptor => {
                let metadata = MidiEventMetadataValue {
                    timestamp_ns: binding_timestamp_now(),
                    sequence: *next_sequence,
                    dropped_count: queue.dropped_count(),
                    source,
                    backend,
                };
                *next_sequence = next_sequence.saturating_add(1);

                push_event_with_overflow_policy(
                    queue,
                    MidiEventValue::PortChanged {
                        metadata,
                        direction: *direction,
                        descriptor: descriptor.clone(),
                    },
                    overflow_policy,
                )?;
            }
            Some(_) => {}
        }
    }

    // removals
    for (key, (direction, descriptor)) in snapshot {
        if next_snapshot.contains_key(key) {
            continue;
        }

        let metadata = MidiEventMetadataValue {
            timestamp_ns: binding_timestamp_now(),
            sequence: *next_sequence,
            dropped_count: queue.dropped_count(),
            source,
            backend,
        };
        *next_sequence = next_sequence.saturating_add(1);

        push_event_with_overflow_policy(
            queue,
            MidiEventValue::PortRemoved {
                metadata,
                direction: *direction,
                id: descriptor.id.clone(),
                group_id: descriptor.group_id.clone(),
            },
            overflow_policy,
        )?;
    }

    Ok(())
}

/// Refresh one stored topology snapshot and queue all delta events.
pub(crate) fn refresh_snapshot_event_subscription<K>(
    queue: &Arc<BoundedQueue<MidiEventValue>>,
    backend: MidiBackend,
    overflow_policy: MidiEventOverflowPolicy,
    next_sequence: &mut u64,
    snapshot: &mut BTreeMap<K, (MidiPortDirection, MidiPortDescriptorValue)>,
    next_snapshot: BTreeMap<K, (MidiPortDirection, MidiPortDescriptorValue)>,
    source: MidiEventSource,
) -> RuntimeResult<()>
where
    K: Ord,
{
    push_snapshot_delta_events(
        queue,
        backend,
        overflow_policy,
        next_sequence,
        snapshot,
        &next_snapshot,
        source,
    )?;

    *snapshot = next_snapshot;
    Ok(())
}

/// Push one backend-disconnected event into one queue.
pub(crate) fn push_backend_disconnected_event(
    queue: &Arc<BoundedQueue<MidiEventValue>>,
    backend: MidiBackend,
    overflow_policy: MidiEventOverflowPolicy,
    next_sequence: &mut u64,
    source: MidiEventSource,
    flags: u32,
) -> RuntimeResult<()> {
    let metadata = MidiEventMetadataValue {
        timestamp_ns: binding_timestamp_now(),
        sequence: *next_sequence,
        dropped_count: queue.dropped_count(),
        source,
        backend,
    };
    *next_sequence = next_sequence.saturating_add(1);

    push_event_with_overflow_policy(
        queue,
        MidiEventValue::BackendDisconnected { metadata, flags },
        overflow_policy,
    )
}

/// Collect all live weakly held event sessions and prune stale ids.
pub(crate) fn collect_live_event_sessions<S>(
    sessions: &mut BTreeMap<u64, Weak<Mutex<S>>>,
) -> Vec<Arc<Mutex<S>>> {
    let mut live_sessions = Vec::with_capacity(sessions.len());
    let mut stale_ids = Vec::new();

    for (registration_id, session) in sessions.iter() {
        if let Some(session) = session.upgrade() {
            live_sessions.push(session);
        } else {
            stale_ids.push(*registration_id);
        }
    }

    for registration_id in stale_ids {
        sessions.remove(&registration_id);
    }

    live_sessions
}
