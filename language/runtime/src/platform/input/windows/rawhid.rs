use super::{core as input_core, raw as raw_input};

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::input::{InputRawHidReport, validation as input_validation};
use crate::platform::{NativeSlice, PlatformError, resource};
use crate::runtime::RuntimeCallContext;

/// Resolve one opened raw-hid-capable device descriptor.
fn resolve_raw_hid_device(
    context: &RuntimeCallContext,
    handle: resource::InputDeviceHandle,
    operation: &'static str,
) -> RuntimeResult<raw_input::RawInputDeviceDescriptor> {
    // resolve one opened raw-input descriptor for this handle
    let device = input_core::raw_device(context, handle, operation)?;

    // reject non-hid devices for raw-hid operations
    if !device.supports_raw_hid {
        return Err(RuntimeError::from(PlatformError::not_supported(operation)).boxed());
    }

    Ok(device)
}

/// Read one raw-hid feature report.
///
/// Read one feature report from one opened raw-hid-capable input endpoint.
///
/// # Platform
/// Unix and Windows, with operation-level `notSupported` where raw-hid feature reports are unavailable.
/// Uses hid feature-report query APIs on Unix and Windows.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `input.control`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_input_raw_hid_get_feature(
    context: &RuntimeCallContext,
    out: *mut NativeSlice<u8>,
    handle: resource::InputDeviceHandle,
    reportid: u8,
    maxbytes: u32,
) -> RuntimeResult<()> {
    // validate output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // validate read bounds and capability support
    input_validation::validate_raw_hid_max_bytes(maxbytes)?;

    // resolve one raw-hid-capable device
    let device = resolve_raw_hid_device(context, handle, "destack.input.rawhid.getFeature")?;

    // read one feature report from hid backend APIs
    let report = raw_input::get_feature_report(
        &device,
        reportid,
        maxbytes,
        "destack.input.rawhid.getFeature",
    )?;

    // write report bytes into runtime-managed slice storage
    unsafe {
        *out = context.store_slice(report);
    }

    Ok(())
}

/// Read one raw-hid report.
///
/// Read one pending raw-hid report from one opened raw-hid-capable input endpoint.
/// Timeout and blocking behavior follow backend raw-hid queue semantics.
///
/// # Platform
/// Unix and Windows, with operation-level `notSupported` where raw-hid reports are unavailable.
/// Uses hidraw or equivalent raw report APIs on Unix and raw-input hid report APIs on Windows.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, ioInterrupted, notSupported.
///
/// # Security
/// Requires `input.read`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_input_raw_hid_read(
    context: &RuntimeCallContext,
    out: *mut InputRawHidReport,
    handle: resource::InputDeviceHandle,
    maxbytes: u32,
    timeoutns: u64,
) -> RuntimeResult<()> {
    // validate output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // validate read bounds and capability support
    input_validation::validate_raw_hid_max_bytes(maxbytes)?;

    // resolve one raw-hid-capable device
    let device = resolve_raw_hid_device(context, handle, "destack.input.rawhid.read")?;

    // read one report with bounded timeout semantics
    let report = raw_input::read_raw_hid_report_with_timeout(
        context,
        &device,
        maxbytes,
        timeoutns,
        "destack.input.rawhid.read",
    )?;

    // write one decoded report payload
    unsafe {
        *out = report;
    }

    Ok(())
}

/// Write one raw-hid feature report.
///
/// Write one feature report to one opened raw-hid-capable input endpoint.
///
/// # Platform
/// Unix and Windows, with operation-level `notSupported` where raw-hid feature reports are unavailable.
/// Uses hid feature-report set APIs on Unix and Windows.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `input.control`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_input_raw_hid_set_feature(
    context: &RuntimeCallContext,
    handle: resource::InputDeviceHandle,
    reportid: u8,
    data: NativeSlice<u8>,
) -> RuntimeResult<()> {
    // resolve one raw-hid-capable device
    let device = resolve_raw_hid_device(context, handle, "destack.input.rawhid.setFeature")?;

    // validate feature payload shape
    let payload = unsafe { data.as_slice()? };
    input_validation::validate_non_empty_bytes("data", payload)?;

    // apply one feature report write to hid backend
    raw_input::set_feature_report(
        &device,
        reportid,
        payload,
        "destack.input.rawhid.setFeature",
    )
}

/// Poll one raw-hid report without blocking.
///
/// Poll one pending raw-hid report and return immediately when none is available.
/// Empty queue state is reported through ioWouldBlock.
///
/// # Platform
/// Unix and Windows, with operation-level `notSupported` where raw-hid reports are unavailable.
/// Uses nonblocking hidraw or equivalent raw report APIs on Unix and nonblocking raw-input hid report APIs on Windows.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `input.read`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_input_raw_hid_try_read(
    context: &RuntimeCallContext,
    out: *mut InputRawHidReport,
    handle: resource::InputDeviceHandle,
    maxbytes: u32,
) -> RuntimeResult<()> {
    // validate output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // validate read bounds and capability support
    input_validation::validate_raw_hid_max_bytes(maxbytes)?;

    // resolve one raw-hid-capable device
    let device = resolve_raw_hid_device(context, handle, "destack.input.rawhid.tryRead")?;

    // poll one queued hid report without blocking
    let report = raw_input::read_raw_hid_report(
        context,
        &device,
        maxbytes,
        true,
        "destack.input.rawhid.tryRead",
    )?;

    // write one decoded report payload
    unsafe {
        *out = report;
    }

    Ok(())
}

/// Write one raw-hid output report.
///
/// Submit one raw-hid output report to one opened raw-hid-capable input endpoint.
/// Short writes can occur based on backend transport behavior.
///
/// # Platform
/// Unix and Windows, with operation-level `notSupported` where raw-hid output reports are unavailable.
/// Uses hidraw or equivalent raw report write APIs on Unix and raw-input hid report write APIs on Windows.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `input.write`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_input_raw_hid_write(
    context: &RuntimeCallContext,
    out: *mut u32,
    handle: resource::InputDeviceHandle,
    reportid: u8,
    data: NativeSlice<u8>,
) -> RuntimeResult<()> {
    // validate output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // resolve one raw-hid-capable device
    let device = resolve_raw_hid_device(context, handle, "destack.input.rawhid.write")?;

    // validate capability support and payload shape
    let payload = unsafe { data.as_slice()? };
    input_validation::validate_non_empty_bytes("data", payload)?;

    // write one output report and expose payload-byte count
    let written =
        raw_input::write_output_report(&device, reportid, payload, "destack.input.rawhid.write")?;
    unsafe {
        *out = written;
    }

    Ok(())
}
