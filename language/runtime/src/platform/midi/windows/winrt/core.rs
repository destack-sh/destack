use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Duration;

use parking_lot::Mutex;
use windows::Devices::Midi::{MidiInPort, MidiOutPort};

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::core::{self as core_platform};
use crate::platform::midi::core::{MidiEventValue, MidiInputRecordValue, MidiPortDescriptorValue};
pub(super) use crate::platform::midi::shared::{
    MIDI_EVENT_RESOURCE_LABEL, MIDI_INPUT_RESOURCE_LABEL, MIDI_OUTPUT_RESOURCE_LABEL, SharedQueue,
    binding_timestamp_now, direction_mask_includes, endpoint_direction_name, event_poll_interval,
    event_queue_capacity, event_snapshot_list_flags, input_queue_capacity, missing_handle,
};
use crate::platform::midi::{
    MIDI_DATA_FORMAT_FLAG_MIDI1_BYTES, MIDI_PROTOCOL_FLAG_MIDI1, MidiBackend, MidiDataFormat,
    MidiDataFormatFlags, MidiEventOverflowPolicy, MidiEventSubscriptionFlags, MidiPortDirection,
    MidiPortDirectionFlags, MidiPortListFlags, MidiProtocol, MidiProtocolFlags,
};
use crate::platform::resource;
use crate::platform::resource::{ResourceEntry, ResourceId, ResourceKind};
use crate::runtime::BindingCallContext;

use super::service::{
    WinRtNativeEventRegistry, WinRtService, ensure_current_thread_winrt_apartment_for_drop,
};

/// One cached WinRT endpoint row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct WinRtEndpointInfo {
    /// Descriptor snapshot for this endpoint.
    pub(super) descriptor: MidiPortDescriptorValue,
    /// WinRT backend id used to reopen the endpoint.
    pub(super) backend_id: String,
}

/// One process-global snapshot of WinRT topology.
#[derive(Default)]
pub(super) struct WinRtTopologyState {
    /// Current input endpoint rows.
    pub(super) inputs: BTreeMap<String, WinRtEndpointInfo>,
    /// Current output endpoint rows.
    pub(super) outputs: BTreeMap<String, WinRtEndpointInfo>,
}

/// One opened WinRT input session.
pub(super) struct WinRtInputSession {
    /// Shared service owner.
    pub(super) _service: Arc<WinRtService>,
    /// Current descriptor snapshot.
    pub(super) descriptor: MidiPortDescriptorValue,
    /// Opened WinRT input port.
    pub(super) port: MidiInPort,
    /// Message-received registration token.
    pub(super) token: i64,
    /// Shared input queue.
    pub(super) queue: Arc<SharedQueue<MidiInputRecordValue>>,
}

/// One opened WinRT output session.
pub(super) struct WinRtOutputSession {
    /// Shared service owner.
    pub(super) _service: Arc<WinRtService>,
    /// Current descriptor snapshot.
    pub(super) descriptor: MidiPortDescriptorValue,
    /// Opened WinRT output port.
    pub(super) port: MidiOutPort,
}

/// One stable key for one direction-scoped snapshot row.
pub(super) type SnapshotKey = String;

/// One event delivery mode for one opened subscription.
#[derive(Clone)]
pub(super) enum WinRtEventDeliveryKind {
    /// Use the native WinRT device-watcher feed.
    Native {
        /// Shared native event registry.
        registry: Arc<Mutex<WinRtNativeEventRegistry>>,
        /// Registration id in the native registry.
        registration_id: u64,
    },
    /// Use synthetic polling snapshots.
    Poll,
}

/// One opened MIDI event subscription.
#[derive(Clone)]
pub(super) struct WinRtEventSession {
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
    pub(super) delivery_kind: WinRtEventDeliveryKind,
    /// Pending event queue.
    pub(super) queue: Arc<SharedQueue<MidiEventValue>>,
    /// Next sequence number.
    pub(super) next_sequence: u64,
    /// Previous endpoint snapshot.
    pub(super) snapshot: BTreeMap<SnapshotKey, (MidiPortDirection, MidiPortDescriptorValue)>,
}

/// Resource payload for one input session.
pub(super) struct WinRtInputResource {
    /// Shared session state.
    pub(super) session: Arc<WinRtInputSession>,
}

/// Resource payload for one output session.
pub(super) struct WinRtOutputResource {
    /// Shared session state.
    pub(super) session: Arc<WinRtOutputSession>,
}

/// Resource payload for one event subscription.
#[derive(Clone)]
pub(super) struct WinRtEventResource {
    /// Shared subscription state.
    pub(super) session: Arc<Mutex<WinRtEventSession>>,
}

impl Drop for WinRtInputSession {
    /// Release WinRT resources for one input session.
    fn drop(&mut self) {
        // drop paths may run on any runtime thread
        ensure_current_thread_winrt_apartment_for_drop();

        let _ = self.port.RemoveMessageReceived(self.token);
        let _ = self.port.Close();
        self.queue.close();
    }
}

impl Drop for WinRtOutputSession {
    /// Release WinRT resources for one output session.
    fn drop(&mut self) {
        // drop paths may run on any runtime thread
        ensure_current_thread_winrt_apartment_for_drop();

        let _ = self.port.Close();
    }
}

/// Return the flag bit for one data format.
pub(super) fn data_format_flag(data_format: MidiDataFormat) -> MidiDataFormatFlags {
    match data_format {
        MidiDataFormat::Midi1Bytes => MidiDataFormatFlags(MIDI_DATA_FORMAT_FLAG_MIDI1_BYTES.0),
        MidiDataFormat::Ump => MidiDataFormatFlags(0),
    }
}

/// Return the flag bit for one protocol.
pub(super) fn protocol_flag(protocol: MidiProtocol) -> MidiProtocolFlags {
    match protocol {
        MidiProtocol::Midi1 => MidiProtocolFlags(MIDI_PROTOCOL_FLAG_MIDI1.0),
        MidiProtocol::Midi2 => MidiProtocolFlags(0),
    }
}

/// Return one exact transport-support tuple for the WinRT backend.
pub(super) fn exact_transport_support() -> (
    MidiDataFormatFlags,
    Option<MidiDataFormat>,
    MidiProtocolFlags,
    Option<MidiProtocol>,
) {
    (
        MidiDataFormatFlags(MIDI_DATA_FORMAT_FLAG_MIDI1_BYTES.0),
        Some(MidiDataFormat::Midi1Bytes),
        MidiProtocolFlags(MIDI_PROTOCOL_FLAG_MIDI1.0),
        Some(MidiProtocol::Midi1),
    )
}

/// Convert one WinRT port-relative timestamp into runtime monotonic nanoseconds.
///
/// WinRT MIDI receive timestamps are reported as `TimeSpan` durations relative to
/// the opened `MidiInPort`, so the session records one monotonic open epoch and
/// projects each callback timestamp into the shared runtime monotonic domain.
pub(super) fn winrt_relative_timestamp_to_mono_ns(open_epoch_ns: u64, duration_100ns: i64) -> u64 {
    if duration_100ns <= 0 {
        return open_epoch_ns;
    }

    let delta_ns = (duration_100ns as u128).saturating_mul(100);
    let delta_ns = delta_ns.min(u64::MAX as u128) as u64;

    open_epoch_ns.saturating_add(delta_ns)
}

/// Allocate one resource entry for one input session.
pub(super) fn insert_input_resource(
    binding: &BindingCallContext,
    session: Arc<WinRtInputSession>,
) -> resource::MidiInputPortHandle {
    let entry = ResourceEntry::new(ResourceKind::MidiInputPort)
        .with_label(MIDI_INPUT_RESOURCE_LABEL)
        .with_payload(WinRtInputResource { session });
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
    session: Arc<WinRtOutputSession>,
) -> resource::MidiOutputPortHandle {
    let entry = ResourceEntry::new(ResourceKind::MidiOutputPort)
        .with_label(MIDI_OUTPUT_RESOURCE_LABEL)
        .with_payload(WinRtOutputResource { session });
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
    session: Arc<Mutex<WinRtEventSession>>,
) -> resource::MidiEventHandle {
    let entry = ResourceEntry::new(ResourceKind::MidiEvent)
        .with_label(MIDI_EVENT_RESOURCE_LABEL)
        .with_payload(WinRtEventResource { session });
    let resource_id =
        binding
            .agent()
            .resources
            .insert(binding.world(), entry, Some(binding.engine()));

    resource::MidiEventHandle(resource_id)
}
