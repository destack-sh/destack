#![allow(unsafe_op_in_unsafe_fn)]

pub(super) use std::collections::{BTreeMap, HashMap};
pub(super) use std::sync::Arc;
pub(super) use std::time::Duration;

pub(super) use parking_lot::Mutex;
pub(super) use zbus::MatchRule;
pub(super) use zbus::blocking::Connection;
pub(super) use zbus::blocking::fdo::{DBusProxy, ObjectManagerProxy};
pub(super) use zbus::message::Type as MessageType;
pub(super) use zbus::names::OwnedInterfaceName;
pub(super) use zbus::zvariant::{OwnedObjectPath, OwnedValue, Str};

pub(super) use crate::diagnostic::{RuntimeError, RuntimeResult};
pub(super) use crate::platform::abi::{NativeSlice, NativeStringRef};
pub(super) use crate::platform::core::{self as core_platform, BoundedQueue, NativeAbiCodec};
pub(super) use crate::platform::device::bluetooth::core::{
    BLUETOOTH_ADAPTER_WATCH_RESOURCE_LABEL, BLUETOOTH_DEVICE_RESOURCE_LABEL,
    BLUETOOTH_NOTIFICATION_QUEUE_CAPACITY, BLUETOOTH_SCAN_EVENT_QUEUE_CAPACITY,
    BLUETOOTH_SCAN_RESOURCE_LABEL, BLUETOOTH_SESSION_EVENT_QUEUE_CAPACITY,
    BLUETOOTH_SUBSCRIPTION_RESOURCE_LABEL, BluetoothEventState, adapter_attached_event,
    adapter_changed_event, adapter_detached_event, bluetooth_payload, close_bluetooth_resource,
    disconnected_event, gatt_database_changed_event, gatt_value_event, matches_scan_filter,
    pair_state_changed_event, scan_discovered_event, scan_lost_event, scan_updated_event,
    stored_adapter_event, stored_gatt_value_event, stored_scan_event, stored_session_event,
};
pub(super) use crate::platform::device::{
    BluetoothAdapterDescriptor, BluetoothAdapterDescriptorValue, BluetoothAdapterEvent,
    BluetoothAdapterEventValue, BluetoothAdvertisementDataValue,
    BluetoothAdvertisementManufacturerDataValue, BluetoothAdvertisementServiceDataValue,
    BluetoothDeviceDescriptor, BluetoothDeviceDescriptorValue, BluetoothGattCharacteristic,
    BluetoothGattCharacteristicProperties, BluetoothGattCharacteristicValue,
    BluetoothGattDescriptor, BluetoothGattDescriptorValue, BluetoothGattService,
    BluetoothGattServiceValue, BluetoothGattValueEvent, BluetoothGattValueEventValue,
    BluetoothGattWriteMode, BluetoothLeTransport, BluetoothPairState, BluetoothScanEvent,
    BluetoothScanEventValue, BluetoothScanFilter, BluetoothScanFilterValue, BluetoothSessionEvent,
    BluetoothSessionEventValue,
};
pub(super) use crate::platform::diagnostic::PlatformErrorCode;
pub(super) use crate::platform::resource::{ResourceEntry, ResourceFinalizer, ResourceKind};
pub(super) use crate::platform::{PlatformError, resource};
pub(super) use crate::runtime::BindingCallContext;

use super::service::LinuxBluetoothService;

/// BlueZ service name.
pub(super) const BLUEZ_SERVICE: &str = "org.bluez";

/// BlueZ root object path.
pub(super) const BLUEZ_ROOT_PATH: &str = "/";

/// BlueZ adapter interface name.
pub(super) const BLUEZ_ADAPTER_INTERFACE: &str = "org.bluez.Adapter1";

/// BlueZ device interface name.
pub(super) const BLUEZ_DEVICE_INTERFACE: &str = "org.bluez.Device1";

/// BlueZ GATT service interface name.
pub(super) const BLUEZ_GATT_SERVICE_INTERFACE: &str = "org.bluez.GattService1";

/// BlueZ GATT characteristic interface name.
pub(super) const BLUEZ_GATT_CHARACTERISTIC_INTERFACE: &str = "org.bluez.GattCharacteristic1";

/// BlueZ GATT descriptor interface name.
pub(super) const BLUEZ_GATT_DESCRIPTOR_INTERFACE: &str = "org.bluez.GattDescriptor1";

/// One scan device queue capacity.
pub(super) const BLUETOOTH_SCAN_DEVICE_QUEUE_CAPACITY: usize = 256;

/// One adapter-watch event queue capacity.
pub(super) const BLUETOOTH_ADAPTER_EVENT_QUEUE_CAPACITY: usize = 64;

/// One opened Linux Bluetooth scan resource.
pub(super) struct LinuxBluetoothScanResource {
    /// The adapter path used for this scan.
    pub(super) adapter_path: String,
    /// The active filter snapshot.
    pub(super) filter: Option<BluetoothScanFilterValue>,
    /// The queued device snapshots.
    pub(super) device_queue: Arc<BoundedQueue<BluetoothDeviceDescriptorValue>>,
    /// The queued scan events.
    pub(super) event_queue: Arc<BoundedQueue<BluetoothScanEventValue>>,
    /// The per-stream sequence state.
    pub(super) event_state: Mutex<BluetoothEventState>,
}

/// One opened Linux Bluetooth adapter-watch resource.
pub(super) struct LinuxBluetoothAdapterWatchResource {
    /// The queued adapter events.
    pub(super) event_queue: Arc<BoundedQueue<BluetoothAdapterEventValue>>,
    /// The per-stream sequence state.
    pub(super) event_state: Mutex<BluetoothEventState>,
}

/// One opened Linux Bluetooth device resource.
pub(super) struct LinuxBluetoothDeviceResource {
    /// Shared BlueZ transport service.
    pub(super) service: Arc<LinuxBluetoothService>,
    /// The stable device path.
    pub(super) device_path: String,
    /// The queued session events.
    pub(super) event_queue: Arc<BoundedQueue<BluetoothSessionEventValue>>,
    /// The per-stream sequence state.
    pub(super) event_state: Mutex<BluetoothEventState>,
    /// The cached GATT graph for this device.
    pub(super) cache: Mutex<LinuxBluetoothGattCache>,
}

/// One opened Linux Bluetooth notification subscription resource.
pub(super) struct LinuxBluetoothSubscriptionResource {
    /// The stable service identifier.
    pub(super) service_id: String,
    /// The stable characteristic identifier.
    pub(super) characteristic_id: String,
    /// The characteristic UUID.
    pub(super) characteristic_uuid: String,
    /// The service UUID.
    pub(super) service_uuid: String,
    /// The subscribed characteristic path.
    pub(super) characteristic_path: String,
    /// The queued value events.
    pub(super) queue: Arc<BoundedQueue<BluetoothGattValueEventValue>>,
}

/// One cached BlueZ GATT graph.
#[derive(Debug, Default, Clone, PartialEq)]
pub(super) struct LinuxBluetoothGattCache {
    /// Services keyed by stable service identifier.
    pub(super) services: BTreeMap<String, BluetoothGattServiceValue>,
    /// Characteristics keyed by stable characteristic identifier.
    pub(super) characteristics: BTreeMap<String, BluetoothGattCharacteristicValue>,
    /// Descriptors keyed by stable descriptor identifier.
    pub(super) descriptors: BTreeMap<String, BluetoothGattDescriptorValue>,
}

/// Finalizer for one scan resource.
pub(super) struct LinuxBluetoothScanFinalizer {
    /// Shared BlueZ transport service.
    pub(super) service: Arc<LinuxBluetoothService>,
    /// Signal registration identifier for this scan.
    pub(super) registration_id: u64,
    /// The adapter path used by the scan.
    pub(super) adapter_path: String,
    /// The shared device queue.
    pub(super) device_queue: Arc<BoundedQueue<BluetoothDeviceDescriptorValue>>,
    /// The shared event queue.
    pub(super) event_queue: Arc<BoundedQueue<BluetoothScanEventValue>>,
}

/// Finalizer for one adapter-watch resource.
pub(super) struct LinuxBluetoothAdapterWatchFinalizer {
    /// Shared BlueZ transport service.
    pub(super) service: Arc<LinuxBluetoothService>,
    /// Signal registration identifier for this adapter watch.
    pub(super) registration_id: u64,
    /// The shared event queue.
    pub(super) event_queue: Arc<BoundedQueue<BluetoothAdapterEventValue>>,
}

/// Finalizer for one device resource.
pub(super) struct LinuxBluetoothDeviceFinalizer {
    /// Shared BlueZ transport service.
    pub(super) service: Arc<LinuxBluetoothService>,
    /// Signal registration identifier for this device session.
    pub(super) registration_id: u64,
    /// The device path to disconnect.
    pub(super) device_path: String,
    /// The shared event queue.
    pub(super) event_queue: Arc<BoundedQueue<BluetoothSessionEventValue>>,
}

/// Finalizer for one subscription resource.
pub(super) struct LinuxBluetoothSubscriptionFinalizer {
    /// Shared BlueZ transport service.
    pub(super) service: Arc<LinuxBluetoothService>,
    /// Signal registration identifier for this subscription.
    pub(super) registration_id: u64,
    /// The characteristic path to stop notifying.
    pub(super) characteristic_path: String,
    /// The shared value-event queue.
    pub(super) queue: Arc<BoundedQueue<BluetoothGattValueEventValue>>,
}

impl ResourceFinalizer for LinuxBluetoothScanFinalizer {
    /// Stop discovery and wake blocked scan consumers.
    fn finalize(self: Box<Self>, _resource_id: resource::ResourceId) {
        self.service.unregister_scan(self.registration_id);
        self.device_queue.close();
        self.event_queue.close();
        self.service.release_adapter_scan(&self.adapter_path);
    }
}

impl ResourceFinalizer for LinuxBluetoothAdapterWatchFinalizer {
    /// Unregister the adapter watch and wake blocked consumers.
    fn finalize(self: Box<Self>, _resource_id: resource::ResourceId) {
        self.service.unregister_adapter_watch(self.registration_id);
        self.event_queue.close();
    }
}

impl ResourceFinalizer for LinuxBluetoothDeviceFinalizer {
    /// Disconnect the device and wake blocked session consumers.
    fn finalize(self: Box<Self>, _resource_id: resource::ResourceId) {
        self.service.unregister_device(self.registration_id);
        self.event_queue.close();
        self.service.release_device_connection(&self.device_path);
    }
}

impl ResourceFinalizer for LinuxBluetoothSubscriptionFinalizer {
    /// Stop notifications and wake blocked value consumers.
    fn finalize(self: Box<Self>, _resource_id: resource::ResourceId) {
        self.service.unregister_subscription(self.registration_id);
        self.queue.close();
        self.service
            .release_characteristic_notify(&self.characteristic_path);
    }
}
