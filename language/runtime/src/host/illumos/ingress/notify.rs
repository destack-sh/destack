use destack_artifact::Platform;

use crate::diagnostic::RuntimeResult;
use crate::host::unix::ingress::notify::{
    UnixApplicationLifecycle, unix_notify_application_lifecycle, unix_notify_interruption_changed,
    unix_notify_memory_pressure_changed, unix_notify_permission_result,
    unix_notify_power_mode_changed, unix_notify_thermal_state_changed, unix_notify_wake,
    unix_notify_wall_clock_changed,
};
use crate::host::{HostMemoryPressureLevel, HostPowerMode, HostThermalState};

/// Illumos application lifecycle transitions from native ingress hooks.
pub type IllumosApplicationLifecycle = UnixApplicationLifecycle;

/// Route one Illumos application lifecycle ingress notification.
pub fn illumos_notify_application_lifecycle(
    runtime_id: u64,
    lifecycle: IllumosApplicationLifecycle,
) -> RuntimeResult<()> {
    unix_notify_application_lifecycle(runtime_id, Platform::Illumos, lifecycle)
}

/// Route one Illumos permission-result ingress notification.
pub fn illumos_notify_permission_result(
    runtime_id: u64,
    permission: &str,
    granted: bool,
) -> RuntimeResult<()> {
    unix_notify_permission_result(runtime_id, Platform::Illumos, permission, granted)
}

/// Route one Illumos interruption ingress notification.
pub fn illumos_notify_interruption_changed(
    runtime_id: u64,
    interrupted: bool,
) -> RuntimeResult<()> {
    unix_notify_interruption_changed(runtime_id, Platform::Illumos, interrupted)
}

/// Route one Illumos memory pressure ingress notification.
pub fn illumos_notify_memory_pressure_changed(
    runtime_id: u64,
    level: HostMemoryPressureLevel,
) -> RuntimeResult<()> {
    unix_notify_memory_pressure_changed(runtime_id, Platform::Illumos, level)
}

/// Route one Illumos thermal state ingress notification.
pub fn illumos_notify_thermal_state_changed(
    runtime_id: u64,
    state: HostThermalState,
) -> RuntimeResult<()> {
    unix_notify_thermal_state_changed(runtime_id, Platform::Illumos, state)
}

/// Route one Illumos power mode ingress notification.
pub fn illumos_notify_power_mode_changed(
    runtime_id: u64,
    mode: HostPowerMode,
) -> RuntimeResult<()> {
    unix_notify_power_mode_changed(runtime_id, Platform::Illumos, mode)
}

/// Route one Illumos wall clock ingress notification.
pub fn illumos_notify_wall_clock_changed(runtime_id: u64) -> RuntimeResult<()> {
    unix_notify_wall_clock_changed(runtime_id, Platform::Illumos)
}

/// Wake one blocked host poll operation for Illumos.
pub fn illumos_notify_wake(runtime_id: u64) -> RuntimeResult<()> {
    unix_notify_wake(runtime_id, Platform::Illumos)
}
