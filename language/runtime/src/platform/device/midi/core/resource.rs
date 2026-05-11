use std::sync::Arc;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::core::{self as core_platform};
use crate::platform::device::{
    MIDI_EVENT_SUBSCRIPTION_INCLUDE_DISCONNECTED, MIDI_EVENT_SUBSCRIPTION_INCLUDE_VIRTUAL,
    MIDI_PORT_DIRECTION_FLAG_INPUT, MIDI_PORT_DIRECTION_FLAG_OUTPUT,
    MIDI_PORT_LIST_INCLUDE_DISCONNECTED, MIDI_PORT_LIST_INCLUDE_VIRTUAL, MidiBackend,
    MidiEventSubscriptionFlags, MidiPortDirection, MidiPortDirectionFlags, MidiPortListFlags,
};
use crate::platform::resource::{ResourceEntry, ResourceId, ResourceKind, ResourceRoute};
use crate::runtime::BindingCallContext;

macro_rules! define_backend_midi_resource_inserters {
    (
        vis = $vis:vis,
        backend = $backend:path,
        input = ($input_fn:ident, $input_payload:ident, $input_session:path),
        output = ($output_fn:ident, $output_payload:ident, $output_session:path),
        event = ($event_fn:ident, $event_payload:ident, $event_session:path)
    ) => {
        /// Allocate one resource entry for one input session.
        $vis fn $input_fn(
            binding: &crate::runtime::BindingCallContext,
            session: std::sync::Arc<$input_session>,
        ) -> crate::platform::resource::MidiInputPortHandle {
            crate::platform::device::midi::core::insert_backend_midi_input_resource(
                binding,
                $backend,
                $input_payload { session },
            )
        }

        /// Allocate one resource entry for one output session.
        $vis fn $output_fn(
            binding: &crate::runtime::BindingCallContext,
            session: std::sync::Arc<$output_session>,
        ) -> crate::platform::resource::MidiOutputPortHandle {
            crate::platform::device::midi::core::insert_backend_midi_output_resource(
                binding,
                $backend,
                $output_payload { session },
            )
        }

        /// Allocate one resource entry for one event subscription.
        $vis fn $event_fn(
            binding: &crate::runtime::BindingCallContext,
            session: std::sync::Arc<parking_lot::Mutex<$event_session>>,
        ) -> crate::platform::resource::MidiEventHandle {
            crate::platform::device::midi::core::insert_backend_midi_event_resource(
                binding,
                $backend,
                $event_payload { session },
            )
        }
    };
}

pub(crate) use define_backend_midi_resource_inserters;

macro_rules! define_backend_midi_resource_accessors {
    (
        vis = $vis:vis,
        backend = $backend:path,
        input = ($input_fn:ident, $input_payload:ident, $input_session:path, $input_kind:literal),
        output = ($output_fn:ident, $output_payload:ident, $output_session:path, $output_kind:literal),
        event = ($event_fn:ident, $event_payload:ident, $event_session:path, $event_kind:literal)
    ) => {
        /// Read one input resource payload.
        $vis fn $input_fn(
            binding: &crate::runtime::BindingCallContext,
            handle: crate::platform::resource::MidiInputPortHandle,
            operation: &'static str,
        ) -> crate::diagnostic::RuntimeResult<std::sync::Arc<$input_session>> {
            crate::platform::device::midi::core::read_backend_midi_input_resource_arc(
                binding,
                handle.0,
                $backend,
                operation,
                $input_kind,
                |payload: &$input_payload| &payload.session,
            )
        }

        /// Read one output resource payload.
        $vis fn $output_fn(
            binding: &crate::runtime::BindingCallContext,
            handle: crate::platform::resource::MidiOutputPortHandle,
            operation: &'static str,
        ) -> crate::diagnostic::RuntimeResult<std::sync::Arc<$output_session>> {
            crate::platform::device::midi::core::read_backend_midi_output_resource_arc(
                binding,
                handle.0,
                $backend,
                operation,
                $output_kind,
                |payload: &$output_payload| &payload.session,
            )
        }

        /// Read one event resource payload.
        $vis fn $event_fn(
            binding: &crate::runtime::BindingCallContext,
            handle: crate::platform::resource::MidiEventHandle,
            operation: &'static str,
        ) -> crate::diagnostic::RuntimeResult<std::sync::Arc<parking_lot::Mutex<$event_session>>> {
            crate::platform::device::midi::core::read_backend_midi_event_resource_arc(
                binding,
                handle.0,
                $backend,
                operation,
                $event_kind,
                |payload: &$event_payload| &payload.session,
            )
        }
    };
}

pub(crate) use define_backend_midi_resource_accessors;

/// Resource label for one opened MIDI input session.
pub(crate) const MIDI_INPUT_RESOURCE_LABEL: &str = "midi.input.port";
/// Resource label for one opened MIDI output session.
pub(crate) const MIDI_OUTPUT_RESOURCE_LABEL: &str = "midi.output.port";
/// Resource label for one opened MIDI event subscription.
pub(crate) const MIDI_EVENT_RESOURCE_LABEL: &str = "midi.event";

/// Return one stable endpoint kind prefix.
pub(crate) fn endpoint_direction_name(direction: MidiPortDirection) -> &'static str {
    match direction {
        MidiPortDirection::Input => "input",
        MidiPortDirection::Output => "output",
    }
}

/// Return whether one direction mask includes one endpoint direction.
pub(crate) fn direction_mask_includes(
    mask: MidiPortDirectionFlags,
    direction: MidiPortDirection,
) -> bool {
    let direction_flag = match direction {
        MidiPortDirection::Input => MIDI_PORT_DIRECTION_FLAG_INPUT.0,
        MidiPortDirection::Output => MIDI_PORT_DIRECTION_FLAG_OUTPUT.0,
    };

    mask.0 & direction_flag != 0
}

/// Return port-list flags used to build one topology snapshot from event flags.
pub(crate) fn event_snapshot_list_flags(flags: MidiEventSubscriptionFlags) -> MidiPortListFlags {
    let mut list_flags = 0u32;

    // virtual endpoints are only included when the subscriber explicitly asks for them
    if flags.0 & MIDI_EVENT_SUBSCRIPTION_INCLUDE_VIRTUAL.0 != 0 {
        list_flags |= MIDI_PORT_LIST_INCLUDE_VIRTUAL.0;
    }

    // disconnected rows are opt-in so event snapshots stay tight by default
    if flags.0 & MIDI_EVENT_SUBSCRIPTION_INCLUDE_DISCONNECTED.0 != 0 {
        list_flags |= MIDI_PORT_LIST_INCLUDE_DISCONNECTED.0;
    }

    MidiPortListFlags(list_flags)
}

/// Build one runtime error for one missing resource handle.
pub(crate) fn missing_handle(
    operation: &'static str,
    handle_kind: &str,
    handle_id: ResourceId,
) -> Box<RuntimeError> {
    core_platform::io_not_found(
        operation,
        format!("{handle_kind} handle {} not found", handle_id.0),
    )
}

#[cfg(any(target_os = "linux", windows))]
/// Resolve the owning backend for one MIDI resource handle from its typed route.
pub(crate) fn midi_handle_backend(
    binding: &BindingCallContext,
    handle_id: ResourceId,
    kind: ResourceKind,
) -> Option<MidiBackend> {
    binding
        .worker()
        .resources
        .with_entry(handle_id, |entry| {
            if entry.kind != kind {
                return None;
            }

            entry
                .route
                .map(|ResourceRoute::MidiBackend(backend)| backend)
        })
        .flatten()
}

#[cfg(any(target_os = "linux", windows))]
/// Resolve the owning backend for one MIDI resource handle or fail loudly.
pub(crate) fn require_midi_handle_backend(
    binding: &BindingCallContext,
    handle_id: ResourceId,
    kind: ResourceKind,
    operation: &'static str,
    handle_kind: &str,
) -> RuntimeResult<MidiBackend> {
    match midi_handle_backend(binding, handle_id, kind) {
        Some(backend) => Ok(backend),
        None => Err(missing_handle(operation, handle_kind, handle_id)),
    }
}

#[cfg(any(target_os = "linux", windows))]
/// Resolve the owning backend for one MIDI input handle or fail loudly.
pub(crate) fn require_midi_input_handle_backend(
    binding: &BindingCallContext,
    handle_id: ResourceId,
    operation: &'static str,
) -> RuntimeResult<MidiBackend> {
    require_midi_handle_backend(
        binding,
        handle_id,
        ResourceKind::MidiInputPort,
        operation,
        "midi input port",
    )
}

#[cfg(any(target_os = "linux", windows))]
/// Resolve the owning backend for one MIDI output handle or fail loudly.
pub(crate) fn require_midi_output_handle_backend(
    binding: &BindingCallContext,
    handle_id: ResourceId,
    operation: &'static str,
) -> RuntimeResult<MidiBackend> {
    require_midi_handle_backend(
        binding,
        handle_id,
        ResourceKind::MidiOutputPort,
        operation,
        "midi output port",
    )
}

#[cfg(any(target_os = "linux", windows))]
/// Resolve the owning backend for one MIDI event handle or fail loudly.
pub(crate) fn require_midi_event_handle_backend(
    binding: &BindingCallContext,
    handle_id: ResourceId,
    operation: &'static str,
) -> RuntimeResult<MidiBackend> {
    require_midi_handle_backend(
        binding,
        handle_id,
        ResourceKind::MidiEvent,
        operation,
        "midi event",
    )
}

/// Read one typed session payload from one labeled resource entry.
pub(crate) fn read_routed_resource_payload<R, F>(
    binding: &BindingCallContext,
    handle_id: ResourceId,
    kind: ResourceKind,
    label: &str,
    route: Option<ResourceRoute>,
    operation: &'static str,
    handle_kind: &str,
    payload: F,
) -> RuntimeResult<R>
where
    F: FnOnce(&ResourceEntry) -> Option<R>,
{
    let mut payload = Some(payload);
    let session = binding
        .worker()
        .resources
        .with_entry(handle_id, |entry| {
            // reject entries from a different resource family
            if entry.kind != kind {
                return None;
            }

            // reject stale ids that point at a different label
            if entry.label.as_deref() != Some(label) {
                return None;
            }

            // reject entries that were allocated by a different route owner
            if entry.route != route {
                return None;
            }

            payload.take().and_then(|payload| payload(entry))
        })
        .flatten();

    match session {
        Some(session) => Ok(session),
        None => Err(missing_handle(operation, handle_kind, handle_id)),
    }
}

/// Remove one labeled resource entry or report one missing handle.
pub(crate) fn remove_labeled_resource(
    binding: &BindingCallContext,
    handle_id: ResourceId,
    operation: &'static str,
    handle_kind: &str,
) -> RuntimeResult<()> {
    let removed = binding.worker().resources.remove_and_finalize(
        binding.world(),
        handle_id,
        Some(binding.engine()),
    );

    match removed {
        true => Ok(()),
        false => Err(missing_handle(operation, handle_kind, handle_id)),
    }
}

/// Insert one backend-qualified MIDI resource entry.
pub(crate) fn insert_backend_midi_resource<H, P>(
    binding: &BindingCallContext,
    kind: ResourceKind,
    base_label: &str,
    backend: MidiBackend,
    payload: P,
    wrap: impl FnOnce(ResourceId) -> H,
) -> H
where
    P: Send + Sync + 'static,
{
    let entry = ResourceEntry::new(kind)
        .with_label(base_label)
        .with_route(ResourceRoute::MidiBackend(backend))
        .with_finalizer(
            binding
                .worker()
                .platform_state
                .device
                .retain_runtime_activity(),
        )
        .with_payload(payload);
    let resource_id =
        binding
            .worker()
            .resources
            .insert(binding.world(), entry, Some(binding.engine()));

    wrap(resource_id)
}

/// Insert one backend-qualified MIDI input resource entry.
pub(crate) fn insert_backend_midi_input_resource<P>(
    binding: &BindingCallContext,
    backend: MidiBackend,
    payload: P,
) -> crate::platform::resource::MidiInputPortHandle
where
    P: Send + Sync + 'static,
{
    insert_backend_midi_resource(
        binding,
        ResourceKind::MidiInputPort,
        MIDI_INPUT_RESOURCE_LABEL,
        backend,
        payload,
        crate::platform::resource::MidiInputPortHandle,
    )
}

/// Insert one backend-qualified MIDI output resource entry.
pub(crate) fn insert_backend_midi_output_resource<P>(
    binding: &BindingCallContext,
    backend: MidiBackend,
    payload: P,
) -> crate::platform::resource::MidiOutputPortHandle
where
    P: Send + Sync + 'static,
{
    insert_backend_midi_resource(
        binding,
        ResourceKind::MidiOutputPort,
        MIDI_OUTPUT_RESOURCE_LABEL,
        backend,
        payload,
        crate::platform::resource::MidiOutputPortHandle,
    )
}

/// Insert one backend-qualified MIDI event resource entry.
pub(crate) fn insert_backend_midi_event_resource<P>(
    binding: &BindingCallContext,
    backend: MidiBackend,
    payload: P,
) -> crate::platform::resource::MidiEventHandle
where
    P: Send + Sync + 'static,
{
    insert_backend_midi_resource(
        binding,
        ResourceKind::MidiEvent,
        MIDI_EVENT_RESOURCE_LABEL,
        backend,
        payload,
        crate::platform::resource::MidiEventHandle,
    )
}

/// Read one backend-qualified MIDI resource payload.
pub(crate) fn read_backend_midi_resource_payload<R, F>(
    binding: &BindingCallContext,
    handle_id: ResourceId,
    kind: ResourceKind,
    base_label: &str,
    backend: MidiBackend,
    operation: &'static str,
    handle_kind: &str,
    payload: F,
) -> RuntimeResult<R>
where
    F: FnOnce(&ResourceEntry) -> Option<R>,
{
    read_routed_resource_payload(
        binding,
        handle_id,
        kind,
        base_label,
        Some(ResourceRoute::MidiBackend(backend)),
        operation,
        handle_kind,
        payload,
    )
}

/// Read one backend-qualified MIDI resource payload that stores one shared `Arc` session.
pub(crate) fn read_backend_midi_resource_arc<P, S>(
    binding: &BindingCallContext,
    handle_id: ResourceId,
    kind: ResourceKind,
    base_label: &str,
    backend: MidiBackend,
    operation: &'static str,
    handle_kind: &str,
    session: impl FnOnce(&P) -> &Arc<S>,
) -> RuntimeResult<Arc<S>>
where
    P: Send + Sync + 'static,
    S: Send + Sync + 'static,
{
    let mut session = Some(session);

    read_backend_midi_resource_payload(
        binding,
        handle_id,
        kind,
        base_label,
        backend,
        operation,
        handle_kind,
        move |entry| {
            let session = session.take()?;

            entry
                .payload_ref::<P>()
                .map(|payload| session(payload).clone())
        },
    )
}

/// Read one backend-qualified MIDI input payload that stores one shared `Arc` session.
pub(crate) fn read_backend_midi_input_resource_arc<P, S>(
    binding: &BindingCallContext,
    handle_id: ResourceId,
    backend: MidiBackend,
    operation: &'static str,
    handle_kind: &str,
    session: impl FnOnce(&P) -> &Arc<S>,
) -> RuntimeResult<Arc<S>>
where
    P: Send + Sync + 'static,
    S: Send + Sync + 'static,
{
    read_backend_midi_resource_arc(
        binding,
        handle_id,
        ResourceKind::MidiInputPort,
        MIDI_INPUT_RESOURCE_LABEL,
        backend,
        operation,
        handle_kind,
        session,
    )
}

/// Read one backend-qualified MIDI output payload that stores one shared `Arc` session.
pub(crate) fn read_backend_midi_output_resource_arc<P, S>(
    binding: &BindingCallContext,
    handle_id: ResourceId,
    backend: MidiBackend,
    operation: &'static str,
    handle_kind: &str,
    session: impl FnOnce(&P) -> &Arc<S>,
) -> RuntimeResult<Arc<S>>
where
    P: Send + Sync + 'static,
    S: Send + Sync + 'static,
{
    read_backend_midi_resource_arc(
        binding,
        handle_id,
        ResourceKind::MidiOutputPort,
        MIDI_OUTPUT_RESOURCE_LABEL,
        backend,
        operation,
        handle_kind,
        session,
    )
}

/// Read one backend-qualified MIDI event payload that stores one shared `Arc` session.
pub(crate) fn read_backend_midi_event_resource_arc<P, S>(
    binding: &BindingCallContext,
    handle_id: ResourceId,
    backend: MidiBackend,
    operation: &'static str,
    handle_kind: &str,
    session: impl FnOnce(&P) -> &Arc<S>,
) -> RuntimeResult<Arc<S>>
where
    P: Send + Sync + 'static,
    S: Send + Sync + 'static,
{
    read_backend_midi_resource_arc(
        binding,
        handle_id,
        ResourceKind::MidiEvent,
        MIDI_EVENT_RESOURCE_LABEL,
        backend,
        operation,
        handle_kind,
        session,
    )
}

/// Remove one MIDI input resource entry or report one missing handle.
#[cfg(any(target_os = "android", target_os = "linux", windows))]
pub(crate) fn remove_midi_input_resource(
    binding: &BindingCallContext,
    handle_id: ResourceId,
    operation: &'static str,
    handle_kind: &str,
) -> RuntimeResult<()> {
    remove_labeled_resource(binding, handle_id, operation, handle_kind)
}

/// Remove one MIDI output resource entry or report one missing handle.
#[cfg(any(target_os = "android", target_os = "linux", windows))]
pub(crate) fn remove_midi_output_resource(
    binding: &BindingCallContext,
    handle_id: ResourceId,
    operation: &'static str,
    handle_kind: &str,
) -> RuntimeResult<()> {
    remove_labeled_resource(binding, handle_id, operation, handle_kind)
}

/// Remove one MIDI event resource entry or report one missing handle.
#[cfg(any(target_os = "android", target_os = "linux", windows))]
pub(crate) fn remove_midi_event_resource(
    binding: &BindingCallContext,
    handle_id: ResourceId,
    operation: &'static str,
    handle_kind: &str,
) -> RuntimeResult<()> {
    remove_labeled_resource(binding, handle_id, operation, handle_kind)
}

#[cfg(test)]
mod tests {
    use destack_core::{Capture, CaptureMode};

    use super::{
        MIDI_INPUT_RESOURCE_LABEL, insert_backend_midi_input_resource, remove_labeled_resource,
    };
    use crate::diagnostic::RuntimeError;
    use crate::platform::device::MidiBackend;
    use crate::tests::runtime::TestRuntime;

    /// Clear the MIDI capture barrier after the last live MIDI resource closes.
    #[test]
    fn test_midi_resource_lifetime_releases_runtime_activity_on_close() {
        let mut runtime = TestRuntime::deterministic_random();

        // baseline capture
        runtime
            .worker
            .platform_state
            .device
            .capture_image(CaptureMode::Suspend, ())
            .expect("midi-free platform midi capture should succeed");

        // insert one synthetic midi resource
        let handle = runtime.with_native_call_context(|binding| {
            insert_backend_midi_input_resource(binding, MidiBackend::CoreMIDI, ())
        });

        // active midi resource should block capture
        let error = runtime
            .worker
            .platform_state
            .device
            .capture_image(CaptureMode::Suspend, ())
            .expect_err("live midi resource should block platform midi capture");
        let RuntimeError::CaptureBarrier {
            component, detail, ..
        } = &*error
        else {
            panic!("expected midi capture barrier, got {error:?}");
        };
        assert_eq!(component, "platform.device");
        assert_eq!(detail, "runtime state is active");

        // closing the resource should release midi runtime activity
        runtime.with_native_call_context(|binding| {
            remove_labeled_resource(
                binding,
                handle.0,
                "destack.test.midi.input.close",
                MIDI_INPUT_RESOURCE_LABEL,
            )
            .expect("closing synthetic midi resource should succeed");
        });

        // capture should recover after the last midi resource closes
        runtime
            .worker
            .platform_state
            .device
            .capture_image(CaptureMode::Suspend, ())
            .expect("closing the last midi resource should clear the midi capture barrier");
    }
}
