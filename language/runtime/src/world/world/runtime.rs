use std::collections::BTreeMap;
use std::sync::Arc;

use destack_core::CaptureMode;
use destack_engine as engine;
use destack_workspace::{ExecutionMode, RuntimeOptions};

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::host::resource::ResourceRebinders;
use crate::runtime::engine::{Engine, Entry};
use crate::runtime::{Runtime, RuntimeImage, Worker, WorkerId, WorkerImage, WorkerOptions};
use crate::world::trace::{EntrypointInvocation, Outcome, SpawnedWorkerImage};

use super::{Mutation, RuntimeId, World};

impl World {
    /// Spawn one live runtime owned by this world and return its identifier.
    pub fn spawn_runtime(
        &mut self,
        process_args: impl Into<Arc<[String]>>,
        options: &RuntimeOptions,
        engine: impl Into<Engine>,
    ) -> RuntimeResult<RuntimeId> {
        let mode = self.state.trace.mode();
        let history = self.history.read();
        let allocator = history.allocator();
        let collector = history.collector();
        drop(history);
        let world = &mut self.state;
        let mut runtime = Runtime::from_options_in_world(
            process_args,
            options,
            world,
            self.host.clone(),
            allocator,
            collector,
            engine,
        )?;
        let runtime_id = runtime.runtime_id();

        // fast and deterministic modes do not need one structural spawn image
        let replay_image = if mode == ExecutionMode::Record {
            Some(runtime.capture_image(CaptureMode::Suspend)?)
        } else {
            None
        };

        // install the live runtime
        if self
            .runtimes
            .insert(runtime_id, Box::new(runtime))
            .is_some()
        {
            return Err(RuntimeError::RuntimeAlreadyExists {
                runtime_id: runtime_id.0,
            }
            .boxed());
        }

        // record structural spawn state for replay
        if let Some((runtime, workers)) = replay_image {
            let runtime_labels = self.runtime_labels(runtime_id)?;
            let workers = workers
                .into_iter()
                .map(|(worker_id, worker)| {
                    let worker_name = self
                        .runtimes
                        .get(&runtime_id)
                        .and_then(|runtime| runtime.worker(worker_id))
                        .map(|worker| worker.name().to_string())
                        .ok_or_else(|| {
                            RuntimeError::WorkerNotFound {
                                worker_id: worker_id.0,
                            }
                            .boxed()
                        })?;
                    let worker_labels = self.worker_labels(worker_id)?;

                    Ok((
                        worker_id,
                        SpawnedWorkerImage {
                            name: worker_name,
                            labels: worker_labels,
                            image: worker,
                        },
                    ))
                })
                .collect::<RuntimeResult<_>>()?;

            self.record_outcome(Outcome::RuntimeSpawned {
                runtime_id,
                runtime_name: self
                    .runtimes
                    .get(&runtime_id)
                    .map(|runtime| runtime.name().to_string())
                    .ok_or_else(|| {
                        RuntimeError::RuntimeNotFound {
                            runtime_id: runtime_id.0,
                        }
                        .boxed()
                    })?,
                runtime_labels,
                runtime,
                workers,
            })?;
        }

        Ok(runtime_id)
    }

    /// Remove one stored runtime and all of its workers.
    pub fn remove_runtime(&mut self, runtime_id: RuntimeId) -> RuntimeResult<Box<Runtime>> {
        let mutation = Mutation::RemoveRuntime { runtime_id };
        let mutation = self.resolve_mutation(mutation)?;
        let Mutation::RemoveRuntime { runtime_id } = mutation.clone() else {
            return Err(RuntimeError::Internal {
                message: "runtime remove replay resolved to a non-remove mutation".to_string(),
            }
            .boxed());
        };

        let runtime = self.do_remove_runtime(runtime_id)?;

        if self.state.trace.mode() == ExecutionMode::Record {
            self.record_mutation(mutation)?;
        }

        Ok(runtime)
    }

    /// Spawn one additional worker in one stored runtime.
    pub fn spawn_worker(
        &mut self,
        runtime_id: RuntimeId,
        options: &RuntimeOptions,
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

        let worker_id = runtime.spawn_worker(world, options, worker_options, engine)?;

        // record structural spawn state for replay
        let replay_image = if mode == ExecutionMode::Record {
            Some(runtime.capture_worker_image(CaptureMode::Suspend, worker_id)?)
        } else {
            None
        };

        if let Some(worker) = replay_image {
            let worker_name = runtime
                .worker(worker_id)
                .map(|worker| worker.name().to_string())
                .ok_or_else(|| {
                    RuntimeError::WorkerNotFound {
                        worker_id: worker_id.0,
                    }
                    .boxed()
                })?;
            let worker_labels = self.worker_labels(worker_id)?;

            self.record_outcome(Outcome::WorkerSpawned {
                runtime_id,
                worker_id,
                worker_name,
                worker_labels,
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
        let invocation = EntrypointInvocation {
            runtime_id,
            entry: entry.clone(),
            args: args.to_vec(),
        };
        let invocation = self.resolve_entrypoint(invocation)?;

        let result =
            self.do_run_entrypoint(invocation.runtime_id, &invocation.entry, &invocation.args)?;

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
        self.runtimes
            .get(&runtime_id)
            .map(Box::as_ref)
            .ok_or_else(|| {
                RuntimeError::RuntimeNotFound {
                    runtime_id: runtime_id.0,
                }
                .boxed()
            })
    }

    /// Borrow one stored runtime mutably.
    pub fn runtime_mut(&mut self, runtime_id: RuntimeId) -> RuntimeResult<&mut Runtime> {
        self.runtimes
            .get_mut(&runtime_id)
            .map(Box::as_mut)
            .ok_or_else(|| {
                RuntimeError::RuntimeNotFound {
                    runtime_id: runtime_id.0,
                }
                .boxed()
            })
    }

    /// Remove one stored runtime without tracing the outer invocation.
    pub(crate) fn do_remove_runtime(
        &mut self,
        runtime_id: RuntimeId,
    ) -> RuntimeResult<Box<Runtime>> {
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

    /// Run one entrypoint without tracing the outer invocation.
    pub(crate) fn do_run_entrypoint(
        &mut self,
        runtime_id: RuntimeId,
        entry: &Entry,
        args: &[engine::Value],
    ) -> RuntimeResult<engine::Value> {
        let world = &mut self.state;
        let runtime = self.runtimes.get_mut(&runtime_id).ok_or_else(|| {
            RuntimeError::RuntimeNotFound {
                runtime_id: runtime_id.0,
            }
            .boxed()
        })?;

        runtime.run_entrypoint(world, entry, args)
    }

    /// Restore one runtime image without tracing the outer invocation.
    pub(crate) fn restore_runtime_image(
        &mut self,
        runtime_id: RuntimeId,
        runtime_name: String,
        runtime_labels: BTreeMap<String, String>,
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
        world.register_runtime_topology(runtime_id, runtime_name.clone(), runtime_labels)?;

        // restored worker metadata
        for (worker_id, worker) in worker_images {
            world.register_worker_topology(
                runtime_id,
                *worker_id,
                worker.name.clone(),
                worker.labels.clone(),
            )?;
        }

        let worker_names = worker_images
            .iter()
            .map(|(worker_id, worker)| (*worker_id, worker.name.clone()))
            .collect();
        let worker_images = worker_images
            .iter()
            .map(|(worker_id, worker)| (*worker_id, worker.image.clone()))
            .collect();
        let history = self.history.read();
        let allocator = history.allocator();
        let collector = history.collector();
        drop(history);

        let runtime = Runtime::from_image(
            world,
            allocator,
            collector,
            self.host.clone(),
            runtime_id,
            runtime_name,
            runtime_image.as_ref(),
            &worker_names,
            &worker_images,
            rebind_context,
        )?;

        if self
            .runtimes
            .insert(runtime_id, Box::new(runtime))
            .is_some()
        {
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
        worker_name: String,
        worker_labels: BTreeMap<String, String>,
        worker_image: &Arc<WorkerImage>,
        rebind_context: Option<&ResourceRebinders>,
    ) -> RuntimeResult<()> {
        let process_args = self
            .runtimes
            .get(&runtime_id)
            .ok_or_else(|| {
                RuntimeError::RuntimeNotFound {
                    runtime_id: runtime_id.0,
                }
                .boxed()
            })?
            .process_args();

        let world = &mut self.state;
        world.register_worker_topology(
            runtime_id,
            worker_id,
            worker_name.clone(),
            worker_labels,
        )?;
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
                worker_name,
                process_args,
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
