use std::collections::{HashMap, HashSet};

use crate::diagnostic::RuntimeResult;
use crate::platform::audio::{
    AudioBackend, AudioBackendSelectionPolicy, AudioDeviceDirection, AudioDeviceListFlags,
    AudioDeviceListRequest,
};

use super::super::device::enumerate_devices_for_request;
use super::super::model::{
    AudioDeviceMonitorBaseline, HostDeviceDescriptor, audio_device_monitor_baseline,
};

/// One monitor snapshot used to compare device publication state.
pub(crate) struct MonitorSnapshot {
    /// Device signatures keyed by stable device id.
    pub(crate) signatures: HashMap<String, u64>,
    /// Device id set for one snapshot.
    pub(crate) ids: HashSet<String>,
    /// Default playback id from one snapshot.
    pub(crate) default_playback: Option<String>,
    /// Default capture id from one snapshot.
    pub(crate) default_capture: Option<String>,
    /// Default loopback id from one snapshot.
    pub(crate) default_loopback: Option<String>,
}

/// Build one stable hash for one identifier payload.
pub(crate) fn stable_hash(text: &str) -> u64 {
    let mut hash = 0xcbf29ce484222325u64;

    for byte in text.as_bytes() {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }

    hash
}

/// Build one signature value for one device descriptor.
pub(crate) fn device_signature(info: &HostDeviceDescriptor) -> u64 {
    let text = format!(
        "{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}",
        info.id,
        info.name,
        info.backend as u8,
        info.direction as u8,
        info.supported_directions,
        info.min_sample_rate,
        info.max_sample_rate,
        info.min_channels,
        info.max_channels,
        info.format_mask,
        info.supported_channel_mask,
    );

    stable_hash(&text)
}

/// Enumerate monitor-visible devices for one backend selector.
fn enumerate_monitor_devices(backend: AudioBackend) -> RuntimeResult<Vec<HostDeviceDescriptor>> {
    let mut devices = Vec::new();
    let mut seen = HashSet::new();

    for direction in [
        AudioDeviceDirection::Playback,
        AudioDeviceDirection::Capture,
        AudioDeviceDirection::Duplex,
        AudioDeviceDirection::Loopback,
    ] {
        let request = AudioDeviceListRequest {
            direction,
            backend,
            backend_policy: AudioBackendSelectionPolicy::Strict,
            flags: AudioDeviceListFlags(0),
        };

        for device in enumerate_devices_for_request(request)? {
            if seen.insert(device.id.clone()) {
                devices.push(device);
            }
        }
    }

    Ok(devices)
}

/// Capture one monitor snapshot for one backend.
pub(crate) fn monitor_snapshot(backend: AudioBackend) -> RuntimeResult<MonitorSnapshot> {
    let devices = enumerate_monitor_devices(backend)?;
    let mut signatures = HashMap::new();
    let mut ids = HashSet::new();
    let mut default_playback = None;
    let mut default_capture = None;
    let mut default_loopback = None;

    for device in &devices {
        ids.insert(device.id.clone());
        signatures.insert(device.id.clone(), device_signature(device));

        if device.is_default_playback {
            default_playback = Some(device.id.clone());
        }

        if device.is_default_capture {
            default_capture = Some(device.id.clone());
        }

        if device.is_default_loopback {
            default_loopback = Some(device.id.clone());
        }
    }

    Ok(MonitorSnapshot {
        signatures,
        ids,
        default_playback,
        default_capture,
        default_loopback,
    })
}

/// Capture one initial device monitor baseline for one backend.
pub(crate) fn initial_device_monitor_baseline(
    backend: AudioBackend,
    now: u64,
) -> RuntimeResult<AudioDeviceMonitorBaseline> {
    let snapshot = monitor_snapshot(backend)?;

    Ok(audio_device_monitor_baseline(
        snapshot.signatures,
        snapshot.default_playback,
        snapshot.default_capture,
        snapshot.default_loopback,
        now,
    ))
}
