use destack_core::{Capture, CaptureMode};
use destack_engine as engine;
use destack_heap as heap;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::Arc;

use crate::diagnostic::{DiagnosticSnapshot, DiagnosticStore, RuntimeError, RuntimeResult};
use crate::host::binding::{BindingAccess, BindingRegistry};
use crate::host::resource::{ResourceRebinders, ResourceTableSnapshot};
use crate::host::{HostEventKind, ResourceId, ResourceTable};
use crate::runtime::engine::{Continuation, Engine, Image, MemoryContext};
use crate::runtime::heap::{HeapHandle, HeapHandleTable, resolve_local_heap_options};
use crate::runtime::scheduler::{EventLoop, EventLoopSnapshot, Readiness, Waiter};
use crate::runtime::{RuntimeFinalizers, RuntimeFinalizersImage, ScenarioRunner, SharedHeap};
use crate::world::scenario::ScenarioRunnerSnapshot;
use crate::world::{Entity, EntityKind, RuntimeId, WorldState};
use destack_workspace::{Environment, ExecutionMode, RuntimeOptions};

/// Execution worker owned by one runtime.
pub struct Worker {
    /// Monotonic world-local worker identity.
    pub(crate) id: WorkerId,
    /// Runtime owner identifier in world topology.
    pub(crate) runtime_id: RuntimeId,
    /// Immutable ambient environment for host bindings.
    pub(crate) environment: Arc<Environment>,
    /// Immutable runtime options.
    pub(crate) options: Arc<RuntimeOptions>,

    /// External resource table and finalizers.
    pub(crate) resources: ResourceTable,
    /// Worker scenario runner.
    pub(crate) scenario: Arc<ScenarioRunner>,
    /// Worker-level finalizer registry for module services.
    pub(crate) finalizers: RuntimeFinalizers,
    /// Diagnostics storage for runtime errors and warning events.
    pub(crate) diagnostics: Arc<DiagnosticStore>,
    /// External binding registry and policy enforcement.
    pub(crate) bindings: BindingRegistry,
    /// Runtime-owned handles for host-retained local heap references.
    pub(crate) handles: HeapHandleTable,

    /// Shared GC worker queue handle.
    pub(crate) shared_gc_worker: heap::SharedGcWorker,
    /// Worker-local shared allocation cache.
    pub(crate) shared_cache: heap::SharedAllocationCache,
    /// Authoritative worker heap.
    pub(crate) heap: heap::Heap,
    /// Worker-owned static byte space.
    pub(crate) statics: engine::StaticSpace,
    /// Worker-owned execution engine.
    pub(crate) engine: Engine,
    /// HostEvent loop for tasks, microtasks, and timers.
    pub(crate) event_loop: Box<EventLoop>,
}

/// Stable identifier for one world-managed worker.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct WorkerId(pub u64);

/// Worker creation options.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct WorkerOptions {
    /// Worker name used for identity selection and diagnostics.
    pub name: Option<String>,
    /// Worker labels used for topology and policy selection.
    pub labels: BTreeMap<String, String>,
}

/// Materialized worker metadata captured in one world image.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerImage {
    /// Worker options captured for reconstruction.
    pub options: WorkerOptionsImage,
    /// Captured diagnostics store state.
    pub diagnostics: DiagnosticSnapshot,
    /// Captured scenario runner state.
    pub scenario: ScenarioRunnerSnapshot,
    /// Captured resource table state.
    pub resources: ResourceTableSnapshot,
    /// Captured finalizer lifecycle state.
    pub finalizers: RuntimeFinalizersImage,
    /// Captured event-loop state.
    pub event_loop: EventLoopSnapshot,
    /// Captured authoritative heap snapshot.
    pub heap: heap::HeapSnapshot,
    /// Captured worker-owned static bytes.
    pub statics: engine::StaticSpace,
    /// Captured worker-owned execution image.
    pub engine_image: Image,
}

/// Captured worker options with shared runtime storage when possible.
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

impl PartialEq for WorkerImage {
    fn eq(&self, other: &Self) -> bool {
        let heap = worker_heap_snapshot_bytes(&self.heap);
        let other_heap = worker_heap_snapshot_bytes(&other.heap);

        self.options == other.options
            && self.diagnostics == other.diagnostics
            && self.scenario == other.scenario
            && self.resources == other.resources
            && self.finalizers == other.finalizers
            && self.event_loop == other.event_loop
            && heap.is_ok()
            && heap == other_heap
            && self.statics == other.statics
            && self.engine_image == other.engine_image
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
            .field("runtime_id", &self.runtime_id)
            .field("environment", &self.environment)
            .field("options", &self.options)
            .field("resources", &self.resources)
            .field("scenario", &self.scenario)
            .field("finalizers", &self.finalizers)
            .field("diagnostics", &self.diagnostics)
            .field("bindings", &self.bindings)
            .field("handles", &self.handles.len())
            .field("heap", &self.heap)
            .field("engine", &"<worker execution engine>")
            .field("event_loop", &self.event_loop)
            .finish()
    }
}

impl Worker {
    /// Create one worker in one new runtime in one shared world.
    pub(crate) fn new_in_world(
        environment: impl Into<Arc<Environment>>,
        options: &RuntimeOptions,
        world: &mut WorldState,
        shared: &SharedHeap,
        runtime_static: &engine::StaticSpace,
        worker_options: WorkerOptions,
        engine: impl Into<Engine>,
    ) -> RuntimeResult<Self> {
        let environment = environment.into();
        let (runtime_id, worker_id) = Self::register_runtime(world, options, &worker_options)?;

        Self::from_registered(
            environment,
            options,
            world,
            shared,
            runtime_static,
            runtime_id,
            worker_id,
            engine,
        )
    }

    /// Create one worker in one existing runtime.
    pub(crate) fn new_in_runtime(
        environment: impl Into<Arc<Environment>>,
        options: &RuntimeOptions,
        world: &mut WorldState,
        shared: &SharedHeap,
        runtime_static: &engine::StaticSpace,
        runtime_id: RuntimeId,
        worker_options: WorkerOptions,
        engine: impl Into<Engine>,
    ) -> RuntimeResult<Self> {
        let environment = environment.into();
        let worker_id = Self::register_worker(world, runtime_id, &worker_options)?;

        Self::from_registered(
            environment,
            options,
            world,
            shared,
            runtime_static,
            runtime_id,
            worker_id,
            engine,
        )
    }

    /// Create one worker from registered world topology metadata.
    fn from_registered(
        environment: Arc<Environment>,
        options: &RuntimeOptions,
        world: &mut WorldState,
        shared: &SharedHeap,
        runtime_static: &engine::StaticSpace,
        runtime_id: RuntimeId,
        worker_id: WorkerId,
        engine: impl Into<Engine>,
    ) -> RuntimeResult<Self> {
        let mut engine = engine.into();

        // scenario and resources
        let scenario = Arc::new(ScenarioRunner::new(
            runtime_id,
            worker_id,
            world.trace.mode(),
            options.conditions.clone(),
        ));
        let resources = ResourceTable::new(worker_id);

        // bindings
        let mut bindings = BindingRegistry::new();
        bindings.set_access(BindingAccess::new(world.trace.mode()));
        bindings.apply_runtime_defaults(options);

        // heap and statics
        let heap_options = resolve_local_heap_options(&options.heap)?;
        let heap = heap::Heap::with_allocator_limits_and_options(
            shared.allocator.clone(),
            heap_options.limits,
            heap_options.options,
        )
        .map_err(Box::<RuntimeError>::from)?;
        let mut heap = heap;
        let mut statics = engine::StaticSpace::empty();
        let shared_gc_worker = shared.register_collector_worker();
        let mut shared_cache = shared.heap.allocation_cache();
        let context = MemoryContext {
            heap: &mut heap,
            shared_heap: shared.heap.as_ref(),
            shared_cache: &mut shared_cache,
            shared_gc_worker: &shared_gc_worker,
            worker_static: &mut statics,
            runtime_static,
        };
        engine.initialize(context)?;

        let event_loop = Box::new(EventLoop::default());

        // worker state
        Ok(Self {
            id: worker_id,
            runtime_id,
            environment,
            options: Arc::new(options.clone()),
            resources,
            scenario,
            finalizers: RuntimeFinalizers::default(),
            diagnostics: Arc::new(DiagnosticStore::from_options(&options.diagnostic)),
            bindings,
            handles: HeapHandleTable::default(),
            shared_gc_worker,
            shared_cache,
            heap,
            statics,
            engine,
            event_loop,
        })
    }

    /// Return immutable environment exposed to host bindings.
    pub fn environment(&self) -> &Environment {
        self.environment.as_ref()
    }

    /// Return immutable launch arguments exposed to host bindings.
    pub fn arguments(&self) -> &[String] {
        self.environment.args.as_slice()
    }

    /// Return this worker identifier.
    pub fn worker_id(&self) -> WorkerId {
        self.id
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

    /// Register one new runtime and its first worker in one world.
    fn register_runtime(
        world: &mut WorldState,
        options: &RuntimeOptions,
        worker_options: &WorkerOptions,
    ) -> RuntimeResult<(RuntimeId, WorkerId)> {
        // allocate topology identities
        let runtime_id = world.allocate_runtime_id();
        let worker_id = world.allocate_worker_id();

        // runtime selector metadata
        let runtime_name = options
            .identity
            .name
            .clone()
            .unwrap_or_else(|| "runtime".to_string());
        let runtime_entity = Entity::new(runtime_id.entity_id(), EntityKind::RUNTIME)
            .named(runtime_name)
            .labels(options.identity.labels.clone());

        let worker_name = worker_options
            .name
            .clone()
            .unwrap_or_else(|| format!("worker-{}", worker_id.0));
        let worker_entity = Entity::new(worker_id.entity_id(), EntityKind::WORKER)
            .named(worker_name)
            .labels(worker_options.labels.clone());

        // runtime metadata
        world.register_runtime_topology(runtime_id, runtime_entity)?;

        // worker metadata
        world.register_worker_topology(runtime_id, worker_id, worker_entity)?;

        Ok((runtime_id, worker_id))
    }

    /// Register one worker in one existing runtime.
    fn register_worker(
        world: &mut WorldState,
        runtime_id: RuntimeId,
        worker_options: &WorkerOptions,
    ) -> RuntimeResult<WorkerId> {
        // worker selector metadata
        let worker_id = world.allocate_worker_id();
        let worker_name = worker_options
            .name
            .clone()
            .unwrap_or_else(|| format!("worker-{}", worker_id.0));
        let worker_entity = Entity::new(worker_id.entity_id(), EntityKind::WORKER)
            .named(worker_name)
            .labels(worker_options.labels.clone());

        // register one worker in one existing runtime
        world.register_worker_topology(runtime_id, worker_id, worker_entity)?;

        Ok(worker_id)
    }

    /// Add one waiter for a timer resource.
    pub fn add_timer_waiter(
        &mut self,
        handle: ResourceId,
        runnable: Continuation,
        resume_value: engine::Value,
        priority: u8,
    ) -> RuntimeResult<()> {
        self.event_loop
            .add_timer_waiter(handle, runnable, resume_value, priority, &mut self.engine)
    }

    /// Remove the waiter registered for one timer resource.
    pub fn remove_timer_waiter(&mut self, handle: ResourceId) -> Option<Waiter> {
        self.event_loop.remove_timer_waiter(handle)
    }

    /// Add one waiter for one resource readiness.
    pub fn add_resource_waiter(
        &mut self,
        resource_id: ResourceId,
        readiness: Readiness,
        runnable: Continuation,
        resume_value: engine::Value,
        priority: u8,
    ) -> RuntimeResult<()> {
        self.event_loop.add_resource_waiter(
            resource_id,
            readiness,
            runnable,
            resume_value,
            priority,
            &mut self.engine,
        )
    }

    /// Add one waiter for a host event kind.
    pub fn add_host_waiter(
        &mut self,
        kind: HostEventKind,
        runnable: Continuation,
        resume_value: engine::Value,
        priority: u8,
    ) -> RuntimeResult<()> {
        self.event_loop
            .add_host_waiter(kind, runnable, resume_value, priority, &mut self.engine)
    }

    /// Retain one local heap reference for host-owned state.
    pub fn retain_heap_reference(&mut self, reference: heap::HeapReference) -> HeapHandle {
        self.handles.retain(reference)
    }

    /// Retain one existing heap handle owner.
    pub fn retain_heap_handle(&mut self, handle: HeapHandle) -> RuntimeResult<HeapHandle> {
        self.handles.retain_handle(handle)
    }

    /// Release one heap handle owner.
    pub fn release_heap_handle(&mut self, handle: HeapHandle) -> RuntimeResult<()> {
        self.handles.release(handle)
    }

    /// Return the current local heap reference retained by one handle.
    pub fn heap_reference(&self, handle: HeapHandle) -> RuntimeResult<heap::HeapReference> {
        self.handles.reference(handle)
    }

    /// Visit roots from engine, scheduler, and registered providers.
    pub fn visit_roots(&mut self, roots: &mut impl heap::RootSink) -> RuntimeResult<()> {
        let mut visit = |slot: heap::RootSlot<'_>| {
            let root = slot.load()?;
            roots.push(root);

            Ok(())
        };

        self.visit_root_slots(&mut visit)?;

        Ok(())
    }

    /// Visit mutable root slots from engine, scheduler, and retained host handles.
    pub fn visit_root_slots(
        &mut self,
        visit: &mut dyn FnMut(heap::RootSlot<'_>) -> heap::HeapResult<()>,
    ) -> RuntimeResult<()> {
        self.engine.visit_root_slots(&mut self.statics, visit)?;
        self.event_loop.visit_root_slots(&mut self.engine, visit)?;
        self.handles.visit_root_slots(visit)?;

        Ok(())
    }

    /// Return a snapshot of the GC state.
    pub fn gc_state(&self) -> heap::GcState {
        self.heap.gc_state().clone()
    }

    /// Return the exact retained heap usage for this worker.
    pub fn heap_usage(&self) -> heap::HeapUsage {
        self.heap.usage()
    }

    /// Run one budgeted local collection step using the current root set.
    pub fn collect_local_step(&mut self) -> RuntimeResult<heap::GcProgress> {
        let budget_bytes = self.heap.take_collection_budget_bytes();
        let engine = &mut self.engine;
        let event_loop = &mut self.event_loop;
        let handles = &mut self.handles;
        let statics = &mut self.statics;
        let mut roots = |visit: &mut dyn FnMut(heap::RootSlot<'_>) -> heap::HeapResult<()>| {
            engine.visit_root_slots(statics, visit)?;
            event_loop.visit_root_slots(engine, visit)?;
            handles.visit_root_slots(visit)?;

            Ok::<(), Box<RuntimeError>>(())
        };

        self.heap.collect_step(&mut roots, budget_bytes)
    }

    /// Collect shared heap roots from engine, scheduler, and registered providers.
    pub(crate) fn collect_shared_roots(&mut self) -> RuntimeResult<Vec<heap::SharedHeapReference>> {
        let mut roots = Vec::new();

        self.visit_roots(&mut roots)?;

        Ok(roots)
    }

    /// Start one incremental local-to-shared edge scan for this worker heap.
    pub(crate) fn start_shared_edge_scan(&mut self) {
        self.heap.start_shared_edge_scan();
    }

    /// Return whether this worker heap has drained its local-to-shared edge scan.
    pub(crate) fn shared_edge_scan_idle(&self) -> bool {
        self.heap.shared_edge_scan_idle()
    }

    /// Finish the current local-to-shared edge scan for this worker heap.
    pub(crate) fn finish_shared_edge_scan(&mut self) {
        self.heap.finish_shared_edge_scan();
    }

    /// Publish worker-local shared heap buffers.
    pub(crate) fn flush_shared_cache(&mut self, shared: &heap::SharedHeap) {
        shared.flush_allocation_cache(&mut self.shared_cache);
    }

    /// Scan bounded local-to-shared reference work into the provided root buffer.
    pub(crate) fn scan_shared_references(
        &mut self,
        roots: &mut Vec<heap::SharedHeapReference>,
        budget_bytes: usize,
    ) -> RuntimeResult<usize> {
        self.heap
            .scan_shared_references(roots, budget_bytes)
            .map_err(Box::<RuntimeError>::from)
    }

    /// Check configured retained-heap limits for this worker.
    pub fn check_heap_limits(&self) -> RuntimeResult<()> {
        self.heap.check_limits().map_err(Into::into)
    }

    /// Capture one materialized worker image.
    pub(crate) fn capture_image(
        &mut self,
        mode: CaptureMode,
        shared: &SharedHeap,
        runtime_static: &engine::StaticSpace,
    ) -> RuntimeResult<WorkerImage> {
        // host-retained local handles cannot be materialized without the owning host state
        if !self.handles.is_empty() {
            return Err(RuntimeError::CaptureBarrier {
                component: "runtime.heap_handles".to_string(),
                mode: format!("{mode:?}"),
                detail: "host-retained local heap handles are live".to_string(),
            }
            .boxed());
        }

        // local scheduler and external state
        let event_loop = self.event_loop.capture_image(mode, &mut self.engine)?;
        let resources = self.resources.capture_image(mode, ())?;
        let diagnostics = self.diagnostics.snapshot()?;
        let scenario = self.scenario.snapshot();

        // runtime-owned service state
        let finalizers = self.finalizers.capture_image(mode, ())?;

        // capture the worker-local image payload
        Ok(WorkerImage {
            options: WorkerOptionsImage::explicit_arc(self.options.clone()),
            diagnostics,
            scenario,
            resources,
            finalizers,
            event_loop,
            heap: self
                .heap
                .image()
                .and_then(|image| image.snapshot())
                .map_err(|error| {
                    RuntimeError::CaptureBarrier {
                        component: "runtime.heap".to_string(),
                        mode: format!("{mode:?}"),
                        detail: error.to_string(),
                    }
                    .boxed()
                })?,
            statics: self.statics.clone(),
            engine_image: self.engine.image(engine::MemoryContext {
                heap: &mut self.heap,
                shared_heap: shared.heap.as_ref(),
                shared_cache: &mut self.shared_cache,
                shared_gc_worker: &self.shared_gc_worker,
                worker_static: &mut self.statics,
                runtime_static,
            })?,
        })
    }

    /// Fork one live worker when all branch-sensitive state is quiescent.
    pub(crate) fn try_fork(
        &mut self,
        execution_mode: ExecutionMode,
        shared: &SharedHeap,
        runtime_static: &engine::StaticSpace,
        shared_gc_worker: heap::SharedGcWorker,
    ) -> RuntimeResult<Option<Self>> {
        // host-retained handles need their owning host resource to fork them
        if !self.handles.is_empty() {
            return Ok(None);
        }

        // scenario and diagnostics state
        let scenario = Arc::new(self.scenario.fork());
        let diagnostics = match self.diagnostics.try_fork()? {
            Some(diagnostics) => Arc::new(diagnostics),
            None => return Ok(None),
        };

        // resources, finalizers, and event loop
        let resources = match self.resources.try_fork() {
            Some(resources) => resources,
            None => return Ok(None),
        };
        let finalizers = match self.finalizers.try_fork() {
            Some(finalizers) => finalizers,
            None => return Ok(None),
        };

        // bindings and heap
        let mut bindings = BindingRegistry::new();
        bindings.set_access(BindingAccess::new(execution_mode));
        bindings.apply_runtime_defaults(&self.options);

        let mut heap = self.heap.fork()?;
        let mut statics = self.statics.clone();
        let mut shared_cache = shared.heap.allocation_cache();
        let mut engine = self.engine.fork(engine::MemoryContext {
            heap: &mut heap,
            shared_heap: shared.heap.as_ref(),
            shared_cache: &mut shared_cache,
            shared_gc_worker: &shared_gc_worker,
            worker_static: &mut statics,
            runtime_static,
        })?;
        let event_loop = Box::new(self.event_loop.fork(&mut self.engine, &mut engine)?);

        Ok(Some(Self {
            id: self.id,
            runtime_id: self.runtime_id,
            environment: self.environment.clone(),
            options: self.options.clone(),
            resources,
            scenario,
            finalizers,
            diagnostics,
            bindings,
            handles: HeapHandleTable::default(),
            shared_gc_worker,
            shared_cache,
            heap,
            statics,
            engine,
            event_loop,
        }))
    }

    /// Restore one worker from one materialized image.
    pub(crate) fn from_image(
        world: &mut WorldState,
        shared: &SharedHeap,
        runtime_static: &engine::StaticSpace,
        runtime_id: RuntimeId,
        worker_id: WorkerId,
        environment: Arc<Environment>,
        image: &WorkerImage,
        shared_options: Option<&Arc<RuntimeOptions>>,
        rebind_context: Option<&ResourceRebinders>,
    ) -> RuntimeResult<Self> {
        // resolve the captured options first
        let options = image.options.resolve(shared_options)?;

        // scenario and resources
        let scenario = Arc::new(ScenarioRunner::new(
            runtime_id,
            worker_id,
            world.trace.mode(),
            options.conditions.clone(),
        ));
        let resources = ResourceTable::new(worker_id);

        // bindings
        let mut bindings = BindingRegistry::new();
        bindings.set_access(BindingAccess::new(world.trace.mode()));
        bindings.apply_runtime_defaults(&options);

        // diagnostics and event loop
        let diagnostics = Arc::new(DiagnosticStore::from_options(&options.diagnostic));
        let mut event_loop = Box::new(EventLoop::default());

        // heap and engine
        let heap_options = resolve_local_heap_options(&options.heap)?;
        let mut heap = heap::Heap::from_snapshot_with_allocator(
            &image.heap,
            shared.allocator.clone(),
            heap_options.limits,
        )
        .map_err(Box::<RuntimeError>::from)?;
        let mut statics = image.statics.clone();
        let shared_gc_worker = shared.register_collector_worker();
        let mut shared_cache = shared.heap.allocation_cache();

        // rebuild the engine from the materialized worker image
        let mut engine = Engine::from_image(&image.engine_image)?;

        // restore backend execution state over the restored heap
        let context = MemoryContext {
            heap: &mut heap,
            shared_heap: shared.heap.as_ref(),
            shared_cache: &mut shared_cache,
            shared_gc_worker: &shared_gc_worker,
            worker_static: &mut statics,
            runtime_static,
        };
        engine.initialize(context)?;
        engine.restore(
            engine::MemoryContext {
                heap: &mut heap,
                shared_heap: shared.heap.as_ref(),
                shared_cache: &mut shared_cache,
                shared_gc_worker: &shared_gc_worker,
                worker_static: &mut statics,
                runtime_static,
            },
            &image.engine_image,
        )?;

        // restore local state on fresh containers
        event_loop.restore_snapshot(&image.event_loop, &mut engine)?;
        diagnostics.restore_snapshot(&image.diagnostics)?;
        scenario.restore_snapshot(&image.scenario);
        resources.restore_snapshot(&image.resources, rebind_context)?;

        Ok(Self {
            id: worker_id,
            runtime_id,
            environment,
            options,
            resources,
            scenario,
            finalizers: {
                let mut finalizers = RuntimeFinalizers::default();
                finalizers.restore_image(&image.finalizers, ())?;
                finalizers
            },
            diagnostics,
            bindings,
            handles: HeapHandleTable::default(),
            shared_gc_worker,
            shared_cache,
            heap,
            statics,
            engine,
            event_loop,
        })
    }
}

/// Serialize one captured worker heap snapshot for exact equality checks.
fn worker_heap_snapshot_bytes(snapshot: &heap::HeapSnapshot) -> Result<Vec<u8>, postcard::Error> {
    postcard::to_allocvec(snapshot)
}
