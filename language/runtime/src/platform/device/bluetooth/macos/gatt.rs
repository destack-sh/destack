#![allow(unsafe_op_in_unsafe_fn)]

use super::core::*;
use super::metadata::*;
use super::runtime::*;

/// List one macOS bluetooth device's GATT services.
pub(crate) unsafe fn destack_device_bluetooth_gatt_service_list(
    binding: &BindingCallContext,
    out: *mut NativeSlice<BluetoothGattService>,
    handle: resource::BluetoothDeviceHandle,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    let resource = device_resource(binding, handle, "destack.device.bluetooth.gatt.serviceList")?;
    refresh_device_cache_if_needed(&resource)?;
    let services = resource
        .cache
        .lock()
        .services
        .values()
        .cloned()
        .map(|entry| BluetoothGattService::from_value(binding, entry.descriptor))
        .collect::<Vec<_>>();
    out.write(binding.store_slice(services));

    Ok(())
}

/// List cached GATT characteristics.
pub(crate) unsafe fn destack_device_bluetooth_gatt_characteristic_list(
    binding: &BindingCallContext,
    out: *mut NativeSlice<BluetoothGattCharacteristic>,
    handle: resource::BluetoothDeviceHandle,
    serviceid: NativeStringRef,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    let service_id = serviceid.as_str()?;
    let resource = device_resource(
        binding,
        handle,
        "destack.device.bluetooth.gatt.characteristicList",
    )?;
    refresh_device_cache_if_needed(&resource)?;
    let cache = resource.cache.lock();
    if !cache.services.contains_key(service_id) {
        return Err(core_platform::io_not_found(
            "destack.device.bluetooth.gatt.characteristicList",
            "unknown bluetooth GATT service",
        ));
    }

    let characteristics = cache
        .characteristics
        .values()
        .filter(|entry| entry.descriptor.service_id == service_id)
        .cloned()
        .map(|entry| BluetoothGattCharacteristic::from_value(binding, entry.descriptor))
        .collect::<Vec<_>>();
    out.write(binding.store_slice(characteristics));

    Ok(())
}

/// List cached GATT descriptors.
pub(crate) unsafe fn destack_device_bluetooth_gatt_descriptor_list(
    binding: &BindingCallContext,
    out: *mut NativeSlice<BluetoothGattDescriptor>,
    handle: resource::BluetoothDeviceHandle,
    characteristicid: NativeStringRef,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    let characteristic_id = characteristicid.as_str()?;
    let resource = device_resource(
        binding,
        handle,
        "destack.device.bluetooth.gatt.descriptorList",
    )?;
    refresh_device_cache_if_needed(&resource)?;
    let cache = resource.cache.lock();
    if !cache.characteristics.contains_key(characteristic_id) {
        return Err(core_platform::io_not_found(
            "destack.device.bluetooth.gatt.descriptorList",
            "unknown bluetooth GATT characteristic",
        ));
    }

    let descriptors = cache
        .descriptors
        .values()
        .filter(|entry| entry.descriptor.characteristic_id == characteristic_id)
        .cloned()
        .map(|entry| BluetoothGattDescriptor::from_value(binding, entry.descriptor))
        .collect::<Vec<_>>();
    out.write(binding.store_slice(descriptors));

    Ok(())
}

/// Read one GATT descriptor value.
pub(crate) unsafe fn destack_device_bluetooth_gatt_read_descriptor(
    binding: &BindingCallContext,
    out: *mut NativeSlice<u8>,
    handle: resource::BluetoothDeviceHandle,
    descriptorid: NativeStringRef,
    timeoutns: u64,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    let descriptor_id = descriptorid.as_str()?;
    let resource = device_resource(
        binding,
        handle,
        "destack.device.bluetooth.gatt.readDescriptor",
    )?;
    refresh_device_cache_if_needed(&resource)?;
    let descriptor = resource
        .cache
        .lock()
        .descriptors
        .get(descriptor_id)
        .cloned()
        .ok_or_else(|| {
            core_platform::io_not_found(
                "destack.device.bluetooth.gatt.readDescriptor",
                "unknown bluetooth GATT descriptor",
            )
        })?;

    let _operation_lock = resource.operation_lock.lock();
    resource
        .pending
        .begin(MacBluetoothPendingTask::ReadDescriptor {
            descriptor_object_id: descriptor.object_id,
        });
    resource.peripheral.dispatch(|peripheral| unsafe {
        peripheral.readValueForDescriptor(descriptor.descriptor_object.get_unchecked());
    });
    let value = wait_for_pending_operation(
        &resource.pending,
        "destack.device.bluetooth.gatt.readDescriptor",
        timeoutns,
    )?;
    let MacBluetoothPendingValue::Bytes(value) = value else {
        return Err(core_platform::io_operation_error(
            "destack.device.bluetooth.gatt.readDescriptor",
            Some(PlatformErrorCode::IoInvalidData),
            "corebluetooth returned one mismatched descriptor callback payload",
        ));
    };

    out.write(binding.store_slice(value));

    Ok(())
}

/// Write one GATT descriptor value.
pub(crate) unsafe fn destack_device_bluetooth_gatt_write_descriptor(
    binding: &BindingCallContext,
    handle: resource::BluetoothDeviceHandle,
    descriptorid: NativeStringRef,
    value: NativeSlice<u8>,
    timeoutns: u64,
) -> RuntimeResult<()> {
    let descriptor_id = descriptorid.as_str()?;
    let resource = device_resource(
        binding,
        handle,
        "destack.device.bluetooth.gatt.writeDescriptor",
    )?;
    refresh_device_cache_if_needed(&resource)?;
    let descriptor = resource
        .cache
        .lock()
        .descriptors
        .get(descriptor_id)
        .cloned()
        .ok_or_else(|| {
            core_platform::io_not_found(
                "destack.device.bluetooth.gatt.writeDescriptor",
                "unknown bluetooth GATT descriptor",
            )
        })?;

    let bytes = value.as_slice()?;
    let bytes = bytes.to_vec();
    let _operation_lock = resource.operation_lock.lock();
    resource
        .pending
        .begin(MacBluetoothPendingTask::WriteDescriptor {
            descriptor_object_id: descriptor.object_id,
        });
    resource.peripheral.dispatch(move |peripheral| unsafe {
        let data = NSData::with_bytes(&bytes);
        peripheral.writeValue_forDescriptor(&data, descriptor.descriptor_object.get_unchecked());
    });
    let _ = wait_for_pending_operation(
        &resource.pending,
        "destack.device.bluetooth.gatt.writeDescriptor",
        timeoutns,
    )?;

    Ok(())
}

/// Read the current ATT MTU.
pub(crate) unsafe fn destack_device_bluetooth_gatt_mtu(
    binding: &BindingCallContext,
    out: *mut u16,
    handle: resource::BluetoothDeviceHandle,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    let resource = device_resource(binding, handle, "destack.device.bluetooth.gatt.mtu")?;
    out.write(inferred_mtu(&resource.peripheral));

    Ok(())
}

/// Read one GATT characteristic value.
pub(crate) unsafe fn destack_device_bluetooth_gatt_read(
    binding: &BindingCallContext,
    out: *mut NativeSlice<u8>,
    handle: resource::BluetoothDeviceHandle,
    characteristicid: NativeStringRef,
    timeoutns: u64,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    let characteristic_id = characteristicid.as_str()?;
    let resource = device_resource(binding, handle, "destack.device.bluetooth.gatt.read")?;
    refresh_device_cache_if_needed(&resource)?;
    let characteristic = resource
        .cache
        .lock()
        .characteristics
        .get(characteristic_id)
        .cloned()
        .ok_or_else(|| {
            core_platform::io_not_found(
                "destack.device.bluetooth.gatt.read",
                "unknown bluetooth GATT characteristic",
            )
        })?;

    let _operation_lock = resource.operation_lock.lock();
    resource
        .pending
        .begin(MacBluetoothPendingTask::ReadCharacteristic {
            characteristic_object_id: characteristic.object_id,
        });
    resource.peripheral.dispatch(|peripheral| unsafe {
        peripheral.readValueForCharacteristic(characteristic.characteristic.get_unchecked());
    });
    let value = wait_for_pending_operation(
        &resource.pending,
        "destack.device.bluetooth.gatt.read",
        timeoutns,
    )?;
    let MacBluetoothPendingValue::Bytes(value) = value else {
        return Err(core_platform::io_operation_error(
            "destack.device.bluetooth.gatt.read",
            Some(PlatformErrorCode::IoInvalidData),
            "corebluetooth returned one mismatched characteristic callback payload",
        ));
    };

    out.write(binding.store_slice(value));

    Ok(())
}

/// Write one GATT characteristic value.
pub(crate) unsafe fn destack_device_bluetooth_gatt_write(
    binding: &BindingCallContext,
    handle: resource::BluetoothDeviceHandle,
    characteristicid: NativeStringRef,
    value: NativeSlice<u8>,
    mode: BluetoothGattWriteMode,
    timeoutns: u64,
) -> RuntimeResult<()> {
    let characteristic_id = characteristicid.as_str()?;
    let resource = device_resource(binding, handle, "destack.device.bluetooth.gatt.write")?;
    refresh_device_cache_if_needed(&resource)?;
    let characteristic = resource
        .cache
        .lock()
        .characteristics
        .get(characteristic_id)
        .cloned()
        .ok_or_else(|| {
            core_platform::io_not_found(
                "destack.device.bluetooth.gatt.write",
                "unknown bluetooth GATT characteristic",
            )
        })?;

    let bytes = value.as_slice()?;
    let bytes = bytes.to_vec();
    let write_type = match mode {
        BluetoothGattWriteMode::WithResponse => CBCharacteristicWriteType::WithResponse,
        BluetoothGattWriteMode::WithoutResponse => CBCharacteristicWriteType::WithoutResponse,
    };
    let _operation_lock = resource.operation_lock.lock();
    if write_type == CBCharacteristicWriteType::WithResponse {
        resource
            .pending
            .begin(MacBluetoothPendingTask::WriteCharacteristic {
                characteristic_object_id: characteristic.object_id,
            });
    }
    resource.peripheral.dispatch(move |peripheral| unsafe {
        let data = NSData::with_bytes(&bytes);
        peripheral.writeValue_forCharacteristic_type(
            &data,
            characteristic.characteristic.get_unchecked(),
            write_type,
        );
    });
    if write_type == CBCharacteristicWriteType::WithResponse {
        let _ = wait_for_pending_operation(
            &resource.pending,
            "destack.device.bluetooth.gatt.write",
            timeoutns,
        )?;
    }

    Ok(())
}

/// Subscribe one GATT characteristic.
pub(crate) unsafe fn destack_device_bluetooth_gatt_subscribe(
    binding: &BindingCallContext,
    out: *mut resource::BluetoothSubscriptionHandle,
    handle: resource::BluetoothDeviceHandle,
    characteristicid: NativeStringRef,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    let characteristic_id = characteristicid.as_str()?;
    let resource = device_resource(binding, handle, "destack.device.bluetooth.gatt.subscribe")?;
    refresh_device_cache_if_needed(&resource)?;
    let cache = resource.cache.lock();
    let characteristic = cache
        .characteristics
        .get(characteristic_id)
        .cloned()
        .ok_or_else(|| {
            core_platform::io_not_found(
                "destack.device.bluetooth.gatt.subscribe",
                "unknown bluetooth GATT characteristic",
            )
        })?;
    if !characteristic.descriptor.properties.notify
        && !characteristic.descriptor.properties.indicate
    {
        return Err(core_platform::invalid_argument(
            "characteristicId",
            "characteristic does not support notifications or indications",
        ));
    }
    let service = cache
        .services
        .get(&characteristic.descriptor.service_id)
        .cloned()
        .ok_or_else(|| {
            core_platform::io_not_found(
                "destack.device.bluetooth.gatt.subscribe",
                "unknown bluetooth GATT service",
            )
        })?;
    drop(cache);

    let queue = Arc::new(BoundedQueue::new(BLUETOOTH_NOTIFICATION_QUEUE_CAPACITY));
    resource.subscriptions.lock().insert(
        characteristic.object_id,
        MacBluetoothSubscriptionState {
            service_id: service.descriptor.id.clone(),
            characteristic_id: characteristic.descriptor.id.clone(),
            service_uuid: service.descriptor.uuid.clone(),
            characteristic_uuid: characteristic.descriptor.uuid.clone(),
            queue: queue.clone(),
        },
    );

    let _operation_lock = resource.operation_lock.lock();
    resource
        .pending
        .begin(MacBluetoothPendingTask::UpdateNotification {
            characteristic_object_id: characteristic.object_id,
        });
    resource.peripheral.dispatch(|peripheral| unsafe {
        peripheral
            .setNotifyValue_forCharacteristic(true, characteristic.characteristic.get_unchecked());
    });
    if let Err(error) = wait_for_pending_operation(
        &resource.pending,
        "destack.device.bluetooth.gatt.subscribe",
        BLUETOOTH_DEFAULT_TIMEOUT_NS,
    ) {
        resource
            .subscriptions
            .lock()
            .remove(&characteristic.object_id);
        return Err(error);
    }

    let entry = ResourceEntry::new(ResourceKind::BluetoothSubscription)
        .with_label(BLUETOOTH_SUBSCRIPTION_RESOURCE_LABEL)
        .with_payload(Arc::new(MacBluetoothSubscriptionResource {
            queue: queue.clone(),
        }))
        .with_finalizer(binding.worker().platform_state.device.wrap_finalizer(
            MacBluetoothSubscriptionFinalizer {
                peripheral: resource.peripheral.clone(),
                characteristic: characteristic.characteristic.clone(),
                pending: resource.pending.clone(),
                operation_lock: resource.operation_lock.clone(),
                subscriptions: resource.subscriptions.clone(),
                queue,
            },
        ));
    let resource_id =
        binding
            .worker()
            .resources
            .insert(binding.world(), entry, Some(binding.engine()));

    out.write(resource::BluetoothSubscriptionHandle(resource_id));

    Ok(())
}

/// Unsubscribe one GATT characteristic.
pub(crate) unsafe fn destack_device_bluetooth_gatt_unsubscribe(
    binding: &BindingCallContext,
    handle: resource::BluetoothSubscriptionHandle,
) -> RuntimeResult<()> {
    close_bluetooth_resource(
        binding,
        handle.0,
        ResourceKind::BluetoothSubscription,
        "destack.device.bluetooth.gatt.unsubscribe",
        "subscription",
    )
}

/// Read one blocking notification event.
pub(crate) unsafe fn destack_device_bluetooth_gatt_read_event(
    binding: &BindingCallContext,
    out: *mut BluetoothGattValueEvent,
    handle: resource::BluetoothSubscriptionHandle,
    timeoutns: u64,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    let resource =
        subscription_resource(binding, handle, "destack.device.bluetooth.gatt.readEvent")?;
    out.write(pop_gatt_value_event(
        binding,
        &resource,
        timeoutns,
        "destack.device.bluetooth.gatt.readEvent",
    )?);

    Ok(())
}

/// Poll one notification event without blocking.
pub(crate) unsafe fn destack_device_bluetooth_gatt_try_read_event(
    binding: &BindingCallContext,
    out: *mut BluetoothGattValueEvent,
    handle: resource::BluetoothSubscriptionHandle,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    let resource = subscription_resource(
        binding,
        handle,
        "destack.device.bluetooth.gatt.tryReadEvent",
    )?;
    let value = resource.queue.try_pop_or_else(|| {
        Err(core_platform::io_would_block(
            "destack.device.bluetooth.gatt.tryReadEvent",
            "bluetooth value event queue is empty",
        ))
    })?;
    out.write(stored_gatt_value_event(binding, value));

    Ok(())
}
