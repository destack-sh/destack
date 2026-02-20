use std::sync::Arc;

use crate::diagnostic::RuntimeResult;
use crate::platform::{PlatformContext, PlatformPoller};
use crate::runtime::RuntimeHookState;
use crate::runtime::bindings::{BindingPolicy, BindingRegistry};
use crate::runtime::memory::Heap;
use crate::runtime::scheduler::EventLoop;
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
        heap.configure_gc(state.gc_options.clone());

        Self {
            bindings,
            state,
            heap,
            event_loop,
            poller: None,
        }
    }

    /// Create a runtime with explicit runtime options.
    pub fn from_runtime_options(
        platform: PlatformContext,
        options: &RuntimeOptions,
    ) -> RuntimeResult<Self> {
        let state = Arc::new(RuntimeState::from_runtime_options(platform, options));
        let mut runtime = Self::new(state);
        runtime.event_loop.configure(options.scheduler.clone());
        runtime.bindings.apply_runtime_options(options);
        if let Some(poller) = poller_for_options(options)? {
            runtime.set_poller(poller);
        }
        Ok(runtime)
    }

    /// Create a runtime with the configured platform poller.
    pub fn with_configured_poller(
        state: Arc<RuntimeState>,
        options: &RuntimeOptions,
    ) -> RuntimeResult<Self> {
        let mut runtime = Self::new(state);
        runtime.event_loop.configure(options.scheduler.clone());
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

    /// Run runtime ticks until no work remains.
    pub fn tick_until_idle(&mut self) -> RuntimeResult<()> {
        loop {
            // drive one runtime tick
            let progressed = self.tick_once()?;

            // exit once no work remains
            if !progressed {
                break;
            }
        }

        Ok(())
    }

    /// Execute one runtime tick.
    pub fn tick_once(&mut self) -> RuntimeResult<bool> {
        // poll timers for ready callbacks
        let now = self.state.time.wall_nanos();
        let ready_timers = self.event_loop.poll_timers(now)?;
        let mut progressed = !ready_timers.is_empty();
        if progressed {
            self.state
                .rules
                .on_scheduler_timer_fire(RuntimeHookState::empty());
        }

        // TODO #Incomplete: wire timer callbacks into tasks

        // poll platform events if a poller is installed
        if let Some(poller) = self.poller.as_mut() {
            let event_count = self.event_loop.poll_poller(poller.as_mut(), Some(0))?;
            if event_count > 0 {
                progressed = true;
                self.state.rules.on_scheduler_event_wake(RuntimeHookState {
                    external_event_count: Some(event_count),
                    ..RuntimeHookState::empty()
                });
            }
        }

        // run one gc cycle when pacing says a cycle is due
        if self.heap.should_collect() {
            let _stats = self.heap.collect();
            progressed = true;
        }

        // TODO #Incomplete: wire event loop runnables into tasks

        Ok(progressed)
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
