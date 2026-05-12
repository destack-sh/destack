use std::collections::BTreeMap;
use std::sync::{Arc, Weak};

use parking_lot::Mutex;
use windows::Devices::Enumeration::{
    DeviceInformation, DeviceInformationUpdate, DeviceWatcher, DeviceWatcherStatus,
};
use windows::Devices::Midi::{IMidiOutPort, MidiInPort, MidiMessageReceivedEventArgs, MidiOutPort};
use windows::Foundation::TypedEventHandler;
use windows::Storage::Streams::DataWriter;
use windows::core::{Error as WinError, HSTRING, Ref};

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::core::{self as core_platform, BoundedQueue};
use crate::platform::device::midi::core::{
    MidiInputRecordValue, MidiOutputRecordValue, MidiPortDescriptorValue, MidiRecordBytes,
    binding_timestamp_now,
};
use crate::platform::device::{
    MidiDataFormat, MidiEventSource, MidiPortDirection, MidiProtocol, MidiRecordFraming,
};
use crate::runtime::service::executor::thread::ServiceThreadExecutor;
use crate::runtime::service::{Service, spawn_service_thread};
use crate::runtime::{ExecutionAffinity, ExecutionMode, ExecutionPolicy};

use super::core::{
    WinRtEndpointInfo, WinRtEventDeliveryKind, WinRtEventRepository, WinRtTopologyState,
    winrt_relative_timestamp_to_mono_ns,
};
use super::descriptor::device_descriptor;
use super::event::{queue_backend_disconnected_events, refresh_native_event_sessions};

/// One process-global WinRT runtime service.
pub(crate) struct WinRtService {
    /// Dedicated WinRT service-thread executor.
    executor: ServiceThreadExecutor<WinRtServiceState>,
    /// Shared topology cache.
    pub(super) topology: Arc<Mutex<WinRtTopologyState>>,
    /// Registered native event subscriptions.
    pub(super) native_event_registry: Arc<Mutex<WinRtNativeEventRegistry>>,
}

/// One registry of native event subscriptions.
pub(super) struct WinRtNativeEventRegistry {
    /// Next registration id.
    pub(super) next_registration_id: u64,
    /// Registered native event subscriptions.
    pub(super) sessions: BTreeMap<u64, Weak<Mutex<WinRtEventRepository>>>,
}

/// One host-owned WinRT service state that lives on the executor thread.
struct WinRtServiceState {
    /// Shared topology cache.
    _topology: Arc<Mutex<WinRtTopologyState>>,
    /// Registered native event subscriptions.
    _native_event_registry: Arc<Mutex<WinRtNativeEventRegistry>>,
    /// Next input host-session id.
    next_input_session_id: u64,
    /// Live input host sessions.
    input_sessions: BTreeMap<u64, WinRtInputHostRepository>,
    /// Next output host-session id.
    next_output_session_id: u64,
    /// Live output host sessions.
    output_sessions: BTreeMap<u64, WinRtOutputHostRepository>,
    /// Live input watcher and its event registrations.
    _input_watcher: WinRtWatcherRegistration,
    /// Live output watcher and its event registrations.
    _output_watcher: WinRtWatcherRegistration,
}

/// One host-owned input session.
struct WinRtInputHostRepository {
    /// Opened WinRT input port.
    port: MidiInPort,
    /// Message-received registration token.
    token: i64,
}

/// One host-owned output session.
struct WinRtOutputHostRepository {
    /// Opened WinRT output port.
    port: IMidiOutPort,
}

/// One watcher registration bundle.
struct WinRtWatcherRegistration {
    /// Live watcher object.
    watcher: DeviceWatcher,
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

impl Drop for WinRtInputHostRepository {
    /// Tear down one host-owned WinRT input session.
    fn drop(&mut self) {
        let _ = self.port.RemoveMessageReceived(self.token);
        let _ = self.port.Close();
    }
}

impl Drop for WinRtOutputHostRepository {
    /// Tear down one host-owned WinRT output session.
    fn drop(&mut self) {
        let _ = self.port.Close();
    }
}

impl Drop for WinRtWatcherRegistration {
    /// Tear down one WinRT device watcher.
    fn drop(&mut self) {
        let _ = self.watcher.RemoveAdded(self.added_token);
        let _ = self.watcher.RemoveUpdated(self.updated_token);
        let _ = self.watcher.RemoveRemoved(self.removed_token);
        let _ = self
            .watcher
            .RemoveEnumerationCompleted(self.enumeration_completed_token);
        let _ = self.watcher.RemoveStopped(self.stopped_token);

        // stop one live watcher before the service thread exits
        if let Ok(status) = self.watcher.Status()
            && status == DeviceWatcherStatus::Started
        {
            let _ = self.watcher.Stop();
        }
    }
}

impl WinRtService {
    /// Open one input host session on the dedicated service thread.
    pub(super) fn open_input_session(
        &self,
        backend_id: String,
        descriptor: MidiPortDescriptorValue,
        queue: Arc<BoundedQueue<MidiInputRecordValue>>,
        terminal_error: Arc<Mutex<Option<String>>>,
        operation: &'static str,
    ) -> RuntimeResult<u64> {
        self.executor.call(operation, move |state| {
            // anchor the relative timestamp domain around the actual host open
            let before_open_ns = binding_timestamp_now();
            let port = MidiInPort::FromIdAsync(&HSTRING::from(backend_id.as_str()))
                .map_err(|error| winrt_error(operation, "MidiInPort::FromIdAsync", &error))?
                .get()
                .map_err(|error| winrt_error(operation, "IAsyncOperation::get", &error))?;
            let after_open_ns = binding_timestamp_now();
            let open_epoch_ns =
                before_open_ns.saturating_add((after_open_ns.saturating_sub(before_open_ns)) / 2);

            // input callback
            let callback_queue = queue.clone();
            let callback_terminal_error = terminal_error.clone();
            let source_id = Arc::<str>::from(descriptor.id.clone());
            let token = port
                .MessageReceived(&TypedEventHandler::new(
                    move |_port: Ref<'_, MidiInPort>,
                          args: Ref<'_, MidiMessageReceivedEventArgs>| {
                        if let Some(args) = args.as_ref()
                        {
                            match decode_input_message(open_epoch_ns, source_id.clone(), args) {
                                Ok(record) => callback_queue.push_drop_oldest(record),
                                Err(error) => {
                                    let mut terminal_error = callback_terminal_error.lock();
                                    if terminal_error.is_none() {
                                        *terminal_error = Some(format!(
                                            "winrt midi input session failed to decode one inbound message: {error}",
                                        ));
                                    }
                                    callback_queue.close();
                                }
                            }
                        }

                        Ok(())
                    },
                ))
                .map_err(|error| winrt_error(operation, "MidiInPort::MessageReceived", &error))?;

            // publish one new host session id
            let host_session_id = state.next_input_session_id;
            state.next_input_session_id = state.next_input_session_id.saturating_add(1);
            state
                .input_sessions
                .insert(host_session_id, WinRtInputHostRepository { port, token });

            Ok(host_session_id)
        })
    }

    /// Close one input host session during resource drop.
    pub(super) fn close_input_session_for_drop(&self, host_session_id: u64) {
        let _ = self
            .executor
            .call("destack.device.midi.winrt.input.drop", move |state| {
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
            let port = MidiOutPort::FromIdAsync(&HSTRING::from(backend_id.as_str()))
                .map_err(|error| winrt_error(operation, "MidiOutPort::FromIdAsync", &error))?
                .get()
                .map_err(|error| winrt_error(operation, "IAsyncOperation::get", &error))?;

            let host_session_id = state.next_output_session_id;
            state.next_output_session_id = state.next_output_session_id.saturating_add(1);
            state
                .output_sessions
                .insert(host_session_id, WinRtOutputHostRepository { port });

            Ok(host_session_id)
        })
    }

    /// Close one output host session during resource drop.
    pub(super) fn close_output_session_for_drop(&self, host_session_id: u64) {
        let _ = self
            .executor
            .call("destack.device.midi.winrt.output.drop", move |state| {
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

            for record in &records {
                let buffer = output_buffer(record, operation)?;
                session
                    .port
                    .SendBuffer(&buffer)
                    .map_err(|error| winrt_error(operation, "MidiOutPort::SendBuffer", &error))?;
            }

            Ok(records.len() as u32)
        })
    }
}

impl Service for WinRtService {
    const POLICY: ExecutionPolicy = ExecutionPolicy::process(ExecutionMode::Thread)
        .with_affinity(ExecutionAffinity::WindowsMta);
}

/// Map one WinRT error into one runtime error.
pub(super) fn winrt_error(
    operation: &'static str,
    action: &str,
    error: &WinError,
) -> Box<RuntimeError> {
    core_platform::io_operation_error(operation, None, format!("{action}: {error}"))
}

/// Decode one WinRT input message into one inbound record.
fn decode_input_message(
    open_epoch_ns: u64,
    source_id: Arc<str>,
    args: &MidiMessageReceivedEventArgs,
) -> windows::core::Result<MidiInputRecordValue> {
    let message = args.Message()?;
    let buffer = message.RawData()?;
    let reader = windows::Storage::Streams::DataReader::FromBuffer(&buffer)?;
    let mut data = MidiRecordBytes::from_elem(0u8, buffer.Length()? as usize);
    reader.ReadBytes(&mut data)?;

    Ok(MidiInputRecordValue {
        received_at_ns: winrt_relative_timestamp_to_mono_ns(
            open_epoch_ns,
            message.Timestamp()?.Duration,
        ),
        source_id: Some(source_id),
        data_format: MidiDataFormat::Midi1Bytes,
        protocol: Some(MidiProtocol::Midi1),
        framing: MidiRecordFraming::Complete,
        data,
    })
}

/// Encode one outbound record into one WinRT byte buffer.
fn output_buffer(
    record: &MidiOutputRecordValue,
    operation: &'static str,
) -> RuntimeResult<windows::Storage::Streams::IBuffer> {
    let writer =
        DataWriter::new().map_err(|error| winrt_error(operation, "DataWriter::new", &error))?;

    writer
        .WriteBytes(&record.data)
        .map_err(|error| winrt_error(operation, "DataWriter::WriteBytes", &error))?;

    writer
        .DetachBuffer()
        .map_err(|error| winrt_error(operation, "DataWriter::DetachBuffer", &error))
}

/// Enumerate one direction of WinRT endpoints.
fn enumerate_direction(
    direction: MidiPortDirection,
    selector: &HSTRING,
    operation: &'static str,
) -> RuntimeResult<BTreeMap<String, WinRtEndpointInfo>> {
    let collection = DeviceInformation::FindAllAsyncAqsFilter(selector)
        .map_err(|error| {
            winrt_error(
                operation,
                "DeviceInformation::FindAllAsyncAqsFilter",
                &error,
            )
        })?
        .get()
        .map_err(|error| winrt_error(operation, "IAsyncOperation::get", &error))?;

    let mut descriptors = BTreeMap::new();

    // descriptor rows
    for device in &collection {
        let descriptor = device_descriptor(direction, &device)
            .map_err(|error| winrt_error(operation, "device_descriptor", &error))?;
        descriptors.insert(
            descriptor.id.clone(),
            WinRtEndpointInfo {
                backend_id: descriptor.backend_id.clone().unwrap_or_default(),
                descriptor,
            },
        );
    }

    Ok(descriptors)
}

/// Refresh one direction in the shared WinRT topology cache.
fn refresh_direction_cache(
    topology: &Arc<Mutex<WinRtTopologyState>>,
    direction: MidiPortDirection,
    selector: &HSTRING,
    operation: &'static str,
) -> RuntimeResult<()> {
    let descriptors = enumerate_direction(direction, selector, operation)?;
    let mut topology = topology.lock();

    match direction {
        MidiPortDirection::Input => topology.inputs = descriptors,
        MidiPortDirection::Output => topology.outputs = descriptors,
    }

    Ok(())
}

/// Refresh both directions in the shared WinRT topology cache.
fn refresh_topology_cache(
    topology: &Arc<Mutex<WinRtTopologyState>>,
    input_selector: &HSTRING,
    output_selector: &HSTRING,
    operation: &'static str,
) -> RuntimeResult<()> {
    // input rows
    refresh_direction_cache(
        topology,
        MidiPortDirection::Input,
        input_selector,
        operation,
    )?;

    // output rows
    refresh_direction_cache(
        topology,
        MidiPortDirection::Output,
        output_selector,
        operation,
    )?;

    Ok(())
}

/// Build one watcher callback that refreshes one direction and notifies subscriptions.
fn watcher_refresh_handler<T>(
    topology: Arc<Mutex<WinRtTopologyState>>,
    registry: Arc<Mutex<WinRtNativeEventRegistry>>,
    direction: MidiPortDirection,
    selector: HSTRING,
    operation: &'static str,
) -> TypedEventHandler<DeviceWatcher, T>
where
    T: windows::core::RuntimeType + 'static,
{
    TypedEventHandler::new(move |_watcher, _args| {
        if refresh_direction_cache(&topology, direction, &selector, operation).is_err() {
            queue_backend_disconnected_events(&registry, MidiEventSource::Native, 0);
            return Ok(());
        }

        refresh_native_event_sessions(&topology, &registry, MidiEventSource::Native);
        Ok(())
    })
}

/// Build one watcher callback that only notifies subscriptions on watcher stop.
fn watcher_stopped_handler(
    registry: Arc<Mutex<WinRtNativeEventRegistry>>,
) -> TypedEventHandler<DeviceWatcher, windows::core::IInspectable> {
    TypedEventHandler::new(move |_watcher, _args| {
        queue_backend_disconnected_events(&registry, MidiEventSource::Native, 0);
        Ok(())
    })
}

/// Create and start one direction-specific device watcher.
fn create_watcher(
    direction: MidiPortDirection,
    selector: &HSTRING,
    topology: Arc<Mutex<WinRtTopologyState>>,
    registry: Arc<Mutex<WinRtNativeEventRegistry>>,
    operation: &'static str,
) -> RuntimeResult<WinRtWatcherRegistration> {
    let watcher = DeviceInformation::CreateWatcherAqsFilter(selector).map_err(|error| {
        winrt_error(
            operation,
            "DeviceInformation::CreateWatcherAqsFilter",
            &error,
        )
    })?;

    // mutation callbacks
    let added_token = watcher
        .Added(&watcher_refresh_handler::<DeviceInformation>(
            topology.clone(),
            registry.clone(),
            direction,
            selector.clone(),
            operation,
        ))
        .map_err(|error| winrt_error(operation, "DeviceWatcher::Added", &error))?;
    let updated_token = watcher
        .Updated(&watcher_refresh_handler::<DeviceInformationUpdate>(
            topology.clone(),
            registry.clone(),
            direction,
            selector.clone(),
            operation,
        ))
        .map_err(|error| winrt_error(operation, "DeviceWatcher::Updated", &error))?;
    let removed_token = watcher
        .Removed(&watcher_refresh_handler::<DeviceInformationUpdate>(
            topology.clone(),
            registry.clone(),
            direction,
            selector.clone(),
            operation,
        ))
        .map_err(|error| winrt_error(operation, "DeviceWatcher::Removed", &error))?;
    let enumeration_completed_token = watcher
        .EnumerationCompleted(&watcher_refresh_handler::<windows::core::IInspectable>(
            topology.clone(),
            registry.clone(),
            direction,
            selector.clone(),
            operation,
        ))
        .map_err(|error| winrt_error(operation, "DeviceWatcher::EnumerationCompleted", &error))?;
    let stopped_token = watcher
        .Stopped(&watcher_stopped_handler(registry))
        .map_err(|error| winrt_error(operation, "DeviceWatcher::Stopped", &error))?;

    watcher
        .Start()
        .map_err(|error| winrt_error(operation, "DeviceWatcher::Start", &error))?;

    Ok(WinRtWatcherRegistration {
        watcher,
        added_token,
        updated_token,
        removed_token,
        enumeration_completed_token,
        stopped_token,
    })
}

/// Build one host-owned WinRT service state on the dedicated executor thread.
fn build_winrt_service_state(
    topology: Arc<Mutex<WinRtTopologyState>>,
    native_event_registry: Arc<Mutex<WinRtNativeEventRegistry>>,
    operation: &'static str,
) -> RuntimeResult<WinRtServiceState> {
    let input_selector = MidiInPort::GetDeviceSelector()
        .map_err(|error| winrt_error(operation, "MidiInPort::GetDeviceSelector", &error))?;
    let output_selector = MidiOutPort::GetDeviceSelector()
        .map_err(|error| winrt_error(operation, "MidiOutPort::GetDeviceSelector", &error))?;

    // initial topology
    refresh_topology_cache(&topology, &input_selector, &output_selector, operation)?;

    // live watchers
    let input_watcher = create_watcher(
        MidiPortDirection::Input,
        &input_selector,
        topology.clone(),
        native_event_registry.clone(),
        operation,
    )?;
    let output_watcher = create_watcher(
        MidiPortDirection::Output,
        &output_selector,
        topology.clone(),
        native_event_registry.clone(),
        operation,
    )?;

    Ok(WinRtServiceState {
        _topology: topology,
        _native_event_registry: native_event_registry,
        next_input_session_id: 1,
        input_sessions: BTreeMap::new(),
        next_output_session_id: 1,
        output_sessions: BTreeMap::new(),
        _input_watcher: input_watcher,
        _output_watcher: output_watcher,
    })
}

/// Return the shared WinRT service.
pub(crate) fn winrt_service(operation: &'static str) -> RuntimeResult<Arc<WinRtService>> {
    WinRtService::global(|| {
        let topology = Arc::new(Mutex::new(WinRtTopologyState::default()));
        let native_event_registry = Arc::new(Mutex::new(WinRtNativeEventRegistry {
            next_registration_id: 1,
            sessions: BTreeMap::new(),
        }));
        let build_topology = topology.clone();
        let build_registry = native_event_registry.clone();
        let executor =
            spawn_service_thread("destack-midi-winrt", WinRtService::POLICY, move || {
                build_winrt_service_state(build_topology, build_registry, operation)
            })?;

        Ok(WinRtService {
            executor,
            topology,
            native_event_registry,
        })
    })
}

/// Check whether the WinRT MIDI API surface is reachable on this host.
pub(crate) fn check_winrt_support(operation: &'static str) -> RuntimeResult<()> {
    let _input_selector = MidiInPort::GetDeviceSelector()
        .map_err(|error| winrt_error(operation, "MidiInPort::GetDeviceSelector", &error))?;
    let _output_selector = MidiOutPort::GetDeviceSelector()
        .map_err(|error| winrt_error(operation, "MidiOutPort::GetDeviceSelector", &error))?;

    Ok(())
}

/// Register one native event subscription.
pub(super) fn register_native_event_session(
    service: &Arc<WinRtService>,
    session: &Arc<Mutex<WinRtEventRepository>>,
) -> WinRtEventDeliveryKind {
    let mut registry = service.native_event_registry.lock();
    let registration_id = registry.next_registration_id;
    registry.next_registration_id = registry.next_registration_id.saturating_add(1);
    registry
        .sessions
        .insert(registration_id, Arc::downgrade(session));

    WinRtEventDeliveryKind::Native {
        registry: service.native_event_registry.clone(),
        registration_id,
    }
}

/// Remove one native event subscription from the shared registry.
pub(super) fn unregister_native_event_session(delivery_kind: &WinRtEventDeliveryKind) {
    let WinRtEventDeliveryKind::Native {
        registry,
        registration_id,
    } = delivery_kind
    else {
        return;
    };

    registry.lock().sessions.remove(registration_id);
}

/// Return one cached input endpoint descriptor snapshot.
pub(super) fn input_descriptors(service: &Arc<WinRtService>) -> Vec<WinRtEndpointInfo> {
    service.topology.lock().inputs.values().cloned().collect()
}

/// Return one cached output endpoint descriptor snapshot.
pub(super) fn output_descriptors(service: &Arc<WinRtService>) -> Vec<WinRtEndpointInfo> {
    service.topology.lock().outputs.values().cloned().collect()
}
