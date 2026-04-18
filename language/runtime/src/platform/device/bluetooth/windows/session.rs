use super::core::*;
use super::metadata::*;

/// Resolve one Windows bluetooth device resource.
pub(super) fn device_resource(
    binding: &BindingCallContext,
    handle: resource::BluetoothDeviceHandle,
    operation: &'static str,
) -> DiagnosticResult<Arc<WindowsBluetoothDeviceResource>> {
    bluetooth_payload(
        binding,
        handle.0,
        ResourceKind::BluetoothDevice,
        operation,
        "device",
    )
}

/// Open one Windows bluetooth device session.
pub(crate) unsafe fn destack_device_bluetooth_open(
    binding: &BindingCallContext,
    out: *mut resource::BluetoothDeviceHandle,
    adapterid: NativeStringRef,
    deviceid: NativeStringRef,
) -> DiagnosticResult<()> {
    core_platform::ensure_out(out, "out")?;

    // validate the selected adapter and parse the stable device id
    let adapter_id = unsafe { adapterid.as_str()? };
    let adapters = enumerate_adapters("destack.device.bluetooth.session.open")?;
    if !adapters
        .iter()
        .any(|(descriptor, _)| descriptor.id == adapter_id)
    {
        return Err(core_platform::io_not_found(
            "destack.device.bluetooth.session.open",
            format!("bluetooth adapter {adapter_id} not found"),
        ));
    }
    let device_id = unsafe { deviceid.as_str()? };
    let (address, address_type) =
        parse_stable_device_id(device_id, "destack.device.bluetooth.session.open")?;

    // open the LE device and prime the GATT cache
    let device =
        BluetoothLEDevice::FromBluetoothAddressWithBluetoothAddressTypeAsync(address, address_type)
            .map_err(|error| {
                windows_bluetooth_error(
                    "destack.device.bluetooth.session.open",
                    "BluetoothLEDevice::FromBluetoothAddressWithBluetoothAddressTypeAsync",
                    &error,
                )
            })?
            .get()
            .map_err(|error| {
                windows_bluetooth_error(
                    "destack.device.bluetooth.session.open",
                    "IAsyncOperation::get",
                    &error,
                )
            })?;
    let cache = Arc::new(Mutex::new(refresh_gatt_cache(
        &device,
        "destack.device.bluetooth.session.open",
    )?));

    // build the session event queue
    let event_queue = Arc::new(BoundedQueue::new(BLUETOOTH_SESSION_EVENT_QUEUE_CAPACITY));
    let event_state = Arc::new(Mutex::new(BluetoothEventState { next_sequence: 1 }));

    // connection status delivery
    let callback_queue = event_queue.clone();
    let callback_state = event_state.clone();
    let callback_device = device.clone();
    let connection_status_token = device
        .ConnectionStatusChanged(&TypedEventHandler::new(
            move |_device: Ref<'_, BluetoothLEDevice>, _args| {
                let is_connected = callback_device
                    .ConnectionStatus()
                    .map(|status| status == BluetoothConnectionStatus::Connected)
                    .unwrap_or(false);
                if !is_connected {
                    callback_queue.push_drop_oldest(disconnected_event(&callback_state));
                }

                Ok(())
            },
        ))
        .map_err(|error| {
            windows_bluetooth_error(
                "destack.device.bluetooth.session.open",
                "BluetoothLEDevice::ConnectionStatusChanged",
                &error,
            )
        })?;

    // GATT database delivery
    let changed_queue = event_queue.clone();
    let changed_state = event_state.clone();
    let is_cache_dirty = Arc::new(AtomicBool::new(false));
    let callback_is_cache_dirty = is_cache_dirty.clone();
    let gatt_services_changed_token = device
        .GattServicesChanged(&TypedEventHandler::new(move |_device, _args| {
            // mark the cached graph stale before publishing the change event
            callback_is_cache_dirty.store(true, Ordering::Release);
            changed_queue.push_drop_oldest(gatt_database_changed_event(&changed_state));
            Ok(())
        }))
        .map_err(|error| {
            windows_bluetooth_error(
                "destack.device.bluetooth.session.open",
                "BluetoothLEDevice::GattServicesChanged",
                &error,
            )
        })?;

    // store the opened session resource
    let resource = Arc::new(WindowsBluetoothDeviceResource {
        device: device.clone(),
        connection_status_token,
        gatt_services_changed_token,
        event_queue: event_queue.clone(),
        event_state: event_state.clone(),
        cache: cache.clone(),
        is_cache_dirty: is_cache_dirty.clone(),
    });
    let entry = ResourceEntry::new(ResourceKind::BluetoothDevice)
        .with_label(BLUETOOTH_DEVICE_RESOURCE_LABEL)
        .with_payload(resource)
        .with_finalizer(binding.worker().platform_state.device.wrap_finalizer(
            WindowsBluetoothDeviceResource {
                device,
                connection_status_token,
                gatt_services_changed_token,
                event_queue,
                event_state,
                cache,
                is_cache_dirty,
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

/// Close one Windows bluetooth device session.
pub(crate) unsafe fn destack_device_bluetooth_close(
    binding: &BindingCallContext,
    handle: resource::BluetoothDeviceHandle,
) -> DiagnosticResult<()> {
    close_bluetooth_resource(
        binding,
        handle.0,
        ResourceKind::BluetoothDevice,
        "destack.device.bluetooth.session.close",
        "device",
    )
}

/// Read one Windows bluetooth device descriptor snapshot.
pub(crate) unsafe fn destack_device_bluetooth_descriptor(
    binding: &BindingCallContext,
    out: *mut BluetoothDeviceDescriptor,
    handle: resource::BluetoothDeviceHandle,
) -> DiagnosticResult<()> {
    core_platform::ensure_out(out, "out")?;

    // resolve the active device session
    let resource = device_resource(binding, handle, "destack.device.bluetooth.descriptor")?;
    let descriptor =
        opened_device_descriptor(&resource.device, "destack.device.bluetooth.descriptor")?;

    // store the current descriptor snapshot
    unsafe {
        out.write(BluetoothDeviceDescriptor::from_value(binding, descriptor));
    }

    Ok(())
}

/// Pair one Windows bluetooth device.
pub(crate) unsafe fn destack_device_bluetooth_pair(
    binding: &BindingCallContext,
    handle: resource::BluetoothDeviceHandle,
    _timeoutns: u64,
) -> DiagnosticResult<()> {
    let resource = device_resource(binding, handle, "destack.device.bluetooth.session.pair")?;
    let device_information = resource.device.DeviceInformation().map_err(|error| {
        windows_bluetooth_error(
            "destack.device.bluetooth.session.pair",
            "BluetoothLEDevice::DeviceInformation",
            &error,
        )
    })?;
    let pairing = device_information.Pairing().map_err(|error| {
        windows_bluetooth_error(
            "destack.device.bluetooth.session.pair",
            "DeviceInformation::Pairing",
            &error,
        )
    })?;
    let result = pairing
        .PairAsync()
        .map_err(|error| {
            windows_bluetooth_error(
                "destack.device.bluetooth.session.pair",
                "DeviceInformationPairing::PairAsync",
                &error,
            )
        })?
        .get()
        .map_err(|error| {
            windows_bluetooth_error(
                "destack.device.bluetooth.session.pair",
                "IAsyncOperation::get",
                &error,
            )
        })?;
    let status = result.Status().map_err(|error| {
        windows_bluetooth_error(
            "destack.device.bluetooth.session.pair",
            "DevicePairingResult::Status",
            &error,
        )
    })?;
    if status != DevicePairingResultStatus::Paired
        && status != DevicePairingResultStatus::AlreadyPaired
    {
        return Err(core_platform::io_operation_error(
            "destack.device.bluetooth.session.pair",
            None,
            format!("bluetooth pair failed with status {}", status.0),
        ));
    }

    resource
        .event_queue
        .push_drop_oldest(pair_state_changed_event(
            &resource.event_state,
            BluetoothPairState::Paired,
        ));

    Ok(())
}

/// Unpair one Windows bluetooth device.
pub(crate) unsafe fn destack_device_bluetooth_unpair(
    _binding: &BindingCallContext,
    adapterid: NativeStringRef,
    deviceid: NativeStringRef,
) -> DiagnosticResult<()> {
    let adapter_id = unsafe { adapterid.as_str()? };
    let adapters = enumerate_adapters("destack.device.bluetooth.session.unpair")?;
    if !adapters
        .iter()
        .any(|(descriptor, _)| descriptor.id == adapter_id)
    {
        return Err(core_platform::io_not_found(
            "destack.device.bluetooth.session.unpair",
            format!("bluetooth adapter {adapter_id} not found"),
        ));
    }
    let device_id = unsafe { deviceid.as_str()? };
    let (address, address_type) =
        parse_stable_device_id(device_id, "destack.device.bluetooth.session.unpair")?;
    let device =
        BluetoothLEDevice::FromBluetoothAddressWithBluetoothAddressTypeAsync(address, address_type)
            .map_err(|error| {
                windows_bluetooth_error(
                    "destack.device.bluetooth.session.unpair",
                    "BluetoothLEDevice::FromBluetoothAddressWithBluetoothAddressTypeAsync",
                    &error,
                )
            })?
            .get()
            .map_err(|error| {
                windows_bluetooth_error(
                    "destack.device.bluetooth.session.unpair",
                    "IAsyncOperation::get",
                    &error,
                )
            })?;
    let device_information = device.DeviceInformation().map_err(|error| {
        windows_bluetooth_error(
            "destack.device.bluetooth.session.unpair",
            "BluetoothLEDevice::DeviceInformation",
            &error,
        )
    })?;
    let pairing = device_information.Pairing().map_err(|error| {
        windows_bluetooth_error(
            "destack.device.bluetooth.session.unpair",
            "DeviceInformation::Pairing",
            &error,
        )
    })?;
    let result = pairing
        .UnpairAsync()
        .map_err(|error| {
            windows_bluetooth_error(
                "destack.device.bluetooth.session.unpair",
                "DeviceInformationPairing::UnpairAsync",
                &error,
            )
        })?
        .get()
        .map_err(|error| {
            windows_bluetooth_error(
                "destack.device.bluetooth.session.unpair",
                "IAsyncOperation::get",
                &error,
            )
        })?;
    let status = result.Status().map_err(|error| {
        windows_bluetooth_error(
            "destack.device.bluetooth.session.unpair",
            "DeviceUnpairingResult::Status",
            &error,
        )
    })?;
    if status != DeviceUnpairingResultStatus::Unpaired
        && status != DeviceUnpairingResultStatus::AlreadyUnpaired
    {
        return Err(core_platform::io_operation_error(
            "destack.device.bluetooth.session.unpair",
            None,
            format!("bluetooth unpair failed with status {}", status.0),
        ));
    }

    Ok(())
}

/// Read one Windows bluetooth device RSSI value.
pub(crate) unsafe fn destack_device_bluetooth_read_rssi(
    binding: &BindingCallContext,
    out: *mut i32,
    handle: resource::BluetoothDeviceHandle,
    _timeoutns: u64,
) -> DiagnosticResult<()> {
    core_platform::ensure_out(out, "out")?;

    let resource = device_resource(binding, handle, "destack.device.bluetooth.session.rssi")?;
    let descriptor =
        opened_device_descriptor(&resource.device, "destack.device.bluetooth.session.rssi")?;
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

/// Read one Windows bluetooth session event.
pub(crate) unsafe fn destack_device_bluetooth_session_read_event(
    binding: &BindingCallContext,
    out: *mut BluetoothSessionEvent,
    handle: resource::BluetoothDeviceHandle,
    timeoutns: u64,
) -> DiagnosticResult<()> {
    core_platform::ensure_out(out, "out")?;

    let resource = device_resource(
        binding,
        handle,
        "destack.device.bluetooth.session.readEvent",
    )?;
    let value = resource.event_queue.pop_with_timeout_or_else(
        std::time::Duration::from_nanos(timeoutns),
        || {
            Err(core_platform::io_would_block(
                "destack.device.bluetooth.session.readEvent",
                "bluetooth session did not yield one event",
            ))
        },
    )?;

    unsafe {
        out.write(stored_session_event(binding, value));
    }

    Ok(())
}

/// Poll one Windows bluetooth session event without blocking.
pub(crate) unsafe fn destack_device_bluetooth_session_try_read_event(
    binding: &BindingCallContext,
    out: *mut BluetoothSessionEvent,
    handle: resource::BluetoothDeviceHandle,
) -> DiagnosticResult<()> {
    core_platform::ensure_out(out, "out")?;

    let resource = device_resource(
        binding,
        handle,
        "destack.device.bluetooth.session.tryReadEvent",
    )?;
    let value = resource.event_queue.try_pop_or_else(|| {
        Err(core_platform::io_would_block(
            "destack.device.bluetooth.session.tryReadEvent",
            "bluetooth session did not yield one event",
        ))
    })?;

    unsafe {
        out.write(stored_session_event(binding, value));
    }

    Ok(())
}
