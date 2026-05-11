use super::*;
use crate::platform::os::document::state::DocumentTransactionState;
use crate::platform::os::permission::state::PermissionTransactionState;

/// Runtime-owned `platform.os` module state.
#[derive(Clone)]
pub(crate) struct PlatformOsState {
    /// Activation marker for capture barriers.
    is_active: Arc<OnceLock<()>>,
    /// Current lifecycle state for this runtime.
    pub(super) lifecycle_state: Arc<RwLock<LifecycleState>>,
    /// Last host lifecycle state observed for this runtime.
    pub(super) host_lifecycle_state: Arc<RwLock<HostLifecycleState>>,
    /// Last observed permission states for this runtime.
    pub(crate) permission_states: Arc<RwLock<HashMap<Permission, PermissionState>>>,
    /// Last observed location sample for this runtime.
    pub(super) last_location_sample: Arc<RwLock<Option<LocationSampleValue>>>,
    /// Successful host queue bootstrap marker.
    host_queue_bootstrapped: Arc<OnceLock<()>>,
    /// Host queue bootstrap serialization for this runtime.
    host_queue_bootstrap_lock: Arc<Mutex<()>>,
    /// Monotonic identifier source for runtime-owned stream handles.
    next_stream_id: Arc<AtomicU64>,
    /// Monotonic identifier source for runtime-owned watch handles.
    next_watch_id: Arc<AtomicU64>,
    /// Runtime-owned lifecycle streams.
    pub(super) lifecycle_events: Arc<Mutex<HashMap<u64, Arc<LifecycleEventStream>>>>,
    /// Runtime-owned intent streams.
    pub(super) intent_events: Arc<Mutex<HashMap<u64, Arc<IntentEventStream>>>>,
    /// Runtime-owned background streams.
    pub(super) background_events: Arc<Mutex<HashMap<u64, Arc<BackgroundEventStream>>>>,
    /// Runtime-owned notification streams.
    pub(super) notification_events: Arc<Mutex<HashMap<u64, Arc<NotificationEventStream>>>>,
    /// Pending document pick transactions keyed by host request id.
    pub(crate) document_transactions:
        Arc<Mutex<HashMap<HostRequestId, Arc<DocumentTransactionState>>>>,
    /// Pending permission request transactions keyed by host request id.
    pub(crate) permission_transactions:
        Arc<Mutex<HashMap<HostRequestId, Arc<PermissionTransactionState>>>>,
    /// Runtime-owned network watches.
    pub(super) network_watches: Arc<Mutex<HashMap<u64, Arc<NetworkWatchStream>>>>,
    /// Active network poll callback handle when one is registered.
    pub(super) network_watch_callback: Arc<Mutex<Option<WorkerCallbackHandle>>>,
    /// Runtime-owned location watches.
    pub(super) location_watches: Arc<Mutex<HashMap<String, Arc<LocationWatchStream>>>>,
}

impl Default for PlatformOsState {
    fn default() -> Self {
        Self {
            is_active: Arc::new(OnceLock::new()),
            lifecycle_state: Arc::new(RwLock::new(LifecycleState::Inactive)),
            host_lifecycle_state: Arc::new(RwLock::new(HostLifecycleState::Initializing)),
            permission_states: Arc::new(RwLock::new(HashMap::new())),
            last_location_sample: Arc::new(RwLock::new(None)),
            host_queue_bootstrapped: Arc::new(OnceLock::new()),
            host_queue_bootstrap_lock: Arc::new(Mutex::new(())),
            next_stream_id: Arc::new(AtomicU64::new(1)),
            next_watch_id: Arc::new(AtomicU64::new(1)),
            lifecycle_events: Arc::new(Mutex::new(HashMap::new())),
            intent_events: Arc::new(Mutex::new(HashMap::new())),
            background_events: Arc::new(Mutex::new(HashMap::new())),
            notification_events: Arc::new(Mutex::new(HashMap::new())),
            document_transactions: Arc::new(Mutex::new(HashMap::new())),
            permission_transactions: Arc::new(Mutex::new(HashMap::new())),
            network_watches: Arc::new(Mutex::new(HashMap::new())),
            network_watch_callback: Arc::new(Mutex::new(None)),
            location_watches: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

impl std::fmt::Debug for PlatformOsState {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("PlatformOsState")
            .finish_non_exhaustive()
    }
}

impl PlatformOsState {
    /// Mark this OS state as active for capture barriers.
    pub(crate) fn mark_active(&self) {
        let _ = self.is_active.set(());
    }

    /// Return the current lifecycle state.
    pub(crate) fn lifecycle_state(&self) -> LifecycleState {
        *self.lifecycle_state.read()
    }

    /// Insert one runtime-owned lifecycle stream.
    pub(crate) fn insert_lifecycle_stream(&self, stream: Arc<LifecycleEventStream>) -> u64 {
        let stream_id = self.next_stream_id();
        self.lifecycle_events.lock().insert(stream_id, stream);

        stream_id
    }

    /// Return one runtime-owned lifecycle stream by id.
    pub(crate) fn lifecycle_stream(&self, id: u64) -> Option<Arc<LifecycleEventStream>> {
        self.lifecycle_events.lock().get(&id).cloned()
    }

    /// Remove one runtime-owned lifecycle stream by id.
    pub(crate) fn remove_lifecycle_stream(&self, id: u64) -> Option<Arc<LifecycleEventStream>> {
        self.lifecycle_events.lock().remove(&id)
    }

    /// Return every runtime-owned lifecycle stream.
    pub(crate) fn lifecycle_streams(&self) -> Vec<Arc<LifecycleEventStream>> {
        self.lifecycle_events.lock().values().cloned().collect()
    }

    /// Insert one runtime-owned intent stream.
    pub(crate) fn insert_intent_stream(&self, stream: Arc<IntentEventStream>) -> u64 {
        let stream_id = self.next_stream_id();
        self.intent_events.lock().insert(stream_id, stream);

        stream_id
    }

    /// Return one runtime-owned intent stream by id.
    pub(crate) fn intent_stream(&self, id: u64) -> Option<Arc<IntentEventStream>> {
        self.intent_events.lock().get(&id).cloned()
    }

    /// Remove one runtime-owned intent stream by id.
    pub(crate) fn remove_intent_stream(&self, id: u64) -> Option<Arc<IntentEventStream>> {
        self.intent_events.lock().remove(&id)
    }

    /// Return every runtime-owned intent stream.
    pub(crate) fn intent_streams(&self) -> Vec<Arc<IntentEventStream>> {
        self.intent_events.lock().values().cloned().collect()
    }

    /// Insert one runtime-owned background stream.
    pub(crate) fn insert_background_stream(&self, stream: Arc<BackgroundEventStream>) -> u64 {
        let stream_id = self.next_stream_id();
        self.background_events.lock().insert(stream_id, stream);

        stream_id
    }

    /// Return one runtime-owned background stream by id.
    pub(crate) fn background_stream(&self, id: u64) -> Option<Arc<BackgroundEventStream>> {
        self.background_events.lock().get(&id).cloned()
    }

    /// Remove one runtime-owned background stream by id.
    pub(crate) fn remove_background_stream(&self, id: u64) -> Option<Arc<BackgroundEventStream>> {
        self.background_events.lock().remove(&id)
    }

    /// Return every runtime-owned background stream.
    pub(crate) fn background_streams(&self) -> Vec<Arc<BackgroundEventStream>> {
        self.background_events.lock().values().cloned().collect()
    }

    /// Insert one runtime-owned notification stream.
    pub(crate) fn insert_notification_stream(&self, stream: Arc<NotificationEventStream>) -> u64 {
        let stream_id = self.next_stream_id();
        self.notification_events.lock().insert(stream_id, stream);

        stream_id
    }

    /// Return one runtime-owned notification stream by id.
    pub(crate) fn notification_stream(&self, id: u64) -> Option<Arc<NotificationEventStream>> {
        self.notification_events.lock().get(&id).cloned()
    }

    /// Remove one runtime-owned notification stream by id.
    pub(crate) fn remove_notification_stream(
        &self,
        id: u64,
    ) -> Option<Arc<NotificationEventStream>> {
        self.notification_events.lock().remove(&id)
    }

    /// Return every runtime-owned notification stream.
    pub(crate) fn notification_streams(&self) -> Vec<Arc<NotificationEventStream>> {
        self.notification_events.lock().values().cloned().collect()
    }

    /// Insert one runtime-owned network watch.
    pub(crate) fn insert_network_watch(&self, stream: Arc<NetworkWatchStream>) -> u64 {
        let stream_id = self.next_stream_id();
        self.network_watches.lock().insert(stream_id, stream);

        stream_id
    }

    /// Return one runtime-owned network watch by id.
    pub(crate) fn network_watch(&self, id: u64) -> Option<Arc<NetworkWatchStream>> {
        self.network_watches.lock().get(&id).cloned()
    }

    /// Remove one runtime-owned network watch by id.
    pub(crate) fn remove_network_watch(&self, id: u64) -> Option<Arc<NetworkWatchStream>> {
        self.network_watches.lock().remove(&id)
    }

    /// Return every runtime-owned network watch.
    pub(crate) fn network_watch_streams(&self) -> Vec<Arc<NetworkWatchStream>> {
        self.network_watches.lock().values().cloned().collect()
    }

    /// Return whether any runtime-owned network watch is still open.
    pub(crate) fn has_network_watches(&self) -> bool {
        !self.network_watches.lock().is_empty()
    }

    /// Return the active network watch callback handle when one exists.
    pub(crate) fn network_watch_callback(&self) -> Option<WorkerCallbackHandle> {
        *self.network_watch_callback.lock()
    }

    /// Store one network watch callback handle.
    pub(crate) fn set_network_watch_callback(&self, handle: WorkerCallbackHandle) {
        *self.network_watch_callback.lock() = Some(handle);
    }

    /// Clear one registered network watch callback handle.
    pub(crate) fn clear_network_watch_callback(&self, handle: WorkerCallbackHandle) {
        let mut callback = self.network_watch_callback.lock();
        if callback.is_some_and(|current| current == handle) {
            *callback = None;
        }
    }

    /// Allocate one fresh runtime-owned location watch id.
    pub(crate) fn next_location_watch_id(&self) -> String {
        self.next_watch_id("location-watch")
    }

    /// Insert one runtime-owned location watch.
    pub(crate) fn insert_location_watch(&self, watch_id: String, stream: Arc<LocationWatchStream>) {
        self.location_watches.lock().insert(watch_id, stream);
    }

    /// Return one runtime-owned location watch by id.
    pub(crate) fn location_watch(&self, watch_id: &str) -> Option<Arc<LocationWatchStream>> {
        self.location_watches.lock().get(watch_id).cloned()
    }

    /// Remove one runtime-owned location watch by id.
    pub(crate) fn remove_location_watch(&self, watch_id: &str) -> Option<Arc<LocationWatchStream>> {
        self.location_watches.lock().remove(watch_id)
    }

    /// Reconcile current lifecycle state from already queued host events.
    pub(crate) fn reconcile_host_queue(
        &self,
        queue: &HostQueue,
        observer: &Arc<dyn HostEventObserver>,
    ) -> RuntimeResult<()> {
        if self.host_queue_bootstrapped.get().is_some() {
            return Ok(());
        }

        // serialize retries before the first successful completion
        let _lock = self.host_queue_bootstrap_lock.lock();
        if self.host_queue_bootstrapped.get().is_some() {
            return Ok(());
        }

        // register and snapshot under one queue lock so live host delivery cannot
        // race one bootstrap replay
        for event in queue.register_observer_and_snapshot(observer) {
            self.apply_host_event(&event);
        }

        let _ = self.host_queue_bootstrapped.set(());

        Ok(())
    }

    /// Return whether any OS state is active.
    fn is_active(&self) -> bool {
        self.is_active.get().is_some()
    }

    /// Allocate one fresh runtime-owned stream id.
    fn next_stream_id(&self) -> u64 {
        self.next_stream_id.fetch_add(1, Ordering::Relaxed)
    }

    /// Allocate one fresh runtime-owned watch id.
    fn next_watch_id(&self, prefix: &str) -> String {
        let sequence = self.next_watch_id.fetch_add(1, Ordering::Relaxed);

        format!("{prefix}-{sequence}")
    }

    /// Observe one host event and update OS state.
    fn apply_host_event(&self, event: &HostEvent) {
        // lifecycle streams only depend on lifecycle, memory, and power signals
        match event {
            HostEvent::Lifecycle(event) => self.observe_lifecycle_transition(event.state),
            HostEvent::Intent(event) => self.observe_intent_event(event),
            HostEvent::Background(event) => self.observe_background_event(event),
            HostEvent::Notification(event) => self.observe_notification_event(event),
            HostEvent::Location(event) => self.observe_location_event(event),
            HostEvent::Permission(event) => self.observe_permission_event(event),
            HostEvent::RequestCompletion(event) => {
                self.observe_document_completion(event);
                self.observe_permission_completion(event);
            }
            HostEvent::MemoryPressure(event) => self.observe_memory_pressure(event.level),
            HostEvent::PowerMode(event) => self.observe_power_mode(event.mode),
            _ => {}
        }
    }
}

impl Capture for PlatformOsState {
    type Image = ();
    type Error = Box<RuntimeError>;
    type CaptureContext<'a> = ();
    type RestoreContext<'a> = ();

    /// Capture one OS platform-state image.
    fn capture_image(
        &mut self,
        mode: CaptureMode,
        _context: Self::CaptureContext<'_>,
    ) -> Result<Self::Image, Self::Error> {
        if !self.is_active() {
            return Ok(());
        }

        Err(RuntimeError::CaptureBarrier {
            component: "platform.os".to_string(),
            mode: format!("{mode:?}"),
            detail: "state is active".to_string(),
        }
        .boxed())
    }

    /// Restore one OS platform-state image.
    fn restore_image(
        &mut self,
        _image: &Self::Image,
        _context: Self::RestoreContext<'_>,
    ) -> Result<(), Self::Error> {
        *self = Self::default();

        Ok(())
    }
}

impl HostEventObserver for PlatformOsState {
    /// Observe one host event and update OS state.
    fn observe_host_event(&self, event: &HostEvent) -> RuntimeResult<()> {
        self.apply_host_event(event);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_inserted_streams_are_owned_by_platform_os_state() {
        let state = PlatformOsState::default();
        let stream = Arc::new(LifecycleEventStream::default());

        let stream_id = state.insert_lifecycle_stream(stream.clone());
        let resolved = state
            .lifecycle_stream(stream_id)
            .expect("inserted lifecycle stream should resolve by id");

        assert!(Arc::ptr_eq(&stream, &resolved));

        let removed = state
            .remove_lifecycle_stream(stream_id)
            .expect("inserted lifecycle stream should remove by id");

        assert!(Arc::ptr_eq(&stream, &removed));
        assert!(
            state.lifecycle_stream(stream_id).is_none(),
            "removed lifecycle stream should no longer resolve"
        );
    }

    #[test]
    fn test_inserted_watches_are_owned_by_platform_os_state() {
        let state = PlatformOsState::default();
        let watch_id = state.next_location_watch_id();
        let stream = Arc::new(LocationWatchStream::new());

        state.insert_location_watch(watch_id.clone(), stream.clone());

        let resolved = state
            .location_watch(&watch_id)
            .expect("inserted location watch should resolve by id");

        assert!(Arc::ptr_eq(&stream, &resolved));

        let removed = state
            .remove_location_watch(&watch_id)
            .expect("inserted location watch should remove by id");

        assert!(Arc::ptr_eq(&stream, &removed));
        assert!(
            state.location_watch(&watch_id).is_none(),
            "removed location watch should no longer resolve"
        );
    }
}
