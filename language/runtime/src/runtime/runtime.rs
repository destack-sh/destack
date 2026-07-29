use std::collections::BTreeMap;
use std::fmt;
use std::sync::Arc;

use destack_artifact::ConditionSet;
use destack_heap as heap;
use destack_memory::MemoryMap;
use destack_program as program;
use destack_repository::{Environment, RuntimeOptions};

use crate::binding::BindingTable;
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::machine::Engine;
use crate::runtime::SharedCollector;
use crate::runtime::heap::SharedHeap;
use crate::worker::{Worker, WorkerId, WorkerOptions};
use crate::world::{Entity, EntityKind, RuntimeId, WorldState};

/// Runtime container that owns one or more workers in one shared world.
pub struct Runtime {
    /// Runtime identifier in world topology.
    pub(super) id: RuntimeId,
    /// Immutable ambient environment shared by newly spawned workers.
    pub(crate) environment: Arc<Environment>,
    /// The immutable runtime options.
    pub(super) options: Arc<RuntimeOptions>,
    /// The active runtime conditions.
    pub(crate) conditions: Arc<ConditionSet>,
    /// Durable program instantiated by this runtime.
    pub(crate) program: Arc<program::Program>,
    /// Immutable execution engine shared by all workers.
    pub(crate) engine: Engine,
    /// Runtime binding implementations shared by all workers.
    pub(crate) binding_table: Arc<BindingTable>,
    /// Runtime-owned shared heap and GC state.
    pub(crate) heap: Arc<SharedHeap>,
    /// Immutable program constant space.
    pub(crate) constant_space: program::StaticImage,
    /// Runtime-owned shared static space.
    pub(crate) shared_static: program::StaticSpace,
    /// All active workers keyed by identifier.
    pub(super) workers: BTreeMap<WorkerId, Worker>,
    /// Default worker used by convenience accessors.
    pub(super) default_worker_id: WorkerId,
    /// The next worker slot to schedule first.
    pub(super) next_worker_cursor: usize,
}

impl fmt::Debug for Runtime {
    /// Format one runtime without traversing executable internals.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Runtime")
            .field("runtime_id", &self.id)
            .field("environment", &self.environment)
            .field("options", &self.options)
            .field("conditions", &self.conditions)
            .field("heap", &self.heap)
            .field("workers", &self.workers)
            .field("default_worker_id", &self.default_worker_id)
            .field("next_worker_cursor", &self.next_worker_cursor)
            .finish()
    }
}

impl Runtime {
    /// Create a runtime with one default worker in one explicit shared world.
    pub(crate) fn new_in_world(
        environment: impl Into<Arc<Environment>>,
        options: &RuntimeOptions,
        conditions: Arc<ConditionSet>,
        world: &mut WorldState,
        memory: Arc<MemoryMap>,
        collector: Arc<SharedCollector>,
        program: impl Into<Arc<program::Program>>,
        binding_table: Arc<BindingTable>,
        engine: Engine,
    ) -> RuntimeResult<Self> {
        let environment = environment.into();
        let program = program.into();
        let options = Arc::new(options.clone());
        binding_table.require(&program)?;

        // allocate the runtime and default worker identities
        let runtime_id = world.allocate_runtime_id()?;
        let default_worker_id = world.allocate_worker_id()?;
        let worker_options = WorkerOptions::default();

        // materialize runtime-owned storage before publishing topology
        let constant_space = *program.constants();
        let shared_static = program.materialize_shared_statics(memory.clone())?;
        let heap = SharedHeap::new(memory, collector, &options, &program)?;
        let default_worker = Worker::new(
            environment.clone(),
            options.clone(),
            conditions.clone(),
            world,
            &heap,
            runtime_id,
            default_worker_id,
            program.clone(),
            binding_table.clone(),
            &engine,
        )?;

        // publish runtime selector metadata
        let runtime_name = options
            .identity
            .name
            .clone()
            .unwrap_or_else(|| "runtime".to_string());
        let runtime_entity = Entity::new(runtime_id.entity_id(), EntityKind::RUNTIME)
            .named(runtime_name)
            .labels(options.identity.labels.clone());
        world.register_runtime_topology(runtime_id, runtime_entity)?;

        // publish default worker selector metadata
        let worker_name = worker_options
            .name
            .unwrap_or_else(|| format!("worker-{}", default_worker_id.0));
        let worker_entity = Entity::new(default_worker_id.entity_id(), EntityKind::WORKER)
            .named(worker_name)
            .labels(worker_options.labels);
        world.register_worker_topology(runtime_id, default_worker_id, worker_entity)?;

        // install the default worker
        let mut workers = BTreeMap::new();
        workers.insert(default_worker_id, default_worker);

        Ok(Self {
            id: runtime_id,
            environment,
            options,
            conditions,
            program,
            binding_table,
            engine,
            heap,
            constant_space,
            shared_static,
            workers,
            default_worker_id,
            next_worker_cursor: 0,
        })
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

    /// Visit roots from runtime shared statics and every worker.
    pub fn visit_roots(&mut self, roots: &mut impl heap::RootSink) -> RuntimeResult<()> {
        let worker = self
            .workers
            .get_mut(&self.default_worker_id)
            .ok_or_else(|| RuntimeError::worker_not_found(self.default_worker_id.0).boxed())?;
        worker.visit_static_roots(&mut self.shared_static, roots)?;

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
        let shared = &self.heap.shared;

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
        self.workers.get(&worker_id)
    }

    /// Return one mutable worker by id.
    pub fn worker_mut(&mut self, worker_id: WorkerId) -> Option<&mut Worker> {
        self.workers.get_mut(&worker_id)
    }

    /// Spawn one additional worker in this runtime.
    pub(crate) fn spawn_worker(
        &mut self,
        world: &mut WorldState,
        worker_options: WorkerOptions,
    ) -> RuntimeResult<WorkerId> {
        // create one new worker attached to the runtime world
        let worker_id = world.allocate_worker_id()?;
        let worker = Worker::new(
            self.environment.clone(),
            self.options.clone(),
            self.conditions.clone(),
            world,
            &self.heap,
            self.id,
            worker_id,
            self.program.clone(),
            self.binding_table.clone(),
            &self.engine,
        )?;

        // publish worker selector metadata after construction succeeds
        let worker_name = worker_options
            .name
            .unwrap_or_else(|| format!("worker-{}", worker_id.0));
        let worker_entity = Entity::new(worker_id.entity_id(), EntityKind::WORKER)
            .named(worker_name)
            .labels(worker_options.labels);
        world.register_worker_topology(self.id, worker_id, worker_entity)?;

        self.insert_worker(worker)
    }

    /// Remove one worker from this runtime and return it.
    pub fn remove_worker(&mut self, worker_id: WorkerId) -> RuntimeResult<Worker> {
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

    /// Insert one worker and return its id.
    fn insert_worker(&mut self, mut worker: Worker) -> RuntimeResult<WorkerId> {
        let worker_id = worker.id;

        // reject duplicate ownership before mutating runtime state
        if self.workers.contains_key(&worker_id) {
            return Err(RuntimeError::worker_already_exists(worker_id.0).boxed());
        }

        // join active shared marking before publishing the worker
        if self.heap.is_marking() {
            worker.start_shared_edge_scan();
            self.heap.join_mark(worker_id);
        }

        self.workers.insert(worker_id, worker);

        Ok(worker_id)
    }

    /// Insert one restored worker image into this runtime.
    pub(crate) fn insert_restored_worker(&mut self, worker: Worker) -> RuntimeResult<WorkerId> {
        self.insert_worker(worker)
    }
}
