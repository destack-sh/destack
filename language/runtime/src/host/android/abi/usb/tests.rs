use std::sync::{Arc, Mutex, OnceLock};

use crate::host::android::abi::usb::ffi::{
    destack_host_android_usb_device_list, destack_host_android_usb_open,
    destack_host_android_usb_watch_close, destack_host_android_usb_watch_open,
    destack_host_android_usb_watch_read, destack_host_android_usb_watch_try_read,
};
use crate::host::android::abi::usb::types::{
    AndroidHostUsbCallbacks, AndroidHostUsbDeviceDescriptorHeader, AndroidHostUsbHotplugEventHeader,
};
use crate::host::android::tests::{
    callback_test_lock, register_android_bindings_usb, register_android_runtime,
};
use crate::host::core::registry::HostSessionRegistrationGuard;
use crate::host::core::{HostQueue, HostStatus};
use crate::runtime::{NativeSlice, NativeStringRef};

/// One deterministic Android USB device id.
const TEST_USB_DEVICE_ID: &str = "android.usb.device";

/// One deterministic Android USB product string.
const TEST_USB_PRODUCT: &str = "Destack USB Device";

/// One deterministic Android USB watch id.
const TEST_USB_WATCH_ID: u64 = 303;

/// One deterministic Android USB file descriptor.
const TEST_USB_FILE_DESCRIPTOR: i32 = 41;

/// One live Android USB callback registration.
struct AndroidUsbTestCallbacks {
    /// The runtime id bound to the callback table.
    runtime_id: u64,
    /// The host queue kept alive for the registration.
    _queue: Arc<HostQueue>,
    /// The host registration guard kept alive for the callback table.
    _registration: HostSessionRegistrationGuard,
}

/// One snapshot of recorded Android USB callback traffic.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct AndroidUsbTestState {
    /// The most recent opened device id.
    opened_device_id: Option<String>,
    /// The closed watch identifiers in callback order.
    closed_watch_ids: Vec<u64>,
    /// The most recent blocking read timeout.
    last_watch_read_timeout_ns: Option<u64>,
}

/// Return the shared Android USB test state.
fn test_state() -> &'static Mutex<AndroidUsbTestState> {
    static STATE: OnceLock<Mutex<AndroidUsbTestState>> = OnceLock::new();

    STATE.get_or_init(|| Mutex::new(AndroidUsbTestState::default()))
}

/// Lock the shared Android USB callback state.
fn lock_test_callbacks() -> std::sync::MutexGuard<'static, ()> {
    callback_test_lock()
        .lock()
        .expect("android usb test lock should not be poisoned")
}

/// Reset the shared Android USB callback state.
fn reset_test_state() {
    *test_state()
        .lock()
        .expect("android usb test state should not be poisoned") = AndroidUsbTestState::default();
}

/// Append one string to the shared output buffer and return its offset pair.
fn append_string(buffer: &mut Vec<u8>, value: &str) -> (u32, u32) {
    let offset = buffer.len() as u32;
    buffer.extend_from_slice(value.as_bytes());

    (offset, value.len() as u32)
}

/// Decode one borrowed native string reference into one owned Rust string.
fn decode_native_string(value: NativeStringRef) -> String {
    let bytes = unsafe { std::slice::from_raw_parts(value.data, value.len as usize) };

    std::str::from_utf8(bytes)
        .expect("android usb test strings should be valid utf8")
        .to_string()
}

/// Build one deterministic Android USB descriptor header.
fn build_device_header(
    string_bytes: &mut Vec<u8>,
    port_bytes: &mut Vec<u8>,
) -> AndroidHostUsbDeviceDescriptorHeader {
    let (id_offset, id_len) = append_string(string_bytes, TEST_USB_DEVICE_ID);
    let (product_offset, product_len) = append_string(string_bytes, TEST_USB_PRODUCT);
    let port_path_offset = port_bytes.len() as u32;
    port_bytes.extend_from_slice(&[1, 4, 2]);

    AndroidHostUsbDeviceDescriptorHeader {
        id_offset,
        id_len,
        manufacturer_offset: 0,
        manufacturer_len: 0,
        product_offset,
        product_len,
        serial_number_offset: 0,
        serial_number_len: 0,
        port_path_offset,
        port_path_len: 3,
        usb_version_bcd: 0x0200,
        has_usb_version_bcd: 1,
        device_version_bcd: 0x0100,
        has_device_version_bcd: 1,
        vendor_id: 0x1234,
        product_id: 0xabcd,
        class_code: 0xff,
        subclass_code: 0,
        protocol_code: 0,
        speed: 4,
        has_speed: 1,
        bus_number: 2,
        has_bus_number: 1,
    }
}

/// Handle one Android USB device-list callback.
unsafe extern "C" fn test_device_list(
    _runtime_id: u64,
    devices: NativeSlice<AndroidHostUsbDeviceDescriptorHeader>,
    device_count_written: *mut u32,
    string_bytes: NativeSlice<u8>,
    string_bytes_written: *mut u32,
    port_bytes: NativeSlice<u8>,
    port_bytes_written: *mut u32,
) -> u32 {
    let mut string_buffer = Vec::new();
    let mut port_buffer = Vec::new();
    let device = build_device_header(&mut string_buffer, &mut port_buffer);

    // report required buffer sizes first
    if devices.data.is_null()
        || devices.len == 0
        || string_bytes.data.is_null()
        || string_bytes.len < string_buffer.len() as u32
        || port_bytes.data.is_null()
        || port_bytes.len < port_buffer.len() as u32
    {
        unsafe {
            *device_count_written = 1;
            *string_bytes_written = string_buffer.len() as u32;
            *port_bytes_written = port_buffer.len() as u32;
        }

        return HostStatus::BufferTooSmall.code();
    }

    // write the descriptor payload
    unsafe {
        *devices.data = device;
        *device_count_written = 1;
        *string_bytes_written = string_buffer.len() as u32;
        *port_bytes_written = port_buffer.len() as u32;
    }

    let output_strings =
        unsafe { std::slice::from_raw_parts_mut(string_bytes.data, string_bytes.len as usize) };
    output_strings[..string_buffer.len()].copy_from_slice(&string_buffer);

    let output_ports =
        unsafe { std::slice::from_raw_parts_mut(port_bytes.data, port_bytes.len as usize) };
    output_ports[..port_buffer.len()].copy_from_slice(&port_buffer);

    HostStatus::Ok.code()
}

/// Handle one Android USB watch-open callback.
unsafe extern "C" fn test_watch_open(_runtime_id: u64, watch_id: *mut u64) -> u32 {
    unsafe {
        *watch_id = TEST_USB_WATCH_ID;
    }

    HostStatus::Ok.code()
}

/// Handle one Android USB watch-close callback.
unsafe extern "C" fn test_watch_close(_runtime_id: u64, watch_id: u64) -> u32 {
    test_state()
        .lock()
        .expect("android usb test state should not be poisoned")
        .closed_watch_ids
        .push(watch_id);

    HostStatus::Ok.code()
}

/// Handle one Android USB blocking watch-read callback.
unsafe extern "C" fn test_watch_read(
    runtime_id: u64,
    watch_id: u64,
    timeout_ns: u64,
    event: *mut AndroidHostUsbHotplugEventHeader,
    string_bytes: NativeSlice<u8>,
    string_bytes_written: *mut u32,
    port_bytes: NativeSlice<u8>,
    port_bytes_written: *mut u32,
) -> u32 {
    test_state()
        .lock()
        .expect("android usb test state should not be poisoned")
        .last_watch_read_timeout_ns = Some(timeout_ns);

    unsafe {
        test_watch_try_read(
            runtime_id,
            watch_id,
            event,
            string_bytes,
            string_bytes_written,
            port_bytes,
            port_bytes_written,
        )
    }
}

/// Handle one Android USB watch-try-read callback.
unsafe extern "C" fn test_watch_try_read(
    _runtime_id: u64,
    watch_id: u64,
    event: *mut AndroidHostUsbHotplugEventHeader,
    string_bytes: NativeSlice<u8>,
    string_bytes_written: *mut u32,
    port_bytes: NativeSlice<u8>,
    port_bytes_written: *mut u32,
) -> u32 {
    assert_eq!(watch_id, TEST_USB_WATCH_ID);

    let mut string_buffer = Vec::new();
    let mut port_buffer = Vec::new();
    let descriptor = build_device_header(&mut string_buffer, &mut port_buffer);

    // report required buffer sizes first
    if string_bytes.data.is_null()
        || string_bytes.len < string_buffer.len() as u32
        || port_bytes.data.is_null()
        || port_bytes.len < port_buffer.len() as u32
    {
        unsafe {
            *string_bytes_written = string_buffer.len() as u32;
            *port_bytes_written = port_buffer.len() as u32;
        }

        return HostStatus::BufferTooSmall.code();
    }

    unsafe {
        *event = AndroidHostUsbHotplugEventHeader {
            timestamp_ns: 99,
            kind: 1,
            descriptor,
        };
        *string_bytes_written = string_buffer.len() as u32;
        *port_bytes_written = port_buffer.len() as u32;
    }

    let output_strings =
        unsafe { std::slice::from_raw_parts_mut(string_bytes.data, string_bytes.len as usize) };
    output_strings[..string_buffer.len()].copy_from_slice(&string_buffer);

    let output_ports =
        unsafe { std::slice::from_raw_parts_mut(port_bytes.data, port_bytes.len as usize) };
    output_ports[..port_buffer.len()].copy_from_slice(&port_buffer);

    HostStatus::Ok.code()
}

/// Handle one Android USB open callback.
unsafe extern "C" fn test_open(
    _runtime_id: u64,
    id: NativeStringRef,
    file_descriptor: *mut i32,
) -> u32 {
    test_state()
        .lock()
        .expect("android usb test state should not be poisoned")
        .opened_device_id = Some(decode_native_string(id));

    unsafe {
        *file_descriptor = TEST_USB_FILE_DESCRIPTOR;
    }

    HostStatus::Ok.code()
}

/// Register one Android USB callback table for the current test.
fn register_test_callbacks() -> AndroidUsbTestCallbacks {
    reset_test_state();

    let (queue, registration, runtime_id) = register_android_runtime();
    let status = register_android_bindings_usb(
        runtime_id,
        AndroidHostUsbCallbacks {
            device_list: Some(test_device_list),
            watch_open: Some(test_watch_open),
            watch_close: Some(test_watch_close),
            watch_read: Some(test_watch_read),
            watch_try_read: Some(test_watch_try_read),
            open: Some(test_open),
        },
    );
    assert_eq!(status, HostStatus::Ok.code());

    AndroidUsbTestCallbacks {
        runtime_id,
        _queue: queue,
        _registration: registration,
    }
}

/// Route one Android USB device list through the registered callback table.
#[test]
fn test_android_usb_device_list_routes_through_registered_callbacks() {
    let _lock = lock_test_callbacks();
    let callbacks = register_test_callbacks();
    let mut rows = [AndroidHostUsbDeviceDescriptorHeader::default(); 1];
    let mut string_bytes = [0u8; 128];
    let mut port_bytes = [0u8; 16];
    let mut row_count_written = 0u32;
    let mut string_bytes_written = 0u32;
    let mut port_bytes_written = 0u32;

    let status = unsafe {
        destack_host_android_usb_device_list(
            callbacks.runtime_id,
            NativeSlice {
                data: rows.as_mut_ptr(),
                len: rows.len() as u32,
            },
            &mut row_count_written,
            NativeSlice {
                data: string_bytes.as_mut_ptr(),
                len: string_bytes.len() as u32,
            },
            &mut string_bytes_written,
            NativeSlice {
                data: port_bytes.as_mut_ptr(),
                len: port_bytes.len() as u32,
            },
            &mut port_bytes_written,
        )
    };

    assert_eq!(status, HostStatus::Ok.code());
    assert_eq!(row_count_written, 1);
    assert!(string_bytes_written != 0);
    assert!(port_bytes_written != 0);
}

/// Route one Android USB watch event through the registered callback table.
#[test]
fn test_android_usb_watch_routes_through_registered_callbacks() {
    let _lock = lock_test_callbacks();
    let callbacks = register_test_callbacks();
    let mut watch_id = 0u64;
    let status =
        unsafe { destack_host_android_usb_watch_open(callbacks.runtime_id, &mut watch_id) };
    assert_eq!(status, HostStatus::Ok.code());
    assert_eq!(watch_id, TEST_USB_WATCH_ID);

    let mut event = AndroidHostUsbHotplugEventHeader::default();
    let mut string_bytes = [0u8; 128];
    let mut port_bytes = [0u8; 16];
    let mut string_bytes_written = 0u32;
    let mut port_bytes_written = 0u32;
    let status = unsafe {
        destack_host_android_usb_watch_try_read(
            callbacks.runtime_id,
            watch_id,
            &mut event,
            NativeSlice {
                data: string_bytes.as_mut_ptr(),
                len: string_bytes.len() as u32,
            },
            &mut string_bytes_written,
            NativeSlice {
                data: port_bytes.as_mut_ptr(),
                len: port_bytes.len() as u32,
            },
            &mut port_bytes_written,
        )
    };

    assert_eq!(status, HostStatus::Ok.code());
    assert_eq!(event.kind, 1);
    assert!(string_bytes_written != 0);
    assert!(port_bytes_written != 0);
}

/// Route one Android USB blocking watch read and close through the registered callback table.
#[test]
fn test_android_usb_watch_read_and_close_route_through_registered_callbacks() {
    let _lock = lock_test_callbacks();
    let callbacks = register_test_callbacks();
    let mut watch_id = 0u64;
    let status =
        unsafe { destack_host_android_usb_watch_open(callbacks.runtime_id, &mut watch_id) };
    assert_eq!(status, HostStatus::Ok.code());

    let mut event = AndroidHostUsbHotplugEventHeader::default();
    let mut string_bytes = [0u8; 128];
    let mut port_bytes = [0u8; 16];
    let mut string_bytes_written = 0u32;
    let mut port_bytes_written = 0u32;

    let status = unsafe {
        destack_host_android_usb_watch_read(
            callbacks.runtime_id,
            watch_id,
            12_345,
            &mut event,
            NativeSlice {
                data: string_bytes.as_mut_ptr(),
                len: string_bytes.len() as u32,
            },
            &mut string_bytes_written,
            NativeSlice {
                data: port_bytes.as_mut_ptr(),
                len: port_bytes.len() as u32,
            },
            &mut port_bytes_written,
        )
    };

    assert_eq!(status, HostStatus::Ok.code());
    assert_eq!(event.kind, 1);
    assert_eq!(
        test_state()
            .lock()
            .expect("android usb test state should not be poisoned")
            .last_watch_read_timeout_ns,
        Some(12_345)
    );

    let status = unsafe { destack_host_android_usb_watch_close(callbacks.runtime_id, watch_id) };

    assert_eq!(status, HostStatus::Ok.code());
    assert_eq!(
        test_state()
            .lock()
            .expect("android usb test state should not be poisoned")
            .closed_watch_ids
            .as_slice(),
        &[TEST_USB_WATCH_ID]
    );
}

/// Route one Android USB open request through the registered callback table.
#[test]
fn test_android_usb_open_routes_through_registered_callbacks() {
    let _lock = lock_test_callbacks();
    let callbacks = register_test_callbacks();
    let mut file_descriptor = -1i32;

    let status = unsafe {
        destack_host_android_usb_open(
            callbacks.runtime_id,
            NativeStringRef {
                data: TEST_USB_DEVICE_ID.as_ptr().cast_mut(),
                len: TEST_USB_DEVICE_ID.len() as u32,
            },
            &mut file_descriptor,
        )
    };

    assert_eq!(status, HostStatus::Ok.code());
    assert_eq!(file_descriptor, TEST_USB_FILE_DESCRIPTOR);
    assert_eq!(
        test_state()
            .lock()
            .expect("android usb test state should not be poisoned")
            .opened_device_id
            .as_deref(),
        Some(TEST_USB_DEVICE_ID)
    );
}
