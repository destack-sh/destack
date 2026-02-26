use crate::diagnostic::RuntimeResult;
use crate::host::unix::{
    UnixApplicationLifecycle, unix_notify_application_lifecycle, unix_notify_interruption_changed,
    unix_notify_memory_pressure_changed, unix_notify_permission_result,
    unix_notify_power_mode_changed, unix_notify_thermal_state_changed, unix_notify_wake,
    unix_notify_wall_clock_changed, unix_notify_window_available, unix_notify_window_focus_changed,
    unix_notify_window_resized, unix_notify_window_terminated,
};
use crate::host::{HostMemoryPressureLevel, HostPlatform, HostPowerMode, HostThermalState};

/// solaris application lifecycle transitions from native callbacks.
pub type SolarisApplicationLifecycle = UnixApplicationLifecycle;

/// Submit one solaris application lifecycle callback.
pub fn solaris_notify_application_lifecycle(
    runtime_id: u64,
    lifecycle: SolarisApplicationLifecycle,
) -> RuntimeResult<()> {
    unix_notify_application_lifecycle(runtime_id, HostPlatform::Solaris, lifecycle)
}

/// Submit one solaris window-available callback.
pub fn solaris_notify_window_available(runtime_id: u64, window_id: u64) -> RuntimeResult<()> {
    unix_notify_window_available(runtime_id, HostPlatform::Solaris, window_id)
}

/// Submit one solaris window-terminated callback.
pub fn solaris_notify_window_terminated(runtime_id: u64, window_id: u64) -> RuntimeResult<()> {
    unix_notify_window_terminated(runtime_id, HostPlatform::Solaris, window_id)
}

/// Submit one solaris window-resized callback.
pub fn solaris_notify_window_resized(
    runtime_id: u64,
    window_id: u64,
    width_px: u32,
    height_px: u32,
) -> RuntimeResult<()> {
    unix_notify_window_resized(
        runtime_id,
        HostPlatform::Solaris,
        window_id,
        width_px,
        height_px,
    )
}

/// Submit one solaris permission-result callback.
pub fn solaris_notify_permission_result(
    runtime_id: u64,
    permission: &str,
    granted: bool,
) -> RuntimeResult<()> {
    unix_notify_permission_result(runtime_id, HostPlatform::Solaris, permission, granted)
}

/// Submit one solaris interruption callback.
pub fn solaris_notify_interruption_changed(
    runtime_id: u64,
    interrupted: bool,
) -> RuntimeResult<()> {
    unix_notify_interruption_changed(runtime_id, HostPlatform::Solaris, interrupted)
}

/// Submit one solaris window focus callback.
pub fn solaris_notify_window_focus_changed(
    runtime_id: u64,
    window_id: u64,
    is_focused: bool,
) -> RuntimeResult<()> {
    unix_notify_window_focus_changed(runtime_id, HostPlatform::Solaris, window_id, is_focused)
}

/// Submit one solaris memory pressure callback.
pub fn solaris_notify_memory_pressure_changed(
    runtime_id: u64,
    level: HostMemoryPressureLevel,
) -> RuntimeResult<()> {
    unix_notify_memory_pressure_changed(runtime_id, HostPlatform::Solaris, level)
}

/// Submit one solaris thermal state callback.
pub fn solaris_notify_thermal_state_changed(
    runtime_id: u64,
    state: HostThermalState,
) -> RuntimeResult<()> {
    unix_notify_thermal_state_changed(runtime_id, HostPlatform::Solaris, state)
}

/// Submit one solaris power mode callback.
pub fn solaris_notify_power_mode_changed(
    runtime_id: u64,
    mode: HostPowerMode,
) -> RuntimeResult<()> {
    unix_notify_power_mode_changed(runtime_id, HostPlatform::Solaris, mode)
}

/// Submit one solaris wall clock callback.
pub fn solaris_notify_wall_clock_changed(runtime_id: u64) -> RuntimeResult<()> {
    unix_notify_wall_clock_changed(runtime_id, HostPlatform::Solaris)
}

/// Wake one blocked host poll operation for solaris.
pub fn solaris_notify_wake(runtime_id: u64) -> RuntimeResult<()> {
    unix_notify_wake(runtime_id, HostPlatform::Solaris)
}
