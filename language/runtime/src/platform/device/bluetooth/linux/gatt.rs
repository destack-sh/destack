use super::core::*;
use super::metadata::*;
use super::watch::*;

/// Resolve one Linux bluetooth subscription resource.
fn subscription_resource(
    binding: &BindingCallContext,
    handle: resource::BluetoothSubscriptionHandle,
    operation: &'static str,
) -> RuntimeResult<Arc<LinuxBluetoothSubscriptionResource>> {
    bluetooth_payload(
        binding,
        handle.0,
        ResourceKind::BluetoothSubscription,
        operation,
        "subscription",
    )
}

/// Refresh and return the current BlueZ GATT cache for one device.
fn refreshed_cache(
    resource: &LinuxBluetoothDeviceResource,
    operation: &'static str,
) -> RuntimeResult<LinuxBluetoothGattCache> {
    let connection = resource.service.connection();
    let objects = bluez_managed_objects(&connection, operation)?;
    let cache = gatt_cache(&resource.device_path, &objects, operation)?;
    *resource.cache.lock() = cache.clone();

    Ok(cache)
}

/// List one Linux bluetooth device's GATT services.
pub(crate) unsafe fn destack_device_bluetooth_gatt_service_list(
    binding: &BindingCallContext,
    out: *mut NativeSlice<BluetoothGattService>,
    handle: resource::BluetoothDeviceHandle,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    let resource = device_resource(binding, handle, "destack.device.bluetooth.gatt.serviceList")?;
    let cache = refreshed_cache(&resource, "destack.device.bluetooth.gatt.serviceList")?;
    let services = cache
        .services
        .values()
        .cloned()
        .map(|service| BluetoothGattService::from_value(binding, service))
        .collect::<Vec<_>>();

    unsafe {
        out.write(binding.store_slice(services));
    }

    Ok(())
}

/// List one Linux bluetooth service's GATT characteristics.
pub(crate) unsafe fn destack_device_bluetooth_gatt_characteristic_list(
    binding: &BindingCallContext,
    out: *mut NativeSlice<BluetoothGattCharacteristic>,
    handle: resource::BluetoothDeviceHandle,
    serviceid: NativeStringRef,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    let service_id = unsafe { serviceid.as_str()? };
    let resource = device_resource(
        binding,
        handle,
        "destack.device.bluetooth.gatt.characteristicList",
    )?;
    let cache = refreshed_cache(
        &resource,
        "destack.device.bluetooth.gatt.characteristicList",
    )?;
    let characteristics = cache
        .characteristics
        .values()
        .filter(|characteristic| characteristic.service_id == service_id)
        .cloned()
        .map(|characteristic| BluetoothGattCharacteristic::from_value(binding, characteristic))
        .collect::<Vec<_>>();

    unsafe {
        out.write(binding.store_slice(characteristics));
    }

    Ok(())
}

/// List one Linux bluetooth characteristic's GATT descriptors.
pub(crate) unsafe fn destack_device_bluetooth_gatt_descriptor_list(
    binding: &BindingCallContext,
    out: *mut NativeSlice<BluetoothGattDescriptor>,
    handle: resource::BluetoothDeviceHandle,
    characteristicid: NativeStringRef,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    let characteristic_id = unsafe { characteristicid.as_str()? };
    let resource = device_resource(
        binding,
        handle,
        "destack.device.bluetooth.gatt.descriptorList",
    )?;
    let cache = refreshed_cache(&resource, "destack.device.bluetooth.gatt.descriptorList")?;
    let descriptors = cache
        .descriptors
        .values()
        .filter(|descriptor| descriptor.characteristic_id == characteristic_id)
        .cloned()
        .map(|descriptor| BluetoothGattDescriptor::from_value(binding, descriptor))
        .collect::<Vec<_>>();

    unsafe {
        out.write(binding.store_slice(descriptors));
    }

    Ok(())
}

/// Read one Linux bluetooth ATT MTU value.
pub(crate) unsafe fn destack_device_bluetooth_gatt_mtu(
    binding: &BindingCallContext,
    out: *mut u16,
    handle: resource::BluetoothDeviceHandle,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    let resource = device_resource(binding, handle, "destack.device.bluetooth.gatt.mtu")?;

    unsafe {
        out.write(current_mtu(&resource));
    }

    Ok(())
}

/// Read one Linux bluetooth characteristic value.
pub(crate) unsafe fn destack_device_bluetooth_gatt_read(
    binding: &BindingCallContext,
    out: *mut NativeSlice<u8>,
    handle: resource::BluetoothDeviceHandle,
    characteristicid: NativeStringRef,
    _timeoutns: u64,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    let resource = device_resource(binding, handle, "destack.device.bluetooth.gatt.read")?;
    let cache = refreshed_cache(&resource, "destack.device.bluetooth.gatt.read")?;
    let characteristic_id = unsafe { characteristicid.as_str()? };
    let characteristic = cache
        .characteristics
        .get(characteristic_id)
        .ok_or_else(|| {
            core_platform::io_not_found(
                "destack.device.bluetooth.gatt.read",
                "unknown bluetooth GATT characteristic",
            )
        })?;
    let connection = resource.service.connection();
    let bytes = read_gatt_value(
        &connection,
        &characteristic.id,
        BLUEZ_GATT_CHARACTERISTIC_INTERFACE,
        "destack.device.bluetooth.gatt.read",
    )?;

    unsafe {
        out.write(binding.store_slice(bytes));
    }

    Ok(())
}

/// Read one Linux bluetooth descriptor value.
pub(crate) unsafe fn destack_device_bluetooth_gatt_read_descriptor(
    binding: &BindingCallContext,
    out: *mut NativeSlice<u8>,
    handle: resource::BluetoothDeviceHandle,
    descriptorid: NativeStringRef,
    _timeoutns: u64,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    let resource = device_resource(
        binding,
        handle,
        "destack.device.bluetooth.gatt.readDescriptor",
    )?;
    let cache = refreshed_cache(&resource, "destack.device.bluetooth.gatt.readDescriptor")?;
    let descriptor_id = unsafe { descriptorid.as_str()? };
    let descriptor = cache.descriptors.get(descriptor_id).ok_or_else(|| {
        core_platform::io_not_found(
            "destack.device.bluetooth.gatt.readDescriptor",
            "unknown bluetooth GATT descriptor",
        )
    })?;
    let connection = resource.service.connection();
    let bytes = read_gatt_value(
        &connection,
        &descriptor.id,
        BLUEZ_GATT_DESCRIPTOR_INTERFACE,
        "destack.device.bluetooth.gatt.readDescriptor",
    )?;

    unsafe {
        out.write(binding.store_slice(bytes));
    }

    Ok(())
}

/// Write one Linux bluetooth characteristic value.
pub(crate) unsafe fn destack_device_bluetooth_gatt_write(
    binding: &BindingCallContext,
    handle: resource::BluetoothDeviceHandle,
    characteristicid: NativeStringRef,
    value: NativeSlice<u8>,
    mode: BluetoothGattWriteMode,
    _timeoutns: u64,
) -> RuntimeResult<()> {
    let resource = device_resource(binding, handle, "destack.device.bluetooth.gatt.write")?;
    let cache = refreshed_cache(&resource, "destack.device.bluetooth.gatt.write")?;
    let characteristic_id = unsafe { characteristicid.as_str()? };
    let characteristic = cache
        .characteristics
        .get(characteristic_id)
        .ok_or_else(|| {
            core_platform::io_not_found(
                "destack.device.bluetooth.gatt.write",
                "unknown bluetooth GATT characteristic",
            )
        })?;
    let bytes = unsafe { value.as_slice()? };
    let mut options = HashMap::<String, OwnedValue>::new();
    let write_type = match mode {
        BluetoothGattWriteMode::WithResponse => "request",
        BluetoothGattWriteMode::WithoutResponse => "command",
    };
    options.insert(
        String::from("type"),
        OwnedValue::from(Str::from(write_type)),
    );

    let connection = resource.service.connection();
    write_gatt_value(
        &connection,
        &characteristic.id,
        BLUEZ_GATT_CHARACTERISTIC_INTERFACE,
        bytes,
        options,
        "destack.device.bluetooth.gatt.write",
    )
}

/// Write one Linux bluetooth descriptor value.
pub(crate) unsafe fn destack_device_bluetooth_gatt_write_descriptor(
    binding: &BindingCallContext,
    handle: resource::BluetoothDeviceHandle,
    descriptorid: NativeStringRef,
    value: NativeSlice<u8>,
    _timeoutns: u64,
) -> RuntimeResult<()> {
    let resource = device_resource(
        binding,
        handle,
        "destack.device.bluetooth.gatt.writeDescriptor",
    )?;
    let cache = refreshed_cache(&resource, "destack.device.bluetooth.gatt.writeDescriptor")?;
    let descriptor_id = unsafe { descriptorid.as_str()? };
    let descriptor = cache.descriptors.get(descriptor_id).ok_or_else(|| {
        core_platform::io_not_found(
            "destack.device.bluetooth.gatt.writeDescriptor",
            "unknown bluetooth GATT descriptor",
        )
    })?;
    let bytes = unsafe { value.as_slice()? };

    let connection = resource.service.connection();
    write_gatt_value(
        &connection,
        &descriptor.id,
        BLUEZ_GATT_DESCRIPTOR_INTERFACE,
        bytes,
        HashMap::new(),
        "destack.device.bluetooth.gatt.writeDescriptor",
    )
}

/// Subscribe to one Linux bluetooth characteristic value stream.
pub(crate) unsafe fn destack_device_bluetooth_gatt_subscribe(
    binding: &BindingCallContext,
    out: *mut resource::BluetoothSubscriptionHandle,
    handle: resource::BluetoothDeviceHandle,
    characteristicid: NativeStringRef,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    let device_resource =
        device_resource(binding, handle, "destack.device.bluetooth.gatt.subscribe")?;
    let cache = refreshed_cache(&device_resource, "destack.device.bluetooth.gatt.subscribe")?;
    let characteristic_id = unsafe { characteristicid.as_str()? };
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
    if !characteristic.properties.notify && !characteristic.properties.indicate {
        return Err(core_platform::invalid_argument(
            "characteristicId",
            "characteristic does not support notifications or indications",
        ));
    }
    let service_descriptor = cache
        .services
        .get(&characteristic.service_id)
        .cloned()
        .ok_or_else(|| {
            core_platform::io_not_found(
                "destack.device.bluetooth.gatt.subscribe",
                "unknown bluetooth GATT service",
            )
        })?;
    let bluetooth_service = device_resource.service.clone();
    let queue = Arc::new(BoundedQueue::new(BLUETOOTH_NOTIFICATION_QUEUE_CAPACITY));
    let subscription_resource = Arc::new(LinuxBluetoothSubscriptionResource {
        service_id: service_descriptor.id.clone(),
        characteristic_id: characteristic.id.clone(),
        characteristic_uuid: characteristic.uuid.clone(),
        service_uuid: service_descriptor.uuid.clone(),
        characteristic_path: characteristic.id.clone(),
        queue: queue.clone(),
    });
    let registration_id = bluetooth_service.register_subscription(&subscription_resource);
    if let Err(error) = bluetooth_service.retain_characteristic_notify(&characteristic.id) {
        bluetooth_service.unregister_subscription(registration_id);

        return Err(error);
    }

    let entry = ResourceEntry::new(ResourceKind::BluetoothSubscription)
        .with_label(BLUETOOTH_SUBSCRIPTION_RESOURCE_LABEL)
        .with_payload(subscription_resource.clone())
        .with_finalizer(binding.worker().platform_state.device.wrap_finalizer(
            LinuxBluetoothSubscriptionFinalizer {
                service: bluetooth_service,
                registration_id,
                characteristic_path: subscription_resource.characteristic_path.clone(),
                queue,
            },
        ));
    let handle = binding
        .worker()
        .resources
        .insert(&binding.world(), entry, Some(binding.engine()));

    unsafe {
        out.write(resource::BluetoothSubscriptionHandle(handle));
    }

    Ok(())
}

/// Unsubscribe from one Linux bluetooth characteristic value stream.
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

/// Read one Linux bluetooth characteristic value event.
pub(crate) unsafe fn destack_device_bluetooth_gatt_read_event(
    binding: &BindingCallContext,
    out: *mut BluetoothGattValueEvent,
    handle: resource::BluetoothSubscriptionHandle,
    timeoutns: u64,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    let resource =
        subscription_resource(binding, handle, "destack.device.bluetooth.gatt.readEvent")?;
    let event = pop_gatt_value_event(
        binding,
        &resource,
        timeoutns,
        "destack.device.bluetooth.gatt.readEvent",
    )?;

    unsafe {
        out.write(event);
    }

    Ok(())
}

/// Poll one Linux bluetooth characteristic value event without blocking.
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

    unsafe {
        out.write(stored_gatt_value_event(binding, value));
    }

    Ok(())
}
