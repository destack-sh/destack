use destack_workspace::Platform;

use crate::diagnostic::RuntimeResult;
use crate::host::unix::{
    UnixApplicationLifecycle, unix_notify_application_lifecycle, unix_notify_interruption_changed,
    unix_notify_memory_pressure_changed, unix_notify_permission_result,
    unix_notify_power_mode_changed, unix_notify_thermal_state_changed, unix_notify_wake,
    unix_notify_wall_clock_changed,
};
use crate::host::{HostMemoryPressureLevel, HostPowerMode, HostThermalState};

/// Solaris application lifecycle transitions from native callbacks.
pub type SolarisApplicationLifecycle = UnixApplicationLifecycle;

/// Submit one Solaris application lifecycle callback.
pub fn solaris_notify_application_lifecycle(
    runtime_id: u64,
    lifecycle: SolarisApplicationLifecycle,
) -> RuntimeResult<()> {
    unix_notify_application_lifecycle(runtime_id, Platform::Solaris, lifecycle)
}

/// Submit one Solaris permission-result callback.
pub fn solaris_notify_permission_result(
    runtime_id: u64,
    permission: &str,
    granted: bool,
) -> RuntimeResult<()> {
    unix_notify_permission_result(runtime_id, Platform::Solaris, permission, granted)
}

/// Submit one Solaris interruption callback.
pub fn solaris_notify_interruption_changed(
    runtime_id: u64,
    interrupted: bool,
) -> RuntimeResult<()> {
    unix_notify_interruption_changed(runtime_id, Platform::Solaris, interrupted)
}

/// Submit one Solaris memory pressure callback.
pub fn solaris_notify_memory_pressure_changed(
    runtime_id: u64,
    level: HostMemoryPressureLevel,
) -> RuntimeResult<()> {
    unix_notify_memory_pressure_changed(runtime_id, Platform::Solaris, level)
}

/// Submit one Solaris thermal state callback.
pub fn solaris_notify_thermal_state_changed(
    runtime_id: u64,
    state: HostThermalState,
) -> RuntimeResult<()> {
    unix_notify_thermal_state_changed(runtime_id, Platform::Solaris, state)
}

/// Submit one Solaris power mode callback.
pub fn solaris_notify_power_mode_changed(
    runtime_id: u64,
    mode: HostPowerMode,
) -> RuntimeResult<()> {
    unix_notify_power_mode_changed(runtime_id, Platform::Solaris, mode)
}

/// Submit one Solaris wall clock callback.
pub fn solaris_notify_wall_clock_changed(runtime_id: u64) -> RuntimeResult<()> {
    unix_notify_wall_clock_changed(runtime_id, Platform::Solaris)
}

/// Wake one blocked host poll operation for Solaris.
pub fn solaris_notify_wake(runtime_id: u64) -> RuntimeResult<()> {
    unix_notify_wake(runtime_id, Platform::Solaris)
}
