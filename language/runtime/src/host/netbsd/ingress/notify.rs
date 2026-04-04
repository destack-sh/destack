use destack_artifact::Platform;

use crate::diagnostic::RuntimeResult;
use crate::host::os::unix::ingress::notify::{
    UnixApplicationLifecycle, unix_notify_application_lifecycle, unix_notify_interruption_changed,
    unix_notify_memory_pressure_changed, unix_notify_permission_result,
    unix_notify_power_mode_changed, unix_notify_thermal_state_changed, unix_notify_wake,
    unix_notify_wall_clock_changed,
};
use crate::host::{HostMemoryPressureLevel, HostPowerMode, HostThermalState};

/// NetBsd application lifecycle transitions from native ingress hooks.
pub type NetBsdApplicationLifecycle = UnixApplicationLifecycle;

/// Route one NetBsd application lifecycle ingress notification.
pub fn netbsd_notify_application_lifecycle(
    runtime_id: u64,
    lifecycle: NetBsdApplicationLifecycle,
) -> RuntimeResult<()> {
    unix_notify_application_lifecycle(runtime_id, Platform::NetBsd, lifecycle)
}

/// Route one NetBsd permission-result ingress notification.
pub fn netbsd_notify_permission_result(
    runtime_id: u64,
    permission: &str,
    granted: bool,
) -> RuntimeResult<()> {
    unix_notify_permission_result(runtime_id, Platform::NetBsd, permission, granted)
}

/// Route one NetBsd interruption ingress notification.
pub fn netbsd_notify_interruption_changed(runtime_id: u64, interrupted: bool) -> RuntimeResult<()> {
    unix_notify_interruption_changed(runtime_id, Platform::NetBsd, interrupted)
}

/// Route one NetBsd memory pressure ingress notification.
pub fn netbsd_notify_memory_pressure_changed(
    runtime_id: u64,
    level: HostMemoryPressureLevel,
) -> RuntimeResult<()> {
    unix_notify_memory_pressure_changed(runtime_id, Platform::NetBsd, level)
}

/// Route one NetBsd thermal state ingress notification.
pub fn netbsd_notify_thermal_state_changed(
    runtime_id: u64,
    state: HostThermalState,
) -> RuntimeResult<()> {
    unix_notify_thermal_state_changed(runtime_id, Platform::NetBsd, state)
}

/// Route one NetBsd power mode ingress notification.
pub fn netbsd_notify_power_mode_changed(runtime_id: u64, mode: HostPowerMode) -> RuntimeResult<()> {
    unix_notify_power_mode_changed(runtime_id, Platform::NetBsd, mode)
}

/// Route one NetBsd wall clock ingress notification.
pub fn netbsd_notify_wall_clock_changed(runtime_id: u64) -> RuntimeResult<()> {
    unix_notify_wall_clock_changed(runtime_id, Platform::NetBsd)
}

/// Wake one blocked host poll operation for NetBsd.
pub fn netbsd_notify_wake(runtime_id: u64) -> RuntimeResult<()> {
    unix_notify_wake(runtime_id, Platform::NetBsd)
}
