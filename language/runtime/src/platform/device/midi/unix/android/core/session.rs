use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Duration;

use parking_lot::Mutex;

use crate::platform::device::midi::core::{
    MidiEventValue, MidiPortDescriptorValue, define_backend_midi_resource_inserters,
};
use crate::platform::device::{
    MidiBackend, MidiBackendCapabilityFlags, MidiDataFormat, MidiDataFormatFlags,
    MidiEventOverflowPolicy, MidiEventSubscriptionFlags, MidiPortDirection, MidiPortDirectionFlags,
    MidiProtocol, MidiProtocolFlags,
};
use crate::runtime::WorkerCallbackHandle;
use crate::runtime::control::queue::BoundedQueue;

/// One resolved Android backend description.
#[derive(Clone, Copy, Debug)]
pub(crate) struct AndroidBackendDescription {
    /// Backend capability flags.
    pub(crate) capability_flags: MidiBackendCapabilityFlags,
    /// Supported transport data formats.
    pub(crate) supported_data_formats: MidiDataFormatFlags,
    /// Supported protocol flags.
    pub(crate) supported_protocols: MidiProtocolFlags,
}

/// One opened Android MIDI input session.
pub(crate) struct AndroidInputRepository {
    /// Current descriptor snapshot.
    pub(crate) descriptor: MidiPortDescriptorValue,
    /// Opaque host session id.
    pub(crate) host_session_id: u64,
}

/// One opened Android MIDI output session.
pub(crate) struct AndroidOutputRepository {
    /// Current descriptor snapshot.
    pub(crate) descriptor: MidiPortDescriptorValue,
    /// Selected transport data format.
    pub(crate) data_format: MidiDataFormat,
    /// Selected protocol semantics.
    pub(crate) protocol: Option<MidiProtocol>,
    /// Opaque host session id.
    pub(crate) host_session_id: u64,
}

/// One stable key for one direction-scoped topology row.
pub(crate) type SnapshotKey = String;

/// One event delivery mode for one opened Android event subscription.
#[derive(Clone, Copy, Debug)]
pub(crate) enum AndroidEventDeliveryKind {
    /// Use the host-native Android topology feed.
    Native {
        /// Opaque host event-subscription id.
        host_session_id: u64,
    },
    /// Use synthetic polling snapshots.
    Poll,
}

/// One opened Android MIDI topology subscription.
#[derive(Clone)]
pub(crate) struct AndroidEventRepository {
    /// Selected backend for this subscription.
    pub(crate) backend: MidiBackend,
    /// Included directions.
    pub(crate) direction_mask: MidiPortDirectionFlags,
    /// Subscription flags.
    pub(crate) flags: MidiEventSubscriptionFlags,
    /// Overflow policy.
    pub(crate) overflow_policy: MidiEventOverflowPolicy,
    /// Selected delivery kind.
    pub(crate) delivery_kind: AndroidEventDeliveryKind,
    /// Synthetic poll interval.
    pub(crate) poll_interval: Duration,
    /// Registered synthetic poll callback.
    pub(crate) poll_callback: Option<WorkerCallbackHandle>,
    /// Pending event queue.
    pub(crate) queue: Arc<BoundedQueue<MidiEventValue>>,
    /// Next sequence number.
    pub(crate) next_sequence: u64,
    /// Previous topology snapshot.
    pub(crate) snapshot: BTreeMap<SnapshotKey, (MidiPortDirection, MidiPortDescriptorValue)>,
}

/// Resource payload for one Android input session.
pub(crate) struct AndroidInputResource {
    /// Shared session state.
    pub(crate) session: Arc<AndroidInputRepository>,
}

/// Resource payload for one Android output session.
pub(crate) struct AndroidOutputResource {
    /// Shared session state.
    pub(crate) session: Arc<AndroidOutputRepository>,
}

/// Resource payload for one Android event subscription.
#[derive(Clone)]
pub(crate) struct AndroidEventResource {
    /// Shared subscription state.
    pub(crate) session: Arc<Mutex<AndroidEventRepository>>,
}

define_backend_midi_resource_inserters!(
    vis = pub(crate),
    backend = MidiBackend::AndroidMidi,
    input = (insert_input_resource, AndroidInputResource, AndroidInputRepository),
    output = (insert_output_resource, AndroidOutputResource, AndroidOutputRepository),
    event = (insert_event_resource, AndroidEventResource, AndroidEventRepository)
);
