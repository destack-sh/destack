use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Instant;

use parking_lot::Mutex;

use crate::diagnostic::RuntimeResult;
use crate::platform::core::{self as core_platform};
use crate::platform::midi::core::{
    MidiEventValue, MidiPortDescriptorValue, collect_live_event_sessions, direction_mask_includes,
    event_queue_capacity, event_snapshot_list_flags,
    push_backend_disconnected_event as push_backend_disconnected_midi_event,
    refresh_snapshot_event_subscription, remove_midi_event_resource, require_queued_event,
    require_queued_event_batch, snapshot_key, try_pop_queued_event, try_pop_queued_event_batch,
};
use crate::platform::midi::{
    MidiBackend, MidiEventDeliveryMode, MidiEventSource, MidiEventSubscriptionFlags,
    MidiEventSubscriptionOptions, MidiPortDirection, MidiPortDirectionFlags,
};
use crate::platform::resource;
use crate::runtime::BindingCallContext;
use crate::runtime::control::queue::BoundedQueue;

use super::core::{
    JackEventDeliveryKind, JackEventSession, JackSnapshotKey, insert_event_resource,
};
use super::descriptor::filtered_descriptors;
use super::resource::event_resource;
use super::service::{
    JackNativeEventRegistry, JackService, register_native_event_session,
    unregister_native_event_session,
};

/// Build one current snapshot for one event subscription.
fn event_snapshot(
    service: &Arc<JackService>,
    flags: MidiEventSubscriptionFlags,
    direction_mask: MidiPortDirectionFlags,
) -> BTreeMap<JackSnapshotKey, (MidiPortDirection, MidiPortDescriptorValue)> {
    let list_flags = event_snapshot_list_flags(flags);
    let mut snapshot = BTreeMap::new();

    // input rows
    if direction_mask_includes(direction_mask, MidiPortDirection::Input) {
        for descriptor in filtered_descriptors(service, MidiPortDirection::Input, list_flags) {
            let key = snapshot_key(MidiPortDirection::Input, &descriptor.id);
            snapshot.insert(key, (MidiPortDirection::Input, descriptor));
        }
    }

    // output rows
    if direction_mask_includes(direction_mask, MidiPortDirection::Output) {
        for descriptor in filtered_descriptors(service, MidiPortDirection::Output, list_flags) {
            let key = snapshot_key(MidiPortDirection::Output, &descriptor.id);
            snapshot.insert(key, (MidiPortDirection::Output, descriptor));
        }
    }

    snapshot
}

/// Refresh one event subscription queue from the current snapshot.
fn refresh_event_subscription(
    service: &Arc<JackService>,
    session: &mut JackEventSession,
    source: MidiEventSource,
) -> RuntimeResult<()> {
    let next_snapshot = event_snapshot(service, session.flags, session.direction_mask);
    refresh_snapshot_event_subscription(
        &session.queue,
        session.backend,
        session.overflow_policy,
        &mut session.next_sequence,
        &mut session.snapshot,
        next_snapshot,
        source,
    )
}

/// Queue one backend-disconnected event into one subscription.
fn queue_backend_disconnected_event(
    session: &mut JackEventSession,
    source: MidiEventSource,
    flags: u32,
) -> RuntimeResult<()> {
    push_backend_disconnected_midi_event(
        &session.queue,
        session.backend,
        session.overflow_policy,
        &mut session.next_sequence,
        source,
        flags,
    )
}

/// Pop one queued event after honoring deferred overflow errors.
fn try_pop_session_event(
    session: &JackEventSession,
    operation: &'static str,
) -> RuntimeResult<Option<MidiEventValue>> {
    try_pop_queued_event(&session.queue, operation)
}

/// Pop one queued event batch after honoring deferred overflow errors.
fn try_pop_session_event_batch(
    session: &JackEventSession,
    max_events: usize,
    operation: &'static str,
) -> RuntimeResult<Vec<MidiEventValue>> {
    try_pop_queued_event_batch(&session.queue, max_events, operation)
}

/// Collect all live native event subscriptions from one registry.
fn collect_native_event_sessions(
    registry: &Arc<Mutex<JackNativeEventRegistry>>,
) -> Vec<Arc<Mutex<JackEventSession>>> {
    let mut registry = registry.lock();

    collect_live_event_sessions(&mut registry.sessions)
}

/// Refresh all native event subscriptions from the current service topology.
pub(super) fn refresh_native_event_sessions(service: &Arc<JackService>, source: MidiEventSource) {
    let sessions = collect_native_event_sessions(&service.native_event_registry);
    for session in sessions {
        let mut session = session.lock();
        if refresh_event_subscription(service, &mut session, source).is_err() {
            let _ = queue_backend_disconnected_event(&mut session, source, 0);
        }
    }
}

/// Queue one backend-disconnected event for all native subscriptions.
pub(super) fn queue_backend_disconnected_events(
    registry: &Arc<Mutex<JackNativeEventRegistry>>,
    source: MidiEventSource,
    flags: u32,
) {
    let sessions = collect_native_event_sessions(registry);
    for session in sessions {
        let mut session = session.lock();
        let _ = queue_backend_disconnected_event(&mut session, source, flags);
    }
}

/// Open one JACK event subscription.
pub(crate) fn midi_event_open(
    binding: &BindingCallContext,
    options: MidiEventSubscriptionOptions,
) -> RuntimeResult<resource::MidiEventHandle> {
    // direction validation
    if options.direction_mask.0 == 0 {
        return Err(core_platform::invalid_argument(
            "directionMask",
            "destack.midi.event.open: directionMask must include at least one direction",
        ));
    }

    let service = binding
        .agent()
        .platform_state
        .midi
        .jack_service("destack.midi.event.open")?;

    // delivery kind
    let delivery_kind = match options.delivery_mode {
        MidiEventDeliveryMode::NativeOnly | MidiEventDeliveryMode::Auto => {
            let registry_session = Arc::new(Mutex::new(JackEventSession {
                backend: MidiBackend::JackMidi,
                direction_mask: options.direction_mask,
                flags: options.flags,
                overflow_policy: options.overflow_policy,
                delivery_kind: JackEventDeliveryKind::Poll,
                queue: Arc::new(BoundedQueue::new(event_queue_capacity(
                    options.queue_capacity,
                ))),
                next_sequence: 0,
                snapshot: event_snapshot(&service, options.flags, options.direction_mask),
            }));

            let delivery_kind = register_native_event_session(&service, &registry_session);
            registry_session.lock().delivery_kind = delivery_kind.clone();

            return Ok(insert_event_resource(binding, registry_session));
        }
        MidiEventDeliveryMode::PollOnly => JackEventDeliveryKind::Poll,
    };

    let session = Arc::new(Mutex::new(JackEventSession {
        backend: MidiBackend::JackMidi,
        direction_mask: options.direction_mask,
        flags: options.flags,
        overflow_policy: options.overflow_policy,
        delivery_kind,
        queue: Arc::new(BoundedQueue::new(event_queue_capacity(
            options.queue_capacity,
        ))),
        next_sequence: 0,
        snapshot: event_snapshot(&service, options.flags, options.direction_mask),
    }));

    Ok(insert_event_resource(binding, session))
}

/// Close one JACK event subscription.
pub(crate) fn midi_event_close(
    binding: &BindingCallContext,
    handle: resource::MidiEventHandle,
) -> RuntimeResult<()> {
    let session = event_resource(binding, handle, "destack.midi.event.close")?;
    let delivery_kind = session.lock().delivery_kind.clone();
    unregister_native_event_session(&delivery_kind);

    remove_midi_event_resource(binding, handle.0, "destack.midi.event.close", "midi event")
}

/// Wait for one JACK topology event.
pub(crate) fn midi_event_read(
    binding: &BindingCallContext,
    handle: resource::MidiEventHandle,
    timeout_ns: u64,
) -> RuntimeResult<MidiEventValue> {
    let session = event_resource(binding, handle, "destack.midi.event.read")?;
    let poll_service = binding
        .agent()
        .platform_state
        .midi
        .jack_service("destack.midi.event.read")?;
    let deadline = core_platform::timeout_deadline(timeout_ns);

    loop {
        {
            let mut session = session.lock();
            if let JackEventDeliveryKind::Poll = session.delivery_kind {
                refresh_event_subscription(
                    &poll_service,
                    &mut session,
                    MidiEventSource::SyntheticPoll,
                )?;
            }

            if let Some(event) = try_pop_session_event(&session, "destack.midi.event.read")? {
                return Ok(event);
            }
        }

        if deadline.is_some_and(|deadline| Instant::now() >= deadline) {
            return Err(core_platform::io_would_block(
                "destack.midi.event.read",
                "no queued MIDI topology event is available",
            ));
        }

        std::thread::sleep(std::time::Duration::from_millis(5));
    }
}

/// Wait for one batch of JACK topology events.
pub(crate) fn midi_event_read_batch(
    binding: &BindingCallContext,
    handle: resource::MidiEventHandle,
    max_events: u32,
    timeout_ns: u64,
) -> RuntimeResult<Vec<MidiEventValue>> {
    let session = event_resource(binding, handle, "destack.midi.event.readBatch")?;
    let poll_service = binding
        .agent()
        .platform_state
        .midi
        .jack_service("destack.midi.event.readBatch")?;
    let deadline = core_platform::timeout_deadline(timeout_ns);

    loop {
        {
            let mut session = session.lock();
            if let JackEventDeliveryKind::Poll = session.delivery_kind {
                refresh_event_subscription(
                    &poll_service,
                    &mut session,
                    MidiEventSource::SyntheticPoll,
                )?;
            }

            let events = try_pop_session_event_batch(
                &session,
                max_events as usize,
                "destack.midi.event.readBatch",
            )?;
            if !events.is_empty() {
                return Ok(events);
            }
        }

        if deadline.is_some_and(|deadline| Instant::now() >= deadline) {
            return Err(core_platform::io_would_block(
                "destack.midi.event.readBatch",
                "no queued MIDI topology events are available",
            ));
        }

        std::thread::sleep(std::time::Duration::from_millis(5));
    }
}

/// Poll one JACK topology event without blocking.
pub(crate) fn midi_event_try_read(
    binding: &BindingCallContext,
    handle: resource::MidiEventHandle,
) -> RuntimeResult<MidiEventValue> {
    let session = event_resource(binding, handle, "destack.midi.event.tryRead")?;
    let poll_service = binding
        .agent()
        .platform_state
        .midi
        .jack_service("destack.midi.event.tryRead")?;
    let mut session = session.lock();

    if let JackEventDeliveryKind::Poll = session.delivery_kind {
        refresh_event_subscription(&poll_service, &mut session, MidiEventSource::SyntheticPoll)?;
    }

    require_queued_event(
        try_pop_session_event(&session, "destack.midi.event.tryRead")?,
        "destack.midi.event.tryRead",
        "no queued MIDI topology event is available",
    )
}

/// Poll one batch of JACK topology events without blocking.
pub(crate) fn midi_event_try_read_batch(
    binding: &BindingCallContext,
    handle: resource::MidiEventHandle,
    max_events: u32,
) -> RuntimeResult<Vec<MidiEventValue>> {
    let session = event_resource(binding, handle, "destack.midi.event.tryReadBatch")?;
    let poll_service = binding
        .agent()
        .platform_state
        .midi
        .jack_service("destack.midi.event.tryReadBatch")?;
    let mut session = session.lock();

    if let JackEventDeliveryKind::Poll = session.delivery_kind {
        refresh_event_subscription(&poll_service, &mut session, MidiEventSource::SyntheticPoll)?;
    }

    let events = try_pop_session_event_batch(
        &session,
        max_events as usize,
        "destack.midi.event.tryReadBatch",
    )?;
    require_queued_event_batch(
        events,
        "destack.midi.event.tryReadBatch",
        "no queued MIDI topology events are available",
    )
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::sync::Arc;

    use super::queue_backend_disconnected_event;
    use crate::platform::midi::core::MidiEventValue;
    use crate::platform::midi::{
        MIDI_EVENT_SUBSCRIPTION_INCLUDE_DISCONNECTED, MIDI_PORT_DIRECTION_FLAG_OUTPUT, MidiBackend,
        MidiEventOverflowPolicy, MidiEventSource, MidiEventSubscriptionFlags,
        MidiPortDirectionFlags,
    };
    use crate::runtime::control::queue::BoundedQueue;

    use super::super::core::{JackEventDeliveryKind, JackEventSession};

    /// Queue one backend-disconnected event with the active backend metadata.
    #[test]
    fn test_queue_backend_disconnected_event_pushes_backend_event() {
        let queue = Arc::new(BoundedQueue::new(4));
        let mut session = JackEventSession {
            backend: MidiBackend::JackMidi,
            direction_mask: MidiPortDirectionFlags(MIDI_PORT_DIRECTION_FLAG_OUTPUT.0),
            flags: MidiEventSubscriptionFlags(MIDI_EVENT_SUBSCRIPTION_INCLUDE_DISCONNECTED.0),
            overflow_policy: MidiEventOverflowPolicy::DropOldest,
            delivery_kind: JackEventDeliveryKind::Poll,
            queue: queue.clone(),
            next_sequence: 1,
            snapshot: BTreeMap::new(),
        };

        queue_backend_disconnected_event(&mut session, MidiEventSource::Native, 5)
            .expect("backend disconnected event should queue successfully");

        let event = queue
            .try_pop()
            .expect("event queue should contain one event");
        match event {
            MidiEventValue::BackendDisconnected { metadata, flags } => {
                assert_eq!(metadata.backend, MidiBackend::JackMidi);
                assert_eq!(metadata.source, MidiEventSource::Native);
                assert_eq!(flags, 5);
            }
            other => panic!("expected backendDisconnected event, got {other:?}"),
        }
    }
}
