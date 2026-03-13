use crate::diagnostic::RuntimeResult;
use crate::platform::core::{self as core_platform};
#[cfg(any(target_os = "linux", windows))]
use crate::platform::midi::MidiPortDirection;
use crate::platform::midi::{
    MIDI_DATA_FORMAT_FLAG_MIDI1_BYTES, MIDI_DATA_FORMAT_FLAG_UMP,
    MIDI_PORT_LIST_INCLUDE_DISCONNECTED, MIDI_PORT_LIST_INCLUDE_VIRTUAL, MIDI_PROTOCOL_FLAG_MIDI1,
    MIDI_PROTOCOL_FLAG_MIDI2, MidiDataFormat, MidiPortListFlags, MidiProtocol,
};

use super::MidiPortDescriptorValue;

/// Return whether one descriptor should appear in one list request.
pub(crate) fn descriptor_matches_list_flags(
    descriptor: &MidiPortDescriptorValue,
    flags: MidiPortListFlags,
) -> bool {
    let include_virtual = flags.0 & MIDI_PORT_LIST_INCLUDE_VIRTUAL.0 != 0;
    let include_disconnected = flags.0 & MIDI_PORT_LIST_INCLUDE_DISCONNECTED.0 != 0;

    (include_virtual || !descriptor.is_virtual) && (include_disconnected || descriptor.is_connected)
}

/// Filter one descriptor list according to one list request.
pub(crate) fn filter_port_descriptors(
    descriptors: impl IntoIterator<Item = MidiPortDescriptorValue>,
    flags: MidiPortListFlags,
) -> Vec<MidiPortDescriptorValue> {
    descriptors
        .into_iter()
        .filter(|descriptor| descriptor_matches_list_flags(descriptor, flags))
        .collect()
}

/// Filter one direction-scoped descriptor row set according to one list request.
#[cfg(any(target_os = "linux", windows))]
pub(crate) fn filter_direction_descriptor_rows<R>(
    direction: MidiPortDirection,
    input_rows: impl FnOnce() -> Vec<R>,
    output_rows: impl FnOnce() -> Vec<R>,
    flags: MidiPortListFlags,
    descriptor: impl FnMut(R) -> MidiPortDescriptorValue,
) -> Vec<MidiPortDescriptorValue> {
    let rows = match direction {
        MidiPortDirection::Input => input_rows(),
        MidiPortDirection::Output => output_rows(),
    };

    filter_port_descriptors(rows.into_iter().map(descriptor), flags)
}

/// Resolve one descriptor-bearing row by stable runtime id.
#[cfg(any(target_os = "android", target_os = "linux", windows))]
pub(crate) fn resolve_descriptor_row<R>(
    rows: impl IntoIterator<Item = R>,
    id: &str,
    operation: &'static str,
    descriptor: impl Fn(&R) -> &MidiPortDescriptorValue,
) -> RuntimeResult<R> {
    rows.into_iter()
        .find(|row| descriptor(row).id == id)
        .ok_or_else(|| {
            core_platform::io_not_found(operation, format!("midi endpoint {id} not found"))
        })
}

/// Resolve one direction-scoped descriptor row by stable runtime id.
#[cfg(any(target_os = "linux", windows))]
pub(crate) fn resolve_direction_descriptor_row<R>(
    direction: MidiPortDirection,
    input_rows: impl FnOnce() -> Vec<R>,
    output_rows: impl FnOnce() -> Vec<R>,
    id: &str,
    operation: &'static str,
    descriptor: impl Fn(&R) -> &MidiPortDescriptorValue,
) -> RuntimeResult<R> {
    let rows = match direction {
        MidiPortDirection::Input => input_rows(),
        MidiPortDirection::Output => output_rows(),
    };

    resolve_descriptor_row(rows, id, operation, descriptor)
}

/// Reject one requested transport shape that the descriptor does not advertise.
pub(crate) fn validate_descriptor_transport_request(
    operation: &'static str,
    descriptor: &MidiPortDescriptorValue,
    data_format: MidiDataFormat,
    protocol: Option<MidiProtocol>,
) -> RuntimeResult<()> {
    let requested_data_format_flag = match data_format {
        MidiDataFormat::Midi1Bytes => MIDI_DATA_FORMAT_FLAG_MIDI1_BYTES,
        MidiDataFormat::Ump => MIDI_DATA_FORMAT_FLAG_UMP,
    };

    if descriptor.supported_data_formats.0 & requested_data_format_flag.0 == 0 {
        return Err(core_platform::invalid_argument(
            "dataFormat",
            format!("{operation}: requested data format is not supported by the endpoint"),
        ));
    }

    if let Some(protocol) = protocol {
        let requested_protocol_flag = match protocol {
            MidiProtocol::Midi1 => MIDI_PROTOCOL_FLAG_MIDI1,
            MidiProtocol::Midi2 => MIDI_PROTOCOL_FLAG_MIDI2,
        };

        if descriptor.supported_protocols.0 & requested_protocol_flag.0 == 0 {
            return Err(core_platform::invalid_argument(
                "protocol",
                format!("{operation}: requested protocol is not supported by the endpoint"),
            ));
        }
    }

    Ok(())
}
