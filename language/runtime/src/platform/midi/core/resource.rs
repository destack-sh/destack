use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::core::{self as core_platform};
use crate::platform::midi::{
    MIDI_EVENT_SUBSCRIPTION_INCLUDE_DISCONNECTED, MIDI_EVENT_SUBSCRIPTION_INCLUDE_VIRTUAL,
    MIDI_PORT_DIRECTION_FLAG_INPUT, MIDI_PORT_DIRECTION_FLAG_OUTPUT,
    MIDI_PORT_LIST_INCLUDE_DISCONNECTED, MIDI_PORT_LIST_INCLUDE_VIRTUAL,
    MidiEventSubscriptionFlags, MidiPortDirection, MidiPortDirectionFlags, MidiPortListFlags,
};
use crate::platform::resource::{ResourceEntry, ResourceId, ResourceKind};
use crate::runtime::BindingCallContext;

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

/// Read one typed session payload from one labeled resource entry.
pub(crate) fn read_labeled_resource_payload<R, F>(
    binding: &BindingCallContext,
    handle_id: ResourceId,
    kind: ResourceKind,
    label: &'static str,
    operation: &'static str,
    handle_kind: &str,
    payload: F,
) -> RuntimeResult<R>
where
    F: FnOnce(&ResourceEntry) -> Option<R>,
{
    let mut payload = Some(payload);
    let session = binding
        .agent()
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
    let removed =
        binding
            .agent()
            .resources
            .remove(binding.world(), handle_id, Some(binding.engine()));

    match removed {
        Some(_) => Ok(()),
        None => Err(missing_handle(operation, handle_kind, handle_id)),
    }
}
