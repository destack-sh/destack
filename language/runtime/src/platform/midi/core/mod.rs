#[cfg(any(
    target_os = "android",
    target_os = "linux",
    target_os = "macos",
    target_os = "ios",
    windows
))]
mod backend;
#[cfg(any(
    target_os = "android",
    target_os = "linux",
    target_os = "macos",
    target_os = "ios",
    windows
))]
mod descriptor;
#[cfg(any(
    target_os = "android",
    target_os = "linux",
    target_os = "macos",
    target_os = "ios",
    windows
))]
mod event;
#[cfg(any(
    target_os = "android",
    target_os = "linux",
    target_os = "macos",
    target_os = "ios",
    windows
))]
mod queue;
#[cfg(windows)]
pub(crate) use descriptor::descriptor_matches_list_flags;
#[cfg(any(target_os = "linux", windows))]
pub(crate) use descriptor::filter_direction_descriptor_rows;
#[cfg(any(target_os = "macos", target_os = "ios", target_os = "android"))]
pub(crate) use descriptor::filter_port_descriptors;
#[cfg(target_os = "android")]
pub(crate) use descriptor::resolve_descriptor_row;
#[cfg(any(target_os = "linux", windows))]
pub(crate) use descriptor::resolve_direction_descriptor_row;
#[cfg(any(target_os = "linux", target_os = "macos", target_os = "ios", windows))]
pub(crate) use event::{collect_live_event_sessions, push_backend_disconnected_event};
#[cfg(any(
    target_os = "android",
    target_os = "linux",
    target_os = "macos",
    target_os = "ios",
    windows
))]
pub(crate) use event::{refresh_snapshot_event_subscription, snapshot_key};
#[cfg(any(
    target_os = "android",
    target_os = "linux",
    target_os = "macos",
    target_os = "ios",
    windows
))]
mod resource;
mod store;
#[cfg(any(
    target_os = "android",
    target_os = "linux",
    target_os = "macos",
    target_os = "ios",
    windows
))]
mod validate;
mod value;

#[cfg(any(target_os = "android", target_os = "macos", target_os = "ios", windows))]
pub(crate) use crate::platform::core::monotonic_now_ns as binding_timestamp_now;
#[cfg(any(target_os = "linux", target_os = "macos", target_os = "ios", windows))]
pub(crate) use backend::single_midi_backend_metadata;
#[cfg(any(
    target_os = "android",
    target_os = "linux",
    target_os = "macos",
    target_os = "ios",
    windows
))]
pub(crate) use backend::{
    MidiBackendMetadata, midi_backend_descriptors, midi_backend_metadata, midi_selector_support,
    resolve_midi_backend_selection,
};
#[cfg(any(target_os = "linux", windows))]
pub(crate) use queue::surface_terminal_error;
#[cfg(any(
    target_os = "android",
    target_os = "linux",
    target_os = "macos",
    target_os = "ios",
    windows
))]
pub(crate) use queue::{
    event_poll_interval, event_queue_capacity, input_queue_capacity,
    push_event_with_overflow_policy, require_queued_event, require_queued_event_batch,
    try_pop_queued_event, try_pop_queued_event_batch,
};
#[cfg(any(target_os = "linux", target_os = "macos", target_os = "ios", windows))]
pub(crate) use queue::{
    read_queued_batch, read_queued_event, read_queued_event_batch, read_queued_item,
    try_read_queued_batch, try_read_queued_item,
};
#[cfg(any(target_os = "linux", target_os = "macos", target_os = "ios", windows))]
pub(crate) use resource::remove_labeled_resource;
pub(crate) use resource::{
    define_backend_midi_resource_accessors, define_backend_midi_resource_inserters,
    direction_mask_includes, endpoint_direction_name, event_snapshot_list_flags,
    insert_backend_midi_event_resource, insert_backend_midi_input_resource,
    insert_backend_midi_output_resource, read_backend_midi_event_resource_arc,
    read_backend_midi_input_resource_arc, read_backend_midi_output_resource_arc,
};
#[cfg(any(target_os = "android", target_os = "linux", windows))]
pub(crate) use resource::{
    remove_midi_event_resource, remove_midi_input_resource, remove_midi_output_resource,
};
#[cfg(any(target_os = "linux", windows))]
pub(crate) use resource::{
    require_midi_event_handle_backend, require_midi_input_handle_backend,
    require_midi_output_handle_backend,
};
pub(crate) use store::{
    store_backend_descriptors_native, store_backend_descriptors_vm, store_event_native,
    store_event_vm, store_events_native, store_events_vm, store_input_record_native,
    store_input_record_vm, store_input_records_native, store_input_records_vm,
    store_port_descriptors_native, store_port_descriptors_vm,
};
#[cfg(windows)]
pub(crate) use validate::{
    canonical_midi1_message_length, resolve_descriptor_open_transport,
    validate_output_record_payload, validate_record_shape,
};
#[cfg(any(
    target_os = "android",
    target_os = "linux",
    target_os = "macos",
    target_os = "ios"
))]
pub(crate) use validate::{
    resolve_descriptor_open_transport, validate_output_record_payload, validate_record_shape,
};
#[cfg(any(
    target_os = "android",
    target_os = "linux",
    target_os = "macos",
    target_os = "ios",
    windows
))]
pub(crate) use value::MidiEventMetadataValue;
#[cfg(any(target_os = "linux", target_os = "macos", target_os = "ios", windows))]
pub(crate) use value::MidiRecordBytes;
pub(crate) use value::{
    MidiBackendDescriptorValue, MidiEventValue, MidiInputRecordValue, MidiOutputRecordValue,
    MidiPortDescriptorValue,
};
