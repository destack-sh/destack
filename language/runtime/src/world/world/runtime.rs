use std::collections::BTreeMap;
use std::sync::Arc;

use tspp_artifact::ConditionSet;
use tspp_core::CaptureMode;
use tspp_program as program;
use tspp_repository::{Environment, ExecutionMode, RuntimeOptions};

use crate::binding::BindingTable;
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::machine::{Engine, Entry};
use crate::runtime::{Runtime, RuntimeId, RuntimeImage};
use crate::worker::{WorkerId, WorkerImage, WorkerOptions};
use crate::world::observation::Observation;
use crate::world::trace::EntrypointCall;

use super::{Entity, Mutation, RestoreContext, SpawnedWorker, World};

impl World {
    /// Spawn one live runtime owned by this world and return its identifier.
    pub fn spawn_runtime(
        &mut self,
        environment: impl Into<Arc<Environment>>,
        options: &RuntimeOptions,
        conditions: impl Into<Arc<ConditionSet>>,
        binding_table: Arc<BindingTable>,
        engine: Engine,
    ) -> RuntimeResult<RuntimeId> {
        let environment = environment.into();
        let conditions = conditions.into();
        let mode = self.state.trace.mode();
        let memory = self.memory.clone();
        let collector = self.collector.clone();
        let world = &mut self.state;
        let mut runtime = Runtime::new_in_world(
            environment.clone(),
            options,
            conditions,
            world,
            memory,
            collector,
            binding_table,
            engine,
        )?;
        let runtime_id = runtime.runtime_id();
        let worker_count = runtime.worker_count();

        // fast and strict modes do not need one structural spawn image
        let replay_image = if mode == ExecutionMode::Record {
            Some(runtime.capture_image(CaptureMode::Suspend)?)
        } else {
            None
        };

        // install the live runtime
        if self.runtimes.insert(runtime_id, runtime).is_some() {
            return Err(RuntimeError::runtime_already_exists(runtime_id.0).boxed());
        }

        // record the visible runtime spawn
        self.state.advance_moment()?;
        self.state.observe(Observation::RuntimeSpawned {
            runtime_id,
            worker_count,
        })?;

        // record structural spawn state for replay
        if let Some((runtime, workers)) = replay_image {
            let runtime_entity = self.runtime_entity(runtime_id)?;
            let workers = workers
                .into_iter()
                .map(|(worker_id, worker)| {
                    let worker_entity = self.worker_entity(worker_id)?;

                    Ok((
                        worker_id,
                        SpawnedWorker {
                            entity: worker_entity,
                            image: worker,
                        },
                    ))
                })
                .collect::<RuntimeResult<_>>()?;

            self.record_mutation(Mutation::SpawnRuntime {
                runtime_id,
                runtime_entity,
                runtime,
                workers,
            })?;
        }

        Ok(runtime_id)
    }

    /// Remove one stored runtime and all of its workers.
    pub fn remove_runtime(&mut self, runtime_id: RuntimeId) -> RuntimeResult<Runtime> {
        let mutation = Mutation::RemoveRuntime { runtime_id };
        let mutation = self.resolve_mutation(mutation)?;
        let Mutation::RemoveRuntime { runtime_id } = mutation.clone() else {
            return Err(RuntimeError::Internal {
                message: "runtime remove replay resolved to a non-remove mutation".to_string(),
            }
            .boxed());
        };

        let runtime = self.remove_stored_runtime(runtime_id)?;

        // record the visible runtime removal
        self.state.advance_moment()?;
        self.state
            .observe(Observation::RuntimeRemoved { runtime_id })?;

        if self.state.trace.mode() == ExecutionMode::Record {
            self.record_mutation(mutation)?;
        }

        Ok(runtime)
    }

    /// Spawn one additional worker in one stored runtime.
    pub fn spawn_worker(
        &mut self,
        runtime_id: RuntimeId,
        worker_options: WorkerOptions,
    ) -> RuntimeResult<WorkerId> {
        let mode = self.state.trace.mode();
        let world = &mut self.state;
        let runtime = self
            .runtimes
            .get_mut(&runtime_id)
            .ok_or_else(|| RuntimeError::runtime_not_found(runtime_id.0).boxed())?;

        let worker_id = runtime.spawn_worker(world, worker_options)?;

        // record the visible worker spawn
        self.state.advance_moment()?;
        self.state.observe(Observation::WorkerSpawned {
            runtime_id,
            worker_id,
        })?;

        // record structural spawn state for replay
        let replay_image = if mode == ExecutionMode::Record {
            Some(runtime.capture_worker_image(CaptureMode::Suspend, worker_id)?)
        } else {
            None
        };

        if let Some(worker) = replay_image {
            let worker_entity = self.worker_entity(worker_id)?;

            self.record_mutation(Mutation::SpawnWorker {
                runtime_id,
                worker_id,
                worker_entity,
                worker: Arc::new(worker),
            })?;
        }

        Ok(worker_id)
    }

    /// Invoke one entrypoint on one stored Runtime.
    pub fn invoke(
        &mut self,
        runtime_id: RuntimeId,
        entry: &Entry,
        args: &[program::Value],
    ) -> RuntimeResult<program::Value> {
        let invocation = EntrypointCall {
            runtime_id,
            entry: entry.clone(),
            args: args.iter().map(program::Value::fork).collect(),
        };
        let invocation = self.resolve_entrypoint(invocation)?;

        let result =
            self.execute_entrypoint(invocation.runtime_id, &invocation.entry, &invocation.args)?;

        if self.state.trace.mode() == ExecutionMode::Record {
            self.record_entrypoint(invocation)?;
        }

        Ok(result)
    }

    /// Run one stored Runtime's module initializers in dependency order.
    pub fn run_initializers(&mut self, runtime_id: RuntimeId) -> RuntimeResult<()> {
        let runtime = self
            .runtimes
            .get_mut(&runtime_id)
            .ok_or_else(|| RuntimeError::runtime_not_found(runtime_id.0).boxed())?;
        let worker_id = runtime.default_worker_id();
        let initializers: Vec<program::FunctionId> = runtime
            .program
            .initializers()
            .iter()
            .map(|initializer| initializer.function())
            .collect();
        for initializer in initializers {
            runtime.run_function(
                &mut self.state,
                self.host.as_ref(),
                &self.host_queue,
                worker_id,
                initializer,
                &[],
            )?;
        }

        Ok(())
    }

    /// Return the stored runtime ids in stable order.
    pub fn runtime_ids(&self) -> Vec<RuntimeId> {
        self.runtimes.keys().copied().collect()
    }

    /// Borrow one stored runtime immutably.
    pub fn runtime(&self, runtime_id: RuntimeId) -> RuntimeResult<&Runtime> {
        self.runtimes
            .get(&runtime_id)
            .ok_or_else(|| RuntimeError::runtime_not_found(runtime_id.0).boxed())
    }

    /// Borrow one stored runtime mutably.
    pub(crate) fn runtime_mut(&mut self, runtime_id: RuntimeId) -> RuntimeResult<&mut Runtime> {
        self.runtimes
            .get_mut(&runtime_id)
            .ok_or_else(|| RuntimeError::runtime_not_found(runtime_id.0).boxed())
    }

    /// Remove one stored runtime without recording a new mutation.
    pub(crate) fn remove_stored_runtime(
        &mut self,
        runtime_id: RuntimeId,
    ) -> RuntimeResult<Runtime> {
        let worker_ids = self.runtime(runtime_id)?.worker_ids();

        // clean worker-owned shared roots before dropping the runtime
        if let Some(runtime) = self.runtimes.get(&runtime_id) {
            for worker_id in &worker_ids {
                runtime.shared_collection.remove_worker(
                    &runtime.shared_heap,
                    &runtime.program,
                    *worker_id,
                );
            }
        }

        // clean worker topology without applying per-worker removal rules
        for worker_id in worker_ids {
            self.remove_worker_topology(worker_id);
        }

        let runtime = self
            .runtimes
            .remove(&runtime_id)
            .ok_or_else(|| RuntimeError::runtime_not_found(runtime_id.0).boxed())?;

        Ok(runtime)
    }

    /// Execute one entrypoint without recording a new invocation.
    pub(crate) fn execute_entrypoint(
        &mut self,
        runtime_id: RuntimeId,
        entry: &Entry,
        args: &[program::Value],
    ) -> RuntimeResult<program::Value> {
        let runtime = self
            .runtimes
            .get_mut(&runtime_id)
            .ok_or_else(|| RuntimeError::runtime_not_found(runtime_id.0).boxed())?;

        let worker_id = runtime.default_worker_id();

        runtime.run_entrypoint(
            &mut self.state,
            self.host.as_ref(),
            &self.host_queue,
            worker_id,
            entry,
            args,
        )
    }

    /// Restore one runtime image without tracing the outer invocation.
    pub(crate) fn restore_runtime_image(
        &mut self,
        runtime_id: RuntimeId,
        runtime_entity: Entity,
        runtime_image: &Arc<RuntimeImage>,
        worker_images: &BTreeMap<WorkerId, SpawnedWorker>,
        restore: RestoreContext<'_>,
    ) -> RuntimeResult<()> {
        // validate image shape before mutating topology
        if !worker_images.contains_key(&runtime_image.default_worker_id) {
            return Err(RuntimeError::default_worker_missing(
                runtime_id.0,
                runtime_image.default_worker_id.0,
            )
            .boxed());
        }

        let world = &mut self.state;
        world.register_runtime_topology(runtime_id, runtime_entity)?;

        // restored worker topology
        for (worker_id, worker) in worker_images {
            world.register_worker_topology(runtime_id, *worker_id, worker.entity.clone())?;
        }

        let worker_images = worker_images
            .iter()
            .map(|(worker_id, worker)| (*worker_id, worker.image.clone()))
            .collect();
        let memory = self.memory.clone();
        let collector = self.collector.clone();

        let runtime = Runtime::from_image(
            world,
            memory,
            collector,
            runtime_id,
            runtime_image.as_ref(),
            &worker_images,
            restore,
        )?;

        if self.runtimes.insert(runtime_id, runtime).is_some() {
            return Err(RuntimeError::runtime_already_exists(runtime_id.0).boxed());
        }

        Ok(())
    }

    /// Restore one worker image without tracing the outer invocation.
    pub(crate) fn restore_worker_image(
        &mut self,
        runtime_id: RuntimeId,
        worker_id: WorkerId,
        worker_entity: Entity,
        worker_image: &Arc<WorkerImage>,
        restore: RestoreContext<'_>,
    ) -> RuntimeResult<()> {
        let world = &mut self.state;
        let runtime = self
            .runtimes
            .get_mut(&runtime_id)
            .ok_or_else(|| RuntimeError::runtime_not_found(runtime_id.0).boxed())?;
        runtime.restore_worker(
            world,
            worker_id,
            worker_entity,
            worker_image.as_ref(),
            restore,
        )?;

        Ok(())
    }
}
