use std::collections::BTreeSet;
#[cfg(unix)]
use std::ffi::CStr;

#[cfg(unix)]
use crate::diagnostic::RuntimeError;
use crate::diagnostic::RuntimeResult;
#[cfg(unix)]
use crate::platform::PlatformError;
use crate::platform::device::tests::{DeviceHarnessContext, HarnessValue};
#[cfg(unix)]
use crate::platform::device::{
    SerialDataBits, SerialEvent, SerialEventValue, SerialEventVm, SerialFlowControl,
    SerialOutputSignals, SerialOutputSignalsVm, SerialParity, SerialPortConfig,
    SerialPortConfigValue, SerialPortConfigVm, SerialPortOpenOptions, SerialStopBits,
};
use crate::platform::device::{
    SerialPortDescriptor, SerialPortDescriptorValue, SerialPortDescriptorVm, SerialPortTransport,
    SerialWatchEvent, SerialWatchEventValue, SerialWatchEventVm,
};
#[cfg(unix)]
use crate::platform::diagnostic::PlatformErrorCode;
#[cfg(unix)]
use crate::platform::resource::SerialPortHandle;
use crate::platform::{NativeSlice, VmSlice};
#[cfg(unix)]
use crate::tests::platform::assert_ok_or_expected_error;

/// Build one default serial configuration for Unix serial tests.
#[cfg(unix)]
pub(super) fn default_serial_config() -> SerialPortConfig {
    SerialPortConfig {
        baud_rate: 115_200,
        data_bits: SerialDataBits::Eight,
        parity: SerialParity::None,
        stop_bits: SerialStopBits::One,
        flow_control: SerialFlowControl {
            request_to_send_clear_to_send_enabled: false,
            data_terminal_ready_data_set_ready_enabled: false,
            xon_xoff_enabled: false,
        },
    }
}

/// Build one default serial open options payload for Unix serial tests.
#[cfg(unix)]
pub(super) fn default_serial_options() -> SerialPortOpenOptions {
    SerialPortOpenOptions {
        config: default_serial_config(),
        exclusive: None,
        read_buffer_size: None,
        write_buffer_size: None,
    }
}

/// Open one pty pair for Unix serial tests.
#[cfg(unix)]
pub(super) fn open_test_pty_pair() -> RuntimeResult<(libc::c_int, libc::c_int)> {
    let mut controller = -1;
    let mut worker = -1;

    let status = unsafe {
        libc::openpty(
            &mut controller,
            &mut worker,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        )
    };
    if status < 0 {
        let errno = std::io::Error::last_os_error().raw_os_error().unwrap_or(0);
        return Err(RuntimeError::from(PlatformError::io(format!(
            "openpty failed for serial test: errno {errno}",
        )))
        .boxed());
    }

    Ok((controller, worker))
}

/// Close one raw descriptor and ignore close errors in test cleanup.
#[cfg(unix)]
pub(super) fn close_descriptor(descriptor: libc::c_int) {
    unsafe {
        libc::close(descriptor);
    }
}

/// Resolve one path string for one slave descriptor.
#[cfg(unix)]
pub(super) fn slave_path(descriptor: libc::c_int) -> RuntimeResult<String> {
    let pointer = unsafe { libc::ttyname(descriptor) };
    if pointer.is_null() {
        let errno = std::io::Error::last_os_error().raw_os_error().unwrap_or(0);
        return Err(RuntimeError::from(PlatformError::io(format!(
            "ttyname failed for serial test: errno {errno}",
        )))
        .boxed());
    }

    let name = unsafe { CStr::from_ptr(pointer) };

    Ok(name.to_string_lossy().into_owned())
}

/// Write all bytes to one unix descriptor.
#[cfg(unix)]
pub(super) fn write_all(descriptor: libc::c_int, bytes: &[u8]) -> RuntimeResult<()> {
    let mut offset = 0usize;
    while offset < bytes.len() {
        let status = unsafe {
            libc::write(
                descriptor,
                bytes[offset..].as_ptr().cast::<libc::c_void>(),
                bytes.len() - offset,
            )
        };
        if status < 0 {
            let errno = std::io::Error::last_os_error().raw_os_error().unwrap_or(0);
            if errno == libc::EINTR {
                continue;
            }

            return Err(RuntimeError::from(PlatformError::io(format!(
                "write failed for serial test descriptor: errno {errno}",
            )))
            .boxed());
        }

        offset += status as usize;
    }

    Ok(())
}

/// Read an exact number of bytes from one unix descriptor.
#[cfg(unix)]
pub(super) fn read_exact(descriptor: libc::c_int, length: usize) -> RuntimeResult<Vec<u8>> {
    let mut output = vec![0u8; length];
    let mut offset = 0usize;
    while offset < length {
        let status = unsafe {
            libc::read(
                descriptor,
                output[offset..].as_mut_ptr().cast::<libc::c_void>(),
                length - offset,
            )
        };
        if status < 0 {
            let errno = std::io::Error::last_os_error().raw_os_error().unwrap_or(0);
            if errno == libc::EINTR {
                continue;
            }

            return Err(RuntimeError::from(PlatformError::io(format!(
                "read failed for serial test descriptor: errno {errno}",
            )))
            .boxed());
        }
        if status == 0 {
            return Err(RuntimeError::from(PlatformError::io(
                "unexpected EOF while reading from serial test descriptor".to_string(),
            ))
            .boxed());
        }

        offset += status as usize;
    }

    Ok(output)
}

/// Assert one serial result is success or one expected not-supported outcome.
#[cfg(unix)]
pub(super) fn assert_supported_or_not_supported<T>(
    result: Result<T, Box<RuntimeError>>,
) -> RuntimeResult<Option<T>> {
    assert_ok_or_expected_error(result, &[PlatformErrorCode::NotSupported])
}

/// Assert one serial result is success or one expected platform error.
#[cfg(unix)]
pub(super) fn assert_platform_error_codes<T>(
    result: Result<T, Box<RuntimeError>>,
    expected: &[PlatformErrorCode],
) -> RuntimeResult<Option<T>> {
    assert_ok_or_expected_error(result, expected)
}

/// Open one serial handle through the active harness.
#[cfg(unix)]
pub(super) fn open_serial_handle(
    context: &mut DeviceHarnessContext<'_>,
    path: &str,
) -> RuntimeResult<SerialPortHandle> {
    open_serial_handle_with_options(context, path, default_serial_options())
}

/// Open one serial handle with explicit options through the active harness.
#[cfg(unix)]
pub(super) fn open_serial_handle_with_options(
    context: &mut DeviceHarnessContext<'_>,
    path: &str,
    options: SerialPortOpenOptions,
) -> RuntimeResult<SerialPortHandle> {
    let id = context.harness_value_from(path.to_string())?;
    let options = context.harness_value_from(options)?;

    context.destack_device_serial_open(id, options)
}

/// Decode one serial descriptor from one native or VM harness result.
#[cfg(unix)]
pub(super) fn serial_descriptor_value(
    context: &mut DeviceHarnessContext<'_>,
    value: HarnessValue<SerialPortDescriptor, SerialPortDescriptorVm>,
) -> RuntimeResult<SerialPortDescriptorValue> {
    context.harness_value_into(value)
}

/// Decode one listed serial descriptor slice from one native or VM harness result.
pub(super) fn serial_descriptor_list_value(
    context: &mut DeviceHarnessContext<'_>,
    value: HarnessValue<NativeSlice<SerialPortDescriptor>, VmSlice<SerialPortDescriptorVm>>,
) -> RuntimeResult<Vec<SerialPortDescriptorValue>> {
    context.harness_value_into(value)
}

/// Decode one serial config from one native or VM harness result.
#[cfg(unix)]
pub(super) fn serial_config_value(
    context: &mut DeviceHarnessContext<'_>,
    value: HarnessValue<SerialPortConfig, SerialPortConfigVm>,
) -> RuntimeResult<SerialPortConfigValue> {
    context.harness_value_into(value)
}

/// Decode one serial event from one native or VM harness result.
#[cfg(unix)]
pub(super) fn serial_event_value(
    context: &mut DeviceHarnessContext<'_>,
    value: HarnessValue<SerialEvent, SerialEventVm>,
) -> RuntimeResult<SerialEventValue> {
    context.harness_value_into(value)
}

/// Decode one serial watch event from one native or VM harness result.
pub(super) fn serial_watch_event_value(
    context: &mut DeviceHarnessContext<'_>,
    value: HarnessValue<SerialWatchEvent, SerialWatchEventVm>,
) -> RuntimeResult<SerialWatchEventValue> {
    context.harness_value_into(value)
}

/// Encode one serial output-signal update for one harness call.
#[cfg(unix)]
pub(super) fn serial_config_argument(
    context: &mut DeviceHarnessContext<'_>,
    config: SerialPortConfig,
) -> RuntimeResult<HarnessValue<SerialPortConfig, SerialPortConfigVm>> {
    context.harness_value_from(config)
}

/// Encode one serial output-signal update for one harness call.
#[cfg(unix)]
pub(super) fn serial_output_signals_argument(
    context: &mut DeviceHarnessContext<'_>,
    signals: SerialOutputSignals,
) -> RuntimeResult<HarnessValue<SerialOutputSignals, SerialOutputSignalsVm>> {
    context.harness_value_from(signals)
}

/// Encode one immutable byte slice for one harness call.
#[cfg(unix)]
pub(super) fn serial_write_bytes_argument(
    context: &mut DeviceHarnessContext<'_>,
    bytes: &[u8],
) -> RuntimeResult<HarnessValue<NativeSlice<u8>, VmSlice<u8>>> {
    context.harness_value_from(bytes.to_vec())
}

/// Assert one serial descriptor is self-consistent.
pub(super) fn assert_serial_descriptor_shape(descriptor: &SerialPortDescriptorValue) {
    // core identity
    assert!(!descriptor.id.is_empty());
    assert!(!descriptor.name.is_empty());

    // optional strings
    if let Some(manufacturer) = &descriptor.manufacturer {
        assert!(!manufacturer.is_empty());
    }

    if let Some(product) = &descriptor.product {
        assert!(!product.is_empty());
    }

    if let Some(serial_number) = &descriptor.serial_number {
        assert!(!serial_number.is_empty());
    }

    if let Some(service_class_id) = &descriptor.bluetooth_service_class_id {
        assert!(!service_class_id.is_empty());
        assert_eq!(descriptor.transport, SerialPortTransport::Bluetooth);
    }

    // transport-scoped metadata
    let has_usb_vendor_id = descriptor.usb_vendor_id.is_some();
    let has_usb_product_id = descriptor.usb_product_id.is_some();
    assert_eq!(has_usb_vendor_id, has_usb_product_id);

    if has_usb_vendor_id {
        assert_eq!(descriptor.transport, SerialPortTransport::Usb);
    }
}

/// Assert one serial descriptor is self-consistent and uniquely identified.
pub(super) fn assert_serial_descriptor_invariants(
    descriptor: &SerialPortDescriptorValue,
    ids: &mut BTreeSet<String>,
) {
    assert_serial_descriptor_shape(descriptor);
    assert!(
        ids.insert(descriptor.id.clone()),
        "duplicate serial id: {}",
        descriptor.id
    );
}

/// Assert that one listed descriptor sequence is stable across one immediate repeat.
pub(super) fn assert_serial_descriptor_list_stable(
    previous: &[SerialPortDescriptorValue],
    current: &[SerialPortDescriptorValue],
) {
    assert_eq!(current.len(), previous.len());

    // compare the full descriptor snapshots in order
    for (current, previous) in current.iter().zip(previous) {
        assert_eq!(current, previous);
    }
}
