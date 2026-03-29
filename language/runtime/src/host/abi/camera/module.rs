use crate::host::abi::describe::{
    HostAbiModule, HostAbiSelectorControl, HostAbiType, host_abi_module,
};

host_abi_module! {
    fn host_abi_module_base() -> "camera" {
        platforms: [android];
        types: super::types::host_abi_types();
        requests {
            /// List one slice of host camera devices.
            fn device_list(
                session_handle: session_handle,
                devices: slice(AndroidHostCameraDeviceDescriptorHeader),
                device_count_written: output(u32),
                string_bytes: slice(u8),
                string_bytes_written: output(u32),
            ) -> host_status;

            /// Open one host camera device session.
            fn device_open(
                session_handle: session_handle,
                id: string_ref,
                session_id: output(u64),
            ) -> host_status;

            /// Close one host camera device session.
            fn device_close(
                session_handle: session_handle,
                session_id: u64,
            ) -> host_status;

            /// List one host camera stream-config slice.
            fn stream_config_list(
                session_handle: session_handle,
                session_id: u64,
                configs: slice(AndroidHostCameraStreamConfigHeader),
                config_count_written: output(u32),
            ) -> host_status;

            /// List one host camera stream-capability slice.
            fn stream_capability_list(
                session_handle: session_handle,
                session_id: u64,
                capabilities: slice(AndroidHostCameraStreamCapabilityHeader),
                capability_count_written: output(u32),
            ) -> host_status;

            /// Open one host camera stream session.
            fn stream_open(
                session_handle: session_handle,
                session_id: u64,
                config: AndroidHostCameraStreamConfigHeader,
                stream_id: output(u64),
            ) -> host_status;

            /// Close one host camera stream session.
            fn stream_close(
                session_handle: session_handle,
                stream_id: u64,
            ) -> host_status;

            /// Start one host camera stream session.
            fn stream_start(
                session_handle: session_handle,
                stream_id: u64,
            ) -> host_status;

            /// Stop one host camera stream session.
            fn stream_stop(
                session_handle: session_handle,
                stream_id: u64,
            ) -> host_status;

            /// Read one host camera frame.
            fn stream_read(
                session_handle: session_handle,
                stream_id: u64,
                timeout_ns: u64,
                header: output(AndroidHostCameraFrameHeader),
                bytes: slice(u8),
                bytes_written: output(u32),
            ) -> host_status;

            /// Try one nonblocking host camera frame read.
            fn stream_try_read(
                session_handle: session_handle,
                stream_id: u64,
                header: output(AndroidHostCameraFrameHeader),
                bytes: slice(u8),
                bytes_written: output(u32),
            ) -> host_status;

            /// Capture one host camera still photo.
            fn stream_take_photo(
                session_handle: session_handle,
                stream_id: u64,
                timeout_ns: u64,
                header: output(AndroidHostCameraFrameHeader),
                bytes: slice(u8),
                bytes_written: output(u32),
            ) -> host_status;

            /// Query one active host camera stream config.
            fn stream_config(
                session_handle: session_handle,
                stream_id: u64,
                config: output(AndroidHostCameraStreamConfigHeader),
            ) -> host_status;

            /// Query one host camera recording capability descriptor.
            fn stream_recording_capabilities(
                session_handle: session_handle,
                stream_id: u64,
                capabilities: output(AndroidHostCameraRecordingCapabilitiesHeader),
            ) -> host_status;

            /// Start one host camera recording session.
            fn stream_start_recording(
                session_handle: session_handle,
                stream_id: u64,
                options: AndroidHostCameraRecordingOptionsHeader,
                output_path: string_ref,
            ) -> host_status;

            /// Pause one active host camera recording session.
            fn stream_pause_recording(
                session_handle: session_handle,
                stream_id: u64,
            ) -> host_status;

            /// Resume one paused host camera recording session.
            fn stream_resume_recording(
                session_handle: session_handle,
                stream_id: u64,
            ) -> host_status;

            /// Stop one active host camera recording session.
            fn stream_stop_recording(
                session_handle: session_handle,
                stream_id: u64,
                timeout_ns: u64,
            ) -> host_status;

            /// Read one `u64` host camera control value.
            fn stream_get_u64(
                session_handle: session_handle,
                stream_id: u64,
                selector: u32,
                value: output(u64),
            ) -> host_status;

            /// Write one `u64` host camera control value.
            fn stream_set_u64(
                session_handle: session_handle,
                stream_id: u64,
                selector: u32,
                value: u64,
            ) -> host_status;

            /// Read one `u32` host camera control value.
            fn stream_get_u32(
                session_handle: session_handle,
                stream_id: u64,
                selector: u32,
                value: output(u32),
            ) -> host_status;

            /// Write one `u32` host camera control value.
            fn stream_set_u32(
                session_handle: session_handle,
                stream_id: u64,
                selector: u32,
                value: u32,
            ) -> host_status;

            /// Read one `f64` host camera control value.
            fn stream_get_f64(
                session_handle: session_handle,
                stream_id: u64,
                selector: u32,
                value: output(f64),
            ) -> host_status;

            /// Write one `f64` host camera control value.
            fn stream_set_f64(
                session_handle: session_handle,
                stream_id: u64,
                selector: u32,
                value: f64,
            ) -> host_status;

            /// Read one `f64` host camera control range.
            fn stream_get_range_f64(
                session_handle: session_handle,
                stream_id: u64,
                selector: u32,
                minimum: output(f64),
                maximum: output(f64),
                step: output(f64),
            ) -> host_status;

            /// Read one `u64` host camera control range.
            fn stream_get_range_u64(
                session_handle: session_handle,
                stream_id: u64,
                selector: u32,
                minimum: output(u64),
                maximum: output(u64),
                step: output(u64),
            ) -> host_status;

            /// Read one `u32` host camera control range.
            fn stream_get_range_u32(
                session_handle: session_handle,
                stream_id: u64,
                selector: u32,
                minimum: output(u32),
                maximum: output(u32),
                step: output(u32),
            ) -> host_status;
        }
        ingress {}
    }
}

/// Return the authored camera ABI declaration.
pub fn host_abi_module() -> HostAbiModule {
    let mut module = host_abi_module_base();

    module.selector_controls = vec![
        selector_control(
            "stream_exposure_compensation",
            Some("stream_set_exposure_compensation"),
            Some("stream_exposure_compensation_range"),
            1,
            HostAbiType::F64,
        ),
        selector_control(
            "stream_exposure_mode",
            Some("stream_set_exposure_mode"),
            None,
            2,
            HostAbiType::U32,
        ),
        selector_control(
            "stream_exposure_time_ns",
            Some("stream_set_exposure_time_ns"),
            Some("stream_exposure_time_range"),
            3,
            HostAbiType::U64,
        ),
        selector_control(
            "stream_sensor_iso",
            Some("stream_set_sensor_iso"),
            Some("stream_sensor_iso_range"),
            4,
            HostAbiType::U32,
        ),
        selector_control(
            "stream_white_balance_kelvin",
            Some("stream_set_white_balance_kelvin"),
            Some("stream_white_balance_range"),
            5,
            HostAbiType::U32,
        ),
        selector_control(
            "stream_white_balance_mode",
            Some("stream_set_white_balance_mode"),
            None,
            6,
            HostAbiType::U32,
        ),
        selector_control(
            "stream_focus_distance_diopters",
            Some("stream_set_focus_distance_diopters"),
            Some("stream_focus_distance_range"),
            7,
            HostAbiType::F64,
        ),
        selector_control(
            "stream_focus_mode",
            Some("stream_set_focus_mode"),
            None,
            8,
            HostAbiType::U32,
        ),
        selector_control(
            "stream_brightness",
            Some("stream_set_brightness"),
            Some("stream_brightness_range"),
            9,
            HostAbiType::F64,
        ),
        selector_control(
            "stream_contrast",
            Some("stream_set_contrast"),
            Some("stream_contrast_range"),
            10,
            HostAbiType::F64,
        ),
        selector_control(
            "stream_saturation",
            Some("stream_set_saturation"),
            Some("stream_saturation_range"),
            11,
            HostAbiType::F64,
        ),
        selector_control(
            "stream_sharpness",
            Some("stream_set_sharpness"),
            Some("stream_sharpness_range"),
            12,
            HostAbiType::F64,
        ),
        selector_control(
            "stream_pan_degrees",
            Some("stream_set_pan_degrees"),
            Some("stream_pan_range"),
            13,
            HostAbiType::F64,
        ),
        selector_control(
            "stream_tilt_degrees",
            Some("stream_set_tilt_degrees"),
            Some("stream_tilt_range"),
            14,
            HostAbiType::F64,
        ),
        selector_control(
            "stream_zoom_ratio",
            Some("stream_set_zoom_ratio"),
            Some("stream_zoom_ratio_range"),
            15,
            HostAbiType::F64,
        ),
        selector_control(
            "stream_stabilization_mode",
            Some("stream_set_stabilization_mode"),
            None,
            16,
            HostAbiType::U32,
        ),
        selector_control(
            "stream_torch_mode",
            Some("stream_set_torch_mode"),
            None,
            17,
            HostAbiType::U32,
        ),
    ];

    module
}

/// Build one selector-backed camera control declaration.
fn selector_control(
    getter_export_stem: &'static str,
    setter_export_stem: Option<&'static str>,
    range_export_stem: Option<&'static str>,
    selector: u32,
    value_ty: HostAbiType,
) -> HostAbiSelectorControl {
    HostAbiSelectorControl {
        getter_export_stem,
        setter_export_stem,
        range_export_stem,
        selector,
        value_ty,
    }
}
