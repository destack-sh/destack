use destack_workspace::Platform;

use crate::diagnostic::RuntimeResult;
use crate::host::unix::{
    UnixApplicationLifecycle, unix_notify_application_lifecycle, unix_notify_interruption_changed,
    unix_notify_memory_pressure_changed, unix_notify_permission_result,
    unix_notify_power_mode_changed, unix_notify_thermal_state_changed, unix_notify_wake,
    unix_notify_wall_clock_changed,
};
use crate::host::{HostMemoryPressureLevel, HostPowerMode, HostThermalState};

/// Linux application lifecycle transitions from native callbacks.
pub type LinuxApplicationLifecycle = UnixApplicationLifecycle;

/// Submit one Linux application lifecycle callback.
pub fn linux_notify_application_lifecycle(
    runtime_id: u64,
    lifecycle: LinuxApplicationLifecycle,
) -> RuntimeResult<()> {
    unix_notify_application_lifecycle(runtime_id, Platform::Linux, lifecycle)
}

/// Submit one Linux permission-result callback.
pub fn linux_notify_permission_result(
    runtime_id: u64,
    permission: &str,
    granted: bool,
) -> RuntimeResult<()> {
    unix_notify_permission_result(runtime_id, Platform::Linux, permission, granted)
}

/// Submit one Linux interruption callback.
pub fn linux_notify_interruption_changed(runtime_id: u64, interrupted: bool) -> RuntimeResult<()> {
    unix_notify_interruption_changed(runtime_id, Platform::Linux, interrupted)
}

/// Submit one Linux memory pressure callback.
pub fn linux_notify_memory_pressure_changed(
    runtime_id: u64,
    level: HostMemoryPressureLevel,
) -> RuntimeResult<()> {
    unix_notify_memory_pressure_changed(runtime_id, Platform::Linux, level)
}

/// Submit one Linux thermal state callback.
pub fn linux_notify_thermal_state_changed(
    runtime_id: u64,
    state: HostThermalState,
) -> RuntimeResult<()> {
    unix_notify_thermal_state_changed(runtime_id, Platform::Linux, state)
}

/// Submit one Linux power mode callback.
pub fn linux_notify_power_mode_changed(runtime_id: u64, mode: HostPowerMode) -> RuntimeResult<()> {
    unix_notify_power_mode_changed(runtime_id, Platform::Linux, mode)
}

/// Submit one Linux wall clock callback.
pub fn linux_notify_wall_clock_changed(runtime_id: u64) -> RuntimeResult<()> {
    unix_notify_wall_clock_changed(runtime_id, Platform::Linux)
}

/// Wake one blocked host poll operation for Linux.
pub fn linux_notify_wake(runtime_id: u64) -> RuntimeResult<()> {
    unix_notify_wake(runtime_id, Platform::Linux)
}
