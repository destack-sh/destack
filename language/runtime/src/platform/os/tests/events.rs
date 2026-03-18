use crate::diagnostic::RuntimeResult;
use crate::host::core::HostRuntimeRegistry;
use crate::host::{
    HostBackgroundEvent, HostEvent, HostIntentEvent, HostIntentPayload, HostLifecycleEvent,
    HostLifecycleState, HostMemoryPressureEvent, HostMemoryPressureLevel, HostNotificationEvent,
    HostPermissionEvent, HostPowerMode, HostPowerModeEvent,
};
use crate::platform::os::abi_generated::{BackgroundEventValue, NotificationEventValue};

use super::harness::HarnessContext;

impl HarnessContext<'_> {
    /// Enqueue one host event through the runtime-owned host queue when available.
    pub(crate) fn dispatch_host_event(&self, event: HostEvent) -> RuntimeResult<()> {
        let runtime_id = self.call_context.host().host_runtime_id();
        let platform = self.call_context.host().platform();
        let queue = HostRuntimeRegistry::queue_for_runtime(runtime_id, platform);

        // prefer one queued host delivery so bootstrap and live observers share one path
        if let Ok(queue) = queue {
            queue.enqueue(event);
            return Ok(());
        }

        HostRuntimeRegistry::dispatch_host_event(runtime_id, &event)
    }

    /// Enqueue one host lifecycle transition for this harness runtime.
    pub(crate) fn enqueue_lifecycle_event(&self, state: HostLifecycleState) -> RuntimeResult<()> {
        self.dispatch_host_event(HostEvent::Lifecycle(HostLifecycleEvent { state }))
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
        self.dispatch_host_event(HostEvent::Permission(HostPermissionEvent {
            permission: permission.to_string(),
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
