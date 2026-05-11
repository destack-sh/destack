use super::codec::*;
use super::session::*;

/// Resolve one Android bluetooth subscription resource.
fn subscription_resource(
    binding: &BindingCallContext,
    handle: resource::BluetoothSubscriptionHandle,
    operation: &'static str,
) -> RuntimeResult<Arc<AndroidBluetoothSubscriptionResource>> {
    bluetooth_payload(
        binding,
        handle.0,
        ResourceKind::BluetoothSubscription,
        operation,
        "subscription",
    )
}

/// Read one variable-length Android bluetooth value payload.
fn read_android_bluetooth_bytes(
    binding: &BindingCallContext,
    operation: &'static str,
    action: &'static str,
    call: impl Fn(NativeSlice<u8>, &mut u32) -> u32,
) -> RuntimeResult<Vec<u8>> {
    let _runtime_id = host_session_id(binding, operation)?;
    let mut bytes = vec![0u8; INITIAL_BLUETOOTH_STRING_CAPACITY];

    loop {
        let capacity = checked_u32_length(bytes.len(), operation, "value bytes")?;
        let mut bytes_written = 0u32;
        let status = call(
            NativeSlice {
                data: bytes.as_mut_ptr(),
                len: capacity,
            },
            &mut bytes_written,
        );

        if status == HostStatus::BufferTooSmall.code() {
            let next_capacity = bytes.len().max(bytes_written as usize).saturating_mul(2);
            if next_capacity > MAX_BLUETOOTH_STRING_CAPACITY {
                return Err(invalid_data(
                    operation,
                    "android host bluetooth payload exceeded the maximum byte payload size",
                ));
            }

            bytes.resize(next_capacity, 0);
            continue;
        }

        host_status_result(status, operation, action)?;
        bytes.truncate(bytes_written as usize);

        return Ok(bytes);
    }
}

/// List one Android bluetooth device's GATT services.
pub(crate) unsafe fn destack_device_bluetooth_gatt_service_list(
    binding: &BindingCallContext,
    out: *mut NativeSlice<BluetoothGattService>,
    handle: resource::BluetoothDeviceHandle,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    let resource = device_resource(binding, handle, "destack.device.bluetooth.gatt.serviceList")?;
    let cache = refresh_gatt_cache(
        binding,
        &resource,
        "destack.device.bluetooth.gatt.serviceList",
    )?;
    let services = cache
        .services
        .values()
        .cloned()
        .map(|service| BluetoothGattService::from_value(binding, service))
        .collect();
    *resource.cache.lock() = cache;

    unsafe {
        out.write(binding.store_slice(services));
    }

    Ok(())
}

/// List one Android bluetooth service's GATT characteristics.
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
    let characteristics = read_characteristics(
        binding,
        resource.session_id,
        service_id,
        "destack.device.bluetooth.gatt.characteristicList",
    )?;

    // refresh the cached characteristic slice
    {
        let mut cache = resource.cache.lock();
        cache
            .characteristics
            .retain(|_, candidate| candidate.service_id != service_id);
        for characteristic in &characteristics {
            cache
                .characteristics
                .insert(characteristic.id.clone(), characteristic.clone());
        }
    }

    let characteristics = characteristics
        .into_iter()
        .map(|characteristic| BluetoothGattCharacteristic::from_value(binding, characteristic))
        .collect();

    unsafe {
        out.write(binding.store_slice(characteristics));
    }

    Ok(())
}

/// List one Android bluetooth characteristic's GATT descriptors.
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
    let cache_snapshot = resource.cache.lock().clone();
    let descriptors = read_descriptors(
        binding,
        resource.session_id,
        characteristic_id,
        &cache_snapshot,
        "destack.device.bluetooth.gatt.descriptorList",
    )?;

    // refresh the cached descriptor slice
    {
        let mut cache = resource.cache.lock();
        cache
            .descriptors
            .retain(|_, candidate| candidate.characteristic_id != characteristic_id);
        for descriptor in &descriptors {
            cache
                .descriptors
                .insert(descriptor.id.clone(), descriptor.clone());
        }
    }

    let descriptors = descriptors
        .into_iter()
        .map(|descriptor| BluetoothGattDescriptor::from_value(binding, descriptor))
        .collect();

    unsafe {
        out.write(binding.store_slice(descriptors));
    }

    Ok(())
}

/// Read one Android bluetooth ATT MTU value.
pub(crate) unsafe fn destack_device_bluetooth_gatt_mtu(
    binding: &BindingCallContext,
    out: *mut u16,
    handle: resource::BluetoothDeviceHandle,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    let resource = device_resource(binding, handle, "destack.device.bluetooth.gatt.mtu")?;
    let runtime_id = host_session_id(binding, "destack.device.bluetooth.gatt.mtu")?;
    let mut mtu = 0u16;
    let status = unsafe {
        destack_host_android_bluetooth_gatt_mtu(runtime_id, resource.session_id, &mut mtu)
    };
    host_status_result(
        status,
        "destack.device.bluetooth.gatt.mtu",
        "bluetooth ATT MTU",
    )?;

    unsafe {
        out.write(mtu);
    }

    Ok(())
}

/// Read one Android bluetooth characteristic value.
pub(crate) unsafe fn destack_device_bluetooth_gatt_read(
    binding: &BindingCallContext,
    out: *mut NativeSlice<u8>,
    handle: resource::BluetoothDeviceHandle,
    characteristicid: NativeStringRef,
    timeoutns: u64,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    let resource = device_resource(binding, handle, "destack.device.bluetooth.gatt.read")?;
    let characteristic_id = unsafe { characteristicid.as_str()? };
    let runtime_id = host_session_id(binding, "destack.device.bluetooth.gatt.read")?;
    let bytes = read_android_bluetooth_bytes(
        binding,
        "destack.device.bluetooth.gatt.read",
        "bluetooth characteristic read",
        |buffer, bytes_written| unsafe {
            destack_host_android_bluetooth_gatt_read(
                runtime_id,
                resource.session_id,
                NativeStringRef::from(characteristic_id),
                timeoutns,
                buffer,
                bytes_written,
            )
        },
    )?;

    unsafe {
        out.write(binding.store_slice(bytes));
    }

    Ok(())
}

/// Read one Android bluetooth descriptor value.
pub(crate) unsafe fn destack_device_bluetooth_gatt_read_descriptor(
    binding: &BindingCallContext,
    out: *mut NativeSlice<u8>,
    handle: resource::BluetoothDeviceHandle,
    descriptorid: NativeStringRef,
    timeoutns: u64,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    let resource = device_resource(
        binding,
        handle,
        "destack.device.bluetooth.gatt.readDescriptor",
    )?;
    let descriptor_id = unsafe { descriptorid.as_str()? };
    let runtime_id = host_session_id(binding, "destack.device.bluetooth.gatt.readDescriptor")?;
    let bytes = read_android_bluetooth_bytes(
        binding,
        "destack.device.bluetooth.gatt.readDescriptor",
        "bluetooth descriptor read",
        |buffer, bytes_written| unsafe {
            destack_host_android_bluetooth_gatt_read_descriptor(
                runtime_id,
                resource.session_id,
                NativeStringRef::from(descriptor_id),
                timeoutns,
                buffer,
                bytes_written,
            )
        },
    )?;

    unsafe {
        out.write(binding.store_slice(bytes));
    }

    Ok(())
}

/// Write one Android bluetooth characteristic value.
pub(crate) unsafe fn destack_device_bluetooth_gatt_write(
    binding: &BindingCallContext,
    handle: resource::BluetoothDeviceHandle,
    characteristicid: NativeStringRef,
    value: NativeSlice<u8>,
    mode: BluetoothGattWriteMode,
    timeoutns: u64,
) -> RuntimeResult<()> {
    let resource = device_resource(binding, handle, "destack.device.bluetooth.gatt.write")?;
    let characteristic_id = unsafe { characteristicid.as_str()? };
    let bytes = unsafe { value.as_slice()? };
    let runtime_id = host_session_id(binding, "destack.device.bluetooth.gatt.write")?;
    let mode_code = match mode {
        BluetoothGattWriteMode::WithResponse => 1,
        BluetoothGattWriteMode::WithoutResponse => 2,
    };
    let status = unsafe {
        destack_host_android_bluetooth_gatt_write(
            runtime_id,
            resource.session_id,
            NativeStringRef::from(characteristic_id),
            mode_code,
            NativeSlice {
                data: bytes.as_ptr() as *mut u8,
                len: bytes.len() as u32,
            },
            timeoutns,
        )
    };

    host_status_result(
        status,
        "destack.device.bluetooth.gatt.write",
        "bluetooth characteristic write",
    )
}

/// Write one Android bluetooth descriptor value.
pub(crate) unsafe fn destack_device_bluetooth_gatt_write_descriptor(
    binding: &BindingCallContext,
    handle: resource::BluetoothDeviceHandle,
    descriptorid: NativeStringRef,
    value: NativeSlice<u8>,
    timeoutns: u64,
) -> RuntimeResult<()> {
    let resource = device_resource(
        binding,
        handle,
        "destack.device.bluetooth.gatt.writeDescriptor",
    )?;
    let descriptor_id = unsafe { descriptorid.as_str()? };
    let bytes = unsafe { value.as_slice()? };
    let runtime_id = host_session_id(binding, "destack.device.bluetooth.gatt.writeDescriptor")?;
    let status = unsafe {
        destack_host_android_bluetooth_gatt_write_descriptor(
            runtime_id,
            resource.session_id,
            NativeStringRef::from(descriptor_id),
            NativeSlice {
                data: bytes.as_ptr() as *mut u8,
                len: bytes.len() as u32,
            },
            timeoutns,
        )
    };

    host_status_result(
        status,
        "destack.device.bluetooth.gatt.writeDescriptor",
        "bluetooth descriptor write",
    )
}

/// Subscribe to one Android bluetooth characteristic value stream.
pub(crate) unsafe fn destack_device_bluetooth_gatt_subscribe(
    binding: &BindingCallContext,
    out: *mut resource::BluetoothSubscriptionHandle,
    handle: resource::BluetoothDeviceHandle,
    characteristicid: NativeStringRef,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    let resource = device_resource(binding, handle, "destack.device.bluetooth.gatt.subscribe")?;
    let characteristic_id = unsafe { characteristicid.as_str()? };
    let (service, characteristic) = characteristic_cache_entry(
        binding,
        &resource,
        characteristic_id,
        "destack.device.bluetooth.gatt.subscribe",
    )?;
    let runtime_id = host_session_id(binding, "destack.device.bluetooth.gatt.subscribe")?;
    let mut subscription_id = 0u64;
    let status = unsafe {
        destack_host_android_bluetooth_gatt_subscribe(
            runtime_id,
            resource.session_id,
            NativeStringRef::from(characteristic_id),
            &mut subscription_id,
        )
    };
    host_status_result(
        status,
        "destack.device.bluetooth.gatt.subscribe",
        "bluetooth subscribe",
    )?;

    let subscription = Arc::new(AndroidBluetoothSubscriptionResource {
        subscription_id,
        service_id: service.id.clone(),
        characteristic_id: characteristic.id.clone(),
        service_uuid: service.uuid.clone(),
        characteristic_uuid: characteristic.uuid.clone(),
    });
    let entry = ResourceEntry::new(ResourceKind::BluetoothSubscription)
        .with_label(BLUETOOTH_SUBSCRIPTION_RESOURCE_LABEL)
        .with_payload(subscription)
        .with_finalizer(binding.worker().platform_state.device.wrap_finalizer(
            AndroidBluetoothSubscriptionFinalizer {
                runtime_id,
                subscription_id,
            },
        ));
    let handle = binding
        .worker()
        .resources
        .insert(binding.world(), entry, Some(binding.engine()));

    unsafe {
        out.write(resource::BluetoothSubscriptionHandle(handle));
    }

    Ok(())
}

/// Unsubscribe from one Android bluetooth characteristic value stream.
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

/// Read one Android bluetooth characteristic value event.
pub(crate) unsafe fn destack_device_bluetooth_gatt_read_event(
    binding: &BindingCallContext,
    out: *mut BluetoothGattValueEvent,
    handle: resource::BluetoothSubscriptionHandle,
    timeoutns: u64,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    let resource =
        subscription_resource(binding, handle, "destack.device.bluetooth.gatt.readEvent")?;
    let runtime_id = host_session_id(binding, "destack.device.bluetooth.gatt.readEvent")?;
    let mut event = NativeSlice {
        data: std::ptr::null_mut(),
        len: 0,
    };
    let mut timestamp_ns = 0u64;
    let status = unsafe {
        destack_host_android_bluetooth_gatt_read_event(
            runtime_id,
            resource.subscription_id,
            timeoutns,
            &mut event,
            &mut timestamp_ns,
        )
    };
    host_status_result(
        status,
        "destack.device.bluetooth.gatt.readEvent",
        "bluetooth value event read",
    )?;
    let value = unsafe { event.as_slice()? }.to_vec();
    let value = gatt_value_event(
        timestamp_ns,
        resource.service_id.clone(),
        resource.characteristic_id.clone(),
        resource.service_uuid.clone(),
        resource.characteristic_uuid.clone(),
        value,
    );

    unsafe {
        out.write(stored_gatt_value_event(binding, value));
    }

    Ok(())
}

/// Poll one Android bluetooth characteristic value event without blocking.
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
    let runtime_id = host_session_id(binding, "destack.device.bluetooth.gatt.tryReadEvent")?;
    let mut event = NativeSlice {
        data: std::ptr::null_mut(),
        len: 0,
    };
    let mut timestamp_ns = 0u64;
    let status = unsafe {
        destack_host_android_bluetooth_gatt_try_read_event(
            runtime_id,
            resource.subscription_id,
            &mut event,
            &mut timestamp_ns,
        )
    };
    host_status_result(
        status,
        "destack.device.bluetooth.gatt.tryReadEvent",
        "bluetooth value event try-read",
    )?;
    let value = unsafe { event.as_slice()? }.to_vec();
    let value = gatt_value_event(
        timestamp_ns,
        resource.service_id.clone(),
        resource.characteristic_id.clone(),
        resource.service_uuid.clone(),
        resource.characteristic_uuid.clone(),
        value,
    );

    unsafe {
        out.write(stored_gatt_value_event(binding, value));
    }

    Ok(())
}
