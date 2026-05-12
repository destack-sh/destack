pub(super) use std::collections::BTreeMap;
pub(super) use std::ffi::CString;
pub(super) use std::os::fd::RawFd;
pub(super) use std::path::{Path, PathBuf};
pub(super) use std::sync::Arc;
pub(super) use std::sync::atomic::{AtomicBool, Ordering};
pub(super) use std::thread::JoinHandle;
pub(super) use std::time::Instant;

pub(super) use parking_lot::{Condvar, Mutex};
pub(super) use v4l2_sys_mit as v4l2;

pub(super) use crate::diagnostic::{RuntimeError, RuntimeResult};
pub(super) use crate::platform::abi::{NativeSlice, NativeStringRef};
pub(super) use crate::platform::device::camera::core::{
    CAMERA_DEVICE_RESOURCE_LABEL, CAMERA_STREAM_RESOURCE_LABEL, CAMERA_WATCH_RESOURCE_LABEL,
    CameraControlModes, CameraRecordingRuntimeState, CameraStreamState, CameraWatchEventState,
    attached_watch_event, basic_camera_stream_capability, camera_descriptor_from_value,
    camera_device_resource, camera_frame_from_value, camera_invalid_state,
    camera_not_supported as shared_camera_not_supported, camera_photo_capabilities_from_config,
    camera_photo_from_frame_value, camera_photo_state_from_config,
    camera_recording_capabilities_from_value, camera_recording_from_path,
    camera_recording_output_path, camera_recording_state_from_runtime, camera_stream_resource,
    camera_watch_payload, close_camera_device_resource, close_camera_stream_resource,
    close_camera_watch_resource, detached_watch_event, empty_camera_metadata, frame_plane_layouts,
    store_camera_capabilities, validate_camera_photo_settings,
};
pub(super) use crate::platform::device::{
    CameraColorSpace, CameraDeviceDescriptor, CameraDeviceDescriptorValue,
    CameraExposureCompensationRange, CameraExposureMode, CameraExposureTimeRange, CameraFacingMode,
    CameraFloatControlRange, CameraFocusDistanceRange, CameraFocusMode, CameraFrame,
    CameraFrameValue, CameraPanAngleRange, CameraPhoto, CameraPhotoCapabilities,
    CameraPhotoSettings, CameraPhotoState, CameraPixelFormat, CameraPixelFormatDescriptorValue,
    CameraPixelFormatFamily, CameraRecording, CameraRecordingCapabilities,
    CameraRecordingCapabilitiesValue, CameraRecordingOptions, CameraRecordingState,
    CameraSensorIsoRange, CameraStabilizationMode, CameraStreamCapability,
    CameraStreamCapabilityValue, CameraStreamConfig, CameraStreamConfigValue, CameraTiltAngleRange,
    CameraTorchMode, CameraWatchEvent, CameraWhiteBalanceMode, CameraWhiteBalanceRange,
    CameraZoomRatioRange,
};
pub(super) use crate::platform::resource::{ResourceEntry, ResourceKind};
pub(super) use crate::platform::{NativeAbiCodec, PlatformError, core as core_platform, resource};
pub(super) use crate::runtime::BindingCallContext;

pub(super) use super::recording::stop_active_recording_worker;
pub(super) use crate::platform::core::BoundedQueue;

/// Stable camera descriptor snapshot carried by one opened device resource.
#[derive(Debug, Clone)]
pub(crate) struct LinuxCameraDescriptorInfo {
    /// Stable camera descriptor payload.
    pub(crate) descriptor: CameraDeviceDescriptorValue,
    /// Canonical device path.
    pub(crate) path: PathBuf,
    /// Supported stream configurations.
    pub(crate) configs: Vec<CameraStreamConfigValue>,
    /// Supported stream capabilities.
    pub(crate) capabilities: Vec<CameraStreamCapabilityValue>,
    /// Effective V4L2 capability bits.
    pub(crate) capability_bits: u32,
}

/// Device resource payload for one opened linux camera.
#[derive(Debug, Clone)]
pub(crate) struct LinuxCameraDeviceResource {
    /// Stable camera descriptor snapshot and cached host metadata.
    pub(crate) info: LinuxCameraDescriptorInfo,
}

/// One mapped V4L2 camera buffer.
#[derive(Debug, Clone, Copy)]
pub(crate) struct LinuxCameraMappedBuffer {
    /// The mapped buffer base address.
    pub(crate) address: *mut u8,
    /// The mapped buffer length.
    pub(crate) length: usize,
}

unsafe impl Send for LinuxCameraMappedBuffer {}
unsafe impl Sync for LinuxCameraMappedBuffer {}

/// Host capture mode for one opened linux camera stream.
#[derive(Debug, Clone)]
pub(crate) enum LinuxCameraStreamMode {
    /// Read one frame directly from the descriptor.
    ReadWrite {
        /// One conservative frame-size allocation hint.
        frame_bytes_hint: usize,
    },
    /// Dequeue one frame from a queued MMAP capture stream.
    Mmap {
        /// The selected V4L2 capture buffer type.
        capture_type: u32,
        /// The mapped MMAP buffers.
        buffers: Vec<LinuxCameraMappedBuffer>,
    },
}

/// Shared linux camera stream resource payload.
#[derive(Debug)]
pub(crate) struct LinuxCameraStreamResource {
    /// Owned host descriptor.
    pub(crate) descriptor: i32,
    /// Canonical device path.
    pub(crate) path: PathBuf,
    /// Effective V4L2 capability bits.
    pub(crate) capability_bits: u32,
    /// Selected stream configuration.
    pub(crate) config: CameraStreamConfigValue,
    /// Selected host stream mode.
    pub(crate) mode: LinuxCameraStreamMode,
    /// Mutable stream state.
    pub(crate) state: Mutex<CameraStreamState>,
    /// Active recording state.
    pub(crate) recording: Arc<Mutex<LinuxCameraRecordingState>>,
}

/// Close one owned linux camera descriptor during finalization.
pub(crate) struct LinuxCameraStreamFinalizer {
    /// Descriptor to close.
    pub(crate) descriptor: i32,
    /// Selected host stream mode.
    pub(crate) mode: LinuxCameraStreamMode,
    /// Active recording state.
    pub(crate) recording: Arc<Mutex<LinuxCameraRecordingState>>,
}

impl ResourceFinalizer for LinuxCameraStreamFinalizer {
    /// Close the descriptor during resource finalization.
    fn finalize(self: Box<Self>, _resource_id: resource::ResourceId) {
        // stop one active recording worker before tearing down the descriptor
        let _ = stop_active_recording_worker(&self.recording, None);

        // release mapped buffers before closing the descriptor
        if let LinuxCameraStreamMode::Mmap { buffers, .. } = &self.mode {
            for buffer in buffers {
                unsafe {
                    libc::munmap(buffer.address.cast::<libc::c_void>(), buffer.length);
                }
            }
        }

        unsafe {
            libc::close(self.descriptor);
        }
    }
}

/// One completed Linux recording result.
pub(crate) struct LinuxCameraRecordingFinish {
    /// Finalized recording duration.
    pub(crate) duration_ns: Option<u64>,
    /// Finalized output size.
    pub(crate) size_bytes: Option<u64>,
}

/// One recording-worker completion slot.
pub(crate) struct LinuxCameraRecordingCompletion {
    /// Worker completion result.
    pub(crate) result: Mutex<Option<Result<LinuxCameraRecordingFinish, String>>>,
    /// Wake one waiter after worker completion.
    pub(crate) wake: Condvar,
}

/// One active Linux recording worker.
pub(crate) struct LinuxCameraRecordingWorker {
    /// Stop signal observed by the worker thread.
    pub(crate) stop_signal: Arc<AtomicBool>,
    /// Completion slot filled by the worker thread.
    pub(crate) completion: Arc<LinuxCameraRecordingCompletion>,
    /// Join handle for the worker thread.
    pub(crate) handle: Option<JoinHandle<()>>,
}

/// Mutable Linux recording state stored on one opened stream.
pub(crate) struct LinuxCameraRecordingState {
    /// Runtime-visible recording state.
    pub(crate) runtime: CameraRecordingRuntimeState,
    /// Output path for one active recording.
    pub(crate) path: Option<PathBuf>,
    /// Active recording worker when recording is running.
    pub(crate) worker: Option<LinuxCameraRecordingWorker>,
}

impl Default for LinuxCameraRecordingState {
    /// Build one empty Linux recording state.
    fn default() -> Self {
        Self {
            runtime: CameraRecordingRuntimeState::default(),
            path: None,
            worker: None,
        }
    }
}

/// Candidate linux camera prefixes under `/dev`.
pub(super) const CAMERA_DEVICE_PREFIX: &str = "video";

/// One maximum poll timeout chunk.
pub(super) const MAX_POLL_TIMEOUT_MILLIS: i32 = i32::MAX;

/// One poll event mask for readable camera frames.
pub(super) const CAMERA_POLL_READ_FLAGS: i16 = libc::POLLIN | libc::POLLERR | libc::POLLHUP;

/// One linux V4L2 ioctl type byte.
pub(super) const V4L2_IOCTL_TYPE: u8 = b'V';

/// One queued MMAP buffer count.
pub(super) const CAMERA_STREAM_BUFFER_COUNT: u32 = 4;

/// One V4L2 capture type for single-plane capture.
pub(super) const V4L2_BUF_TYPE_VIDEO_CAPTURE: u32 =
    v4l2::v4l2_buf_type_V4L2_BUF_TYPE_VIDEO_CAPTURE as u32;

/// One V4L2 field selector for progressive frames.
pub(super) const V4L2_FIELD_NONE: u32 = v4l2::v4l2_field_V4L2_FIELD_NONE as u32;

/// One V4L2 memory selector for MMAP capture.
pub(super) const V4L2_MEMORY_MMAP: u32 = v4l2::v4l2_memory_V4L2_MEMORY_MMAP as u32;

/// One linux ioctl direction mask for read access.
pub(super) const IOC_READ: u32 = 2;

/// One linux ioctl direction mask for write access.
pub(super) const IOC_WRITE: u32 = 1;

/// Build one linux `_IOC` request code.
pub(super) const fn ioctl_code(direction: u32, ioctl_type: u8, number: u8, size: usize) -> usize {
    const IOC_NR_SHIFT: u32 = 0;
    const IOC_TYPE_SHIFT: u32 = 8;
    const IOC_SIZE_SHIFT: u32 = 16;
    const IOC_DIR_SHIFT: u32 = 30;

    ((direction as usize) << IOC_DIR_SHIFT)
        | ((ioctl_type as usize) << IOC_TYPE_SHIFT)
        | ((number as usize) << IOC_NR_SHIFT)
        | (size << IOC_SIZE_SHIFT)
}

/// Build one linux `_IOWR` request code.
pub(super) const fn iowr<T>(number: u8) -> usize {
    ioctl_code(
        IOC_READ | IOC_WRITE,
        V4L2_IOCTL_TYPE,
        number,
        std::mem::size_of::<T>(),
    )
}

/// Build one linux `_IOW` request code.
pub(super) const fn iow<T>(number: u8) -> usize {
    ioctl_code(IOC_WRITE, V4L2_IOCTL_TYPE, number, std::mem::size_of::<T>())
}

/// Build one V4L2 fourcc value.
pub(super) const fn v4l2_fourcc(a: u8, b: u8, c: u8, d: u8) -> u32 {
    (a as u32) | ((b as u32) << 8) | ((c as u32) << 16) | ((d as u32) << 24)
}

/// One `VIDIOC_QUERYCAP` request code.
pub(super) const VIDIOC_QUERYCAP: usize = iowr::<v4l2::v4l2_capability>(0);

/// One `VIDIOC_S_FMT` request code.
pub(super) const VIDIOC_S_FMT: usize = iowr::<v4l2::v4l2_format>(5);

/// One `VIDIOC_REQBUFS` request code.
pub(super) const VIDIOC_REQBUFS: usize = iowr::<v4l2::v4l2_requestbuffers>(8);

/// One `VIDIOC_QUERYBUF` request code.
pub(super) const VIDIOC_QUERYBUF: usize = iowr::<v4l2::v4l2_buffer>(9);

/// One `VIDIOC_QBUF` request code.
pub(super) const VIDIOC_QBUF: usize = iowr::<v4l2::v4l2_buffer>(15);

/// One `VIDIOC_DQBUF` request code.
pub(super) const VIDIOC_DQBUF: usize = iowr::<v4l2::v4l2_buffer>(17);

/// One `VIDIOC_STREAMON` request code.
pub(super) const VIDIOC_STREAMON: usize = iow::<u32>(18);

/// One `VIDIOC_STREAMOFF` request code.
pub(super) const VIDIOC_STREAMOFF: usize = iow::<u32>(19);

/// One `VIDIOC_S_PARM` request code.
pub(super) const VIDIOC_S_PARM: usize = iowr::<v4l2::v4l2_streamparm>(22);

/// One `VIDIOC_G_CTRL` request code.
pub(super) const VIDIOC_G_CTRL: usize = iowr::<v4l2::v4l2_control>(27);

/// One `VIDIOC_S_CTRL` request code.
pub(super) const VIDIOC_S_CTRL: usize = iowr::<v4l2::v4l2_control>(28);

/// One `VIDIOC_QUERYCTRL` request code.
pub(super) const VIDIOC_QUERYCTRL: usize = iowr::<v4l2::v4l2_queryctrl>(36);

/// One `VIDIOC_QUERYMENU` request code.
pub(super) const VIDIOC_QUERYMENU: usize = iowr::<v4l2::v4l2_querymenu>(37);

/// One `VIDIOC_ENUM_FMT` request code.
pub(super) const VIDIOC_ENUM_FMT: usize = iowr::<v4l2::v4l2_fmtdesc>(2);

/// One `VIDIOC_ENUM_FRAMESIZES` request code.
pub(super) const VIDIOC_ENUM_FRAMESIZES: usize = iowr::<v4l2::v4l2_frmsizeenum>(74);

/// One `VIDIOC_ENUM_FRAMEINTERVALS` request code.
pub(super) const VIDIOC_ENUM_FRAMEINTERVALS: usize = iowr::<v4l2::v4l2_frmivalenum>(75);

/// One `VIDIOC_QUERY_EXT_CTRL` request code.
pub(super) const VIDIOC_QUERY_EXT_CTRL: usize = iowr::<v4l2::v4l2_query_ext_ctrl>(103);
