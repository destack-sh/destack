use super::common::*;
use super::*;

/// Read one Android Bluetooth adapter list.
pub(crate) fn read_adapter_descriptors(
    binding: &BindingCallContext,
    operation: &'static str,
) -> RuntimeResult<Vec<BluetoothAdapterDescriptorValue>> {
    let runtime_id = host_runtime_id(binding, operation)?;
    let mut headers = vec![
        AndroidHostBluetoothAdapterDescriptorHeader::default();
        INITIAL_BLUETOOTH_ROW_CAPACITY
    ];
    let mut string_bytes = vec![0u8; INITIAL_BLUETOOTH_STRING_CAPACITY];

    loop {
        let header_capacity = checked_u32_length(headers.len(), operation, "adapter rows")?;
        let string_capacity = checked_u32_length(string_bytes.len(), operation, "string bytes")?;
        let mut header_count_written = 0u32;
        let mut string_bytes_written = 0u32;

        let status = unsafe {
            destack_host_android_bluetooth_adapter_list(
                runtime_id,
                NativeSlice {
                    data: headers.as_mut_ptr(),
                    len: header_capacity,
                },
                &mut header_count_written,
                NativeSlice {
                    data: string_bytes.as_mut_ptr(),
                    len: string_capacity,
                },
                &mut string_bytes_written,
            )
        };

        // grow the buffers when the host reports truncation
        if status == HostStatus::BufferTooSmall.code() {
            grow_row_and_string_buffers(
                &mut headers,
                header_count_written as usize,
                &mut string_bytes,
                string_bytes_written as usize,
                operation,
            )?;
            continue;
        }

        host_status_result(status, operation, "bluetooth adapter list")?;

        let header_count = header_count_written as usize;
        let string_count = string_bytes_written as usize;
        headers.truncate(header_count);
        string_bytes.truncate(string_count);

        let mut adapters = Vec::with_capacity(headers.len());

        // decode each adapter row
        for header in &headers {
            adapters.push(decode_adapter_descriptor(header, &string_bytes, operation)?);
        }

        return Ok(adapters);
    }
}

/// Read one Android Bluetooth service list and refresh the cache.
fn read_services(
    binding: &BindingCallContext,
    session_id: u64,
    operation: &'static str,
) -> RuntimeResult<Vec<BluetoothGattServiceValue>> {
    let runtime_id = host_runtime_id(binding, operation)?;
    let mut headers =
        vec![AndroidHostBluetoothGattServiceHeader::default(); INITIAL_BLUETOOTH_ROW_CAPACITY];
    let mut string_bytes = vec![0u8; INITIAL_BLUETOOTH_STRING_CAPACITY];

    loop {
        let header_capacity = checked_u32_length(headers.len(), operation, "service rows")?;
        let string_capacity = checked_u32_length(string_bytes.len(), operation, "string bytes")?;
        let mut header_count_written = 0u32;
        let mut string_bytes_written = 0u32;

        let status = unsafe {
            destack_host_android_bluetooth_gatt_service_list(
                runtime_id,
                session_id,
                NativeSlice {
                    data: headers.as_mut_ptr(),
                    len: header_capacity,
                },
                &mut header_count_written,
                NativeSlice {
                    data: string_bytes.as_mut_ptr(),
                    len: string_capacity,
                },
                &mut string_bytes_written,
            )
        };

        // grow the buffers when the host reports truncation
        if status == HostStatus::BufferTooSmall.code() {
            grow_row_and_string_buffers(
                &mut headers,
                header_count_written as usize,
                &mut string_bytes,
                string_bytes_written as usize,
                operation,
            )?;
            continue;
        }

        host_status_result(status, operation, "bluetooth gatt service list")?;

        let header_count = header_count_written as usize;
        let string_count = string_bytes_written as usize;
        headers.truncate(header_count);
        string_bytes.truncate(string_count);

        let mut services = Vec::with_capacity(headers.len());

        // decode each service row
        for header in &headers {
            services.push(decode_service_descriptor(header, &string_bytes, operation)?);
        }

        return Ok(services);
    }
}

/// Read one Android Bluetooth characteristic list and refresh the cache.
pub(crate) fn read_characteristics(
    binding: &BindingCallContext,
    session_id: u64,
    service_id: &str,
    operation: &'static str,
) -> RuntimeResult<Vec<BluetoothGattCharacteristicValue>> {
    let runtime_id = host_runtime_id(binding, operation)?;
    let mut headers = vec![
        AndroidHostBluetoothGattCharacteristicHeader::default();
        INITIAL_BLUETOOTH_ROW_CAPACITY
    ];
    let mut string_bytes = vec![0u8; INITIAL_BLUETOOTH_STRING_CAPACITY];

    loop {
        let header_capacity = checked_u32_length(headers.len(), operation, "characteristic rows")?;
        let string_capacity = checked_u32_length(string_bytes.len(), operation, "string bytes")?;
        let mut header_count_written = 0u32;
        let mut string_bytes_written = 0u32;

        let status = unsafe {
            destack_host_android_bluetooth_gatt_characteristic_list(
                runtime_id,
                session_id,
                NativeStringRef::from(service_id),
                NativeSlice {
                    data: headers.as_mut_ptr(),
                    len: header_capacity,
                },
                &mut header_count_written,
                NativeSlice {
                    data: string_bytes.as_mut_ptr(),
                    len: string_capacity,
                },
                &mut string_bytes_written,
            )
        };

        // grow the buffers when the host reports truncation
        if status == HostStatus::BufferTooSmall.code() {
            grow_row_and_string_buffers(
                &mut headers,
                header_count_written as usize,
                &mut string_bytes,
                string_bytes_written as usize,
                operation,
            )?;
            continue;
        }

        host_status_result(status, operation, "bluetooth gatt characteristic list")?;

        let header_count = header_count_written as usize;
        let string_count = string_bytes_written as usize;
        headers.truncate(header_count);
        string_bytes.truncate(string_count);

        let mut characteristics = Vec::with_capacity(headers.len());

        // decode each characteristic row
        for header in &headers {
            characteristics.push(decode_characteristic_descriptor(
                header,
                &string_bytes,
                operation,
            )?);
        }

        return Ok(characteristics);
    }
}

/// Read one Android Bluetooth descriptor list and refresh the cache.
pub(crate) fn read_descriptors(
    binding: &BindingCallContext,
    session_id: u64,
    characteristic_id: &str,
    cache: &AndroidBluetoothGattCache,
    operation: &'static str,
) -> RuntimeResult<Vec<BluetoothGattDescriptorValue>> {
    let runtime_id = host_runtime_id(binding, operation)?;
    let mut headers =
        vec![AndroidHostBluetoothGattDescriptorHeader::default(); INITIAL_BLUETOOTH_ROW_CAPACITY];
    let mut string_bytes = vec![0u8; INITIAL_BLUETOOTH_STRING_CAPACITY];

    loop {
        let header_capacity = checked_u32_length(headers.len(), operation, "descriptor rows")?;
        let string_capacity = checked_u32_length(string_bytes.len(), operation, "string bytes")?;
        let mut header_count_written = 0u32;
        let mut string_bytes_written = 0u32;

        let status = unsafe {
            destack_host_android_bluetooth_gatt_descriptor_list(
                runtime_id,
                session_id,
                NativeStringRef::from(characteristic_id),
                NativeSlice {
                    data: headers.as_mut_ptr(),
                    len: header_capacity,
                },
                &mut header_count_written,
                NativeSlice {
                    data: string_bytes.as_mut_ptr(),
                    len: string_capacity,
                },
                &mut string_bytes_written,
            )
        };

        // grow the buffers when the host reports truncation
        if status == HostStatus::BufferTooSmall.code() {
            grow_row_and_string_buffers(
                &mut headers,
                header_count_written as usize,
                &mut string_bytes,
                string_bytes_written as usize,
                operation,
            )?;
            continue;
        }

        host_status_result(status, operation, "bluetooth gatt descriptor list")?;

        let header_count = header_count_written as usize;
        let string_count = string_bytes_written as usize;
        headers.truncate(header_count);
        string_bytes.truncate(string_count);

        let mut descriptors = Vec::with_capacity(headers.len());

        // decode each descriptor row
        for header in &headers {
            descriptors.push(decode_descriptor_descriptor(
                header,
                &string_bytes,
                cache,
                operation,
            )?);
        }

        return Ok(descriptors);
    }
}

/// Refresh one Android Bluetooth cache from the host.
pub(crate) fn refresh_gatt_cache(
    binding: &BindingCallContext,
    resource: &AndroidBluetoothDeviceResource,
    operation: &'static str,
) -> RuntimeResult<AndroidBluetoothGattCache> {
    let services = read_services(binding, resource.session_id, operation)?;
    let mut cache = AndroidBluetoothGattCache::default();

    // refresh services
    for service in services {
        cache.services.insert(service.id.clone(), service);
    }

    // refresh characteristics
    for service_id in cache.services.keys().cloned().collect::<Vec<_>>() {
        let characteristics =
            read_characteristics(binding, resource.session_id, &service_id, operation)?;
        for characteristic in characteristics {
            cache
                .characteristics
                .insert(characteristic.id.clone(), characteristic);
        }
    }

    // refresh descriptors
    for characteristic_id in cache.characteristics.keys().cloned().collect::<Vec<_>>() {
        let descriptors = read_descriptors(
            binding,
            resource.session_id,
            &characteristic_id,
            &cache,
            operation,
        )?;
        for descriptor in descriptors {
            cache.descriptors.insert(descriptor.id.clone(), descriptor);
        }
    }

    Ok(cache)
}

/// Ensure one characteristic cache entry exists.
pub(crate) fn characteristic_cache_entry(
    binding: &BindingCallContext,
    resource: &AndroidBluetoothDeviceResource,
    characteristic_id: &str,
    operation: &'static str,
) -> RuntimeResult<(BluetoothGattCharacteristicValue, BluetoothGattServiceValue)> {
    {
        let cache = resource.cache.lock();
        if let Some(characteristic) = cache.characteristics.get(characteristic_id).cloned() {
            let service = cache
                .services
                .get(&characteristic.service_id)
                .cloned()
                .ok_or_else(|| {
                    invalid_data(
                        operation,
                        "android bluetooth cache was missing one parent service",
                    )
                })?;
            return Ok((characteristic, service));
        }
    }

    let cache = refresh_gatt_cache(binding, resource, operation)?;
    let characteristic = cache
        .characteristics
        .get(characteristic_id)
        .cloned()
        .ok_or_else(|| {
            core_platform::io_not_found(
                operation,
                format!("bluetooth characteristic {characteristic_id} not found"),
            )
        })?;
    let service = cache
        .services
        .get(&characteristic.service_id)
        .cloned()
        .ok_or_else(|| {
            invalid_data(
                operation,
                "android bluetooth cache was missing one refreshed parent service",
            )
        })?;

    *resource.cache.lock() = cache;

    Ok((characteristic, service))
}
