use super::core::*;
use super::state::*;

/// Set or clear one DCB bit flag.
pub(super) fn set_dcb_flag(bits: &mut u32, mask: u32, enabled: bool) {
    if enabled {
        *bits |= mask;
    } else {
        *bits &= !mask;
    }
}

/// Return whether one DCB bit flag is set.
pub(super) fn dcb_flag(bits: u32, mask: u32) -> bool {
    bits & mask != 0
}

/// Write one DCB field value.
pub(super) fn set_dcb_field(bits: &mut u32, mask: u32, shift: u32, value: u32) {
    *bits = (*bits & !mask) | ((value << shift) & mask);
}

/// Read one DCB field value.
pub(super) fn dcb_field(bits: u32, mask: u32, shift: u32) -> u32 {
    (bits & mask) >> shift
}

/// Convert one serial data-bit selection into one DCB byte-size value.
pub(super) fn serial_byte_size(data_bits: SerialDataBits) -> u8 {
    match data_bits {
        SerialDataBits::Five => 5,
        SerialDataBits::Six => 6,
        SerialDataBits::Seven => 7,
        SerialDataBits::Eight => 8,
    }
}

/// Decode one DCB byte-size value into one binding data-bit selection.
pub(super) fn serial_data_bits(
    byte_size: u8,
    operation: &'static str,
) -> RuntimeResult<SerialDataBits> {
    let data_bits = match byte_size {
        5 => SerialDataBits::Five,
        6 => SerialDataBits::Six,
        7 => SerialDataBits::Seven,
        8 => SerialDataBits::Eight,
        _ => {
            return Err(RuntimeError::from(PlatformError::io_with(
                Some(PlatformErrorCode::IoInvalidData),
                None,
                None,
                Some(operation.to_string()),
                None,
                "host serial port reports one unsupported data-bit configuration",
            ))
            .boxed());
        }
    };

    Ok(data_bits)
}

/// Convert one binding parity mode into one DCB parity pair.
pub(super) fn serial_parity_bits(parity: SerialParity) -> (bool, u8) {
    match parity {
        SerialParity::None => (false, NOPARITY),
        SerialParity::Odd => (true, ODDPARITY),
        SerialParity::Even => (true, EVENPARITY),
        SerialParity::Mark => (true, MARKPARITY),
        SerialParity::Space => (true, SPACEPARITY),
    }
}

/// Decode one host parity configuration into one binding parity mode.
pub(super) fn serial_parity(
    parity_enabled: bool,
    parity: u8,
    operation: &'static str,
) -> RuntimeResult<SerialParity> {
    if !parity_enabled {
        return Ok(SerialParity::None);
    }

    let parity = match parity {
        x if x == ODDPARITY => SerialParity::Odd,
        x if x == EVENPARITY => SerialParity::Even,
        x if x == MARKPARITY => SerialParity::Mark,
        x if x == SPACEPARITY => SerialParity::Space,
        _ => {
            return Err(RuntimeError::from(PlatformError::io_with(
                Some(PlatformErrorCode::IoInvalidData),
                None,
                None,
                Some(operation.to_string()),
                None,
                "host serial port reports one unsupported parity mode",
            ))
            .boxed());
        }
    };

    Ok(parity)
}

/// Convert one binding stop-bit selection into one DCB stop-bit value.
pub(super) fn serial_stop_bits_bits(stop_bits: SerialStopBits) -> u8 {
    match stop_bits {
        SerialStopBits::One => ONESTOPBIT,
        SerialStopBits::OnePointFive => ONE5STOPBITS,
        SerialStopBits::Two => TWOSTOPBITS,
    }
}

/// Decode one host stop-bit configuration into one binding stop-bit mode.
pub(super) fn serial_stop_bits(
    stop_bits: u8,
    operation: &'static str,
) -> RuntimeResult<SerialStopBits> {
    let stop_bits = match stop_bits {
        x if x == ONESTOPBIT => SerialStopBits::One,
        x if x == ONE5STOPBITS => SerialStopBits::OnePointFive,
        x if x == TWOSTOPBITS => SerialStopBits::Two,
        _ => {
            return Err(RuntimeError::from(PlatformError::io_with(
                Some(PlatformErrorCode::IoInvalidData),
                None,
                None,
                Some(operation.to_string()),
                None,
                "host serial port reports one unsupported stop-bit mode",
            ))
            .boxed());
        }
    };

    Ok(stop_bits)
}

/// Configure one serial handle for immediate reads and caller-shaped writes.
pub(super) fn apply_serial_timeouts(
    handle: HANDLE,
    write_timeout_ns: Option<u64>,
    operation: &'static str,
) -> RuntimeResult<()> {
    let write_timeout_constant = match write_timeout_ns {
        Some(timeout_ns) if core_platform::timeout_deadline(timeout_ns).is_some() => {
            let timeout = Duration::from_nanos(timeout_ns);
            let timeout_millis = timeout.as_millis();
            if timeout.subsec_nanos() == 0 {
                timeout_millis.min(u128::from(u32::MAX)) as u32
            } else if timeout_millis == 0 {
                1
            } else {
                timeout_millis.saturating_add(1).min(u128::from(u32::MAX)) as u32
            }
        }
        _ => 0,
    };

    let timeouts = COMMTIMEOUTS {
        ReadIntervalTimeout: u32::MAX,
        ReadTotalTimeoutMultiplier: 0,
        ReadTotalTimeoutConstant: 0,
        WriteTotalTimeoutMultiplier: 0,
        WriteTotalTimeoutConstant: write_timeout_constant,
    };

    let status = unsafe { SetCommTimeouts(handle, &timeouts) };
    if status == 0 {
        return Err(serial_io_error(
            operation,
            "SetCommTimeouts",
            "failed to apply serial timeouts",
        ));
    }

    Ok(())
}

/// Read one DCB snapshot from one serial handle.
pub(super) fn read_serial_dcb(handle: HANDLE, operation: &'static str) -> RuntimeResult<DCB> {
    let mut dcb = DCB {
        DCBlength: std::mem::size_of::<DCB>() as u32,
        ..unsafe { std::mem::zeroed() }
    };
    let status = unsafe { GetCommState(handle, &mut dcb) };
    if status == 0 {
        return Err(serial_io_error(
            operation,
            "GetCommState",
            "failed to read serial configuration",
        ));
    }

    Ok(dcb)
}

/// Apply one binding serial configuration onto one DCB snapshot.
pub(super) fn apply_serial_config_bits(dcb: &mut DCB, config: SerialPortConfig) {
    let mut bits = dcb._bitfield;

    // baseline
    set_dcb_flag(&mut bits, DCB_BINARY_BIT, true);
    set_dcb_flag(&mut bits, DCB_DSR_SENSITIVITY_BIT, false);
    set_dcb_flag(&mut bits, DCB_TX_CONTINUE_ON_XOFF_BIT, true);
    set_dcb_flag(&mut bits, DCB_ERROR_CHAR_BIT, false);
    set_dcb_flag(&mut bits, DCB_NULL_BIT, false);
    set_dcb_flag(&mut bits, DCB_ABORT_ON_ERROR_BIT, false);

    // parity
    let (parity_enabled, parity) = serial_parity_bits(config.parity);
    set_dcb_flag(&mut bits, DCB_PARITY_ENABLE_BIT, parity_enabled);
    dcb.Parity = parity;

    // flow control
    set_dcb_flag(
        &mut bits,
        DCB_OUTX_CTS_FLOW_BIT,
        config.flow_control.request_to_send_clear_to_send_enabled,
    );
    set_dcb_field(
        &mut bits,
        DCB_RTS_CONTROL_MASK,
        DCB_RTS_CONTROL_SHIFT,
        if config.flow_control.request_to_send_clear_to_send_enabled {
            RTS_CONTROL_HANDSHAKE
        } else {
            RTS_CONTROL_ENABLE
        },
    );
    set_dcb_flag(
        &mut bits,
        DCB_OUTX_DSR_FLOW_BIT,
        config
            .flow_control
            .data_terminal_ready_data_set_ready_enabled,
    );
    set_dcb_field(
        &mut bits,
        DCB_DTR_CONTROL_MASK,
        DCB_DTR_CONTROL_SHIFT,
        if config
            .flow_control
            .data_terminal_ready_data_set_ready_enabled
        {
            DTR_CONTROL_HANDSHAKE
        } else {
            DTR_CONTROL_ENABLE
        },
    );
    set_dcb_flag(
        &mut bits,
        DCB_OUT_X_BIT,
        config.flow_control.xon_xoff_enabled,
    );
    set_dcb_flag(
        &mut bits,
        DCB_IN_X_BIT,
        config.flow_control.xon_xoff_enabled,
    );

    // frame
    dcb.BaudRate = config.baud_rate;
    dcb.ByteSize = serial_byte_size(config.data_bits);
    dcb.StopBits = serial_stop_bits_bits(config.stop_bits);
    dcb._bitfield = bits;
}

/// Apply one serial configuration to one host handle.
pub(super) fn apply_serial_config(handle: HANDLE, config: SerialPortConfig) -> RuntimeResult<()> {
    let mut dcb = read_serial_dcb(handle, "destack.device.serial.configure")?;

    apply_serial_config_bits(&mut dcb, config);

    let status = unsafe { SetCommState(handle, &dcb) };
    if status == 0 {
        return Err(serial_io_error(
            "destack.device.serial.configure",
            "SetCommState",
            "failed to apply serial configuration",
        ));
    }

    Ok(())
}

/// Decode one serial configuration snapshot from one host handle.
pub(super) fn read_serial_config(
    handle: HANDLE,
    operation: &'static str,
) -> RuntimeResult<SerialPortConfig> {
    let dcb = read_serial_dcb(handle, operation)?;
    let bits = dcb._bitfield;

    Ok(SerialPortConfig {
        baud_rate: dcb.BaudRate,
        data_bits: serial_data_bits(dcb.ByteSize, operation)?,
        parity: serial_parity(dcb_flag(bits, DCB_PARITY_ENABLE_BIT), dcb.Parity, operation)?,
        stop_bits: serial_stop_bits(dcb.StopBits, operation)?,
        flow_control: SerialFlowControl {
            request_to_send_clear_to_send_enabled: dcb_flag(bits, DCB_OUTX_CTS_FLOW_BIT)
                && dcb_field(bits, DCB_RTS_CONTROL_MASK, DCB_RTS_CONTROL_SHIFT)
                    == RTS_CONTROL_HANDSHAKE,
            data_terminal_ready_data_set_ready_enabled: dcb_flag(bits, DCB_OUTX_DSR_FLOW_BIT)
                && dcb_field(bits, DCB_DTR_CONTROL_MASK, DCB_DTR_CONTROL_SHIFT)
                    == DTR_CONTROL_HANDSHAKE,
            xon_xoff_enabled: dcb_flag(bits, DCB_OUT_X_BIT) || dcb_flag(bits, DCB_IN_X_BIT),
        },
    })
}

/// Read one `ClearCommError` snapshot from one serial handle.
pub(super) fn clear_comm_status_raw(
    handle: HANDLE,
) -> Result<(CLEAR_COMM_ERROR_FLAGS, u64, u64), u32> {
    let mut errors = 0u32;
    let mut status = unsafe { std::mem::zeroed::<COMSTAT>() };
    let call_status = unsafe { ClearCommError(handle, &mut errors, &mut status) };
    if call_status == 0 {
        return Err(core_platform::last_error_code() as u32);
    }

    Ok((
        errors,
        u64::from(status.cbInQue),
        u64::from(status.cbOutQue),
    ))
}

/// Read one modem-status snapshot from one serial handle.
pub(super) fn modem_status_raw(handle: HANDLE) -> Result<u32, u32> {
    let mut status = 0u32;
    let call_status = unsafe { GetCommModemStatus(handle, &mut status) };
    if call_status == 0 {
        return Err(core_platform::last_error_code() as u32);
    }

    Ok(status)
}

/// Decode one modem-status snapshot into one binding payload.
pub(super) fn decode_input_signals(status: u32) -> SerialInputSignals {
    SerialInputSignals {
        clear_to_send: status & MS_CTS_ON != 0,
        data_set_ready: status & MS_DSR_ON != 0,
        data_carrier_detect: status & MS_RLSD_ON != 0,
        ring_indicator: status & MS_RING_ON != 0,
    }
}

/// Read one current signal snapshot when the host supports it.
pub(super) fn read_signals_optional(
    handle: HANDLE,
    operation: &'static str,
) -> RuntimeResult<Option<SerialInputSignals>> {
    match modem_status_raw(handle) {
        Ok(status) => Ok(Some(decode_input_signals(status))),
        Err(code) if is_optional_signal_code(code) => Ok(None),
        Err(code) => Err(serial_io_error_with_code(
            operation,
            "GetCommModemStatus",
            code,
            "failed to read serial modem status",
        )),
    }
}

/// Read one current signal snapshot and require support.
pub(super) fn read_signals(
    handle: HANDLE,
    operation: &'static str,
) -> RuntimeResult<SerialInputSignals> {
    let signals = read_signals_optional(handle, operation)?;

    signals.ok_or_else(|| RuntimeError::from(PlatformError::not_supported(operation)).boxed())
}

/// Map one backend comm-error bitset into one deferred serial error.
pub(super) fn pending_error_from_flags(
    errors: CLEAR_COMM_ERROR_FLAGS,
) -> Option<WindowsSerialPendingError> {
    let kind = if errors & CE_BREAK != 0 {
        SerialErrorKind::Break
    } else if errors & CE_FRAME != 0 {
        SerialErrorKind::Framing
    } else if errors & CE_RXPARITY != 0 {
        SerialErrorKind::Parity
    } else if errors & CE_OVERRUN != 0 {
        SerialErrorKind::Overrun
    } else if errors & CE_RXOVER != 0 {
        SerialErrorKind::BufferOverflow
    } else if errors != 0 {
        SerialErrorKind::Unknown
    } else {
        return None;
    };

    Some(WindowsSerialPendingError {
        kind,
        backend_code: Some(errors as i32),
        backend_detail: None,
    })
}
