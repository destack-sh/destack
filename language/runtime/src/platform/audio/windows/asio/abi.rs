use std::ffi::c_void;

use windows_sys::Win32::System::Com::{CLSCTX_INPROC_SERVER, CoCreateInstance};
use windows_sys::core::{GUID, HRESULT};

use super::constants::{ASE_OK, ASIO_MAX_DRIVER_NAME_BYTES, ASIO_MAX_ERROR_MESSAGE_BYTES};
use super::core::succeeded;

/// One raw ASIODriverInfo payload.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub(super) struct AsioDriverInfo {
    /// Requested host ASIO version.
    pub(super) asio_version: i32,
    /// Returned driver version field.
    pub(super) driver_version: i32,
    /// Returned driver name buffer.
    pub(super) name: [u8; ASIO_MAX_DRIVER_NAME_BYTES],
    /// Returned driver error message buffer.
    pub(super) error_message: [u8; ASIO_MAX_ERROR_MESSAGE_BYTES],
    /// Platform-specific system reference.
    pub(super) sys_ref: *mut c_void,
}

/// One raw ASIOCallbacks payload.
#[repr(C)]
#[derive(Clone, Copy)]
pub(super) struct AsioCallbacks {
    /// One callback for one buffer-switch event.
    pub(super) buffer_switch: Option<extern "system" fn(i32, i32)>,
    /// One callback for one sample-rate change event.
    pub(super) sample_rate_did_change: Option<extern "system" fn(f64)>,
    /// One callback for one generic ASIO message.
    pub(super) asio_message: Option<extern "system" fn(i32, i32, *mut c_void, *mut f64) -> i32>,
    /// One callback for one time-info buffer-switch event.
    pub(super) buffer_switch_time_info:
        Option<extern "system" fn(*mut AsioTime, i32, i32) -> *mut AsioTime>,
}

/// One raw ASIOTime payload placeholder.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub(super) struct AsioTime {
    /// Opaque bytes ignored by this runtime.
    _opaque: [u8; 0],
}

/// One raw ASIOBufferInfo payload.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub(super) struct AsioBufferInfo {
    /// Whether this row targets one input channel.
    pub(super) is_input: i32,
    /// Target channel index.
    pub(super) channel_num: i32,
    /// Driver-owned double-buffer pointers.
    pub(super) buffers: [*mut c_void; 2],
}

/// One raw ASIOChannelInfo payload.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub(super) struct AsioChannelInfo {
    /// Queried channel index.
    pub(super) channel: i32,
    /// Whether this descriptor targets one input channel.
    pub(super) is_input: i32,
    /// Whether this channel is currently active.
    pub(super) is_active: i32,
    /// Channel group identifier.
    pub(super) channel_group: i32,
    /// Channel sample type selector.
    pub(super) sample_type: i32,
    /// Driver-provided channel display name bytes.
    pub(super) name: [u8; 32],
}

#[repr(C)]
pub(super) struct AsioDriverVTable {
    /// COM QueryInterface entry.
    query_interface:
        unsafe extern "system" fn(*mut c_void, *const GUID, *mut *mut c_void) -> HRESULT,
    /// COM AddRef entry.
    add_ref: unsafe extern "system" fn(*mut c_void) -> u32,
    /// COM Release entry.
    release: unsafe extern "system" fn(*mut c_void) -> u32,
    /// IASIO init entry.
    init: unsafe extern "system" fn(*mut c_void, *mut c_void) -> i32,
    /// IASIO getDriverName entry.
    get_driver_name: unsafe extern "system" fn(*mut c_void, *mut i8),
    /// IASIO getDriverVersion entry.
    get_driver_version: unsafe extern "system" fn(*mut c_void) -> i32,
    /// IASIO getErrorMessage entry.
    get_error_message: unsafe extern "system" fn(*mut c_void, *mut i8),
    /// IASIO start entry.
    start: unsafe extern "system" fn(*mut c_void) -> i32,
    /// IASIO stop entry.
    stop: unsafe extern "system" fn(*mut c_void) -> i32,
    /// IASIO getChannels entry.
    get_channels: unsafe extern "system" fn(*mut c_void, *mut i32, *mut i32) -> i32,
    /// IASIO getLatencies entry.
    get_latencies: unsafe extern "system" fn(*mut c_void, *mut i32, *mut i32) -> i32,
    /// IASIO getBufferSize entry.
    get_buffer_size:
        unsafe extern "system" fn(*mut c_void, *mut i32, *mut i32, *mut i32, *mut i32) -> i32,
    /// IASIO canSampleRate entry.
    can_sample_rate: unsafe extern "system" fn(*mut c_void, f64) -> i32,
    /// IASIO getSampleRate entry.
    get_sample_rate: unsafe extern "system" fn(*mut c_void, *mut f64) -> i32,
    /// IASIO setSampleRate entry.
    set_sample_rate: unsafe extern "system" fn(*mut c_void, f64) -> i32,
    /// IASIO getClockSources entry.
    get_clock_sources: unsafe extern "system" fn(*mut c_void, *mut c_void, *mut i32) -> i32,
    /// IASIO setClockSource entry.
    set_clock_source: unsafe extern "system" fn(*mut c_void, i32) -> i32,
    /// IASIO getSamplePosition entry.
    get_sample_position: unsafe extern "system" fn(*mut c_void, *mut c_void, *mut c_void) -> i32,
    /// IASIO getChannelInfo entry.
    get_channel_info: unsafe extern "system" fn(*mut c_void, *mut AsioChannelInfo) -> i32,
    /// IASIO createBuffers entry.
    create_buffers: unsafe extern "system" fn(
        *mut c_void,
        *mut AsioBufferInfo,
        i32,
        i32,
        *const AsioCallbacks,
    ) -> i32,
    /// IASIO disposeBuffers entry.
    dispose_buffers: unsafe extern "system" fn(*mut c_void) -> i32,
    /// IASIO controlPanel entry.
    control_panel: unsafe extern "system" fn(*mut c_void) -> i32,
    /// IASIO future entry.
    future: unsafe extern "system" fn(*mut c_void, i32, *mut c_void) -> i32,
    /// IASIO outputReady entry.
    output_ready: unsafe extern "system" fn(*mut c_void) -> i32,
}

/// Return one IASIO vtable pointer for one raw interface.
pub(super) unsafe fn asio_driver_vtable(driver: *mut c_void) -> *const AsioDriverVTable {
    unsafe { *(driver as *const *const AsioDriverVTable) }
}

/// Release one IASIO interface pointer.
pub(super) unsafe fn asio_driver_release(driver: *mut c_void) {
    if driver.is_null() {
        return;
    }

    unsafe {
        ((*asio_driver_vtable(driver)).release)(driver);
    }
}

/// Initialize one IASIO driver instance.
pub(super) unsafe fn asio_driver_init(driver: *mut c_void, info: *mut AsioDriverInfo) -> i32 {
    unsafe { ((*asio_driver_vtable(driver)).init)(driver, info as *mut c_void) }
}

/// Exit one IASIO driver instance.
pub(super) unsafe fn asio_driver_exit(driver: *mut c_void) -> i32 {
    let _ = driver;
    ASE_OK
}

/// Read one IASIO driver name into one C buffer.
pub(super) unsafe fn asio_driver_get_name(driver: *mut c_void, name: *mut i8) {
    unsafe {
        ((*asio_driver_vtable(driver)).get_driver_name)(driver, name);
    }
}

/// Read one IASIO driver error message into one C buffer.
pub(super) unsafe fn asio_driver_get_error_message(driver: *mut c_void, message: *mut i8) {
    unsafe {
        ((*asio_driver_vtable(driver)).get_error_message)(driver, message);
    }
}

/// Start one IASIO stream engine.
pub(super) unsafe fn asio_driver_start(driver: *mut c_void) -> i32 {
    unsafe { ((*asio_driver_vtable(driver)).start)(driver) }
}

/// Stop one IASIO stream engine.
pub(super) unsafe fn asio_driver_stop(driver: *mut c_void) -> i32 {
    unsafe { ((*asio_driver_vtable(driver)).stop)(driver) }
}

/// Query one IASIO channel-count pair.
pub(super) unsafe fn asio_driver_get_channels(
    driver: *mut c_void,
    input_channels: *mut i32,
    output_channels: *mut i32,
) -> i32 {
    unsafe { ((*asio_driver_vtable(driver)).get_channels)(driver, input_channels, output_channels) }
}

/// Query one IASIO latency pair.
pub(super) unsafe fn asio_driver_get_latencies(
    driver: *mut c_void,
    input_latency: *mut i32,
    output_latency: *mut i32,
) -> i32 {
    unsafe { ((*asio_driver_vtable(driver)).get_latencies)(driver, input_latency, output_latency) }
}

/// Query one IASIO buffer-size tuple.
pub(super) unsafe fn asio_driver_get_buffer_size(
    driver: *mut c_void,
    min_size: *mut i32,
    max_size: *mut i32,
    preferred_size: *mut i32,
    granularity: *mut i32,
) -> i32 {
    unsafe {
        ((*asio_driver_vtable(driver)).get_buffer_size)(
            driver,
            min_size,
            max_size,
            preferred_size,
            granularity,
        )
    }
}

/// Query whether one sample rate is supported.
pub(super) unsafe fn asio_driver_can_sample_rate(driver: *mut c_void, sample_rate: f64) -> i32 {
    unsafe { ((*asio_driver_vtable(driver)).can_sample_rate)(driver, sample_rate) }
}

/// Query one current IASIO sample rate.
pub(super) unsafe fn asio_driver_get_sample_rate(
    driver: *mut c_void,
    sample_rate: *mut f64,
) -> i32 {
    unsafe { ((*asio_driver_vtable(driver)).get_sample_rate)(driver, sample_rate) }
}

/// Apply one IASIO sample rate.
pub(super) unsafe fn asio_driver_set_sample_rate(driver: *mut c_void, sample_rate: f64) -> i32 {
    unsafe { ((*asio_driver_vtable(driver)).set_sample_rate)(driver, sample_rate) }
}

/// Query one IASIO channel descriptor.
pub(super) unsafe fn asio_driver_get_channel_info(
    driver: *mut c_void,
    info: *mut AsioChannelInfo,
) -> i32 {
    unsafe { ((*asio_driver_vtable(driver)).get_channel_info)(driver, info) }
}

/// Create one IASIO callback buffer set.
pub(super) unsafe fn asio_driver_create_buffers(
    driver: *mut c_void,
    buffer_infos: *mut AsioBufferInfo,
    num_channels: i32,
    buffer_size: i32,
    callbacks: *const AsioCallbacks,
) -> i32 {
    unsafe {
        ((*asio_driver_vtable(driver)).create_buffers)(
            driver,
            buffer_infos,
            num_channels,
            buffer_size,
            callbacks,
        )
    }
}

/// Dispose one IASIO callback buffer set.
pub(super) unsafe fn asio_driver_dispose_buffers(driver: *mut c_void) -> i32 {
    unsafe { ((*asio_driver_vtable(driver)).dispose_buffers)(driver) }
}

/// Signal one ASIO output-ready hint.
pub(super) unsafe fn asio_driver_output_ready(driver: *mut c_void) -> i32 {
    unsafe { ((*asio_driver_vtable(driver)).output_ready)(driver) }
}

/// Activate one ASIO driver interface from one class id.
pub(super) unsafe fn asio_activate_driver(class_id: &GUID) -> Result<*mut c_void, HRESULT> {
    let mut interface_pointer = std::ptr::null_mut();
    let status = unsafe {
        CoCreateInstance(
            class_id,
            std::ptr::null_mut(),
            CLSCTX_INPROC_SERVER,
            class_id,
            &mut interface_pointer,
        )
    };

    if succeeded(status) && !interface_pointer.is_null() {
        return Ok(interface_pointer);
    }

    Err(status)
}
