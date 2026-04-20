use super::core::*;
use super::descriptor::*;
use super::ffi::ffi;
use super::service::*;
use super::transfer::*;

/// Enumerate visible usb devices.
#[cfg(not(target_os = "android"))]
pub(crate) unsafe fn destack_device_usb_list(
    binding: &BindingCallContext,
    out: *mut NativeSlice<UsbDeviceDescriptor>,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    // enumerate current devices through the shared libusb service
    let service = binding
        .worker()
        .platform_state
        .device
        .usb_service("destack.device.usb.list")?;
    let descriptors = enumerate_usb_devices(&service.state, "destack.device.usb.list")?
        .into_iter()
        .map(|value| UsbDeviceDescriptor::from_value(binding, value))
        .collect::<Vec<_>>();

    let descriptors = binding.store_slice(descriptors);
    unsafe {
        out.write(descriptors);
    }

    Ok(())
}

/// Open one usb watch stream.
#[cfg(not(target_os = "android"))]
pub(crate) unsafe fn destack_device_usb_watch_open(
    binding: &BindingCallContext,
    out: *mut resource::UsbWatchHandle,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    // start the shared runtime before registering one live watch
    let service = binding
        .worker()
        .platform_state
        .device
        .usb_service("destack.device.usb.watchOpen")?;
    ensure_usb_service_runtime(&service)?;

    // create one queued watch resource and register it with the shared service
    let resource = Arc::new(UsbWatchResource {
        queue: BoundedQueue::new(USB_WATCH_QUEUE_CAPACITY),
        state: Mutex::new(UsbWatchState {
            next_sequence: 1,
            reported_dropped_count: 0,
        }),
    });

    let entry =
        ResourceEntry::new(ResourceKind::UsbWatch)
            .with_label(USB_WATCH_RESOURCE_LABEL)
            .with_payload(resource.clone())
            .with_finalizer(binding.worker().platform_state.device.wrap_finalizer(
                UsbWatchFinalizer {
                    service: service.state.clone(),
                    queue: resource.clone(),
                },
            ));
    let resource_id =
        binding
            .worker()
            .resources
            .insert(&binding.world(), entry, Some(binding.engine()));

    // register first, then seed the watcher from the current topology snapshot
    let seed_records = {
        let snapshot = service.state.watch_snapshot.lock();
        service
            .state
            .watch_registry
            .lock()
            .register(resource_id, &resource);

        snapshot
            .values()
            .cloned()
            .map(|descriptor| UsbHotplugEventRecord {
                timestamp_ns: core_platform::monotonic_now_ns(),
                kind: UsbWatchEventKind::Instance,
                descriptor,
            })
            .collect::<Vec<_>>()
    };

    // preserve current topology as explicit initial attach events
    for record in seed_records {
        resource.queue.push_drop_oldest(record);
    }

    signal_usb_service_runtime(&service.state);

    unsafe {
        out.write(resource::UsbWatchHandle(resource_id));
    }

    Ok(())
}

/// Close one usb watch stream.
#[cfg(not(target_os = "android"))]
pub(crate) unsafe fn destack_device_usb_watch_close(
    binding: &BindingCallContext,
    handle: resource::UsbWatchHandle,
) -> RuntimeResult<()> {
    let kind = binding
        .worker()
        .resources
        .with_entry(handle.0, |entry| entry.kind)
        .ok_or_else(|| {
            invalid_usb_handle("destack.device.usb.watchClose", "unknown usb watch handle")
        })?;
    if kind != ResourceKind::UsbWatch {
        return Err(invalid_usb_handle(
            "destack.device.usb.watchClose",
            "unknown usb watch handle",
        ));
    }

    if !binding.worker().resources.remove_and_finalize(
        &binding.world(),
        handle.0,
        Some(binding.engine()),
    ) {
        return Err(invalid_usb_handle(
            "destack.device.usb.watchClose",
            "unknown usb watch handle",
        ));
    }

    Ok(())
}

/// Wait for one usb hotplug event.
#[cfg(not(target_os = "android"))]
pub(crate) unsafe fn destack_device_usb_watch_read(
    binding: &BindingCallContext,
    out: *mut UsbHotplugEvent,
    handle: resource::UsbWatchHandle,
    timeoutns: u64,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    let watch = usb_watch_resource(binding, handle, "destack.device.usb.watchRead")?;
    if let Some(dropped_count) = take_watch_overflow_count(&watch.queue, &watch.state) {
        unsafe {
            out.write(hotplug_overflow_event(binding, &watch.state, dropped_count));
        }

        return Ok(());
    }

    let timeout = Duration::from_nanos(timeoutns);
    let Some(record) = watch.queue.pop_with_timeout(timeout) else {
        return Err(usb_watch_would_block("destack.device.usb.watchRead"));
    };
    let event = hotplug_event_from_record(binding, &watch, record);

    unsafe {
        out.write(event);
    }

    Ok(())
}

/// Poll one usb hotplug event without blocking.
#[cfg(not(target_os = "android"))]
pub(crate) unsafe fn destack_device_usb_watch_try_read(
    binding: &BindingCallContext,
    out: *mut UsbHotplugEvent,
    handle: resource::UsbWatchHandle,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    let watch = usb_watch_resource(binding, handle, "destack.device.usb.watchTryRead")?;
    if let Some(dropped_count) = take_watch_overflow_count(&watch.queue, &watch.state) {
        unsafe {
            out.write(hotplug_overflow_event(binding, &watch.state, dropped_count));
        }

        return Ok(());
    }

    let Some(record) = watch.queue.try_pop() else {
        return Err(usb_watch_would_block("destack.device.usb.watchTryRead"));
    };
    let event = hotplug_event_from_record(binding, &watch, record);

    unsafe {
        out.write(event);
    }

    Ok(())
}

/// Open one usb device session.
#[cfg(not(target_os = "android"))]
pub(crate) unsafe fn destack_device_usb_open(
    binding: &BindingCallContext,
    out: *mut resource::UsbDeviceHandle,
    id: NativeStringRef,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    // start the shared runtime before opening one device that may use async transfers
    let service = binding
        .worker()
        .platform_state
        .device
        .usb_service("destack.device.usb.open")?;
    ensure_usb_service_runtime(&service)?;

    // open the selected device and retain a libusb device reference for resource lifetime
    let id = unsafe { id.as_str()? };
    let resource = open_usb_device(&service.state, id, "destack.device.usb.open")?;
    let handle = resource.handle;
    let device = resource.device;
    let resource = Arc::new(resource);

    let entry = ResourceEntry::new(ResourceKind::UsbDevice)
        .with_label(USB_DEVICE_RESOURCE_LABEL)
        .with_payload(resource.clone())
        .with_finalizer(binding.worker().platform_state.device.wrap_finalizer(
            UsbDeviceFinalizer {
                service: service.state.clone(),
                handle,
                device,
            },
        ));
    let resource_id =
        binding
            .worker()
            .resources
            .insert(&binding.world(), entry, Some(binding.engine()));

    unsafe {
        out.write(resource::UsbDeviceHandle(resource_id));
    }

    Ok(())
}

/// Close one usb device session.
pub(crate) unsafe fn destack_device_usb_close(
    binding: &BindingCallContext,
    handle: resource::UsbDeviceHandle,
) -> RuntimeResult<()> {
    let kind = binding
        .worker()
        .resources
        .with_entry(handle.0, |entry| entry.kind)
        .ok_or_else(|| {
            invalid_usb_handle("destack.device.usb.close", "unknown usb device handle")
        })?;
    if kind != ResourceKind::UsbDevice {
        return Err(invalid_usb_handle(
            "destack.device.usb.close",
            "unknown usb device handle",
        ));
    }

    if !binding.worker().resources.remove_and_finalize(
        &binding.world(),
        handle.0,
        Some(binding.engine()),
    ) {
        return Err(invalid_usb_handle(
            "destack.device.usb.close",
            "unknown usb device handle",
        ));
    }

    Ok(())
}

/// Claim one usb interface.
pub(crate) unsafe fn destack_device_usb_claim_interface(
    binding: &BindingCallContext,
    handle: resource::UsbDeviceHandle,
    interfacenumber: u8,
) -> RuntimeResult<()> {
    let resource = usb_device_resource(binding, handle, "destack.device.usb.claimInterface")?;
    let _operation_lock = resource.operation_lock.lock();
    let status = unsafe {
        (resource.service.api.libusb_claim_interface)(resource.handle, c_int::from(interfacenumber))
    };
    if status != ffi::LIBUSB_SUCCESS {
        return Err(libusb_error(
            "destack.device.usb.claimInterface",
            "libusb_claim_interface",
            status,
        ));
    }

    Ok(())
}

/// Release one usb interface.
pub(crate) unsafe fn destack_device_usb_release_interface(
    binding: &BindingCallContext,
    handle: resource::UsbDeviceHandle,
    interfacenumber: u8,
) -> RuntimeResult<()> {
    let resource = usb_device_resource(binding, handle, "destack.device.usb.releaseInterface")?;
    let _operation_lock = resource.operation_lock.lock();
    let status = unsafe {
        (resource.service.api.libusb_release_interface)(
            resource.handle,
            c_int::from(interfacenumber),
        )
    };
    if status != ffi::LIBUSB_SUCCESS {
        return Err(libusb_error(
            "destack.device.usb.releaseInterface",
            "libusb_release_interface",
            status,
        ));
    }

    Ok(())
}

/// Set one usb interface alternate setting.
pub(crate) unsafe fn destack_device_usb_set_interface_alternate_setting(
    binding: &BindingCallContext,
    handle: resource::UsbDeviceHandle,
    interfacenumber: u8,
    alternatesetting: u8,
) -> RuntimeResult<()> {
    let resource = usb_device_resource(
        binding,
        handle,
        "destack.device.usb.setInterfaceAlternateSetting",
    )?;
    let _operation_lock = resource.operation_lock.lock();
    let status = unsafe {
        (resource.service.api.libusb_set_interface_alt_setting)(
            resource.handle,
            c_int::from(interfacenumber),
            c_int::from(alternatesetting),
        )
    };
    if status != ffi::LIBUSB_SUCCESS {
        return Err(libusb_error(
            "destack.device.usb.setInterfaceAlternateSetting",
            "libusb_set_interface_alt_setting",
            status,
        ));
    }

    Ok(())
}

/// Read one opened usb device descriptor.
pub(crate) unsafe fn destack_device_usb_descriptor(
    binding: &BindingCallContext,
    out: *mut UsbDeviceDescriptor,
    handle: resource::UsbDeviceHandle,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    let resource = usb_device_resource(binding, handle, "destack.device.usb.descriptor")?;
    let descriptor = UsbDeviceDescriptor::from_value(binding, resource.descriptor.clone());

    unsafe {
        out.write(descriptor);
    }

    Ok(())
}

/// List string-descriptor languages.
pub(crate) unsafe fn destack_device_usb_string_language_list(
    binding: &BindingCallContext,
    out: *mut NativeSlice<u16>,
    handle: resource::UsbDeviceHandle,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    let resource = usb_device_resource(binding, handle, "destack.device.usb.stringLanguageList")?;
    let languages = read_string_languages(&resource.service, resource.handle)?;
    let languages = binding.store_slice(languages);

    unsafe {
        out.write(languages);
    }

    Ok(())
}

/// Read one selected language string descriptor set.
pub(crate) unsafe fn destack_device_usb_string_descriptor(
    binding: &BindingCallContext,
    out: *mut UsbStringDescriptor,
    handle: resource::UsbDeviceHandle,
    languageid: u16,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    let resource = usb_device_resource(binding, handle, "destack.device.usb.stringDescriptor")?;
    let raw = read_raw_device_descriptor(
        &resource.service,
        resource.device,
        "destack.device.usb.stringDescriptor",
    )?;
    let value = UsbStringDescriptorValue {
        language_id: languageid,
        manufacturer: read_string_descriptor_utf16(
            &resource.service,
            resource.handle,
            raw.i_manufacturer,
            languageid,
            "destack.device.usb.stringDescriptor",
        )?,
        product: read_string_descriptor_utf16(
            &resource.service,
            resource.handle,
            raw.i_product,
            languageid,
            "destack.device.usb.stringDescriptor",
        )?,
        serial_number: read_string_descriptor_utf16(
            &resource.service,
            resource.handle,
            raw.i_serial_number,
            languageid,
            "destack.device.usb.stringDescriptor",
        )?,
    };
    let descriptor = UsbStringDescriptor::from_value(binding, value);

    unsafe {
        out.write(descriptor);
    }

    Ok(())
}

/// List BOS capabilities.
pub(crate) unsafe fn destack_device_usb_bos_capability_list(
    binding: &BindingCallContext,
    out: *mut NativeSlice<UsbBosCapabilityDescriptor>,
    handle: resource::UsbDeviceHandle,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    let resource = usb_device_resource(binding, handle, "destack.device.usb.bosCapabilityList")?;
    let capabilities = read_bos_capabilities(
        &resource.service,
        resource.handle,
        "destack.device.usb.bosCapabilityList",
    )?
    .into_iter()
    .map(|value| UsbBosCapabilityDescriptor::from_value(binding, value))
    .collect::<Vec<_>>();
    let capabilities = binding.store_slice(capabilities);

    unsafe {
        out.write(capabilities);
    }

    Ok(())
}

/// List configurations for one opened device.
pub(crate) unsafe fn destack_device_usb_configuration_list(
    binding: &BindingCallContext,
    out: *mut NativeSlice<UsbConfigurationDescriptor>,
    handle: resource::UsbDeviceHandle,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    let resource = usb_device_resource(binding, handle, "destack.device.usb.configurationList")?;
    let configurations = read_configuration_descriptors(
        &resource.service,
        resource.handle,
        resource.device,
        "destack.device.usb.configurationList",
    )?
    .into_iter()
    .map(|value| UsbConfigurationDescriptor::from_value(binding, value))
    .collect::<Vec<_>>();
    let configurations = binding.store_slice(configurations);

    unsafe {
        out.write(configurations);
    }

    Ok(())
}

/// Read the active configuration value.
pub(crate) unsafe fn destack_device_usb_configuration_get(
    binding: &BindingCallContext,
    out: *mut u8,
    handle: resource::UsbDeviceHandle,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    let resource = usb_device_resource(binding, handle, "destack.device.usb.configurationGet")?;
    let mut configuration = 0;
    let status = unsafe {
        (resource.service.api.libusb_get_configuration)(resource.handle, &mut configuration)
    };
    if status != ffi::LIBUSB_SUCCESS {
        return Err(libusb_error(
            "destack.device.usb.configurationGet",
            "libusb_get_configuration",
            status,
        ));
    }

    unsafe {
        out.write(configuration as u8);
    }

    Ok(())
}

/// Set the active configuration value.
pub(crate) unsafe fn destack_device_usb_configuration_set(
    binding: &BindingCallContext,
    handle: resource::UsbDeviceHandle,
    configurationvalue: u8,
) -> RuntimeResult<()> {
    let resource = usb_device_resource(binding, handle, "destack.device.usb.configurationSet")?;
    let _operation_lock = resource.operation_lock.lock();
    let status = unsafe {
        (resource.service.api.libusb_set_configuration)(
            resource.handle,
            c_int::from(configurationvalue),
        )
    };
    if status != ffi::LIBUSB_SUCCESS {
        return Err(libusb_error(
            "destack.device.usb.configurationSet",
            "libusb_set_configuration",
            status,
        ));
    }

    Ok(())
}

/// Read one control transfer.
pub(crate) unsafe fn destack_device_usb_control_read(
    binding: &BindingCallContext,
    out: *mut UsbInTransferResult,
    handle: resource::UsbDeviceHandle,
    setup: UsbControlSetup,
    timeoutns: u64,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    let resource = usb_device_resource(binding, handle, "destack.device.usb.controlRead")?;
    let setup = unsafe { setup.into_value()? };
    let result = execute_control_read(
        &resource,
        &setup,
        timeoutns,
        "destack.device.usb.controlRead",
    )?;
    let result = UsbInTransferResult::from_value(binding, result);

    unsafe {
        out.write(result);
    }

    Ok(())
}

/// Write one control transfer.
pub(crate) unsafe fn destack_device_usb_control_write(
    binding: &BindingCallContext,
    out: *mut UsbOutTransferResult,
    handle: resource::UsbDeviceHandle,
    setup: UsbControlSetup,
    argument_bytes: NativeSlice<u8>,
    timeoutns: u64,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    let resource = usb_device_resource(binding, handle, "destack.device.usb.controlWrite")?;
    let setup = unsafe { setup.into_value()? };
    let bytes = unsafe { argument_bytes.as_slice()? };
    let result = execute_control_write(
        &resource,
        &setup,
        bytes,
        timeoutns,
        "destack.device.usb.controlWrite",
    )?;
    let result = UsbOutTransferResult::from_value(binding, result);

    unsafe {
        out.write(result);
    }

    Ok(())
}

/// Read one bulk endpoint.
pub(crate) unsafe fn destack_device_usb_bulk_read(
    binding: &BindingCallContext,
    out: *mut UsbInTransferResult,
    handle: resource::UsbDeviceHandle,
    endpoint: UsbEndpointSelector,
    maxbytes: u32,
    timeoutns: u64,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    let resource = usb_device_resource(binding, handle, "destack.device.usb.bulkRead")?;
    let endpoint = unsafe { endpoint.into_value()? };
    validate_endpoint(&endpoint, UsbEndpointDirection::In, "endpoint")?;
    let result = execute_in_transfer(
        &resource,
        &endpoint,
        maxbytes,
        timeoutns,
        "destack.device.usb.bulkRead",
        UsbAsyncTransferKind::Bulk,
    )?;
    let result = UsbInTransferResult::from_value(binding, result);

    unsafe {
        out.write(result);
    }

    Ok(())
}

/// Write one bulk endpoint.
pub(crate) unsafe fn destack_device_usb_bulk_write(
    binding: &BindingCallContext,
    out: *mut UsbOutTransferResult,
    handle: resource::UsbDeviceHandle,
    endpoint: UsbEndpointSelector,
    argument_bytes: NativeSlice<u8>,
    timeoutns: u64,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    let resource = usb_device_resource(binding, handle, "destack.device.usb.bulkWrite")?;
    let endpoint = unsafe { endpoint.into_value()? };
    validate_endpoint(&endpoint, UsbEndpointDirection::Out, "endpoint")?;
    let bytes = unsafe { argument_bytes.as_slice()? };
    let result = execute_out_transfer(
        &resource,
        &endpoint,
        bytes,
        timeoutns,
        "destack.device.usb.bulkWrite",
        UsbAsyncTransferKind::Bulk,
    )?;
    let result = UsbOutTransferResult::from_value(binding, result);

    unsafe {
        out.write(result);
    }

    Ok(())
}

/// Read one interrupt endpoint.
pub(crate) unsafe fn destack_device_usb_interrupt_read(
    binding: &BindingCallContext,
    out: *mut UsbInTransferResult,
    handle: resource::UsbDeviceHandle,
    endpoint: UsbEndpointSelector,
    maxbytes: u32,
    timeoutns: u64,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    let resource = usb_device_resource(binding, handle, "destack.device.usb.interruptRead")?;
    let endpoint = unsafe { endpoint.into_value()? };
    validate_endpoint(&endpoint, UsbEndpointDirection::In, "endpoint")?;
    let result = execute_in_transfer(
        &resource,
        &endpoint,
        maxbytes,
        timeoutns,
        "destack.device.usb.interruptRead",
        UsbAsyncTransferKind::Interrupt,
    )?;
    let result = UsbInTransferResult::from_value(binding, result);

    unsafe {
        out.write(result);
    }

    Ok(())
}

/// Write one interrupt endpoint.
pub(crate) unsafe fn destack_device_usb_interrupt_write(
    binding: &BindingCallContext,
    out: *mut UsbOutTransferResult,
    handle: resource::UsbDeviceHandle,
    endpoint: UsbEndpointSelector,
    argument_bytes: NativeSlice<u8>,
    timeoutns: u64,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    let resource = usb_device_resource(binding, handle, "destack.device.usb.interruptWrite")?;
    let endpoint = unsafe { endpoint.into_value()? };
    validate_endpoint(&endpoint, UsbEndpointDirection::Out, "endpoint")?;
    let bytes = unsafe { argument_bytes.as_slice()? };
    let result = execute_out_transfer(
        &resource,
        &endpoint,
        bytes,
        timeoutns,
        "destack.device.usb.interruptWrite",
        UsbAsyncTransferKind::Interrupt,
    )?;
    let result = UsbOutTransferResult::from_value(binding, result);

    unsafe {
        out.write(result);
    }

    Ok(())
}

/// Read one isochronous transfer.
pub(crate) unsafe fn destack_device_usb_isochronous_read(
    binding: &BindingCallContext,
    out: *mut UsbIsochronousTransferResult,
    handle: resource::UsbDeviceHandle,
    endpoint: UsbEndpointSelector,
    packetsizes: NativeSlice<u32>,
    timeoutns: u64,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    let resource = usb_device_resource(binding, handle, "destack.device.usb.isochronousRead")?;
    let endpoint = unsafe { endpoint.into_value()? };
    validate_endpoint(&endpoint, UsbEndpointDirection::In, "endpoint")?;
    let packet_sizes = unsafe { packetsizes.as_slice()? };
    let total_length = packet_sizes
        .iter()
        .fold(0usize, |sum, size| sum + *size as usize);
    let bytes = vec![0u8; total_length];
    let result = execute_isochronous_transfer(
        &resource,
        &endpoint,
        &bytes,
        packet_sizes,
        timeoutns,
        "destack.device.usb.isochronousRead",
    )?;
    let result = UsbIsochronousTransferResult::from_value(binding, result);

    unsafe {
        out.write(result);
    }

    Ok(())
}

/// Write one isochronous transfer.
pub(crate) unsafe fn destack_device_usb_isochronous_write(
    binding: &BindingCallContext,
    out: *mut UsbIsochronousTransferResult,
    handle: resource::UsbDeviceHandle,
    endpoint: UsbEndpointSelector,
    argument_bytes: NativeSlice<u8>,
    packetsizes: NativeSlice<u32>,
    timeoutns: u64,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    let resource = usb_device_resource(binding, handle, "destack.device.usb.isochronousWrite")?;
    let endpoint = unsafe { endpoint.into_value()? };
    validate_endpoint(&endpoint, UsbEndpointDirection::Out, "endpoint")?;
    let bytes = unsafe { argument_bytes.as_slice()? };
    let packet_sizes = unsafe { packetsizes.as_slice()? };
    let result = execute_isochronous_transfer(
        &resource,
        &endpoint,
        bytes,
        packet_sizes,
        timeoutns,
        "destack.device.usb.isochronousWrite",
    )?;
    let result = UsbIsochronousTransferResult::from_value(binding, result);

    unsafe {
        out.write(result);
    }

    Ok(())
}

/// Cancel one pending transfer lane.
pub(crate) unsafe fn destack_device_usb_transfer_cancel(
    binding: &BindingCallContext,
    handle: resource::UsbDeviceHandle,
    endpoint: UsbEndpointSelector,
) -> RuntimeResult<()> {
    let resource = usb_device_resource(binding, handle, "destack.device.usb.transferCancel")?;
    let endpoint = unsafe { endpoint.into_value()? };
    validate_endpoint_selector(&endpoint, "endpoint")?;
    let endpoint_address = endpoint_address(&endpoint) as u8;

    cancel_active_endpoint_transfers(
        &resource,
        endpoint_address,
        "destack.device.usb.transferCancel",
    )
}

/// Cancel all pending transfer lanes.
pub(crate) unsafe fn destack_device_usb_transfer_cancel_all(
    binding: &BindingCallContext,
    handle: resource::UsbDeviceHandle,
) -> RuntimeResult<()> {
    let resource = usb_device_resource(binding, handle, "destack.device.usb.transferCancelAll")?;

    cancel_all_active_transfers(&resource, "destack.device.usb.transferCancelAll")
}

/// Clear one halted endpoint.
pub(crate) unsafe fn destack_device_usb_clear_halt(
    binding: &BindingCallContext,
    handle: resource::UsbDeviceHandle,
    endpoint: UsbEndpointSelector,
) -> RuntimeResult<()> {
    let resource = usb_device_resource(binding, handle, "destack.device.usb.clearHalt")?;
    let endpoint = unsafe { endpoint.into_value()? };
    validate_endpoint_selector(&endpoint, "endpoint")?;
    let endpoint = endpoint_address(&endpoint) as u8;
    let _operation_lock = resource.operation_lock.lock();
    let status = unsafe { (resource.service.api.libusb_clear_halt)(resource.handle, endpoint) };
    if status != ffi::LIBUSB_SUCCESS {
        return Err(libusb_error(
            "destack.device.usb.clearHalt",
            "libusb_clear_halt",
            status,
        ));
    }

    Ok(())
}

/// Reset one opened usb device.
pub(crate) unsafe fn destack_device_usb_reset(
    binding: &BindingCallContext,
    handle: resource::UsbDeviceHandle,
) -> RuntimeResult<()> {
    let resource = usb_device_resource(binding, handle, "destack.device.usb.reset")?;
    let _operation_lock = resource.operation_lock.lock();
    let status = unsafe { (resource.service.api.libusb_reset_device)(resource.handle) };
    if status != ffi::LIBUSB_SUCCESS {
        return Err(libusb_error(
            "destack.device.usb.reset",
            "libusb_reset_device",
            status,
        ));
    }

    Ok(())
}
