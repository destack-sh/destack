use std::mem::MaybeUninit;

use crate::diagnostic::RuntimeResult;
use crate::platform::core as core_platform;
use crate::platform::os::info::core::{
    OS_INFO_BOOT_TIME_UNIX_NS_OPERATION, OS_INFO_LOAD_AVERAGE_OPERATION,
    OS_INFO_SYSTEM_SNAPSHOT_OPERATION, OS_INFO_UPTIME_NS_OPERATION, checked_mul_u64,
    checked_u64_from_signed, clock_gettime_ns, invalid_data,
};
use crate::platform::os::{LoadAverage, SystemSnapshot};
use crate::runtime::BindingCallContext;

/// Clock used for uptime sampling on apple Unix hosts.
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

/// Read available physical memory bytes on apple hosts.
fn available_memory_bytes(page_size: u64) -> RuntimeResult<u64> {
    let mut statistics = MaybeUninit::<libc::vm_statistics64>::zeroed();
    let mut count = libc::HOST_VM_INFO64_COUNT;
    #[allow(deprecated)]
    let host = unsafe { libc::mach_host_self() };
    let status = unsafe {
        libc::host_statistics64(
            host,
            libc::HOST_VM_INFO64,
            statistics.as_mut_ptr().cast::<libc::integer_t>(),
            &mut count,
        )
    };
    if status != libc::KERN_SUCCESS {
        return Err(invalid_data(
            OS_INFO_SYSTEM_SNAPSHOT_OPERATION,
            format!("host_statistics64 failed with status {status}"),
        ));
    }

    let statistics = unsafe { statistics.assume_init() };
    let free_pages = u64::from(statistics.free_count);
    let inactive_pages = u64::from(statistics.inactive_count);
    let speculative_pages = u64::from(statistics.speculative_count);
    let available_pages = free_pages
        .checked_add(inactive_pages)
        .and_then(|value| value.checked_add(speculative_pages))
        .ok_or_else(|| {
            invalid_data(
                OS_INFO_SYSTEM_SNAPSHOT_OPERATION,
                "available page count overflowed during accumulation",
            )
        })?;
    checked_mul_u64(
        page_size,
        available_pages,
        OS_INFO_SYSTEM_SNAPSHOT_OPERATION,
        "memoryAvailable",
    )
}

/// Read uptime value in nanoseconds.
fn uptime_ns_value() -> RuntimeResult<u64> {
    clock_gettime_ns(UPTIME_CLOCK_ID, OS_INFO_UPTIME_NS_OPERATION)
}

/// Read boot-time unix nanoseconds from sysctl on apple hosts.
fn boot_time_unix_ns_value() -> RuntimeResult<u64> {
    let mut mib = [libc::CTL_KERN, libc::KERN_BOOTTIME];
    let mut value = libc::timeval {
        tv_sec: 0,
        tv_usec: 0,
    };
    let mut size = std::mem::size_of::<libc::timeval>();
    let status = unsafe {
        libc::sysctl(
            mib.as_mut_ptr(),
            mib.len() as u32,
            (&mut value as *mut libc::timeval).cast(),
            &mut size,
            std::ptr::null_mut(),
            0,
        )
    };
    if status != 0 {
        return Err(core_platform::io_error("sysctl(KERN_BOOTTIME)", None));
    }

    if size != std::mem::size_of::<libc::timeval>() {
        return Err(invalid_data(
            OS_INFO_BOOT_TIME_UNIX_NS_OPERATION,
            "sysctl(KERN_BOOTTIME) returned unexpected payload size",
        ));
    }

    let seconds = checked_u64_from_signed(
        i128::from(value.tv_sec),
        OS_INFO_BOOT_TIME_UNIX_NS_OPERATION,
        "bootTimeSeconds",
    )?;
    let micros = checked_u64_from_signed(
        i128::from(value.tv_usec),
        OS_INFO_BOOT_TIME_UNIX_NS_OPERATION,
        "bootTimeMicros",
    )?;
    let seconds_ns = checked_mul_u64(
        seconds,
        1_000_000_000,
        OS_INFO_BOOT_TIME_UNIX_NS_OPERATION,
        "bootTimeSecondsNs",
    )?;
    let micros_ns = checked_mul_u64(
        micros,
        1_000,
        OS_INFO_BOOT_TIME_UNIX_NS_OPERATION,
        "bootTimeMicrosNs",
    )?;
    seconds_ns.checked_add(micros_ns).ok_or_else(|| {
        invalid_data(
            OS_INFO_BOOT_TIME_UNIX_NS_OPERATION,
            "boot-time nanoseconds overflowed during conversion",
        )
    })
}

/// Read one host system-information snapshot from apple Unix APIs.
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

/// Read one host uptime value from apple Unix APIs.
pub(crate) fn read_uptime_ns(_binding: &BindingCallContext) -> RuntimeResult<u64> {
    uptime_ns_value()
}

/// Read one host boot-time value from apple Unix APIs.
pub(crate) fn read_boot_time_unix_ns(_binding: &BindingCallContext) -> RuntimeResult<u64> {
    boot_time_unix_ns_value()
}

/// Read one host load-average payload from apple Unix APIs.
pub(crate) fn read_load_average(_binding: &BindingCallContext) -> RuntimeResult<LoadAverage> {
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
