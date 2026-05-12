pub(super) use std::collections::BTreeMap;
pub(super) use std::sync::Arc;

pub(super) use parking_lot::Mutex;

pub(super) use crate::diagnostic::{RuntimeError, RuntimeResult};
pub(super) use crate::host::HostStatus;
pub(super) use crate::host::os::android::abi::bluetooth::ffi::{
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
    destack_host_android_bluetooth_scan_read_event,
    destack_host_android_bluetooth_scan_try_read_event,
    destack_host_android_bluetooth_session_read_event,
    destack_host_android_bluetooth_session_try_read_event, destack_host_android_bluetooth_unpair,
};
pub(super) use crate::host::os::android::abi::bluetooth::types::{
    AndroidHostBluetoothAdapterDescriptorHeader, AndroidHostBluetoothDeviceDescriptorHeader,
    AndroidHostBluetoothGattCharacteristicHeader, AndroidHostBluetoothGattDescriptorHeader,
    AndroidHostBluetoothGattServiceHeader, AndroidHostBluetoothScanEventHeader,
    AndroidHostBluetoothScanFilterHeader, AndroidHostBluetoothSessionEventHeader,
};
pub(super) use crate::platform::abi::{NativeSlice, NativeStringRef};
pub(super) use crate::platform::core::android::{
    checked_u32_length, host_session_id, host_status_result, invalid_data,
};
pub(super) use crate::platform::core::{
    self as core_platform, BoundedQueue, NativeAbiCodec, decode_optional_string,
    decode_required_string,
};
pub(super) use crate::platform::device::bluetooth::core::{
    BLUETOOTH_ADAPTER_WATCH_RESOURCE_LABEL, BLUETOOTH_DEVICE_RESOURCE_LABEL,
    BLUETOOTH_SCAN_RESOURCE_LABEL, BLUETOOTH_SUBSCRIPTION_RESOURCE_LABEL, BluetoothEventState,
    adapter_attached_event, adapter_changed_event, adapter_detached_event, bluetooth_payload,
    close_bluetooth_resource, disconnected_event, gatt_database_changed_event, gatt_value_event,
    matches_scan_filter, pair_state_changed_event, scan_discovered_event, scan_lost_event,
    scan_updated_event, stored_adapter_event, stored_gatt_value_event, stored_scan_event,
    stored_session_event,
};
pub(super) use crate::platform::device::{
    BluetoothAdapterDescriptor, BluetoothAdapterDescriptorValue, BluetoothAdapterEvent,
    BluetoothAdapterEventValue, BluetoothAdvertisementDataValue,
    BluetoothAdvertisementManufacturerDataValue, BluetoothAdvertisementServiceDataValue,
    BluetoothDataFilterValue, BluetoothDeviceDescriptor, BluetoothDeviceDescriptorValue,
    BluetoothGattCharacteristic, BluetoothGattCharacteristicProperties,
    BluetoothGattCharacteristicValue, BluetoothGattDescriptor, BluetoothGattDescriptorValue,
    BluetoothGattService, BluetoothGattServiceValue, BluetoothGattValueEvent,
    BluetoothGattWriteMode, BluetoothLeTransport, BluetoothManufacturerDataFilterValue,
    BluetoothPairState, BluetoothScanEvent, BluetoothScanFilter, BluetoothScanFilterValue,
    BluetoothServiceDataFilterValue, BluetoothSessionEvent,
};
pub(super) use crate::platform::resource::{ResourceEntry, ResourceFinalizer, ResourceKind};
pub(super) use crate::platform::{PlatformError, resource};
pub(super) use crate::runtime::BindingCallContext;

/// Initial Android Bluetooth row scratch capacity.
pub(super) const INITIAL_BLUETOOTH_ROW_CAPACITY: usize = 8;

/// Initial Android Bluetooth string scratch capacity.
pub(super) const INITIAL_BLUETOOTH_STRING_CAPACITY: usize = 512;

/// Maximum Android Bluetooth row scratch capacity.
pub(super) const MAX_BLUETOOTH_ROW_CAPACITY: usize = 4096;

/// Maximum Android Bluetooth string scratch capacity.
pub(super) const MAX_BLUETOOTH_STRING_CAPACITY: usize = 1024 * 1024;

/// Android flag for extended advertising support.
pub(super) const BLUETOOTH_ADAPTER_CAPABILITY_EXTENDED_ADVERTISING: u32 = 1 << 4;

/// Android flag for characteristic broadcast support.
pub(super) const BLUETOOTH_CHARACTERISTIC_PROPERTY_BROADCAST: u32 = 1 << 0;
/// Android flag for characteristic read support.
pub(super) const BLUETOOTH_CHARACTERISTIC_PROPERTY_READ: u32 = 1 << 1;
/// Android flag for write-without-response support.
pub(super) const BLUETOOTH_CHARACTERISTIC_PROPERTY_WRITE_WITHOUT_RESPONSE: u32 = 1 << 2;
/// Android flag for write-with-response support.
pub(super) const BLUETOOTH_CHARACTERISTIC_PROPERTY_WRITE: u32 = 1 << 3;
/// Android flag for notify support.
pub(super) const BLUETOOTH_CHARACTERISTIC_PROPERTY_NOTIFY: u32 = 1 << 4;
/// Android flag for indicate support.
pub(super) const BLUETOOTH_CHARACTERISTIC_PROPERTY_INDICATE: u32 = 1 << 5;
/// Android flag for authenticated signed writes.
pub(super) const BLUETOOTH_CHARACTERISTIC_PROPERTY_AUTH_SIGNED_WRITE: u32 = 1 << 6;
/// Android flag for reliable write support.
pub(super) const BLUETOOTH_CHARACTERISTIC_PROPERTY_RELIABLE_WRITE: u32 = 1 << 7;
/// Android flag for writable auxiliaries.
pub(super) const BLUETOOTH_CHARACTERISTIC_PROPERTY_WRITABLE_AUXILIARIES: u32 = 1 << 8;

/// Android scan event code for discovered.
pub(super) const BLUETOOTH_SCAN_EVENT_DISCOVERED: u32 = 1;
/// Android scan event code for updated.
pub(super) const BLUETOOTH_SCAN_EVENT_UPDATED: u32 = 2;
/// Android scan event code for lost.
pub(super) const BLUETOOTH_SCAN_EVENT_LOST: u32 = 3;

/// Android session event code for disconnected.
pub(super) const BLUETOOTH_SESSION_EVENT_DISCONNECTED: u32 = 1;
/// Android session event code for pair-state changes.
pub(super) const BLUETOOTH_SESSION_EVENT_PAIR_STATE_CHANGED: u32 = 2;
/// Android session event code for GATT-database changes.
pub(super) const BLUETOOTH_SESSION_EVENT_GATT_DATABASE_CHANGED: u32 = 3;

/// One opened Android Bluetooth scan resource.
#[derive(Clone)]
pub(super) struct AndroidBluetoothScanResource {
    /// The host scan session identifier.
    pub(super) session_id: u64,
    /// The requested filter snapshot.
    pub(super) filter: Option<BluetoothScanFilterValue>,
    /// The event sequencing state.
    pub(super) event_state: Arc<Mutex<BluetoothEventState>>,
}

/// One cached Android Bluetooth GATT graph.
#[derive(Debug, Default, Clone)]
pub(super) struct AndroidBluetoothGattCache {
    /// Services keyed by service id.
    pub(super) services: BTreeMap<String, BluetoothGattServiceValue>,
    /// Characteristics keyed by characteristic id.
    pub(super) characteristics: BTreeMap<String, BluetoothGattCharacteristicValue>,
    /// Descriptors keyed by descriptor id.
    pub(super) descriptors: BTreeMap<String, BluetoothGattDescriptorValue>,
}

/// One opened Android Bluetooth device resource.
#[derive(Clone)]
pub(super) struct AndroidBluetoothDeviceResource {
    /// The host device session identifier.
    pub(super) session_id: u64,
    /// The latest known device descriptor snapshot.
    pub(super) descriptor: Arc<Mutex<BluetoothDeviceDescriptorValue>>,
    /// The event sequencing state.
    pub(super) event_state: Arc<Mutex<BluetoothEventState>>,
    /// The cached GATT graph.
    pub(super) cache: Arc<Mutex<AndroidBluetoothGattCache>>,
}

/// One opened Android Bluetooth subscription resource.
#[derive(Clone)]
pub(super) struct AndroidBluetoothSubscriptionResource {
    /// The host subscription identifier.
    pub(super) subscription_id: u64,
    /// The parent service identifier.
    pub(super) service_id: String,
    /// The parent characteristic identifier.
    pub(super) characteristic_id: String,
    /// The parent service UUID.
    pub(super) service_uuid: String,
    /// The parent characteristic UUID.
    pub(super) characteristic_uuid: String,
}

/// Finalizer for one Android Bluetooth scan.
pub(super) struct AndroidBluetoothScanFinalizer {
    /// The owning runtime identifier.
    pub(super) runtime_id: u64,
    /// The host scan session identifier.
    pub(super) session_id: u64,
}

/// Finalizer for one Android Bluetooth device.
pub(super) struct AndroidBluetoothDeviceFinalizer {
    /// The owning runtime identifier.
    pub(super) runtime_id: u64,
    /// The host device session identifier.
    pub(super) session_id: u64,
}

/// Finalizer for one Android Bluetooth subscription.
pub(super) struct AndroidBluetoothSubscriptionFinalizer {
    /// The owning runtime identifier.
    pub(super) runtime_id: u64,
    /// The host subscription identifier.
    pub(super) subscription_id: u64,
}

impl ResourceFinalizer for AndroidBluetoothScanFinalizer {
    /// Close one Android Bluetooth scan.
    fn finalize(self: Box<Self>, _resource_id: resource::ResourceId) {
        unsafe {
            destack_host_android_bluetooth_scan_close(self.runtime_id, self.session_id);
        }
    }
}

impl ResourceFinalizer for AndroidBluetoothDeviceFinalizer {
    /// Close one Android Bluetooth device.
    fn finalize(self: Box<Self>, _resource_id: resource::ResourceId) {
        unsafe {
            destack_host_android_bluetooth_close(self.runtime_id, self.session_id);
        }
    }
}

impl ResourceFinalizer for AndroidBluetoothSubscriptionFinalizer {
    /// Close one Android Bluetooth subscription.
    fn finalize(self: Box<Self>, _resource_id: resource::ResourceId) {
        unsafe {
            destack_host_android_bluetooth_gatt_unsubscribe(self.runtime_id, self.subscription_id);
        }
    }
}
