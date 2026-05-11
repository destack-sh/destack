#![allow(unsafe_op_in_unsafe_fn)]

use super::core::*;
use super::delegates::*;
use super::metadata::*;
use super::runtime::*;

/// Validate one CoreBluetooth scan filter against the exposed host surface.
fn validate_scan_filter(filter: &BluetoothScanFilterValue) -> RuntimeResult<()> {
    // corebluetooth does not expose explicit active or passive scan selection
    if filter.scan_mode.is_some() {
        return Err(core_platform::invalid_argument(
            "filter.scanMode",
            "corebluetooth does not expose passive or active scan selection",
        ));
    }

    // corebluetooth does not expose explicit phy selection on central scans
    if filter.primary_phy.is_some() {
        return Err(core_platform::invalid_argument(
            "filter.primaryPhy",
            "corebluetooth does not expose scan primary PHY selection",
        ));
    }

    // corebluetooth does not expose explicit phy selection on central scans
    if filter.secondary_phy.is_some() {
        return Err(core_platform::invalid_argument(
            "filter.secondaryPhy",
            "corebluetooth does not expose scan secondary PHY selection",
        ));
    }

    Ok(())
}

/// List the macOS bluetooth adapters exposed to the runtime.
pub(crate) unsafe fn destack_device_bluetooth_adapter_list(
    binding: &BindingCallContext,
    out: *mut NativeSlice<BluetoothAdapterDescriptor>,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;
    require_bluetooth_authorization("destack.device.bluetooth.adapterList")?;

    let state_queue = Arc::new(BoundedQueue::new(8));
    let delegate = MacBluetoothScanDelegate::new(
        state_queue.clone(),
        None,
        Arc::new(BoundedQueue::new(1)),
        Arc::new(Mutex::new(BluetoothEventState { next_sequence: 1 })),
    );
    let central = central_manager_with_delegate(delegate.as_protocol());
    wait_for_central_ready(
        &central,
        &state_queue,
        "destack.device.bluetooth.adapterList",
    )?;
    let powered =
        central.dispatch(|central| unsafe { central.state() } == CBManagerState::PoweredOn);

    out.write(adapter_list(binding, powered));

    Ok(())
}

/// Open one Bluetooth scan session.
pub(crate) unsafe fn destack_device_bluetooth_scan_open(
    binding: &BindingCallContext,
    out: *mut resource::BluetoothScanHandle,
    adapterid: NativeStringRef,
    filter: Option<BluetoothScanFilter>,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    let adapter_id = adapterid.as_str()?;
    require_adapter_id(adapter_id, "destack.device.bluetooth.scan.open")?;
    let filter = <Option<BluetoothScanFilter> as NativeAbiCodec>::into_value(filter)?;
    if let Some(filter) = filter.as_ref() {
        validate_scan_filter(filter)?;
    }

    let state_queue = Arc::new(BoundedQueue::new(16));
    let event_queue = Arc::new(BoundedQueue::new(BLUETOOTH_SCAN_EVENT_QUEUE_CAPACITY));
    let event_state = Arc::new(Mutex::new(BluetoothEventState { next_sequence: 1 }));
    let delegate = MacBluetoothScanDelegate::new(
        state_queue.clone(),
        filter.clone(),
        event_queue.clone(),
        event_state,
    );
    let central = central_manager_with_delegate(delegate.as_protocol());
    wait_for_central_ready(&central, &state_queue, "destack.device.bluetooth.scan.open")?;

    let service_uuid_strings = filter.as_ref().map(|filter| filter.service_uuids.clone());
    let allows_duplicates = scan_allows_duplicates(filter.as_ref());
    central.dispatch(move |central| unsafe {
        let service_filter = service_uuid_strings
            .as_ref()
            .and_then(|service_uuid_strings| {
                if service_uuid_strings.is_empty() {
                    return None;
                }

                let uuids = service_uuid_strings
                    .iter()
                    .map(|service_uuid| CBUUID::UUIDWithString(&NSString::from_str(service_uuid)))
                    .collect::<Vec<_>>();

                Some(NSArray::from_retained_slice(&uuids))
            });
        let options = NSDictionary::from_retained_objects(
            &[CBCentralManagerScanOptionAllowDuplicatesKey],
            &[Retained::cast_unchecked::<AnyObject>(NSNumber::new_bool(
                allows_duplicates,
            ))],
        );

        central.scanForPeripheralsWithServices_options(service_filter.as_deref(), Some(&options));
    });

    let resource = Arc::new(MacBluetoothScanResource {
        event_queue: event_queue.clone(),
    });

    let entry = ResourceEntry::new(ResourceKind::BluetoothScan)
        .with_label(BLUETOOTH_SCAN_RESOURCE_LABEL)
        .with_payload(resource.clone())
        .with_finalizer(binding.worker().platform_state.device.wrap_finalizer(
            MacBluetoothScanFinalizer {
                central,
                event_queue,
            },
        ));
    let resource_id =
        binding
            .worker()
            .resources
            .insert(binding.world(), entry, Some(binding.engine()));

    out.write(resource::BluetoothScanHandle(resource_id));

    Ok(())
}

/// Close one Bluetooth scan session.
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

/// Read one blocking scan event.
pub(crate) unsafe fn destack_device_bluetooth_scan_read_event(
    binding: &BindingCallContext,
    out: *mut BluetoothScanEvent,
    handle: resource::BluetoothScanHandle,
    timeoutns: u64,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    let resource = scan_resource(binding, handle, "destack.device.bluetooth.scan.readEvent")?;
    out.write(pop_scan_event(
        binding,
        &resource,
        timeoutns,
        "destack.device.bluetooth.scan.readEvent",
    )?);

    Ok(())
}

/// Poll one scan event without blocking.
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
    out.write(stored_scan_event(binding, value));

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::device::BluetoothScanMode;

    /// Reject one CoreBluetooth scan filter that requests one explicit scan mode.
    #[test]
    fn test_validate_scan_filter_rejects_scan_mode() {
        let filter = BluetoothScanFilterValue {
            service_uuids: Vec::new(),
            name: None,
            name_prefix: None,
            manufacturer_data: Vec::new(),
            service_data: Vec::new(),
            keep_repeated_devices: false,
            minimum_rssi: None,
            scan_mode: Some(BluetoothScanMode::Active),
            primary_phy: None,
            secondary_phy: None,
        };

        let result = validate_scan_filter(&filter);

        assert!(result.is_err());
    }
}
