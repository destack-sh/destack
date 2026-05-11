use std::collections::BTreeMap;
use std::sync::{Arc, Weak};

use parking_lot::Mutex;
use windows::Foundation::TypedEventHandler;
use windows::core::{Error as WinError, HSTRING, IInspectable, Ref};

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::core::{
    self as core_platform, qpc_hundred_nanos_to_process_nanos, qpc_process_nanos_to_hundred_nanos,
};
use crate::platform::device::midi::core::{
    MidiInputRecordValue, MidiOutputRecordValue, MidiPortDescriptorValue, MidiRecordBytes,
    binding_timestamp_now,
};
use crate::platform::device::{
    MidiDataFormat, MidiEventSource, MidiPortDirection, MidiProtocol, MidiRecordFraming,
};
use crate::runtime::control::queue::BoundedQueue;
use crate::runtime::service::executor::thread::ServiceThreadExecutor;
use crate::runtime::service::{Service, spawn_service_thread};
use crate::runtime::{ExecutionAffinity, ExecutionMode, ExecutionPolicy};

use super::abi::{
    connection_add_message_received, watcher_add_added, watcher_add_enumeration_completed,
    watcher_add_removed, watcher_add_stopped, watcher_add_updated,
};
use super::core::{
    WindowsMidiEndpointInfo, WindowsMidiEventDeliveryKind, WindowsMidiEventRepository,
    WindowsMidiTopologyState,
};
use super::descriptor::device_descriptor;
use super::event::{queue_backend_disconnected_events, refresh_native_event_sessions};
use super::sdk::{
    MidiDeclaredDeviceIdentity, MidiDeclaredEndpointInfo, MidiEndpointConnection,
    MidiEndpointConnectionBasicSettings, MidiEndpointDeviceInformation,
    MidiEndpointDeviceInformationAddedEventArgs, MidiEndpointDeviceInformationRemovedEventArgs,
    MidiEndpointDeviceInformationUpdatedEventArgs, MidiEndpointDeviceWatcher,
    MidiEndpointUserSuppliedInfo, MidiMessageReceivedEventArgs, MidiMessageStruct, MidiRepository,
    MidiSendMessageResults, MidiVirtualDevice, MidiVirtualDeviceCreationConfig,
    MidiVirtualDeviceManager,
};

/// One process-global Windows MIDI runtime service.
pub(crate) struct WindowsMidiService {
    /// Dedicated Windows MIDI service-thread executor.
    executor: ServiceThreadExecutor<WindowsMidiServiceState>,
    /// Shared topology cache.
    pub(super) topology: Arc<Mutex<WindowsMidiTopologyState>>,
    /// Registered native event subscriptions.
    pub(super) native_event_registry: Arc<Mutex<WindowsMidiNativeEventRegistry>>,
}

/// One registry of native event subscriptions.
pub(super) struct WindowsMidiNativeEventRegistry {
    /// Next registration id.
    pub(super) next_registration_id: u64,
    /// Registered native event subscriptions.
    pub(super) sessions: BTreeMap<u64, Weak<Mutex<WindowsMidiEventRepository>>>,
}

/// One host-owned Windows MIDI service state that lives on the executor thread.
struct WindowsMidiServiceState {
    /// Shared MIDI session.
    _session: MidiRepository,
    /// Shared topology cache.
    _topology: Arc<Mutex<WindowsMidiTopologyState>>,
    /// Registered native event subscriptions.
    _native_event_registry: Arc<Mutex<WindowsMidiNativeEventRegistry>>,
    /// Next input host-session id.
    next_input_session_id: u64,
    /// Live input host sessions.
    input_sessions: BTreeMap<u64, WindowsMidiInputHostRepository>,
    /// Next output host-session id.
    next_output_session_id: u64,
    /// Live output host sessions.
    output_sessions: BTreeMap<u64, WindowsMidiOutputHostRepository>,
    /// Live watcher and its event registrations.
    _watcher: WindowsMidiWatcherRegistration,
}

/// One host-owned input session.
struct WindowsMidiInputHostRepository {
    /// Shared MIDI session used for disconnection.
    session: MidiRepository,
    /// Opened Windows MIDI connection.
    connection: MidiEndpointConnection,
    /// Message-received registration token.
    token: i64,
    /// Owned virtual device when this session created one.
    _virtual_device: Option<MidiVirtualDevice>,
}

/// One host-owned output session.
struct WindowsMidiOutputHostRepository {
    /// Shared MIDI session used for disconnection.
    session: MidiRepository,
    /// Opened Windows MIDI connection.
    connection: MidiEndpointConnection,
    /// Owned virtual device when this session created one.
    _virtual_device: Option<MidiVirtualDevice>,
}

/// One watcher registration bundle.
struct WindowsMidiWatcherRegistration {
    /// Live watcher object.
    watcher: MidiEndpointDeviceWatcher,
    /// Added callback token.
    added_token: i64,
    /// Updated callback token.
    updated_token: i64,
    /// Removed callback token.
    removed_token: i64,
    /// Enumeration-completed callback token.
    enumeration_completed_token: i64,
    /// Stopped callback token.
    stopped_token: i64,
}

impl Drop for WindowsMidiInputHostRepository {
    /// Tear down one host-owned Windows MIDI input session.
    fn drop(&mut self) {
        let _ = self.connection.RemoveMessageReceived(self.token);
        disconnect_connection(&self.session, &self.connection);
    }
}

impl Drop for WindowsMidiOutputHostRepository {
    /// Tear down one host-owned Windows MIDI output session.
    fn drop(&mut self) {
        disconnect_connection(&self.session, &self.connection);
    }
}

impl Drop for WindowsMidiWatcherRegistration {
    /// Tear down one Windows MIDI device watcher.
    fn drop(&mut self) {
        let _ = self.watcher.RemoveAdded(self.added_token);
        let _ = self.watcher.RemoveUpdated(self.updated_token);
        let _ = self.watcher.RemoveRemoved(self.removed_token);
        let _ = self
            .watcher
            .RemoveEnumerationCompleted(self.enumeration_completed_token);
        let _ = self.watcher.RemoveStopped(self.stopped_token);
        let _ = self.watcher.Stop();
    }
}

impl WindowsMidiService {
    /// Create one virtual input host session on the dedicated service thread.
    pub(super) fn create_virtual_input_session(
        &self,
        name: String,
        manufacturer: Option<String>,
        model: Option<String>,
        version: Option<String>,
        protocol: MidiProtocol,
        queue: Arc<BoundedQueue<MidiInputRecordValue>>,
        terminal_error: Arc<Mutex<Option<String>>>,
        operation: &'static str,
    ) -> RuntimeResult<(u64, MidiPortDescriptorValue)> {
        self.executor.call(operation, move |state| {
            // virtual device
            let (virtual_device, device_endpoint_id, descriptor) = create_virtual_device(
                MidiPortDirection::Input,
                name,
                manufacturer,
                model,
                version,
                protocol,
                operation,
            )?;

            // device-side connection
            let connection =
                open_endpoint_connection(&state._session, &device_endpoint_id, operation)?;

            // input callback
            let callback_queue = queue.clone();
            let callback_terminal_error = terminal_error.clone();
            let token = connection_add_message_received(
                &connection,
                &TypedEventHandler::new(
                    move |_connection: Ref<'_, MidiEndpointConnection>,
                          args: Ref<'_, MidiMessageReceivedEventArgs>| {
                        if let Some(args) = args.as_ref() {
                            match decode_input_message(None, Some(protocol), args) {
                                Ok(record) => callback_queue.push_drop_oldest(record),
                                Err(error) => {
                                    let mut terminal_error = callback_terminal_error.lock();
                                    if terminal_error.is_none() {
                                        *terminal_error = Some(format!(
                                            "windows midi virtual input session failed to decode one inbound message: {error}",
                                        ));
                                    }
                                    callback_queue.close();
                                }
                            }
                        }

                        Ok(())
                    },
                ),
            )
            .map_err(|error| {
                windows_midi_error(operation, "MidiEndpointConnection::MessageReceived", &error)
            })?;

            // publish one new host session id
            let host_session_id = state.next_input_session_id;
            state.next_input_session_id = state.next_input_session_id.saturating_add(1);
            state.input_sessions.insert(
                host_session_id,
                WindowsMidiInputHostRepository {
                    session: state._session.clone(),
                    connection,
                    token,
                    _virtual_device: Some(virtual_device),
                },
            );

            // publish topology updates
            refresh_topology_cache(&state._topology, operation)?;
            refresh_native_event_sessions(
                &state._topology,
                &state._native_event_registry,
                MidiEventSource::Native,
            );

            Ok((host_session_id, descriptor))
        })
    }

    /// Open one input host session on the dedicated service thread.
    pub(super) fn open_input_session(
        &self,
        backend_id: String,
        descriptor: MidiPortDescriptorValue,
        protocol: Option<MidiProtocol>,
        queue: Arc<BoundedQueue<MidiInputRecordValue>>,
        terminal_error: Arc<Mutex<Option<String>>>,
        operation: &'static str,
    ) -> RuntimeResult<u64> {
        self.executor.call(operation, move |state| {
            // endpoint connection
            let connection = open_endpoint_connection(
                &state._session,
                &backend_id,
                "destack.device.midi.input.port.open",
            )?;

            // input callback
            let callback_queue = queue.clone();
            let callback_terminal_error = terminal_error.clone();
            let source_id = Arc::<str>::from(descriptor.id.clone());
            let token = connection_add_message_received(
                &connection,
                &TypedEventHandler::new(
                    move |_connection: Ref<'_, MidiEndpointConnection>,
                          args: Ref<'_, MidiMessageReceivedEventArgs>| {
                        if let Some(args) = args.as_ref()
                        {
                            match decode_input_message(Some(source_id.clone()), protocol, args) {
                                Ok(record) => callback_queue.push_drop_oldest(record),
                                Err(error) => {
                                    let mut terminal_error = callback_terminal_error.lock();
                                    if terminal_error.is_none() {
                                        *terminal_error = Some(format!(
                                            "windows midi input session failed to decode one inbound message: {error}",
                                        ));
                                    }
                                    callback_queue.close();
                                }
                            }
                        }

                        Ok(())
                    },
                ),
            )
            .map_err(|error| {
                windows_midi_error(operation, "MidiEndpointConnection::MessageReceived", &error)
            })?;

            // publish one new host session id
            let host_session_id = state.next_input_session_id;
            state.next_input_session_id = state.next_input_session_id.saturating_add(1);
            state.input_sessions.insert(
                host_session_id,
                WindowsMidiInputHostRepository {
                    session: state._session.clone(),
                    connection,
                    token,
                    _virtual_device: None,
                },
            );

            Ok(host_session_id)
        })
    }

    /// Close one input host session during resource drop.
    pub(super) fn close_input_session_for_drop(&self, host_session_id: u64) {
        let _ = self.executor.call(
            "destack.device.midi.windows-midi.input.drop",
            move |state| {
                let removed_session = state.input_sessions.remove(&host_session_id);
                let is_virtual = removed_session
                    .as_ref()
                    .and_then(|session| session._virtual_device.as_ref())
                    .is_some();
                drop(removed_session);

                if is_virtual {
                    refresh_topology_cache(
                        &state._topology,
                        "destack.device.midi.windows-midi.input.drop",
                    )?;
                    refresh_native_event_sessions(
                        &state._topology,
                        &state._native_event_registry,
                        MidiEventSource::Native,
                    );
                }

                Ok(())
            },
        );
    }

    /// Create one virtual output host session on the dedicated service thread.
    pub(super) fn create_virtual_output_session(
        &self,
        name: String,
        manufacturer: Option<String>,
        model: Option<String>,
        version: Option<String>,
        protocol: MidiProtocol,
        operation: &'static str,
    ) -> RuntimeResult<(u64, MidiPortDescriptorValue)> {
        self.executor.call(operation, move |state| {
            // virtual device
            let (virtual_device, device_endpoint_id, descriptor) = create_virtual_device(
                MidiPortDirection::Output,
                name,
                manufacturer,
                model,
                version,
                protocol,
                operation,
            )?;

            // device-side connection
            let connection =
                open_endpoint_connection(&state._session, &device_endpoint_id, operation)?;

            // publish one new host session id
            let host_session_id = state.next_output_session_id;
            state.next_output_session_id = state.next_output_session_id.saturating_add(1);
            state.output_sessions.insert(
                host_session_id,
                WindowsMidiOutputHostRepository {
                    session: state._session.clone(),
                    connection,
                    _virtual_device: Some(virtual_device),
                },
            );

            // publish topology updates
            refresh_topology_cache(&state._topology, operation)?;
            refresh_native_event_sessions(
                &state._topology,
                &state._native_event_registry,
                MidiEventSource::Native,
            );

            Ok((host_session_id, descriptor))
        })
    }

    /// Open one output host session on the dedicated service thread.
    pub(super) fn open_output_session(
        &self,
        backend_id: String,
        operation: &'static str,
    ) -> RuntimeResult<u64> {
        self.executor.call(operation, move |state| {
            // endpoint connection
            let connection = open_endpoint_connection(
                &state._session,
                &backend_id,
                "destack.device.midi.output.port.open",
            )?;

            // publish one new host session id
            let host_session_id = state.next_output_session_id;
            state.next_output_session_id = state.next_output_session_id.saturating_add(1);
            state.output_sessions.insert(
                host_session_id,
                WindowsMidiOutputHostRepository {
                    session: state._session.clone(),
                    connection,
                    _virtual_device: None,
                },
            );

            Ok(host_session_id)
        })
    }

    /// Close one output host session during resource drop.
    pub(super) fn close_output_session_for_drop(&self, host_session_id: u64) {
        let _ = self.executor.call(
            "destack.device.midi.windows-midi.output.drop",
            move |state| {
                let removed_session = state.output_sessions.remove(&host_session_id);
                let is_virtual = removed_session
                    .as_ref()
                    .and_then(|session| session._virtual_device.as_ref())
                    .is_some();
                drop(removed_session);

                if is_virtual {
                    refresh_topology_cache(
                        &state._topology,
                        "destack.device.midi.windows-midi.output.drop",
                    )?;
                    refresh_native_event_sessions(
                        &state._topology,
                        &state._native_event_registry,
                        MidiEventSource::Native,
                    );
                }

                Ok(())
            },
        );
    }

    /// Write one outbound record batch on the dedicated service thread.
    pub(super) fn write_output_records(
        &self,
        host_session_id: u64,
        records: Vec<MidiOutputRecordValue>,
        operation: &'static str,
    ) -> RuntimeResult<u32> {
        self.executor.call(operation, move |state| {
            let Some(session) = state.output_sessions.get(&host_session_id) else {
                return Err(core_platform::io_not_found(
                    operation,
                    format!("midi output host session {host_session_id} not found"),
                ));
            };

            // send one record at a time
            for record in &records {
                let words = output_words(record, operation)?;
                let timestamp_ns = record.send_at_ns.unwrap_or_else(binding_timestamp_now);
                let timestamp_100ns = qpc_process_nanos_to_hundred_nanos(timestamp_ns)
                    .ok_or_else(|| {
                        core_platform::io_operation_error(
                            operation,
                            None,
                            "failed to convert one scheduled timestamp into the Windows MIDI clock domain",
                        )
                    })?;

                let send_result = if words.len() <= u8::MAX as usize {
                    session
                        .connection
                        .SendSingleMessageWordArray(timestamp_100ns, 0, words.len() as u8, &words)
                } else {
                    session.connection.SendMultipleMessagesWordArray(
                        timestamp_100ns,
                        0,
                        words.len() as u32,
                        &words,
                    )
                }
                .map_err(|error| {
                    windows_midi_error(operation, "MidiEndpointConnection::SendMessage", &error)
                })?;

                validate_send_result(operation, send_result)?;
            }

            Ok(records.len() as u32)
        })
    }
}

impl Service for WindowsMidiService {
    const POLICY: ExecutionPolicy = ExecutionPolicy::process(ExecutionMode::Thread)
        .with_affinity(ExecutionAffinity::WindowsMta);
}

/// Return one shared connection-settings object.
fn connection_settings(
    operation: &'static str,
) -> RuntimeResult<MidiEndpointConnectionBasicSettings> {
    MidiEndpointConnectionBasicSettings::CreateInstance2(false, true)
        .or_else(|_| MidiEndpointConnectionBasicSettings::CreateInstance(false))
        .map_err(|error| {
            windows_midi_error(
                operation,
                "MidiEndpointConnectionBasicSettings::CreateInstance2",
                &error,
            )
        })
}

/// Open one endpoint connection on the service thread.
fn open_endpoint_connection(
    session: &MidiRepository,
    backend_id: &str,
    operation: &'static str,
) -> RuntimeResult<MidiEndpointConnection> {
    let settings = connection_settings(operation)?;
    let backend_id = HSTRING::from(backend_id);
    let connection = session
        .CreateEndpointConnection2(&backend_id, &settings)
        .map_err(|error| {
            windows_midi_error(
                operation,
                "MidiRepository::CreateEndpointConnection2",
                &error,
            )
        })?;

    let is_open = connection
        .Open()
        .map_err(|error| windows_midi_error(operation, "MidiEndpointConnection::Open", &error))?;
    if !is_open {
        disconnect_connection(session, &connection);
        return Err(core_platform::io_operation_error(
            operation,
            None,
            "MidiEndpointConnection::Open returned false",
        ));
    }

    Ok(connection)
}

/// Disconnect one endpoint connection.
fn disconnect_connection(session: &MidiRepository, connection: &MidiEndpointConnection) {
    let Ok(connection_id) = connection.ConnectionId() else {
        return;
    };

    let _ = session.DisconnectEndpointConnection(connection_id);
}

/// Decode one Windows MIDI input message into one inbound record.
fn decode_input_message(
    source_id: Option<Arc<str>>,
    protocol: Option<MidiProtocol>,
    args: &MidiMessageReceivedEventArgs,
) -> windows::core::Result<MidiInputRecordValue> {
    let mut message = MidiMessageStruct::default();
    let word_count = args.FillMessageStruct(&mut message)? as usize;
    let words: [u32; 4] = [message.Word0, message.Word1, message.Word2, message.Word3];
    let mut data = MidiRecordBytes::with_capacity(word_count.saturating_mul(4));

    for word in words.iter().take(word_count) {
        data.extend_from_slice(&word.to_be_bytes());
    }

    let received_at_ns =
        qpc_hundred_nanos_to_process_nanos(args.Timestamp()?).ok_or_else(|| {
            windows::core::Error::new(
                windows::core::HRESULT(0x8000_4005u32 as i32),
                "failed to convert one Windows MIDI timestamp into runtime monotonic time",
            )
        })?;

    Ok(MidiInputRecordValue {
        received_at_ns,
        source_id,
        data_format: MidiDataFormat::Ump,
        protocol,
        framing: MidiRecordFraming::Complete,
        data,
    })
}

/// Return whether the Windows MIDI virtual-device transport is available.
pub(crate) fn windows_midi_virtual_transport_available(
    operation: &'static str,
) -> RuntimeResult<bool> {
    MidiVirtualDeviceManager::IsTransportAvailable().map_err(|error| {
        windows_midi_error(
            operation,
            "MidiVirtualDeviceManager::IsTransportAvailable",
            &error,
        )
    })
}

/// Reject virtual-device requests when the Windows MIDI transport is unavailable.
fn require_virtual_transport_available(operation: &'static str) -> RuntimeResult<()> {
    if windows_midi_virtual_transport_available(operation)? {
        return Ok(());
    }

    Err(core_platform::not_supported(format!(
        "{operation}: Windows MIDI virtual devices are unavailable on this host",
    )))
}

/// Return one virtual endpoint description from optional model and version strings.
fn virtual_endpoint_description(model: Option<&str>, version: Option<&str>) -> String {
    match (model, version) {
        (Some(model), Some(version)) if !model.is_empty() && !version.is_empty() => {
            format!("{model} {version}")
        }
        (Some(model), _) if !model.is_empty() => model.to_string(),
        (_, Some(version)) if !version.is_empty() => version.to_string(),
        _ => String::new(),
    }
}

/// Build one declared endpoint-info payload for one requested protocol.
fn virtual_declared_endpoint_info(name: &str, protocol: MidiProtocol) -> MidiDeclaredEndpointInfo {
    MidiDeclaredEndpointInfo {
        Name: HSTRING::from(name),
        ProductInstanceId: HSTRING::new(),
        SupportsMidi10Protocol: matches!(protocol, MidiProtocol::Midi1),
        SupportsMidi20Protocol: matches!(protocol, MidiProtocol::Midi2),
        SupportsReceivingJitterReductionTimestamps: false,
        SupportsSendingJitterReductionTimestamps: false,
        HasStaticFunctionBlocks: false,
        DeclaredFunctionBlockCount: 0,
        SpecificationVersionMajor: 2,
        SpecificationVersionMinor: 0,
    }
}

/// Build one user-supplied-info payload for one virtual endpoint.
fn virtual_user_supplied_info(name: &str, description: &str) -> MidiEndpointUserSuppliedInfo {
    MidiEndpointUserSuppliedInfo {
        Name: HSTRING::from(name),
        Description: HSTRING::from(description),
        ImageFileName: HSTRING::new(),
        RequiresNoteOffTranslation: false,
        RecommendedControlChangeAutomationIntervalMilliseconds: 0,
        SupportsMidiPolyphonicExpression: false,
    }
}

/// Create one Windows MIDI virtual device and descriptor for one direction-scoped handle.
fn create_virtual_device(
    direction: MidiPortDirection,
    name: String,
    manufacturer: Option<String>,
    model: Option<String>,
    version: Option<String>,
    protocol: MidiProtocol,
    operation: &'static str,
) -> RuntimeResult<(MidiVirtualDevice, String, MidiPortDescriptorValue)> {
    require_virtual_transport_available(operation)?;

    // config
    let description = virtual_endpoint_description(model.as_deref(), version.as_deref());
    let declared_endpoint_info = virtual_declared_endpoint_info(&name, protocol);
    let user_supplied_info = virtual_user_supplied_info(&name, &description);
    let manufacturer_name = HSTRING::from(manufacturer.clone().unwrap_or_default());
    let description_name = HSTRING::from(description);
    let endpoint_name = HSTRING::from(name.clone());
    let config = MidiVirtualDeviceCreationConfig::CreateInstance3(
        &endpoint_name,
        &description_name,
        &manufacturer_name,
        &declared_endpoint_info,
        MidiDeclaredDeviceIdentity::default(),
        &user_supplied_info,
    )
    .map_err(|error| {
        windows_midi_error(
            operation,
            "MidiVirtualDeviceCreationConfig::CreateInstance3",
            &error,
        )
    })?;
    config.SetCreateOnlyUmpEndpoints(true).map_err(|error| {
        windows_midi_error(
            operation,
            "MidiVirtualDeviceCreationConfig::SetCreateOnlyUmpEndpoints",
            &error,
        )
    })?;

    // virtual device
    let virtual_device =
        MidiVirtualDeviceManager::CreateVirtualDevice(&config).map_err(|error| {
            windows_midi_error(
                operation,
                "MidiVirtualDeviceManager::CreateVirtualDevice",
                &error,
            )
        })?;
    let device_endpoint_id = virtual_device
        .DeviceEndpointDeviceId()
        .map_err(|error| {
            windows_midi_error(
                operation,
                "MidiVirtualDevice::DeviceEndpointDeviceId",
                &error,
            )
        })?
        .to_string();
    if device_endpoint_id.is_empty() {
        return Err(core_platform::io_operation_error(
            operation,
            None,
            "MidiVirtualDevice returned one empty device endpoint id",
        ));
    }

    // descriptor
    let association_id = virtual_device.AssociationId().map_err(|error| {
        windows_midi_error(operation, "MidiVirtualDevice::AssociationId", &error)
    })?;
    let descriptor = super::descriptor::virtual_device_descriptor(
        direction,
        device_endpoint_id.clone(),
        association_id,
        name,
        manufacturer,
        model,
        version,
        protocol,
    );

    Ok((virtual_device, device_endpoint_id, descriptor))
}

/// Convert one outbound record into one UMP word vector.
fn output_words(
    record: &MidiOutputRecordValue,
    operation: &'static str,
) -> RuntimeResult<Vec<u32>> {
    if !record.data.len().is_multiple_of(4) {
        return Err(core_platform::invalid_argument(
            "records",
            format!("{operation}: UMP record payload must be a multiple of 4 bytes"),
        ));
    }

    let mut words = Vec::with_capacity(record.data.len() / 4);

    for chunk in record.data.chunks_exact(4) {
        words.push(u32::from_be_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]));
    }

    Ok(words)
}

/// Validate one Windows MIDI send result.
fn validate_send_result(
    operation: &'static str,
    send_result: MidiSendMessageResults,
) -> RuntimeResult<()> {
    let is_failed = MidiEndpointConnection::SendMessageFailed(send_result).unwrap_or(false);
    if !is_failed {
        return Ok(());
    }

    if send_result.contains(MidiSendMessageResults::BufferFull) {
        return Err(core_platform::io_would_block(
            operation,
            "windows midi send buffer is full",
        ));
    }

    if send_result.contains(MidiSendMessageResults::EndpointConnectionClosedOrInvalid) {
        return Err(core_platform::io_not_found(
            operation,
            "windows midi endpoint connection is closed or invalid",
        ));
    }

    if send_result.contains(MidiSendMessageResults::TimestampOutOfRange)
        || send_result.contains(MidiSendMessageResults::TransmissionWordCountExceeded)
        || send_result.contains(MidiSendMessageResults::InvalidMessageTypeForWordCount)
        || send_result.contains(MidiSendMessageResults::InvalidMessageOther)
        || send_result.contains(MidiSendMessageResults::DataIndexOutOfRange)
    {
        return Err(core_platform::invalid_argument(
            "records",
            format!(
                "windows midi rejected the outbound UMP payload: 0x{:08X}",
                send_result.0
            ),
        ));
    }

    Err(core_platform::io_operation_error(
        operation,
        None,
        format!("windows midi send failed: 0x{:08X}", send_result.0),
    ))
}

/// Map one Windows MIDI error into one runtime error.
pub(super) fn windows_midi_error(
    operation: &'static str,
    action: &str,
    error: &WinError,
) -> Box<RuntimeError> {
    core_platform::io_operation_error(operation, None, format!("{action}: {error}"))
}

/// Enumerate one full Windows MIDI endpoint snapshot.
fn enumerate_endpoints(
    direction: MidiPortDirection,
    operation: &'static str,
) -> RuntimeResult<BTreeMap<String, WindowsMidiEndpointInfo>> {
    let collection = MidiEndpointDeviceInformation::FindAll().map_err(|error| {
        windows_midi_error(operation, "MidiEndpointDeviceInformation::FindAll", &error)
    })?;

    let mut descriptors = BTreeMap::new();

    // descriptor rows
    for device in &collection {
        let descriptor = device_descriptor(direction, &device)
            .map_err(|error| windows_midi_error(operation, "device_descriptor", &error))?;
        descriptors.insert(
            descriptor.id.clone(),
            WindowsMidiEndpointInfo {
                backend_id: descriptor.backend_id.clone().unwrap_or_default(),
                descriptor,
            },
        );
    }

    Ok(descriptors)
}

/// Refresh both directions in the shared Windows MIDI topology cache.
fn refresh_topology_cache(
    topology: &Arc<Mutex<WindowsMidiTopologyState>>,
    operation: &'static str,
) -> RuntimeResult<()> {
    let inputs = enumerate_endpoints(MidiPortDirection::Input, operation)?;
    let outputs = enumerate_endpoints(MidiPortDirection::Output, operation)?;
    let mut topology = topology.lock();

    topology.inputs = inputs;
    topology.outputs = outputs;

    Ok(())
}

/// Build one watcher callback that refreshes topology and notifies subscriptions.
fn watcher_refresh_handler<T>(
    topology: Arc<Mutex<WindowsMidiTopologyState>>,
    registry: Arc<Mutex<WindowsMidiNativeEventRegistry>>,
    operation: &'static str,
) -> TypedEventHandler<MidiEndpointDeviceWatcher, T>
where
    T: windows::core::RuntimeType + 'static,
{
    TypedEventHandler::new(move |_watcher, _args| {
        if refresh_topology_cache(&topology, operation).is_err() {
            queue_backend_disconnected_events(&registry, MidiEventSource::Native, 0);
            return Ok(());
        }

        refresh_native_event_sessions(&topology, &registry, MidiEventSource::Native);

        Ok(())
    })
}

/// Build one watcher callback that only notifies subscriptions on watcher stop.
fn watcher_stopped_handler(
    registry: Arc<Mutex<WindowsMidiNativeEventRegistry>>,
) -> TypedEventHandler<MidiEndpointDeviceWatcher, IInspectable> {
    TypedEventHandler::new(move |_watcher, _args| {
        queue_backend_disconnected_events(&registry, MidiEventSource::Native, 0);
        Ok(())
    })
}

/// Create and start one Windows MIDI endpoint watcher.
fn create_watcher(
    topology: Arc<Mutex<WindowsMidiTopologyState>>,
    registry: Arc<Mutex<WindowsMidiNativeEventRegistry>>,
    operation: &'static str,
) -> RuntimeResult<WindowsMidiWatcherRegistration> {
    let watcher = MidiEndpointDeviceWatcher::Create().map_err(|error| {
        windows_midi_error(operation, "MidiEndpointDeviceWatcher::Create", &error)
    })?;

    // mutation callbacks
    let added_token = watcher_add_added(
        &watcher,
        &watcher_refresh_handler::<MidiEndpointDeviceInformationAddedEventArgs>(
            topology.clone(),
            registry.clone(),
            operation,
        ),
    )
    .map_err(|error| windows_midi_error(operation, "MidiEndpointDeviceWatcher::Added", &error))?;
    let updated_token = watcher_add_updated(
        &watcher,
        &watcher_refresh_handler::<MidiEndpointDeviceInformationUpdatedEventArgs>(
            topology.clone(),
            registry.clone(),
            operation,
        ),
    )
    .map_err(|error| windows_midi_error(operation, "MidiEndpointDeviceWatcher::Updated", &error))?;
    let removed_token = watcher_add_removed(
        &watcher,
        &watcher_refresh_handler::<MidiEndpointDeviceInformationRemovedEventArgs>(
            topology.clone(),
            registry.clone(),
            operation,
        ),
    )
    .map_err(|error| windows_midi_error(operation, "MidiEndpointDeviceWatcher::Removed", &error))?;
    let enumeration_completed_token = watcher_add_enumeration_completed(
        &watcher,
        &watcher_refresh_handler::<IInspectable>(topology.clone(), registry.clone(), operation),
    )
    .map_err(|error| {
        windows_midi_error(
            operation,
            "MidiEndpointDeviceWatcher::EnumerationCompleted",
            &error,
        )
    })?;
    let stopped_token =
        watcher_add_stopped(&watcher, &watcher_stopped_handler(registry)).map_err(|error| {
            windows_midi_error(operation, "MidiEndpointDeviceWatcher::Stopped", &error)
        })?;

    watcher.Start().map_err(|error| {
        windows_midi_error(operation, "MidiEndpointDeviceWatcher::Start", &error)
    })?;

    Ok(WindowsMidiWatcherRegistration {
        watcher,
        added_token,
        updated_token,
        removed_token,
        enumeration_completed_token,
        stopped_token,
    })
}

/// Build one host-owned Windows MIDI service state on the dedicated executor thread.
fn build_windows_midi_service_state(
    topology: Arc<Mutex<WindowsMidiTopologyState>>,
    native_event_registry: Arc<Mutex<WindowsMidiNativeEventRegistry>>,
    operation: &'static str,
) -> RuntimeResult<WindowsMidiServiceState> {
    let session_name = HSTRING::from("Destack MIDI");
    let session = MidiRepository::Create(&session_name)
        .map_err(|error| windows_midi_error(operation, "MidiRepository::Create", &error))?;

    // initial topology
    refresh_topology_cache(&topology, operation)?;

    // live watcher
    let watcher = create_watcher(topology.clone(), native_event_registry.clone(), operation)?;

    Ok(WindowsMidiServiceState {
        _session: session,
        _topology: topology,
        _native_event_registry: native_event_registry,
        next_input_session_id: 1,
        input_sessions: BTreeMap::new(),
        next_output_session_id: 1,
        output_sessions: BTreeMap::new(),
        _watcher: watcher,
    })
}

/// Return the shared Windows MIDI service.
pub(crate) fn windows_midi_service(
    operation: &'static str,
) -> RuntimeResult<Arc<WindowsMidiService>> {
    WindowsMidiService::global(|| {
        let topology = Arc::new(Mutex::new(WindowsMidiTopologyState::default()));
        let native_event_registry = Arc::new(Mutex::new(WindowsMidiNativeEventRegistry {
            next_registration_id: 1,
            sessions: BTreeMap::new(),
        }));
        let build_topology = topology.clone();
        let build_registry = native_event_registry.clone();
        let executor = spawn_service_thread(
            "destack-midi-windows-midi",
            WindowsMidiService::POLICY,
            move || build_windows_midi_service_state(build_topology, build_registry, operation),
        )?;

        Ok(WindowsMidiService {
            executor,
            topology,
            native_event_registry,
        })
    })
}

/// Check whether Windows MIDI Services can create one session on this host.
pub(crate) fn check_windows_midi_support(operation: &'static str) -> RuntimeResult<()> {
    let session_name = HSTRING::from("Destack MIDI Support Probe");
    let _session = MidiRepository::Create(&session_name)
        .map_err(|error| windows_midi_error(operation, "MidiRepository::Create", &error))?;

    Ok(())
}

/// Register one native event subscription.
pub(super) fn register_native_event_session(
    service: &Arc<WindowsMidiService>,
    session: &Arc<Mutex<WindowsMidiEventRepository>>,
) -> WindowsMidiEventDeliveryKind {
    let mut registry = service.native_event_registry.lock();
    let registration_id = registry.next_registration_id;
    registry.next_registration_id = registry.next_registration_id.saturating_add(1);
    registry
        .sessions
        .insert(registration_id, Arc::downgrade(session));

    WindowsMidiEventDeliveryKind::Native {
        registry: service.native_event_registry.clone(),
        registration_id,
    }
}

/// Remove one native event subscription from the shared registry.
pub(super) fn unregister_native_event_session(delivery_kind: &WindowsMidiEventDeliveryKind) {
    let WindowsMidiEventDeliveryKind::Native {
        registry,
        registration_id,
    } = delivery_kind
    else {
        return;
    };

    registry.lock().sessions.remove(registration_id);
}

/// Return one cached input endpoint descriptor snapshot.
pub(super) fn input_descriptors(service: &Arc<WindowsMidiService>) -> Vec<WindowsMidiEndpointInfo> {
    service.topology.lock().inputs.values().cloned().collect()
}

/// Return one cached output endpoint descriptor snapshot.
pub(super) fn output_descriptors(
    service: &Arc<WindowsMidiService>,
) -> Vec<WindowsMidiEndpointInfo> {
    service.topology.lock().outputs.values().cloned().collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::device::midi::core::MidiOutputRecordValue;
    use crate::platform::device::{MidiDataFormat, MidiProtocol, MidiRecordFraming};
    use crate::platform::diagnostic::PlatformErrorCode;

    /// Encode aligned UMP payload bytes into exact Windows MIDI word values.
    #[test]
    fn test_output_words_encodes_exact_ump_words() {
        let record = MidiOutputRecordValue {
            send_at_ns: None,
            data_format: MidiDataFormat::Ump,
            protocol: Some(MidiProtocol::Midi1),
            framing: MidiRecordFraming::Complete,
            data: vec![0x41, 0x10, 0x00, 0x00, 0x20, 0x90, 0x3C, 0x40].into(),
        };

        let words = output_words(&record, "test.output_words")
            .expect("aligned UMP payloads should encode into Windows MIDI words");

        assert_eq!(words, vec![0x4110_0000, 0x2090_3C40]);
    }

    /// Reject unaligned UMP payload bytes before they reach Windows MIDI Services.
    #[test]
    fn test_output_words_rejects_unaligned_payloads() {
        let record = MidiOutputRecordValue {
            send_at_ns: None,
            data_format: MidiDataFormat::Ump,
            protocol: Some(MidiProtocol::Midi1),
            framing: MidiRecordFraming::Complete,
            data: vec![0x41, 0x10, 0x00].into(),
        };

        let error = output_words(&record, "test.output_words")
            .expect_err("unaligned UMP payloads should be rejected");

        assert_eq!(
            error.platform_error().map(|error| error.code),
            Some(PlatformErrorCode::InvalidArgument),
        );
    }

    /// Map Windows MIDI send-result flags onto the shared runtime error contract.
    #[test]
    fn test_validate_send_result_maps_known_windows_midi_failures() {
        validate_send_result(
            "test.validate_send_result",
            MidiSendMessageResults::Succeeded,
        )
        .expect("successful Windows MIDI sends should succeed");

        let error = validate_send_result(
            "test.validate_send_result",
            MidiSendMessageResults::Failed | MidiSendMessageResults::BufferFull,
        )
        .expect_err("buffer-full Windows MIDI sends should fail loudly");
        assert_eq!(
            error.platform_error().map(|error| error.code),
            Some(PlatformErrorCode::IoWouldBlock),
        );

        let error = validate_send_result(
            "test.validate_send_result",
            MidiSendMessageResults::Failed
                | MidiSendMessageResults::EndpointConnectionClosedOrInvalid,
        )
        .expect_err("closed Windows MIDI endpoint connections should fail loudly");
        assert_eq!(
            error.platform_error().map(|error| error.code),
            Some(PlatformErrorCode::IoNotFound),
        );

        let error = validate_send_result(
            "test.validate_send_result",
            MidiSendMessageResults::Failed | MidiSendMessageResults::TimestampOutOfRange,
        )
        .expect_err("invalid Windows MIDI timestamps should fail loudly");
        assert_eq!(
            error.platform_error().map(|error| error.code),
            Some(PlatformErrorCode::InvalidArgument),
        );
    }
}
