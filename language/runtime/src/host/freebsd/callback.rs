use crate::diagnostic::RuntimeResult;
use crate::host::unix::{
    UnixApplicationLifecycle, unix_notify_application_lifecycle, unix_notify_interruption_changed,
    unix_notify_memory_pressure_changed, unix_notify_permission_result,
    unix_notify_power_mode_changed, unix_notify_thermal_state_changed, unix_notify_wake,
    unix_notify_wall_clock_changed, unix_notify_window_available, unix_notify_window_focus_changed,
    unix_notify_window_resized, unix_notify_window_terminated,
};
use crate::host::{HostMemoryPressureLevel, HostPlatform, HostPowerMode, HostThermalState};

/// freebsd application lifecycle transitions from native callbacks.
pub type FreeBsdApplicationLifecycle = UnixApplicationLifecycle;

/// Submit one freebsd application lifecycle callback.
pub fn freebsd_notify_application_lifecycle(
    runtime_id: u64,
    lifecycle: FreeBsdApplicationLifecycle,
) -> RuntimeResult<()> {
    unix_notify_application_lifecycle(runtime_id, HostPlatform::FreeBsd, lifecycle)
}

/// Submit one freebsd window-available callback.
pub fn freebsd_notify_window_available(runtime_id: u64) -> RuntimeResult<()> {
    unix_notify_window_available(runtime_id, HostPlatform::FreeBsd)
}

/// Submit one freebsd window-terminated callback.
pub fn freebsd_notify_window_terminated(runtime_id: u64) -> RuntimeResult<()> {
    unix_notify_window_terminated(runtime_id, HostPlatform::FreeBsd)
}

/// Submit one freebsd window-resized callback.
pub fn freebsd_notify_window_resized(
    runtime_id: u64,
    width_px: u32,
    height_px: u32,
) -> RuntimeResult<()> {
    unix_notify_window_resized(runtime_id, HostPlatform::FreeBsd, width_px, height_px)
}

/// Submit one freebsd permission-result callback.
pub fn freebsd_notify_permission_result(
    runtime_id: u64,
    permission: &str,
    granted: bool,
) -> RuntimeResult<()> {
    unix_notify_permission_result(runtime_id, HostPlatform::FreeBsd, permission, granted)
}

/// Submit one freebsd interruption callback.
pub fn freebsd_notify_interruption_changed(
    runtime_id: u64,
    interrupted: bool,
) -> RuntimeResult<()> {
    unix_notify_interruption_changed(runtime_id, HostPlatform::FreeBsd, interrupted)
}

/// Submit one freebsd window focus callback.
pub fn freebsd_notify_window_focus_changed(runtime_id: u64, is_focused: bool) -> RuntimeResult<()> {
    unix_notify_window_focus_changed(runtime_id, HostPlatform::FreeBsd, is_focused)
}

/// Submit one freebsd memory pressure callback.
pub fn freebsd_notify_memory_pressure_changed(
    runtime_id: u64,
    level: HostMemoryPressureLevel,
) -> RuntimeResult<()> {
    unix_notify_memory_pressure_changed(runtime_id, HostPlatform::FreeBsd, level)
}

/// Submit one freebsd thermal state callback.
pub fn freebsd_notify_thermal_state_changed(
    runtime_id: u64,
    state: HostThermalState,
) -> RuntimeResult<()> {
    unix_notify_thermal_state_changed(runtime_id, HostPlatform::FreeBsd, state)
}

/// Submit one freebsd power mode callback.
pub fn freebsd_notify_power_mode_changed(
    runtime_id: u64,
    mode: HostPowerMode,
) -> RuntimeResult<()> {
    unix_notify_power_mode_changed(runtime_id, HostPlatform::FreeBsd, mode)
}

/// Submit one freebsd wall clock callback.
pub fn freebsd_notify_wall_clock_changed(runtime_id: u64) -> RuntimeResult<()> {
    unix_notify_wall_clock_changed(runtime_id, HostPlatform::FreeBsd)
}

/// Wake one blocked host poll operation for freebsd.
pub fn freebsd_notify_wake(runtime_id: u64) -> RuntimeResult<()> {
    unix_notify_wake(runtime_id, HostPlatform::FreeBsd)
}
