use std::sync::Arc;
#[cfg(any(test, target_os = "android"))]
use std::sync::atomic::{AtomicBool, Ordering};

use destack_workspace::PlatformHostOptions;
#[cfg(any(test, target_os = "android"))]
use parking_lot::RwLock;

use super::{
    HostEvent, HostInterruptionEvent, HostLifecycleEvent, HostLifecycleState,
    HostMemoryPressureEvent, HostMemoryPressureLevel, HostPermissionEvent, HostPowerMode,
    HostPowerModeEvent, HostStateStore, HostThermalEvent, HostThermalState, HostWallClockEvent,
    HostWindowEvent, HostWindowFocusEvent,
};
use crate::diagnostic::RuntimeResult;
#[cfg(any(test, target_os = "android"))]
use crate::runtime::host::android::AndroidHostBindings;
use crate::runtime::poller::HostPollerWakeHandle;

/// Shared per-runtime host bridge for adapter event ingestion.
#[derive(Debug, Clone)]
pub(crate) struct HostBridge {
    /// Shared host event queue.
    events: super::HostEventQueue,
    /// Shared mutable host service state.
    state_store: Arc<HostStateStore>,
    /// Shared Android callback bindings payload for this runtime.
    #[cfg(any(test, target_os = "android"))]
    android_bindings: Arc<RwLock<AndroidHostBindings>>,
    /// Whether Android bindings were registered for this runtime.
    #[cfg(any(test, target_os = "android"))]
    is_android_bindings_registered: Arc<AtomicBool>,
}

impl HostBridge {
    /// Create one host bridge from one shared service state object.
    pub(crate) fn new(state_store: Arc<HostStateStore>) -> Self {
        // initialize host bridge shared state
        Self {
            events: super::HostEventQueue::new(),
            state_store,
            #[cfg(any(test, target_os = "android"))]
            android_bindings: Arc::new(RwLock::new(AndroidHostBindings::default())),
            #[cfg(any(test, target_os = "android"))]
            is_android_bindings_registered: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Return the shared host service state for this bridge.
    pub(crate) fn state_store(&self) -> &Arc<HostStateStore> {
        &self.state_store
    }

    /// Return one shared wake handle for this bridge.
    pub(crate) fn wake_handle(&self) -> Arc<dyn HostPollerWakeHandle> {
        self.events.wake_handle()
    }

    /// Register one runtime-scoped Android bindings payload.
    #[cfg(any(test, target_os = "android"))]
    pub(crate) fn register_android_bindings(&self, bindings: AndroidHostBindings) -> bool {
        // reject duplicate registration for one runtime
        if self
            .is_android_bindings_registered
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_err()
        {
            return false;
        }

        // store one runtime bindings payload
        let mut current_bindings = self.android_bindings.write();
        *current_bindings = bindings;

        true
    }

    /// Return one runtime-scoped Android bindings snapshot.
    #[cfg(any(test, target_os = "android"))]
    pub(crate) fn android_bindings(&self) -> Option<AndroidHostBindings> {
        // report missing bindings until one registration succeeds
        if !self.is_android_bindings_registered.load(Ordering::SeqCst) {
            return None;
        }

        // copy one stable bindings snapshot
        let current_bindings = self.android_bindings.read();
        Some(*current_bindings)
    }

    /// Configure host integration options on bridge queue policy.
    pub(crate) fn configure_host_options(&self, host_options: &PlatformHostOptions) {
        let queue_capacity = host_options
            .event_queue_capacity
            .and_then(|capacity| usize::try_from(capacity).ok());
        self.events.configure(queue_capacity);
    }

    /// Take the number of dropped events observed by the bridge queue.
    pub(crate) fn take_dropped_event_count(&self) -> u64 {
        self.events.take_dropped_event_count()
    }

    /// Poll events from this bridge and apply service-state updates.
    pub(crate) fn poll_events(&self, timeout_nanos: Option<u64>) -> RuntimeResult<Vec<HostEvent>> {
        let events = self.events.poll_events(timeout_nanos)?;
        for event in &events {
            self.state_store.apply_event(event);
        }

        Ok(events)
    }

    /// Enqueue one raw host event.
    pub(crate) fn push_event(&self, event: HostEvent) {
        self.events.enqueue(event);
    }

    /// Enqueue one lifecycle event.
    pub(crate) fn push_lifecycle(&self, state: HostLifecycleState) {
        self.push_event(HostEvent::Lifecycle(HostLifecycleEvent { state }));
    }

    /// Enqueue one window event.
    pub(crate) fn push_window(&self, event: HostWindowEvent) {
        self.push_event(HostEvent::Window(event));
    }

    /// Enqueue one window focus event.
    pub(crate) fn push_window_focus(&self, is_focused: bool) {
        self.push_event(HostEvent::WindowFocus(HostWindowFocusEvent { is_focused }));
    }

    /// Enqueue one permission result event.
    pub(crate) fn push_permission_result(&self, permission: &str, granted: bool) {
        self.push_event(HostEvent::Permission(HostPermissionEvent {
            permission: permission.to_string(),
            granted,
        }));
    }

    /// Enqueue one interruption event.
    pub(crate) fn push_interruption(&self, interrupted: bool) {
        self.push_event(HostEvent::Interruption(HostInterruptionEvent {
            interrupted,
        }));
    }

    /// Enqueue one memory pressure event.
    pub(crate) fn push_memory_pressure(&self, level: HostMemoryPressureLevel) {
        self.push_event(HostEvent::MemoryPressure(HostMemoryPressureEvent { level }));
    }

    /// Enqueue one thermal state event.
    pub(crate) fn push_thermal_state(&self, state: HostThermalState) {
        self.push_event(HostEvent::ThermalState(HostThermalEvent { state }));
    }

    /// Enqueue one power mode event.
    pub(crate) fn push_power_mode(&self, mode: HostPowerMode) {
        self.push_event(HostEvent::PowerMode(HostPowerModeEvent { mode }));
    }

    /// Enqueue one wall clock change event.
    pub(crate) fn push_wall_clock_changed(&self) {
        self.push_event(HostEvent::WallClock(HostWallClockEvent));
    }
}
