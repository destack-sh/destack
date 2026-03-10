use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Duration;

use parking_lot::Mutex;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::core::{self as core_platform};
use crate::platform::midi::core::{MidiEventValue, MidiInputRecordValue, MidiPortDescriptorValue};
pub(super) use crate::platform::midi::shared::{
    MIDI_EVENT_RESOURCE_LABEL, MIDI_INPUT_RESOURCE_LABEL, MIDI_OUTPUT_RESOURCE_LABEL, SharedQueue,
    binding_timestamp_now, direction_mask_includes, endpoint_direction_name, event_poll_interval,
    event_queue_capacity, event_snapshot_list_flags, input_queue_capacity,
};
use crate::platform::midi::{
    MIDI_DATA_FORMAT_FLAG_MIDI1_BYTES, MIDI_DATA_FORMAT_FLAG_UMP, MIDI_PROTOCOL_FLAG_MIDI1,
    MIDI_PROTOCOL_FLAG_MIDI2, MidiBackend, MidiDataFormat, MidiDataFormatFlags,
    MidiEventOverflowPolicy, MidiEventSubscriptionFlags, MidiPortDirection, MidiPortDirectionFlags,
    MidiProtocol, MidiProtocolFlags,
};
use crate::platform::resource::{ResourceEntry, ResourceKind};
use crate::platform::{NativeStringRef, resource};
use crate::runtime::BindingCallContext;

use super::abi::{
    K_MIDI_PROTOCOL_1_0, K_MIDI_PROTOCOL_2_0, MIDIEndpointDispose, MIDIEndpointRef,
    MIDIPortDisconnectSource, MIDIPortDispose, MIDIPortRef, MIDIProtocolID, MIDITimeStamp,
};
use super::callback::{CoreMidiReceiveBlock, LegacyInputCallbackToken};
use super::descriptor::unregister_endpoint_override;
use super::service::{CoreMidiNativeEventRegistry, CoreMidiService};

/// One runtime transport override for one virtual endpoint.
#[derive(Debug, Clone, Copy)]
pub(super) struct CoreMidiEndpointOverride {
    /// Live CoreMIDI endpoint reference for this virtual endpoint.
    pub(super) endpoint: MIDIEndpointRef,
    /// Transport data format reported for the endpoint.
    pub(super) data_format: MidiDataFormat,
    /// Protocol semantics reported for the endpoint.
    pub(super) protocol: MidiProtocol,
}

/// One opened input-session variant.
pub(super) enum CoreMidiInputSessionKind {
    /// One legacy source connection.
    LegacySource {
        /// Connected input port.
        port: MIDIPortRef,
        /// Connected source endpoint.
        source: MIDIEndpointRef,
        /// Callback context token.
        _callback_context: LegacyInputCallbackToken,
    },
    /// One protocol-aware source connection.
    ModernSource {
        /// Connected input port.
        port: MIDIPortRef,
        /// Connected source endpoint.
        source: MIDIEndpointRef,
        /// Retained callback block.
        _receive_block: CoreMidiReceiveBlock,
    },
    /// One legacy virtual destination.
    LegacyVirtualDestination {
        /// Destination endpoint.
        endpoint: MIDIEndpointRef,
        /// Callback context token.
        _callback_context: LegacyInputCallbackToken,
    },
    /// One protocol-aware virtual destination.
    ModernVirtualDestination {
        /// Destination endpoint.
        endpoint: MIDIEndpointRef,
        /// Retained callback block.
        _receive_block: CoreMidiReceiveBlock,
    },
}

/// One opened output-session variant.
pub(super) enum CoreMidiOutputSessionKind {
    /// One host destination with one output port.
    Destination {
        /// Output port used for MIDISend or MIDISendEventList.
        port: MIDIPortRef,
        /// Destination endpoint.
        destination: MIDIEndpointRef,
    },
    /// One virtual source endpoint.
    VirtualSource {
        /// Virtual source endpoint.
        endpoint: MIDIEndpointRef,
    },
}

/// One opened CoreMIDI input session.
pub(super) struct CoreMidiInputSession {
    /// Shared service owner.
    pub(super) _service: Arc<CoreMidiService>,
    /// Current descriptor snapshot.
    pub(super) descriptor: MidiPortDescriptorValue,
    /// Shared input queue.
    pub(super) queue: Arc<SharedQueue<MidiInputRecordValue>>,
    /// Session resources that must be released.
    pub(super) kind: CoreMidiInputSessionKind,
}

/// One opened CoreMIDI output session.
pub(super) struct CoreMidiOutputSession {
    /// Shared service owner.
    pub(super) _service: Arc<CoreMidiService>,
    /// Current descriptor snapshot.
    pub(super) descriptor: MidiPortDescriptorValue,
    /// Selected data format.
    pub(super) data_format: MidiDataFormat,
    /// Selected protocol.
    pub(super) protocol: Option<MidiProtocol>,
    /// Session resources that must be released.
    pub(super) kind: CoreMidiOutputSessionKind,
}

/// One stable key for one direction-scoped snapshot row.
pub(super) type SnapshotKey = String;

/// One event delivery mode for one opened subscription.
#[derive(Clone)]
pub(super) enum CoreMidiEventDeliveryKind {
    /// Use the native CoreMIDI notification feed.
    Native {
        /// Shared native event registry.
        registry: Arc<Mutex<CoreMidiNativeEventRegistry>>,
        /// Registration id in the native registry.
        registration_id: u64,
    },
    /// Use synthetic polling snapshots.
    Poll,
}

/// One opened MIDI event subscription.
#[derive(Clone)]
pub(super) struct CoreMidiEventSession {
    /// Selected backend for the subscription.
    pub(super) backend: MidiBackend,
    /// Included directions.
    pub(super) direction_mask: MidiPortDirectionFlags,
    /// Subscription flags.
    pub(super) flags: MidiEventSubscriptionFlags,
    /// Overflow policy.
    pub(super) overflow_policy: MidiEventOverflowPolicy,
    /// Poll interval for synthetic snapshots.
    pub(super) poll_interval: Duration,
    /// Selected delivery kind.
    pub(super) delivery_kind: CoreMidiEventDeliveryKind,
    /// Pending event queue.
    pub(super) queue: Arc<SharedQueue<MidiEventValue>>,
    /// Next sequence number.
    pub(super) next_sequence: u64,
    /// Previous endpoint snapshot.
    pub(super) snapshot: BTreeMap<SnapshotKey, (MidiPortDirection, MidiPortDescriptorValue)>,
}

/// Resource payload for one input session.
pub(super) struct CoreMidiInputResource {
    /// Shared session state.
    pub(super) session: Arc<CoreMidiInputSession>,
}

/// Resource payload for one output session.
pub(super) struct CoreMidiOutputResource {
    /// Shared session state.
    pub(super) session: Arc<CoreMidiOutputSession>,
}

/// Resource payload for one event subscription.
#[derive(Clone)]
pub(super) struct CoreMidiEventResource {
    /// Shared subscription state.
    pub(super) session: Arc<Mutex<CoreMidiEventSession>>,
}

impl Drop for CoreMidiInputSession {
    /// Release CoreMIDI resources for one input session.
    fn drop(&mut self) {
        match &self.kind {
            CoreMidiInputSessionKind::LegacySource {
                port,
                source,
                _callback_context: _,
            } => unsafe {
                let _ = MIDIPortDisconnectSource(*port, *source);
                let _ = MIDIPortDispose(*port);
            },
            CoreMidiInputSessionKind::ModernSource {
                port,
                source,
                _receive_block: _,
            } => unsafe {
                let _ = MIDIPortDisconnectSource(*port, *source);
                let _ = MIDIPortDispose(*port);
            },
            CoreMidiInputSessionKind::LegacyVirtualDestination {
                endpoint,
                _callback_context: _,
            } => unsafe {
                unregister_endpoint_override(*endpoint);
                let _ = MIDIEndpointDispose(*endpoint);
            },
            CoreMidiInputSessionKind::ModernVirtualDestination {
                endpoint,
                _receive_block: _,
            } => unsafe {
                unregister_endpoint_override(*endpoint);
                let _ = MIDIEndpointDispose(*endpoint);
            },
        }

        self.queue.close();
    }
}

impl Drop for CoreMidiOutputSession {
    /// Release CoreMIDI resources for one output session.
    fn drop(&mut self) {
        match &self.kind {
            CoreMidiOutputSessionKind::Destination { port, .. } => unsafe {
                let _ = MIDIPortDispose(*port);
            },
            CoreMidiOutputSessionKind::VirtualSource { endpoint } => unsafe {
                unregister_endpoint_override(*endpoint);
                let _ = MIDIEndpointDispose(*endpoint);
            },
        }
    }
}

impl CoreMidiInputSession {
    /// Return whether this session owns one virtual endpoint.
    pub(super) fn is_virtual_endpoint(&self) -> bool {
        matches!(
            self.kind,
            CoreMidiInputSessionKind::LegacyVirtualDestination { .. }
                | CoreMidiInputSessionKind::ModernVirtualDestination { .. }
        )
    }
}

impl CoreMidiOutputSession {
    /// Return whether this session owns one virtual endpoint.
    pub(super) fn is_virtual_endpoint(&self) -> bool {
        matches!(self.kind, CoreMidiOutputSessionKind::VirtualSource { .. })
    }
}

/// Return the CoreMIDI protocol id for one selected protocol.
pub(super) fn selected_protocol_id(
    protocol: Option<MidiProtocol>,
    data_format: MidiDataFormat,
) -> MIDIProtocolID {
    match (data_format, protocol) {
        (MidiDataFormat::Ump, Some(MidiProtocol::Midi2)) => K_MIDI_PROTOCOL_2_0,
        (MidiDataFormat::Ump, _) => K_MIDI_PROTOCOL_1_0,
        (MidiDataFormat::Midi1Bytes, Some(MidiProtocol::Midi2)) => K_MIDI_PROTOCOL_2_0,
        (MidiDataFormat::Midi1Bytes, _) => K_MIDI_PROTOCOL_1_0,
    }
}

/// Return the flag bit for one data format.
pub(super) fn data_format_flag(data_format: MidiDataFormat) -> MidiDataFormatFlags {
    match data_format {
        MidiDataFormat::Midi1Bytes => MIDI_DATA_FORMAT_FLAG_MIDI1_BYTES,
        MidiDataFormat::Ump => MIDI_DATA_FORMAT_FLAG_UMP,
    }
}

/// Return the flag bit for one protocol.
pub(super) fn protocol_flag(protocol: MidiProtocol) -> MidiProtocolFlags {
    match protocol {
        MidiProtocol::Midi1 => MIDI_PROTOCOL_FLAG_MIDI1,
        MidiProtocol::Midi2 => MIDI_PROTOCOL_FLAG_MIDI2,
    }
}

/// Return one exact transport-support tuple for one selected shape.
pub(super) fn exact_transport_support(
    data_format: MidiDataFormat,
    protocol: MidiProtocol,
) -> (
    MidiDataFormatFlags,
    Option<MidiDataFormat>,
    MidiProtocolFlags,
    Option<MidiProtocol>,
) {
    (
        data_format_flag(data_format),
        Some(data_format),
        protocol_flag(protocol),
        Some(protocol),
    )
}

/// Build one CoreMIDI operation error.
pub(super) fn core_midi_status_error(
    operation: &'static str,
    action: &str,
    status: i32,
) -> Box<RuntimeError> {
    core_platform::io_operation_error(
        operation,
        None,
        format!("{action} failed with CoreMIDI status {status}"),
    )
}

/// Convert one CoreMIDI host timestamp into runtime monotonic nanoseconds.
pub(super) fn core_midi_host_time_to_mono_ns(time_stamp: MIDITimeStamp) -> u64 {
    if time_stamp == 0 {
        return binding_timestamp_now();
    }

    core_platform::apple_host_time_to_process_nanos(time_stamp)
}

/// Convert one runtime monotonic nanosecond timestamp into one CoreMIDI host timestamp.
pub(super) fn core_midi_mono_ns_to_host_time(send_at_ns: Option<u64>) -> MIDITimeStamp {
    match send_at_ns {
        Some(send_at_ns) if send_at_ns != 0 => {
            core_platform::apple_process_nanos_to_host_time(send_at_ns)
        }
        _ => 0,
    }
}

/// Decode one native binding string into one owned string.
pub(super) fn native_string(value: NativeStringRef) -> RuntimeResult<String> {
    let value = unsafe { value.as_str()? };

    Ok(value.to_string())
}

/// Decode one optional native binding string.
pub(super) fn native_optional_string(
    value: Option<NativeStringRef>,
) -> RuntimeResult<Option<String>> {
    match value {
        Some(value) => Ok(Some(native_string(value)?)),
        None => Ok(None),
    }
}

/// Allocate one resource entry for one input session.
pub(super) fn insert_input_resource(
    binding: &BindingCallContext,
    session: Arc<CoreMidiInputSession>,
) -> resource::MidiInputPortHandle {
    let entry = ResourceEntry::new(ResourceKind::MidiInputPort)
        .with_label(MIDI_INPUT_RESOURCE_LABEL)
        .with_payload(CoreMidiInputResource { session });
    let resource_id =
        binding
            .agent()
            .resources
            .insert(binding.world(), entry, Some(binding.engine()));

    resource::MidiInputPortHandle(resource_id)
}

/// Allocate one resource entry for one output session.
pub(super) fn insert_output_resource(
    binding: &BindingCallContext,
    session: Arc<CoreMidiOutputSession>,
) -> resource::MidiOutputPortHandle {
    let entry = ResourceEntry::new(ResourceKind::MidiOutputPort)
        .with_label(MIDI_OUTPUT_RESOURCE_LABEL)
        .with_payload(CoreMidiOutputResource { session });
    let resource_id =
        binding
            .agent()
            .resources
            .insert(binding.world(), entry, Some(binding.engine()));

    resource::MidiOutputPortHandle(resource_id)
}

/// Allocate one resource entry for one event subscription.
pub(super) fn insert_event_resource(
    binding: &BindingCallContext,
    session: Arc<Mutex<CoreMidiEventSession>>,
) -> resource::MidiEventHandle {
    let entry = ResourceEntry::new(ResourceKind::MidiEvent)
        .with_label(MIDI_EVENT_RESOURCE_LABEL)
        .with_payload(CoreMidiEventResource { session });
    let resource_id =
        binding
            .agent()
            .resources
            .insert(binding.world(), entry, Some(binding.engine()));

    resource::MidiEventHandle(resource_id)
}
