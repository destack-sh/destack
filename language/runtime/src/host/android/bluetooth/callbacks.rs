use crate::runtime::{NativeSlice, NativeStringRef};

use super::types::{
    AndroidHostBluetoothAdapterDescriptorHeader, AndroidHostBluetoothDeviceDescriptorHeader,
    AndroidHostBluetoothGattCharacteristicHeader, AndroidHostBluetoothGattDescriptorHeader,
    AndroidHostBluetoothGattServiceHeader, AndroidHostBluetoothScanEventHeader,
    AndroidHostBluetoothSessionEventHeader,
};

/// Host callback for listing one slice of Android bluetooth adapters.
pub(crate) type AndroidHostBluetoothAdapterListCallback = unsafe extern "C" fn(
    runtime_id: u64,
    adapters: NativeSlice<AndroidHostBluetoothAdapterDescriptorHeader>,
    adapter_count_written: *mut u32,
    string_bytes: NativeSlice<u8>,
    string_bytes_written: *mut u32,
) -> u32;

/// Host callback for opening one Android bluetooth scan session.
pub(crate) type AndroidHostBluetoothScanOpenCallback = unsafe extern "C" fn(
    runtime_id: u64,
    adapter_id: NativeStringRef,
    filter_flags: u32,
    session_id: *mut u64,
) -> u32;

/// Host callback for closing one Android bluetooth scan session.
pub(crate) type AndroidHostBluetoothScanCloseCallback =
    unsafe extern "C" fn(runtime_id: u64, session_id: u64) -> u32;

/// Host callback for reading one batch of Android bluetooth scan results.
pub(crate) type AndroidHostBluetoothScanReadCallback = unsafe extern "C" fn(
    runtime_id: u64,
    session_id: u64,
    timeout_ns: u64,
    devices: NativeSlice<AndroidHostBluetoothDeviceDescriptorHeader>,
    device_count_written: *mut u32,
    string_bytes: NativeSlice<u8>,
    string_bytes_written: *mut u32,
) -> u32;

/// Host callback for trying one non-blocking Android bluetooth scan read.
pub(crate) type AndroidHostBluetoothScanTryReadCallback = unsafe extern "C" fn(
    runtime_id: u64,
    session_id: u64,
    devices: NativeSlice<AndroidHostBluetoothDeviceDescriptorHeader>,
    device_count_written: *mut u32,
    string_bytes: NativeSlice<u8>,
    string_bytes_written: *mut u32,
) -> u32;

/// Host callback for reading one Android bluetooth scan event.
pub(crate) type AndroidHostBluetoothScanReadEventCallback = unsafe extern "C" fn(
    runtime_id: u64,
    session_id: u64,
    timeout_ns: u64,
    event: *mut AndroidHostBluetoothScanEventHeader,
    string_bytes: NativeSlice<u8>,
    string_bytes_written: *mut u32,
) -> u32;

/// Host callback for trying one non-blocking Android bluetooth scan event read.
pub(crate) type AndroidHostBluetoothScanTryReadEventCallback = unsafe extern "C" fn(
    runtime_id: u64,
    session_id: u64,
    event: *mut AndroidHostBluetoothScanEventHeader,
    string_bytes: NativeSlice<u8>,
    string_bytes_written: *mut u32,
) -> u32;

/// Host callback for opening one Android bluetooth device session.
pub(crate) type AndroidHostBluetoothOpenCallback = unsafe extern "C" fn(
    runtime_id: u64,
    adapter_id: NativeStringRef,
    device_id: NativeStringRef,
    session_id: *mut u64,
    descriptor: *mut AndroidHostBluetoothDeviceDescriptorHeader,
    string_bytes: NativeSlice<u8>,
    string_bytes_written: *mut u32,
) -> u32;

/// Host callback for closing one Android bluetooth device session.
pub(crate) type AndroidHostBluetoothCloseCallback =
    unsafe extern "C" fn(runtime_id: u64, session_id: u64) -> u32;

/// Host callback for pairing one Android bluetooth device session.
pub(crate) type AndroidHostBluetoothPairCallback =
    unsafe extern "C" fn(runtime_id: u64, session_id: u64, timeout_ns: u64) -> u32;

/// Host callback for unpairing one Android bluetooth device.
pub(crate) type AndroidHostBluetoothUnpairCallback = unsafe extern "C" fn(
    runtime_id: u64,
    adapter_id: NativeStringRef,
    device_id: NativeStringRef,
) -> u32;

/// Host callback for reading one Android bluetooth RSSI sample.
pub(crate) type AndroidHostBluetoothReadRssiCallback = unsafe extern "C" fn(
    runtime_id: u64,
    session_id: u64,
    timeout_ns: u64,
    rssi_dbm: *mut i16,
) -> u32;

/// Host callback for reading one Android bluetooth session event.
pub(crate) type AndroidHostBluetoothSessionReadEventCallback = unsafe extern "C" fn(
    runtime_id: u64,
    session_id: u64,
    timeout_ns: u64,
    event: *mut AndroidHostBluetoothSessionEventHeader,
) -> u32;

/// Host callback for trying one non-blocking Android bluetooth session event read.
pub(crate) type AndroidHostBluetoothSessionTryReadEventCallback = unsafe extern "C" fn(
    runtime_id: u64,
    session_id: u64,
    event: *mut AndroidHostBluetoothSessionEventHeader,
) -> u32;

/// Host callback for listing one Android bluetooth GATT service slice.
pub(crate) type AndroidHostBluetoothGattServiceListCallback = unsafe extern "C" fn(
    runtime_id: u64,
    session_id: u64,
    services: NativeSlice<AndroidHostBluetoothGattServiceHeader>,
    service_count_written: *mut u32,
    string_bytes: NativeSlice<u8>,
    string_bytes_written: *mut u32,
) -> u32;

/// Host callback for listing one Android bluetooth GATT characteristic slice.
pub(crate) type AndroidHostBluetoothGattCharacteristicListCallback = unsafe extern "C" fn(
    runtime_id: u64,
    session_id: u64,
    service_id: NativeStringRef,
    characteristics: NativeSlice<AndroidHostBluetoothGattCharacteristicHeader>,
    characteristic_count_written: *mut u32,
    string_bytes: NativeSlice<u8>,
    string_bytes_written: *mut u32,
) -> u32;

/// Host callback for listing one Android bluetooth GATT descriptor slice.
pub(crate) type AndroidHostBluetoothGattDescriptorListCallback = unsafe extern "C" fn(
    runtime_id: u64,
    session_id: u64,
    characteristic_id: NativeStringRef,
    descriptors: NativeSlice<AndroidHostBluetoothGattDescriptorHeader>,
    descriptor_count_written: *mut u32,
    string_bytes: NativeSlice<u8>,
    string_bytes_written: *mut u32,
) -> u32;

/// Host callback for querying one Android bluetooth GATT MTU.
pub(crate) type AndroidHostBluetoothGattMtuCallback =
    unsafe extern "C" fn(runtime_id: u64, session_id: u64, mtu: *mut u16) -> u32;

/// Host callback for reading one Android bluetooth GATT characteristic value.
pub(crate) type AndroidHostBluetoothGattReadCallback = unsafe extern "C" fn(
    runtime_id: u64,
    session_id: u64,
    characteristic_id: NativeStringRef,
    timeout_ns: u64,
    bytes: NativeSlice<u8>,
    bytes_written: *mut u32,
) -> u32;

/// Host callback for reading one Android bluetooth GATT descriptor value.
pub(crate) type AndroidHostBluetoothGattReadDescriptorCallback = unsafe extern "C" fn(
    runtime_id: u64,
    session_id: u64,
    descriptor_id: NativeStringRef,
    timeout_ns: u64,
    bytes: NativeSlice<u8>,
    bytes_written: *mut u32,
) -> u32;

/// Host callback for writing one Android bluetooth GATT characteristic value.
pub(crate) type AndroidHostBluetoothGattWriteCallback = unsafe extern "C" fn(
    runtime_id: u64,
    session_id: u64,
    characteristic_id: NativeStringRef,
    mode: u32,
    value: NativeSlice<u8>,
    timeout_ns: u64,
) -> u32;

/// Host callback for writing one Android bluetooth GATT descriptor value.
pub(crate) type AndroidHostBluetoothGattWriteDescriptorCallback = unsafe extern "C" fn(
    runtime_id: u64,
    session_id: u64,
    descriptor_id: NativeStringRef,
    value: NativeSlice<u8>,
    timeout_ns: u64,
) -> u32;

/// Host callback for subscribing to one Android bluetooth GATT characteristic.
pub(crate) type AndroidHostBluetoothGattSubscribeCallback = unsafe extern "C" fn(
    runtime_id: u64,
    session_id: u64,
    characteristic_id: NativeStringRef,
    subscription_id: *mut u64,
) -> u32;

/// Host callback for unsubscribing from one Android bluetooth GATT characteristic.
pub(crate) type AndroidHostBluetoothGattUnsubscribeCallback =
    unsafe extern "C" fn(runtime_id: u64, subscription_id: u64) -> u32;

/// Host callback for reading one Android bluetooth GATT subscription event.
pub(crate) type AndroidHostBluetoothGattReadEventCallback = unsafe extern "C" fn(
    runtime_id: u64,
    subscription_id: u64,
    timeout_ns: u64,
    event: *mut NativeSlice<u8>,
    timestamp_ns: *mut u64,
) -> u32;

/// Host callback for trying one non-blocking Android bluetooth GATT event read.
pub(crate) type AndroidHostBluetoothGattTryReadEventCallback = unsafe extern "C" fn(
    runtime_id: u64,
    subscription_id: u64,
    event: *mut NativeSlice<u8>,
    timestamp_ns: *mut u64,
) -> u32;
