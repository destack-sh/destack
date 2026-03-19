use super::tests::{
    TEST_ADAPTER_ID, TEST_CHARACTERISTIC_ID, TEST_DESCRIPTOR_ID, TEST_DEVICE_ID, TEST_DEVICE_NAME,
    TEST_GATT_DESCRIPTOR_VALUE, TEST_GATT_EVENT_VALUE, TEST_GATT_MTU, TEST_GATT_READ_VALUE,
    TEST_SCAN_SESSION_ID, TEST_SERVICE_ID, TEST_STATUS_NOT_FOUND, TEST_STATUS_NOT_SUPPORTED,
    TEST_SUBSCRIPTION_ID, lock_test_callbacks, native_string_ref, recorded_test_state,
    register_test_callbacks,
};
use crate::host::abi::HostStatus;
use crate::host::android::bluetooth::ffi::{
    destack_host_android_bluetooth_adapter_list, destack_host_android_bluetooth_close,
    destack_host_android_bluetooth_gatt_characteristic_list,
    destack_host_android_bluetooth_gatt_descriptor_list, destack_host_android_bluetooth_gatt_mtu,
    destack_host_android_bluetooth_gatt_read, destack_host_android_bluetooth_gatt_read_descriptor,
    destack_host_android_bluetooth_gatt_read_event,
    destack_host_android_bluetooth_gatt_service_list,
    destack_host_android_bluetooth_gatt_subscribe,
    destack_host_android_bluetooth_gatt_try_read_event,
    destack_host_android_bluetooth_gatt_unsubscribe, destack_host_android_bluetooth_gatt_write,
    destack_host_android_bluetooth_gatt_write_descriptor, destack_host_android_bluetooth_open,
    destack_host_android_bluetooth_pair, destack_host_android_bluetooth_read_rssi,
    destack_host_android_bluetooth_scan_close, destack_host_android_bluetooth_scan_open,
    destack_host_android_bluetooth_scan_read, destack_host_android_bluetooth_scan_read_event,
    destack_host_android_bluetooth_scan_try_read,
    destack_host_android_bluetooth_scan_try_read_event,
    destack_host_android_bluetooth_session_read_event,
    destack_host_android_bluetooth_session_try_read_event, destack_host_android_bluetooth_unpair,
};
use crate::host::android::bluetooth::types::{
    AndroidHostBluetoothAdapterDescriptorHeader, AndroidHostBluetoothCallbacks,
    AndroidHostBluetoothDeviceDescriptorHeader, AndroidHostBluetoothGattCharacteristicHeader,
    AndroidHostBluetoothGattDescriptorHeader, AndroidHostBluetoothGattServiceHeader,
    AndroidHostBluetoothScanEventHeader, AndroidHostBluetoothSessionEventHeader,
};
use crate::host::android::tests::{register_android_bindings_bluetooth, register_android_runtime};
use crate::runtime::NativeSlice;

/// Decode one utf8 string from one caller-owned byte buffer.
fn decode_string(bytes: &[u8], offset: u32, len: u32) -> &str {
    let start = offset as usize;
    let end = start + len as usize;

    std::str::from_utf8(&bytes[start..end])
        .expect("android bluetooth fixture strings should decode")
}

/// Return not-supported when Android bluetooth callbacks are not registered.
#[test]
fn test_default_callbacks_return_not_supported() {
    let _lock = lock_test_callbacks();
    let (_queue, _registration, runtime_id) = register_android_runtime();
    let mut header_count_written = 0u32;
    let mut string_bytes_written = 0u32;

    let status = unsafe {
        destack_host_android_bluetooth_adapter_list(
            runtime_id,
            NativeSlice {
                data: std::ptr::null_mut(),
                len: 0,
            },
            &mut header_count_written,
            NativeSlice {
                data: std::ptr::null_mut(),
                len: 0,
            },
            &mut string_bytes_written,
        )
    };

    assert_eq!(status, TEST_STATUS_NOT_SUPPORTED);
}

/// Reject Android bluetooth callback registration for one unknown runtime.
#[test]
fn test_register_bindings_rejects_unknown_runtime() {
    let _lock = lock_test_callbacks();

    let status = register_android_bindings_bluetooth(0, AndroidHostBluetoothCallbacks::default());

    assert_eq!(status, TEST_STATUS_NOT_FOUND);
}

/// Route adapter listing through the registered Android bluetooth callback table.
#[test]
fn test_register_bindings_routes_adapter_list() {
    let _lock = lock_test_callbacks();
    let callbacks = register_test_callbacks();
    let mut header = AndroidHostBluetoothAdapterDescriptorHeader::default();
    let mut header_count_written = 0u32;
    let mut string_bytes_written = 0u32;

    // verify adapter enumeration honors the two-pass buffer contract
    let status = unsafe {
        destack_host_android_bluetooth_adapter_list(
            callbacks.runtime_id,
            NativeSlice {
                data: std::ptr::null_mut(),
                len: 0,
            },
            &mut header_count_written,
            NativeSlice {
                data: std::ptr::null_mut(),
                len: 0,
            },
            &mut string_bytes_written,
        )
    };
    assert_eq!(status, HostStatus::BufferTooSmall.code());
    assert_eq!(header_count_written, 1);
    assert!(string_bytes_written > 0);

    let mut string_bytes = vec![0u8; string_bytes_written as usize];
    let status = unsafe {
        destack_host_android_bluetooth_adapter_list(
            callbacks.runtime_id,
            NativeSlice {
                data: &mut header,
                len: 1,
            },
            &mut header_count_written,
            NativeSlice {
                data: string_bytes.as_mut_ptr(),
                len: string_bytes.len() as u32,
            },
            &mut string_bytes_written,
        )
    };
    assert_eq!(status, HostStatus::Ok.code());
    assert_eq!(header_count_written, 1);

    // verify the returned row decodes to the deterministic adapter fixture
    assert_eq!(
        decode_string(&string_bytes, header.name_offset, header.name_len),
        "Android Adapter"
    );
}

/// Route scan open and device open through the registered callback table.
#[test]
fn test_register_bindings_routes_scan_and_device_lifecycle() {
    let _lock = lock_test_callbacks();
    let callbacks = register_test_callbacks();
    let mut scan_session_id = 0u64;
    let mut device_session_id = 0u64;
    let mut descriptor = AndroidHostBluetoothDeviceDescriptorHeader::default();
    let mut string_bytes_written = 0u32;
    let mut string_bytes = vec![0u8; 128];

    // verify scan open forwards the adapter id and filter flags
    let status = unsafe {
        destack_host_android_bluetooth_scan_open(
            callbacks.runtime_id,
            native_string_ref(TEST_ADAPTER_ID),
            0x33,
            &mut scan_session_id,
        )
    };
    assert_eq!(status, HostStatus::Ok.code());
    assert_eq!(scan_session_id, TEST_SCAN_SESSION_ID);

    let state = recorded_test_state();
    let scan_open = state.scan_open.expect("scan open should be recorded");
    assert_eq!(scan_open.adapter_id, TEST_ADAPTER_ID);
    assert_eq!(scan_open.filter_flags, 0x33);

    // verify device open returns the deterministic descriptor payload
    let status = unsafe {
        destack_host_android_bluetooth_open(
            callbacks.runtime_id,
            native_string_ref(TEST_ADAPTER_ID),
            native_string_ref(TEST_DEVICE_ID),
            &mut device_session_id,
            &mut descriptor,
            NativeSlice {
                data: string_bytes.as_mut_ptr(),
                len: string_bytes.len() as u32,
            },
            &mut string_bytes_written,
        )
    };
    assert_eq!(status, HostStatus::Ok.code());
    assert_eq!(device_session_id, 202);
    assert_eq!(
        decode_string(&string_bytes, descriptor.name_offset, descriptor.name_len),
        TEST_DEVICE_NAME
    );

    let state = recorded_test_state();
    let device_open = state.device_open.expect("device open should be recorded");
    assert_eq!(device_open.adapter_id, TEST_ADAPTER_ID);
    assert_eq!(device_open.device_id, TEST_DEVICE_ID);

    // verify close calls route through the registered callbacks
    let status =
        unsafe { destack_host_android_bluetooth_scan_close(callbacks.runtime_id, scan_session_id) };
    assert_eq!(status, HostStatus::Ok.code());
    let status =
        unsafe { destack_host_android_bluetooth_close(callbacks.runtime_id, device_session_id) };
    assert_eq!(status, HostStatus::Ok.code());

    let state = recorded_test_state();
    assert_eq!(state.closed_scan_sessions, vec![TEST_SCAN_SESSION_ID]);
    assert_eq!(state.closed_device_sessions, vec![202]);
}

/// Route pair, unpair, and RSSI calls through the registered callback table.
#[test]
fn test_register_bindings_routes_pair_unpair_and_rssi() {
    let _lock = lock_test_callbacks();
    let callbacks = register_test_callbacks();
    let mut rssi_dbm = 0i16;

    // verify pairing forwards the session and timeout
    let status = unsafe { destack_host_android_bluetooth_pair(callbacks.runtime_id, 202, 55_000) };
    assert_eq!(status, HostStatus::Ok.code());

    // verify unpair forwards the adapter and device ids
    let status = unsafe {
        destack_host_android_bluetooth_unpair(
            callbacks.runtime_id,
            native_string_ref(TEST_ADAPTER_ID),
            native_string_ref(TEST_DEVICE_ID),
        )
    };
    assert_eq!(status, HostStatus::Ok.code());

    // verify RSSI reads forward the session and timeout and return the fixture value
    let status = unsafe {
        destack_host_android_bluetooth_read_rssi(callbacks.runtime_id, 202, 77_000, &mut rssi_dbm)
    };
    assert_eq!(status, HostStatus::Ok.code());
    assert_eq!(rssi_dbm, -48);

    let state = recorded_test_state();
    let pair = state.pair.expect("pair should be recorded");
    let unpair = state.unpair.expect("unpair should be recorded");
    let read_rssi = state.read_rssi.expect("rssi should be recorded");

    assert_eq!(pair.session_id, 202);
    assert_eq!(pair.timeout_ns, 55_000);
    assert_eq!(unpair.adapter_id, TEST_ADAPTER_ID);
    assert_eq!(unpair.device_id, TEST_DEVICE_ID);
    assert_eq!(read_rssi.session_id, 202);
    assert_eq!(read_rssi.timeout_ns, 77_000);
}

/// Route scan descriptor and event reads through the registered callback table.
#[test]
fn test_register_bindings_routes_scan_reads_and_events() {
    let _lock = lock_test_callbacks();
    let callbacks = register_test_callbacks();
    let mut device = AndroidHostBluetoothDeviceDescriptorHeader::default();
    let mut device_count_written = 0u32;
    let mut string_bytes_written = 0u32;
    let mut string_bytes = vec![0u8; 128];
    let mut event = AndroidHostBluetoothScanEventHeader::default();

    // verify scan descriptor reads honor the two-pass buffer contract
    let status = unsafe {
        destack_host_android_bluetooth_scan_read(
            callbacks.runtime_id,
            TEST_SCAN_SESSION_ID,
            12_345,
            NativeSlice {
                data: std::ptr::null_mut(),
                len: 0,
            },
            &mut device_count_written,
            NativeSlice {
                data: std::ptr::null_mut(),
                len: 0,
            },
            &mut string_bytes_written,
        )
    };
    assert_eq!(status, HostStatus::BufferTooSmall.code());
    assert_eq!(device_count_written, 1);
    assert!(string_bytes_written > 0);

    let status = unsafe {
        destack_host_android_bluetooth_scan_read(
            callbacks.runtime_id,
            TEST_SCAN_SESSION_ID,
            12_345,
            NativeSlice {
                data: &mut device,
                len: 1,
            },
            &mut device_count_written,
            NativeSlice {
                data: string_bytes.as_mut_ptr(),
                len: string_bytes.len() as u32,
            },
            &mut string_bytes_written,
        )
    };
    assert_eq!(status, HostStatus::Ok.code());
    assert_eq!(device_count_written, 1);
    assert_eq!(
        decode_string(&string_bytes, device.name_offset, device.name_len),
        TEST_DEVICE_NAME
    );

    let status = unsafe {
        destack_host_android_bluetooth_scan_try_read(
            callbacks.runtime_id,
            TEST_SCAN_SESSION_ID,
            NativeSlice {
                data: &mut device,
                len: 1,
            },
            &mut device_count_written,
            NativeSlice {
                data: string_bytes.as_mut_ptr(),
                len: string_bytes.len() as u32,
            },
            &mut string_bytes_written,
        )
    };
    assert_eq!(status, HostStatus::Ok.code());

    let state = recorded_test_state();
    let scan_read = state.scan_read.expect("scan read should be recorded");
    assert_eq!(scan_read.session_id, TEST_SCAN_SESSION_ID);
    assert_eq!(scan_read.timeout_ns, None);

    // verify scan event reads return the deterministic event payload
    let status = unsafe {
        destack_host_android_bluetooth_scan_read_event(
            callbacks.runtime_id,
            TEST_SCAN_SESSION_ID,
            99_000,
            &mut event,
            NativeSlice {
                data: string_bytes.as_mut_ptr(),
                len: string_bytes.len() as u32,
            },
            &mut string_bytes_written,
        )
    };
    assert_eq!(status, HostStatus::Ok.code());
    assert_eq!(event.kind, 1);
    assert_eq!(
        decode_string(
            &string_bytes,
            event.descriptor.name_offset,
            event.descriptor.name_len
        ),
        TEST_DEVICE_NAME
    );

    let status = unsafe {
        destack_host_android_bluetooth_scan_try_read_event(
            callbacks.runtime_id,
            TEST_SCAN_SESSION_ID,
            &mut event,
            NativeSlice {
                data: string_bytes.as_mut_ptr(),
                len: string_bytes.len() as u32,
            },
            &mut string_bytes_written,
        )
    };
    assert_eq!(status, HostStatus::Ok.code());

    let state = recorded_test_state();
    let scan_event_read = state
        .scan_event_read
        .expect("scan event read should be recorded");
    assert_eq!(scan_event_read.session_id, TEST_SCAN_SESSION_ID);
    assert_eq!(scan_event_read.timeout_ns, None);
}

/// Route session events and GATT metadata reads through the registered callback table.
#[test]
fn test_register_bindings_routes_session_events_and_gatt_metadata() {
    let _lock = lock_test_callbacks();
    let callbacks = register_test_callbacks();
    let mut event = AndroidHostBluetoothSessionEventHeader::default();
    let mut service = AndroidHostBluetoothGattServiceHeader::default();
    let mut service_count_written = 0u32;
    let mut characteristic = AndroidHostBluetoothGattCharacteristicHeader::default();
    let mut characteristic_count_written = 0u32;
    let mut descriptor = AndroidHostBluetoothGattDescriptorHeader::default();
    let mut descriptor_count_written = 0u32;
    let mut string_bytes_written = 0u32;
    let mut mtu = 0u16;
    let mut string_bytes = vec![0u8; 256];

    // verify session event reads route to the correct callback slot
    let status = unsafe {
        destack_host_android_bluetooth_session_read_event(
            callbacks.runtime_id,
            202,
            66_000,
            &mut event,
        )
    };
    assert_eq!(status, HostStatus::Ok.code());
    assert_eq!(event.kind, 2);
    assert_eq!(event.pair_state, 4);

    let status = unsafe {
        destack_host_android_bluetooth_session_try_read_event(callbacks.runtime_id, 202, &mut event)
    };
    assert_eq!(status, HostStatus::Ok.code());

    let state = recorded_test_state();
    let session_event_read = state
        .session_event_read
        .expect("session event read should be recorded");
    assert_eq!(session_event_read.session_id, 202);
    assert_eq!(session_event_read.timeout_ns, None);

    // verify service enumeration honors the two-pass string buffer contract
    let status = unsafe {
        destack_host_android_bluetooth_gatt_service_list(
            callbacks.runtime_id,
            202,
            NativeSlice {
                data: std::ptr::null_mut(),
                len: 0,
            },
            &mut service_count_written,
            NativeSlice {
                data: std::ptr::null_mut(),
                len: 0,
            },
            &mut string_bytes_written,
        )
    };
    assert_eq!(status, HostStatus::BufferTooSmall.code());
    assert_eq!(service_count_written, 1);

    let status = unsafe {
        destack_host_android_bluetooth_gatt_service_list(
            callbacks.runtime_id,
            202,
            NativeSlice {
                data: &mut service,
                len: 1,
            },
            &mut service_count_written,
            NativeSlice {
                data: string_bytes.as_mut_ptr(),
                len: string_bytes.len() as u32,
            },
            &mut string_bytes_written,
        )
    };
    assert_eq!(status, HostStatus::Ok.code());
    assert_eq!(
        decode_string(&string_bytes, service.id_offset, service.id_len),
        TEST_SERVICE_ID
    );

    // verify characteristic and descriptor enumeration forward the expected ids
    let status = unsafe {
        destack_host_android_bluetooth_gatt_characteristic_list(
            callbacks.runtime_id,
            202,
            native_string_ref(TEST_SERVICE_ID),
            NativeSlice {
                data: &mut characteristic,
                len: 1,
            },
            &mut characteristic_count_written,
            NativeSlice {
                data: string_bytes.as_mut_ptr(),
                len: string_bytes.len() as u32,
            },
            &mut string_bytes_written,
        )
    };
    assert_eq!(status, HostStatus::Ok.code());
    assert_eq!(
        decode_string(
            &string_bytes,
            characteristic.id_offset,
            characteristic.id_len
        ),
        TEST_CHARACTERISTIC_ID
    );

    let status = unsafe {
        destack_host_android_bluetooth_gatt_descriptor_list(
            callbacks.runtime_id,
            202,
            native_string_ref(TEST_CHARACTERISTIC_ID),
            NativeSlice {
                data: &mut descriptor,
                len: 1,
            },
            &mut descriptor_count_written,
            NativeSlice {
                data: string_bytes.as_mut_ptr(),
                len: string_bytes.len() as u32,
            },
            &mut string_bytes_written,
        )
    };
    assert_eq!(status, HostStatus::Ok.code());
    assert_eq!(
        decode_string(&string_bytes, descriptor.id_offset, descriptor.id_len),
        TEST_DESCRIPTOR_ID
    );

    let status =
        unsafe { destack_host_android_bluetooth_gatt_mtu(callbacks.runtime_id, 202, &mut mtu) };
    assert_eq!(status, HostStatus::Ok.code());
    assert_eq!(mtu, TEST_GATT_MTU);

    let state = recorded_test_state();
    assert_eq!(state.gatt_service_list_session_id, Some(202));

    let gatt_characteristic_list = state
        .gatt_characteristic_list
        .expect("gatt characteristic list should be recorded");
    assert_eq!(gatt_characteristic_list.session_id, 202);
    assert_eq!(gatt_characteristic_list.service_id, TEST_SERVICE_ID);

    let gatt_descriptor_list = state
        .gatt_descriptor_list
        .expect("gatt descriptor list should be recorded");
    assert_eq!(gatt_descriptor_list.session_id, 202);
    assert_eq!(
        gatt_descriptor_list.characteristic_id,
        TEST_CHARACTERISTIC_ID
    );
}

/// Route GATT IO and subscription calls through the registered callback table.
#[test]
fn test_register_bindings_routes_gatt_io_and_subscription() {
    let _lock = lock_test_callbacks();
    let callbacks = register_test_callbacks();
    let mut bytes_written = 0u32;
    let mut read_buffer = vec![0u8; 32];
    let mut subscription_id = 0u64;
    let mut event = NativeSlice {
        data: std::ptr::null_mut(),
        len: 0,
    };
    let mut timestamp_ns = 0u64;
    let write_value = [0x10, 0x20, 0x30];

    // verify characteristic and descriptor reads return the deterministic payloads
    let status = unsafe {
        destack_host_android_bluetooth_gatt_read(
            callbacks.runtime_id,
            202,
            native_string_ref(TEST_CHARACTERISTIC_ID),
            44_000,
            NativeSlice {
                data: read_buffer.as_mut_ptr(),
                len: read_buffer.len() as u32,
            },
            &mut bytes_written,
        )
    };
    assert_eq!(status, HostStatus::Ok.code());
    assert_eq!(&read_buffer[..bytes_written as usize], TEST_GATT_READ_VALUE);

    let status = unsafe {
        destack_host_android_bluetooth_gatt_read_descriptor(
            callbacks.runtime_id,
            202,
            native_string_ref(TEST_DESCRIPTOR_ID),
            55_000,
            NativeSlice {
                data: read_buffer.as_mut_ptr(),
                len: read_buffer.len() as u32,
            },
            &mut bytes_written,
        )
    };
    assert_eq!(status, HostStatus::Ok.code());
    assert_eq!(
        &read_buffer[..bytes_written as usize],
        TEST_GATT_DESCRIPTOR_VALUE
    );

    // verify writes forward the ids, payloads, and timeout values
    let status = unsafe {
        destack_host_android_bluetooth_gatt_write(
            callbacks.runtime_id,
            202,
            native_string_ref(TEST_CHARACTERISTIC_ID),
            1,
            NativeSlice {
                data: write_value.as_ptr().cast_mut(),
                len: write_value.len() as u32,
            },
            66_000,
        )
    };
    assert_eq!(status, HostStatus::Ok.code());

    let status = unsafe {
        destack_host_android_bluetooth_gatt_write_descriptor(
            callbacks.runtime_id,
            202,
            native_string_ref(TEST_DESCRIPTOR_ID),
            NativeSlice {
                data: write_value.as_ptr().cast_mut(),
                len: write_value.len() as u32,
            },
            77_000,
        )
    };
    assert_eq!(status, HostStatus::Ok.code());

    // verify subscription lifecycle and event delivery
    let status = unsafe {
        destack_host_android_bluetooth_gatt_subscribe(
            callbacks.runtime_id,
            202,
            native_string_ref(TEST_CHARACTERISTIC_ID),
            &mut subscription_id,
        )
    };
    assert_eq!(status, HostStatus::Ok.code());
    assert_eq!(subscription_id, TEST_SUBSCRIPTION_ID);

    let status = unsafe {
        destack_host_android_bluetooth_gatt_read_event(
            callbacks.runtime_id,
            TEST_SUBSCRIPTION_ID,
            88_000,
            &mut event,
            &mut timestamp_ns,
        )
    };
    assert_eq!(status, HostStatus::Ok.code());
    assert_eq!(timestamp_ns, 99);

    let event_bytes = unsafe { std::slice::from_raw_parts(event.data, event.len as usize) };
    assert_eq!(event_bytes, TEST_GATT_EVENT_VALUE);

    let status = unsafe {
        destack_host_android_bluetooth_gatt_try_read_event(
            callbacks.runtime_id,
            TEST_SUBSCRIPTION_ID,
            &mut event,
            &mut timestamp_ns,
        )
    };
    assert_eq!(status, HostStatus::Ok.code());

    let status = unsafe {
        destack_host_android_bluetooth_gatt_unsubscribe(callbacks.runtime_id, TEST_SUBSCRIPTION_ID)
    };
    assert_eq!(status, HostStatus::Ok.code());

    let state = recorded_test_state();

    let gatt_read = state.gatt_read.expect("gatt read should be recorded");
    assert_eq!(gatt_read.session_id, 202);
    assert_eq!(gatt_read.characteristic_id, TEST_CHARACTERISTIC_ID);
    assert_eq!(gatt_read.timeout_ns, 44_000);

    let gatt_read_descriptor = state
        .gatt_read_descriptor
        .expect("gatt descriptor read should be recorded");
    assert_eq!(gatt_read_descriptor.session_id, 202);
    assert_eq!(gatt_read_descriptor.descriptor_id, TEST_DESCRIPTOR_ID);
    assert_eq!(gatt_read_descriptor.timeout_ns, 55_000);

    let gatt_write = state.gatt_write.expect("gatt write should be recorded");
    assert_eq!(gatt_write.session_id, 202);
    assert_eq!(gatt_write.characteristic_id, TEST_CHARACTERISTIC_ID);
    assert_eq!(gatt_write.mode, 1);
    assert_eq!(gatt_write.value, write_value);
    assert_eq!(gatt_write.timeout_ns, 66_000);

    let gatt_write_descriptor = state
        .gatt_write_descriptor
        .expect("gatt descriptor write should be recorded");
    assert_eq!(gatt_write_descriptor.session_id, 202);
    assert_eq!(gatt_write_descriptor.descriptor_id, TEST_DESCRIPTOR_ID);
    assert_eq!(gatt_write_descriptor.value, write_value);
    assert_eq!(gatt_write_descriptor.timeout_ns, 77_000);

    let gatt_subscribe = state
        .gatt_subscribe
        .expect("gatt subscribe should be recorded");
    assert_eq!(gatt_subscribe.session_id, 202);
    assert_eq!(gatt_subscribe.characteristic_id, TEST_CHARACTERISTIC_ID);

    let gatt_event_read = state
        .gatt_event_read
        .expect("gatt event read should be recorded");
    assert_eq!(gatt_event_read.subscription_id, TEST_SUBSCRIPTION_ID);
    assert_eq!(gatt_event_read.timeout_ns, None);

    assert_eq!(state.closed_subscription_ids, vec![TEST_SUBSCRIPTION_ID]);
}
