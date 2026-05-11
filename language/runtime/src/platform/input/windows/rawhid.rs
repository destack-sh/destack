use super::{core as input_core, raw as raw_input};

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::abi::NativeSlice;
use crate::platform::input::{InputRawHidReport, validation as input_validation};
use crate::platform::{PlatformError, resource};
use crate::runtime::BindingCallContext;

/// Resolve one opened raw-hid-capable device descriptor.
fn resolve_raw_hid_device(
    binding: &BindingCallContext,
    handle: resource::InputDeviceHandle,
    operation: &'static str,
) -> RuntimeResult<raw_input::RawInputDeviceDescriptor> {
    // resolve one opened raw-input descriptor for this handle
    let device = input_core::raw_device(binding, handle, operation)?;

    // reject non-hid devices for raw-hid operations
    if !device.supports_raw_hid {
        return Err(RuntimeError::from(PlatformError::not_supported(operation)).boxed());
    }

    Ok(device)
}

/// Read one raw-hid feature report.
pub(crate) unsafe fn destack_input_raw_hid_get_feature(
    binding: &BindingCallContext,
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
    let device = resolve_raw_hid_device(binding, handle, "destack.input.rawhid.getFeature")?;

    // read one feature report from hid backend APIs
    let report = raw_input::get_feature_report(
        &device,
        reportid,
        maxbytes,
        "destack.input.rawhid.getFeature",
    )?;

    // write report bytes into runtime-managed slice storage
    unsafe {
        *out = binding.store_slice(report);
    }

    Ok(())
}

/// Read one raw-hid report.
pub(crate) unsafe fn destack_input_raw_hid_read(
    binding: &BindingCallContext,
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
    let device = resolve_raw_hid_device(binding, handle, "destack.input.rawhid.read")?;

    // read one report with bounded timeout semantics
    let report = raw_input::read_raw_hid_report_with_timeout(
        binding,
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
pub(crate) unsafe fn destack_input_raw_hid_set_feature(
    binding: &BindingCallContext,
    handle: resource::InputDeviceHandle,
    reportid: u8,
    data: NativeSlice<u8>,
) -> RuntimeResult<()> {
    // resolve one raw-hid-capable device
    let device = resolve_raw_hid_device(binding, handle, "destack.input.rawhid.setFeature")?;

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
pub(crate) unsafe fn destack_input_raw_hid_try_read(
    binding: &BindingCallContext,
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
    let device = resolve_raw_hid_device(binding, handle, "destack.input.rawhid.tryRead")?;

    // poll one queued hid report without blocking
    let report = raw_input::read_raw_hid_report(
        binding,
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
pub(crate) unsafe fn destack_input_raw_hid_write(
    binding: &BindingCallContext,
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
    let device = resolve_raw_hid_device(binding, handle, "destack.input.rawhid.write")?;

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
