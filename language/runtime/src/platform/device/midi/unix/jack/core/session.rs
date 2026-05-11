use std::collections::{BTreeMap, VecDeque};
use std::sync::Arc;

use parking_lot::Mutex;

use crate::platform::device::midi::core::{
    MidiEventValue, MidiInputRecordValue, MidiOutputRecordValue, MidiPortDescriptorValue,
    define_backend_midi_resource_inserters,
};
use crate::platform::device::{
    MidiBackend, MidiDataFormat, MidiEventOverflowPolicy, MidiEventSubscriptionFlags,
    MidiPortDirection, MidiPortDirectionFlags, MidiProtocol,
};
use crate::runtime::control::queue::BoundedQueue;
use crate::runtime::service::executor::periodic::PeriodicTaskHandle;

use super::super::abi::{JackClient, JackPort};
use super::native::{
    JackClientHandle, JackLibrary, release_input_callback_context, release_output_callback_context,
    unregister_port,
};

/// One cached JACK endpoint row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct JackEndpointInfo {
    /// Descriptor snapshot for this endpoint.
    pub(crate) descriptor: MidiPortDescriptorValue,
    /// Backend-native full port name.
    pub(crate) backend_port_name: String,
}

/// One stable JACK snapshot key.
pub(crate) type JackSnapshotKey = String;

/// One process-global snapshot of JACK MIDI topology.
#[derive(Default)]
pub(crate) struct JackTopologyState {
    /// Current input endpoint rows.
    pub(crate) inputs: BTreeMap<JackSnapshotKey, JackEndpointInfo>,
    /// Current output endpoint rows.
    pub(crate) outputs: BTreeMap<JackSnapshotKey, JackEndpointInfo>,
}

/// One event delivery mode for one JACK subscription.
#[derive(Clone)]
pub(crate) enum JackEventDeliveryKind {
    /// Use the native JACK port-registration feed.
    Native {
        /// Shared native registry.
        registry: Arc<Mutex<super::super::service::JackNativeEventRegistry>>,
        /// Registration id in the native registry.
        registration_id: u64,
    },
    /// Use synthetic polling snapshots.
    Poll,
}

/// One opened JACK event subscription.
#[derive(Clone)]
pub(crate) struct JackEventRepository {
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
    pub(crate) delivery_kind: JackEventDeliveryKind,
    /// Registered synthetic poll task.
    pub(crate) poll_task: Option<Arc<PeriodicTaskHandle>>,
    /// Pending event queue.
    pub(crate) queue: Arc<BoundedQueue<MidiEventValue>>,
    /// Next sequence number.
    pub(crate) next_sequence: u64,
    /// Previous endpoint snapshot.
    pub(crate) snapshot: BTreeMap<JackSnapshotKey, (MidiPortDirection, MidiPortDescriptorValue)>,
}

/// One JACK input callback context.
pub(crate) struct JackInputCallbackContext {
    /// Shared JACK library owner.
    pub(crate) library: Arc<JackLibrary>,
    /// Raw JACK client pointer used for cycle-time queries.
    pub(crate) client: *mut JackClient,
    /// Local JACK input port owned by the session.
    pub(crate) port: *mut JackPort,
    /// Runtime monotonic epoch captured when the session opened.
    pub(crate) open_epoch_ns: u64,
    /// JACK clock sample captured when the session opened.
    pub(crate) open_jack_time_usecs: u64,
    /// Shared input queue.
    pub(crate) queue: Arc<BoundedQueue<MidiInputRecordValue>>,
    /// Stable source id when the session is connected to one concrete source.
    pub(crate) source_id: Option<Arc<str>>,
    /// Terminal session error.
    pub(crate) terminal_error: Arc<Mutex<Option<JackInputTerminalError>>>,
}

unsafe impl Send for JackInputCallbackContext {}
unsafe impl Sync for JackInputCallbackContext {}

/// One terminal JACK input failure.
#[derive(Debug, Clone)]
pub(crate) enum JackInputTerminalError {
    /// The JACK session disconnected.
    BackendDisconnected,
}

impl JackInputTerminalError {
    /// Return one stable runtime error message for this terminal failure.
    pub(crate) fn message(&self) -> &'static str {
        match self {
            Self::BackendDisconnected => "JACK input session disconnected from the JACK server",
        }
    }
}

/// One opened JACK input-session kind.
pub(crate) enum JackInputRepositoryKind {
    /// One client input connected to one concrete source.
    Source {
        /// The owned JACK client.
        client: JackClientHandle,
        /// The local JACK input port.
        _port: *mut JackPort,
        /// Callback state owned for the JACK process callback lifetime.
        _context_owner: Arc<JackInputCallbackContext>,
        /// Retained callback token held by JACK.
        callback_context_token: usize,
    },
    /// One runtime-owned virtual destination.
    VirtualDestination {
        /// The owned JACK client.
        client: JackClientHandle,
        /// The local JACK input port.
        _port: *mut JackPort,
        /// Callback state owned for the JACK process callback lifetime.
        _context_owner: Arc<JackInputCallbackContext>,
        /// Retained callback token held by JACK.
        callback_context_token: usize,
    },
}

unsafe impl Send for JackInputRepositoryKind {}
unsafe impl Sync for JackInputRepositoryKind {}

/// One opened JACK output callback context.
pub(crate) struct JackOutputCallbackContext {
    /// Shared JACK library owner.
    pub(crate) library: Arc<JackLibrary>,
    /// Local JACK output port owned by the session.
    pub(crate) port: *mut JackPort,
    /// Pending outbound records.
    pub(crate) pending_records: Arc<Mutex<VecDeque<MidiOutputRecordValue>>>,
    /// Terminal session error.
    pub(crate) terminal_error: Arc<Mutex<Option<JackOutputTerminalError>>>,
}

unsafe impl Send for JackOutputCallbackContext {}
unsafe impl Sync for JackOutputCallbackContext {}

/// One terminal JACK output failure.
#[derive(Debug, Clone)]
pub(crate) enum JackOutputTerminalError {
    /// The JACK session disconnected.
    BackendDisconnected,
}

impl JackOutputTerminalError {
    /// Return one stable runtime error message for this terminal failure.
    pub(crate) fn message(&self) -> &'static str {
        match self {
            Self::BackendDisconnected => "JACK output session disconnected from the JACK server",
        }
    }
}

/// One opened JACK output-session kind.
pub(crate) enum JackOutputRepositoryKind {
    /// One client output connected to one concrete destination.
    Destination {
        /// The owned JACK client.
        client: JackClientHandle,
        /// The local JACK output port.
        _port: *mut JackPort,
        /// Callback state owned for the JACK process callback lifetime.
        _context_owner: Arc<JackOutputCallbackContext>,
        /// Retained callback token held by JACK.
        callback_context_token: usize,
    },
    /// One runtime-owned virtual source.
    VirtualSource {
        /// The owned JACK client.
        client: JackClientHandle,
        /// The local JACK output port.
        _port: *mut JackPort,
        /// Callback state owned for the JACK process callback lifetime.
        _context_owner: Arc<JackOutputCallbackContext>,
        /// Retained callback token held by JACK.
        callback_context_token: usize,
    },
}

unsafe impl Send for JackOutputRepositoryKind {}
unsafe impl Sync for JackOutputRepositoryKind {}

/// One opened JACK input session.
pub(crate) struct JackInputRepository {
    /// Shared service owner.
    pub(crate) _service: Arc<super::super::service::JackService>,
    /// Current descriptor snapshot.
    pub(crate) descriptor: MidiPortDescriptorValue,
    /// Shared input queue.
    pub(crate) queue: Arc<BoundedQueue<MidiInputRecordValue>>,
    /// Deferred terminal reader failure.
    pub(crate) terminal_error: Arc<Mutex<Option<JackInputTerminalError>>>,
    /// Repository resources that must be released.
    pub(crate) kind: Mutex<JackInputRepositoryKind>,
}

impl JackInputRepository {
    /// Return whether the opened endpoint is virtual.
    pub(crate) fn is_virtual_endpoint(&self) -> bool {
        self.descriptor.is_virtual
    }
}

/// One opened JACK output session.
pub(crate) struct JackOutputRepository {
    /// Shared service owner.
    pub(crate) _service: Arc<super::super::service::JackService>,
    /// Current descriptor snapshot.
    pub(crate) descriptor: MidiPortDescriptorValue,
    /// Selected data format.
    pub(crate) data_format: MidiDataFormat,
    /// Selected protocol.
    pub(crate) protocol: Option<MidiProtocol>,
    /// Pending outbound records.
    pub(crate) pending_records: Arc<Mutex<VecDeque<MidiOutputRecordValue>>>,
    /// Deferred terminal writer failure.
    pub(crate) terminal_error: Arc<Mutex<Option<JackOutputTerminalError>>>,
    /// Repository resources that must be released.
    pub(crate) kind: Mutex<JackOutputRepositoryKind>,
}

impl JackOutputRepository {
    /// Return whether the opened endpoint is virtual.
    pub(crate) fn is_virtual_endpoint(&self) -> bool {
        self.descriptor.is_virtual
    }
}

impl Drop for JackInputRepository {
    /// Release JACK resources for one input session.
    fn drop(&mut self) {
        let kind = self.kind.get_mut();

        // session-specific teardown
        match kind {
            JackInputRepositoryKind::Source {
                client,
                _port,
                callback_context_token,
                ..
            }
            | JackInputRepositoryKind::VirtualDestination {
                client,
                _port,
                callback_context_token,
                ..
            } => {
                let _ = unregister_port(client, *_port, "destack.device.midi.input.port.close");
                release_input_callback_context(*callback_context_token);
            }
        }

        self.queue.close();
    }
}

impl Drop for JackOutputRepository {
    /// Release JACK resources for one output session.
    fn drop(&mut self) {
        let kind = self.kind.get_mut();

        // session-specific teardown
        match kind {
            JackOutputRepositoryKind::Destination {
                client,
                _port,
                callback_context_token,
                ..
            }
            | JackOutputRepositoryKind::VirtualSource {
                client,
                _port,
                callback_context_token,
                ..
            } => {
                let _ = unregister_port(client, *_port, "destack.device.midi.output.port.close");
                release_output_callback_context(*callback_context_token);
            }
        }
    }
}

/// Resource payload for one JACK input session.
pub(crate) struct JackInputResource {
    /// Shared session state.
    pub(crate) session: Arc<JackInputRepository>,
}

/// Resource payload for one JACK output session.
pub(crate) struct JackOutputResource {
    /// Shared session state.
    pub(crate) session: Arc<JackOutputRepository>,
}

/// Resource payload for one JACK event subscription.
#[derive(Clone)]
pub(crate) struct JackEventResource {
    /// Shared subscription state.
    pub(crate) session: Arc<Mutex<JackEventRepository>>,
}

define_backend_midi_resource_inserters!(
    vis = pub(crate),
    backend = MidiBackend::JackMidi,
    input = (insert_input_resource, JackInputResource, JackInputRepository),
    output = (insert_output_resource, JackOutputResource, JackOutputRepository),
    event = (insert_event_resource, JackEventResource, JackEventRepository)
);
