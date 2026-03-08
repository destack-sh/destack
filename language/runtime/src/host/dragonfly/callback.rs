use destack_workspace::Platform;

use crate::diagnostic::RuntimeResult;
use crate::host::unix::{
    UnixApplicationLifecycle, unix_notify_application_lifecycle, unix_notify_interruption_changed,
    unix_notify_memory_pressure_changed, unix_notify_permission_result,
    unix_notify_power_mode_changed, unix_notify_thermal_state_changed, unix_notify_wake,
    unix_notify_wall_clock_changed,
};
use crate::host::{HostMemoryPressureLevel, HostPowerMode, HostThermalState};

/// Dragonfly application lifecycle transitions from native callbacks.
pub type DragonflyApplicationLifecycle = UnixApplicationLifecycle;

/// Submit one Dragonfly application lifecycle callback.
pub fn dragonfly_notify_application_lifecycle(
    runtime_id: u64,
    lifecycle: DragonflyApplicationLifecycle,
) -> RuntimeResult<()> {
    unix_notify_application_lifecycle(runtime_id, Platform::DragonFly, lifecycle)
}

/// Submit one Dragonfly permission-result callback.
pub fn dragonfly_notify_permission_result(
    runtime_id: u64,
    permission: &str,
    granted: bool,
) -> RuntimeResult<()> {
    unix_notify_permission_result(runtime_id, Platform::DragonFly, permission, granted)
}

/// Submit one Dragonfly interruption callback.
pub fn dragonfly_notify_interruption_changed(
    runtime_id: u64,
    interrupted: bool,
) -> RuntimeResult<()> {
    unix_notify_interruption_changed(runtime_id, Platform::DragonFly, interrupted)
}

/// Submit one Dragonfly memory pressure callback.
pub fn dragonfly_notify_memory_pressure_changed(
    runtime_id: u64,
    level: HostMemoryPressureLevel,
) -> RuntimeResult<()> {
    unix_notify_memory_pressure_changed(runtime_id, Platform::DragonFly, level)
}

/// Submit one Dragonfly thermal state callback.
pub fn dragonfly_notify_thermal_state_changed(
    runtime_id: u64,
    state: HostThermalState,
) -> RuntimeResult<()> {
    unix_notify_thermal_state_changed(runtime_id, Platform::DragonFly, state)
}

/// Submit one Dragonfly power mode callback.
pub fn dragonfly_notify_power_mode_changed(
    runtime_id: u64,
    mode: HostPowerMode,
) -> RuntimeResult<()> {
    unix_notify_power_mode_changed(runtime_id, Platform::DragonFly, mode)
}

/// Submit one Dragonfly wall clock callback.
pub fn dragonfly_notify_wall_clock_changed(runtime_id: u64) -> RuntimeResult<()> {
    unix_notify_wall_clock_changed(runtime_id, Platform::DragonFly)
}

/// Wake one blocked host poll operation for Dragonfly.
pub fn dragonfly_notify_wake(runtime_id: u64) -> RuntimeResult<()> {
    unix_notify_wake(runtime_id, Platform::DragonFly)
}
