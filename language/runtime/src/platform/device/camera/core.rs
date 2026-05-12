#![cfg_attr(windows, allow(dead_code))]

#[cfg(any(target_os = "android", target_os = "macos", windows))]
use std::path::PathBuf;
use std::sync::Arc;
#[cfg(any(target_os = "android", target_os = "macos", windows))]
use std::sync::atomic::{AtomicU64, Ordering};
#[cfg(target_os = "windows")]
use std::time::Duration;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::abi::NativeSlice;
#[cfg(target_os = "windows")]
use crate::platform::core::BoundedQueue;
#[cfg(any(target_os = "android", target_os = "linux"))]
use crate::platform::device::CameraPhoto;
use crate::platform::device::{
    CameraAttachedEvent, CameraControlCapabilitiesValue, CameraDetachedEvent,
    CameraDeviceDescriptor, CameraDeviceDescriptorValue, CameraFloatControlRange, CameraFrame,
    CameraFrameValue, CameraPhotoCapabilities, CameraPhotoFlashMode, CameraPhotoOptions,
    CameraPhotoSettings, CameraPhotoState, CameraRecordingCapabilities,
    CameraRecordingCapabilitiesValue, CameraRecordingOptions, CameraRecordingOptionsValue,
    CameraRecordingState, CameraRedEyeReduction, CameraStreamCapability,
    CameraStreamCapabilityValue, CameraStreamConfigValue, CameraWatchEvent,
    CameraWatchEventMetadata,
};
#[cfg(any(target_os = "android", target_os = "macos", windows))]
use crate::platform::device::{
    CameraAudioCodec, CameraRecording, CameraRecordingContainer, CameraVideoCodec,
};
#[cfg(any(target_os = "linux", target_os = "macos", windows))]
use crate::platform::device::{CameraColorSpace, CameraDynamicRange};
#[cfg(any(
    target_os = "android",
    target_os = "linux",
    target_os = "macos",
    windows
))]
use crate::platform::device::{
    CameraExposureMode, CameraFocusMode, CameraStabilizationMode, CameraTorchMode,
    CameraWhiteBalanceMode,
};
#[cfg(any(target_os = "linux", target_os = "macos", windows))]
use crate::platform::device::{
    CameraFrameMetadataValue, CameraPixelFormat, CameraPlaneLayoutValue,
};
#[cfg(any(target_os = "macos", windows))]
use crate::platform::device::{CameraPixelFormatDescriptorValue, CameraPixelFormatFamily};
use crate::platform::diagnostic::PlatformErrorCode;
#[cfg(any(target_os = "android", target_os = "macos", windows))]
use crate::platform::fs::{abi_generated::OsPathValue, core as core_fs};
use crate::platform::resource::ResourceKind;
use crate::platform::{NativeAbiCodec, PlatformError, core as core_platform, resource};
use crate::runtime::BindingCallContext;
use parking_lot::Mutex;

/// Resource-table label for one camera device handle.
pub(super) const CAMERA_DEVICE_RESOURCE_LABEL: &str = "device.camera.device";

/// Resource-table label for one camera stream handle.
pub(super) const CAMERA_STREAM_RESOURCE_LABEL: &str = "device.camera.stream";

/// Resource-table label for one camera watch handle.
pub(super) const CAMERA_WATCH_RESOURCE_LABEL: &str = "device.camera.watch";

/// Default recording file-name prefix.
#[cfg(any(target_os = "android", target_os = "macos", windows))]
const CAMERA_RECORDING_FILE_PREFIX: &str = "destack-camera-recording";

/// Monotonic suffix for generated recording file names.
#[cfg(any(target_os = "android", target_os = "macos", windows))]
static CAMERA_RECORDING_PATH_SEQUENCE: AtomicU64 = AtomicU64::new(0);

/// Mutable runtime state for one opened camera stream.
#[cfg(any(target_os = "linux", target_os = "macos", windows))]
#[derive(Debug, Clone, Copy)]
pub(crate) struct CameraStreamState {
    /// Whether the stream is started.
    pub(super) is_started: bool,
    /// The next frame sequence number.
    pub(super) next_sequence: u64,
}

/// Monotonic event sequencing for one camera watch queue.
#[derive(Debug, Clone, Copy)]
pub(super) struct CameraWatchEventState {
    /// The next event sequence number to emit.
    pub(super) next_sequence: u64,
}

/// Mutable recording state for one opened camera stream.
#[derive(Debug, Clone, Default)]
pub(crate) struct CameraRecordingRuntimeState {
    /// Whether recording is currently active.
    pub(super) is_active: bool,
    /// Whether recording is currently paused.
    pub(super) is_paused: bool,
    /// Active recording options when recording is live.
    pub(super) options: Option<CameraRecordingOptionsValue>,
    /// Active recording start timestamp when recording is live.
    pub(super) started_timestamp_ns: Option<u64>,
}

/// Build one invalid camera handle error.
pub(super) fn invalid_camera_handle(
    operation: &'static str,
    kind: &'static str,
) -> Box<RuntimeError> {
    core_platform::io_operation_error(
        operation,
        Some(PlatformErrorCode::InvalidArgumentValue),
        format!("unknown camera {kind} handle"),
    )
}

/// Build one invalid camera watch-handle error.
pub(super) fn invalid_camera_watch_handle(operation: &'static str) -> Box<RuntimeError> {
    core_platform::io_operation_error(
        operation,
        Some(PlatformErrorCode::InvalidArgumentValue),
        "unknown camera watch handle",
    )
}

/// Build one would-block error for one empty camera queue.
#[cfg(target_os = "windows")]
pub(super) fn empty_camera_queue_error(operation: &'static str) -> Box<RuntimeError> {
    core_platform::io_operation_error(
        operation,
        Some(PlatformErrorCode::IoWouldBlock),
        "camera frame is not ready",
    )
}

/// Build one not-supported error for one camera binding.
pub(super) fn camera_not_supported(operation: &'static str) -> Box<RuntimeError> {
    RuntimeError::from(PlatformError::not_supported(operation)).boxed()
}

/// Build one invalid-state error for one camera binding.
#[cfg(any(target_os = "android", target_os = "macos", windows))]
pub(crate) fn camera_invalid_state(message: impl Into<String>) -> Box<RuntimeError> {
    RuntimeError::from(PlatformError::invalid_argument(message)).boxed()
}

/// Resolve one typed camera device resource from the resource table.
pub(super) fn camera_device_resource<T: Send + Sync + 'static>(
    binding: &BindingCallContext,
    handle: resource::CameraDeviceHandle,
    operation: &'static str,
) -> RuntimeResult<Arc<T>> {
    let resolved = binding.worker().resources.with_entry(handle.0, |entry| {
        if entry.kind != ResourceKind::CameraDevice {
            return None;
        }

        entry
            .payload
            .as_ref()
            .and_then(|payload| payload.downcast_ref::<Arc<T>>())
            .map(Arc::clone)
    });

    resolved
        .flatten()
        .ok_or_else(|| invalid_camera_handle(operation, "device"))
}

/// Resolve one typed camera stream resource from the resource table.
pub(super) fn camera_stream_resource<T: Send + Sync + 'static>(
    binding: &BindingCallContext,
    handle: resource::CameraStreamHandle,
    operation: &'static str,
) -> RuntimeResult<Arc<T>> {
    let resolved = binding.worker().resources.with_entry(handle.0, |entry| {
        if entry.kind != ResourceKind::CameraStream {
            return None;
        }

        entry
            .payload
            .as_ref()
            .and_then(|payload| payload.downcast_ref::<Arc<T>>())
            .map(Arc::clone)
    });

    resolved
        .flatten()
        .ok_or_else(|| invalid_camera_handle(operation, "stream"))
}

/// Close one camera device resource.
pub(super) fn close_camera_device_resource(
    binding: &BindingCallContext,
    handle: resource::CameraDeviceHandle,
    operation: &'static str,
) -> RuntimeResult<()> {
    let kind = binding
        .worker()
        .resources
        .with_entry(handle.0, |entry| entry.kind)
        .ok_or_else(|| invalid_camera_handle(operation, "device"))?;
    if kind != ResourceKind::CameraDevice {
        return Err(invalid_camera_handle(operation, "device"));
    }

    if !binding.worker().resources.remove_and_finalize(
        binding.world(),
        handle.0,
        Some(binding.engine()),
    ) {
        return Err(invalid_camera_handle(operation, "device"));
    }

    Ok(())
}

/// Close one camera stream resource.
pub(super) fn close_camera_stream_resource(
    binding: &BindingCallContext,
    handle: resource::CameraStreamHandle,
    operation: &'static str,
) -> RuntimeResult<()> {
    let kind = binding
        .worker()
        .resources
        .with_entry(handle.0, |entry| entry.kind)
        .ok_or_else(|| invalid_camera_handle(operation, "stream"))?;
    if kind != ResourceKind::CameraStream {
        return Err(invalid_camera_handle(operation, "stream"));
    }

    if !binding.worker().resources.remove_and_finalize(
        binding.world(),
        handle.0,
        Some(binding.engine()),
    ) {
        return Err(invalid_camera_handle(operation, "stream"));
    }

    Ok(())
}

/// Resolve one typed camera-watch payload from the resource table.
pub(super) fn camera_watch_payload<T: Clone + 'static>(
    binding: &BindingCallContext,
    handle: resource::CameraWatchHandle,
    operation: &'static str,
) -> RuntimeResult<T> {
    let resolved = binding.worker().resources.with_entry(handle.0, |entry| {
        if entry.kind != ResourceKind::CameraWatch {
            return None;
        }

        entry.payload_cloned::<T>()
    });

    resolved
        .flatten()
        .ok_or_else(|| invalid_camera_watch_handle(operation))
}

/// Close one camera watch resource.
pub(super) fn close_camera_watch_resource(
    binding: &BindingCallContext,
    handle: resource::CameraWatchHandle,
    operation: &'static str,
) -> RuntimeResult<()> {
    let kind = binding
        .worker()
        .resources
        .with_entry(handle.0, |entry| entry.kind)
        .ok_or_else(|| invalid_camera_watch_handle(operation))?;
    if kind != ResourceKind::CameraWatch {
        return Err(invalid_camera_watch_handle(operation));
    }

    if !binding.worker().resources.remove_and_finalize(
        binding.world(),
        handle.0,
        Some(binding.engine()),
    ) {
        return Err(invalid_camera_watch_handle(operation));
    }

    Ok(())
}

/// Build one binding-visible camera descriptor from one stored snapshot.
pub(super) fn camera_descriptor_from_value(
    binding: &BindingCallContext,
    descriptor: CameraDeviceDescriptorValue,
) -> CameraDeviceDescriptor {
    CameraDeviceDescriptor::from_value(binding, descriptor)
}

/// Store one camera capability list on the binding heap.
pub(super) fn store_camera_capabilities(
    binding: &BindingCallContext,
    capabilities: &[CameraStreamCapabilityValue],
) -> NativeSlice<CameraStreamCapability> {
    let capabilities = capabilities
        .iter()
        .cloned()
        .map(|capability| CameraStreamCapability::from_value(binding, capability))
        .collect();

    binding.store_slice(capabilities)
}

/// Build one binding-visible frame payload.
pub(super) fn camera_frame_from_value(
    binding: &BindingCallContext,
    frame: CameraFrameValue,
) -> CameraFrame {
    CameraFrame::from_value(binding, frame)
}

/// Build one still-photo options snapshot from one stream configuration.
pub(super) fn camera_photo_options_from_config(
    config: &CameraStreamConfigValue,
) -> CameraPhotoOptions {
    CameraPhotoOptions {
        width: config.width,
        height: config.height,
        pixel_format: config.pixel_format,
        color_space: config.color_space,
        dynamic_range: config.dynamic_range,
    }
}

/// Build one still-photo state snapshot for one stream configuration.
pub(super) fn camera_photo_state_from_config(config: &CameraStreamConfigValue) -> CameraPhotoState {
    CameraPhotoState {
        options: Some(camera_photo_options_from_config(config)),
        quality: None,
        flash_mode: None,
        red_eye_reduction_enabled: None,
    }
}

/// Build one still-photo capability descriptor for one stream configuration.
pub(super) fn camera_photo_capabilities_from_config(
    binding: &BindingCallContext,
    config: &CameraStreamConfigValue,
) -> CameraPhotoCapabilities {
    let options = vec![camera_photo_options_from_config(config)];

    CameraPhotoCapabilities {
        options: binding.store_slice(options),
        quality_range: None::<CameraFloatControlRange>,
        flash_modes: None::<NativeSlice<CameraPhotoFlashMode>>,
        red_eye_reduction: None::<CameraRedEyeReduction>,
    }
}

/// Validate one still-photo settings payload against the current stream configuration.
pub(super) fn validate_camera_photo_settings(
    settings: &CameraPhotoSettings,
    config: &CameraStreamConfigValue,
    operation: &'static str,
) -> RuntimeResult<()> {
    // option override
    if let Some(options) = settings.options {
        let expected_options = camera_photo_options_from_config(config);
        if options != expected_options {
            return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "settings.options",
                "photo options must match the opened stream configuration",
            ))
            .boxed());
        }
    }

    // unsupported quality
    if settings.quality.is_some() {
        return Err(camera_not_supported(operation));
    }

    // unsupported flash
    if settings.flash_mode.is_some() {
        return Err(camera_not_supported(operation));
    }

    // unsupported red eye reduction
    if settings.red_eye_reduction_enabled.is_some() {
        return Err(camera_not_supported(operation));
    }

    Ok(())
}

/// Build one still-photo payload from one frame payload.
#[cfg(any(target_os = "android", target_os = "linux"))]
pub(super) fn camera_photo_from_frame_value(
    binding: &BindingCallContext,
    frame: CameraFrameValue,
) -> CameraPhoto {
    CameraPhoto {
        timestamp_ns: frame.timestamp_ns,
        width: frame.width,
        height: frame.height,
        pixel_format: frame.pixel_format,
        color_space: frame.color_space,
        dynamic_range: frame.dynamic_range,
        planes: binding.store_slice(frame.planes),
        metadata: frame.metadata,
        bytes: binding.store_slice(frame.bytes),
    }
}

/// Build one recording-state snapshot from one runtime recording state.
pub(super) fn camera_recording_state_from_runtime(
    binding: &BindingCallContext,
    state: &CameraRecordingRuntimeState,
) -> CameraRecordingState {
    let options = state
        .options
        .clone()
        .map(|options| CameraRecordingOptions::from_value(binding, options));

    CameraRecordingState {
        active: state.is_active,
        paused: state.is_paused,
        options,
        started_timestamp_ns: state.started_timestamp_ns,
    }
}

/// Store one recording-capability descriptor on the binding heap.
pub(super) fn camera_recording_capabilities_from_value(
    binding: &BindingCallContext,
    capabilities: &CameraRecordingCapabilitiesValue,
) -> CameraRecordingCapabilities {
    CameraRecordingCapabilities {
        containers: binding.store_slice(capabilities.containers.clone()),
        video_codecs: binding.store_slice(capabilities.video_codecs.clone()),
        audio_supported: capabilities.audio_supported,
        audio_codecs: capabilities
            .audio_codecs
            .clone()
            .map(|audio_codecs| binding.store_slice(audio_codecs)),
        pause_supported: capabilities.pause_supported,
        maximum_video_bit_rate: capabilities.maximum_video_bit_rate,
        maximum_audio_bit_rate: capabilities.maximum_audio_bit_rate,
    }
}

/// Build one recording result payload from one finalized output path.
#[cfg(any(
    target_os = "android",
    target_os = "linux",
    target_os = "macos",
    windows
))]
pub(super) fn camera_recording_from_path(
    binding: &BindingCallContext,
    path: &std::path::Path,
    container: CameraRecordingContainer,
    video_codec: Option<CameraVideoCodec>,
    audio_codec: Option<CameraAudioCodec>,
    config: &CameraStreamConfigValue,
    duration_ns: Option<u64>,
    size_bytes: Option<u64>,
) -> CameraRecording {
    let path = core_fs::os_path_from_path(binding, path);

    CameraRecording {
        path,
        container,
        video_codec,
        audio_codec,
        width: config.width,
        height: config.height,
        frame_rate_milli_hz: Some(config.frame_rate_milli_hz),
        duration_ns,
        size_bytes,
    }
}

/// Decode one binding-visible output path into one host path.
#[cfg(any(
    target_os = "android",
    target_os = "linux",
    target_os = "macos",
    windows
))]
fn path_buf_from_os_path_value(value: OsPathValue, label: &'static str) -> RuntimeResult<PathBuf> {
    #[cfg(unix)]
    {
        use std::ffi::OsString;
        use std::os::unix::ffi::OsStringExt;

        let path = match value {
            OsPathValue::OsPathBytes(value) => PathBuf::from(OsString::from_vec(value.bytes.0)),
            OsPathValue::OsPathUtf16(value) => {
                let string = String::from_utf16(&value.utf16.0).map_err(|_| {
                    RuntimeError::from(PlatformError::invalid_argument_value(
                        label,
                        "output path utf16 is not valid",
                    ))
                    .boxed()
                })?;

                PathBuf::from(string)
            }
        };

        Ok(path)
    }

    #[cfg(windows)]
    {
        use std::ffi::OsString;
        use std::os::windows::ffi::OsStringExt;

        let path = match value {
            OsPathValue::OsPathBytes(value) => {
                let string = String::from_utf8(value.bytes.0).map_err(|_| {
                    RuntimeError::from(PlatformError::invalid_argument_value(
                        label,
                        "output path bytes are not valid utf8",
                    ))
                    .boxed()
                })?;

                PathBuf::from(string)
            }
            OsPathValue::OsPathUtf16(value) => PathBuf::from(OsString::from_wide(&value.utf16.0)),
        };

        Ok(path)
    }
}

/// Build one validated output path for one new recording.
#[cfg(any(
    target_os = "android",
    target_os = "linux",
    target_os = "macos",
    windows
))]
pub(super) fn camera_recording_output_path(
    options: &CameraRecordingOptionsValue,
    extension: &str,
    operation: &'static str,
) -> RuntimeResult<PathBuf> {
    let path = if let Some(path) = options.output_path.clone() {
        path_buf_from_os_path_value(path, "options.output_path")?
    } else {
        unique_camera_recording_path(extension)
    };
    let path = if path.is_absolute() {
        path
    } else {
        std::env::current_dir()
            .map_err(|error| {
                core_platform::io_operation_error(
                    operation,
                    None,
                    format!("failed to resolve current directory: {error}"),
                )
            })?
            .join(path)
    };

    // output path validation
    if path.as_os_str().is_empty() {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "options.output_path",
            "recording output path must not be empty",
        ))
        .boxed());
    }

    // parent path
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
        && !parent.exists()
    {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "options.output_path",
            "recording output path parent directory does not exist",
        ))
        .boxed());
    }

    // existing path
    if path.exists() {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "options.output_path",
            "recording output path already exists",
        ))
        .boxed());
    }

    Ok(path)
}

/// Build one unique temporary output path for one new recording.
#[cfg(any(
    target_os = "android",
    target_os = "linux",
    target_os = "macos",
    windows
))]
fn unique_camera_recording_path(extension: &str) -> PathBuf {
    let sequence = CAMERA_RECORDING_PATH_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let process_id = std::process::id();
    let timestamp_ns = core_platform::monotonic_now_ns();
    let file_name = format!(
        "{CAMERA_RECORDING_FILE_PREFIX}-{process_id}-{timestamp_ns}-{sequence}.{extension}"
    );

    std::env::temp_dir().join(file_name)
}

/// Build one camera-attached watch event.
pub(super) fn attached_watch_event(
    binding: &BindingCallContext,
    state: &Mutex<CameraWatchEventState>,
    descriptor: CameraDeviceDescriptorValue,
) -> CameraWatchEvent {
    CameraWatchEvent::CameraAttachedEvent(CameraAttachedEvent {
        kind: binding.store_string("attached"),
        metadata: next_watch_metadata(binding, state, descriptor),
    })
}

/// Build one camera-detached watch event.
pub(super) fn detached_watch_event(
    binding: &BindingCallContext,
    state: &Mutex<CameraWatchEventState>,
    descriptor: CameraDeviceDescriptorValue,
) -> CameraWatchEvent {
    CameraWatchEvent::CameraDetachedEvent(CameraDetachedEvent {
        kind: binding.store_string("detached"),
        metadata: next_watch_metadata(binding, state, descriptor),
    })
}

/// Return one fresh camera watch-event metadata payload.
fn next_watch_metadata(
    binding: &BindingCallContext,
    state: &Mutex<CameraWatchEventState>,
    descriptor: CameraDeviceDescriptorValue,
) -> CameraWatchEventMetadata {
    let mut state = state.lock();
    let sequence = state.next_sequence;
    state.next_sequence = state.next_sequence.saturating_add(1);

    CameraWatchEventMetadata {
        timestamp_ns: core_platform::monotonic_now_ns(),
        sequence,
        device: camera_descriptor_from_value(binding, descriptor),
    }
}

/// Pop one frame from one bounded queue with one blocking timeout.
#[cfg(target_os = "windows")]
pub(super) fn read_camera_frame_queue(
    queue: &BoundedQueue<CameraFrameValue>,
    timeout_ns: u64,
    operation: &'static str,
) -> RuntimeResult<CameraFrameValue> {
    queue.pop_with_timeout_or_else(Duration::from_nanos(timeout_ns), || {
        Err(empty_camera_queue_error(operation))
    })
}

/// Pop one frame from one bounded queue without blocking.
#[cfg(target_os = "windows")]
pub(super) fn try_read_camera_frame_queue(
    queue: &BoundedQueue<CameraFrameValue>,
    operation: &'static str,
) -> RuntimeResult<CameraFrameValue> {
    queue.try_pop_or_else(|| Err(empty_camera_queue_error(operation)))
}

/// Increment one stream sequence counter.
#[cfg(target_os = "windows")]
pub(super) fn next_camera_sequence(state: &Mutex<CameraStreamState>) -> u64 {
    let mut state = state.lock();
    let sequence = state.next_sequence;
    state.next_sequence = state.next_sequence.saturating_add(1);

    sequence
}

/// Return one normalized BGRA pixel-format descriptor.
#[cfg(any(target_os = "macos", windows))]
pub(super) fn bgra_camera_pixel_format_descriptor() -> CameraPixelFormatDescriptorValue {
    CameraPixelFormatDescriptorValue {
        format: CameraPixelFormat::Bgra8,
        family: CameraPixelFormatFamily::PackedRgb,
        compressed: false,
    }
}

/// Return one empty camera metadata payload.
#[cfg(any(target_os = "linux", target_os = "macos", windows))]
pub(super) fn empty_camera_metadata() -> CameraFrameMetadataValue {
    CameraFrameMetadataValue {
        exposure_time_ns: None,
        sensor_iso: None,
        white_balance_kelvin: None,
        focus_distance_diopters: None,
        zoom_ratio: None,
    }
}

/// Build one frame plane layout list for one delivered frame.
#[cfg(any(target_os = "linux", target_os = "macos", windows))]
pub(super) fn frame_plane_layouts(
    config: &CameraStreamConfigValue,
    bytes_len: usize,
) -> Vec<CameraPlaneLayoutValue> {
    let plane = match config.pixel_format.format {
        CameraPixelFormat::Bgra8 | CameraPixelFormat::Rgba8 => CameraPlaneLayoutValue {
            offset_bytes: 0,
            length_bytes: bytes_len as u32,
            row_stride_bytes: config.width.saturating_mul(4),
            pixel_stride_bytes: 4,
        },
        CameraPixelFormat::Yuv420 => CameraPlaneLayoutValue {
            offset_bytes: 0,
            length_bytes: bytes_len as u32,
            row_stride_bytes: config.width,
            pixel_stride_bytes: 1,
        },
        CameraPixelFormat::Jpeg => CameraPlaneLayoutValue {
            offset_bytes: 0,
            length_bytes: bytes_len as u32,
            row_stride_bytes: 0,
            pixel_stride_bytes: 0,
        },
    };

    vec![plane]
}

/// Build one default control-mode set for one stream capability.
#[cfg(any(
    target_os = "android",
    target_os = "linux",
    target_os = "macos",
    windows
))]
pub(crate) fn empty_camera_control_modes() -> CameraControlModes {
    CameraControlModes {
        exposure_modes: Vec::new(),
        white_balance_modes: Vec::new(),
        focus_modes: Vec::new(),
        stabilization_modes: Vec::new(),
        torch_modes: Vec::new(),
    }
}

/// Build one baseline stream capability from one stream configuration.
#[cfg(any(target_os = "linux", target_os = "macos", windows))]
pub(super) fn basic_camera_stream_capability(
    config: &CameraStreamConfigValue,
    control_modes: &CameraControlModes,
) -> CameraStreamCapabilityValue {
    let controls = camera_control_capabilities(control_modes);
    let color_space = config.color_space.unwrap_or(CameraColorSpace::Unknown);
    let dynamic_range = config.dynamic_range.unwrap_or(CameraDynamicRange::Standard);

    CameraStreamCapabilityValue {
        config: *config,
        minimum_frame_rate_milli_hz: config.frame_rate_milli_hz,
        maximum_frame_rate_milli_hz: config.frame_rate_milli_hz,
        color_spaces: vec![color_space],
        dynamic_ranges: vec![dynamic_range],
        controls,
    }
}

/// Build one optional supported-mode list from one backend mode vector.
#[cfg(any(
    target_os = "android",
    target_os = "linux",
    target_os = "macos",
    windows
))]
fn optional_supported_modes<T>(modes: &[T]) -> Option<Vec<T>>
where
    T: Clone,
{
    if modes.is_empty() {
        return None;
    }

    Some(modes.to_vec())
}

/// Build one grouped control-capability snapshot from supported control modes.
#[cfg(any(
    target_os = "android",
    target_os = "linux",
    target_os = "macos",
    windows
))]
pub(super) fn camera_control_capabilities(
    control_modes: &CameraControlModes,
) -> CameraControlCapabilitiesValue {
    CameraControlCapabilitiesValue {
        exposure_modes: optional_supported_modes(&control_modes.exposure_modes),
        white_balance_modes: optional_supported_modes(&control_modes.white_balance_modes),
        focus_modes: optional_supported_modes(&control_modes.focus_modes),
        stabilization_modes: optional_supported_modes(&control_modes.stabilization_modes),
        torch_modes: optional_supported_modes(&control_modes.torch_modes),
        exposure_compensation_range: None,
        exposure_time_range: None,
        sensor_iso_range: None,
        white_balance_range: None,
        focus_distance_range: None,
        brightness_range: None,
        contrast_range: None,
        saturation_range: None,
        sharpness_range: None,
        pan_range: None,
        tilt_range: None,
        zoom_ratio_range: None,
    }
}

/// Supported control-mode lists for one camera backend.
#[derive(Debug, Clone)]
#[cfg(any(
    target_os = "android",
    target_os = "linux",
    target_os = "macos",
    windows
))]
pub(super) struct CameraControlModes {
    /// Supported exposure modes.
    pub(super) exposure_modes: Vec<CameraExposureMode>,
    /// Supported white-balance modes.
    pub(super) white_balance_modes: Vec<CameraWhiteBalanceMode>,
    /// Supported focus modes.
    pub(super) focus_modes: Vec<CameraFocusMode>,
    /// Supported stabilization modes.
    pub(super) stabilization_modes: Vec<CameraStabilizationMode>,
    /// Supported torch modes.
    pub(super) torch_modes: Vec<CameraTorchMode>,
}
