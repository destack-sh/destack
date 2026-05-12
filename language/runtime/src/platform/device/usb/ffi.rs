use super::core::*;

/// Return common soname candidates for the host libusb runtime.
#[cfg(target_os = "macos")]
pub(super) fn libusb_library_candidates() -> &'static [&'static str] {
    &["libusb-1.0.local_id.dylib", "libusb-1.0.dylib"]
}

/// Return common soname candidates for the host libusb runtime.
#[cfg(all(unix, not(any(target_vendor = "apple", target_os = "android"))))]
pub(super) fn libusb_library_candidates() -> &'static [&'static str] {
    &["libusb-1.0.so.0", "libusb-1.0.so"]
}

/// Return common soname candidates for the host libusb runtime.
#[cfg(target_os = "android")]
pub(super) fn libusb_library_candidates() -> &'static [&'static str] {
    &["libusb-1.0.so", "libusb-1.0.so.0", "libusb1.0.so"]
}

/// Return common soname candidates for the host libusb runtime.
#[cfg(windows)]
pub(super) fn libusb_library_candidates() -> &'static [&'static str] {
    &["libusb-1.0.dll"]
}

/// Return common soname candidates for the host libusb runtime.
#[cfg(target_os = "ios")]
pub(super) fn libusb_library_candidates() -> &'static [&'static str] {
    &[]
}

/// Return common soname candidates for the host libusb runtime.
#[cfg(not(any(unix, windows, target_os = "android", target_os = "ios")))]
pub(super) fn libusb_library_candidates() -> &'static [&'static str] {
    &[]
}

/// Define one libusb calling-convention function pointer.
macro_rules! extern_libusb_fn {
    (fn($($argument:ty),* $(,)?) -> $output:ty) => {
        extern_libusb! { fn($($argument),*) -> $output }
    };
    (fn($($argument:ty),* $(,)?)) => {
        extern_libusb! { fn($($argument),*) }
    };
}

#[cfg(windows)]
macro_rules! extern_libusb {
    (fn($($argument:ty),* $(,)?) -> $output:ty) => {
        unsafe extern "system" fn($($argument),*) -> $output
    };
    (fn($($argument:ty),* $(,)?)) => {
        unsafe extern "system" fn($($argument),*)
    };
}

#[cfg(not(windows))]
macro_rules! extern_libusb {
    (fn($($argument:ty),* $(,)?) -> $output:ty) => {
        unsafe extern "C" fn($($argument),*) -> $output
    };
    (fn($($argument:ty),* $(,)?)) => {
        unsafe extern "C" fn($($argument),*)
    };
}

/// One loaded libusb api table.
#[derive(Debug, Clone, Copy)]
pub(super) struct LibusbApi {
    /// `libusb_init`
    pub(crate) libusb_init: extern_libusb_fn! { fn(*mut *mut ffi::LibusbContext) -> c_int },
    /// `libusb_exit`
    pub(crate) libusb_exit: extern_libusb_fn! { fn(*mut ffi::LibusbContext) },
    /// `libusb_error_name`
    pub(crate) libusb_error_name: extern_libusb_fn! { fn(c_int) -> *const c_char },
    /// `libusb_get_device_list`
    pub(crate) libusb_get_device_list: extern_libusb_fn! { fn(*mut ffi::LibusbContext, *mut *const *mut ffi::LibusbDevice) -> isize },
    /// `libusb_free_device_list`
    pub(crate) libusb_free_device_list: extern_libusb_fn! { fn(*const *mut ffi::LibusbDevice, c_int) },
    /// `libusb_ref_device`
    pub(crate) libusb_ref_device: extern_libusb_fn! { fn(*mut ffi::LibusbDevice) -> *mut ffi::LibusbDevice },
    /// `libusb_unref_device`
    pub(crate) libusb_unref_device: extern_libusb_fn! { fn(*mut ffi::LibusbDevice) },
    /// `libusb_get_device_descriptor`
    pub(crate) libusb_get_device_descriptor: extern_libusb_fn! { fn(*mut ffi::LibusbDevice, *mut ffi::LibusbDeviceDescriptor) -> c_int },
    /// `libusb_get_config_descriptor`
    pub(crate) libusb_get_config_descriptor: extern_libusb_fn! { fn(*mut ffi::LibusbDevice, u8, *mut *mut ffi::LibusbConfigDescriptor) -> c_int },
    /// `libusb_free_config_descriptor`
    pub(crate) libusb_free_config_descriptor: extern_libusb_fn! { fn(*mut ffi::LibusbConfigDescriptor) },
    /// `libusb_get_bos_descriptor`
    pub(crate) libusb_get_bos_descriptor: extern_libusb_fn! { fn(*mut ffi::LibusbDeviceHandle, *mut *mut ffi::LibusbBosDescriptor) -> c_int },
    /// `libusb_free_bos_descriptor`
    pub(crate) libusb_free_bos_descriptor: extern_libusb_fn! { fn(*mut ffi::LibusbBosDescriptor) },
    /// `libusb_get_usb_2_0_extension_descriptor`
    pub(crate) libusb_get_usb_2_0_extension_descriptor: extern_libusb_fn! { fn(*mut ffi::LibusbContext, *mut ffi::LibusbBosDevCapabilityDescriptor, *mut *mut ffi::LibusbUsb20ExtensionDescriptor) -> c_int },
    /// `libusb_free_usb_2_0_extension_descriptor`
    pub(crate) libusb_free_usb_2_0_extension_descriptor: extern_libusb_fn! { fn(*mut ffi::LibusbUsb20ExtensionDescriptor) },
    /// `libusb_get_ss_usb_device_capability_descriptor`
    pub(crate) libusb_get_ss_usb_device_capability_descriptor: extern_libusb_fn! { fn(*mut ffi::LibusbContext, *mut ffi::LibusbBosDevCapabilityDescriptor, *mut *mut ffi::LibusbSsUsbDeviceCapabilityDescriptor) -> c_int },
    /// `libusb_free_ss_usb_device_capability_descriptor`
    pub(crate) libusb_free_ss_usb_device_capability_descriptor: extern_libusb_fn! { fn(*mut ffi::LibusbSsUsbDeviceCapabilityDescriptor) },
    /// `libusb_get_container_id_descriptor`
    pub(crate) libusb_get_container_id_descriptor: extern_libusb_fn! { fn(*mut ffi::LibusbContext, *mut ffi::LibusbBosDevCapabilityDescriptor, *mut *mut ffi::LibusbContainerIdDescriptor) -> c_int },
    /// `libusb_free_container_id_descriptor`
    pub(crate) libusb_free_container_id_descriptor: extern_libusb_fn! { fn(*mut ffi::LibusbContainerIdDescriptor) },
    /// `libusb_get_platform_descriptor`
    pub(crate) libusb_get_platform_descriptor: extern_libusb_fn! { fn(*mut ffi::LibusbContext, *mut ffi::LibusbBosDevCapabilityDescriptor, *mut *mut ffi::LibusbPlatformDescriptor) -> c_int },
    /// `libusb_free_platform_descriptor`
    pub(crate) libusb_free_platform_descriptor: extern_libusb_fn! { fn(*mut ffi::LibusbPlatformDescriptor) },
    /// `libusb_get_bus_number`
    pub(crate) libusb_get_bus_number: extern_libusb_fn! { fn(*mut ffi::LibusbDevice) -> u8 },
    /// `libusb_get_port_numbers`
    pub(crate) libusb_get_port_numbers: extern_libusb_fn! { fn(*mut ffi::LibusbDevice, *mut u8, c_int) -> c_int },
    /// `libusb_get_device_speed`
    pub(crate) libusb_get_device_speed: extern_libusb_fn! { fn(*mut ffi::LibusbDevice) -> c_int },
    /// `libusb_open`
    pub(crate) libusb_open: extern_libusb_fn! { fn(*mut ffi::LibusbDevice, *mut *mut ffi::LibusbDeviceHandle) -> c_int },
    /// `libusb_close`
    pub(crate) libusb_close: extern_libusb_fn! { fn(*mut ffi::LibusbDeviceHandle) },

    // android raw host handoff
    /// `libusb_wrap_sys_device`
    #[cfg(target_os = "android")]
    pub(crate) libusb_wrap_sys_device: Option<
        extern_libusb_fn! { fn(*mut ffi::LibusbContext, isize, *mut *mut ffi::LibusbDeviceHandle) -> c_int },
    >,
    /// `libusb_get_device`
    #[cfg(target_os = "android")]
    pub(crate) libusb_get_device: extern_libusb_fn! { fn(*mut ffi::LibusbDeviceHandle) -> *mut ffi::LibusbDevice },
    /// `libusb_get_configuration`
    pub(crate) libusb_get_configuration: extern_libusb_fn! { fn(*mut ffi::LibusbDeviceHandle, *mut c_int) -> c_int },
    /// `libusb_set_configuration`
    pub(crate) libusb_set_configuration: extern_libusb_fn! { fn(*mut ffi::LibusbDeviceHandle, c_int) -> c_int },
    /// `libusb_claim_interface`
    pub(crate) libusb_claim_interface: extern_libusb_fn! { fn(*mut ffi::LibusbDeviceHandle, c_int) -> c_int },
    /// `libusb_release_interface`
    pub(crate) libusb_release_interface: extern_libusb_fn! { fn(*mut ffi::LibusbDeviceHandle, c_int) -> c_int },
    /// `libusb_set_interface_alt_setting`
    pub(crate) libusb_set_interface_alt_setting: extern_libusb_fn! { fn(*mut ffi::LibusbDeviceHandle, c_int, c_int) -> c_int },
    /// `libusb_clear_halt`
    pub(crate) libusb_clear_halt: extern_libusb_fn! { fn(*mut ffi::LibusbDeviceHandle, c_uchar) -> c_int },
    /// `libusb_reset_device`
    pub(crate) libusb_reset_device: extern_libusb_fn! { fn(*mut ffi::LibusbDeviceHandle) -> c_int },
    /// `libusb_control_transfer`
    pub(crate) libusb_control_transfer: extern_libusb_fn! { fn(*mut ffi::LibusbDeviceHandle, c_uchar, c_uchar, u16, u16, *mut u8, u16, c_uint) -> c_int },
    /// `libusb_alloc_transfer`
    pub(crate) libusb_alloc_transfer: extern_libusb_fn! { fn(c_int) -> *mut ffi::LibusbTransfer },
    /// `libusb_submit_transfer`
    pub(crate) libusb_submit_transfer: extern_libusb_fn! { fn(*mut ffi::LibusbTransfer) -> c_int },
    /// `libusb_cancel_transfer`
    pub(crate) libusb_cancel_transfer: extern_libusb_fn! { fn(*mut ffi::LibusbTransfer) -> c_int },
    /// `libusb_free_transfer`
    pub(crate) libusb_free_transfer: extern_libusb_fn! { fn(*mut ffi::LibusbTransfer) },
    /// `libusb_get_string_descriptor`
    pub(crate) libusb_get_string_descriptor: extern_libusb_fn! { fn(*mut ffi::LibusbDeviceHandle, u8, u16, *mut u8, c_int) -> c_int },
    /// `libusb_has_capability`
    pub(crate) libusb_has_capability: extern_libusb_fn! { fn(u32) -> c_int },
    /// `libusb_hotplug_register_callback`
    pub(crate) libusb_hotplug_register_callback: extern_libusb_fn! { fn(*mut ffi::LibusbContext, c_int, c_int, c_int, c_int, c_int, Option<LibusbHotplugCallback>, *mut c_void, *mut ffi::LibusbHotplugCallbackHandle) -> c_int },
    /// `libusb_hotplug_deregister_callback`
    pub(crate) libusb_hotplug_deregister_callback: extern_libusb_fn! { fn(*mut ffi::LibusbContext, ffi::LibusbHotplugCallbackHandle) },
    /// `libusb_handle_events_timeout_completed`
    pub(crate) libusb_handle_events_timeout_completed: extern_libusb_fn! { fn(*mut ffi::LibusbContext, *const ffi::LibusbTimeval, *mut c_int) -> c_int },
}

/// One Windows libusb transfer callback signature.
#[cfg(windows)]
type LibusbTransferCallback = unsafe extern "system" fn(*mut ffi::LibusbTransfer);

/// One non-Windows libusb transfer callback signature.
#[cfg(not(windows))]
type LibusbTransferCallback = unsafe extern "C" fn(*mut ffi::LibusbTransfer);

/// One Windows libusb hotplug callback signature.
#[cfg(windows)]
type LibusbHotplugCallback = unsafe extern "system" fn(
    *mut ffi::LibusbContext,
    *mut ffi::LibusbDevice,
    c_int,
    *mut c_void,
) -> c_int;

/// One non-Windows libusb hotplug callback signature.
#[cfg(not(windows))]
type LibusbHotplugCallback = unsafe extern "C" fn(
    *mut ffi::LibusbContext,
    *mut ffi::LibusbDevice,
    c_int,
    *mut c_void,
) -> c_int;

/// Minimal libusb ffi surface.
pub(super) mod ffi {
    #![allow(dead_code)]
    #![allow(unreachable_pub)]

    use super::{LibusbTransferCallback, c_int, c_uint, c_void};

    /// `libusb_context`
    #[repr(C)]
    pub struct LibusbContext {
        _private: [u8; 0],
    }

    /// `libusb_device`
    #[repr(C)]
    pub struct LibusbDevice {
        _private: [u8; 0],
    }

    /// `libusb_device_handle`
    #[repr(C)]
    pub struct LibusbDeviceHandle {
        _private: [u8; 0],
    }

    /// `struct timeval`
    #[repr(C)]
    #[derive(Debug, Clone, Copy)]
    pub struct LibusbTimeval {
        pub tv_sec: i64,
        pub tv_usec: i64,
    }

    /// `libusb_hotplug_callback_handle`
    pub type LibusbHotplugCallbackHandle = c_int;

    /// `struct libusb_device_descriptor`
    #[repr(C)]
    #[derive(Debug, Clone, Copy)]
    pub struct LibusbDeviceDescriptor {
        pub b_length: u8,
        pub b_descriptor_type: u8,
        pub bcd_usb: u16,
        pub b_device_class: u8,
        pub b_device_sub_class: u8,
        pub b_device_protocol: u8,
        pub b_max_packet_size0: u8,
        pub id_vendor: u16,
        pub id_product: u16,
        pub bcd_device: u16,
        pub i_manufacturer: u8,
        pub i_product: u8,
        pub i_serial_number: u8,
        pub b_num_configurations: u8,
    }

    /// `struct libusb_endpoint_descriptor`
    #[repr(C)]
    #[derive(Debug, Clone, Copy)]
    pub struct LibusbEndpointDescriptor {
        pub b_length: u8,
        pub b_descriptor_type: u8,
        pub b_endpoint_address: u8,
        pub bm_attributes: u8,
        pub w_max_packet_size: u16,
        pub b_interval: u8,
        pub b_refresh: u8,
        pub b_synch_address: u8,
        pub extra: *const u8,
        pub extra_length: c_int,
    }

    /// `struct libusb_interface_descriptor`
    #[repr(C)]
    #[derive(Debug, Clone, Copy)]
    pub struct LibusbInterfaceDescriptor {
        pub b_length: u8,
        pub b_descriptor_type: u8,
        pub b_interface_number: u8,
        pub b_alternate_setting: u8,
        pub b_num_endpoints: u8,
        pub b_interface_class: u8,
        pub b_interface_sub_class: u8,
        pub b_interface_protocol: u8,
        pub i_interface: u8,
        pub endpoint: *const LibusbEndpointDescriptor,
        pub extra: *const u8,
        pub extra_length: c_int,
    }

    /// `struct libusb_interface`
    #[repr(C)]
    #[derive(Debug, Clone, Copy)]
    pub struct LibusbInterface {
        pub altsetting: *const LibusbInterfaceDescriptor,
        pub num_altsetting: c_int,
    }

    /// `struct libusb_config_descriptor`
    #[repr(C)]
    #[derive(Debug, Clone, Copy)]
    pub struct LibusbConfigDescriptor {
        pub b_length: u8,
        pub b_descriptor_type: u8,
        pub w_total_length: u16,
        pub b_num_interfaces: u8,
        pub b_configuration_value: u8,
        pub i_configuration: u8,
        pub bm_attributes: u8,
        pub max_power: u8,
        pub interface: *const LibusbInterface,
        pub extra: *const u8,
        pub extra_length: c_int,
    }

    /// `struct libusb_bos_dev_capability_descriptor`
    #[repr(C)]
    #[derive(Debug, Clone, Copy)]
    pub struct LibusbBosDevCapabilityDescriptor {
        pub b_length: u8,
        pub b_descriptor_type: u8,
        pub b_dev_capability_type: u8,
        pub dev_capability_data: [u8; 0],
    }

    /// `struct libusb_bos_descriptor`
    #[repr(C)]
    #[derive(Debug, Clone, Copy)]
    pub struct LibusbBosDescriptor {
        pub b_length: u8,
        pub b_descriptor_type: u8,
        pub w_total_length: u16,
        pub b_num_device_caps: u8,
        pub dev_capability: [*mut LibusbBosDevCapabilityDescriptor; 0],
    }

    /// `struct libusb_usb_2_0_extension_descriptor`
    #[repr(C)]
    #[derive(Debug, Clone, Copy)]
    pub struct LibusbUsb20ExtensionDescriptor {
        pub b_length: u8,
        pub b_descriptor_type: u8,
        pub b_dev_capability_type: u8,
        pub bm_attributes: u32,
    }

    /// `struct libusb_ss_usb_device_capability_descriptor`
    #[repr(C)]
    #[derive(Debug, Clone, Copy)]
    pub struct LibusbSsUsbDeviceCapabilityDescriptor {
        pub b_length: u8,
        pub b_descriptor_type: u8,
        pub b_dev_capability_type: u8,
        pub bm_attributes: u8,
        pub w_speed_supported: u16,
        pub b_functionality_support: u8,
        pub b_u1_dev_exit_lat: u8,
        pub b_u2_dev_exit_lat: u16,
    }

    /// `struct libusb_container_id_descriptor`
    #[repr(C)]
    #[derive(Debug, Clone, Copy)]
    pub struct LibusbContainerIdDescriptor {
        pub b_length: u8,
        pub b_descriptor_type: u8,
        pub b_dev_capability_type: u8,
        pub b_reserved: u8,
        pub container_id: [u8; 16],
    }

    /// `struct libusb_platform_descriptor`
    #[repr(C)]
    #[derive(Debug, Clone, Copy)]
    pub struct LibusbPlatformDescriptor {
        pub b_length: u8,
        pub b_descriptor_type: u8,
        pub b_dev_capability_type: u8,
        pub b_reserved: u8,
        pub platform_capability_uuid: [u8; 16],
        pub capability_data: [u8; 0],
    }

    /// `struct libusb_iso_packet_descriptor`
    #[repr(C)]
    #[derive(Debug, Clone, Copy)]
    pub struct LibusbIsoPacketDescriptor {
        pub length: c_uint,
        pub actual_length: c_uint,
        pub status: c_int,
    }

    /// `struct libusb_transfer`
    #[repr(C)]
    pub struct LibusbTransfer {
        pub dev_handle: *mut LibusbDeviceHandle,
        pub flags: u8,
        pub endpoint: u8,
        pub transfer_type: u8,
        pub timeout: c_uint,
        pub status: c_int,
        pub length: c_int,
        pub actual_length: c_int,
        pub callback: Option<LibusbTransferCallback>,
        pub user_data: *mut c_void,
        pub buffer: *mut u8,
        pub num_iso_packets: c_int,
        pub iso_packet_desc: [LibusbIsoPacketDescriptor; 0],
    }

    pub const LIBUSB_SUCCESS: c_int = 0;
    pub const LIBUSB_ERROR_IO: c_int = -1;
    pub const LIBUSB_ERROR_INVALID_PARAM: c_int = -2;
    pub const LIBUSB_ERROR_ACCESS: c_int = -3;
    pub const LIBUSB_ERROR_NO_DEVICE: c_int = -4;
    pub const LIBUSB_ERROR_NOT_FOUND: c_int = -5;
    pub const LIBUSB_ERROR_BUSY: c_int = -6;
    pub const LIBUSB_ERROR_TIMEOUT: c_int = -7;
    pub const LIBUSB_ERROR_OVERFLOW: c_int = -8;
    pub const LIBUSB_ERROR_PIPE: c_int = -9;
    pub const LIBUSB_ERROR_INTERRUPTED: c_int = -10;
    pub const LIBUSB_ERROR_NO_MEM: c_int = -11;
    pub const LIBUSB_ERROR_NOT_SUPPORTED: c_int = -12;
    pub const LIBUSB_ERROR_OTHER: c_int = -99;

    pub const LIBUSB_SPEED_SUPER: c_int = 4;
    pub const LIBUSB_SPEED_SUPER_PLUS: c_int = 5;
    pub const LIBUSB_SPEED_SUPER_PLUS_X2: c_int = 6;

    pub const LIBUSB_TRANSFER_TYPE_ISOCHRONOUS: u8 = 1;
    pub const LIBUSB_TRANSFER_TYPE_BULK: u8 = 2;
    pub const LIBUSB_TRANSFER_TYPE_INTERRUPT: u8 = 3;

    pub const LIBUSB_TRANSFER_COMPLETED: c_int = 0;
    pub const LIBUSB_TRANSFER_TIMED_OUT: c_int = 2;
    pub const LIBUSB_TRANSFER_CANCELLED: c_int = 3;
    pub const LIBUSB_TRANSFER_STALL: c_int = 4;
    pub const LIBUSB_TRANSFER_NO_DEVICE: c_int = 5;
    pub const LIBUSB_TRANSFER_OVERFLOW: c_int = 6;

    pub const LIBUSB_ENDPOINT_IN: u8 = 0x80;
    pub const LIBUSB_ENDPOINT_OUT: u8 = 0x00;

    pub const LIBUSB_REQUEST_TYPE_STANDARD: u8 = 0x00 << 5;
    pub const LIBUSB_REQUEST_TYPE_CLASS: u8 = 0x01 << 5;
    pub const LIBUSB_REQUEST_TYPE_VENDOR: u8 = 0x02 << 5;

    pub const LIBUSB_RECIPIENT_DEVICE: u8 = 0x00;
    pub const LIBUSB_RECIPIENT_INTERFACE: u8 = 0x01;
    pub const LIBUSB_RECIPIENT_ENDPOINT: u8 = 0x02;
    pub const LIBUSB_RECIPIENT_OTHER: u8 = 0x03;

    pub const LIBUSB_ENDPOINT_TRANSFER_TYPE_MASK: u8 = 0x03;

    pub const LIBUSB_TRANSFER_FREE_BUFFER: u8 = 1 << 1;

    pub const LIBUSB_CAP_HAS_HOTPLUG: u32 = 0x0001;
    pub const LIBUSB_HOTPLUG_EVENT_DEVICE_ARRIVED: c_int = 1 << 0;
    pub const LIBUSB_HOTPLUG_EVENT_DEVICE_LEFT: c_int = 1 << 1;
    pub const LIBUSB_HOTPLUG_NO_FLAGS: c_int = 0;
    pub const LIBUSB_HOTPLUG_MATCH_ANY: c_int = -1;

    pub const LIBUSB_BT_USB_2_0_EXTENSION: u8 = 2;
    pub const LIBUSB_BT_SS_USB_DEVICE_CAPABILITY: u8 = 3;
    pub const LIBUSB_BT_CONTAINER_ID: u8 = 4;
    pub const LIBUSB_BT_PLATFORM_DESCRIPTOR: u8 = 5;

    pub const USB_CONFIG_SELF_POWERED: u8 = 0x40;
    pub const USB_CONFIG_REMOTE_WAKEUP: u8 = 0x20;
}

/// Load one libusb api table from one dynamic library candidate.
pub(super) fn load_libraryusb_api(
    library: &DynamicLibrary,
    candidate: &str,
) -> Result<LibusbApi, String> {
    macro_rules! load {
        ($name:literal) => {
            library.load_required_symbol(candidate, concat!($name, "\0").as_bytes())
        };
    }

    Ok(LibusbApi {
        libusb_init: load!("libusb_init")?,
        libusb_exit: load!("libusb_exit")?,
        libusb_error_name: load!("libusb_error_name")?,
        libusb_get_device_list: load!("libusb_get_device_list")?,
        libusb_free_device_list: load!("libusb_free_device_list")?,
        libusb_ref_device: load!("libusb_ref_device")?,
        libusb_unref_device: load!("libusb_unref_device")?,
        libusb_get_device_descriptor: load!("libusb_get_device_descriptor")?,
        libusb_get_config_descriptor: load!("libusb_get_config_descriptor")?,
        libusb_free_config_descriptor: load!("libusb_free_config_descriptor")?,
        libusb_get_bos_descriptor: load!("libusb_get_bos_descriptor")?,
        libusb_free_bos_descriptor: load!("libusb_free_bos_descriptor")?,
        libusb_get_usb_2_0_extension_descriptor: load!("libusb_get_usb_2_0_extension_descriptor")?,
        libusb_free_usb_2_0_extension_descriptor: load!(
            "libusb_free_usb_2_0_extension_descriptor"
        )?,
        libusb_get_ss_usb_device_capability_descriptor: load!(
            "libusb_get_ss_usb_device_capability_descriptor"
        )?,
        libusb_free_ss_usb_device_capability_descriptor: load!(
            "libusb_free_ss_usb_device_capability_descriptor"
        )?,
        libusb_get_container_id_descriptor: load!("libusb_get_container_id_descriptor")?,
        libusb_free_container_id_descriptor: load!("libusb_free_container_id_descriptor")?,
        libusb_get_platform_descriptor: load!("libusb_get_platform_descriptor")?,
        libusb_free_platform_descriptor: load!("libusb_free_platform_descriptor")?,
        libusb_get_bus_number: load!("libusb_get_bus_number")?,
        libusb_get_port_numbers: load!("libusb_get_port_numbers")?,
        libusb_get_device_speed: load!("libusb_get_device_speed")?,
        libusb_open: load!("libusb_open")?,
        libusb_close: load!("libusb_close")?,
        libusb_get_configuration: load!("libusb_get_configuration")?,
        libusb_set_configuration: load!("libusb_set_configuration")?,
        libusb_claim_interface: load!("libusb_claim_interface")?,
        libusb_release_interface: load!("libusb_release_interface")?,
        libusb_set_interface_alt_setting: load!("libusb_set_interface_alt_setting")?,
        libusb_clear_halt: load!("libusb_clear_halt")?,
        libusb_reset_device: load!("libusb_reset_device")?,
        libusb_control_transfer: load!("libusb_control_transfer")?,
        libusb_alloc_transfer: load!("libusb_alloc_transfer")?,
        libusb_submit_transfer: load!("libusb_submit_transfer")?,
        libusb_cancel_transfer: load!("libusb_cancel_transfer")?,
        libusb_free_transfer: load!("libusb_free_transfer")?,
        libusb_get_string_descriptor: load!("libusb_get_string_descriptor")?,
        libusb_has_capability: load!("libusb_has_capability")?,
        libusb_hotplug_register_callback: load!("libusb_hotplug_register_callback")?,
        libusb_hotplug_deregister_callback: load!("libusb_hotplug_deregister_callback")?,
        libusb_handle_events_timeout_completed: load!("libusb_handle_events_timeout_completed")?,
        #[cfg(target_os = "android")]
        libusb_wrap_sys_device: library
            .load_symbol(concat!("libusb_wrap_sys_device", "\0").as_bytes())
            .ok(),
        #[cfg(target_os = "android")]
        libusb_get_device: load!("libusb_get_device")?,
    })
}
