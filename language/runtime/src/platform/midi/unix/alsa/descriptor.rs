use std::sync::Arc;

use crate::diagnostic::RuntimeResult;
use crate::platform::core::{self as core_platform};
use crate::platform::midi::core::MidiPortDescriptorValue;
use crate::platform::midi::{
    MIDI_PORT_LIST_INCLUDE_DISCONNECTED, MidiBackend, MidiDataFormat, MidiPortDirection,
    MidiPortListFlags, MidiProtocol,
};

use super::core::{
    AlsaEndpointInfo, backend_id, exact_transport_support, group_id, is_virtual_port,
    parse_backend_id, runtime_id,
};
use super::service::{AlsaService, input_descriptors, output_descriptors};

/// Build one descriptor row from one ALSA client and port.
pub(super) fn endpoint_descriptor(
    direction: MidiPortDirection,
    client_id: i32,
    client_name: &str,
    port_id: i32,
    port_name: String,
    port_type: u32,
) -> MidiPortDescriptorValue {
    let backend_id = backend_id(client_id, port_id);
    let (supported_data_formats, default_data_format, supported_protocols, default_protocol) =
        exact_transport_support();

    MidiPortDescriptorValue {
        backend: MidiBackend::Alsa,
        id: runtime_id(direction, client_id, port_id),
        group_id: Some(group_id(client_id)),
        backend_id: Some(backend_id),
        name: port_name,
        group_name: Some(client_name.to_string()),
        manufacturer: None,
        model: None,
        version: None,
        supported_data_formats,
        default_data_format,
        supported_protocols,
        default_protocol,
        is_virtual: is_virtual_port(port_type),
        is_connected: true,
    }
}

/// Filter cached descriptors according to list flags.
pub(super) fn filtered_descriptors(
    service: &Arc<AlsaService>,
    direction: MidiPortDirection,
    flags: MidiPortListFlags,
) -> Vec<MidiPortDescriptorValue> {
    let include_disconnected = flags.0 & MIDI_PORT_LIST_INCLUDE_DISCONNECTED.0 != 0;

    let rows = match direction {
        MidiPortDirection::Input => input_descriptors(service),
        MidiPortDirection::Output => output_descriptors(service),
    };

    rows.into_iter()
        .map(|row| row.descriptor)
        .filter(|descriptor| include_disconnected || descriptor.is_connected)
        .collect()
}

/// Resolve one cached endpoint row by stable runtime id.
pub(super) fn resolve_endpoint(
    service: &Arc<AlsaService>,
    direction: MidiPortDirection,
    id: &str,
    operation: &'static str,
) -> RuntimeResult<AlsaEndpointInfo> {
    let rows = match direction {
        MidiPortDirection::Input => input_descriptors(service),
        MidiPortDirection::Output => output_descriptors(service),
    };

    rows.into_iter()
        .find(|row| row.descriptor.id == id)
        .ok_or_else(|| {
            core_platform::io_not_found(operation, format!("midi endpoint {id} not found"))
        })
}

/// Reject one requested transport shape that the descriptor does not advertise.
pub(super) fn validate_endpoint_transport_request(
    operation: &'static str,
    descriptor: &MidiPortDescriptorValue,
    data_format: MidiDataFormat,
    protocol: Option<MidiProtocol>,
) -> RuntimeResult<()> {
    let supported_data_format = descriptor.default_data_format == Some(data_format);
    if !supported_data_format {
        return Err(core_platform::invalid_argument(
            "dataFormat",
            format!("{operation}: requested data format is not supported by the endpoint"),
        ));
    }

    if let Some(protocol) = protocol
        && descriptor.default_protocol != Some(protocol)
    {
        return Err(core_platform::invalid_argument(
            "protocol",
            format!("{operation}: requested protocol is not supported by the endpoint"),
        ));
    }

    Ok(())
}

/// Build one opened-session descriptor for one virtual input destination.
pub(super) fn virtual_input_descriptor(
    name: String,
    client_id: i32,
    port_id: i32,
    data_format: MidiDataFormat,
    protocol: MidiProtocol,
) -> MidiPortDescriptorValue {
    let (supported_data_formats, default_data_format, supported_protocols, default_protocol) =
        exact_transport_support();
    debug_assert_eq!(default_data_format, Some(data_format));
    debug_assert_eq!(default_protocol, Some(protocol));

    MidiPortDescriptorValue {
        backend: MidiBackend::Alsa,
        id: runtime_id(MidiPortDirection::Input, client_id, port_id),
        group_id: Some(group_id(client_id)),
        backend_id: Some(backend_id(client_id, port_id)),
        name: name.clone(),
        group_name: Some(name),
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
    name: String,
    client_id: i32,
    port_id: i32,
    data_format: MidiDataFormat,
    protocol: MidiProtocol,
) -> MidiPortDescriptorValue {
    let (supported_data_formats, default_data_format, supported_protocols, default_protocol) =
        exact_transport_support();
    debug_assert_eq!(default_data_format, Some(data_format));
    debug_assert_eq!(default_protocol, Some(protocol));

    MidiPortDescriptorValue {
        backend: MidiBackend::Alsa,
        id: runtime_id(MidiPortDirection::Output, client_id, port_id),
        group_id: Some(group_id(client_id)),
        backend_id: Some(backend_id(client_id, port_id)),
        name: name.clone(),
        group_name: Some(name),
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

/// Parse one backend id into one client and port pair.
pub(super) fn endpoint_address(
    descriptor: &MidiPortDescriptorValue,
    operation: &'static str,
) -> RuntimeResult<(i32, i32)> {
    let Some(backend_id) = descriptor.backend_id.as_deref() else {
        return Err(core_platform::io_not_found(
            operation,
            "midi endpoint is missing one backend id",
        ));
    };

    parse_backend_id(backend_id, operation)
}
