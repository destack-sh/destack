use std::sync::Arc;
#[cfg(target_os = "macos")]
use std::sync::{Condvar, Mutex};

#[cfg(target_os = "macos")]
use super::abi::{CoreAudioHostStreamOps, CoreAudioStreamRuntime};
#[cfg(target_os = "macos")]
use super::property::{enable_hog_mode, validate_open_stream_config};
#[cfg(target_os = "macos")]
use super::runtime::{create_capture_queue, create_playback_queue, dispose_runtime_handles};
#[cfg(target_os = "macos")]
use super::sample::device_id_from_stable_id;
use crate::diagnostic::RuntimeResult;
#[cfg(target_os = "macos")]
use crate::platform::audio::AudioDeviceDirection;
#[cfg(not(target_os = "macos"))]
use crate::platform::audio::backend::backend_not_supported;
#[cfg(not(target_os = "macos"))]
use crate::platform::audio::core::constants::AudioBackendOpenFlags;
#[cfg(target_os = "macos")]
use crate::platform::audio::core::constants::{
    AudioBackendOpenFlags, MIN_STREAM_PERIOD_FRAMES, resolved_max_queued_frames,
};
#[cfg(target_os = "macos")]
use crate::platform::audio::core::model::{
    AudioHostStreamOps, AudioStreamHostState, AudioStreamRuntimeCapabilities, AudioStreamSync,
    HostDeviceDescriptor, initial_stream_state,
};
#[cfg(not(target_os = "macos"))]
use crate::platform::audio::core::model::{AudioStreamHostState, HostDeviceDescriptor};
use crate::platform::audio::{AudioShareMode, AudioStreamConfig};

/// Build an initialized stream host state for a CoreAudio stream open request.
#[cfg(target_os = "macos")]
fn new_stream_state(
    device_info: &HostDeviceDescriptor,
    config: AudioStreamConfig,
    share_mode: AudioShareMode,
    runtime: Arc<CoreAudioStreamRuntime>,
) -> Arc<AudioStreamHostState> {
    // install host stream operations for queue lifecycle control
    let host_ops: Arc<dyn AudioHostStreamOps> = Arc::new(CoreAudioHostStreamOps { runtime });

    // build the stream host state
    Arc::new(AudioStreamHostState {
        device: device_info.clone(),
        direction: device_info.direction,
        requested: config,
        sample_rate: config.sample_rate,
        channels: config.channels,
        period_frames: config.period_frames.max(MIN_STREAM_PERIOD_FRAMES),
        max_queued_frames: resolved_max_queued_frames(),
        share_mode,
        runtime_capabilities: AudioStreamRuntimeCapabilities {
            supports_write_at: matches!(
                device_info.direction,
                AudioDeviceDirection::Playback | AudioDeviceDirection::Duplex
            ),
            supports_pause: true,
            supports_non_interleaved: false,
            supports_volume: true,
            supports_mute: true,
            supports_hardware_timestamps: true,
        },
        host_ops: Mutex::new(Some(host_ops)),
        name: Mutex::new(String::new()),
        sync: Arc::new(AudioStreamSync {
            state: Mutex::new(initial_stream_state()),
            wake: Condvar::new(),
        }),
        stream_handle_raw: std::sync::atomic::AtomicU64::new(0),
        runtime_owner: Mutex::new(None),
        worker_thread: Mutex::new(None),
    })
}

/// Open a CoreAudio stream on macOS.
#[cfg(target_os = "macos")]
fn open_host_stream_macos(
    device_info: &HostDeviceDescriptor,
    config: AudioStreamConfig,
    share_mode: AudioShareMode,
) -> RuntimeResult<Arc<AudioStreamHostState>> {
    // resolve and validate device configuration before queue creation
    let device_id = device_id_from_stable_id(&device_info.id)?;
    validate_open_stream_config(device_id, device_info.direction, config)?;

    // acquire exclusive hog mode only when explicitly requested
    let release_hog_mode_on_drop = if share_mode == AudioShareMode::Exclusive {
        enable_hog_mode(device_id)?
    } else {
        false
    };

    // initialize stream runtime and host state
    let runtime = Arc::new(CoreAudioStreamRuntime {
        queue_handles: Mutex::new(Vec::new()),
        device_id,
        release_hog_mode_on_drop,
    });
    let stream_state = new_stream_state(device_info, config, share_mode, runtime.clone());

    // derive queue requirements from stream direction
    let needs_playback_queue = matches!(
        device_info.direction,
        AudioDeviceDirection::Playback | AudioDeviceDirection::Duplex
    );
    let needs_capture_queue = matches!(
        device_info.direction,
        AudioDeviceDirection::Capture
            | AudioDeviceDirection::Duplex
            | AudioDeviceDirection::Loopback
    );

    // create and attach the playback queue when needed
    if needs_playback_queue {
        let queue_handle = match create_playback_queue(&stream_state, device_id) {
            Ok(queue_handle) => queue_handle,
            Err(error) => {
                dispose_runtime_handles(&runtime);
                return Err(error);
            }
        };

        let mut queue_handles = runtime
            .queue_handles
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        queue_handles.push(queue_handle);
    }

    // create and attach the capture queue when needed
    if needs_capture_queue {
        match create_capture_queue(&stream_state, device_id) {
            Ok(queue_handle) => {
                let mut queue_handles = runtime
                    .queue_handles
                    .lock()
                    .unwrap_or_else(|error| error.into_inner());
                queue_handles.push(queue_handle);
            }
            Err(error) => {
                dispose_runtime_handles(&runtime);
                return Err(error);
            }
        }
    }

    Ok(stream_state)
}

/// Return whether CoreAudio backend support is implemented for this build.
pub(crate) fn is_backend_supported() -> bool {
    cfg!(target_os = "macos")
}

/// Return whether CoreAudio stream open support is implemented for this build.
pub(crate) fn is_stream_supported() -> bool {
    cfg!(target_os = "macos")
}

/// Open a CoreAudio stream.
pub(crate) fn open_host_stream(
    device_info: &HostDeviceDescriptor,
    config: AudioStreamConfig,
    share_mode: AudioShareMode,
    backend_flags: AudioBackendOpenFlags,
) -> RuntimeResult<Arc<AudioStreamHostState>> {
    #[cfg(target_os = "macos")]
    {
        let _ = backend_flags;
        open_host_stream_macos(device_info, config, share_mode)
    }

    #[cfg(not(target_os = "macos"))]
    {
        let _ = (device_info, config, share_mode, backend_flags);
        Err(backend_not_supported(
            "destack.audio.stream.open",
            "coreaudio",
        ))
    }
}
