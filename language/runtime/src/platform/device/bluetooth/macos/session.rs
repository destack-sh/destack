#![allow(unsafe_op_in_unsafe_fn)]

use super::core::*;
use super::delegates::*;
use super::metadata::*;
use super::runtime::*;

/// Open one macOS bluetooth device session.
pub(crate) unsafe fn destack_device_bluetooth_open(
    binding: &BindingCallContext,
    out: *mut resource::BluetoothDeviceHandle,
    adapterid: NativeStringRef,
    deviceid: NativeStringRef,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    let adapter_id = adapterid.as_str()?;
    let device_id = deviceid.as_str()?;
    require_adapter_id(adapter_id, "destack.device.bluetooth.session.open")?;
    let device_uuid = NSString::from_str(device_id);
    let device_uuid = NSUUID::from_string(&device_uuid).ok_or_else(|| {
        invalid_bluetooth_identifier("deviceId", "device id must be one valid NSUUID string")
    })?;

    let central_queue = Arc::new(BoundedQueue::new(16));
    let session_event_queue = Arc::new(BoundedQueue::new(BLUETOOTH_SESSION_EVENT_QUEUE_CAPACITY));
    let session_event_state = Arc::new(Mutex::new(BluetoothEventState { next_sequence: 1 }));
    let central_delegate = MacBluetoothCentralDelegate::new(
        central_queue.clone(),
        session_event_queue.clone(),
        session_event_state.clone(),
    );
    let central = central_manager_with_delegate(central_delegate.as_protocol());
    wait_for_device_central_ready(
        &central,
        &central_queue,
        "destack.device.bluetooth.session.open",
    )?;

    let peripheral = central.dispatch(|central| unsafe {
        let identifiers = NSArray::from_retained_slice(&[device_uuid.retain()]);
        let peripherals = central.retrievePeripheralsWithIdentifiers(&identifiers);

        peripherals
            .iter()
            .next()
            .map(|peripheral| BluetoothDispatchBound::new(peripheral))
    });
    let peripheral = peripheral.ok_or_else(|| {
        core_platform::io_not_found(
            "destack.device.bluetooth.session.open",
            "unknown bluetooth device",
        )
    })?;

    let pending = Arc::new(MacBluetoothPendingSlot::new());
    let subscriptions = Arc::new(Mutex::new(HashMap::new()));
    let cache_stale = Arc::new(AtomicBool::new(false));
    let peripheral_delegate = MacBluetoothPeripheralDelegate::new(
        pending.clone(),
        session_event_queue.clone(),
        session_event_state.clone(),
        subscriptions.clone(),
        cache_stale.clone(),
    );

    peripheral.dispatch(|peripheral| unsafe {
        peripheral.setDelegate(Some(peripheral_delegate.as_protocol()));
    });
    central.dispatch(|central| unsafe {
        central.connectPeripheral_options(peripheral.get_unchecked(), None);
    });
    wait_for_connection(
        &central_queue,
        device_id,
        "destack.device.bluetooth.session.open",
    )?;

    let operation_lock = Arc::new(Mutex::new(()));
    let cache = refresh_gatt_cache(&peripheral, &pending, &operation_lock, None)?;
    let cache = Arc::new(Mutex::new(cache));

    let resource = Arc::new(MacBluetoothDeviceResource {
        peripheral: peripheral.clone(),
        event_queue: session_event_queue.clone(),
        cache,
        cache_stale,
        operation_lock,
        pending: pending.clone(),
        subscriptions: subscriptions.clone(),
    });

    let entry = ResourceEntry::new(ResourceKind::BluetoothDevice)
        .with_label(BLUETOOTH_DEVICE_RESOURCE_LABEL)
        .with_payload(resource.clone())
        .with_finalizer(binding.worker().platform_state.device.wrap_finalizer(
            MacBluetoothDeviceFinalizer {
                central,
                peripheral,
                event_queue: session_event_queue,
                pending,
            },
        ));
    let resource_id =
        binding
            .worker()
            .resources
            .insert(&binding.world(), entry, Some(binding.engine()));

    out.write(resource::BluetoothDeviceHandle(resource_id));

    Ok(())
}

/// Close one Bluetooth device session.
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

/// Read one macOS bluetooth device descriptor snapshot.
pub(crate) unsafe fn destack_device_bluetooth_descriptor(
    binding: &BindingCallContext,
    out: *mut BluetoothDeviceDescriptor,
    handle: resource::BluetoothDeviceHandle,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    // resolve the active device session
    let resource = device_resource(binding, handle, "destack.device.bluetooth.descriptor")?;
    let descriptor = current_device_descriptor(&resource.peripheral);

    // store the current descriptor snapshot
    unsafe {
        out.write(BluetoothDeviceDescriptor::from_value(binding, descriptor));
    }

    Ok(())
}

/// Read one blocking session event.
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
    out.write(pop_session_event(
        binding,
        &resource,
        timeoutns,
        "destack.device.bluetooth.session.readEvent",
    )?);

    Ok(())
}

/// Poll one session event without blocking.
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
    out.write(stored_session_event(binding, value));

    Ok(())
}

/// Pair one Bluetooth device session.
pub(crate) unsafe fn destack_device_bluetooth_pair(
    _binding: &BindingCallContext,
    _handle: resource::BluetoothDeviceHandle,
    _timeoutns: u64,
) -> RuntimeResult<()> {
    Err(core_platform::not_supported(
        "destack.device.bluetooth.session.pair",
    ))
}

/// Remove one Bluetooth device bond.
pub(crate) unsafe fn destack_device_bluetooth_unpair(
    _binding: &BindingCallContext,
    _adapterid: NativeStringRef,
    _deviceid: NativeStringRef,
) -> RuntimeResult<()> {
    Err(core_platform::not_supported(
        "destack.device.bluetooth.session.unpair",
    ))
}

/// Read one device RSSI value.
pub(crate) unsafe fn destack_device_bluetooth_read_rssi(
    binding: &BindingCallContext,
    out: *mut i32,
    handle: resource::BluetoothDeviceHandle,
    timeoutns: u64,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    let resource = device_resource(binding, handle, "destack.device.bluetooth.session.rssi")?;
    let _operation_lock = resource.operation_lock.lock();
    resource.pending.begin(MacBluetoothPendingTask::ReadRssi);
    resource.peripheral.dispatch(|peripheral| unsafe {
        peripheral.readRSSI();
    });
    let value = wait_for_pending_operation(
        &resource.pending,
        "destack.device.bluetooth.session.rssi",
        timeoutns,
    )?;
    let MacBluetoothPendingValue::Rssi(rssi) = value else {
        return Err(core_platform::io_operation_error(
            "destack.device.bluetooth.session.rssi",
            Some(PlatformErrorCode::IoInvalidData),
            "corebluetooth returned one mismatched RSSI callback payload",
        ));
    };

    out.write(rssi);

    Ok(())
}
