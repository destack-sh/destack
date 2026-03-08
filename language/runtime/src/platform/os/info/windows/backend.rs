use windows_sys::Win32::System::SystemInformation::{
    GetNativeSystemInfo, GetTickCount64, GlobalMemoryStatusEx, MEMORYSTATUSEX, SYSTEM_INFO,
};

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::os::{LoadAverage, SystemSnapshot};
use crate::platform::{PlatformError, core as core_platform};
use crate::runtime::BindingCallContext;

use super::super::core::{
    OS_INFO_BOOT_TIME_UNIX_NS_OPERATION, OS_INFO_LOAD_AVERAGE_OPERATION,
    OS_INFO_SYSTEM_SNAPSHOT_OPERATION, OS_INFO_UPTIME_NS_OPERATION, invalid_data,
    system_time_unix_ns,
};

/// Read one host system-information payload from windows APIs.
fn system_snapshot_value() -> RuntimeResult<SystemSnapshot> {
    // query processor topology and page-size metadata
    let mut info = unsafe { std::mem::zeroed::<SYSTEM_INFO>() };
    unsafe {
        GetNativeSystemInfo(&mut info);
    }
    if info.dwNumberOfProcessors == 0 {
        return Err(invalid_data(
            OS_INFO_SYSTEM_SNAPSHOT_OPERATION,
            "GetNativeSystemInfo returned zero processors",
        ));
    }
    if info.dwPageSize == 0 {
        return Err(invalid_data(
            OS_INFO_SYSTEM_SNAPSHOT_OPERATION,
            "GetNativeSystemInfo returned zero page size",
        ));
    }

    // query memory-capacity counters
    let mut memory = unsafe { std::mem::zeroed::<MEMORYSTATUSEX>() };
    memory.dwLength = std::mem::size_of::<MEMORYSTATUSEX>() as u32;
    let status = unsafe { GlobalMemoryStatusEx(&mut memory) };
    if status == 0 {
        return Err(core_platform::io_error("GlobalMemoryStatusEx"));
    }

    Ok(SystemSnapshot {
        cpu_count: info.dwNumberOfProcessors,
        memory_total: memory.ullTotalPhys,
        memory_available: memory.ullAvailPhys,
        page_size: u64::from(info.dwPageSize),
    })
}

/// Read uptime value in nanoseconds.
fn uptime_ns_value() -> RuntimeResult<u64> {
    // query boot-relative uptime in milliseconds
    let uptime_ms = unsafe { GetTickCount64() };

    // convert milliseconds into nanoseconds with overflow validation
    uptime_ms.checked_mul(1_000_000).ok_or_else(|| {
        invalid_data(
            OS_INFO_UPTIME_NS_OPERATION,
            "GetTickCount64 nanosecond conversion overflowed",
        )
    })
}

/// Read boot-time unix nanoseconds from realtime and uptime clocks.
fn boot_time_unix_ns_value() -> RuntimeResult<u64> {
    // sample realtime and uptime, then project boot time
    let now_unix_ns = system_time_unix_ns(OS_INFO_BOOT_TIME_UNIX_NS_OPERATION)?;
    let uptime_ns = uptime_ns_value()?;
    now_unix_ns.checked_sub(uptime_ns).ok_or_else(|| {
        invalid_data(
            OS_INFO_BOOT_TIME_UNIX_NS_OPERATION,
            "boot time projection underflowed against current unix time",
        )
    })
}

/// Read one host system-information snapshot from windows APIs.
pub(crate) fn read_system_snapshot(_binding: &BindingCallContext) -> RuntimeResult<SystemSnapshot> {
    system_snapshot_value()
}

/// Read one host uptime value from windows APIs.
pub(crate) fn read_uptime_ns(_binding: &BindingCallContext) -> RuntimeResult<u64> {
    uptime_ns_value()
}

/// Read one host boot-time value from windows APIs.
pub(crate) fn read_boot_time_unix_ns(_binding: &BindingCallContext) -> RuntimeResult<u64> {
    boot_time_unix_ns_value()
}

/// Read one host load-average payload from windows APIs.
pub(crate) fn read_load_average(_binding: &BindingCallContext) -> RuntimeResult<LoadAverage> {
    Err(RuntimeError::from(PlatformError::not_supported(OS_INFO_LOAD_AVERAGE_OPERATION)).boxed())
}
