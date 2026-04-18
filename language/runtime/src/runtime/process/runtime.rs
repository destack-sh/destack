use serde::{Deserialize, Serialize};

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::host::{HostEvent, Session};
use crate::platform::resource::ResourceRebinders;
use crate::runtime::engine::{Engine, EngineLayout, Entry, ExecutionOutput};
use crate::runtime::poller::{HostPoller, PollerEvent};
use crate::runtime::scheduler::Timer;
use crate::runtime::time::WorldInstant;
use crate::runtime::world::{RuntimeId, Wake, WorldRef};
use crate::runtime::{DropCounts, DropReason};
use destack_core::CaptureMode;
use destack_heap as heap;
use destack_workspace::{
    ExecutionMode, PlatformHostOptions, PlatformOsOptions, PollerBackend, RuntimeAppDeclaration,
    RuntimeOptions,
};
use std::collections::BTreeMap;
use std::sync::Arc;

use super::poller::{poller_for_backend, poller_for_options};
use super::{Worker, WorkerId, WorkerImage, WorkerOptionsImage};

/// Immutable runtime host reconstruction settings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeHostOptions {
    /// Captured platform host options for runtime restore.
    pub host_options: PlatformHostOptions,
    /// Captured OS service options for runtime restore.
    pub os_options: PlatformOsOptions,
    /// Captured app declaration for runtime restore.
    pub app_declaration: RuntimeAppDeclaration,
    /// Captured poller backend for runtime restore.
    pub poller_backend: PollerBackend,
}

impl RuntimeHostOptions {
    /// Build one runtime host reconstruction configuration from runtime options.
    fn from_runtime_options(options: &RuntimeOptions) -> Self {
        let (host_options, os_options, app_declaration) =
            Session::restore_config_from_runtime_options(options);

        Self {
            host_options,
            os_options,
            app_declaration,
            poller_backend: options.scheduler.poller_backend,
        }
    }

    /// Build one host session for the given runtime id.
    fn host_session(&self, runtime_id: RuntimeId) -> Session {
        Session::from_restore_config(
            runtime_id,
            self.host_options.clone(),
            self.os_options.clone(),
            self.app_declaration.clone(),
        )
    }

    /// Build one poller for this runtime configuration.
    fn poller(&self) -> RuntimeResult<Option<Box<dyn HostPoller>>> {
        poller_for_backend(self.poller_backend)
    }
}

/// Runtime-local arrived work that is not caused by world time advancing.
#[derive(Debug, Clone, PartialEq)]
enum RuntimeIngress {
    /// Runtime-wide host semantic arrival.
    Host {
        /// Host event payload.
        event: HostEvent,
    },
    /// Runtime-wide poller arrival.
    Poller {
        /// Poller event payload.
        event: PollerEvent,
    },
}

/// Runtime container that owns one or more workers in one shared world.
pub struct Runtime {
    /// Runtime identifier in world topology.
    id: RuntimeId,
    /// Runtime name used for identity selection and diagnostics.
    name: String,
    /// Immutable process arguments shared by newly spawned workers.
    platform_args: Arc<[String]>,
    /// Runtime-owned worker defaults.
    worker_options: Arc<RuntimeOptions>,
    /// Runtime host reconstruction settings.
    host_options: RuntimeHostOptions,
    /// Shared host integration for all workers in this runtime.
    host: Session,
    /// Shared platform poller for external events.
    poller: Option<Box<dyn HostPoller>>,
    /// Drop accounting at the runtime coordination boundary.
    drop_counts: DropCounts,
    /// All active workers keyed by identifier.
    workers: BTreeMap<WorkerId, Box<Worker>>,
    /// Default worker used by convenience accessors.
    primary_worker_id: WorkerId,
}

/// Materialized runtime metadata captured in one world image.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeImage {
    /// Primary worker identifier for this runtime.
    pub primary_worker_id: WorkerId,
    /// Runtime launch arguments.
    pub platform_args: Arc<[String]>,
    /// Runtime-owned worker defaults captured for reconstruction.
    pub worker_options: Arc<RuntimeOptions>,
    /// Runtime host reconstruction settings captured for reconstruction.
    pub host_options: RuntimeHostOptions,
}

/// Result of one runtime scheduler tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TickOutcome {
    /// One deterministic unit of work or ingress handling completed.
    Progressed,
    /// Virtual time advanced to the next deadline.
    AdvancedTime,
    /// No runnable work or future deadlines remained.
    Idle,
}

impl TickOutcome {
    /// Return whether this tick made deterministic progress.
    pub const fn progressed(self) -> bool {
        !matches!(self, Self::Idle)
    }
}

impl std::fmt::Debug for Runtime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Runtime")
            .field("runtime_id", &self.id)
            .field("name", &self.name)
            .field("platform_args", &self.platform_args)
            .field("worker_options", &self.worker_options)
            .field("host_options", &self.host_options)
            .field("host", &self.host)
            .field("workers", &self.workers)
            .field("primary_worker_id", &self.primary_worker_id)
            .field("poller", &"<shared platform poller>")
            .finish()
    }
}

impl Runtime {
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

    /// Create a runtime with one primary worker in one explicit shared world.
    pub(crate) fn from_options_in_world(
        platform_args: impl Into<Arc<[String]>>,
        options: &RuntimeOptions,
        world: &WorldRef,
        engine: impl Engine + EngineLayout + 'static,
    ) -> RuntimeResult<Self> {
        let platform_args = platform_args.into();
        let primary_worker = Worker::new_in_world(platform_args.clone(), options, world, engine)?;
        let mut runtime = Self::new(platform_args, options, primary_worker)?;
        if let Some(poller) = poller_for_options(options)? {
            runtime.set_poller(poller);
        }

        Ok(runtime)
    }

    /// Return the shared host integration for this runtime.
    pub fn host(&self) -> &Session {
        &self.host
    }

    /// Return the shared platform arguments backing for this runtime.
    pub(crate) fn platform_args_arc(&self) -> Arc<[String]> {
        self.platform_args.clone()
    }

    /// Return the current primary worker id.
    pub fn primary_worker_id(&self) -> WorkerId {
        self.primary_worker_id
    }

    /// Return the world topology runtime id.
    pub fn runtime_id(&self) -> RuntimeId {
        self.id
    }

    /// Return this runtime name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Return drop accounting observed by this runtime coordinator.
    pub const fn drop_counts(&self) -> DropCounts {
        self.drop_counts
    }

    /// Set one explicit primary worker.
    pub fn set_primary_worker(&mut self, worker_id: WorkerId) -> RuntimeResult<()> {
        if self.workers.contains_key(&worker_id) {
            self.primary_worker_id = worker_id;
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

    /// Return one immutable worker by id.
    pub fn worker(&self, worker_id: WorkerId) -> Option<&Worker> {
        self.workers.get(&worker_id).map(Box::as_ref)
    }

    /// Return one mutable worker by id.
    pub fn worker_mut(&mut self, worker_id: WorkerId) -> Option<&mut Worker> {
        self.workers.get_mut(&worker_id).map(Box::as_mut)
    }

    /// Spawn one additional worker with explicit options in the shared runtime world.
    pub(crate) fn spawn_worker_with_options(
        &mut self,
        world: &WorldRef,
        options: &RuntimeOptions,
        engine: impl Engine + EngineLayout + 'static,
    ) -> RuntimeResult<WorkerId> {
        // force runtime identity to stay shared across all workers in this runtime
        let mut options = options.clone();
        options.name = Some(self.name.clone());
        options.labels = self.worker_options.labels.clone();
        self.align_spawn_options_with_runtime(&mut options);

        // create one new worker attached to the runtime world
        let worker =
            Worker::new_in_runtime(self.platform_args.clone(), &options, world, self.id, engine)?;

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

        // reject implicit primary fallback to keep ownership explicit
        if self.primary_worker_id == worker_id {
            self.workers.insert(worker_id, removed_worker);
            return Err(RuntimeError::PrimaryWorkerRemoval.boxed());
        }

        Ok(removed_worker)
    }

    /// Attach a shared platform poller for all workers in this runtime.
    pub fn set_poller(&mut self, poller: Box<dyn HostPoller>) {
        self.poller = Some(poller);
    }

    /// Run one entrypoint through the default runtime worker event loop.
    pub(crate) fn run_entrypoint(
        &mut self,
        world: &WorldRef,
        entry: &Entry,
        args: &[heap::Value],
    ) -> RuntimeResult<ExecutionOutput> {
        self.run_entrypoint_for_worker(world, self.primary_worker_id, entry, args)
    }

    /// Run one entrypoint through one explicit runtime worker event loop.
    pub(crate) fn run_entrypoint_for_worker(
        &mut self,
        world: &WorldRef,
        worker_id: WorkerId,
        entry: &Entry,
        args: &[heap::Value],
    ) -> RuntimeResult<ExecutionOutput> {
        let host = &self.host;
        let poller = &mut self.poller;
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
        worker.run_entrypoint_with_host_and_poller(world, host, entry, args, poller)
    }

    /// Execute one runtime tick across all workers without advancing world time.
    pub(crate) fn tick(&mut self, world: &WorldRef) -> RuntimeResult<TickOutcome> {
        // poll and handle runtime ingress first
        let ingress_handled = self.poll_ingress(world)?;

        // run one local worker tick in stable id order
        let worker_ids = self.worker_ids();
        let host = &self.host;
        let workers = &mut self.workers;
        for worker_id in worker_ids {
            let worker = workers
                .get_mut(&worker_id)
                .map(Box::as_mut)
                .ok_or_else(|| {
                    RuntimeError::WorkerNotFound {
                        worker_id: worker_id.0,
                    }
                    .boxed()
                })?;
            if worker.tick(world, host)? {
                return Ok(TickOutcome::Progressed);
            }
        }

        // ingress handling counts as runnable scheduler progress
        if ingress_handled {
            return Ok(TickOutcome::Progressed);
        }

        Ok(TickOutcome::Idle)
    }

    /// Create one runtime from one already-constructed primary worker.
    fn new(
        platform_args: Arc<[String]>,
        options: &RuntimeOptions,
        primary_worker: Worker,
    ) -> RuntimeResult<Self> {
        // seed runtime identity from runtime options
        let primary_worker = Box::new(primary_worker);
        let primary_worker_id = primary_worker.id;
        let runtime_id = primary_worker.runtime_id;
        let worker_options = Arc::new(options.clone());
        let host_options = RuntimeHostOptions::from_runtime_options(options);
        let host = host_options.host_session(runtime_id);
        let runtime_name = options
            .name
            .clone()
            .unwrap_or_else(|| "runtime".to_string());
        let mut workers = BTreeMap::new();
        workers.insert(primary_worker_id, primary_worker);

        // store runtime state
        Ok(Self {
            id: runtime_id,
            name: runtime_name,
            platform_args,
            worker_options,
            host_options,
            host,
            poller: None,
            drop_counts: DropCounts::default(),
            workers,
            primary_worker_id,
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

        Ok(worker_id)
    }

    /// Insert one restored worker image into this runtime.
    pub(crate) fn insert_restored_worker(&mut self, worker: Worker) -> RuntimeResult<WorkerId> {
        self.insert_worker(worker)
    }

    /// Return the next virtual deadline across all workers and simulation.
    pub(crate) fn next_deadline(&self, world: &WorldRef) -> Option<WorldInstant> {
        // current virtual timestamps: monotonic deadlines are projected onto wall time
        let wall_now = world.wall();
        let mono_now = world.mono();

        world.next_deadline(
            self.workers
                .values()
                .map(|worker| worker.event_loop.next_deadline(wall_now, mono_now)),
        )
    }

    /// Drain due worker timers after the world advances time.
    pub(crate) fn collect_due_timers(
        &mut self,
        world: &WorldRef,
    ) -> RuntimeResult<Vec<(RuntimeId, WorkerId, Timer)>> {
        let wall_now = world.wall();
        let mono_now = world.mono();
        let mut worker_timers = Vec::new();

        for worker in self.workers.values_mut() {
            let ready_timers = worker.event_loop.poll_timers(wall_now, mono_now)?;
            for timer in ready_timers {
                worker_timers.push((self.id, worker.id, timer));
            }
        }

        Ok(worker_timers)
    }

    /// Poll runtime-owned ingress sources and deliver arrivals to worker event loops.
    fn poll_ingress(&mut self, world: &WorldRef) -> RuntimeResult<bool> {
        let mut ingress = Vec::new();

        // host semantic ingress
        let poll_result = self.host.poll(Some(0))?;
        if poll_result.dropped_event_count > 0 {
            self.drop_counts
                .record(DropReason::QueuePressure, poll_result.dropped_event_count);
        }
        for event in poll_result.events {
            ingress.push(RuntimeIngress::Host { event });
        }

        // poller ingress
        if let Some(poller) = self.poller.as_mut() {
            let events = poller.poll(Some(0))?;
            for event in events {
                ingress.push(RuntimeIngress::Poller { event });
            }
        }

        self.deliver_ingress(world, ingress)
    }

    /// Deliver coordinator-owned ingress into worker event loops.
    fn deliver_ingress(
        &mut self,
        world: &WorldRef,
        ingress: Vec<RuntimeIngress>,
    ) -> RuntimeResult<bool> {
        let mut handled_any = false;

        for item in ingress {
            match item {
                RuntimeIngress::Host { event } => {
                    let kind = event.kind();
                    let targets = self
                        .worker_ids()
                        .into_iter()
                        .filter(|worker_id| {
                            self.worker(*worker_id)
                                .map(|worker| worker.watches_host_event(kind))
                                .unwrap_or(false)
                        })
                        .collect::<Vec<_>>();

                    // explicit unmatched ingress
                    if targets.is_empty() {
                        self.drop_counts.record(DropReason::UnmatchedIngress, 1);
                        handled_any = true;
                        continue;
                    }

                    // matched host ingress
                    for worker_id in targets {
                        let worker = self.worker_mut(worker_id).ok_or_else(|| {
                            RuntimeError::WorkerNotFound {
                                worker_id: worker_id.0,
                            }
                            .boxed()
                        })?;
                        worker.event_loop.enqueue_host_events(vec![event.clone()]);
                        worker.hooks.on_ingress_enqueue(world);
                        handled_any = true;
                    }
                }
                RuntimeIngress::Poller { event } => {
                    let targets = self
                        .worker_ids()
                        .into_iter()
                        .filter(|worker_id| {
                            self.worker(*worker_id)
                                .map(|worker| worker.watches_event(event.token))
                                .unwrap_or(false)
                        })
                        .collect::<Vec<_>>();

                    // explicit unmatched ingress
                    if targets.is_empty() {
                        self.drop_counts.record(DropReason::UnmatchedIngress, 1);
                        handled_any = true;
                        continue;
                    }

                    // matched poller ingress
                    for worker_id in targets {
                        let worker = self.worker_mut(worker_id).ok_or_else(|| {
                            RuntimeError::WorkerNotFound {
                                worker_id: worker_id.0,
                            }
                            .boxed()
                        })?;
                        worker.event_loop.enqueue_events(vec![event]);
                        worker.hooks.on_ingress_enqueue(world);
                        handled_any = true;
                    }
                }
            }
        }

        Ok(handled_any)
    }

    /// Deliver one batch of due worker-timer wakes.
    pub(crate) fn deliver_wakes(
        &mut self,
        world: &WorldRef,
        wakes: Vec<Wake>,
    ) -> RuntimeResult<()> {
        for wake in wakes {
            match wake {
                Wake::WorkerTimer {
                    runtime_id,
                    worker_id,
                    timer,
                } => {
                    if runtime_id != self.id {
                        continue;
                    }

                    let worker = self.worker_mut(worker_id).ok_or_else(|| {
                        RuntimeError::WorkerNotFound {
                            worker_id: worker_id.0,
                        }
                        .boxed()
                    })?;
                    worker.deliver_timer_wake(world, timer)?;
                }
            }
        }

        Ok(())
    }

    /// Align world-scoped options for workers spawned in one existing runtime.
    fn align_spawn_options_with_runtime(&self, options: &mut RuntimeOptions) {
        // world scoped settings: all workers in one runtime share one world
        options.execution = self.worker_options.execution;
        options.world = self.worker_options.world;
        options.access = self.worker_options.access;
        options.replay = self.worker_options.replay.clone();
        options.time = self.worker_options.time.clone();
        options.random = self.worker_options.random.clone();
        options.rules = self.worker_options.rules.clone();
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
            primary_worker_id: self.primary_worker_id,
            platform_args: self.platform_args.clone(),
            worker_options: self.worker_options.clone(),
            host_options: self.host_options.clone(),
        });

        // worker images
        let mut worker_images = BTreeMap::new();
        for worker in self.workers.values_mut() {
            let mut image = worker.capture_image(mode)?;

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

    /// Fork one live runtime when all owned workers are quiescent.
    pub(crate) fn try_fork(
        &mut self,
        execution_mode: ExecutionMode,
    ) -> RuntimeResult<Option<Self>> {
        // fork each owned worker first
        let mut workers = BTreeMap::new();
        for (worker_id, worker) in &mut self.workers {
            let Some(worker) = worker.try_fork(execution_mode)? else {
                return Ok(None);
            };
            workers.insert(*worker_id, Box::new(worker));
        }

        // rebuild one fresh host integration boundary
        let host = self.host_options.host_session(self.id);
        let poller = self.host_options.poller()?;

        Ok(Some(Self {
            id: self.id,
            name: self.name.clone(),
            platform_args: self.platform_args.clone(),
            worker_options: self.worker_options.clone(),
            host_options: self.host_options.clone(),
            host,
            poller,
            drop_counts: self.drop_counts,
            workers,
            primary_worker_id: self.primary_worker_id,
        }))
    }

    /// Restore one runtime from one materialized runtime image.
    pub(crate) fn from_image(
        world: &WorldRef,
        runtime_id: RuntimeId,
        runtime_name: String,
        image: &RuntimeImage,
        worker_names: &BTreeMap<WorkerId, String>,
        worker_images: &BTreeMap<WorkerId, Arc<WorkerImage>>,
        rebind_context: Option<&ResourceRebinders>,
    ) -> RuntimeResult<Self> {
        // runtime-wide reconstructed state
        let platform_args = image.platform_args.clone();
        let host = image.host_options.host_session(runtime_id);
        let poller = image.host_options.poller()?;
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
                runtime_id,
                *worker_id,
                worker_name.clone(),
                platform_args.clone(),
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

        // validate the primary worker after reconstruction
        if !workers.contains_key(&image.primary_worker_id) {
            return Err(RuntimeError::PrimaryWorkerMissing {
                runtime_id: runtime_id.0,
                worker_id: image.primary_worker_id.0,
            }
            .boxed());
        }

        Ok(Self {
            id: runtime_id,
            name: runtime_name,
            platform_args,
            worker_options: image.worker_options.clone(),
            host_options: image.host_options.clone(),
            host,
            poller,
            drop_counts: DropCounts::default(),
            workers,
            primary_worker_id: image.primary_worker_id,
        })
    }
}
