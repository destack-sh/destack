use std::collections::BTreeMap;
use std::sync::{Arc, Weak};

use parking_lot::Mutex;

use crate::diagnostic::RuntimeResult;
use crate::platform::core::{self as core_platform};
use crate::platform::device::midi::core::{
    MidiEventValue, MidiPortDescriptorValue, direction_mask_includes, event_queue_capacity,
    event_snapshot_list_flags, read_queued_event, read_queued_event_batch,
    refresh_snapshot_event_subscription, remove_midi_event_resource, require_queued_event,
    require_queued_event_batch, snapshot_key, try_pop_queued_event, try_pop_queued_event_batch,
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

use super::core::{SnapshotKey, WinMmEventRepository, insert_event_resource};
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
    session: &mut WinMmEventRepository,
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
    session: &mut WinMmEventRepository,
    source: MidiEventSource,
    flags: u32,
) -> RuntimeResult<()> {
    crate::platform::device::midi::core::push_backend_disconnected_event(
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
    session: &WinMmEventRepository,
    operation: &'static str,
) -> RuntimeResult<Option<MidiEventValue>> {
    try_pop_queued_event(&session.queue, operation)
}

/// Pop one queued event batch after honoring deferred overflow errors.
fn try_pop_session_event_batch(
    session: &WinMmEventRepository,
    max_events: usize,
    operation: &'static str,
) -> RuntimeResult<Vec<MidiEventValue>> {
    try_pop_queued_event_batch(&session.queue, max_events, operation)
}

/// Register one synthetic poll delivery for one WinMM event subscription.
fn register_poll_event_session(
    service: &Arc<super::service::WinMmService>,
    session: &Arc<Mutex<WinMmEventRepository>>,
    poll_interval: std::time::Duration,
) -> RuntimeResult<Arc<crate::runtime::service::executor::periodic::PeriodicTaskHandle>> {
    let service = service.clone();
    let session = Arc::downgrade(session);
    let is_failed = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let failure_state = is_failed.clone();

    let task = open_periodic_task(
        "destack-midi-winmm-event",
        super::service::WinMmService::POLICY,
        poll_interval,
        move || {
            if failure_state.load(std::sync::atomic::Ordering::Acquire) {
                return Ok(());
            }

            let Some(session) = Weak::upgrade(&session) else {
                return Ok(());
            };

            let refresh_result = service
                .refresh_topology("destack.device.midi.event.syntheticPoll")
                .and_then(|()| {
                    let mut session = session.lock();
                    refresh_event_subscription(
                        &service,
                        &mut session,
                        MidiEventSource::SyntheticPoll,
                    )
                });

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

/// Open one WinMM event subscription.
pub(crate) fn midi_event_open(
    binding: &BindingCallContext,
    options: MidiEventSubscriptionOptions,
) -> RuntimeResult<resource::MidiEventHandle> {
    // direction validation
    if options.direction_mask.0 == 0 {
        return Err(core_platform::invalid_argument(
            "directionMask",
            "destack.device.midi.event.open: directionMask must include at least one direction",
        ));
    }

    // native delivery is unavailable
    if options.delivery_mode == MidiEventDeliveryMode::NativeOnly {
        return Err(core_platform::not_supported(
            "destack.device.midi.event.open",
        ));
    }

    // service and topology
    let service = binding
        .worker()
        .platform_state
        .device
        .winmm_service("destack.device.midi.event.open")?;
    service.refresh_topology("destack.device.midi.event.open")?;

    let session = Arc::new(Mutex::new(WinMmEventRepository {
        backend: MidiBackend::WinMM,
        direction_mask: options.direction_mask,
        flags: options.flags,
        overflow_policy: options.overflow_policy,
        poll_task: None,
        queue: Arc::new(BoundedQueue::new(event_queue_capacity(
            options.queue_capacity,
        ))),
        next_sequence: 1,
        snapshot: event_snapshot(&service, options.flags, options.direction_mask),
    }));

    let poll_task = register_poll_event_session(
        &service,
        &session,
        crate::platform::device::midi::core::event_poll_interval(options.poll_interval_ns),
    )?;
    session.lock().poll_task = Some(poll_task);

    Ok(insert_event_resource(binding, session))
}

/// Close one WinMM event subscription.
pub(crate) fn midi_event_close(
    binding: &BindingCallContext,
    handle: resource::MidiEventHandle,
) -> RuntimeResult<()> {
    remove_midi_event_resource(
        binding,
        handle.0,
        "destack.device.midi.event.close",
        "midi event",
    )
}

/// Wait for one WinMM topology event.
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
        "no queued MIDI event is available",
    )
}

/// Wait for one batch of WinMM topology events.
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
        "no queued MIDI events are available",
    )
}

/// Poll one WinMM topology event.
pub(crate) fn midi_event_try_read(
    binding: &BindingCallContext,
    handle: resource::MidiEventHandle,
) -> RuntimeResult<MidiEventValue> {
    let session = event_resource(binding, handle, "destack.device.midi.event.tryRead")?;
    let session = session.lock();

    require_queued_event(
        try_pop_session_event(&session, "destack.device.midi.event.tryRead")?,
        "destack.device.midi.event.tryRead",
        "no queued MIDI event is available",
    )
}

/// Poll one batch of WinMM topology events.
pub(crate) fn midi_event_try_read_batch(
    binding: &BindingCallContext,
    handle: resource::MidiEventHandle,
    max_events: u32,
) -> RuntimeResult<Vec<MidiEventValue>> {
    let session = event_resource(binding, handle, "destack.device.midi.event.tryReadBatch")?;
    let session = session.lock();

    let events = try_pop_session_event_batch(
        &session,
        max_events.max(1) as usize,
        "destack.device.midi.event.tryReadBatch",
    )?;
    require_queued_event_batch(
        events,
        "destack.device.midi.event.tryReadBatch",
        "no queued MIDI events are available",
    )
}
