use super::core::*;
use super::identity::*;
use super::state::*;

/// Map one requested baud rate to one termios speed.
pub(super) fn speed_from_baud_rate(baud_rate: u32) -> RuntimeResult<libc::speed_t> {
    let speed = match baud_rate {
        0 => libc::B0,
        50 => libc::B50,
        75 => libc::B75,
        110 => libc::B110,
        134 => libc::B134,
        150 => libc::B150,
        200 => libc::B200,
        300 => libc::B300,
        600 => libc::B600,
        1200 => libc::B1200,
        1800 => libc::B1800,
        2400 => libc::B2400,
        4800 => libc::B4800,
        9600 => libc::B9600,
        19200 => libc::B19200,
        38400 => libc::B38400,
        #[cfg(any(
            target_os = "linux",
            target_vendor = "apple",
            target_os = "freebsd",
            target_os = "openbsd",
            target_os = "netbsd",
            target_os = "dragonfly"
        ))]
        57600 => libc::B57600,
        #[cfg(any(
            target_os = "linux",
            target_vendor = "apple",
            target_os = "freebsd",
            target_os = "openbsd",
            target_os = "netbsd",
            target_os = "dragonfly"
        ))]
        115200 => libc::B115200,
        #[cfg(any(
            target_os = "linux",
            target_vendor = "apple",
            target_os = "freebsd",
            target_os = "openbsd",
            target_os = "netbsd",
            target_os = "dragonfly"
        ))]
        230400 => libc::B230400,
        _ => {
            return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "config.baudRate",
                "unsupported baud rate for this Unix serial backend",
            ))
            .boxed());
        }
    };

    Ok(speed)
}

/// Map one host termios speed constant back into one baud rate.
pub(super) fn baud_rate_from_speed(
    speed: libc::speed_t,
    operation: &'static str,
) -> RuntimeResult<u32> {
    let baud_rate = match speed {
        x if x == libc::B0 => 0,
        x if x == libc::B50 => 50,
        x if x == libc::B75 => 75,
        x if x == libc::B110 => 110,
        x if x == libc::B134 => 134,
        x if x == libc::B150 => 150,
        x if x == libc::B200 => 200,
        x if x == libc::B300 => 300,
        x if x == libc::B600 => 600,
        x if x == libc::B1200 => 1200,
        x if x == libc::B1800 => 1800,
        x if x == libc::B2400 => 2400,
        x if x == libc::B4800 => 4800,
        x if x == libc::B9600 => 9600,
        x if x == libc::B19200 => 19200,
        x if x == libc::B38400 => 38400,
        #[cfg(any(
            target_os = "linux",
            target_vendor = "apple",
            target_os = "freebsd",
            target_os = "openbsd",
            target_os = "netbsd",
            target_os = "dragonfly"
        ))]
        x if x == libc::B57600 => 57600,
        #[cfg(any(
            target_os = "linux",
            target_vendor = "apple",
            target_os = "freebsd",
            target_os = "openbsd",
            target_os = "netbsd",
            target_os = "dragonfly"
        ))]
        x if x == libc::B115200 => 115200,
        #[cfg(any(
            target_os = "linux",
            target_vendor = "apple",
            target_os = "freebsd",
            target_os = "openbsd",
            target_os = "netbsd",
            target_os = "dragonfly"
        ))]
        x if x == libc::B230400 => 230400,
        _ => {
            return Err(RuntimeError::from(PlatformError::io_with(
                Some(PlatformErrorCode::IoInvalidData),
                None,
                None,
                Some(operation.to_string()),
                None,
                "host serial port reports one unsupported baud rate",
            ))
            .boxed());
        }
    };

    Ok(baud_rate)
}

/// Return one optional modem-status snapshot when the host supports it.
pub(super) fn read_signals_optional(
    descriptor: RawFd,
    operation: &'static str,
) -> RuntimeResult<Option<SerialInputSignals>> {
    let mut bits = 0 as libc::c_int;
    let status = unsafe { libc::ioctl(descriptor, libc::TIOCMGET, &mut bits) };
    if status < 0 {
        let errno = core_platform::get_errno();
        if matches!(errno, libc::ENOTTY | libc::EINVAL | libc::ENODEV) {
            return Ok(None);
        }

        return Err(serial_io_error_with_errno(
            operation,
            "ioctl(TIOCMGET)",
            errno,
            "failed to query serial modem status",
        ));
    }

    Ok(Some(SerialInputSignals {
        clear_to_send: bits & libc::TIOCM_CTS != 0,
        data_set_ready: bits & libc::TIOCM_DSR != 0,
        data_carrier_detect: bits & libc::TIOCM_CAR != 0,
        ring_indicator: bits & libc::TIOCM_RNG != 0,
    }))
}

/// Read one strict modem-status snapshot or fail loudly.
pub(super) fn read_signals(
    descriptor: RawFd,
    operation: &'static str,
) -> RuntimeResult<SerialInputSignals> {
    let Some(signals) = read_signals_optional(descriptor, operation)? else {
        return Err(RuntimeError::from(PlatformError::not_supported(operation)).boxed());
    };

    Ok(signals)
}

/// Return one available-byte estimate when the host exposes it.
pub(super) fn available_bytes(descriptor: RawFd) -> Option<u64> {
    let mut bytes = 0 as libc::c_int;
    let status = unsafe { libc::ioctl(descriptor, libc::FIONREAD, &mut bytes) };
    if status < 0 || bytes < 0 {
        return None;
    }

    Some(bytes as u64)
}

/// Apply one requested output-signal update on one descriptor.
pub(super) fn apply_output_signals(
    descriptor: RawFd,
    signals: SerialOutputSignals,
    operation: &'static str,
) -> RuntimeResult<()> {
    // update the DTR and RTS modem bits when the caller asked for them
    if signals.data_terminal_ready.is_some() || signals.request_to_send.is_some() {
        let mut bits = 0 as libc::c_int;
        let get_status = unsafe { libc::ioctl(descriptor, libc::TIOCMGET, &mut bits) };
        if get_status < 0 {
            let errno = core_platform::get_errno();
            if matches!(errno, libc::ENOTTY | libc::EINVAL | libc::ENODEV) {
                return Err(RuntimeError::from(PlatformError::not_supported(operation)).boxed());
            }

            return Err(serial_io_error_with_errno(
                operation,
                "ioctl(TIOCMGET)",
                errno,
                "failed to read serial modem bits before one line update",
            ));
        }

        if let Some(enabled) = signals.data_terminal_ready {
            if enabled {
                bits |= libc::TIOCM_DTR;
            } else {
                bits &= !libc::TIOCM_DTR;
            }
        }

        if let Some(enabled) = signals.request_to_send {
            if enabled {
                bits |= libc::TIOCM_RTS;
            } else {
                bits &= !libc::TIOCM_RTS;
            }
        }

        let set_status = unsafe { libc::ioctl(descriptor, libc::TIOCMSET, &bits) };
        if set_status < 0 {
            let errno = core_platform::get_errno();
            if matches!(errno, libc::ENOTTY | libc::EINVAL | libc::ENODEV) {
                return Err(RuntimeError::from(PlatformError::not_supported(operation)).boxed());
            }

            return Err(serial_io_error_with_errno(
                operation,
                "ioctl(TIOCMSET)",
                errno,
                "failed to update serial modem bits",
            ));
        }
    }

    // update break state when the caller asked for it
    if let Some(enabled) = signals.break_condition_active {
        let request = if enabled {
            libc::TIOCSBRK
        } else {
            libc::TIOCCBRK
        };

        let status =
            unsafe { libc::ioctl(descriptor, unix_ioctl_request(request as libc::c_ulong)) };
        if status < 0 {
            let errno = core_platform::get_errno();
            if matches!(errno, libc::ENOTTY | libc::EINVAL | libc::ENODEV) {
                return Err(RuntimeError::from(PlatformError::not_supported(operation)).boxed());
            }

            let syscall = if enabled {
                "ioctl(TIOCSBRK)"
            } else {
                "ioctl(TIOCCBRK)"
            };

            return Err(serial_io_error_with_errno(
                operation,
                syscall,
                errno,
                "failed to update serial break state",
            ));
        }
    }

    Ok(())
}

/// Apply one serial flow-control configuration on one host termios payload.
pub(super) fn apply_flow_control(
    attributes: &mut libc::termios,
    flow_control: SerialFlowControl,
) -> RuntimeResult<()> {
    if flow_control.data_terminal_ready_data_set_ready_enabled {
        return Err(RuntimeError::from(PlatformError::not_supported(
            "destack.device.serial.configure",
        ))
        .boxed());
    }

    // reset hardware and software flow-control bits before applying the caller config
    #[cfg(any(
        target_os = "linux",
        target_vendor = "apple",
        target_os = "freebsd",
        target_os = "openbsd",
        target_os = "netbsd",
        target_os = "dragonfly"
    ))]
    {
        attributes.c_cflag &= !libc::CRTSCTS;
    }
    attributes.c_iflag &= !(libc::IXON | libc::IXOFF);

    // apply one caller-selected RTS or CTS mode when the host supports it
    if flow_control.request_to_send_clear_to_send_enabled {
        #[cfg(any(
            target_os = "linux",
            target_vendor = "apple",
            target_os = "freebsd",
            target_os = "openbsd",
            target_os = "netbsd",
            target_os = "dragonfly"
        ))]
        {
            attributes.c_cflag |= libc::CRTSCTS;
        }

        #[cfg(not(any(
            target_os = "linux",
            target_vendor = "apple",
            target_os = "freebsd",
            target_os = "openbsd",
            target_os = "netbsd",
            target_os = "dragonfly"
        )))]
        {
            return Err(RuntimeError::from(PlatformError::not_supported(
                "destack.device.serial.configure",
            ))
            .boxed());
        }
    }

    // apply one caller-selected xon or xoff mode
    if flow_control.xon_xoff_enabled {
        attributes.c_iflag |= libc::IXON | libc::IXOFF;
    }

    Ok(())
}

/// Apply one serial configuration on one host descriptor.
pub(super) fn apply_serial_config(
    descriptor: RawFd,
    config: SerialPortConfig,
) -> RuntimeResult<()> {
    // load the current host termios payload and move it into one raw baseline
    let mut attributes = std::mem::MaybeUninit::<libc::termios>::uninit();
    let get_status = unsafe { libc::tcgetattr(descriptor, attributes.as_mut_ptr()) };
    if get_status < 0 {
        return Err(serial_io_error(
            "destack.device.serial.configure",
            "tcgetattr",
            "failed to read current serial attributes",
        ));
    }
    let mut attributes = unsafe { attributes.assume_init() };
    unsafe {
        libc::cfmakeraw(&mut attributes);
    }

    // apply one serial framing baseline
    attributes.c_cflag |= libc::CLOCAL | libc::CREAD;
    attributes.c_cflag &= !libc::CSIZE;
    attributes.c_cflag &= !(libc::PARENB | libc::PARODD | libc::CSTOPB);
    #[cfg(target_os = "linux")]
    {
        attributes.c_cflag &= !libc::CMSPAR;
    }

    // apply one caller-selected data-bit width
    attributes.c_cflag |= match config.data_bits {
        SerialDataBits::Five => libc::CS5,
        SerialDataBits::Six => libc::CS6,
        SerialDataBits::Seven => libc::CS7,
        SerialDataBits::Eight => libc::CS8,
    };

    // apply one caller-selected parity mode
    match config.parity {
        SerialParity::None => {}
        SerialParity::Odd => {
            attributes.c_cflag |= libc::PARENB | libc::PARODD;
        }
        SerialParity::Even => {
            attributes.c_cflag |= libc::PARENB;
        }
        SerialParity::Mark => {
            #[cfg(target_os = "linux")]
            {
                attributes.c_cflag |= libc::PARENB | libc::PARODD | libc::CMSPAR;
            }

            #[cfg(not(target_os = "linux"))]
            {
                return Err(RuntimeError::from(PlatformError::not_supported(
                    "destack.device.serial.configure",
                ))
                .boxed());
            }
        }
        SerialParity::Space => {
            #[cfg(target_os = "linux")]
            {
                attributes.c_cflag |= libc::PARENB | libc::CMSPAR;
            }

            #[cfg(not(target_os = "linux"))]
            {
                return Err(RuntimeError::from(PlatformError::not_supported(
                    "destack.device.serial.configure",
                ))
                .boxed());
            }
        }
    }

    // apply one caller-selected stop-bit mode
    match config.stop_bits {
        SerialStopBits::One => {}
        SerialStopBits::OnePointFive => {
            if config.data_bits != SerialDataBits::Five {
                return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                    "config.stopBits",
                    "one-point-five stop bits require five data bits on Unix hosts",
                ))
                .boxed());
            }

            attributes.c_cflag |= libc::CSTOPB;
        }
        SerialStopBits::Two => {
            attributes.c_cflag |= libc::CSTOPB;
        }
    }

    // apply one caller-selected flow-control policy
    apply_flow_control(&mut attributes, config.flow_control)?;

    // force one noncanonical nonblocking read policy on the descriptor itself
    attributes.c_cc[libc::VMIN] = 0;
    attributes.c_cc[libc::VTIME] = 0;

    // apply one caller-selected baud rate to both directions
    let speed = speed_from_baud_rate(config.baud_rate)?;
    let input_status = unsafe { libc::cfsetispeed(&mut attributes, speed) };
    if input_status < 0 {
        return Err(serial_io_error(
            "destack.device.serial.configure",
            "cfsetispeed",
            "failed to set serial input speed",
        ));
    }

    let output_status = unsafe { libc::cfsetospeed(&mut attributes, speed) };
    if output_status < 0 {
        return Err(serial_io_error(
            "destack.device.serial.configure",
            "cfsetospeed",
            "failed to set serial output speed",
        ));
    }

    // commit one host termios update
    let set_status = unsafe { libc::tcsetattr(descriptor, libc::TCSANOW, &attributes) };
    if set_status < 0 {
        return Err(serial_io_error(
            "destack.device.serial.configure",
            "tcsetattr",
            "failed to apply serial termios attributes",
        ));
    }

    Ok(())
}

/// Read one serial configuration snapshot from one descriptor.
pub(super) fn read_serial_config(
    descriptor: RawFd,
    operation: &'static str,
) -> RuntimeResult<SerialPortConfig> {
    let mut attributes = std::mem::MaybeUninit::<libc::termios>::uninit();
    let get_status = unsafe { libc::tcgetattr(descriptor, attributes.as_mut_ptr()) };
    if get_status < 0 {
        return Err(serial_io_error(
            operation,
            "tcgetattr",
            "failed to read serial termios attributes",
        ));
    }
    let attributes = unsafe { attributes.assume_init() };

    let data_bits = match attributes.c_cflag & libc::CSIZE {
        x if x == libc::CS5 => SerialDataBits::Five,
        x if x == libc::CS6 => SerialDataBits::Six,
        x if x == libc::CS7 => SerialDataBits::Seven,
        x if x == libc::CS8 => SerialDataBits::Eight,
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

    let parity = if attributes.c_cflag & libc::PARENB == 0 {
        SerialParity::None
    } else if attributes.c_cflag & libc::PARODD != 0 {
        #[cfg(target_os = "linux")]
        {
            if attributes.c_cflag & libc::CMSPAR != 0 {
                SerialParity::Mark
            } else {
                SerialParity::Odd
            }
        }

        #[cfg(not(target_os = "linux"))]
        {
            SerialParity::Odd
        }
    } else {
        #[cfg(target_os = "linux")]
        {
            if attributes.c_cflag & libc::CMSPAR != 0 {
                SerialParity::Space
            } else {
                SerialParity::Even
            }
        }

        #[cfg(not(target_os = "linux"))]
        {
            SerialParity::Even
        }
    };

    let stop_bits = if attributes.c_cflag & libc::CSTOPB == 0 {
        SerialStopBits::One
    } else if data_bits == SerialDataBits::Five {
        SerialStopBits::OnePointFive
    } else {
        SerialStopBits::Two
    };

    let flow_control = SerialFlowControl {
        #[cfg(any(
            target_os = "linux",
            target_vendor = "apple",
            target_os = "freebsd",
            target_os = "openbsd",
            target_os = "netbsd",
            target_os = "dragonfly"
        ))]
        request_to_send_clear_to_send_enabled: attributes.c_cflag & libc::CRTSCTS != 0,
        #[cfg(not(any(
            target_os = "linux",
            target_vendor = "apple",
            target_os = "freebsd",
            target_os = "openbsd",
            target_os = "netbsd",
            target_os = "dragonfly"
        )))]
        request_to_send_clear_to_send_enabled: false,
        data_terminal_ready_data_set_ready_enabled: false,
        xon_xoff_enabled: attributes.c_iflag & (libc::IXON | libc::IXOFF) != 0,
    };

    Ok(SerialPortConfig {
        baud_rate: baud_rate_from_speed(unsafe { libc::cfgetospeed(&attributes) }, operation)?,
        data_bits,
        parity,
        stop_bits,
        flow_control,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Roundtrip one standard unix baud rate through the termios mapping.
    #[test]
    fn test_speed_from_baud_rate_roundtrips_standard_rates() {
        let speed = speed_from_baud_rate(9_600).unwrap();
        let baud_rate = baud_rate_from_speed(speed, "serial.config").unwrap();

        assert_eq!(baud_rate, 9_600);
    }

    /// Roundtrip one extended unix baud rate when the host libc exposes it.
    #[test]
    fn test_speed_from_baud_rate_roundtrips_extended_rates() {
        let speed = speed_from_baud_rate(115_200).unwrap();
        let baud_rate = baud_rate_from_speed(speed, "serial.config").unwrap();

        assert_eq!(baud_rate, 115_200);
    }

    /// Map one hang-up baud rate through the termios mapping.
    #[test]
    fn test_speed_from_baud_rate_roundtrips_hangup() {
        let speed = speed_from_baud_rate(0).unwrap();
        let baud_rate = baud_rate_from_speed(speed, "serial.config").unwrap();

        assert_eq!(baud_rate, 0);
    }

    /// Reject one unsupported baud rate loudly instead of accepting it silently.
    #[test]
    fn test_speed_from_baud_rate_rejects_unsupported_values() {
        let error = speed_from_baud_rate(123_456).unwrap_err();

        assert_eq!(
            error
                .platform_error()
                .expect("baud-rate rejection should carry one platform error")
                .code,
            PlatformErrorCode::InvalidArgumentValue
        );
    }
}
