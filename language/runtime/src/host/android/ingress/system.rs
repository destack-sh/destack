use crate::diagnostic::RuntimeResult;
use crate::host::android::ingress::core::android_host_queue;
use crate::host::core::HostSessionHandle;
use crate::host::{
    HostEvent, HostInterruptionEvent, HostMemoryPressureEvent, HostMemoryPressureLevel,
    HostPowerMode, HostPowerModeEvent, HostThermalEvent, HostThermalState, HostWallClockEvent,
};

/// Submit one Android interruption callback.
pub(crate) fn android_notify_interruption_changed(
    session_handle: HostSessionHandle,
    interrupted: bool,
) -> RuntimeResult<()> {
    let queue = android_host_queue(session_handle)?;

    queue.enqueue(HostEvent::Interruption(HostInterruptionEvent {
        interrupted,
    }));

    Ok(())
}

/// Submit one Android memory pressure callback.
pub(crate) fn android_notify_memory_pressure_changed(
    session_handle: HostSessionHandle,
    level: HostMemoryPressureLevel,
) -> RuntimeResult<()> {
    let queue = android_host_queue(session_handle)?;

    queue.enqueue(HostEvent::MemoryPressure(HostMemoryPressureEvent { level }));

    Ok(())
}

/// Submit one Android thermal state callback.
pub(crate) fn android_notify_thermal_state_changed(
    session_handle: HostSessionHandle,
    state: HostThermalState,
) -> RuntimeResult<()> {
    let queue = android_host_queue(session_handle)?;

    queue.enqueue(HostEvent::ThermalState(HostThermalEvent { state }));

    Ok(())
}

/// Submit one Android power mode callback.
pub(crate) fn android_notify_power_mode_changed(
    session_handle: HostSessionHandle,
    mode: HostPowerMode,
) -> RuntimeResult<()> {
    let queue = android_host_queue(session_handle)?;

    queue.enqueue(HostEvent::PowerMode(HostPowerModeEvent { mode }));

    Ok(())
}

/// Submit one Android wall clock callback.
pub(crate) fn android_notify_wall_clock_changed(
    session_handle: HostSessionHandle,
) -> RuntimeResult<()> {
    let queue = android_host_queue(session_handle)?;

    queue.enqueue(HostEvent::WallClock(HostWallClockEvent));

    Ok(())
}

/// Wake one blocked host poll operation for Android.
pub(crate) fn android_notify_wake(session_handle: HostSessionHandle) -> RuntimeResult<()> {
    let queue = android_host_queue(session_handle)?;

    queue.poll_wake_handle().wake()?;

    Ok(())
}
