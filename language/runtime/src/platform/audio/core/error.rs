use std::sync::Arc;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::abi::NativeStringRef;
use crate::platform::audio::{
    AudioBackend, AudioDeviceDirection, AudioStreamConfig, AudioStreamOpenOptions,
    AudioStreamRequirementFlags, AudioStreamStateKind, AudioStreamTransferMode,
};
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::resource::ResourceKind;
use crate::platform::{PlatformError, resource};
use crate::runtime::BindingCallContext;

use super::constants::{
    AUDIO_DEVICE_RESOURCE_LABEL, AUDIO_EVENT_RESOURCE_LABEL, AUDIO_STREAM_RESOURCE_LABEL,
    KNOWN_STREAM_FLAGS_MASK, KNOWN_STREAM_REQUIREMENT_FLAGS_MASK,
};
use super::model::{
    AudioDeviceHostState, AudioEventStream, AudioStreamHostState, AudioStreamStateInner,
};

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

/// Build one ioBusy error.
pub(crate) fn audio_busy(operation: &'static str, message: impl Into<String>) -> Box<RuntimeError> {
    RuntimeError::from(PlatformError::io_with(
        Some(PlatformErrorCode::IoBusy),
        None,
        None,
        Some(operation.to_string()),
        None,
        message.into(),
    ))
    .boxed()
}

/// Build one audio-broken-pipe error.
pub(crate) fn audio_broken_pipe(
    operation: &'static str,
    message: impl Into<String>,
) -> Box<RuntimeError> {
    RuntimeError::from(PlatformError::io_with(
        Some(PlatformErrorCode::IoBrokenPipe),
        None,
        None,
        Some(operation.to_string()),
        None,
        message.into(),
    ))
    .boxed()
}

/// Validate that requested stream requirements are fully satisfied.
pub(crate) fn ensure_stream_requirements_satisfied(
    requested: AudioStreamRequirementFlags,
    satisfied: AudioStreamRequirementFlags,
    operation: &'static str,
) -> RuntimeResult<()> {
    let unsatisfied_requirements = requested.0 & !satisfied.0;
    if unsatisfied_requirements != 0 {
        return Err(RuntimeError::from(PlatformError::not_supported(format!(
            "{operation} unsatisfied stream requirements: 0x{unsatisfied_requirements:08x}",
        )))
        .boxed());
    }

    Ok(())
}

/// Build one stream-shutdown error from one stream state payload.
pub(crate) fn stream_shutdown_error(
    operation: &'static str,
    state: &AudioStreamStateInner,
) -> Box<RuntimeError> {
    if state.state_kind == AudioStreamStateKind::BackendDisconnected {
        let message = state
            .last_backend_message
            .as_deref()
            .unwrap_or("audio backend disconnected");
        return audio_broken_pipe(operation, message.to_string());
    }

    if state.state_kind == AudioStreamStateKind::DeviceLost {
        let message = state
            .last_backend_message
            .as_deref()
            .unwrap_or("audio device was lost");
        return audio_not_found(operation, message.to_string());
    }

    audio_not_found(operation, "stream has been closed")
}

/// Return whether one stream state is terminal for I/O operations.
pub(crate) fn stream_state_is_terminal(state: &AudioStreamStateInner) -> bool {
    if state.shutdown {
        return true;
    }

    matches!(
        state.state_kind,
        AudioStreamStateKind::DeviceLost | AudioStreamStateKind::BackendDisconnected
    )
}

/// Resolve one typed resource payload by kind and label.
fn resolve_resource_payload<T: Clone + 'static>(
    ctx: &BindingCallContext,
    resource_id: resource::ResourceId,
    resource_kind: ResourceKind,
    resource_label: &'static str,
) -> Option<T> {
    let resolved = ctx.worker().resources.with_entry(resource_id, |entry| {
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
pub(crate) fn resolve_device_host_state(
    ctx: &BindingCallContext,
    handle: resource::AudioDeviceHandle,
    operation: &'static str,
) -> RuntimeResult<Arc<AudioDeviceHostState>> {
    resolve_resource_payload::<Arc<AudioDeviceHostState>>(
        ctx,
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
pub(crate) fn resolve_stream_host_state(
    ctx: &BindingCallContext,
    handle: resource::AudioStreamHandle,
    operation: &'static str,
) -> RuntimeResult<Arc<AudioStreamHostState>> {
    resolve_resource_payload::<Arc<AudioStreamHostState>>(
        ctx,
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
pub(crate) fn resolve_event_stream(
    ctx: &BindingCallContext,
    handle: resource::AudioEventHandle,
    operation: &'static str,
) -> RuntimeResult<Arc<AudioEventStream>> {
    resolve_resource_payload::<Arc<AudioEventStream>>(
        ctx,
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

/// Validate one stream config payload.
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

/// Validate one stream open-options payload.
pub(crate) fn validate_stream_open_options(
    options: AudioStreamOpenOptions,
    _operation: &'static str,
) -> RuntimeResult<()> {
    // reject unknown stream-option flag bits
    let unknown_stream_flags = options.flags.0 & !KNOWN_STREAM_FLAGS_MASK;
    if unknown_stream_flags != 0 {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "options.flags",
            format!("options.flags contains unknown bits: 0x{unknown_stream_flags:08x}"),
        ))
        .boxed());
    }

    // reject unknown stream-requirement flag bits
    let unknown_requirements = options.requirements.0 & !KNOWN_STREAM_REQUIREMENT_FLAGS_MASK;
    if unknown_requirements != 0 {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "options.requirements",
            format!("options.requirements contains unknown bits: 0x{unknown_requirements:08x}"),
        ))
        .boxed());
    }

    Ok(())
}

/// Validate stream open options for one selected backend.
pub(crate) fn validate_stream_open_options_for_backend(
    options: AudioStreamOpenOptions,
    backend: AudioBackend,
    operation: &'static str,
) -> RuntimeResult<()> {
    let supported_flags = super::device::supported_backend_stream_flags(backend);
    let unsupported_flags = options.flags.0 & !supported_flags.0;
    if unsupported_flags != 0 {
        return Err(RuntimeError::from(PlatformError::not_supported(format!(
            "{operation} unsupported stream flags for backend {backend:?}: 0x{unsupported_flags:08x}"
        )))
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

#[cfg(test)]
mod tests {
    use crate::diagnostic::RuntimeError;
    use crate::platform::audio::AudioStreamStateKind;
    use crate::platform::audio::core::error::{stream_shutdown_error, stream_state_is_terminal};
    use crate::platform::audio::core::model::initial_stream_state;
    use crate::platform::diagnostic::PlatformErrorCode;

    /// Return one platform error code from one runtime error payload.
    fn platform_error_code(error: &RuntimeError) -> Option<PlatformErrorCode> {
        error.platform_error().map(|platform| platform.code)
    }

    /// Return one platform error message from one runtime error payload.
    fn platform_error_message(error: &RuntimeError) -> Option<String> {
        error
            .platform_error()
            .and_then(|platform| platform.message.clone())
    }

    #[test]
    fn test_stream_shutdown_error_uses_broken_pipe_for_backend_disconnect() {
        let mut state = initial_stream_state();
        state.shutdown = true;
        state.state_kind = AudioStreamStateKind::BackendDisconnected;
        state.last_backend_message = Some("backend transport dropped".to_string());

        let error = stream_shutdown_error("destack.audio.stream.read", &state);
        let code = platform_error_code(error.as_ref());
        assert_eq!(code, Some(PlatformErrorCode::IoBrokenPipe));

        let message = platform_error_message(error.as_ref());
        assert_eq!(message.as_deref(), Some("backend transport dropped"));
    }

    #[test]
    fn test_stream_shutdown_error_uses_not_found_for_closed_stream() {
        let mut state = initial_stream_state();
        state.shutdown = true;

        let error = stream_shutdown_error("destack.audio.stream.read", &state);
        let code = platform_error_code(error.as_ref());
        assert_eq!(code, Some(PlatformErrorCode::IoNotFound));
    }

    #[test]
    fn test_stream_shutdown_error_uses_not_found_for_device_lost() {
        let mut state = initial_stream_state();
        state.state_kind = AudioStreamStateKind::DeviceLost;

        let error = stream_shutdown_error("destack.audio.stream.read", &state);
        let code = platform_error_code(error.as_ref());
        assert_eq!(code, Some(PlatformErrorCode::IoNotFound));
    }

    #[test]
    fn test_stream_shutdown_error_keeps_device_lost_backend_message() {
        let mut state = initial_stream_state();
        state.state_kind = AudioStreamStateKind::DeviceLost;
        state.last_backend_message = Some("ASIO driver requested reset".to_string());

        let error = stream_shutdown_error("destack.audio.stream.read", &state);
        let message = platform_error_message(error.as_ref());
        assert_eq!(message.as_deref(), Some("ASIO driver requested reset"));
    }

    #[test]
    fn test_stream_state_is_terminal_for_device_lost_without_shutdown() {
        let mut state = initial_stream_state();
        state.state_kind = AudioStreamStateKind::DeviceLost;
        assert!(stream_state_is_terminal(&state));
    }
}
