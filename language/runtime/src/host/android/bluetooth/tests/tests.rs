use std::sync::{Arc, Mutex, OnceLock};

use crate::host::abi::HostStatus;
use crate::host::android::bluetooth::types::{
    AndroidHostBluetoothAdapterDescriptorHeader, AndroidHostBluetoothCallbacks,
    AndroidHostBluetoothDeviceDescriptorHeader, AndroidHostBluetoothGattCharacteristicHeader,
    AndroidHostBluetoothGattDescriptorHeader, AndroidHostBluetoothGattServiceHeader,
    AndroidHostBluetoothScanEventHeader, AndroidHostBluetoothSessionEventHeader,
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
/// One deterministic Android bluetooth subscription id.
pub(super) const TEST_SUBSCRIPTION_ID: u64 = 303;
/// One deterministic Android bluetooth not-supported status.
pub(super) const TEST_STATUS_NOT_SUPPORTED: u32 = HostStatus::NotSupported.code();
/// One deterministic Android bluetooth not-found status.
pub(super) const TEST_STATUS_NOT_FOUND: u32 = HostStatus::NotFound.code();
/// One deterministic Android bluetooth GATT service id.
pub(super) const TEST_SERVICE_ID: &str = "android.bluetooth.service";
/// One deterministic Android bluetooth GATT service uuid.
pub(super) const TEST_SERVICE_UUID: &str = "12345678-1234-1234-1234-1234567890ab";
/// One deterministic Android bluetooth GATT characteristic id.
pub(super) const TEST_CHARACTERISTIC_ID: &str = "android.bluetooth.characteristic";
/// One deterministic Android bluetooth GATT characteristic uuid.
pub(super) const TEST_CHARACTERISTIC_UUID: &str = "abcdefab-cdef-cdef-cdef-abcdefabcdef";
/// One deterministic Android bluetooth GATT descriptor id.
pub(super) const TEST_DESCRIPTOR_ID: &str = "android.bluetooth.descriptor";
/// One deterministic Android bluetooth GATT descriptor uuid.
pub(super) const TEST_DESCRIPTOR_UUID: &str = "2902";
/// One deterministic Android bluetooth ATT MTU.
pub(super) const TEST_GATT_MTU: u16 = 185;
/// One deterministic Android bluetooth GATT read payload.
pub(super) const TEST_GATT_READ_VALUE: &[u8] = &[0xde, 0xad, 0xbe, 0xef];
/// One deterministic Android bluetooth GATT descriptor read payload.
pub(super) const TEST_GATT_DESCRIPTOR_VALUE: &[u8] = &[0xca, 0xfe];
/// One deterministic Android bluetooth notification payload.
pub(super) const TEST_GATT_EVENT_VALUE: &[u8] = &[0xaa, 0xbb, 0xcc];

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

/// One recorded Android bluetooth unpair call.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct RecordedDeviceUnpairCall {
    /// The forwarded adapter id.
    pub adapter_id: String,
    /// The forwarded device id.
    pub device_id: String,
}

/// One recorded Android bluetooth pair call.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct RecordedPairCall {
    /// The forwarded device session id.
    pub session_id: u64,
    /// The forwarded timeout in nanoseconds.
    pub timeout_ns: u64,
}

/// One recorded Android bluetooth RSSI call.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct RecordedReadRssiCall {
    /// The forwarded device session id.
    pub session_id: u64,
    /// The forwarded timeout in nanoseconds.
    pub timeout_ns: u64,
}

/// One recorded Android bluetooth scan event read.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct RecordedScanReadCall {
    /// The forwarded scan session id.
    pub session_id: u64,
    /// The forwarded timeout in nanoseconds.
    pub timeout_ns: Option<u64>,
}

/// One recorded Android bluetooth scan event read.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct RecordedScanEventReadCall {
    /// The forwarded scan session id.
    pub session_id: u64,
    /// The forwarded timeout in nanoseconds.
    pub timeout_ns: Option<u64>,
}

/// One recorded Android bluetooth session event read.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct RecordedSessionEventReadCall {
    /// The forwarded device session id.
    pub session_id: u64,
    /// The forwarded timeout in nanoseconds.
    pub timeout_ns: Option<u64>,
}

/// One recorded Android bluetooth GATT characteristic list call.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct RecordedGattCharacteristicListCall {
    /// The forwarded device session id.
    pub session_id: u64,
    /// The forwarded service id.
    pub service_id: String,
}

/// One recorded Android bluetooth GATT descriptor list call.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct RecordedGattDescriptorListCall {
    /// The forwarded device session id.
    pub session_id: u64,
    /// The forwarded characteristic id.
    pub characteristic_id: String,
}

/// One recorded Android bluetooth GATT read call.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct RecordedGattReadCall {
    /// The forwarded device session id.
    pub session_id: u64,
    /// The forwarded characteristic id.
    pub characteristic_id: String,
    /// The forwarded timeout in nanoseconds.
    pub timeout_ns: u64,
}

/// One recorded Android bluetooth GATT descriptor read call.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct RecordedGattReadDescriptorCall {
    /// The forwarded device session id.
    pub session_id: u64,
    /// The forwarded descriptor id.
    pub descriptor_id: String,
    /// The forwarded timeout in nanoseconds.
    pub timeout_ns: u64,
}

/// One recorded Android bluetooth GATT write call.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct RecordedGattWriteCall {
    /// The forwarded device session id.
    pub session_id: u64,
    /// The forwarded characteristic id.
    pub characteristic_id: String,
    /// The forwarded write mode.
    pub mode: u32,
    /// The forwarded payload bytes.
    pub value: Vec<u8>,
    /// The forwarded timeout in nanoseconds.
    pub timeout_ns: u64,
}

/// One recorded Android bluetooth GATT descriptor write call.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct RecordedGattWriteDescriptorCall {
    /// The forwarded device session id.
    pub session_id: u64,
    /// The forwarded descriptor id.
    pub descriptor_id: String,
    /// The forwarded payload bytes.
    pub value: Vec<u8>,
    /// The forwarded timeout in nanoseconds.
    pub timeout_ns: u64,
}

/// One recorded Android bluetooth GATT subscribe call.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct RecordedGattSubscribeCall {
    /// The forwarded device session id.
    pub session_id: u64,
    /// The forwarded characteristic id.
    pub characteristic_id: String,
}

/// One recorded Android bluetooth GATT event read.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct RecordedGattEventReadCall {
    /// The forwarded subscription id.
    pub subscription_id: u64,
    /// The forwarded timeout in nanoseconds.
    pub timeout_ns: Option<u64>,
}

/// One snapshot of recorded Android bluetooth callback traffic.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct AndroidBluetoothTestState {
    /// The most recent scan-open call.
    pub scan_open: Option<RecordedScanOpenCall>,
    /// The most recent device-open call.
    pub device_open: Option<RecordedDeviceOpenCall>,
    /// The most recent pair call.
    pub pair: Option<RecordedPairCall>,
    /// The most recent unpair call.
    pub unpair: Option<RecordedDeviceUnpairCall>,
    /// The most recent RSSI call.
    pub read_rssi: Option<RecordedReadRssiCall>,
    /// The most recent scan descriptor read.
    pub scan_read: Option<RecordedScanReadCall>,
    /// The most recent scan event read.
    pub scan_event_read: Option<RecordedScanEventReadCall>,
    /// The most recent session event read.
    pub session_event_read: Option<RecordedSessionEventReadCall>,
    /// The most recent GATT service-list call session id.
    pub gatt_service_list_session_id: Option<u64>,
    /// The most recent GATT characteristic-list call.
    pub gatt_characteristic_list: Option<RecordedGattCharacteristicListCall>,
    /// The most recent GATT descriptor-list call.
    pub gatt_descriptor_list: Option<RecordedGattDescriptorListCall>,
    /// The most recent GATT read call.
    pub gatt_read: Option<RecordedGattReadCall>,
    /// The most recent GATT descriptor read call.
    pub gatt_read_descriptor: Option<RecordedGattReadDescriptorCall>,
    /// The most recent GATT write call.
    pub gatt_write: Option<RecordedGattWriteCall>,
    /// The most recent GATT descriptor write call.
    pub gatt_write_descriptor: Option<RecordedGattWriteDescriptorCall>,
    /// The most recent GATT subscribe call.
    pub gatt_subscribe: Option<RecordedGattSubscribeCall>,
    /// The most recent GATT event read.
    pub gatt_event_read: Option<RecordedGattEventReadCall>,
    /// Closed scan session ids.
    pub closed_scan_sessions: Vec<u64>,
    /// Closed device session ids.
    pub closed_device_sessions: Vec<u64>,
    /// Closed subscription ids.
    pub closed_subscription_ids: Vec<u64>,
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

/// Build one deterministic GATT service header and append its strings.
fn build_service_header(buffer: &mut Vec<u8>) -> AndroidHostBluetoothGattServiceHeader {
    let (id_offset, id_len) = append_string(buffer, TEST_SERVICE_ID);
    let (uuid_offset, uuid_len) = append_string(buffer, TEST_SERVICE_UUID);

    AndroidHostBluetoothGattServiceHeader {
        id_offset,
        id_len,
        uuid_offset,
        uuid_len,
        is_primary: 1,
    }
}

/// Build one deterministic GATT characteristic header and append its strings.
fn build_characteristic_header(
    buffer: &mut Vec<u8>,
) -> AndroidHostBluetoothGattCharacteristicHeader {
    let (id_offset, id_len) = append_string(buffer, TEST_CHARACTERISTIC_ID);
    let (service_id_offset, service_id_len) = append_string(buffer, TEST_SERVICE_ID);
    let (uuid_offset, uuid_len) = append_string(buffer, TEST_CHARACTERISTIC_UUID);

    AndroidHostBluetoothGattCharacteristicHeader {
        id_offset,
        id_len,
        service_id_offset,
        service_id_len,
        uuid_offset,
        uuid_len,
        property_flags: (1 << 1) | (1 << 3) | (1 << 4),
    }
}

/// Build one deterministic GATT descriptor header and append its strings.
fn build_descriptor_header(buffer: &mut Vec<u8>) -> AndroidHostBluetoothGattDescriptorHeader {
    let (id_offset, id_len) = append_string(buffer, TEST_DESCRIPTOR_ID);
    let (characteristic_id_offset, characteristic_id_len) =
        append_string(buffer, TEST_CHARACTERISTIC_ID);
    let (uuid_offset, uuid_len) = append_string(buffer, TEST_DESCRIPTOR_UUID);

    AndroidHostBluetoothGattDescriptorHeader {
        id_offset,
        id_len,
        characteristic_id_offset,
        characteristic_id_len,
        uuid_offset,
        uuid_len,
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

/// Copy one device-list payload into caller buffers.
unsafe fn write_device_list_payload(
    devices: NativeSlice<AndroidHostBluetoothDeviceDescriptorHeader>,
    device_count_written: *mut u32,
    string_bytes: NativeSlice<u8>,
    string_bytes_written: *mut u32,
) -> u32 {
    let mut buffer = Vec::new();
    let device = build_device_header(&mut buffer);

    if devices.data.is_null()
        || devices.len == 0
        || string_bytes.data.is_null()
        || string_bytes.len < buffer.len() as u32
    {
        unsafe {
            *device_count_written = 1;
            *string_bytes_written = buffer.len() as u32;
        }
        return HostStatus::BufferTooSmall.code();
    }

    unsafe {
        *devices.data = device;
        *device_count_written = 1;
        *string_bytes_written = buffer.len() as u32;
    }

    let output =
        unsafe { std::slice::from_raw_parts_mut(string_bytes.data, string_bytes.len as usize) };
    output[..buffer.len()].copy_from_slice(&buffer);

    HostStatus::Ok.code()
}

/// Copy one service payload into caller buffers.
unsafe fn write_service_payload(
    services: NativeSlice<AndroidHostBluetoothGattServiceHeader>,
    service_count_written: *mut u32,
    string_bytes: NativeSlice<u8>,
    string_bytes_written: *mut u32,
) -> u32 {
    let mut buffer = Vec::new();
    let service = build_service_header(&mut buffer);

    if services.data.is_null()
        || services.len == 0
        || string_bytes.data.is_null()
        || string_bytes.len < buffer.len() as u32
    {
        unsafe {
            *service_count_written = 1;
            *string_bytes_written = buffer.len() as u32;
        }
        return HostStatus::BufferTooSmall.code();
    }

    unsafe {
        *services.data = service;
        *service_count_written = 1;
        *string_bytes_written = buffer.len() as u32;
    }

    let output =
        unsafe { std::slice::from_raw_parts_mut(string_bytes.data, string_bytes.len as usize) };
    output[..buffer.len()].copy_from_slice(&buffer);

    HostStatus::Ok.code()
}

/// Copy one characteristic payload into caller buffers.
unsafe fn write_characteristic_payload(
    characteristics: NativeSlice<AndroidHostBluetoothGattCharacteristicHeader>,
    characteristic_count_written: *mut u32,
    string_bytes: NativeSlice<u8>,
    string_bytes_written: *mut u32,
) -> u32 {
    let mut buffer = Vec::new();
    let characteristic = build_characteristic_header(&mut buffer);

    if characteristics.data.is_null()
        || characteristics.len == 0
        || string_bytes.data.is_null()
        || string_bytes.len < buffer.len() as u32
    {
        unsafe {
            *characteristic_count_written = 1;
            *string_bytes_written = buffer.len() as u32;
        }
        return HostStatus::BufferTooSmall.code();
    }

    unsafe {
        *characteristics.data = characteristic;
        *characteristic_count_written = 1;
        *string_bytes_written = buffer.len() as u32;
    }

    let output =
        unsafe { std::slice::from_raw_parts_mut(string_bytes.data, string_bytes.len as usize) };
    output[..buffer.len()].copy_from_slice(&buffer);

    HostStatus::Ok.code()
}

/// Copy one descriptor payload into caller buffers.
unsafe fn write_descriptor_payload(
    descriptors: NativeSlice<AndroidHostBluetoothGattDescriptorHeader>,
    descriptor_count_written: *mut u32,
    string_bytes: NativeSlice<u8>,
    string_bytes_written: *mut u32,
) -> u32 {
    let mut buffer = Vec::new();
    let descriptor = build_descriptor_header(&mut buffer);

    if descriptors.data.is_null()
        || descriptors.len == 0
        || string_bytes.data.is_null()
        || string_bytes.len < buffer.len() as u32
    {
        unsafe {
            *descriptor_count_written = 1;
            *string_bytes_written = buffer.len() as u32;
        }
        return HostStatus::BufferTooSmall.code();
    }

    unsafe {
        *descriptors.data = descriptor;
        *descriptor_count_written = 1;
        *string_bytes_written = buffer.len() as u32;
    }

    let output =
        unsafe { std::slice::from_raw_parts_mut(string_bytes.data, string_bytes.len as usize) };
    output[..buffer.len()].copy_from_slice(&buffer);

    HostStatus::Ok.code()
}

/// Copy one byte payload into one caller-owned byte buffer.
unsafe fn write_byte_payload(
    bytes: &[u8],
    output: NativeSlice<u8>,
    bytes_written: *mut u32,
) -> u32 {
    if bytes_written.is_null() {
        return HostStatus::InvalidArgument.code();
    }

    if output.data.is_null() || output.len < bytes.len() as u32 {
        unsafe {
            *bytes_written = bytes.len() as u32;
        }
        return HostStatus::BufferTooSmall.code();
    }

    unsafe {
        *bytes_written = bytes.len() as u32;
    }

    let output = unsafe { std::slice::from_raw_parts_mut(output.data, output.len as usize) };
    output[..bytes.len()].copy_from_slice(bytes);

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

/// Handle one scan descriptor read callback.
unsafe extern "C" fn test_scan_read(
    _runtime_id: u64,
    session_id: u64,
    timeout_ns: u64,
    devices: NativeSlice<AndroidHostBluetoothDeviceDescriptorHeader>,
    device_count_written: *mut u32,
    string_bytes: NativeSlice<u8>,
    string_bytes_written: *mut u32,
) -> u32 {
    if device_count_written.is_null() || string_bytes_written.is_null() {
        return HostStatus::InvalidArgument.code();
    }

    test_state()
        .lock()
        .expect("android bluetooth test state should not be poisoned")
        .scan_read = Some(RecordedScanReadCall {
        session_id,
        timeout_ns: Some(timeout_ns),
    });

    unsafe {
        write_device_list_payload(
            devices,
            device_count_written,
            string_bytes,
            string_bytes_written,
        )
    }
}

/// Handle one scan descriptor try-read callback.
unsafe extern "C" fn test_scan_try_read(
    runtime_id: u64,
    session_id: u64,
    devices: NativeSlice<AndroidHostBluetoothDeviceDescriptorHeader>,
    device_count_written: *mut u32,
    string_bytes: NativeSlice<u8>,
    string_bytes_written: *mut u32,
) -> u32 {
    let status = unsafe {
        test_scan_read(
            runtime_id,
            session_id,
            0,
            devices,
            device_count_written,
            string_bytes,
            string_bytes_written,
        )
    };

    test_state()
        .lock()
        .expect("android bluetooth test state should not be poisoned")
        .scan_read = Some(RecordedScanReadCall {
        session_id,
        timeout_ns: None,
    });

    status
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

/// Handle one pair callback.
unsafe extern "C" fn test_pair(_runtime_id: u64, session_id: u64, timeout_ns: u64) -> u32 {
    test_state()
        .lock()
        .expect("android bluetooth test state should not be poisoned")
        .pair = Some(RecordedPairCall {
        session_id,
        timeout_ns,
    });

    HostStatus::Ok.code()
}

/// Handle one unpair callback.
unsafe extern "C" fn test_unpair(
    _runtime_id: u64,
    adapter_id: NativeStringRef,
    device_id: NativeStringRef,
) -> u32 {
    test_state()
        .lock()
        .expect("android bluetooth test state should not be poisoned")
        .unpair = Some(RecordedDeviceUnpairCall {
        adapter_id: decode_native_string(adapter_id),
        device_id: decode_native_string(device_id),
    });

    HostStatus::Ok.code()
}

/// Handle one RSSI callback.
unsafe extern "C" fn test_read_rssi(
    _runtime_id: u64,
    session_id: u64,
    timeout_ns: u64,
    rssi_dbm: *mut i16,
) -> u32 {
    if rssi_dbm.is_null() {
        return HostStatus::InvalidArgument.code();
    }

    test_state()
        .lock()
        .expect("android bluetooth test state should not be poisoned")
        .read_rssi = Some(RecordedReadRssiCall {
        session_id,
        timeout_ns,
    });

    unsafe {
        *rssi_dbm = -48;
    }

    HostStatus::Ok.code()
}

/// Handle one scan event read callback.
unsafe extern "C" fn test_scan_read_event(
    _runtime_id: u64,
    session_id: u64,
    timeout_ns: u64,
    event: *mut AndroidHostBluetoothScanEventHeader,
    string_bytes: NativeSlice<u8>,
    string_bytes_written: *mut u32,
) -> u32 {
    if event.is_null() || string_bytes_written.is_null() {
        return HostStatus::InvalidArgument.code();
    }

    test_state()
        .lock()
        .expect("android bluetooth test state should not be poisoned")
        .scan_event_read = Some(RecordedScanEventReadCall {
        session_id,
        timeout_ns: Some(timeout_ns),
    });

    let mut buffer = Vec::new();
    let descriptor = build_device_header(&mut buffer);
    let header = AndroidHostBluetoothScanEventHeader {
        timestamp_ns: 77,
        kind: 1,
        descriptor,
    };

    if string_bytes.data.is_null() || string_bytes.len < buffer.len() as u32 {
        unsafe {
            *string_bytes_written = buffer.len() as u32;
        }
        return HostStatus::BufferTooSmall.code();
    }

    unsafe {
        *event = header;
        *string_bytes_written = buffer.len() as u32;
    }

    let output =
        unsafe { std::slice::from_raw_parts_mut(string_bytes.data, string_bytes.len as usize) };
    output[..buffer.len()].copy_from_slice(&buffer);

    HostStatus::Ok.code()
}

/// Handle one scan event try-read callback.
unsafe extern "C" fn test_scan_try_read_event(
    runtime_id: u64,
    session_id: u64,
    event: *mut AndroidHostBluetoothScanEventHeader,
    string_bytes: NativeSlice<u8>,
    string_bytes_written: *mut u32,
) -> u32 {
    let status = unsafe {
        test_scan_read_event(
            runtime_id,
            session_id,
            0,
            event,
            string_bytes,
            string_bytes_written,
        )
    };

    test_state()
        .lock()
        .expect("android bluetooth test state should not be poisoned")
        .scan_event_read = Some(RecordedScanEventReadCall {
        session_id,
        timeout_ns: None,
    });

    status
}

/// Handle one session event read callback.
unsafe extern "C" fn test_session_read_event(
    _runtime_id: u64,
    session_id: u64,
    timeout_ns: u64,
    event: *mut AndroidHostBluetoothSessionEventHeader,
) -> u32 {
    if event.is_null() {
        return HostStatus::InvalidArgument.code();
    }

    test_state()
        .lock()
        .expect("android bluetooth test state should not be poisoned")
        .session_event_read = Some(RecordedSessionEventReadCall {
        session_id,
        timeout_ns: Some(timeout_ns),
    });

    unsafe {
        *event = AndroidHostBluetoothSessionEventHeader {
            timestamp_ns: 88,
            kind: 2,
            pair_state: 4,
            flags: 0,
        };
    }

    HostStatus::Ok.code()
}

/// Handle one session event try-read callback.
unsafe extern "C" fn test_session_try_read_event(
    runtime_id: u64,
    session_id: u64,
    event: *mut AndroidHostBluetoothSessionEventHeader,
) -> u32 {
    let status = unsafe { test_session_read_event(runtime_id, session_id, 0, event) };

    test_state()
        .lock()
        .expect("android bluetooth test state should not be poisoned")
        .session_event_read = Some(RecordedSessionEventReadCall {
        session_id,
        timeout_ns: None,
    });

    status
}

/// Handle one GATT service-list callback.
unsafe extern "C" fn test_gatt_service_list(
    _runtime_id: u64,
    session_id: u64,
    services: NativeSlice<AndroidHostBluetoothGattServiceHeader>,
    service_count_written: *mut u32,
    string_bytes: NativeSlice<u8>,
    string_bytes_written: *mut u32,
) -> u32 {
    test_state()
        .lock()
        .expect("android bluetooth test state should not be poisoned")
        .gatt_service_list_session_id = Some(session_id);

    unsafe {
        write_service_payload(
            services,
            service_count_written,
            string_bytes,
            string_bytes_written,
        )
    }
}

/// Handle one GATT characteristic-list callback.
unsafe extern "C" fn test_gatt_characteristic_list(
    _runtime_id: u64,
    session_id: u64,
    service_id: NativeStringRef,
    characteristics: NativeSlice<AndroidHostBluetoothGattCharacteristicHeader>,
    characteristic_count_written: *mut u32,
    string_bytes: NativeSlice<u8>,
    string_bytes_written: *mut u32,
) -> u32 {
    test_state()
        .lock()
        .expect("android bluetooth test state should not be poisoned")
        .gatt_characteristic_list = Some(RecordedGattCharacteristicListCall {
        session_id,
        service_id: decode_native_string(service_id),
    });

    unsafe {
        write_characteristic_payload(
            characteristics,
            characteristic_count_written,
            string_bytes,
            string_bytes_written,
        )
    }
}

/// Handle one GATT descriptor-list callback.
unsafe extern "C" fn test_gatt_descriptor_list(
    _runtime_id: u64,
    session_id: u64,
    characteristic_id: NativeStringRef,
    descriptors: NativeSlice<AndroidHostBluetoothGattDescriptorHeader>,
    descriptor_count_written: *mut u32,
    string_bytes: NativeSlice<u8>,
    string_bytes_written: *mut u32,
) -> u32 {
    test_state()
        .lock()
        .expect("android bluetooth test state should not be poisoned")
        .gatt_descriptor_list = Some(RecordedGattDescriptorListCall {
        session_id,
        characteristic_id: decode_native_string(characteristic_id),
    });

    unsafe {
        write_descriptor_payload(
            descriptors,
            descriptor_count_written,
            string_bytes,
            string_bytes_written,
        )
    }
}

/// Handle one GATT MTU callback.
unsafe extern "C" fn test_gatt_mtu(_runtime_id: u64, _session_id: u64, mtu: *mut u16) -> u32 {
    if mtu.is_null() {
        return HostStatus::InvalidArgument.code();
    }

    unsafe {
        *mtu = TEST_GATT_MTU;
    }

    HostStatus::Ok.code()
}

/// Handle one GATT characteristic read callback.
unsafe extern "C" fn test_gatt_read(
    _runtime_id: u64,
    session_id: u64,
    characteristic_id: NativeStringRef,
    timeout_ns: u64,
    bytes: NativeSlice<u8>,
    bytes_written: *mut u32,
) -> u32 {
    test_state()
        .lock()
        .expect("android bluetooth test state should not be poisoned")
        .gatt_read = Some(RecordedGattReadCall {
        session_id,
        characteristic_id: decode_native_string(characteristic_id),
        timeout_ns,
    });

    unsafe { write_byte_payload(TEST_GATT_READ_VALUE, bytes, bytes_written) }
}

/// Handle one GATT descriptor read callback.
unsafe extern "C" fn test_gatt_read_descriptor(
    _runtime_id: u64,
    session_id: u64,
    descriptor_id: NativeStringRef,
    timeout_ns: u64,
    bytes: NativeSlice<u8>,
    bytes_written: *mut u32,
) -> u32 {
    test_state()
        .lock()
        .expect("android bluetooth test state should not be poisoned")
        .gatt_read_descriptor = Some(RecordedGattReadDescriptorCall {
        session_id,
        descriptor_id: decode_native_string(descriptor_id),
        timeout_ns,
    });

    unsafe { write_byte_payload(TEST_GATT_DESCRIPTOR_VALUE, bytes, bytes_written) }
}

/// Handle one GATT characteristic write callback.
unsafe extern "C" fn test_gatt_write(
    _runtime_id: u64,
    session_id: u64,
    characteristic_id: NativeStringRef,
    mode: u32,
    value: NativeSlice<u8>,
    timeout_ns: u64,
) -> u32 {
    let value = unsafe { std::slice::from_raw_parts(value.data, value.len as usize) }.to_vec();

    test_state()
        .lock()
        .expect("android bluetooth test state should not be poisoned")
        .gatt_write = Some(RecordedGattWriteCall {
        session_id,
        characteristic_id: decode_native_string(characteristic_id),
        mode,
        value,
        timeout_ns,
    });

    HostStatus::Ok.code()
}

/// Handle one GATT descriptor write callback.
unsafe extern "C" fn test_gatt_write_descriptor(
    _runtime_id: u64,
    session_id: u64,
    descriptor_id: NativeStringRef,
    value: NativeSlice<u8>,
    timeout_ns: u64,
) -> u32 {
    let value = unsafe { std::slice::from_raw_parts(value.data, value.len as usize) }.to_vec();

    test_state()
        .lock()
        .expect("android bluetooth test state should not be poisoned")
        .gatt_write_descriptor = Some(RecordedGattWriteDescriptorCall {
        session_id,
        descriptor_id: decode_native_string(descriptor_id),
        value,
        timeout_ns,
    });

    HostStatus::Ok.code()
}

/// Handle one GATT subscribe callback.
unsafe extern "C" fn test_gatt_subscribe(
    _runtime_id: u64,
    session_id: u64,
    characteristic_id: NativeStringRef,
    subscription_id: *mut u64,
) -> u32 {
    if subscription_id.is_null() {
        return HostStatus::InvalidArgument.code();
    }

    test_state()
        .lock()
        .expect("android bluetooth test state should not be poisoned")
        .gatt_subscribe = Some(RecordedGattSubscribeCall {
        session_id,
        characteristic_id: decode_native_string(characteristic_id),
    });

    unsafe {
        *subscription_id = TEST_SUBSCRIPTION_ID;
    }

    HostStatus::Ok.code()
}

/// Handle one GATT unsubscribe callback.
unsafe extern "C" fn test_gatt_unsubscribe(_runtime_id: u64, subscription_id: u64) -> u32 {
    test_state()
        .lock()
        .expect("android bluetooth test state should not be poisoned")
        .closed_subscription_ids
        .push(subscription_id);

    HostStatus::Ok.code()
}

/// Handle one GATT event read callback.
unsafe extern "C" fn test_gatt_read_event(
    _runtime_id: u64,
    subscription_id: u64,
    timeout_ns: u64,
    event: *mut NativeSlice<u8>,
    timestamp_ns: *mut u64,
) -> u32 {
    if event.is_null() || timestamp_ns.is_null() {
        return HostStatus::InvalidArgument.code();
    }

    test_state()
        .lock()
        .expect("android bluetooth test state should not be poisoned")
        .gatt_event_read = Some(RecordedGattEventReadCall {
        subscription_id,
        timeout_ns: Some(timeout_ns),
    });

    unsafe {
        *event = NativeSlice {
            data: TEST_GATT_EVENT_VALUE.as_ptr().cast_mut(),
            len: TEST_GATT_EVENT_VALUE.len() as u32,
        };
        *timestamp_ns = 99;
    }

    HostStatus::Ok.code()
}

/// Handle one GATT event try-read callback.
unsafe extern "C" fn test_gatt_try_read_event(
    runtime_id: u64,
    subscription_id: u64,
    event: *mut NativeSlice<u8>,
    timestamp_ns: *mut u64,
) -> u32 {
    let status =
        unsafe { test_gatt_read_event(runtime_id, subscription_id, 0, event, timestamp_ns) };

    test_state()
        .lock()
        .expect("android bluetooth test state should not be poisoned")
        .gatt_event_read = Some(RecordedGattEventReadCall {
        subscription_id,
        timeout_ns: None,
    });

    status
}

/// Build one complete Android bluetooth callback table.
fn test_callbacks() -> AndroidHostBluetoothCallbacks {
    AndroidHostBluetoothCallbacks {
        adapter_list: Some(test_adapter_list),
        scan_open: Some(test_scan_open),
        scan_close: Some(test_scan_close),
        scan_read: Some(test_scan_read),
        scan_try_read: Some(test_scan_try_read),
        scan_read_event: Some(test_scan_read_event),
        scan_try_read_event: Some(test_scan_try_read_event),
        open: Some(test_open),
        close: Some(test_close),
        pair: Some(test_pair),
        unpair: Some(test_unpair),
        read_rssi: Some(test_read_rssi),
        session_read_event: Some(test_session_read_event),
        session_try_read_event: Some(test_session_try_read_event),
        gatt_service_list: Some(test_gatt_service_list),
        gatt_characteristic_list: Some(test_gatt_characteristic_list),
        gatt_descriptor_list: Some(test_gatt_descriptor_list),
        gatt_mtu: Some(test_gatt_mtu),
        gatt_read: Some(test_gatt_read),
        gatt_read_descriptor: Some(test_gatt_read_descriptor),
        gatt_write: Some(test_gatt_write),
        gatt_write_descriptor: Some(test_gatt_write_descriptor),
        gatt_subscribe: Some(test_gatt_subscribe),
        gatt_unsubscribe: Some(test_gatt_unsubscribe),
        gatt_read_event: Some(test_gatt_read_event),
        gatt_try_read_event: Some(test_gatt_try_read_event),
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
