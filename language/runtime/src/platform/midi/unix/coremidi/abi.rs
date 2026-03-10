use std::ffi::c_void;

use block2::Block;
use core_foundation_sys::base::{CFIndex, CFRelease, CFTypeRef, OSStatus, kCFAllocatorDefault};
use core_foundation_sys::string::{
    CFStringCreateWithBytes, CFStringGetCString, CFStringGetLength,
    CFStringGetMaximumSizeForEncoding, CFStringRef, kCFStringEncodingUTF8,
};
use objc2::encode::{Encoding, RefEncode};

/// CoreMIDI client reference.
pub(super) type MIDIClientRef = u32;
/// CoreMIDI port reference.
pub(super) type MIDIPortRef = u32;
/// CoreMIDI endpoint reference.
pub(super) type MIDIEndpointRef = u32;
/// CoreMIDI object reference.
pub(super) type MIDIObjectRef = u32;
/// CoreMIDI entity reference.
pub(super) type MIDIEntityRef = u32;
/// CoreMIDI device reference.
pub(super) type MIDIDeviceRef = u32;
/// CoreMIDI unique identifier.
pub(super) type MIDIUniqueID = i32;
/// CoreMIDI object type.
pub(super) type MIDIObjectType = i32;
/// CoreMIDI protocol identifier.
pub(super) type MIDIProtocolID = i32;
/// CoreMIDI notification message identifier.
pub(super) type MIDINotificationMessageID = i32;
/// CoreMIDI timestamp type.
pub(super) type MIDITimeStamp = u64;
/// CoreMIDI item count.
pub(super) type ItemCount = usize;

/// CoreMIDI protocol id for MIDI 1 semantics.
pub(super) const K_MIDI_PROTOCOL_1_0: MIDIProtocolID = 1;
/// CoreMIDI protocol id for MIDI 2 semantics.
pub(super) const K_MIDI_PROTOCOL_2_0: MIDIProtocolID = 2;

/// CoreMIDI object type for endpoints.
pub(super) const K_MIDI_OBJECT_TYPE_ENDPOINT: MIDIObjectType = 3;
/// CoreMIDI setup changed notification id.
pub(super) const K_MIDI_MSG_SETUP_CHANGED: MIDINotificationMessageID = 1;
/// CoreMIDI object added notification id.
pub(super) const K_MIDI_MSG_OBJECT_ADDED: MIDINotificationMessageID = 2;
/// CoreMIDI object removed notification id.
pub(super) const K_MIDI_MSG_OBJECT_REMOVED: MIDINotificationMessageID = 3;
/// CoreMIDI property changed notification id.
pub(super) const K_MIDI_MSG_PROPERTY_CHANGED: MIDINotificationMessageID = 4;
/// CoreMIDI I/O error notification id.
pub(super) const K_MIDI_MSG_IO_ERROR: MIDINotificationMessageID = 7;

/// One legacy CoreMIDI packet.
#[repr(C, packed(4))]
pub(super) struct MIDIPacket {
    /// Timestamp for the first byte in the packet.
    pub time_stamp: MIDITimeStamp,
    /// Number of valid bytes in the data payload.
    pub length: u16,
    /// Inline packet data storage.
    pub data: [u8; 256],
}

/// One legacy CoreMIDI packet list.
#[repr(C, packed(4))]
pub(super) struct MIDIPacketList {
    /// Number of packets in the list.
    pub num_packets: u32,
    /// First packet payload.
    pub packet: [MIDIPacket; 1],
}

/// One modern CoreMIDI event packet.
#[repr(C, packed(4))]
pub(super) struct MIDIEventPacket {
    /// Timestamp shared by the words in the packet.
    pub time_stamp: MIDITimeStamp,
    /// Number of valid words in the packet.
    pub word_count: u32,
    /// Inline word payload storage.
    pub words: [u32; 64],
}

/// One modern CoreMIDI event list.
#[repr(C, packed(4))]
pub(super) struct MIDIEventList {
    /// Protocol identifier for the list.
    pub protocol: MIDIProtocolID,
    /// Number of packets in the list.
    pub num_packets: u32,
    /// First packet payload.
    pub packet: [MIDIEventPacket; 1],
}

unsafe impl RefEncode for MIDIEventList {
    const ENCODING_REF: Encoding = Encoding::Pointer(&Encoding::Unknown);
}

/// One CoreMIDI notification header.
#[repr(C)]
pub(super) struct MIDINotification {
    /// Notification message kind.
    pub message_id: MIDINotificationMessageID,
    /// Total message size.
    pub message_size: u32,
}

unsafe impl RefEncode for MIDINotification {
    const ENCODING_REF: Encoding = Encoding::Pointer(&Encoding::Unknown);
}

/// One CoreMIDI object add or remove notification.
#[repr(C)]
pub(super) struct MIDIObjectAddRemoveNotification {
    /// Notification message kind.
    pub message_id: MIDINotificationMessageID,
    /// Total message size.
    pub message_size: u32,
    /// Parent object reference when present.
    pub parent: MIDIObjectRef,
    /// Parent object type.
    pub parent_type: MIDIObjectType,
    /// Added or removed child object.
    pub child: MIDIObjectRef,
    /// Child object type.
    pub child_type: MIDIObjectType,
}

/// One CoreMIDI object property change notification.
#[repr(C)]
pub(super) struct MIDIObjectPropertyChangeNotification {
    /// Notification message kind.
    pub message_id: MIDINotificationMessageID,
    /// Total message size.
    pub message_size: u32,
    /// Changed object reference.
    pub object: MIDIObjectRef,
    /// Changed object type.
    pub object_type: MIDIObjectType,
    /// Changed property name.
    pub property_name: CFStringRef,
}

/// One legacy input callback.
pub(super) type MIDIReadProc =
    unsafe extern "C" fn(*const MIDIPacketList, *mut c_void, *mut c_void);

#[cfg(target_os = "macos")]
#[link(name = "CoreMIDI", kind = "framework")]
unsafe extern "C" {
    /// Property key for endpoint name.
    pub(super) static kMIDIPropertyName: CFStringRef;
    /// Property key for display name.
    pub(super) static kMIDIPropertyDisplayName: CFStringRef;
    /// Property key for manufacturer.
    pub(super) static kMIDIPropertyManufacturer: CFStringRef;
    /// Property key for model.
    pub(super) static kMIDIPropertyModel: CFStringRef;
    /// Property key for unique id.
    pub(super) static kMIDIPropertyUniqueID: CFStringRef;
    /// Property key for driver version.
    pub(super) static kMIDIPropertyDriverVersion: CFStringRef;
    /// Property key for offline state.
    pub(super) static kMIDIPropertyOffline: CFStringRef;
    /// Property key for protocol id.
    pub(super) static kMIDIPropertyProtocolID: CFStringRef;
    /// Create one CoreMIDI client with one notification block.
    pub(super) fn MIDIClientCreateWithBlock(
        name: CFStringRef,
        out_client: *mut MIDIClientRef,
        notify_block: Option<&Block<dyn Fn(*const MIDINotification)>>,
    ) -> OSStatus;

    /// Dispose one CoreMIDI client.
    pub(super) fn MIDIClientDispose(client: MIDIClientRef) -> OSStatus;

    /// Create one protocol-aware input port.
    pub(super) fn MIDIInputPortCreateWithProtocol(
        client: MIDIClientRef,
        port_name: CFStringRef,
        protocol: MIDIProtocolID,
        out_port: *mut MIDIPortRef,
        receive_block: &Block<dyn Fn(*const MIDIEventList, *mut u8)>,
    ) -> OSStatus;

    /// Create one legacy input port.
    pub(super) fn MIDIInputPortCreate(
        client: MIDIClientRef,
        port_name: CFStringRef,
        read_proc: Option<MIDIReadProc>,
        ref_con: *mut c_void,
        out_port: *mut MIDIPortRef,
    ) -> OSStatus;

    /// Create one output port.
    pub(super) fn MIDIOutputPortCreate(
        client: MIDIClientRef,
        port_name: CFStringRef,
        out_port: *mut MIDIPortRef,
    ) -> OSStatus;

    /// Dispose one port.
    pub(super) fn MIDIPortDispose(port: MIDIPortRef) -> OSStatus;

    /// Connect one input port to one source.
    pub(super) fn MIDIPortConnectSource(
        port: MIDIPortRef,
        source: MIDIEndpointRef,
        conn_ref_con: *mut c_void,
    ) -> OSStatus;

    /// Disconnect one input port from one source.
    pub(super) fn MIDIPortDisconnectSource(port: MIDIPortRef, source: MIDIEndpointRef) -> OSStatus;

    /// Return the number of sources.
    pub(super) fn MIDIGetNumberOfSources() -> ItemCount;

    /// Return one source by index.
    pub(super) fn MIDIGetSource(index: ItemCount) -> MIDIEndpointRef;

    /// Return the number of destinations.
    pub(super) fn MIDIGetNumberOfDestinations() -> ItemCount;

    /// Return one destination by index.
    pub(super) fn MIDIGetDestination(index: ItemCount) -> MIDIEndpointRef;

    /// Resolve one entity for one endpoint.
    pub(super) fn MIDIEndpointGetEntity(
        endpoint: MIDIEndpointRef,
        out_entity: *mut MIDIEntityRef,
    ) -> OSStatus;

    /// Resolve one device for one entity.
    pub(super) fn MIDIEntityGetDevice(
        entity: MIDIEntityRef,
        out_device: *mut MIDIDeviceRef,
    ) -> OSStatus;

    /// Read one integer property.
    pub(super) fn MIDIObjectGetIntegerProperty(
        object: MIDIObjectRef,
        property_id: CFStringRef,
        out_value: *mut i32,
    ) -> OSStatus;

    /// Set one integer property.
    pub(super) fn MIDIObjectSetIntegerProperty(
        object: MIDIObjectRef,
        property_id: CFStringRef,
        value: i32,
    ) -> OSStatus;

    /// Read one string property.
    pub(super) fn MIDIObjectGetStringProperty(
        object: MIDIObjectRef,
        property_id: CFStringRef,
        out_string: *mut CFStringRef,
    ) -> OSStatus;

    /// Set one string property.
    pub(super) fn MIDIObjectSetStringProperty(
        object: MIDIObjectRef,
        property_id: CFStringRef,
        value: CFStringRef,
    ) -> OSStatus;

    /// Resolve one object by unique id.
    pub(super) fn MIDIObjectFindByUniqueID(
        unique_id: MIDIUniqueID,
        out_object: *mut MIDIObjectRef,
        out_object_type: *mut MIDIObjectType,
    ) -> OSStatus;

    /// Create one protocol-aware virtual destination.
    pub(super) fn MIDIDestinationCreateWithProtocol(
        client: MIDIClientRef,
        name: CFStringRef,
        protocol: MIDIProtocolID,
        out_destination: *mut MIDIEndpointRef,
        receive_block: &Block<dyn Fn(*const MIDIEventList, *mut u8)>,
    ) -> OSStatus;

    /// Create one legacy virtual destination.
    pub(super) fn MIDIDestinationCreate(
        client: MIDIClientRef,
        name: CFStringRef,
        read_proc: Option<MIDIReadProc>,
        ref_con: *mut c_void,
        out_destination: *mut MIDIEndpointRef,
    ) -> OSStatus;

    /// Create one protocol-aware virtual source.
    pub(super) fn MIDISourceCreateWithProtocol(
        client: MIDIClientRef,
        name: CFStringRef,
        protocol: MIDIProtocolID,
        out_source: *mut MIDIEndpointRef,
    ) -> OSStatus;

    /// Create one legacy virtual source.
    pub(super) fn MIDISourceCreate(
        client: MIDIClientRef,
        name: CFStringRef,
        out_source: *mut MIDIEndpointRef,
    ) -> OSStatus;

    /// Dispose one virtual endpoint.
    pub(super) fn MIDIEndpointDispose(endpoint: MIDIEndpointRef) -> OSStatus;

    /// Send one UMP event list to one destination.
    pub(super) fn MIDISendEventList(
        port: MIDIPortRef,
        destination: MIDIEndpointRef,
        event_list: *const MIDIEventList,
    ) -> OSStatus;

    /// Send one legacy packet list to one destination.
    pub(super) fn MIDISend(
        port: MIDIPortRef,
        destination: MIDIEndpointRef,
        packet_list: *const MIDIPacketList,
    ) -> OSStatus;

    /// Flush scheduled output for one destination or all destinations.
    pub(super) fn MIDIFlushOutput(destination: MIDIEndpointRef) -> OSStatus;

    /// Deliver one UMP event list from one virtual source.
    pub(super) fn MIDIReceivedEventList(
        source: MIDIEndpointRef,
        event_list: *const MIDIEventList,
    ) -> OSStatus;

    /// Deliver one legacy packet list from one virtual source.
    pub(super) fn MIDIReceived(
        source: MIDIEndpointRef,
        packet_list: *const MIDIPacketList,
    ) -> OSStatus;

    /// Initialize one UMP event list.
    pub(super) fn MIDIEventListInit(
        event_list: *mut MIDIEventList,
        protocol: MIDIProtocolID,
    ) -> *mut MIDIEventPacket;

    /// Add one UMP packet payload to one event list.
    pub(super) fn MIDIEventListAdd(
        event_list: *mut MIDIEventList,
        list_size: usize,
        current_packet: *mut MIDIEventPacket,
        time_stamp: MIDITimeStamp,
        word_count: usize,
        words: *const u32,
    ) -> *mut MIDIEventPacket;

    /// Initialize one legacy packet list.
    pub(super) fn MIDIPacketListInit(packet_list: *mut MIDIPacketList) -> *mut MIDIPacket;

    /// Add one legacy packet payload to one packet list.
    pub(super) fn MIDIPacketListAdd(
        packet_list: *mut MIDIPacketList,
        list_size: usize,
        current_packet: *mut MIDIPacket,
        time_stamp: MIDITimeStamp,
        length: usize,
        data: *const u8,
    ) -> *mut MIDIPacket;
}

/// Create one CFString from UTF-8 text.
pub(super) fn create_cf_string(value: &str) -> Option<CFStringRef> {
    let value_bytes = value.as_bytes();

    let string = unsafe {
        CFStringCreateWithBytes(
            kCFAllocatorDefault,
            value_bytes.as_ptr(),
            value_bytes.len() as isize,
            kCFStringEncodingUTF8,
            0,
        )
    };

    if string.is_null() {
        return None;
    }

    Some(string)
}

/// Copy one CoreFoundation string into UTF-8.
pub(super) fn copy_cf_string(value: CFStringRef) -> Option<String> {
    if value.is_null() {
        return None;
    }

    let length = unsafe { CFStringGetLength(value) };
    if length < 0 {
        return None;
    }

    let max_utf8 = unsafe { CFStringGetMaximumSizeForEncoding(length, kCFStringEncodingUTF8) };
    if max_utf8 < 0 {
        return None;
    }

    let mut bytes = vec![0u8; max_utf8 as usize + 1];
    let converted = unsafe {
        CFStringGetCString(
            value,
            bytes.as_mut_ptr().cast(),
            bytes.len() as CFIndex,
            kCFStringEncodingUTF8,
        )
    };
    if converted == 0 {
        return None;
    }

    let terminator = bytes
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(bytes.len());
    String::from_utf8(bytes[..terminator].to_vec()).ok()
}

/// Release one CoreFoundation object.
pub(super) fn release_cf(value: CFTypeRef) {
    if value.is_null() {
        return;
    }

    unsafe {
        CFRelease(value);
    }
}

/// Advance one legacy packet pointer to the next packet in memory.
pub(super) unsafe fn midi_packet_next(packet: *const MIDIPacket) -> *const MIDIPacket {
    let packet = packet.cast::<u8>();
    let base_size = std::mem::size_of::<MIDIPacket>() - 256;
    let length = unsafe { (*packet.cast::<MIDIPacket>()).length as usize };
    let padded = (length + 3) & !3;

    unsafe { packet.add(base_size + padded).cast::<MIDIPacket>() }
}

/// Advance one modern event-packet pointer to the next packet in memory.
pub(super) unsafe fn midi_event_packet_next(
    packet: *const MIDIEventPacket,
) -> *const MIDIEventPacket {
    let word_count = unsafe { (*packet).word_count as usize };
    let base_size = std::mem::size_of::<MIDIEventPacket>() - 64 * std::mem::size_of::<u32>();

    unsafe {
        packet
            .cast::<u8>()
            .add(base_size + word_count * std::mem::size_of::<u32>())
            .cast::<MIDIEventPacket>()
    }
}
