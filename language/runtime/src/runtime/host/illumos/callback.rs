use crate::diagnostic::RuntimeResult;
use crate::runtime::host::unix::{
    UnixApplicationLifecycle, unix_notify_application_lifecycle, unix_notify_interruption_changed,
    unix_notify_memory_pressure_changed, unix_notify_permission_result,
    unix_notify_power_mode_changed, unix_notify_thermal_state_changed, unix_notify_wake,
    unix_notify_wall_clock_changed, unix_notify_window_available, unix_notify_window_focus_changed,
    unix_notify_window_resized, unix_notify_window_terminated,
};
use crate::runtime::host::{
    HostMemoryPressureLevel, HostPlatform, HostPowerMode, HostThermalState,
};

/// illumos application lifecycle transitions from native callbacks.
pub type IllumosApplicationLifecycle = UnixApplicationLifecycle;

/// Submit one illumos application lifecycle callback.
pub fn illumos_notify_application_lifecycle(
    runtime_id: u64,
    lifecycle: IllumosApplicationLifecycle,
) -> RuntimeResult<()> {
    unix_notify_application_lifecycle(runtime_id, HostPlatform::Illumos, lifecycle)
}

/// Submit one illumos window-available callback.
pub fn illumos_notify_window_available(runtime_id: u64) -> RuntimeResult<()> {
    unix_notify_window_available(runtime_id, HostPlatform::Illumos)
}

/// Submit one illumos window-terminated callback.
pub fn illumos_notify_window_terminated(runtime_id: u64) -> RuntimeResult<()> {
    unix_notify_window_terminated(runtime_id, HostPlatform::Illumos)
}

/// Submit one illumos window-resized callback.
pub fn illumos_notify_window_resized(
    runtime_id: u64,
    width_px: u32,
    height_px: u32,
) -> RuntimeResult<()> {
    unix_notify_window_resized(runtime_id, HostPlatform::Illumos, width_px, height_px)
}

/// Submit one illumos permission-result callback.
pub fn illumos_notify_permission_result(
    runtime_id: u64,
    permission: &str,
    granted: bool,
) -> RuntimeResult<()> {
    unix_notify_permission_result(runtime_id, HostPlatform::Illumos, permission, granted)
}

/// Submit one illumos interruption callback.
pub fn illumos_notify_interruption_changed(
    runtime_id: u64,
    interrupted: bool,
) -> RuntimeResult<()> {
    unix_notify_interruption_changed(runtime_id, HostPlatform::Illumos, interrupted)
}

/// Submit one illumos window focus callback.
pub fn illumos_notify_window_focus_changed(runtime_id: u64, is_focused: bool) -> RuntimeResult<()> {
    unix_notify_window_focus_changed(runtime_id, HostPlatform::Illumos, is_focused)
}

/// Submit one illumos memory pressure callback.
pub fn illumos_notify_memory_pressure_changed(
    runtime_id: u64,
    level: HostMemoryPressureLevel,
) -> RuntimeResult<()> {
    unix_notify_memory_pressure_changed(runtime_id, HostPlatform::Illumos, level)
}

/// Submit one illumos thermal state callback.
pub fn illumos_notify_thermal_state_changed(
    runtime_id: u64,
    state: HostThermalState,
) -> RuntimeResult<()> {
    unix_notify_thermal_state_changed(runtime_id, HostPlatform::Illumos, state)
}

/// Submit one illumos power mode callback.
pub fn illumos_notify_power_mode_changed(
    runtime_id: u64,
    mode: HostPowerMode,
) -> RuntimeResult<()> {
    unix_notify_power_mode_changed(runtime_id, HostPlatform::Illumos, mode)
}

/// Submit one illumos wall clock callback.
pub fn illumos_notify_wall_clock_changed(runtime_id: u64) -> RuntimeResult<()> {
    unix_notify_wall_clock_changed(runtime_id, HostPlatform::Illumos)
}

/// Wake one blocked host poll operation for illumos.
pub fn illumos_notify_wake(runtime_id: u64) -> RuntimeResult<()> {
    unix_notify_wake(runtime_id, HostPlatform::Illumos)
}
