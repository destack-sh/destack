use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::{self, JoinHandle};

use parking_lot::Mutex;

use crate::diagnostic::RuntimeResult;
use crate::platform::core::{self as core_platform, BackendSupport};
use crate::platform::device::{MidiEventSource, MidiPortDirection};
use crate::runtime::service::Service;
use crate::runtime::{ExecutionMode, ExecutionPolicy, start_with_policy};

use super::abi::{
    POLLIN, SND_SEQ_CLIENT_SYSTEM, SND_SEQ_OPEN_INPUT, SND_SEQ_PORT_CAP_NO_EXPORT,
    SND_SEQ_PORT_SYSTEM_ANNOUNCE, poll, pollfd, snd_seq_client_info_t, snd_seq_event_t,
    snd_seq_port_info_t,
};
use super::core::{
    AlsaEndpointInfo, AlsaEventRepository, AlsaHandle, AlsaLibrary, AlsaTopologyState,
    alsa_operation_error, create_simple_port, default_port_type,
    hidden_destination_port_capability, is_input_source, is_internal_client_name,
    is_output_destination, open_sequencer_handle,
};
use super::descriptor::endpoint_descriptor;
use super::event::{queue_backend_disconnected_events, refresh_native_event_sessions};
use super::ffi::alsa_library;

/// One registry of native event subscriptions.
pub(super) struct AlsaNativeEventRegistry {
    /// Next registration id.
    pub(super) next_registration_id: u64,
    /// Registered native event subscriptions.
    pub(super) sessions: BTreeMap<u64, std::sync::Weak<Mutex<AlsaEventRepository>>>,
}

/// One process-global ALSA sequencer service.
pub(crate) struct AlsaService {
    /// Shared ALSA dynamic library owner.
    pub(super) library: Arc<AlsaLibrary>,
    /// Shared topology cache.
    pub(super) topology: Arc<Mutex<AlsaTopologyState>>,
    /// Registered native event subscriptions.
    pub(super) native_event_registry: Arc<Mutex<AlsaNativeEventRegistry>>,
    /// Live announce watcher thread.
    announce_thread: Mutex<Option<JoinHandle<()>>>,
    /// Stop flag for the announce watcher.
    announce_stop_flag: Arc<AtomicBool>,
}

impl Drop for AlsaService {
    /// Stop the announce watcher when the process-global service tears down.
    fn drop(&mut self) {
        self.announce_stop_flag.store(true, Ordering::Release);

        let announce_thread = self.announce_thread.get_mut().take();
        if let Some(announce_thread) = announce_thread {
            let _ = announce_thread.join();
        }
    }
}

impl Service for AlsaService {
    const POLICY: ExecutionPolicy = ExecutionPolicy::process(ExecutionMode::Inline);
}

/// Return the shared ALSA sequencer client service.
pub(crate) fn alsa_service(operation: &'static str) -> RuntimeResult<Arc<AlsaService>> {
    AlsaService::global(|| {
        let Some(library) = alsa_library() else {
            return Err(core_platform::backend_support_error(
                operation,
                "alsa",
                BackendSupport::HostUnavailable,
            ));
        };

        let control_handle = open_sequencer_handle(
            &library,
            "Destack MIDI",
            super::abi::SND_SEQ_OPEN_DUPLEX,
            operation,
        )?;
        let topology = Arc::new(Mutex::new(query_topology_snapshot(&control_handle)?));
        let native_event_registry = Arc::new(Mutex::new(AlsaNativeEventRegistry {
            next_registration_id: 1,
            sessions: BTreeMap::new(),
        }));
        let announce_stop_flag = Arc::new(AtomicBool::new(false));
        let announce_thread = spawn_announce_thread(
            library.clone(),
            topology.clone(),
            native_event_registry.clone(),
            announce_stop_flag.clone(),
        )?;

        Ok(AlsaService {
            library,
            topology,
            native_event_registry,
            announce_thread: Mutex::new(Some(announce_thread)),
            announce_stop_flag,
        })
    })
}

/// Return one current cloned input descriptor snapshot.
pub(super) fn input_descriptors(service: &Arc<AlsaService>) -> Vec<AlsaEndpointInfo> {
    let topology = service.topology.lock();

    topology.inputs.values().cloned().collect()
}

/// Return one current cloned output descriptor snapshot.
pub(super) fn output_descriptors(service: &Arc<AlsaService>) -> Vec<AlsaEndpointInfo> {
    let topology = service.topology.lock();

    topology.outputs.values().cloned().collect()
}

/// Register one native event subscription.
pub(super) fn register_native_event_session(
    service: &Arc<AlsaService>,
    session: &Arc<Mutex<AlsaEventRepository>>,
) -> super::core::AlsaEventDeliveryKind {
    let mut registry = service.native_event_registry.lock();
    let registration_id = registry.next_registration_id;
    registry.next_registration_id = registry.next_registration_id.saturating_add(1);
    registry
        .sessions
        .insert(registration_id, Arc::downgrade(session));

    super::core::AlsaEventDeliveryKind::Native {
        registry: service.native_event_registry.clone(),
        registration_id,
    }
}

/// Remove one native event subscription from the shared registry.
pub(super) fn unregister_native_event_session(delivery_kind: &super::core::AlsaEventDeliveryKind) {
    let super::core::AlsaEventDeliveryKind::Native {
        registry,
        registration_id,
    } = delivery_kind
    else {
        return;
    };

    registry.lock().sessions.remove(registration_id);
}

/// Spawn one ALSA announce watcher thread.
fn spawn_announce_thread(
    library: Arc<AlsaLibrary>,
    topology: Arc<Mutex<AlsaTopologyState>>,
    native_event_registry: Arc<Mutex<AlsaNativeEventRegistry>>,
    stop_flag: Arc<AtomicBool>,
) -> RuntimeResult<JoinHandle<()>> {
    let handle = open_sequencer_handle(
        &library,
        "Destack MIDI Internal Announce",
        SND_SEQ_OPEN_INPUT,
        "destack.device.midi.alsa.announce.open",
    )?;
    let port_id = create_simple_port(
        &handle,
        "Destack MIDI Internal Announce",
        hidden_destination_port_capability(),
        default_port_type(),
        "destack.device.midi.alsa.announce.port",
    )?;
    let status = unsafe {
        (library.api.snd_seq_connect_from)(
            handle.raw,
            port_id,
            SND_SEQ_CLIENT_SYSTEM,
            SND_SEQ_PORT_SYSTEM_ANNOUNCE,
        )
    };
    if status < 0 {
        return Err(alsa_operation_error(
            &library,
            "destack.device.midi.alsa.announce.connect",
            "snd_seq_connect_from",
            status,
        ));
    }

    let announce_thread = start_with_policy(
        "destack-midi-alsa-announce",
        "destack.device.midi.alsa.announce.spawn",
        ExecutionPolicy::process(ExecutionMode::Loop),
        move || {
            run_announce_thread(handle, port_id, topology, native_event_registry, stop_flag);
        },
    )?;

    Ok(announce_thread)
}

/// Run one blocking ALSA announce loop.
fn run_announce_thread(
    handle: AlsaHandle,
    local_port_id: i32,
    topology: Arc<Mutex<AlsaTopologyState>>,
    native_event_registry: Arc<Mutex<AlsaNativeEventRegistry>>,
    stop_flag: Arc<AtomicBool>,
) {
    let mut poll_fds = match sequencer_poll_fds(&handle) {
        Ok(poll_fds) => poll_fds,
        Err(_) => {
            queue_backend_disconnected_events(&native_event_registry, MidiEventSource::Native, 0);
            return;
        }
    };

    loop {
        // stop request
        if stop_flag.load(Ordering::Acquire) {
            break;
        }

        // wait for announce activity or periodic shutdown checks
        let poll_status = unsafe { poll(poll_fds.as_mut_ptr(), poll_fds.len() as _, 100) };
        if poll_status < 0 {
            queue_backend_disconnected_events(&native_event_registry, MidiEventSource::Native, 0);
            break;
        }

        if poll_status == 0 {
            continue;
        }

        // drain all queued announce events
        loop {
            let mut event = std::ptr::null_mut::<snd_seq_event_t>();
            let status =
                unsafe { (handle.library.api.snd_seq_event_input)(handle.raw, &mut event) };

            // no more queued events
            if status == -11 {
                break;
            }

            // backend failure
            if status < 0 {
                queue_backend_disconnected_events(
                    &native_event_registry,
                    MidiEventSource::Native,
                    0,
                );
                return;
            }

            if !event.is_null() {
                unsafe {
                    (handle.library.api.snd_seq_free_event)(event);
                }
            }

            // recompute topology from the authoritative control handle snapshot
            let refreshed = refresh_topology_snapshot(&handle, &topology);
            if refreshed.is_ok() {
                refresh_native_event_sessions(
                    &topology,
                    &native_event_registry,
                    MidiEventSource::Native,
                );
            } else {
                queue_backend_disconnected_events(
                    &native_event_registry,
                    MidiEventSource::Native,
                    0,
                );
                return;
            }
        }
    }

    let _ = unsafe {
        (handle.library.api.snd_seq_disconnect_from)(
            handle.raw,
            local_port_id,
            SND_SEQ_CLIENT_SYSTEM,
            SND_SEQ_PORT_SYSTEM_ANNOUNCE,
        )
    };
    let _ = unsafe { (handle.library.api.snd_seq_delete_simple_port)(handle.raw, local_port_id) };
}

/// Refresh one shared topology cache from one live ALSA handle.
fn refresh_topology_snapshot(
    handle: &AlsaHandle,
    topology: &Arc<Mutex<AlsaTopologyState>>,
) -> RuntimeResult<()> {
    let snapshot = query_topology_snapshot(handle)?;

    let mut topology = topology.lock();
    *topology = snapshot;

    Ok(())
}

/// Query one full ALSA topology snapshot.
fn query_topology_snapshot(handle: &AlsaHandle) -> RuntimeResult<AlsaTopologyState> {
    let client_info = allocate_client_info(&handle.library)?;
    let port_info = allocate_port_info(&handle.library)?;
    let mut topology = AlsaTopologyState::default();

    // start client iteration before the first client id
    unsafe {
        (handle.library.api.snd_seq_client_info_set_client)(client_info, -1);
    }

    loop {
        // next client
        let status =
            unsafe { (handle.library.api.snd_seq_query_next_client)(handle.raw, client_info) };
        if status < 0 {
            break;
        }

        let client_id = unsafe { (handle.library.api.snd_seq_client_info_get_client)(client_info) };
        let client_name = unsafe { (handle.library.api.snd_seq_client_info_get_name)(client_info) };
        let client_name = core_platform::string_from_c_str(client_name)
            .unwrap_or_else(|| format!("ALSA Client {client_id}"));

        // skip internal plumbing clients
        if is_internal_client_name(&client_name) {
            continue;
        }

        unsafe {
            (handle.library.api.snd_seq_port_info_set_client)(port_info, client_id);
            (handle.library.api.snd_seq_port_info_set_port)(port_info, -1);
        }

        loop {
            // next port
            let status =
                unsafe { (handle.library.api.snd_seq_query_next_port)(handle.raw, port_info) };
            if status < 0 {
                break;
            }

            let port_id = unsafe { (handle.library.api.snd_seq_port_info_get_port)(port_info) };
            let port_name = unsafe { (handle.library.api.snd_seq_port_info_get_name)(port_info) };
            let port_name = core_platform::string_from_c_str(port_name)
                .unwrap_or_else(|| format!("Port {port_id}"));
            let capability =
                unsafe { (handle.library.api.snd_seq_port_info_get_capability)(port_info) };
            let port_type = unsafe { (handle.library.api.snd_seq_port_info_get_type)(port_info) };

            // skip hidden internal helper ports
            if capability & SND_SEQ_PORT_CAP_NO_EXPORT != 0 {
                continue;
            }

            // input sources
            if is_input_source(capability) {
                let descriptor = endpoint_descriptor(
                    MidiPortDirection::Input,
                    client_id,
                    &client_name,
                    port_id,
                    port_name.clone(),
                    port_type,
                );
                topology.inputs.insert(
                    (client_id, port_id),
                    AlsaEndpointInfo {
                        descriptor,
                        client_id,
                        port_id,
                    },
                );
            }

            // output destinations
            if is_output_destination(capability) {
                let descriptor = endpoint_descriptor(
                    MidiPortDirection::Output,
                    client_id,
                    &client_name,
                    port_id,
                    port_name.clone(),
                    port_type,
                );
                topology.outputs.insert(
                    (client_id, port_id),
                    AlsaEndpointInfo {
                        descriptor,
                        client_id,
                        port_id,
                    },
                );
            }
        }
    }

    unsafe {
        (handle.library.api.snd_seq_client_info_free)(client_info);
        (handle.library.api.snd_seq_port_info_free)(port_info);
    }

    Ok(topology)
}

/// Allocate one ALSA client-info payload.
fn allocate_client_info(library: &Arc<AlsaLibrary>) -> RuntimeResult<*mut snd_seq_client_info_t> {
    let mut info = std::ptr::null_mut();
    let status = unsafe { (library.api.snd_seq_client_info_malloc)(&mut info) };
    if status < 0 || info.is_null() {
        return Err(alsa_operation_error(
            library,
            "destack.device.midi.alsa.clientInfo",
            "snd_seq_client_info_malloc",
            status,
        ));
    }

    Ok(info)
}

/// Allocate one ALSA port-info payload.
fn allocate_port_info(library: &Arc<AlsaLibrary>) -> RuntimeResult<*mut snd_seq_port_info_t> {
    let mut info = std::ptr::null_mut();
    let status = unsafe { (library.api.snd_seq_port_info_malloc)(&mut info) };
    if status < 0 || info.is_null() {
        return Err(alsa_operation_error(
            library,
            "destack.device.midi.alsa.portInfo",
            "snd_seq_port_info_malloc",
            status,
        ));
    }

    Ok(info)
}

/// Return the poll descriptors for one ALSA sequencer handle.
fn sequencer_poll_fds(handle: &AlsaHandle) -> RuntimeResult<Vec<pollfd>> {
    let count = unsafe { (handle.library.api.snd_seq_poll_descriptors_count)(handle.raw, POLLIN) };
    if count <= 0 {
        return Err(core_platform::io_operation_error(
            "destack.device.midi.alsa.pollDescriptors",
            None,
            "ALSA did not report any poll descriptors",
        ));
    }

    let mut poll_fds = vec![pollfd::default(); count as usize];
    let status = unsafe {
        (handle.library.api.snd_seq_poll_descriptors)(
            handle.raw,
            poll_fds.as_mut_ptr(),
            poll_fds.len() as u32,
            POLLIN,
        )
    };
    if status < 0 {
        return Err(alsa_operation_error(
            &handle.library,
            "destack.device.midi.alsa.pollDescriptors",
            "snd_seq_poll_descriptors",
            status,
        ));
    }

    Ok(poll_fds)
}
