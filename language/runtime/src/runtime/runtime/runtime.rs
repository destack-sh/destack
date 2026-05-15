use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::host::poller::{HostPoller, PollerEvent};
use crate::host::resource::ResourceRebinders;
use crate::host::{Host, HostEvent, HostSession};
use crate::runtime::SharedCollector;
use crate::runtime::engine::{Engine, Entry};
use crate::runtime::heap::SharedHeap;
use crate::runtime::runtime::RuntimeHostOptions;
use crate::runtime::scheduler::{
    HostWake, Readiness, ResourceWake, ScheduledTimer, TickResult, Wake,
};
use crate::runtime::time::Instant;
use crate::runtime::worker::{Worker, WorkerId, WorkerImage, WorkerOptions, WorkerOptionsImage};
use crate::world::{RuntimeId, WorkerWake, WorldState};
use destack_core::CaptureMode;
use destack_workspace::{ExecutionMode, RuntimeOptions};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::Arc;
use {destack_engine as engine, destack_heap as heap};

/// Runtime container that owns one or more workers in one shared world.
pub struct Runtime {
    /// Runtime identifier in world topology.
    id: RuntimeId,
    /// Runtime name used for identity selection and diagnostics.
    name: String,
    /// Immutable host arguments shared by newly spawned workers.
    process_args: Arc<[String]>,
    /// Runtime-owned worker defaults.
    worker_options: Arc<RuntimeOptions>,
    /// Runtime host reconstruction settings.
    host_options: RuntimeHostOptions,
    /// Shared host integration for all workers in this runtime.
    host: HostSession,
    /// Shared host poller for external events.
    poller: Box<dyn HostPoller>,

    /// Runtime-owned shared heap and collection state.
    pub(crate) shared: SharedHeap,
    /// Runtime-owned static byte space.
    statics: engine::StaticSpace,
    /// All active workers keyed by identifier.
    workers: BTreeMap<WorkerId, Box<Worker>>,
    /// Default worker used by convenience accessors.
    default_worker_id: WorkerId,
    /// The next worker slot to schedule first.
    next_worker_cursor: usize,
}

/// Materialized runtime metadata captured in one world image.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeImage {
    /// Default worker identifier for this runtime.
    pub default_worker_id: WorkerId,
    /// Runtime launch arguments.
    pub process_args: Arc<[String]>,
    /// Runtime-owned worker defaults captured for reconstruction.
    pub worker_options: Arc<RuntimeOptions>,
    /// Runtime host reconstruction settings captured for reconstruction.
    pub host_options: RuntimeHostOptions,
    /// Captured runtime-owned shared heap state.
    pub shared_heap: heap::SharedHeapSnapshot,
    /// Captured runtime-owned static bytes.
    pub statics: engine::StaticSpace,
    /// The next worker slot to schedule first.
    pub next_worker_cursor: usize,
}

impl std::fmt::Debug for Runtime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Runtime")
            .field("runtime_id", &self.id)
            .field("name", &self.name)
            .field("process_args", &self.process_args)
            .field("worker_options", &self.worker_options)
            .field("host_options", &self.host_options)
            .field("host", &self.host)
            .field("shared", &self.shared)
            .field("workers", &self.workers)
            .field("default_worker_id", &self.default_worker_id)
            .field("next_worker_cursor", &self.next_worker_cursor)
            .field("poller", &"<shared host poller>")
            .finish()
    }
}

impl Runtime {
    /// Create a runtime with one default worker in one explicit shared world.
    pub(crate) fn from_options_in_world(
        process_args: impl Into<Arc<[String]>>,
        options: &RuntimeOptions,
        world: &mut WorldState,
        host: Arc<dyn Host>,
        allocator: Arc<heap::Allocator>,
        collector: Arc<SharedCollector>,
        engine: impl Into<Engine>,
    ) -> RuntimeResult<Self> {
        let process_args = process_args.into();
        let shared = SharedHeap::new(allocator, collector, options)?;
        let statics = engine::StaticSpace::empty();
        let default_worker = Worker::new_in_world(
            process_args.clone(),
            options,
            world,
            &shared,
            &statics,
            WorkerOptions::default(),
            engine,
        )?;
        let runtime = Self::new(process_args, options, shared, statics, host, default_worker)?;

        Ok(runtime)
    }

    /// Return the shared host integration for this runtime.
    pub fn host(&self) -> &HostSession {
        &self.host
    }

    /// Return the shared process arguments backing for this runtime.
    pub(crate) fn process_args(&self) -> Arc<[String]> {
        self.process_args.clone()
    }

    /// Return the current default worker id.
    pub fn default_worker_id(&self) -> WorkerId {
        self.default_worker_id
    }

    /// Return the world topology runtime id.
    pub fn runtime_id(&self) -> RuntimeId {
        self.id
    }

    /// Return this runtime name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Set one explicit default worker.
    pub fn set_default_worker(&mut self, worker_id: WorkerId) -> RuntimeResult<()> {
        if self.workers.contains_key(&worker_id) {
            self.default_worker_id = worker_id;
            return Ok(());
        }

        Err(RuntimeError::WorkerNotFound {
            worker_id: worker_id.0,
        }
        .boxed())
    }

    /// Return all active worker ids.
    pub fn worker_ids(&self) -> Vec<WorkerId> {
        self.workers.keys().copied().collect()
    }

    /// Return the number of active workers.
    pub fn worker_count(&self) -> usize {
        self.workers.len()
    }

    /// Borrow runtime-owned static bytes.
    pub(crate) fn statics(&self) -> &engine::StaticSpace {
        &self.statics
    }

    /// Visit roots from every worker owned by this runtime.
    pub fn visit_roots(&mut self, roots: &mut impl heap::RootSink) -> RuntimeResult<()> {
        for worker in self.workers.values_mut() {
            worker.visit_roots(roots)?;
        }

        Ok(())
    }

    /// Start one incremental local-to-shared edge scan across all workers.
    pub(crate) fn start_shared_edge_scan(&mut self) {
        for worker in self.workers.values_mut() {
            worker.start_shared_edge_scan();
        }
    }

    /// Publish allocator-local shared heap buffers across all workers.
    pub(crate) fn flush_shared_allocators(&mut self) {
        let shared = self.shared.heap();

        for worker in self.workers.values_mut() {
            worker.flush_shared_allocator(shared);
        }
    }

    /// Finish the current local-to-shared edge scan across all workers.
    pub(crate) fn finish_shared_edge_scan(&mut self) {
        for worker in self.workers.values_mut() {
            worker.finish_shared_edge_scan();
        }
    }

    /// Return one immutable worker by id.
    pub fn worker(&self, worker_id: WorkerId) -> Option<&Worker> {
        self.workers.get(&worker_id).map(Box::as_ref)
    }

    /// Return one mutable worker by id.
    pub fn worker_mut(&mut self, worker_id: WorkerId) -> Option<&mut Worker> {
        self.workers.get_mut(&worker_id).map(Box::as_mut)
    }

    /// Run one closure with one worker and its runtime context.
    #[cfg(test)]
    pub(crate) fn with_worker_context<R>(
        &mut self,
        worker_id: WorkerId,
        callback: impl FnOnce(&HostSession, &SharedHeap, &engine::StaticSpace, &mut Worker) -> R,
    ) -> RuntimeResult<R> {
        let Runtime {
            host,
            shared,
            statics,
            workers,
            ..
        } = self;
        let worker = workers
            .get_mut(&worker_id)
            .map(Box::as_mut)
            .ok_or_else(|| {
                RuntimeError::WorkerNotFound {
                    worker_id: worker_id.0,
                }
                .boxed()
            })?;

        Ok(callback(host, shared, statics, worker))
    }

    /// Spawn one additional worker in this runtime.
    pub(crate) fn spawn_worker(
        &mut self,
        world: &mut WorldState,
        options: &RuntimeOptions,
        worker_options: WorkerOptions,
        engine: impl Into<Engine>,
    ) -> RuntimeResult<WorkerId> {
        // force runtime identity to stay shared across all workers in this runtime
        let mut options = options.clone();
        options.name = Some(self.name.clone());
        self.align_spawn_options_with_runtime(&mut options);

        // create one new worker attached to the runtime world
        let worker = Worker::new_in_runtime(
            self.process_args.clone(),
            &options,
            world,
            &self.shared,
            &self.statics,
            self.id,
            worker_options,
            engine,
        )?;

        self.insert_worker(worker)
    }

    /// Remove one worker from this runtime and return its boxed handle.
    pub fn remove_worker(&mut self, worker_id: WorkerId) -> RuntimeResult<Box<Worker>> {
        // remove the target worker from the registry
        let removed_worker = self.workers.remove(&worker_id).ok_or_else(|| {
            RuntimeError::WorkerNotFound {
                worker_id: worker_id.0,
            }
            .boxed()
        })?;

        // reject removing the last remaining worker
        if self.workers.is_empty() {
            self.workers.insert(worker_id, removed_worker);
            return Err(RuntimeError::LastWorkerRemoval.boxed());
        }

        // reject implicit default fallback to keep ownership explicit
        if self.default_worker_id == worker_id {
            self.workers.insert(worker_id, removed_worker);
            return Err(RuntimeError::DefaultWorkerRemoval.boxed());
        }

        Ok(removed_worker)
    }

    /// Attach a shared host poller for all workers in this runtime.
    pub fn set_poller(&mut self, poller: Box<dyn HostPoller>) {
        self.poller = poller;
    }

    /// Run one entrypoint through the default runtime worker event loop.
    pub(crate) fn run_entrypoint(
        &mut self,
        world: &mut WorldState,
        entry: &Entry,
        args: &[engine::Value],
    ) -> RuntimeResult<engine::Value> {
        self.run_entrypoint_for_worker(world, self.default_worker_id, entry, args)
    }

    /// Run one entrypoint through one explicit runtime worker event loop.
    pub(crate) fn run_entrypoint_for_worker(
        &mut self,
        world: &mut WorldState,
        worker_id: WorkerId,
        entry: &Entry,
        args: &[engine::Value],
    ) -> RuntimeResult<engine::Value> {
        let host = &self.host;
        let poller = self.poller.as_mut();
        let worker = self
            .workers
            .get_mut(&worker_id)
            .map(Box::as_mut)
            .ok_or_else(|| {
                RuntimeError::WorkerNotFound {
                    worker_id: worker_id.0,
                }
                .boxed()
            })?;
        worker.run_entrypoint_with_host_and_poller(
            world,
            &self.shared,
            &self.statics,
            host,
            entry,
            args,
            poller,
        )
    }

    /// Execute one runtime tick across all workers without advancing world time.
    pub(crate) fn tick(&mut self, world: &mut WorldState) -> RuntimeResult<TickResult> {
        // events
        let event_handled = self.poll_events(world)?;

        // workers
        let worker_count = self.workers.len();
        let start_index = if worker_count == 0 {
            0
        } else {
            self.next_worker_cursor % worker_count
        };
        let shared = &self.shared;
        let statics = &self.statics;
        let host = &self.host;
        let workers = &mut self.workers;

        for (worker_index, worker) in workers.values_mut().enumerate().skip(start_index) {
            if worker.tick(world, shared, statics, host)? {
                self.next_worker_cursor = (worker_index + 1) % worker_count;

                return Ok(TickResult::Progress);
            }
        }

        for (worker_index, worker) in workers.values_mut().enumerate().take(start_index) {
            if worker.tick(world, shared, statics, host)? {
                self.next_worker_cursor = worker_index + 1;

                return Ok(TickResult::Progress);
            }
        }

        // event handling counts as runnable scheduler progress
        if event_handled {
            return Ok(TickResult::Progress);
        }

        Ok(TickResult::Idle)
    }

    /// Create one runtime from one already-constructed default worker.
    fn new(
        process_args: Arc<[String]>,
        options: &RuntimeOptions,
        shared: SharedHeap,
        runtime_static: engine::StaticSpace,
        host: Arc<dyn Host>,
        default_worker: Worker,
    ) -> RuntimeResult<Self> {
        // seed runtime identity from runtime options
        let default_worker = Box::new(default_worker);
        let default_worker_id = default_worker.id;
        let runtime_id = default_worker.runtime_id;
        let worker_options = Arc::new(options.clone());
        let host_options = RuntimeHostOptions::from_runtime_options(options);
        let host = host_options.host_session(host, runtime_id);
        let poller = host_options.poller()?;
        let runtime_name = options
            .name
            .clone()
            .unwrap_or_else(|| "runtime".to_string());
        let mut workers = BTreeMap::new();
        workers.insert(default_worker_id, default_worker);

        // store runtime state
        Ok(Self {
            id: runtime_id,
            name: runtime_name,
            process_args,
            worker_options,
            host_options,
            host,
            poller,
            shared,
            statics: runtime_static,
            workers,
            default_worker_id,
            next_worker_cursor: 0,
        })
    }

    /// Insert one worker and return its id.
    fn insert_worker(&mut self, worker: Worker) -> RuntimeResult<WorkerId> {
        // derive one stable id from the underlying runtime context
        let worker = Box::new(worker);
        let worker_id = worker.id;

        // reject duplicate ids loudly: runtime ownership must stay one to one
        if self.workers.insert(worker_id, worker).is_some() {
            return Err(RuntimeError::WorkerAlreadyExists {
                worker_id: worker_id.0,
            }
            .boxed());
        }

        // active shared mark cycles must see the new worker roots
        if self.shared.is_marking() {
            self.join_mark(worker_id)?;
        }

        Ok(worker_id)
    }

    /// Join one worker to the active shared mark cycle.
    fn join_mark(&mut self, worker_id: WorkerId) -> RuntimeResult<()> {
        let worker = self
            .worker_mut(worker_id)
            .ok_or(RuntimeError::WorkerNotFound {
                worker_id: worker_id.0,
            })?;

        worker.start_shared_edge_scan();
        self.shared.join_mark(worker_id);

        Ok(())
    }

    /// Insert one restored worker image into this runtime.
    pub(crate) fn insert_restored_worker(&mut self, worker: Worker) -> RuntimeResult<WorkerId> {
        self.insert_worker(worker)
    }

    /// Return the next virtual deadline across all workers and simulation.
    pub(crate) fn next_deadline(&mut self, world: &mut WorldState) -> Option<Instant> {
        // current virtual timestamps: monotonic deadlines are projected onto wall time
        let wall_now = world.wall();
        let mono_now = world.mono();

        world.next_deadline(
            self.workers
                .values_mut()
                .map(|worker| worker.event_loop.next_deadline(wall_now, mono_now)),
        )
    }

    /// Drain due worker timers after the world advances time.
    pub(crate) fn collect_due_timers(
        &mut self,
        world: &mut WorldState,
    ) -> RuntimeResult<Vec<(RuntimeId, WorkerId, ScheduledTimer)>> {
        let wall_now = world.wall();
        let mono_now = world.mono();
        let mut worker_timers = Vec::new();

        for worker in self.workers.values_mut() {
            while let Some(timer) = worker.event_loop.pop_ready_timer(wall_now, mono_now)? {
                worker_timers.push((self.id, worker.id, timer));
            }
        }

        Ok(worker_timers)
    }

    /// Poll runtime-owned event sources and deliver events to worker event loops.
    #[inline(never)]
    fn poll_events(&mut self, world: &mut WorldState) -> RuntimeResult<bool> {
        let mut handled_any = false;
        let is_marking_shared = self.shared.is_marking();

        // host events
        let poll_result = self.host.poll(Some(0))?;
        for event in poll_result.events {
            if self.deliver_host_event(world, event, is_marking_shared)? {
                handled_any = true;
            }
        }

        // poller events
        let polled_events = self.poller.poll(Some(0))?;
        for event in polled_events {
            if self.deliver_poller_event(world, event, is_marking_shared)? {
                handled_any = true;
            }
        }

        Ok(handled_any)
    }

    /// Deliver one host event into matching worker event loops.
    fn deliver_host_event(
        &mut self,
        world: &mut WorldState,
        event: HostEvent,
        is_marking_shared: bool,
    ) -> RuntimeResult<bool> {
        let kind = event.kind();
        let shared = &self.shared;

        for (worker_id, worker) in &mut self.workers {
            if !worker.has_host_waiter(kind) {
                continue;
            }

            worker
                .event_loop
                .enqueue_wake(Wake::Host(HostWake::new(event.clone())));
            worker.hooks.on_ingress_enqueue(world);

            // shared mark: event can change direct worker roots without a worker tick
            if is_marking_shared {
                shared.queue_root_scan(*worker_id);
            }
        }

        Ok(true)
    }

    /// Deliver one poller event into matching worker event loops.
    fn deliver_poller_event(
        &mut self,
        world: &mut WorldState,
        event: PollerEvent,
        is_marking_shared: bool,
    ) -> RuntimeResult<bool> {
        let readiness = Readiness::from_poller_mask(event.mask);
        let shared = &self.shared;

        for (worker_id, worker) in &mut self.workers {
            if !worker.has_resource_waiter(event.resource_id, readiness) {
                continue;
            }

            worker
                .event_loop
                .enqueue_wake(Wake::Resource(ResourceWake::poller(event)));
            worker.hooks.on_ingress_enqueue(world);

            // shared mark: event can change direct worker roots without a worker tick
            if is_marking_shared {
                shared.queue_root_scan(*worker_id);
            }
        }

        Ok(true)
    }

    /// Deliver one batch of due worker-timer wakes.
    pub(crate) fn deliver_wakes(
        &mut self,
        world: &mut WorldState,
        wakes: Vec<WorkerWake>,
    ) -> RuntimeResult<()> {
        for wake in wakes {
            if wake.runtime_id != self.id {
                continue;
            }

            let worker = self.worker_mut(wake.worker_id).ok_or_else(|| {
                RuntimeError::WorkerNotFound {
                    worker_id: wake.worker_id.0,
                }
                .boxed()
            })?;
            worker.event_loop.enqueue_wake(wake.wake);
            worker.hooks.on_ingress_enqueue(world);
        }

        Ok(())
    }

    /// Align world-scoped options for workers spawned in one existing runtime.
    fn align_spawn_options_with_runtime(&self, options: &mut RuntimeOptions) {
        // world scoped settings: all workers in one runtime share one world
        options.scheduler = self.worker_options.scheduler.clone();
        options.time = self.worker_options.time.clone();
        options.random = self.worker_options.random.clone();
        options.trace = self.worker_options.trace.clone();
        options.conditions = self.worker_options.conditions.clone();
    }

    /// Capture one materialized runtime image and all owned worker images.
    pub(crate) fn capture_image(
        &mut self,
        mode: CaptureMode,
    ) -> RuntimeResult<(Arc<RuntimeImage>, BTreeMap<WorkerId, Arc<WorkerImage>>)> {
        // shared worker options
        let mut interned_options = vec![self.worker_options.clone()];

        // runtime metadata
        let runtime_image = Arc::new(RuntimeImage {
            default_worker_id: self.default_worker_id,
            process_args: self.process_args.clone(),
            worker_options: self.worker_options.clone(),
            host_options: self.host_options.clone(),
            shared_heap: self.shared.snapshot()?,
            statics: self.statics.clone(),
            next_worker_cursor: self.next_worker_cursor,
        });

        // worker images
        let mut worker_images = BTreeMap::new();
        for worker in self.workers.values_mut() {
            let mut image = worker.capture_image(mode, &self.shared, &self.statics)?;

            // collapse one shared options payload across matching workers
            if let Some(options) = image.options.explicit_options() {
                image.options = if options.as_ref() == self.worker_options.as_ref() {
                    WorkerOptionsImage::Shared
                } else {
                    WorkerOptionsImage::Explicit(Self::intern_worker_options(
                        &mut interned_options,
                        options.clone(),
                    ))
                };
            }

            if worker_images.insert(worker.id, Arc::new(image)).is_some() {
                return Err(RuntimeError::DuplicateWorkerImage {
                    runtime_id: self.id.0,
                    worker_id: worker.id.0,
                }
                .boxed());
            }
        }

        Ok((runtime_image, worker_images))
    }

    /// Capture one worker image from this runtime.
    pub(crate) fn capture_worker_image(
        &mut self,
        mode: CaptureMode,
        worker_id: WorkerId,
    ) -> RuntimeResult<WorkerImage> {
        let worker = self.workers.get_mut(&worker_id).ok_or_else(|| {
            RuntimeError::WorkerNotFound {
                worker_id: worker_id.0,
            }
            .boxed()
        })?;

        worker.capture_image(mode, &self.shared, &self.statics)
    }

    /// Fork one live runtime when all owned workers are quiescent.
    pub(crate) fn try_fork(
        &mut self,
        execution_mode: ExecutionMode,
        host: Arc<dyn Host>,
        collector: Arc<SharedCollector>,
    ) -> RuntimeResult<Option<Self>> {
        let shared = self.shared.fork(collector)?;

        // fork each owned worker first
        let mut workers = BTreeMap::new();
        for (worker_id, worker) in &mut self.workers {
            let shared_gc_worker = shared.register_collector_worker();
            let Some(worker) =
                worker.try_fork(execution_mode, &shared, &self.statics, shared_gc_worker)?
            else {
                return Ok(None);
            };
            workers.insert(*worker_id, Box::new(worker));
        }

        // rebuild one fresh host integration boundary
        let host = self.host_options.host_session(host, self.id);
        let poller = self.host_options.poller()?;

        Ok(Some(Self {
            id: self.id,
            name: self.name.clone(),
            process_args: self.process_args.clone(),
            worker_options: self.worker_options.clone(),
            host_options: self.host_options.clone(),
            host,
            poller,
            shared,
            statics: self.statics.clone(),
            workers,
            default_worker_id: self.default_worker_id,
            next_worker_cursor: self.next_worker_cursor,
        }))
    }

    /// Restore one runtime from one materialized runtime image.
    pub(crate) fn from_image(
        world: &mut WorldState,
        allocator: Arc<heap::Allocator>,
        collector: Arc<SharedCollector>,
        host: Arc<dyn Host>,
        runtime_id: RuntimeId,
        runtime_name: String,
        image: &RuntimeImage,
        worker_names: &BTreeMap<WorkerId, String>,
        worker_images: &BTreeMap<WorkerId, Arc<WorkerImage>>,
        rebind_context: Option<&ResourceRebinders>,
    ) -> RuntimeResult<Self> {
        // runtime-wide reconstructed state
        let process_args = image.process_args.clone();
        let host = image.host_options.host_session(host, runtime_id);
        let poller = image.host_options.poller()?;
        let shared = SharedHeap::from_snapshot(
            &image.shared_heap,
            &image.worker_options,
            allocator,
            collector,
        )?;
        let statics = image.statics.clone();
        let mut workers = BTreeMap::new();

        // workers
        for (worker_id, worker_image) in worker_images {
            let worker_name = worker_names.get(worker_id).ok_or_else(|| {
                RuntimeError::WorkerNotFound {
                    worker_id: worker_id.0,
                }
                .boxed()
            })?;
            let worker = Worker::from_image(
                world,
                &shared,
                &statics,
                runtime_id,
                *worker_id,
                worker_name.clone(),
                process_args.clone(),
                worker_image.as_ref(),
                Some(&image.worker_options),
                rebind_context,
            )?;
            if workers.insert(*worker_id, Box::new(worker)).is_some() {
                return Err(RuntimeError::DuplicateWorkerImage {
                    runtime_id: runtime_id.0,
                    worker_id: worker_id.0,
                }
                .boxed());
            }
        }

        // validate the default worker after reconstruction
        if !workers.contains_key(&image.default_worker_id) {
            return Err(RuntimeError::DefaultWorkerMissing {
                runtime_id: runtime_id.0,
                worker_id: image.default_worker_id.0,
            }
            .boxed());
        }

        Ok(Self {
            id: runtime_id,
            name: runtime_name,
            process_args,
            worker_options: image.worker_options.clone(),
            host_options: image.host_options.clone(),
            host,
            poller,
            shared,
            statics,
            workers,
            default_worker_id: image.default_worker_id,
            next_worker_cursor: image.next_worker_cursor,
        })
    }

    /// Reuse one explicit options payload when it already exists.
    fn intern_worker_options(
        interned_options: &mut Vec<Arc<RuntimeOptions>>,
        options: Arc<RuntimeOptions>,
    ) -> Arc<RuntimeOptions> {
        // reuse one existing payload before cloning more options
        if let Some(existing_options) = interned_options
            .iter()
            .find(|existing_options| existing_options.as_ref() == options.as_ref())
        {
            return existing_options.clone();
        }

        interned_options.push(options.clone());

        options
    }
}

#[cfg(test)]
mod tests {
    use super::Runtime;
    use crate::host::{
        HostEvent, HostEventKind, HostSession, LifecycleEvent, LifecycleSourceKind, LifecycleState,
    };
    use crate::runtime::tests::{TestEngine, TestWorldRuntime, start_worker_continuation};
    use crate::runtime::{SharedHeap, TickResult, Worker, WorkerOptions};
    use crate::world::World;
    use destack_engine as engine;
    use destack_heap::{AllocationShape, Payload};
    use destack_mir::ReferenceMap;
    use destack_workspace::RuntimeOptions;

    /// Allocate one shared byte payload for runtime tests.
    fn allocate_shared_bytes(
        heap: &destack_heap::SharedHeap,
        bytes: &[u8],
    ) -> destack_heap::HeapResult<destack_heap::SharedHeapReference> {
        let reference_map = ReferenceMap::empty();
        let shape = AllocationShape::new(bytes.len(), 1, &reference_map);
        let layout = heap.allocation_layout(shape);
        let mut allocator = heap.allocator();
        let worker = heap.register_collector_worker();

        heap.allocate(&worker, &mut allocator, &layout, Payload::Bytes(bytes))
    }

    /// Build runtime-owned shared heap state for one test world.
    fn runtime_shared_heap(world: &World, options: &RuntimeOptions) -> SharedHeap {
        let history = world.history.read();

        SharedHeap::new(history.allocator(), history.collector(), options)
            .expect("runtime shared heap should construct")
    }

    /// Spawned workers inherit runtime source graph conditions.
    #[test]
    fn test_spawn_worker_inherits_runtime_conditions() {
        let mut options = RuntimeOptions::default();
        options.conditions.modes.insert("test".to_string());
        options.conditions.roles.insert("server".to_string());
        options.conditions.features.insert("payments".to_string());

        let mut runtime =
            TestWorldRuntime::with_options_and_engine(&options, TestEngine::default());
        let worker_id = runtime.spawn_worker(TestEngine::default());

        runtime.with_worker_mut(worker_id, |worker| {
            assert!(worker.options.conditions.contains_mode("test"));
            assert!(worker.options.conditions.contains_role("server"));
            assert!(worker.options.conditions.contains_feature("payments"));
        });
    }

    /// Queue one shared direct-root rescan when events mutate worker state during marking.
    #[test]
    fn test_deliver_host_event_queues_shared_root_rescan_during_mark() {
        let options = RuntimeOptions::default();
        let mut world = World::from_options(&options).expect("world should construct");
        let shared = runtime_shared_heap(&world, &options);
        let world_state = &mut world.state;

        let mut worker = Worker::new_in_world(
            Vec::new(),
            &options,
            world_state,
            &shared,
            &engine::StaticSpace::empty(),
            WorkerOptions::default(),
            TestEngine::default(),
        )
        .expect("worker should construct");
        let worker_id = worker.id;
        let shared_root = allocate_shared_bytes(shared.heap(), &[0xA1])
            .expect("shared allocation should succeed");
        let host = HostSession::new(
            crate::host::default_compile_target_host(),
            worker.runtime_id,
        );

        // suspended host state: the shared root only lives through the event loop
        let continuation = start_worker_continuation(
            &mut worker,
            &host,
            world_state,
            &shared,
            &engine::StaticSpace::empty(),
            "test.task",
            7,
        );

        worker
            .add_host_waiter(
                HostEventKind::Lifecycle,
                continuation,
                engine::Value::SharedHeapReference(shared_root),
                0,
            )
            .expect("host waiter should register");

        let mut runtime = Runtime::new(
            Vec::new().into(),
            &options,
            shared,
            engine::StaticSpace::empty(),
            crate::host::default_compile_target_host(),
            worker,
        )
        .expect("runtime should construct");

        // active shared mark
        runtime.shared.heap().request_gc();
        runtime
            .tick_shared_gc()
            .expect("shared gc should start through runtime roots");

        assert!(runtime.shared.is_marking());

        // initial publication drains before events mutate roots
        let outcome = runtime
            .tick(world_state)
            .expect("runtime tick should publish initial roots");
        assert_eq!(outcome, TickResult::Progress);
        assert!(
            runtime
                .shared
                .roots()
                .pending_root_epoch(worker_id)
                .is_none()
        );

        // events should requeue the touched worker even before it ticks
        let handled = runtime
            .deliver_host_event(
                world_state,
                HostEvent::Lifecycle(LifecycleEvent {
                    source_kind: LifecycleSourceKind::Application,
                    state: LifecycleState::Running,
                }),
                true,
            )
            .expect("event delivery should succeed");

        assert!(handled);
        assert!(
            runtime
                .shared
                .roots()
                .pending_root_epoch(worker_id)
                .is_some()
        );
    }

    /// Publish direct shared roots from the owning worker checkpoint during marking.
    #[test]
    fn test_runtime_tick_publishes_pending_shared_roots_from_worker() {
        let options = RuntimeOptions::default();
        let mut world = World::from_options(&options).expect("world should construct");
        let shared = runtime_shared_heap(&world, &options);
        let world_state = &mut world.state;

        let mut worker = Worker::new_in_world(
            Vec::new(),
            &options,
            world_state,
            &shared,
            &engine::StaticSpace::empty(),
            WorkerOptions::default(),
            TestEngine::default(),
        )
        .expect("worker should construct");
        let worker_id = worker.id;
        let shared_root = allocate_shared_bytes(shared.heap(), &[0xB2])
            .expect("shared allocation should succeed");
        let host = HostSession::new(
            crate::host::default_compile_target_host(),
            worker.runtime_id,
        );

        // suspended host state: the shared root only lives through the event loop
        let continuation = start_worker_continuation(
            &mut worker,
            &host,
            world_state,
            &shared,
            &engine::StaticSpace::empty(),
            "test.task",
            9,
        );

        worker
            .add_host_waiter(
                HostEventKind::Lifecycle,
                continuation,
                engine::Value::SharedHeapReference(shared_root),
                0,
            )
            .expect("host waiter should register");

        let mut runtime = Runtime::new(
            Vec::new().into(),
            &options,
            shared,
            engine::StaticSpace::empty(),
            crate::host::default_compile_target_host(),
            worker,
        )
        .expect("runtime should construct");

        // active shared mark
        runtime.shared.heap().request_gc();
        runtime
            .tick_shared_gc()
            .expect("shared gc should start through runtime roots");
        runtime.shared.queue_root_scan(worker_id);

        assert!(
            runtime
                .shared
                .roots()
                .pending_root_epoch(worker_id)
                .is_some()
        );

        // one runtime tick should let the owning worker publish its direct roots
        let outcome = runtime
            .tick(world_state)
            .expect("runtime tick should succeed");

        assert_eq!(outcome, TickResult::Progress);
        assert!(
            runtime
                .shared
                .roots()
                .pending_root_epoch(worker_id)
                .is_none()
        );
        assert_eq!(
            runtime.shared.roots().roots_snapshot().as_ref(),
            &[shared_root]
        );
    }
}
