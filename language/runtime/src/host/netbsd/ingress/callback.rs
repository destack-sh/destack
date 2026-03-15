use destack_workspace::Platform;

use crate::diagnostic::RuntimeResult;
use crate::host::unix::{
    UnixApplicationLifecycle, unix_notify_application_lifecycle, unix_notify_interruption_changed,
    unix_notify_memory_pressure_changed, unix_notify_permission_result,
    unix_notify_power_mode_changed, unix_notify_thermal_state_changed, unix_notify_wake,
    unix_notify_wall_clock_changed,
};
use crate::host::{HostMemoryPressureLevel, HostPowerMode, HostThermalState};

/// NetBsd application lifecycle transitions from native callbacks.
pub type NetBsdApplicationLifecycle = UnixApplicationLifecycle;

/// Submit one NetBsd application lifecycle callback.
pub fn netbsd_notify_application_lifecycle(
    runtime_id: u64,
    lifecycle: NetBsdApplicationLifecycle,
) -> RuntimeResult<()> {
    unix_notify_application_lifecycle(runtime_id, Platform::NetBsd, lifecycle)
}

/// Submit one NetBsd permission-result callback.
pub fn netbsd_notify_permission_result(
    runtime_id: u64,
    permission: &str,
    granted: bool,
) -> RuntimeResult<()> {
    unix_notify_permission_result(runtime_id, Platform::NetBsd, permission, granted)
}

/// Submit one NetBsd interruption callback.
pub fn netbsd_notify_interruption_changed(runtime_id: u64, interrupted: bool) -> RuntimeResult<()> {
    unix_notify_interruption_changed(runtime_id, Platform::NetBsd, interrupted)
}

/// Submit one NetBsd memory pressure callback.
pub fn netbsd_notify_memory_pressure_changed(
    runtime_id: u64,
    level: HostMemoryPressureLevel,
) -> RuntimeResult<()> {
    unix_notify_memory_pressure_changed(runtime_id, Platform::NetBsd, level)
}

/// Submit one NetBsd thermal state callback.
pub fn netbsd_notify_thermal_state_changed(
    runtime_id: u64,
    state: HostThermalState,
) -> RuntimeResult<()> {
    unix_notify_thermal_state_changed(runtime_id, Platform::NetBsd, state)
}

/// Submit one NetBsd power mode callback.
pub fn netbsd_notify_power_mode_changed(runtime_id: u64, mode: HostPowerMode) -> RuntimeResult<()> {
    unix_notify_power_mode_changed(runtime_id, Platform::NetBsd, mode)
}

/// Submit one NetBsd wall clock callback.
pub fn netbsd_notify_wall_clock_changed(runtime_id: u64) -> RuntimeResult<()> {
    unix_notify_wall_clock_changed(runtime_id, Platform::NetBsd)
}

/// Wake one blocked host poll operation for NetBsd.
pub fn netbsd_notify_wake(runtime_id: u64) -> RuntimeResult<()> {
    unix_notify_wake(runtime_id, Platform::NetBsd)
}
