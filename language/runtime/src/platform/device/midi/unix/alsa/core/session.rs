use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::JoinHandle;
use std::time::Duration;

use parking_lot::Mutex;

use crate::platform::core::{BoundedQueue, DynamicLibrary};
use crate::platform::device::midi::core::{
    MidiEventValue, MidiInputRecordValue, MidiPortDescriptorValue,
    define_backend_midi_resource_inserters,
};
use crate::platform::device::{
    MidiBackend, MidiDataFormat, MidiEventOverflowPolicy, MidiEventSubscriptionFlags,
    MidiPortDirection, MidiPortDirectionFlags, MidiProtocol,
};
use crate::runtime::service::executor::periodic::PeriodicTaskHandle;

use super::super::abi::{AlsaApi, snd_midi_event_t, snd_seq_t};
use super::super::service::{AlsaNativeEventRegistry, AlsaService};
use super::native::{
    delete_simple_port, disconnect_to, free_queue, unsubscribe_from_with_timestamps,
};

/// Prefix used for internal hidden ALSA clients.
pub(crate) const INTERNAL_CLIENT_NAME_PREFIX: &str = "Destack MIDI Internal";

/// One opened ALSA sequencer dynamic library.
pub(crate) struct AlsaLibrary {
    /// The owning dynamic-library handle.
    pub(crate) _library: DynamicLibrary,
    /// The loaded symbol table.
    pub(crate) api: AlsaApi,
}

/// One owned ALSA sequencer handle.
pub(crate) struct AlsaHandle {
    /// Shared library owner.
    pub(crate) library: Arc<AlsaLibrary>,
    /// Raw ALSA sequencer handle.
    pub(crate) raw: *mut snd_seq_t,
    /// The current ALSA client id.
    pub(crate) client_id: i32,
}

/// One owned ALSA MIDI parser.
pub(crate) struct AlsaMidiParser {
    /// Shared library owner.
    pub(crate) library: Arc<AlsaLibrary>,
    /// Raw parser handle.
    pub(crate) raw: *mut snd_midi_event_t,
}

/// One owned ALSA sequencer queue with one runtime time anchor.
pub(crate) struct AlsaSequencerQueue {
    /// The host queue id.
    pub(crate) id: i32,
    /// The process-monotonic epoch used for this queue.
    pub(crate) start_epoch_ns: u64,
}

/// One stable ALSA endpoint key.
pub(crate) type AlsaEndpointKey = (i32, i32);

/// One cached ALSA endpoint row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AlsaEndpointInfo {
    /// Descriptor snapshot for this endpoint.
    pub(crate) descriptor: MidiPortDescriptorValue,
    /// ALSA client id.
    pub(crate) client_id: i32,
    /// ALSA port id.
    pub(crate) port_id: i32,
}

/// One process-global snapshot of ALSA sequencer topology.
#[derive(Default)]
pub(crate) struct AlsaTopologyState {
    /// Current input endpoint rows.
    pub(crate) inputs: BTreeMap<AlsaEndpointKey, AlsaEndpointInfo>,
    /// Current output endpoint rows.
    pub(crate) outputs: BTreeMap<AlsaEndpointKey, AlsaEndpointInfo>,
}

/// One opened ALSA input-session kind.
pub(crate) enum AlsaInputRepositoryKind {
    /// One host source connected into one hidden destination port.
    Source {
        /// The owned ALSA client handle.
        handle: AlsaHandle,
        /// The queue that timestamps inbound events.
        queue: AlsaSequencerQueue,
        /// The local destination port id.
        local_port_id: i32,
        /// The connected remote source client id.
        remote_client_id: i32,
        /// The connected remote source port id.
        remote_port_id: i32,
        /// Stop flag for the reader thread.
        stop_flag: Arc<AtomicBool>,
        /// The reader thread handle.
        reader_thread: Option<JoinHandle<()>>,
    },
    /// One runtime-owned virtual destination port.
    VirtualDestination {
        /// The owned ALSA client handle.
        handle: AlsaHandle,
        /// The queue that timestamps inbound events.
        queue: AlsaSequencerQueue,
        /// The visible destination port id.
        local_port_id: i32,
        /// Stop flag for the reader thread.
        stop_flag: Arc<AtomicBool>,
        /// The reader thread handle.
        reader_thread: Option<JoinHandle<()>>,
    },
}

/// One opened ALSA output-session kind.
pub(crate) enum AlsaOutputRepositoryKind {
    /// One hidden source connected to one remote destination.
    Destination {
        /// The owned ALSA client handle.
        handle: AlsaHandle,
        /// The queue that schedules outbound events.
        queue: AlsaSequencerQueue,
        /// The local source port id.
        local_port_id: i32,
        /// The connected remote destination client id.
        remote_client_id: i32,
        /// The connected remote destination port id.
        remote_port_id: i32,
        /// The MIDI encoder state.
        parser: Mutex<AlsaMidiParser>,
    },
    /// One runtime-owned virtual source port.
    VirtualSource {
        /// The owned ALSA client handle.
        handle: AlsaHandle,
        /// The queue that schedules outbound events.
        queue: AlsaSequencerQueue,
        /// The visible source port id.
        local_port_id: i32,
        /// The MIDI encoder state.
        parser: Mutex<AlsaMidiParser>,
    },
}

/// One opened ALSA input session.
pub(crate) struct AlsaInputRepository {
    /// Shared service owner.
    pub(crate) _service: Arc<AlsaService>,
    /// Current descriptor snapshot.
    pub(crate) descriptor: MidiPortDescriptorValue,
    /// Shared input queue.
    pub(crate) queue: Arc<BoundedQueue<MidiInputRecordValue>>,
    /// Deferred terminal reader failure.
    pub(crate) terminal_error: Arc<Mutex<Option<AlsaInputTerminalError>>>,
    /// Repository resources that must be released.
    pub(crate) kind: Mutex<AlsaInputRepositoryKind>,
}

/// One terminal input-reader failure.
#[derive(Debug, Clone)]
pub(crate) enum AlsaInputTerminalError {
    /// The reader could not create one ALSA MIDI parser.
    ParserInitializationFailed,
    /// The reader could not acquire any poll descriptors.
    PollDescriptorUnavailable,
    /// The reader could not query the current poll descriptors.
    PollDescriptorQueryFailed,
    /// The reader received one event without one realtime queue timestamp.
    MissingRealtimeTimestamp,
    /// The sequencer input stream failed after the session was opened.
    SequencerInputFailed(String),
}

impl AlsaInputTerminalError {
    /// Return one stable runtime error message for this terminal failure.
    pub(crate) fn message(&self) -> String {
        match self {
            Self::ParserInitializationFailed => {
                "ALSA input reader could not initialize one MIDI parser".to_string()
            }
            Self::PollDescriptorUnavailable => {
                "ALSA input reader could not acquire any poll descriptors".to_string()
            }
            Self::PollDescriptorQueryFailed => {
                "ALSA input reader could not query poll descriptors".to_string()
            }
            Self::MissingRealtimeTimestamp => {
                "ALSA input reader received one event without one realtime queue timestamp"
                    .to_string()
            }
            Self::SequencerInputFailed(error) => {
                format!("ALSA input reader lost sequencer input: {error}")
            }
        }
    }
}

/// One opened ALSA output session.
pub(crate) struct AlsaOutputRepository {
    /// Shared service owner.
    pub(crate) _service: Arc<AlsaService>,
    /// Current descriptor snapshot.
    pub(crate) descriptor: MidiPortDescriptorValue,
    /// Selected data format.
    pub(crate) data_format: MidiDataFormat,
    /// Selected protocol.
    pub(crate) protocol: Option<MidiProtocol>,
    /// Repository resources that must be released.
    pub(crate) kind: AlsaOutputRepositoryKind,
}

/// One stable key for one direction-scoped snapshot row.
pub(crate) type SnapshotKey = String;

/// One event delivery mode for one opened subscription.
#[derive(Clone)]
pub(crate) enum AlsaEventDeliveryKind {
    /// Use the native ALSA announce feed.
    Native {
        /// Shared native event registry.
        registry: Arc<Mutex<AlsaNativeEventRegistry>>,
        /// Registration id in the native registry.
        registration_id: u64,
    },
    /// Use synthetic polling snapshots.
    Poll,
}

/// One opened MIDI event subscription.
#[derive(Clone)]
pub(crate) struct AlsaEventRepository {
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
    pub(crate) delivery_kind: AlsaEventDeliveryKind,
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
pub(crate) struct AlsaInputResource {
    /// Shared session state.
    pub(crate) session: Arc<AlsaInputRepository>,
}

/// Resource payload for one output session.
pub(crate) struct AlsaOutputResource {
    /// Shared session state.
    pub(crate) session: Arc<AlsaOutputRepository>,
}

/// Resource payload for one event subscription.
#[derive(Clone)]
pub(crate) struct AlsaEventResource {
    /// Shared subscription state.
    pub(crate) session: Arc<Mutex<AlsaEventRepository>>,
}

unsafe impl Send for AlsaHandle {}
unsafe impl Sync for AlsaHandle {}
unsafe impl Send for AlsaMidiParser {}

impl Drop for AlsaHandle {
    /// Close one owned ALSA sequencer handle.
    fn drop(&mut self) {
        if self.raw.is_null() {
            return;
        }

        unsafe {
            let _ = (self.library.api.snd_seq_close)(self.raw);
        }
    }
}

impl Drop for AlsaMidiParser {
    /// Release one ALSA MIDI parser.
    fn drop(&mut self) {
        if self.raw.is_null() {
            return;
        }

        unsafe {
            (self.library.api.snd_midi_event_free)(self.raw);
        }
    }
}

impl Drop for AlsaInputRepository {
    /// Release ALSA resources for one input session.
    fn drop(&mut self) {
        let mut kind = self.kind.lock();

        // stop the reader thread before tearing down the ALSA handle
        match &mut *kind {
            AlsaInputRepositoryKind::Source {
                stop_flag,
                reader_thread,
                ..
            }
            | AlsaInputRepositoryKind::VirtualDestination {
                stop_flag,
                reader_thread,
                ..
            } => {
                stop_flag.store(true, Ordering::Release);

                if let Some(reader_thread) = reader_thread.take() {
                    let _ = reader_thread.join();
                }
            }
        }

        // session-specific teardown
        match &*kind {
            AlsaInputRepositoryKind::Source {
                handle,
                queue,
                local_port_id,
                remote_client_id,
                remote_port_id,
                ..
            } => {
                let _ = unsubscribe_from_with_timestamps(
                    handle,
                    *local_port_id,
                    *remote_client_id,
                    *remote_port_id,
                );
                let _ = free_queue(handle, queue.id);
                let _ = delete_simple_port(handle, *local_port_id);
            }
            AlsaInputRepositoryKind::VirtualDestination {
                handle,
                queue,
                local_port_id,
                ..
            } => {
                let _ = free_queue(handle, queue.id);
                let _ = delete_simple_port(handle, *local_port_id);
            }
        }

        self.queue.close();
    }
}

impl Drop for AlsaOutputRepository {
    /// Release ALSA resources for one output session.
    fn drop(&mut self) {
        match &self.kind {
            AlsaOutputRepositoryKind::Destination {
                handle,
                queue,
                local_port_id,
                remote_client_id,
                remote_port_id,
                ..
            } => {
                let _ = disconnect_to(handle, *local_port_id, *remote_client_id, *remote_port_id);
                let _ = free_queue(handle, queue.id);
                let _ = delete_simple_port(handle, *local_port_id);
            }
            AlsaOutputRepositoryKind::VirtualSource {
                handle,
                queue,
                local_port_id,
                ..
            } => {
                let _ = free_queue(handle, queue.id);
                let _ = delete_simple_port(handle, *local_port_id);
            }
        }
    }
}

define_backend_midi_resource_inserters!(
    vis = pub(crate),
    backend = MidiBackend::Alsa,
    input = (insert_input_resource, AlsaInputResource, AlsaInputRepository),
    output = (insert_output_resource, AlsaOutputResource, AlsaOutputRepository),
    event = (insert_event_resource, AlsaEventResource, AlsaEventRepository)
);
