#![allow(unsafe_op_in_unsafe_fn)]

pub(super) use std::collections::{BTreeMap, HashMap};
pub(super) use std::sync::Arc;
pub(super) use std::sync::atomic::{AtomicBool, Ordering};

pub(super) use parking_lot::Mutex;
pub(super) use windows::Devices::Bluetooth::Advertisement::{
    BluetoothLEAdvertisementReceivedEventArgs, BluetoothLEAdvertisementWatcher,
    BluetoothLEScanningMode,
};
pub(super) use windows::Devices::Bluetooth::GenericAttributeProfile::{
    GattCharacteristic, GattClientCharacteristicConfigurationDescriptorValue,
    GattCommunicationStatus, GattDescriptor, GattDeviceService, GattValueChangedEventArgs,
    GattWriteOption,
};
pub(super) use windows::Devices::Bluetooth::{
    BluetoothAdapter, BluetoothAddressType, BluetoothCacheMode, BluetoothConnectionStatus,
    BluetoothLEDevice,
};
pub(super) use windows::Devices::Enumeration::{
    DeviceInformation, DevicePairingResultStatus, DeviceUnpairingResultStatus,
};
pub(super) use windows::Devices::Radios::RadioState;
pub(super) use windows::Foundation::TypedEventHandler;
pub(super) use windows::Storage::Streams::{DataReader, DataWriter, IBuffer};
pub(super) use windows::core::{Error as WinError, Ref};

pub(super) use crate::platform::abi::{NativeSlice, NativeStringRef};
pub(super) use crate::platform::device::{
    BluetoothAdapterDescriptor, BluetoothDeviceDescriptor, BluetoothGattCharacteristic,
    BluetoothGattDescriptor, BluetoothGattService, BluetoothGattValueEvent, BluetoothGattWriteMode,
    BluetoothScanEvent, BluetoothScanFilter, BluetoothSessionEvent,
};
pub(super) use crate::platform::resource::{ResourceEntry, ResourceFinalizer, ResourceKind};
pub(super) use crate::platform::{NativeAbiCodec, PlatformError, resource};
pub(super) use crate::runtime::BindingCallContext;

pub(super) use crate::diagnostic::{RuntimeError, RuntimeResult as DiagnosticResult};
pub(super) use crate::platform::core::{self as core_platform, BoundedQueue};
pub(super) use crate::platform::device::bluetooth::core::{
    BLUETOOTH_ADAPTER_WATCH_RESOURCE_LABEL, BLUETOOTH_DEVICE_RESOURCE_LABEL,
    BLUETOOTH_NOTIFICATION_QUEUE_CAPACITY, BLUETOOTH_SCAN_EVENT_QUEUE_CAPACITY,
    BLUETOOTH_SCAN_RESOURCE_LABEL, BLUETOOTH_SESSION_EVENT_QUEUE_CAPACITY,
    BLUETOOTH_SUBSCRIPTION_RESOURCE_LABEL, BluetoothEventState, adapter_attached_event,
    adapter_changed_event, adapter_detached_event, bluetooth_payload, close_bluetooth_resource,
    disconnected_event, gatt_database_changed_event, gatt_value_event, matches_scan_filter,
    pair_state_changed_event, scan_discovered_event, scan_updated_event, stored_adapter_event,
    stored_gatt_value_event, stored_scan_event, stored_session_event,
};
pub(super) use crate::platform::device::{
    BluetoothAdapterDescriptorValue, BluetoothAdapterEvent, BluetoothAdapterEventValue,
    BluetoothAdvertisementDataValue, BluetoothAdvertisementManufacturerDataValue,
    BluetoothAdvertisementServiceDataValue, BluetoothDeviceDescriptorValue,
    BluetoothGattCharacteristicProperties, BluetoothGattCharacteristicValue,
    BluetoothGattDescriptorValue, BluetoothGattServiceValue, BluetoothGattValueEventValue,
    BluetoothLeTransport, BluetoothPairState, BluetoothScanEventValue, BluetoothScanFilterValue,
    BluetoothScanMode, BluetoothSessionEventValue,
};

/// One stable device identifier prefix.
pub(super) const WINDOWS_BLUETOOTH_DEVICE_ID_PREFIX: &str = "winrt-bluetoothle";

/// One stable service identifier prefix.
pub(super) const WINDOWS_BLUETOOTH_SERVICE_ID_PREFIX: &str = "winrt-gatt-service";

/// One stable characteristic identifier prefix.
pub(super) const WINDOWS_BLUETOOTH_CHARACTERISTIC_ID_PREFIX: &str = "winrt-gatt-characteristic";

/// One stable descriptor identifier prefix.
pub(super) const WINDOWS_BLUETOOTH_DESCRIPTOR_ID_PREFIX: &str = "winrt-gatt-descriptor";

/// One opened Windows bluetooth scan resource.
pub(super) struct WindowsBluetoothScanResource {
    /// The queued scan events.
    pub(super) event_queue: Arc<BoundedQueue<BluetoothScanEventValue>>,
}

/// One opened Windows bluetooth device resource.
pub(super) struct WindowsBluetoothDeviceResource {
    /// The opened LE device object.
    pub(super) device: BluetoothLEDevice,
    /// The connection-status callback token.
    pub(super) connection_status_token: i64,
    /// The GATT-services-changed callback token.
    pub(super) gatt_services_changed_token: i64,
    /// The queued session events.
    pub(super) event_queue: Arc<BoundedQueue<BluetoothSessionEventValue>>,
    /// The per-stream sequence state.
    pub(super) event_state: Arc<Mutex<BluetoothEventState>>,
    /// The cached GATT graph.
    pub(super) cache: Arc<Mutex<WindowsBluetoothGattCache>>,
    /// Whether the cached GATT graph needs a refresh.
    pub(super) is_cache_dirty: Arc<AtomicBool>,
}

/// One opened Windows bluetooth subscription resource.
pub(super) struct WindowsBluetoothSubscriptionResource {
    /// The queued value events.
    pub(super) queue: Arc<BoundedQueue<BluetoothGattValueEventValue>>,
}

/// Finalizer for one Windows bluetooth scan.
pub(super) struct WindowsBluetoothScanFinalizer {
    /// The active watcher.
    pub(super) watcher: BluetoothLEAdvertisementWatcher,
    /// The received callback token.
    pub(super) received_token: i64,
    /// The stopped callback token.
    pub(super) stopped_token: i64,
    /// The shared event queue.
    pub(super) event_queue: Arc<BoundedQueue<BluetoothScanEventValue>>,
}

/// Finalizer for one Windows bluetooth subscription.
pub(super) struct WindowsBluetoothSubscriptionFinalizer {
    /// The subscribed characteristic object.
    pub(super) characteristic: GattCharacteristic,
    /// The value-changed callback token.
    pub(super) value_changed_token: i64,
    /// The shared value queue.
    pub(super) queue: Arc<BoundedQueue<BluetoothGattValueEventValue>>,
}

/// One cached Windows GATT graph.
#[derive(Default)]
pub(super) struct WindowsBluetoothGattCache {
    /// Services keyed by stable service id.
    pub(super) services: BTreeMap<String, BluetoothGattServiceValue>,
    /// Characteristics keyed by stable characteristic id.
    pub(super) characteristics: BTreeMap<String, BluetoothGattCharacteristicValue>,
    /// Descriptors keyed by stable descriptor id.
    pub(super) descriptors: BTreeMap<String, BluetoothGattDescriptorValue>,
    /// Live service objects keyed by stable service id.
    pub(super) service_objects: BTreeMap<String, GattDeviceService>,
    /// Live characteristic objects keyed by stable characteristic id.
    pub(super) characteristic_objects: BTreeMap<String, GattCharacteristic>,
    /// Live descriptor objects keyed by stable descriptor id.
    pub(super) descriptor_objects: BTreeMap<String, GattDescriptor>,
    /// Negotiated ATT MTU when known.
    pub(super) mtu: u16,
}

impl ResourceFinalizer for WindowsBluetoothScanFinalizer {
    /// Stop the advertisement watcher and wake blocked readers.
    fn finalize(self: Box<Self>, _resource_id: resource::ResourceId) {
        let _ = self.watcher.RemoveReceived(self.received_token);
        let _ = self.watcher.RemoveStopped(self.stopped_token);
        let _ = self.watcher.Stop();
        self.event_queue.close();
    }
}

impl ResourceFinalizer for WindowsBluetoothDeviceResource {
    /// Tear down one LE device session and wake blocked readers.
    fn finalize(self: Box<Self>, _resource_id: resource::ResourceId) {
        let _ = self
            .device
            .RemoveConnectionStatusChanged(self.connection_status_token);
        let _ = self
            .device
            .RemoveGattServicesChanged(self.gatt_services_changed_token);
        let _ = self.device.Close();
        self.event_queue.close();
    }
}

impl ResourceFinalizer for WindowsBluetoothSubscriptionFinalizer {
    /// Tear down one notification subscription and wake blocked readers.
    fn finalize(self: Box<Self>, _resource_id: resource::ResourceId) {
        let _ = self
            .characteristic
            .RemoveValueChanged(self.value_changed_token);
        let _ = self
            .characteristic
            .WriteClientCharacteristicConfigurationDescriptorAsync(
                GattClientCharacteristicConfigurationDescriptorValue::None,
            )
            .and_then(|result| result.get());
        self.queue.close();
    }
}
