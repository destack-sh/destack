use std::collections::BTreeMap;
use std::sync::{Arc, Weak};

use parking_lot::Mutex;
use windows::Foundation::TypedEventHandler;
use windows::core::{Error as WinError, HSTRING, IInspectable, Ref};

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::core::{
    self as core_platform, qpc_hundred_nanos_to_process_nanos, qpc_process_nanos_to_hundred_nanos,
};
use crate::platform::midi::core::{
    MidiInputRecordValue, MidiOutputRecordValue, MidiPortDescriptorValue, MidiRecordBytes,
    binding_timestamp_now,
};
use crate::platform::midi::{
    MidiDataFormat, MidiEventSource, MidiPortDirection, MidiProtocol, MidiRecordFraming,
};
use crate::runtime::core::queue::BoundedQueue;
use crate::runtime::process::service::affinity::{ServiceAffinity, ServiceThreadBootstrap};
use crate::runtime::process::service::executor::dedicated::DedicatedThreadExecutor;
use crate::runtime::process::service::{self};

use super::abi::{
    connection_add_message_received, watcher_add_added, watcher_add_enumeration_completed,
    watcher_add_removed, watcher_add_stopped, watcher_add_updated,
};
use super::core::{
    WindowsMidiEndpointInfo, WindowsMidiEventDeliveryKind, WindowsMidiEventSession,
    WindowsMidiTopologyState,
};
use super::descriptor::device_descriptor;
use super::event::{queue_backend_disconnected_events, refresh_native_event_sessions};
use super::sdk::{
    MidiEndpointConnection, MidiEndpointConnectionBasicSettings, MidiEndpointDeviceInformation,
    MidiEndpointDeviceInformationAddedEventArgs, MidiEndpointDeviceInformationRemovedEventArgs,
    MidiEndpointDeviceInformationUpdatedEventArgs, MidiEndpointDeviceWatcher,
    MidiMessageReceivedEventArgs, MidiMessageStruct, MidiSendMessageResults, MidiSession,
};

/// One process-global Windows MIDI runtime service.
pub(crate) struct WindowsMidiService {
    /// Dedicated Windows MIDI service-thread executor.
    executor: DedicatedThreadExecutor<WindowsMidiServiceState>,
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
    pub(super) sessions: BTreeMap<u64, Weak<Mutex<WindowsMidiEventSession>>>,
}

/// One host-owned Windows MIDI service state that lives on the executor thread.
struct WindowsMidiServiceState {
    /// Shared MIDI session.
    _session: MidiSession,
    /// Shared topology cache.
    _topology: Arc<Mutex<WindowsMidiTopologyState>>,
    /// Registered native event subscriptions.
    _native_event_registry: Arc<Mutex<WindowsMidiNativeEventRegistry>>,
    /// Next input host-session id.
    next_input_session_id: u64,
    /// Live input host sessions.
    input_sessions: BTreeMap<u64, WindowsMidiInputHostSession>,
    /// Next output host-session id.
    next_output_session_id: u64,
    /// Live output host sessions.
    output_sessions: BTreeMap<u64, WindowsMidiOutputHostSession>,
    /// Live watcher and its event registrations.
    _watcher: WindowsMidiWatcherRegistration,
}

/// One host-owned input session.
struct WindowsMidiInputHostSession {
    /// Shared MIDI session used for disconnection.
    session: MidiSession,
    /// Opened Windows MIDI connection.
    connection: MidiEndpointConnection,
    /// Message-received registration token.
    token: i64,
}

/// One host-owned output session.
struct WindowsMidiOutputHostSession {
    /// Shared MIDI session used for disconnection.
    session: MidiSession,
    /// Opened Windows MIDI connection.
    connection: MidiEndpointConnection,
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

impl Drop for WindowsMidiInputHostSession {
    /// Tear down one host-owned Windows MIDI input session.
    fn drop(&mut self) {
        let _ = self.connection.RemoveMessageReceived(self.token);
        disconnect_connection(&self.session, &self.connection);
    }
}

impl Drop for WindowsMidiOutputHostSession {
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
    /// The host-affinity domain for the Windows MIDI backend service.
    pub(crate) const AFFINITY: ServiceAffinity =
        ServiceAffinity::DedicatedThread(ServiceThreadBootstrap::WindowsMta);

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
                "destack.midi.input.port.open",
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
                            match decode_input_message(source_id.clone(), protocol, args) {
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
                WindowsMidiInputHostSession {
                    session: state._session.clone(),
                    connection,
                    token,
                },
            );

            Ok(host_session_id)
        })
    }

    /// Close one input host session during resource drop.
    pub(super) fn close_input_session_for_drop(&self, host_session_id: u64) {
        let _ = self
            .executor
            .call("destack.midi.windows-midi.input.drop", move |state| {
                state.input_sessions.remove(&host_session_id);
                Ok(())
            });
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
                "destack.midi.output.port.open",
            )?;

            // publish one new host session id
            let host_session_id = state.next_output_session_id;
            state.next_output_session_id = state.next_output_session_id.saturating_add(1);
            state.output_sessions.insert(
                host_session_id,
                WindowsMidiOutputHostSession {
                    session: state._session.clone(),
                    connection,
                },
            );

            Ok(host_session_id)
        })
    }

    /// Close one output host session during resource drop.
    pub(super) fn close_output_session_for_drop(&self, host_session_id: u64) {
        let _ = self
            .executor
            .call("destack.midi.windows-midi.output.drop", move |state| {
                state.output_sessions.remove(&host_session_id);
                Ok(())
            });
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
    session: &MidiSession,
    backend_id: &str,
    operation: &'static str,
) -> RuntimeResult<MidiEndpointConnection> {
    let settings = connection_settings(operation)?;
    let backend_id = HSTRING::from(backend_id);
    let connection = session
        .CreateEndpointConnection2(&backend_id, &settings)
        .map_err(|error| {
            windows_midi_error(operation, "MidiSession::CreateEndpointConnection2", &error)
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
fn disconnect_connection(session: &MidiSession, connection: &MidiEndpointConnection) {
    let Ok(connection_id) = connection.ConnectionId() else {
        return;
    };

    let _ = session.DisconnectEndpointConnection(connection_id);
}

/// Decode one Windows MIDI input message into one inbound record.
fn decode_input_message(
    source_id: Arc<str>,
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
        source_id: Some(source_id),
        data_format: MidiDataFormat::Ump,
        protocol,
        framing: MidiRecordFraming::Complete,
        data,
    })
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
    let session = MidiSession::Create(&session_name)
        .map_err(|error| windows_midi_error(operation, "MidiSession::Create", &error))?;

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
    service::global_service(|| {
        let topology = Arc::new(Mutex::new(WindowsMidiTopologyState::default()));
        let native_event_registry = Arc::new(Mutex::new(WindowsMidiNativeEventRegistry {
            next_registration_id: 1,
            sessions: BTreeMap::new(),
        }));
        let build_topology = topology.clone();
        let build_registry = native_event_registry.clone();
        let ServiceAffinity::DedicatedThread(thread_bootstrap) = WindowsMidiService::AFFINITY
        else {
            return Err(core_platform::io_operation_error(
                operation,
                None,
                "windows midi service requires one dedicated thread affinity domain",
            ));
        };
        let executor = DedicatedThreadExecutor::spawn(
            "destack-midi-windows-midi",
            thread_bootstrap,
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
    let _session = MidiSession::Create(&session_name)
        .map_err(|error| windows_midi_error(operation, "MidiSession::Create", &error))?;

    Ok(())
}

/// Register one native event subscription.
pub(super) fn register_native_event_session(
    service: &Arc<WindowsMidiService>,
    session: &Arc<Mutex<WindowsMidiEventSession>>,
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
    use crate::platform::diagnostic::PlatformErrorCode;
    use crate::platform::midi::core::MidiOutputRecordValue;
    use crate::platform::midi::{MidiDataFormat, MidiProtocol, MidiRecordFraming};

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
