use super::core::*;
use super::metadata::*;
use super::session::device_resource;

/// Resolve one typed Windows bluetooth subscription resource from the resource table.
fn subscription_resource(
    binding: &BindingCallContext,
    handle: resource::BluetoothSubscriptionHandle,
    operation: &'static str,
) -> DiagnosticResult<Arc<WindowsBluetoothSubscriptionResource>> {
    bluetooth_payload(
        binding,
        handle.0,
        ResourceKind::BluetoothSubscription,
        operation,
        "subscription",
    )
}

/// Refresh and return the cached Windows GATT graph for one session.
fn refreshed_cache<'a>(
    resource: &'a WindowsBluetoothDeviceResource,
    operation: &'static str,
) -> DiagnosticResult<parking_lot::MutexGuard<'a, WindowsBluetoothGattCache>> {
    // refresh only when the cache was invalidated by a session change
    if resource.is_cache_dirty.swap(false, Ordering::AcqRel) {
        let cache = refresh_gatt_cache(&resource.device, operation)?;
        let mut cache_guard = resource.cache.lock();
        *cache_guard = cache;

        return Ok(cache_guard);
    }

    Ok(resource.cache.lock())
}

/// List one Windows bluetooth device's GATT services.
pub(crate) unsafe fn destack_device_bluetooth_gatt_service_list(
    binding: &BindingCallContext,
    out: *mut NativeSlice<BluetoothGattService>,
    handle: resource::BluetoothDeviceHandle,
) -> DiagnosticResult<()> {
    core_platform::ensure_out(out, "out")?;

    let resource = device_resource(binding, handle, "destack.device.bluetooth.gatt.serviceList")?;
    let cache = refreshed_cache(&resource, "destack.device.bluetooth.gatt.serviceList")?;
    let services = cache
        .services
        .values()
        .cloned()
        .map(|service| BluetoothGattService::from_value(binding, service))
        .collect();

    unsafe {
        out.write(binding.store_slice(services));
    }

    Ok(())
}

/// List one Windows bluetooth service's GATT characteristics.
pub(crate) unsafe fn destack_device_bluetooth_gatt_characteristic_list(
    binding: &BindingCallContext,
    out: *mut NativeSlice<BluetoothGattCharacteristic>,
    handle: resource::BluetoothDeviceHandle,
    serviceid: NativeStringRef,
) -> DiagnosticResult<()> {
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
        .collect();

    unsafe {
        out.write(binding.store_slice(characteristics));
    }

    Ok(())
}

/// List one Windows bluetooth characteristic's GATT descriptors.
pub(crate) unsafe fn destack_device_bluetooth_gatt_descriptor_list(
    binding: &BindingCallContext,
    out: *mut NativeSlice<BluetoothGattDescriptor>,
    handle: resource::BluetoothDeviceHandle,
    characteristicid: NativeStringRef,
) -> DiagnosticResult<()> {
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
        .collect();

    unsafe {
        out.write(binding.store_slice(descriptors));
    }

    Ok(())
}

/// Read one Windows bluetooth ATT MTU value.
pub(crate) unsafe fn destack_device_bluetooth_gatt_mtu(
    binding: &BindingCallContext,
    out: *mut u16,
    handle: resource::BluetoothDeviceHandle,
) -> DiagnosticResult<()> {
    core_platform::ensure_out(out, "out")?;

    let resource = device_resource(binding, handle, "destack.device.bluetooth.gatt.mtu")?;
    let cache = refreshed_cache(&resource, "destack.device.bluetooth.gatt.mtu")?;

    unsafe {
        out.write(cache.mtu);
    }

    Ok(())
}

/// Read one Windows bluetooth characteristic value.
pub(crate) unsafe fn destack_device_bluetooth_gatt_read(
    binding: &BindingCallContext,
    out: *mut NativeSlice<u8>,
    handle: resource::BluetoothDeviceHandle,
    characteristicid: NativeStringRef,
    _timeoutns: u64,
) -> DiagnosticResult<()> {
    core_platform::ensure_out(out, "out")?;

    let characteristic_id = unsafe { characteristicid.as_str()? };
    let resource = device_resource(binding, handle, "destack.device.bluetooth.gatt.read")?;
    let cache = refreshed_cache(&resource, "destack.device.bluetooth.gatt.read")?;
    let characteristic = cache
        .characteristic_objects
        .get(characteristic_id)
        .cloned()
        .ok_or_else(|| {
            core_platform::io_not_found(
                "destack.device.bluetooth.gatt.read",
                format!("bluetooth characteristic {characteristic_id} not found"),
            )
        })?;
    drop(cache);

    let result = characteristic
        .ReadValueWithCacheModeAsync(BluetoothCacheMode::Uncached)
        .map_err(|error| {
            windows_bluetooth_error(
                "destack.device.bluetooth.gatt.read",
                "GattCharacteristic::ReadValueWithCacheModeAsync",
                &error,
            )
        })?
        .get()
        .map_err(|error| {
            windows_bluetooth_error(
                "destack.device.bluetooth.gatt.read",
                "IAsyncOperation::get",
                &error,
            )
        })?;
    ensure_gatt_status(
        "destack.device.bluetooth.gatt.read",
        "GattCharacteristic::ReadValueWithCacheModeAsync",
        result.Status().map_err(|error| {
            windows_bluetooth_error(
                "destack.device.bluetooth.gatt.read",
                "GattReadResult::Status",
                &error,
            )
        })?,
        result.ProtocolError().ok().and_then(protocol_error_code),
    )?;
    let value = result.Value().map_err(|error| {
        windows_bluetooth_error(
            "destack.device.bluetooth.gatt.read",
            "GattReadResult::Value",
            &error,
        )
    })?;
    let bytes = buffer_to_vec(
        &value,
        "destack.device.bluetooth.gatt.read",
        "GattReadResult::Value",
    )?;

    unsafe {
        out.write(binding.store_slice(bytes));
    }

    Ok(())
}

/// Read one Windows bluetooth descriptor value.
pub(crate) unsafe fn destack_device_bluetooth_gatt_read_descriptor(
    binding: &BindingCallContext,
    out: *mut NativeSlice<u8>,
    handle: resource::BluetoothDeviceHandle,
    descriptorid: NativeStringRef,
    _timeoutns: u64,
) -> DiagnosticResult<()> {
    core_platform::ensure_out(out, "out")?;

    let descriptor_id = unsafe { descriptorid.as_str()? };
    let resource = device_resource(
        binding,
        handle,
        "destack.device.bluetooth.gatt.readDescriptor",
    )?;
    let cache = refreshed_cache(&resource, "destack.device.bluetooth.gatt.readDescriptor")?;
    let descriptor = cache
        .descriptor_objects
        .get(descriptor_id)
        .cloned()
        .ok_or_else(|| {
            core_platform::io_not_found(
                "destack.device.bluetooth.gatt.readDescriptor",
                format!("bluetooth descriptor {descriptor_id} not found"),
            )
        })?;
    drop(cache);

    let result = descriptor
        .ReadValueWithCacheModeAsync(BluetoothCacheMode::Uncached)
        .map_err(|error| {
            windows_bluetooth_error(
                "destack.device.bluetooth.gatt.readDescriptor",
                "GattDescriptor::ReadValueWithCacheModeAsync",
                &error,
            )
        })?
        .get()
        .map_err(|error| {
            windows_bluetooth_error(
                "destack.device.bluetooth.gatt.readDescriptor",
                "IAsyncOperation::get",
                &error,
            )
        })?;
    ensure_gatt_status(
        "destack.device.bluetooth.gatt.readDescriptor",
        "GattDescriptor::ReadValueWithCacheModeAsync",
        result.Status().map_err(|error| {
            windows_bluetooth_error(
                "destack.device.bluetooth.gatt.readDescriptor",
                "GattReadResult::Status",
                &error,
            )
        })?,
        result.ProtocolError().ok().and_then(protocol_error_code),
    )?;
    let value = result.Value().map_err(|error| {
        windows_bluetooth_error(
            "destack.device.bluetooth.gatt.readDescriptor",
            "GattReadResult::Value",
            &error,
        )
    })?;
    let bytes = buffer_to_vec(
        &value,
        "destack.device.bluetooth.gatt.readDescriptor",
        "GattReadResult::Value",
    )?;

    unsafe {
        out.write(binding.store_slice(bytes));
    }

    Ok(())
}

/// Write one Windows bluetooth characteristic value.
pub(crate) unsafe fn destack_device_bluetooth_gatt_write(
    binding: &BindingCallContext,
    handle: resource::BluetoothDeviceHandle,
    characteristicid: NativeStringRef,
    bytes: NativeSlice<u8>,
    writemode: BluetoothGattWriteMode,
    _timeoutns: u64,
) -> DiagnosticResult<()> {
    let characteristic_id = unsafe { characteristicid.as_str()? };
    let bytes = unsafe { bytes.as_slice()? };
    let resource = device_resource(binding, handle, "destack.device.bluetooth.gatt.write")?;
    let cache = refreshed_cache(&resource, "destack.device.bluetooth.gatt.write")?;
    let characteristic = cache
        .characteristic_objects
        .get(characteristic_id)
        .cloned()
        .ok_or_else(|| {
            core_platform::io_not_found(
                "destack.device.bluetooth.gatt.write",
                format!("bluetooth characteristic {characteristic_id} not found"),
            )
        })?;
    drop(cache);

    let buffer = vec_to_buffer(bytes, "destack.device.bluetooth.gatt.write", "DataWriter")?;
    let result = characteristic
        .WriteValueWithResultAndOptionAsync(&buffer, gatt_write_option(writemode))
        .map_err(|error| {
            windows_bluetooth_error(
                "destack.device.bluetooth.gatt.write",
                "GattCharacteristic::WriteValueWithResultAndOptionAsync",
                &error,
            )
        })?
        .get()
        .map_err(|error| {
            windows_bluetooth_error(
                "destack.device.bluetooth.gatt.write",
                "IAsyncOperation::get",
                &error,
            )
        })?;
    ensure_gatt_status(
        "destack.device.bluetooth.gatt.write",
        "GattCharacteristic::WriteValueWithResultAndOptionAsync",
        result.Status().map_err(|error| {
            windows_bluetooth_error(
                "destack.device.bluetooth.gatt.write",
                "GattWriteResult::Status",
                &error,
            )
        })?,
        result.ProtocolError().ok().and_then(protocol_error_code),
    )
}

/// Write one Windows bluetooth descriptor value.
pub(crate) unsafe fn destack_device_bluetooth_gatt_write_descriptor(
    binding: &BindingCallContext,
    handle: resource::BluetoothDeviceHandle,
    descriptorid: NativeStringRef,
    bytes: NativeSlice<u8>,
    _timeoutns: u64,
) -> DiagnosticResult<()> {
    let descriptor_id = unsafe { descriptorid.as_str()? };
    let bytes = unsafe { bytes.as_slice()? };
    let resource = device_resource(
        binding,
        handle,
        "destack.device.bluetooth.gatt.writeDescriptor",
    )?;
    let cache = refreshed_cache(&resource, "destack.device.bluetooth.gatt.writeDescriptor")?;
    let descriptor = cache
        .descriptor_objects
        .get(descriptor_id)
        .cloned()
        .ok_or_else(|| {
            core_platform::io_not_found(
                "destack.device.bluetooth.gatt.writeDescriptor",
                format!("bluetooth descriptor {descriptor_id} not found"),
            )
        })?;
    drop(cache);

    let buffer = vec_to_buffer(
        bytes,
        "destack.device.bluetooth.gatt.writeDescriptor",
        "DataWriter",
    )?;
    let result = descriptor
        .WriteValueWithResultAsync(&buffer)
        .map_err(|error| {
            windows_bluetooth_error(
                "destack.device.bluetooth.gatt.writeDescriptor",
                "GattDescriptor::WriteValueWithResultAsync",
                &error,
            )
        })?
        .get()
        .map_err(|error| {
            windows_bluetooth_error(
                "destack.device.bluetooth.gatt.writeDescriptor",
                "IAsyncOperation::get",
                &error,
            )
        })?;
    ensure_gatt_status(
        "destack.device.bluetooth.gatt.writeDescriptor",
        "GattDescriptor::WriteValueWithResultAsync",
        result.Status().map_err(|error| {
            windows_bluetooth_error(
                "destack.device.bluetooth.gatt.writeDescriptor",
                "GattWriteResult::Status",
                &error,
            )
        })?,
        result.ProtocolError().ok().and_then(protocol_error_code),
    )
}

/// Subscribe to one Windows bluetooth characteristic value stream.
pub(crate) unsafe fn destack_device_bluetooth_gatt_subscribe(
    binding: &BindingCallContext,
    out: *mut resource::BluetoothSubscriptionHandle,
    handle: resource::BluetoothDeviceHandle,
    characteristicid: NativeStringRef,
) -> DiagnosticResult<()> {
    core_platform::ensure_out(out, "out")?;

    let characteristic_id = unsafe { characteristicid.as_str()? };
    let resource = device_resource(binding, handle, "destack.device.bluetooth.gatt.subscribe")?;
    let cache = refreshed_cache(&resource, "destack.device.bluetooth.gatt.subscribe")?;
    let characteristic = cache
        .characteristic_objects
        .get(characteristic_id)
        .cloned()
        .ok_or_else(|| {
            core_platform::io_not_found(
                "destack.device.bluetooth.gatt.subscribe",
                format!("bluetooth characteristic {characteristic_id} not found"),
            )
        })?;
    let descriptor = cache
        .characteristics
        .get(characteristic_id)
        .cloned()
        .ok_or_else(|| {
            core_platform::io_not_found(
                "destack.device.bluetooth.gatt.subscribe",
                format!("bluetooth characteristic {characteristic_id} not found"),
            )
        })?;
    let service = cache
        .services
        .get(&descriptor.service_id)
        .cloned()
        .ok_or_else(|| {
            core_platform::io_not_found(
                "destack.device.bluetooth.gatt.subscribe",
                format!("bluetooth service {} not found", descriptor.service_id),
            )
        })?;
    drop(cache);

    let queue = Arc::new(BoundedQueue::new(BLUETOOTH_NOTIFICATION_QUEUE_CAPACITY));
    let callback_queue = queue.clone();
    let callback_service_id = descriptor.service_id.clone();
    let callback_characteristic_id = descriptor.id.clone();
    let callback_service_uuid = service.uuid.clone();
    let callback_characteristic_uuid = descriptor.uuid.clone();
    let value_changed_token = characteristic
        .ValueChanged(&TypedEventHandler::new(
            move |_characteristic: Ref<'_, GattCharacteristic>,
                  args: Ref<'_, GattValueChangedEventArgs>| {
                let Some(args) = args.as_ref() else {
                    return Ok(());
                };
                let buffer = match args.CharacteristicValue() {
                    Ok(buffer) => buffer,
                    Err(_) => return Ok(()),
                };
                let bytes = match buffer_to_vec(
                    &buffer,
                    "destack.device.bluetooth.gatt.subscribe",
                    "GattValueChangedEventArgs::CharacteristicValue",
                ) {
                    Ok(bytes) => bytes,
                    Err(_) => return Ok(()),
                };

                callback_queue.push_drop_oldest(gatt_value_event(
                    core_platform::monotonic_now_ns(),
                    callback_service_id.clone(),
                    callback_characteristic_id.clone(),
                    callback_service_uuid.clone(),
                    callback_characteristic_uuid.clone(),
                    bytes,
                ));

                Ok(())
            },
        ))
        .map_err(|error| {
            windows_bluetooth_error(
                "destack.device.bluetooth.gatt.subscribe",
                "GattCharacteristic::ValueChanged",
                &error,
            )
        })?;
    let result = characteristic
        .WriteClientCharacteristicConfigurationDescriptorWithResultAsync(
            GattClientCharacteristicConfigurationDescriptorValue::Notify,
        )
        .map_err(|error| {
            windows_bluetooth_error(
                "destack.device.bluetooth.gatt.subscribe",
                "GattCharacteristic::WriteClientCharacteristicConfigurationDescriptorWithResultAsync",
                &error,
            )
        })?
        .get()
        .map_err(|error| {
            windows_bluetooth_error(
                "destack.device.bluetooth.gatt.subscribe",
                "IAsyncOperation::get",
                &error,
            )
        })?;
    ensure_gatt_status(
        "destack.device.bluetooth.gatt.subscribe",
        "GattCharacteristic::WriteClientCharacteristicConfigurationDescriptorWithResultAsync",
        result.Status().map_err(|error| {
            windows_bluetooth_error(
                "destack.device.bluetooth.gatt.subscribe",
                "GattWriteResult::Status",
                &error,
            )
        })?,
        result.ProtocolError().ok().and_then(protocol_error_code),
    )?;

    let payload = Arc::new(WindowsBluetoothSubscriptionResource {
        queue: queue.clone(),
    });
    let entry = ResourceEntry::new(ResourceKind::BluetoothSubscription)
        .with_label(BLUETOOTH_SUBSCRIPTION_RESOURCE_LABEL)
        .with_payload(payload)
        .with_finalizer(binding.worker().platform_state.device.wrap_finalizer(
            WindowsBluetoothSubscriptionFinalizer {
                characteristic,
                value_changed_token,
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

/// Unsubscribe from one Windows bluetooth characteristic value stream.
pub(crate) unsafe fn destack_device_bluetooth_gatt_unsubscribe(
    binding: &BindingCallContext,
    handle: resource::BluetoothSubscriptionHandle,
) -> DiagnosticResult<()> {
    close_bluetooth_resource(
        binding,
        handle.0,
        ResourceKind::BluetoothSubscription,
        "destack.device.bluetooth.gatt.unsubscribe",
        "subscription",
    )
}

/// Read one Windows bluetooth characteristic value event.
pub(crate) unsafe fn destack_device_bluetooth_gatt_read_event(
    binding: &BindingCallContext,
    out: *mut BluetoothGattValueEvent,
    handle: resource::BluetoothSubscriptionHandle,
    timeoutns: u64,
) -> DiagnosticResult<()> {
    core_platform::ensure_out(out, "out")?;

    let resource =
        subscription_resource(binding, handle, "destack.device.bluetooth.gatt.readEvent")?;
    let value = resource.queue.pop_with_timeout_or_else(
        std::time::Duration::from_nanos(timeoutns),
        || {
            Err(core_platform::io_would_block(
                "destack.device.bluetooth.gatt.readEvent",
                "bluetooth notification did not yield one value",
            ))
        },
    )?;

    unsafe {
        out.write(stored_gatt_value_event(binding, value));
    }

    Ok(())
}

/// Poll one Windows bluetooth characteristic value event without blocking.
pub(crate) unsafe fn destack_device_bluetooth_gatt_try_read_event(
    binding: &BindingCallContext,
    out: *mut BluetoothGattValueEvent,
    handle: resource::BluetoothSubscriptionHandle,
) -> DiagnosticResult<()> {
    core_platform::ensure_out(out, "out")?;

    let resource = subscription_resource(
        binding,
        handle,
        "destack.device.bluetooth.gatt.tryReadEvent",
    )?;
    let value = resource.queue.try_pop_or_else(|| {
        Err(core_platform::io_would_block(
            "destack.device.bluetooth.gatt.tryReadEvent",
            "bluetooth notification did not yield one value",
        ))
    })?;

    unsafe {
        out.write(stored_gatt_value_event(binding, value));
    }

    Ok(())
}
