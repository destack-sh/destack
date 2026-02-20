use std::sync::Arc;

use crate::diagnostic::RuntimeResult;
use crate::platform::{PlatformContext, PlatformPoller, PollerToken, ResourceId};
use crate::runtime::bindings::{BindingPolicy, BindingRegistry};
use crate::runtime::engine::{EngineContinuation, RuntimeValue};
use crate::runtime::memory::Heap;
use crate::runtime::scheduler::{EventLoop, EventLoopWatch};
use crate::runtime::snapshot::SnapshotStore;
use destack_workspace::RuntimeOptions;

use super::RuntimeState;
use super::poller::poller_for_options;

/// Primary runtime instance for executing Destack programs.
pub struct Runtime {
    /// External binding registry and policy enforcement.
    pub bindings: BindingRegistry,
    /// Shared runtime state for platform bindings and execution.
    pub state: Arc<RuntimeState>,
    /// Managed heap and GC coordination.
    pub heap: Heap,
    /// Event loop for tasks, microtasks, and timers.
    pub event_loop: Box<EventLoop>,
    /// Optional platform poller for external events.
    pub poller: Option<Box<dyn PlatformPoller>>,
}

impl std::fmt::Debug for Runtime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Runtime")
            .field("bindings", &self.bindings)
            .field("state", &self.state)
            .field("heap", &self.heap)
            .field("event_loop", &self.event_loop)
            .field("poller", &"<platform poller>")
            .finish()
    }
}

impl Runtime {
    /// Create a runtime with explicit shared runtime state.
    pub fn new(state: Arc<RuntimeState>) -> Self {
        let event_loop = Box::new(EventLoop::default());
        let mut bindings = BindingRegistry::new();
        bindings.set_policy(BindingPolicy::new(state.replay.mode()));
        bindings.set_runtime_handles(&state, event_loop.as_ref());
        bindings.install_native_defaults();
        let mut heap = Heap::default();
        heap.configure_gc(state.gc.clone());

        Self {
            bindings,
            state,
            heap,
            event_loop,
            poller: None,
        }
    }

    /// Create a runtime with explicit runtime options.
    pub fn from_options(
        platform: PlatformContext,
        options: &RuntimeOptions,
    ) -> RuntimeResult<Self> {
        let state = Arc::new(RuntimeState::from_options(platform, options));
        let mut runtime = Self::new(state);
        runtime.event_loop.configure(options.scheduler.clone())?;
        runtime.bindings.apply_runtime_options(options);
        if let Some(poller) = poller_for_options(options)? {
            runtime.set_poller(poller);
        }
        Ok(runtime)
    }

    /// Attach a platform poller for external events.
    pub fn set_poller(&mut self, poller: Box<dyn PlatformPoller>) {
        self.poller = Some(poller);
    }

    /// Register one timer watch.
    pub fn watch_timer(
        &mut self,
        handle: ResourceId,
        runnable: EngineContinuation,
        resume_value: RuntimeValue,
        priority: u8,
    ) -> RuntimeResult<()> {
        let watch = EventLoopWatch {
            runnable,
            resume_value,
            priority,
        };
        self.event_loop.watch_timer(handle, watch)
    }

    /// Remove the timer watch registered for one timer handle.
    pub fn unwatch_timer(&mut self, handle: ResourceId) -> Option<EventLoopWatch> {
        self.event_loop.unwatch_timer(handle)
    }

    /// Register one event watch.
    pub fn watch_event(
        &mut self,
        token: PollerToken,
        runnable: EngineContinuation,
        resume_value: RuntimeValue,
        priority: u8,
    ) -> RuntimeResult<()> {
        let watch = EventLoopWatch {
            runnable,
            resume_value,
            priority,
        };
        self.event_loop.watch_event(token, watch)
    }

    /// Remove the event watch registered for one poller token.
    pub fn unwatch_event(&mut self, token: PollerToken) -> Option<EventLoopWatch> {
        self.event_loop.unwatch_event(token)
    }

    /// Return the number of dropped external events observed by the event loop.
    pub fn dropped_external_events(&self) -> u64 {
        self.event_loop.dropped_external_events()
    }

    /// Capture a runtime snapshot and record a checkpoint in the replay log.
    pub fn snapshot(&mut self, store: &SnapshotStore) -> RuntimeResult<()> {
        // allocate a new checkpoint id
        let checkpoint_id = store.allocate_checkpoint_id();

        // capture replay metadata
        let branch_id = self.state.replay.log().branch_id();
        let sequence = self.state.replay.log().next_sequence();

        // NOTE #Incomplete: snapshot payload capture is not implemented yet
        let payload = Vec::new();

        // write snapshot payload and register in the replay log
        let metadata = store.write_snapshot(checkpoint_id, branch_id, sequence, &payload)?;
        self.state
            .replay
            .log()
            .record_checkpoint(metadata.into_checkpoint_index())?;

        Ok(())
    }
}

impl Default for Runtime {
    fn default() -> Self {
        Self::new(Arc::new(RuntimeState::new(
            PlatformContext::new(Vec::new()),
        )))
    }
}
