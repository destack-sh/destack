use super::core::*;
use super::metadata::*;

/// One scan-delegate ivar set.
pub(super) struct MacBluetoothScanDelegateState {
    /// The shared manager-state queue.
    state_queue: Arc<BoundedQueue<CBManagerState>>,
    /// The optional scan filter.
    filter: Option<BluetoothScanFilterValue>,
    /// The shared scan event queue.
    event_queue: Arc<BoundedQueue<BluetoothScanEventValue>>,
    /// The shared scan event state.
    event_state: Arc<Mutex<BluetoothEventState>>,
    /// Known devices by stable identifier.
    known_devices: Arc<Mutex<HashMap<String, BluetoothDeviceDescriptorValue>>>,
}

define_class!(
    #[unsafe(super(NSObject))]
    #[ivars = MacBluetoothScanDelegateState]
    #[name = "DestackBluetoothScanDelegate"]
    pub(super) struct MacBluetoothScanDelegate;

    unsafe impl NSObjectProtocol for MacBluetoothScanDelegate {}

    unsafe impl CBCentralManagerDelegate for MacBluetoothScanDelegate {
        #[unsafe(method(centralManagerDidUpdateState:))]
        fn central_manager_did_update_state(&self, central: &CBCentralManager) {
            self.ivars()
                .state_queue
                .push_drop_oldest(unsafe { central.state() });
        }

        #[unsafe(method(centralManager:didDiscoverPeripheral:advertisementData:RSSI:))]
        fn central_manager_did_discover_peripheral_advertisement_data_rssi(
            &self,
            _central: &CBCentralManager,
            peripheral: &CBPeripheral,
            advertisement_data: &NSDictionary<NSString, AnyObject>,
            rssi: &NSNumber,
        ) {
            let state = self.ivars();
            let descriptor = device_descriptor(peripheral, advertisement_data, rssi.intValue());
            if let Some(filter) = state.filter.as_ref()
                && !matches_scan_filter(&descriptor, filter)
            {
                return;
            }

            let mut known_devices = state.known_devices.lock();
            let event = if known_devices.contains_key(&descriptor.id) {
                scan_updated_event(&state.event_state, descriptor.clone())
            } else {
                scan_discovered_event(&state.event_state, descriptor.clone())
            };
            known_devices.insert(descriptor.id.clone(), descriptor.clone());
            drop(known_devices);

            state.event_queue.push_drop_oldest(event);
        }
    }
);

impl MacBluetoothScanDelegate {
    /// Create one scan delegate instance.
    pub(super) fn new(
        state_queue: Arc<BoundedQueue<CBManagerState>>,
        filter: Option<BluetoothScanFilterValue>,
        event_queue: Arc<BoundedQueue<BluetoothScanEventValue>>,
        event_state: Arc<Mutex<BluetoothEventState>>,
    ) -> Retained<Self> {
        let value = Self::alloc().set_ivars(MacBluetoothScanDelegateState {
            state_queue,
            filter,
            event_queue,
            event_state,
            known_devices: Arc::new(Mutex::new(HashMap::new())),
        });

        unsafe { objc2::msg_send![super(value), init] }
    }

    /// Return the delegate as one protocol object.
    pub(super) fn as_protocol(&self) -> &ProtocolObject<dyn CBCentralManagerDelegate> {
        ProtocolObject::from_ref(self)
    }
}

/// One central-event payload.
#[derive(Debug)]
pub(super) enum MacBluetoothCentralEvent {
    /// One manager state change.
    StateChanged(CBManagerState),
    /// One successful connection.
    Connected { peripheral_id: String },
    /// One failed connection.
    ConnectFailed {
        peripheral_id: String,
        error: Option<String>,
    },
}

/// One device-central delegate ivar set.
pub(super) struct MacBluetoothCentralDelegateState {
    /// The central event queue.
    queue: Arc<BoundedQueue<MacBluetoothCentralEvent>>,
    /// The session event queue when the device is open.
    session_event_queue: Arc<BoundedQueue<BluetoothSessionEventValue>>,
    /// The session event state.
    session_event_state: Arc<Mutex<BluetoothEventState>>,
}

define_class!(
    #[unsafe(super(NSObject))]
    #[ivars = MacBluetoothCentralDelegateState]
    #[name = "DestackBluetoothCentralDelegate"]
    pub(super) struct MacBluetoothCentralDelegate;

    unsafe impl NSObjectProtocol for MacBluetoothCentralDelegate {}

    unsafe impl CBCentralManagerDelegate for MacBluetoothCentralDelegate {
        #[unsafe(method(centralManagerDidUpdateState:))]
        fn central_manager_did_update_state(&self, central: &CBCentralManager) {
            self.ivars()
                .queue
                .push_drop_oldest(MacBluetoothCentralEvent::StateChanged(unsafe {
                    central.state()
                }));
        }

        #[unsafe(method(centralManager:didConnectPeripheral:))]
        fn central_manager_did_connect_peripheral(
            &self,
            _central: &CBCentralManager,
            peripheral: &CBPeripheral,
        ) {
            let peripheral_id = unsafe { peripheral.identifier() };
            let peripheral_id = core_platform::nsstring_to_string(&peripheral_id.UUIDString());

            self.ivars()
                .queue
                .push_drop_oldest(MacBluetoothCentralEvent::Connected { peripheral_id });
        }

        #[unsafe(method(centralManager:didFailToConnectPeripheral:error:))]
        fn central_manager_did_fail_to_connect_peripheral_error(
            &self,
            _central: &CBCentralManager,
            peripheral: &CBPeripheral,
            error: Option<&NSError>,
        ) {
            let peripheral_id = unsafe { peripheral.identifier() };
            let peripheral_id = core_platform::nsstring_to_string(&peripheral_id.UUIDString());

            self.ivars()
                .queue
                .push_drop_oldest(MacBluetoothCentralEvent::ConnectFailed {
                    peripheral_id,
                    error: error.map(ToString::to_string),
                });
        }

        #[unsafe(method(centralManager:didDisconnectPeripheral:error:))]
        fn central_manager_did_disconnect_peripheral_error(
            &self,
            _central: &CBCentralManager,
            _peripheral: &CBPeripheral,
            _error: Option<&NSError>,
        ) {
            let state = self.ivars();
            state
                .session_event_queue
                .push_drop_oldest(disconnected_event(&state.session_event_state));
        }
    }
);

impl MacBluetoothCentralDelegate {
    /// Create one device-central delegate.
    pub(super) fn new(
        queue: Arc<BoundedQueue<MacBluetoothCentralEvent>>,
        session_event_queue: Arc<BoundedQueue<BluetoothSessionEventValue>>,
        session_event_state: Arc<Mutex<BluetoothEventState>>,
    ) -> Retained<Self> {
        let value = Self::alloc().set_ivars(MacBluetoothCentralDelegateState {
            queue,
            session_event_queue,
            session_event_state,
        });

        unsafe { objc2::msg_send![super(value), init] }
    }

    /// Return the delegate as one protocol object.
    pub(super) fn as_protocol(&self) -> &ProtocolObject<dyn CBCentralManagerDelegate> {
        ProtocolObject::from_ref(self)
    }
}

/// One peripheral delegate ivar set.
pub(super) struct MacBluetoothPeripheralDelegateState {
    /// The pending operation slot.
    pending: Arc<MacBluetoothPendingSlot>,
    /// The session event queue.
    session_event_queue: Arc<BoundedQueue<BluetoothSessionEventValue>>,
    /// The session event state.
    session_event_state: Arc<Mutex<BluetoothEventState>>,
    /// The subscription map.
    subscriptions: Arc<Mutex<HashMap<usize, MacBluetoothSubscriptionState>>>,
    /// The cache invalidation flag.
    cache_stale: Arc<AtomicBool>,
}

define_class!(
    #[unsafe(super(NSObject))]
    #[ivars = MacBluetoothPeripheralDelegateState]
    #[name = "DestackBluetoothPeripheralDelegate"]
    pub(super) struct MacBluetoothPeripheralDelegate;

    unsafe impl NSObjectProtocol for MacBluetoothPeripheralDelegate {}

    unsafe impl CBPeripheralDelegate for MacBluetoothPeripheralDelegate {
        #[unsafe(method(peripheral:didModifyServices:))]
        fn peripheral_did_modify_services(
            &self,
            _peripheral: &CBPeripheral,
            _invalidated_services: &NSArray<CBService>,
        ) {
            let state = self.ivars();

            state.cache_stale.store(true, Ordering::Relaxed);
            state
                .session_event_queue
                .push_drop_oldest(gatt_database_changed_event(&state.session_event_state));
        }

        #[unsafe(method(peripheral:didReadRSSI:error:))]
        fn peripheral_did_read_rssi_error(
            &self,
            _peripheral: &CBPeripheral,
            rssi: &NSNumber,
            error: Option<&NSError>,
        ) {
            let result = match error {
                Some(error) => Err(macos_bluetooth_nserror(
                    "destack.device.bluetooth.session.rssi",
                    "peripheral:didReadRSSI:error:",
                    error,
                )),
                None => Ok(MacBluetoothPendingValue::Rssi(rssi.intValue())),
            };

            let _ = self.ivars().pending.complete_matching(
                |operation| matches!(operation, MacBluetoothPendingTask::ReadRssi),
                result,
            );
        }

        #[unsafe(method(peripheral:didDiscoverServices:))]
        fn peripheral_did_discover_services(
            &self,
            _peripheral: &CBPeripheral,
            error: Option<&NSError>,
        ) {
            let result = match error {
                Some(error) => Err(macos_bluetooth_nserror(
                    "destack.device.bluetooth.session.open",
                    "peripheral:didDiscoverServices:",
                    error,
                )),
                None => Ok(MacBluetoothPendingValue::Done),
            };

            let _ = self.ivars().pending.complete_matching(
                |operation| matches!(operation, MacBluetoothPendingTask::DiscoverServices),
                result,
            );
        }

        #[unsafe(method(peripheral:didDiscoverCharacteristicsForService:error:))]
        fn peripheral_did_discover_characteristics_for_service_error(
            &self,
            _peripheral: &CBPeripheral,
            service: &CBService,
            error: Option<&NSError>,
        ) {
            let service_object_id = object_identity(service);
            let result = match error {
                Some(error) => Err(macos_bluetooth_nserror(
                    "destack.device.bluetooth.gatt.characteristicList",
                    "peripheral:didDiscoverCharacteristicsForService:error:",
                    error,
                )),
                None => Ok(MacBluetoothPendingValue::Done),
            };

            let _ = self.ivars().pending.complete_matching(
                |operation| {
                    matches!(
                        operation,
                        MacBluetoothPendingTask::DiscoverCharacteristics {
                            service_object_id: expected,
                        } if *expected == service_object_id
                    )
                },
                result,
            );
        }

        #[unsafe(method(peripheral:didDiscoverDescriptorsForCharacteristic:error:))]
        fn peripheral_did_discover_descriptors_for_characteristic_error(
            &self,
            _peripheral: &CBPeripheral,
            characteristic: &CBCharacteristic,
            error: Option<&NSError>,
        ) {
            let characteristic_object_id = object_identity(characteristic);
            let result = match error {
                Some(error) => Err(macos_bluetooth_nserror(
                    "destack.device.bluetooth.gatt.descriptorList",
                    "peripheral:didDiscoverDescriptorsForCharacteristic:error:",
                    error,
                )),
                None => Ok(MacBluetoothPendingValue::Done),
            };

            let _ = self.ivars().pending.complete_matching(
                |operation| {
                    matches!(
                        operation,
                        MacBluetoothPendingTask::DiscoverDescriptors {
                            characteristic_object_id: expected,
                        } if *expected == characteristic_object_id
                    )
                },
                result,
            );
        }

        #[unsafe(method(peripheral:didUpdateValueForCharacteristic:error:))]
        fn peripheral_did_update_value_for_characteristic_error(
            &self,
            _peripheral: &CBPeripheral,
            characteristic: &CBCharacteristic,
            error: Option<&NSError>,
        ) {
            let characteristic_object_id = object_identity(characteristic);
            let operation_result = match error {
                Some(error) => Err(macos_bluetooth_nserror(
                    "destack.device.bluetooth.gatt.read",
                    "peripheral:didUpdateValueForCharacteristic:error:",
                    error,
                )),
                None => Ok(MacBluetoothPendingValue::Bytes(
                    unsafe { characteristic.value() }
                        .map(|value| core_platform::nsdata_to_vec(&value))
                        .unwrap_or_default(),
                )),
            };

            if self.ivars().pending.complete_matching(
                |operation| {
                    matches!(
                        operation,
                        MacBluetoothPendingTask::ReadCharacteristic {
                            characteristic_object_id: expected,
                        } if *expected == characteristic_object_id
                    )
                },
                operation_result,
            ) {
                return;
            }

            let timestamp_ns = core_platform::monotonic_now_ns();
            let Some(subscription) = self
                .ivars()
                .subscriptions
                .lock()
                .get(&characteristic_object_id)
                .cloned()
            else {
                return;
            };
            let value = unsafe { characteristic.value() }
                .map(|value| core_platform::nsdata_to_vec(&value))
                .unwrap_or_default();

            subscription.queue.push_drop_oldest(gatt_value_event(
                timestamp_ns,
                subscription.service_id,
                subscription.characteristic_id,
                subscription.service_uuid,
                subscription.characteristic_uuid,
                value,
            ));
        }

        #[unsafe(method(peripheral:didWriteValueForCharacteristic:error:))]
        fn peripheral_did_write_value_for_characteristic_error(
            &self,
            _peripheral: &CBPeripheral,
            characteristic: &CBCharacteristic,
            error: Option<&NSError>,
        ) {
            let characteristic_object_id = object_identity(characteristic);
            let result = match error {
                Some(error) => Err(macos_bluetooth_nserror(
                    "destack.device.bluetooth.gatt.write",
                    "peripheral:didWriteValueForCharacteristic:error:",
                    error,
                )),
                None => Ok(MacBluetoothPendingValue::Done),
            };

            let _ = self.ivars().pending.complete_matching(
                |operation| {
                    matches!(
                        operation,
                        MacBluetoothPendingTask::WriteCharacteristic {
                            characteristic_object_id: expected,
                        } if *expected == characteristic_object_id
                    )
                },
                result,
            );
        }

        #[unsafe(method(peripheral:didUpdateNotificationStateForCharacteristic:error:))]
        fn peripheral_did_update_notification_state_for_characteristic_error(
            &self,
            _peripheral: &CBPeripheral,
            characteristic: &CBCharacteristic,
            error: Option<&NSError>,
        ) {
            let characteristic_object_id = object_identity(characteristic);
            let result = match error {
                Some(error) => Err(macos_bluetooth_nserror(
                    "destack.device.bluetooth.gatt.subscribe",
                    "peripheral:didUpdateNotificationStateForCharacteristic:error:",
                    error,
                )),
                None => Ok(MacBluetoothPendingValue::Done),
            };

            let _ = self.ivars().pending.complete_matching(
                |operation| {
                    matches!(
                        operation,
                        MacBluetoothPendingTask::UpdateNotification {
                            characteristic_object_id: expected,
                        } if *expected == characteristic_object_id
                    )
                },
                result,
            );
        }

        #[unsafe(method(peripheral:didUpdateValueForDescriptor:error:))]
        fn peripheral_did_update_value_for_descriptor_error(
            &self,
            _peripheral: &CBPeripheral,
            descriptor: &CBDescriptor,
            error: Option<&NSError>,
        ) {
            let descriptor_object_id = object_identity(descriptor);
            let result = match error {
                Some(error) => Err(macos_bluetooth_nserror(
                    "destack.device.bluetooth.gatt.readDescriptor",
                    "peripheral:didUpdateValueForDescriptor:error:",
                    error,
                )),
                None => Ok(MacBluetoothPendingValue::Bytes(
                    unsafe { descriptor.value() }
                        .map(|value| descriptor_value_bytes(&value))
                        .unwrap_or_default(),
                )),
            };

            let _ = self.ivars().pending.complete_matching(
                |operation| {
                    matches!(
                        operation,
                        MacBluetoothPendingTask::ReadDescriptor {
                            descriptor_object_id: expected,
                        } if *expected == descriptor_object_id
                    )
                },
                result,
            );
        }

        #[unsafe(method(peripheral:didWriteValueForDescriptor:error:))]
        fn peripheral_did_write_value_for_descriptor_error(
            &self,
            _peripheral: &CBPeripheral,
            descriptor: &CBDescriptor,
            error: Option<&NSError>,
        ) {
            let descriptor_object_id = object_identity(descriptor);
            let result = match error {
                Some(error) => Err(macos_bluetooth_nserror(
                    "destack.device.bluetooth.gatt.writeDescriptor",
                    "peripheral:didWriteValueForDescriptor:error:",
                    error,
                )),
                None => Ok(MacBluetoothPendingValue::Done),
            };

            let _ = self.ivars().pending.complete_matching(
                |operation| {
                    matches!(
                        operation,
                        MacBluetoothPendingTask::WriteDescriptor {
                            descriptor_object_id: expected,
                        } if *expected == descriptor_object_id
                    )
                },
                result,
            );
        }
    }
);

impl MacBluetoothPeripheralDelegate {
    /// Create one peripheral delegate instance.
    pub(super) fn new(
        pending: Arc<MacBluetoothPendingSlot>,
        session_event_queue: Arc<BoundedQueue<BluetoothSessionEventValue>>,
        session_event_state: Arc<Mutex<BluetoothEventState>>,
        subscriptions: Arc<Mutex<HashMap<usize, MacBluetoothSubscriptionState>>>,
        cache_stale: Arc<AtomicBool>,
    ) -> Retained<Self> {
        let value = Self::alloc().set_ivars(MacBluetoothPeripheralDelegateState {
            pending,
            session_event_queue,
            session_event_state,
            subscriptions,
            cache_stale,
        });

        unsafe { objc2::msg_send![super(value), init] }
    }

    /// Return the delegate as one protocol object.
    pub(super) fn as_protocol(&self) -> &ProtocolObject<dyn CBPeripheralDelegate> {
        ProtocolObject::from_ref(self)
    }
}
