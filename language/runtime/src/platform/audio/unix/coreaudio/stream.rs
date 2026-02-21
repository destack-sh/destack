use std::sync::Arc;

#[cfg(not(target_os = "macos"))]
use super::super::backend::backend_not_supported;
use super::core::*;
use crate::diagnostic::RuntimeResult;
use crate::platform::audio::core as audio_core;

/// Build one initialized stream binding for one CoreAudio stream open request.
#[cfg(target_os = "macos")]
fn new_stream_binding(
    device_info: &audio_core::HostDeviceDescriptor,
    config: audio_core::AudioStreamConfig,
    share_mode: audio_core::AudioShareMode,
    runtime: Arc<CoreAudioStreamRuntime>,
) -> Arc<audio_core::AudioStreamBinding> {
    // install host stream operations for queue lifecycle control
    let host_ops: Arc<dyn audio_core::AudioHostStreamOps> =
        Arc::new(CoreAudioHostStreamOps { runtime });

    // build one stream binding with initialized runtime state
    Arc::new(audio_core::AudioStreamBinding {
        device: device_info.clone(),
        direction: device_info.direction,
        requested: config,
        sample_rate: config.sample_rate,
        channels: config.channels,
        period_frames: config
            .period_frames
            .max(audio_core::MIN_STREAM_PERIOD_FRAMES),
        share_mode,
        runtime_capabilities: audio_core::AudioStreamRuntimeCapabilities {
            supports_write_at: false,
            supports_pause: true,
            supports_non_interleaved: false,
            supports_volume: true,
            supports_mute: true,
            supports_hardware_timestamps: false,
        },
        host_ops: audio_core::Mutex::new(Some(host_ops)),
        name: audio_core::Mutex::new(String::new()),
        sync: Arc::new(audio_core::AudioStreamSync {
            state: audio_core::Mutex::new(audio_core::initial_stream_state()),
            wake: audio_core::Condvar::new(),
        }),
        null_worker: audio_core::Mutex::new(None),
    })
}

/// Open one CoreAudio stream on macOS.
#[cfg(target_os = "macos")]
fn open_host_stream_macos(
    device_info: &audio_core::HostDeviceDescriptor,
    config: audio_core::AudioStreamConfig,
    share_mode: audio_core::AudioShareMode,
) -> RuntimeResult<Arc<audio_core::AudioStreamBinding>> {
    // resolve and validate device configuration before queue creation
    let device_id = device_id_from_stable_id(&device_info.id)?;
    validate_open_stream_config(device_id, device_info.direction, config)?;

    // acquire exclusive hog mode only when explicitly requested
    let release_hog_mode_on_drop = if share_mode == audio_core::AudioShareMode::Exclusive {
        enable_hog_mode(device_id)?
    } else {
        false
    };

    // initialize stream runtime and binding state
    let runtime = Arc::new(CoreAudioStreamRuntime {
        queue_handles: audio_core::Mutex::new(Vec::new()),
        device_id,
        release_hog_mode_on_drop,
    });
    let binding = new_stream_binding(device_info, config, share_mode, runtime.clone());

    // derive queue requirements from stream direction
    let needs_playback_queue = matches!(
        device_info.direction,
        audio_core::AudioDeviceDirection::Playback | audio_core::AudioDeviceDirection::Duplex
    );
    let needs_capture_queue = matches!(
        device_info.direction,
        audio_core::AudioDeviceDirection::Capture
            | audio_core::AudioDeviceDirection::Duplex
            | audio_core::AudioDeviceDirection::Loopback
    );

    // create and attach one playback queue when needed
    if needs_playback_queue {
        let queue_handle = match create_playback_queue(&binding, device_id) {
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

    // create and attach one capture queue when needed
    if needs_capture_queue {
        match create_capture_queue(&binding, device_id) {
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

    // launch deferred cleanup worker tied to stream shutdown state
    let worker = spawn_cleanup_thread(binding.clone(), runtime);
    *binding
        .null_worker
        .lock()
        .unwrap_or_else(|error| error.into_inner()) = Some(worker);

    Ok(binding)
}

/// Return whether CoreAudio backend support is implemented for this build.
pub(crate) fn is_backend_supported() -> bool {
    cfg!(target_os = "macos")
}

/// Return whether CoreAudio stream open support is implemented for this build.
pub(crate) fn is_stream_supported() -> bool {
    cfg!(target_os = "macos")
}

/// Open one CoreAudio stream.
pub(crate) fn open_host_stream(
    device_info: &audio_core::HostDeviceDescriptor,
    config: audio_core::AudioStreamConfig,
    share_mode: audio_core::AudioShareMode,
) -> RuntimeResult<Arc<audio_core::AudioStreamBinding>> {
    #[cfg(target_os = "macos")]
    {
        open_host_stream_macos(device_info, config, share_mode)
    }

    #[cfg(not(target_os = "macos"))]
    {
        let _ = (device_info, config, share_mode);
        Err(backend_not_supported(
            "destack.audio.stream.open",
            "coreaudio",
        ))
    }
}
