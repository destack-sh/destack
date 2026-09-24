use std::fmt;
use std::sync::Arc;

use destack_artifact::ConditionSet;
use destack_core::{Capture, CaptureMode};
use destack_heap as heap;
use destack_memory::MemoryRange;
use destack_program as program;
use destack_repository::{Environment, ExecutionMode, RuntimeOptions};
use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use crate::binding::{BindingAccess, BindingTable, ReplayPayload};
use crate::diagnostic::{DiagnosticImage, DiagnosticStore, RuntimeError, RuntimeResult};
use crate::host::resource::ResourceImage;
use crate::host::{HostEventKind, ResourceId, ResourceTable};
use crate::machine::{Engine, Machine, MachineImage};
use crate::runtime::RuntimeId;
use crate::scheduler::{
    Callback, EventLoop, EventLoopImage, Readiness, RetainedRunnable, ScheduledTimer,
};
use crate::world::topology::LabelSet;
use crate::world::{RestoreContext, WorkerSequence, WorldState};

use super::{Handshake, Request};

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
    /// Runtime-shared binding implementations.
    pub(crate) binding_table: Arc<BindingTable>,
    /// Worker-local binding access policy.
    pub(crate) binding_access: BindingAccess,
    /// Runtime-owned shared heap.
    pub(crate) shared_heap: Arc<heap::SharedHeap>,
    /// Allocation plans indexed by Program allocation site id.
    pub(crate) allocation_plans: Arc<[heap::AllocationPlan]>,
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
    /// Process-local execution handshake.
    pub(crate) handshake: Arc<Handshake>,
    /// Runnable retained across a handshake or debugger stop.
    pub(crate) retained: Option<RetainedRunnable>,
    /// Event loop for tasks, microtasks, and timers.
    pub(crate) event_loop: EventLoop,
}

/// Stable identifier for one world-managed worker.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Reflect,
)]
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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkerImage {
    /// Runtime owner identifier.
    pub runtime_id: RuntimeId,
    /// Worker identifier.
    pub worker_id: WorkerId,
    /// Captured worker-local execution sequence.
    pub sequence: WorkerSequence,
    /// Captured diagnostics store state.
    pub diagnostics: DiagnosticImage,
    /// Captured resource table state.
    pub resources: ResourceImage,
    /// Captured event-loop state.
    pub event_loop: EventLoopImage,
    /// Captured authoritative heap image.
    pub heap: heap::HeapImage,
    /// Captured worker-owned static bytes.
    pub local_static: program::StaticSpaceImage,
    /// Captured canonical machine state.
    pub machine: MachineImage,
    /// Captured retained runnable state.
    pub retained: Option<RetainedRunnable>,
    /// Captured runtime profile state.
    pub profile: Option<program::Profile>,
}

impl WorkerImage {
    /// Return whether the captured worker still has pending event-loop work.
    pub fn has_pending_work(&self) -> bool {
        self.retained.is_some() || self.event_loop.has_pending_work()
    }
}

impl fmt::Debug for Worker {
    /// Format one worker without traversing machine execution state.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Worker")
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
    /// Create one worker with identities allocated by its owning runtime.
    pub(crate) fn new(
        environment: Arc<Environment>,
        options: Arc<RuntimeOptions>,
        conditions: Arc<ConditionSet>,
        world: &mut WorldState,
        shared_heap: &Arc<heap::SharedHeap>,
        allocation_plans: &Arc<[heap::AllocationPlan]>,
        constants: &program::StaticSpace,
        shared_statics: &program::StaticSpace,
        constant_range: MemoryRange,
        runtime_id: RuntimeId,
        worker_id: WorkerId,
        binding_table: Arc<BindingTable>,
        engine: &Engine,
    ) -> RuntimeResult<Self> {
        let program = engine.program().clone();

        // resources
        let resources = ResourceTable::new(worker_id);

        // binding access
        let replay_payload = world.trace.store().header().replay_payload;
        let binding_access = BindingAccess::new(world.trace.mode(), replay_payload);
        let diagnostics = Arc::new(DiagnosticStore::from_options(&options.diagnostic));

        // execution storage
        let heap_options = options
            .heap
            .local_heap_options()
            .map_err(Box::<RuntimeError>::from)?;
        let mut heap = heap::Heap::new(
            shared_heap.memory().clone(),
            options.heap.local.limits(),
            heap_options,
        )
        .map_err(Box::<RuntimeError>::from)?;
        heap.set_constant_range(constant_range);
        let machine = engine.spawn(shared_heap.memory().clone())?;
        let handshake = Arc::new(Handshake::new());
        let local_static = program.materialize_local_statics(
            shared_heap.memory().clone(),
            constants,
            shared_statics,
        )?;
        let shared_mark_worker = shared_heap.register_mark_worker();
        let shared_cache = shared_heap.allocation_cache();
        let event_loop = EventLoop::default();

        // worker state
        let profile = Self::instrument_profile(&program);
        Ok(Self {
            id: worker_id,
            runtime_id,
            sequence: WorkerSequence::new(0),
            environment,
            options,
            conditions,
            program,
            debug_generation: world.debugger.generation(),
            stop_points: world.debugger.stop_set(runtime_id, worker_id),
            watch_points: world.debugger.watch_set(runtime_id, worker_id),
            profile,
            resources,
            diagnostics,
            binding_table,
            binding_access,
            shared_heap: shared_heap.clone(),
            allocation_plans: allocation_plans.clone(),
            shared_mark_worker,
            shared_cache,
            heap,
            local_static,
            machine,
            handshake,
            retained: None,
            event_loop,
        })
    }

    /// Return immutable environment exposed to host bindings.
    pub fn environment(&self) -> &Environment {
        self.environment.as_ref()
    }

    /// Return immutable process arguments exposed to host bindings.
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

    /// Publish one process-local request to this worker.
    pub fn request(&self, request: Request) {
        self.handshake.request(request);
    }

    /// Return whether this worker still has pending scheduler work.
    pub fn has_pending_work(&self) -> bool {
        self.retained.is_some() || self.event_loop.has_pending_work()
    }

    /// Start a profile for programs carrying instrument sites, so their counts are kept.
    fn instrument_profile(program: &program::Program) -> Option<program::Profile> {
        let sections = program.sections();
        let sites = program.sites();
        let instruments = sites.counter_count(sections) + sites.sampler_count(sections);

        (instruments > 0).then(|| program::Profile::new(program, program::ProfileOptions::STANDARD))
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

    /// Add one waiter for a timer resource.
    pub fn add_timer_waiter(&mut self, handle: ResourceId, callback: Callback) {
        self.event_loop.add_timer_waiter(handle, callback);
    }

    /// Remove the waiter registered for one timer resource.
    pub fn remove_timer_waiter(&mut self, handle: ResourceId) -> Option<Callback> {
        self.event_loop.remove_timer_waiter(handle)
    }

    /// Schedule one timer for this worker.
    pub fn schedule_timer(&mut self, timer: ScheduledTimer) -> RuntimeResult<()> {
        self.event_loop.schedule_timer(timer)
    }

    /// Cancel one timer for this worker.
    pub fn cancel_timer(&mut self, handle: ResourceId) -> RuntimeResult<()> {
        self.event_loop.cancel_timer(handle)
    }

    /// Add one waiter for one resource readiness.
    pub fn add_resource_waiter(
        &mut self,
        resource_id: ResourceId,
        readiness: Readiness,
        callback: Callback,
    ) {
        self.event_loop
            .add_resource_waiter(resource_id, readiness, callback);
    }

    /// Add one waiter for a host event kind.
    pub fn add_host_waiter(&mut self, kind: HostEventKind, callback: Callback) {
        self.event_loop.add_host_waiter(kind, callback);
    }

    /// Visit mutable root slots from machine, scheduler, and retained host handles.
    pub fn visit_root_slots(
        &mut self,
        visit: &mut dyn FnMut(heap::RootSlot<'_>) -> heap::HeapResult<()>,
    ) -> RuntimeResult<()> {
        // visit worker static roots through their exact Program types
        self.program
            .visit_static_root_slots(
                program::GlobalLocation::LocalStatic,
                &mut self.local_static,
                visit,
            )
            .map_err(Box::<RuntimeError>::from)?;

        // visit scheduler and machine roots in the shared world memory
        self.event_loop.visit_root_slots(&self.program, visit)?;
        self.machine.visit_root_slots(visit)?;
        for execution in self.event_loop.executions_mut() {
            self.machine.visit_fiber_root_slots(execution, visit)?;
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
    pub(crate) fn collect_shared_roots(
        &mut self,
        shared_static: &mut program::StaticSpace,
    ) -> RuntimeResult<Vec<heap::SharedHeapReference>> {
        let shared_heap = self.shared_heap.clone();
        let mut roots = Vec::new();

        // keep the runtime heap references and the borrows into the shared heap
        let mut visit = |slot: heap::RootSlot<'_>| {
            match slot {
                heap::RootSlot::SharedHeapBytes(bytes) => {
                    let reference = heap::SharedHeapReference::read_from_bytes(bytes)?;
                    if !reference.is_nullish() {
                        roots.push(reference);
                    }
                }
                heap::RootSlot::BorrowBytes(bytes) => {
                    let reference = heap::SharedHeapReference::read_from_bytes(bytes)?;
                    if shared_heap.contains(reference) {
                        roots.push(reference);
                    }
                }
                heap::RootSlot::HeapReference(_) | heap::RootSlot::HeapBytes(_) => {}
            }

            Ok(())
        };

        // visit the runtime's shared statics beside this worker's roots
        self.program
            .visit_static_root_slots(
                program::GlobalLocation::SharedStatic,
                shared_static,
                &mut visit,
            )
            .map_err(Box::<RuntimeError>::from)?;
        self.visit_root_slots(&mut visit)?;

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
    pub(crate) fn flush_shared_cache(&mut self, heap: &heap::SharedHeap) {
        heap.flush_allocation_cache(&mut self.shared_cache);
    }

    /// Trace bounded local-to-shared edges into the provided root buffer.
    pub(crate) fn trace_shared_roots(
        &mut self,
        roots: &mut Vec<heap::SharedHeapReference>,
        budget_bytes: usize,
    ) -> RuntimeResult<usize> {
        let trace_view = self.program.trace_view();

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
        let diagnostics = self.diagnostics.image()?;

        // capture the worker-local image payload
        Ok(WorkerImage {
            runtime_id: self.runtime_id,
            worker_id: self.id,
            sequence: self.sequence,
            diagnostics,
            resources,
            event_loop,
            heap: self.heap.image().map_err(|error| {
                RuntimeError::capture_barrier(
                    "runtime.heap",
                    format!("{mode:?}"),
                    error.to_string(),
                )
                .boxed()
            })?,
            local_static: self.local_static.image(),
            machine: self.machine.image(),
            retained: self.retained.clone(),
            profile: self.profile.clone(),
        })
    }

    /// Fork one live worker when all branch-sensitive state is quiescent.
    pub(crate) fn try_fork(
        &mut self,
        execution_mode: ExecutionMode,
        replay_payload: ReplayPayload,
        shared_heap: &Arc<heap::SharedHeap>,
        allocation_plans: &Arc<[heap::AllocationPlan]>,
        shared_mark_worker: heap::SharedMarkWorker,
    ) -> RuntimeResult<Option<Self>> {
        // diagnostics state
        let diagnostics = match self.diagnostics.try_fork()? {
            Some(diagnostics) => Arc::new(diagnostics),
            None => return Ok(None),
        };

        // external resources
        let resources = match self.resources.try_fork() {
            Some(resources) => resources,
            None => return Ok(None),
        };

        // worker execution state over the already-forked world memory
        let binding_access = BindingAccess::new(execution_mode, replay_payload);
        let trace_view = self.program.trace_view();
        let heap = self.heap.fork(shared_heap.memory().clone(), trace_view)?;
        let local_static = self.local_static.fork(shared_heap.memory().clone());
        let shared_cache = shared_heap.allocation_cache();
        let machine = self.machine.fork(shared_heap.memory().clone());
        let handshake = Arc::new(Handshake::new());
        let event_loop = self.event_loop.fork(shared_heap.memory())?;
        let retained = self.retained.clone();
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
            binding_table: self.binding_table.clone(),
            binding_access,
            shared_heap: shared_heap.clone(),
            allocation_plans: allocation_plans.clone(),
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
            handshake,
            retained,
            event_loop,
        }))
    }

    /// Restore one worker from one materialized image.
    pub(crate) fn from_image(
        world: &mut WorldState,
        shared_heap: &Arc<heap::SharedHeap>,
        allocation_plans: &Arc<[heap::AllocationPlan]>,
        constant_range: MemoryRange,
        runtime_id: RuntimeId,
        worker_id: WorkerId,
        environment: Arc<Environment>,
        options: Arc<RuntimeOptions>,
        conditions: Arc<ConditionSet>,
        image: &WorkerImage,
        binding_table: Arc<BindingTable>,
        engine: &Engine,
        restore: RestoreContext<'_>,
    ) -> RuntimeResult<Self> {
        let program = engine.program().clone();

        // resources
        let resources = ResourceTable::new(worker_id);

        // binding access
        let replay_payload = world.trace.store().header().replay_payload;
        let binding_access = BindingAccess::new(world.trace.mode(), replay_payload);

        // diagnostics and event loop
        let diagnostics = Arc::new(DiagnosticStore::from_options(&options.diagnostic));
        let mut event_loop = EventLoop::default();

        // heap
        let mut heap = heap::Heap::from_image(
            &image.heap,
            shared_heap.memory().clone(),
            options.heap.local.limits(),
            program.trace_view(),
        )
        .map_err(Box::<RuntimeError>::from)?;
        heap.set_constant_range(constant_range);
        let mut machine = engine.spawn(shared_heap.memory().clone())?;
        machine.restore(&image.machine)?;
        let handshake = Arc::new(Handshake::new());
        let local_static =
            program::StaticSpace::from_image(shared_heap.memory().clone(), &image.local_static);
        let shared_mark_worker = shared_heap.register_mark_worker();
        let shared_cache = shared_heap.allocation_cache();

        // restore local state on fresh containers
        diagnostics.restore(&image.diagnostics)?;
        resources.restore(&image.resources, restore.resource_rebinders())?;
        event_loop.restore(&image.event_loop, shared_heap.memory())?;

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
            binding_access,
            shared_heap: shared_heap.clone(),
            allocation_plans: allocation_plans.clone(),
            shared_mark_worker,
            shared_cache,
            heap,
            local_static,
            machine,
            handshake,
            retained: image.retained.clone(),
            event_loop,
        })
    }
}
