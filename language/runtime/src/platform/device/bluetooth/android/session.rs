use super::codec::*;

/// Resolve one Android bluetooth device resource.
pub(super) fn device_resource(
    binding: &BindingCallContext,
    handle: resource::BluetoothDeviceHandle,
    operation: &'static str,
) -> RuntimeResult<Arc<AndroidBluetoothDeviceResource>> {
    bluetooth_payload(
        binding,
        handle.0,
        ResourceKind::BluetoothDevice,
        operation,
        "device",
    )
}

/// Open one Android bluetooth device session.
pub(crate) unsafe fn destack_device_bluetooth_open(
    binding: &BindingCallContext,
    out: *mut resource::BluetoothDeviceHandle,
    adapterid: NativeStringRef,
    deviceid: NativeStringRef,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    // decode the selected adapter and device ids
    let adapter_id = unsafe { adapterid.as_str()? };
    let device_id = unsafe { deviceid.as_str()? };
    ensure_android_primary_adapter(binding, adapter_id, "destack.device.bluetooth.session.open")?;

    // open the host device session
    let runtime_id = host_session_id(binding, "destack.device.bluetooth.session.open")?;
    let mut descriptor = AndroidHostBluetoothDeviceDescriptorHeader::default();
    let mut string_bytes = vec![0u8; INITIAL_BLUETOOTH_STRING_CAPACITY];
    let mut session_id = 0u64;

    let descriptor = loop {
        let string_capacity = checked_u32_length(
            string_bytes.len(),
            "destack.device.bluetooth.session.open",
            "string bytes",
        )?;
        let mut string_bytes_written = 0u32;
        let status = unsafe {
            destack_host_android_bluetooth_open(
                runtime_id,
                NativeStringRef::from(adapter_id),
                NativeStringRef::from(device_id),
                &mut session_id,
                &mut descriptor,
                NativeSlice {
                    data: string_bytes.as_mut_ptr(),
                    len: string_capacity,
                },
                &mut string_bytes_written,
            )
        };

        if status == HostStatus::BufferTooSmall.code() {
            let next_capacity = string_bytes
                .len()
                .max(string_bytes_written as usize)
                .saturating_mul(2);
            if next_capacity > MAX_BLUETOOTH_STRING_CAPACITY {
                return Err(invalid_data(
                    "destack.device.bluetooth.session.open",
                    "android host bluetooth open exceeded the maximum string payload size",
                ));
            }

            string_bytes.resize(next_capacity, 0);
            continue;
        }

        host_status_result(
            status,
            "destack.device.bluetooth.session.open",
            "bluetooth device open",
        )?;
        string_bytes.truncate(string_bytes_written as usize);
        let descriptor = decode_device_descriptor(
            &descriptor,
            &string_bytes,
            "destack.device.bluetooth.session.open",
        )?;
        break descriptor;
    };

    // store the opened device resource
    let resource = Arc::new(AndroidBluetoothDeviceResource {
        session_id,
        descriptor: Arc::new(Mutex::new(descriptor)),
        event_state: Arc::new(Mutex::new(BluetoothEventState { next_sequence: 1 })),
        cache: Arc::new(Mutex::new(AndroidBluetoothGattCache::default())),
    });
    let entry = ResourceEntry::new(ResourceKind::BluetoothDevice)
        .with_label(BLUETOOTH_DEVICE_RESOURCE_LABEL)
        .with_payload(resource)
        .with_finalizer(binding.worker().platform_state.device.wrap_finalizer(
            AndroidBluetoothDeviceFinalizer {
                runtime_id,
                session_id,
            },
        ));
    let handle = binding
        .worker()
        .resources
        .insert(binding.world(), entry, Some(binding.engine()));

    unsafe {
        out.write(resource::BluetoothDeviceHandle(handle));
    }

    Ok(())
}

/// Read one Android bluetooth device descriptor snapshot.
pub(crate) unsafe fn destack_device_bluetooth_descriptor(
    binding: &BindingCallContext,
    out: *mut BluetoothDeviceDescriptor,
    handle: resource::BluetoothDeviceHandle,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    // resolve the active device session
    let resource = device_resource(binding, handle, "destack.device.bluetooth.descriptor")?;
    let descriptor = resource.descriptor.lock().clone();

    // store the latest descriptor snapshot
    unsafe {
        out.write(BluetoothDeviceDescriptor::from_value(binding, descriptor));
    }

    Ok(())
}

/// Close one Android bluetooth device session.
pub(crate) unsafe fn destack_device_bluetooth_close(
    binding: &BindingCallContext,
    handle: resource::BluetoothDeviceHandle,
) -> RuntimeResult<()> {
    close_bluetooth_resource(
        binding,
        handle.0,
        ResourceKind::BluetoothDevice,
        "destack.device.bluetooth.session.close",
        "device",
    )
}

/// Pair one Android bluetooth device.
pub(crate) unsafe fn destack_device_bluetooth_pair(
    binding: &BindingCallContext,
    handle: resource::BluetoothDeviceHandle,
    timeoutns: u64,
) -> RuntimeResult<()> {
    let resource = device_resource(binding, handle, "destack.device.bluetooth.session.pair")?;
    let runtime_id = host_session_id(binding, "destack.device.bluetooth.session.pair")?;
    let status =
        unsafe { destack_host_android_bluetooth_pair(runtime_id, resource.session_id, timeoutns) };

    host_status_result(
        status,
        "destack.device.bluetooth.session.pair",
        "bluetooth pair",
    )?;

    // reflect the updated pair state in the session snapshot
    let mut descriptor = resource.descriptor.lock();
    descriptor.pair_state = Some(BluetoothPairState::Paired);

    Ok(())
}

/// Unpair one Android bluetooth device.
pub(crate) unsafe fn destack_device_bluetooth_unpair(
    binding: &BindingCallContext,
    adapterid: NativeStringRef,
    deviceid: NativeStringRef,
) -> RuntimeResult<()> {
    let adapter_id = unsafe { adapterid.as_str()? };
    let device_id = unsafe { deviceid.as_str()? };
    ensure_android_primary_adapter(
        binding,
        adapter_id,
        "destack.device.bluetooth.session.unpair",
    )?;
    let runtime_id = host_session_id(binding, "destack.device.bluetooth.session.unpair")?;
    let status = unsafe {
        destack_host_android_bluetooth_unpair(
            runtime_id,
            NativeStringRef::from(adapter_id),
            NativeStringRef::from(device_id),
        )
    };

    host_status_result(
        status,
        "destack.device.bluetooth.session.unpair",
        "bluetooth unpair",
    )?;

    Ok(())
}

/// Read one Android bluetooth device RSSI value.
pub(crate) unsafe fn destack_device_bluetooth_read_rssi(
    binding: &BindingCallContext,
    out: *mut i32,
    handle: resource::BluetoothDeviceHandle,
    timeoutns: u64,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    let resource = device_resource(binding, handle, "destack.device.bluetooth.session.rssi")?;
    let runtime_id = host_session_id(binding, "destack.device.bluetooth.session.rssi")?;
    let mut rssi_dbm = 0i16;
    let status = unsafe {
        destack_host_android_bluetooth_read_rssi(
            runtime_id,
            resource.session_id,
            timeoutns,
            &mut rssi_dbm,
        )
    };
    host_status_result(
        status,
        "destack.device.bluetooth.session.rssi",
        "bluetooth RSSI read",
    )?;

    // reflect the latest RSSI in the session snapshot
    let mut descriptor = resource.descriptor.lock();
    descriptor.rssi = Some(i32::from(rssi_dbm));

    unsafe {
        out.write(i32::from(rssi_dbm));
    }

    Ok(())
}

/// Read one Android bluetooth session event.
pub(crate) unsafe fn destack_device_bluetooth_session_read_event(
    binding: &BindingCallContext,
    out: *mut BluetoothSessionEvent,
    handle: resource::BluetoothDeviceHandle,
    timeoutns: u64,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    let resource = device_resource(
        binding,
        handle,
        "destack.device.bluetooth.session.readEvent",
    )?;
    let header = read_session_event_from_host(
        binding,
        resource.session_id,
        Some(timeoutns),
        false,
        "destack.device.bluetooth.session.readEvent",
    )?;

    let value = match header.kind {
        BLUETOOTH_SESSION_EVENT_DISCONNECTED => disconnected_event(&resource.event_state),
        BLUETOOTH_SESSION_EVENT_PAIR_STATE_CHANGED => {
            let pair_state = decode_optional_pair_state(
                header.pair_state,
                "destack.device.bluetooth.session.readEvent",
            )?
            .ok_or_else(|| {
                invalid_data(
                    "destack.device.bluetooth.session.readEvent",
                    "android host returned one pair-state event without one pair state",
                )
            })?;
            pair_state_changed_event(&resource.event_state, pair_state)
        }
        BLUETOOTH_SESSION_EVENT_GATT_DATABASE_CHANGED => {
            gatt_database_changed_event(&resource.event_state)
        }
        kind => {
            return Err(invalid_data(
                "destack.device.bluetooth.session.readEvent",
                format!("android host returned one unknown bluetooth session event kind {kind}"),
            ));
        }
    };

    unsafe {
        out.write(stored_session_event(binding, value));
    }

    Ok(())
}

/// Poll one Android bluetooth session event without blocking.
pub(crate) unsafe fn destack_device_bluetooth_session_try_read_event(
    binding: &BindingCallContext,
    out: *mut BluetoothSessionEvent,
    handle: resource::BluetoothDeviceHandle,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    let resource = device_resource(
        binding,
        handle,
        "destack.device.bluetooth.session.tryReadEvent",
    )?;
    let header = read_session_event_from_host(
        binding,
        resource.session_id,
        None,
        true,
        "destack.device.bluetooth.session.tryReadEvent",
    )?;

    let value = match header.kind {
        BLUETOOTH_SESSION_EVENT_DISCONNECTED => disconnected_event(&resource.event_state),
        BLUETOOTH_SESSION_EVENT_PAIR_STATE_CHANGED => {
            let pair_state = decode_optional_pair_state(
                header.pair_state,
                "destack.device.bluetooth.session.tryReadEvent",
            )?
            .ok_or_else(|| {
                invalid_data(
                    "destack.device.bluetooth.session.tryReadEvent",
                    "android host returned one pair-state event without one pair state",
                )
            })?;
            pair_state_changed_event(&resource.event_state, pair_state)
        }
        BLUETOOTH_SESSION_EVENT_GATT_DATABASE_CHANGED => {
            gatt_database_changed_event(&resource.event_state)
        }
        kind => {
            return Err(invalid_data(
                "destack.device.bluetooth.session.tryReadEvent",
                format!("android host returned one unknown bluetooth session event kind {kind}"),
            ));
        }
    };

    unsafe {
        out.write(stored_session_event(binding, value));
    }

    Ok(())
}
