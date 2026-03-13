use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use parking_lot::Mutex;

use crate::diagnostic::RuntimeResult;
use crate::platform::core::{self as core_platform};
use crate::platform::midi::MidiEventSource;
use crate::runtime::process::service::affinity::ServiceAffinity;
use crate::runtime::process::service::{self};

use super::core::{
    JackClientHandle, JackEndpointInfo, JackTopologyState, activate_client, jack_error,
    monitor_client_name, open_jack_client, query_topology_snapshot, require_jack_library,
};
use super::event::{queue_backend_disconnected_events, refresh_native_event_sessions};

/// Default JACK monitor poll interval.
const DEFAULT_MONITOR_POLL_INTERVAL: Duration = Duration::from_millis(25);

/// One registry of native JACK event subscriptions.
pub(super) struct JackNativeEventRegistry {
    /// Next registration id.
    pub(super) next_registration_id: u64,
    /// Registered native event subscriptions.
    pub(super) sessions: BTreeMap<u64, std::sync::Weak<Mutex<super::core::JackEventSession>>>,
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
    /// Stop flag for the monitor thread.
    stop_flag: Arc<AtomicBool>,
}

impl Drop for JackService {
    /// Stop the monitor thread when the process-global service tears down.
    fn drop(&mut self) {
        self.stop_flag.store(true, Ordering::Release);

        let monitor_thread = self.monitor_thread.get_mut().take();
        if let Some(monitor_thread) = monitor_thread {
            let _ = monitor_thread.join();
        }

        self.monitor_client.get_mut().take();
    }
}

impl JackService {
    /// The host-affinity domain for the JACK backend service.
    pub(crate) const AFFINITY: ServiceAffinity = ServiceAffinity::CallerThread;
}

/// Return the shared JACK MIDI service.
pub(crate) fn jack_service(operation: &'static str) -> RuntimeResult<Arc<JackService>> {
    service::global_service(|| {
        let service_affinity = JackService::AFFINITY;
        debug_assert!(matches!(service_affinity, ServiceAffinity::CallerThread));

        let library = require_jack_library(operation)?;
        let topology = Arc::new(Mutex::new(query_topology_snapshot(operation)?));
        let native_event_registry = Arc::new(Mutex::new(JackNativeEventRegistry {
            next_registration_id: 1,
            sessions: BTreeMap::new(),
        }));
        let stop_flag = Arc::new(AtomicBool::new(false));
        let pending_refresh = Arc::new(AtomicBool::new(false));
        let is_disconnected = Arc::new(AtomicBool::new(false));

        let monitor_client = open_jack_client(library.clone(), operation, &monitor_client_name())?;
        install_monitor_callbacks(&monitor_client, &pending_refresh, &is_disconnected)?;
        activate_client(&monitor_client, operation)?;

        let monitor_thread = spawn_monitor_thread(
            topology.clone(),
            native_event_registry.clone(),
            stop_flag.clone(),
            pending_refresh.clone(),
            is_disconnected.clone(),
        )?;

        Ok(JackService {
            library,
            topology,
            native_event_registry,
            monitor_client: Mutex::new(Some(monitor_client)),
            monitor_thread: Mutex::new(Some(monitor_thread)),
            stop_flag,
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
    session: &Arc<Mutex<super::core::JackEventSession>>,
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
    let Ok(topology) = query_topology_snapshot("destack.midi.jack.refresh") else {
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
    pending_refresh: &Arc<AtomicBool>,
    is_disconnected: &Arc<AtomicBool>,
) -> RuntimeResult<()> {
    let pending_argument = Arc::as_ptr(pending_refresh) as *mut std::ffi::c_void;
    let disconnect_argument = Arc::as_ptr(is_disconnected) as *mut std::ffi::c_void;

    let registration_status = unsafe {
        (client.library.api.jack_set_port_registration_callback)(
            client.raw,
            Some(port_registration_callback),
            pending_argument,
        )
    };
    if registration_status != 0 {
        return Err(jack_error(
            "destack.midi.event.open",
            format!(
                "failed to install JACK port-registration callback (status {registration_status})"
            ),
        ));
    }

    let connect_status = unsafe {
        (client.library.api.jack_set_port_connect_callback)(
            client.raw,
            Some(port_connect_callback),
            pending_argument,
        )
    };
    if connect_status != 0 {
        return Err(jack_error(
            "destack.midi.event.open",
            format!("failed to install JACK port-connect callback (status {connect_status})"),
        ));
    }

    unsafe {
        (client.library.api.jack_on_shutdown)(
            client.raw,
            Some(shutdown_callback),
            disconnect_argument,
        );
    }

    Ok(())
}

/// Spawn one polling monitor thread.
fn spawn_monitor_thread(
    topology: Arc<Mutex<JackTopologyState>>,
    native_event_registry: Arc<Mutex<JackNativeEventRegistry>>,
    stop_flag: Arc<AtomicBool>,
    pending_refresh: Arc<AtomicBool>,
    is_disconnected: Arc<AtomicBool>,
) -> RuntimeResult<JoinHandle<()>> {
    let builder = thread::Builder::new().name("destack-midi-jack-monitor".to_string());

    builder
        .spawn(move || {
            loop {
                // stop request
                if stop_flag.load(Ordering::Acquire) {
                    return;
                }

                // disconnect
                if is_disconnected.swap(false, Ordering::AcqRel) {
                    queue_backend_disconnected_events(
                        &native_event_registry,
                        MidiEventSource::Native,
                        0,
                    );
                    return;
                }

                // topology refresh
                if pending_refresh.swap(false, Ordering::AcqRel) {
                    let next_topology = match query_topology_snapshot("destack.midi.event.refresh")
                    {
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

                    let service = match jack_service("destack.midi.event.refresh") {
                        Ok(service) => service,
                        Err(_) => return,
                    };
                    refresh_native_event_sessions(&service, MidiEventSource::Native);
                }

                thread::sleep(DEFAULT_MONITOR_POLL_INTERVAL);
            }
        })
        .map_err(|error| {
            core_platform::io_operation_error(
                "destack.midi.jack.monitor.spawn",
                None,
                format!("failed to spawn JACK monitor thread: {error}"),
            )
        })
}

/// Handle one JACK port-registration callback.
unsafe extern "C" fn port_registration_callback(
    _port_id: u32,
    _is_registered: libc::c_int,
    argument: *mut std::ffi::c_void,
) {
    let pending = argument.cast::<AtomicBool>();
    if pending.is_null() {
        return;
    }

    unsafe {
        (*pending).store(true, Ordering::Release);
    }
}

/// Handle one JACK port-connect callback.
unsafe extern "C" fn port_connect_callback(
    _source_port_id: u32,
    _destination_port_id: u32,
    _is_connected: libc::c_int,
    argument: *mut std::ffi::c_void,
) {
    let pending = argument.cast::<AtomicBool>();
    if pending.is_null() {
        return;
    }

    unsafe {
        (*pending).store(true, Ordering::Release);
    }
}

/// Handle one JACK backend shutdown callback.
unsafe extern "C" fn shutdown_callback(argument: *mut std::ffi::c_void) {
    let disconnected = argument.cast::<AtomicBool>();
    if disconnected.is_null() {
        return;
    }

    unsafe {
        (*disconnected).store(true, Ordering::Release);
    }
}
