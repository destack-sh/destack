use std::sync::Arc;

use smallvec::SmallVec;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::VmSlice;
use crate::platform::core::BackendSupport;
use crate::platform::device::{
    MidiBackend, MidiBackendCapabilityFlags, MidiBackendDescriptor, MidiBackendDescriptorVm,
    MidiBackendDisconnectedEvent, MidiBackendDisconnectedEventVm, MidiDataFormat,
    MidiDataFormatFlags, MidiEvent, MidiEventMetadata, MidiEventSource, MidiEventVm,
    MidiInputRecord, MidiInputRecordVm, MidiOutputRecord, MidiOutputRecordVm, MidiPortAddedEvent,
    MidiPortAddedEventVm, MidiPortChangedEvent, MidiPortChangedEventVm, MidiPortDescriptor,
    MidiPortDescriptorVm, MidiPortDirection, MidiPortRemovedEvent, MidiPortRemovedEventVm,
    MidiProtocol, MidiProtocolFlags, MidiRecordFraming,
};
use crate::runtime::BindingCallContext;
use destack_vm as vm;

/// Backend descriptor data before ABI encoding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MidiBackendDescriptorValue {
    /// Backend selector.
    pub backend: MidiBackend,
    /// Stable backend name.
    pub name: &'static str,
    /// Support state for the selector.
    pub support: BackendSupport,
    /// Auto-selection priority.
    pub priority: u16,
    /// Backend capability flags.
    pub capability_flags: MidiBackendCapabilityFlags,
    /// Supported transport data formats.
    pub supported_data_formats: MidiDataFormatFlags,
    /// Supported transport protocols.
    pub supported_protocols: MidiProtocolFlags,
}

impl MidiBackendDescriptorValue {
    /// Encode one backend descriptor for native bindings.
    pub(crate) fn into_native(self, binding: &BindingCallContext) -> MidiBackendDescriptor {
        MidiBackendDescriptor {
            backend: self.backend,
            name: binding.store_string(self.name),
            support: self.support,
            priority: self.priority,
            capability_flags: self.capability_flags,
            supported_data_formats: self.supported_data_formats,
            supported_protocols: self.supported_protocols,
        }
    }

    /// Encode one backend descriptor for VM bindings.
    pub(crate) fn into_vm(
        self,
        context: &mut vm::BindingContext<'_>,
    ) -> RuntimeResult<MidiBackendDescriptorVm> {
        Ok(MidiBackendDescriptorVm {
            backend: self.backend,
            name: context
                .string_handle(self.name)
                .map_err(Box::<RuntimeError>::from)?,
            support: self.support,
            priority: self.priority,
            capability_flags: self.capability_flags,
            supported_data_formats: self.supported_data_formats,
            supported_protocols: self.supported_protocols,
        })
    }
}

/// MIDI port descriptor data before ABI encoding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MidiPortDescriptorValue {
    /// Resolved backend.
    pub backend: MidiBackend,
    /// Stable runtime id.
    pub id: String,
    /// Stable runtime group id.
    pub group_id: Option<String>,
    /// Backend-native endpoint id.
    pub backend_id: Option<String>,
    /// Host-visible endpoint name.
    pub name: String,
    /// Host-visible group or device name.
    pub group_name: Option<String>,
    /// Host-visible manufacturer.
    pub manufacturer: Option<String>,
    /// Host-visible model.
    pub model: Option<String>,
    /// Host-visible version.
    pub version: Option<String>,
    /// Supported data formats.
    pub supported_data_formats: MidiDataFormatFlags,
    /// Default data format.
    pub default_data_format: Option<MidiDataFormat>,
    /// Supported protocols.
    pub supported_protocols: MidiProtocolFlags,
    /// Default protocol.
    pub default_protocol: Option<MidiProtocol>,
    /// Whether the endpoint is virtual.
    pub is_virtual: bool,
    /// Whether the endpoint is currently connected.
    pub is_connected: bool,
}

impl MidiPortDescriptorValue {
    /// Encode one port descriptor for native bindings.
    pub(crate) fn into_native(self, binding: &BindingCallContext) -> MidiPortDescriptor {
        MidiPortDescriptor {
            backend: self.backend,
            id: binding.store_string(&self.id),
            group_id: self
                .group_id
                .as_ref()
                .map(|value| binding.store_string(value)),
            backend_id: self
                .backend_id
                .as_ref()
                .map(|value| binding.store_string(value)),
            name: binding.store_string(&self.name),
            group_name: self
                .group_name
                .as_ref()
                .map(|value| binding.store_string(value)),
            manufacturer: self
                .manufacturer
                .as_ref()
                .map(|value| binding.store_string(value)),
            model: self.model.as_ref().map(|value| binding.store_string(value)),
            version: self
                .version
                .as_ref()
                .map(|value| binding.store_string(value)),
            supported_data_formats: self.supported_data_formats,
            default_data_format: self.default_data_format,
            supported_protocols: self.supported_protocols,
            default_protocol: self.default_protocol,
            is_virtual: self.is_virtual,
            is_connected: self.is_connected,
        }
    }

    /// Encode one port descriptor for VM bindings.
    pub(crate) fn into_vm(
        self,
        context: &mut vm::BindingContext<'_>,
    ) -> RuntimeResult<MidiPortDescriptorVm> {
        Ok(MidiPortDescriptorVm {
            backend: self.backend,
            id: context
                .string_handle(&self.id)
                .map_err(Box::<RuntimeError>::from)?,
            group_id: self
                .group_id
                .as_ref()
                .map(|value| {
                    context
                        .string_handle(value)
                        .map_err(Box::<RuntimeError>::from)
                })
                .transpose()?,
            backend_id: self
                .backend_id
                .as_ref()
                .map(|value| {
                    context
                        .string_handle(value)
                        .map_err(Box::<RuntimeError>::from)
                })
                .transpose()?,
            name: context
                .string_handle(&self.name)
                .map_err(Box::<RuntimeError>::from)?,
            group_name: self
                .group_name
                .as_ref()
                .map(|value| {
                    context
                        .string_handle(value)
                        .map_err(Box::<RuntimeError>::from)
                })
                .transpose()?,
            manufacturer: self
                .manufacturer
                .as_ref()
                .map(|value| {
                    context
                        .string_handle(value)
                        .map_err(Box::<RuntimeError>::from)
                })
                .transpose()?,
            model: self
                .model
                .as_ref()
                .map(|value| {
                    context
                        .string_handle(value)
                        .map_err(Box::<RuntimeError>::from)
                })
                .transpose()?,
            version: self
                .version
                .as_ref()
                .map(|value| {
                    context
                        .string_handle(value)
                        .map_err(Box::<RuntimeError>::from)
                })
                .transpose()?,
            supported_data_formats: self.supported_data_formats,
            default_data_format: self.default_data_format,
            supported_protocols: self.supported_protocols,
            default_protocol: self.default_protocol,
            is_virtual: self.is_virtual,
            is_connected: self.is_connected,
        })
    }
}

/// MIDI record payload bytes with a small inline fast path.
pub(crate) type MidiRecordBytes = SmallVec<[u8; 16]>;

/// Inbound MIDI record data before ABI encoding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MidiInputRecordValue {
    /// Receive timestamp in runtime monotonic nanoseconds.
    pub received_at_ns: u64,
    /// Source id when the backend reports one.
    pub source_id: Option<Arc<str>>,
    /// Transport data format.
    pub data_format: MidiDataFormat,
    /// Protocol semantics when known.
    pub protocol: Option<MidiProtocol>,
    /// Record framing.
    pub framing: MidiRecordFraming,
    /// Record payload bytes.
    pub data: MidiRecordBytes,
}

impl MidiInputRecordValue {
    /// Encode one input record for native bindings.
    pub(crate) fn into_native(self, binding: &BindingCallContext) -> MidiInputRecord {
        MidiInputRecord {
            received_at_ns: self.received_at_ns,
            source_id: self
                .source_id
                .as_deref()
                .map(|value| binding.store_string(value)),
            data_format: self.data_format,
            protocol: self.protocol,
            framing: self.framing,
            data: binding.store_slice(self.data.into_vec()),
        }
    }

    /// Encode one input record for VM bindings.
    pub(crate) fn into_vm(
        self,
        context: &mut vm::BindingContext<'_>,
    ) -> RuntimeResult<MidiInputRecordVm> {
        Ok(MidiInputRecordVm {
            received_at_ns: self.received_at_ns,
            source_id: self
                .source_id
                .as_deref()
                .map(|value| {
                    context
                        .intern_string(value)
                        .map(vm::StringHandle::new)
                        .map_err(RuntimeError::from)
                })
                .transpose()?,
            data_format: self.data_format,
            protocol: self.protocol,
            framing: self.framing,
            data: VmSlice::from_bytes(&mut context.write(), &self.data)?,
        })
    }
}

/// Outbound MIDI record data before ABI encoding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MidiOutputRecordValue {
    /// Scheduled send timestamp in runtime monotonic nanoseconds.
    pub send_at_ns: Option<u64>,
    /// Transport data format.
    pub data_format: MidiDataFormat,
    /// Protocol semantics when known.
    pub protocol: Option<MidiProtocol>,
    /// Record framing.
    pub framing: MidiRecordFraming,
    /// Record payload bytes.
    pub data: MidiRecordBytes,
}

impl MidiOutputRecordValue {
    /// Decode one native output record into owned bytes.
    pub(crate) fn from_native(record: MidiOutputRecord) -> RuntimeResult<Self> {
        Ok(Self {
            send_at_ns: record.send_at_ns,
            data_format: record.data_format,
            protocol: record.protocol,
            framing: record.framing,
            data: SmallVec::from_slice(unsafe { record.data.as_slice()? }),
        })
    }

    /// Decode one VM output record into owned bytes.
    pub(crate) fn from_vm(
        context: &mut vm::BindingContext<'_>,
        record: MidiOutputRecordVm,
    ) -> RuntimeResult<Self> {
        Ok(Self {
            send_at_ns: record.send_at_ns,
            data_format: record.data_format,
            protocol: record.protocol,
            framing: record.framing,
            data: SmallVec::from_vec(record.data.read_bytes(&context.read())?),
        })
    }
}

/// Shared metadata for one MIDI event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct MidiEventMetadataValue {
    /// Event timestamp in runtime monotonic nanoseconds.
    pub timestamp_ns: u64,
    /// Sequence number in one subscription queue.
    pub sequence: u64,
    /// Number of dropped events before this event.
    pub dropped_count: u64,
    /// Event source selector.
    pub source: MidiEventSource,
    /// Backend selector.
    pub backend: MidiBackend,
}

impl MidiEventMetadataValue {
    /// Encode one event metadata payload for native bindings.
    pub(crate) fn into_native(self) -> MidiEventMetadata {
        MidiEventMetadata {
            timestamp_ns: self.timestamp_ns,
            sequence: self.sequence,
            dropped_count: self.dropped_count,
            source: self.source,
            backend: self.backend,
        }
    }

    /// Encode one event metadata payload for VM bindings.
    pub(crate) fn into_vm(self) -> MidiEventMetadata {
        self.into_native()
    }
}

/// Event data before ABI encoding.
#[cfg_attr(any(target_os = "ios", target_os = "android"), allow(dead_code))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum MidiEventValue {
    /// Port added event.
    PortAdded {
        /// Shared metadata.
        metadata: MidiEventMetadataValue,
        /// Port direction.
        direction: MidiPortDirection,
        /// Added descriptor.
        descriptor: MidiPortDescriptorValue,
    },
    /// Port removed event.
    PortRemoved {
        /// Shared metadata.
        metadata: MidiEventMetadataValue,
        /// Port direction.
        direction: MidiPortDirection,
        /// Removed runtime id.
        id: String,
        /// Removed runtime group id.
        group_id: Option<String>,
    },
    /// Port changed event.
    PortChanged {
        /// Shared metadata.
        metadata: MidiEventMetadataValue,
        /// Port direction.
        direction: MidiPortDirection,
        /// Updated descriptor.
        descriptor: MidiPortDescriptorValue,
    },
    /// Backend disconnected event.
    BackendDisconnected {
        /// Shared metadata.
        metadata: MidiEventMetadataValue,
        /// Future detail flags.
        flags: u32,
    },
}

impl MidiEventValue {
    /// Encode one MIDI event for native bindings.
    pub(crate) fn into_native(self, binding: &BindingCallContext) -> MidiEvent {
        match self {
            Self::PortAdded {
                metadata,
                direction,
                descriptor,
            } => MidiEvent::MidiPortAddedEvent(MidiPortAddedEvent {
                kind: binding.store_string("portAdded"),
                metadata: metadata.into_native(),
                direction,
                descriptor: descriptor.into_native(binding),
            }),
            Self::PortRemoved {
                metadata,
                direction,
                id,
                group_id,
            } => MidiEvent::MidiPortRemovedEvent(MidiPortRemovedEvent {
                kind: binding.store_string("portRemoved"),
                metadata: metadata.into_native(),
                direction,
                id: binding.store_string(&id),
                group_id: group_id.as_ref().map(|value| binding.store_string(value)),
            }),
            Self::PortChanged {
                metadata,
                direction,
                descriptor,
            } => MidiEvent::MidiPortChangedEvent(MidiPortChangedEvent {
                kind: binding.store_string("portChanged"),
                metadata: metadata.into_native(),
                direction,
                descriptor: descriptor.into_native(binding),
            }),
            Self::BackendDisconnected { metadata, flags } => {
                MidiEvent::MidiBackendDisconnectedEvent(MidiBackendDisconnectedEvent {
                    kind: binding.store_string("backendDisconnected"),
                    metadata: metadata.into_native(),
                    flags,
                })
            }
        }
    }

    /// Encode one MIDI event for VM bindings.
    pub(crate) fn into_vm(
        self,
        context: &mut vm::BindingContext<'_>,
    ) -> RuntimeResult<MidiEventVm> {
        match self {
            Self::PortAdded {
                metadata,
                direction,
                descriptor,
            } => Ok(MidiEventVm::MidiPortAddedEvent(MidiPortAddedEventVm {
                kind: context
                    .string_handle("portAdded")
                    .map_err(Box::<RuntimeError>::from)?,
                metadata: metadata.into_vm(),
                direction,
                descriptor: descriptor.into_vm(context)?,
            })),
            Self::PortRemoved {
                metadata,
                direction,
                id,
                group_id,
            } => Ok(MidiEventVm::MidiPortRemovedEvent(MidiPortRemovedEventVm {
                kind: context
                    .string_handle("portRemoved")
                    .map_err(Box::<RuntimeError>::from)?,
                metadata: metadata.into_vm(),
                direction,
                id: context
                    .string_handle(&id)
                    .map_err(Box::<RuntimeError>::from)?,
                group_id: group_id
                    .as_ref()
                    .map(|value| {
                        context
                            .string_handle(value)
                            .map_err(Box::<RuntimeError>::from)
                    })
                    .transpose()?,
            })),
            Self::PortChanged {
                metadata,
                direction,
                descriptor,
            } => Ok(MidiEventVm::MidiPortChangedEvent(MidiPortChangedEventVm {
                kind: context
                    .string_handle("portChanged")
                    .map_err(Box::<RuntimeError>::from)?,
                metadata: metadata.into_vm(),
                direction,
                descriptor: descriptor.into_vm(context)?,
            })),
            Self::BackendDisconnected { metadata, flags } => Ok(
                MidiEventVm::MidiBackendDisconnectedEvent(MidiBackendDisconnectedEventVm {
                    kind: context
                        .string_handle("backendDisconnected")
                        .map_err(Box::<RuntimeError>::from)?,
                    metadata: metadata.into_vm(),
                    flags,
                }),
            ),
        }
    }
}
