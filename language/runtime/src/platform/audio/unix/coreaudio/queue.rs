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

/// Compute one stream buffer size in bytes for one binding period.
#[cfg(target_os = "macos")]
pub(super) fn buffer_bytes(binding: &Arc<audio_core::AudioStreamBinding>) -> RuntimeResult<u32> {
    Ok(binding
        .period_frames
        .max(audio_core::MIN_STREAM_PERIOD_FRAMES)
        .saturating_mul(audio_core::frame_bytes(binding.requested.format, binding.channels)? as u32)
        .max(binding.channels as u32))
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
