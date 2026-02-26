use crate::diagnostic::RuntimeResult;
use crate::host::unix::{
    UnixApplicationLifecycle, unix_notify_application_lifecycle, unix_notify_interruption_changed,
    unix_notify_memory_pressure_changed, unix_notify_permission_result,
    unix_notify_power_mode_changed, unix_notify_thermal_state_changed, unix_notify_wake,
    unix_notify_wall_clock_changed, unix_notify_window_available, unix_notify_window_focus_changed,
    unix_notify_window_resized, unix_notify_window_terminated,
};
use crate::host::{HostMemoryPressureLevel, HostPlatform, HostPowerMode, HostThermalState};

/// linux application lifecycle transitions from native callbacks.
pub type LinuxApplicationLifecycle = UnixApplicationLifecycle;

/// Submit one linux application lifecycle callback.
pub fn linux_notify_application_lifecycle(
    runtime_id: u64,
    lifecycle: LinuxApplicationLifecycle,
) -> RuntimeResult<()> {
    unix_notify_application_lifecycle(runtime_id, HostPlatform::Linux, lifecycle)
}

/// Submit one linux window-available callback.
pub fn linux_notify_window_available(runtime_id: u64, window_id: u64) -> RuntimeResult<()> {
    unix_notify_window_available(runtime_id, HostPlatform::Linux, window_id)
}

/// Submit one linux window-terminated callback.
pub fn linux_notify_window_terminated(runtime_id: u64, window_id: u64) -> RuntimeResult<()> {
    unix_notify_window_terminated(runtime_id, HostPlatform::Linux, window_id)
}

/// Submit one linux window-resized callback.
pub fn linux_notify_window_resized(
    runtime_id: u64,
    window_id: u64,
    width_px: u32,
    height_px: u32,
) -> RuntimeResult<()> {
    unix_notify_window_resized(
        runtime_id,
        HostPlatform::Linux,
        window_id,
        width_px,
        height_px,
    )
}

/// Submit one linux permission-result callback.
pub fn linux_notify_permission_result(
    runtime_id: u64,
    permission: &str,
    granted: bool,
) -> RuntimeResult<()> {
    unix_notify_permission_result(runtime_id, HostPlatform::Linux, permission, granted)
}

/// Submit one linux interruption callback.
pub fn linux_notify_interruption_changed(runtime_id: u64, interrupted: bool) -> RuntimeResult<()> {
    unix_notify_interruption_changed(runtime_id, HostPlatform::Linux, interrupted)
}

/// Submit one linux window focus callback.
pub fn linux_notify_window_focus_changed(
    runtime_id: u64,
    window_id: u64,
    is_focused: bool,
) -> RuntimeResult<()> {
    unix_notify_window_focus_changed(runtime_id, HostPlatform::Linux, window_id, is_focused)
}

/// Submit one linux memory pressure callback.
pub fn linux_notify_memory_pressure_changed(
    runtime_id: u64,
    level: HostMemoryPressureLevel,
) -> RuntimeResult<()> {
    unix_notify_memory_pressure_changed(runtime_id, HostPlatform::Linux, level)
}

/// Submit one linux thermal state callback.
pub fn linux_notify_thermal_state_changed(
    runtime_id: u64,
    state: HostThermalState,
) -> RuntimeResult<()> {
    unix_notify_thermal_state_changed(runtime_id, HostPlatform::Linux, state)
}

/// Submit one linux power mode callback.
pub fn linux_notify_power_mode_changed(runtime_id: u64, mode: HostPowerMode) -> RuntimeResult<()> {
    unix_notify_power_mode_changed(runtime_id, HostPlatform::Linux, mode)
}

/// Submit one linux wall clock callback.
pub fn linux_notify_wall_clock_changed(runtime_id: u64) -> RuntimeResult<()> {
    unix_notify_wall_clock_changed(runtime_id, HostPlatform::Linux)
}

/// Wake one blocked host poll operation for linux.
pub fn linux_notify_wake(runtime_id: u64) -> RuntimeResult<()> {
    unix_notify_wake(runtime_id, HostPlatform::Linux)
}
