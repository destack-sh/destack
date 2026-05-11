use crate::platform::core as core_platform;
#[cfg(unix)]
use crate::platform::core::io_error;
use crate::platform::core::io_operation_error;
#[cfg(not(target_vendor = "apple"))]
use std::time::{SystemTime, UNIX_EPOCH};

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::os::{LoadAverage, SystemSnapshot};
use crate::runtime::BindingCallContext;

/// Binding operation name for system snapshot reads.
pub(crate) const OS_INFO_SYSTEM_SNAPSHOT_OPERATION: &str = "destack.os.info.systemSnapshot";
/// Binding operation name for uptime reads.
pub(crate) const OS_INFO_UPTIME_NS_OPERATION: &str = "destack.os.info.uptimeNs";
/// Binding operation name for boot time reads.
pub(crate) const OS_INFO_BOOT_TIME_UNIX_NS_OPERATION: &str = "destack.os.info.bootTimeUnixNs";
/// Binding operation name for load-average reads.
pub(crate) const OS_INFO_LOAD_AVERAGE_OPERATION: &str = "destack.os.info.loadAverage";

/// Build one ioInvalidData runtime error.
pub(super) fn invalid_data(
    operation: &'static str,
    message: impl Into<String>,
) -> Box<RuntimeError> {
    io_operation_error(operation, Some(PlatformErrorCode::IoInvalidData), message)
}

#[cfg(unix)]
/// Convert one signed integer into one checked u64 value.
pub(super) fn checked_u64_from_signed(
    value: i128,
    operation: &'static str,
    field: &'static str,
) -> RuntimeResult<u64> {
    if value < 0 {
        return Err(invalid_data(
            operation,
            format!("{field} was negative: {value}"),
        ));
    }

    u64::try_from(value)
        .map_err(|_| invalid_data(operation, format!("{field} exceeded u64 range: {value}")))
}

#[cfg(unix)]
/// Multiply two u64 values with overflow validation.
pub(super) fn checked_mul_u64(
    left: u64,
    right: u64,
    operation: &'static str,
    field: &'static str,
) -> RuntimeResult<u64> {
    left.checked_mul(right).ok_or_else(|| {
        invalid_data(
            operation,
            format!("{field} overflowed while multiplying {left} and {right}"),
        )
    })
}

/// Read one unix clock value and convert it to nanoseconds.
#[cfg(unix)]
pub(super) fn clock_gettime_ns(
    clock_id: libc::clockid_t,
    operation: &'static str,
) -> RuntimeResult<u64> {
    // read one timespec from the requested clock
    let mut value = libc::timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    let status = unsafe { libc::clock_gettime(clock_id, &mut value) };
    if status != 0 {
        return Err(io_error("clock_gettime", None));
    }

    // convert seconds and nanoseconds fields into one u64 nanosecond value
    let seconds = checked_u64_from_signed(i128::from(value.tv_sec), operation, "timespec.tv_sec")?;
    let nanoseconds =
        checked_u64_from_signed(i128::from(value.tv_nsec), operation, "timespec.tv_nsec")?;
    let seconds_ns = checked_mul_u64(seconds, 1_000_000_000, operation, "timespec.secondsNs")?;
    let total = seconds_ns
        .checked_add(nanoseconds)
        .ok_or_else(|| invalid_data(operation, "timespec to nanoseconds conversion overflowed"))?;

    Ok(total)
}

/// Read current unix time in nanoseconds.
#[cfg(not(target_vendor = "apple"))]
pub(super) fn system_time_unix_ns(operation: &'static str) -> RuntimeResult<u64> {
    // sample wall-clock unix time from std time APIs
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| {
            invalid_data(operation, format!("system time before unix epoch: {error}"))
        })?;

    // validate nanosecond conversion stays in u64 range
    u64::try_from(duration.as_nanos()).map_err(|_| {
        invalid_data(
            operation,
            "system time nanoseconds exceeded u64 range during conversion",
        )
    })
}

/// Read host system information.
pub(crate) unsafe fn destack_os_system_snapshot(
    binding: &BindingCallContext,
    out: *mut SystemSnapshot,
) -> RuntimeResult<()> {
    // validate output argument before host calls
    core_platform::ensure_out(out, "out")?;

    // query one backend snapshot and write output
    let snapshot = super::target::read_system_snapshot(binding)?;
    unsafe {
        out.write(snapshot);
    }

    Ok(())
}

/// Read host system information through the VM ABI surface.
pub(crate) fn read_system_snapshot(binding: &BindingCallContext) -> RuntimeResult<SystemSnapshot> {
    super::target::read_system_snapshot(binding)
}

/// Read host uptime.
pub(crate) unsafe fn destack_os_uptime_ns(
    binding: &BindingCallContext,
    out: *mut u64,
) -> RuntimeResult<()> {
    // validate output argument before host calls
    core_platform::ensure_out(out, "out")?;

    // query one backend uptime value and write output
    let uptime_ns = super::target::read_uptime_ns(binding)?;
    unsafe {
        out.write(uptime_ns);
    }

    Ok(())
}

/// Read host uptime through the VM ABI surface.
pub(crate) fn read_uptime_ns(binding: &BindingCallContext) -> RuntimeResult<u64> {
    super::target::read_uptime_ns(binding)
}

/// Read host boot time.
pub(crate) unsafe fn destack_os_boot_time_unix_ns(
    binding: &BindingCallContext,
    out: *mut u64,
) -> RuntimeResult<()> {
    // validate output argument before host calls
    core_platform::ensure_out(out, "out")?;

    // query one backend boot-time value and write output
    let boot_time_unix_ns = super::target::read_boot_time_unix_ns(binding)?;
    unsafe {
        out.write(boot_time_unix_ns);
    }

    Ok(())
}

/// Read host boot time through the VM ABI surface.
pub(crate) fn read_boot_time_unix_ns(binding: &BindingCallContext) -> RuntimeResult<u64> {
    super::target::read_boot_time_unix_ns(binding)
}

/// Read host load averages.
pub(crate) unsafe fn destack_os_load_average(
    binding: &BindingCallContext,
    out: *mut LoadAverage,
) -> RuntimeResult<()> {
    // validate output argument before host calls
    core_platform::ensure_out(out, "out")?;

    // query one backend load-average payload and write output
    let load_average = super::target::read_load_average(binding)?;
    unsafe {
        out.write(load_average);
    }

    Ok(())
}

/// Read host load averages through the VM ABI surface.
pub(crate) fn read_load_average(binding: &BindingCallContext) -> RuntimeResult<LoadAverage> {
    super::target::read_load_average(binding)
}
