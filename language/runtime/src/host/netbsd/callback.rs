use crate::diagnostic::RuntimeResult;
use crate::host::unix::{
    UnixApplicationLifecycle, unix_notify_application_lifecycle, unix_notify_interruption_changed,
    unix_notify_memory_pressure_changed, unix_notify_permission_result,
    unix_notify_power_mode_changed, unix_notify_thermal_state_changed, unix_notify_wake,
    unix_notify_wall_clock_changed, unix_notify_window_available, unix_notify_window_focus_changed,
    unix_notify_window_resized, unix_notify_window_terminated,
};
use crate::host::{HostMemoryPressureLevel, HostPlatform, HostPowerMode, HostThermalState};

/// netbsd application lifecycle transitions from native callbacks.
pub type NetBsdApplicationLifecycle = UnixApplicationLifecycle;

/// Submit one netbsd application lifecycle callback.
pub fn netbsd_notify_application_lifecycle(
    runtime_id: u64,
    lifecycle: NetBsdApplicationLifecycle,
) -> RuntimeResult<()> {
    unix_notify_application_lifecycle(runtime_id, HostPlatform::NetBsd, lifecycle)
}

/// Submit one netbsd window-available callback.
pub fn netbsd_notify_window_available(runtime_id: u64, window_id: u64) -> RuntimeResult<()> {
    unix_notify_window_available(runtime_id, HostPlatform::NetBsd, window_id)
}

/// Submit one netbsd window-terminated callback.
pub fn netbsd_notify_window_terminated(runtime_id: u64, window_id: u64) -> RuntimeResult<()> {
    unix_notify_window_terminated(runtime_id, HostPlatform::NetBsd, window_id)
}

/// Submit one netbsd window-resized callback.
pub fn netbsd_notify_window_resized(
    runtime_id: u64,
    window_id: u64,
    width_px: u32,
    height_px: u32,
) -> RuntimeResult<()> {
    unix_notify_window_resized(
        runtime_id,
        HostPlatform::NetBsd,
        window_id,
        width_px,
        height_px,
    )
}

/// Submit one netbsd permission-result callback.
pub fn netbsd_notify_permission_result(
    runtime_id: u64,
    permission: &str,
    granted: bool,
) -> RuntimeResult<()> {
    unix_notify_permission_result(runtime_id, HostPlatform::NetBsd, permission, granted)
}

/// Submit one netbsd interruption callback.
pub fn netbsd_notify_interruption_changed(runtime_id: u64, interrupted: bool) -> RuntimeResult<()> {
    unix_notify_interruption_changed(runtime_id, HostPlatform::NetBsd, interrupted)
}

/// Submit one netbsd window focus callback.
pub fn netbsd_notify_window_focus_changed(
    runtime_id: u64,
    window_id: u64,
    is_focused: bool,
) -> RuntimeResult<()> {
    unix_notify_window_focus_changed(runtime_id, HostPlatform::NetBsd, window_id, is_focused)
}

/// Submit one netbsd memory pressure callback.
pub fn netbsd_notify_memory_pressure_changed(
    runtime_id: u64,
    level: HostMemoryPressureLevel,
) -> RuntimeResult<()> {
    unix_notify_memory_pressure_changed(runtime_id, HostPlatform::NetBsd, level)
}

/// Submit one netbsd thermal state callback.
pub fn netbsd_notify_thermal_state_changed(
    runtime_id: u64,
    state: HostThermalState,
) -> RuntimeResult<()> {
    unix_notify_thermal_state_changed(runtime_id, HostPlatform::NetBsd, state)
}

/// Submit one netbsd power mode callback.
pub fn netbsd_notify_power_mode_changed(runtime_id: u64, mode: HostPowerMode) -> RuntimeResult<()> {
    unix_notify_power_mode_changed(runtime_id, HostPlatform::NetBsd, mode)
}

/// Submit one netbsd wall clock callback.
pub fn netbsd_notify_wall_clock_changed(runtime_id: u64) -> RuntimeResult<()> {
    unix_notify_wall_clock_changed(runtime_id, HostPlatform::NetBsd)
}

/// Wake one blocked host poll operation for netbsd.
pub fn netbsd_notify_wake(runtime_id: u64) -> RuntimeResult<()> {
    unix_notify_wake(runtime_id, HostPlatform::NetBsd)
}
