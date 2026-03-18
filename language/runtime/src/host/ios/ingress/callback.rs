use destack_workspace::Platform;

use crate::diagnostic::RuntimeResult;
use crate::host::core::{HostIngressHandle, HostRuntimeId, HostRuntimeRegistry};
use crate::host::{
    HostBackgroundEvent, HostEvent, HostIntentEvent, HostIntentPayload, HostInterruptionEvent,
    HostLifecycleEvent, HostLifecycleState, HostLocationEvent, HostMediaEvent, HostMediaEventKind,
    HostMemoryPressureEvent, HostMemoryPressureLevel, HostNotificationEvent, HostPermissionEvent,
    HostPowerMode, HostPowerModeEvent, HostThermalEvent, HostThermalState, HostWallClockEvent,
};
use crate::platform::os::{
    BackgroundEventValue, LocationSampleValue, MediaEventValue, NotificationEventValue,
};
/// iOS application lifecycle transitions from native callbacks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IosApplicationLifecycle {
    /// App launch finished and normal processing can begin.
    DidFinishLaunching,
    /// App became active in the foreground.
    DidBecomeActive,
    /// App is resigning active foreground state.
    WillResignActive,
    /// App entered background execution state.
    DidEnterBackground,
    /// App is returning to the foreground.
    WillEnterForeground,
    /// App is terminating.
    WillTerminate,
}

/// Return the active iOS host queue for this process.
fn ios_host_bridge(runtime_id: u64) -> RuntimeResult<HostIngressHandle> {
    HostRuntimeRegistry::ingress_handle_for_runtime(HostRuntimeId(runtime_id), Platform::IOS)
}

/// Submit one iOS application lifecycle callback.
pub fn ios_notify_application_lifecycle(
    runtime_id: u64,
    lifecycle: IosApplicationLifecycle,
) -> RuntimeResult<()> {
    let bridge = ios_host_bridge(runtime_id)?;
    let state = host_lifecycle_state_for_application_lifecycle(lifecycle);
    bridge.publish_event(HostEvent::Lifecycle(HostLifecycleEvent { state }));

    Ok(())
}

/// Submit one iOS permission-result callback.
pub fn ios_notify_permission_result(
    runtime_id: u64,
    permission: &str,
    granted: bool,
) -> RuntimeResult<()> {
    let bridge = ios_host_bridge(runtime_id)?;
    bridge.publish_event(HostEvent::Permission(HostPermissionEvent {
        permission: permission.to_string(),
        granted,
    }));

    Ok(())
}

/// Submit one iOS open-url intent callback.
pub fn ios_notify_intent_open_url(
    runtime_id: u64,
    source: Option<&str>,
    url: &str,
) -> RuntimeResult<()> {
    let bridge = ios_host_bridge(runtime_id)?;
    bridge.publish_event(HostEvent::Intent(HostIntentEvent {
        source: source.map(str::to_string),
        payload: HostIntentPayload::OpenUrl {
            url: url.to_string(),
        },
    }));

    Ok(())
}

/// Submit one iOS open-file intent callback.
pub fn ios_notify_intent_open_file(
    runtime_id: u64,
    source: Option<&str>,
    path: &str,
    mime_type: Option<&str>,
) -> RuntimeResult<()> {
    let bridge = ios_host_bridge(runtime_id)?;
    bridge.publish_event(HostEvent::Intent(HostIntentEvent {
        source: source.map(str::to_string),
        payload: HostIntentPayload::OpenFile {
            path: path.to_string(),
            content_type: mime_type.map(str::to_string),
        },
    }));

    Ok(())
}

/// Submit one iOS shared-text intent callback.
pub fn ios_notify_intent_share_text(
    runtime_id: u64,
    source: Option<&str>,
    text: &str,
    mime_type: Option<&str>,
) -> RuntimeResult<()> {
    let bridge = ios_host_bridge(runtime_id)?;
    bridge.publish_event(HostEvent::Intent(HostIntentEvent {
        source: source.map(str::to_string),
        payload: HostIntentPayload::ShareText {
            text: text.to_string(),
            content_type: mime_type.map(str::to_string),
        },
    }));

    Ok(())
}

/// Submit one iOS shared-file intent callback.
pub fn ios_notify_intent_share_files(
    runtime_id: u64,
    source: Option<&str>,
    paths: &[String],
    mime_type: Option<&str>,
) -> RuntimeResult<()> {
    let bridge = ios_host_bridge(runtime_id)?;
    bridge.publish_event(HostEvent::Intent(HostIntentEvent {
        source: source.map(str::to_string),
        payload: HostIntentPayload::ShareFiles {
            paths: paths.to_vec(),
            content_type: mime_type.map(str::to_string),
        },
    }));

    Ok(())
}

/// Submit one iOS custom-action intent callback.
pub fn ios_notify_intent_custom_action(
    runtime_id: u64,
    source: Option<&str>,
    action: &str,
    url: Option<&str>,
    paths: &[String],
    text: Option<&str>,
    mime_type: Option<&str>,
) -> RuntimeResult<()> {
    let bridge = ios_host_bridge(runtime_id)?;
    bridge.publish_event(HostEvent::Intent(HostIntentEvent {
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

/// Submit one iOS interruption callback.
pub fn ios_notify_interruption_changed(runtime_id: u64, interrupted: bool) -> RuntimeResult<()> {
    let bridge = ios_host_bridge(runtime_id)?;
    bridge.publish_event(HostEvent::Interruption(HostInterruptionEvent {
        interrupted,
    }));

    Ok(())
}

/// Submit one iOS notification callback.
pub fn ios_notify_notification_event(
    runtime_id: u64,
    event: NotificationEventValue,
) -> RuntimeResult<()> {
    let bridge = ios_host_bridge(runtime_id)?;
    bridge.publish_event(HostEvent::Notification(Box::new(HostNotificationEvent {
        event,
    })));

    Ok(())
}

/// Submit one iOS background callback.
pub fn ios_notify_background_event(
    runtime_id: u64,
    event: BackgroundEventValue,
) -> RuntimeResult<()> {
    let bridge = ios_host_bridge(runtime_id)?;
    bridge.publish_event(HostEvent::Background(Box::new(HostBackgroundEvent {
        event,
    })));

    Ok(())
}

/// Submit one iOS location callback.
pub fn ios_notify_location_sample(
    runtime_id: u64,
    watch_id: &str,
    sample: LocationSampleValue,
) -> RuntimeResult<()> {
    let bridge = ios_host_bridge(runtime_id)?;
    bridge.publish_event(HostEvent::Location(Box::new(HostLocationEvent {
        watch_id: watch_id.to_string(),
        sample,
    })));

    Ok(())
}

/// Submit one iOS media callback.
pub fn ios_notify_media_event(
    runtime_id: u64,
    watch_id: &str,
    event: MediaEventValue,
) -> RuntimeResult<()> {
    let bridge = ios_host_bridge(runtime_id)?;
    let event = host_media_event(watch_id, event);
    bridge.publish_event(HostEvent::Media(Box::new(event)));

    Ok(())
}

/// Submit one iOS memory pressure callback.
pub fn ios_notify_memory_pressure_changed(
    runtime_id: u64,
    level: HostMemoryPressureLevel,
) -> RuntimeResult<()> {
    let bridge = ios_host_bridge(runtime_id)?;
    bridge.publish_event(HostEvent::MemoryPressure(HostMemoryPressureEvent { level }));

    Ok(())
}

/// Submit one iOS thermal state callback.
pub fn ios_notify_thermal_state_changed(
    runtime_id: u64,
    state: HostThermalState,
) -> RuntimeResult<()> {
    let bridge = ios_host_bridge(runtime_id)?;
    bridge.publish_event(HostEvent::ThermalState(HostThermalEvent { state }));

    Ok(())
}

/// Submit one iOS power mode callback.
pub fn ios_notify_power_mode_changed(runtime_id: u64, mode: HostPowerMode) -> RuntimeResult<()> {
    let bridge = ios_host_bridge(runtime_id)?;
    bridge.publish_event(HostEvent::PowerMode(HostPowerModeEvent { mode }));

    Ok(())
}

/// Submit one iOS wall clock callback.
pub fn ios_notify_wall_clock_changed(runtime_id: u64) -> RuntimeResult<()> {
    let bridge = ios_host_bridge(runtime_id)?;
    bridge.publish_event(HostEvent::WallClock(HostWallClockEvent));

    Ok(())
}

/// Wake one blocked host poll operation for iOS.
pub fn ios_notify_wake(runtime_id: u64) -> RuntimeResult<()> {
    let bridge = ios_host_bridge(runtime_id)?;
    bridge.wake()?;

    Ok(())
}

/// Convert one media event payload into one host media event.
fn host_media_event(watch_id: &str, event: MediaEventValue) -> HostMediaEvent {
    match event {
        MediaEventValue::MediaAddedEvent(value) => HostMediaEvent {
            watch_id: watch_id.to_string(),
            kind: HostMediaEventKind::Added,
            asset: value.asset,
        },
        MediaEventValue::MediaUpdatedEvent(value) => HostMediaEvent {
            watch_id: watch_id.to_string(),
            kind: HostMediaEventKind::Updated,
            asset: value.asset,
        },
        MediaEventValue::MediaRemovedEvent(value) => HostMediaEvent {
            watch_id: watch_id.to_string(),
            kind: HostMediaEventKind::Removed,
            asset: value.asset,
        },
    }
}

/// Map one iOS application lifecycle transition to host lifecycle state.
pub(crate) fn host_lifecycle_state_for_application_lifecycle(
    lifecycle: IosApplicationLifecycle,
) -> HostLifecycleState {
    match lifecycle {
        IosApplicationLifecycle::DidFinishLaunching => HostLifecycleState::Initializing,
        IosApplicationLifecycle::DidBecomeActive => HostLifecycleState::Running,
        IosApplicationLifecycle::WillResignActive => HostLifecycleState::Paused,
        IosApplicationLifecycle::DidEnterBackground => HostLifecycleState::Paused,
        IosApplicationLifecycle::WillEnterForeground => HostLifecycleState::Running,
        IosApplicationLifecycle::WillTerminate => HostLifecycleState::Destroyed,
    }
}
