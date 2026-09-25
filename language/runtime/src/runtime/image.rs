use std::collections::BTreeMap;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tspp_artifact::ConditionSet;
use tspp_core::CaptureMode;
use tspp_heap as heap;
use tspp_memory::{MemoryMap, MemoryRange};
use tspp_program as program;
use tspp_repository::{Environment, ExecutionMode, RuntimeOptions};

use crate::binding::ReplayPayload;
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::heap::{SharedCollectionState, WorldCollector};
use crate::machine::{Engine, EngineImage};
use crate::runtime::{Runtime, RuntimeId};
use crate::worker::{Worker, WorkerId, WorkerImage};
use crate::world::{Entity, RestoreContext, WorldState};

/// One captured runtime image.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeImage {
    /// Runtime environment.
    pub environment: Arc<Environment>,
    /// The captured runtime options.
    pub options: Arc<RuntimeOptions>,
    /// The active runtime conditions.
    pub conditions: Arc<ConditionSet>,
    /// Durable program instantiated by this runtime.
    pub program: Arc<program::Program>,
    /// Captured runtime execution engine configuration.
    pub engine: EngineImage,
    /// Captured runtime-owned shared heap state.
    pub shared_heap: heap::SharedHeapImage,
    /// Captured immutable constant bytes.
    pub constant_space: program::StaticSpaceImage,
    /// Captured runtime-owned shared static bytes.
    pub shared_static: program::StaticSpaceImage,
    /// Default worker identifier for this runtime.
    pub default_worker_id: WorkerId,
    /// The next worker slot to schedule first.
    pub next_worker_cursor: usize,
}

impl PartialEq for RuntimeImage {
    fn eq(&self, other: &Self) -> bool {
        let is_same_program = Arc::ptr_eq(&self.program, &other.program)
            || self.program.bytes() == other.program.bytes();

        self.environment == other.environment
            && self.options == other.options
            && self.conditions == other.conditions
            && is_same_program
            && self.engine == other.engine
            && self.shared_heap == other.shared_heap
            && self.constant_space == other.constant_space
            && self.shared_static == other.shared_static
            && self.default_worker_id == other.default_worker_id
            && self.next_worker_cursor == other.next_worker_cursor
    }
}

impl Runtime {
    /// Capture one materialized runtime image and all owned worker images.
    pub(crate) fn capture_image(
        &mut self,
        mode: CaptureMode,
    ) -> RuntimeResult<(Arc<RuntimeImage>, BTreeMap<WorkerId, Arc<WorkerImage>>)> {
        // publish worker-local shared allocations before capturing the shared heap
        self.flush_shared_caches();

        // runtime metadata
        let runtime_image = Arc::new(RuntimeImage {
            default_worker_id: self.default_worker_id,
            environment: self.environment.clone(),
            options: self.options.clone(),
            conditions: self.conditions.clone(),
            program: self.program.clone(),
            engine: self.engine.image(),
            shared_heap: self.shared_heap.image(),
            constant_space: self.constant_space.image(),
            shared_static: self.shared_static.image(),
            next_worker_cursor: self.next_worker_cursor,
        });

        // worker images
        let mut worker_images = BTreeMap::new();
        for worker in self.workers.values_mut() {
            let image = worker.capture_image(mode)?;

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

        worker.capture_image(mode)
    }

    /// Restore and publish one worker image in this runtime.
    pub(crate) fn restore_worker(
        &mut self,
        world: &mut WorldState,
        worker_id: WorkerId,
        worker_entity: Entity,
        image: &WorkerImage,
        restore: RestoreContext<'_>,
    ) -> RuntimeResult<WorkerId> {
        // reject duplicate ownership before building or publishing the worker
        if self.workers.contains_key(&worker_id) {
            return Err(RuntimeError::worker_already_exists(worker_id.0).boxed());
        }

        // rebuild the worker from runtime-owned state
        let worker = Worker::from_image(
            world,
            &self.shared_heap,
            &self.allocation_plans,
            self.constant_range(),
            self.id,
            worker_id,
            self.environment.clone(),
            self.options.clone(),
            self.conditions.clone(),
            image,
            self.binding_table.clone(),
            &self.engine,
            restore,
        )?;

        // publish topology and runtime ownership together
        world.register_worker_topology(self.id, worker_id, worker_entity)?;
        self.insert_restored_worker(worker)
    }

    /// Fork one live runtime when all owned workers are quiescent.
    pub(crate) fn try_fork(
        &mut self,
        memory: Arc<MemoryMap>,
        execution_mode: ExecutionMode,
        replay_payload: ReplayPayload,
        collector: Arc<WorldCollector>,
    ) -> RuntimeResult<Option<Self>> {
        let shared_heap = Arc::new(
            self.shared_heap
                .fork(memory.clone())
                .map_err(Box::<RuntimeError>::from)?,
        );
        let shared_collection = SharedCollectionState::new(&collector);
        let allocation_plans = self.allocation_plans.clone();
        let constant_space = self.constant_space.fork(memory.clone());
        let shared_static = self.shared_static.fork(memory.clone());

        // fork each owned worker first
        let mut workers = BTreeMap::new();
        for (worker_id, worker) in &mut self.workers {
            let shared_mark_worker = shared_heap.register_mark_worker();
            let Some(worker) = worker.try_fork(
                execution_mode,
                replay_payload,
                &shared_heap,
                &allocation_plans,
                shared_mark_worker,
            )?
            else {
                return Ok(None);
            };
            workers.insert(*worker_id, worker);
        }

        Ok(Some(Self {
            id: self.id,
            environment: self.environment.clone(),
            options: self.options.clone(),
            conditions: self.conditions.clone(),
            program: self.program.clone(),
            binding_table: self.binding_table.clone(),
            engine: self.engine.clone(),
            shared_heap,
            shared_collection,
            allocation_plans,
            constant_space,
            shared_static,
            workers,
            default_worker_id: self.default_worker_id,
            next_worker_cursor: self.next_worker_cursor,
        }))
    }

    /// Restore one runtime from one materialized runtime image.
    pub(crate) fn from_image(
        world: &mut WorldState,
        memory: Arc<MemoryMap>,
        collector: Arc<WorldCollector>,
        runtime_id: RuntimeId,
        image: &RuntimeImage,
        worker_images: &BTreeMap<WorkerId, Arc<WorkerImage>>,
        restore: RestoreContext<'_>,
    ) -> RuntimeResult<Self> {
        // restore runtime state
        let environment = image.environment.clone();
        if worker_images.is_empty() {
            return Err(RuntimeError::Internal {
                message: "runtime image has no workers".to_string(),
            }
            .boxed());
        }
        let program = image.program.clone();
        let binding_table = restore.binding_table(&program)?;
        let engine = Engine::restore(program.clone(), image.engine, restore.native_loader())?;
        let constant_space =
            program::StaticSpace::from_image(memory.clone(), &image.constant_space);
        let local_heap_options = image
            .options
            .heap
            .local_heap_options()
            .map_err(Box::<RuntimeError>::from)?;
        let constant_range = MemoryRange {
            offset: constant_space.offset(),
            byte_len: constant_space.byte_len(),
        };
        let mut shared_heap = heap::SharedHeap::from_image_with_limits(
            &image.shared_heap,
            memory.clone(),
            image.options.heap.shared.limits(),
        )
        .map_err(Box::<RuntimeError>::from)?;
        shared_heap.set_constant_range(constant_range);
        let shared_heap = Arc::new(shared_heap);
        let shared_collection = SharedCollectionState::new(&collector);
        let allocation_plans = program
            .plan_allocations(&local_heap_options, shared_heap.options())?
            .into();
        let shared_static = program::StaticSpace::from_image(memory, &image.shared_static);
        let mut workers = BTreeMap::new();

        // workers
        for (worker_id, worker_image) in worker_images {
            let worker = Worker::from_image(
                world,
                &shared_heap,
                &allocation_plans,
                constant_range,
                runtime_id,
                *worker_id,
                environment.clone(),
                image.options.clone(),
                image.conditions.clone(),
                worker_image.as_ref(),
                binding_table.clone(),
                &engine,
                restore,
            )?;
            if workers.insert(*worker_id, worker).is_some() {
                return Err(
                    RuntimeError::duplicate_worker_image(runtime_id.0, worker_id.0).boxed(),
                );
            }
        }

        // validate the default worker
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
            conditions: image.conditions.clone(),
            program,
            binding_table,
            engine,
            shared_heap,
            shared_collection,
            allocation_plans,
            constant_space,
            shared_static,
            workers,
            default_worker_id: image.default_worker_id,
            next_worker_cursor: image.next_worker_cursor,
        })
    }
}
