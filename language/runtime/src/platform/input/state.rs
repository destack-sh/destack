#[cfg(any(target_os = "android", target_os = "ios"))]
use std::collections::HashMap;
#[cfg(any(unix, windows, target_os = "android", target_os = "ios"))]
use std::sync::{Arc, OnceLock};

use destack_core::{Capture, CaptureMode};
#[cfg(any(target_os = "android", target_os = "ios"))]
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};

use crate::diagnostic::RuntimeError;
#[cfg(any(unix, windows, target_os = "android", target_os = "ios"))]
use crate::diagnostic::RuntimeResult;
#[cfg(any(target_os = "android", target_os = "ios"))]
use crate::host::abi::text::HostTextInputCloseRequest;
#[cfg(any(target_os = "android", target_os = "ios"))]
use crate::host::{HostEvent, HostEventObserver, HostQueue, HostSessionRegistry};
#[cfg(any(target_os = "android", target_os = "ios"))]
use crate::platform::diagnostic::PlatformErrorCode;
#[cfg(any(target_os = "android", target_os = "ios"))]
use crate::platform::input::{
    InputTextGeometry, InputTextSessionEventValue, InputTextSessionStateValue,
};
#[cfg(any(target_os = "android", target_os = "ios"))]
use crate::platform::resource;
#[cfg(any(target_os = "android", target_os = "ios"))]
use crate::platform::resource::ResourceFinalizer;
#[cfg(any(target_os = "android", target_os = "ios"))]
use crate::platform::resource::ResourceId;
#[cfg(any(unix, windows, target_os = "android", target_os = "ios"))]
use crate::runtime::BindingCallContext;
#[cfg(any(target_os = "android", target_os = "ios"))]
use crate::runtime::RuntimeEventQueue;
#[cfg(any(unix, windows))]
use crate::runtime::service::ServiceHandle;

#[cfg(any(target_os = "android", target_os = "ios"))]
use super::core::text::{push_host_text_state_event, text_session_not_found};
#[cfg(unix)]
use super::host::{
    UnixInputMonitorRuntimeState, UnixInputMonitorService, unix_input_monitor_service,
};
#[cfg(windows)]
use super::host::{
    WindowsRawInputRuntimeState, WindowsRawInputService, WindowsXInputService,
    windows_raw_input_service, windows_xinput_service,
};

/// Runtime-owned host text state.
#[cfg(any(target_os = "android", target_os = "ios"))]
#[derive(Debug, Default)]
pub(super) struct HostTextState {
    /// Successful host queue bootstrap marker.
    host_queue_bootstrapped: Arc<OnceLock<()>>,

    /// Host queue bootstrap serialization for this runtime.
    host_queue_bootstrap_lock: Arc<Mutex<()>>,

    /// Active host text sessions keyed by text-session resource id.
    sessions: Arc<Mutex<HashMap<u64, HostTextSession>>>,
}

/// Runtime-owned host text session payload.
#[cfg(any(target_os = "android", target_os = "ios"))]
#[derive(Debug)]
struct HostTextSession {
    /// The current text-geometry hint.
    geometry: Option<InputTextGeometry>,

    /// The current renderer or host-authoritative text state.
    state: InputTextSessionStateValue,

    /// The next session event sequence number.
    next_sequence: u64,

    /// The target window hint for session metadata.
    target_window: Option<resource::WindowHandle>,

    /// The queued session events.
    events: Arc<RuntimeEventQueue<InputTextSessionEventValue>>,
}

/// Leak finalizer for one host text session.
#[cfg(any(target_os = "android", target_os = "ios"))]
#[derive(Debug)]
struct HostTextSessionFinalizer {
    /// The owning host session handle.
    host_session_id: u64,

    /// The finalized text-session resource id.
    session_id: u64,

    /// Shared host text session storage.
    sessions: Arc<Mutex<HashMap<u64, HostTextSession>>>,
}

/// Host event observer for runtime-owned host text state.
#[cfg(any(target_os = "android", target_os = "ios"))]
#[derive(Debug)]
struct HostTextObserver {
    /// Shared host text session storage.
    sessions: Arc<Mutex<HashMap<u64, HostTextSession>>>,
}

/// Worker-owned input module state.
#[derive(Default)]
pub(crate) struct PlatformInputState {
    /// Worker-owned host text state.
    #[cfg(any(target_os = "android", target_os = "ios"))]
    pub(super) host_text_state: HostTextState,

    /// Shared unix input-monitor service handle for this worker.
    #[cfg(unix)]
    unix_input_monitor_service: ServiceHandle<UnixInputMonitorService>,

    /// Worker-owned unix input-monitor state.
    #[cfg(unix)]
    unix_input_monitor_runtime_state: OnceLock<Arc<UnixInputMonitorRuntimeState>>,

    /// Shared windows raw-input service handle for this worker.
    #[cfg(windows)]
    windows_raw_input_service: ServiceHandle<WindowsRawInputService>,

    /// Shared windows xinput packet service handle for this worker.
    #[cfg(windows)]
    windows_xinput_service: ServiceHandle<WindowsXInputService>,

    /// Worker-owned windows raw-input state.
    #[cfg(windows)]
    windows_raw_input_runtime_state: OnceLock<Arc<WindowsRawInputRuntimeState>>,
}

impl std::fmt::Debug for PlatformInputState {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("PlatformInputState")
            .finish_non_exhaustive()
    }
}

impl PlatformInputState {
    /// Return whether any worker-owned input state is active.
    fn has_runtime_state(&self) -> bool {
        #[cfg(any(target_os = "android", target_os = "ios"))]
        if self.host_text_state.has_live_sessions() {
            return true;
        }

        #[cfg(unix)]
        if self.unix_input_monitor_runtime_state.get().is_some() {
            return true;
        }

        #[cfg(windows)]
        if self.windows_raw_input_runtime_state.get().is_some() {
            return true;
        }

        false
    }

    /// Capture one input-state image.
    fn image(&self, mode: CaptureMode) -> Result<PlatformInputImage, Box<RuntimeError>> {
        if !self.has_runtime_state() {
            return Ok(PlatformInputImage);
        }

        Err(RuntimeError::CaptureBarrier {
            component: "platform.input".to_string(),
            mode: format!("{mode:?}"),
            detail: "runtime state is active".to_string(),
        }
        .boxed())
    }

    /// Bootstrap one shared input state from runtime host queue delivery.
    #[cfg(any(target_os = "android", target_os = "ios"))]
    pub(super) fn bootstrap_host_text_state(
        &self,
        binding: &BindingCallContext,
    ) -> RuntimeResult<()> {
        let observer: Arc<dyn HostEventObserver> = Arc::new(HostTextObserver {
            sessions: Arc::clone(&self.host_text_state.sessions),
        });
        let host_session_id = binding.host().host_session_id();
        let platform = binding.host().platform();
        let queue = HostSessionRegistry::queue_for_session(host_session_id, platform);

        // text input can still function without queue catchup if ingress routing
        // is not registered yet, or if tests are dispatching host events directly
        match queue {
            Ok(queue) => self.host_text_state.reconcile_host_queue(&queue, &observer),
            Err(error) if is_missing_host_queue_error(&error) => Ok(()),
            Err(error) => Err(error),
        }
    }

    /// Insert one host text session before host open so early ingress is not lost.
    #[cfg(any(target_os = "android", target_os = "ios"))]
    pub(super) fn insert_host_text_session(
        &self,
        session_id: u64,
        target_window: Option<resource::WindowHandle>,
        state: InputTextSessionStateValue,
    ) -> Arc<RuntimeEventQueue<InputTextSessionEventValue>> {
        let events = Arc::new(RuntimeEventQueue::default());
        let session = HostTextSession {
            geometry: None,
            state,
            next_sequence: 1,
            target_window,
            events: Arc::clone(&events),
        };

        self.host_text_state
            .sessions
            .lock()
            .insert(session_id, session);

        events
    }

    /// Build one leak finalizer for one host text session.
    #[cfg(any(target_os = "android", target_os = "ios"))]
    pub(super) fn host_text_session_finalizer(
        &self,
        host_session_id: u64,
        session_id: u64,
    ) -> Box<dyn ResourceFinalizer> {
        Box::new(HostTextSessionFinalizer {
            host_session_id,
            session_id,
            sessions: Arc::clone(&self.host_text_state.sessions),
        })
    }

    /// Remove one host text session.
    #[cfg(any(target_os = "android", target_os = "ios"))]
    pub(super) fn remove_host_text_session(
        &self,
        session_id: u64,
    ) -> Option<Arc<RuntimeEventQueue<InputTextSessionEventValue>>> {
        self.host_text_state
            .sessions
            .lock()
            .remove(&session_id)
            .map(|session| session.events)
    }

    /// Return one host text session queue.
    #[cfg(any(target_os = "android", target_os = "ios"))]
    pub(super) fn host_text_session_queue(
        &self,
        session_id: u64,
    ) -> Option<Arc<RuntimeEventQueue<InputTextSessionEventValue>>> {
        self.host_text_state
            .sessions
            .lock()
            .get(&session_id)
            .map(|session| Arc::clone(&session.events))
    }

    /// Return one host text session geometry.
    #[cfg(any(target_os = "android", target_os = "ios"))]
    pub(super) fn host_text_session_geometry(&self, session_id: u64) -> Option<InputTextGeometry> {
        self.host_text_state
            .sessions
            .lock()
            .get(&session_id)
            .and_then(|session| session.geometry)
    }

    /// Update one host text session geometry.
    #[cfg(any(target_os = "android", target_os = "ios"))]
    pub(super) fn set_host_text_session_geometry(
        &self,
        session_id: u64,
        geometry: InputTextGeometry,
    ) -> RuntimeResult<()> {
        let mut sessions = self.host_text_state.sessions.lock();
        let Some(session) = sessions.get_mut(&session_id) else {
            return Err(text_session_not_found(
                "destack.input.text.setGeometry",
                resource::InputTextSessionHandle(resource::ResourceId::local(session_id)),
            ));
        };

        session.geometry = Some(geometry);

        Ok(())
    }

    /// Update one host text session state.
    #[cfg(any(target_os = "android", target_os = "ios"))]
    pub(super) fn set_host_text_session_state(
        &self,
        session_id: u64,
        state: InputTextSessionStateValue,
    ) -> RuntimeResult<()> {
        let mut sessions = self.host_text_state.sessions.lock();
        let Some(session) = sessions.get_mut(&session_id) else {
            return Err(text_session_not_found(
                "destack.input.text.setState",
                resource::InputTextSessionHandle(resource::ResourceId::local(session_id)),
            ));
        };

        session.state = state;

        Ok(())
    }

    /// Return one shared unix input-monitor service handle for this worker.
    #[cfg(unix)]
    pub(crate) fn unix_input_monitor_service(
        &self,
        operation: &'static str,
    ) -> RuntimeResult<Arc<UnixInputMonitorService>> {
        self.unix_input_monitor_service
            .get_or_try_init(|| unix_input_monitor_service(operation))
    }

    /// Return one worker-owned unix input-monitor state.
    #[cfg(unix)]
    pub(crate) fn unix_input_monitor_runtime_state(
        &self,
        ctx: &BindingCallContext,
    ) -> Arc<UnixInputMonitorRuntimeState> {
        Arc::clone(
            self.unix_input_monitor_runtime_state
                .get_or_init(|| Arc::new(UnixInputMonitorRuntimeState::new(ctx.worker().id))),
        )
    }

    /// Return one shared windows raw-input service handle for this worker.
    #[cfg(windows)]
    pub(crate) fn windows_raw_input_service(
        &self,
        operation: &'static str,
    ) -> RuntimeResult<Arc<WindowsRawInputService>> {
        self.windows_raw_input_service
            .get_or_try_init(|| windows_raw_input_service(operation))
    }

    /// Return one shared windows xinput packet service handle for this worker.
    #[cfg(windows)]
    pub(crate) fn windows_xinput_service(
        &self,
        operation: &'static str,
    ) -> RuntimeResult<Arc<WindowsXInputService>> {
        self.windows_xinput_service
            .get_or_try_init(|| windows_xinput_service(operation))
    }

    /// Return one worker-owned windows raw-input state.
    #[cfg(windows)]
    pub(crate) fn windows_raw_input_runtime_state(
        &self,
        ctx: &BindingCallContext,
    ) -> Arc<WindowsRawInputRuntimeState> {
        Arc::clone(
            self.windows_raw_input_runtime_state
                .get_or_init(|| Arc::new(WindowsRawInputRuntimeState::new(ctx.worker().id))),
        )
    }
}

#[cfg(any(target_os = "android", target_os = "ios"))]
impl HostTextState {
    /// Return whether any host text session is still active.
    #[cfg(any(target_os = "android", target_os = "ios"))]
    pub(crate) fn has_live_sessions(&self) -> bool {
        !self.sessions.lock().is_empty()
    }

    /// Reconcile queued host text events and register the observer once.
    #[cfg(any(target_os = "android", target_os = "ios"))]
    fn reconcile_host_queue(
        &self,
        queue: &HostQueue,
        observer: &Arc<dyn HostEventObserver>,
    ) -> RuntimeResult<()> {
        if self.host_queue_bootstrapped.get().is_some() {
            return Ok(());
        }

        let _lock = self.host_queue_bootstrap_lock.lock();
        if self.host_queue_bootstrapped.get().is_some() {
            return Ok(());
        }

        for event in queue.register_observer_and_snapshot(observer) {
            observer.observe_host_event(&event)?;
        }

        let _ = self.host_queue_bootstrapped.set(());

        Ok(())
    }
}

#[cfg(any(target_os = "android", target_os = "ios"))]
impl ResourceFinalizer for HostTextSessionFinalizer {
    /// Close one leaked host text session.
    #[cfg(any(target_os = "android", target_os = "ios"))]
    fn finalize(self: Box<Self>, _resource_id: ResourceId) {
        let queue = {
            let mut sessions = self.sessions.lock();
            sessions
                .remove(&self.session_id)
                .map(|session| session.events)
        };

        if let Some(queue) = queue {
            queue.close();
        }

        let request = HostTextInputCloseRequest {
            session_id: self.session_id,
        };
        let _ = unsafe { super::unix::host_text_close(self.host_session_id, request) };
    }
}

#[cfg(any(target_os = "android", target_os = "ios"))]
impl HostEventObserver for HostTextObserver {
    /// Observe one queued host text event.
    #[cfg(any(target_os = "android", target_os = "ios"))]
    fn observe_host_event(&self, event: &HostEvent) -> RuntimeResult<()> {
        let HostEvent::Text(event) = event else {
            return Ok(());
        };

        let mut sessions = self.sessions.lock();
        let Some(session) = sessions.get_mut(&event.session_id) else {
            return Ok(());
        };

        session.state = event.state.clone();

        push_host_text_state_event(
            &session.events,
            &mut session.next_sequence,
            session.target_window,
            event.state.clone(),
        );

        Ok(())
    }
}

/// Return whether one queue lookup failed because no host queue exists yet.
#[cfg(any(target_os = "android", target_os = "ios"))]
fn is_missing_host_queue_error(error: &RuntimeError) -> bool {
    matches!(
        error,
        RuntimeError::Platform(platform_error)
            if platform_error.code == PlatformErrorCode::NotSupported
                && platform_error
                    .context
                    .as_ref()
                    .and_then(|context| context.feature.as_deref())
                    .is_some_and(|feature| feature.starts_with("runtime.host.queue."))
    )
}

/// Materialized input platform-state image.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct PlatformInputImage;

impl Capture for PlatformInputState {
    type Image = PlatformInputImage;
    type Error = Box<RuntimeError>;
    type CaptureContext<'a> = ();
    type RestoreContext<'a> = ();

    /// Capture one input platform-state image.
    fn capture_image(
        &mut self,
        mode: CaptureMode,
        _context: Self::CaptureContext<'_>,
    ) -> Result<Self::Image, Self::Error> {
        self.image(mode)
    }

    /// Restore one input platform-state image.
    fn restore_image(
        &mut self,
        _image: &Self::Image,
        _context: Self::RestoreContext<'_>,
    ) -> Result<(), Self::Error> {
        *self = Self::default();

        Ok(())
    }
}
