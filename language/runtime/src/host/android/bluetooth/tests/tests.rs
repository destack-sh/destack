use std::sync::{Arc, Mutex, OnceLock};

use crate::host::abi::HostStatus;
use crate::host::android::bluetooth::types::{
    AndroidHostBluetoothAdapterDescriptorHeader, AndroidHostBluetoothCallbacks,
    AndroidHostBluetoothDeviceDescriptorHeader,
};
use crate::host::android::tests::{
    callback_test_lock, register_android_bindings_bluetooth, register_android_runtime,
};
use crate::host::core::HostQueue;
use crate::host::core::registry::HostRegistrationGuard;
use crate::runtime::{NativeSlice, NativeStringRef};

/// One deterministic Android bluetooth adapter id.
pub(super) const TEST_ADAPTER_ID: &str = "android.bluetooth.adapter";
/// One deterministic Android bluetooth adapter name.
pub(super) const TEST_ADAPTER_NAME: &str = "Android Adapter";
/// One deterministic Android bluetooth device id.
pub(super) const TEST_DEVICE_ID: &str = "android.bluetooth.device";
/// One deterministic Android bluetooth device name.
pub(super) const TEST_DEVICE_NAME: &str = "Destack Peripheral";
/// One deterministic Android bluetooth manufacturer name.
pub(super) const TEST_MANUFACTURER: &str = "Destack";
/// One deterministic Android bluetooth model name.
pub(super) const TEST_MODEL: &str = "Beacon";
/// One deterministic Android bluetooth address.
pub(super) const TEST_ADDRESS: &str = "01:23:45:67:89:AB";
/// One deterministic Android bluetooth scan session id.
pub(super) const TEST_SCAN_SESSION_ID: u64 = 101;
/// One deterministic Android bluetooth device session id.
pub(super) const TEST_DEVICE_SESSION_ID: u64 = 202;
/// One deterministic Android bluetooth not-supported status.
pub(super) const TEST_STATUS_NOT_SUPPORTED: u32 = HostStatus::NotSupported.code();
/// One deterministic Android bluetooth not-found status.
pub(super) const TEST_STATUS_NOT_FOUND: u32 = HostStatus::NotFound.code();

/// One live Android bluetooth callback registration.
pub(super) struct AndroidBluetoothTestCallbacks {
    /// The runtime id bound to the callback table.
    pub runtime_id: u64,
    /// The host queue kept alive for the registration.
    _queue: Arc<HostQueue>,
    /// The host registration guard kept alive for the callback table.
    _registration: HostRegistrationGuard,
}

/// One recorded Android bluetooth scan-open call.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct RecordedScanOpenCall {
    /// The forwarded adapter id.
    pub adapter_id: String,
    /// The forwarded filter flags.
    pub filter_flags: u32,
}

/// One recorded Android bluetooth open call.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct RecordedDeviceOpenCall {
    /// The forwarded adapter id.
    pub adapter_id: String,
    /// The forwarded device id.
    pub device_id: String,
}

/// One snapshot of recorded Android bluetooth callback traffic.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct AndroidBluetoothTestState {
    /// The most recent scan-open call.
    pub scan_open: Option<RecordedScanOpenCall>,
    /// The most recent device-open call.
    pub device_open: Option<RecordedDeviceOpenCall>,
    /// Closed scan session ids.
    pub closed_scan_sessions: Vec<u64>,
    /// Closed device session ids.
    pub closed_device_sessions: Vec<u64>,
}

/// Return the shared Android bluetooth test state.
fn test_state() -> &'static Mutex<AndroidBluetoothTestState> {
    static STATE: OnceLock<Mutex<AndroidBluetoothTestState>> = OnceLock::new();

    STATE.get_or_init(|| Mutex::new(AndroidBluetoothTestState::default()))
}

/// Lock the shared Android bluetooth test callback state.
pub(super) fn lock_test_callbacks() -> std::sync::MutexGuard<'static, ()> {
    callback_test_lock()
        .lock()
        .expect("android bluetooth test lock should not be poisoned")
}

/// Return the current Android bluetooth callback snapshot.
pub(super) fn recorded_test_state() -> AndroidBluetoothTestState {
    test_state()
        .lock()
        .expect("android bluetooth test state should not be poisoned")
        .clone()
}

/// Reset the shared Android bluetooth callback state.
fn reset_test_state() {
    *test_state()
        .lock()
        .expect("android bluetooth test state should not be poisoned") =
        AndroidBluetoothTestState::default();
}

/// Return one native string reference for the given string.
pub(super) fn native_string_ref(value: &str) -> NativeStringRef {
    NativeStringRef {
        data: value.as_ptr().cast_mut(),
        len: value.len() as u32,
    }
}

/// Decode one native string reference into one owned Rust string.
fn decode_native_string(value: NativeStringRef) -> String {
    // treat empty references as empty strings in the fixture layer
    if value.data.is_null() || value.len == 0 {
        return String::new();
    }

    // decode the borrowed bytes as one owned utf8 string
    let bytes = unsafe { std::slice::from_raw_parts(value.data, value.len as usize) };

    std::str::from_utf8(bytes)
        .expect("android bluetooth test strings should be utf8")
        .to_string()
}

/// Append one string to the shared output buffer and return its offset and length.
fn append_string(buffer: &mut Vec<u8>, value: &str) -> (u32, u32) {
    let offset = buffer.len() as u32;
    buffer.extend_from_slice(value.as_bytes());

    (offset, value.len() as u32)
}

/// Build one deterministic adapter descriptor and append its strings.
fn build_adapter_header(buffer: &mut Vec<u8>) -> AndroidHostBluetoothAdapterDescriptorHeader {
    let (id_offset, id_len) = append_string(buffer, TEST_ADAPTER_ID);
    let (name_offset, name_len) = append_string(buffer, TEST_ADAPTER_NAME);

    AndroidHostBluetoothAdapterDescriptorHeader {
        id_offset,
        id_len,
        name_offset,
        name_len,
        capability_flags: (1 << 0) | (1 << 1),
        transport_flags: (1 << 0) | (1 << 1),
        is_powered: 1,
        is_discoverable: 1,
        is_discovering: 0,
    }
}

/// Build one deterministic device descriptor and append its strings.
fn build_device_header(buffer: &mut Vec<u8>) -> AndroidHostBluetoothDeviceDescriptorHeader {
    let (id_offset, id_len) = append_string(buffer, TEST_DEVICE_ID);
    let (adapter_id_offset, adapter_id_len) = append_string(buffer, TEST_ADAPTER_ID);
    let (name_offset, name_len) = append_string(buffer, TEST_DEVICE_NAME);
    let (manufacturer_offset, manufacturer_len) = append_string(buffer, TEST_MANUFACTURER);
    let (model_offset, model_len) = append_string(buffer, TEST_MODEL);
    let (address_offset, address_len) = append_string(buffer, TEST_ADDRESS);

    AndroidHostBluetoothDeviceDescriptorHeader {
        id_offset,
        id_len,
        adapter_id_offset,
        adapter_id_len,
        name_offset,
        name_len,
        local_name_offset: 0,
        local_name_len: 0,
        manufacturer_offset,
        manufacturer_len,
        model_offset,
        model_len,
        address_offset,
        address_len,
        transport: 1,
        rssi_dbm: -48,
        pair_state: 4,
        phy_flags: 1,
        is_connected: 1,
    }
}

/// Copy one adapter payload into caller buffers.
unsafe fn write_adapter_payload(
    adapters: NativeSlice<AndroidHostBluetoothAdapterDescriptorHeader>,
    adapter_count_written: *mut u32,
    string_bytes: NativeSlice<u8>,
    string_bytes_written: *mut u32,
) -> u32 {
    let mut buffer = Vec::new();
    let adapter = build_adapter_header(&mut buffer);

    // report required buffer sizes first
    if adapters.data.is_null()
        || adapters.len == 0
        || string_bytes.data.is_null()
        || string_bytes.len < buffer.len() as u32
    {
        unsafe {
            *adapter_count_written = 1;
            *string_bytes_written = buffer.len() as u32;
        }
        return HostStatus::BufferTooSmall.code();
    }

    // write the adapter row and string bytes
    unsafe {
        *adapters.data = adapter;
        *adapter_count_written = 1;
        *string_bytes_written = buffer.len() as u32;
    }

    let output =
        unsafe { std::slice::from_raw_parts_mut(string_bytes.data, string_bytes.len as usize) };
    output[..buffer.len()].copy_from_slice(&buffer);

    HostStatus::Ok.code()
}

/// Copy one device payload into caller buffers.
unsafe fn write_device_payload(
    descriptor: *mut AndroidHostBluetoothDeviceDescriptorHeader,
    string_bytes: NativeSlice<u8>,
    string_bytes_written: *mut u32,
) -> u32 {
    let mut buffer = Vec::new();
    let device = build_device_header(&mut buffer);

    // report required string storage first
    if string_bytes.data.is_null() || string_bytes.len < buffer.len() as u32 {
        unsafe {
            *string_bytes_written = buffer.len() as u32;
        }
        return HostStatus::BufferTooSmall.code();
    }

    // write the opened device payload and string bytes
    unsafe {
        *descriptor = device;
        *string_bytes_written = buffer.len() as u32;
    }

    let output =
        unsafe { std::slice::from_raw_parts_mut(string_bytes.data, string_bytes.len as usize) };
    output[..buffer.len()].copy_from_slice(&buffer);

    HostStatus::Ok.code()
}

/// Handle one adapter-list callback.
unsafe extern "C" fn test_adapter_list(
    _runtime_id: u64,
    adapters: NativeSlice<AndroidHostBluetoothAdapterDescriptorHeader>,
    adapter_count_written: *mut u32,
    string_bytes: NativeSlice<u8>,
    string_bytes_written: *mut u32,
) -> u32 {
    // require writable count outputs
    if adapter_count_written.is_null() || string_bytes_written.is_null() {
        return HostStatus::InvalidArgument.code();
    }

    // forward to the shared adapter payload helper
    unsafe {
        write_adapter_payload(
            adapters,
            adapter_count_written,
            string_bytes,
            string_bytes_written,
        )
    }
}

/// Handle one scan-open callback.
unsafe extern "C" fn test_scan_open(
    _runtime_id: u64,
    adapter_id: NativeStringRef,
    filter_flags: u32,
    session_id: *mut u64,
) -> u32 {
    // require one writable session output
    if session_id.is_null() {
        return HostStatus::InvalidArgument.code();
    }

    // record the forwarded request
    let mut state = test_state()
        .lock()
        .expect("android bluetooth test state should not be poisoned");
    state.scan_open = Some(RecordedScanOpenCall {
        adapter_id: decode_native_string(adapter_id),
        filter_flags,
    });

    // return the deterministic scan session id
    unsafe {
        *session_id = TEST_SCAN_SESSION_ID;
    }

    HostStatus::Ok.code()
}

/// Handle one scan-close callback.
unsafe extern "C" fn test_scan_close(_runtime_id: u64, session_id: u64) -> u32 {
    // record the closed session id
    test_state()
        .lock()
        .expect("android bluetooth test state should not be poisoned")
        .closed_scan_sessions
        .push(session_id);

    HostStatus::Ok.code()
}

/// Handle one device-open callback.
unsafe extern "C" fn test_open(
    _runtime_id: u64,
    adapter_id: NativeStringRef,
    device_id: NativeStringRef,
    session_id: *mut u64,
    descriptor: *mut AndroidHostBluetoothDeviceDescriptorHeader,
    string_bytes: NativeSlice<u8>,
    string_bytes_written: *mut u32,
) -> u32 {
    // require writable opened-session outputs
    if session_id.is_null() || descriptor.is_null() || string_bytes_written.is_null() {
        return HostStatus::InvalidArgument.code();
    }

    // record the forwarded request
    let mut state = test_state()
        .lock()
        .expect("android bluetooth test state should not be poisoned");
    state.device_open = Some(RecordedDeviceOpenCall {
        adapter_id: decode_native_string(adapter_id),
        device_id: decode_native_string(device_id),
    });

    // return the deterministic opened-session payload
    unsafe {
        *session_id = TEST_DEVICE_SESSION_ID;
        write_device_payload(descriptor, string_bytes, string_bytes_written)
    }
}

/// Handle one device-close callback.
unsafe extern "C" fn test_close(_runtime_id: u64, session_id: u64) -> u32 {
    // record the closed device session id
    test_state()
        .lock()
        .expect("android bluetooth test state should not be poisoned")
        .closed_device_sessions
        .push(session_id);

    HostStatus::Ok.code()
}

/// Build one complete Android bluetooth callback table.
fn test_callbacks() -> AndroidHostBluetoothCallbacks {
    AndroidHostBluetoothCallbacks {
        adapter_list: Some(test_adapter_list),
        scan_open: Some(test_scan_open),
        scan_close: Some(test_scan_close),
        open: Some(test_open),
        close: Some(test_close),
        ..AndroidHostBluetoothCallbacks::default()
    }
}

/// Register the deterministic Android bluetooth callback table.
pub(super) fn register_test_callbacks() -> AndroidBluetoothTestCallbacks {
    // register one temporary android runtime and reset the recorded state
    let (queue, registration, runtime_id) = register_android_runtime();
    reset_test_state();

    // install the deterministic callback table
    let status = register_android_bindings_bluetooth(runtime_id, test_callbacks());
    assert_eq!(status, HostStatus::Ok.code());

    AndroidBluetoothTestCallbacks {
        runtime_id,
        _queue: queue,
        _registration: registration,
    }
}
