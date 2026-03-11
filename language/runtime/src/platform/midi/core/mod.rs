#[cfg(any(target_os = "linux", target_os = "macos", windows))]
mod queue;
#[cfg(any(target_os = "linux", target_os = "macos", windows))]
mod resource;
mod store;
#[cfg(any(target_os = "linux", target_os = "macos", windows))]
mod support;
#[cfg(any(target_os = "linux", target_os = "macos", windows))]
mod validate;
mod value;

#[cfg(any(target_os = "linux", target_os = "macos", windows))]
pub(crate) use queue::{
    BoundedQueue, event_poll_interval, event_queue_capacity, input_queue_capacity,
    push_event_with_overflow_policy, take_event_overflow_error,
};
#[cfg(any(target_os = "linux", target_os = "macos", windows))]
pub(crate) use resource::{
    MIDI_EVENT_RESOURCE_LABEL, MIDI_INPUT_RESOURCE_LABEL, MIDI_OUTPUT_RESOURCE_LABEL,
    direction_mask_includes, endpoint_direction_name, event_snapshot_list_flags,
    read_labeled_resource_payload, remove_labeled_resource,
};
pub(crate) use store::{
    store_backend_descriptors_native, store_backend_descriptors_vm, store_event_native,
    store_event_vm, store_events_native, store_events_vm, store_input_record_native,
    store_input_record_vm, store_input_records_native, store_input_records_vm,
    store_port_descriptors_native, store_port_descriptors_vm,
};
#[cfg(any(target_os = "linux", target_os = "macos", windows))]
pub(crate) use support::binding_timestamp_now;
#[cfg(any(target_os = "linux", target_os = "macos", windows))]
pub(crate) use validate::{validate_output_record_payload, validate_record_shape};
#[cfg(any(target_os = "linux", target_os = "macos", windows))]
pub(crate) use value::MidiEventMetadataValue;
pub(crate) use value::{
    MidiBackendDescriptorValue, MidiEventValue, MidiInputRecordValue, MidiOutputRecordValue,
    MidiPortDescriptorValue,
};
