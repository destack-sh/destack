use destack_core::{Capture, CaptureMode, fnv1a_64};
use destack_heap as heap;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::diagnostic::{DiagnosticSnapshot, DiagnosticStore, RuntimeError, RuntimeResult};
use crate::host::HostEventKind;
use crate::platform::resource::ResourceTableSnapshot;
use crate::platform::state::{PlatformState, PlatformStateImage};
use crate::platform::{ResourceId, ResourceTable};
use crate::runtime::bindings::{BindingPolicy, BindingRegistry};
use crate::runtime::capability::resolve_capability_profile;
use crate::runtime::engine::{Engine, EngineContinuation, EngineImage};
use crate::runtime::memory::{Gc, RootSet, RootVisitor};
use crate::runtime::policy::HookSnapshot;
use crate::runtime::poller::PollerToken;
use crate::runtime::scheduler::{EventLoop, EventLoopSnapshot, EventLoopWatch};
use crate::runtime::world::{RebindContext, RuntimeId, World};
use crate::runtime::{
    DropCounts, DropReason, ExecutionContextId, Hooks, RuntimeFinalizers, RuntimeFinalizersImage,
};
use destack_vm as vm;
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

    /// Runtime GC controller for this agent heap.
    pub(crate) gc: Gc,
    /// Root visitors contributing GC roots.
    pub(crate) root_visitors: Vec<Box<dyn RootVisitor>>,
    /// Authoritative agent heap.
    pub(crate) heap: heap::Heap,
    /// Agent-owned execution engine.
    pub(crate) engine: Box<dyn Engine>,
    /// Event loop for tasks, microtasks, and timers.
    pub(crate) event_loop: Box<EventLoop>,
}

/// Materialized agent metadata captured in one world image.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentImage {
    /// Agent identifier in the world.
    pub agent_id: AgentId,
    /// Owning runtime identifier.
    pub runtime_id: RuntimeId,
    /// Agent display name.
    pub name: String,
    /// Agent options captured for reconstruction.
    pub options: RuntimeOptions,
    /// Agent drop counts.
    pub drop_counts: DropCounts,
    /// Captured diagnostics store state.
    pub diagnostics: DiagnosticSnapshot,
    /// Captured hook state.
    pub hooks: HookSnapshot,
    /// Captured resource table state.
    pub resources: ResourceTableSnapshot,
    /// Captured finalizer lifecycle state.
    pub finalizers: RuntimeFinalizersImage,
    /// Captured platform-state lifecycle state.
    pub platform_state: PlatformStateImage,
    /// Captured event-loop state.
    pub event_loop: EventLoopSnapshot,
    /// Captured authoritative heap image.
    pub heap_image: heap::HeapImage,
    /// Captured agent-owned execution image.
    pub engine_image: EngineImage,
}

impl AgentImage {
    /// Return whether the captured agent still has pending event-loop work.
    pub fn has_pending_work(&self) -> bool {
        !self.event_loop.tasks.is_empty()
            || !self.event_loop.microtasks.is_empty()
            || !self.event_loop.events.is_empty()
            || !self.event_loop.host_events.is_empty()
            || !self.event_loop.ready_timers.is_empty()
            || !self.event_loop.timers.is_empty()
            || !self.event_loop.timer_watches.is_empty()
            || !self.event_loop.poller_event_watches.is_empty()
            || !self.event_loop.host_event_watches.is_empty()
    }
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
            .field("gc", &self.gc)
            .field("root_visitors", &self.root_visitors.len())
            .field("heap", &self.heap)
            .field("engine", &"<agent execution engine>")
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
        engine: Box<dyn Engine>,
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
            engine,
        )
    }

    /// Create one agent with explicit runtime options in one existing runtime.
    pub(crate) fn new_in_runtime(
        platform_args: impl Into<Arc<[String]>>,
        options: &RuntimeOptions,
        world: &World,
        runtime_id: RuntimeId,
        engine: Box<dyn Engine>,
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
            engine,
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
        engine: Box<dyn Engine>,
    ) -> RuntimeResult<Self> {
        // hooks and resources
        let hooks = Arc::new(Hooks::new(runtime_id, agent_id, world.trace().mode()));
        let resources = ResourceTable::default();
        resources.set_hooks(hooks.clone());

        // bindings, heap, event loop
        let mut bindings = BindingRegistry::new();
        bindings.set_policy(BindingPolicy::new(world.trace().mode()));
        bindings.install_native_defaults();
        bindings.apply_runtime_defaults(options);
        Self::apply_capability_profile(&mut bindings, options)?;

        let mut gc = Gc::default();
        gc.configure(options.heap.clone());
        let heap = heap::Heap::with_limits_and_large_span_thresholds(
            heap::HeapLimits {
                max_bytes: options.heap.max_bytes,
                max_managed_bytes: options.heap.max_managed_bytes,
                max_raw_bytes: options.heap.max_raw_bytes,
            },
            options.heap.managed_large_span_values,
            options.heap.raw_large_span_bytes,
        );

        let mut event_loop = Box::new(EventLoop::default());
        event_loop.configure(options.scheduler.clone())?;

        let execution_context_id = Self::event_loop_execution_context_id(runtime_id, agent_id);
        event_loop.initialize_execution_context(execution_context_id);

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
            gc,
            root_visitors: Vec::new(),
            heap,
            engine,
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
            RuntimeError::CapabilityProfileInvalid {
                profile: capability_profile.to_string(),
                detail: message,
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

    /// Return this agent identifier.
    pub fn agent_id(&self) -> AgentId {
        self.id
    }

    /// Return this agent name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Return the owning runtime identifier.
    pub fn runtime_id(&self) -> RuntimeId {
        self.runtime_id
    }

    /// Return whether this agent still has pending scheduler work.
    pub fn has_pending_work(&self) -> bool {
        self.event_loop.has_pending_work()
    }

    /// Return the number of stored resources for this agent.
    pub fn resource_count(&self) -> usize {
        self.resources.len()
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

        // register runtime and agent directly in world topology
        world.register_runtime_topology(
            runtime_id,
            runtime_name.clone(),
            runtime_labels,
            agent_id,
            agent_name.clone(),
            agent_labels,
        )?;

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
        world.register_agent_topology(runtime_id, agent_id, agent_name.clone(), agent_labels)?;

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

    /// Register a root visitor for GC coordination.
    pub fn register_root_visitor(&mut self, visitor: Box<dyn RootVisitor>) {
        self.root_visitors.push(visitor);
    }

    /// Collect roots from all registered providers.
    pub fn collect_roots(&self) -> RootSet {
        let mut roots = RootSet::new();
        for visitor in &self.root_visitors {
            visitor.collect_roots(&mut roots);
        }

        roots
    }

    /// Return a snapshot of the GC state.
    pub fn gc_state(&self) -> heap::GcState {
        self.heap.managed_gc_state().clone()
    }

    /// Return the exact retained heap usage for this agent.
    pub fn heap_usage(&self) -> heap::HeapUsage {
        self.heap.usage()
    }

    /// Check whether the heap should trigger a GC cycle.
    pub fn should_collect(&mut self) -> bool {
        // read the current heap size
        let heap_bytes = self.heap.managed_heap_bytes();

        // evaluate runtime gc pacing policy
        self.gc.should_collect(heap_bytes)
    }

    /// Run garbage collection using the current root set.
    pub fn collect(&mut self) -> heap::GcStats {
        // gather managed references from root visitors
        let roots = self.collect_roots();
        let handles = roots.managed_references();

        // run collection
        let stats = self.heap.collect_managed_handles(handles.iter().copied());

        // update runtime gc pacing from cycle results
        self.gc.on_cycle_complete(stats);

        stats
    }

    /// Check configured retained-heap limits for this agent.
    pub fn check_heap_limits(&self) -> RuntimeResult<()> {
        self.heap.check_limits().map_err(Into::into)
    }

    /// Capture one materialized agent image.
    pub(crate) fn capture_image(&mut self, mode: CaptureMode) -> RuntimeResult<AgentImage> {
        // local scheduler and external state
        let event_loop = self.event_loop.capture_image(mode, self.engine.as_mut())?;
        let resources = self.resources.capture_image(mode, ())?;
        let diagnostics = self.diagnostics.snapshot()?;
        let hooks = self.hooks.snapshot()?;

        // runtime-owned service state
        let platform_state = self.platform_state.capture_image(mode, ())?;
        let finalizers = self.finalizers.capture_image(mode, ())?;

        // capture the agent-local image payload
        Ok(AgentImage {
            agent_id: self.id,
            runtime_id: self.runtime_id,
            name: self.name.clone(),
            options: self.options.clone(),
            drop_counts: self.drop_counts,
            diagnostics,
            hooks,
            resources,
            finalizers,
            platform_state,
            event_loop,
            heap_image: self.heap.image().map_err(|error| {
                RuntimeError::CaptureBarrier {
                    component: "runtime.heap".to_string(),
                    mode: format!("{mode:?}"),
                    detail: error.to_string(),
                }
                .boxed()
            })?,
            engine_image: self.engine.image()?,
        })
    }

    /// Restore one agent from one materialized image.
    pub(crate) fn from_image(
        world: &World,
        platform_args: Arc<[String]>,
        image: &AgentImage,
        rebind_context: Option<&RebindContext>,
    ) -> RuntimeResult<Self> {
        // hooks and resources
        let hooks = Arc::new(Hooks::new(
            image.runtime_id,
            image.agent_id,
            world.trace().mode(),
        ));
        let resources = ResourceTable::default();
        resources.set_hooks(hooks.clone());

        // bindings
        let mut bindings = BindingRegistry::new();
        bindings.set_policy(BindingPolicy::new(world.trace().mode()));
        bindings.install_native_defaults();
        bindings.apply_runtime_defaults(&image.options);
        Self::apply_capability_profile(&mut bindings, &image.options)?;

        // diagnostics and event loop
        let diagnostics = Arc::new(DiagnosticStore::from_options(&image.options.diagnostic));
        let mut event_loop = Box::new(EventLoop::default());

        // heap and engine
        let mut heap = heap::Heap::from_image(&image.heap_image);
        heap.set_limits(heap::HeapLimits {
            max_bytes: image.options.heap.max_bytes,
            max_managed_bytes: image.options.heap.max_managed_bytes,
            max_raw_bytes: image.options.heap.max_raw_bytes,
        })?;
        let mut gc = Gc::default();
        gc.configure(image.options.heap.clone());
        // rebuild the engine from the materialized agent image
        let mut engine: Box<dyn Engine> = match &image.engine_image {
            EngineImage::Vm(image) => {
                let isolate = vm::Isolate::new(image.clone()).map_err(Box::<RuntimeError>::from)?;

                Box::new(isolate)
            }
        };

        // restore backend execution state over the restored heap
        engine.restore_image(&mut heap, &image.engine_image)?;

        // restore local state on fresh containers
        let execution_context_id =
            Self::event_loop_execution_context_id(image.runtime_id, image.agent_id);
        event_loop.initialize_execution_context(execution_context_id);
        event_loop.restore_snapshot(&image.event_loop, engine.as_mut())?;
        diagnostics.restore_snapshot(&image.diagnostics)?;
        hooks.restore_snapshot(&image.hooks)?;
        resources.restore_snapshot(
            &image.resources,
            rebind_context.map(RebindContext::resources),
        )?;

        Ok(Self {
            id: image.agent_id,
            name: image.name.clone(),
            runtime_id: image.runtime_id,
            platform_args,
            options: image.options.clone(),
            resources,
            hooks,
            finalizers: {
                let mut finalizers = RuntimeFinalizers::default();
                finalizers.restore_image(&image.finalizers, ())?;
                finalizers
            },
            platform_state: {
                let mut platform_state = PlatformState::default();
                platform_state.restore_image(&image.platform_state, ())?;
                platform_state
            },
            diagnostics,
            drop_counts: image.drop_counts,
            bindings,
            gc,
            root_visitors: Vec::new(),
            heap,
            engine,
            event_loop,
        })
    }
}
