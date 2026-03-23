use crate::runtime::{NativeSlice, NativeStringRef};

use super::types::{AndroidHostUsbDeviceDescriptorHeader, AndroidHostUsbHotplugEventHeader};

/// Host callback for listing one slice of Android USB devices.
pub(crate) type AndroidHostUsbDeviceListCallback = unsafe extern "C" fn(
    runtime_id: u64,
    devices: NativeSlice<AndroidHostUsbDeviceDescriptorHeader>,
    device_count_written: *mut u32,
    string_bytes: NativeSlice<u8>,
    string_bytes_written: *mut u32,
    port_bytes: NativeSlice<u8>,
    port_bytes_written: *mut u32,
) -> u32;

/// Host callback for opening one Android USB hotplug watch.
pub(crate) type AndroidHostUsbWatchOpenCallback =
    unsafe extern "C" fn(runtime_id: u64, watch_id: *mut u64) -> u32;

/// Host callback for closing one Android USB hotplug watch.
pub(crate) type AndroidHostUsbWatchCloseCallback =
    unsafe extern "C" fn(runtime_id: u64, watch_id: u64) -> u32;

/// Host callback for reading one Android USB hotplug event.
pub(crate) type AndroidHostUsbWatchReadCallback = unsafe extern "C" fn(
    runtime_id: u64,
    watch_id: u64,
    timeout_ns: u64,
    event: *mut AndroidHostUsbHotplugEventHeader,
    string_bytes: NativeSlice<u8>,
    string_bytes_written: *mut u32,
    port_bytes: NativeSlice<u8>,
    port_bytes_written: *mut u32,
) -> u32;

/// Host callback for trying one nonblocking Android USB hotplug event read.
pub(crate) type AndroidHostUsbWatchTryReadCallback = unsafe extern "C" fn(
    runtime_id: u64,
    watch_id: u64,
    event: *mut AndroidHostUsbHotplugEventHeader,
    string_bytes: NativeSlice<u8>,
    string_bytes_written: *mut u32,
    port_bytes: NativeSlice<u8>,
    port_bytes_written: *mut u32,
) -> u32;

/// Host callback for opening one permission checked Android USB device.
pub(crate) type AndroidHostUsbOpenCallback =
    unsafe extern "C" fn(runtime_id: u64, id: NativeStringRef, file_descriptor: *mut i32) -> u32;
