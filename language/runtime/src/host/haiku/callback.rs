use destack_workspace::Platform;

use crate::diagnostic::RuntimeResult;
use crate::host::unix::{
    UnixApplicationLifecycle, unix_notify_application_lifecycle, unix_notify_interruption_changed,
    unix_notify_memory_pressure_changed, unix_notify_permission_result,
    unix_notify_power_mode_changed, unix_notify_thermal_state_changed, unix_notify_wake,
    unix_notify_wall_clock_changed,
};
use crate::host::{HostMemoryPressureLevel, HostPowerMode, HostThermalState};

/// Haiku application lifecycle transitions from native callbacks.
pub type HaikuApplicationLifecycle = UnixApplicationLifecycle;

/// Submit one Haiku application lifecycle callback.
pub fn haiku_notify_application_lifecycle(
    runtime_id: u64,
    lifecycle: HaikuApplicationLifecycle,
) -> RuntimeResult<()> {
    unix_notify_application_lifecycle(runtime_id, Platform::Haiku, lifecycle)
}

/// Submit one Haiku permission-result callback.
pub fn haiku_notify_permission_result(
    runtime_id: u64,
    permission: &str,
    granted: bool,
) -> RuntimeResult<()> {
    unix_notify_permission_result(runtime_id, Platform::Haiku, permission, granted)
}

/// Submit one Haiku interruption callback.
pub fn haiku_notify_interruption_changed(runtime_id: u64, interrupted: bool) -> RuntimeResult<()> {
    unix_notify_interruption_changed(runtime_id, Platform::Haiku, interrupted)
}

/// Submit one Haiku memory pressure callback.
pub fn haiku_notify_memory_pressure_changed(
    runtime_id: u64,
    level: HostMemoryPressureLevel,
) -> RuntimeResult<()> {
    unix_notify_memory_pressure_changed(runtime_id, Platform::Haiku, level)
}

/// Submit one Haiku thermal state callback.
pub fn haiku_notify_thermal_state_changed(
    runtime_id: u64,
    state: HostThermalState,
) -> RuntimeResult<()> {
    unix_notify_thermal_state_changed(runtime_id, Platform::Haiku, state)
}

/// Submit one Haiku power mode callback.
pub fn haiku_notify_power_mode_changed(runtime_id: u64, mode: HostPowerMode) -> RuntimeResult<()> {
    unix_notify_power_mode_changed(runtime_id, Platform::Haiku, mode)
}

/// Submit one Haiku wall clock callback.
pub fn haiku_notify_wall_clock_changed(runtime_id: u64) -> RuntimeResult<()> {
    unix_notify_wall_clock_changed(runtime_id, Platform::Haiku)
}

/// Wake one blocked host poll operation for Haiku.
pub fn haiku_notify_wake(runtime_id: u64) -> RuntimeResult<()> {
    unix_notify_wake(runtime_id, Platform::Haiku)
}
