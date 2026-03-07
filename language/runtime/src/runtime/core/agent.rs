use std::sync::Arc;

use destack_base::fnv1a_64;
use destack_heap as heap;
use serde::{Deserialize, Serialize};

use crate::diagnostic::{DiagnosticStore, RuntimeError, RuntimeResult};
use crate::host::HostEventKind;
use crate::platform::state::PlatformState;
use crate::platform::{ResourceId, ResourceTable};
use crate::runtime::bindings::{BindingPolicy, BindingRegistry};
use crate::runtime::capability::resolve_capability_profile;
use crate::runtime::engine::EngineContinuation;
use crate::runtime::memory::Heap;
use crate::runtime::poller::PollerToken;
use crate::runtime::scheduler::{EventLoop, EventLoopWatch};
use crate::runtime::world::{RuntimeId, World, WorldCommand};
use crate::runtime::{DropCounts, DropReason, ExecutionContextId, Hooks, RuntimeFinalizers};
use destack_workspace::RuntimeOptions;

/// Stable identifier for one world-managed agent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct AgentId(pub u64);

/// Primary agent lane for executing Destack programs.
pub struct Agent {
    /// Monotonic world-local agent identity.
    pub(crate) id: AgentId,
    /// Agent name used for identity selection and diagnostics.
    pub(crate) name: String,
    /// Runtime owner identifier in world topology.
    pub(crate) runtime_id: RuntimeId,
    /// Immutable process arguments for platform bindings.
    pub(crate) platform_args: Arc<[String]>,
    /// Immutable runtime options.
    pub(crate) options: RuntimeOptions,

    /// External resource table and finalizers.
    pub(crate) resources: ResourceTable,
    /// Agent hooks and effect state.
    pub(crate) hooks: Arc<Hooks>,
    /// Agent-level finalizer registry for module services.
    pub(crate) finalizers: RuntimeFinalizers,
    /// Agent-owned platform state store.
    pub(crate) platform_state: PlatformState,
    /// Diagnostics storage for runtime errors and warning events.
    pub(crate) diagnostics: Arc<DiagnosticStore>,
    /// Coordinator-owned drop accounting for standalone agent flows.
    pub(crate) drop_counts: DropCounts,
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
            .field("name", &self.name)
            .field("runtime_id", &self.runtime_id)
            .field("platform_args", &self.platform_args)
            .field("options", &self.options)
            .field("resources", &self.resources)
            .field("hooks", &self.hooks)
            .field("finalizers", &self.finalizers)
            .field("platform_state", &self.platform_state)
            .field("diagnostics", &self.diagnostics)
            .field("bindings", &self.bindings)
            .field("heap", &self.heap)
            .field("event_loop", &self.event_loop)
            .finish()
    }
}

impl Agent {
    /// Return the canonical event-loop execution context identifier for one agent.
    fn event_loop_execution_context_id(
        runtime_id: RuntimeId,
        agent_id: AgentId,
    ) -> ExecutionContextId {
        let mut bytes = [0u8; 16];
        bytes[..8].copy_from_slice(&runtime_id.0.to_le_bytes());
        bytes[8..].copy_from_slice(&agent_id.0.to_le_bytes());

        ExecutionContextId(fnv1a_64(&bytes))
    }

    /// Create one agent with explicit runtime options in one shared world.
    pub fn new_in_world(
        platform_args: impl Into<Arc<[String]>>,
        options: &RuntimeOptions,
        world: &World,
    ) -> RuntimeResult<Self> {
        let platform_args = platform_args.into();
        let (runtime_id, agent_id, _runtime_name, agent_name) =
            Self::register_runtime(world, options)?;

        Self::assemble(
            platform_args,
            options,
            world,
            runtime_id,
            agent_id,
            agent_name,
        )
    }

    /// Create one agent with explicit runtime options in one existing runtime.
    pub(crate) fn new_in_runtime(
        platform_args: impl Into<Arc<[String]>>,
        options: &RuntimeOptions,
        world: &World,
        runtime_id: RuntimeId,
    ) -> RuntimeResult<Self> {
        let platform_args = platform_args.into();
        let (agent_id, agent_name) = Self::register_agent(world, options, runtime_id)?;

        Self::assemble(
            platform_args,
            options,
            world,
            runtime_id,
            agent_id,
            agent_name,
        )
    }

    /// Assemble one agent from registered world topology metadata.
    fn assemble(
        platform_args: Arc<[String]>,
        options: &RuntimeOptions,
        world: &World,
        runtime_id: RuntimeId,
        agent_id: AgentId,
        agent_name: String,
    ) -> RuntimeResult<Self> {
        // hooks and resources
        let hooks = Arc::new(Hooks::new(runtime_id, agent_id, world.replay().mode()));
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

        let execution_context_id = Self::event_loop_execution_context_id(runtime_id, agent_id);
        let _ = event_loop.initialize_execution_context(execution_context_id);

        // agent state
        Ok(Self {
            id: agent_id,
            runtime_id,
            name: agent_name,
            platform_args,
            options: options.clone(),
            resources,
            hooks,
            finalizers: RuntimeFinalizers::default(),
            platform_state: PlatformState::default(),
            diagnostics: Arc::new(DiagnosticStore::from_options(&options.diagnostic)),
            drop_counts: DropCounts::default(),
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

    /// Return immutable process arguments exposed to platform bindings.
    pub fn platform_args(&self) -> &[String] {
        self.platform_args.as_ref()
    }

    /// Return this agent name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Register one new runtime and one primary agent in one world.
    fn register_runtime(
        world: &World,
        options: &RuntimeOptions,
    ) -> RuntimeResult<(RuntimeId, AgentId, String, String)> {
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
        let _ = world.apply_control_command(WorldCommand::CreateRuntime {
            runtime_id,
            runtime_name: runtime_name.clone(),
            runtime_labels,
            primary_agent_id: agent_id,
            primary_agent_name: agent_name.clone(),
            primary_agent_labels: agent_labels,
        })?;

        Ok((runtime_id, agent_id, runtime_name, agent_name))
    }

    /// Register one agent in one existing runtime.
    fn register_agent(
        world: &World,
        options: &RuntimeOptions,
        runtime_id: RuntimeId,
    ) -> RuntimeResult<(AgentId, String)> {
        // agent selector metadata
        let agent_id = world.allocate_agent_id();
        let agent_name = options
            .primary_agent
            .name
            .clone()
            .unwrap_or_else(|| format!("agent-{}", agent_id.0));
        let agent_labels = options.primary_agent.labels.clone();

        // register one agent in one existing runtime
        let _ = world.apply_control_command(WorldCommand::CreateAgent {
            runtime_id,
            agent_id,
            agent_name: agent_name.clone(),
            agent_labels,
        })?;

        Ok((agent_id, agent_name))
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

    /// Return whether one event watch is registered for the given poller token.
    pub fn watches_event(&self, token: PollerToken) -> bool {
        self.event_loop.watches_event(token)
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

    /// Return whether one host semantic watch is registered for the given kind.
    pub fn watches_host_event(&self, kind: HostEventKind) -> bool {
        self.event_loop.watches_host_event(kind)
    }

    /// Return drop accounting observed by this agent event loop.
    pub fn drop_counts(&self) -> DropCounts {
        let event_loop_drops = self.event_loop.drop_counts();

        DropCounts {
            queue_pressure: self
                .drop_counts
                .queue_pressure
                .saturating_add(event_loop_drops.queue_pressure),
            unmatched_ingress: self
                .drop_counts
                .unmatched_ingress
                .saturating_add(event_loop_drops.unmatched_ingress),
            unwatched_dispatch: self
                .drop_counts
                .unwatched_dispatch
                .saturating_add(event_loop_drops.unwatched_dispatch),
        }
    }

    /// Record one coordinator-owned drop in standalone agent flows.
    pub(crate) fn record_drop(&mut self, reason: DropReason, count: u64) {
        self.drop_counts.record(reason, count);
    }
}
