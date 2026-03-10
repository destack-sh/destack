use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use parking_lot::Mutex;

use crate::diagnostic::RuntimeResult;
use crate::platform::core::{self as core_platform};
use crate::platform::midi::core::{
    MidiEventMetadataValue, MidiEventValue, MidiPortDescriptorValue,
};
use crate::platform::midi::shared::remove_labeled_resource;
use crate::platform::midi::{
    MidiEventDeliveryMode, MidiEventSource, MidiEventSubscriptionFlags,
    MidiEventSubscriptionOptions, MidiPortDirection, MidiPortDirectionFlags,
};
use crate::platform::resource;
use crate::runtime::BindingCallContext;

use super::abi::{
    K_MIDI_MSG_IO_ERROR, K_MIDI_MSG_OBJECT_ADDED, K_MIDI_MSG_OBJECT_REMOVED,
    K_MIDI_MSG_PROPERTY_CHANGED, K_MIDI_MSG_SETUP_CHANGED, MIDINotification,
    MIDIObjectAddRemoveNotification, MIDIObjectPropertyChangeNotification,
};
use super::backend::resolve_backend;
use super::core::{
    CoreMidiEventDeliveryKind, CoreMidiEventSession, SharedQueue, SnapshotKey,
    binding_timestamp_now, direction_mask_includes, endpoint_direction_name, event_poll_interval,
    event_queue_capacity, event_snapshot_list_flags, insert_event_resource,
};
use super::descriptor::filtered_descriptors;
use super::resource::event_resource;
use super::service::{
    CoreMidiNativeEventRegistry, core_midi_service, register_native_event_session,
    unregister_native_event_session,
};

/// Return one stable snapshot key for one direction and descriptor id.
fn snapshot_key(direction: MidiPortDirection, id: &str) -> SnapshotKey {
    let direction_name = endpoint_direction_name(direction);

    format!("{direction_name}:{id}")
}

/// Build one current snapshot for one event subscription.
fn event_snapshot(
    flags: MidiEventSubscriptionFlags,
    direction_mask: MidiPortDirectionFlags,
) -> BTreeMap<SnapshotKey, (MidiPortDirection, MidiPortDescriptorValue)> {
    let list_flags = event_snapshot_list_flags(flags);
    let mut snapshot = BTreeMap::new();

    if direction_mask_includes(direction_mask, MidiPortDirection::Input) {
        for descriptor in filtered_descriptors(MidiPortDirection::Input, list_flags) {
            let key = snapshot_key(MidiPortDirection::Input, &descriptor.id);

            snapshot.insert(key, (MidiPortDirection::Input, descriptor));
        }
    }

    if direction_mask_includes(direction_mask, MidiPortDirection::Output) {
        for descriptor in filtered_descriptors(MidiPortDirection::Output, list_flags) {
            let key = snapshot_key(MidiPortDirection::Output, &descriptor.id);

            snapshot.insert(key, (MidiPortDirection::Output, descriptor));
        }
    }

    snapshot
}

/// Refresh one event subscription queue from the current snapshot.
fn refresh_event_subscription(
    session: &mut CoreMidiEventSession,
    source: MidiEventSource,
) -> RuntimeResult<()> {
    let next_snapshot = event_snapshot(session.flags, session.direction_mask);

    for (key, (direction, descriptor)) in &next_snapshot {
        match session.snapshot.get(key) {
            None => {
                let metadata = MidiEventMetadataValue {
                    timestamp_ns: binding_timestamp_now(),
                    sequence: session.next_sequence,
                    dropped_count: session.queue.dropped_count(),
                    source,
                    backend: session.backend,
                };
                session.next_sequence = session.next_sequence.saturating_add(1);

                session.queue.push_with_overflow_policy(
                    MidiEventValue::PortAdded {
                        metadata,
                        direction: *direction,
                        descriptor: descriptor.clone(),
                    },
                    session.overflow_policy,
                )?;
            }
            Some((_, previous)) if previous != descriptor => {
                let metadata = MidiEventMetadataValue {
                    timestamp_ns: binding_timestamp_now(),
                    sequence: session.next_sequence,
                    dropped_count: session.queue.dropped_count(),
                    source,
                    backend: session.backend,
                };
                session.next_sequence = session.next_sequence.saturating_add(1);

                session.queue.push_with_overflow_policy(
                    MidiEventValue::PortChanged {
                        metadata,
                        direction: *direction,
                        descriptor: descriptor.clone(),
                    },
                    session.overflow_policy,
                )?;
            }
            Some(_) => {}
        }
    }

    for (key, (direction, descriptor)) in &session.snapshot {
        if next_snapshot.contains_key(key) {
            continue;
        }

        let metadata = MidiEventMetadataValue {
            timestamp_ns: binding_timestamp_now(),
            sequence: session.next_sequence,
            dropped_count: session.queue.dropped_count(),
            source,
            backend: session.backend,
        };
        session.next_sequence = session.next_sequence.saturating_add(1);

        session.queue.push_with_overflow_policy(
            MidiEventValue::PortRemoved {
                metadata,
                direction: *direction,
                id: descriptor.id.clone(),
                group_id: descriptor.group_id.clone(),
            },
            session.overflow_policy,
        )?;
    }

    session.snapshot = next_snapshot;
    Ok(())
}

/// Refresh one event subscription from synthetic polling.
fn refresh_poll_event_subscription(session: &mut CoreMidiEventSession) -> RuntimeResult<()> {
    refresh_event_subscription(session, MidiEventSource::SyntheticPoll)
}

/// Queue one backend-disconnected event into one subscription.
fn queue_backend_disconnected_event(
    session: &mut CoreMidiEventSession,
    source: MidiEventSource,
    flags: u32,
) -> RuntimeResult<()> {
    let metadata = MidiEventMetadataValue {
        timestamp_ns: binding_timestamp_now(),
        sequence: session.next_sequence,
        dropped_count: session.queue.dropped_count(),
        source,
        backend: session.backend,
    };
    session.next_sequence = session.next_sequence.saturating_add(1);

    session.queue.push_with_overflow_policy(
        MidiEventValue::BackendDisconnected { metadata, flags },
        session.overflow_policy,
    )
}

/// Pop one queued event after honoring deferred overflow errors.
fn try_pop_session_event(
    session: &CoreMidiEventSession,
    operation: &'static str,
) -> RuntimeResult<Option<MidiEventValue>> {
    // surface deferred native overflow before returning more events
    session.queue.take_overflow_error(operation)?;

    Ok(session.queue.try_pop())
}

/// Pop one queued event batch after honoring deferred overflow errors.
fn try_pop_session_event_batch(
    session: &CoreMidiEventSession,
    max_events: usize,
    operation: &'static str,
) -> RuntimeResult<Vec<MidiEventValue>> {
    // surface deferred native overflow before returning more events
    session.queue.take_overflow_error(operation)?;

    Ok(session.queue.try_pop_batch(max_events))
}

/// Collect all live native event subscriptions from one registry.
fn collect_native_event_sessions(
    registry: &Arc<Mutex<CoreMidiNativeEventRegistry>>,
) -> Vec<Arc<Mutex<CoreMidiEventSession>>> {
    let sessions = {
        let mut registry = registry.lock();
        let mut sessions = Vec::with_capacity(registry.sessions.len());
        let mut stale_ids = Vec::new();

        for (registration_id, session) in &registry.sessions {
            if let Some(session) = session.upgrade() {
                sessions.push(session);
            } else {
                stale_ids.push(*registration_id);
            }
        }

        for registration_id in stale_ids {
            registry.sessions.remove(&registration_id);
        }

        sessions
    };

    sessions
}

/// Refresh all native event subscriptions from one backend topology mutation.
pub(super) fn refresh_native_event_sessions(
    registry: &Arc<Mutex<CoreMidiNativeEventRegistry>>,
    source: MidiEventSource,
) {
    let sessions = collect_native_event_sessions(registry);

    for session in sessions {
        let mut session = session.lock();
        let _ = refresh_event_subscription(&mut session, source);
    }
}

/// Queue one backend-disconnected event for all native subscriptions.
fn queue_backend_disconnected_events(
    registry: &Arc<Mutex<CoreMidiNativeEventRegistry>>,
    source: MidiEventSource,
    flags: u32,
) {
    let sessions = collect_native_event_sessions(registry);

    for session in sessions {
        let mut session = session.lock();
        let _ = queue_backend_disconnected_event(&mut session, source, flags);
    }
}

/// Dispatch one native CoreMIDI notification to registered subscriptions.
pub(super) fn dispatch_native_notification(
    notification: *const MIDINotification,
    registry: &Arc<Mutex<CoreMidiNativeEventRegistry>>,
) {
    if notification.is_null() {
        return;
    }

    let notification = unsafe { &*notification };

    match notification.message_id {
        K_MIDI_MSG_SETUP_CHANGED => {}
        K_MIDI_MSG_OBJECT_ADDED | K_MIDI_MSG_OBJECT_REMOVED => {
            let _ = unsafe {
                &*(notification as *const MIDINotification
                    as *const MIDIObjectAddRemoveNotification)
            };
        }
        K_MIDI_MSG_PROPERTY_CHANGED => {
            let _ = unsafe {
                &*(notification as *const MIDINotification
                    as *const MIDIObjectPropertyChangeNotification)
            };
        }
        K_MIDI_MSG_IO_ERROR => {
            queue_backend_disconnected_events(registry, MidiEventSource::Native, 0);
            return;
        }
        _ => return,
    }

    refresh_native_event_sessions(registry, MidiEventSource::Native);
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::sync::Arc;
    use std::time::Duration;

    use super::super::core::{CoreMidiEventDeliveryKind, CoreMidiEventSession, SharedQueue};
    use super::queue_backend_disconnected_event;
    use crate::platform::midi::core::MidiEventValue;
    use crate::platform::midi::{
        MIDI_EVENT_SUBSCRIPTION_INCLUDE_DISCONNECTED, MIDI_PORT_DIRECTION_FLAG_INPUT, MidiBackend,
        MidiEventOverflowPolicy, MidiEventSource, MidiEventSubscriptionFlags,
        MidiPortDirectionFlags,
    };

    /// Queue one backend-disconnected event with the active backend metadata.
    #[test]
    fn test_queue_backend_disconnected_event_pushes_backend_event() {
        let queue = Arc::new(SharedQueue::new(4));
        let mut session = CoreMidiEventSession {
            backend: MidiBackend::CoreMIDI,
            direction_mask: MidiPortDirectionFlags(MIDI_PORT_DIRECTION_FLAG_INPUT.0),
            flags: MidiEventSubscriptionFlags(MIDI_EVENT_SUBSCRIPTION_INCLUDE_DISCONNECTED.0),
            overflow_policy: MidiEventOverflowPolicy::DropOldest,
            poll_interval: Duration::from_millis(1),
            delivery_kind: CoreMidiEventDeliveryKind::Poll,
            queue: queue.clone(),
            next_sequence: 1,
            snapshot: BTreeMap::new(),
        };

        queue_backend_disconnected_event(&mut session, MidiEventSource::Native, 7)
            .expect("backend disconnected event should queue successfully");

        let event = queue
            .try_pop()
            .expect("event queue should contain one event");
        match event {
            MidiEventValue::BackendDisconnected { metadata, flags } => {
                assert_eq!(metadata.backend, MidiBackend::CoreMIDI);
                assert_eq!(metadata.source, MidiEventSource::Native);
                assert_eq!(flags, 7);
            }
            other => panic!("expected backendDisconnected event, got {other:?}"),
        }
    }
}

/// Open one CoreMIDI event subscription.
pub(crate) fn midi_event_open(
    binding: &BindingCallContext,
    options: MidiEventSubscriptionOptions,
) -> RuntimeResult<resource::MidiEventHandle> {
    let _runtime_state = binding.agent().platform_state.midi.runtime_state(binding);

    let backend = resolve_backend(
        options.backend,
        options.backend_policy,
        "destack.midi.event.open",
    )?;
    if options.direction_mask.0 == 0 {
        return Err(core_platform::invalid_argument(
            "directionMask",
            "directionMask must not be zero",
        ));
    }
    let service = core_midi_service("destack.midi.event.open")?;
    let delivery_kind = match options.delivery_mode {
        MidiEventDeliveryMode::PollOnly => CoreMidiEventDeliveryKind::Poll,
        MidiEventDeliveryMode::Auto | MidiEventDeliveryMode::NativeOnly => {
            CoreMidiEventDeliveryKind::Poll
        }
    };

    let session = Arc::new(Mutex::new(CoreMidiEventSession {
        backend,
        direction_mask: options.direction_mask,
        flags: options.flags,
        overflow_policy: options.overflow_policy,
        poll_interval: event_poll_interval(options.poll_interval_ns),
        delivery_kind,
        queue: Arc::new(SharedQueue::new(event_queue_capacity(
            options.queue_capacity,
        ))),
        next_sequence: 1,
        snapshot: event_snapshot(options.flags, options.direction_mask),
    }));

    if options.delivery_mode != MidiEventDeliveryMode::PollOnly {
        let delivery_kind = register_native_event_session(&service, &session);
        session.lock().delivery_kind = delivery_kind;
    }

    Ok(insert_event_resource(binding, session))
}

/// Close one CoreMIDI event subscription.
pub(crate) fn midi_event_close(
    binding: &BindingCallContext,
    handle: resource::MidiEventHandle,
) -> RuntimeResult<()> {
    if let Ok(session) = event_resource(binding, handle, "destack.midi.event.close") {
        let session = session.lock();
        unregister_native_event_session(&session.delivery_kind);
    }

    remove_labeled_resource(binding, handle.0, "destack.midi.event.close", "midi event")?;

    Ok(())
}

/// Wait for one CoreMIDI event.
pub(crate) fn midi_event_read(
    binding: &BindingCallContext,
    handle: resource::MidiEventHandle,
    timeout_ns: u64,
) -> RuntimeResult<MidiEventValue> {
    let session = event_resource(binding, handle, "destack.midi.event.read")?;

    // native feeds block directly on the shared queue
    let is_native = {
        let session = session.lock();
        matches!(
            session.delivery_kind,
            CoreMidiEventDeliveryKind::Native { .. }
        )
    };
    let timeout = Duration::from_nanos(timeout_ns);

    if is_native {
        let session = session.lock();
        session
            .queue
            .take_overflow_error("destack.midi.event.read")?;

        return match session.queue.pop_with_timeout(timeout) {
            Some(event) => Ok(event),
            None => Err(core_platform::io_would_block(
                "destack.midi.event.read",
                "midi event queue is empty",
            )),
        };
    }

    let deadline = Instant::now() + timeout;

    loop {
        // return any queued event before polling again
        if let Some(event) = {
            let session = session.lock();
            try_pop_session_event(&session, "destack.midi.event.read")?
        } {
            return Ok(event);
        }

        // refresh the synthetic snapshot and consume any new queued event
        {
            let mut session = session.lock();
            refresh_poll_event_subscription(&mut session)?;

            if let Some(event) = try_pop_session_event(&session, "destack.midi.event.read")? {
                return Ok(event);
            }
        }

        // stop once the caller timeout expires
        let now = Instant::now();
        if now >= deadline {
            return Err(core_platform::io_would_block(
                "destack.midi.event.read",
                "midi event queue is empty",
            ));
        }

        // otherwise sleep for the requested poll cadence
        let sleep_duration = {
            let session = session.lock();
            session.poll_interval.min(deadline - now)
        };
        std::thread::sleep(sleep_duration);
    }
}

/// Wait for one CoreMIDI event batch.
pub(crate) fn midi_event_read_batch(
    binding: &BindingCallContext,
    handle: resource::MidiEventHandle,
    max_events: u32,
    timeout_ns: u64,
) -> RuntimeResult<Vec<MidiEventValue>> {
    let session = event_resource(binding, handle, "destack.midi.event.readBatch")?;

    // native feeds block directly on the shared queue
    let is_native = {
        let session = session.lock();
        matches!(
            session.delivery_kind,
            CoreMidiEventDeliveryKind::Native { .. }
        )
    };
    let timeout = Duration::from_nanos(timeout_ns);

    if is_native {
        let session = session.lock();
        session
            .queue
            .take_overflow_error("destack.midi.event.readBatch")?;
        let batch = session
            .queue
            .pop_batch_with_timeout(max_events.max(1) as usize, timeout);

        if batch.is_empty() {
            return Err(core_platform::io_would_block(
                "destack.midi.event.readBatch",
                "midi event queue is empty",
            ));
        }

        return Ok(batch);
    }

    let deadline = Instant::now() + timeout;

    loop {
        // return any queued batch before polling again
        let batch = {
            let session = session.lock();
            try_pop_session_event_batch(
                &session,
                max_events.max(1) as usize,
                "destack.midi.event.readBatch",
            )?
        };
        if !batch.is_empty() {
            return Ok(batch);
        }

        // refresh the synthetic snapshot and consume any queued batch
        {
            let mut session = session.lock();
            refresh_poll_event_subscription(&mut session)?;
            let batch = try_pop_session_event_batch(
                &session,
                max_events.max(1) as usize,
                "destack.midi.event.readBatch",
            )?;
            if !batch.is_empty() {
                return Ok(batch);
            }
        }

        // stop once the caller timeout expires
        let now = Instant::now();
        if now >= deadline {
            return Err(core_platform::io_would_block(
                "destack.midi.event.readBatch",
                "midi event queue is empty",
            ));
        }

        // otherwise sleep for the requested poll cadence
        let sleep_duration = {
            let session = session.lock();
            session.poll_interval.min(deadline - now)
        };
        std::thread::sleep(sleep_duration);
    }
}

/// Poll one CoreMIDI event.
pub(crate) fn midi_event_try_read(
    binding: &BindingCallContext,
    handle: resource::MidiEventHandle,
) -> RuntimeResult<MidiEventValue> {
    let session = event_resource(binding, handle, "destack.midi.event.tryRead")?;

    // refresh synthetic subscriptions before peeking the queue
    let mut session = session.lock();
    if matches!(session.delivery_kind, CoreMidiEventDeliveryKind::Poll) {
        refresh_poll_event_subscription(&mut session)?;
    }

    match try_pop_session_event(&session, "destack.midi.event.tryRead")? {
        Some(event) => Ok(event),
        None => Err(core_platform::io_would_block(
            "destack.midi.event.tryRead",
            "midi event queue is empty",
        )),
    }
}

/// Poll one CoreMIDI event batch.
pub(crate) fn midi_event_try_read_batch(
    binding: &BindingCallContext,
    handle: resource::MidiEventHandle,
    max_events: u32,
) -> RuntimeResult<Vec<MidiEventValue>> {
    let session = event_resource(binding, handle, "destack.midi.event.tryReadBatch")?;

    // refresh synthetic subscriptions before peeking the queue
    let mut session = session.lock();
    if matches!(session.delivery_kind, CoreMidiEventDeliveryKind::Poll) {
        refresh_poll_event_subscription(&mut session)?;
    }

    let batch = try_pop_session_event_batch(
        &session,
        max_events.max(1) as usize,
        "destack.midi.event.tryReadBatch",
    )?;

    if batch.is_empty() {
        return Err(core_platform::io_would_block(
            "destack.midi.event.tryReadBatch",
            "midi event queue is empty",
        ));
    }

    Ok(batch)
}
