use std::sync::Arc;

use crate::diagnostic::RuntimeResult;
use crate::platform::device::midi::core::{
    MidiPortDescriptorValue, endpoint_direction_name, filter_direction_descriptor_rows,
    resolve_direction_descriptor_row,
};
use crate::platform::device::{
    MIDI_PROTOCOL_FLAG_MIDI1, MIDI_PROTOCOL_FLAG_MIDI2, MidiBackend, MidiPortDirection,
    MidiPortListFlags, MidiProtocol, MidiProtocolFlags,
};

use super::core::{WindowsMidiEndpointInfo, exact_transport_support};
use super::sdk::MidiEndpointDeviceInformation;
use super::service::{WindowsMidiService, input_descriptors, output_descriptors};

/// Return one preferred endpoint name.
fn endpoint_name(device: &MidiEndpointDeviceInformation) -> windows::core::Result<String> {
    let user_name = device.GetUserSuppliedInfo()?.Name.to_string();
    if !user_name.is_empty() {
        return Ok(user_name);
    }

    let declared_name = device.GetDeclaredEndpointInfo()?.Name.to_string();
    if !declared_name.is_empty() {
        return Ok(declared_name);
    }

    let endpoint_name = device.Name()?.to_string();
    if !endpoint_name.is_empty() {
        return Ok(endpoint_name);
    }

    Ok("Windows MIDI Endpoint".to_string())
}

/// Return one protocol-flag set for one endpoint.
fn endpoint_supported_protocols(
    device: &MidiEndpointDeviceInformation,
) -> windows::core::Result<MidiProtocolFlags> {
    let declared = device.GetDeclaredEndpointInfo()?;
    let mut flags = 0u32;

    if declared.SupportsMidi10Protocol {
        flags |= MIDI_PROTOCOL_FLAG_MIDI1.0;
    }

    if declared.SupportsMidi20Protocol {
        flags |= MIDI_PROTOCOL_FLAG_MIDI2.0;
    }

    Ok(MidiProtocolFlags(flags))
}

/// Return one stable container guid string.
pub(super) fn container_id_string(container_id: windows::core::GUID) -> String {
    format!(
        "{:08x}-{:04x}-{:04x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        container_id.data1,
        container_id.data2,
        container_id.data3,
        container_id.data4[0],
        container_id.data4[1],
        container_id.data4[2],
        container_id.data4[3],
        container_id.data4[4],
        container_id.data4[5],
        container_id.data4[6],
        container_id.data4[7],
    )
}

/// Build one stable runtime id for one Windows MIDI endpoint.
pub(super) fn runtime_id(direction: MidiPortDirection, backend_id: &str) -> String {
    let direction_name = endpoint_direction_name(direction);

    format!("windows-midi:{direction_name}:{backend_id}")
}

/// Return one exact protocol-flag set for one selected protocol.
fn exact_protocol_flags(protocol: MidiProtocol) -> MidiProtocolFlags {
    match protocol {
        MidiProtocol::Midi1 => MidiProtocolFlags(MIDI_PROTOCOL_FLAG_MIDI1.0),
        MidiProtocol::Midi2 => MidiProtocolFlags(MIDI_PROTOCOL_FLAG_MIDI2.0),
    }
}

/// Build one descriptor row from one Windows MIDI device-information record.
pub(super) fn device_descriptor(
    direction: MidiPortDirection,
    device: &MidiEndpointDeviceInformation,
) -> windows::core::Result<MidiPortDescriptorValue> {
    let backend_id = device.EndpointDeviceId()?.to_string();
    let supported_protocols = endpoint_supported_protocols(device)?;
    let (supported_data_formats, default_data_format, supported_protocols, default_protocol) =
        exact_transport_support(supported_protocols);

    Ok(MidiPortDescriptorValue {
        backend: MidiBackend::WindowsMidi,
        id: runtime_id(direction, &backend_id),
        group_id: Some(container_id_string(device.ContainerId()?)),
        backend_id: Some(backend_id),
        name: endpoint_name(device)?,
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

/// Build one descriptor for one runtime-created Windows MIDI virtual endpoint.
pub(super) fn virtual_device_descriptor(
    direction: MidiPortDirection,
    backend_id: String,
    association_id: windows::core::GUID,
    name: String,
    manufacturer: Option<String>,
    model: Option<String>,
    version: Option<String>,
    protocol: MidiProtocol,
) -> MidiPortDescriptorValue {
    let (supported_data_formats, default_data_format, supported_protocols, default_protocol) =
        exact_transport_support(exact_protocol_flags(protocol));

    MidiPortDescriptorValue {
        backend: MidiBackend::WindowsMidi,
        id: runtime_id(direction, &backend_id),
        group_id: Some(container_id_string(association_id)),
        backend_id: Some(backend_id),
        name,
        group_name: None,
        manufacturer,
        model,
        version,
        supported_data_formats,
        default_data_format,
        supported_protocols,
        default_protocol,
        is_virtual: true,
        is_connected: true,
    }
}

/// Filter cached descriptors according to list flags.
pub(super) fn filtered_descriptors(
    service: &Arc<WindowsMidiService>,
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
    service: &Arc<WindowsMidiService>,
    direction: MidiPortDirection,
    id: &str,
    operation: &'static str,
) -> RuntimeResult<WindowsMidiEndpointInfo> {
    resolve_direction_descriptor_row(
        direction,
        || input_descriptors(service),
        || output_descriptors(service),
        id,
        operation,
        |row| &row.descriptor,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Format Windows MIDI container ids as canonical lowercase guid strings.
    #[test]
    fn test_container_id_string_formats_lowercase_guid_output() {
        let guid = windows::core::GUID {
            data1: 0x1234ABCD,
            data2: 0x5678,
            data3: 0x9ABC,
            data4: [0xDE, 0xF0, 0x12, 0x34, 0x56, 0x78, 0x9A, 0xBC],
        };

        assert_eq!(
            container_id_string(guid),
            "1234abcd-5678-9abc-def0-123456789abc",
        );
    }

    /// Build stable Windows MIDI runtime ids with the expected direction prefixes.
    #[test]
    fn test_runtime_id_uses_direction_scoped_windows_midi_prefixes() {
        assert_eq!(
            runtime_id(MidiPortDirection::Input, "endpoint"),
            "windows-midi:input:endpoint",
        );
        assert_eq!(
            runtime_id(MidiPortDirection::Output, "endpoint"),
            "windows-midi:output:endpoint",
        );
    }
}
