use super::SERIAL_PORT_RESOURCE_LABEL;
use super::config::*;
use super::core::*;
use super::identity::*;
use super::io::*;
use super::state::*;
#[cfg(test)]
use crate::platform::device::serial::{
    close_test_serial_resource, test_serial_config, test_serial_configure, test_serial_descriptor,
    test_serial_discard_input, test_serial_discard_output, test_serial_drain,
    test_serial_get_signals, test_serial_read_event, test_serial_read_into, test_serial_resource,
    test_serial_set_signals, test_serial_try_read_event, test_serial_try_read_into,
    test_serial_write, try_open_test_serial,
};
use crate::runtime::control::queue::BoundedQueue;

/// Close one serial endpoint.
pub(crate) unsafe fn destack_device_serial_close(
    binding: &BindingCallContext,
    handle: resource::SerialPortHandle,
) -> RuntimeResult<()> {
    #[cfg(test)]
    if test_serial_resource(binding, handle).is_some() {
        return close_test_serial_resource(binding, handle, "destack.device.serial.close");
    }

    close_serial_resource(binding, handle, "destack.device.serial.close")
}

/// Read serial endpoint configuration.
pub(crate) unsafe fn destack_device_serial_config(
    binding: &BindingCallContext,
    out: *mut SerialPortConfig,
    handle: resource::SerialPortHandle,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    #[cfg(test)]
    if let Some(resource) = test_serial_resource(binding, handle) {
        unsafe {
            out.write(test_serial_config(&resource));
        }

        return Ok(());
    }

    let resource = serial_resource(binding, handle, "destack.device.serial.config")?;
    let _operation_lock = resource.operation_lock.lock();
    let config = read_serial_config(resource.descriptor, "destack.device.serial.config")?;

    unsafe {
        out.write(config);
    }

    Ok(())
}

/// Reconfigure serial endpoint.
pub(crate) unsafe fn destack_device_serial_configure(
    binding: &BindingCallContext,
    handle: resource::SerialPortHandle,
    config: SerialPortConfig,
) -> RuntimeResult<()> {
    #[cfg(test)]
    if let Some(resource) = test_serial_resource(binding, handle) {
        return test_serial_configure(&resource, config);
    }

    let resource = serial_resource(binding, handle, "destack.device.serial.configure")?;
    let _operation_lock = resource.operation_lock.lock();

    apply_serial_config(resource.descriptor, config)
}

/// Read serial endpoint descriptor.
pub(crate) unsafe fn destack_device_serial_descriptor(
    binding: &BindingCallContext,
    out: *mut SerialPortDescriptor,
    handle: resource::SerialPortHandle,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    #[cfg(test)]
    if let Some(resource) = test_serial_resource(binding, handle) {
        unsafe {
            out.write(test_serial_descriptor(binding, &resource));
        }

        return Ok(());
    }

    let resource = serial_resource(binding, handle, "destack.device.serial.descriptor")?;
    let descriptor = serial_descriptor_from_info(binding, &resource.descriptor_info);

    unsafe {
        out.write(descriptor);
    }

    Ok(())
}

/// Discard queued inbound serial bytes.
pub(crate) unsafe fn destack_device_serial_discard_input(
    binding: &BindingCallContext,
    handle: resource::SerialPortHandle,
) -> RuntimeResult<()> {
    #[cfg(test)]
    if let Some(resource) = test_serial_resource(binding, handle) {
        test_serial_discard_input(&resource);

        return Ok(());
    }

    let resource = serial_resource(binding, handle, "destack.device.serial.discardInput")?;
    let _operation_lock = resource.operation_lock.lock();

    let status = unsafe { libc::tcflush(resource.descriptor, libc::TCIFLUSH) };
    if status < 0 {
        return Err(serial_io_error(
            "destack.device.serial.discardInput",
            "tcflush",
            "failed to discard queued serial input",
        ));
    }

    Ok(())
}

/// Discard queued outbound serial bytes.
pub(crate) unsafe fn destack_device_serial_discard_output(
    binding: &BindingCallContext,
    handle: resource::SerialPortHandle,
) -> RuntimeResult<()> {
    #[cfg(test)]
    if let Some(resource) = test_serial_resource(binding, handle) {
        test_serial_discard_output(&resource);

        return Ok(());
    }

    let resource = serial_resource(binding, handle, "destack.device.serial.discardOutput")?;
    let _operation_lock = resource.operation_lock.lock();

    let status = unsafe { libc::tcflush(resource.descriptor, libc::TCOFLUSH) };
    if status < 0 {
        return Err(serial_io_error(
            "destack.device.serial.discardOutput",
            "tcflush",
            "failed to discard queued serial output",
        ));
    }

    Ok(())
}

/// Drain serial output.
pub(crate) unsafe fn destack_device_serial_drain(
    binding: &BindingCallContext,
    handle: resource::SerialPortHandle,
) -> RuntimeResult<()> {
    #[cfg(test)]
    if let Some(resource) = test_serial_resource(binding, handle) {
        return test_serial_drain(&resource, "destack.device.serial.drain");
    }

    let resource = serial_resource(binding, handle, "destack.device.serial.drain")?;
    let _operation_lock = resource.operation_lock.lock();

    let status = unsafe { libc::tcdrain(resource.descriptor) };
    if status < 0 {
        return Err(serial_io_error(
            "destack.device.serial.drain",
            "tcdrain",
            "failed to drain serial output",
        ));
    }

    Ok(())
}

/// Read serial input signal state.
pub(crate) unsafe fn destack_device_serial_get_signals(
    binding: &BindingCallContext,
    out: *mut SerialInputSignals,
    handle: resource::SerialPortHandle,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    #[cfg(test)]
    if let Some(resource) = test_serial_resource(binding, handle) {
        unsafe {
            out.write(test_serial_get_signals(&resource));
        }

        return Ok(());
    }

    let resource = serial_resource(binding, handle, "destack.device.serial.getSignals")?;
    let signals = read_signals(resource.descriptor, "destack.device.serial.getSignals")?;

    unsafe {
        out.write(signals);
    }

    Ok(())
}

/// List serial endpoints.
pub(crate) unsafe fn destack_device_serial_list(
    binding: &BindingCallContext,
    out: *mut NativeSlice<SerialPortDescriptor>,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    let mut descriptors = Vec::new();

    // materialize one stable serial descriptor list from the current snapshot
    for info in serial_descriptor_snapshot("destack.device.serial.list")?.into_values() {
        descriptors.push(serial_descriptor_from_info(binding, &info));
    }

    let descriptors = binding.store_slice(descriptors);
    unsafe {
        out.write(descriptors);
    }

    Ok(())
}

/// Open serial endpoint.
pub(crate) unsafe fn destack_device_serial_open(
    binding: &BindingCallContext,
    out: *mut resource::SerialPortHandle,
    id: NativeStringRef,
    options: SerialPortOpenOptions,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    // validate caller-selected open policy before touching the host
    validate_open_options(&options)?;

    // decode one caller-selected identifier into one unix device path
    let id = unsafe { id.as_str()? };

    #[cfg(test)]
    if let Some(handle) = try_open_test_serial(binding, id)? {
        unsafe {
            out.write(handle);
        }

        return Ok(());
    }

    let path_bytes = decode_serial_id(id)?;
    let path = serial_path_cstring(&path_bytes, "id")?;

    // open one nonblocking serial descriptor with runtime ownership
    let descriptor = unsafe {
        libc::open(
            path.as_ptr(),
            libc::O_RDWR | libc::O_NOCTTY | libc::O_NONBLOCK | libc::O_CLOEXEC,
        )
    };
    if descriptor < 0 {
        return Err(serial_io_error(
            "destack.device.serial.open",
            "open",
            "failed to open serial endpoint",
        ));
    }

    // request one exclusive lane when the caller asked for it
    if options.exclusive == Some(true) {
        let exclusive_status = unsafe {
            libc::ioctl(
                descriptor,
                unix_ioctl_request(libc::TIOCEXCL as libc::c_ulong),
            )
        };
        if exclusive_status < 0 {
            let error = serial_io_error(
                "destack.device.serial.open",
                "ioctl(TIOCEXCL)",
                "failed to request exclusive serial access",
            );
            unsafe {
                libc::close(descriptor);
            }
            return Err(error);
        }
    }

    // apply one initial serial line configuration before registration
    if let Err(error) = apply_serial_config(descriptor, options.config) {
        unsafe {
            libc::close(descriptor);
        }
        return Err(error);
    }

    // build one stable descriptor snapshot from the opened device path
    let descriptor_info = descriptor_info_from_path(
        Path::new(OsStr::from_bytes(&path_bytes)),
        "destack.device.serial.open",
    );
    let last_signals = read_signals_optional(descriptor, "destack.device.serial.open")?;
    let event_queue = Arc::new(BoundedQueue::new(SERIAL_EVENT_QUEUE_CAPACITY));
    let resource = Arc::new(UnixSerialPortResource {
        descriptor,
        descriptor_info,
        operation_lock: parking_lot::Mutex::new(()),
        event_queue: Arc::clone(&event_queue),
        state: parking_lot::Mutex::new(UnixSerialPortState {
            next_sequence: 1,
            reported_dropped_count: 0,
            last_signals,
            is_read_ready_queued: false,
        }),
    });

    // start one dedicated session-event runtime before registration
    let event_runtime = match start_event_runtime(Arc::clone(&resource)) {
        Ok(event_runtime) => event_runtime,
        Err(error) => {
            unsafe {
                libc::close(descriptor);
            }

            return Err(error);
        }
    };

    // insert one serial session resource into the runtime table
    let entry = ResourceEntry::new(ResourceKind::SerialPort)
        .with_label(SERIAL_PORT_RESOURCE_LABEL)
        .with_payload(Arc::clone(&resource))
        .with_finalizer(binding.worker().platform_state.device.wrap_finalizer(
            UnixSerialDescriptorFinalizer {
                runtime: event_runtime,
                descriptor,
                event_queue,
            },
        ));
    let resource_id =
        binding
            .worker()
            .resources
            .insert(&binding.world(), entry, Some(binding.engine()));

    unsafe {
        out.write(resource::SerialPortHandle(resource_id));
    }

    Ok(())
}

/// Wait for one serial event.
pub(crate) unsafe fn destack_device_serial_read_event(
    binding: &BindingCallContext,
    out: *mut SerialEvent,
    handle: resource::SerialPortHandle,
    timeoutns: u64,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    #[cfg(test)]
    if let Some(resource) = test_serial_resource(binding, handle) {
        let event = test_serial_read_event(
            binding,
            &resource,
            timeoutns,
            "destack.device.serial.readEvent",
        )?;

        unsafe {
            out.write(event);
        }

        return Ok(());
    }

    let resource = serial_resource(binding, handle, "destack.device.serial.readEvent")?;
    let event = wait_serial_event(
        binding,
        &resource,
        timeoutns,
        "destack.device.serial.readEvent",
    )?;

    unsafe {
        out.write(event);
    }

    Ok(())
}

/// Read serial bytes.
pub(crate) unsafe fn destack_device_serial_read_into(
    binding: &BindingCallContext,
    out: *mut u64,
    handle: resource::SerialPortHandle,
    buffer: NativeSlice<u8>,
    timeoutns: u64,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    #[cfg(test)]
    if let Some(resource) = test_serial_resource(binding, handle) {
        let read = test_serial_read_into(
            &resource,
            buffer,
            timeoutns,
            "destack.device.serial.readInto",
        )?;

        unsafe {
            out.write(read);
        }

        return Ok(());
    }

    let resource = serial_resource(binding, handle, "destack.device.serial.readInto")?;
    let read = read_serial_bytes(
        &resource,
        buffer,
        timeoutns,
        "destack.device.serial.readInto",
    )?;

    unsafe {
        out.write(read);
    }

    Ok(())
}

/// Update serial output signal state.
pub(crate) unsafe fn destack_device_serial_set_signals(
    binding: &BindingCallContext,
    handle: resource::SerialPortHandle,
    signals: SerialOutputSignals,
) -> RuntimeResult<()> {
    #[cfg(test)]
    if let Some(resource) = test_serial_resource(binding, handle) {
        return test_serial_set_signals(&resource, signals, "destack.device.serial.setSignals");
    }

    let resource = serial_resource(binding, handle, "destack.device.serial.setSignals")?;
    let _operation_lock = resource.operation_lock.lock();

    apply_output_signals(
        resource.descriptor,
        signals,
        "destack.device.serial.setSignals",
    )
}

/// Poll one serial event without blocking.
pub(crate) unsafe fn destack_device_serial_try_read_event(
    binding: &BindingCallContext,
    out: *mut SerialEvent,
    handle: resource::SerialPortHandle,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    #[cfg(test)]
    if let Some(resource) = test_serial_resource(binding, handle) {
        let event =
            test_serial_try_read_event(binding, &resource, "destack.device.serial.tryReadEvent")?;

        unsafe {
            out.write(event);
        }

        return Ok(());
    }

    let resource = serial_resource(binding, handle, "destack.device.serial.tryReadEvent")?;
    let event = wait_serial_event(binding, &resource, 0, "destack.device.serial.tryReadEvent")?;

    unsafe {
        out.write(event);
    }

    Ok(())
}

/// Poll serial bytes without blocking.
pub(crate) unsafe fn destack_device_serial_try_read_into(
    binding: &BindingCallContext,
    out: *mut u64,
    handle: resource::SerialPortHandle,
    buffer: NativeSlice<u8>,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    #[cfg(test)]
    if let Some(resource) = test_serial_resource(binding, handle) {
        let read =
            test_serial_try_read_into(&resource, buffer, "destack.device.serial.tryReadInto")?;

        unsafe {
            out.write(read);
        }

        return Ok(());
    }

    let resource = serial_resource(binding, handle, "destack.device.serial.tryReadInto")?;
    let read = try_read_serial_bytes(&resource, buffer, "destack.device.serial.tryReadInto")?;

    unsafe {
        out.write(read);
    }

    Ok(())
}

/// Write serial bytes.
pub(crate) unsafe fn destack_device_serial_write(
    binding: &BindingCallContext,
    out: *mut u64,
    handle: resource::SerialPortHandle,
    data: NativeSlice<u8>,
    timeoutns: u64,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    #[cfg(test)]
    if let Some(resource) = test_serial_resource(binding, handle) {
        let written = test_serial_write(&resource, data, "destack.device.serial.write")?;

        unsafe {
            out.write(written);
        }

        return Ok(());
    }

    let resource = serial_resource(binding, handle, "destack.device.serial.write")?;
    let written = write_serial_bytes(&resource, data, timeoutns, "destack.device.serial.write")?;

    unsafe {
        out.write(written);
    }

    Ok(())
}
