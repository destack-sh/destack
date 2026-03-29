pub(super) use std::path::PathBuf;
pub(super) use std::sync::Arc;

pub(super) use crate::diagnostic::{RuntimeError, RuntimeResult};
pub(super) use crate::host::android::abi::camera::ffi::{
    destack_host_android_camera_device_close, destack_host_android_camera_device_list,
    destack_host_android_camera_device_open,
    destack_host_android_camera_device_stream_capability_list,
    destack_host_android_camera_device_stream_config_list,
    destack_host_android_camera_stream_close, destack_host_android_camera_stream_config,
    destack_host_android_camera_stream_get_f64, destack_host_android_camera_stream_get_range_f64,
    destack_host_android_camera_stream_get_range_u32,
    destack_host_android_camera_stream_get_range_u64, destack_host_android_camera_stream_get_u32,
    destack_host_android_camera_stream_get_u64, destack_host_android_camera_stream_open,
    destack_host_android_camera_stream_pause_recording, destack_host_android_camera_stream_read,
    destack_host_android_camera_stream_recording_capabilities,
    destack_host_android_camera_stream_resume_recording,
    destack_host_android_camera_stream_set_f64, destack_host_android_camera_stream_set_u32,
    destack_host_android_camera_stream_set_u64, destack_host_android_camera_stream_start,
    destack_host_android_camera_stream_start_recording, destack_host_android_camera_stream_stop,
    destack_host_android_camera_stream_stop_recording,
    destack_host_android_camera_stream_take_photo, destack_host_android_camera_stream_try_read,
};
pub(super) use crate::host::android::abi::camera::types::{
    AndroidHostCameraDeviceDescriptorHeader, AndroidHostCameraFrameHeader,
    AndroidHostCameraRecordingCapabilitiesHeader, AndroidHostCameraRecordingOptionsHeader,
    AndroidHostCameraStreamCapabilityHeader, AndroidHostCameraStreamConfigHeader,
};
pub(super) use crate::host::core::HostStatus;
pub(super) use crate::platform::abi::{NativeSlice, NativeStringRef};
pub(super) use crate::platform::core::android::{
    checked_u32_length, host_session_id, host_status_result, invalid_data,
};
pub(super) use crate::platform::core::{
    self as core_platform, NativeAbiCodec, decode_optional_string, decode_required_string,
};
pub(super) use crate::platform::device::camera::core::{
    CAMERA_DEVICE_RESOURCE_LABEL, CAMERA_STREAM_RESOURCE_LABEL, CAMERA_WATCH_RESOURCE_LABEL,
    CameraControlModes, CameraRecordingRuntimeState, CameraWatchEventState, attached_watch_event,
    camera_control_capabilities, camera_descriptor_from_value, camera_device_resource,
    camera_frame_from_value, camera_invalid_state, camera_not_supported,
    camera_photo_capabilities_from_config, camera_photo_from_frame_value,
    camera_photo_state_from_config, camera_recording_capabilities_from_value,
    camera_recording_from_path, camera_recording_output_path, camera_recording_state_from_runtime,
    camera_stream_resource, camera_watch_payload, close_camera_device_resource,
    close_camera_stream_resource, close_camera_watch_resource, detached_watch_event,
    store_camera_capabilities, validate_camera_photo_settings,
};
pub(super) use crate::platform::device::{
    CameraAudioCodec, CameraColorSpace, CameraDeviceDescriptor, CameraDeviceDescriptorValue,
    CameraDynamicRange, CameraExposureCompensationRange, CameraExposureMode,
    CameraExposureTimeRange, CameraFacingMode, CameraFloatControlRange, CameraFocusDistanceRange,
    CameraFocusMode, CameraFrame, CameraFrameMetadataValue, CameraFrameValue, CameraPanAngleRange,
    CameraPhoto, CameraPhotoCapabilities, CameraPhotoSettings, CameraPhotoState, CameraPixelFormat,
    CameraPixelFormatDescriptorValue, CameraPixelFormatFamily, CameraPlaneLayoutValue,
    CameraRecording, CameraRecordingCapabilities, CameraRecordingCapabilitiesValue,
    CameraRecordingContainer, CameraRecordingOptions, CameraRecordingOptionsValue,
    CameraRecordingState, CameraSensorIsoRange, CameraStabilizationMode, CameraStreamCapability,
    CameraStreamCapabilityValue, CameraStreamConfig, CameraStreamConfigValue, CameraTiltAngleRange,
    CameraTorchMode, CameraVideoCodec, CameraWatchEvent, CameraWhiteBalanceMode,
    CameraWhiteBalanceRange, CameraZoomRatioRange,
};
pub(super) use crate::platform::fs::core as core_fs;
pub(super) use crate::platform::resource::{ResourceEntry, ResourceFinalizer, ResourceKind};
pub(super) use crate::platform::{PlatformError, resource};
pub(super) use crate::runtime::BindingCallContext;
pub(super) use parking_lot::Mutex;

/// Initial Android camera row scratch capacity.
pub(super) const INITIAL_CAMERA_ROW_CAPACITY: usize = 8;

/// Initial Android camera string scratch capacity.
pub(super) const INITIAL_CAMERA_STRING_CAPACITY: usize = 512;

/// Initial Android camera frame scratch capacity.
pub(super) const INITIAL_CAMERA_FRAME_CAPACITY: usize = 1024 * 1024;

/// Maximum Android camera row scratch capacity.
pub(super) const MAX_CAMERA_ROW_CAPACITY: usize = 4096;

/// Maximum Android camera string scratch capacity.
pub(super) const MAX_CAMERA_STRING_CAPACITY: usize = 1024 * 1024;

/// Maximum Android camera frame scratch capacity.
pub(super) const MAX_CAMERA_FRAME_CAPACITY: usize = 32 * 1024 * 1024;

/// Android selector for exposure compensation.
pub(super) const CAMERA_CONTROL_EXPOSURE_COMPENSATION: u32 = 1;
/// Android selector for exposure mode.
pub(super) const CAMERA_CONTROL_EXPOSURE_MODE: u32 = 2;
/// Android selector for exposure time.
pub(super) const CAMERA_CONTROL_EXPOSURE_TIME_NS: u32 = 3;
/// Android selector for sensor ISO.
pub(super) const CAMERA_CONTROL_SENSOR_ISO: u32 = 4;
/// Android selector for white balance kelvin.
pub(super) const CAMERA_CONTROL_WHITE_BALANCE_KELVIN: u32 = 5;
/// Android selector for white balance mode.
pub(super) const CAMERA_CONTROL_WHITE_BALANCE_MODE: u32 = 6;
/// Android selector for focus distance.
pub(super) const CAMERA_CONTROL_FOCUS_DISTANCE_DIOPTERS: u32 = 7;
/// Android selector for focus mode.
pub(super) const CAMERA_CONTROL_FOCUS_MODE: u32 = 8;
/// Android selector for brightness.
pub(super) const CAMERA_CONTROL_BRIGHTNESS: u32 = 9;
/// Android selector for contrast.
pub(super) const CAMERA_CONTROL_CONTRAST: u32 = 10;
/// Android selector for saturation.
pub(super) const CAMERA_CONTROL_SATURATION: u32 = 11;
/// Android selector for sharpness.
pub(super) const CAMERA_CONTROL_SHARPNESS: u32 = 12;
/// Android selector for pan angle.
pub(super) const CAMERA_CONTROL_PAN_DEGREES: u32 = 13;
/// Android selector for tilt angle.
pub(super) const CAMERA_CONTROL_TILT_DEGREES: u32 = 14;
/// Android selector for zoom ratio.
pub(super) const CAMERA_CONTROL_ZOOM_RATIO: u32 = 15;
/// Android selector for stabilization mode.
pub(super) const CAMERA_CONTROL_STABILIZATION_MODE: u32 = 16;
/// Android selector for torch mode.
pub(super) const CAMERA_CONTROL_TORCH_MODE: u32 = 17;

/// Android flag for sRGB color space.
pub(super) const CAMERA_COLOR_SPACE_SRGB: u32 = 1 << 0;
/// Android flag for BT.601 color space.
pub(super) const CAMERA_COLOR_SPACE_BT601: u32 = 1 << 1;
/// Android flag for BT.709 color space.
pub(super) const CAMERA_COLOR_SPACE_BT709: u32 = 1 << 2;
/// Android flag for BT.2020 color space.
pub(super) const CAMERA_COLOR_SPACE_BT2020: u32 = 1 << 3;

/// Android flag for standard dynamic range.
pub(super) const CAMERA_DYNAMIC_RANGE_STANDARD: u32 = 1 << 0;
/// Android flag for HDR10 dynamic range.
pub(super) const CAMERA_DYNAMIC_RANGE_HDR10: u32 = 1 << 1;
/// Android flag for HLG dynamic range.
pub(super) const CAMERA_DYNAMIC_RANGE_HLG: u32 = 1 << 2;

/// Android flag for auto exposure.
pub(super) const CAMERA_EXPOSURE_MODE_AUTO: u32 = 1 << 0;
/// Android flag for continuous auto exposure.
pub(super) const CAMERA_EXPOSURE_MODE_CONTINUOUS_AUTO: u32 = 1 << 1;
/// Android flag for manual exposure.
pub(super) const CAMERA_EXPOSURE_MODE_MANUAL: u32 = 1 << 2;

/// Android flag for auto white balance.
pub(super) const CAMERA_WHITE_BALANCE_MODE_AUTO: u32 = 1 << 0;
/// Android flag for continuous auto white balance.
pub(super) const CAMERA_WHITE_BALANCE_MODE_CONTINUOUS_AUTO: u32 = 1 << 1;
/// Android flag for manual white balance.
pub(super) const CAMERA_WHITE_BALANCE_MODE_MANUAL: u32 = 1 << 2;

/// Android flag for auto focus.
pub(super) const CAMERA_FOCUS_MODE_AUTO: u32 = 1 << 0;
/// Android flag for continuous auto focus.
pub(super) const CAMERA_FOCUS_MODE_CONTINUOUS_AUTO: u32 = 1 << 1;
/// Android flag for manual focus.
pub(super) const CAMERA_FOCUS_MODE_MANUAL: u32 = 1 << 2;

/// Android flag for stabilization off.
pub(super) const CAMERA_STABILIZATION_MODE_OFF: u32 = 1 << 0;
/// Android flag for standard stabilization.
pub(super) const CAMERA_STABILIZATION_MODE_STANDARD: u32 = 1 << 1;
/// Android flag for high-quality stabilization.
pub(super) const CAMERA_STABILIZATION_MODE_HIGH_QUALITY: u32 = 1 << 2;

/// Android flag for torch off.
pub(super) const CAMERA_TORCH_MODE_OFF: u32 = 1 << 0;
/// Android flag for torch on.
pub(super) const CAMERA_TORCH_MODE_ON: u32 = 1 << 1;
/// Android flag for automatic torch.
pub(super) const CAMERA_TORCH_MODE_AUTO: u32 = 1 << 2;

/// Android code for MP4 recording output.
pub(super) const CAMERA_RECORDING_CONTAINER_MP4: u32 = 1;

/// Android code for H264 recording output.
pub(super) const CAMERA_RECORDING_VIDEO_CODEC_H264: u32 = 1;

/// Android code for AAC recording audio.
pub(super) const CAMERA_RECORDING_AUDIO_CODEC_AAC: u32 = 1;

/// Android-generated recording file extension.
pub(super) const CAMERA_RECORDING_EXTENSION: &str = "mp4";

/// One opened Android camera device resource.
#[derive(Debug, Clone)]
pub(super) struct AndroidCameraDeviceResource {
    /// The host session identifier.
    pub(super) session_id: u64,
    /// The supported stream configurations.
    pub(super) configs: Vec<CameraStreamConfigValue>,
    /// The supported stream capabilities.
    pub(super) capabilities: Vec<CameraStreamCapabilityValue>,
}

/// One opened Android camera stream resource.
#[derive(Debug, Clone)]
pub(super) struct AndroidCameraStreamResource {
    /// The host stream identifier.
    pub(super) stream_id: u64,
    /// The matched stream capability when available.
    pub(super) capability: Option<CameraStreamCapabilityValue>,
    /// Mutable recording state.
    pub(super) recording: Arc<Mutex<AndroidCameraRecordingState>>,
}

/// Mutable recording state for one opened Android camera stream.
#[derive(Debug, Clone, Default)]
pub(super) struct AndroidCameraRecordingState {
    /// Public recording-state snapshot.
    pub(super) runtime: CameraRecordingRuntimeState,
    /// Output path for the active recording when present.
    pub(super) path: Option<PathBuf>,
}

/// Finalizer for one Android camera device.
pub(super) struct AndroidCameraDeviceFinalizer {
    /// The owning runtime identifier.
    pub(super) runtime_id: u64,
    /// The host session identifier.
    pub(super) session_id: u64,
}

/// Finalizer for one Android camera stream.
pub(super) struct AndroidCameraStreamFinalizer {
    /// The owning runtime identifier.
    pub(super) runtime_id: u64,
    /// The host stream identifier.
    pub(super) stream_id: u64,
}

impl ResourceFinalizer for AndroidCameraDeviceFinalizer {
    /// Close one Android camera device session.
    fn finalize(self: Box<Self>, _resource_id: resource::ResourceId) {
        unsafe {
            destack_host_android_camera_device_close(self.runtime_id, self.session_id);
        }
    }
}

impl ResourceFinalizer for AndroidCameraStreamFinalizer {
    /// Close one Android camera stream.
    fn finalize(self: Box<Self>, _resource_id: resource::ResourceId) {
        unsafe {
            destack_host_android_camera_stream_close(self.runtime_id, self.stream_id);
        }
    }
}
