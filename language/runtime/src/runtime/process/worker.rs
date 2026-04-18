use destack_core::{Capture, CaptureMode, fnv1a_64};
use destack_heap as heap;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use super::{
    BindingCallContext, RuntimeScheduledCallbackControl, RuntimeScheduledCallbackHandle,
    RuntimeScheduledCallbackRegistry,
};
use crate::diagnostic::{DiagnosticSnapshot, DiagnosticStore, RuntimeError, RuntimeResult};
use crate::host::HostEventKind;
use crate::platform::resource::{ResourceRebinders, ResourceTableSnapshot};
use crate::platform::{ResourceId, ResourceTable};
use crate::runtime::bindings::{BindingPolicy, BindingRegistry};
use crate::runtime::capability::resolve_capability_profile;
use crate::runtime::engine::{Engine, EngineImage, EngineLayout, LiveContinuation};
use crate::runtime::memory::{Gc, RootSet, RootVisitor, resolve_heap_options};
use crate::runtime::policy::HookSnapshot;
use crate::runtime::poller::PollerToken;
use crate::runtime::scheduler::{EventLoop, EventLoopSnapshot, EventLoopWatch};
use crate::runtime::world::{RuntimeId, WorldRef};
use crate::runtime::{
    DropCounts, DropReason, ExecutionContextId, Hooks, PlatformState, PlatformStateImage,
    RuntimeFinalizers, RuntimeFinalizersImage,
};
use destack_vm as vm;
use destack_workspace::{ExecutionMode, RuntimeOptions};

/// Stable identifier for one world-managed worker.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct WorkerId(pub u64);

/// Primary worker lane for executing Destack programs.
pub struct Worker {
    /// Monotonic world-local worker identity.
    pub(crate) id: WorkerId,
    /// Worker name used for identity selection and diagnostics.
    pub(crate) name: String,
    /// Runtime owner identifier in world topology.
    pub(crate) runtime_id: RuntimeId,
    /// Immutable process arguments for platform bindings.
    pub(crate) platform_args: Arc<[String]>,
    /// Immutable runtime options.
    pub(crate) options: Arc<RuntimeOptions>,

    /// External resource table and finalizers.
    pub(crate) resources: ResourceTable,
    /// Worker hooks and effect state.
    pub(crate) hooks: Arc<Hooks>,
    /// Worker-level finalizer registry for module services.
    pub(crate) finalizers: RuntimeFinalizers,
    /// Worker-local runtime scheduled callbacks.
    pub(crate) runtime_callbacks: RuntimeScheduledCallbackRegistry,
    /// Worker-owned platform state store.
    pub(crate) platform_state: PlatformState,
    /// Diagnostics storage for runtime errors and warning events.
    pub(crate) diagnostics: Arc<DiagnosticStore>,
    /// Coordinator-owned drop accounting for standalone worker flows.
    pub(crate) drop_counts: DropCounts,
    /// External binding registry and policy enforcement.
    pub(crate) bindings: BindingRegistry,

    /// Runtime GC controller for this worker heap.
    pub(crate) gc: Gc,
    /// Root visitors contributing GC roots.
    pub(crate) root_visitors: Vec<Box<dyn RootVisitor>>,
    /// Authoritative worker heap.
    pub(crate) heap: heap::Heap,
    /// Worker-owned execution engine.
    pub(crate) engine: Box<dyn Engine>,
    /// Event loop for tasks, microtasks, and timers.
    pub(crate) event_loop: Box<EventLoop>,
}

/// Materialized worker metadata captured in one world image.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkerImage {
    /// Worker options captured for reconstruction.
    pub options: WorkerOptionsImage,
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
    /// Captured worker-owned execution image.
    pub engine_image: EngineImage,
}

/// Captured worker options with one shared-runtime fast path.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum WorkerOptionsImage {
    /// The worker uses the runtime-shared captured options.
    Shared,
    /// The worker carries one explicit options payload.
    Explicit(Arc<RuntimeOptions>),
}

impl WorkerImage {
    /// Return whether the captured worker still has pending event-loop work.
    pub fn has_pending_work(&self) -> bool {
        self.event_loop.has_pending_work()
    }
}

impl WorkerOptionsImage {
    /// Capture one explicit worker options payload.
    pub fn explicit(options: RuntimeOptions) -> Self {
        Self::explicit_arc(Arc::new(options))
    }

    /// Capture one explicit shared worker options payload.
    pub fn explicit_arc(options: Arc<RuntimeOptions>) -> Self {
        Self::Explicit(options)
    }

    /// Return the explicit options payload when present.
    pub fn explicit_options(&self) -> Option<&Arc<RuntimeOptions>> {
        match self {
            Self::Shared => None,
            Self::Explicit(options) => Some(options),
        }
    }

    /// Resolve one captured options payload against one runtime-shared base.
    pub fn resolve(
        &self,
        shared_options: Option<&Arc<RuntimeOptions>>,
    ) -> RuntimeResult<Arc<RuntimeOptions>> {
        match self {
            Self::Shared => {
                let Some(shared_options) = shared_options else {
                    return Err(RuntimeError::Internal {
                        message: "shared worker image options require one runtime image base"
                            .to_string(),
                    }
                    .boxed());
                };

                Ok(shared_options.clone())
            }
            Self::Explicit(options) => Ok(options.clone()),
        }
    }
}

impl std::fmt::Debug for Worker {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Worker")
            .field("worker_id", &self.id)
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
            .field("engine", &"<worker execution engine>")
            .field("event_loop", &self.event_loop)
            .finish()
    }
}

impl Worker {
    /// Return the canonical event-loop execution context identifier for one worker.
    fn event_loop_execution_context_id(
        runtime_id: RuntimeId,
        worker_id: WorkerId,
    ) -> ExecutionContextId {
        let mut bytes = [0u8; 16];
        bytes[..8].copy_from_slice(&runtime_id.0.to_le_bytes());
        bytes[8..].copy_from_slice(&worker_id.0.to_le_bytes());

        ExecutionContextId(fnv1a_64(&bytes))
    }

    /// Create one worker with explicit runtime options in one shared world.
    pub(crate) fn new_in_world(
        platform_args: impl Into<Arc<[String]>>,
        options: &RuntimeOptions,
        world: &WorldRef,
        engine: impl Engine + EngineLayout + 'static,
    ) -> RuntimeResult<Self> {
        let platform_args = platform_args.into();
        let (runtime_id, worker_id, _runtime_name, worker_name) =
            Self::register_runtime(world, options)?;

        Self::assemble(
            platform_args,
            options,
            world,
            runtime_id,
            worker_id,
            worker_name,
            engine,
        )
    }

    /// Create one worker with explicit runtime options in one existing runtime.
    pub(crate) fn new_in_runtime(
        platform_args: impl Into<Arc<[String]>>,
        options: &RuntimeOptions,
        world: &WorldRef,
        runtime_id: RuntimeId,
        engine: impl Engine + EngineLayout + 'static,
    ) -> RuntimeResult<Self> {
        let platform_args = platform_args.into();
        let (worker_id, worker_name) = Self::register_worker(world, options, runtime_id)?;

        Self::assemble(
            platform_args,
            options,
            world,
            runtime_id,
            worker_id,
            worker_name,
            engine,
        )
    }

    /// Assemble one worker from registered world topology metadata.
    fn assemble(
        platform_args: Arc<[String]>,
        options: &RuntimeOptions,
        world: &WorldRef,
        runtime_id: RuntimeId,
        worker_id: WorkerId,
        worker_name: String,
        engine: impl Engine + EngineLayout + 'static,
    ) -> RuntimeResult<Self> {
        // hooks and resources
        let hooks = Arc::new(Hooks::new(runtime_id, worker_id, world.trace().mode()));
        let resources = ResourceTable::default();
        resources.set_hooks(hooks.clone());

        // bindings, heap, event loop
        let mut bindings = BindingRegistry::new();
        bindings.set_policy(BindingPolicy::new(world.trace().mode()));
        bindings.install_native_defaults();
        bindings.apply_runtime_defaults(options);
        Self::apply_capability_profile(&mut bindings, options)?;

        // heap options follow the engine storage metadata
        let mut gc = Gc::default();
        gc.configure(options.heap.clone());
        let managed_reference_bytes = engine.managed_reference_bytes();
        let heap_options = resolve_heap_options(&options.heap, managed_reference_bytes)?;
        let heap = heap::Heap::with_limits_and_options(heap_options.limits, heap_options.options)
            .map_err(Box::<RuntimeError>::from)?;

        let mut event_loop = Box::new(EventLoop::default());
        event_loop.configure(options.scheduler.clone())?;

        let execution_context_id = Self::event_loop_execution_context_id(runtime_id, worker_id);
        event_loop.initialize_execution_context(execution_context_id);

        // worker state
        Ok(Self {
            id: worker_id,
            runtime_id,
            name: worker_name,
            platform_args,
            options: Arc::new(options.clone()),
            resources,
            hooks,
            finalizers: RuntimeFinalizers::default(),
            runtime_callbacks: RuntimeScheduledCallbackRegistry::default(),
            platform_state: PlatformState::default(),
            diagnostics: Arc::new(DiagnosticStore::from_options(&options.diagnostic)),
            drop_counts: DropCounts::default(),
            bindings,
            gc,
            root_visitors: Vec::new(),
            heap,
            engine: Box::new(engine),
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

    /// Return this worker identifier.
    pub fn worker_id(&self) -> WorkerId {
        self.id
    }

    /// Return this worker name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Return the owning runtime identifier.
    pub fn runtime_id(&self) -> RuntimeId {
        self.runtime_id
    }

    /// Return whether this worker still has pending scheduler work.
    pub fn has_pending_work(&self) -> bool {
        self.event_loop.has_pending_work()
    }

    /// Return the number of stored resources for this worker.
    pub fn resource_count(&self) -> usize {
        self.resources.len()
    }

    /// Register one new runtime and one primary worker in one world.
    fn register_runtime(
        world: &WorldRef,
        options: &RuntimeOptions,
    ) -> RuntimeResult<(RuntimeId, WorkerId, String, String)> {
        // runtime selector metadata
        let runtime_name = options
            .name
            .clone()
            .unwrap_or_else(|| "runtime".to_string());
        let runtime_labels = options.labels.clone();

        // worker selector metadata
        let runtime_id = world.allocate_runtime_id();
        let worker_id = world.allocate_worker_id();
        let worker_name = options
            .primary_worker
            .name
            .clone()
            .unwrap_or_else(|| format!("worker-{}", worker_id.0));
        let worker_labels = options.primary_worker.labels.clone();

        // register runtime and worker directly in world topology
        world.register_runtime_topology(
            runtime_id,
            runtime_name.clone(),
            runtime_labels,
            worker_id,
            worker_name.clone(),
            worker_labels,
        )?;

        Ok((runtime_id, worker_id, runtime_name, worker_name))
    }

    /// Register one worker in one existing runtime.
    fn register_worker(
        world: &WorldRef,
        options: &RuntimeOptions,
        runtime_id: RuntimeId,
    ) -> RuntimeResult<(WorkerId, String)> {
        // worker selector metadata
        let worker_id = world.allocate_worker_id();
        let worker_name = options
            .primary_worker
            .name
            .clone()
            .unwrap_or_else(|| format!("worker-{}", worker_id.0));
        let worker_labels = options.primary_worker.labels.clone();

        // register one worker in one existing runtime
        world.register_worker_topology(runtime_id, worker_id, worker_name.clone(), worker_labels)?;

        Ok((worker_id, worker_name))
    }

    /// Register one timer watch.
    pub fn watch_timer(
        &mut self,
        handle: ResourceId,
        runnable: LiveContinuation,
        resume_value: heap::Value,
        priority: u8,
    ) -> RuntimeResult<()> {
        self.event_loop.watch_timer(
            handle,
            runnable,
            resume_value,
            priority,
            self.engine.as_mut(),
        )
    }

    /// Remove the timer watch registered for one timer handle.
    pub fn unwatch_timer(&mut self, handle: ResourceId) -> Option<EventLoopWatch> {
        self.event_loop.unwatch_timer(handle)
    }

    /// Schedule one runtime callback on the owning event loop thread.
    pub(crate) fn schedule_runtime_callback(
        &self,
        binding: &BindingCallContext,
        delay_ns: u64,
        interval_ns: Option<u64>,
        callback: impl FnMut(&BindingCallContext) -> RuntimeResult<RuntimeScheduledCallbackControl>
        + Send
        + 'static,
    ) -> RuntimeResult<RuntimeScheduledCallbackHandle> {
        self.runtime_callbacks
            .schedule(binding, delay_ns, interval_ns, callback)
    }

    /// Cancel one scheduled runtime callback.
    pub(crate) fn cancel_runtime_callback(
        &self,
        binding: &BindingCallContext,
        handle: RuntimeScheduledCallbackHandle,
    ) -> RuntimeResult<()> {
        self.runtime_callbacks.cancel(binding, handle)
    }

    /// Service due runtime callbacks on the owning event loop thread.
    pub(crate) fn service_runtime_callbacks(
        &self,
        binding: &BindingCallContext,
    ) -> RuntimeResult<()> {
        // skip the timer scan when no runtime callback is registered
        if !self.runtime_callbacks.has_active_callbacks() {
            return Ok(());
        }

        // collect only runtime owned timers from the shared ready set
        let wall_now = binding.world().wall();
        let mono_now = binding.world().mono();
        let due_timers =
            binding
                .event_loop()
                .take_due_timers_matching(wall_now, mono_now, |handle| {
                    handle.internal_id().is_some_and(|handle| {
                        self.runtime_callbacks
                            .contains(RuntimeScheduledCallbackHandle::from_internal_id(handle))
                    })
                })?;

        // run only runtime owned timer callbacks in this blocked wait path
        for timer in due_timers {
            let Some(handle) = timer.handle.internal_id() else {
                continue;
            };

            self.runtime_callbacks.service_due_callback(
                binding,
                RuntimeScheduledCallbackHandle::from_internal_id(handle),
            )?;
        }

        Ok(())
    }

    /// Register one event watch.
    pub fn watch_event(
        &mut self,
        token: PollerToken,
        runnable: LiveContinuation,
        resume_value: heap::Value,
        priority: u8,
    ) -> RuntimeResult<()> {
        self.event_loop.watch_event(
            token,
            runnable,
            resume_value,
            priority,
            self.engine.as_mut(),
        )
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
        runnable: LiveContinuation,
        resume_value: heap::Value,
        priority: u8,
    ) -> RuntimeResult<()> {
        self.event_loop.watch_host_event(
            kind,
            runnable,
            resume_value,
            priority,
            self.engine.as_mut(),
        )
    }

    /// Remove the host event watch registered for one host event kind.
    pub fn unwatch_host_event(&mut self, kind: HostEventKind) -> Option<EventLoopWatch> {
        self.event_loop.unwatch_host_event(kind)
    }

    /// Return whether one host semantic watch is registered for the given kind.
    pub fn watches_host_event(&self, kind: HostEventKind) -> bool {
        self.event_loop.watches_host_event(kind)
    }

    /// Return drop accounting observed by this worker event loop.
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

    /// Record one coordinator-owned drop in standalone worker flows.
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
        self.heap.gc_state().clone()
    }

    /// Return the exact retained heap usage for this worker.
    pub fn heap_usage(&self) -> RuntimeResult<heap::HeapUsage> {
        self.heap.usage().map_err(Box::<RuntimeError>::from)
    }

    /// Check whether the heap should trigger a GC cycle.
    pub fn should_collect(&mut self) -> bool {
        // read the current heap size
        let managed_allocated_bytes = self.heap.managed_allocated_bytes();

        // evaluate runtime gc pacing policy
        self.gc.should_collect(managed_allocated_bytes)
    }

    /// Run garbage collection using the current root set.
    pub fn collect(&mut self) -> RuntimeResult<heap::GcStats> {
        // gather managed references from root visitors
        let roots = self.collect_roots();
        let references = roots.handles();

        // run young collection first under ordinary heap pressure
        let stats = self.heap.collect_young(references.iter().copied())?;

        // escalate to one full cycle if mature pressure is still high
        let stats = if self.gc.should_collect(self.heap.managed_allocated_bytes()) {
            self.heap
                .collect(references.iter().copied())
                .map_err(Box::<RuntimeError>::from)?
        } else {
            stats
        };

        // update runtime gc pacing from cycle results
        self.gc.on_cycle_complete(stats);

        Ok(stats)
    }

    /// Check configured retained-heap limits for this worker.
    pub fn check_heap_limits(&self) -> RuntimeResult<()> {
        self.heap.check_limits().map_err(Into::into)
    }

    /// Capture one materialized worker image.
    pub(crate) fn capture_image(&mut self, mode: CaptureMode) -> RuntimeResult<WorkerImage> {
        // runtime callback barrier
        if self.runtime_callbacks.has_active_callbacks() {
            return Err(self.runtime_callbacks.capture_barrier_error(mode));
        }

        // local scheduler and external state
        let event_loop = self.event_loop.capture_image(mode, self.engine.as_mut())?;
        let resources = self.resources.capture_image(mode, ())?;
        let diagnostics = self.diagnostics.snapshot()?;
        let hooks = self.hooks.snapshot()?;

        // runtime-owned service state
        let platform_state = self.platform_state.capture_image(mode, ())?;
        let finalizers = self.finalizers.capture_image(mode, ())?;

        // capture the worker-local image payload
        Ok(WorkerImage {
            options: WorkerOptionsImage::explicit_arc(self.options.clone()),
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

    /// Fork one live worker when all branch-sensitive state is quiescent.
    pub(crate) fn try_fork(
        &mut self,
        execution_mode: ExecutionMode,
    ) -> RuntimeResult<Option<Self>> {
        // runtime callbacks
        if self.runtime_callbacks.has_active_callbacks() {
            return Ok(None);
        }

        // hook and diagnostics state
        let hooks = match self.hooks.try_fork()? {
            Some(hooks) => Arc::new(hooks),
            None => return Ok(None),
        };
        let diagnostics = match self.diagnostics.try_fork()? {
            Some(diagnostics) => Arc::new(diagnostics),
            None => return Ok(None),
        };

        // resources, finalizers, and event loop
        let resources = match self.resources.try_fork(hooks.clone()) {
            Some(resources) => resources,
            None => return Ok(None),
        };
        let finalizers = match self.finalizers.try_fork() {
            Some(finalizers) => finalizers,
            None => return Ok(None),
        };

        // bindings and heap
        let mut bindings = BindingRegistry::new();
        bindings.set_policy(BindingPolicy::new(execution_mode));
        bindings.install_native_defaults();
        bindings.apply_runtime_defaults(&self.options);
        Self::apply_capability_profile(&mut bindings, &self.options)?;

        let mut heap = self.heap.fork()?;
        let mut engine = self.engine.fork(&mut heap)?;
        let event_loop = Box::new(
            self.event_loop
                .fork(self.engine.as_mut(), engine.as_mut())?,
        );

        // platform and gc state
        let platform_state = self.platform_state.fork()?;
        let gc = self.gc.clone();

        Ok(Some(Self {
            id: self.id,
            name: self.name.clone(),
            runtime_id: self.runtime_id,
            platform_args: self.platform_args.clone(),
            options: self.options.clone(),
            resources,
            hooks,
            finalizers,
            runtime_callbacks: RuntimeScheduledCallbackRegistry::default(),
            platform_state,
            diagnostics,
            drop_counts: self.drop_counts,
            bindings,
            gc,
            root_visitors: Vec::new(),
            heap,
            engine,
            event_loop,
        }))
    }

    /// Restore one worker from one materialized image.
    pub(crate) fn from_image(
        world: &WorldRef,
        runtime_id: RuntimeId,
        worker_id: WorkerId,
        worker_name: String,
        platform_args: Arc<[String]>,
        image: &WorkerImage,
        shared_options: Option<&Arc<RuntimeOptions>>,
        rebind_context: Option<&ResourceRebinders>,
    ) -> RuntimeResult<Self> {
        // resolve the captured options first
        let options = image.options.resolve(shared_options)?;

        // hooks and resources
        let hooks = Arc::new(Hooks::new(runtime_id, worker_id, world.trace().mode()));
        let resources = ResourceTable::default();
        resources.set_hooks(hooks.clone());

        // bindings
        let mut bindings = BindingRegistry::new();
        bindings.set_policy(BindingPolicy::new(world.trace().mode()));
        bindings.install_native_defaults();
        bindings.apply_runtime_defaults(&options);
        Self::apply_capability_profile(&mut bindings, &options)?;

        // diagnostics and event loop
        let diagnostics = Arc::new(DiagnosticStore::from_options(&options.diagnostic));
        let mut event_loop = Box::new(EventLoop::default());
        event_loop.configure(options.scheduler.clone())?;

        // heap and engine
        let heap_limits = heap::HeapLimits {
            max_bytes: options.heap.max_bytes,
            managed: heap::ManagedLimits {
                max_bytes: options.heap.max_managed_bytes,
            },
            raw: heap::RawLimits {
                max_bytes: options.heap.max_raw_bytes,
            },
        };
        let mut heap = heap::Heap::from_image_with_limits(&image.heap_image, heap_limits)
            .map_err(Box::<RuntimeError>::from)?;
        let mut gc = Gc::default();
        gc.configure(options.heap.clone());
        // rebuild the engine from the materialized worker image
        let mut engine: Box<dyn Engine> = match &image.engine_image {
            EngineImage::Vm(image) => {
                let isolate = vm::Isolate::new(image.clone()).map_err(Box::<RuntimeError>::from)?;

                Box::new(isolate)
            }
        };

        // restore backend execution state over the restored heap
        engine.restore_image(&mut heap, &image.engine_image)?;

        // restore local state on fresh containers
        let execution_context_id = Self::event_loop_execution_context_id(runtime_id, worker_id);
        event_loop.initialize_execution_context(execution_context_id);
        event_loop.restore_snapshot(&image.event_loop, engine.as_mut())?;
        diagnostics.restore_snapshot(&image.diagnostics)?;
        hooks.restore_snapshot(&image.hooks)?;
        resources.restore_snapshot(&image.resources, rebind_context)?;

        Ok(Self {
            id: worker_id,
            name: worker_name,
            runtime_id,
            platform_args,
            options,
            resources,
            hooks,
            finalizers: {
                let mut finalizers = RuntimeFinalizers::default();
                finalizers.restore_image(&image.finalizers, ())?;
                finalizers
            },
            runtime_callbacks: RuntimeScheduledCallbackRegistry::default(),
            platform_state: {
                let mut platform_state = PlatformState::default();
                platform_state.restore_image(&image.platform_state, ())?;
                platform_state
            },
            diagnostics,
            drop_counts: DropCounts::default(),
            bindings,
            gc,
            root_visitors: Vec::new(),
            heap,
            engine,
            event_loop,
        })
    }
}
