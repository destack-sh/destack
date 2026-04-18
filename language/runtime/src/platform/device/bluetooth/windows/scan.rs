use super::core::*;
use super::metadata::*;

/// Validate one Windows bluetooth scan filter against the WinRT surface.
fn validate_scan_filter(filter: &BluetoothScanFilterValue) -> DiagnosticResult<()> {
    // winrt does not expose explicit phy selection on the watcher surface
    if filter.primary_phy.is_some() {
        return Err(core_platform::invalid_argument(
            "filter.primaryPhy",
            "winrt does not expose scan primary PHY selection",
        ));
    }

    // winrt does not expose explicit phy selection on the watcher surface
    if filter.secondary_phy.is_some() {
        return Err(core_platform::invalid_argument(
            "filter.secondaryPhy",
            "winrt does not expose scan secondary PHY selection",
        ));
    }

    Ok(())
}

/// Map one public scan mode onto the WinRT watcher mode.
fn watcher_scan_mode(filter: &Option<BluetoothScanFilterValue>) -> BluetoothLEScanningMode {
    match filter.as_ref().and_then(|filter| filter.scan_mode) {
        Some(BluetoothScanMode::Passive) => BluetoothLEScanningMode::Passive,
        Some(BluetoothScanMode::Active) | None => BluetoothLEScanningMode::Active,
    }
}

/// Validate that one WinRT LE watcher can represent the requested adapter scope.
fn ensure_scan_adapter_scope(
    adapter_count: usize,
    operation: &'static str,
) -> DiagnosticResult<()> {
    // reject adapter-scoped scans when WinRT cannot bind the watcher to one adapter
    if adapter_count > 1 {
        return Err(core_platform::not_supported(format!(
            "{operation}: winrt le scanning is not adapter-scoped when multiple adapters are present",
        )));
    }

    Ok(())
}

/// Resolve one typed Windows bluetooth scan resource from the resource table.
fn scan_resource(
    binding: &BindingCallContext,
    handle: resource::BluetoothScanHandle,
    operation: &'static str,
) -> DiagnosticResult<Arc<WindowsBluetoothScanResource>> {
    bluetooth_payload(
        binding,
        handle.0,
        ResourceKind::BluetoothScan,
        operation,
        "scan",
    )
}

/// List the Windows bluetooth adapters exposed to the runtime.
pub(crate) unsafe fn destack_device_bluetooth_adapter_list(
    binding: &BindingCallContext,
    out: *mut NativeSlice<BluetoothAdapterDescriptor>,
) -> DiagnosticResult<()> {
    core_platform::ensure_out(out, "out")?;

    // enumerate visible adapters
    let adapters = enumerate_adapters("destack.device.bluetooth.adapterList")?;
    let adapters = adapters
        .into_iter()
        .map(|(descriptor, _)| BluetoothAdapterDescriptor::from_value(binding, descriptor))
        .collect();

    // store the adapter slice
    unsafe {
        out.write(binding.store_slice(adapters));
    }

    Ok(())
}

/// Open one Windows bluetooth scan session.
pub(crate) unsafe fn destack_device_bluetooth_scan_open(
    binding: &BindingCallContext,
    out: *mut resource::BluetoothScanHandle,
    adapterid: NativeStringRef,
    filter: Option<BluetoothScanFilter>,
) -> DiagnosticResult<()> {
    core_platform::ensure_out(out, "out")?;

    // resolve the requested adapter
    let adapter_id = unsafe { adapterid.as_str()? };
    let adapters = enumerate_adapters("destack.device.bluetooth.scan.open")?;
    let Some((_descriptor, _adapter)) = adapters
        .iter()
        .find(|(descriptor, _)| descriptor.id == adapter_id)
    else {
        return Err(core_platform::io_not_found(
            "destack.device.bluetooth.scan.open",
            format!("bluetooth adapter {adapter_id} not found"),
        ));
    };
    ensure_scan_adapter_scope(adapters.len(), "destack.device.bluetooth.scan.open")?;
    let filter = unsafe { <Option<BluetoothScanFilter> as NativeAbiCodec>::into_value(filter)? };
    if let Some(filter) = filter.as_ref() {
        validate_scan_filter(filter)?;
    }
    let scanning_mode = watcher_scan_mode(&filter);
    let keep_repeated_devices = filter
        .as_ref()
        .is_some_and(|filter| filter.keep_repeated_devices);

    // create the watcher and shared queues
    let watcher = BluetoothLEAdvertisementWatcher::new().map_err(|error| {
        windows_bluetooth_error(
            "destack.device.bluetooth.scan.open",
            "BluetoothLEAdvertisementWatcher::new",
            &error,
        )
    })?;
    watcher.SetScanningMode(scanning_mode).map_err(|error| {
        windows_bluetooth_error(
            "destack.device.bluetooth.scan.open",
            "BluetoothLEAdvertisementWatcher::SetScanningMode",
            &error,
        )
    })?;

    let event_queue = Arc::new(BoundedQueue::new(BLUETOOTH_SCAN_EVENT_QUEUE_CAPACITY));
    let event_state = Arc::new(Mutex::new(BluetoothEventState { next_sequence: 1 }));
    let known_devices = Arc::new(Mutex::new(
        HashMap::<String, BluetoothDeviceDescriptorValue>::new(),
    ));

    // advertisement delivery
    let callback_filter = filter.clone();
    let callback_events = event_queue.clone();
    let callback_state = event_state.clone();
    let callback_known_devices = known_devices.clone();
    let received_token = watcher
        .Received(&TypedEventHandler::new(
            move |_watcher: Ref<'_, BluetoothLEAdvertisementWatcher>,
                  args: Ref<'_, BluetoothLEAdvertisementReceivedEventArgs>| {
                let Some(args) = args.as_ref() else {
                    return Ok(());
                };

                let descriptor =
                    match received_device_descriptor(args, "destack.device.bluetooth.scan.open") {
                        Ok(descriptor) => descriptor,
                        Err(_) => return Ok(()),
                    };
                if callback_filter
                    .as_ref()
                    .is_some_and(|filter| !matches_scan_filter(&descriptor, filter))
                {
                    return Ok(());
                }

                // dedupe events by stable device id
                let mut known_devices = callback_known_devices.lock();
                let event = match known_devices.get(&descriptor.id) {
                    Some(previous) if previous == &descriptor && !keep_repeated_devices => None,
                    Some(_) => Some(scan_updated_event(&callback_state, descriptor.clone())),
                    None => Some(scan_discovered_event(&callback_state, descriptor.clone())),
                };
                known_devices.insert(descriptor.id.clone(), descriptor.clone());
                drop(known_devices);

                if let Some(event) = event {
                    callback_events.push_drop_oldest(event);
                }

                Ok(())
            },
        ))
        .map_err(|error| {
            windows_bluetooth_error(
                "destack.device.bluetooth.scan.open",
                "BluetoothLEAdvertisementWatcher::Received",
                &error,
            )
        })?;

    // watcher stop
    let stopped_events = event_queue.clone();
    let stopped_token = watcher
        .Stopped(&TypedEventHandler::new(move |_watcher, _args| {
            stopped_events.close();
            Ok(())
        }))
        .map_err(|error| {
            windows_bluetooth_error(
                "destack.device.bluetooth.scan.open",
                "BluetoothLEAdvertisementWatcher::Stopped",
                &error,
            )
        })?;

    // start the watcher before publishing the resource
    watcher.Start().map_err(|error| {
        windows_bluetooth_error(
            "destack.device.bluetooth.scan.open",
            "BluetoothLEAdvertisementWatcher::Start",
            &error,
        )
    })?;

    // store the resource
    let finalizer_watcher = watcher.clone();
    let finalizer_event_queue = event_queue.clone();

    let entry = ResourceEntry::new(ResourceKind::BluetoothScan)
        .with_label(BLUETOOTH_SCAN_RESOURCE_LABEL)
        .with_payload(Arc::new(WindowsBluetoothScanResource { event_queue }))
        .with_finalizer(binding.worker().platform_state.device.wrap_finalizer(
            WindowsBluetoothScanFinalizer {
                watcher: finalizer_watcher,
                received_token,
                stopped_token,
                event_queue: finalizer_event_queue,
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Reject one adapter-scoped WinRT scan request when multiple adapters are visible.
    #[test]
    fn test_ensure_scan_adapter_scope_rejects_multiple_adapters() {
        let result = ensure_scan_adapter_scope(2, "destack.device.bluetooth.scan.open");

        assert!(result.is_err());
    }

    /// Reject one Windows bluetooth scan filter that requests one unsupported primary PHY.
    #[test]
    fn test_validate_scan_filter_rejects_primary_phy() {
        let filter = BluetoothScanFilterValue {
            service_uuids: Vec::new(),
            name: None,
            name_prefix: None,
            manufacturer_data: Vec::new(),
            service_data: Vec::new(),
            keep_repeated_devices: false,
            minimum_rssi: None,
            primary_phy: Some(BluetoothPhy::Le1M),
            scan_mode: None,
            secondary_phy: None,
        };

        let result = validate_scan_filter(&filter);

        assert!(result.is_err());
    }

    /// Map passive scan mode onto the WinRT watcher mode.
    #[test]
    fn test_watcher_scan_mode_uses_passive_when_requested() {
        let filter = Some(BluetoothScanFilterValue {
            service_uuids: Vec::new(),
            name: None,
            name_prefix: None,
            manufacturer_data: Vec::new(),
            service_data: Vec::new(),
            keep_repeated_devices: false,
            minimum_rssi: None,
            scan_mode: Some(BluetoothScanMode::Passive),
            primary_phy: None,
            secondary_phy: None,
        });

        let mode = watcher_scan_mode(&filter);

        assert_eq!(mode, BluetoothLEScanningMode::Passive);
    }
}

/// Close one Windows bluetooth scan session.
pub(crate) unsafe fn destack_device_bluetooth_scan_close(
    binding: &BindingCallContext,
    handle: resource::BluetoothScanHandle,
) -> DiagnosticResult<()> {
    close_bluetooth_resource(
        binding,
        handle.0,
        ResourceKind::BluetoothScan,
        "destack.device.bluetooth.scan.close",
        "scan",
    )
}

/// Read one Windows bluetooth scan event.
pub(crate) unsafe fn destack_device_bluetooth_scan_read_event(
    binding: &BindingCallContext,
    out: *mut BluetoothScanEvent,
    handle: resource::BluetoothScanHandle,
    timeoutns: u64,
) -> DiagnosticResult<()> {
    core_platform::ensure_out(out, "out")?;

    // resolve the active scan session
    let resource = scan_resource(binding, handle, "destack.device.bluetooth.scan.readEvent")?;
    let value = resource.event_queue.pop_with_timeout_or_else(
        std::time::Duration::from_nanos(timeoutns),
        || {
            Err(core_platform::io_would_block(
                "destack.device.bluetooth.scan.readEvent",
                "bluetooth scan did not yield one event",
            ))
        },
    )?;

    unsafe {
        out.write(stored_scan_event(binding, value));
    }

    Ok(())
}

/// Poll one Windows bluetooth scan event without blocking.
pub(crate) unsafe fn destack_device_bluetooth_scan_try_read_event(
    binding: &BindingCallContext,
    out: *mut BluetoothScanEvent,
    handle: resource::BluetoothScanHandle,
) -> DiagnosticResult<()> {
    core_platform::ensure_out(out, "out")?;

    // resolve the active scan session
    let resource = scan_resource(
        binding,
        handle,
        "destack.device.bluetooth.scan.tryReadEvent",
    )?;
    let value = resource.event_queue.try_pop_or_else(|| {
        Err(core_platform::io_would_block(
            "destack.device.bluetooth.scan.tryReadEvent",
            "bluetooth scan did not yield one event",
        ))
    })?;

    unsafe {
        out.write(stored_scan_event(binding, value));
    }

    Ok(())
}
