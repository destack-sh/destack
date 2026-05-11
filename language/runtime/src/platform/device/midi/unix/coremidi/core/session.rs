use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Duration;

use parking_lot::Mutex;

use crate::platform::device::midi::core::{
    MidiEventValue, MidiInputRecordValue, MidiPortDescriptorValue,
    define_backend_midi_resource_inserters,
};
use crate::platform::device::{
    MidiBackend, MidiDataFormat, MidiEventOverflowPolicy, MidiEventSubscriptionFlags,
    MidiPortDirection, MidiPortDirectionFlags, MidiProtocol,
};
use crate::runtime::control::queue::BoundedQueue;
use crate::runtime::service::executor::periodic::PeriodicTaskHandle;

use super::super::abi::{
    MIDIEndpointDispose, MIDIEndpointRef, MIDIPortDisconnectSource, MIDIPortDispose, MIDIPortRef,
};
use super::super::callback::{CoreMidiReceiveBlock, LegacyInputCallbackToken};
use super::super::descriptor::unregister_endpoint_override;
use super::super::service::{CoreMidiNativeEventRegistry, CoreMidiService};

/// One runtime transport override for one virtual endpoint.
#[derive(Debug, Clone, Copy)]
pub(crate) struct CoreMidiEndpointOverride {
    /// Live CoreMIDI endpoint reference for this virtual endpoint.
    pub(crate) endpoint: MIDIEndpointRef,
    /// Transport data format reported for the endpoint.
    pub(crate) data_format: MidiDataFormat,
    /// Protocol semantics reported for the endpoint.
    pub(crate) protocol: MidiProtocol,
}

/// One opened input-session variant.
pub(crate) enum CoreMidiInputRepositoryKind {
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
pub(crate) enum CoreMidiOutputRepositoryKind {
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
pub(crate) struct CoreMidiInputRepository {
    /// Shared service owner.
    pub(crate) _service: Arc<CoreMidiService>,
    /// Current descriptor snapshot.
    pub(crate) descriptor: MidiPortDescriptorValue,
    /// Shared input queue.
    pub(crate) queue: Arc<BoundedQueue<MidiInputRecordValue>>,
    /// Repository resources that must be released.
    pub(crate) kind: CoreMidiInputRepositoryKind,
}

/// One opened CoreMIDI output session.
pub(crate) struct CoreMidiOutputRepository {
    /// Shared service owner.
    pub(crate) _service: Arc<CoreMidiService>,
    /// Current descriptor snapshot.
    pub(crate) descriptor: MidiPortDescriptorValue,
    /// Selected data format.
    pub(crate) data_format: MidiDataFormat,
    /// Selected protocol.
    pub(crate) protocol: Option<MidiProtocol>,
    /// Repository resources that must be released.
    pub(crate) kind: CoreMidiOutputRepositoryKind,
}

/// One stable key for one direction-scoped snapshot row.
pub(crate) type SnapshotKey = String;

/// One event delivery mode for one opened subscription.
#[derive(Clone)]
pub(crate) enum CoreMidiEventDeliveryKind {
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
pub(crate) struct CoreMidiEventRepository {
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
    pub(crate) delivery_kind: CoreMidiEventDeliveryKind,
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
pub(crate) struct CoreMidiInputResource {
    /// Shared session state.
    pub(crate) session: Arc<CoreMidiInputRepository>,
}

/// Resource payload for one output session.
pub(crate) struct CoreMidiOutputResource {
    /// Shared session state.
    pub(crate) session: Arc<CoreMidiOutputRepository>,
}

/// Resource payload for one event subscription.
#[derive(Clone)]
pub(crate) struct CoreMidiEventResource {
    /// Shared subscription state.
    pub(crate) session: Arc<Mutex<CoreMidiEventRepository>>,
}

impl Drop for CoreMidiInputRepository {
    /// Release CoreMIDI resources for one input session.
    fn drop(&mut self) {
        match &self.kind {
            CoreMidiInputRepositoryKind::LegacySource {
                port,
                source,
                _callback_context: _,
            } => unsafe {
                let _ = MIDIPortDisconnectSource(*port, *source);
                let _ = MIDIPortDispose(*port);
            },
            CoreMidiInputRepositoryKind::ModernSource {
                port,
                source,
                _receive_block: _,
            } => unsafe {
                let _ = MIDIPortDisconnectSource(*port, *source);
                let _ = MIDIPortDispose(*port);
            },
            CoreMidiInputRepositoryKind::LegacyVirtualDestination {
                endpoint,
                _callback_context: _,
            } => unsafe {
                unregister_endpoint_override(&self._service, *endpoint);
                let _ = MIDIEndpointDispose(*endpoint);
            },
            CoreMidiInputRepositoryKind::ModernVirtualDestination {
                endpoint,
                _receive_block: _,
            } => unsafe {
                unregister_endpoint_override(&self._service, *endpoint);
                let _ = MIDIEndpointDispose(*endpoint);
            },
        }

        self.queue.close();
    }
}

impl Drop for CoreMidiOutputRepository {
    /// Release CoreMIDI resources for one output session.
    fn drop(&mut self) {
        match &self.kind {
            CoreMidiOutputRepositoryKind::Destination { port, .. } => unsafe {
                let _ = MIDIPortDispose(*port);
            },
            CoreMidiOutputRepositoryKind::VirtualSource { endpoint } => unsafe {
                unregister_endpoint_override(&self._service, *endpoint);
                let _ = MIDIEndpointDispose(*endpoint);
            },
        }
    }
}

impl CoreMidiInputRepository {
    /// Return whether this session owns one virtual endpoint.
    pub(crate) fn is_virtual_endpoint(&self) -> bool {
        matches!(
            self.kind,
            CoreMidiInputRepositoryKind::LegacyVirtualDestination { .. }
                | CoreMidiInputRepositoryKind::ModernVirtualDestination { .. }
        )
    }
}

impl CoreMidiOutputRepository {
    /// Return whether this session owns one virtual endpoint.
    pub(crate) fn is_virtual_endpoint(&self) -> bool {
        matches!(
            self.kind,
            CoreMidiOutputRepositoryKind::VirtualSource { .. }
        )
    }
}

define_backend_midi_resource_inserters!(
    vis = pub(crate),
    backend = MidiBackend::CoreMIDI,
    input = (insert_input_resource, CoreMidiInputResource, CoreMidiInputRepository),
    output = (insert_output_resource, CoreMidiOutputResource, CoreMidiOutputRepository),
    event = (insert_event_resource, CoreMidiEventResource, CoreMidiEventRepository)
);
