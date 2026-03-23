use destack_artifact::Platform;

use crate::diagnostic::RuntimeResult;
use crate::host::unix::ingress::notify::{
    UnixApplicationLifecycle, unix_notify_application_lifecycle, unix_notify_interruption_changed,
    unix_notify_memory_pressure_changed, unix_notify_permission_result,
    unix_notify_power_mode_changed, unix_notify_thermal_state_changed, unix_notify_wake,
    unix_notify_wall_clock_changed,
};
use crate::host::{HostMemoryPressureLevel, HostPowerMode, HostThermalState};

/// FreeBsd application lifecycle transitions from native ingress hooks.
pub type FreeBsdApplicationLifecycle = UnixApplicationLifecycle;

/// Route one FreeBsd application lifecycle ingress notification.
pub fn freebsd_notify_application_lifecycle(
    runtime_id: u64,
    lifecycle: FreeBsdApplicationLifecycle,
) -> RuntimeResult<()> {
    unix_notify_application_lifecycle(runtime_id, Platform::FreeBsd, lifecycle)
}

/// Route one FreeBsd permission-result ingress notification.
pub fn freebsd_notify_permission_result(
    runtime_id: u64,
    permission: &str,
    granted: bool,
) -> RuntimeResult<()> {
    unix_notify_permission_result(runtime_id, Platform::FreeBsd, permission, granted)
}

/// Route one FreeBsd interruption ingress notification.
pub fn freebsd_notify_interruption_changed(
    runtime_id: u64,
    interrupted: bool,
) -> RuntimeResult<()> {
    unix_notify_interruption_changed(runtime_id, Platform::FreeBsd, interrupted)
}

/// Route one FreeBsd memory pressure ingress notification.
pub fn freebsd_notify_memory_pressure_changed(
    runtime_id: u64,
    level: HostMemoryPressureLevel,
) -> RuntimeResult<()> {
    unix_notify_memory_pressure_changed(runtime_id, Platform::FreeBsd, level)
}

/// Route one FreeBsd thermal state ingress notification.
pub fn freebsd_notify_thermal_state_changed(
    runtime_id: u64,
    state: HostThermalState,
) -> RuntimeResult<()> {
    unix_notify_thermal_state_changed(runtime_id, Platform::FreeBsd, state)
}

/// Route one FreeBsd power mode ingress notification.
pub fn freebsd_notify_power_mode_changed(
    runtime_id: u64,
    mode: HostPowerMode,
) -> RuntimeResult<()> {
    unix_notify_power_mode_changed(runtime_id, Platform::FreeBsd, mode)
}

/// Route one FreeBsd wall clock ingress notification.
pub fn freebsd_notify_wall_clock_changed(runtime_id: u64) -> RuntimeResult<()> {
    unix_notify_wall_clock_changed(runtime_id, Platform::FreeBsd)
}

/// Wake one blocked host poll operation for FreeBsd.
pub fn freebsd_notify_wake(runtime_id: u64) -> RuntimeResult<()> {
    unix_notify_wake(runtime_id, Platform::FreeBsd)
}
