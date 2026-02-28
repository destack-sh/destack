use crate::diagnostic::RuntimeResult;
use crate::platform::os::{LoadAverage, SystemSnapshot};
use crate::runtime::BindingCallContext;

/// Read one host system-information snapshot from the active backend.
pub(super) fn read_system_snapshot(context: &BindingCallContext) -> RuntimeResult<SystemSnapshot> {
    // dispatch to the platform backend
    platform_backend::read_system_snapshot(context)
}

/// Read one host uptime value from the active backend.
pub(super) fn read_uptime_ns(context: &BindingCallContext) -> RuntimeResult<u64> {
    // dispatch to the platform backend
    platform_backend::read_uptime_ns(context)
}

/// Read one host boot-time unix timestamp from the active backend.
pub(super) fn read_boot_time_unix_ns(context: &BindingCallContext) -> RuntimeResult<u64> {
    // dispatch to the platform backend
    platform_backend::read_boot_time_unix_ns(context)
}

/// Read one host load-average payload from the active backend.
pub(super) fn read_load_average(context: &BindingCallContext) -> RuntimeResult<LoadAverage> {
    // dispatch to the platform backend
    platform_backend::read_load_average(context)
}

#[cfg(unix)]
use super::unix as platform_backend;
#[cfg(not(any(unix, windows)))]
use super::unsupported as platform_backend;
#[cfg(windows)]
use super::windows as platform_backend;
