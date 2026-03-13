use windows::Devices::Enumeration::DeviceInformation;
use windows::core::Error as WinError;

use std::sync::Arc;

use crate::diagnostic::RuntimeResult;
use crate::platform::midi::core::{
    MidiPortDescriptorValue, endpoint_direction_name, filter_direction_descriptor_rows,
    resolve_direction_descriptor_row,
};
use crate::platform::midi::{MidiBackend, MidiPortDirection, MidiPortListFlags};

use super::core::{WinRtEndpointInfo, exact_transport_support};
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
    service: &Arc<WinRtService>,
    direction: MidiPortDirection,
    id: &str,
    operation: &'static str,
) -> RuntimeResult<WinRtEndpointInfo> {
    resolve_direction_descriptor_row(
        direction,
        || input_descriptors(service),
        || output_descriptors(service),
        id,
        operation,
        |row| &row.descriptor,
    )
}
