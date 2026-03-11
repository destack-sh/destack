use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use parking_lot::Mutex;

use crate::diagnostic::RuntimeResult;
use crate::platform::core::{self as core_platform};
use crate::platform::midi::core::{
    MidiEventMetadataValue, MidiEventValue, MidiPortDescriptorValue,
    push_event_with_overflow_policy, take_event_overflow_error,
};
use crate::platform::midi::{
    MidiEventDeliveryMode, MidiEventSource, MidiEventSubscriptionFlags,
    MidiEventSubscriptionOptions, MidiPortDirection, MidiPortDirectionFlags,
};
use crate::platform::resource;
use crate::runtime::BindingCallContext;

use super::backend::resolve_backend;
use super::core::{
    AlsaEventDeliveryKind, AlsaEventSession, AlsaTopologyState, BoundedQueue, SnapshotKey,
    binding_timestamp_now, direction_mask_includes, endpoint_direction_name, event_poll_interval,
    event_queue_capacity, event_snapshot_list_flags, insert_event_resource,
};
use super::descriptor::filtered_descriptors;
use super::resource::{event_resource, remove_event_resource};
use super::service::{
    AlsaNativeEventRegistry, AlsaService, alsa_service, register_native_event_session,
    unregister_native_event_session,
};

/// Return one stable snapshot key for one direction and descriptor id.
fn snapshot_key(direction: MidiPortDirection, id: &str) -> SnapshotKey {
    let direction_name = endpoint_direction_name(direction);

    format!("{direction_name}:{id}")
}

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
    session: &mut AlsaEventSession,
    source: MidiEventSource,
) -> RuntimeResult<()> {
    let next_snapshot = event_snapshot(service, session.flags, session.direction_mask);

    // additions and changes
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

                push_event_with_overflow_policy(
                    &session.queue,
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

                push_event_with_overflow_policy(
                    &session.queue,
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

    // removals
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

        push_event_with_overflow_policy(
            &session.queue,
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
fn refresh_poll_event_subscription(
    service: &Arc<AlsaService>,
    session: &mut AlsaEventSession,
) -> RuntimeResult<()> {
    refresh_event_subscription(service, session, MidiEventSource::SyntheticPoll)
}

/// Queue one backend-disconnected event into one subscription.
fn queue_backend_disconnected_event(
    session: &mut AlsaEventSession,
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

    push_event_with_overflow_policy(
        &session.queue,
        MidiEventValue::BackendDisconnected { metadata, flags },
        session.overflow_policy,
    )
}

/// Pop one queued event after honoring deferred overflow errors.
fn try_pop_session_event(
    session: &AlsaEventSession,
    operation: &'static str,
) -> RuntimeResult<Option<MidiEventValue>> {
    // deferred overflow
    take_event_overflow_error(&session.queue, operation)?;

    Ok(session.queue.try_pop())
}

/// Pop one queued event batch after honoring deferred overflow errors.
fn try_pop_session_event_batch(
    session: &AlsaEventSession,
    max_events: usize,
    operation: &'static str,
) -> RuntimeResult<Vec<MidiEventValue>> {
    // deferred overflow
    take_event_overflow_error(&session.queue, operation)?;

    Ok(session.queue.try_pop_batch(max_events))
}

/// Collect all live native event subscriptions from one registry.
fn collect_native_event_sessions(
    registry: &Arc<Mutex<AlsaNativeEventRegistry>>,
) -> Vec<Arc<Mutex<AlsaEventSession>>> {
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
}

/// Refresh all native event subscriptions from one backend topology mutation.
pub(super) fn refresh_native_event_sessions(
    topology: &Arc<Mutex<AlsaTopologyState>>,
    registry: &Arc<Mutex<AlsaNativeEventRegistry>>,
    source: MidiEventSource,
) {
    let Ok(service) = alsa_service("destack.midi.event.refresh") else {
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

    let sessions = collect_native_event_sessions(registry);
    for session in sessions {
        let mut session = session.lock();
        let _ = refresh_event_subscription(&service, &mut session, source);
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
    binding
        .agent()
        .platform_state
        .midi
        .mark_runtime_active(binding);

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

    let service = binding
        .agent()
        .platform_state
        .midi
        .alsa_service("destack.midi.event.open")?;
    let delivery_kind = match options.delivery_mode {
        MidiEventDeliveryMode::PollOnly => AlsaEventDeliveryKind::Poll,
        MidiEventDeliveryMode::Auto | MidiEventDeliveryMode::NativeOnly => {
            AlsaEventDeliveryKind::Poll
        }
    };

    let session = Arc::new(Mutex::new(AlsaEventSession {
        backend,
        direction_mask: options.direction_mask,
        flags: options.flags,
        overflow_policy: options.overflow_policy,
        poll_interval: event_poll_interval(options.poll_interval_ns),
        delivery_kind,
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
    }

    Ok(insert_event_resource(binding, session))
}

/// Close one ALSA sequencer event subscription.
pub(crate) fn midi_event_close(
    binding: &BindingCallContext,
    handle: resource::MidiEventHandle,
) -> RuntimeResult<()> {
    if let Ok(session) = event_resource(binding, handle, "destack.midi.event.close") {
        let session = session.lock();
        unregister_native_event_session(&session.delivery_kind);
    }

    remove_event_resource(binding, handle, "destack.midi.event.close")
}

/// Read one ALSA sequencer event.
pub(crate) fn midi_event_read(
    binding: &BindingCallContext,
    handle: resource::MidiEventHandle,
    timeout_ns: u64,
) -> RuntimeResult<MidiEventValue> {
    let session = event_resource(binding, handle, "destack.midi.event.read")?;
    let mut session = session.lock();

    // synthetic polling
    if matches!(session.delivery_kind, AlsaEventDeliveryKind::Poll) {
        let service = binding
            .agent()
            .platform_state
            .midi
            .alsa_service("destack.midi.event.read")?;
        let deadline = Instant::now()
            .checked_add(Duration::from_nanos(timeout_ns))
            .unwrap_or_else(Instant::now);

        loop {
            refresh_poll_event_subscription(&service, &mut session)?;

            if let Some(event) = try_pop_session_event(&session, "destack.midi.event.read")? {
                return Ok(event);
            }

            if Instant::now() >= deadline {
                return Err(core_platform::io_would_block(
                    "destack.midi.event.read",
                    "no queued MIDI topology event is available",
                ));
            }

            std::thread::sleep(session.poll_interval);
        }
    }

    drop(session);

    let deadline = Instant::now()
        .checked_add(Duration::from_nanos(timeout_ns))
        .unwrap_or_else(Instant::now);

    loop {
        let session = event_resource(binding, handle, "destack.midi.event.read")?;
        let session = session.lock();
        if let Some(event) = try_pop_session_event(&session, "destack.midi.event.read")? {
            return Ok(event);
        }

        if Instant::now() >= deadline {
            return Err(core_platform::io_would_block(
                "destack.midi.event.read",
                "no queued MIDI topology event is available",
            ));
        }

        std::thread::sleep(Duration::from_millis(1));
    }
}

/// Read one ALSA sequencer event batch.
pub(crate) fn midi_event_read_batch(
    binding: &BindingCallContext,
    handle: resource::MidiEventHandle,
    max_events: u32,
    timeout_ns: u64,
) -> RuntimeResult<Vec<MidiEventValue>> {
    let session = event_resource(binding, handle, "destack.midi.event.readBatch")?;
    let mut session = session.lock();

    // synthetic polling
    if matches!(session.delivery_kind, AlsaEventDeliveryKind::Poll) {
        let service = binding
            .agent()
            .platform_state
            .midi
            .alsa_service("destack.midi.event.readBatch")?;
        let deadline = Instant::now()
            .checked_add(Duration::from_nanos(timeout_ns))
            .unwrap_or_else(Instant::now);

        loop {
            refresh_poll_event_subscription(&service, &mut session)?;

            let events = try_pop_session_event_batch(
                &session,
                max_events as usize,
                "destack.midi.event.readBatch",
            )?;
            if !events.is_empty() {
                return Ok(events);
            }

            if Instant::now() >= deadline {
                return Err(core_platform::io_would_block(
                    "destack.midi.event.readBatch",
                    "no queued MIDI topology events are available",
                ));
            }

            std::thread::sleep(session.poll_interval);
        }
    }

    drop(session);

    let deadline = Instant::now()
        .checked_add(Duration::from_nanos(timeout_ns))
        .unwrap_or_else(Instant::now);

    loop {
        let session = event_resource(binding, handle, "destack.midi.event.readBatch")?;
        let session = session.lock();
        let events = try_pop_session_event_batch(
            &session,
            max_events as usize,
            "destack.midi.event.readBatch",
        )?;
        if !events.is_empty() {
            return Ok(events);
        }

        if Instant::now() >= deadline {
            return Err(core_platform::io_would_block(
                "destack.midi.event.readBatch",
                "no queued MIDI topology events are available",
            ));
        }

        std::thread::sleep(Duration::from_millis(1));
    }
}

/// Poll one ALSA sequencer event without blocking.
pub(crate) fn midi_event_try_read(
    binding: &BindingCallContext,
    handle: resource::MidiEventHandle,
) -> RuntimeResult<MidiEventValue> {
    let session = event_resource(binding, handle, "destack.midi.event.tryRead")?;
    let mut session = session.lock();

    // synthetic polling
    if matches!(session.delivery_kind, AlsaEventDeliveryKind::Poll) {
        let service = binding
            .agent()
            .platform_state
            .midi
            .alsa_service("destack.midi.event.tryRead")?;
        refresh_poll_event_subscription(&service, &mut session)?;
    }

    try_pop_session_event(&session, "destack.midi.event.tryRead")?.ok_or_else(|| {
        core_platform::io_would_block(
            "destack.midi.event.tryRead",
            "no queued MIDI topology event is available",
        )
    })
}

/// Poll one ALSA sequencer event batch without blocking.
pub(crate) fn midi_event_try_read_batch(
    binding: &BindingCallContext,
    handle: resource::MidiEventHandle,
    max_events: u32,
) -> RuntimeResult<Vec<MidiEventValue>> {
    let session = event_resource(binding, handle, "destack.midi.event.tryReadBatch")?;
    let mut session = session.lock();

    // synthetic polling
    if matches!(session.delivery_kind, AlsaEventDeliveryKind::Poll) {
        let service = binding
            .agent()
            .platform_state
            .midi
            .alsa_service("destack.midi.event.tryReadBatch")?;
        refresh_poll_event_subscription(&service, &mut session)?;
    }

    let events = try_pop_session_event_batch(
        &session,
        max_events as usize,
        "destack.midi.event.tryReadBatch",
    )?;
    if events.is_empty() {
        return Err(core_platform::io_would_block(
            "destack.midi.event.tryReadBatch",
            "no queued MIDI topology events are available",
        ));
    }

    Ok(events)
}
