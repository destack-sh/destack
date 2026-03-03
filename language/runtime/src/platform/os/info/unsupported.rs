use crate::diagnostic::RuntimeResult;
use crate::platform::core as core_platform;
use crate::platform::os::{LoadAverage, SystemSnapshot};
use crate::runtime::BindingCallContext;

use super::core::{
    OS_INFO_BOOT_TIME_UNIX_NS_OPERATION, OS_INFO_LOAD_AVERAGE_OPERATION,
    OS_INFO_SYSTEM_SNAPSHOT_OPERATION, OS_INFO_UPTIME_NS_OPERATION,
};

/// Read one host system snapshot from unsupported backends.
pub(super) fn read_system_snapshot(_context: &BindingCallContext) -> RuntimeResult<SystemSnapshot> {
    Err(core_platform::not_supported(
        OS_INFO_SYSTEM_SNAPSHOT_OPERATION,
    ))
}

/// Read one host uptime value from unsupported backends.
pub(super) fn read_uptime_ns(_context: &BindingCallContext) -> RuntimeResult<u64> {
    Err(core_platform::not_supported(OS_INFO_UPTIME_NS_OPERATION))
}

/// Read one host boot-time value from unsupported backends.
pub(super) fn read_boot_time_unix_ns(_context: &BindingCallContext) -> RuntimeResult<u64> {
    Err(core_platform::not_supported(
        OS_INFO_BOOT_TIME_UNIX_NS_OPERATION,
    ))
}

/// Read one host load-average payload from unsupported backends.
pub(super) fn read_load_average(_context: &BindingCallContext) -> RuntimeResult<LoadAverage> {
    Err(core_platform::not_supported(OS_INFO_LOAD_AVERAGE_OPERATION))
}
