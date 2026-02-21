use super::*;

/// Build one audio-not-found error.
pub(crate) fn audio_not_found(
    operation: &'static str,
    message: impl Into<String>,
) -> Box<RuntimeError> {
    RuntimeError::from(PlatformError::io_with(
        Some(PlatformErrorCode::IoNotFound),
        None,
        None,
        Some(operation.to_string()),
        None,
        message.into(),
    ))
    .boxed()
}

/// Build one ioWouldBlock error.
pub(crate) fn audio_would_block(
    operation: &'static str,
    message: impl Into<String>,
) -> Box<RuntimeError> {
    RuntimeError::from(PlatformError::io_with(
        Some(PlatformErrorCode::IoWouldBlock),
        None,
        None,
        Some(operation.to_string()),
        None,
        message.into(),
    ))
    .boxed()
}

/// Resolve one typed resource payload by kind and label.
fn resolve_resource_payload<T: Clone + 'static>(
    context: &BindingCallContext,
    resource_id: resource::ResourceId,
    resource_kind: ResourceKind,
    resource_label: &'static str,
) -> Option<T> {
    let resolved = context
        .runtime()
        .resources
        .with_entry(resource_id, |entry| {
            if entry.kind != resource_kind {
                return None;
            }

            if entry.label.as_deref() != Some(resource_label) {
                return None;
            }

            entry
                .payload
                .as_ref()
                .and_then(|payload| payload.downcast_ref::<T>())
                .cloned()
        });

    resolved.flatten()
}

/// Resolve one opened device handle.
pub(crate) fn resolve_device_binding(
    context: &BindingCallContext,
    handle: resource::AudioDeviceHandle,
    operation: &'static str,
) -> RuntimeResult<Arc<AudioDeviceBinding>> {
    resolve_resource_payload::<Arc<AudioDeviceBinding>>(
        context,
        handle.0,
        ResourceKind::AudioDevice,
        AUDIO_DEVICE_RESOURCE_LABEL,
    )
    .ok_or_else(|| {
        audio_not_found(
            operation,
            format!("unknown audio device handle {}", handle.0.0),
        )
    })
}

/// Resolve one opened stream handle.
pub(crate) fn resolve_stream_binding(
    context: &BindingCallContext,
    handle: resource::AudioStreamHandle,
    operation: &'static str,
) -> RuntimeResult<Arc<AudioStreamBinding>> {
    resolve_resource_payload::<Arc<AudioStreamBinding>>(
        context,
        handle.0,
        ResourceKind::AudioStream,
        AUDIO_STREAM_RESOURCE_LABEL,
    )
    .ok_or_else(|| {
        audio_not_found(
            operation,
            format!("unknown audio stream handle {}", handle.0.0),
        )
    })
}

/// Resolve one opened event handle.
pub(crate) fn resolve_event_binding(
    context: &BindingCallContext,
    handle: resource::AudioEventHandle,
    operation: &'static str,
) -> RuntimeResult<Arc<Mutex<AudioEventBinding>>> {
    resolve_resource_payload::<Arc<Mutex<AudioEventBinding>>>(
        context,
        handle.0,
        ResourceKind::AudioEvent,
        AUDIO_EVENT_RESOURCE_LABEL,
    )
    .ok_or_else(|| {
        audio_not_found(
            operation,
            format!("unknown audio event handle {}", handle.0.0),
        )
    })
}

/// Read one utf8 string argument from one native reference.
pub(crate) fn read_utf8(value: NativeStringRef, field: &'static str) -> RuntimeResult<String> {
    let text = unsafe { value.as_str() }.map_err(|_| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            field,
            "expected valid utf8 string",
        ))
        .boxed()
    })?;
    Ok(text.to_string())
}

/// Validate a stream config payload.
pub(crate) fn validate_stream_config(config: AudioStreamConfig) -> RuntimeResult<()> {
    if config.sample_rate == 0 {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "config.sampleRate",
            "config.sampleRate must be greater than zero",
        ))
        .boxed());
    }

    if config.channels == 0 {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "config.channels",
            "config.channels must be greater than zero",
        ))
        .boxed());
    }

    if config.transfer_mode != AudioStreamTransferMode::Push {
        return Err(RuntimeError::from(PlatformError::not_supported(
            "destack.audio.stream.open transfer mode",
        ))
        .boxed());
    }

    Ok(())
}

/// Validate one stream handle direction supports playback writes.
pub(crate) fn ensure_playback_direction(
    direction: AudioDeviceDirection,
    operation: &'static str,
) -> RuntimeResult<()> {
    if direction == AudioDeviceDirection::Capture || direction == AudioDeviceDirection::Loopback {
        return Err(RuntimeError::from(PlatformError::not_supported(operation)).boxed());
    }

    Ok(())
}

/// Validate one stream handle direction supports capture reads.
pub(crate) fn ensure_capture_direction(
    direction: AudioDeviceDirection,
    operation: &'static str,
) -> RuntimeResult<()> {
    if direction == AudioDeviceDirection::Playback {
        return Err(RuntimeError::from(PlatformError::not_supported(operation)).boxed());
    }

    Ok(())
}

/// Validate one stream capability lane is supported.
pub(crate) fn ensure_stream_capability(
    is_supported: bool,
    operation: &'static str,
) -> RuntimeResult<()> {
    if !is_supported {
        return Err(RuntimeError::from(PlatformError::not_supported(operation)).boxed());
    }

    Ok(())
}
