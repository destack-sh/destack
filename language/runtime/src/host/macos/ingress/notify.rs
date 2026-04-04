use std::sync::Arc;

use destack_artifact::Platform;

use crate::diagnostic::RuntimeResult;
use crate::host::{
    HostEvent, HostIntentEvent, HostIntentPayload, HostInterruptionEvent, HostLifecycleEvent,
    HostLifecycleSourceKind, HostLifecycleState, HostLocationEvent, HostMemoryPressureEvent,
    HostMemoryPressureLevel, HostPermissionEvent, HostPowerMode, HostPowerModeEvent, HostQueue,
    HostSessionId, HostSessionRegistry, HostThermalEvent, HostThermalState, HostWallClockEvent,
};
use crate::platform::os::abi_generated::LocationSampleValue;
use crate::platform::os::{invalid_data, parse_host_permission_name};
/// macOS application lifecycle transitions from native ingress hooks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum MacosApplicationLifecycle {
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
fn macos_host_bridge(runtime_id: u64) -> RuntimeResult<Arc<HostQueue>> {
    HostSessionRegistry::queue_for_session(HostSessionId(runtime_id), Platform::MacOS)
}

/// Route one macOS application lifecycle ingress notification.
pub(crate) fn macos_notify_application_lifecycle(
    runtime_id: u64,
    lifecycle: MacosApplicationLifecycle,
) -> RuntimeResult<()> {
    let bridge = macos_host_bridge(runtime_id)?;
    let state = host_lifecycle_state_for_application_lifecycle(lifecycle);
    bridge.enqueue(HostEvent::Lifecycle(HostLifecycleEvent {
        source_kind: HostLifecycleSourceKind::Application,
        state,
    }));

    Ok(())
}

/// Route one macOS permission-result ingress notification.
pub(crate) fn macos_notify_permission_result(
    runtime_id: u64,
    permission: &str,
    granted: bool,
) -> RuntimeResult<()> {
    let bridge = macos_host_bridge(runtime_id)?;
    let permission = parse_host_permission_name(permission).ok_or_else(|| {
        invalid_data(
            "destack.host.macos.notify_permission_result",
            format!("unknown host permission {permission}"),
        )
    })?;

    bridge.enqueue(HostEvent::Permission(HostPermissionEvent {
        request_id: None,
        permission,
        granted,
    }));

    Ok(())
}

/// Route one macOS location sample ingress notification.
pub(crate) fn macos_notify_location_sample(
    runtime_id: u64,
    watch_id: &str,
    sample: LocationSampleValue,
) -> RuntimeResult<()> {
    let bridge = macos_host_bridge(runtime_id)?;
    bridge.enqueue(HostEvent::Location(Box::new(HostLocationEvent {
        watch_id: watch_id.to_string(),
        sample,
    })));

    Ok(())
}

/// Route one macOS open-url intent ingress notification.
pub(crate) fn macos_notify_intent_open_url(
    runtime_id: u64,
    source: Option<&str>,
    url: &str,
) -> RuntimeResult<()> {
    let bridge = macos_host_bridge(runtime_id)?;
    bridge.enqueue(HostEvent::Intent(HostIntentEvent {
        source: source.map(str::to_string),
        payload: HostIntentPayload::OpenUrl {
            url: url.to_string(),
        },
    }));

    Ok(())
}

/// Route one macOS open-file intent ingress notification.
pub(crate) fn macos_notify_intent_open_file(
    runtime_id: u64,
    source: Option<&str>,
    path: &str,
    mime_type: Option<&str>,
) -> RuntimeResult<()> {
    let bridge = macos_host_bridge(runtime_id)?;
    bridge.enqueue(HostEvent::Intent(HostIntentEvent {
        source: source.map(str::to_string),
        payload: HostIntentPayload::OpenFile {
            path: path.to_string(),
            content_type: mime_type.map(str::to_string),
        },
    }));

    Ok(())
}

/// Route one macOS shared-text intent ingress notification.
pub(crate) fn macos_notify_intent_share_text(
    runtime_id: u64,
    source: Option<&str>,
    text: &str,
    mime_type: Option<&str>,
) -> RuntimeResult<()> {
    let bridge = macos_host_bridge(runtime_id)?;
    bridge.enqueue(HostEvent::Intent(HostIntentEvent {
        source: source.map(str::to_string),
        payload: HostIntentPayload::ShareText {
            text: text.to_string(),
            content_type: mime_type.map(str::to_string),
        },
    }));

    Ok(())
}

/// Route one macOS shared-file intent ingress notification.
pub(crate) fn macos_notify_intent_share_files(
    runtime_id: u64,
    source: Option<&str>,
    paths: &[String],
    mime_type: Option<&str>,
) -> RuntimeResult<()> {
    let bridge = macos_host_bridge(runtime_id)?;
    bridge.enqueue(HostEvent::Intent(HostIntentEvent {
        source: source.map(str::to_string),
        payload: HostIntentPayload::ShareFiles {
            paths: paths.to_vec(),
            content_type: mime_type.map(str::to_string),
        },
    }));

    Ok(())
}

/// Route one macOS custom-action intent ingress notification.
pub(crate) fn macos_notify_intent_custom_action(
    runtime_id: u64,
    source: Option<&str>,
    action: &str,
    url: Option<&str>,
    paths: &[String],
    text: Option<&str>,
    mime_type: Option<&str>,
) -> RuntimeResult<()> {
    let bridge = macos_host_bridge(runtime_id)?;
    bridge.enqueue(HostEvent::Intent(HostIntentEvent {
        source: source.map(str::to_string),
        payload: HostIntentPayload::CustomAction {
            action: action.to_string(),
            url: url.map(str::to_string),
            paths: paths.to_vec(),
            text: text.map(str::to_string),
            content_type: mime_type.map(str::to_string),
        },
    }));

    Ok(())
}

/// Route one macOS interruption ingress notification.
pub(crate) fn macos_notify_interruption_changed(
    runtime_id: u64,
    interrupted: bool,
) -> RuntimeResult<()> {
    let bridge = macos_host_bridge(runtime_id)?;
    bridge.enqueue(HostEvent::Interruption(HostInterruptionEvent {
        interrupted,
    }));

    Ok(())
}

/// Route one macOS memory pressure ingress notification.
pub(crate) fn macos_notify_memory_pressure_changed(
    runtime_id: u64,
    level: HostMemoryPressureLevel,
) -> RuntimeResult<()> {
    let bridge = macos_host_bridge(runtime_id)?;
    bridge.enqueue(HostEvent::MemoryPressure(HostMemoryPressureEvent { level }));

    Ok(())
}

/// Route one macOS thermal state ingress notification.
pub(crate) fn macos_notify_thermal_state_changed(
    runtime_id: u64,
    state: HostThermalState,
) -> RuntimeResult<()> {
    let bridge = macos_host_bridge(runtime_id)?;
    bridge.enqueue(HostEvent::ThermalState(HostThermalEvent { state }));

    Ok(())
}

/// Route one macOS power mode ingress notification.
pub(crate) fn macos_notify_power_mode_changed(
    runtime_id: u64,
    mode: HostPowerMode,
) -> RuntimeResult<()> {
    let bridge = macos_host_bridge(runtime_id)?;
    bridge.enqueue(HostEvent::PowerMode(HostPowerModeEvent { mode }));

    Ok(())
}

/// Route one macOS wall clock ingress notification.
pub(crate) fn macos_notify_wall_clock_changed(runtime_id: u64) -> RuntimeResult<()> {
    let bridge = macos_host_bridge(runtime_id)?;
    bridge.enqueue(HostEvent::WallClock(HostWallClockEvent));

    Ok(())
}

/// Wake one blocked host poll operation for macOS.
pub(crate) fn macos_notify_wake(runtime_id: u64) -> RuntimeResult<()> {
    let bridge = macos_host_bridge(runtime_id)?;
    bridge.poll_wake_handle().wake()?;

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
