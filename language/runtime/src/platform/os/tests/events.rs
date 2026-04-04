#![allow(dead_code)]

use crate::diagnostic::RuntimeResult;
use crate::host::{
    HostBackgroundEvent, HostEvent, HostIntentEvent, HostIntentPayload, HostLifecycleEvent,
    HostLifecycleSourceKind, HostLifecycleState, HostMemoryPressureEvent, HostMemoryPressureLevel,
    HostNotificationEvent, HostPermissionEvent, HostPowerMode, HostPowerModeEvent,
    HostSessionRegistry,
};
use crate::platform::os::abi_generated::{BackgroundEventValue, NotificationEventValue};
use crate::platform::os::{invalid_data, parse_host_permission_name};

use super::harness::HarnessContext;

impl HarnessContext<'_> {
    /// Enqueue one host event through the runtime-owned host queue when available.
    pub(crate) fn dispatch_host_event(&self, event: HostEvent) -> RuntimeResult<()> {
        let runtime_id = self.call_context.host().host_session_id();
        let platform = self.call_context.host().platform();
        let queue = HostSessionRegistry::queue_for_session(runtime_id, platform);

        // prefer one queued host delivery so bootstrap and live observers share one path
        if let Ok(queue) = queue {
            queue.enqueue(event);
            return Ok(());
        }

        let queue = HostSessionRegistry::queue_for_session(runtime_id, platform)?;
        queue.dispatch_host_event(&event)
    }

    /// Enqueue one host lifecycle transition for this harness runtime.
    pub(crate) fn enqueue_lifecycle_event(&self, state: HostLifecycleState) -> RuntimeResult<()> {
        self.dispatch_host_event(HostEvent::Lifecycle(HostLifecycleEvent {
            source_kind: HostLifecycleSourceKind::Application,
            state,
        }))
    }

    /// Enqueue one host memory-pressure event for this harness runtime.
    pub(crate) fn enqueue_memory_pressure_event(
        &self,
        level: HostMemoryPressureLevel,
    ) -> RuntimeResult<()> {
        self.dispatch_host_event(HostEvent::MemoryPressure(HostMemoryPressureEvent { level }))
    }

    /// Enqueue one host power-mode event for this harness runtime.
    pub(crate) fn enqueue_power_mode_event(&self, mode: HostPowerMode) -> RuntimeResult<()> {
        self.dispatch_host_event(HostEvent::PowerMode(HostPowerModeEvent { mode }))
    }

    /// Enqueue one host permission result for this harness runtime.
    pub(crate) fn enqueue_permission_event(
        &self,
        permission: &str,
        granted: bool,
    ) -> RuntimeResult<()> {
        let permission = parse_host_permission_name(permission).ok_or_else(|| {
            invalid_data(
                "destack.platform.os.tests.enqueue_permission_event",
                format!("unknown host permission {permission}"),
            )
        })?;

        self.dispatch_host_event(HostEvent::Permission(HostPermissionEvent {
            request_id: None,
            permission,
            granted,
        }))
    }

    /// Enqueue one host notification event for this harness runtime.
    pub(crate) fn enqueue_notification_event(
        &self,
        event: NotificationEventValue,
    ) -> RuntimeResult<()> {
        self.dispatch_host_event(HostEvent::Notification(Box::new(HostNotificationEvent {
            event,
        })))
    }

    /// Enqueue one host background event for this harness runtime.
    pub(crate) fn enqueue_background_event(
        &self,
        event: BackgroundEventValue,
    ) -> RuntimeResult<()> {
        self.dispatch_host_event(HostEvent::Background(Box::new(HostBackgroundEvent {
            event,
        })))
    }

    /// Enqueue one open-url intent event for this harness runtime.
    pub(crate) fn enqueue_intent_open_url_event(
        &self,
        source: Option<&str>,
        url: &str,
    ) -> RuntimeResult<()> {
        self.dispatch_host_event(HostEvent::Intent(HostIntentEvent {
            source: source.map(str::to_string),
            payload: HostIntentPayload::OpenUrl {
                url: url.to_string(),
            },
        }))
    }

    /// Enqueue one open-file intent event for this harness runtime.
    pub(crate) fn enqueue_intent_open_file_event(
        &self,
        source: Option<&str>,
        path: &str,
        content_type: Option<&str>,
    ) -> RuntimeResult<()> {
        self.dispatch_host_event(HostEvent::Intent(HostIntentEvent {
            source: source.map(str::to_string),
            payload: HostIntentPayload::OpenFile {
                path: path.to_string(),
                content_type: content_type.map(str::to_string),
            },
        }))
    }
}
