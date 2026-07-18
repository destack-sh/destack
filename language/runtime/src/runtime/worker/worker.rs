use destack_artifact::ConditionSet;
use destack_core::{Capture, CaptureMode};
use destack_heap as heap;
use destack_program as program;
use destack_serde::{Error as SerdeError, to_vec};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::diagnostic::{DiagnosticSnapshot, DiagnosticStore, RuntimeError, RuntimeResult};
use crate::host::binding::{BindingAccess, BindingTable};
use crate::host::resource::ResourceTableSnapshot;
use crate::host::{HostEventKind, ResourceId, ResourceTable};
use crate::runtime::RuntimeHeap;
use crate::runtime::heap::resolve_local_heap_options;
use crate::runtime::machine::{Continuation, Execution, Image, Machine, MachineId};
use crate::runtime::scheduler::{
    EventLoop, EventLoopSnapshot, Readiness, StoppedRunnable, StoppedRunnableImage, Waiter,
};
use crate::world::topology::LabelSet;
use crate::world::{Entity, EntityKind, RestoreContext, RuntimeId, WorkerSequence, WorldState};
use destack_repository::{Environment, ExecutionMode, RuntimeOptions};

/// Worker owned by one runtime.
pub struct Worker {
    /// Monotonic world-local worker identity.
    pub(crate) id: WorkerId,
    /// Runtime owner identifier in world topology.
    pub(crate) runtime_id: RuntimeId,
    /// Worker-local execution sequence.
    pub(crate) sequence: WorkerSequence,
    /// Immutable ambient environment for host bindings.
    pub(crate) environment: Arc<Environment>,
    /// Immutable runtime options.
    pub(crate) options: Arc<RuntimeOptions>,
    /// The active runtime conditions.
    pub(crate) conditions: Arc<ConditionSet>,
    /// Immutable executable program.
    pub(crate) program: Arc<program::Program>,
    /// Debugger generation used to derive executable debug sets.
    pub(crate) debug_generation: u64,
    /// Executable stop points active for this worker.
    pub(crate) stop_points: program::StopSet,
    /// Executable watchpoints active for this worker.
    pub(crate) watch_points: program::WatchSet,
    /// Runtime profile accumulated by this worker.
    pub(crate) profile: Option<program::Profile>,

    /// External resource table.
    pub(crate) resources: ResourceTable,
    /// Diagnostics storage for runtime errors and warning events.
    pub(crate) diagnostics: Arc<DiagnosticStore>,
    /// Runtime binding table and policy enforcement.
    pub(crate) binding_table: BindingTable,
    /// Shared mark worker queue handle.
    pub(crate) shared_mark_worker: heap::SharedMarkWorker,
    /// Worker-local shared allocation cache.
    pub(crate) shared_cache: heap::AllocationCache,
    /// Authoritative worker heap.
    pub(crate) heap: heap::Heap,
    /// Worker-owned static byte space.
    pub(crate) local_static: program::StaticSpace,
    /// Worker-owned machine.
    pub(crate) machine: Machine,
    /// Runnable stopped at a runtime stop point.
    pub(crate) stop: Option<StoppedRunnable>,
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
    pub labels: LabelSet,
}

/// One captured worker image.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerImage {
    /// Runtime owner identifier.
    pub runtime_id: RuntimeId,
    /// Worker identifier.
    pub worker_id: WorkerId,
    /// Captured worker-local execution sequence.
    pub sequence: WorkerSequence,
    /// The captured worker options.
    pub options: WorkerOptionsImage,
    /// Captured diagnostics store state.
    pub diagnostics: DiagnosticSnapshot,
    /// Captured resource table state.
    pub resources: ResourceTableSnapshot,
    /// Captured event-loop state.
    pub event_loop: EventLoopSnapshot,
    /// Captured authoritative heap snapshot.
    pub heap: heap::HeapSnapshot,
    /// Captured worker-owned static bytes.
    pub local_static: program::StaticSpaceImage,
    /// Captured worker-owned machine image.
    pub machine_image: Image,
    /// Captured stopped runnable state.
    pub stop: Option<StoppedRunnableImage>,
    /// Captured runtime profile state.
    pub profile: Option<program::Profile>,
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
        self.stop.is_some() || self.event_loop.has_pending_work()
    }
}

impl PartialEq for WorkerImage {
    fn eq(&self, other: &Self) -> bool {
        let heap = worker_heap_snapshot_bytes(&self.heap);
        let other_heap = worker_heap_snapshot_bytes(&other.heap);

        self.runtime_id == other.runtime_id
            && self.worker_id == other.worker_id
            && self.options == other.options
            && self.sequence == other.sequence
            && self.diagnostics == other.diagnostics
            && self.resources == other.resources
            && self.event_loop == other.event_loop
            && heap.is_ok()
            && heap == other_heap
            && self.local_static == other.local_static
            && self.machine_image == other.machine_image
            && self.stop == other.stop
            && self.profile == other.profile
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
            .field("sequence", &self.sequence)
            .field("environment", &self.environment)
            .field("options", &self.options)
            .field("resources", &self.resources)
            .field("diagnostics", &self.diagnostics)
            .field("binding_table", &self.binding_table)
            .field("heap", &self.heap)
            .field("machine", &"<worker machine>")
            .field("event_loop", &self.event_loop)
            .finish()
    }
}

impl Worker {
    /// Create one worker in one new runtime in one shared world.
    pub(crate) fn new_in_world(
        environment: impl Into<Arc<Environment>>,
        options: &RuntimeOptions,
        conditions: Arc<ConditionSet>,
        world: &mut WorldState,
        runtime_heap: &RuntimeHeap,
        worker_options: WorkerOptions,
        program: Arc<program::Program>,
        execution: &Execution,
    ) -> RuntimeResult<Self> {
        let environment = environment.into();
        let (runtime_id, worker_id) = Self::register_runtime(world, options, &worker_options)?;

        Self::from_registered(
            environment,
            options,
            conditions,
            world,
            runtime_heap,
            runtime_id,
            worker_id,
            program,
            execution,
        )
    }

    /// Create one worker in one existing runtime.
    pub(crate) fn new_in_runtime(
        environment: impl Into<Arc<Environment>>,
        options: &RuntimeOptions,
        conditions: Arc<ConditionSet>,
        world: &mut WorldState,
        runtime_heap: &RuntimeHeap,
        runtime_id: RuntimeId,
        worker_options: WorkerOptions,
        program: Arc<program::Program>,
        execution: &Execution,
    ) -> RuntimeResult<Self> {
        let environment = environment.into();
        let worker_id = Self::register_worker(world, runtime_id, &worker_options)?;

        Self::from_registered(
            environment,
            options,
            conditions,
            world,
            runtime_heap,
            runtime_id,
            worker_id,
            program,
            execution,
        )
    }

    /// Create one worker from registered world topology metadata.
    fn from_registered(
        environment: Arc<Environment>,
        options: &RuntimeOptions,
        conditions: Arc<ConditionSet>,
        world: &mut WorldState,
        runtime_heap: &RuntimeHeap,
        runtime_id: RuntimeId,
        worker_id: WorkerId,
        program: Arc<program::Program>,
        execution: &Execution,
    ) -> RuntimeResult<Self> {
        let machine_id = MachineId::new(worker_id.0);
        let machine = Machine::new(
            machine_id,
            program.clone(),
            runtime_heap.memory.clone(),
            execution,
        )?;

        // resources
        let resources = ResourceTable::new(worker_id);

        // binding table
        let mut binding_table = BindingTable::new();
        binding_table.set_access(BindingAccess::new(world.trace.mode()));
        binding_table.apply_runtime_defaults(options);

        // heap and local_static
        let heap_options = resolve_local_heap_options(&options.heap)?;
        let heap = heap::Heap::new(
            runtime_heap.memory.clone(),
            heap_options.limits,
            heap_options.options,
        )
        .map_err(Box::<RuntimeError>::from)?;
        let local_static = program.materialize_local_statics(runtime_heap.memory.clone())?;
        let shared_mark_worker = runtime_heap.register_mark_worker();
        let shared_cache = runtime_heap.shared.allocation_cache();
        machine.require_heap_compatibility(&heap, runtime_heap.shared.as_ref())?;

        let event_loop = Box::new(EventLoop::default());

        // worker state
        Ok(Self {
            id: worker_id,
            runtime_id,
            sequence: WorkerSequence::new(0),
            environment,
            options: Arc::new(options.clone()),
            conditions,
            program,
            debug_generation: world.debugger.generation(),
            stop_points: world.debugger.stop_set(runtime_id, worker_id),
            watch_points: world.debugger.watch_set(runtime_id, worker_id),
            profile: None,
            resources,
            diagnostics: Arc::new(DiagnosticStore::from_options(&options.diagnostic)),
            binding_table,
            shared_mark_worker,
            shared_cache,
            heap,
            local_static,
            machine,
            stop: None,
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
        self.stop.is_some() || self.event_loop.has_pending_work()
    }

    /// Return the accumulated runtime profile when active.
    pub fn profile(&self) -> Option<&program::Profile> {
        self.profile.as_ref()
    }

    /// Start runtime profiling for this worker.
    pub fn start_profile(&mut self, options: program::ProfileOptions) {
        self.profile = Some(program::Profile::new(self.program.as_ref(), options));
    }

    /// Stop runtime profiling and return the accumulated profile.
    pub fn stop_profile(&mut self) -> Option<program::Profile> {
        self.profile.take()
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
        let runtime_id = world.allocate_runtime_id()?;
        let worker_id = world.allocate_worker_id()?;

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
        let worker_id = world.allocate_worker_id()?;
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
        resume_value: program::Value,
    ) -> RuntimeResult<()> {
        self.event_loop
            .add_timer_waiter(handle, runnable, resume_value);

        Ok(())
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
        resume_value: program::Value,
    ) -> RuntimeResult<()> {
        self.event_loop
            .add_resource_waiter(resource_id, readiness, runnable, resume_value);

        Ok(())
    }

    /// Add one waiter for a host event kind.
    pub fn add_host_waiter(
        &mut self,
        kind: HostEventKind,
        runnable: Continuation,
        resume_value: program::Value,
    ) -> RuntimeResult<()> {
        self.event_loop
            .add_host_waiter(kind, runnable, resume_value);

        Ok(())
    }

    /// Visit roots from machine, scheduler, and registered providers.
    pub fn visit_roots(&mut self, roots: &mut impl heap::RootSink) -> RuntimeResult<()> {
        let mut visit = |slot: heap::RootSlot<'_>| {
            let root = slot.load()?;
            roots.push(root);

            Ok(())
        };

        self.visit_root_slots(&mut visit)?;

        Ok(())
    }

    /// Visit roots from one static space through this worker machine.
    pub fn visit_static_roots(
        &mut self,
        static_space: &mut program::StaticSpace,
        roots: &mut impl heap::RootSink,
    ) -> RuntimeResult<()> {
        let mut visit = |slot: heap::RootSlot<'_>| {
            let root = slot.load()?;
            roots.push(root);

            Ok(())
        };

        self.machine.visit_static_root_slots(
            program::GlobalLocation::SharedStatic,
            static_space,
            &mut visit,
        )?;

        Ok(())
    }

    /// Visit mutable root slots from machine, scheduler, and retained host handles.
    pub fn visit_root_slots(
        &mut self,
        visit: &mut dyn FnMut(heap::RootSlot<'_>) -> heap::HeapResult<()>,
    ) -> RuntimeResult<()> {
        self.machine
            .visit_root_slots(&mut self.local_static, visit)?;
        self.event_loop.visit_root_slots(&mut self.machine, visit)?;
        if let Some(stop) = &mut self.stop {
            stop.visit_root_slots(&mut self.machine, visit)?;
        }

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

    /// Collect shared heap roots from machine, scheduler, and registered providers.
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

    /// Trace bounded local-to-shared edges into the provided root buffer.
    pub(crate) fn trace_shared_roots(
        &mut self,
        roots: &mut Vec<heap::SharedHeapReference>,
        budget_bytes: usize,
    ) -> RuntimeResult<usize> {
        let trace_view = self.machine.trace_view();

        self.heap
            .trace_shared_roots(roots, budget_bytes, trace_view)
            .map_err(Box::<RuntimeError>::from)
    }

    /// Check configured retained-heap limits for this worker.
    pub fn check_heap_limits(&self) -> RuntimeResult<()> {
        self.heap.check_limits().map_err(Into::into)
    }

    /// Capture one materialized worker image.
    pub(crate) fn capture_image(&mut self, mode: CaptureMode) -> RuntimeResult<WorkerImage> {
        // local scheduler and external state
        let event_loop = self.event_loop.capture_image(mode, ())?;
        let resources = self.resources.capture_image(mode, ())?;
        let diagnostics = self.diagnostics.snapshot()?;

        // capture the worker-local image payload
        Ok(WorkerImage {
            runtime_id: self.runtime_id,
            worker_id: self.id,
            sequence: self.sequence,
            options: WorkerOptionsImage::explicit_arc(self.options.clone()),
            diagnostics,
            resources,
            event_loop,
            heap: self
                .heap
                .image()
                .map(|image| image.snapshot())
                .map_err(|error| {
                    RuntimeError::capture_barrier(
                        "runtime.heap",
                        format!("{mode:?}"),
                        error.to_string(),
                    )
                    .boxed()
                })?,
            local_static: self.local_static.image()?,
            machine_image: self.machine.image()?,
            stop: self.stop.as_ref().map(StoppedRunnableImage::capture),
            profile: self.profile.clone(),
        })
    }

    /// Fork one live worker when all branch-sensitive state is quiescent.
    pub(crate) fn try_fork(
        &mut self,
        execution_mode: ExecutionMode,
        runtime_heap: &RuntimeHeap,
        shared_mark_worker: heap::SharedMarkWorker,
    ) -> RuntimeResult<Option<Self>> {
        // diagnostics state
        let diagnostics = match self.diagnostics.try_fork()? {
            Some(diagnostics) => Arc::new(diagnostics),
            None => return Ok(None),
        };

        // resources and event loop
        let resources = match self.resources.try_fork() {
            Some(resources) => resources,
            None => return Ok(None),
        };

        // binding table and heap
        let mut binding_table = BindingTable::new();
        binding_table.set_access(BindingAccess::new(execution_mode));
        binding_table.apply_runtime_defaults(&self.options);

        let trace_view = self.machine.trace_view();
        let heap = self.heap.fork(runtime_heap.memory.clone(), trace_view)?;
        let local_static = self.local_static.fork(runtime_heap.memory.clone());
        let shared_cache = runtime_heap.shared.allocation_cache();
        let machine = self.machine.fork(runtime_heap.memory.clone())?;
        let event_loop = Box::new(self.event_loop.fork()?);
        let stop = self.stop.clone();
        let profile = self.profile.clone();

        Ok(Some(Self {
            id: self.id,
            runtime_id: self.runtime_id,
            sequence: self.sequence,
            environment: self.environment.clone(),
            options: self.options.clone(),
            conditions: self.conditions.clone(),
            resources,
            diagnostics,
            binding_table,
            program: self.program.clone(),
            debug_generation: self.debug_generation,
            stop_points: self.stop_points.clone(),
            watch_points: self.watch_points.clone(),
            profile,
            shared_mark_worker,
            shared_cache,
            heap,
            local_static,
            machine,
            stop,
            event_loop,
        }))
    }

    /// Restore one worker from one materialized image.
    pub(crate) fn from_image(
        world: &mut WorldState,
        runtime_heap: &RuntimeHeap,
        runtime_id: RuntimeId,
        worker_id: WorkerId,
        environment: Arc<Environment>,
        conditions: Arc<ConditionSet>,
        image: &WorkerImage,
        shared_options: Option<&Arc<RuntimeOptions>>,
        program: Arc<program::Program>,
        execution: &Execution,
        restore: RestoreContext<'_>,
    ) -> RuntimeResult<Self> {
        // resolve the captured options first
        let options = image.options.resolve(shared_options)?;

        // resources
        let resources = ResourceTable::new(worker_id);

        // binding table
        let mut binding_table = BindingTable::new();
        binding_table.set_access(BindingAccess::new(world.trace.mode()));
        binding_table.apply_runtime_defaults(&options);

        // diagnostics and event loop
        let diagnostics = Arc::new(DiagnosticStore::from_options(&options.diagnostic));
        let mut event_loop = Box::new(EventLoop::default());

        // machine
        let machine = Machine::from_image(
            MachineId::new(worker_id.0),
            program.clone(),
            runtime_heap.memory.clone(),
            &image.machine_image,
            execution,
        )?;
        // heap
        let heap_options = resolve_local_heap_options(&options.heap)?;
        let heap = heap::Heap::from_snapshot(
            &image.heap,
            runtime_heap.memory.clone(),
            heap_options.limits,
            program.trace_view(),
        )
        .map_err(Box::<RuntimeError>::from)?;
        let local_static =
            program::StaticSpace::from_image(runtime_heap.memory.clone(), &image.local_static)?;
        let shared_mark_worker = runtime_heap.register_mark_worker();
        let shared_cache = runtime_heap.shared.allocation_cache();

        // require the restored heaps to match the restored program
        machine.require_heap_compatibility(&heap, runtime_heap.shared.as_ref())?;

        // restore local state on fresh containers
        event_loop.restore_snapshot(&image.event_loop)?;
        diagnostics.restore_snapshot(&image.diagnostics)?;
        resources.restore_snapshot(&image.resources, restore.resource_rebinders())?;

        Ok(Self {
            id: worker_id,
            runtime_id,
            sequence: image.sequence,
            environment,
            options,
            conditions,
            program,
            debug_generation: world.debugger.generation(),
            stop_points: world.debugger.stop_set(runtime_id, worker_id),
            watch_points: world.debugger.watch_set(runtime_id, worker_id),
            profile: image.profile.clone(),
            resources,
            diagnostics,
            binding_table,
            shared_mark_worker,
            shared_cache,
            heap,
            local_static,
            machine,
            stop: image.stop.as_ref().map(StoppedRunnableImage::restore),
            event_loop,
        })
    }
}

/// Serialize one captured worker heap snapshot for exact equality checks.
fn worker_heap_snapshot_bytes(snapshot: &heap::HeapSnapshot) -> Result<Vec<u8>, SerdeError> {
    to_vec(snapshot)
}
