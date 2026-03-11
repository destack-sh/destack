use std::ptr;
use std::sync::Arc;

use core_foundation_sys::string::CFStringRef;

use crate::diagnostic::RuntimeResult;
use crate::platform::core::{self as core_platform};
use crate::platform::midi::core::MidiPortDescriptorValue;
use crate::platform::midi::{
    MIDI_DATA_FORMAT_FLAG_MIDI1_BYTES, MIDI_DATA_FORMAT_FLAG_UMP,
    MIDI_PORT_LIST_INCLUDE_DISCONNECTED, MIDI_PORT_LIST_INCLUDE_VIRTUAL, MIDI_PROTOCOL_FLAG_MIDI1,
    MIDI_PROTOCOL_FLAG_MIDI2, MidiBackend, MidiDataFormat, MidiDataFormatFlags, MidiPortDirection,
    MidiPortListFlags, MidiProtocol, MidiProtocolFlags,
};

use super::abi::{
    K_MIDI_OBJECT_TYPE_ENDPOINT, K_MIDI_PROTOCOL_2_0, MIDIEndpointGetEntity, MIDIEndpointRef,
    MIDIEntityGetDevice, MIDIGetDestination, MIDIGetNumberOfDestinations, MIDIGetNumberOfSources,
    MIDIGetSource, MIDIObjectFindByUniqueID, MIDIObjectGetIntegerProperty,
    MIDIObjectGetStringProperty, MIDIObjectRef, copy_cf_string, kMIDIPropertyDisplayName,
    kMIDIPropertyDriverVersion, kMIDIPropertyManufacturer, kMIDIPropertyModel, kMIDIPropertyName,
    kMIDIPropertyOffline, kMIDIPropertyProtocolID, kMIDIPropertyUniqueID, release_cf,
};
use super::core::{
    CoreMidiEndpointOverride, data_format_flag, endpoint_direction_name, exact_transport_support,
    protocol_flag,
};
use super::service::CoreMidiService;

/// Read one integer property from one CoreMIDI object.
fn integer_property(object: MIDIObjectRef, property: CFStringRef) -> Option<i32> {
    let mut value = 0i32;

    let status = unsafe { MIDIObjectGetIntegerProperty(object, property, &mut value) };
    if status != 0 {
        return None;
    }

    Some(value)
}

/// Read one string property from one CoreMIDI object.
fn string_property(object: MIDIObjectRef, property: CFStringRef) -> Option<String> {
    let mut value: CFStringRef = ptr::null();

    let status = unsafe { MIDIObjectGetStringProperty(object, property, &mut value) };
    if status != 0 || value.is_null() {
        return None;
    }

    let copied = copy_cf_string(value);
    release_cf(value.cast());
    copied
}

/// Return one stable numeric backend id for one endpoint.
fn endpoint_unique_id(endpoint: MIDIEndpointRef) -> i32 {
    integer_property(endpoint, unsafe { kMIDIPropertyUniqueID }).unwrap_or(endpoint as i32)
}

/// Return one stable runtime id for one endpoint.
fn endpoint_runtime_id(direction: MidiPortDirection, endpoint: MIDIEndpointRef) -> String {
    let unique_id = endpoint_unique_id(endpoint);

    format!(
        "coremidi:{}:{unique_id}",
        endpoint_direction_name(direction),
    )
}

/// Return one stable backend id for one endpoint.
fn endpoint_backend_id(endpoint: MIDIEndpointRef) -> String {
    let unique_id = endpoint_unique_id(endpoint);

    unique_id.to_string()
}

/// Return one group id and group name for one endpoint.
fn endpoint_group_identity(endpoint: MIDIEndpointRef) -> (Option<String>, Option<String>) {
    let mut entity = 0u32;

    let entity_status = unsafe { MIDIEndpointGetEntity(endpoint, &mut entity) };
    if entity_status != 0 || entity == 0 {
        return (None, None);
    }

    let mut device = 0u32;

    let device_status = unsafe { MIDIEntityGetDevice(entity, &mut device) };
    if device_status != 0 || device == 0 {
        let unique_id = integer_property(entity, unsafe { kMIDIPropertyUniqueID });
        let group_id = unique_id.map(|value| format!("coremidi:group:{value}"));
        let group_name = string_property(entity, unsafe { kMIDIPropertyDisplayName })
            .or_else(|| string_property(entity, unsafe { kMIDIPropertyName }));

        return (group_id, group_name);
    }

    let unique_id = integer_property(device, unsafe { kMIDIPropertyUniqueID });
    let group_id = unique_id.map(|value| format!("coremidi:group:{value}"));
    let group_name = string_property(device, unsafe { kMIDIPropertyDisplayName })
        .or_else(|| string_property(device, unsafe { kMIDIPropertyName }));

    (group_id, group_name)
}

/// Return supported formats and protocols for one endpoint.
fn endpoint_transport_support(
    service: &Arc<CoreMidiService>,
    endpoint: MIDIEndpointRef,
    is_virtual: bool,
) -> (
    MidiDataFormatFlags,
    Option<MidiDataFormat>,
    MidiProtocolFlags,
    Option<MidiProtocol>,
) {
    let unique_id = endpoint_unique_id(endpoint);
    if let Some(override_value) = service.endpoint_override(unique_id) {
        return exact_transport_support(override_value.data_format, override_value.protocol);
    }

    let Some(protocol_id) = integer_property(endpoint, unsafe { kMIDIPropertyProtocolID }) else {
        return exact_transport_support(MidiDataFormat::Midi1Bytes, MidiProtocol::Midi1);
    };

    if protocol_id == K_MIDI_PROTOCOL_2_0 {
        return (
            MIDI_DATA_FORMAT_FLAG_UMP,
            Some(MidiDataFormat::Ump),
            MidiProtocolFlags(MIDI_PROTOCOL_FLAG_MIDI1.0 | MIDI_PROTOCOL_FLAG_MIDI2.0),
            Some(MidiProtocol::Midi2),
        );
    }

    if is_virtual {
        return exact_transport_support(MidiDataFormat::Ump, MidiProtocol::Midi1);
    }

    (
        MidiDataFormatFlags(MIDI_DATA_FORMAT_FLAG_MIDI1_BYTES.0 | MIDI_DATA_FORMAT_FLAG_UMP.0),
        Some(MidiDataFormat::Ump),
        MIDI_PROTOCOL_FLAG_MIDI1,
        Some(MidiProtocol::Midi1),
    )
}

/// Register one transport override for one runtime-created virtual endpoint.
pub(super) fn register_endpoint_override(
    service: &Arc<CoreMidiService>,
    endpoint: MIDIEndpointRef,
    data_format: MidiDataFormat,
    protocol: MidiProtocol,
) {
    let unique_id = endpoint_unique_id(endpoint);
    let override_value = CoreMidiEndpointOverride {
        endpoint,
        data_format,
        protocol,
    };

    service.insert_endpoint_override(unique_id, override_value);
}

/// Remove one transport override for one disposed virtual endpoint.
pub(super) fn unregister_endpoint_override(
    service: &Arc<CoreMidiService>,
    endpoint: MIDIEndpointRef,
) {
    let unique_id = endpoint_unique_id(endpoint);
    service.remove_endpoint_override(unique_id);
}

/// Return whether one endpoint is virtual.
fn endpoint_is_virtual(endpoint: MIDIEndpointRef) -> bool {
    let mut entity = 0u32;

    unsafe { MIDIEndpointGetEntity(endpoint, &mut entity) != 0 || entity == 0 }
}

/// Return whether one endpoint is currently connected.
fn endpoint_is_connected(endpoint: MIDIEndpointRef) -> bool {
    let is_offline = integer_property(endpoint, unsafe { kMIDIPropertyOffline }).unwrap_or(0) != 0;

    !is_offline
}

/// Build one descriptor row for one endpoint.
pub(super) fn endpoint_descriptor(
    service: &Arc<CoreMidiService>,
    direction: MidiPortDirection,
    endpoint: MIDIEndpointRef,
) -> MidiPortDescriptorValue {
    let id = endpoint_runtime_id(direction, endpoint);
    let backend_id = Some(endpoint_backend_id(endpoint));
    let is_virtual = endpoint_is_virtual(endpoint);
    let (supported_data_formats, default_data_format, supported_protocols, default_protocol) =
        endpoint_transport_support(service, endpoint, is_virtual);
    let (group_id, group_name) = endpoint_group_identity(endpoint);

    MidiPortDescriptorValue {
        backend: MidiBackend::CoreMIDI,
        id,
        group_id,
        backend_id,
        name: string_property(endpoint, unsafe { kMIDIPropertyDisplayName })
            .or_else(|| string_property(endpoint, unsafe { kMIDIPropertyName }))
            .unwrap_or_else(|| "CoreMIDI Endpoint".to_string()),
        group_name,
        manufacturer: string_property(endpoint, unsafe { kMIDIPropertyManufacturer }),
        model: string_property(endpoint, unsafe { kMIDIPropertyModel }),
        version: integer_property(endpoint, unsafe { kMIDIPropertyDriverVersion })
            .map(|value| value.to_string()),
        supported_data_formats,
        default_data_format,
        supported_protocols,
        default_protocol,
        is_virtual,
        is_connected: endpoint_is_connected(endpoint),
    }
}

/// Enumerate endpoint descriptors for one direction.
fn enumerate_endpoint_descriptors(
    service: &Arc<CoreMidiService>,
    direction: MidiPortDirection,
) -> Vec<MidiPortDescriptorValue> {
    let count = match direction {
        MidiPortDirection::Input => unsafe { MIDIGetNumberOfSources() },
        MidiPortDirection::Output => unsafe { MIDIGetNumberOfDestinations() },
    };

    let mut descriptors = Vec::new();

    for index in 0..count {
        let endpoint = match direction {
            MidiPortDirection::Input => unsafe { MIDIGetSource(index) },
            MidiPortDirection::Output => unsafe { MIDIGetDestination(index) },
        };
        if endpoint == 0 {
            continue;
        }

        descriptors.push(endpoint_descriptor(service, direction, endpoint));
    }

    descriptors.sort_by(|left, right| left.id.cmp(&right.id));
    descriptors
}

/// Filter descriptors according to list flags.
pub(super) fn filtered_descriptors(
    service: &Arc<CoreMidiService>,
    direction: MidiPortDirection,
    flags: MidiPortListFlags,
) -> Vec<MidiPortDescriptorValue> {
    let include_virtual = flags.0 & MIDI_PORT_LIST_INCLUDE_VIRTUAL.0 != 0;
    let include_disconnected = flags.0 & MIDI_PORT_LIST_INCLUDE_DISCONNECTED.0 != 0;

    enumerate_endpoint_descriptors(service, direction)
        .into_iter()
        .filter(|descriptor| include_virtual || !descriptor.is_virtual)
        .filter(|descriptor| include_disconnected || descriptor.is_connected)
        .collect()
}

/// Resolve one endpoint object by stable runtime id.
pub(super) fn resolve_endpoint(
    service: &Arc<CoreMidiService>,
    direction: MidiPortDirection,
    id: &str,
    operation: &'static str,
) -> RuntimeResult<(MIDIEndpointRef, MidiPortDescriptorValue)> {
    for descriptor in enumerate_endpoint_descriptors(service, direction) {
        if descriptor.id != id {
            continue;
        }

        let Some(backend_id) = descriptor.backend_id.as_ref() else {
            break;
        };

        let unique_id = match backend_id.parse::<i32>() {
            Ok(unique_id) => unique_id,
            Err(_) => break,
        };

        let mut object = 0u32;
        let mut object_type = 0i32;

        let status = unsafe { MIDIObjectFindByUniqueID(unique_id, &mut object, &mut object_type) };
        if status != 0 || object == 0 || object_type != K_MIDI_OBJECT_TYPE_ENDPOINT {
            if let Some(override_value) = service.endpoint_override(unique_id) {
                return Ok((override_value.endpoint, descriptor));
            }

            return Err(core_platform::io_not_found(
                operation,
                format!("midi endpoint {id} is no longer available"),
            ));
        }

        return Ok((object as MIDIEndpointRef, descriptor));
    }

    Err(core_platform::io_not_found(
        operation,
        format!("midi endpoint {id} not found"),
    ))
}

/// Reject one requested transport shape that the descriptor does not advertise.
pub(super) fn validate_endpoint_transport_request(
    operation: &'static str,
    descriptor: &MidiPortDescriptorValue,
    data_format: MidiDataFormat,
    protocol: Option<MidiProtocol>,
) -> RuntimeResult<()> {
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
