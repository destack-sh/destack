use std::collections::BTreeMap;
use std::sync::Arc;

use destack_core::CaptureMode;
use destack_engine as engine;
use destack_workspace::{Environment, ExecutionMode, RuntimeOptions};

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::host::resource::ResourceRebinders;
use crate::runtime::engine::{Engine, Entry};
use crate::runtime::{Runtime, RuntimeImage, Worker, WorkerId, WorkerImage, WorkerOptions};
use crate::world::trace::{EntrypointCall, Outcome, SpawnedWorkerImage};

use super::{Entity, Mutation, RuntimeId, World};

impl World {
    /// Spawn one live runtime owned by this world and return its identifier.
    pub fn spawn_runtime(
        &mut self,
        environment: impl Into<Arc<Environment>>,
        options: &RuntimeOptions,
        engine: impl Into<Engine>,
    ) -> RuntimeResult<RuntimeId> {
        let environment = environment.into();
        let mode = self.state.trace.mode();
        let history = self.history.read();
        let allocator = history.allocator.clone();
        let collector = history.collector.clone();
        drop(history);
        let world = &mut self.state;
        let mut runtime = Runtime::from_options_in_world(
            environment.clone(),
            options,
            world,
            allocator,
            collector,
            engine,
        )?;
        let runtime_id = runtime.runtime_id();

        // fast and strict modes do not need one structural spawn image
        let replay_image = if mode == ExecutionMode::Record {
            Some(runtime.capture_image(CaptureMode::Suspend)?)
        } else {
            None
        };

        // install the live runtime
        if self.runtimes.insert(runtime_id, runtime).is_some() {
            return Err(RuntimeError::RuntimeAlreadyExists {
                runtime_id: runtime_id.0,
            }
            .boxed());
        }

        // record structural spawn state for replay
        if let Some((runtime, workers)) = replay_image {
            let runtime_entity = self.runtime_entity(runtime_id)?;
            let workers = workers
                .into_iter()
                .map(|(worker_id, worker)| {
                    let worker_entity = self.worker_entity(worker_id)?;

                    Ok((
                        worker_id,
                        SpawnedWorkerImage {
                            entity: worker_entity,
                            image: worker,
                        },
                    ))
                })
                .collect::<RuntimeResult<_>>()?;

            self.record_outcome(Outcome::RuntimeSpawned {
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
        engine: impl Into<Engine>,
    ) -> RuntimeResult<WorkerId> {
        let mode = self.state.trace.mode();
        let world = &mut self.state;
        let runtime = self.runtimes.get_mut(&runtime_id).ok_or_else(|| {
            RuntimeError::RuntimeNotFound {
                runtime_id: runtime_id.0,
            }
            .boxed()
        })?;

        let worker_id = runtime.spawn_worker(world, worker_options, engine)?;

        // record structural spawn state for replay
        let replay_image = if mode == ExecutionMode::Record {
            Some(runtime.capture_worker_image(CaptureMode::Suspend, worker_id)?)
        } else {
            None
        };

        if let Some(worker) = replay_image {
            let worker_entity = self.worker_entity(worker_id)?;

            self.record_outcome(Outcome::WorkerSpawned {
                runtime_id,
                worker_id,
                worker_entity,
                worker: Arc::new(worker),
            })?;
        }

        Ok(worker_id)
    }

    /// Run one entrypoint on one stored runtime.
    pub fn run_entrypoint(
        &mut self,
        runtime_id: RuntimeId,
        entry: &Entry,
        args: &[engine::Value],
    ) -> RuntimeResult<engine::Value> {
        let invocation = EntrypointCall {
            runtime_id,
            entry: entry.clone(),
            args: args.to_vec(),
        };
        let invocation = self.resolve_entrypoint(invocation)?;

        let result =
            self.execute_entrypoint(invocation.runtime_id, &invocation.entry, &invocation.args)?;

        if self.state.trace.mode() == ExecutionMode::Record {
            self.record_entrypoint(invocation)?;
        }

        Ok(result)
    }

    /// Return the stored runtime ids in stable order.
    pub fn runtime_ids(&self) -> Vec<RuntimeId> {
        self.runtimes.keys().copied().collect()
    }

    /// Borrow one stored runtime immutably.
    pub(crate) fn runtime(&self, runtime_id: RuntimeId) -> RuntimeResult<&Runtime> {
        self.runtimes.get(&runtime_id).ok_or_else(|| {
            RuntimeError::RuntimeNotFound {
                runtime_id: runtime_id.0,
            }
            .boxed()
        })
    }

    /// Borrow one stored runtime mutably.
    pub fn runtime_mut(&mut self, runtime_id: RuntimeId) -> RuntimeResult<&mut Runtime> {
        self.runtimes.get_mut(&runtime_id).ok_or_else(|| {
            RuntimeError::RuntimeNotFound {
                runtime_id: runtime_id.0,
            }
            .boxed()
        })
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
                runtime.shared.remove_worker(*worker_id);
            }
        }

        // clean world metadata without applying per-worker removal rules
        for worker_id in worker_ids {
            self.remove_worker_metadata(worker_id);
        }

        let runtime = self.runtimes.remove(&runtime_id).ok_or_else(|| {
            RuntimeError::RuntimeNotFound {
                runtime_id: runtime_id.0,
            }
            .boxed()
        })?;

        Ok(runtime)
    }

    /// Execute one entrypoint without recording a new invocation.
    pub(crate) fn execute_entrypoint(
        &mut self,
        runtime_id: RuntimeId,
        entry: &Entry,
        args: &[engine::Value],
    ) -> RuntimeResult<engine::Value> {
        let runtime = self.runtimes.get_mut(&runtime_id).ok_or_else(|| {
            RuntimeError::RuntimeNotFound {
                runtime_id: runtime_id.0,
            }
            .boxed()
        })?;

        let worker_id = runtime.default_worker_id();

        runtime.run_entrypoint(
            &mut self.state,
            self.host.as_ref(),
            &self.host_queue,
            &mut self.poller,
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
        worker_images: &BTreeMap<WorkerId, SpawnedWorkerImage>,
        rebind_context: Option<&ResourceRebinders>,
    ) -> RuntimeResult<()> {
        // validate image shape before mutating topology
        if !worker_images.contains_key(&runtime_image.default_worker_id) {
            return Err(RuntimeError::DefaultWorkerMissing {
                runtime_id: runtime_id.0,
                worker_id: runtime_image.default_worker_id.0,
            }
            .boxed());
        }

        let world = &mut self.state;
        world.register_runtime_topology(runtime_id, runtime_entity)?;

        // restored worker metadata
        for (worker_id, worker) in worker_images {
            world.register_worker_topology(runtime_id, *worker_id, worker.entity.clone())?;
        }

        let worker_images = worker_images
            .iter()
            .map(|(worker_id, worker)| (*worker_id, worker.image.clone()))
            .collect();
        let history = self.history.read();
        let allocator = history.allocator.clone();
        let collector = history.collector.clone();
        drop(history);

        let runtime = Runtime::from_image(
            world,
            allocator,
            collector,
            runtime_id,
            runtime_image.as_ref(),
            &worker_images,
            rebind_context,
        )?;

        if self.runtimes.insert(runtime_id, runtime).is_some() {
            return Err(RuntimeError::RuntimeAlreadyExists {
                runtime_id: runtime_id.0,
            }
            .boxed());
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
        rebind_context: Option<&ResourceRebinders>,
    ) -> RuntimeResult<()> {
        let environment = self
            .runtimes
            .get(&runtime_id)
            .ok_or_else(|| {
                RuntimeError::RuntimeNotFound {
                    runtime_id: runtime_id.0,
                }
                .boxed()
            })?
            .environment
            .clone();

        let world = &mut self.state;
        world.register_worker_topology(runtime_id, worker_id, worker_entity)?;
        let runtime = self.runtimes.get(&runtime_id).ok_or_else(|| {
            RuntimeError::RuntimeNotFound {
                runtime_id: runtime_id.0,
            }
            .boxed()
        })?;
        let worker = {
            Worker::from_image(
                world,
                &runtime.shared,
                runtime.statics(),
                runtime_id,
                worker_id,
                environment,
                worker_image.as_ref(),
                None,
                rebind_context,
            )?
        };

        let runtime = self.runtimes.get_mut(&runtime_id).ok_or_else(|| {
            RuntimeError::RuntimeNotFound {
                runtime_id: runtime_id.0,
            }
            .boxed()
        })?;

        runtime.insert_restored_worker(worker)?;

        Ok(())
    }
}
