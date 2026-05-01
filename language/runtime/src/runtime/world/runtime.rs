use std::sync::Arc;

use destack_core::CaptureMode;
use destack_engine as engine;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::resource::ResourceRebinders;
use crate::runtime::engine::{Engine, Entry, Output};
use crate::runtime::trace::{Outcome, SpawnedWorkerImage};
use crate::runtime::{Runtime, RuntimeImage, Worker, WorkerId, WorkerImage};
use destack_workspace::{ExecutionMode, RuntimeOptions};

use super::{Command, RuntimeId, World};

impl World {
    /// Spawn one live runtime owned by this world and return its identifier.
    pub fn spawn_runtime(
        &mut self,
        platform_args: impl Into<Arc<[String]>>,
        options: &RuntimeOptions,
        engine: impl Into<Engine>,
    ) -> RuntimeResult<RuntimeId> {
        let mode = self.trace.mode();
        let world = self.world_scope();
        let lineage = self.lineage.read();
        let allocator = lineage.allocator();
        let collector = lineage.collector();
        drop(lineage);
        let mut runtime = Runtime::from_options_in_world(
            platform_args,
            options,
            &world,
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

        // record mode needs one structural spawn record for suffix replay
        if let Some((runtime, workers)) = replay_image {
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

                    Ok((
                        worker_id,
                        SpawnedWorkerImage {
                            name: worker_name,
                            image: worker,
                        },
                    ))
                })
                .collect::<RuntimeResult<_>>()?;

            self.accept(Outcome::RuntimeSpawned {
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
                runtime,
                workers,
            })?;
        }

        Ok(runtime_id)
    }

    /// Remove one stored runtime and all of its workers.
    pub fn remove_runtime(&mut self, runtime_id: RuntimeId) -> RuntimeResult<Box<Runtime>> {
        let command = Command::RemoveRuntime { runtime_id };
        let command = self.resolve_command(command)?;

        if self.trace.mode() == ExecutionMode::Record {
            self.ingest(command.clone())?;
        }

        let runtime = self.remove_runtime_inner(runtime_id)?;

        Ok(runtime)
    }

    /// Spawn one additional worker in one stored runtime.
    pub fn spawn_worker(
        &mut self,
        runtime_id: RuntimeId,
        engine: impl Into<Engine>,
    ) -> RuntimeResult<WorkerId> {
        self.spawn_worker_with_options(runtime_id, &RuntimeOptions::default(), engine)
    }

    /// Spawn one additional worker with explicit options in one stored runtime.
    pub fn spawn_worker_with_options(
        &mut self,
        runtime_id: RuntimeId,
        options: &RuntimeOptions,
        engine: impl Into<Engine>,
    ) -> RuntimeResult<WorkerId> {
        let world = self.world_scope();
        let runtime = self.runtimes.get_mut(&runtime_id).ok_or_else(|| {
            RuntimeError::RuntimeNotFound {
                runtime_id: runtime_id.0,
            }
            .boxed()
        })?;

        let mode = self.trace.mode();
        let worker_id = runtime.spawn_worker_with_options(&world, options, engine)?;

        // record mode needs one structural spawn record for suffix replay
        let replay_image = if mode == ExecutionMode::Record {
            let worker = runtime.worker_mut(worker_id).ok_or_else(|| {
                RuntimeError::WorkerNotFound {
                    worker_id: worker_id.0,
                }
                .boxed()
            })?;

            Some(worker.capture_image(CaptureMode::Suspend)?)
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
            self.accept(Outcome::WorkerSpawned {
                runtime_id,
                worker_id,
                worker_name,
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
    ) -> RuntimeResult<Output> {
        let command = Command::RunEntrypoint {
            runtime_id,
            entry: entry.clone(),
            args: args.to_vec(),
        };
        let command = self.resolve_command(command)?;

        if self.trace.mode() == ExecutionMode::Record {
            self.ingest(command.clone())?;
        }

        self.run_entrypoint_inner(runtime_id, entry, args)
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
    pub(crate) fn remove_runtime_inner(
        &mut self,
        runtime_id: RuntimeId,
    ) -> RuntimeResult<Box<Runtime>> {
        let runtime = self.runtimes.remove(&runtime_id).ok_or_else(|| {
            RuntimeError::RuntimeNotFound {
                runtime_id: runtime_id.0,
            }
            .boxed()
        })?;

        // remove all owned workers through the normal world mutation path
        let worker_ids = runtime.worker_ids();
        for worker_id in worker_ids {
            self.remove_worker(worker_id)?;
        }

        Ok(runtime)
    }

    /// Run one entrypoint without tracing the outer invocation.
    pub(crate) fn run_entrypoint_inner(
        &mut self,
        runtime_id: RuntimeId,
        entry: &Entry,
        args: &[engine::Value],
    ) -> RuntimeResult<Output> {
        let world = self.world_scope();
        let runtime = self.runtimes.get_mut(&runtime_id).ok_or_else(|| {
            RuntimeError::RuntimeNotFound {
                runtime_id: runtime_id.0,
            }
            .boxed()
        })?;

        runtime.run_entrypoint(&world, entry, args)
    }

    /// Install one restored runtime image without tracing the outer invocation.
    pub(crate) fn install_runtime_image(
        &mut self,
        runtime_id: RuntimeId,
        runtime_name: String,
        runtime_image: &Arc<RuntimeImage>,
        worker_images: &std::collections::BTreeMap<WorkerId, SpawnedWorkerImage>,
        rebind_context: Option<&ResourceRebinders>,
    ) -> RuntimeResult<()> {
        let world = self.world_scope();
        let primary_worker = worker_images
            .get(&runtime_image.primary_worker_id)
            .ok_or_else(|| {
                RuntimeError::PrimaryWorkerMissing {
                    runtime_id: runtime_id.0,
                    worker_id: runtime_image.primary_worker_id.0,
                }
                .boxed()
            })?;
        let primary_worker_labels = primary_worker
            .image
            .options
            .resolve(Some(&runtime_image.worker_options))?
            .labels
            .clone();
        world.register_runtime_topology(
            runtime_id,
            runtime_name.clone(),
            runtime_image.worker_options.labels.clone(),
            runtime_image.primary_worker_id,
            primary_worker.name.clone(),
            primary_worker_labels,
        )?;

        let worker_names = worker_images
            .iter()
            .map(|(worker_id, worker)| (*worker_id, worker.name.clone()))
            .collect();
        let worker_images = worker_images
            .iter()
            .map(|(worker_id, worker)| (*worker_id, worker.image.clone()))
            .collect();
        let lineage = self.lineage.read();
        let allocator = lineage.allocator();
        let collector = lineage.collector();
        drop(lineage);

        let runtime = Runtime::from_image(
            &world,
            allocator,
            collector,
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

    /// Install one restored worker image without tracing the outer invocation.
    pub(crate) fn install_worker_image(
        &mut self,
        runtime_id: RuntimeId,
        worker_id: WorkerId,
        worker_name: String,
        worker_image: &Arc<WorkerImage>,
        rebind_context: Option<&ResourceRebinders>,
    ) -> RuntimeResult<()> {
        let platform_args = self
            .runtimes
            .get(&runtime_id)
            .ok_or_else(|| {
                RuntimeError::RuntimeNotFound {
                    runtime_id: runtime_id.0,
                }
                .boxed()
            })?
            .platform_args_arc();

        let world = self.world_scope();
        let worker_labels = worker_image.options.resolve(None)?.labels.clone();
        world.register_worker_topology(
            runtime_id,
            worker_id,
            worker_name.clone(),
            worker_labels,
        )?;
        let worker = {
            let runtime = self.runtimes.get(&runtime_id).ok_or_else(|| {
                RuntimeError::RuntimeNotFound {
                    runtime_id: runtime_id.0,
                }
                .boxed()
            })?;

            Worker::from_image(
                &world,
                &runtime.shared,
                runtime.runtime_static(),
                runtime_id,
                worker_id,
                worker_name,
                platform_args,
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
