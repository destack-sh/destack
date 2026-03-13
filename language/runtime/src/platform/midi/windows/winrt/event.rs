use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use parking_lot::Mutex;

use crate::diagnostic::RuntimeResult;
use crate::platform::core::{self as core_platform};
use crate::platform::midi::core::{
    MidiEventValue, MidiPortDescriptorValue, collect_live_event_sessions,
    descriptor_matches_list_flags, direction_mask_includes, event_poll_interval,
    event_queue_capacity, event_snapshot_list_flags,
    push_backend_disconnected_event as push_backend_disconnected_midi_event, read_queued_event,
    read_queued_event_batch, refresh_snapshot_event_subscription, remove_labeled_resource,
    require_queued_event, require_queued_event_batch, snapshot_key, try_pop_queued_event,
    try_pop_queued_event_batch,
};
use crate::platform::midi::{
    MidiBackend, MidiEventDeliveryMode, MidiEventSource, MidiEventSubscriptionFlags,
    MidiEventSubscriptionOptions, MidiPortDirection, MidiPortDirectionFlags,
};
use crate::platform::resource;
use crate::runtime::BindingCallContext;
use crate::runtime::control::queue::BoundedQueue;

use super::core::{
    SnapshotKey, WinRtEventDeliveryKind, WinRtEventSession, WinRtTopologyState,
    insert_event_resource,
};
use super::resource::event_resource;
use super::service::{
    WinRtNativeEventRegistry, register_native_event_session, unregister_native_event_session,
};

/// Build one current snapshot for one event subscription.
fn event_snapshot(
    topology: &Arc<Mutex<WinRtTopologyState>>,
    flags: MidiEventSubscriptionFlags,
    direction_mask: MidiPortDirectionFlags,
) -> BTreeMap<SnapshotKey, (MidiPortDirection, MidiPortDescriptorValue)> {
    let list_flags = event_snapshot_list_flags(flags);
    let mut snapshot = BTreeMap::new();
    let topology = topology.lock();

    if direction_mask_includes(direction_mask, MidiPortDirection::Input) {
        for descriptor in topology
            .inputs
            .values()
            .map(|row| row.descriptor.clone())
            .filter(|descriptor| descriptor_matches_list_flags(descriptor, list_flags))
        {
            let key = snapshot_key(MidiPortDirection::Input, &descriptor.id);
            snapshot.insert(key, (MidiPortDirection::Input, descriptor));
        }
    }

    if direction_mask_includes(direction_mask, MidiPortDirection::Output) {
        for descriptor in topology
            .outputs
            .values()
            .map(|row| row.descriptor.clone())
            .filter(|descriptor| descriptor_matches_list_flags(descriptor, list_flags))
        {
            let key = snapshot_key(MidiPortDirection::Output, &descriptor.id);
            snapshot.insert(key, (MidiPortDirection::Output, descriptor));
        }
    }

    snapshot
}

/// Refresh one event subscription queue from the current snapshot.
fn refresh_event_subscription(
    topology: &Arc<Mutex<WinRtTopologyState>>,
    session: &mut WinRtEventSession,
    source: MidiEventSource,
) -> RuntimeResult<()> {
    let next_snapshot = event_snapshot(topology, session.flags, session.direction_mask);
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

/// Refresh one event subscription from synthetic polling.
fn refresh_poll_event_subscription(
    topology: &Arc<Mutex<WinRtTopologyState>>,
    session: &mut WinRtEventSession,
) -> RuntimeResult<()> {
    refresh_event_subscription(topology, session, MidiEventSource::SyntheticPoll)
}

/// Queue one backend-disconnected event into one subscription.
fn queue_backend_disconnected_event(
    session: &mut WinRtEventSession,
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
    session: &WinRtEventSession,
    operation: &'static str,
) -> RuntimeResult<Option<MidiEventValue>> {
    try_pop_queued_event(&session.queue, operation)
}

/// Pop one queued event batch after honoring deferred overflow errors.
fn try_pop_session_event_batch(
    session: &WinRtEventSession,
    max_events: usize,
    operation: &'static str,
) -> RuntimeResult<Vec<MidiEventValue>> {
    try_pop_queued_event_batch(&session.queue, max_events, operation)
}

/// Collect all live native event subscriptions from one registry.
fn collect_native_event_sessions(
    registry: &Arc<Mutex<WinRtNativeEventRegistry>>,
) -> Vec<Arc<Mutex<WinRtEventSession>>> {
    let mut registry = registry.lock();

    collect_live_event_sessions(&mut registry.sessions)
}

/// Refresh all native event subscriptions from one backend topology mutation.
pub(super) fn refresh_native_event_sessions(
    topology: &Arc<Mutex<WinRtTopologyState>>,
    registry: &Arc<Mutex<WinRtNativeEventRegistry>>,
    source: MidiEventSource,
) {
    let sessions = collect_native_event_sessions(registry);

    for session in sessions {
        let mut session = session.lock();
        if refresh_event_subscription(topology, &mut session, source).is_err() {
            let _ = queue_backend_disconnected_event(&mut session, source, 0);
        }
    }
}

/// Queue one backend-disconnected event for all native subscriptions.
pub(super) fn queue_backend_disconnected_events(
    registry: &Arc<Mutex<WinRtNativeEventRegistry>>,
    source: MidiEventSource,
    flags: u32,
) {
    let sessions = collect_native_event_sessions(registry);

    for session in sessions {
        let mut session = session.lock();
        let _ = queue_backend_disconnected_event(&mut session, source, flags);
    }
}

/// Open one WinRT event subscription.
pub(crate) fn midi_event_open(
    binding: &BindingCallContext,
    options: MidiEventSubscriptionOptions,
) -> RuntimeResult<resource::MidiEventHandle> {
    if options.direction_mask.0 == 0 {
        return Err(core_platform::invalid_argument(
            "directionMask",
            "directionMask must not be zero",
        ));
    }

    let service = binding
        .agent()
        .platform_state
        .midi
        .winrt_service("destack.midi.event.open")?;
    let delivery_kind = match options.delivery_mode {
        MidiEventDeliveryMode::PollOnly => WinRtEventDeliveryKind::Poll,
        MidiEventDeliveryMode::Auto | MidiEventDeliveryMode::NativeOnly => {
            WinRtEventDeliveryKind::Poll
        }
    };

    let session = Arc::new(Mutex::new(WinRtEventSession {
        backend: MidiBackend::WinRT,
        direction_mask: options.direction_mask,
        flags: options.flags,
        overflow_policy: options.overflow_policy,
        poll_interval: event_poll_interval(options.poll_interval_ns),
        delivery_kind,
        queue: Arc::new(BoundedQueue::new(event_queue_capacity(
            options.queue_capacity,
        ))),
        next_sequence: 1,
        snapshot: event_snapshot(&service.topology, options.flags, options.direction_mask),
    }));

    if options.delivery_mode != MidiEventDeliveryMode::PollOnly {
        let delivery_kind = register_native_event_session(&service, &session);
        session.lock().delivery_kind = delivery_kind;
    }

    Ok(insert_event_resource(binding, session))
}

/// Close one WinRT event subscription.
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

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::sync::Arc;
    use std::time::Duration;

    use super::queue_backend_disconnected_event;
    use crate::platform::midi::core::MidiEventValue;
    use crate::platform::midi::{
        MIDI_EVENT_SUBSCRIPTION_INCLUDE_DISCONNECTED, MIDI_PORT_DIRECTION_FLAG_OUTPUT, MidiBackend,
        MidiEventOverflowPolicy, MidiEventSource, MidiEventSubscriptionFlags,
        MidiPortDirectionFlags,
    };
    use crate::runtime::control::queue::BoundedQueue;

    use super::super::core::{WinRtEventDeliveryKind, WinRtEventSession};

    /// Queue one backend-disconnected event with the active backend metadata.
    #[test]
    fn test_queue_backend_disconnected_event_pushes_backend_event() {
        let queue = Arc::new(BoundedQueue::new(4));
        let mut session = WinRtEventSession {
            backend: MidiBackend::WinRT,
            direction_mask: MidiPortDirectionFlags(MIDI_PORT_DIRECTION_FLAG_OUTPUT.0),
            flags: MidiEventSubscriptionFlags(MIDI_EVENT_SUBSCRIPTION_INCLUDE_DISCONNECTED.0),
            overflow_policy: MidiEventOverflowPolicy::DropOldest,
            poll_interval: Duration::from_millis(1),
            delivery_kind: WinRtEventDeliveryKind::Poll,
            queue: queue.clone(),
            next_sequence: 1,
            snapshot: BTreeMap::new(),
        };

        queue_backend_disconnected_event(&mut session, MidiEventSource::Native, 3)
            .expect("backend disconnected event should queue successfully");

        let event = queue
            .try_pop()
            .expect("event queue should contain one event");
        match event {
            MidiEventValue::BackendDisconnected { metadata, flags } => {
                assert_eq!(metadata.backend, MidiBackend::WinRT);
                assert_eq!(metadata.source, MidiEventSource::Native);
                assert_eq!(flags, 3);
            }
            other => panic!("expected backendDisconnected event, got {other:?}"),
        }
    }
}

/// Wait for one WinRT event.
pub(crate) fn midi_event_read(
    binding: &BindingCallContext,
    handle: resource::MidiEventHandle,
    timeout_ns: u64,
) -> RuntimeResult<MidiEventValue> {
    let service = binding
        .agent()
        .platform_state
        .midi
        .winrt_service("destack.midi.event.read")?;
    let session = event_resource(binding, handle, "destack.midi.event.read")?;

    // native feeds block directly on the shared queue
    let is_native = {
        let session = session.lock();
        matches!(session.delivery_kind, WinRtEventDeliveryKind::Native { .. })
    };
    let timeout = Duration::from_nanos(timeout_ns);

    if is_native {
        let session = session.lock();
        return read_queued_event(
            &session.queue,
            timeout.as_nanos() as u64,
            "destack.midi.event.read",
            "midi event queue is empty",
        );
    }

    let deadline = core_platform::timeout_deadline(timeout_ns);

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
            refresh_poll_event_subscription(&service.topology, &mut session)?;

            if let Some(event) = try_pop_session_event(&session, "destack.midi.event.read")? {
                return Ok(event);
            }
        }

        // stop once the caller timeout expires
        let now = Instant::now();
        if deadline.is_some_and(|deadline| now >= deadline) {
            return Err(core_platform::io_would_block(
                "destack.midi.event.read",
                "midi event queue is empty",
            ));
        }

        // otherwise sleep for the requested poll cadence
        let sleep_duration = {
            let session = session.lock();
            match deadline {
                Some(deadline) => session
                    .poll_interval
                    .min(deadline.saturating_duration_since(now)),
                None => session.poll_interval,
            }
        };
        std::thread::sleep(sleep_duration);
    }
}

/// Wait for one WinRT event batch.
pub(crate) fn midi_event_read_batch(
    binding: &BindingCallContext,
    handle: resource::MidiEventHandle,
    max_events: u32,
    timeout_ns: u64,
) -> RuntimeResult<Vec<MidiEventValue>> {
    let service = binding
        .agent()
        .platform_state
        .midi
        .winrt_service("destack.midi.event.readBatch")?;
    let session = event_resource(binding, handle, "destack.midi.event.readBatch")?;

    // native feeds block directly on the shared queue
    let is_native = {
        let session = session.lock();
        matches!(session.delivery_kind, WinRtEventDeliveryKind::Native { .. })
    };
    let timeout = Duration::from_nanos(timeout_ns);

    if is_native {
        let session = session.lock();
        return read_queued_event_batch(
            &session.queue,
            max_events.max(1) as usize,
            timeout.as_nanos() as u64,
            "destack.midi.event.readBatch",
            "midi event queue is empty",
        );
    }

    let deadline = core_platform::timeout_deadline(timeout_ns);

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
            refresh_poll_event_subscription(&service.topology, &mut session)?;
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
        if deadline.is_some_and(|deadline| now >= deadline) {
            return Err(core_platform::io_would_block(
                "destack.midi.event.readBatch",
                "midi event queue is empty",
            ));
        }

        // otherwise sleep for the requested poll cadence
        let sleep_duration = {
            let session = session.lock();
            match deadline {
                Some(deadline) => session
                    .poll_interval
                    .min(deadline.saturating_duration_since(now)),
                None => session.poll_interval,
            }
        };
        std::thread::sleep(sleep_duration);
    }
}

/// Poll one WinRT event.
pub(crate) fn midi_event_try_read(
    binding: &BindingCallContext,
    handle: resource::MidiEventHandle,
) -> RuntimeResult<MidiEventValue> {
    let service = binding
        .agent()
        .platform_state
        .midi
        .winrt_service("destack.midi.event.tryRead")?;
    let session = event_resource(binding, handle, "destack.midi.event.tryRead")?;

    // refresh synthetic subscriptions before peeking the queue
    let mut session = session.lock();
    if matches!(session.delivery_kind, WinRtEventDeliveryKind::Poll) {
        refresh_poll_event_subscription(&service.topology, &mut session)?;
    }

    require_queued_event(
        try_pop_session_event(&session, "destack.midi.event.tryRead")?,
        "destack.midi.event.tryRead",
        "midi event queue is empty",
    )
}

/// Poll one WinRT event batch.
pub(crate) fn midi_event_try_read_batch(
    binding: &BindingCallContext,
    handle: resource::MidiEventHandle,
    max_events: u32,
) -> RuntimeResult<Vec<MidiEventValue>> {
    let service = binding
        .agent()
        .platform_state
        .midi
        .winrt_service("destack.midi.event.tryReadBatch")?;
    let session = event_resource(binding, handle, "destack.midi.event.tryReadBatch")?;

    // refresh synthetic subscriptions before peeking the queue
    let mut session = session.lock();
    if matches!(session.delivery_kind, WinRtEventDeliveryKind::Poll) {
        refresh_poll_event_subscription(&service.topology, &mut session)?;
    }

    let batch = try_pop_session_event_batch(
        &session,
        max_events.max(1) as usize,
        "destack.midi.event.tryReadBatch",
    )?;

    require_queued_event_batch(
        batch,
        "destack.midi.event.tryReadBatch",
        "midi event queue is empty",
    )
}
