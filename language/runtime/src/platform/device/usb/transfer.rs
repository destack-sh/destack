use super::core::*;
use super::descriptor::*;
use super::ffi::ffi;
use super::service::{
    UsbAsyncTransferKind, UsbDeviceResource, UsbHotplugEventRecord, UsbServiceState,
    UsbTransferWaiter, UsbWatchEventKind, UsbWatchResource, libusb_error, queue_usb_hotplug_event,
    register_active_transfer, transfer_status_from_libusb_result, unregister_active_transfer,
};

/// Execute one synchronous inbound transfer.
pub(crate) fn execute_in_transfer(
    resource: &UsbDeviceResource,
    endpoint: &UsbEndpointSelectorValue,
    max_bytes: u32,
    timeout_ns: u64,
    operation: &'static str,
    kind: UsbAsyncTransferKind,
) -> RuntimeResult<UsbInTransferResultValue> {
    let length = usize::try_from(max_bytes).map_err(|_| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            "maxBytes",
            "maxBytes exceeded the host usize range",
        ))
        .boxed()
    })?;
    let endpoint_address = u8::try_from(endpoint_address(endpoint)).unwrap_or(0);
    let (status, bytes_written, bytes) = execute_async_endpoint_transfer(
        resource,
        endpoint_address,
        vec![0u8; length],
        timeout_ns,
        operation,
        kind,
    )?;

    Ok(UsbInTransferResultValue {
        status,
        bytes: bytes[..bytes_written as usize].to_vec(),
    })
}

/// Execute one synchronous outbound transfer.
pub(crate) fn execute_out_transfer(
    resource: &UsbDeviceResource,
    endpoint: &UsbEndpointSelectorValue,
    bytes: &[u8],
    timeout_ns: u64,
    operation: &'static str,
    kind: UsbAsyncTransferKind,
) -> RuntimeResult<UsbOutTransferResultValue> {
    let endpoint_address = u8::try_from(endpoint_address(endpoint)).unwrap_or(0);
    let (status, bytes_written, _) = execute_async_endpoint_transfer(
        resource,
        endpoint_address,
        bytes.to_vec(),
        timeout_ns,
        operation,
        kind,
    )?;

    Ok(UsbOutTransferResultValue {
        status,
        bytes_written,
    })
}

/// Execute one one-shot asynchronous bulk or interrupt transfer.
fn execute_async_endpoint_transfer(
    resource: &UsbDeviceResource,
    endpoint_address: u8,
    mut bytes: Vec<u8>,
    timeout_ns: u64,
    operation: &'static str,
    kind: UsbAsyncTransferKind,
) -> RuntimeResult<(UsbTransferStatus, u32, Vec<u8>)> {
    let transfer = unsafe { (resource.service.api.libusb_alloc_transfer)(0) };
    let transfer = NonNull::new(transfer).ok_or_else(|| {
        RuntimeError::from(PlatformError::io(format!(
            "{operation}: failed to allocate libusb transfer",
        )))
        .boxed()
    })?;

    let waiter = UsbTransferWaiter::new();
    let waiter_pointer = Box::into_raw(Box::new(waiter.clone()));
    let timeout_ms = timeout_ns_to_millis(timeout_ns);
    let transfer_type = match kind {
        UsbAsyncTransferKind::Bulk => ffi::LIBUSB_TRANSFER_TYPE_BULK,
        UsbAsyncTransferKind::Interrupt => ffi::LIBUSB_TRANSFER_TYPE_INTERRUPT,
    };

    // initialize one explicit transfer object because the helper is inline in libusb
    unsafe {
        let transfer_ref = transfer.as_ptr();
        (*transfer_ref).dev_handle = resource.handle;
        (*transfer_ref).flags = ffi::LIBUSB_TRANSFER_FREE_BUFFER;
        (*transfer_ref).endpoint = endpoint_address;
        (*transfer_ref).transfer_type = transfer_type;
        (*transfer_ref).timeout = timeout_ms;
        (*transfer_ref).status = 0;
        (*transfer_ref).length = bytes.len() as c_int;
        (*transfer_ref).actual_length = 0;
        (*transfer_ref).callback = Some(usb_transfer_complete_callback);
        (*transfer_ref).user_data = waiter_pointer.cast::<c_void>();
        (*transfer_ref).buffer = bytes.as_mut_ptr();
        (*transfer_ref).num_iso_packets = 0;
    }

    std::mem::forget(bytes);

    let submit_status = {
        let _operation_lock = resource.operation_lock.lock();
        register_active_transfer(resource, endpoint_address, transfer);

        let submit_status =
            unsafe { (resource.service.api.libusb_submit_transfer)(transfer.as_ptr()) };
        if submit_status != ffi::LIBUSB_SUCCESS {
            unregister_active_transfer(resource, endpoint_address, transfer.as_ptr());
        }

        submit_status
    };
    if submit_status != ffi::LIBUSB_SUCCESS {
        unsafe {
            drop(Box::from_raw(waiter_pointer));
            (resource.service.api.libusb_free_transfer)(transfer.as_ptr());
        }

        return Err(libusb_error(
            operation,
            "libusb_submit_transfer",
            submit_status,
        ));
    }

    waiter.wait();
    unregister_active_transfer(resource, endpoint_address, transfer.as_ptr());

    let transfer_status = unsafe { (*transfer.as_ptr()).status };
    if transfer_status == ffi::LIBUSB_TRANSFER_TIMED_OUT {
        unsafe {
            (resource.service.api.libusb_free_transfer)(transfer.as_ptr());
        }

        return Err(core_platform::io_would_block(
            operation,
            "usb transfer timed out",
        ));
    }
    if transfer_status == ffi::LIBUSB_TRANSFER_CANCELLED {
        unsafe {
            (resource.service.api.libusb_free_transfer)(transfer.as_ptr());
        }

        return Err(core_platform::io_operation_error(
            operation,
            Some(PlatformErrorCode::IoInterrupted),
            "usb transfer was cancelled",
        ));
    }
    if transfer_status == ffi::LIBUSB_TRANSFER_NO_DEVICE {
        unsafe {
            (resource.service.api.libusb_free_transfer)(transfer.as_ptr());
        }

        return Err(core_platform::io_not_found(
            operation,
            "usb device disconnected during one transfer",
        ));
    }

    let transfer_status = transfer_status_from_libusb_async_status(transfer_status)
        .ok_or_else(|| libusb_error(operation, "libusb transfer", transfer_status))?;
    let bytes_written = unsafe { (*transfer.as_ptr()).actual_length.max(0) as u32 };
    let result_bytes = unsafe {
        let transfer_ref = transfer.as_ptr();
        std::slice::from_raw_parts(
            (*transfer_ref).buffer,
            (*transfer_ref).length.max(0) as usize,
        )
        .to_vec()
    };

    unsafe {
        (resource.service.api.libusb_free_transfer)(transfer.as_ptr());
    }

    Ok((transfer_status, bytes_written, result_bytes))
}

/// Execute one control read transfer.
pub(crate) fn execute_control_read(
    resource: &UsbDeviceResource,
    setup: &UsbControlSetupValue,
    timeout_ns: u64,
    operation: &'static str,
) -> RuntimeResult<UsbInTransferResultValue> {
    let mut buffer = vec![0u8; usize::from(setup.length)];
    let request_type = request_type_bits(setup, UsbEndpointDirection::In);
    let request_index = request_index(setup);
    let timeout_ms = timeout_ns_to_millis(timeout_ns);

    let _operation_lock = resource.operation_lock.lock();

    let status = unsafe {
        (resource.service.api.libusb_control_transfer)(
            resource.handle,
            request_type,
            setup.request,
            setup.value,
            request_index,
            buffer.as_mut_ptr(),
            setup.length,
            timeout_ms,
        )
    };

    if let Some(transfer_status) = transfer_status_from_libusb_result(status) {
        buffer.truncate(status.max(0) as usize);
        return Ok(UsbInTransferResultValue {
            status: transfer_status,
            bytes: buffer,
        });
    }

    Err(libusb_error(operation, "libusb_control_transfer", status))
}

/// Execute one control write transfer.
pub(crate) fn execute_control_write(
    resource: &UsbDeviceResource,
    setup: &UsbControlSetupValue,
    bytes: &[u8],
    timeout_ns: u64,
    operation: &'static str,
) -> RuntimeResult<UsbOutTransferResultValue> {
    if usize::from(setup.length) != bytes.len() {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "bytes",
            "control write payload length must match setup.length",
        ))
        .boxed());
    }

    let request_type = request_type_bits(setup, UsbEndpointDirection::Out);
    let request_index = request_index(setup);
    let timeout_ms = timeout_ns_to_millis(timeout_ns);
    let mut bytes = bytes.to_vec();

    let _operation_lock = resource.operation_lock.lock();

    let status = unsafe {
        (resource.service.api.libusb_control_transfer)(
            resource.handle,
            request_type,
            setup.request,
            setup.value,
            request_index,
            bytes.as_mut_ptr(),
            setup.length,
            timeout_ms,
        )
    };

    if let Some(transfer_status) = transfer_status_from_libusb_result(status) {
        return Ok(UsbOutTransferResultValue {
            status: transfer_status,
            bytes_written: status.max(0) as u32,
        });
    }

    Err(libusb_error(operation, "libusb_control_transfer", status))
}

/// Execute one one-shot isochronous transfer.
pub(crate) fn execute_isochronous_transfer(
    resource: &UsbDeviceResource,
    endpoint: &UsbEndpointSelectorValue,
    bytes: &[u8],
    packet_sizes: &[u32],
    timeout_ns: u64,
    operation: &'static str,
) -> RuntimeResult<UsbIsochronousTransferResultValue> {
    let packet_count = c_int::try_from(packet_sizes.len()).map_err(|_| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            "packetSizes",
            "packetSizes length exceeded the host c_int range",
        ))
        .boxed()
    })?;
    let total_length = packet_sizes.iter().try_fold(0usize, |accumulator, size| {
        accumulator.checked_add(*size as usize).ok_or_else(|| {
            RuntimeError::from(PlatformError::invalid_argument_value(
                "packetSizes",
                "packetSizes total length overflowed",
            ))
            .boxed()
        })
    })?;

    if bytes.len() != total_length {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "bytes",
            "isochronous payload length must match the sum of packetSizes",
        ))
        .boxed());
    }

    let transfer = unsafe { (resource.service.api.libusb_alloc_transfer)(packet_count) };
    if transfer.is_null() {
        return Err(RuntimeError::from(PlatformError::io(
            "destack.device.usb.isochronousTransfer: failed to allocate libusb transfer",
        ))
        .boxed());
    }

    let waiter = UsbTransferWaiter::new();
    let waiter_pointer = Box::into_raw(Box::new(waiter.clone()));
    let timeout_ms = timeout_ns_to_millis(timeout_ns);
    let endpoint_address = u8::try_from(endpoint_address(endpoint)).unwrap_or(0);
    let mut buffer = bytes.to_vec();

    // initialize one explicit transfer object because the helper is inline in libusb
    unsafe {
        (*transfer).dev_handle = resource.handle;
        (*transfer).flags = ffi::LIBUSB_TRANSFER_FREE_BUFFER;
        (*transfer).endpoint = endpoint_address;
        (*transfer).transfer_type = ffi::LIBUSB_TRANSFER_TYPE_ISOCHRONOUS;
        (*transfer).timeout = timeout_ms;
        (*transfer).status = 0;
        (*transfer).length = total_length as c_int;
        (*transfer).actual_length = 0;
        (*transfer).callback = Some(usb_transfer_complete_callback);
        (*transfer).user_data = waiter_pointer.cast::<c_void>();
        (*transfer).buffer = buffer.as_mut_ptr();
        (*transfer).num_iso_packets = packet_count;

        for (index, packet_size) in packet_sizes.iter().enumerate() {
            let descriptor = &mut *(*transfer).iso_packet_desc.as_mut_ptr().add(index);
            descriptor.length = *packet_size;
            descriptor.actual_length = 0;
            descriptor.status = 0;
        }
    }

    std::mem::forget(buffer);

    let submit_status = {
        let _operation_lock = resource.operation_lock.lock();
        let transfer = NonNull::new(transfer).ok_or_else(|| {
            RuntimeError::from(PlatformError::io(format!(
                "{operation}: libusb_alloc_transfer returned one null transfer",
            )))
            .boxed()
        })?;
        register_active_transfer(resource, endpoint_address, transfer);

        let submit_status =
            unsafe { (resource.service.api.libusb_submit_transfer)(transfer.as_ptr()) };
        if submit_status != ffi::LIBUSB_SUCCESS {
            unregister_active_transfer(resource, endpoint_address, transfer.as_ptr());
        }

        submit_status
    };
    if submit_status != ffi::LIBUSB_SUCCESS {
        unsafe {
            drop(Box::from_raw(waiter_pointer));
            (resource.service.api.libusb_free_transfer)(transfer);
        }

        return Err(libusb_error(
            operation,
            "libusb_submit_transfer",
            submit_status,
        ));
    }

    waiter.wait();
    unregister_active_transfer(resource, endpoint_address, transfer);

    let transfer_status = unsafe { (*transfer).status };
    if transfer_status == ffi::LIBUSB_TRANSFER_TIMED_OUT {
        unsafe {
            (resource.service.api.libusb_free_transfer)(transfer);
        }

        return Err(core_platform::io_would_block(
            operation,
            "usb isochronous transfer timed out",
        ));
    }
    if transfer_status == ffi::LIBUSB_TRANSFER_CANCELLED {
        unsafe {
            (resource.service.api.libusb_free_transfer)(transfer);
        }

        return Err(core_platform::io_operation_error(
            operation,
            Some(PlatformErrorCode::IoInterrupted),
            "usb isochronous transfer was cancelled",
        ));
    }
    if transfer_status == ffi::LIBUSB_TRANSFER_NO_DEVICE {
        unsafe {
            (resource.service.api.libusb_free_transfer)(transfer);
        }

        return Err(core_platform::io_not_found(
            operation,
            "usb device disconnected during one isochronous transfer",
        ));
    }

    let packet_count = unsafe { (*transfer).num_iso_packets.max(0) as usize };
    let mut packets = Vec::with_capacity(packet_count);
    let mut offsets = Vec::with_capacity(packet_count);
    let mut offset = 0usize;

    unsafe {
        for index in 0..packet_count {
            let descriptor = &*(*transfer).iso_packet_desc.as_ptr().add(index);
            let packet_status = transfer_status_from_libusb_async_status(descriptor.status)
                .ok_or_else(|| {
                    libusb_error(operation, "libusb isochronous packet", descriptor.status)
                })?;
            packets.push(UsbIsochronousPacketResultValue {
                status: packet_status,
                actual_length: descriptor.actual_length,
            });
            offsets.push((offset, descriptor.actual_length as usize));
            offset = offset.saturating_add(descriptor.length as usize);
        }
    }

    let result_bytes = unsafe {
        let transfer_buffer =
            std::slice::from_raw_parts((*transfer).buffer, (*transfer).length.max(0) as usize);
        let mut output = Vec::new();
        for (offset, length) in offsets {
            output.extend_from_slice(&transfer_buffer[offset..offset.saturating_add(length)]);
        }
        output
    };

    unsafe {
        (resource.service.api.libusb_free_transfer)(transfer);
    }

    Ok(UsbIsochronousTransferResultValue {
        bytes: result_bytes,
        packets,
    })
}

/// One-shot libusb transfer completion callback.
#[cfg(windows)]
unsafe extern "system" fn usb_transfer_complete_callback(transfer: *mut ffi::LibusbTransfer) {
    let waiter = unsafe { Box::from_raw((*transfer).user_data.cast::<Arc<UsbTransferWaiter>>()) };
    waiter.complete();
}

/// One-shot libusb transfer completion callback.
#[cfg(not(windows))]
unsafe extern "C" fn usb_transfer_complete_callback(transfer: *mut ffi::LibusbTransfer) {
    let waiter = unsafe { Box::from_raw((*transfer).user_data.cast::<Arc<UsbTransferWaiter>>()) };
    waiter.complete();
}

/// Dispatch one libusb hotplug callback on Windows.
#[cfg(windows)]
pub(crate) unsafe extern "system" fn usb_hotplug_callback(
    _context: *mut ffi::LibusbContext,
    device: *mut ffi::LibusbDevice,
    event: c_int,
    user_data: *mut c_void,
) -> c_int {
    unsafe { handle_usb_hotplug_callback(device, event, user_data) }
}

/// Dispatch one libusb hotplug callback on non-Windows platforms.
#[cfg(not(windows))]
pub(crate) unsafe extern "C" fn usb_hotplug_callback(
    _context: *mut ffi::LibusbContext,
    device: *mut ffi::LibusbDevice,
    event: c_int,
    user_data: *mut c_void,
) -> c_int {
    unsafe { handle_usb_hotplug_callback(device, event, user_data) }
}

/// Handle one libusb hotplug callback.
unsafe fn handle_usb_hotplug_callback(
    device: *mut ffi::LibusbDevice,
    event: c_int,
    user_data: *mut c_void,
) -> c_int {
    let service = unsafe { &*user_data.cast::<Arc<UsbServiceState>>() };
    let kind = match event {
        ffi::LIBUSB_HOTPLUG_EVENT_DEVICE_ARRIVED => UsbWatchEventKind::Instance,
        ffi::LIBUSB_HOTPLUG_EVENT_DEVICE_LEFT => UsbWatchEventKind::Detached,
        _ => return 0,
    };

    let descriptor =
        match read_hotplug_descriptor_value(service, device, "destack.device.usb.watchOpen") {
            Ok(descriptor) => descriptor,
            Err(error) => {
                tracing::warn!("usb hotplug callback dropped one event: {error}");
                return 0;
            }
        };

    let descriptor = {
        let mut snapshot = service.watch_snapshot.lock();
        match kind {
            UsbWatchEventKind::Instance => {
                snapshot.insert(descriptor.id.clone(), descriptor.clone());
                descriptor
            }
            UsbWatchEventKind::Detached => snapshot.remove(&descriptor.id).unwrap_or(descriptor),
        }
    };

    queue_usb_hotplug_event(
        service,
        UsbHotplugEventRecord {
            timestamp_ns: core_platform::monotonic_now_ns(),
            kind,
            descriptor,
        },
    );

    0
}

/// Convert one queued watch record into one binding event.
pub(crate) fn hotplug_event_from_record(
    binding: &BindingCallContext,
    watch: &UsbWatchResource,
    record: UsbHotplugEventRecord,
) -> UsbHotplugEvent {
    let sequence = next_watch_sequence(&watch.state);

    let metadata_value = UsbHotplugEventMetadataValue {
        timestamp_ns: record.timestamp_ns,
        sequence,
        device: record.descriptor,
    };
    let metadata = UsbHotplugEventMetadata::from_value(binding, metadata_value);

    match record.kind {
        UsbWatchEventKind::Instance => {
            UsbHotplugEvent::UsbHotplugAttachedEvent(UsbHotplugAttachedEvent {
                kind: binding.store_string("attached"),
                metadata,
            })
        }
        UsbWatchEventKind::Detached => {
            UsbHotplugEvent::UsbHotplugDetachedEvent(UsbHotplugDetachedEvent {
                kind: binding.store_string("detached"),
                metadata,
            })
        }
    }
}

/// Return one watch read timeout error.
#[cfg(not(target_os = "android"))]
pub(crate) fn usb_watch_would_block(operation: &'static str) -> Box<RuntimeError> {
    core_platform::io_would_block(operation, "usb watch read would block")
}
