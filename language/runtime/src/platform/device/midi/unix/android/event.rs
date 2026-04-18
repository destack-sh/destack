use std::collections::BTreeMap;
use std::sync::Arc;

use parking_lot::Mutex;

use crate::diagnostic::RuntimeResult;
use crate::platform::device::midi::core::{
    MidiEventValue, MidiPortDescriptorValue, direction_mask_includes, event_poll_interval,
    event_queue_capacity, event_snapshot_list_flags, push_backend_disconnected_event,
    read_queued_event, read_queued_event_batch, refresh_snapshot_event_subscription,
    remove_midi_event_resource, require_queued_event, require_queued_event_batch,
    try_pop_queued_event, try_pop_queued_event_batch,
};
use crate::platform::device::{
    MIDI_BACKEND_CAP_NATIVE_EVENT_FEED, MidiBackend, MidiEventDeliveryMode, MidiEventSource,
    MidiEventSubscriptionFlags, MidiEventSubscriptionOptions, MidiPortDirection,
    MidiPortDirectionFlags,
};
use crate::platform::{core as core_platform, resource};
use crate::runtime::BindingCallContext;
use crate::runtime::control::queue::BoundedQueue;
use crate::runtime::process::RuntimeScheduledCallbackHandle;

use super::backend::resolve_backend;
use super::core::{
    AndroidEventDeliveryKind, AndroidEventRepository, SnapshotKey, close_event_session,
    insert_event_resource, open_event_session, read_native_events, snapshot_key,
};
use super::input::list_input_descriptors;
use super::output::list_output_descriptors;
use super::resource::event_resource;

/// Build one current Android topology snapshot.
fn event_snapshot(
    binding: &BindingCallContext,
    flags: MidiEventSubscriptionFlags,
    direction_mask: MidiPortDirectionFlags,
    operation: &'static str,
) -> RuntimeResult<BTreeMap<SnapshotKey, (MidiPortDirection, MidiPortDescriptorValue)>> {
    let list_flags = event_snapshot_list_flags(flags);
    let mut snapshot = BTreeMap::new();

    // input rows
    if direction_mask_includes(direction_mask, MidiPortDirection::Input) {
        for descriptor in list_input_descriptors(binding, list_flags, operation)? {
            let key = snapshot_key(MidiPortDirection::Input, &descriptor.id);
            snapshot.insert(key, (MidiPortDirection::Input, descriptor));
        }
    }

    // output rows
    if direction_mask_includes(direction_mask, MidiPortDirection::Output) {
        for descriptor in list_output_descriptors(binding, list_flags, operation)? {
            let key = snapshot_key(MidiPortDirection::Output, &descriptor.id);
            snapshot.insert(key, (MidiPortDirection::Output, descriptor));
        }
    }

    Ok(snapshot)
}

/// Refresh one Android event subscription from the current snapshot.
fn refresh_event_subscription(
    binding: &BindingCallContext,
    session: &mut AndroidEventRepository,
    source: MidiEventSource,
    operation: &'static str,
) -> RuntimeResult<()> {
    let next_snapshot = event_snapshot(binding, session.flags, session.direction_mask, operation)?;
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
    session: &mut AndroidEventRepository,
    source: MidiEventSource,
    flags: u32,
) -> RuntimeResult<()> {
    push_backend_disconnected_event(
        &session.queue,
        session.backend,
        session.overflow_policy,
        &mut session.next_sequence,
        source,
        flags,
    )
}

/// Register one synthetic poll callback on the owning runtime thread.
fn register_poll_event_session(
    binding: &BindingCallContext,
    session: &Arc<Mutex<AndroidEventRepository>>,
    poll_interval_ns: u64,
) -> RuntimeResult<RuntimeScheduledCallbackHandle> {
    let session = session.clone();

    binding.worker().schedule_runtime_callback(
        binding,
        poll_interval_ns,
        Some(poll_interval_ns),
        move |binding| {
            let mut session = session.lock();

            let refresh_result = refresh_event_subscription(
                binding,
                &mut session,
                MidiEventSource::SyntheticPoll,
                "destack.device.midi.event.syntheticPoll",
            );
            if refresh_result.is_ok() {
                return Ok(crate::runtime::process::RuntimeScheduledCallbackControl::Keep);
            }

            queue_backend_disconnected_event(&mut session, MidiEventSource::SyntheticPoll, 0)?;

            Ok(crate::runtime::process::RuntimeScheduledCallbackControl::Cancel)
        },
    )
}

/// Pop one queued event after surfacing deferred overflow.
fn try_pop_session_event(
    session: &AndroidEventRepository,
    operation: &'static str,
) -> RuntimeResult<Option<MidiEventValue>> {
    try_pop_queued_event(&session.queue, operation)
}

/// Pop one queued event batch after surfacing deferred overflow.
fn try_pop_session_event_batch(
    session: &AndroidEventRepository,
    max_events: usize,
    operation: &'static str,
) -> RuntimeResult<Vec<MidiEventValue>> {
    try_pop_queued_event_batch(&session.queue, max_events, operation)
}

/// Open one Android MIDI event subscription.
pub(crate) fn midi_event_open(
    binding: &BindingCallContext,
    options: MidiEventSubscriptionOptions,
) -> RuntimeResult<resource::MidiEventHandle> {
    // backend selection
    let backend = resolve_backend(
        binding,
        options.backend,
        options.backend_policy,
        "destack.device.midi.event.open",
    )?;

    debug_assert_eq!(backend, MidiBackend::AndroidMidi);

    // direction validation
    if options.direction_mask.0 == 0 {
        return Err(core_platform::invalid_argument(
            "directionMask",
            "destack.device.midi.event.open: directionMask must include at least one direction",
        ));
    }

    // prefer host-native topology delivery when the backend advertises it
    let description = binding
        .worker()
        .platform_state
        .device
        .describe_android_backend(binding, "destack.device.midi.event.open")?;
    let has_native_event_feed =
        description.capability_flags.0 & MIDI_BACKEND_CAP_NATIVE_EVENT_FEED.0 != 0;
    let delivery_kind = match options.delivery_mode {
        MidiEventDeliveryMode::PollOnly => AndroidEventDeliveryKind::Poll,
        MidiEventDeliveryMode::Auto if !has_native_event_feed => AndroidEventDeliveryKind::Poll,
        MidiEventDeliveryMode::NativeOnly if !has_native_event_feed => {
            return Err(core_platform::not_supported(
                "destack.device.midi.event.open",
            ));
        }
        MidiEventDeliveryMode::Auto | MidiEventDeliveryMode::NativeOnly => {
            let host_session_id = open_event_session(
                binding,
                options.flags,
                options.direction_mask,
                "destack.device.midi.event.open",
            )?;

            AndroidEventDeliveryKind::Native { host_session_id }
        }
    };

    let session = Arc::new(Mutex::new(AndroidEventRepository {
        backend: MidiBackend::AndroidMidi,
        direction_mask: options.direction_mask,
        flags: options.flags,
        overflow_policy: options.overflow_policy,
        delivery_kind,
        poll_interval: event_poll_interval(options.poll_interval_ns),
        poll_callback: None,
        queue: Arc::new(BoundedQueue::new(event_queue_capacity(
            options.queue_capacity,
        ))),
        next_sequence: 1,
        snapshot: event_snapshot(
            binding,
            options.flags,
            options.direction_mask,
            "destack.device.midi.event.open",
        )?,
    }));

    if matches!(session.lock().delivery_kind, AndroidEventDeliveryKind::Poll) {
        let poll_interval_ns = session.lock().poll_interval.as_nanos() as u64;
        let poll_callback = register_poll_event_session(binding, &session, poll_interval_ns)?;
        session.lock().poll_callback = Some(poll_callback);
    }

    Ok(insert_event_resource(binding, session))
}

/// Close one Android MIDI event subscription.
pub(crate) fn midi_event_close(
    binding: &BindingCallContext,
    handle: resource::MidiEventHandle,
) -> RuntimeResult<()> {
    if let Ok(session) = event_resource(binding, handle, "destack.device.midi.event.close") {
        let session = session.lock();

        // close the host-native subscription before dropping the resource
        if let AndroidEventDeliveryKind::Native { host_session_id } = session.delivery_kind {
            close_event_session(binding, host_session_id, "destack.device.midi.event.close")?;
        }

        // cancel the synthetic poll callback before dropping the resource
        if let Some(poll_callback) = session.poll_callback {
            binding
                .worker()
                .cancel_runtime_callback(binding, poll_callback)?;
        }
    }

    remove_midi_event_resource(
        binding,
        handle.0,
        "destack.device.midi.event.close",
        "midi event",
    )
}

/// Wait for one Android MIDI topology event.
pub(crate) fn midi_event_read(
    binding: &BindingCallContext,
    handle: resource::MidiEventHandle,
    timeout_ns: u64,
) -> RuntimeResult<MidiEventValue> {
    let session = event_resource(binding, handle, "destack.device.midi.event.read")?;
    let mut session = session.lock();

    // host-native delivery
    if let AndroidEventDeliveryKind::Native { host_session_id } = session.delivery_kind {
        let mut events = read_native_events(
            binding,
            host_session_id,
            1,
            timeout_ns,
            &mut session.next_sequence,
            "destack.device.midi.event.read",
        )?;
        if let Some(event) = events.pop() {
            return Ok(event);
        }

        return Err(core_platform::io_would_block(
            "destack.device.midi.event.read",
            "no queued MIDI event is available",
        ));
    }

    read_queued_event(
        &session.queue,
        timeout_ns,
        "destack.device.midi.event.read",
        "no queued MIDI event is available",
    )
}

/// Wait for one batch of Android MIDI topology events.
pub(crate) fn midi_event_read_batch(
    binding: &BindingCallContext,
    handle: resource::MidiEventHandle,
    max_events: u32,
    timeout_ns: u64,
) -> RuntimeResult<Vec<MidiEventValue>> {
    let session = event_resource(binding, handle, "destack.device.midi.event.readBatch")?;
    let mut session = session.lock();

    // host-native delivery
    if let AndroidEventDeliveryKind::Native { host_session_id } = session.delivery_kind {
        let events = read_native_events(
            binding,
            host_session_id,
            max_events.max(1),
            timeout_ns,
            &mut session.next_sequence,
            "destack.device.midi.event.readBatch",
        )?;
        if !events.is_empty() {
            return Ok(events);
        }

        return Err(core_platform::io_would_block(
            "destack.device.midi.event.readBatch",
            "no queued MIDI events are available",
        ));
    }

    read_queued_event_batch(
        &session.queue,
        max_events.max(1) as usize,
        timeout_ns,
        "destack.device.midi.event.readBatch",
        "no queued MIDI events are available",
    )
}

/// Poll one Android MIDI topology event without blocking.
pub(crate) fn midi_event_try_read(
    binding: &BindingCallContext,
    handle: resource::MidiEventHandle,
) -> RuntimeResult<MidiEventValue> {
    let session = event_resource(binding, handle, "destack.device.midi.event.tryRead")?;
    let mut session = session.lock();

    // host-native delivery
    if let AndroidEventDeliveryKind::Native { host_session_id } = session.delivery_kind {
        let mut events = read_native_events(
            binding,
            host_session_id,
            1,
            0,
            &mut session.next_sequence,
            "destack.device.midi.event.tryRead",
        )?;
        return require_queued_event(
            events.pop(),
            "destack.device.midi.event.tryRead",
            "no queued MIDI event is available",
        );
    }

    require_queued_event(
        try_pop_session_event(&session, "destack.device.midi.event.tryRead")?,
        "destack.device.midi.event.tryRead",
        "no queued MIDI event is available",
    )
}

/// Poll one batch of Android MIDI topology events without blocking.
pub(crate) fn midi_event_try_read_batch(
    binding: &BindingCallContext,
    handle: resource::MidiEventHandle,
    max_events: u32,
) -> RuntimeResult<Vec<MidiEventValue>> {
    let session = event_resource(binding, handle, "destack.device.midi.event.tryReadBatch")?;
    let mut session = session.lock();

    // host-native delivery
    if let AndroidEventDeliveryKind::Native { host_session_id } = session.delivery_kind {
        let events = read_native_events(
            binding,
            host_session_id,
            max_events.max(1),
            0,
            &mut session.next_sequence,
            "destack.device.midi.event.tryReadBatch",
        )?;
        return require_queued_event_batch(
            events,
            "destack.device.midi.event.tryReadBatch",
            "no queued MIDI events are available",
        );
    }

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
