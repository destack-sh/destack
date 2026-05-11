use std::collections::BTreeMap;
use std::sync::{Arc, Weak};
use std::time::Duration;

use parking_lot::Mutex;

use crate::diagnostic::RuntimeResult;
use crate::platform::core::{self as core_platform};
use crate::platform::device::midi::core::{
    MidiEventValue, MidiPortDescriptorValue, collect_live_event_sessions,
    descriptor_matches_list_flags, direction_mask_includes, event_poll_interval,
    event_queue_capacity, event_snapshot_list_flags,
    push_backend_disconnected_event as push_backend_disconnected_midi_event, read_queued_event,
    read_queued_event_batch, refresh_snapshot_event_subscription, remove_labeled_resource,
    require_queued_event, require_queued_event_batch, snapshot_key, try_pop_queued_event,
    try_pop_queued_event_batch,
};
use crate::platform::device::{
    MidiBackend, MidiEventDeliveryMode, MidiEventSource, MidiEventSubscriptionFlags,
    MidiEventSubscriptionOptions, MidiPortDirection, MidiPortDirectionFlags,
};
use crate::platform::resource;
use crate::runtime::BindingCallContext;
use crate::runtime::control::queue::BoundedQueue;
use crate::runtime::service::Service;
use crate::runtime::service::executor::periodic::open_periodic_task;

use super::core::{
    SnapshotKey, WindowsMidiEventDeliveryKind, WindowsMidiEventSession, WindowsMidiTopologyState,
    insert_event_resource,
};
use super::resource::event_resource;
use super::service::{
    WindowsMidiNativeEventRegistry, WindowsMidiService, register_native_event_session,
    unregister_native_event_session,
};

/// Build one current snapshot for one event subscription.
fn event_snapshot(
    topology: &Arc<Mutex<WindowsMidiTopologyState>>,
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
    topology: &Arc<Mutex<WindowsMidiTopologyState>>,
    session: &mut WindowsMidiEventSession,
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
    topology: &Arc<Mutex<WindowsMidiTopologyState>>,
    session: &mut WindowsMidiEventSession,
) -> RuntimeResult<()> {
    refresh_event_subscription(topology, session, MidiEventSource::SyntheticPoll)
}

/// Queue one backend-disconnected event into one subscription.
fn queue_backend_disconnected_event(
    session: &mut WindowsMidiEventSession,
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

/// Register one synthetic poll delivery for one Windows MIDI event subscription.
fn register_poll_event_session(
    service: &Arc<WindowsMidiService>,
    session: &Arc<Mutex<WindowsMidiEventSession>>,
    poll_interval: Duration,
) -> RuntimeResult<Arc<crate::runtime::service::executor::periodic::PeriodicTaskHandle>> {
    let topology = service.topology.clone();
    let session = Arc::downgrade(session);
    let is_failed = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let failure_state = is_failed.clone();

    let task = open_periodic_task(
        "destack-midi-windows-midi-event",
        WindowsMidiService::POLICY,
        poll_interval,
        move || {
            if failure_state.load(std::sync::atomic::Ordering::Acquire) {
                return Ok(());
            }

            let Some(session) = Weak::upgrade(&session) else {
                return Ok(());
            };

            let refresh_result = {
                let mut session = session.lock();
                refresh_poll_event_subscription(&topology, &mut session)
            };

            if refresh_result.is_err() {
                let mut session = session.lock();
                let _ = queue_backend_disconnected_event(
                    &mut session,
                    MidiEventSource::SyntheticPoll,
                    0,
                );
                failure_state.store(true, std::sync::atomic::Ordering::Release);
            }

            Ok(())
        },
    )?;

    Ok(Arc::new(task))
}

/// Pop one queued event after honoring deferred overflow errors.
fn try_pop_session_event(
    session: &WindowsMidiEventSession,
    operation: &'static str,
) -> RuntimeResult<Option<MidiEventValue>> {
    try_pop_queued_event(&session.queue, operation)
}

/// Pop one queued event batch after honoring deferred overflow errors.
fn try_pop_session_event_batch(
    session: &WindowsMidiEventSession,
    max_events: usize,
    operation: &'static str,
) -> RuntimeResult<Vec<MidiEventValue>> {
    try_pop_queued_event_batch(&session.queue, max_events, operation)
}

/// Collect all live native event subscriptions from one registry.
fn collect_native_event_sessions(
    registry: &Arc<Mutex<WindowsMidiNativeEventRegistry>>,
) -> Vec<Arc<Mutex<WindowsMidiEventSession>>> {
    let mut registry = registry.lock();

    collect_live_event_sessions(&mut registry.sessions)
}

/// Refresh all native event subscriptions from one backend topology mutation.
pub(super) fn refresh_native_event_sessions(
    topology: &Arc<Mutex<WindowsMidiTopologyState>>,
    registry: &Arc<Mutex<WindowsMidiNativeEventRegistry>>,
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
    registry: &Arc<Mutex<WindowsMidiNativeEventRegistry>>,
    source: MidiEventSource,
    flags: u32,
) {
    let sessions = collect_native_event_sessions(registry);

    for session in sessions {
        let mut session = session.lock();
        let _ = queue_backend_disconnected_event(&mut session, source, flags);
    }
}

/// Open one Windows MIDI event subscription.
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
        .worker()
        .platform_state
        .device
        .windows_midi_service("destack.device.midi.event.open")?;
    let delivery_kind = match options.delivery_mode {
        MidiEventDeliveryMode::PollOnly => WindowsMidiEventDeliveryKind::Poll,
        MidiEventDeliveryMode::Auto | MidiEventDeliveryMode::NativeOnly => {
            WindowsMidiEventDeliveryKind::Poll
        }
    };

    let session = Arc::new(Mutex::new(WindowsMidiEventSession {
        backend: MidiBackend::WindowsMidi,
        direction_mask: options.direction_mask,
        flags: options.flags,
        overflow_policy: options.overflow_policy,
        poll_interval: event_poll_interval(options.poll_interval_ns),
        delivery_kind,
        poll_task: None,
        queue: Arc::new(BoundedQueue::new(event_queue_capacity(
            options.queue_capacity,
        ))),
        next_sequence: 1,
        snapshot: event_snapshot(&service.topology, options.flags, options.direction_mask),
    }));

    if options.delivery_mode != MidiEventDeliveryMode::PollOnly {
        let delivery_kind = register_native_event_session(&service, &session);
        session.lock().delivery_kind = delivery_kind;
    } else {
        let poll_interval = session.lock().poll_interval;
        let poll_task = register_poll_event_session(&service, &session, poll_interval)?;
        session.lock().poll_task = Some(poll_task);
    }

    Ok(insert_event_resource(binding, session))
}

/// Close one Windows MIDI event subscription.
pub(crate) fn midi_event_close(
    binding: &BindingCallContext,
    handle: resource::MidiEventHandle,
) -> RuntimeResult<()> {
    if let Ok(session) = event_resource(binding, handle, "destack.device.midi.event.close") {
        let session = session.lock();
        unregister_native_event_session(&session.delivery_kind);
    }

    remove_labeled_resource(
        binding,
        handle.0,
        "destack.device.midi.event.close",
        "midi event",
    )?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::sync::Arc;
    use std::time::Duration;

    use super::queue_backend_disconnected_event;
    use crate::platform::device::midi::core::MidiEventValue;
    use crate::platform::device::{
        MIDI_EVENT_SUBSCRIPTION_INCLUDE_DISCONNECTED, MIDI_PORT_DIRECTION_FLAG_OUTPUT, MidiBackend,
        MidiEventOverflowPolicy, MidiEventSource, MidiEventSubscriptionFlags,
        MidiPortDirectionFlags,
    };
    use crate::runtime::control::queue::BoundedQueue;

    use super::super::core::{WindowsMidiEventDeliveryKind, WindowsMidiEventSession};

    /// Queue one backend-disconnected event with the active backend metadata.
    #[test]
    fn test_queue_backend_disconnected_event_pushes_backend_event() {
        let queue = Arc::new(BoundedQueue::new(4));
        let mut session = WindowsMidiEventSession {
            backend: MidiBackend::WindowsMidi,
            direction_mask: MidiPortDirectionFlags(MIDI_PORT_DIRECTION_FLAG_OUTPUT.0),
            flags: MidiEventSubscriptionFlags(MIDI_EVENT_SUBSCRIPTION_INCLUDE_DISCONNECTED.0),
            overflow_policy: MidiEventOverflowPolicy::DropOldest,
            poll_interval: Duration::from_millis(1),
            delivery_kind: WindowsMidiEventDeliveryKind::Poll,
            poll_task: None,
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
                assert_eq!(metadata.backend, MidiBackend::WindowsMidi);
                assert_eq!(metadata.source, MidiEventSource::Native);
                assert_eq!(flags, 3);
            }
            other => panic!("expected backendDisconnected event, got {other:?}"),
        }
    }
}

/// Wait for one Windows MIDI event.
pub(crate) fn midi_event_read(
    binding: &BindingCallContext,
    handle: resource::MidiEventHandle,
    timeout_ns: u64,
) -> RuntimeResult<MidiEventValue> {
    let session = event_resource(binding, handle, "destack.device.midi.event.read")?;

    // native feeds block directly on the shared queue
    let is_native = {
        let session = session.lock();
        matches!(
            session.delivery_kind,
            WindowsMidiEventDeliveryKind::Native { .. }
        )
    };
    let timeout = Duration::from_nanos(timeout_ns);

    if is_native {
        let session = session.lock();
        return read_queued_event(
            &session.queue,
            timeout.as_nanos() as u64,
            "destack.device.midi.event.read",
            "midi event queue is empty",
        );
    }

    let session = session.lock();
    read_queued_event(
        &session.queue,
        timeout_ns,
        "destack.device.midi.event.read",
        "midi event queue is empty",
    )
}

/// Wait for one Windows MIDI event batch.
pub(crate) fn midi_event_read_batch(
    binding: &BindingCallContext,
    handle: resource::MidiEventHandle,
    max_events: u32,
    timeout_ns: u64,
) -> RuntimeResult<Vec<MidiEventValue>> {
    let session = event_resource(binding, handle, "destack.device.midi.event.readBatch")?;

    // native feeds block directly on the shared queue
    let is_native = {
        let session = session.lock();
        matches!(
            session.delivery_kind,
            WindowsMidiEventDeliveryKind::Native { .. }
        )
    };
    let timeout = Duration::from_nanos(timeout_ns);

    if is_native {
        let session = session.lock();
        return read_queued_event_batch(
            &session.queue,
            max_events.max(1) as usize,
            timeout.as_nanos() as u64,
            "destack.device.midi.event.readBatch",
            "midi event queue is empty",
        );
    }

    let session = session.lock();
    read_queued_event_batch(
        &session.queue,
        max_events.max(1) as usize,
        timeout_ns,
        "destack.device.midi.event.readBatch",
        "midi event queue is empty",
    )
}

/// Poll one Windows MIDI event.
pub(crate) fn midi_event_try_read(
    binding: &BindingCallContext,
    handle: resource::MidiEventHandle,
) -> RuntimeResult<MidiEventValue> {
    let session = event_resource(binding, handle, "destack.device.midi.event.tryRead")?;
    let session = session.lock();

    require_queued_event(
        try_pop_session_event(&session, "destack.device.midi.event.tryRead")?,
        "destack.device.midi.event.tryRead",
        "midi event queue is empty",
    )
}

/// Poll one Windows MIDI event batch.
pub(crate) fn midi_event_try_read_batch(
    binding: &BindingCallContext,
    handle: resource::MidiEventHandle,
    max_events: u32,
) -> RuntimeResult<Vec<MidiEventValue>> {
    let session = event_resource(binding, handle, "destack.device.midi.event.tryReadBatch")?;
    let session = session.lock();

    let batch = try_pop_session_event_batch(
        &session,
        max_events.max(1) as usize,
        "destack.device.midi.event.tryReadBatch",
    )?;

    require_queued_event_batch(
        batch,
        "destack.device.midi.event.tryReadBatch",
        "midi event queue is empty",
    )
}
