use std::collections::BTreeSet;

use super::core::*;
use super::delegates::*;
use super::metadata::*;

/// Build one central manager bound to the shared bluetooth queue.
pub(super) fn central_manager_with_delegate(
    delegate: &ProtocolObject<dyn CBCentralManagerDelegate>,
) -> BluetoothDispatchBound<CBCentralManager> {
    let manager = unsafe {
        CBCentralManager::initWithDelegate_queue(
            CBCentralManager::alloc(),
            Some(delegate),
            Some(bluetooth_dispatch_queue()),
        )
    };

    unsafe { BluetoothDispatchBound::new(manager) }
}

/// Wait for the central manager to reach one usable state.
pub(super) fn wait_for_central_ready(
    central: &BluetoothDispatchBound<CBCentralManager>,
    state_queue: &Arc<BoundedQueue<CBManagerState>>,
    operation: &'static str,
) -> RuntimeResult<()> {
    require_bluetooth_authorization(operation)?;

    let initial_state = central.dispatch(|central| unsafe { central.state() });
    if initial_state != CBManagerState::Unknown {
        return require_central_state(operation, initial_state);
    }

    let state = state_queue.pop_with_timeout_or_else(
        std::time::Duration::from_nanos(BLUETOOTH_DEFAULT_TIMEOUT_NS),
        || {
            Err(core_platform::io_would_block(
                operation,
                "corebluetooth manager did not become ready",
            ))
        },
    )?;

    require_central_state(operation, state)
}

/// Wait for the device-open central manager to reach one usable state.
pub(super) fn wait_for_device_central_ready(
    central: &BluetoothDispatchBound<CBCentralManager>,
    queue: &Arc<BoundedQueue<MacBluetoothCentralEvent>>,
    operation: &'static str,
) -> RuntimeResult<()> {
    require_bluetooth_authorization(operation)?;

    let initial_state = central.dispatch(|central| unsafe { central.state() });
    if initial_state != CBManagerState::Unknown {
        return require_central_state(operation, initial_state);
    }

    let deadline =
        core_platform::timeout_deadline(BLUETOOTH_DEFAULT_TIMEOUT_NS).ok_or_else(|| {
            core_platform::io_would_block(operation, "corebluetooth manager timeout overflowed")
        })?;

    loop {
        if Instant::now() >= deadline {
            return Err(core_platform::io_would_block(
                operation,
                "corebluetooth manager did not become ready",
            ));
        }

        let remaining = deadline.duration_since(Instant::now());
        let event = queue.pop_with_timeout_or_else(remaining, || {
            Err(core_platform::io_would_block(
                operation,
                "corebluetooth manager did not become ready",
            ))
        })?;

        if let MacBluetoothCentralEvent::StateChanged(state) = event {
            return require_central_state(operation, state);
        }
    }
}

/// Return whether the scan mode requests duplicate advertisements.
pub(super) fn scan_allows_duplicates(filter: Option<&BluetoothScanFilterValue>) -> bool {
    filter.is_some_and(|filter| filter.keep_repeated_devices)
}

/// Create one default adapter descriptor list.
pub(super) fn adapter_list(
    binding: &BindingCallContext,
    powered: bool,
) -> NativeSlice<BluetoothAdapterDescriptor> {
    let extended_advertising = Some(unsafe {
        CBCentralManager::supportsFeatures(CBCentralManagerFeature::ExtendedScanAndConnect)
    });
    let descriptors = vec![BluetoothAdapterDescriptor::from_value(
        binding,
        adapter_descriptor(powered, None, extended_advertising),
    )];

    binding.store_slice(descriptors)
}

/// Wait for one central connection outcome.
pub(super) fn wait_for_connection(
    queue: &Arc<BoundedQueue<MacBluetoothCentralEvent>>,
    device_id: &str,
    operation: &'static str,
) -> RuntimeResult<()> {
    let deadline = core_platform::timeout_deadline(BLUETOOTH_DEFAULT_TIMEOUT_NS)
        .ok_or_else(|| core_platform::io_would_block(operation, "connection timeout overflowed"))?;

    loop {
        if Instant::now() >= deadline {
            return Err(core_platform::io_would_block(
                operation,
                "corebluetooth connection timed out",
            ));
        }

        let remaining = deadline.duration_since(Instant::now());
        let event = queue.pop_with_timeout_or_else(remaining, || {
            Err(core_platform::io_would_block(
                operation,
                "corebluetooth connection timed out",
            ))
        })?;

        match event {
            MacBluetoothCentralEvent::StateChanged(state) => {
                require_central_state(operation, state)?;
            }
            MacBluetoothCentralEvent::Connected { peripheral_id } if peripheral_id == device_id => {
                return Ok(());
            }
            MacBluetoothCentralEvent::ConnectFailed {
                peripheral_id,
                error,
            } if peripheral_id == device_id => {
                return Err(core_platform::io_operation_error(
                    operation,
                    None,
                    format!(
                        "corebluetooth failed to connect device: {}",
                        error.unwrap_or_else(|| String::from("unknown error"))
                    ),
                ));
            }
            _ => {}
        }
    }
}

/// Run one pending peripheral operation and wait for the callback result.
pub(super) fn wait_for_pending_operation(
    pending: &Arc<MacBluetoothPendingSlot>,
    operation: &'static str,
    timeout_ns: u64,
) -> RuntimeResult<MacBluetoothPendingValue> {
    pending.wait(operation, timeout_ns)
}

/// Refresh one CoreBluetooth GATT cache.
pub(super) fn refresh_gatt_cache(
    peripheral: &BluetoothDispatchBound<CBPeripheral>,
    pending: &Arc<MacBluetoothPendingSlot>,
    operation_lock: &Arc<Mutex<()>>,
    previous_cache: Option<&MacBluetoothGattCache>,
) -> RuntimeResult<MacBluetoothGattCache> {
    let _operation_lock = operation_lock.lock();

    pending.begin(MacBluetoothPendingTask::DiscoverServices);
    peripheral.dispatch(|peripheral| unsafe {
        peripheral.discoverServices(None);
    });
    let _ = wait_for_pending_operation(
        pending,
        "destack.device.bluetooth.session.open",
        BLUETOOTH_DEFAULT_TIMEOUT_NS,
    )?;

    let services = peripheral.dispatch(|peripheral| unsafe {
        peripheral.services().map(|services| {
            services
                .iter()
                .map(|service| BluetoothDispatchBound::new(service))
                .collect::<Vec<_>>()
        })
    });
    let services = services.unwrap_or_default();
    let mut cache = MacBluetoothGattCache::default();
    let previous_id_index = previous_id_index(previous_cache);
    let mut used_service_ids = BTreeSet::new();

    for service in &services {
        let service_object_id = service.dispatch(object_identity);
        let service_uuid =
            service.dispatch(|service: &CBService| unsafe { uuid_string(&service.UUID()) });
        let service_id = reused_or_scoped_identifier(
            previous_id_index.service_id(service_object_id),
            &mut used_service_ids,
            |used_ids| stable_service_identifier(&service_uuid, used_ids),
        );
        let service_descriptor = BluetoothGattServiceValue {
            id: service_id.clone(),
            uuid: service_uuid.clone(),
            primary: service.dispatch(|service: &CBService| unsafe { service.isPrimary() }),
            included_service_ids: Vec::new(),
        };

        cache.services.insert(
            service_id.clone(),
            MacBluetoothServiceEntry {
                descriptor: service_descriptor.clone(),
                object_id: service_object_id,
            },
        );

        pending.begin(MacBluetoothPendingTask::DiscoverCharacteristics { service_object_id });
        peripheral.dispatch(|peripheral| unsafe {
            peripheral.discoverCharacteristics_forService(None, service.get_unchecked());
        });
        let _ = wait_for_pending_operation(
            pending,
            "destack.device.bluetooth.gatt.characteristicList",
            BLUETOOTH_DEFAULT_TIMEOUT_NS,
        )?;

        let characteristics = service.dispatch(|service: &CBService| unsafe {
            service.characteristics().map(|characteristics| {
                characteristics
                    .iter()
                    .map(|characteristic| BluetoothDispatchBound::new(characteristic))
                    .collect::<Vec<_>>()
            })
        });
        let characteristics = characteristics.unwrap_or_default();
        let mut used_characteristic_ids = BTreeSet::new();

        for characteristic in &characteristics {
            let characteristic_object_id =
                characteristic.dispatch(object_identity::<CBCharacteristic>);
            let characteristic_uuid =
                characteristic.dispatch(|characteristic: &CBCharacteristic| unsafe {
                    uuid_string(&characteristic.UUID())
                });
            let characteristic_id = reused_or_scoped_identifier(
                previous_id_index.characteristic_id(characteristic_object_id),
                &mut used_characteristic_ids,
                |used_ids| {
                    stable_characteristic_identifier(&service_id, &characteristic_uuid, used_ids)
                },
            );
            let characteristic_descriptor = BluetoothGattCharacteristicValue {
                id: characteristic_id.clone(),
                service_id: service_id.clone(),
                uuid: characteristic_uuid.clone(),
                properties: characteristic.dispatch(|characteristic: &CBCharacteristic| unsafe {
                    characteristic_properties(characteristic.properties())
                }),
            };

            cache.characteristics.insert(
                characteristic_id.clone(),
                MacBluetoothCharacteristicEntry {
                    descriptor: characteristic_descriptor.clone(),
                    characteristic: characteristic.clone(),
                    object_id: characteristic_object_id,
                },
            );

            pending.begin(MacBluetoothPendingTask::DiscoverDescriptors {
                characteristic_object_id,
            });
            peripheral.dispatch(|peripheral| unsafe {
                peripheral.discoverDescriptorsForCharacteristic(characteristic.get_unchecked());
            });
            let _ = wait_for_pending_operation(
                pending,
                "destack.device.bluetooth.gatt.descriptorList",
                BLUETOOTH_DEFAULT_TIMEOUT_NS,
            )?;

            let descriptors = characteristic.dispatch(|characteristic: &CBCharacteristic| unsafe {
                characteristic.descriptors().map(|descriptors| {
                    descriptors
                        .iter()
                        .map(|descriptor| BluetoothDispatchBound::new(descriptor))
                        .collect::<Vec<_>>()
                })
            });
            let descriptors = descriptors.unwrap_or_default();
            let mut used_descriptor_ids = BTreeSet::new();

            for descriptor in &descriptors {
                let descriptor_object_id = descriptor.dispatch(object_identity::<CBDescriptor>);
                let descriptor_uuid = descriptor.dispatch(|descriptor: &CBDescriptor| unsafe {
                    uuid_string(&descriptor.UUID())
                });
                let descriptor_id = reused_or_scoped_identifier(
                    previous_id_index.descriptor_id(descriptor_object_id),
                    &mut used_descriptor_ids,
                    |used_ids| {
                        stable_descriptor_identifier(&characteristic_id, &descriptor_uuid, used_ids)
                    },
                );
                let descriptor_descriptor = BluetoothGattDescriptorValue {
                    id: descriptor_id.clone(),
                    service_id: service_id.clone(),
                    characteristic_id: characteristic_id.clone(),
                    uuid: descriptor_uuid,
                };

                cache.descriptors.insert(
                    descriptor_id.clone(),
                    MacBluetoothDescriptorEntry {
                        descriptor: descriptor_descriptor,
                        descriptor_object: descriptor.clone(),
                        object_id: descriptor_object_id,
                    },
                );
            }
        }
    }

    Ok(cache)
}

/// Return one lookup index for previously assigned CoreBluetooth ids.
fn previous_id_index(previous_cache: Option<&MacBluetoothGattCache>) -> MacBluetoothIdIndex {
    let mut id_index = MacBluetoothIdIndex::default();

    let Some(previous_cache) = previous_cache else {
        return id_index;
    };

    for entry in previous_cache.services.values() {
        id_index
            .service_id_by_object
            .insert(entry.object_id, entry.descriptor.id.clone());
    }

    for entry in previous_cache.characteristics.values() {
        id_index
            .characteristic_id_by_object
            .insert(entry.object_id, entry.descriptor.id.clone());
    }

    for entry in previous_cache.descriptors.values() {
        id_index
            .descriptor_id_by_object
            .insert(entry.object_id, entry.descriptor.id.clone());
    }

    id_index
}

/// Reuse one previous identifier when possible, otherwise allocate one scoped identifier.
fn reused_or_scoped_identifier(
    previous_id: Option<String>,
    used_ids: &mut BTreeSet<String>,
    allocate: impl FnOnce(&BTreeSet<String>) -> String,
) -> String {
    // prefer the exact previous id when the host reused the same object identity
    if let Some(previous_id) = previous_id {
        used_ids.insert(previous_id.clone());

        return previous_id;
    }

    let identifier = allocate(used_ids);
    used_ids.insert(identifier.clone());

    identifier
}

// NOTE #Architecture: CoreBluetooth does not expose stable ATT handles here,
// so these ids stay session-local and are derived from UUID plus sibling occurrence
/// Build one session-local CoreBluetooth service identifier.
fn stable_service_identifier(uuid: &str, used_ids: &BTreeSet<String>) -> String {
    allocate_scoped_identifier("service", uuid, used_ids)
}

// NOTE #Architecture: CoreBluetooth does not expose stable ATT handles here,
// so these ids stay session-local and are derived from UUID plus sibling occurrence
/// Build one session-local CoreBluetooth characteristic identifier.
fn stable_characteristic_identifier(
    service_id: &str,
    uuid: &str,
    used_ids: &BTreeSet<String>,
) -> String {
    allocate_scoped_identifier(service_id, uuid, used_ids)
}

// NOTE #Architecture: CoreBluetooth does not expose stable ATT handles here,
// so these ids stay session-local and are derived from UUID plus sibling occurrence
/// Build one session-local CoreBluetooth descriptor identifier.
fn stable_descriptor_identifier(
    characteristic_id: &str,
    uuid: &str,
    used_ids: &BTreeSet<String>,
) -> String {
    allocate_scoped_identifier(characteristic_id, uuid, used_ids)
}

/// Allocate one scoped identifier without colliding with already-used ids.
fn allocate_scoped_identifier(scope: &str, uuid: &str, used_ids: &BTreeSet<String>) -> String {
    let mut occurrence = 0usize;

    loop {
        let candidate = format!("{scope}:{uuid}:{occurrence}");
        if !used_ids.contains(&candidate) {
            return candidate;
        }

        occurrence += 1;
    }
}

/// One lookup index for previously assigned CoreBluetooth ids.
#[derive(Default)]
struct MacBluetoothIdIndex {
    /// The service ids keyed by service object identity.
    service_id_by_object: HashMap<usize, String>,
    /// The characteristic ids keyed by characteristic object identity.
    characteristic_id_by_object: HashMap<usize, String>,
    /// The descriptor ids keyed by descriptor object identity.
    descriptor_id_by_object: HashMap<usize, String>,
}

impl MacBluetoothIdIndex {
    /// Return the previous service id for one service object identity.
    fn service_id(&self, object_id: usize) -> Option<String> {
        self.service_id_by_object.get(&object_id).cloned()
    }

    /// Return the previous characteristic id for one characteristic object identity.
    fn characteristic_id(&self, object_id: usize) -> Option<String> {
        self.characteristic_id_by_object.get(&object_id).cloned()
    }

    /// Return the previous descriptor id for one descriptor object identity.
    fn descriptor_id(&self, object_id: usize) -> Option<String> {
        self.descriptor_id_by_object.get(&object_id).cloned()
    }
}

/// Refresh the device cache when the backend marked it stale.
pub(super) fn refresh_device_cache_if_needed(
    resource: &Arc<MacBluetoothDeviceResource>,
) -> RuntimeResult<()> {
    if !resource.cache_stale.swap(false, Ordering::Relaxed) {
        return Ok(());
    }

    let previous_cache = resource.cache.lock().clone();
    let cache = refresh_gatt_cache(
        &resource.peripheral,
        &resource.pending,
        &resource.operation_lock,
        Some(&previous_cache),
    )?;

    *resource.cache.lock() = cache;

    Ok(())
}

/// Resolve one scan resource payload.
pub(super) fn scan_resource(
    binding: &BindingCallContext,
    handle: resource::BluetoothScanHandle,
    operation: &'static str,
) -> RuntimeResult<Arc<MacBluetoothScanResource>> {
    bluetooth_payload(
        binding,
        handle.0,
        ResourceKind::BluetoothScan,
        operation,
        "scan",
    )
}

/// Resolve one device resource payload.
pub(super) fn device_resource(
    binding: &BindingCallContext,
    handle: resource::BluetoothDeviceHandle,
    operation: &'static str,
) -> RuntimeResult<Arc<MacBluetoothDeviceResource>> {
    bluetooth_payload(
        binding,
        handle.0,
        ResourceKind::BluetoothDevice,
        operation,
        "device",
    )
}

/// Resolve one subscription resource payload.
pub(super) fn subscription_resource(
    binding: &BindingCallContext,
    handle: resource::BluetoothSubscriptionHandle,
    operation: &'static str,
) -> RuntimeResult<Arc<MacBluetoothSubscriptionResource>> {
    bluetooth_payload(
        binding,
        handle.0,
        ResourceKind::BluetoothSubscription,
        operation,
        "subscription",
    )
}

/// Resolve one scan event with timeout handling.
pub(super) fn pop_scan_event(
    binding: &BindingCallContext,
    resource: &Arc<MacBluetoothScanResource>,
    timeout_ns: u64,
    operation: &'static str,
) -> RuntimeResult<BluetoothScanEvent> {
    let value = resource.event_queue.pop_with_timeout_or_else(
        std::time::Duration::from_nanos(timeout_ns),
        || {
            Err(core_platform::io_would_block(
                operation,
                "bluetooth scan event queue is empty",
            ))
        },
    )?;

    Ok(stored_scan_event(binding, value))
}

/// Resolve one device event with timeout handling.
pub(super) fn pop_session_event(
    binding: &BindingCallContext,
    resource: &Arc<MacBluetoothDeviceResource>,
    timeout_ns: u64,
    operation: &'static str,
) -> RuntimeResult<BluetoothSessionEvent> {
    let value = resource.event_queue.pop_with_timeout_or_else(
        std::time::Duration::from_nanos(timeout_ns),
        || {
            Err(core_platform::io_would_block(
                operation,
                "bluetooth session event queue is empty",
            ))
        },
    )?;

    Ok(stored_session_event(binding, value))
}

/// Resolve one GATT notification value with timeout handling.
pub(super) fn pop_gatt_value_event(
    binding: &BindingCallContext,
    resource: &Arc<MacBluetoothSubscriptionResource>,
    timeout_ns: u64,
    operation: &'static str,
) -> RuntimeResult<BluetoothGattValueEvent> {
    let value = resource.queue.pop_with_timeout_or_else(
        std::time::Duration::from_nanos(timeout_ns),
        || {
            Err(core_platform::io_would_block(
                operation,
                "bluetooth value event queue is empty",
            ))
        },
    )?;

    Ok(stored_gatt_value_event(binding, value))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Reuse one previous CoreBluetooth id when the object identity is unchanged.
    #[test]
    fn test_reused_or_scoped_identifier_prefers_previous_id() {
        let mut used_ids = BTreeSet::new();

        let identifier = reused_or_scoped_identifier(
            Some(String::from("service:180d:0")),
            &mut used_ids,
            |_| String::from("service:180d:1"),
        );

        assert_eq!(identifier, "service:180d:0");
        assert!(used_ids.contains("service:180d:0"));
    }

    /// Allocate scoped CoreBluetooth ids by UUID instead of discovery position.
    #[test]
    fn test_allocate_scoped_identifier_is_uuid_scoped() {
        let mut used_ids = BTreeSet::new();

        let heart_rate = stable_service_identifier("180d", &used_ids);
        used_ids.insert(heart_rate.clone());

        let battery = stable_service_identifier("180f", &used_ids);
        used_ids.insert(battery.clone());

        let second_heart_rate = stable_service_identifier("180d", &used_ids);

        assert_eq!(heart_rate, "service:180d:0");
        assert_eq!(battery, "service:180f:0");
        assert_eq!(second_heart_rate, "service:180d:1");
    }
}
