use super::core::*;
use super::metadata::*;
use super::watch::*;

/// Open one Linux bluetooth device session.
pub(crate) unsafe fn destack_device_bluetooth_open(
    binding: &BindingCallContext,
    out: *mut resource::BluetoothDeviceHandle,
    adapterid: NativeStringRef,
    deviceid: NativeStringRef,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    // resolve the selected adapter and device
    let service = binding
        .worker()
        .platform_state
        .device
        .linux_bluetooth_service("destack.device.bluetooth.session.open")?;
    let connection = service.connection();
    let adapter_id = unsafe { adapterid.as_str()? };
    let adapter_path = parse_adapter_path(
        &connection,
        adapter_id,
        "destack.device.bluetooth.session.open",
    )?;
    let device_id = unsafe { deviceid.as_str()? };
    let device_path = parse_device_path(
        &connection,
        adapter_path.as_str(),
        device_id,
        "destack.device.bluetooth.session.open",
    )?;

    // connect and prime the GATT cache
    service.retain_device_connection(device_path.as_str())?;
    let opened_session = (|| {
        let current_descriptor = current_device_descriptor(
            &connection,
            device_path.as_str(),
            "destack.device.bluetooth.session.open",
        )?
        .ok_or_else(|| {
            core_platform::io_not_found(
                "destack.device.bluetooth.session.open",
                "unknown bluetooth device",
            )
        })?;
        let objects = bluez_managed_objects(&connection, "destack.device.bluetooth.session.open")?;
        let cache = gatt_cache(
            device_path.as_str(),
            &objects,
            "destack.device.bluetooth.session.open",
        )?;
        let event_queue = Arc::new(BoundedQueue::new(BLUETOOTH_SESSION_EVENT_QUEUE_CAPACITY));
        let resource = Arc::new(LinuxBluetoothDeviceResource {
            service: service.clone(),
            device_path: device_path.as_str().to_string(),
            event_queue: event_queue.clone(),
            event_state: Mutex::new(BluetoothEventState { next_sequence: 1 }),
            cache: Mutex::new(cache.clone()),
        });
        let registration_id =
            service.register_device(&resource, current_descriptor.pair_state, cache);

        Ok::<_, Box<RuntimeError>>((event_queue, resource, registration_id))
    })();
    let (event_queue, resource, registration_id) = match opened_session {
        Ok(opened_session) => opened_session,
        Err(error) => {
            service.release_device_connection(device_path.as_str());

            return Err(error);
        }
    };

    // store the opened session
    let entry = ResourceEntry::new(ResourceKind::BluetoothDevice)
        .with_label(BLUETOOTH_DEVICE_RESOURCE_LABEL)
        .with_payload(resource.clone())
        .with_finalizer(binding.worker().platform_state.device.wrap_finalizer(
            LinuxBluetoothDeviceFinalizer {
                service,
                registration_id,
                device_path: resource.device_path.clone(),
                event_queue,
            },
        ));
    let handle = binding
        .worker()
        .resources
        .insert(&binding.world(), entry, Some(binding.engine()));

    unsafe {
        out.write(resource::BluetoothDeviceHandle(handle));
    }

    Ok(())
}

/// Close one Linux bluetooth device session.
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

/// Read one Linux bluetooth device descriptor snapshot.
pub(crate) unsafe fn destack_device_bluetooth_descriptor(
    binding: &BindingCallContext,
    out: *mut BluetoothDeviceDescriptor,
    handle: resource::BluetoothDeviceHandle,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    // resolve the active device session
    let resource = device_resource(binding, handle, "destack.device.bluetooth.descriptor")?;
    let connection = resource.service.connection();
    let descriptor = current_device_descriptor(
        &connection,
        &resource.device_path,
        "destack.device.bluetooth.descriptor",
    )?
    .ok_or_else(|| {
        core_platform::io_not_found(
            "destack.device.bluetooth.descriptor",
            "unknown bluetooth device",
        )
    })?;

    // store the current descriptor snapshot
    unsafe {
        out.write(BluetoothDeviceDescriptor::from_value(binding, descriptor));
    }

    Ok(())
}

/// Pair one Linux bluetooth device session.
pub(crate) unsafe fn destack_device_bluetooth_pair(
    binding: &BindingCallContext,
    handle: resource::BluetoothDeviceHandle,
    _timeoutns: u64,
) -> RuntimeResult<()> {
    let resource = device_resource(binding, handle, "destack.device.bluetooth.session.pair")?;
    let connection = resource.service.connection();

    pair_device(&connection, &resource.device_path)
}

/// Remove one Linux bluetooth bond.
pub(crate) unsafe fn destack_device_bluetooth_unpair(
    binding: &BindingCallContext,
    adapterid: NativeStringRef,
    deviceid: NativeStringRef,
) -> RuntimeResult<()> {
    let service = binding
        .worker()
        .platform_state
        .device
        .linux_bluetooth_service("destack.device.bluetooth.session.unpair")?;
    let connection = service.connection();
    let adapter_id = unsafe { adapterid.as_str()? };
    let adapter_path = parse_adapter_path(
        &connection,
        adapter_id,
        "destack.device.bluetooth.session.unpair",
    )?;
    let device_id = unsafe { deviceid.as_str()? };
    let device_path = parse_device_path(
        &connection,
        adapter_path.as_str(),
        device_id,
        "destack.device.bluetooth.session.unpair",
    )?;

    remove_device(&connection, adapter_path.as_str(), device_path.as_str())
}

/// Read one Linux bluetooth RSSI value.
pub(crate) unsafe fn destack_device_bluetooth_read_rssi(
    binding: &BindingCallContext,
    out: *mut i32,
    handle: resource::BluetoothDeviceHandle,
    _timeoutns: u64,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    let resource = device_resource(binding, handle, "destack.device.bluetooth.session.rssi")?;
    let connection = resource.service.connection();
    let descriptor = current_device_descriptor(
        &connection,
        &resource.device_path,
        "destack.device.bluetooth.session.rssi",
    )?
    .ok_or_else(|| {
        core_platform::io_not_found(
            "destack.device.bluetooth.session.rssi",
            "unknown bluetooth device",
        )
    })?;
    let rssi = descriptor.rssi.ok_or_else(|| {
        core_platform::io_not_found(
            "destack.device.bluetooth.session.rssi",
            "bluetooth RSSI is not available for this device session",
        )
    })?;

    unsafe {
        out.write(rssi);
    }

    Ok(())
}

/// Read one blocking Linux bluetooth session event.
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
    let event = pop_session_event(
        binding,
        &resource,
        timeoutns,
        "destack.device.bluetooth.session.readEvent",
    )?;

    unsafe {
        out.write(event);
    }

    Ok(())
}

/// Poll one Linux bluetooth session event without blocking.
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
    let value = resource.event_queue.try_pop_or_else(|| {
        Err(core_platform::io_would_block(
            "destack.device.bluetooth.session.tryReadEvent",
            "bluetooth session event queue is empty",
        ))
    })?;

    unsafe {
        out.write(stored_session_event(binding, value));
    }

    Ok(())
}
