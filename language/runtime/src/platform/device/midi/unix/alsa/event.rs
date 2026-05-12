use std::collections::BTreeMap;
use std::sync::{Arc, Weak};

use parking_lot::Mutex;

use crate::diagnostic::RuntimeResult;
use crate::platform::core::{self as core_platform, BoundedQueue};
use crate::platform::device::midi::core::{
    MidiEventValue, MidiPortDescriptorValue, collect_live_event_sessions, direction_mask_includes,
    event_poll_interval, event_queue_capacity, event_snapshot_list_flags,
    push_backend_disconnected_event as push_backend_disconnected_midi_event, read_queued_event,
    read_queued_event_batch, refresh_snapshot_event_subscription, remove_midi_event_resource,
    require_queued_event, require_queued_event_batch, snapshot_key, try_pop_queued_event,
    try_pop_queued_event_batch,
};
use crate::platform::device::{
    MidiEventDeliveryMode, MidiEventSource, MidiEventSubscriptionFlags,
    MidiEventSubscriptionOptions, MidiPortDirection, MidiPortDirectionFlags,
};
use crate::platform::resource;
use crate::runtime::BindingCallContext;
use crate::runtime::service::Service;
use crate::runtime::service::executor::periodic::open_periodic_task;

use super::core::{
    AlsaEventDeliveryKind, AlsaEventRepository, AlsaTopologyState, SnapshotKey,
    insert_event_resource,
};
use super::descriptor::filtered_descriptors;
use super::resource::event_resource;
use super::service::{
    AlsaNativeEventRegistry, AlsaService, alsa_service, register_native_event_session,
    unregister_native_event_session,
};

/// Build one current snapshot for one event subscription.
fn event_snapshot(
    service: &Arc<AlsaService>,
    flags: MidiEventSubscriptionFlags,
    direction_mask: MidiPortDirectionFlags,
) -> BTreeMap<SnapshotKey, (MidiPortDirection, MidiPortDescriptorValue)> {
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
    service: &Arc<AlsaService>,
    session: &mut AlsaEventRepository,
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

/// Refresh one event subscription from synthetic polling.
fn refresh_poll_event_subscription(
    service: &Arc<AlsaService>,
    session: &mut AlsaEventRepository,
) -> RuntimeResult<()> {
    refresh_event_subscription(service, session, MidiEventSource::SyntheticPoll)
}

/// Queue one backend-disconnected event into one subscription.
fn queue_backend_disconnected_event(
    session: &mut AlsaEventRepository,
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

/// Register one synthetic poll delivery for one ALSA event subscription.
fn register_poll_event_session(
    service: &Arc<AlsaService>,
    session: &Arc<Mutex<AlsaEventRepository>>,
    poll_interval: Duration,
) -> RuntimeResult<Arc<crate::runtime::service::executor::periodic::PeriodicTaskHandle>> {
    let service = service.clone();
    let session = Arc::downgrade(session);
    let is_failed = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let failure_state = is_failed.clone();

    let task = open_periodic_task(
        "destack-midi-alsa-event",
        AlsaService::POLICY,
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
                refresh_poll_event_subscription(&service, &mut session)
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
    session: &AlsaEventRepository,
    operation: &'static str,
) -> RuntimeResult<Option<MidiEventValue>> {
    try_pop_queued_event(&session.queue, operation)
}

/// Pop one queued event batch after honoring deferred overflow errors.
fn try_pop_session_event_batch(
    session: &AlsaEventRepository,
    max_events: usize,
    operation: &'static str,
) -> RuntimeResult<Vec<MidiEventValue>> {
    try_pop_queued_event_batch(&session.queue, max_events, operation)
}

/// Collect all live native event subscriptions from one registry.
fn collect_native_event_sessions(
    registry: &Arc<Mutex<AlsaNativeEventRegistry>>,
) -> Vec<Arc<Mutex<AlsaEventRepository>>> {
    let mut registry = registry.lock();

    collect_live_event_sessions(&mut registry.sessions)
}

/// Refresh all native event subscriptions from one backend topology mutation.
pub(super) fn refresh_native_event_sessions(
    topology: &Arc<Mutex<AlsaTopologyState>>,
    registry: &Arc<Mutex<AlsaNativeEventRegistry>>,
    source: MidiEventSource,
) {
    let sessions = collect_native_event_sessions(registry);
    let Ok(service) = alsa_service("destack.device.midi.event.refresh") else {
        for session in sessions {
            let mut session = session.lock();
            let _ = queue_backend_disconnected_event(&mut session, source, 0);
        }

        return;
    };

    {
        let topology = topology.lock();
        let mut service_topology = service.topology.lock();
        *service_topology = AlsaTopologyState {
            inputs: topology.inputs.clone(),
            outputs: topology.outputs.clone(),
        };
    }

    for session in sessions {
        let mut session = session.lock();
        if refresh_event_subscription(&service, &mut session, source).is_err() {
            let _ = queue_backend_disconnected_event(&mut session, source, 0);
        }
    }
}

/// Queue one backend-disconnected event for all native subscriptions.
pub(super) fn queue_backend_disconnected_events(
    registry: &Arc<Mutex<AlsaNativeEventRegistry>>,
    source: MidiEventSource,
    flags: u32,
) {
    let sessions = collect_native_event_sessions(registry);
    for session in sessions {
        let mut session = session.lock();
        let _ = queue_backend_disconnected_event(&mut session, source, flags);
    }
}

/// Open one ALSA sequencer event subscription.
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
        .alsa_service("destack.device.midi.event.open")?;
    let delivery_kind = match options.delivery_mode {
        MidiEventDeliveryMode::PollOnly => AlsaEventDeliveryKind::Poll,
        MidiEventDeliveryMode::Auto | MidiEventDeliveryMode::NativeOnly => {
            AlsaEventDeliveryKind::Poll
        }
    };

    let session = Arc::new(Mutex::new(AlsaEventRepository {
        backend: crate::platform::device::MidiBackend::Alsa,
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
        snapshot: event_snapshot(&service, options.flags, options.direction_mask),
    }));

    // native feed
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

/// Close one ALSA sequencer event subscription.
pub(crate) fn midi_event_close(
    binding: &BindingCallContext,
    handle: resource::MidiEventHandle,
) -> RuntimeResult<()> {
    if let Ok(session) = event_resource(binding, handle, "destack.device.midi.event.close") {
        let session = session.lock();
        unregister_native_event_session(&session.delivery_kind);
    }

    remove_midi_event_resource(
        binding,
        handle.0,
        "destack.device.midi.event.close",
        "midi event",
    )
}

/// Read one ALSA sequencer event.
pub(crate) fn midi_event_read(
    binding: &BindingCallContext,
    handle: resource::MidiEventHandle,
    timeout_ns: u64,
) -> RuntimeResult<MidiEventValue> {
    let session = event_resource(binding, handle, "destack.device.midi.event.read")?;
    let session = session.lock();

    read_queued_event(
        &session.queue,
        timeout_ns,
        "destack.device.midi.event.read",
        "no queued MIDI topology event is available",
    )
}

/// Read one ALSA sequencer event batch.
pub(crate) fn midi_event_read_batch(
    binding: &BindingCallContext,
    handle: resource::MidiEventHandle,
    max_events: u32,
    timeout_ns: u64,
) -> RuntimeResult<Vec<MidiEventValue>> {
    let session = event_resource(binding, handle, "destack.device.midi.event.readBatch")?;
    let session = session.lock();

    read_queued_event_batch(
        &session.queue,
        max_events.max(1) as usize,
        timeout_ns,
        "destack.device.midi.event.readBatch",
        "no queued MIDI topology events are available",
    )
}

/// Poll one ALSA sequencer event without blocking.
pub(crate) fn midi_event_try_read(
    binding: &BindingCallContext,
    handle: resource::MidiEventHandle,
) -> RuntimeResult<MidiEventValue> {
    let session = event_resource(binding, handle, "destack.device.midi.event.tryRead")?;
    let session = session.lock();

    require_queued_event(
        try_pop_session_event(&session, "destack.device.midi.event.tryRead")?,
        "destack.device.midi.event.tryRead",
        "no queued MIDI topology event is available",
    )
}

/// Poll one ALSA sequencer event batch without blocking.
pub(crate) fn midi_event_try_read_batch(
    binding: &BindingCallContext,
    handle: resource::MidiEventHandle,
    max_events: u32,
) -> RuntimeResult<Vec<MidiEventValue>> {
    let session = event_resource(binding, handle, "destack.device.midi.event.tryReadBatch")?;
    let session = session.lock();

    let events = try_pop_session_event_batch(
        &session,
        max_events as usize,
        "destack.device.midi.event.tryReadBatch",
    )?;
    require_queued_event_batch(
        events,
        "destack.device.midi.event.tryReadBatch",
        "no queued MIDI topology events are available",
    )
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::sync::Arc;
    use std::time::Duration;

    use super::queue_backend_disconnected_event;
    use crate::platform::core::BoundedQueue;
    use crate::platform::device::midi::core::MidiEventValue;
    use crate::platform::device::{
        MIDI_EVENT_SUBSCRIPTION_INCLUDE_DISCONNECTED, MIDI_PORT_DIRECTION_FLAG_INPUT, MidiBackend,
        MidiEventOverflowPolicy, MidiEventSource, MidiEventSubscriptionFlags,
        MidiPortDirectionFlags,
    };

    use super::super::core::{AlsaEventDeliveryKind, AlsaEventRepository};

    /// Queue one backend-disconnected event with the active backend metadata.
    #[test]
    fn test_queue_backend_disconnected_event_pushes_backend_event() {
        let queue = Arc::new(BoundedQueue::new(4));
        let mut session = AlsaEventRepository {
            backend: MidiBackend::Alsa,
            direction_mask: MidiPortDirectionFlags(MIDI_PORT_DIRECTION_FLAG_INPUT.0),
            flags: MidiEventSubscriptionFlags(MIDI_EVENT_SUBSCRIPTION_INCLUDE_DISCONNECTED.0),
            overflow_policy: MidiEventOverflowPolicy::DropOldest,
            poll_interval: Duration::from_millis(1),
            delivery_kind: AlsaEventDeliveryKind::Poll,
            poll_task: None,
            queue: queue.clone(),
            next_sequence: 1,
            snapshot: BTreeMap::new(),
        };

        queue_backend_disconnected_event(&mut session, MidiEventSource::Native, 9)
            .expect("backend disconnected event should queue successfully");

        let event = queue
            .try_pop()
            .expect("event queue should contain one event");
        match event {
            MidiEventValue::BackendDisconnected { metadata, flags } => {
                assert_eq!(metadata.backend, MidiBackend::Alsa);
                assert_eq!(metadata.source, MidiEventSource::Native);
                assert_eq!(flags, 9);
            }
            other => panic!("expected backendDisconnected event, got {other:?}"),
        }
    }
}
