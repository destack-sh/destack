use destack_workspace::Platform;

use crate::diagnostic::RuntimeResult;
use crate::host::unix::{
    UnixApplicationLifecycle, unix_notify_application_lifecycle, unix_notify_interruption_changed,
    unix_notify_memory_pressure_changed, unix_notify_permission_result,
    unix_notify_power_mode_changed, unix_notify_thermal_state_changed, unix_notify_wake,
    unix_notify_wall_clock_changed,
};
use crate::host::{HostMemoryPressureLevel, HostPowerMode, HostThermalState};

/// OpenBsd application lifecycle transitions from native ingress hooks.
pub type OpenBsdApplicationLifecycle = UnixApplicationLifecycle;

/// Route one OpenBsd application lifecycle ingress notification.
pub fn openbsd_notify_application_lifecycle(
    runtime_id: u64,
    lifecycle: OpenBsdApplicationLifecycle,
) -> RuntimeResult<()> {
    unix_notify_application_lifecycle(runtime_id, Platform::OpenBsd, lifecycle)
}

/// Route one OpenBsd permission-result ingress notification.
pub fn openbsd_notify_permission_result(
    runtime_id: u64,
    permission: &str,
    granted: bool,
) -> RuntimeResult<()> {
    unix_notify_permission_result(runtime_id, Platform::OpenBsd, permission, granted)
}

/// Route one OpenBsd interruption ingress notification.
pub fn openbsd_notify_interruption_changed(
    runtime_id: u64,
    interrupted: bool,
) -> RuntimeResult<()> {
    unix_notify_interruption_changed(runtime_id, Platform::OpenBsd, interrupted)
}

/// Route one OpenBsd memory pressure ingress notification.
pub fn openbsd_notify_memory_pressure_changed(
    runtime_id: u64,
    level: HostMemoryPressureLevel,
) -> RuntimeResult<()> {
    unix_notify_memory_pressure_changed(runtime_id, Platform::OpenBsd, level)
}

/// Route one OpenBsd thermal state ingress notification.
pub fn openbsd_notify_thermal_state_changed(
    runtime_id: u64,
    state: HostThermalState,
) -> RuntimeResult<()> {
    unix_notify_thermal_state_changed(runtime_id, Platform::OpenBsd, state)
}

/// Route one OpenBsd power mode ingress notification.
pub fn openbsd_notify_power_mode_changed(
    runtime_id: u64,
    mode: HostPowerMode,
) -> RuntimeResult<()> {
    unix_notify_power_mode_changed(runtime_id, Platform::OpenBsd, mode)
}

/// Route one OpenBsd wall clock ingress notification.
pub fn openbsd_notify_wall_clock_changed(runtime_id: u64) -> RuntimeResult<()> {
    unix_notify_wall_clock_changed(runtime_id, Platform::OpenBsd)
}

/// Wake one blocked host poll operation for OpenBsd.
pub fn openbsd_notify_wake(runtime_id: u64) -> RuntimeResult<()> {
    unix_notify_wake(runtime_id, Platform::OpenBsd)
}
