use std::collections::BTreeMap;
use std::fmt;
use std::sync::Arc;

use destack_artifact::ConditionSet;
use destack_heap as heap;
use destack_memory::{MemoryMap, MemoryRange};
use destack_program as program;
use destack_repository::{Environment, RuntimeOptions};
use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use crate::binding::BindingTable;
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::heap::{SharedCollectionState, WorldCollector};
use crate::machine::Engine;
use crate::worker::{Worker, WorkerId, WorkerOptions};
use crate::world::{Entity, EntityKind, WorldState};

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

    /// Runtime-owned shared heap.
    pub(crate) shared_heap: Arc<heap::SharedHeap>,
    /// Collection state for the runtime-owned shared heap.
    pub(crate) shared_collection: Arc<SharedCollectionState>,
    /// Allocation plans indexed by Program allocation site id.
    pub(crate) allocation_plans: Arc<[heap::AllocationPlan]>,
    /// Immutable program constant space.
    pub(crate) constant_space: program::StaticSpace,
    /// Runtime-owned shared static space.
    pub(crate) shared_static: program::StaticSpace,

    /// All active workers keyed by identifier.
    pub(crate) workers: BTreeMap<WorkerId, Worker>,
    /// Default worker used by convenience accessors.
    pub(super) default_worker_id: WorkerId,
    /// The next worker slot to schedule first.
    pub(super) next_worker_cursor: usize,
}

/// Stable identifier for one Runtime in one World.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Reflect,
)]
pub struct RuntimeId(pub u64);

impl fmt::Display for RuntimeId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.0)
    }
}

impl fmt::Debug for Runtime {
    /// Format one runtime without traversing executable internals.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Runtime")
            .field("runtime_id", &self.id)
            .field("environment", &self.environment)
            .field("options", &self.options)
            .field("conditions", &self.conditions)
            .field("shared_heap", &self.shared_heap)
            .field("shared_collection", &self.shared_collection)
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
        collector: Arc<WorldCollector>,
        binding_table: Arc<BindingTable>,
        engine: Engine,
    ) -> RuntimeResult<Self> {
        let environment = environment.into();
        let program = engine.program().clone();
        let options = Arc::new(options.clone());
        binding_table.require(&program)?;

        // allocate the runtime and default worker identities
        let runtime_id = world.allocate_runtime_id()?;
        let default_worker_id = world.allocate_worker_id()?;
        let worker_options = WorkerOptions::default();

        // materialize runtime-owned storage before publishing topology
        let (constant_space, shared_static) =
            program.materialize_runtime_statics(memory.clone())?;
        let local_heap_options = options
            .heap
            .local_heap_options()
            .map_err(Box::<RuntimeError>::from)?;
        let shared_heap_options = options
            .heap
            .shared_heap_options()
            .map_err(Box::<RuntimeError>::from)?;
        let constant_range = MemoryRange {
            offset: constant_space.offset(),
            byte_len: constant_space.byte_len(),
        };
        let mut shared_heap =
            heap::SharedHeap::new(memory, options.heap.shared.limits(), shared_heap_options)
                .map_err(Box::<RuntimeError>::from)?;
        shared_heap.set_constant_range(constant_range);
        let shared_heap = Arc::new(shared_heap);
        let shared_collection = SharedCollectionState::new(&collector);
        let allocation_plans = program
            .plan_allocations(&local_heap_options, shared_heap.options())?
            .into();
        let default_worker = Worker::new(
            environment.clone(),
            options.clone(),
            conditions.clone(),
            world,
            &shared_heap,
            &allocation_plans,
            &constant_space,
            &shared_static,
            constant_range,
            runtime_id,
            default_worker_id,
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
            shared_heap,
            shared_collection,
            allocation_plans,
            constant_space,
            shared_static,
            workers,
            default_worker_id,
            next_worker_cursor: 0,
        })
    }

    /// Return the constant range inside world memory.
    pub(crate) fn constant_range(&self) -> MemoryRange {
        MemoryRange {
            offset: self.constant_space.offset(),
            byte_len: self.constant_space.byte_len(),
        }
    }

    /// Borrow the Program instantiated by this Runtime.
    pub fn program(&self) -> &program::Program {
        &self.program
    }

    /// Borrow this Runtime's immutable options.
    pub fn options(&self) -> &RuntimeOptions {
        &self.options
    }

    /// Borrow this Runtime's ambient environment.
    pub fn environment(&self) -> &Environment {
        &self.environment
    }

    /// Borrow this Runtime's active Program conditions.
    pub fn conditions(&self) -> &ConditionSet {
        &self.conditions
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

    /// Start one incremental local-to-shared edge scan across all workers.
    pub(crate) fn start_shared_edge_scan(&mut self) {
        for worker in self.workers.values_mut() {
            worker.start_shared_edge_scan();
        }
    }

    /// Publish worker-local shared heap buffers across all workers.
    pub(crate) fn flush_shared_caches(&mut self) {
        let shared = &self.shared_heap;

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
            &self.shared_heap,
            &self.allocation_plans,
            &self.constant_space,
            &self.shared_static,
            self.constant_range(),
            self.id,
            worker_id,
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
        // require exact ownership before changing runtime state
        if !self.workers.contains_key(&worker_id) {
            return Err(RuntimeError::worker_not_found(worker_id.0).boxed());
        }

        // preserve one explicit default worker
        if self.workers.len() == 1 {
            return Err(RuntimeError::last_worker_removal().boxed());
        }
        if self.default_worker_id == worker_id {
            return Err(RuntimeError::default_worker_removal().boxed());
        }

        self.workers
            .remove(&worker_id)
            .ok_or_else(|| RuntimeError::worker_not_found(worker_id.0).boxed())
    }

    /// Insert one worker and return its id.
    fn insert_worker(&mut self, mut worker: Worker) -> RuntimeResult<WorkerId> {
        let worker_id = worker.id;

        // reject duplicate ownership before mutating runtime state
        if self.workers.contains_key(&worker_id) {
            return Err(RuntimeError::worker_already_exists(worker_id.0).boxed());
        }

        // join active shared marking before publishing the worker
        if self.shared_heap.gc_phase() == heap::GcPhase::Mark {
            worker.start_shared_edge_scan();
            self.shared_collection.join_mark(worker_id);
        }

        self.workers.insert(worker_id, worker);

        Ok(worker_id)
    }

    /// Insert one restored worker image into this runtime.
    pub(crate) fn insert_restored_worker(&mut self, worker: Worker) -> RuntimeResult<WorkerId> {
        self.insert_worker(worker)
    }
}
