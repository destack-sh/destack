use super::core::*;

/// Build one CoreBluetooth `NSError` runtime error.
pub(super) fn macos_bluetooth_nserror(
    operation: &'static str,
    action: &'static str,
    error: &NSError,
) -> Box<RuntimeError> {
    macos_bluetooth_error(operation, action, error)
}

/// Return one invalid Bluetooth identifier error.
pub(super) fn invalid_bluetooth_identifier(
    field: &'static str,
    message: &'static str,
) -> Box<RuntimeError> {
    core_platform::invalid_argument(field, message)
}

/// Validate one adapter identifier for the macOS backend.
pub(super) fn require_adapter_id(adapter_id: &str, operation: &'static str) -> RuntimeResult<()> {
    if adapter_id == MACOS_BLUETOOTH_ADAPTER_ID {
        return Ok(());
    }

    Err(core_platform::io_not_found(
        operation,
        "unknown bluetooth adapter",
    ))
}

/// Map one CoreBluetooth authorization status into one runtime result.
pub(super) fn require_bluetooth_authorization(operation: &'static str) -> RuntimeResult<()> {
    match unsafe { CBManager::authorization_class() } {
        CBManagerAuthorization::AllowedAlways => Ok(()),
        CBManagerAuthorization::Denied | CBManagerAuthorization::Restricted => {
            Err(core_platform::io_operation_error(
                operation,
                Some(PlatformErrorCode::IoPermissionDenied),
                "corebluetooth access is not authorized",
            ))
        }
        CBManagerAuthorization::NotDetermined => Err(core_platform::io_operation_error(
            operation,
            Some(PlatformErrorCode::IoPermissionDenied),
            "corebluetooth authorization has not been granted",
        )),
        other => Err(core_platform::io_operation_error(
            operation,
            None,
            format!("corebluetooth returned one unknown authorization state {other:?}"),
        )),
    }
}

/// Map one central-manager state into one runtime result.
pub(super) fn require_central_state(
    operation: &'static str,
    state: CBManagerState,
) -> RuntimeResult<()> {
    match state {
        CBManagerState::PoweredOn => Ok(()),
        CBManagerState::PoweredOff => Err(core_platform::io_operation_error(
            operation,
            None,
            "corebluetooth is powered off",
        )),
        CBManagerState::Unsupported => Err(core_platform::not_supported(operation)),
        CBManagerState::Unauthorized => Err(core_platform::io_operation_error(
            operation,
            Some(PlatformErrorCode::IoPermissionDenied),
            "corebluetooth is unauthorized",
        )),
        CBManagerState::Resetting => Err(core_platform::io_operation_error(
            operation,
            None,
            "corebluetooth is resetting",
        )),
        CBManagerState::Unknown => Err(core_platform::io_operation_error(
            operation,
            None,
            "corebluetooth state is unknown",
        )),
        other => Err(core_platform::io_operation_error(
            operation,
            None,
            format!("corebluetooth returned one unknown state {other:?}"),
        )),
    }
}

/// Build one adapter descriptor snapshot for the CoreBluetooth backend.
pub(super) fn adapter_descriptor(
    powered: bool,
    discovering: Option<bool>,
    extended_advertising: Option<bool>,
) -> BluetoothAdapterDescriptorValue {
    BluetoothAdapterDescriptorValue {
        id: MACOS_BLUETOOTH_ADAPTER_ID.to_string(),
        name: MACOS_BLUETOOTH_ADAPTER_NAME.to_string(),
        powered,
        discovering,
        extended_advertising,
    }
}

/// Decode one advertisement payload dictionary into the public binding value.
pub(super) fn advertisement_data(
    advertisement: &NSDictionary<NSString, AnyObject>,
) -> BluetoothAdvertisementDataValue {
    let local_name = advertisement
        .objectForKey(unsafe { CBAdvertisementDataLocalNameKey })
        .and_then(|value| value.downcast::<NSString>().ok())
        .map(|value| core_platform::nsstring_to_string(&value));
    let tx_power = advertisement
        .objectForKey(unsafe { CBAdvertisementDataTxPowerLevelKey })
        .and_then(|value| value.downcast::<NSNumber>().ok())
        .map(|value| value.shortValue());

    let service_uuids = advertisement
        .objectForKey(unsafe { CBAdvertisementDataServiceUUIDsKey })
        .into_iter()
        .chain(advertisement.objectForKey(unsafe { CBAdvertisementDataOverflowServiceUUIDsKey }))
        .flat_map(|value| value.downcast::<NSArray>())
        .flatten()
        .flat_map(|value| value.downcast::<CBUUID>())
        .map(|value| uuid_string(&value))
        .collect::<Vec<_>>();

    let manufacturer_data = advertisement
        .objectForKey(unsafe { CBAdvertisementDataManufacturerDataKey })
        .and_then(|value| value.downcast::<NSData>().ok())
        .and_then(|value| {
            let value = value.to_vec();
            if value.len() < 2 {
                return None;
            }

            let company_id = u16::from_le_bytes([value[0], value[1]]);
            let data = value[2..].to_vec();

            Some(vec![BluetoothAdvertisementManufacturerDataValue {
                company_id,
                data,
            }])
        })
        .unwrap_or_default();

    let service_data = advertisement
        .objectForKey(unsafe { CBAdvertisementDataServiceDataKey })
        .and_then(|value| value.downcast::<NSDictionary>().ok())
        .map(|value| {
            let mut items = Vec::new();

            for service_uuid in value.allKeys() {
                let Some(service_uuid) = service_uuid.downcast_ref::<CBUUID>() else {
                    continue;
                };
                let Some(bytes) = (unsafe { value.objectForKey_unchecked(service_uuid) }) else {
                    continue;
                };
                let Some(bytes) = bytes.downcast_ref::<NSData>() else {
                    continue;
                };

                items.push(BluetoothAdvertisementServiceDataValue {
                    service_uuid: uuid_string(service_uuid),
                    data: bytes.to_vec(),
                });
            }

            items
        })
        .unwrap_or_default();

    BluetoothAdvertisementDataValue {
        local_name,
        tx_power,
        service_uuids,
        manufacturer_data,
        service_data,
    }
}

/// Build one device descriptor from one CoreBluetooth discovery callback.
pub(super) fn device_descriptor(
    peripheral: &CBPeripheral,
    advertisement: &NSDictionary<NSString, AnyObject>,
    rssi: i32,
) -> BluetoothDeviceDescriptorValue {
    let identifier = unsafe { peripheral.identifier() };
    let name = unsafe { peripheral.name() }.map(|name| core_platform::nsstring_to_string(&name));
    let connectable = advertisement
        .objectForKey(unsafe { CBAdvertisementDataIsConnectable })
        .and_then(|value| value.downcast::<NSNumber>().ok())
        .map(|value| value.as_bool());
    let connected = unsafe { peripheral.state() } == CBPeripheralState::Connected;

    BluetoothDeviceDescriptorValue {
        id: core_platform::nsstring_to_string(&identifier.UUIDString()),
        address: None,
        name,
        rssi: Some(rssi),
        pair_state: None,
        connected,
        connectable,
        transport: Some(BluetoothLeTransport::LowEnergy),
        advertisement: advertisement_data(advertisement),
    }
}

/// Build one descriptor snapshot from one connected CoreBluetooth peripheral.
pub(super) fn current_device_descriptor(
    peripheral: &BluetoothDispatchBound<CBPeripheral>,
) -> BluetoothDeviceDescriptorValue {
    let id = peripheral.dispatch(|peripheral| unsafe {
        core_platform::nsstring_to_string(&peripheral.identifier().UUIDString())
    });
    let name = peripheral.dispatch(|peripheral| unsafe {
        peripheral
            .name()
            .map(|name| core_platform::nsstring_to_string(&name))
    });
    let connected = peripheral
        .dispatch(|peripheral| unsafe { peripheral.state() == CBPeripheralState::Connected });

    BluetoothDeviceDescriptorValue {
        id,
        address: None,
        name,
        rssi: None,
        pair_state: None,
        connected,
        connectable: None,
        transport: Some(BluetoothLeTransport::LowEnergy),
        advertisement: BluetoothAdvertisementDataValue {
            local_name: None,
            tx_power: None,
            service_uuids: Vec::new(),
            manufacturer_data: Vec::new(),
            service_data: Vec::new(),
        },
    }
}

/// Convert one CoreBluetooth characteristic property bitset.
pub(super) fn characteristic_properties(
    properties: CBCharacteristicProperties,
) -> BluetoothGattCharacteristicProperties {
    BluetoothGattCharacteristicProperties {
        broadcast: properties.contains(CBCharacteristicProperties::Broadcast),
        read: properties.contains(CBCharacteristicProperties::Read),
        write_without_response: properties
            .contains(CBCharacteristicProperties::WriteWithoutResponse),
        write: properties.contains(CBCharacteristicProperties::Write),
        notify: properties.contains(CBCharacteristicProperties::Notify),
        indicate: properties.contains(CBCharacteristicProperties::Indicate),
        authenticated_signed_writes: properties
            .contains(CBCharacteristicProperties::AuthenticatedSignedWrites),
        reliable_write: properties.contains(CBCharacteristicProperties::ExtendedProperties),
        writable_auxiliaries: false,
    }
}

/// Return one inferred ATT MTU from one peripheral snapshot.
pub(super) fn inferred_mtu(peripheral: &BluetoothDispatchBound<CBPeripheral>) -> u16 {
    let payload = peripheral.dispatch(|peripheral| unsafe {
        peripheral.maximumWriteValueLengthForType(CBCharacteristicWriteType::WithResponse)
    });

    payload.saturating_add(3).min(u16::MAX as usize) as u16
}

/// One cached service entry.
#[derive(Clone)]
pub(super) struct MacBluetoothServiceEntry {
    /// The public descriptor snapshot.
    pub(super) descriptor: BluetoothGattServiceValue,
    /// The stable object identity.
    pub(super) object_id: usize,
}

/// One cached characteristic entry.
#[derive(Clone)]
pub(super) struct MacBluetoothCharacteristicEntry {
    /// The public descriptor snapshot.
    pub(super) descriptor: BluetoothGattCharacteristicValue,
    /// The live characteristic object.
    pub(super) characteristic: BluetoothDispatchBound<CBCharacteristic>,
    /// The stable object identity.
    pub(super) object_id: usize,
}

/// One cached descriptor entry.
#[derive(Clone)]
pub(super) struct MacBluetoothDescriptorEntry {
    /// The public descriptor snapshot.
    pub(super) descriptor: BluetoothGattDescriptorValue,
    /// The live descriptor object.
    pub(super) descriptor_object: BluetoothDispatchBound<CBDescriptor>,
    /// The stable object identity.
    pub(super) object_id: usize,
}

/// One cached GATT graph.
#[derive(Clone, Default)]
pub(super) struct MacBluetoothGattCache {
    /// Services keyed by stable identifier.
    pub(super) services: BTreeMap<String, MacBluetoothServiceEntry>,
    /// Characteristics keyed by stable identifier.
    pub(super) characteristics: BTreeMap<String, MacBluetoothCharacteristicEntry>,
    /// Descriptors keyed by stable identifier.
    pub(super) descriptors: BTreeMap<String, MacBluetoothDescriptorEntry>,
}

/// One active notification subscription snapshot.
#[derive(Clone)]
pub(super) struct MacBluetoothSubscriptionState {
    /// The stable service identifier.
    pub(super) service_id: String,
    /// The stable characteristic identifier.
    pub(super) characteristic_id: String,
    /// The stable service UUID string.
    pub(super) service_uuid: String,
    /// The stable characteristic UUID string.
    pub(super) characteristic_uuid: String,
    /// The notification queue.
    pub(super) queue: Arc<BoundedQueue<BluetoothGattValueEventValue>>,
}

/// One pending GATT operation kind.
#[derive(Clone, Debug)]
pub(super) enum MacBluetoothPendingTask {
    /// One service discovery.
    DiscoverServices,
    /// One characteristic discovery.
    DiscoverCharacteristics { service_object_id: usize },
    /// One descriptor discovery.
    DiscoverDescriptors { characteristic_object_id: usize },
    /// One characteristic read.
    ReadCharacteristic { characteristic_object_id: usize },
    /// One characteristic write with response.
    WriteCharacteristic { characteristic_object_id: usize },
    /// One descriptor read.
    ReadDescriptor { descriptor_object_id: usize },
    /// One descriptor write.
    WriteDescriptor { descriptor_object_id: usize },
    /// One notification-state transition.
    UpdateNotification { characteristic_object_id: usize },
    /// One RSSI read.
    ReadRssi,
}

/// One pending GATT operation value.
#[derive(Clone, Debug)]
pub(super) enum MacBluetoothPendingValue {
    /// One unit result.
    Done,
    /// One byte payload.
    Bytes(Vec<u8>),
    /// One RSSI payload.
    Rssi(i32),
}

/// One pending GATT operation state.
#[derive(Default)]
struct MacBluetoothPendingState {
    /// The operation currently being awaited.
    operation: Option<MacBluetoothPendingTask>,
    /// The resolved result when one callback arrives.
    result: Option<RuntimeResult<MacBluetoothPendingValue>>,
}

/// Shared pending GATT operation storage.
pub(super) struct MacBluetoothPendingSlot {
    /// The mutable operation state.
    state: Mutex<MacBluetoothPendingState>,
    /// The condition variable used to wake blocked callers.
    condition: Condvar,
}

impl MacBluetoothPendingSlot {
    /// Create one empty pending-operation slot.
    pub(super) fn new() -> Self {
        Self {
            state: Mutex::new(MacBluetoothPendingState::default()),
            condition: Condvar::new(),
        }
    }

    /// Reset and install one new pending operation.
    pub(super) fn begin(&self, operation: MacBluetoothPendingTask) {
        let mut state = self.state.lock();
        state.operation = Some(operation);
        state.result = None;
    }

    /// Resolve one pending operation when it matches.
    pub(super) fn complete_matching(
        &self,
        predicate: impl FnOnce(&MacBluetoothPendingTask) -> bool,
        result: RuntimeResult<MacBluetoothPendingValue>,
    ) -> bool {
        let mut state = self.state.lock();
        let Some(operation) = state.operation.as_ref() else {
            return false;
        };
        if !predicate(operation) {
            return false;
        }

        state.result = Some(result);
        state.operation = None;
        self.condition.notify_all();

        true
    }

    /// Wait for the current operation result.
    pub(super) fn wait(
        &self,
        operation: &'static str,
        timeout_ns: u64,
    ) -> RuntimeResult<MacBluetoothPendingValue> {
        let mut state = self.state.lock();
        let deadline = core_platform::timeout_deadline(timeout_ns);

        loop {
            if let Some(result) = state.result.take() {
                return result;
            }

            if state.operation.is_none() {
                return Err(core_platform::io_operation_error(
                    operation,
                    Some(PlatformErrorCode::IoInvalidData),
                    "corebluetooth operation completed without one result",
                ));
            }

            let Some(deadline) = deadline else {
                state.operation = None;
                return Err(core_platform::io_would_block(
                    operation,
                    "corebluetooth operation timeout overflowed",
                ));
            };

            if Instant::now() >= deadline {
                state.operation = None;
                return Err(core_platform::io_would_block(
                    operation,
                    "corebluetooth operation timed out",
                ));
            }

            self.condition.wait_until(&mut state, deadline);
        }
    }

    /// Clear one abandoned pending operation.
    pub(super) fn clear(&self) {
        let mut state = self.state.lock();
        state.operation = None;
        state.result = None;
        self.condition.notify_all();
    }
}

/// One opened scan resource.
pub(super) struct MacBluetoothScanResource {
    /// The scan event queue.
    pub(super) event_queue: Arc<BoundedQueue<BluetoothScanEventValue>>,
}

/// One opened device resource.
pub(super) struct MacBluetoothDeviceResource {
    /// The live peripheral object.
    pub(super) peripheral: BluetoothDispatchBound<CBPeripheral>,
    /// The session event queue.
    pub(super) event_queue: Arc<BoundedQueue<BluetoothSessionEventValue>>,
    /// The cached GATT graph.
    pub(super) cache: Arc<Mutex<MacBluetoothGattCache>>,
    /// The cache invalidation flag.
    pub(super) cache_stale: Arc<AtomicBool>,
    /// The serialized operation lock.
    pub(super) operation_lock: Arc<Mutex<()>>,
    /// The pending operation slot.
    pub(super) pending: Arc<MacBluetoothPendingSlot>,
    /// Active subscriptions keyed by characteristic object identity.
    pub(super) subscriptions: Arc<Mutex<HashMap<usize, MacBluetoothSubscriptionState>>>,
}

/// One opened notification subscription resource.
pub(super) struct MacBluetoothSubscriptionResource {
    /// The notification queue.
    pub(super) queue: Arc<BoundedQueue<BluetoothGattValueEventValue>>,
}

/// Finalizer for one scan resource.
pub(super) struct MacBluetoothScanFinalizer {
    /// The live central manager.
    pub(super) central: BluetoothDispatchBound<CBCentralManager>,
    /// The event queue.
    pub(super) event_queue: Arc<BoundedQueue<BluetoothScanEventValue>>,
}

/// Finalizer for one device resource.
pub(super) struct MacBluetoothDeviceFinalizer {
    /// The live central manager.
    pub(super) central: BluetoothDispatchBound<CBCentralManager>,
    /// The live peripheral object.
    pub(super) peripheral: BluetoothDispatchBound<CBPeripheral>,
    /// The event queue.
    pub(super) event_queue: Arc<BoundedQueue<BluetoothSessionEventValue>>,
    /// The pending operation slot.
    pub(super) pending: Arc<MacBluetoothPendingSlot>,
}

/// Finalizer for one subscription resource.
pub(super) struct MacBluetoothSubscriptionFinalizer {
    /// The live peripheral object.
    pub(super) peripheral: BluetoothDispatchBound<CBPeripheral>,
    /// The subscribed characteristic object.
    pub(super) characteristic: BluetoothDispatchBound<CBCharacteristic>,
    /// The shared pending operation slot.
    pub(super) pending: Arc<MacBluetoothPendingSlot>,
    /// The shared operation lock.
    pub(super) operation_lock: Arc<Mutex<()>>,
    /// The shared subscription registry.
    pub(super) subscriptions: Arc<Mutex<HashMap<usize, MacBluetoothSubscriptionState>>>,
    /// The notification queue.
    pub(super) queue: Arc<BoundedQueue<BluetoothGattValueEventValue>>,
}

impl ResourceFinalizer for MacBluetoothScanFinalizer {
    /// Stop scanning and wake blocked scan consumers.
    fn finalize(self: Box<Self>, _resource_id: resource::ResourceId) {
        self.central.dispatch(|central| unsafe {
            central.stopScan();
            central.setDelegate(None);
        });

        self.event_queue.close();
    }
}

impl ResourceFinalizer for MacBluetoothDeviceFinalizer {
    /// Cancel the connection and wake blocked session consumers.
    fn finalize(self: Box<Self>, _resource_id: resource::ResourceId) {
        self.pending.clear();
        self.central.dispatch(|central| unsafe {
            central.cancelPeripheralConnection(self.peripheral.get_unchecked());
            central.setDelegate(None);
        });
        self.peripheral.dispatch(|peripheral| unsafe {
            peripheral.setDelegate(None);
        });

        self.event_queue.close();
    }
}

impl ResourceFinalizer for MacBluetoothSubscriptionFinalizer {
    /// Disable notifications and wake blocked subscription consumers.
    fn finalize(self: Box<Self>, _resource_id: resource::ResourceId) {
        self.pending.clear();

        let characteristic_object_id = self.characteristic.dispatch(object_identity);
        self.subscriptions.lock().remove(&characteristic_object_id);

        let operation_lock = self.operation_lock.lock();
        self.pending
            .begin(MacBluetoothPendingTask::UpdateNotification {
                characteristic_object_id,
            });
        self.peripheral.dispatch(|peripheral| unsafe {
            peripheral.setNotifyValue_forCharacteristic(false, self.characteristic.get_unchecked());
        });
        let _ = self.pending.wait(
            "destack.device.bluetooth.gatt.unsubscribe",
            BLUETOOTH_DEFAULT_TIMEOUT_NS,
        );
        drop(operation_lock);

        self.queue.close();
    }
}
