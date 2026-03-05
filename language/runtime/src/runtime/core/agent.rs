use std::sync::Arc;

use destack_heap as heap;
use serde::{Deserialize, Serialize};

use crate::diagnostic::{AgentDiagnosticStore, RuntimeError, RuntimeResult};
use crate::host::HostEventKind;
use crate::platform::{PlatformContext, ResourceId, ResourceTable};
use crate::runtime::Hooks;
use crate::runtime::bindings::{BindingPolicy, BindingRegistry};
use crate::runtime::capability::resolve_capability_profile;
use crate::runtime::engine::EngineContinuation;
use crate::runtime::memory::Heap;
use crate::runtime::poller::PollerToken;
use crate::runtime::scheduler::{EventLoop, EventLoopWatch};
use crate::runtime::snapshot::SnapshotStore;
use crate::runtime::time::HostClockSource;
use crate::runtime::world::{RuntimeId, World, WorldCommand};
use destack_workspace::RuntimeOptions;

/// Stable identifier for one runtime-managed agent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct AgentId(pub u64);

/// Primary agent lane for executing Destack programs.
pub struct Agent {
    /// Monotonic process-local agent identity.
    pub(crate) id: AgentId,
    /// Runtime owner identifier in world topology.
    pub(crate) runtime_id: RuntimeId,
    /// Platform context for host integrations.
    pub(crate) platform: PlatformContext,
    /// Shared deterministic world for policy, simulation, clock, and randomness.
    pub(crate) world: Arc<World>,
    /// Immutable runtime options.
    pub(crate) options: RuntimeOptions,

    /// External resource table and finalizers.
    pub(crate) resources: ResourceTable,
    /// Agent hooks and effect state.
    pub(crate) hooks: Arc<Hooks>,
    /// Agent diagnostics storage for runtime errors and warning events.
    pub(crate) diagnostic: Arc<AgentDiagnosticStore>,
    /// External binding registry and policy enforcement.
    pub(crate) bindings: BindingRegistry,
    /// Managed heap and GC coordination.
    pub(crate) heap: Heap,
    /// Event loop for tasks, microtasks, and timers.
    pub(crate) event_loop: Box<EventLoop>,
}

impl std::fmt::Debug for Agent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Agent")
            .field("agent_id", &self.id)
            .field("runtime_id", &self.runtime_id)
            .field("platform", &self.platform)
            .field("options", &self.options)
            .field("resources", &self.resources)
            .field("hooks", &self.hooks)
            .field("world", &self.world)
            .field("diagnostic", &self.diagnostic)
            .field("bindings", &self.bindings)
            .field("heap", &self.heap)
            .field("event_loop", &self.event_loop)
            .finish()
    }
}

impl Agent {
    /// Create one agent with explicit runtime options.
    pub fn new(platform: PlatformContext, options: &RuntimeOptions) -> RuntimeResult<Self> {
        let world = Self::create_world(options, None)?;
        let (runtime_id, agent_id) = Self::register_runtime(world.as_ref(), options)?;

        Self::assemble(platform, options, world, runtime_id, agent_id)
    }

    /// Create one agent with explicit runtime options in one shared world.
    pub fn new_in_world(
        platform: PlatformContext,
        options: &RuntimeOptions,
        world: impl Into<Arc<World>>,
    ) -> RuntimeResult<Self> {
        let world = world.into();
        let (runtime_id, agent_id) = Self::register_runtime(world.as_ref(), options)?;

        Self::assemble(platform, options, world, runtime_id, agent_id)
    }

    /// Create one agent with explicit runtime options in one existing runtime.
    pub(crate) fn new_in_runtime(
        platform: PlatformContext,
        options: &RuntimeOptions,
        world: impl Into<Arc<World>>,
        runtime_id: RuntimeId,
    ) -> RuntimeResult<Self> {
        let world = world.into();
        let agent_id = Self::register_agent(world.as_ref(), options, runtime_id)?;

        Self::assemble(platform, options, world, runtime_id, agent_id)
    }

    /// Create one agent with explicit runtime options and host clock source.
    #[cfg(test)]
    pub(crate) fn new_with_host_clock_source(
        platform: PlatformContext,
        options: &RuntimeOptions,
        host_clock_source: Arc<dyn HostClockSource>,
    ) -> RuntimeResult<Self> {
        let world = Self::create_world(options, Some(host_clock_source))?;
        let (runtime_id, agent_id) = Self::register_runtime(world.as_ref(), options)?;

        Self::assemble(platform, options, world, runtime_id, agent_id)
    }

    /// Create one shared world from runtime options.
    fn create_world(
        options: &RuntimeOptions,
        host_clock_source: Option<Arc<dyn HostClockSource>>,
    ) -> RuntimeResult<Arc<World>> {
        let world = World::new(options, host_clock_source)?;
        Ok(Arc::new(world))
    }

    /// Assemble one agent from registered runtime and agent identities.
    fn assemble(
        platform: PlatformContext,
        options: &RuntimeOptions,
        world: Arc<World>,
        runtime_id: RuntimeId,
        agent_id: AgentId,
    ) -> RuntimeResult<Self> {
        // hooks and resources
        let hooks = Arc::new(Hooks::new(
            world.clone(),
            runtime_id,
            agent_id,
            world.replay().mode(),
        ));
        let resources = ResourceTable::default();
        resources.set_hooks(hooks.clone());

        // bindings, heap, event loop
        let mut bindings = BindingRegistry::new();
        bindings.set_policy(BindingPolicy::new(world.replay().mode()));
        bindings.install_native_defaults();
        bindings.apply_runtime_defaults(options);
        Self::apply_capability_profile(&mut bindings, options)?;

        let mut heap = Heap::default();
        heap.configure_gc(options.gc.clone());

        let mut event_loop = Box::new(EventLoop::default());
        event_loop.configure(options.scheduler.clone())?;

        // agent state
        Ok(Self {
            id: agent_id,
            runtime_id,
            platform,
            options: options.clone(),
            resources,
            hooks,
            world,
            diagnostic: Arc::new(AgentDiagnosticStore::from_options(&options.diagnostic)),
            bindings,
            heap,
            event_loop,
        })
    }

    /// Apply one capability profile from runtime options to binding policy checks.
    fn apply_capability_profile(
        bindings: &mut BindingRegistry,
        options: &RuntimeOptions,
    ) -> RuntimeResult<()> {
        let Some(capability_profile) = options.security.capability_profile.as_deref() else {
            return Ok(());
        };

        let capabilities = resolve_capability_profile(capability_profile).map_err(|message| {
            RuntimeError::Internal {
                message: format!(
                    "runtime capability profile `{capability_profile}` is invalid: {message}"
                ),
            }
            .boxed()
        })?;

        bindings.set_capabilities(capabilities);
        bindings.set_capability_requirements_enforced(true);

        Ok(())
    }

    /// Register one new runtime and one primary agent in one world.
    fn register_runtime(
        world: &World,
        options: &RuntimeOptions,
    ) -> RuntimeResult<(RuntimeId, AgentId)> {
        // runtime selector metadata
        let runtime_name = options
            .name
            .clone()
            .unwrap_or_else(|| "runtime".to_string());
        let runtime_labels = options.labels.clone();

        // agent selector metadata
        let runtime_id = world.allocate_runtime_id();
        let agent_id = world.allocate_agent_id();
        let agent_name = options
            .primary_agent
            .name
            .clone()
            .unwrap_or_else(|| format!("agent-{}", agent_id.0));
        let agent_labels = options.primary_agent.labels.clone();

        // register runtime and agent in one world command
        let _ = world.apply(WorldCommand::CreateRuntime {
            runtime_id,
            runtime_name,
            runtime_labels,
            primary_agent_id: agent_id,
            primary_agent_name: agent_name,
            primary_agent_labels: agent_labels,
        })?;

        Ok((runtime_id, agent_id))
    }

    /// Register one agent in one existing runtime.
    fn register_agent(
        world: &World,
        options: &RuntimeOptions,
        runtime_id: RuntimeId,
    ) -> RuntimeResult<AgentId> {
        // agent selector metadata
        let agent_id = world.allocate_agent_id();
        let agent_name = options
            .primary_agent
            .name
            .clone()
            .unwrap_or_else(|| format!("agent-{}", agent_id.0));
        let agent_labels = options.primary_agent.labels.clone();

        // register one agent in one existing runtime
        let _ = world.apply(WorldCommand::CreateAgent {
            runtime_id,
            agent_id,
            agent_name,
            agent_labels,
        })?;

        Ok(agent_id)
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
        resume_value: heap::Value,
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
        resume_value: heap::Value,
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
        resume_value: heap::Value,
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
        let branch_id = self.world.replay().log().branch_id();
        let sequence = self.world.replay().log().next_sequence();

        // NOTE #Incomplete: snapshot payload capture is not implemented yet
        let payload = Vec::new();

        // write snapshot payload and register in the replay log
        let metadata = store.write_snapshot(checkpoint_id, branch_id, sequence, &payload)?;
        self.world
            .replay()
            .log()
            .record_checkpoint(metadata.into_checkpoint_index())?;

        Ok(())
    }
}

impl Drop for Agent {
    fn drop(&mut self) {
        let _ = self
            .world
            .apply(WorldCommand::RemoveAgent { agent_id: self.id });
    }
}
