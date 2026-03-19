#![allow(unreachable_pub)]

use super::types::{
    AndroidHostBluetoothAdapterDescriptorHeader, AndroidHostBluetoothCallbacks,
    AndroidHostBluetoothDeviceDescriptorHeader, AndroidHostBluetoothGattCharacteristicHeader,
    AndroidHostBluetoothGattDescriptorHeader, AndroidHostBluetoothGattServiceHeader,
    AndroidHostBluetoothScanEventHeader, AndroidHostBluetoothSessionEventHeader,
};
use crate::host::abi::HostStatus;
use crate::host::android::bridge::bindings::invoke_android_binding_callback;
use crate::runtime::{NativeSlice, NativeStringRef};

/// Resolve and invoke one Android host bluetooth callback.
fn call_android_bluetooth_callback<T: Copy>(
    runtime_id: u64,
    resolve: impl FnOnce(&AndroidHostBluetoothCallbacks) -> Option<T>,
    invoke: impl FnOnce(T) -> u32,
) -> u32 {
    invoke_android_binding_callback(runtime_id, |bindings| resolve(&bindings.bluetooth), invoke)
}

macro_rules! bluetooth_callback {
    ($name:ident ($($arg:ident : $ty:ty),* $(,)?) -> $resolve:ident $(; guard $guard:expr)? ) => {
        #[doc = "Route one Android host bluetooth callback through the registered callback table."]
        #[unsafe(no_mangle)]
        pub(crate) unsafe extern "C" fn $name(runtime_id: u64, $($arg: $ty),*) -> u32 {
            $(if !($guard) { return HostStatus::InvalidArgument.code(); })?
            call_android_bluetooth_callback(runtime_id, |callbacks| callbacks.$resolve, |callback| unsafe {
                callback(runtime_id, $($arg),*)
            })
        }
    };
}

bluetooth_callback!(
    destack_host_android_bluetooth_adapter_list(
        adapters: NativeSlice<AndroidHostBluetoothAdapterDescriptorHeader>,
        adapter_count_written: *mut u32,
        string_bytes: NativeSlice<u8>,
        string_bytes_written: *mut u32
    ) -> adapter_list;
    guard !adapter_count_written.is_null() && !string_bytes_written.is_null()
);
bluetooth_callback!(destack_host_android_bluetooth_scan_open(adapter_id: NativeStringRef, filter_flags: u32, session_id: *mut u64) -> scan_open; guard !session_id.is_null());
bluetooth_callback!(destack_host_android_bluetooth_scan_close(session_id: u64) -> scan_close);
bluetooth_callback!(
    destack_host_android_bluetooth_scan_read(
        session_id: u64,
        timeout_ns: u64,
        devices: NativeSlice<AndroidHostBluetoothDeviceDescriptorHeader>,
        device_count_written: *mut u32,
        string_bytes: NativeSlice<u8>,
        string_bytes_written: *mut u32
    ) -> scan_read;
    guard !device_count_written.is_null() && !string_bytes_written.is_null()
);
bluetooth_callback!(
    destack_host_android_bluetooth_scan_try_read(
        session_id: u64,
        devices: NativeSlice<AndroidHostBluetoothDeviceDescriptorHeader>,
        device_count_written: *mut u32,
        string_bytes: NativeSlice<u8>,
        string_bytes_written: *mut u32
    ) -> scan_try_read;
    guard !device_count_written.is_null() && !string_bytes_written.is_null()
);
bluetooth_callback!(
    destack_host_android_bluetooth_scan_read_event(
        session_id: u64,
        timeout_ns: u64,
        event: *mut AndroidHostBluetoothScanEventHeader,
        string_bytes: NativeSlice<u8>,
        string_bytes_written: *mut u32
    ) -> scan_read_event;
    guard !event.is_null() && !string_bytes_written.is_null()
);
bluetooth_callback!(
    destack_host_android_bluetooth_scan_try_read_event(
        session_id: u64,
        event: *mut AndroidHostBluetoothScanEventHeader,
        string_bytes: NativeSlice<u8>,
        string_bytes_written: *mut u32
    ) -> scan_try_read_event;
    guard !event.is_null() && !string_bytes_written.is_null()
);
bluetooth_callback!(
    destack_host_android_bluetooth_open(
        adapter_id: NativeStringRef,
        device_id: NativeStringRef,
        session_id: *mut u64,
        descriptor: *mut AndroidHostBluetoothDeviceDescriptorHeader,
        string_bytes: NativeSlice<u8>,
        string_bytes_written: *mut u32
    ) -> open;
    guard !session_id.is_null() && !descriptor.is_null() && !string_bytes_written.is_null()
);
bluetooth_callback!(destack_host_android_bluetooth_close(session_id: u64) -> close);
bluetooth_callback!(destack_host_android_bluetooth_pair(session_id: u64, timeout_ns: u64) -> pair);
bluetooth_callback!(destack_host_android_bluetooth_unpair(adapter_id: NativeStringRef, device_id: NativeStringRef) -> unpair);
bluetooth_callback!(destack_host_android_bluetooth_read_rssi(session_id: u64, timeout_ns: u64, rssi_dbm: *mut i16) -> read_rssi; guard !rssi_dbm.is_null());
bluetooth_callback!(destack_host_android_bluetooth_session_read_event(session_id: u64, timeout_ns: u64, event: *mut AndroidHostBluetoothSessionEventHeader) -> session_read_event; guard !event.is_null());
bluetooth_callback!(destack_host_android_bluetooth_session_try_read_event(session_id: u64, event: *mut AndroidHostBluetoothSessionEventHeader) -> session_try_read_event; guard !event.is_null());
bluetooth_callback!(
    destack_host_android_bluetooth_gatt_service_list(
        session_id: u64,
        services: NativeSlice<AndroidHostBluetoothGattServiceHeader>,
        service_count_written: *mut u32,
        string_bytes: NativeSlice<u8>,
        string_bytes_written: *mut u32
    ) -> gatt_service_list;
    guard !service_count_written.is_null() && !string_bytes_written.is_null()
);
bluetooth_callback!(
    destack_host_android_bluetooth_gatt_characteristic_list(
        session_id: u64,
        service_id: NativeStringRef,
        characteristics: NativeSlice<AndroidHostBluetoothGattCharacteristicHeader>,
        characteristic_count_written: *mut u32,
        string_bytes: NativeSlice<u8>,
        string_bytes_written: *mut u32
    ) -> gatt_characteristic_list;
    guard !characteristic_count_written.is_null() && !string_bytes_written.is_null()
);
bluetooth_callback!(
    destack_host_android_bluetooth_gatt_descriptor_list(
        session_id: u64,
        characteristic_id: NativeStringRef,
        descriptors: NativeSlice<AndroidHostBluetoothGattDescriptorHeader>,
        descriptor_count_written: *mut u32,
        string_bytes: NativeSlice<u8>,
        string_bytes_written: *mut u32
    ) -> gatt_descriptor_list;
    guard !descriptor_count_written.is_null() && !string_bytes_written.is_null()
);
bluetooth_callback!(destack_host_android_bluetooth_gatt_mtu(session_id: u64, mtu: *mut u16) -> gatt_mtu; guard !mtu.is_null());
bluetooth_callback!(destack_host_android_bluetooth_gatt_read(session_id: u64, characteristic_id: NativeStringRef, timeout_ns: u64, bytes: NativeSlice<u8>, bytes_written: *mut u32) -> gatt_read; guard !bytes_written.is_null());
bluetooth_callback!(destack_host_android_bluetooth_gatt_read_descriptor(session_id: u64, descriptor_id: NativeStringRef, timeout_ns: u64, bytes: NativeSlice<u8>, bytes_written: *mut u32) -> gatt_read_descriptor; guard !bytes_written.is_null());
bluetooth_callback!(destack_host_android_bluetooth_gatt_write(session_id: u64, characteristic_id: NativeStringRef, mode: u32, value: NativeSlice<u8>, timeout_ns: u64) -> gatt_write);
bluetooth_callback!(destack_host_android_bluetooth_gatt_write_descriptor(session_id: u64, descriptor_id: NativeStringRef, value: NativeSlice<u8>, timeout_ns: u64) -> gatt_write_descriptor);
bluetooth_callback!(destack_host_android_bluetooth_gatt_subscribe(session_id: u64, characteristic_id: NativeStringRef, subscription_id: *mut u64) -> gatt_subscribe; guard !subscription_id.is_null());
bluetooth_callback!(destack_host_android_bluetooth_gatt_unsubscribe(subscription_id: u64) -> gatt_unsubscribe);
bluetooth_callback!(destack_host_android_bluetooth_gatt_read_event(subscription_id: u64, timeout_ns: u64, event: *mut NativeSlice<u8>, timestamp_ns: *mut u64) -> gatt_read_event; guard !event.is_null() && !timestamp_ns.is_null());
bluetooth_callback!(destack_host_android_bluetooth_gatt_try_read_event(subscription_id: u64, event: *mut NativeSlice<u8>, timestamp_ns: *mut u64) -> gatt_try_read_event; guard !event.is_null() && !timestamp_ns.is_null());
