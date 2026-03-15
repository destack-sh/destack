use crate::diagnostic::RuntimeResult;
use crate::platform::core as core_platform;
use crate::platform::os::info::core::{
    OS_INFO_BOOT_TIME_UNIX_NS_OPERATION, OS_INFO_LOAD_AVERAGE_OPERATION,
    OS_INFO_SYSTEM_SNAPSHOT_OPERATION, OS_INFO_UPTIME_NS_OPERATION, checked_mul_u64,
    checked_u64_from_signed, clock_gettime_ns, invalid_data, system_time_unix_ns,
};
use crate::platform::os::{LoadAverage, SystemSnapshot};
use crate::runtime::BindingCallContext;

#[cfg(any(target_os = "linux", target_os = "android"))]
/// Clock used for uptime sampling on linux-like hosts.
const UPTIME_CLOCK_ID: libc::clockid_t = libc::CLOCK_BOOTTIME;
#[cfg(not(any(target_os = "linux", target_os = "android")))]
/// Clock used for uptime sampling on non-linux posix hosts.
const UPTIME_CLOCK_ID: libc::clockid_t = libc::CLOCK_MONOTONIC;

/// Read one positive sysconf value.
fn read_sysconf_positive(name: libc::c_int, syscall: &'static str) -> RuntimeResult<u64> {
    let value = unsafe { libc::sysconf(name) };
    if value <= 0 {
        return Err(core_platform::io_error(syscall, None));
    }

    checked_u64_from_signed(
        i128::from(value),
        OS_INFO_SYSTEM_SNAPSHOT_OPERATION,
        syscall,
    )
}

/// Read one normalized cpu-count value.
fn cpu_count_value() -> RuntimeResult<u32> {
    if let Ok(count) = std::thread::available_parallelism() {
        let count = u32::try_from(count.get()).map_err(|_| {
            invalid_data(
                OS_INFO_SYSTEM_SNAPSHOT_OPERATION,
                "availableParallelism exceeded u32 range",
            )
        })?;
        return Ok(count);
    }

    let count = read_sysconf_positive(libc::_SC_NPROCESSORS_ONLN, "sysconf(_SC_NPROCESSORS_ONLN)")?;
    u32::try_from(count).map_err(|_| {
        invalid_data(
            OS_INFO_SYSTEM_SNAPSHOT_OPERATION,
            "sysconf(_SC_NPROCESSORS_ONLN) exceeded u32 range",
        )
    })
}

/// Read one page-size value in bytes.
fn page_size_bytes() -> RuntimeResult<u64> {
    read_sysconf_positive(libc::_SC_PAGESIZE, "sysconf(_SC_PAGESIZE)")
}

/// Read total physical memory bytes.
fn total_memory_bytes(page_size: u64) -> RuntimeResult<u64> {
    let pages = read_sysconf_positive(libc::_SC_PHYS_PAGES, "sysconf(_SC_PHYS_PAGES)")?;
    checked_mul_u64(
        page_size,
        pages,
        OS_INFO_SYSTEM_SNAPSHOT_OPERATION,
        "memoryTotal",
    )
}

/// Read available physical memory bytes on posix hosts.
fn available_memory_bytes(page_size: u64) -> RuntimeResult<u64> {
    let pages = read_sysconf_positive(libc::_SC_AVPHYS_PAGES, "sysconf(_SC_AVPHYS_PAGES)")?;
    checked_mul_u64(
        page_size,
        pages,
        OS_INFO_SYSTEM_SNAPSHOT_OPERATION,
        "memoryAvailable",
    )
}

/// Read uptime value in nanoseconds.
fn uptime_ns_value() -> RuntimeResult<u64> {
    clock_gettime_ns(UPTIME_CLOCK_ID, OS_INFO_UPTIME_NS_OPERATION)
}

/// Read boot-time unix nanoseconds from realtime and uptime clocks.
fn boot_time_unix_ns_value() -> RuntimeResult<u64> {
    let now_unix_ns = system_time_unix_ns(OS_INFO_BOOT_TIME_UNIX_NS_OPERATION)?;
    let uptime_ns = uptime_ns_value()?;
    now_unix_ns.checked_sub(uptime_ns).ok_or_else(|| {
        invalid_data(
            OS_INFO_BOOT_TIME_UNIX_NS_OPERATION,
            "boot time projection underflowed against current unix time",
        )
    })
}

/// Read one host system-information snapshot from posix Unix APIs.
pub(crate) fn read_system_snapshot(_binding: &BindingCallContext) -> RuntimeResult<SystemSnapshot> {
    let cpu_count = cpu_count_value()?;
    let page_size = page_size_bytes()?;
    let memory_total = total_memory_bytes(page_size)?;
    let memory_available = available_memory_bytes(page_size)?;

    Ok(SystemSnapshot {
        cpu_count,
        memory_total,
        memory_available,
        page_size,
    })
}

/// Read one host uptime value from posix Unix APIs.
pub(crate) fn read_uptime_ns(_binding: &BindingCallContext) -> RuntimeResult<u64> {
    uptime_ns_value()
}

/// Read one host boot-time value from posix Unix APIs.
pub(crate) fn read_boot_time_unix_ns(_binding: &BindingCallContext) -> RuntimeResult<u64> {
    boot_time_unix_ns_value()
}

/// Read one host load-average payload from posix Unix APIs.
pub(crate) fn read_load_average(_binding: &BindingCallContext) -> RuntimeResult<LoadAverage> {
    #[cfg(target_os = "android")]
    {
        let payload = std::fs::read_to_string("/proc/loadavg")
            .map_err(|_| core_platform::io_error("read(/proc/loadavg)", None))?;
        let mut fields = payload.split_whitespace();
        let one = fields.next().ok_or_else(|| {
            invalid_data(
                OS_INFO_LOAD_AVERAGE_OPERATION,
                "missing 1-minute load average in /proc/loadavg",
            )
        })?;
        let five = fields.next().ok_or_else(|| {
            invalid_data(
                OS_INFO_LOAD_AVERAGE_OPERATION,
                "missing 5-minute load average in /proc/loadavg",
            )
        })?;
        let fifteen = fields.next().ok_or_else(|| {
            invalid_data(
                OS_INFO_LOAD_AVERAGE_OPERATION,
                "missing 15-minute load average in /proc/loadavg",
            )
        })?;
        let one = one.parse::<f64>().map_err(|_| {
            invalid_data(
                OS_INFO_LOAD_AVERAGE_OPERATION,
                "invalid 1-minute load average in /proc/loadavg",
            )
        })?;
        let five = five.parse::<f64>().map_err(|_| {
            invalid_data(
                OS_INFO_LOAD_AVERAGE_OPERATION,
                "invalid 5-minute load average in /proc/loadavg",
            )
        })?;
        let fifteen = fifteen.parse::<f64>().map_err(|_| {
            invalid_data(
                OS_INFO_LOAD_AVERAGE_OPERATION,
                "invalid 15-minute load average in /proc/loadavg",
            )
        })?;

        Ok(LoadAverage { one, five, fifteen })
    }

    #[cfg(not(target_os = "android"))]
    {
        let mut values = [0.0f64; 3];
        let count = unsafe { libc::getloadavg(values.as_mut_ptr(), values.len() as i32) };
        if count < 0 {
            return Err(core_platform::io_error("getloadavg", None));
        }

        if count < values.len() as i32 {
            return Err(invalid_data(
                OS_INFO_LOAD_AVERAGE_OPERATION,
                format!(
                    "getloadavg returned {count} values, expected {}",
                    values.len()
                ),
            ));
        }

        Ok(LoadAverage {
            one: values[0],
            five: values[1],
            fifteen: values[2],
        })
    }
}
