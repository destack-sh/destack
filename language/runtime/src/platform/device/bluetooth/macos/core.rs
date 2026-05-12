#![allow(unsafe_op_in_unsafe_fn)]

pub(super) use std::collections::{BTreeMap, HashMap};
pub(super) use std::sync::atomic::{AtomicBool, Ordering};
pub(super) use std::sync::{Arc, OnceLock};
pub(super) use std::time::Instant;

pub(super) use dispatch2::{DispatchQoS, DispatchQueue, DispatchRetained};
pub(super) use objc2::rc::Retained;
pub(super) use objc2::runtime::{AnyObject, ProtocolObject};
pub(super) use objc2::{AnyThread, DefinedClass, Message, define_class};
pub(super) use objc2_core_bluetooth::{
    CBAdvertisementDataIsConnectable, CBAdvertisementDataLocalNameKey,
    CBAdvertisementDataManufacturerDataKey, CBAdvertisementDataOverflowServiceUUIDsKey,
    CBAdvertisementDataServiceDataKey, CBAdvertisementDataServiceUUIDsKey,
    CBAdvertisementDataTxPowerLevelKey, CBCentralManager, CBCentralManagerDelegate,
    CBCentralManagerFeature, CBCentralManagerScanOptionAllowDuplicatesKey, CBCharacteristic,
    CBCharacteristicProperties, CBCharacteristicWriteType, CBDescriptor, CBManager,
    CBManagerAuthorization, CBManagerState, CBPeripheral, CBPeripheralDelegate, CBPeripheralState,
    CBService, CBUUID,
};
pub(super) use objc2_foundation::{
    NSArray, NSData, NSDictionary, NSError, NSNumber, NSObject, NSObjectProtocol, NSString, NSUUID,
};
pub(super) use parking_lot::{Condvar, Mutex};

pub(super) use crate::diagnostic::{RuntimeError, RuntimeResult};
pub(super) use crate::platform::abi::{NativeSlice, NativeStringRef};
pub(super) use crate::platform::core::{self as core_platform, BoundedQueue, NativeAbiCodec};
pub(super) use crate::platform::device::bluetooth::core::{
    BLUETOOTH_ADAPTER_WATCH_RESOURCE_LABEL, BLUETOOTH_DEVICE_RESOURCE_LABEL,
    BLUETOOTH_NOTIFICATION_QUEUE_CAPACITY, BLUETOOTH_SCAN_EVENT_QUEUE_CAPACITY,
    BLUETOOTH_SCAN_RESOURCE_LABEL, BLUETOOTH_SESSION_EVENT_QUEUE_CAPACITY,
    BLUETOOTH_SUBSCRIPTION_RESOURCE_LABEL, BluetoothEventState, adapter_attached_event,
    adapter_changed_event, bluetooth_payload, close_bluetooth_resource, disconnected_event,
    gatt_database_changed_event, gatt_value_event, matches_scan_filter, scan_discovered_event,
    scan_updated_event, stored_adapter_event, stored_gatt_value_event, stored_scan_event,
    stored_session_event,
};
pub(super) use crate::platform::device::{
    BluetoothAdapterDescriptor, BluetoothAdapterDescriptorValue, BluetoothAdapterEvent,
    BluetoothAdapterEventValue, BluetoothAdvertisementDataValue,
    BluetoothAdvertisementManufacturerDataValue, BluetoothAdvertisementServiceDataValue,
    BluetoothDeviceDescriptor, BluetoothDeviceDescriptorValue, BluetoothGattCharacteristic,
    BluetoothGattCharacteristicProperties, BluetoothGattCharacteristicValue,
    BluetoothGattDescriptor, BluetoothGattDescriptorValue, BluetoothGattService,
    BluetoothGattServiceValue, BluetoothGattValueEvent, BluetoothGattValueEventValue,
    BluetoothGattWriteMode, BluetoothLeTransport, BluetoothScanEvent, BluetoothScanEventValue,
    BluetoothScanFilter, BluetoothScanFilterValue, BluetoothSessionEvent,
    BluetoothSessionEventValue,
};
pub(super) use crate::platform::diagnostic::PlatformErrorCode;
pub(super) use crate::platform::resource::{ResourceEntry, ResourceFinalizer, ResourceKind};
pub(super) use crate::platform::{PlatformError, resource};
pub(super) use crate::runtime::BindingCallContext;

/// Synchronize one CoreBluetooth object on the shared Bluetooth queue.
#[derive(Debug)]
pub(super) struct BluetoothDispatchBound<T>(core_platform::DispatchBound<T>);

unsafe impl<T> Send for BluetoothDispatchBound<T> {}
unsafe impl<T> Sync for BluetoothDispatchBound<T> {}

impl<T: Message> Clone for BluetoothDispatchBound<T> {
    /// Clone the wrapped CoreBluetooth object on the shared bluetooth queue.
    fn clone(&self) -> Self {
        Self(self.0.clone_on(bluetooth_dispatch_queue()))
    }
}

impl<T> BluetoothDispatchBound<T> {
    /// Wrap one retained CoreBluetooth object for queue-confined access.
    pub(super) unsafe fn new(value: Retained<T>) -> Self {
        Self(unsafe { core_platform::DispatchBound::new(value) })
    }

    /// Access the wrapped object on the shared Bluetooth queue.
    pub(super) fn dispatch<F, R>(&self, callback: F) -> R
    where
        F: FnOnce(&T) -> R + Send,
        R: Send,
    {
        self.0.dispatch_on(bluetooth_dispatch_queue(), callback)
    }

    /// Borrow the wrapped object while already executing on the Bluetooth queue.
    pub(super) unsafe fn get_unchecked(&self) -> &T {
        unsafe { self.0.get_unchecked() }
    }
}

/// One synthetic CoreBluetooth adapter identifier.
pub(super) const MACOS_BLUETOOTH_ADAPTER_ID: &str = "corebluetooth.default";

/// One synthetic CoreBluetooth adapter display name.
pub(super) const MACOS_BLUETOOTH_ADAPTER_NAME: &str = "CoreBluetooth";

/// One default CoreBluetooth operation timeout.
pub(super) const BLUETOOTH_DEFAULT_TIMEOUT_NS: u64 = 5_000_000_000;

/// Return the shared serial dispatch queue for CoreBluetooth operations.
pub(super) fn bluetooth_dispatch_queue() -> &'static DispatchQueue {
    static DISPATCH_QUEUE: OnceLock<DispatchRetained<DispatchQueue>> = OnceLock::new();

    DISPATCH_QUEUE.get_or_init(|| {
        let utility = DispatchQueue::global_queue(
            dispatch2::GlobalQueueIdentifier::QualityOfService(DispatchQoS::Utility),
        );

        core_platform::apple_serial_dispatch_queue("DestackBluetooth", Some(&utility))
    })
}

/// Return one stable object identity key.
pub(super) fn object_identity<T>(value: &T) -> usize {
    value as *const T as usize
}

/// Convert one CBUUID into the public string form.
pub(super) fn uuid_string(value: &CBUUID) -> String {
    let value = unsafe { value.UUIDString() };

    core_platform::nsstring_to_string(&value).to_lowercase()
}

/// Decode one descriptor value into a byte payload.
pub(super) fn descriptor_value_bytes(value: &AnyObject) -> Vec<u8> {
    if let Some(value) = value.downcast_ref::<NSNumber>() {
        return value.as_u16().to_le_bytes().to_vec();
    }

    if let Some(value) = value.downcast_ref::<NSString>() {
        return value.to_string().into_bytes();
    }

    if let Some(value) = value.downcast_ref::<NSData>() {
        return value.to_vec();
    }

    Vec::new()
}

/// Return one runtime error for one CoreBluetooth operation.
pub(super) fn macos_bluetooth_error(
    operation: &'static str,
    action: &'static str,
    error: impl std::fmt::Display,
) -> Box<RuntimeError> {
    RuntimeError::from(PlatformError::io_with(
        None,
        None,
        None,
        Some(operation.to_string()),
        Some(action.to_string()),
        format!("{action} failed: {error}"),
    ))
    .boxed()
}
