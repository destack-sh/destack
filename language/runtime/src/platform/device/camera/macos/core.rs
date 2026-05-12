#![allow(unsafe_op_in_unsafe_fn)]

pub(super) use std::collections::BTreeSet;
use std::panic::AssertUnwindSafe;
pub(super) use std::path::PathBuf;
pub(super) use std::sync::Arc;

pub(super) use dispatch2::{DispatchQueue, DispatchRetained};
pub(super) use objc2::rc::Retained;
pub(super) use objc2::runtime::{AnyObject, ProtocolObject};
pub(super) use objc2::{AnyThread, DefinedClass, Message, define_class, exception};
pub(super) use objc2_av_foundation::{
    AVCaptureConnection, AVCaptureDevice, AVCaptureDeviceDiscoverySession, AVCaptureDeviceFormat,
    AVCaptureDeviceInput, AVCaptureDevicePosition, AVCaptureDeviceType,
    AVCaptureDeviceTypeBuiltInDualCamera, AVCaptureDeviceTypeBuiltInDualWideCamera,
    AVCaptureDeviceTypeBuiltInLiDARDepthCamera, AVCaptureDeviceTypeBuiltInTelephotoCamera,
    AVCaptureDeviceTypeBuiltInTripleCamera, AVCaptureDeviceTypeBuiltInTrueDepthCamera,
    AVCaptureDeviceTypeBuiltInUltraWideCamera, AVCaptureDeviceTypeBuiltInWideAngleCamera,
    AVCaptureDeviceTypeContinuityCamera, AVCaptureDeviceTypeExternal,
    AVCaptureExposureDurationCurrent, AVCaptureExposureMode, AVCaptureFileOutput,
    AVCaptureFileOutputRecordingDelegate, AVCaptureFocusMode, AVCaptureISOCurrent,
    AVCaptureMovieFileOutput, AVCapturePhoto, AVCapturePhotoCaptureDelegate, AVCapturePhotoOutput,
    AVCapturePhotoSettings, AVCaptureSession, AVCaptureTorchMode, AVCaptureVideoDataOutput,
    AVCaptureVideoDataOutputSampleBufferDelegate, AVCaptureVideoStabilizationMode,
    AVCaptureWhiteBalanceMode, AVCaptureWhiteBalanceTemperatureAndTintValues, AVFrameRateRange,
    AVMediaTypeVideo,
};
pub(super) use objc2_core_media::{
    CMSampleBuffer, CMTime, CMVideoFormatDescriptionGetDimensions, kCMTimeInvalid,
};
pub(super) use objc2_core_video::{
    CVPixelBuffer, CVPixelBufferGetBaseAddress, CVPixelBufferGetBytesPerRow,
    CVPixelBufferGetDataSize, CVPixelBufferGetHeight, CVPixelBufferGetWidth,
    CVPixelBufferLockBaseAddress, CVPixelBufferLockFlags, CVPixelBufferUnlockBaseAddress,
    kCVPixelBufferPixelFormatTypeKey, kCVPixelFormatType_32BGRA,
};
pub(super) use objc2_foundation::{
    NSArray, NSDictionary, NSError, NSNumber, NSObject, NSObjectProtocol, NSURL,
};
pub(super) use parking_lot::{Condvar, Mutex};

pub(super) use crate::diagnostic::{RuntimeError, RuntimeResult};
pub(super) use crate::platform::abi::{NativeSlice, NativeStringRef};
pub(super) use crate::platform::core::BoundedQueue;
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
    empty_camera_metadata, frame_plane_layouts, store_camera_capabilities,
    validate_camera_photo_settings,
};
pub(super) use crate::platform::device::{
    CameraDeviceDescriptor, CameraDeviceDescriptorValue, CameraDynamicRange,
    CameraExposureCompensationRange, CameraExposureMode, CameraExposureTimeRange, CameraFacingMode,
    CameraFloatControlRange, CameraFocusDistanceRange, CameraFocusMode, CameraFrame,
    CameraFrameValue, CameraPanAngleRange, CameraPhoto, CameraPhotoCapabilities,
    CameraPhotoSettings, CameraPhotoState, CameraPhotoValue, CameraPlaneLayoutValue,
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

use super::metadata::*;

/// One stream frame queue capacity.
pub(super) const CAMERA_FRAME_QUEUE_CAPACITY: usize = 8;

/// One preferred frame-timescale for AVFoundation frame-rate selection.
pub(super) const CAMERA_FRAME_TIME_SCALE: i32 = 1_000_000_000;

/// One public descriptor prefix for AVFoundation devices.
pub(super) const MACOS_CAMERA_DESCRIPTOR_PREFIX: &str = "avfoundation-camera:";

/// One stream callback queue label.
pub(super) const MACOS_CAMERA_QUEUE_LABEL: &str = "DestackCamera";

/// One opened macOS camera descriptor snapshot.
#[derive(Debug, Clone)]
pub(super) struct MacosCameraDescriptorInfo {
    /// Stable AVFoundation unique identifier.
    pub(super) unique_id: String,
    /// Supported stream configurations.
    pub(super) configs: Vec<CameraStreamConfigValue>,
    /// Supported stream capabilities.
    pub(super) capabilities: Vec<CameraStreamCapabilityValue>,
}

/// One opened macOS camera device resource.
#[derive(Debug, Clone)]
pub(super) struct MacosCameraDeviceResource {
    /// Stable cached descriptor info.
    pub(super) info: MacosCameraDescriptorInfo,
}

/// One opened macOS camera stream resource.
pub(super) struct MacosCameraStreamResource {
    /// Selected stream configuration.
    pub(super) config: CameraStreamConfigValue,
    /// Live capture device.
    pub(super) device: core_platform::DispatchBound<AVCaptureDevice>,
    /// Live capture session.
    pub(super) session: core_platform::DispatchBound<AVCaptureSession>,
    /// Live video output connection.
    pub(super) connection: core_platform::DispatchBound<AVCaptureConnection>,
    /// Live movie recording output.
    pub(super) movie_output: core_platform::DispatchBound<AVCaptureMovieFileOutput>,
    /// Live photo output.
    pub(super) photo_output: core_platform::DispatchBound<AVCapturePhotoOutput>,
    /// Live movie recording delegate.
    pub(super) recording_delegate: core_platform::DispatchBound<MacosCameraRecordingDelegate>,
    /// Live photo delegate.
    pub(super) photo_delegate: core_platform::DispatchBound<MacosCameraPhotoDelegate>,
    /// Stream callback queue.
    pub(super) callback_queue: DispatchRetained<DispatchQueue>,
    /// Shared frame queue.
    pub(super) queue: Arc<BoundedQueue<CameraFrameValue>>,
    /// Mutable stream state.
    pub(super) state: Arc<Mutex<CameraStreamState>>,
    /// Mutable recording state.
    pub(super) recording: Arc<MacosCameraRecordingSharedState>,
    /// Mutable photo state.
    pub(super) photo: Arc<MacosCameraPhotoCaptureSharedState>,
}

unsafe impl Send for MacosCameraStreamResource {}
unsafe impl Sync for MacosCameraStreamResource {}

/// Finalizer for one opened macOS camera stream.
pub(super) struct MacosCameraStreamFinalizer {
    /// Live capture session.
    pub(super) session: core_platform::DispatchBound<AVCaptureSession>,
    /// Live capture input.
    pub(super) input: core_platform::DispatchBound<AVCaptureDeviceInput>,
    /// Live video output.
    pub(super) video_output: core_platform::DispatchBound<AVCaptureVideoDataOutput>,
    /// Live movie output.
    pub(super) movie_output: core_platform::DispatchBound<AVCaptureMovieFileOutput>,
    /// Live photo output.
    pub(super) photo_output: core_platform::DispatchBound<AVCapturePhotoOutput>,
    /// Retained frame delegate lifetime anchor.
    pub(super) _frame_delegate: core_platform::DispatchBound<MacosCameraFrameDelegate>,
    /// Retained recording delegate lifetime anchor.
    pub(super) _recording_delegate: core_platform::DispatchBound<MacosCameraRecordingDelegate>,
    /// Retained photo delegate lifetime anchor.
    pub(super) _photo_delegate: core_platform::DispatchBound<MacosCameraPhotoDelegate>,
    /// Stream callback queue.
    pub(super) callback_queue: DispatchRetained<DispatchQueue>,
    /// Shared frame queue.
    pub(super) queue: Arc<BoundedQueue<CameraFrameValue>>,
    /// Mutable recording state.
    pub(super) recording: Arc<MacosCameraRecordingSharedState>,
    /// Mutable photo state.
    pub(super) photo: Arc<MacosCameraPhotoCaptureSharedState>,
}

unsafe impl Send for MacosCameraStreamFinalizer {}
unsafe impl Sync for MacosCameraStreamFinalizer {}

impl ResourceFinalizer for MacosCameraStreamFinalizer {
    /// Stop the capture session and wake blocked readers.
    fn finalize(self: Box<Self>, _resource_id: resource::ResourceId) {
        self.session
            .dispatch_on(self.callback_queue.as_ref(), |session| {
                let _ = exception::catch(AssertUnwindSafe(|| unsafe {
                    let video_output = self.video_output.get_unchecked();
                    let movie_output = self.movie_output.get_unchecked();
                    let photo_output = self.photo_output.get_unchecked();
                    let input = self.input.get_unchecked();

                    // output teardown
                    if movie_output.isRecording() {
                        movie_output.stopRecording();
                    }
                    video_output.setSampleBufferDelegate_queue(None, None);
                    if session.isRunning() {
                        session.stopRunning();
                    }
                    session.removeOutput(movie_output);
                    session.removeOutput(photo_output);
                    session.removeOutput(video_output);
                    session.removeInput(input);
                }));
            });

        // recording teardown
        {
            let mut state = self.recording.state.lock();
            state.runtime = CameraRecordingRuntimeState::default();
            state.path = None;
            state.finish_error = None;
            self.recording.finished.notify_all();
        }

        // photo teardown
        {
            let mut state = self.photo.state.lock();
            state.is_pending = false;
            state.result = None;
            self.photo.finished.notify_all();
        }

        self.queue.close();
    }
}

/// Mutable recording state for one opened macOS camera stream.
pub(super) struct MacosCameraRecordingState {
    /// Public recording-state snapshot.
    pub(super) runtime: CameraRecordingRuntimeState,
    /// Active output path when recording is live.
    pub(super) path: Option<PathBuf>,
    /// Final finish error when one recording stops unexpectedly.
    pub(super) finish_error: Option<String>,
}

/// Shared recording state for one opened macOS camera stream.
pub(super) struct MacosCameraRecordingSharedState {
    /// Mutable recording state.
    pub(super) state: Mutex<MacosCameraRecordingState>,
    /// Waiter notification for stop-recording completion.
    pub(super) finished: Condvar,
}

/// Mutable still-photo state for one opened macOS camera stream.
pub(super) struct MacosCameraPhotoCaptureState {
    /// Whether one capture is currently in flight.
    pub(super) is_pending: bool,
    /// The completed still-photo result when available.
    pub(super) result: Option<Result<CameraPhotoValue, String>>,
}

/// Shared still-photo state for one opened macOS camera stream.
pub(super) struct MacosCameraPhotoCaptureSharedState {
    /// Mutable still-photo state.
    pub(super) state: Mutex<MacosCameraPhotoCaptureState>,
    /// Waiter notification for one completed photo capture.
    pub(super) finished: Condvar,
}

/// One recording delegate ivar set.
pub(super) struct MacosCameraRecordingDelegateState {
    /// Shared recording state.
    recording: Arc<MacosCameraRecordingSharedState>,
}

/// One photo delegate ivar set.
pub(super) struct MacosCameraPhotoDelegateState {
    /// Shared photo state.
    photo: Arc<MacosCameraPhotoCaptureSharedState>,
}

/// One frame delegate ivar set.
pub(super) struct MacosCameraFrameDelegateState {
    /// The shared frame queue.
    queue: Arc<BoundedQueue<CameraFrameValue>>,
    /// Mutable stream state.
    state: Arc<Mutex<CameraStreamState>>,
    /// Selected stream configuration.
    config: CameraStreamConfigValue,
}

define_class!(
    #[unsafe(super(NSObject))]
    #[ivars = MacosCameraFrameDelegateState]
    #[name = "DestackCameraFrameDelegate"]
    /// One AVFoundation frame delegate wrapper.
    pub(super) struct MacosCameraFrameDelegate;

    unsafe impl NSObjectProtocol for MacosCameraFrameDelegate {}

    unsafe impl AVCaptureVideoDataOutputSampleBufferDelegate for MacosCameraFrameDelegate {
        #[allow(non_snake_case)]
        #[unsafe(method(captureOutput:didOutputSampleBuffer:fromConnection:))]
        /// Queue one delivered sample buffer as a runtime camera frame.
        fn captureOutput_didOutputSampleBuffer_fromConnection(
            &self,
            _output: &objc2_av_foundation::AVCaptureOutput,
            sample_buffer: &CMSampleBuffer,
            _connection: &objc2_av_foundation::AVCaptureConnection,
        ) {
            let state = self.ivars();
            let frame = match camera_frame_value_from_sample_buffer(
                sample_buffer,
                &state.config,
                state.state.as_ref(),
                "destack.device.camera.stream.read",
            ) {
                Ok(frame) => frame,
                Err(_) => return,
            };

            state.queue.push_drop_oldest(frame);
        }
    }
);

impl MacosCameraFrameDelegate {
    /// Create one frame delegate instance.
    pub(super) fn new(
        queue: Arc<BoundedQueue<CameraFrameValue>>,
        state: Arc<Mutex<CameraStreamState>>,
        config: CameraStreamConfigValue,
    ) -> Retained<Self> {
        let value = Self::alloc().set_ivars(MacosCameraFrameDelegateState {
            queue,
            state,
            config,
        });

        unsafe { objc2::msg_send![super(value), init] }
    }

    /// Return the delegate as one protocol object.
    pub(super) fn as_protocol(
        &self,
    ) -> &ProtocolObject<dyn AVCaptureVideoDataOutputSampleBufferDelegate> {
        ProtocolObject::from_ref(self)
    }
}

define_class!(
    #[unsafe(super(NSObject))]
    #[ivars = MacosCameraRecordingDelegateState]
    #[name = "DestackCameraRecordingDelegate"]
    /// One AVFoundation recording delegate wrapper.
    pub(super) struct MacosCameraRecordingDelegate;

    unsafe impl NSObjectProtocol for MacosCameraRecordingDelegate {}

    unsafe impl AVCaptureFileOutputRecordingDelegate for MacosCameraRecordingDelegate {
        #[allow(non_snake_case)]
        #[unsafe(method(captureOutput:didPauseRecordingToOutputFileAtURL:fromConnections:))]
        /// Mark one active recording as paused.
        fn captureOutput_didPauseRecordingToOutputFileAtURL_fromConnections(
            &self,
            _output: &AVCaptureFileOutput,
            _file_url: &NSURL,
            _connections: &NSArray<AVCaptureConnection>,
        ) {
            let recording = &self.ivars().recording;
            let mut state = recording.state.lock();

            if state.runtime.is_active {
                state.runtime.is_paused = true;
            }
        }

        #[allow(non_snake_case)]
        #[unsafe(method(captureOutput:didResumeRecordingToOutputFileAtURL:fromConnections:))]
        /// Mark one active recording as resumed.
        fn captureOutput_didResumeRecordingToOutputFileAtURL_fromConnections(
            &self,
            _output: &AVCaptureFileOutput,
            _file_url: &NSURL,
            _connections: &NSArray<AVCaptureConnection>,
        ) {
            let recording = &self.ivars().recording;
            let mut state = recording.state.lock();

            if state.runtime.is_active {
                state.runtime.is_paused = false;
            }
        }

        #[allow(non_snake_case)]
        #[unsafe(method(captureOutput:didFinishRecordingToOutputFileAtURL:fromConnections:error:))]
        /// Mark one active recording as finished and wake waiters.
        fn captureOutput_didFinishRecordingToOutputFileAtURL_fromConnections_error(
            &self,
            _output: &AVCaptureFileOutput,
            _output_file_url: &NSURL,
            _connections: &NSArray<AVCaptureConnection>,
            error: Option<&NSError>,
        ) {
            let recording = &self.ivars().recording;
            let mut state = recording.state.lock();

            // finish state
            state.runtime = CameraRecordingRuntimeState::default();
            state.path = None;
            state.finish_error = error.map(|error| error.localizedDescription().to_string());

            recording.finished.notify_all();
        }
    }
);

impl MacosCameraRecordingDelegate {
    /// Create one recording delegate instance.
    pub(super) fn new(recording: Arc<MacosCameraRecordingSharedState>) -> Retained<Self> {
        let value = Self::alloc().set_ivars(MacosCameraRecordingDelegateState { recording });

        unsafe { objc2::msg_send![super(value), init] }
    }

    /// Return the delegate as one protocol object.
    pub(super) fn as_protocol(&self) -> &ProtocolObject<dyn AVCaptureFileOutputRecordingDelegate> {
        ProtocolObject::from_ref(self)
    }
}

define_class!(
    #[unsafe(super(NSObject))]
    #[ivars = MacosCameraPhotoDelegateState]
    #[name = "DestackCameraPhotoDelegate"]
    /// One AVFoundation still-photo delegate wrapper.
    pub(super) struct MacosCameraPhotoDelegate;

    unsafe impl NSObjectProtocol for MacosCameraPhotoDelegate {}

    unsafe impl AVCapturePhotoCaptureDelegate for MacosCameraPhotoDelegate {
        #[allow(non_snake_case)]
        #[unsafe(method(captureOutput:didFinishProcessingPhoto:error:))]
        /// Store one processed still photo and wake waiters.
        unsafe fn captureOutput_didFinishProcessingPhoto_error(
            &self,
            _output: &AVCapturePhotoOutput,
            photo: &AVCapturePhoto,
            error: Option<&NSError>,
        ) {
            let shared = &self.ivars().photo;
            let mut state = shared.state.lock();

            // photo completion
            state.is_pending = false;
            state.result = Some(match error {
                Some(error) => Err(error.localizedDescription().to_string()),
                None => {
                    camera_photo_value_from_photo(photo, "destack.device.camera.stream.takePhoto")
                        .map_err(|error| error.to_string())
                }
            });

            shared.finished.notify_all();
        }
    }
);

impl MacosCameraPhotoDelegate {
    /// Create one photo delegate instance.
    pub(super) fn new(photo: Arc<MacosCameraPhotoCaptureSharedState>) -> Retained<Self> {
        let value = Self::alloc().set_ivars(MacosCameraPhotoDelegateState { photo });

        unsafe { objc2::msg_send![super(value), init] }
    }

    /// Return the delegate as one protocol object.
    pub(super) fn as_protocol(&self) -> &ProtocolObject<dyn AVCapturePhotoCaptureDelegate> {
        ProtocolObject::from_ref(self)
    }
}
