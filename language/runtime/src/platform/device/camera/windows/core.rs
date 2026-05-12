#![allow(unsafe_op_in_unsafe_fn)]

pub(super) use std::path::PathBuf;
pub(super) use std::sync::Arc;

pub(super) use parking_lot::Mutex;
pub(super) use windows::Devices::Enumeration::Panel;
pub(super) use windows::Foundation::TypedEventHandler;
pub(super) use windows::Graphics::Imaging::{BitmapPixelFormat, SoftwareBitmap};
pub(super) use windows::Media::Capture::Frames::{
    MediaFrameFormat, MediaFrameReader, MediaFrameReaderAcquisitionMode,
    MediaFrameReaderStartStatus, MediaFrameReference, MediaFrameSource, MediaFrameSourceGroup,
    MediaFrameSourceInfo, MediaFrameSourceKind,
};
pub(super) use windows::Media::Capture::{
    CapturedPhoto, LowLagMediaRecording, LowLagPhotoCapture, MediaCapture,
    MediaCaptureInitializationSettings, MediaCaptureMemoryPreference, MediaCaptureSharingMode,
    StreamingCaptureMode,
};
pub(super) use windows::Media::Devices::{
    ColorTemperaturePreset, FocusMode as WindowsFocusMode, FocusSettings,
    MediaCapturePauseBehavior, MediaDeviceControl, OpticalImageStabilizationMode,
    VideoDeviceController, ZoomSettings, ZoomTransitionMode,
};
pub(super) use windows::Media::MediaProperties::{
    ImageEncodingProperties, MediaEncodingProfile, MediaPixelFormat, VideoEncodingQuality,
};
pub(super) use windows::Storage::StorageFile;
pub(super) use windows::Storage::Streams::{Buffer, DataReader};
pub(super) use windows::core::{AgileReference, Error as WinError, HSTRING, Ref};
pub(super) use windows_future::IAsyncAction;

pub(super) use crate::diagnostic::{RuntimeError, RuntimeResult};
pub(super) use crate::platform::abi::{NativeSlice, NativeStringRef};
pub(super) use crate::platform::core::BoundedQueue;
pub(super) use crate::platform::device::{
    CameraColorSpace, CameraDeviceDescriptor, CameraDeviceDescriptorValue, CameraDynamicRange,
    CameraExposureCompensationRange, CameraExposureMode, CameraExposureTimeRange, CameraFacingMode,
    CameraFloatControlRange, CameraFocusDistanceRange, CameraFocusMode, CameraFrame,
    CameraFrameValue, CameraPanAngleRange, CameraPhoto, CameraPhotoCapabilities,
    CameraPhotoSettings, CameraPhotoState, CameraPhotoValue, CameraPixelFormat,
    CameraPixelFormatDescriptorValue, CameraPixelFormatFamily, CameraPlaneLayoutValue,
    CameraRecording, CameraRecordingCapabilities, CameraRecordingCapabilitiesValue,
    CameraRecordingContainer, CameraRecordingOptions, CameraRecordingOptionsValue,
    CameraRecordingState, CameraSensorIsoRange, CameraStabilizationMode, CameraStreamCapability,
    CameraStreamCapabilityValue, CameraStreamConfig, CameraStreamConfigValue, CameraTiltAngleRange,
    CameraTorchMode, CameraVideoCodec, CameraWatchEvent, CameraWhiteBalanceMode,
    CameraWhiteBalanceRange, CameraZoomRatioRange,
};
pub(super) use crate::platform::resource::{ResourceEntry, ResourceFinalizer, ResourceKind};
pub(super) use crate::platform::{NativeAbiCodec, PlatformError, core as core_platform, resource};
pub(super) use crate::runtime::BindingCallContext;

pub(super) use crate::platform::device::camera::core::{
    CAMERA_DEVICE_RESOURCE_LABEL, CAMERA_STREAM_RESOURCE_LABEL, CAMERA_WATCH_RESOURCE_LABEL,
    CameraControlModes, CameraRecordingRuntimeState, CameraStreamState, CameraWatchEventState,
    attached_watch_event, basic_camera_stream_capability, bgra_camera_pixel_format_descriptor,
    camera_control_capabilities, camera_descriptor_from_value, camera_device_resource,
    camera_frame_from_value, camera_invalid_state, camera_not_supported,
    camera_photo_capabilities_from_config, camera_photo_state_from_config,
    camera_recording_capabilities_from_value, camera_recording_from_path,
    camera_recording_output_path, camera_recording_state_from_runtime, camera_stream_resource,
    camera_watch_payload, close_camera_device_resource, close_camera_stream_resource,
    close_camera_watch_resource, detached_watch_event, empty_camera_control_modes,
    empty_camera_metadata, frame_plane_layouts, next_camera_sequence, read_camera_frame_queue,
    store_camera_capabilities, try_read_camera_frame_queue, validate_camera_photo_settings,
};

/// One stream frame queue capacity.
pub(super) const CAMERA_FRAME_QUEUE_CAPACITY: usize = 8;

/// One stable Windows camera id prefix.
pub(super) const WINDOWS_CAMERA_ID_PREFIX: &str = "winrt-camera";

/// One stable stream subtype for BGRA8 output.
pub(super) const WINDOWS_CAMERA_BGRA_SUBTYPE: &str = "BGRA8";

/// One opened Windows camera stream selection.
#[derive(Debug, Clone)]
pub(super) struct WindowsCameraStreamSelection {
    /// The public stream configuration.
    pub(super) config: CameraStreamConfigValue,
    /// The source identifier within the capture session.
    pub(super) source_id: String,
    /// The underlying WinRT subtype string.
    pub(super) subtype: String,
}

/// One opened Windows camera descriptor snapshot.
#[derive(Debug, Clone)]
pub(super) struct WindowsCameraDescriptorInfo {
    /// Supported stream capabilities.
    pub(super) capabilities: Vec<CameraStreamCapabilityValue>,
    /// Openable stream selections keyed by public config.
    pub(super) selections: Vec<WindowsCameraStreamSelection>,
}

/// One opened Windows camera device resource.
#[derive(Debug, Clone)]
pub(super) struct WindowsCameraDeviceResource {
    /// Agile reference to the initialized media-capture session.
    pub(super) capture: AgileReference<MediaCapture>,
    /// Stable camera descriptor snapshot.
    pub(super) info: WindowsCameraDescriptorInfo,
}

/// Finalizer for one opened Windows camera device.
pub(super) struct WindowsCameraDeviceFinalizer {
    /// Agile reference to the initialized media-capture session.
    pub(super) capture: AgileReference<MediaCapture>,
}

/// One opened Windows camera stream resource.
pub(super) struct WindowsCameraStreamResource {
    /// Agile reference to the shared capture session.
    pub(super) capture: AgileReference<MediaCapture>,
    /// Agile reference to the live frame reader.
    pub(super) reader: AgileReference<MediaFrameReader>,
    /// Selected stream configuration.
    pub(super) config: CameraStreamConfigValue,
    /// Shared frame queue.
    pub(super) queue: Arc<BoundedQueue<CameraFrameValue>>,
    /// Mutable stream state.
    pub(super) state: Arc<Mutex<CameraStreamState>>,
    /// Mutable recording state.
    pub(super) recording: Arc<Mutex<WindowsCameraRecordingState>>,
}

/// Finalizer for one opened Windows camera stream.
pub(super) struct WindowsCameraStreamFinalizer {
    /// Agile reference to the live frame reader.
    pub(super) reader: AgileReference<MediaFrameReader>,
    /// Frame-arrived callback token.
    pub(super) frame_arrived_token: i64,
    /// Shared frame queue.
    pub(super) queue: Arc<BoundedQueue<CameraFrameValue>>,
    /// Mutable recording state.
    pub(super) recording: Arc<Mutex<WindowsCameraRecordingState>>,
}

/// Mutable recording state for one opened Windows camera stream.
pub(super) struct WindowsCameraRecordingState {
    /// Public recording-state snapshot.
    pub(super) runtime: CameraRecordingRuntimeState,
    /// Output path for the active recording when present.
    pub(super) path: Option<PathBuf>,
    /// Live low-lag recording object when recording is active.
    pub(super) recording: Option<AgileReference<LowLagMediaRecording>>,
}

impl ResourceFinalizer for WindowsCameraDeviceFinalizer {
    /// Close the capture session.
    fn finalize(self: Box<Self>, _resource_id: resource::ResourceId) {
        let Ok(capture) = self.capture.resolve() else {
            return;
        };

        let _ = capture.Close();
    }
}

impl ResourceFinalizer for WindowsCameraStreamFinalizer {
    /// Stop the frame reader and wake blocked readers.
    fn finalize(self: Box<Self>, _resource_id: resource::ResourceId) {
        // active recording
        let recording = {
            let mut state = self.recording.lock();
            state.runtime = CameraRecordingRuntimeState::default();
            state.path = None;
            state.recording.take()
        };

        // best-effort recording teardown
        if let Some(recording) = recording.and_then(|recording| recording.resolve().ok()) {
            let _ = recording.StopAsync().and_then(|action| action.get());
            let _ = recording.FinishAsync().and_then(|action| action.get());
        }

        let Ok(reader) = self.reader.resolve() else {
            self.queue.close();
            return;
        };

        let _ = reader.RemoveFrameArrived(self.frame_arrived_token);
        let _ = reader.StopAsync().and_then(|action| action.get());
        self.queue.close();
    }
}
