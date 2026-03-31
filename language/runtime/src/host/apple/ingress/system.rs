use crate::diagnostic::RuntimeResult;
use crate::host::apple::ingress::core::ios_host_queue;
use crate::host::core::HostSessionHandle;
use crate::host::{
    HostEvent, HostInterruptionEvent, HostMemoryPressureEvent, HostMemoryPressureLevel,
    HostPowerMode, HostPowerModeEvent, HostThermalEvent, HostThermalState, HostWallClockEvent,
};

/// Submit one iOS interruption callback.
pub(crate) fn ios_notify_interruption_changed(
    session_handle: HostSessionHandle,
    interrupted: bool,
) -> RuntimeResult<()> {
    let queue = ios_host_queue(session_handle)?;

    queue.enqueue(HostEvent::Interruption(HostInterruptionEvent {
        interrupted,
    }));

    Ok(())
}

/// Submit one iOS memory pressure callback.
pub(crate) fn ios_notify_memory_pressure_changed(
    session_handle: HostSessionHandle,
    level: HostMemoryPressureLevel,
) -> RuntimeResult<()> {
    let queue = ios_host_queue(session_handle)?;

    queue.enqueue(HostEvent::MemoryPressure(HostMemoryPressureEvent { level }));

    Ok(())
}

/// Submit one iOS thermal state callback.
pub(crate) fn ios_notify_thermal_state_changed(
    session_handle: HostSessionHandle,
    state: HostThermalState,
) -> RuntimeResult<()> {
    let queue = ios_host_queue(session_handle)?;

    queue.enqueue(HostEvent::ThermalState(HostThermalEvent { state }));

    Ok(())
}

/// Submit one iOS power mode callback.
pub(crate) fn ios_notify_power_mode_changed(
    session_handle: HostSessionHandle,
    mode: HostPowerMode,
) -> RuntimeResult<()> {
    let queue = ios_host_queue(session_handle)?;

    queue.enqueue(HostEvent::PowerMode(HostPowerModeEvent { mode }));

    Ok(())
}

/// Submit one iOS wall clock callback.
pub(crate) fn ios_notify_wall_clock_changed(
    session_handle: HostSessionHandle,
) -> RuntimeResult<()> {
    let queue = ios_host_queue(session_handle)?;

    queue.enqueue(HostEvent::WallClock(HostWallClockEvent));

    Ok(())
}

/// Wake one blocked host poll operation for iOS.
pub(crate) fn ios_notify_wake(session_handle: HostSessionHandle) -> RuntimeResult<()> {
    let queue = ios_host_queue(session_handle)?;

    queue.poll_wake_handle().wake()?;

    Ok(())
}
