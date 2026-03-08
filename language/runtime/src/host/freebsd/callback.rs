use destack_workspace::Platform;

use crate::diagnostic::RuntimeResult;
use crate::host::unix::{
    UnixApplicationLifecycle, unix_notify_application_lifecycle, unix_notify_interruption_changed,
    unix_notify_memory_pressure_changed, unix_notify_permission_result,
    unix_notify_power_mode_changed, unix_notify_thermal_state_changed, unix_notify_wake,
    unix_notify_wall_clock_changed,
};
use crate::host::{HostMemoryPressureLevel, HostPowerMode, HostThermalState};

/// FreeBsd application lifecycle transitions from native callbacks.
pub type FreeBsdApplicationLifecycle = UnixApplicationLifecycle;

/// Submit one FreeBsd application lifecycle callback.
pub fn freebsd_notify_application_lifecycle(
    runtime_id: u64,
    lifecycle: FreeBsdApplicationLifecycle,
) -> RuntimeResult<()> {
    unix_notify_application_lifecycle(runtime_id, Platform::FreeBsd, lifecycle)
}

/// Submit one FreeBsd permission-result callback.
pub fn freebsd_notify_permission_result(
    runtime_id: u64,
    permission: &str,
    granted: bool,
) -> RuntimeResult<()> {
    unix_notify_permission_result(runtime_id, Platform::FreeBsd, permission, granted)
}

/// Submit one FreeBsd interruption callback.
pub fn freebsd_notify_interruption_changed(
    runtime_id: u64,
    interrupted: bool,
) -> RuntimeResult<()> {
    unix_notify_interruption_changed(runtime_id, Platform::FreeBsd, interrupted)
}

/// Submit one FreeBsd memory pressure callback.
pub fn freebsd_notify_memory_pressure_changed(
    runtime_id: u64,
    level: HostMemoryPressureLevel,
) -> RuntimeResult<()> {
    unix_notify_memory_pressure_changed(runtime_id, Platform::FreeBsd, level)
}

/// Submit one FreeBsd thermal state callback.
pub fn freebsd_notify_thermal_state_changed(
    runtime_id: u64,
    state: HostThermalState,
) -> RuntimeResult<()> {
    unix_notify_thermal_state_changed(runtime_id, Platform::FreeBsd, state)
}

/// Submit one FreeBsd power mode callback.
pub fn freebsd_notify_power_mode_changed(
    runtime_id: u64,
    mode: HostPowerMode,
) -> RuntimeResult<()> {
    unix_notify_power_mode_changed(runtime_id, Platform::FreeBsd, mode)
}

/// Submit one FreeBsd wall clock callback.
pub fn freebsd_notify_wall_clock_changed(runtime_id: u64) -> RuntimeResult<()> {
    unix_notify_wall_clock_changed(runtime_id, Platform::FreeBsd)
}

/// Wake one blocked host poll operation for FreeBsd.
pub fn freebsd_notify_wake(runtime_id: u64) -> RuntimeResult<()> {
    unix_notify_wake(runtime_id, Platform::FreeBsd)
}
