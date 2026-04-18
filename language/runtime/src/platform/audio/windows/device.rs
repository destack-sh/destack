use std::sync::Arc;

use super::super::{backend_descriptors, resolve_requested_backend};
use super::core as audio_platform_core;
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::abi::{NativeSlice, NativeStringRef};
use crate::platform::audio::core::{
    AUDIO_DEVICE_RESOURCE_LABEL, AudioDeviceHostState, DEVICE_CAPABILITY_LOOPBACK,
    SHARE_MODE_EXCLUSIVE_BIT, SHARE_MODE_SHARED_BIT, audio_not_found, descriptor_from_device_state,
    descriptor_from_info, ensure_device_open_flags_supported, enumerate_devices_for_request,
    normalize_device_open_options, null_device, read_utf8, refresh_device_subscriptions_for_rescan,
    resolve_device_host_state, supports_device_open_direction,
};
use crate::platform::audio::{
    AudioBackend, AudioBackendDescriptor, AudioBackendSelectionPolicy, AudioDeviceDescriptor,
    AudioDeviceDirection, AudioDeviceListFlags, AudioDeviceListRequest, AudioDeviceOpenOptions,
    AudioShareMode,
};
use crate::platform::resource::{ResourceEntry, ResourceKind};
use crate::platform::{PlatformError, resource};
use crate::runtime::BindingCallContext;

/// List host audio backends.
///
/// Enumerate available backend implementations and backend-level feature flags.
///
/// # Platform
/// Unix and Windows.
/// Mirrors cubeb `cubeb_get_backend_names`, libsoundio `soundio_backend_count` plus `soundio_get_backend`, and miniaudio `ma_get_enabled_backends`.
/// Mirrors PortAudio host-api enumeration through `PaHostApiTypeId`.
///
/// # Errors
/// Returns ioNotFound, ioInvalidData, notSupported.
///
/// # Security
/// Requires `audio.device`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_audio_backend_list(
    binding: &BindingCallContext,
    out: *mut NativeSlice<AudioBackendDescriptor>,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    let backends = backend_descriptors(binding);
    unsafe {
        *out = binding.store_slice(backends);
    }

    Ok(())
}

/// Trigger one backend rescan.
///
/// Request one immediate backend device rescan.
/// This allows recovery from stale backend snapshots after hotplug churn.
///
/// # Platform
/// Unix and Windows.
/// Mirrors libsoundio `soundio_force_device_scan` semantics and backend-native refresh flows.
///
/// # Errors
/// Returns ioNotFound, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `audio.device.monitor`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_audio_device_rescan(
    binding: &BindingCallContext,
    backend: AudioBackend,
    backend_policy: AudioBackendSelectionPolicy,
) -> RuntimeResult<()> {
    let backend =
        resolve_requested_backend(backend, backend_policy, "destack.audio.device.rescan")?;

    if backend != AudioBackend::Null {
        audio_platform_core::rescan_host_backend(backend)?;
    }
    refresh_device_subscriptions_for_rescan(binding, backend)?;

    Ok(())
}

/// Close one audio device endpoint.
///
/// Close one opened audio endpoint and release host resources.
/// Close semantics follow host backend teardown behavior.
///
/// # Platform
/// Unix and Windows.
/// Uses ALSA, PulseAudio, PipeWire, CoreAudio, WASAPI, AAudio, OpenSL ES, JACK, and ASIO where available endpoint close operations.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `audio.device`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_audio_device_close(
    binding: &BindingCallContext,
    handle: resource::AudioDeviceHandle,
) -> RuntimeResult<()> {
    let removed =
        binding
            .worker()
            .resources
            .remove(&binding.world(), handle.0, Some(binding.engine()));
    if removed.is_none() {
        return Err(audio_not_found(
            "destack.audio.device.close",
            format!("unknown audio device handle {}", handle.0.0),
        ));
    }

    Ok(())
}

/// Read one default device identifier for the selected direction.
///
/// Resolve one default host audio endpoint for the selected direction and backend policy.
/// Default selection can change asynchronously as host policy changes.
///
/// # Platform
/// Unix and Windows.
/// Mirrors cubeb and libsoundio default-endpoint query semantics.
/// Mirrors SDL default logical-device routing behavior for playback and recording defaults.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioInvalidData, notSupported.
///
/// # Security
/// Requires `audio.device`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_audio_device_default(
    binding: &BindingCallContext,
    out: *mut NativeStringRef,
    direction: AudioDeviceDirection,
    backend: AudioBackend,
    backend_policy: AudioBackendSelectionPolicy,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    let backend =
        resolve_requested_backend(backend, backend_policy, "destack.audio.device.default")?;

    let request = AudioDeviceListRequest {
        direction,
        backend,
        backend_policy: AudioBackendSelectionPolicy::Strict,
        flags: AudioDeviceListFlags(0),
    };

    let devices = enumerate_devices_for_request(request)?;
    let selected = match direction {
        AudioDeviceDirection::Playback => devices
            .iter()
            .find(|device| device.is_default_playback)
            .or_else(|| devices.first()),
        AudioDeviceDirection::Capture => devices
            .iter()
            .find(|device| device.is_default_capture)
            .or_else(|| devices.first()),
        AudioDeviceDirection::Duplex => devices
            .iter()
            .find(|device| device.is_default_playback && device.is_default_capture)
            .or_else(|| {
                devices
                    .iter()
                    .find(|device| device.direction == AudioDeviceDirection::Duplex)
            })
            .or_else(|| devices.first()),
        AudioDeviceDirection::Loopback => devices
            .iter()
            .find(|device| device.is_default_loopback)
            .or_else(|| {
                devices
                    .iter()
                    .find(|device| device.direction == AudioDeviceDirection::Loopback)
            })
            .or_else(|| devices.first()),
    }
    .ok_or_else(|| {
        audio_not_found(
            "destack.audio.device.default",
            "no default audio device available",
        )
    })?;

    unsafe {
        *out = binding.store_string(&selected.id);
    }
    Ok(())
}

/// Read metadata for one opened device endpoint.
///
/// Read one normalized snapshot for one opened device handle.
/// Snapshot values are advisory and can change as host routes are updated.
///
/// # Platform
/// Unix and Windows.
/// Uses ALSA, PulseAudio, PipeWire, CoreAudio, WASAPI, AAudio, OpenSL ES, JACK, and ASIO where available endpoint information query APIs.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioInvalidData, notSupported.
///
/// # Security
/// Requires `audio.device`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_audio_device_descriptor(
    binding: &BindingCallContext,
    out: *mut AudioDeviceDescriptor,
    handle: resource::AudioDeviceHandle,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    let device_state =
        resolve_device_host_state(binding, handle, "destack.audio.device.descriptor")?;
    let descriptor = descriptor_from_device_state(binding, &device_state);
    unsafe {
        *out = descriptor;
    }

    Ok(())
}

/// List available audio devices.
///
/// Enumerate host audio endpoints and return stable identifiers for later open operations.
/// Device visibility and ordering follow host backend semantics.
///
/// # Platform
/// Unix and Windows.
/// Uses ALSA, PulseAudio, PipeWire, CoreAudio, WASAPI, AAudio, OpenSL ES, JACK, and ASIO where available device enumeration.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, notSupported.
///
/// # Security
/// Requires `audio.device`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_audio_device_list(
    binding: &BindingCallContext,
    out: *mut NativeSlice<AudioDeviceDescriptor>,
    request: AudioDeviceListRequest,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    let devices = enumerate_devices_for_request(request)?;
    let mut descriptors = Vec::with_capacity(devices.len());
    for device in &devices {
        descriptors.push(descriptor_from_info(binding, device));
    }

    unsafe {
        *out = binding.store_slice(descriptors);
    }
    Ok(())
}

/// Open one audio device endpoint.
///
/// Open one host audio endpoint for playback, capture, duplex, or loopback operation.
/// Handle lifetime and exclusivity semantics follow host backend rules.
///
/// # Platform
/// Unix and Windows.
/// Uses ALSA, PulseAudio, PipeWire, CoreAudio, WASAPI, AAudio, OpenSL ES, JACK, and ASIO where available endpoint open operations.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `audio.device`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_audio_device_open(
    binding: &BindingCallContext,
    out: *mut resource::AudioDeviceHandle,
    id: NativeStringRef,
    options: AudioDeviceOpenOptions,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    let id = read_utf8(id, "id")?;
    let backend = resolve_requested_backend(
        options.backend,
        options.backend_policy,
        "destack.audio.device.open",
    )?;
    let options = normalize_device_open_options(options, backend, "destack.audio.device.open")?;

    if id.starts_with("audio:null:") && backend != AudioBackend::Null {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "id",
            "null backend device ids require backend null",
        ))
        .boxed());
    }

    let info = if backend == AudioBackend::Null {
        match id.as_str() {
            "audio:null:playback" => null_device(AudioDeviceDirection::Playback),
            "audio:null:capture" => null_device(AudioDeviceDirection::Capture),
            "audio:null:duplex" => null_device(AudioDeviceDirection::Duplex),
            "audio:null:loopback" => null_device(AudioDeviceDirection::Loopback),
            _ => {
                return Err(audio_not_found(
                    "destack.audio.device.open",
                    format!("unknown null backend device id: {id}"),
                ));
            }
        }
    } else {
        audio_platform_core::resolve_host_device_by_id(backend, &id)?
    };
    ensure_device_open_flags_supported(options, &info, "destack.audio.device.open")?;

    if options.direction == AudioDeviceDirection::Loopback
        && (info.capability_flags.0 & DEVICE_CAPABILITY_LOOPBACK.0) == 0
    {
        return Err(RuntimeError::from(PlatformError::not_supported(
            "destack.audio.device.open loopback direction",
        ))
        .boxed());
    }

    if !supports_device_open_direction(&info, options.direction) {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "options.direction",
            "requested direction is not supported by this device",
        ))
        .boxed());
    }

    let share_mode_bit = match options.share_mode {
        AudioShareMode::Shared => SHARE_MODE_SHARED_BIT,
        AudioShareMode::Exclusive => SHARE_MODE_EXCLUSIVE_BIT,
    };
    if (info.share_mode_mask & share_mode_bit) == 0 {
        return Err(RuntimeError::from(PlatformError::not_supported(
            "destack.audio.device.open share mode",
        ))
        .boxed());
    }

    let mut options = options;
    options.backend = backend;

    let payload = Arc::new(AudioDeviceHostState {
        info,
        opened_direction: options.direction,
        options,
    });
    let handle_id = binding.worker().resources.insert(
        &binding.world(),
        ResourceEntry::new(ResourceKind::AudioDevice)
            .with_label(AUDIO_DEVICE_RESOURCE_LABEL)
            .with_payload(payload),
        Some(binding.engine()),
    );

    unsafe {
        *out = resource::AudioDeviceHandle(handle_id);
    }

    Ok(())
}
