#[cfg(target_os = "macos")]
use std::ffi::c_void;
#[cfg(target_os = "macos")]
use std::mem::size_of;
#[cfg(target_os = "macos")]
use std::sync::Arc;

#[cfg(target_os = "macos")]
use crate::diagnostic::{RuntimeError, RuntimeResult};
#[cfg(target_os = "macos")]
use crate::platform::PlatformError;
#[cfg(target_os = "macos")]
use crate::platform::audio::core::{AudioStreamHostState, MIN_STREAM_PERIOD_FRAMES, frame_bytes};
#[cfg(target_os = "macos")]
use crate::platform::diagnostic::PlatformErrorCode;

#[cfg(target_os = "macos")]
use super::abi::{
    AudioDeviceID, AudioQueueDispose, AudioQueueRef, AudioQueueSetProperty, AudioQueueStop,
    CFRelease, CFStringRef, CFTypeRef, CoreAudioQueueHandle,
};
#[cfg(target_os = "macos")]
use super::constants::{
    K_AUDIO_DEVICE_PROPERTY_DEVICE_UID, K_AUDIO_OBJECT_PROPERTY_SCOPE_GLOBAL,
    K_AUDIO_QUEUE_PROPERTY_CURRENT_DEVICE, K_NO_ERR,
};
#[cfg(target_os = "macos")]
use super::property::{error, get_scalar_optional};

#[cfg(target_os = "macos")]
pub(super) fn bind_queue_device(
    queue: AudioQueueRef,
    device_id: AudioDeviceID,
) -> RuntimeResult<()> {
    let device_uid = get_scalar_optional::<CFStringRef>(
        device_id,
        K_AUDIO_DEVICE_PROPERTY_DEVICE_UID,
        K_AUDIO_OBJECT_PROPERTY_SCOPE_GLOBAL,
    )
    .ok_or_else(|| {
        RuntimeError::from(PlatformError::io_with(
            Some(PlatformErrorCode::IoNotFound),
            None,
            None,
            Some("destack.audio.stream.open".to_string()),
            None,
            "failed to resolve CoreAudio device UID".to_string(),
        ))
        .boxed()
    })?;
    let property_status = unsafe {
        AudioQueueSetProperty(
            queue,
            K_AUDIO_QUEUE_PROPERTY_CURRENT_DEVICE,
            &device_uid as *const CFStringRef as *const c_void,
            size_of::<CFStringRef>() as u32,
        )
    };
    unsafe {
        CFRelease(device_uid as CFTypeRef);
    }
    if property_status != K_NO_ERR {
        return Err(error(
            "destack.audio.stream.open",
            property_status,
            "failed to bind CoreAudio queue to requested device",
        ));
    }

    Ok(())
}

/// Compute one stream buffer size in bytes for one stream period.
#[cfg(target_os = "macos")]
pub(super) fn buffer_bytes(stream: &Arc<AudioStreamHostState>) -> RuntimeResult<u32> {
    Ok(stream
        .period_frames
        .max(MIN_STREAM_PERIOD_FRAMES)
        .saturating_mul(frame_bytes(stream.requested.format, stream.channels)? as u32)
        .max(stream.channels as u32))
}

/// Dispose one CoreAudio queue handle and release callback context ownership.
#[cfg(target_os = "macos")]
pub(super) fn dispose_queue_handle(queue_handle: CoreAudioQueueHandle) {
    unsafe {
        let _ = AudioQueueStop(queue_handle.queue, 1);
        let _ = AudioQueueDispose(queue_handle.queue, 1);

        // release one callback-owned strong reference from raw pointer storage
        Arc::decrement_strong_count(queue_handle.context_raw);
    }
}
