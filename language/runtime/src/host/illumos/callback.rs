use destack_workspace::Platform;

use crate::diagnostic::RuntimeResult;
use crate::host::unix::{
    UnixApplicationLifecycle, unix_notify_application_lifecycle, unix_notify_interruption_changed,
    unix_notify_memory_pressure_changed, unix_notify_permission_result,
    unix_notify_power_mode_changed, unix_notify_thermal_state_changed, unix_notify_wake,
    unix_notify_wall_clock_changed,
};
use crate::host::{HostMemoryPressureLevel, HostPowerMode, HostThermalState};

/// Illumos application lifecycle transitions from native callbacks.
pub type IllumosApplicationLifecycle = UnixApplicationLifecycle;

/// Submit one Illumos application lifecycle callback.
pub fn illumos_notify_application_lifecycle(
    runtime_id: u64,
    lifecycle: IllumosApplicationLifecycle,
) -> RuntimeResult<()> {
    unix_notify_application_lifecycle(runtime_id, Platform::Illumos, lifecycle)
}

/// Submit one Illumos permission-result callback.
pub fn illumos_notify_permission_result(
    runtime_id: u64,
    permission: &str,
    granted: bool,
) -> RuntimeResult<()> {
    unix_notify_permission_result(runtime_id, Platform::Illumos, permission, granted)
}

/// Submit one Illumos interruption callback.
pub fn illumos_notify_interruption_changed(
    runtime_id: u64,
    interrupted: bool,
) -> RuntimeResult<()> {
    unix_notify_interruption_changed(runtime_id, Platform::Illumos, interrupted)
}

/// Submit one Illumos memory pressure callback.
pub fn illumos_notify_memory_pressure_changed(
    runtime_id: u64,
    level: HostMemoryPressureLevel,
) -> RuntimeResult<()> {
    unix_notify_memory_pressure_changed(runtime_id, Platform::Illumos, level)
}

/// Submit one Illumos thermal state callback.
pub fn illumos_notify_thermal_state_changed(
    runtime_id: u64,
    state: HostThermalState,
) -> RuntimeResult<()> {
    unix_notify_thermal_state_changed(runtime_id, Platform::Illumos, state)
}

/// Submit one Illumos power mode callback.
pub fn illumos_notify_power_mode_changed(
    runtime_id: u64,
    mode: HostPowerMode,
) -> RuntimeResult<()> {
    unix_notify_power_mode_changed(runtime_id, Platform::Illumos, mode)
}

/// Submit one Illumos wall clock callback.
pub fn illumos_notify_wall_clock_changed(runtime_id: u64) -> RuntimeResult<()> {
    unix_notify_wall_clock_changed(runtime_id, Platform::Illumos)
}

/// Wake one blocked host poll operation for Illumos.
pub fn illumos_notify_wake(runtime_id: u64) -> RuntimeResult<()> {
    unix_notify_wake(runtime_id, Platform::Illumos)
}
