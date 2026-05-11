use std::collections::BTreeMap;
use std::sync::Arc;

use parking_lot::Mutex;

use crate::diagnostic::RuntimeResult;
use crate::runtime::service::Service;
use crate::runtime::{ExecutionMode, ExecutionPolicy};
use windows_sys::Win32::Media::Audio::{midiInGetNumDevs, midiOutGetNumDevs};

use super::core::{
    WinMmEndpointInfo, WinMmTopologyState, query_input_descriptors, query_output_descriptors,
};
use super::descriptor::{input_descriptor, output_descriptor};

/// One process-global WinMM runtime service.
pub(crate) struct WinMmService {
    /// Shared topology cache.
    pub(super) topology: Arc<Mutex<WinMmTopologyState>>,
}

impl WinMmService {
    /// Refresh one cached WinMM topology snapshot.
    pub(super) fn refresh_topology(&self, operation: &'static str) -> RuntimeResult<()> {
        let topology = query_topology_snapshot(operation)?;
        let mut service_topology = self.topology.lock();
        *service_topology = topology;

        Ok(())
    }
}

impl Service for WinMmService {
    const POLICY: ExecutionPolicy = ExecutionPolicy::process(ExecutionMode::Inline);
}

/// Return the shared WinMM runtime service.
pub(crate) fn winmm_service(operation: &'static str) -> RuntimeResult<Arc<WinMmService>> {
    WinMmService::global(|| {
        let topology = Arc::new(Mutex::new(query_topology_snapshot(operation)?));

        Ok(WinMmService { topology })
    })
}

/// Check whether the legacy WinMM MIDI API surface is reachable on this host.
pub(crate) fn check_winmm_support(_operation: &'static str) -> RuntimeResult<()> {
    let _input_count = unsafe { midiInGetNumDevs() };
    let _output_count = unsafe { midiOutGetNumDevs() };

    Ok(())
}

/// Return one current cloned input descriptor snapshot.
pub(super) fn input_descriptors(service: &Arc<WinMmService>) -> Vec<WinMmEndpointInfo> {
    let topology = service.topology.lock();

    topology.inputs.values().cloned().collect()
}

/// Return one current cloned output descriptor snapshot.
pub(super) fn output_descriptors(service: &Arc<WinMmService>) -> Vec<WinMmEndpointInfo> {
    let topology = service.topology.lock();

    topology.outputs.values().cloned().collect()
}

/// Query one current process-global WinMM topology snapshot.
fn query_topology_snapshot(operation: &'static str) -> RuntimeResult<WinMmTopologyState> {
    let mut inputs = BTreeMap::new();
    let mut outputs = BTreeMap::new();

    // input devices
    for (device_id, caps) in query_input_descriptors(operation)? {
        let descriptor = input_descriptor(device_id, &caps);
        inputs.insert(
            descriptor.id.clone(),
            WinMmEndpointInfo {
                descriptor,
                device_id,
            },
        );
    }

    // output devices
    for (device_id, caps) in query_output_descriptors(operation)? {
        let descriptor = output_descriptor(device_id, &caps);
        outputs.insert(
            descriptor.id.clone(),
            WinMmEndpointInfo {
                descriptor,
                device_id,
            },
        );
    }

    Ok(WinMmTopologyState { inputs, outputs })
}
