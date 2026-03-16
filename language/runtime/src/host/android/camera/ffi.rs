#![allow(unreachable_pub)]

use super::types::{
    AndroidHostCameraCallbacks, AndroidHostCameraDeviceDescriptorHeader,
    AndroidHostCameraFrameHeader, AndroidHostCameraStreamCapabilityHeader,
    AndroidHostCameraStreamConfigHeader,
};
use crate::host::abi::HostStatus;
use crate::host::android::bridge::bindings::invoke_android_binding_callback;
use crate::runtime::{NativeSlice, NativeStringRef};

/// Resolve and invoke one Android host camera callback.
fn call_android_camera_callback<T: Copy>(
    runtime_id: u64,
    resolve: impl FnOnce(&AndroidHostCameraCallbacks) -> Option<T>,
    invoke: impl FnOnce(T) -> u32,
) -> u32 {
    invoke_android_binding_callback(runtime_id, |bindings| resolve(&bindings.camera), invoke)
}

macro_rules! camera_callback {
    ($name:ident ($($arg:ident : $ty:ty),* $(,)?) -> $resolve:ident $(; guard $guard:expr)? ) => {
        #[doc = "Route one Android host camera callback through the registered callback table."]
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn $name(runtime_id: u64, $($arg: $ty),*) -> u32 {
            $(if !($guard) { return HostStatus::InvalidArgument.code(); })?
            call_android_camera_callback(runtime_id, |callbacks| callbacks.$resolve, |callback| unsafe {
                callback(runtime_id, $($arg),*)
            })
        }
    };
}

camera_callback!(
    destack_host_android_camera_device_list(
        devices: NativeSlice<AndroidHostCameraDeviceDescriptorHeader>,
        device_count_written: *mut u32,
        string_bytes: NativeSlice<u8>,
        string_bytes_written: *mut u32
    ) -> device_list;
    guard !device_count_written.is_null() && !string_bytes_written.is_null()
);
camera_callback!(destack_host_android_camera_device_open(id: NativeStringRef, session_id: *mut u64) -> device_open; guard !session_id.is_null());
camera_callback!(destack_host_android_camera_device_close(session_id: u64) -> device_close);
camera_callback!(destack_host_android_camera_device_stream_config_list(session_id: u64, configs: NativeSlice<AndroidHostCameraStreamConfigHeader>, config_count_written: *mut u32) -> stream_config_list; guard !config_count_written.is_null());
camera_callback!(destack_host_android_camera_device_stream_capability_list(session_id: u64, capabilities: NativeSlice<AndroidHostCameraStreamCapabilityHeader>, capability_count_written: *mut u32) -> stream_capability_list; guard !capability_count_written.is_null());
camera_callback!(destack_host_android_camera_stream_open(session_id: u64, config: AndroidHostCameraStreamConfigHeader, stream_id: *mut u64) -> stream_open; guard !stream_id.is_null());
camera_callback!(destack_host_android_camera_stream_close(stream_id: u64) -> stream_close);
camera_callback!(destack_host_android_camera_stream_start(stream_id: u64) -> stream_start);
camera_callback!(destack_host_android_camera_stream_stop(stream_id: u64) -> stream_stop);
camera_callback!(destack_host_android_camera_stream_read(stream_id: u64, timeout_ns: u64, header: *mut AndroidHostCameraFrameHeader, bytes: NativeSlice<u8>, bytes_written: *mut u32) -> stream_read; guard !header.is_null() && !bytes_written.is_null());
camera_callback!(destack_host_android_camera_stream_try_read(stream_id: u64, header: *mut AndroidHostCameraFrameHeader, bytes: NativeSlice<u8>, bytes_written: *mut u32) -> stream_try_read; guard !header.is_null() && !bytes_written.is_null());
camera_callback!(destack_host_android_camera_stream_config(stream_id: u64, config: *mut AndroidHostCameraStreamConfigHeader) -> stream_config; guard !config.is_null());
camera_callback!(destack_host_android_camera_stream_get_u64(stream_id: u64, selector: u32, value: *mut u64) -> stream_get_u64; guard !value.is_null());
camera_callback!(destack_host_android_camera_stream_set_u64(stream_id: u64, selector: u32, value: u64) -> stream_set_u64);
camera_callback!(destack_host_android_camera_stream_get_u32(stream_id: u64, selector: u32, value: *mut u32) -> stream_get_u32; guard !value.is_null());
camera_callback!(destack_host_android_camera_stream_set_u32(stream_id: u64, selector: u32, value: u32) -> stream_set_u32);
camera_callback!(destack_host_android_camera_stream_get_f64(stream_id: u64, selector: u32, value: *mut f64) -> stream_get_f64; guard !value.is_null());
camera_callback!(destack_host_android_camera_stream_set_f64(stream_id: u64, selector: u32, value: f64) -> stream_set_f64);
camera_callback!(destack_host_android_camera_stream_get_range_f64(stream_id: u64, selector: u32, minimum: *mut f64, maximum: *mut f64, step: *mut f64) -> stream_get_range_f64; guard !minimum.is_null() && !maximum.is_null() && !step.is_null());
camera_callback!(destack_host_android_camera_stream_get_range_u64(stream_id: u64, selector: u32, minimum: *mut u64, maximum: *mut u64, step: *mut u64) -> stream_get_range_u64; guard !minimum.is_null() && !maximum.is_null() && !step.is_null());
camera_callback!(destack_host_android_camera_stream_get_range_u32(stream_id: u64, selector: u32, minimum: *mut u32, maximum: *mut u32, step: *mut u32) -> stream_get_range_u32; guard !minimum.is_null() && !maximum.is_null() && !step.is_null());

macro_rules! camera_getter_alias {
    ($name:ident, $selector:expr, u64) => {
        #[doc = "Route one Android host camera getter alias through the shared selector lane."]
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn $name(runtime_id: u64, stream_id: u64, value: *mut u64) -> u32 {
            // shared selector forwarding
            unsafe {
                destack_host_android_camera_stream_get_u64(runtime_id, stream_id, $selector, value)
            }
        }
    };
    ($name:ident, $selector:expr, u32) => {
        #[doc = "Route one Android host camera getter alias through the shared selector lane."]
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn $name(runtime_id: u64, stream_id: u64, value: *mut u32) -> u32 {
            // shared selector forwarding
            unsafe {
                destack_host_android_camera_stream_get_u32(runtime_id, stream_id, $selector, value)
            }
        }
    };
    ($name:ident, $selector:expr, f64) => {
        #[doc = "Route one Android host camera getter alias through the shared selector lane."]
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn $name(runtime_id: u64, stream_id: u64, value: *mut f64) -> u32 {
            // shared selector forwarding
            unsafe {
                destack_host_android_camera_stream_get_f64(runtime_id, stream_id, $selector, value)
            }
        }
    };
}

macro_rules! camera_setter_alias {
    ($name:ident, $selector:expr, u64, $ty:ty) => {
        #[doc = "Route one Android host camera setter alias through the shared selector lane."]
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn $name(runtime_id: u64, stream_id: u64, value: $ty) -> u32 {
            // shared selector forwarding
            unsafe {
                destack_host_android_camera_stream_set_u64(
                    runtime_id,
                    stream_id,
                    $selector,
                    value as u64,
                )
            }
        }
    };
    ($name:ident, $selector:expr, u32, $ty:ty) => {
        #[doc = "Route one Android host camera setter alias through the shared selector lane."]
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn $name(runtime_id: u64, stream_id: u64, value: $ty) -> u32 {
            // shared selector forwarding
            unsafe {
                destack_host_android_camera_stream_set_u32(
                    runtime_id,
                    stream_id,
                    $selector,
                    value as u32,
                )
            }
        }
    };
    ($name:ident, $selector:expr, f64, $ty:ty) => {
        #[doc = "Route one Android host camera setter alias through the shared selector lane."]
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn $name(runtime_id: u64, stream_id: u64, value: $ty) -> u32 {
            // shared selector forwarding
            unsafe {
                destack_host_android_camera_stream_set_f64(
                    runtime_id,
                    stream_id,
                    $selector,
                    value as f64,
                )
            }
        }
    };
}

macro_rules! camera_range_alias {
    ($name:ident, $selector:expr, u64) => {
        #[doc = "Route one Android host camera range alias through the shared selector lane."]
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn $name(
            runtime_id: u64,
            stream_id: u64,
            minimum: *mut u64,
            maximum: *mut u64,
            step: *mut u64,
        ) -> u32 {
            // shared selector forwarding
            unsafe {
                destack_host_android_camera_stream_get_range_u64(
                    runtime_id, stream_id, $selector, minimum, maximum, step,
                )
            }
        }
    };
    ($name:ident, $selector:expr, u32) => {
        #[doc = "Route one Android host camera range alias through the shared selector lane."]
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn $name(
            runtime_id: u64,
            stream_id: u64,
            minimum: *mut u32,
            maximum: *mut u32,
            step: *mut u32,
        ) -> u32 {
            // shared selector forwarding
            unsafe {
                destack_host_android_camera_stream_get_range_u32(
                    runtime_id, stream_id, $selector, minimum, maximum, step,
                )
            }
        }
    };
    ($name:ident, $selector:expr, f64) => {
        #[doc = "Route one Android host camera range alias through the shared selector lane."]
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn $name(
            runtime_id: u64,
            stream_id: u64,
            minimum: *mut f64,
            maximum: *mut f64,
            step: *mut f64,
        ) -> u32 {
            // shared selector forwarding
            unsafe {
                destack_host_android_camera_stream_get_range_f64(
                    runtime_id, stream_id, $selector, minimum, maximum, step,
                )
            }
        }
    };
}

camera_getter_alias!(
    destack_host_android_camera_stream_exposure_compensation,
    1,
    f64
);
camera_setter_alias!(
    destack_host_android_camera_stream_set_exposure_compensation,
    1,
    f64,
    f64
);
camera_range_alias!(
    destack_host_android_camera_stream_exposure_compensation_range,
    1,
    f64
);
camera_getter_alias!(destack_host_android_camera_stream_exposure_mode, 2, u32);
camera_setter_alias!(
    destack_host_android_camera_stream_set_exposure_mode,
    2,
    u32,
    u32
);
camera_getter_alias!(destack_host_android_camera_stream_exposure_time_ns, 3, u64);
camera_setter_alias!(
    destack_host_android_camera_stream_set_exposure_time_ns,
    3,
    u64,
    u64
);
camera_range_alias!(
    destack_host_android_camera_stream_exposure_time_range,
    3,
    u64
);
camera_getter_alias!(destack_host_android_camera_stream_sensor_iso, 4, u32);
camera_setter_alias!(
    destack_host_android_camera_stream_set_sensor_iso,
    4,
    u32,
    u32
);
camera_range_alias!(destack_host_android_camera_stream_sensor_iso_range, 4, u32);
camera_getter_alias!(
    destack_host_android_camera_stream_white_balance_kelvin,
    5,
    u32
);
camera_setter_alias!(
    destack_host_android_camera_stream_set_white_balance_kelvin,
    5,
    u32,
    u32
);
camera_range_alias!(
    destack_host_android_camera_stream_white_balance_range,
    5,
    u32
);
camera_getter_alias!(
    destack_host_android_camera_stream_white_balance_mode,
    6,
    u32
);
camera_setter_alias!(
    destack_host_android_camera_stream_set_white_balance_mode,
    6,
    u32,
    u32
);
camera_getter_alias!(
    destack_host_android_camera_stream_focus_distance_diopters,
    7,
    f64
);
camera_setter_alias!(
    destack_host_android_camera_stream_set_focus_distance_diopters,
    7,
    f64,
    f64
);
camera_range_alias!(
    destack_host_android_camera_stream_focus_distance_range,
    7,
    f64
);
camera_getter_alias!(destack_host_android_camera_stream_focus_mode, 8, u32);
camera_setter_alias!(
    destack_host_android_camera_stream_set_focus_mode,
    8,
    u32,
    u32
);
camera_getter_alias!(destack_host_android_camera_stream_brightness, 9, f64);
camera_setter_alias!(
    destack_host_android_camera_stream_set_brightness,
    9,
    f64,
    f64
);
camera_range_alias!(destack_host_android_camera_stream_brightness_range, 9, f64);
camera_getter_alias!(destack_host_android_camera_stream_contrast, 10, f64);
camera_setter_alias!(
    destack_host_android_camera_stream_set_contrast,
    10,
    f64,
    f64
);
camera_range_alias!(destack_host_android_camera_stream_contrast_range, 10, f64);
camera_getter_alias!(destack_host_android_camera_stream_saturation, 11, f64);
camera_setter_alias!(
    destack_host_android_camera_stream_set_saturation,
    11,
    f64,
    f64
);
camera_range_alias!(destack_host_android_camera_stream_saturation_range, 11, f64);
camera_getter_alias!(destack_host_android_camera_stream_sharpness, 12, f64);
camera_setter_alias!(
    destack_host_android_camera_stream_set_sharpness,
    12,
    f64,
    f64
);
camera_range_alias!(destack_host_android_camera_stream_sharpness_range, 12, f64);
camera_getter_alias!(destack_host_android_camera_stream_pan_degrees, 13, f64);
camera_setter_alias!(
    destack_host_android_camera_stream_set_pan_degrees,
    13,
    f64,
    f64
);
camera_range_alias!(destack_host_android_camera_stream_pan_range, 13, f64);
camera_getter_alias!(destack_host_android_camera_stream_tilt_degrees, 14, f64);
camera_setter_alias!(
    destack_host_android_camera_stream_set_tilt_degrees,
    14,
    f64,
    f64
);
camera_range_alias!(destack_host_android_camera_stream_tilt_range, 14, f64);
camera_getter_alias!(destack_host_android_camera_stream_zoom_ratio, 15, f64);
camera_setter_alias!(
    destack_host_android_camera_stream_set_zoom_ratio,
    15,
    f64,
    f64
);
camera_range_alias!(destack_host_android_camera_stream_zoom_ratio_range, 15, f64);
camera_getter_alias!(
    destack_host_android_camera_stream_stabilization_mode,
    16,
    u32
);
camera_setter_alias!(
    destack_host_android_camera_stream_set_stabilization_mode,
    16,
    u32,
    u32
);
camera_getter_alias!(destack_host_android_camera_stream_torch_mode, 17, u32);
camera_setter_alias!(
    destack_host_android_camera_stream_set_torch_mode,
    17,
    u32,
    u32
);
