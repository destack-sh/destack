use std::sync::Arc;

use crate::diagnostic::RuntimeResult;
use crate::platform::device::midi::core::{
    MidiPortDescriptorValue, filter_direction_descriptor_rows, resolve_direction_descriptor_row,
};
use crate::platform::device::{
    MidiBackend, MidiDataFormat, MidiPortDirection, MidiPortListFlags, MidiProtocol,
};

use super::core::{
    JackEndpointInfo, exact_transport_support, group_id, is_virtual_port, runtime_id,
    split_port_name,
};
use super::service::{JackService, input_descriptors, output_descriptors};

/// Build one descriptor row from one JACK full port name.
pub(super) fn endpoint_descriptor(
    direction: MidiPortDirection,
    backend_port_name: &str,
    port_flags: libc::c_ulong,
) -> MidiPortDescriptorValue {
    let (client_name, port_name) = split_port_name(backend_port_name);
    let (supported_data_formats, default_data_format, supported_protocols, default_protocol) =
        exact_transport_support();

    MidiPortDescriptorValue {
        backend: MidiBackend::JackMidi,
        id: runtime_id(direction, backend_port_name),
        group_id: Some(group_id(client_name)),
        backend_id: Some(backend_port_name.to_string()),
        name: port_name.to_string(),
        group_name: Some(client_name.to_string()),
        manufacturer: None,
        model: None,
        version: None,
        supported_data_formats,
        default_data_format,
        supported_protocols,
        default_protocol,
        is_virtual: is_virtual_port(port_flags),
        is_connected: true,
    }
}

/// Filter cached descriptors according to list flags.
pub(super) fn filtered_descriptors(
    service: &Arc<JackService>,
    direction: MidiPortDirection,
    flags: MidiPortListFlags,
) -> Vec<MidiPortDescriptorValue> {
    filter_direction_descriptor_rows(
        direction,
        || input_descriptors(service),
        || output_descriptors(service),
        flags,
        |row| row.descriptor,
    )
}

/// Resolve one cached endpoint row by stable runtime id.
pub(super) fn resolve_endpoint(
    service: &Arc<JackService>,
    direction: MidiPortDirection,
    id: &str,
    operation: &'static str,
) -> RuntimeResult<JackEndpointInfo> {
    resolve_direction_descriptor_row(
        direction,
        || input_descriptors(service),
        || output_descriptors(service),
        id,
        operation,
        |row| &row.descriptor,
    )
}

/// Build one opened-session descriptor for one virtual input destination.
pub(super) fn virtual_input_descriptor(
    backend_port_name: &str,
    data_format: MidiDataFormat,
    protocol: MidiProtocol,
) -> MidiPortDescriptorValue {
    let (client_name, port_name) = split_port_name(backend_port_name);
    let (supported_data_formats, default_data_format, supported_protocols, default_protocol) =
        exact_transport_support();
    debug_assert_eq!(default_data_format, Some(data_format));
    debug_assert_eq!(default_protocol, Some(protocol));

    MidiPortDescriptorValue {
        backend: MidiBackend::JackMidi,
        id: runtime_id(MidiPortDirection::Input, backend_port_name),
        group_id: Some(group_id(client_name)),
        backend_id: Some(backend_port_name.to_string()),
        name: port_name.to_string(),
        group_name: Some(client_name.to_string()),
        manufacturer: None,
        model: None,
        version: None,
        supported_data_formats,
        default_data_format,
        supported_protocols,
        default_protocol,
        is_virtual: true,
        is_connected: true,
    }
}

/// Build one opened-session descriptor for one virtual output source.
pub(super) fn virtual_output_descriptor(
    backend_port_name: &str,
    data_format: MidiDataFormat,
    protocol: MidiProtocol,
) -> MidiPortDescriptorValue {
    let (client_name, port_name) = split_port_name(backend_port_name);
    let (supported_data_formats, default_data_format, supported_protocols, default_protocol) =
        exact_transport_support();
    debug_assert_eq!(default_data_format, Some(data_format));
    debug_assert_eq!(default_protocol, Some(protocol));

    MidiPortDescriptorValue {
        backend: MidiBackend::JackMidi,
        id: runtime_id(MidiPortDirection::Output, backend_port_name),
        group_id: Some(group_id(client_name)),
        backend_id: Some(backend_port_name.to_string()),
        name: port_name.to_string(),
        group_name: Some(client_name.to_string()),
        manufacturer: None,
        model: None,
        version: None,
        supported_data_formats,
        default_data_format,
        supported_protocols,
        default_protocol,
        is_virtual: true,
        is_connected: true,
    }
}
