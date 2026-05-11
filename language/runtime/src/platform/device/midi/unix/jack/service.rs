use std::collections::BTreeMap;
use std::sync::{Arc, Condvar, Mutex as StdMutex};
use std::thread::{self, JoinHandle};

use parking_lot::Mutex;

use crate::diagnostic::RuntimeResult;
use crate::platform::core::{self as core_platform};
use crate::platform::device::MidiEventSource;
use crate::runtime::service::Service;
use crate::runtime::{ExecutionMode, ExecutionPolicy, start_with_policy};

use super::core::{
    JackClientHandle, JackEndpointInfo, JackTopologyState, activate_client, jack_error,
    monitor_client_name, open_jack_client, query_topology_snapshot, require_jack_library,
};
use super::event::{queue_backend_disconnected_events, refresh_native_event_sessions};

/// One JACK monitor wake state.
#[derive(Debug, Default)]
struct JackMonitorSignalState {
    /// Whether one topology refresh is pending.
    is_pending_refresh: bool,
    /// Whether the backend disconnected.
    is_disconnected: bool,
    /// Whether the monitor should stop.
    is_stopped: bool,
}

/// One JACK monitor wake handle shared with callbacks.
#[derive(Debug, Default)]
struct JackMonitorSignal {
    /// Shared wake state.
    state: StdMutex<JackMonitorSignalState>,
    /// Wake channel for callback driven refresh.
    wake: Condvar,
}

impl JackMonitorSignal {
    /// Request one topology refresh.
    fn request_refresh(&self) {
        let mut state = self.state.lock().unwrap_or_else(|error| error.into_inner());
        state.is_pending_refresh = true;
        self.wake.notify_one();
    }

    /// Request one disconnect wake.
    fn request_disconnect(&self) {
        let mut state = self.state.lock().unwrap_or_else(|error| error.into_inner());
        state.is_disconnected = true;
        self.wake.notify_one();
    }

    /// Request one monitor shutdown.
    fn request_stop(&self) {
        let mut state = self.state.lock().unwrap_or_else(|error| error.into_inner());
        state.is_stopped = true;
        self.wake.notify_one();
    }
}

/// One registry of native JACK event subscriptions.
pub(super) struct JackNativeEventRegistry {
    /// Next registration id.
    pub(super) next_registration_id: u64,
    /// Registered native event subscriptions.
    pub(super) sessions: BTreeMap<u64, std::sync::Weak<Mutex<super::core::JackEventRepository>>>,
}

/// One process-global JACK MIDI service.
pub(crate) struct JackService {
    /// Shared JACK dynamic library owner.
    pub(super) library: Arc<super::core::JackLibrary>,
    /// Shared topology cache.
    pub(super) topology: Arc<Mutex<JackTopologyState>>,
    /// Registered native event subscriptions.
    pub(super) native_event_registry: Arc<Mutex<JackNativeEventRegistry>>,
    /// The monitor client that receives graph callbacks.
    monitor_client: Mutex<Option<JackClientHandle>>,
    /// Live monitor thread.
    monitor_thread: Mutex<Option<JoinHandle<()>>>,
    /// Wake handle shared with callbacks and the monitor thread.
    monitor_signal: Arc<JackMonitorSignal>,
}

impl Drop for JackService {
    /// Stop the monitor thread when the process-global service tears down.
    fn drop(&mut self) {
        self.monitor_signal.request_stop();

        let monitor_thread = self.monitor_thread.get_mut().take();
        if let Some(monitor_thread) = monitor_thread {
            let _ = monitor_thread.join();
        }

        self.monitor_client.get_mut().take();
    }
}

impl Service for JackService {
    const POLICY: ExecutionPolicy = ExecutionPolicy::process(ExecutionMode::Inline);
}

/// Return the shared JACK MIDI service.
pub(crate) fn jack_service(operation: &'static str) -> RuntimeResult<Arc<JackService>> {
    JackService::global(|| {
        let library = require_jack_library(operation)?;
        let topology = Arc::new(Mutex::new(query_topology_snapshot(operation)?));
        let native_event_registry = Arc::new(Mutex::new(JackNativeEventRegistry {
            next_registration_id: 1,
            sessions: BTreeMap::new(),
        }));
        let monitor_signal = Arc::new(JackMonitorSignal::default());

        let monitor_client = open_jack_client(library.clone(), operation, &monitor_client_name())?;
        install_monitor_callbacks(&monitor_client, &monitor_signal)?;
        activate_client(&monitor_client, operation)?;

        let monitor_thread = spawn_monitor_thread(
            topology.clone(),
            native_event_registry.clone(),
            monitor_signal.clone(),
        )?;

        Ok(JackService {
            library,
            topology,
            native_event_registry,
            monitor_client: Mutex::new(Some(monitor_client)),
            monitor_thread: Mutex::new(Some(monitor_thread)),
            monitor_signal,
        })
    })
}

/// Return one current cloned input descriptor snapshot.
pub(super) fn input_descriptors(service: &Arc<JackService>) -> Vec<JackEndpointInfo> {
    let topology = service.topology.lock();

    topology.inputs.values().cloned().collect()
}

/// Return one current cloned output descriptor snapshot.
pub(super) fn output_descriptors(service: &Arc<JackService>) -> Vec<JackEndpointInfo> {
    let topology = service.topology.lock();

    topology.outputs.values().cloned().collect()
}

/// Register one native event subscription.
pub(super) fn register_native_event_session(
    service: &Arc<JackService>,
    session: &Arc<Mutex<super::core::JackEventRepository>>,
) -> super::core::JackEventDeliveryKind {
    let mut registry = service.native_event_registry.lock();
    let registration_id = registry.next_registration_id;
    registry.next_registration_id = registry.next_registration_id.saturating_add(1);
    registry
        .sessions
        .insert(registration_id, Arc::downgrade(session));

    super::core::JackEventDeliveryKind::Native {
        registry: service.native_event_registry.clone(),
        registration_id,
    }
}

/// Remove one native event subscription from the shared registry.
pub(super) fn unregister_native_event_session(delivery_kind: &super::core::JackEventDeliveryKind) {
    let super::core::JackEventDeliveryKind::Native {
        registry,
        registration_id,
    } = delivery_kind
    else {
        return;
    };

    registry.lock().sessions.remove(registration_id);
}

/// Refresh one native event subscription set after one local virtual mutation.
pub(super) fn refresh_local_native_event_sessions(service: &Arc<JackService>) {
    let Ok(topology) = query_topology_snapshot("destack.device.midi.jack.refresh") else {
        return;
    };

    {
        let mut service_topology = service.topology.lock();
        *service_topology = topology;
    }

    refresh_native_event_sessions(service, MidiEventSource::Native);
}

/// Install one set of monitor callbacks on the shared client.
fn install_monitor_callbacks(
    client: &JackClientHandle,
    monitor_signal: &Arc<JackMonitorSignal>,
) -> RuntimeResult<()> {
    let callback_argument = Arc::as_ptr(monitor_signal) as *mut std::ffi::c_void;

    let registration_status = unsafe {
        (client.library.api.jack_set_port_registration_callback)(
            client.raw,
            Some(port_registration_callback),
            callback_argument,
        )
    };
    if registration_status != 0 {
        return Err(jack_error(
            "destack.device.midi.event.open",
            format!(
                "failed to install JACK port-registration callback (status {registration_status})"
            ),
        ));
    }

    let connect_status = unsafe {
        (client.library.api.jack_set_port_connect_callback)(
            client.raw,
            Some(port_connect_callback),
            callback_argument,
        )
    };
    if connect_status != 0 {
        return Err(jack_error(
            "destack.device.midi.event.open",
            format!("failed to install JACK port-connect callback (status {connect_status})"),
        ));
    }

    unsafe {
        (client.library.api.jack_on_shutdown)(
            client.raw,
            Some(shutdown_callback),
            callback_argument,
        );
    }

    Ok(())
}

/// Spawn one polling monitor thread.
fn spawn_monitor_thread(
    topology: Arc<Mutex<JackTopologyState>>,
    native_event_registry: Arc<Mutex<JackNativeEventRegistry>>,
    monitor_signal: Arc<JackMonitorSignal>,
) -> RuntimeResult<JoinHandle<()>> {
    start_with_policy(
        "destack-midi-jack-monitor",
        "destack.device.midi.jack.monitor.spawn",
        ExecutionPolicy::process(ExecutionMode::Loop),
        move || {
            loop {
                // wait for the next callback or teardown request
                let mut signal_state = monitor_signal
                    .state
                    .lock()
                    .unwrap_or_else(|error| error.into_inner());
                while !signal_state.is_stopped
                    && !signal_state.is_disconnected
                    && !signal_state.is_pending_refresh
                {
                    signal_state = monitor_signal
                        .wake
                        .wait(signal_state)
                        .unwrap_or_else(|error| error.into_inner());
                }

                // stop request
                if signal_state.is_stopped {
                    return;
                }

                // disconnect
                if signal_state.is_disconnected {
                    signal_state.is_disconnected = false;
                    drop(signal_state);

                    queue_backend_disconnected_events(
                        &native_event_registry,
                        MidiEventSource::Native,
                        0,
                    );
                    return;
                }

                // topology refresh
                signal_state.is_pending_refresh = false;
                drop(signal_state);

                let next_topology =
                    match query_topology_snapshot("destack.device.midi.event.refresh") {
                        Ok(next_topology) => next_topology,
                        Err(_) => {
                            queue_backend_disconnected_events(
                                &native_event_registry,
                                MidiEventSource::Native,
                                0,
                            );
                            return;
                        }
                    };

                {
                    let mut service_topology = topology.lock();
                    *service_topology = next_topology;
                }

                let service = match jack_service("destack.device.midi.event.refresh") {
                    Ok(service) => service,
                    Err(_) => return,
                };
                refresh_native_event_sessions(&service, MidiEventSource::Native);
            }
        },
    )
}

/// Handle one JACK port-registration callback.
unsafe extern "C" fn port_registration_callback(
    _port_id: u32,
    _is_registered: libc::c_int,
    argument: *mut std::ffi::c_void,
) {
    let monitor_signal = argument.cast::<JackMonitorSignal>();
    if monitor_signal.is_null() {
        return;
    }

    unsafe {
        (*monitor_signal).request_refresh();
    }
}

/// Handle one JACK port-connect callback.
unsafe extern "C" fn port_connect_callback(
    _source_port_id: u32,
    _destination_port_id: u32,
    _is_connected: libc::c_int,
    argument: *mut std::ffi::c_void,
) {
    let monitor_signal = argument.cast::<JackMonitorSignal>();
    if monitor_signal.is_null() {
        return;
    }

    unsafe {
        (*monitor_signal).request_refresh();
    }
}

/// Handle one JACK backend shutdown callback.
unsafe extern "C" fn shutdown_callback(argument: *mut std::ffi::c_void) {
    let monitor_signal = argument.cast::<JackMonitorSignal>();
    if monitor_signal.is_null() {
        return;
    }

    unsafe {
        (*monitor_signal).request_disconnect();
    }
}
