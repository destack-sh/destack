use std::path::Path;
#[cfg(target_os = "linux")]
use std::time::Instant;

use super::core as input_core;
#[cfg(target_os = "linux")]
use super::linux as input_linux;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::abi::NativeSlice;
#[cfg(target_os = "linux")]
use crate::platform::core as core_platform;
#[cfg(target_os = "linux")]
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::input::{InputRawHidReport, validation as input_validation};
use crate::platform::{PlatformError, resource};
use crate::runtime::BindingCallContext;

/// Linux hidraw node prefix.
const HIDRAW_NODE_PREFIX: &str = "hidraw";
/// Linux hidraw directory path.
const HIDRAW_DIRECTORY: &str = "/dev";

/// Build one Linux ioctl request number for write payloads.
#[cfg(target_os = "linux")]
const fn iowr_request(type_byte: u8, number: u8, size: usize) -> libc::c_ulong {
    ioc_request(3, type_byte, number, size)
}

/// Build one Linux ioctl request number from direction, type, number, and size.
#[cfg(target_os = "linux")]
const fn ioc_request(direction: u8, type_byte: u8, number: u8, size: usize) -> libc::c_ulong {
    // ioctl layout constants
    // bit shift for ioctl number bits
    const IOC_NR_SHIFT: u64 = 0;
    // bit shift for ioctl type bits
    const IOC_TYPE_SHIFT: u64 = IOC_NR_SHIFT + 8;
    // bit shift for ioctl size bits
    const IOC_SIZE_SHIFT: u64 = IOC_TYPE_SHIFT + 8;
    // bit shift for ioctl direction bits
    const IOC_DIR_SHIFT: u64 = IOC_SIZE_SHIFT + 14;

    (((direction as u64) << IOC_DIR_SHIFT)
        | ((type_byte as u64) << IOC_TYPE_SHIFT)
        | ((number as u64) << IOC_NR_SHIFT)
        | ((size as u64) << IOC_SIZE_SHIFT)) as libc::c_ulong
}

/// Build HIDIOCGFEATURE request number for one report length.
#[cfg(target_os = "linux")]
fn hidiocgfeature_request(length: usize) -> libc::c_ulong {
    iowr_request(b'H', 0x07, length)
}

/// Build HIDIOCSFEATURE request number for one report length.
#[cfg(target_os = "linux")]
fn hidiocsfeature_request(length: usize) -> libc::c_ulong {
    iowr_request(b'H', 0x06, length)
}

/// Return whether one identifier points to one canonical Linux hidraw node path.
fn is_hidraw_device_id(device_id: &str) -> bool {
    #[cfg(target_os = "linux")]
    if input_linux::is_linux_hidraw_runtime_id(device_id) {
        return true;
    }

    let node_path = Path::new(device_id);
    if node_path.parent() != Some(Path::new(HIDRAW_DIRECTORY)) {
        return false;
    }

    let Some(node_name) = node_path.file_name().and_then(|name| name.to_str()) else {
        return false;
    };
    let Some(suffix) = node_name.strip_prefix(HIDRAW_NODE_PREFIX) else {
        return false;
    };
    if suffix.is_empty() {
        return false;
    }

    suffix.as_bytes().iter().all(|byte| byte.is_ascii_digit())
}

/// Build one io-would-block runtime error for one operation.
#[cfg(target_os = "linux")]
fn io_would_block(
    operation: &'static str,
    syscall: &'static str,
    errno: Option<i32>,
) -> Box<RuntimeError> {
    RuntimeError::from(PlatformError::io_with(
        Some(PlatformErrorCode::IoWouldBlock),
        None,
        errno,
        Some(syscall.to_string()),
        None,
        format!("{operation} would block: raw-hid queue is empty"),
    ))
    .boxed()
}

/// Validate one opened raw-hid-capable unix binding.
fn resolve_raw_hid_binding(
    binding: &BindingCallContext,
    handle: resource::InputDeviceHandle,
    operation: &'static str,
) -> RuntimeResult<input_core::UnixInputBinding> {
    // validate one opened unix input resolved_binding
    let resolved_binding = input_core::resolve_unix_input_binding(binding, handle, operation)?;

    // require descriptor-backed platform handles for raw-hid syscalls
    if resolved_binding.backend != input_core::UnixInputBackend::Platform
        || resolved_binding.descriptor.is_none()
    {
        return Err(RuntimeError::from(PlatformError::not_supported(operation)).boxed());
    }

    // require canonical hidraw paths for raw-hid operations
    if !is_hidraw_device_id(&resolved_binding.device_id) {
        return Err(RuntimeError::from(PlatformError::not_supported(operation)).boxed());
    }

    Ok(resolved_binding)
}

/// Poll one descriptor for readable bytes with one millisecond timeout.
#[cfg(target_os = "linux")]
fn poll_readable(descriptor: i32, timeout_ms: i32) -> RuntimeResult<bool> {
    let mut pollfd = libc::pollfd {
        fd: descriptor,
        events: libc::POLLIN,
        revents: 0,
    };
    let deadline = poll_deadline(timeout_ms);

    loop {
        let timeout_ms = poll_timeout_ms(deadline);
        let status = unsafe { libc::poll(&mut pollfd as *mut libc::pollfd, 1, timeout_ms) };
        if status > 0 {
            return Ok(true);
        }
        if status == 0 {
            return Ok(false);
        }

        let errno = core_platform::get_errno();
        if errno == libc::EINTR {
            continue;
        }

        return Err(core_platform::io_error("poll", None));
    }
}

/// Convert one poll timeout into one absolute deadline when the wait is bounded.
#[cfg(target_os = "linux")]
fn poll_deadline(timeout_ms: i32) -> Option<Instant> {
    if timeout_ms < 0 {
        return None;
    }

    let timeout = u64::try_from(timeout_ms).ok()?;
    let timeout = std::time::Duration::from_millis(timeout);

    Instant::now().checked_add(timeout)
}

/// Convert one optional poll deadline into one relative poll timeout.
#[cfg(target_os = "linux")]
fn poll_timeout_ms(deadline: Option<Instant>) -> i32 {
    let Some(deadline) = deadline else {
        return -1;
    };

    let now = Instant::now();
    if now >= deadline {
        return 0;
    }

    let remaining = deadline.saturating_duration_since(now);
    let remaining_ms = remaining.as_nanos().div_ceil(1_000_000);

    remaining_ms.min(i32::MAX as u128) as i32
}

/// Convert one timeout in nanoseconds to one poll timeout in milliseconds.
#[cfg(target_os = "linux")]
fn timeout_ns_to_ms(timeout_ns: u64) -> i32 {
    if timeout_ns == 0 {
        return 0;
    }

    let rounded_ms = timeout_ns.saturating_add(999_999).saturating_div(1_000_000);
    rounded_ms.min(i32::MAX as u64) as i32
}

/// Read one raw-hid report from one linux hidraw descriptor.
#[cfg(target_os = "linux")]
fn read_raw_hid_report_linux(
    binding: &BindingCallContext,
    handle: resource::InputDeviceHandle,
    descriptor: i32,
    maxbytes: u32,
    timeout_ns: u64,
    nonblocking: bool,
    operation: &'static str,
) -> RuntimeResult<InputRawHidReport> {
    // poll descriptor readiness before issuing the read
    let timeout_ms = if nonblocking {
        0
    } else {
        timeout_ns_to_ms(timeout_ns)
    };
    if !poll_readable(descriptor, timeout_ms)? {
        return Err(io_would_block(operation, "poll", Some(libc::EWOULDBLOCK)));
    }

    // read one report payload from the descriptor
    let mut report_bytes = vec![0u8; maxbytes as usize + 1];
    let read_status = loop {
        let read_status = unsafe {
            libc::read(
                descriptor,
                report_bytes.as_mut_ptr().cast::<libc::c_void>(),
                report_bytes.len(),
            )
        };
        if read_status >= 0 {
            break read_status;
        }

        let errno = core_platform::get_errno();
        if errno == libc::EINTR {
            continue;
        }
        if errno == libc::EAGAIN || errno == libc::EWOULDBLOCK {
            return Err(io_would_block(operation, "read", Some(errno)));
        }

        return Err(core_platform::io_error("read", None));
    };
    if read_status == 0 {
        return Err(RuntimeError::from(PlatformError::io_with(
            Some(PlatformErrorCode::IoNotFound),
            None,
            None,
            Some("read".to_string()),
            None,
            "raw-hid endpoint reached end of stream".to_string(),
        ))
        .boxed());
    }

    // project report-id and payload bytes from one backend report frame
    let read_len = read_status as usize;
    let raw_report = &report_bytes[..read_len];
    let report_id = raw_report[0];
    let payload = if report_id != 0 && raw_report.len() > 1 {
        raw_report[1..].to_vec()
    } else {
        raw_report.to_vec()
    };
    let payload_limit = usize::min(payload.len(), maxbytes as usize);
    let payload = payload[..payload_limit].to_vec();
    let sequence = input_core::next_unix_event_sequence(binding, handle, operation)?;

    Ok(InputRawHidReport {
        timestamp_ns: input_core::monotonic_timestamp_ns(),
        sequence,
        report_id,
        data: binding.store_slice(payload),
    })
}

/// Read one raw-hid feature report from one linux hidraw descriptor.
#[cfg(target_os = "linux")]
fn get_feature_linux(
    descriptor: i32,
    reportid: u8,
    maxbytes: u32,
    operation: &'static str,
) -> RuntimeResult<Vec<u8>> {
    let mut report = vec![0u8; maxbytes as usize + 1];
    report[0] = reportid;

    let status = unsafe {
        libc::ioctl(
            descriptor,
            hidiocgfeature_request(report.len()),
            report.as_mut_ptr(),
        )
    };
    if status < 0 {
        let errno = core_platform::get_errno();
        if errno == libc::EAGAIN || errno == libc::EWOULDBLOCK {
            return Err(io_would_block(
                operation,
                "ioctl(HIDIOCGFEATURE)",
                Some(errno),
            ));
        }
        return Err(core_platform::io_error("ioctl(HIDIOCGFEATURE)", None));
    }

    let read_len = usize::min(status as usize, report.len());
    let report = &report[..read_len];
    let payload = if reportid != 0 && report.len() > 1 {
        report[1..].to_vec()
    } else {
        report.to_vec()
    };
    let payload_limit = usize::min(payload.len(), maxbytes as usize);
    Ok(payload[..payload_limit].to_vec())
}

/// Write one raw-hid feature report to one linux hidraw descriptor.
#[cfg(target_os = "linux")]
fn set_feature_linux(
    descriptor: i32,
    reportid: u8,
    data: &[u8],
    operation: &'static str,
) -> RuntimeResult<()> {
    let mut report = Vec::with_capacity(data.len() + 1);
    report.push(reportid);
    report.extend_from_slice(data);

    let status = unsafe {
        libc::ioctl(
            descriptor,
            hidiocsfeature_request(report.len()),
            report.as_ptr(),
        )
    };
    if status < 0 {
        let errno = core_platform::get_errno();
        if errno == libc::EAGAIN || errno == libc::EWOULDBLOCK {
            return Err(io_would_block(
                operation,
                "ioctl(HIDIOCSFEATURE)",
                Some(errno),
            ));
        }
        return Err(core_platform::io_error("ioctl(HIDIOCSFEATURE)", None));
    }

    Ok(())
}

/// Write one raw-hid output report to one linux hidraw descriptor.
#[cfg(target_os = "linux")]
fn write_output_report_linux(
    descriptor: i32,
    reportid: u8,
    data: &[u8],
    operation: &'static str,
) -> RuntimeResult<u32> {
    let mut report = Vec::with_capacity(data.len() + 1);
    report.push(reportid);
    report.extend_from_slice(data);

    let write_status = loop {
        let write_status = unsafe {
            libc::write(
                descriptor,
                report.as_ptr().cast::<libc::c_void>(),
                report.len(),
            )
        };
        if write_status >= 0 {
            break write_status;
        }

        let errno = core_platform::get_errno();
        if errno == libc::EINTR {
            continue;
        }
        if errno == libc::EAGAIN || errno == libc::EWOULDBLOCK {
            return Err(io_would_block(operation, "write", Some(errno)));
        }

        return Err(core_platform::io_error("write", None));
    };
    if write_status == 0 {
        return Err(RuntimeError::from(PlatformError::io_with(
            Some(PlatformErrorCode::IoNotFound),
            None,
            None,
            Some("write".to_string()),
            None,
            "raw-hid endpoint reached end of stream".to_string(),
        ))
        .boxed());
    }

    let written = write_status as usize;
    let payload_bytes = written.saturating_sub(1);
    let payload_bytes = usize::min(payload_bytes, data.len());
    Ok(payload_bytes as u32)
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

    // validate argument contract and handle shape
    input_validation::validate_raw_hid_max_bytes(maxbytes)?;
    let resolved_binding =
        resolve_raw_hid_binding(binding, handle, "destack.input.rawhid.getFeature")?;
    let descriptor = resolved_binding
        .descriptor
        .ok_or_else(|| input_core::input_not_found("destack.input.rawhid.getFeature", handle))?;

    #[cfg(target_os = "linux")]
    {
        // query one feature report through hidraw ioctls
        let report = get_feature_linux(
            descriptor,
            reportid,
            maxbytes,
            "destack.input.rawhid.getFeature",
        )?;
        unsafe {
            *out = binding.store_slice(report);
        }

        Ok(())
    }

    #[cfg(not(target_os = "linux"))]
    {
        let _ = (descriptor, reportid);
        Err(RuntimeError::from(PlatformError::not_supported(
            "destack.input.rawhid.getFeature",
        ))
        .boxed())
    }
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

    // validate argument contract and handle shape
    input_validation::validate_raw_hid_max_bytes(maxbytes)?;
    let resolved_binding = resolve_raw_hid_binding(binding, handle, "destack.input.rawhid.read")?;
    let descriptor = resolved_binding
        .descriptor
        .ok_or_else(|| input_core::input_not_found("destack.input.rawhid.read", handle))?;

    #[cfg(target_os = "linux")]
    {
        // read one report payload with timeout semantics
        let report = read_raw_hid_report_linux(
            binding,
            handle,
            descriptor,
            maxbytes,
            timeoutns,
            false,
            "destack.input.rawhid.read",
        )?;
        unsafe {
            *out = report;
        }

        Ok(())
    }

    #[cfg(not(target_os = "linux"))]
    {
        let _ = (descriptor, timeoutns);
        Err(RuntimeError::from(PlatformError::not_supported("destack.input.rawhid.read")).boxed())
    }
}

/// Write one raw-hid feature report.
pub(crate) unsafe fn destack_input_raw_hid_set_feature(
    binding: &BindingCallContext,
    handle: resource::InputDeviceHandle,
    reportid: u8,
    data: NativeSlice<u8>,
) -> RuntimeResult<()> {
    // validate handle shape and payload contract
    let resolved_binding =
        resolve_raw_hid_binding(binding, handle, "destack.input.rawhid.setFeature")?;
    let descriptor = resolved_binding
        .descriptor
        .ok_or_else(|| input_core::input_not_found("destack.input.rawhid.setFeature", handle))?;
    let payload = unsafe { data.as_slice()? };
    input_validation::validate_non_empty_bytes("data", payload)?;

    #[cfg(target_os = "linux")]
    {
        // submit one feature-report write through hidraw ioctls
        set_feature_linux(
            descriptor,
            reportid,
            payload,
            "destack.input.rawhid.setFeature",
        )
    }

    #[cfg(not(target_os = "linux"))]
    {
        let _ = (descriptor, reportid, payload);
        Err(RuntimeError::from(PlatformError::not_supported(
            "destack.input.rawhid.setFeature",
        ))
        .boxed())
    }
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

    // validate argument contract and handle shape
    input_validation::validate_raw_hid_max_bytes(maxbytes)?;
    let resolved_binding =
        resolve_raw_hid_binding(binding, handle, "destack.input.rawhid.tryRead")?;
    let descriptor = resolved_binding
        .descriptor
        .ok_or_else(|| input_core::input_not_found("destack.input.rawhid.tryRead", handle))?;

    #[cfg(target_os = "linux")]
    {
        // poll one report payload without blocking
        let report = read_raw_hid_report_linux(
            binding,
            handle,
            descriptor,
            maxbytes,
            0,
            true,
            "destack.input.rawhid.tryRead",
        )?;
        unsafe {
            *out = report;
        }

        Ok(())
    }

    #[cfg(not(target_os = "linux"))]
    {
        let _ = (descriptor, maxbytes);
        Err(
            RuntimeError::from(PlatformError::not_supported("destack.input.rawhid.tryRead"))
                .boxed(),
        )
    }
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

    // validate handle shape and payload contract
    let resolved_binding = resolve_raw_hid_binding(binding, handle, "destack.input.rawhid.write")?;
    let descriptor = resolved_binding
        .descriptor
        .ok_or_else(|| input_core::input_not_found("destack.input.rawhid.write", handle))?;
    let payload = unsafe { data.as_slice()? };
    input_validation::validate_non_empty_bytes("data", payload)?;

    #[cfg(target_os = "linux")]
    {
        // submit one output-report write through hidraw
        let written =
            write_output_report_linux(descriptor, reportid, payload, "destack.input.rawhid.write")?;
        unsafe {
            *out = written;
        }

        Ok(())
    }

    #[cfg(not(target_os = "linux"))]
    {
        let _ = (descriptor, reportid, payload);
        Err(RuntimeError::from(PlatformError::not_supported("destack.input.rawhid.write")).boxed())
    }
}
