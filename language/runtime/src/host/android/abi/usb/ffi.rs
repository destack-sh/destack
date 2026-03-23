#![allow(unreachable_pub)]

use super::types::{
    AndroidHostUsbCallbacks, AndroidHostUsbDeviceDescriptorHeader, AndroidHostUsbHotplugEventHeader,
};
use crate::host::android::abi::bindings::invoke_android_binding_callback;
use crate::host::core::HostStatus;
use crate::runtime::{NativeSlice, NativeStringRef};

/// Resolve and invoke one Android host USB callback.
fn call_android_usb_callback<T: Copy>(
    runtime_id: u64,
    resolve: impl FnOnce(&AndroidHostUsbCallbacks) -> Option<T>,
    invoke: impl FnOnce(T) -> u32,
) -> u32 {
    invoke_android_binding_callback(runtime_id, |bindings| resolve(&bindings.usb), invoke)
}

macro_rules! usb_callback {
    ($name:ident ($($arg:ident : $ty:ty),* $(,)?) -> $resolve:ident $(; guard $guard:expr)? ) => {
        #[doc = "Route one Android host USB callback through the registered callback table."]
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn $name(runtime_id: u64, $($arg: $ty),*) -> u32 {
            $(if !($guard) { return HostStatus::InvalidArgument.code(); })?

            call_android_usb_callback(runtime_id, |callbacks| callbacks.$resolve, |callback| unsafe {
                callback(runtime_id, $($arg),*)
            })
        }
    };
}

usb_callback!(
    destack_host_android_usb_device_list(
        devices: NativeSlice<AndroidHostUsbDeviceDescriptorHeader>,
        device_count_written: *mut u32,
        string_bytes: NativeSlice<u8>,
        string_bytes_written: *mut u32,
        port_bytes: NativeSlice<u8>,
        port_bytes_written: *mut u32
    ) -> device_list;
    guard !device_count_written.is_null()
        && !string_bytes_written.is_null()
        && !port_bytes_written.is_null()
);
usb_callback!(destack_host_android_usb_watch_open(watch_id: *mut u64) -> watch_open; guard !watch_id.is_null());
usb_callback!(destack_host_android_usb_watch_close(watch_id: u64) -> watch_close);
usb_callback!(
    destack_host_android_usb_watch_read(
        watch_id: u64,
        timeout_ns: u64,
        event: *mut AndroidHostUsbHotplugEventHeader,
        string_bytes: NativeSlice<u8>,
        string_bytes_written: *mut u32,
        port_bytes: NativeSlice<u8>,
        port_bytes_written: *mut u32
    ) -> watch_read;
    guard !event.is_null()
        && !string_bytes_written.is_null()
        && !port_bytes_written.is_null()
);
usb_callback!(
    destack_host_android_usb_watch_try_read(
        watch_id: u64,
        event: *mut AndroidHostUsbHotplugEventHeader,
        string_bytes: NativeSlice<u8>,
        string_bytes_written: *mut u32,
        port_bytes: NativeSlice<u8>,
        port_bytes_written: *mut u32
    ) -> watch_try_read;
    guard !event.is_null()
        && !string_bytes_written.is_null()
        && !port_bytes_written.is_null()
);
usb_callback!(destack_host_android_usb_open(id: NativeStringRef, file_descriptor: *mut i32) -> open; guard !file_descriptor.is_null());
