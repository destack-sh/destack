use super::core::*;
use super::metadata::*;
use super::watch::*;

/// List the Linux bluetooth adapters exposed to the runtime.
pub(crate) unsafe fn destack_device_bluetooth_adapter_list(
    binding: &BindingCallContext,
    out: *mut NativeSlice<BluetoothAdapterDescriptor>,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    // enumerate current adapters from the BlueZ object graph
    let service = binding
        .worker()
        .platform_state
        .device
        .linux_bluetooth_service("destack.device.bluetooth.adapterList")?;
    let connection = service.connection();
    let objects = bluez_managed_objects(&connection, "destack.device.bluetooth.adapterList")?;
    let mut adapters = Vec::new();

    for (path, interfaces) in &objects {
        if interface_properties(interfaces, BLUEZ_ADAPTER_INTERFACE).is_none() {
            continue;
        }

        let descriptor =
            adapter_descriptor(path, interfaces, "destack.device.bluetooth.adapterList")?;
        adapters.push(BluetoothAdapterDescriptor::from_value(binding, descriptor));
    }

    unsafe {
        out.write(binding.store_slice(adapters));
    }

    Ok(())
}

/// Open one Linux bluetooth scan session.
pub(crate) unsafe fn destack_device_bluetooth_scan_open(
    binding: &BindingCallContext,
    out: *mut resource::BluetoothScanHandle,
    adapterid: NativeStringRef,
    filter: Option<BluetoothScanFilter>,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    // resolve the selected adapter and scan filter
    let service = binding
        .worker()
        .platform_state
        .device
        .linux_bluetooth_service("destack.device.bluetooth.scan.open")?;
    let connection = service.connection();
    let adapter_id = unsafe { adapterid.as_str()? };
    let adapter_path = parse_adapter_path(
        &connection,
        adapter_id,
        "destack.device.bluetooth.scan.open",
    )?;
    let filter = unsafe { <Option<BluetoothScanFilter> as NativeAbiCodec>::into_value(filter)? };
    if let Some(filter) = filter.as_ref() {
        validate_scan_filter(filter)?;
    }

    // capture the initial matching device snapshot before starting the watcher
    let known_devices = adapter_device_snapshot(
        &connection,
        adapter_path.as_str(),
        filter.as_ref(),
        "destack.device.bluetooth.scan.open",
    )?;
    let device_queue = Arc::new(BoundedQueue::new(BLUETOOTH_SCAN_DEVICE_QUEUE_CAPACITY));
    let event_queue = Arc::new(BoundedQueue::new(BLUETOOTH_SCAN_EVENT_QUEUE_CAPACITY));
    let resource = Arc::new(LinuxBluetoothScanResource {
        adapter_path: adapter_path.as_str().to_string(),
        filter,
        device_queue: device_queue.clone(),
        event_queue: event_queue.clone(),
        event_state: Mutex::new(BluetoothEventState { next_sequence: 1 }),
    });

    // seed the queues with the current snapshot
    for descriptor in known_devices.values() {
        resource.device_queue.push_drop_oldest(descriptor.clone());
        resource.event_queue.push_drop_oldest(scan_discovered_event(
            &resource.event_state,
            descriptor.clone(),
        ));
    }

    // register the scan with the shared service before starting discovery
    let registration_id = service.register_scan(&resource, known_devices);
    if let Err(error) = service.retain_adapter_scan(adapter_path.as_str()) {
        service.unregister_scan(registration_id);

        return Err(error);
    }

    // store the opened scan session
    let entry = ResourceEntry::new(ResourceKind::BluetoothScan)
        .with_label(BLUETOOTH_SCAN_RESOURCE_LABEL)
        .with_payload(resource.clone())
        .with_finalizer(binding.worker().platform_state.device.wrap_finalizer(
            LinuxBluetoothScanFinalizer {
                service,
                registration_id,
                adapter_path: resource.adapter_path.clone(),
                device_queue,
                event_queue,
            },
        ));
    let handle = binding
        .worker()
        .resources
        .insert(binding.world(), entry, Some(binding.engine()));

    unsafe {
        out.write(resource::BluetoothScanHandle(handle));
    }

    Ok(())
}

/// Close one Linux bluetooth scan session.
pub(crate) unsafe fn destack_device_bluetooth_scan_close(
    binding: &BindingCallContext,
    handle: resource::BluetoothScanHandle,
) -> RuntimeResult<()> {
    close_bluetooth_resource(
        binding,
        handle.0,
        ResourceKind::BluetoothScan,
        "destack.device.bluetooth.scan.close",
        "scan",
    )
}

/// Read one blocking Linux bluetooth scan event.
pub(crate) unsafe fn destack_device_bluetooth_scan_read_event(
    binding: &BindingCallContext,
    out: *mut BluetoothScanEvent,
    handle: resource::BluetoothScanHandle,
    timeoutns: u64,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    let resource = scan_resource(binding, handle, "destack.device.bluetooth.scan.readEvent")?;
    let event = pop_scan_event(
        binding,
        &resource,
        timeoutns,
        "destack.device.bluetooth.scan.readEvent",
    )?;

    unsafe {
        out.write(event);
    }

    Ok(())
}

/// Poll one Linux bluetooth scan event without blocking.
pub(crate) unsafe fn destack_device_bluetooth_scan_try_read_event(
    binding: &BindingCallContext,
    out: *mut BluetoothScanEvent,
    handle: resource::BluetoothScanHandle,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    let resource = scan_resource(
        binding,
        handle,
        "destack.device.bluetooth.scan.tryReadEvent",
    )?;
    let value = resource.event_queue.try_pop_or_else(|| {
        Err(core_platform::io_would_block(
            "destack.device.bluetooth.scan.tryReadEvent",
            "bluetooth scan event queue is empty",
        ))
    })?;

    unsafe {
        out.write(stored_scan_event(binding, value));
    }

    Ok(())
}
