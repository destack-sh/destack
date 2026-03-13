use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Instant;

use parking_lot::Mutex;

use crate::diagnostic::RuntimeResult;
use crate::platform::core::{self as core_platform};
use crate::platform::midi::core::{
    MidiEventValue, MidiPortDescriptorValue, direction_mask_includes, event_poll_interval,
    event_queue_capacity, event_snapshot_list_flags, refresh_snapshot_event_subscription,
    remove_midi_event_resource, require_queued_event, require_queued_event_batch, snapshot_key,
    try_pop_queued_event, try_pop_queued_event_batch,
};
use crate::platform::midi::{
    MidiBackend, MidiEventDeliveryMode, MidiEventSource, MidiEventSubscriptionFlags,
    MidiEventSubscriptionOptions, MidiPortDirection, MidiPortDirectionFlags,
};
use crate::platform::resource;
use crate::runtime::BindingCallContext;
use crate::runtime::core::queue::BoundedQueue;

use super::core::{SnapshotKey, WinMmEventSession, insert_event_resource};
use super::descriptor::filtered_descriptors;
use super::resource::event_resource;

/// Build one current WinMM topology snapshot for one event subscription.
fn event_snapshot(
    service: &Arc<super::service::WinMmService>,
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

/// Refresh one WinMM event subscription from the current topology snapshot.
fn refresh_event_subscription(
    service: &Arc<super::service::WinMmService>,
    session: &mut WinMmEventSession,
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

/// Pop one queued event after honoring deferred overflow errors.
fn try_pop_session_event(
    session: &WinMmEventSession,
    operation: &'static str,
) -> RuntimeResult<Option<MidiEventValue>> {
    try_pop_queued_event(&session.queue, operation)
}

/// Pop one queued event batch after honoring deferred overflow errors.
fn try_pop_session_event_batch(
    session: &WinMmEventSession,
    max_events: usize,
    operation: &'static str,
) -> RuntimeResult<Vec<MidiEventValue>> {
    try_pop_queued_event_batch(&session.queue, max_events, operation)
}

/// Open one WinMM event subscription.
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

    // native delivery is unavailable
    if options.delivery_mode == MidiEventDeliveryMode::NativeOnly {
        return Err(core_platform::not_supported("destack.midi.event.open"));
    }

    // service and topology
    let service = binding
        .agent()
        .platform_state
        .midi
        .winmm_service("destack.midi.event.open")?;
    service.refresh_topology("destack.midi.event.open")?;

    let session = Arc::new(Mutex::new(WinMmEventSession {
        backend: MidiBackend::WinMM,
        direction_mask: options.direction_mask,
        flags: options.flags,
        overflow_policy: options.overflow_policy,
        poll_interval: event_poll_interval(options.poll_interval_ns),
        last_poll_at: None,
        queue: Arc::new(BoundedQueue::new(event_queue_capacity(
            options.queue_capacity,
        ))),
        next_sequence: 1,
        snapshot: event_snapshot(&service, options.flags, options.direction_mask),
    }));

    Ok(insert_event_resource(binding, session))
}

/// Close one WinMM event subscription.
pub(crate) fn midi_event_close(
    binding: &BindingCallContext,
    handle: resource::MidiEventHandle,
) -> RuntimeResult<()> {
    remove_midi_event_resource(binding, handle.0, "destack.midi.event.close", "midi event")
}

/// Wait for one WinMM topology event.
pub(crate) fn midi_event_read(
    binding: &BindingCallContext,
    handle: resource::MidiEventHandle,
    timeout_ns: u64,
) -> RuntimeResult<MidiEventValue> {
    let session = event_resource(binding, handle, "destack.midi.event.read")?;
    let service = binding
        .agent()
        .platform_state
        .midi
        .winmm_service("destack.midi.event.read")?;
    let deadline = core_platform::timeout_deadline(timeout_ns);

    loop {
        // poll refresh
        {
            let mut session = session.lock();
            let should_refresh = session
                .last_poll_at
                .map(|last_poll_at| last_poll_at.elapsed() >= session.poll_interval)
                .unwrap_or(true);
            if should_refresh {
                service.refresh_topology("destack.midi.event.read")?;
                refresh_event_subscription(&service, &mut session, MidiEventSource::SyntheticPoll)?;
                session.last_poll_at = Some(Instant::now());
            }

            if let Some(event) = try_pop_session_event(&session, "destack.midi.event.read")? {
                return Ok(event);
            }
        }

        // timeout
        if deadline.is_some_and(|deadline| Instant::now() >= deadline) {
            return Err(core_platform::io_would_block(
                "destack.midi.event.read",
                "no queued MIDI event is available",
            ));
        }

        std::thread::sleep(std::time::Duration::from_millis(1));
    }
}

/// Wait for one batch of WinMM topology events.
pub(crate) fn midi_event_read_batch(
    binding: &BindingCallContext,
    handle: resource::MidiEventHandle,
    max_events: u32,
    timeout_ns: u64,
) -> RuntimeResult<Vec<MidiEventValue>> {
    let session = event_resource(binding, handle, "destack.midi.event.readBatch")?;
    let service = binding
        .agent()
        .platform_state
        .midi
        .winmm_service("destack.midi.event.readBatch")?;
    let deadline = core_platform::timeout_deadline(timeout_ns);

    loop {
        // poll refresh
        {
            let mut session = session.lock();
            let should_refresh = session
                .last_poll_at
                .map(|last_poll_at| last_poll_at.elapsed() >= session.poll_interval)
                .unwrap_or(true);
            if should_refresh {
                service.refresh_topology("destack.midi.event.readBatch")?;
                refresh_event_subscription(&service, &mut session, MidiEventSource::SyntheticPoll)?;
                session.last_poll_at = Some(Instant::now());
            }

            let events = try_pop_session_event_batch(
                &session,
                max_events.max(1) as usize,
                "destack.midi.event.readBatch",
            )?;
            if !events.is_empty() {
                return Ok(events);
            }
        }

        // timeout
        if deadline.is_some_and(|deadline| Instant::now() >= deadline) {
            return Err(core_platform::io_would_block(
                "destack.midi.event.readBatch",
                "no queued MIDI events are available",
            ));
        }

        std::thread::sleep(std::time::Duration::from_millis(1));
    }
}

/// Poll one WinMM topology event.
pub(crate) fn midi_event_try_read(
    binding: &BindingCallContext,
    handle: resource::MidiEventHandle,
) -> RuntimeResult<MidiEventValue> {
    let session = event_resource(binding, handle, "destack.midi.event.tryRead")?;
    let service = binding
        .agent()
        .platform_state
        .midi
        .winmm_service("destack.midi.event.tryRead")?;

    // refresh topology
    let mut session = session.lock();
    service.refresh_topology("destack.midi.event.tryRead")?;
    refresh_event_subscription(&service, &mut session, MidiEventSource::SyntheticPoll)?;
    session.last_poll_at = Some(Instant::now());

    require_queued_event(
        try_pop_session_event(&session, "destack.midi.event.tryRead")?,
        "destack.midi.event.tryRead",
        "no queued MIDI event is available",
    )
}

/// Poll one batch of WinMM topology events.
pub(crate) fn midi_event_try_read_batch(
    binding: &BindingCallContext,
    handle: resource::MidiEventHandle,
    max_events: u32,
) -> RuntimeResult<Vec<MidiEventValue>> {
    let session = event_resource(binding, handle, "destack.midi.event.tryReadBatch")?;
    let service = binding
        .agent()
        .platform_state
        .midi
        .winmm_service("destack.midi.event.tryReadBatch")?;

    // refresh topology
    let mut session = session.lock();
    service.refresh_topology("destack.midi.event.tryReadBatch")?;
    refresh_event_subscription(&service, &mut session, MidiEventSource::SyntheticPoll)?;
    session.last_poll_at = Some(Instant::now());

    let events = try_pop_session_event_batch(
        &session,
        max_events.max(1) as usize,
        "destack.midi.event.tryReadBatch",
    )?;
    require_queued_event_batch(
        events,
        "destack.midi.event.tryReadBatch",
        "no queued MIDI events are available",
    )
}
