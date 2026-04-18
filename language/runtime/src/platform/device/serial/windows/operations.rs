use super::super::core::overflow_event;
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
    let config = read_serial_config(resource.handle, "destack.device.serial.config")?;

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

    apply_serial_config(resource.handle, config)
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

    let status = unsafe { PurgeComm(resource.handle, PURGE_RXABORT | PURGE_RXCLEAR) };
    if status == 0 {
        return Err(serial_io_error(
            "destack.device.serial.discardInput",
            "PurgeComm",
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

    let status = unsafe { PurgeComm(resource.handle, PURGE_TXABORT | PURGE_TXCLEAR) };
    if status == 0 {
        return Err(serial_io_error(
            "destack.device.serial.discardOutput",
            "PurgeComm",
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

    let status = unsafe { FlushFileBuffers(resource.handle) };
    if status == 0 {
        return Err(serial_io_error(
            "destack.device.serial.drain",
            "FlushFileBuffers",
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
    let signals = read_signals(resource.handle, "destack.device.serial.getSignals")?;

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

    // decode the selected COM identifier and open the host endpoint
    let id = unsafe { id.as_str()? };

    #[cfg(test)]
    if let Some(handle) = try_open_test_serial(binding, id)? {
        unsafe {
            out.write(handle);
        }

        return Ok(());
    }

    let port_number = decode_serial_port_number(id)?;
    let port_name = serial_port_name(port_number);
    let descriptor_info = descriptor_info_for_port_name(&port_name, "destack.device.serial.open")?;
    let handle = open_serial_handle(&port_name)?;

    // apply caller-selected queue hints when present
    if options.read_buffer_size.is_some() || options.write_buffer_size.is_some() {
        let read_buffer_size = options
            .read_buffer_size
            .unwrap_or(DEFAULT_SERIAL_BUFFER_SIZE);
        let write_buffer_size = options
            .write_buffer_size
            .unwrap_or(DEFAULT_SERIAL_BUFFER_SIZE);
        let status = unsafe { SetupComm(handle, read_buffer_size, write_buffer_size) };
        if status == 0 {
            unsafe {
                windows_sys::Win32::Foundation::CloseHandle(handle);
            }

            return Err(serial_io_error(
                "destack.device.serial.open",
                "SetupComm",
                "failed to configure serial queue sizes",
            ));
        }
    }

    // install one nonblocking-read timeout profile before configuration
    if let Err(error) = apply_serial_timeouts(handle, None, "destack.device.serial.open") {
        unsafe {
            windows_sys::Win32::Foundation::CloseHandle(handle);
        }

        return Err(error);
    }

    // apply one initial line configuration before registration
    if let Err(error) = apply_serial_config(handle, options.config) {
        unsafe {
            windows_sys::Win32::Foundation::CloseHandle(handle);
        }

        return Err(error);
    }

    let last_signals = read_signals_optional(handle, "destack.device.serial.open")?;
    let resource = Arc::new(WindowsSerialPortResource {
        handle,
        descriptor_info,
        operation_lock: parking_lot::Mutex::new(()),
        event_queue: BoundedQueue::new(SERIAL_EVENT_QUEUE_CAPACITY),
        state: parking_lot::Mutex::new(WindowsSerialPortState {
            next_sequence: 1,
            reported_dropped_count: 0,
            last_signals,
            is_read_ready_queued: false,
        }),
    });

    // start one service-owned comm-event runtime for this handle
    let serial_service = binding
        .worker()
        .platform_state
        .device
        .windows_serial_service("destack.device.serial.open")?;
    let registration_id =
        match serial_service.start_event_runtime(Arc::clone(&resource), &port_name) {
            Ok(registration_id) => registration_id,
            Err(error) => {
                unsafe {
                    windows_sys::Win32::Foundation::CloseHandle(handle);
                }

                return Err(error);
            }
        };

    // insert one serial session resource into the runtime table
    let entry = ResourceEntry::new(ResourceKind::SerialPort)
        .with_label(SERIAL_PORT_RESOURCE_LABEL)
        .with_payload(Arc::clone(&resource))
        .with_finalizer(binding.worker().platform_state.device.wrap_finalizer(
            WindowsSerialEventFinalizer {
                service: serial_service,
                registration_id,
                resource: Arc::clone(&resource),
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

/// Wait for one serial event payload.
pub(crate) unsafe fn destack_device_serial_read_event(
    binding: &BindingCallContext,
    out: *mut SerialEvent,
    handle: resource::SerialPortHandle,
    timeout_ns: u64,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    #[cfg(test)]
    if let Some(resource) = test_serial_resource(binding, handle) {
        let event = test_serial_read_event(
            binding,
            &resource,
            timeout_ns,
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
        timeout_ns,
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
    timeout_ns: u64,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    #[cfg(test)]
    if let Some(resource) = test_serial_resource(binding, handle) {
        let read = test_serial_read_into(
            &resource,
            buffer,
            timeout_ns,
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
        timeout_ns,
        "destack.device.serial.readInto",
    )?;

    unsafe {
        out.write(read);
    }

    Ok(())
}

/// Update serial output lines.
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

    // apply the requested signal deltas one line at a time
    let updates: [(Option<bool>, ESCAPE_COMM_FUNCTION, ESCAPE_COMM_FUNCTION); 3] = [
        (signals.data_terminal_ready, SETDTR, CLRDTR),
        (signals.request_to_send, SETRTS, CLRRTS),
        (signals.break_condition_active, SETBREAK, CLRBREAK),
    ];
    for (requested_state, enable_fn, disable_fn) in updates {
        let Some(requested_state) = requested_state else {
            continue;
        };

        let function = if requested_state {
            enable_fn
        } else {
            disable_fn
        };
        let status = unsafe { EscapeCommFunction(resource.handle, function) };
        if status == 0 {
            return Err(serial_io_error(
                "destack.device.serial.setSignals",
                "EscapeCommFunction",
                "failed to update serial output line state",
            ));
        }
    }

    Ok(())
}

/// Try to read one serial event without blocking.
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
    if let Some(dropped_count) = take_event_overflow_count(&resource) {
        let sequence = next_event_sequence(&resource);
        let event = overflow_event(binding, sequence, dropped_count);

        unsafe {
            out.write(event);
        }

        return Ok(());
    }

    let record = resource
        .event_queue
        .try_pop()
        .ok_or_else(|| serial_event_would_block("destack.device.serial.tryReadEvent"))?;
    let event = event_from_record(binding, &resource, record);

    unsafe {
        out.write(event);
    }

    Ok(())
}

/// Try to read serial bytes without blocking.
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
    timeout_ns: u64,
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
    let written = write_serial_bytes(&resource, data, timeout_ns, "destack.device.serial.write")?;

    unsafe {
        out.write(written);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::super::identity::classify_serial_transport;
    use super::{
        DCB_BINARY_BIT, DCB_DTR_CONTROL_MASK, DCB_DTR_CONTROL_SHIFT, DCB_IN_X_BIT, DCB_OUT_X_BIT,
        DCB_OUTX_CTS_FLOW_BIT, DCB_OUTX_DSR_FLOW_BIT, DCB_PARITY_ENABLE_BIT, DCB_RTS_CONTROL_MASK,
        DCB_RTS_CONTROL_SHIFT, DCB_TX_CONTINUE_ON_XOFF_BIT, apply_serial_config_bits, dcb_field,
        dcb_flag, decode_input_signals, decode_serial_port_number, pending_error_from_flags,
        serial_parity, serial_parity_bits, serial_stop_bits, serial_stop_bits_bits,
        validate_open_options, validate_queue_size,
    };
    use crate::platform::device::{
        SerialDataBits, SerialErrorKind, SerialFlowControl, SerialInputSignals, SerialParity,
        SerialPortConfig, SerialPortOpenOptions, SerialPortTransport, SerialStopBits,
    };
    use crate::platform::diagnostic::PlatformErrorCode;
    use crate::tests::platform::assert_runtime_error_code;
    use windows_sys::Win32::Devices::Communication::{
        CE_BREAK, CE_FRAME, CE_RXOVER, DCB, EVENPARITY, MARKPARITY, MS_CTS_ON, MS_RING_ON,
        ODDPARITY, ONE5STOPBITS, ONESTOPBIT, SPACEPARITY, TWOSTOPBITS,
    };
    use windows_sys::Win32::System::WindowsProgramming::{
        DTR_CONTROL_ENABLE, DTR_CONTROL_HANDSHAKE, RTS_CONTROL_ENABLE, RTS_CONTROL_HANDSHAKE,
    };

    /// Accept plain COM identifiers and namespaced device paths.
    #[test]
    fn test_decode_serial_port_number_accepts_com_identifiers() {
        assert_eq!(decode_serial_port_number("COM7").unwrap(), 7);
        assert_eq!(decode_serial_port_number(r"\\.\COM42").unwrap(), 42);
    }

    /// Reject malformed serial identifiers.
    #[test]
    fn test_decode_serial_port_number_rejects_invalid_identifiers() {
        assert!(decode_serial_port_number("ttyS0").is_err());
        assert!(decode_serial_port_number("COM0").is_err());
        assert!(decode_serial_port_number("COMx").is_err());
    }

    /// Detect USB and Bluetooth targets conservatively from host names.
    #[test]
    fn test_classify_serial_transport_from_target_detects_usb_and_bluetooth() {
        assert_eq!(
            classify_serial_transport(r"USB\\VID_1A86&PID_7523\\SER123", r"\\?\usb#vid_1a86"),
            SerialPortTransport::Usb
        );
        assert_eq!(
            classify_serial_transport(
                r"BTHENUM\\{00001101-0000-1000-8000-00805F9B34FB}\\...",
                r"\\?\BTHENUM#..."
            ),
            SerialPortTransport::Bluetooth
        );
        assert_eq!(
            classify_serial_transport(r"ACPI\\PNP0501\\1", r"\\?\ACPI#PNP0501"),
            SerialPortTransport::Native
        );
    }

    /// Prefer the most specific comm-error kind when multiple bits are set.
    #[test]
    fn test_pending_error_from_flags_uses_expected_precedence() {
        let pending_error = pending_error_from_flags(CE_BREAK | CE_FRAME | CE_RXOVER).unwrap();

        assert_eq!(pending_error.kind, SerialErrorKind::Break);
        assert_eq!(
            pending_error.backend_code,
            Some((CE_BREAK | CE_FRAME | CE_RXOVER) as i32)
        );
    }

    /// Roundtrip every supported parity mode through the DCB encoding.
    #[test]
    fn test_serial_parity_roundtrips_supported_modes() {
        let cases = [
            (SerialParity::None, false, 0),
            (SerialParity::Odd, true, ODDPARITY),
            (SerialParity::Even, true, EVENPARITY),
            (SerialParity::Mark, true, MARKPARITY),
            (SerialParity::Space, true, SPACEPARITY),
        ];

        for (parity, expected_enabled, expected_bits) in cases {
            let (enabled, bits) = serial_parity_bits(parity);
            assert_eq!(enabled, expected_enabled);
            assert_eq!(bits, expected_bits);

            let decoded = serial_parity(enabled, bits, "test").unwrap();
            assert_eq!(decoded, parity);
        }
    }

    /// Roundtrip every supported stop-bit mode through the DCB encoding.
    #[test]
    fn test_serial_stop_bits_roundtrip_supported_modes() {
        let cases = [
            (SerialStopBits::One, ONESTOPBIT),
            (SerialStopBits::OnePointFive, ONE5STOPBITS),
            (SerialStopBits::Two, TWOSTOPBITS),
        ];

        for (stop_bits, expected_bits) in cases {
            assert_eq!(serial_stop_bits_bits(stop_bits), expected_bits);
            assert_eq!(serial_stop_bits(expected_bits, "test").unwrap(), stop_bits);
        }
    }

    /// Apply the portable serial configuration onto one DCB snapshot predictably.
    #[test]
    fn test_apply_serial_config_bits_sets_expected_dcb_fields() {
        let config = SerialPortConfig {
            baud_rate: 230_400,
            data_bits: SerialDataBits::Seven,
            parity: SerialParity::Even,
            stop_bits: SerialStopBits::Two,
            flow_control: SerialFlowControl {
                request_to_send_clear_to_send_enabled: true,
                data_terminal_ready_data_set_ready_enabled: true,
                xon_xoff_enabled: true,
            },
        };
        let mut dcb = DCB {
            DCBlength: std::mem::size_of::<DCB>() as u32,
            ..unsafe { std::mem::zeroed() }
        };

        apply_serial_config_bits(&mut dcb, config);

        assert_eq!(dcb.BaudRate, 230_400);
        assert_eq!(dcb.ByteSize, 7);
        assert_eq!(dcb.StopBits, TWOSTOPBITS);
        assert_eq!(dcb.Parity, EVENPARITY);
        assert!(dcb_flag(dcb._bitfield, DCB_BINARY_BIT));
        assert!(dcb_flag(dcb._bitfield, DCB_PARITY_ENABLE_BIT));
        assert!(dcb_flag(dcb._bitfield, DCB_OUTX_CTS_FLOW_BIT));
        assert!(dcb_flag(dcb._bitfield, DCB_OUTX_DSR_FLOW_BIT));
        assert!(dcb_flag(dcb._bitfield, DCB_OUT_X_BIT));
        assert!(dcb_flag(dcb._bitfield, DCB_IN_X_BIT));
        assert!(dcb_flag(dcb._bitfield, DCB_TX_CONTINUE_ON_XOFF_BIT));
        assert_eq!(
            dcb_field(dcb._bitfield, DCB_RTS_CONTROL_MASK, DCB_RTS_CONTROL_SHIFT),
            RTS_CONTROL_HANDSHAKE
        );
        assert_eq!(
            dcb_field(dcb._bitfield, DCB_DTR_CONTROL_MASK, DCB_DTR_CONTROL_SHIFT),
            DTR_CONTROL_HANDSHAKE
        );
    }

    /// Disable handshake bits cleanly when flow control is not requested.
    #[test]
    fn test_apply_serial_config_bits_disables_flow_control_cleanly() {
        let config = SerialPortConfig {
            baud_rate: 9_600,
            data_bits: SerialDataBits::Eight,
            parity: SerialParity::None,
            stop_bits: SerialStopBits::One,
            flow_control: SerialFlowControl {
                request_to_send_clear_to_send_enabled: false,
                data_terminal_ready_data_set_ready_enabled: false,
                xon_xoff_enabled: false,
            },
        };
        let mut dcb = DCB {
            DCBlength: std::mem::size_of::<DCB>() as u32,
            ..unsafe { std::mem::zeroed() }
        };

        apply_serial_config_bits(&mut dcb, config);

        assert_eq!(dcb.ByteSize, 8);
        assert_eq!(dcb.StopBits, ONESTOPBIT);
        assert!(!dcb_flag(dcb._bitfield, DCB_PARITY_ENABLE_BIT));
        assert!(!dcb_flag(dcb._bitfield, DCB_OUTX_CTS_FLOW_BIT));
        assert!(!dcb_flag(dcb._bitfield, DCB_OUTX_DSR_FLOW_BIT));
        assert!(!dcb_flag(dcb._bitfield, DCB_OUT_X_BIT));
        assert!(!dcb_flag(dcb._bitfield, DCB_IN_X_BIT));
        assert_eq!(
            dcb_field(dcb._bitfield, DCB_RTS_CONTROL_MASK, DCB_RTS_CONTROL_SHIFT),
            RTS_CONTROL_ENABLE
        );
        assert_eq!(
            dcb_field(dcb._bitfield, DCB_DTR_CONTROL_MASK, DCB_DTR_CONTROL_SHIFT),
            DTR_CONTROL_ENABLE
        );
    }

    /// Decode Win32 modem bits into the portable input-signal payload.
    #[test]
    fn test_decode_input_signals_maps_expected_modem_bits() {
        let signals = decode_input_signals(MS_CTS_ON | MS_RING_ON);

        assert_eq!(
            signals,
            SerialInputSignals {
                clear_to_send: true,
                data_set_ready: false,
                data_carrier_detect: false,
                ring_indicator: true,
            }
        );
    }

    /// Reject zero-sized queue hints during windows serial preflight.
    #[test]
    fn test_validate_queue_size_rejects_zero() {
        let error = validate_queue_size("options.readBufferSize", Some(0)).unwrap_err();

        assert_runtime_error_code(&error, PlatformErrorCode::InvalidArgumentValue);
    }

    /// Reject shared-open requests because windows COM ports are exclusive.
    #[test]
    fn test_validate_open_options_rejects_non_exclusive_open() {
        let options = SerialPortOpenOptions {
            config: SerialPortConfig {
                baud_rate: 9_600,
                data_bits: SerialDataBits::Eight,
                parity: SerialParity::None,
                stop_bits: SerialStopBits::One,
                flow_control: SerialFlowControl {
                    request_to_send_clear_to_send_enabled: false,
                    data_terminal_ready_data_set_ready_enabled: false,
                    xon_xoff_enabled: false,
                },
            },
            exclusive: Some(false),
            read_buffer_size: None,
            write_buffer_size: None,
        };
        let error = validate_open_options(&options).unwrap_err();

        assert_runtime_error_code(&error, PlatformErrorCode::NotSupported);
    }
}
