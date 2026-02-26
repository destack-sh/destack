use crate::diagnostic::RuntimeResult;
use crate::host::unix::{
    UnixApplicationLifecycle, unix_notify_application_lifecycle, unix_notify_interruption_changed,
    unix_notify_memory_pressure_changed, unix_notify_permission_result,
    unix_notify_power_mode_changed, unix_notify_thermal_state_changed, unix_notify_wake,
    unix_notify_wall_clock_changed, unix_notify_window_available, unix_notify_window_focus_changed,
    unix_notify_window_resized, unix_notify_window_terminated,
};
use crate::host::{HostMemoryPressureLevel, HostPlatform, HostPowerMode, HostThermalState};

/// dragonfly application lifecycle transitions from native callbacks.
pub type DragonflyApplicationLifecycle = UnixApplicationLifecycle;

/// Submit one dragonfly application lifecycle callback.
pub fn dragonfly_notify_application_lifecycle(
    runtime_id: u64,
    lifecycle: DragonflyApplicationLifecycle,
) -> RuntimeResult<()> {
    unix_notify_application_lifecycle(runtime_id, HostPlatform::DragonFly, lifecycle)
}

/// Submit one dragonfly window-available callback.
pub fn dragonfly_notify_window_available(runtime_id: u64, window_id: u64) -> RuntimeResult<()> {
    unix_notify_window_available(runtime_id, HostPlatform::DragonFly, window_id)
}

/// Submit one dragonfly window-terminated callback.
pub fn dragonfly_notify_window_terminated(runtime_id: u64, window_id: u64) -> RuntimeResult<()> {
    unix_notify_window_terminated(runtime_id, HostPlatform::DragonFly, window_id)
}

/// Submit one dragonfly window-resized callback.
pub fn dragonfly_notify_window_resized(
    runtime_id: u64,
    window_id: u64,
    width_px: u32,
    height_px: u32,
) -> RuntimeResult<()> {
    unix_notify_window_resized(
        runtime_id,
        HostPlatform::DragonFly,
        window_id,
        width_px,
        height_px,
    )
}

/// Submit one dragonfly permission-result callback.
pub fn dragonfly_notify_permission_result(
    runtime_id: u64,
    permission: &str,
    granted: bool,
) -> RuntimeResult<()> {
    unix_notify_permission_result(runtime_id, HostPlatform::DragonFly, permission, granted)
}

/// Submit one dragonfly interruption callback.
pub fn dragonfly_notify_interruption_changed(
    runtime_id: u64,
    interrupted: bool,
) -> RuntimeResult<()> {
    unix_notify_interruption_changed(runtime_id, HostPlatform::DragonFly, interrupted)
}

/// Submit one dragonfly window focus callback.
pub fn dragonfly_notify_window_focus_changed(
    runtime_id: u64,
    window_id: u64,
    is_focused: bool,
) -> RuntimeResult<()> {
    unix_notify_window_focus_changed(runtime_id, HostPlatform::DragonFly, window_id, is_focused)
}

/// Submit one dragonfly memory pressure callback.
pub fn dragonfly_notify_memory_pressure_changed(
    runtime_id: u64,
    level: HostMemoryPressureLevel,
) -> RuntimeResult<()> {
    unix_notify_memory_pressure_changed(runtime_id, HostPlatform::DragonFly, level)
}

/// Submit one dragonfly thermal state callback.
pub fn dragonfly_notify_thermal_state_changed(
    runtime_id: u64,
    state: HostThermalState,
) -> RuntimeResult<()> {
    unix_notify_thermal_state_changed(runtime_id, HostPlatform::DragonFly, state)
}

/// Submit one dragonfly power mode callback.
pub fn dragonfly_notify_power_mode_changed(
    runtime_id: u64,
    mode: HostPowerMode,
) -> RuntimeResult<()> {
    unix_notify_power_mode_changed(runtime_id, HostPlatform::DragonFly, mode)
}

/// Submit one dragonfly wall clock callback.
pub fn dragonfly_notify_wall_clock_changed(runtime_id: u64) -> RuntimeResult<()> {
    unix_notify_wall_clock_changed(runtime_id, HostPlatform::DragonFly)
}

/// Wake one blocked host poll operation for dragonfly.
pub fn dragonfly_notify_wake(runtime_id: u64) -> RuntimeResult<()> {
    unix_notify_wake(runtime_id, HostPlatform::DragonFly)
}
