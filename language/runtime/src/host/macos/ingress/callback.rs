use destack_workspace::Platform;

use crate::diagnostic::RuntimeResult;
use crate::host::core::{HostQueueRegistry, HostSessionIngress};
use crate::host::{
    HostEvent, HostIntentEvent, HostIntentPayload, HostInterruptionEvent, HostLifecycleEvent,
    HostLifecycleState, HostMemoryPressureEvent, HostMemoryPressureLevel, HostPermissionEvent,
    HostPowerMode, HostPowerModeEvent, HostThermalEvent, HostThermalState, HostWallClockEvent,
};
use crate::runtime::world::RuntimeId;

/// macOS application lifecycle transitions from native callbacks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MacosApplicationLifecycle {
    /// App launch finished and normal processing can begin.
    DidFinishLaunching,
    /// App became active in the foreground.
    DidBecomeActive,
    /// App is resigning active foreground state.
    WillResignActive,
    /// App is terminating.
    WillTerminate,
}

/// Return the active macOS host queue for this process.
fn macos_host_bridge(runtime_id: u64) -> RuntimeResult<HostSessionIngress> {
    HostQueueRegistry::shared()
        .write()
        .session_ingress_for_runtime(RuntimeId(runtime_id), Platform::MacOS)
}

/// Submit one macOS application lifecycle callback.
pub fn macos_notify_application_lifecycle(
    runtime_id: u64,
    lifecycle: MacosApplicationLifecycle,
) -> RuntimeResult<()> {
    let bridge = macos_host_bridge(runtime_id)?;
    let state = host_lifecycle_state_for_application_lifecycle(lifecycle);
    bridge.publish_event(HostEvent::Lifecycle(HostLifecycleEvent { state }));

    Ok(())
}

/// Submit one macOS permission-result callback.
pub fn macos_notify_permission_result(
    runtime_id: u64,
    permission: &str,
    granted: bool,
) -> RuntimeResult<()> {
    let bridge = macos_host_bridge(runtime_id)?;
    bridge.publish_event(HostEvent::Permission(HostPermissionEvent {
        permission: permission.to_string(),
        granted,
    }));

    Ok(())
}

/// Submit one macOS open-url intent callback.
pub fn macos_notify_intent_open_url(
    runtime_id: u64,
    source: Option<&str>,
    url: &str,
) -> RuntimeResult<()> {
    let bridge = macos_host_bridge(runtime_id)?;
    bridge.publish_event(HostEvent::Intent(HostIntentEvent {
        source: source.map(str::to_string),
        payload: HostIntentPayload::OpenUrl {
            url: url.to_string(),
        },
    }));

    Ok(())
}

/// Submit one macOS open-file intent callback.
pub fn macos_notify_intent_open_file(
    runtime_id: u64,
    source: Option<&str>,
    path: &str,
    mime_type: Option<&str>,
) -> RuntimeResult<()> {
    let bridge = macos_host_bridge(runtime_id)?;
    bridge.publish_event(HostEvent::Intent(HostIntentEvent {
        source: source.map(str::to_string),
        payload: HostIntentPayload::OpenFile {
            path: path.to_string(),
            mime_type: mime_type.map(str::to_string),
        },
    }));

    Ok(())
}

/// Submit one macOS shared-text intent callback.
pub fn macos_notify_intent_share_text(
    runtime_id: u64,
    source: Option<&str>,
    text: &str,
    mime_type: Option<&str>,
) -> RuntimeResult<()> {
    let bridge = macos_host_bridge(runtime_id)?;
    bridge.publish_event(HostEvent::Intent(HostIntentEvent {
        source: source.map(str::to_string),
        payload: HostIntentPayload::ShareText {
            text: text.to_string(),
            mime_type: mime_type.map(str::to_string),
        },
    }));

    Ok(())
}

/// Submit one macOS shared-file intent callback.
pub fn macos_notify_intent_share_files(
    runtime_id: u64,
    source: Option<&str>,
    paths: &[String],
    mime_type: Option<&str>,
) -> RuntimeResult<()> {
    let bridge = macos_host_bridge(runtime_id)?;
    bridge.publish_event(HostEvent::Intent(HostIntentEvent {
        source: source.map(str::to_string),
        payload: HostIntentPayload::ShareFiles {
            paths: paths.to_vec(),
            mime_type: mime_type.map(str::to_string),
        },
    }));

    Ok(())
}

/// Submit one macOS custom-action intent callback.
pub fn macos_notify_intent_custom_action(
    runtime_id: u64,
    source: Option<&str>,
    action: &str,
    url: Option<&str>,
    paths: &[String],
    text: Option<&str>,
    mime_type: Option<&str>,
) -> RuntimeResult<()> {
    let bridge = macos_host_bridge(runtime_id)?;
    bridge.publish_event(HostEvent::Intent(HostIntentEvent {
        source: source.map(str::to_string),
        payload: HostIntentPayload::CustomAction {
            action: action.to_string(),
            url: url.map(str::to_string),
            paths: paths.to_vec(),
            text: text.map(str::to_string),
            mime_type: mime_type.map(str::to_string),
        },
    }));

    Ok(())
}

/// Submit one macOS interruption callback.
pub fn macos_notify_interruption_changed(runtime_id: u64, interrupted: bool) -> RuntimeResult<()> {
    let bridge = macos_host_bridge(runtime_id)?;
    bridge.publish_event(HostEvent::Interruption(HostInterruptionEvent {
        interrupted,
    }));

    Ok(())
}

/// Submit one macOS memory pressure callback.
pub fn macos_notify_memory_pressure_changed(
    runtime_id: u64,
    level: HostMemoryPressureLevel,
) -> RuntimeResult<()> {
    let bridge = macos_host_bridge(runtime_id)?;
    bridge.publish_event(HostEvent::MemoryPressure(HostMemoryPressureEvent { level }));

    Ok(())
}

/// Submit one macOS thermal state callback.
pub fn macos_notify_thermal_state_changed(
    runtime_id: u64,
    state: HostThermalState,
) -> RuntimeResult<()> {
    let bridge = macos_host_bridge(runtime_id)?;
    bridge.publish_event(HostEvent::ThermalState(HostThermalEvent { state }));

    Ok(())
}

/// Submit one macOS power mode callback.
pub fn macos_notify_power_mode_changed(runtime_id: u64, mode: HostPowerMode) -> RuntimeResult<()> {
    let bridge = macos_host_bridge(runtime_id)?;
    bridge.publish_event(HostEvent::PowerMode(HostPowerModeEvent { mode }));

    Ok(())
}

/// Submit one macOS wall clock callback.
pub fn macos_notify_wall_clock_changed(runtime_id: u64) -> RuntimeResult<()> {
    let bridge = macos_host_bridge(runtime_id)?;
    bridge.publish_event(HostEvent::WallClock(HostWallClockEvent));

    Ok(())
}

/// Wake one blocked host poll operation for macOS.
pub fn macos_notify_wake(runtime_id: u64) -> RuntimeResult<()> {
    let bridge = macos_host_bridge(runtime_id)?;
    bridge.wake()?;

    Ok(())
}

/// Map one macOS application lifecycle transition to host lifecycle state.
pub(crate) fn host_lifecycle_state_for_application_lifecycle(
    lifecycle: MacosApplicationLifecycle,
) -> HostLifecycleState {
    match lifecycle {
        MacosApplicationLifecycle::DidFinishLaunching => HostLifecycleState::Initializing,
        MacosApplicationLifecycle::DidBecomeActive => HostLifecycleState::Running,
        MacosApplicationLifecycle::WillResignActive => HostLifecycleState::Paused,
        MacosApplicationLifecycle::WillTerminate => HostLifecycleState::Destroyed,
    }
}
