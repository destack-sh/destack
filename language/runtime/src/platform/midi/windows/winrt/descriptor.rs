use windows::Devices::Enumeration::DeviceInformation;
use windows::core::Error as WinError;

use crate::platform::core::{self as core_platform};
use crate::platform::midi::core::MidiPortDescriptorValue;
use crate::platform::midi::{
    MIDI_PORT_LIST_INCLUDE_DISCONNECTED, MidiBackend, MidiDataFormat, MidiPortDirection,
    MidiPortListFlags, MidiProtocol,
};

use super::core::{
    WinRtEndpointInfo, data_format_flag, endpoint_direction_name, exact_transport_support,
    protocol_flag,
};
use super::service::{WinRtService, input_descriptors, output_descriptors};

/// Build one stable runtime id for one WinRT endpoint.
fn runtime_id(direction: MidiPortDirection, backend_id: &str) -> String {
    let direction_name = endpoint_direction_name(direction);

    format!("winrt:{direction_name}:{backend_id}")
}

/// Build one descriptor row from one WinRT device-information record.
pub(super) fn device_descriptor(
    direction: MidiPortDirection,
    device: &DeviceInformation,
) -> Result<MidiPortDescriptorValue, WinError> {
    let backend_id = device.Id()?.to_string();
    let name = device.Name()?.to_string();
    let (supported_data_formats, default_data_format, supported_protocols, default_protocol) =
        exact_transport_support();

    Ok(MidiPortDescriptorValue {
        backend: MidiBackend::WinRT,
        id: runtime_id(direction, &backend_id),
        group_id: None,
        backend_id: Some(backend_id),
        name,
        group_name: None,
        manufacturer: None,
        model: None,
        version: None,
        supported_data_formats,
        default_data_format,
        supported_protocols,
        default_protocol,
        is_virtual: false,
        is_connected: true,
    })
}

/// Filter cached descriptors according to list flags.
pub(super) fn filtered_descriptors(
    service: &std::sync::Arc<WinRtService>,
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
    service: &std::sync::Arc<WinRtService>,
    direction: MidiPortDirection,
    id: &str,
    operation: &'static str,
) -> crate::diagnostic::RuntimeResult<WinRtEndpointInfo> {
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
) -> crate::diagnostic::RuntimeResult<()> {
    let data_format_flag = data_format_flag(data_format);
    if descriptor.supported_data_formats.0 & data_format_flag.0 == 0 {
        return Err(core_platform::invalid_argument(
            "dataFormat",
            format!("{operation}: requested data format is not supported by the endpoint"),
        ));
    }

    if let Some(protocol) = protocol {
        let protocol_flag = protocol_flag(protocol);
        if descriptor.supported_protocols.0 & protocol_flag.0 == 0 {
            return Err(core_platform::invalid_argument(
                "protocol",
                format!("{operation}: requested protocol is not supported by the endpoint"),
            ));
        }
    }

    Ok(())
}
