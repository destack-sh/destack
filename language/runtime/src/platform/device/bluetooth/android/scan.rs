use super::codec::*;

/// Resolve one Android bluetooth scan resource.
fn scan_resource(
    binding: &BindingCallContext,
    handle: resource::BluetoothScanHandle,
    operation: &'static str,
) -> RuntimeResult<Arc<AndroidBluetoothScanResource>> {
    bluetooth_payload(
        binding,
        handle.0,
        ResourceKind::BluetoothScan,
        operation,
        "scan",
    )
}

/// List the Android bluetooth adapters exposed to the runtime.
pub(crate) unsafe fn destack_device_bluetooth_adapter_list(
    binding: &BindingCallContext,
    out: *mut NativeSlice<BluetoothAdapterDescriptor>,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    // enumerate the host adapters
    let adapters = read_adapter_descriptors(binding, "destack.device.bluetooth.adapterList")?;
    let adapters = adapters
        .into_iter()
        .map(|adapter| BluetoothAdapterDescriptor::from_value(binding, adapter))
        .collect();

    // store the adapter slice
    unsafe {
        out.write(binding.store_slice(adapters));
    }

    Ok(())
}

/// Open one Android bluetooth scan session.
pub(crate) unsafe fn destack_device_bluetooth_scan_open(
    binding: &BindingCallContext,
    out: *mut resource::BluetoothScanHandle,
    adapterid: NativeStringRef,
    filter: Option<BluetoothScanFilter>,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    // decode and validate the requested scan parameters
    let adapter_id = unsafe { adapterid.as_str()? };
    ensure_android_primary_adapter(binding, adapter_id, "destack.device.bluetooth.scan.open")?;
    let filter = unsafe { <Option<BluetoothScanFilter> as NativeAbiCodec>::into_value(filter)? };
    let (encoded_filter, mut filter_bytes) = encode_android_scan_filter(&filter);

    // open the host scan session
    let runtime_id = host_session_id(binding, "destack.device.bluetooth.scan.open")?;
    let mut session_id = 0u64;
    let status = unsafe {
        destack_host_android_bluetooth_scan_open(
            runtime_id,
            NativeStringRef::from(adapter_id),
            encoded_filter,
            android_filter_flags(&filter),
            NativeSlice {
                data: filter_bytes.as_mut_ptr(),
                len: checked_u32_length(
                    filter_bytes.len(),
                    "destack.device.bluetooth.scan.open",
                    "android bluetooth filter bytes",
                )?,
            },
            &mut session_id,
        )
    };
    host_status_result(
        status,
        "destack.device.bluetooth.scan.open",
        "bluetooth scan open",
    )?;

    // store the scan resource
    let resource = Arc::new(AndroidBluetoothScanResource {
        session_id,
        filter,
        event_state: Arc::new(Mutex::new(BluetoothEventState { next_sequence: 1 })),
    });
    let entry = ResourceEntry::new(ResourceKind::BluetoothScan)
        .with_label(BLUETOOTH_SCAN_RESOURCE_LABEL)
        .with_payload(resource)
        .with_finalizer(binding.worker().platform_state.device.wrap_finalizer(
            AndroidBluetoothScanFinalizer {
                runtime_id,
                session_id,
            },
        ));
    let handle = binding
        .worker()
        .resources
        .insert(&binding.world(), entry, Some(binding.engine()));

    unsafe {
        out.write(resource::BluetoothScanHandle(handle));
    }

    Ok(())
}

/// Close one Android bluetooth scan session.
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

/// Read one Android bluetooth scan event.
pub(crate) unsafe fn destack_device_bluetooth_scan_read_event(
    binding: &BindingCallContext,
    out: *mut BluetoothScanEvent,
    handle: resource::BluetoothScanHandle,
    timeoutns: u64,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    // resolve the active scan session
    let resource = scan_resource(binding, handle, "destack.device.bluetooth.scan.readEvent")?;

    loop {
        let (header, string_bytes) = read_scan_event_from_host(
            binding,
            resource.session_id,
            Some(timeoutns),
            false,
            "destack.device.bluetooth.scan.readEvent",
        )?;
        let descriptor = decode_device_descriptor(
            &header.descriptor,
            &string_bytes,
            "destack.device.bluetooth.scan.readEvent",
        )?;
        if resource
            .filter
            .as_ref()
            .is_some_and(|filter| !matches_scan_filter(&descriptor, filter))
        {
            continue;
        }

        let value = match header.kind {
            BLUETOOTH_SCAN_EVENT_DISCOVERED => {
                scan_discovered_event(&resource.event_state, descriptor)
            }
            BLUETOOTH_SCAN_EVENT_UPDATED => scan_updated_event(&resource.event_state, descriptor),
            BLUETOOTH_SCAN_EVENT_LOST => scan_lost_event(&resource.event_state, descriptor),
            kind => {
                return Err(invalid_data(
                    "destack.device.bluetooth.scan.readEvent",
                    format!("android host returned one unknown bluetooth scan event kind {kind}"),
                ));
            }
        };

        unsafe {
            out.write(stored_scan_event(binding, value));
        }

        return Ok(());
    }
}

/// Poll one Android bluetooth scan event without blocking.
pub(crate) unsafe fn destack_device_bluetooth_scan_try_read_event(
    binding: &BindingCallContext,
    out: *mut BluetoothScanEvent,
    handle: resource::BluetoothScanHandle,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    // resolve the active scan session
    let resource = scan_resource(
        binding,
        handle,
        "destack.device.bluetooth.scan.tryReadEvent",
    )?;

    loop {
        let (header, string_bytes) = read_scan_event_from_host(
            binding,
            resource.session_id,
            None,
            true,
            "destack.device.bluetooth.scan.tryReadEvent",
        )?;
        let descriptor = decode_device_descriptor(
            &header.descriptor,
            &string_bytes,
            "destack.device.bluetooth.scan.tryReadEvent",
        )?;
        if resource
            .filter
            .as_ref()
            .is_some_and(|filter| !matches_scan_filter(&descriptor, filter))
        {
            continue;
        }

        let value = match header.kind {
            BLUETOOTH_SCAN_EVENT_DISCOVERED => {
                scan_discovered_event(&resource.event_state, descriptor)
            }
            BLUETOOTH_SCAN_EVENT_UPDATED => scan_updated_event(&resource.event_state, descriptor),
            BLUETOOTH_SCAN_EVENT_LOST => scan_lost_event(&resource.event_state, descriptor),
            kind => {
                return Err(invalid_data(
                    "destack.device.bluetooth.scan.tryReadEvent",
                    format!("android host returned one unknown bluetooth scan event kind {kind}"),
                ));
            }
        };

        unsafe {
            out.write(stored_scan_event(binding, value));
        }

        return Ok(());
    }
}
