use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::host::core::HostQueue;
use crate::host::poller::{HostPoller, PollerEvent};
use crate::host::resource::ResourceRebinders;
use crate::host::{Host, HostEvent};
use crate::runtime::SharedCollector;
use crate::runtime::engine::{Engine, Entry};
use crate::runtime::heap::RuntimeHeap;
use crate::runtime::scheduler::{
    HostWake, Readiness, ResourceWake, ScheduledTimer, TickResult, Wake,
};
use crate::runtime::time::Instant;
use crate::runtime::worker::{Worker, WorkerId, WorkerImage, WorkerOptions, WorkerOptionsImage};
use crate::world::{RuntimeId, WorkerWake, WorldState};
use destack_core::CaptureMode;
use destack_engine as engine;
use destack_heap as heap;
use destack_workspace::{Environment, ExecutionMode, RuntimeOptions};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::Arc;

/// Runtime container that owns one or more workers in one shared world.
pub struct Runtime {
    /// Runtime identifier in world topology.
    id: RuntimeId,
    /// Immutable ambient environment shared by newly spawned workers.
    pub(crate) environment: Arc<Environment>,
    /// Runtime options captured for worker defaults and reconstruction.
    options: Arc<RuntimeOptions>,
    /// Runtime-owned shared heap and GC state.
    pub(crate) heap: RuntimeHeap,
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
    /// Runtime launch environment.
    pub environment: Arc<Environment>,
    /// Runtime options captured for worker defaults and reconstruction.
    pub options: Arc<RuntimeOptions>,
    /// Captured runtime-owned shared heap state.
    pub shared_heap: heap::SharedHeapSnapshot,
    /// Captured runtime-owned static bytes.
    pub statics: engine::StaticSpace,
    /// Default worker identifier for this runtime.
    pub default_worker_id: WorkerId,
    /// The next worker slot to schedule first.
    pub next_worker_cursor: usize,
}

impl std::fmt::Debug for Runtime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Runtime")
            .field("runtime_id", &self.id)
            .field("environment", &self.environment)
            .field("options", &self.options)
            .field("heap", &self.heap)
            .field("workers", &self.workers)
            .field("default_worker_id", &self.default_worker_id)
            .field("next_worker_cursor", &self.next_worker_cursor)
            .finish()
    }
}

impl Runtime {
    /// Create a runtime with one default worker in one explicit shared world.
    pub(crate) fn from_options_in_world(
        environment: impl Into<Arc<Environment>>,
        options: &RuntimeOptions,
        world: &mut WorldState,
        allocator: Arc<heap::Allocator>,
        collector: Arc<SharedCollector>,
        engine: impl Into<Engine>,
    ) -> RuntimeResult<Self> {
        let environment = environment.into();
        let engine = engine.into();
        let trace_table = engine.trace_table()?;
        let shared = RuntimeHeap::new(allocator, collector, options, trace_table)?;
        let statics = engine::StaticSpace::empty();
        let default_worker = Worker::new_in_world(
            environment.clone(),
            options,
            world,
            &shared,
            &statics,
            WorkerOptions::default(),
            engine,
        )?;
        let runtime = Self::new(environment, options, shared, statics, default_worker)?;

        Ok(runtime)
    }

    /// Return the current default worker id.
    pub fn default_worker_id(&self) -> WorkerId {
        self.default_worker_id
    }

    /// Return the world topology runtime id.
    pub fn runtime_id(&self) -> RuntimeId {
        self.id
    }

    /// Set one explicit default worker.
    pub fn set_default_worker(&mut self, worker_id: WorkerId) -> RuntimeResult<()> {
        if self.workers.contains_key(&worker_id) {
            self.default_worker_id = worker_id;
            return Ok(());
        }

        Err(RuntimeError::worker_not_found(worker_id.0).boxed())
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

    /// Publish worker-local shared heap buffers across all workers.
    pub(crate) fn flush_shared_caches(&mut self) {
        let shared = self.heap.shared.as_ref();

        for worker in self.workers.values_mut() {
            worker.flush_shared_cache(shared);
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

    /// Run one closure with one worker.
    #[cfg(test)]
    pub(crate) fn with_worker<R>(
        &mut self,
        worker_id: WorkerId,
        callback: impl FnOnce(&RuntimeHeap, &engine::StaticSpace, &mut Worker) -> R,
    ) -> RuntimeResult<R> {
        let Runtime {
            heap,
            statics,
            workers,
            ..
        } = self;
        let worker = workers
            .get_mut(&worker_id)
            .map(Box::as_mut)
            .ok_or_else(|| RuntimeError::worker_not_found(worker_id.0).boxed())?;

        Ok(callback(heap, statics, worker))
    }

    /// Spawn one additional worker in this runtime.
    pub(crate) fn spawn_worker(
        &mut self,
        world: &mut WorldState,
        worker_options: WorkerOptions,
        engine: impl Into<Engine>,
    ) -> RuntimeResult<WorkerId> {
        // create one new worker attached to the runtime world
        let worker = Worker::new_in_runtime(
            self.environment.clone(),
            &self.options,
            world,
            &self.heap,
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
        let removed_worker = self
            .workers
            .remove(&worker_id)
            .ok_or_else(|| RuntimeError::worker_not_found(worker_id.0).boxed())?;

        // reject removing the last remaining worker
        if self.workers.is_empty() {
            self.workers.insert(worker_id, removed_worker);
            return Err(RuntimeError::last_worker_removal().boxed());
        }

        // reject implicit default fallback to keep ownership explicit
        if self.default_worker_id == worker_id {
            self.workers.insert(worker_id, removed_worker);
            return Err(RuntimeError::default_worker_removal().boxed());
        }

        Ok(removed_worker)
    }

    /// Run one entrypoint through one runtime worker event loop.
    pub(crate) fn run_entrypoint(
        &mut self,
        world: &mut WorldState,
        host: &dyn Host,
        host_queue: &HostQueue,
        poller: &mut dyn HostPoller,
        worker_id: WorkerId,
        entry: &Entry,
        args: &[engine::Value],
    ) -> RuntimeResult<engine::Value> {
        let worker = self
            .workers
            .get_mut(&worker_id)
            .map(Box::as_mut)
            .ok_or_else(|| RuntimeError::worker_not_found(worker_id.0).boxed())?;
        worker.run_entrypoint(
            world,
            &self.heap,
            &self.statics,
            host,
            host_queue,
            entry,
            args,
            poller,
        )
    }

    /// Execute one runtime tick across all workers without advancing world time.
    pub(crate) fn tick(
        &mut self,
        world: &mut WorldState,
        host: &dyn Host,
        host_queue: &HostQueue,
    ) -> RuntimeResult<TickResult> {
        // workers
        let worker_count = self.workers.len();
        let start_index = if worker_count == 0 {
            0
        } else {
            self.next_worker_cursor % worker_count
        };
        let shared = &self.heap;
        let statics = &self.statics;
        let workers = &mut self.workers;

        for (worker_index, worker) in workers.values_mut().enumerate().skip(start_index) {
            if worker.tick(world, shared, statics, host, host_queue)? {
                self.next_worker_cursor = (worker_index + 1) % worker_count;

                return Ok(TickResult::Progress);
            }
        }

        for (worker_index, worker) in workers.values_mut().enumerate().take(start_index) {
            if worker.tick(world, shared, statics, host, host_queue)? {
                self.next_worker_cursor = worker_index + 1;

                return Ok(TickResult::Progress);
            }
        }

        Ok(TickResult::Idle)
    }

    /// Create one runtime from one already-constructed default worker.
    fn new(
        environment: Arc<Environment>,
        options: &RuntimeOptions,
        shared: RuntimeHeap,
        runtime_static: engine::StaticSpace,
        default_worker: Worker,
    ) -> RuntimeResult<Self> {
        // seed runtime identity from runtime options
        let default_worker = Box::new(default_worker);
        let default_worker_id = default_worker.id;
        let runtime_id = default_worker.runtime_id;
        let options = Arc::new(options.clone());
        let mut workers = BTreeMap::new();
        workers.insert(default_worker_id, default_worker);

        // store runtime state
        Ok(Self {
            id: runtime_id,
            environment,
            options,
            heap: shared,
            statics: runtime_static,
            workers,
            default_worker_id,
            next_worker_cursor: 0,
        })
    }

    /// Insert one worker and return its id.
    fn insert_worker(&mut self, worker: Worker) -> RuntimeResult<WorkerId> {
        // keep worker ownership one to one
        let worker = Box::new(worker);
        let worker_id = worker.id;

        // reject duplicate ids loudly: runtime ownership must stay one to one
        if self.workers.insert(worker_id, worker).is_some() {
            return Err(RuntimeError::worker_already_exists(worker_id.0).boxed());
        }

        // active shared mark cycles must see the new worker roots
        if self.heap.is_marking() {
            self.join_mark(worker_id)?;
        }

        Ok(worker_id)
    }

    /// Join one worker to the active shared mark cycle.
    fn join_mark(&mut self, worker_id: WorkerId) -> RuntimeResult<()> {
        let worker = self
            .worker_mut(worker_id)
            .ok_or(RuntimeError::worker_not_found(worker_id.0))?;

        worker.start_shared_edge_scan();
        self.heap.join_mark(worker_id);

        Ok(())
    }

    /// Insert one restored worker image into this runtime.
    pub(crate) fn insert_restored_worker(&mut self, worker: Worker) -> RuntimeResult<WorkerId> {
        self.insert_worker(worker)
    }

    /// Return the next virtual deadline across all workers.
    pub(crate) fn next_deadline(&mut self, world: &mut WorldState) -> Option<Instant> {
        // current virtual timestamps: monotonic deadlines are projected onto wall time
        let wall_now = world.wall();
        let mono_now = world.mono();

        self.workers
            .values_mut()
            .filter_map(|worker| worker.event_loop.next_deadline(wall_now, mono_now))
            .min()
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

    /// Deliver externally collected events to worker event loops.
    pub(crate) fn deliver_events(
        &mut self,
        host_events: &[HostEvent],
        poller_events: &[PollerEvent],
    ) -> RuntimeResult<bool> {
        let mut handled_any = false;
        let is_marking_shared = self.heap.is_marking();

        // host events
        for event in host_events {
            if self.deliver_host_event(event.clone(), is_marking_shared)? {
                handled_any = true;
            }
        }

        // poller events
        for event in poller_events {
            if self.deliver_poller_event(*event, is_marking_shared)? {
                handled_any = true;
            }
        }

        Ok(handled_any)
    }

    /// Deliver one host event into matching worker event loops.
    pub(crate) fn deliver_host_event(
        &mut self,
        event: HostEvent,
        is_marking_shared: bool,
    ) -> RuntimeResult<bool> {
        let kind = event.kind();
        let shared = &self.heap;
        let mut handled_any = false;

        for (worker_id, worker) in &mut self.workers {
            if !worker.event_loop.has_host_waiter(kind) {
                continue;
            }

            handled_any = true;
            worker
                .event_loop
                .enqueue_wake(Wake::Host(HostWake::new(event.clone())));

            // shared mark: event can change direct worker roots without a worker tick
            if is_marking_shared {
                shared.queue_root_scan(*worker_id);
            }
        }

        Ok(handled_any)
    }

    /// Deliver one poller event into matching worker event loops.
    pub(crate) fn deliver_poller_event(
        &mut self,
        event: PollerEvent,
        is_marking_shared: bool,
    ) -> RuntimeResult<bool> {
        let readiness = Readiness::from_poller_mask(event.mask);
        let shared = &self.heap;
        let mut handled_any = false;

        for (worker_id, worker) in &mut self.workers {
            if !worker
                .event_loop
                .has_resource_waiter(event.resource_id, readiness)
            {
                continue;
            }

            handled_any = true;
            worker
                .event_loop
                .enqueue_wake(Wake::Resource(ResourceWake::poller(event)));

            // shared mark: event can change direct worker roots without a worker tick
            if is_marking_shared {
                shared.queue_root_scan(*worker_id);
            }
        }

        Ok(handled_any)
    }

    /// Deliver one batch of due worker-timer wakes.
    pub(crate) fn deliver_wakes(&mut self, wakes: Vec<WorkerWake>) -> RuntimeResult<()> {
        for wake in wakes {
            if wake.runtime_id != self.id {
                continue;
            }

            let worker = self
                .worker_mut(wake.worker_id)
                .ok_or_else(|| RuntimeError::worker_not_found(wake.worker_id.0).boxed())?;
            worker.event_loop.enqueue_wake(wake.wake);
        }

        Ok(())
    }

    /// Capture one materialized runtime image and all owned worker images.
    pub(crate) fn capture_image(
        &mut self,
        mode: CaptureMode,
    ) -> RuntimeResult<(Arc<RuntimeImage>, BTreeMap<WorkerId, Arc<WorkerImage>>)> {
        // publish worker-local shared allocations before capturing the shared heap
        self.flush_shared_caches();

        // shared worker options
        let mut interned_options = vec![self.options.clone()];

        // runtime metadata
        let runtime_image = Arc::new(RuntimeImage {
            default_worker_id: self.default_worker_id,
            environment: self.environment.clone(),
            options: self.options.clone(),
            shared_heap: self.heap.snapshot()?,
            statics: self.statics.clone(),
            next_worker_cursor: self.next_worker_cursor,
        });

        // worker images
        let mut worker_images = BTreeMap::new();
        for worker in self.workers.values_mut() {
            let mut image = worker.capture_image(mode, &self.heap, &self.statics)?;

            // collapse one shared options payload across matching workers
            if let Some(options) = image.options.explicit_options() {
                image.options = if options.as_ref() == self.options.as_ref() {
                    WorkerOptionsImage::Shared
                } else {
                    WorkerOptionsImage::Explicit(Self::intern_options(
                        &mut interned_options,
                        options.clone(),
                    ))
                };
            }

            if worker_images.insert(worker.id, Arc::new(image)).is_some() {
                return Err(RuntimeError::duplicate_worker_image(self.id.0, worker.id.0).boxed());
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
        let worker = self
            .workers
            .get_mut(&worker_id)
            .ok_or_else(|| RuntimeError::worker_not_found(worker_id.0).boxed())?;

        worker.capture_image(mode, &self.heap, &self.statics)
    }

    /// Fork one live runtime when all owned workers are quiescent.
    pub(crate) fn try_fork(
        &mut self,
        execution_mode: ExecutionMode,
        collector: Arc<SharedCollector>,
    ) -> RuntimeResult<Option<Self>> {
        let shared = self.heap.fork(collector)?;

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

        Ok(Some(Self {
            id: self.id,
            environment: self.environment.clone(),
            options: self.options.clone(),
            heap: shared,
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
        runtime_id: RuntimeId,
        image: &RuntimeImage,
        worker_images: &BTreeMap<WorkerId, Arc<WorkerImage>>,
        rebind_context: Option<&ResourceRebinders>,
    ) -> RuntimeResult<Self> {
        // runtime-wide reconstructed state
        let environment = image.environment.clone();
        let Some((_, first_worker_image)) = worker_images.first_key_value() else {
            return Err(RuntimeError::Internal {
                message: "runtime image has no workers".to_string(),
            }
            .boxed());
        };
        let first_engine = Engine::from_image(&first_worker_image.engine_image)?;
        let trace_table = first_engine.trace_table()?;
        let shared = RuntimeHeap::from_snapshot(
            &image.shared_heap,
            &image.options,
            allocator,
            collector,
            trace_table,
        )?;
        let statics = image.statics.clone();
        let mut workers = BTreeMap::new();

        // workers
        for (worker_id, worker_image) in worker_images {
            let worker = Worker::from_image(
                world,
                &shared,
                &statics,
                runtime_id,
                *worker_id,
                environment.clone(),
                worker_image.as_ref(),
                Some(&image.options),
                rebind_context,
            )?;
            if workers.insert(*worker_id, Box::new(worker)).is_some() {
                return Err(
                    RuntimeError::duplicate_worker_image(runtime_id.0, worker_id.0).boxed(),
                );
            }
        }

        // validate the default worker after reconstruction
        if !workers.contains_key(&image.default_worker_id) {
            return Err(RuntimeError::default_worker_missing(
                runtime_id.0,
                image.default_worker_id.0,
            )
            .boxed());
        }

        Ok(Self {
            id: runtime_id,
            environment,
            options: image.options.clone(),
            heap: shared,
            statics,
            workers,
            default_worker_id: image.default_worker_id,
            next_worker_cursor: image.next_worker_cursor,
        })
    }

    /// Reuse one explicit options payload when it already exists.
    fn intern_options(
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
    use std::sync::Arc;

    use super::Runtime;
    use crate::host::core::HostQueue;
    use crate::host::{
        HostEvent, HostEventKind, LifecycleEvent, LifecycleSourceKind, LifecycleState,
        compile_target_host,
    };
    use crate::runtime::tests::{TestEngine, TestWorldRuntime, start_worker_continuation};
    use crate::runtime::{RuntimeHeap, TickResult, Worker, WorkerOptions};
    use crate::world::World;
    use destack_core::CaptureMode;
    use destack_engine as engine;
    use destack_heap::AllocationShape;
    use destack_mir::{TraceMap, TraceTable};
    use destack_workspace::{Environment, RuntimeOptions};

    /// Allocate one shared byte payload for runtime tests.
    fn allocate_shared_bytes(
        heap: &destack_heap::SharedHeap,
        bytes: &[u8],
    ) -> destack_heap::HeapResult<destack_heap::SharedHeapReference> {
        let trace_map = TraceMap::Empty;
        let shape = AllocationShape::new(bytes.len(), 1, None, &trace_map);
        let mut allocator = heap.allocation_cache();
        let worker = heap.register_collector_worker();

        heap.allocate_dynamic_bytes(&worker, &mut allocator, shape, bytes, &TraceTable::new())
    }

    /// Build runtime-owned shared heap state for one test world.
    fn runtime_shared_heap(world: &World, options: &RuntimeOptions) -> RuntimeHeap {
        RuntimeHeap::new(
            world.memory.allocator.clone(),
            world.memory.shared_collector.clone(),
            options,
            Arc::new(TraceTable::new()),
        )
        .expect("runtime shared heap should construct")
    }

    /// Spawned workers inherit runtime source graph conditions.
    #[test]
    fn test_spawn_worker_inherits_runtime_conditions() {
        let mut options = RuntimeOptions::default();
        options.conditions.modes.insert("test".to_string());
        options.conditions.roles.insert("server".to_string());
        options.conditions.features.insert("payments".to_string());

        let mut runtime = TestWorldRuntime::build(&options, TestEngine::default());
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
        let mut world =
            World::new(&options, Environment::default()).expect("world should construct");
        let shared = runtime_shared_heap(&world, &options);
        let world_state = &mut world.state;

        let mut worker = Worker::new_in_world(
            destack_workspace::Environment::default(),
            &options,
            world_state,
            &shared,
            &engine::StaticSpace::empty(),
            WorkerOptions::default(),
            TestEngine::default(),
        )
        .expect("worker should construct");
        let worker_id = worker.id;
        let shared_root = allocate_shared_bytes(shared.shared.as_ref(), &[0xA1])
            .expect("shared allocation should succeed");
        let host = compile_target_host(None);
        let host_queue = HostQueue::new();

        // suspended host state: the shared root only lives through the event loop
        let continuation = start_worker_continuation(
            &mut worker,
            host.as_ref(),
            &host_queue,
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
            Arc::new(destack_workspace::Environment::default()),
            &options,
            shared,
            engine::StaticSpace::empty(),
            worker,
        )
        .expect("runtime should construct");

        // active shared mark
        runtime.heap.shared.request_gc();
        runtime
            .tick_shared_gc()
            .expect("shared gc should start through runtime roots");

        assert!(runtime.heap.is_marking());

        // initial publication drains before events mutate roots
        let outcome = runtime
            .tick(world_state, host.as_ref(), &host_queue)
            .expect("runtime tick should publish initial roots");
        assert_eq!(outcome, TickResult::Progress);
        assert!(runtime.heap.roots().pending_root_epoch(worker_id).is_none());

        // events should requeue the touched worker even before it ticks
        let handled = runtime
            .deliver_host_event(
                HostEvent::Lifecycle(LifecycleEvent {
                    source_kind: LifecycleSourceKind::Application,
                    state: LifecycleState::Running,
                }),
                true,
            )
            .expect("event delivery should succeed");

        assert!(handled);
        assert!(runtime.heap.roots().pending_root_epoch(worker_id).is_some());
    }

    /// Capture publishes worker-local shared allocation caches before imaging shared heap.
    #[test]
    fn test_capture_image_flushes_worker_shared_cache() {
        let options = RuntimeOptions::default();
        let mut world =
            World::new(&options, Environment::default()).expect("world should construct");
        let shared = runtime_shared_heap(&world, &options);
        let world_state = &mut world.state;
        let mut worker = Worker::new_in_world(
            destack_workspace::Environment::default(),
            &options,
            world_state,
            &shared,
            &engine::StaticSpace::empty(),
            WorkerOptions::default(),
            TestEngine::default(),
        )
        .expect("worker should construct");
        let trace_map = TraceMap::Empty;
        let shape = AllocationShape::new(16, 1, None, &trace_map);

        // allocate through the worker cache without reaching a normal flush point
        let _reference = shared
            .shared
            .allocate_dynamic_zeroed(
                &worker.shared_gc_worker,
                &mut worker.shared_cache,
                shape,
                shared.trace_table(),
            )
            .expect("shared allocation should succeed");

        assert_eq!(shared.shared.usage().allocation_count, 0);

        let mut runtime = Runtime::new(
            Arc::new(destack_workspace::Environment::default()),
            &options,
            shared,
            engine::StaticSpace::empty(),
            worker,
        )
        .expect("runtime should construct");

        // runtime capture must materialize the worker-local shared run
        let _image = runtime
            .capture_image(CaptureMode::Suspend)
            .expect("runtime image should capture");

        assert_eq!(runtime.heap.shared.usage().allocation_count, 1);
    }

    /// Publish direct shared roots from the owning worker checkpoint during marking.
    #[test]
    fn test_runtime_tick_publishes_pending_shared_roots_from_worker() {
        let options = RuntimeOptions::default();
        let mut world =
            World::new(&options, Environment::default()).expect("world should construct");
        let shared = runtime_shared_heap(&world, &options);
        let world_state = &mut world.state;

        let mut worker = Worker::new_in_world(
            destack_workspace::Environment::default(),
            &options,
            world_state,
            &shared,
            &engine::StaticSpace::empty(),
            WorkerOptions::default(),
            TestEngine::default(),
        )
        .expect("worker should construct");
        let worker_id = worker.id;
        let shared_root = allocate_shared_bytes(shared.shared.as_ref(), &[0xB2])
            .expect("shared allocation should succeed");
        let host = compile_target_host(None);
        let host_queue = HostQueue::new();

        // suspended host state: the shared root only lives through the event loop
        let continuation = start_worker_continuation(
            &mut worker,
            host.as_ref(),
            &host_queue,
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
            Arc::new(destack_workspace::Environment::default()),
            &options,
            shared,
            engine::StaticSpace::empty(),
            worker,
        )
        .expect("runtime should construct");

        // active shared mark
        runtime.heap.shared.request_gc();
        runtime
            .tick_shared_gc()
            .expect("shared gc should start through runtime roots");
        runtime.heap.queue_root_scan(worker_id);

        assert!(runtime.heap.roots().pending_root_epoch(worker_id).is_some());

        // one runtime tick should let the owning worker publish its direct roots
        let outcome = runtime
            .tick(world_state, host.as_ref(), &host_queue)
            .expect("runtime tick should succeed");

        assert_eq!(outcome, TickResult::Progress);
        assert!(runtime.heap.roots().pending_root_epoch(worker_id).is_none());
        assert_eq!(
            runtime.heap.roots().roots_snapshot().as_ref(),
            &[shared_root]
        );
    }
}
