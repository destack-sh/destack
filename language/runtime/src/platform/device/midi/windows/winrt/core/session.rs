use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Duration;

use parking_lot::Mutex;

use crate::platform::device::midi::core::{
    MidiEventValue, MidiInputRecordValue, MidiPortDescriptorValue,
    define_backend_midi_resource_inserters,
};
use crate::platform::device::{
    MidiBackend, MidiEventOverflowPolicy, MidiEventSubscriptionFlags, MidiPortDirection,
    MidiPortDirectionFlags,
};
use crate::runtime::control::queue::BoundedQueue;
use crate::runtime::service::executor::periodic::PeriodicTaskHandle;

use super::super::service::{WinRtNativeEventRegistry, WinRtService};

/// One cached WinRT endpoint row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct WinRtEndpointInfo {
    /// Descriptor snapshot for this endpoint.
    pub(crate) descriptor: MidiPortDescriptorValue,
    /// WinRT backend id used to reopen the endpoint.
    pub(crate) backend_id: String,
}

/// One process-global snapshot of WinRT topology.
#[derive(Default)]
pub(crate) struct WinRtTopologyState {
    /// Current input endpoint rows.
    pub(crate) inputs: BTreeMap<String, WinRtEndpointInfo>,
    /// Current output endpoint rows.
    pub(crate) outputs: BTreeMap<String, WinRtEndpointInfo>,
}

/// One opened WinRT input session.
pub(crate) struct WinRtInputRepository {
    /// Shared service owner.
    pub(crate) _service: Arc<WinRtService>,
    /// Current descriptor snapshot.
    pub(crate) descriptor: MidiPortDescriptorValue,
    /// Service-owned host session id.
    pub(crate) host_session_id: u64,
    /// Shared input queue.
    pub(crate) queue: Arc<BoundedQueue<MidiInputRecordValue>>,
    /// Deferred terminal backend failure.
    pub(crate) terminal_error: Arc<Mutex<Option<String>>>,
}

/// One opened WinRT output session.
pub(crate) struct WinRtOutputRepository {
    /// Shared service owner.
    pub(crate) _service: Arc<WinRtService>,
    /// Current descriptor snapshot.
    pub(crate) descriptor: MidiPortDescriptorValue,
    /// Service-owned host session id.
    pub(crate) host_session_id: u64,
}

/// One stable key for one direction-scoped snapshot row.
pub(crate) type SnapshotKey = String;

/// One event delivery mode for one opened subscription.
#[derive(Clone)]
pub(crate) enum WinRtEventDeliveryKind {
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
pub(crate) struct WinRtEventRepository {
    /// Selected backend for the subscription.
    pub(crate) backend: MidiBackend,
    /// Included directions.
    pub(crate) direction_mask: MidiPortDirectionFlags,
    /// Subscription flags.
    pub(crate) flags: MidiEventSubscriptionFlags,
    /// Overflow policy.
    pub(crate) overflow_policy: MidiEventOverflowPolicy,
    /// Poll interval for synthetic snapshots.
    pub(crate) poll_interval: Duration,
    /// Selected delivery kind.
    pub(crate) delivery_kind: WinRtEventDeliveryKind,
    /// Registered synthetic poll task.
    pub(crate) poll_task: Option<Arc<PeriodicTaskHandle>>,
    /// Pending event queue.
    pub(crate) queue: Arc<BoundedQueue<MidiEventValue>>,
    /// Next sequence number.
    pub(crate) next_sequence: u64,
    /// Previous endpoint snapshot.
    pub(crate) snapshot: BTreeMap<SnapshotKey, (MidiPortDirection, MidiPortDescriptorValue)>,
}

/// Resource payload for one input session.
pub(crate) struct WinRtInputResource {
    /// Shared session state.
    pub(crate) session: Arc<WinRtInputRepository>,
}

/// Resource payload for one output session.
pub(crate) struct WinRtOutputResource {
    /// Shared session state.
    pub(crate) session: Arc<WinRtOutputRepository>,
}

/// Resource payload for one event subscription.
#[derive(Clone)]
pub(crate) struct WinRtEventResource {
    /// Shared subscription state.
    pub(crate) session: Arc<Mutex<WinRtEventRepository>>,
}

impl Drop for WinRtInputRepository {
    /// Release WinRT resources for one input session.
    fn drop(&mut self) {
        self._service
            .close_input_session_for_drop(self.host_session_id);
        self.queue.close();
    }
}

impl Drop for WinRtOutputRepository {
    /// Release WinRT resources for one output session.
    fn drop(&mut self) {
        self._service
            .close_output_session_for_drop(self.host_session_id);
    }
}

define_backend_midi_resource_inserters!(
    vis = pub(crate),
    backend = MidiBackend::WinRT,
    input = (insert_input_resource, WinRtInputResource, WinRtInputRepository),
    output = (insert_output_resource, WinRtOutputResource, WinRtOutputRepository),
    event = (insert_event_resource, WinRtEventResource, WinRtEventRepository)
);
