use std::sync::Arc;

use crate::diagnostic::RuntimeResult;
use crate::host::{Host, HostEventKind};
use crate::platform::{PlatformContext, ResourceId};
use crate::runtime::bindings::{BindingPolicy, BindingRegistry};
use crate::runtime::engine::{EngineContinuation, RuntimeValue};
use crate::runtime::memory::Heap;
use crate::runtime::poller::PollerToken;
use crate::runtime::scheduler::{EventLoop, EventLoopWatch};
use crate::runtime::snapshot::SnapshotStore;
use crate::runtime::world::World;
use destack_workspace::RuntimeOptions;

use super::RuntimeContext;

/// Primary agent lane for executing Destack programs.
pub struct Agent {
    /// External binding registry and policy enforcement.
    pub bindings: BindingRegistry,
    /// Shared agent state for platform bindings and execution.
    pub state: Arc<RuntimeContext>,
    /// Shared world attached to this agent.
    pub world: Arc<World>,
    /// Managed heap and GC coordination.
    pub heap: Heap,
    /// Event loop for tasks, microtasks, and timers.
    pub event_loop: Box<EventLoop>,
    /// Last applied policy revision from the attached world.
    applied_policy_revision: u64,
}

impl std::fmt::Debug for Agent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Agent")
            .field("bindings", &self.bindings)
            .field("state", &self.state)
            .field("world", &self.world)
            .field("heap", &self.heap)
            .field("event_loop", &self.event_loop)
            .field("host", &self.state.host)
            .finish()
    }
}

impl Agent {
    /// Create one agent with explicit shared runtime state.
    pub fn new(state: Arc<RuntimeContext>) -> Self {
        let world = Arc::new(state.world.clone());
        let event_loop = Box::new(EventLoop::default());
        let mut bindings = BindingRegistry::new();
        let policy = BindingPolicy::new(state.replay.mode());
        bindings.set_policy(policy);
        bindings.set_runtime_handles(&state, event_loop.as_ref());
        bindings.install_native_defaults();
        let mut heap = Heap::default();
        heap.configure_gc(state.options.gc.clone());

        Self {
            bindings,
            state,
            world,
            heap,
            event_loop,
            applied_policy_revision: 0,
        }
    }

    /// Create one agent with explicit runtime options.
    #[allow(clippy::arc_with_non_send_sync)]
    pub fn from_options(
        platform: PlatformContext,
        options: &RuntimeOptions,
    ) -> RuntimeResult<Self> {
        let state = Arc::new(RuntimeContext::from_options(platform, options));
        let mut agent = Self::new(state);
        agent.event_loop.configure(options.scheduler.clone())?;
        agent.bindings.apply_runtime_defaults(options);
        agent.apply_world_policy_if_needed();
        Ok(agent)
    }

    /// Create one agent with explicit runtime options in one shared world.
    #[allow(clippy::arc_with_non_send_sync)]
    pub fn from_options_in_world(
        platform: PlatformContext,
        options: &RuntimeOptions,
        world: World,
    ) -> RuntimeResult<Self> {
        let state = Arc::new(RuntimeContext::from_options_in_world(
            platform, options, world,
        ));
        let mut agent = Self::new(state);
        agent.event_loop.configure(options.scheduler.clone())?;
        agent.bindings.apply_runtime_defaults(options);
        agent.apply_world_policy_if_needed();
        Ok(agent)
    }

    /// Synchronize agent policy state from the attached world.
    pub(super) fn apply_world_policy_if_needed(&mut self) {
        // skip work when policy state has not changed
        let policy_revision = self.world.policy_revision();
        if policy_revision == self.applied_policy_revision {
            return;
        }

        // rebuild policy and hook plans from the current world policy set
        let policy = self.world.policy();
        self.bindings.apply_runtime_policy(&policy);
        self.state.hooks.apply_policy(&policy);
        self.world
            .prepare_policy_trigger_state(policy_revision, self.state.hooks.rule_count());
        self.applied_policy_revision = policy_revision;
    }

    /// Borrow host integration.
    pub fn host(&self) -> &Host {
        &self.state.host
    }

    /// Return the callback agent id used by native host callback routing.
    pub fn host_callback_agent_id(&self) -> Option<u64> {
        self.state.host.callback_runtime_id()
    }

    /// Borrow the shared world attached to this agent.
    pub fn world(&self) -> &World {
        self.world.as_ref()
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

    /// Register one host semantic event watch.
    pub fn watch_host_event(
        &mut self,
        kind: HostEventKind,
        runnable: EngineContinuation,
        resume_value: RuntimeValue,
        priority: u8,
    ) -> RuntimeResult<()> {
        let watch = EventLoopWatch {
            runnable,
            resume_value,
            priority,
        };
        self.event_loop.watch_host_event(kind, watch)
    }

    /// Remove the host event watch registered for one host event kind.
    pub fn unwatch_host_event(&mut self, kind: HostEventKind) -> Option<EventLoopWatch> {
        self.event_loop.unwatch_host_event(kind)
    }

    /// Return the number of dropped events with no registered dispatch watch.
    pub fn dropped_unwatched_dispatch_events(&self) -> u64 {
        self.event_loop.dropped_unwatched_dispatch_events()
    }

    /// Return the number of dropped host queue events due to queue pressure.
    pub fn dropped_host_queue_events(&self) -> u64 {
        self.event_loop.dropped_host_queue_events()
    }

    /// Return the number of dropped dispatch events observed by the event loop.
    pub fn dropped_dispatch_events(&self) -> u64 {
        self.event_loop.dropped_dispatch_events()
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

impl Default for Agent {
    #[allow(clippy::arc_with_non_send_sync)]
    fn default() -> Self {
        Self::new(Arc::new(RuntimeContext::new(PlatformContext::new(
            Vec::new(),
        ))))
    }
}
