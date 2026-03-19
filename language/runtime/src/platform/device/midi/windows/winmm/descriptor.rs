use std::sync::Arc;

use windows_sys::Win32::Media::Audio::{MIDIINCAPSW, MIDIOUTCAPSW};

use crate::diagnostic::RuntimeResult;
use crate::platform::device::midi::core::{
    MidiPortDescriptorValue, filter_direction_descriptor_rows, resolve_direction_descriptor_row,
};
use crate::platform::device::{MidiBackend, MidiPortDirection, MidiPortListFlags};

use super::core::{
    WinMmEndpointInfo, exact_transport_support, input_runtime_id, output_runtime_id, wide_name,
};
use super::service::{WinMmService, input_descriptors, output_descriptors};

/// Return one copied WinMM endpoint name buffer.
fn caps_name(name: *const [u16; 32]) -> [u16; 32] {
    unsafe { name.read_unaligned() }
}

/// Build one WinMM input descriptor row.
pub(super) fn input_descriptor(device_id: u32, caps: &MIDIINCAPSW) -> MidiPortDescriptorValue {
    let (supported_data_formats, default_data_format, supported_protocols, default_protocol) =
        exact_transport_support();
    let name = caps_name(std::ptr::addr_of!(caps.szPname));

    MidiPortDescriptorValue {
        backend: MidiBackend::WinMM,
        id: input_runtime_id(device_id),
        group_id: None,
        backend_id: Some(device_id.to_string()),
        name: wide_name(&name),
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
    }
}

/// Build one WinMM output descriptor row.
pub(super) fn output_descriptor(device_id: u32, caps: &MIDIOUTCAPSW) -> MidiPortDescriptorValue {
    let (supported_data_formats, default_data_format, supported_protocols, default_protocol) =
        exact_transport_support();
    let name = caps_name(std::ptr::addr_of!(caps.szPname));

    MidiPortDescriptorValue {
        backend: MidiBackend::WinMM,
        id: output_runtime_id(device_id),
        group_id: None,
        backend_id: Some(device_id.to_string()),
        name: wide_name(&name),
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
    }
}

/// Filter cached descriptors according to list flags.
pub(super) fn filtered_descriptors(
    service: &Arc<WinMmService>,
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
    service: &Arc<WinMmService>,
    direction: MidiPortDirection,
    id: &str,
    operation: &'static str,
) -> RuntimeResult<WinMmEndpointInfo> {
    resolve_direction_descriptor_row(
        direction,
        || input_descriptors(service),
        || output_descriptors(service),
        id,
        operation,
        |row| &row.descriptor,
    )
}
