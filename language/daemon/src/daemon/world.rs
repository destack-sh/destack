use std::collections::BTreeMap;
use std::io::Cursor;
use std::sync::Arc;

use destack_core::{Blob, SectionStorage};
use destack_program as program;
use destack_repository::{BlobStore, Environment, WorldOptions};
use destack_rpc::{Code, Request, Response, Status};
use destack_runtime::binding::BindingTable;
use destack_runtime::diagnostic::{EntityError, MachineError, RuntimeError};
use destack_runtime::machine::Engine;
use destack_runtime::service::{
    AddBreakpointRequest, AddProbeRequest, AddWatchpointRequest, CheckpointRequest,
    DebuggerRequest, DebuggerService, ForkRequest, FrameRequest, InvokeRequest, MemoryChunk,
    MemoryRequest, ProbeRequest, ReadBranchRequest, ReadCheckpointRequest, RemoveBreakpointRequest,
    RemoveProbeRequest, RemoveRuntimeRequest, RemoveWatchpointRequest, RestoreRequest,
    RewindRequest, RunRequest, SnapshotRequest, SpawnRuntimeRequest, UpdateBreakpointRequest,
    UpdateProbeRequest, UpdateWatchpointRequest, WorldId, WorldRequest, WorldService,
};
use destack_runtime::world::{
    Branch, Breakpoint, Checkpoint, CheckpointId, Debugger, Frame, Moment, Probe, ProbeId,
    RestoreContext, RunOutcome, RuntimeId, Watchpoint, World, WorldImage, WorldSnapshot,
};
use destack_vm::MachineLimits;
use parking_lot::{Mutex, RwLock};

use crate::DaemonError;

/// Worlds hosted by one daemon process.
#[derive(Clone)]
pub(crate) struct WorldRegistry {
    /// Immutable Program bytes shared with daemon Blob operations.
    blobs: Arc<dyn BlobStore>,
    /// Runtime bindings available to every hosted World.
    bindings: Arc<BindingTable>,
    /// World registrations and identifier allocation.
    state: Arc<RwLock<WorldRegistryState>>,
}

/// Mutable World registry state.
struct WorldRegistryState {
    /// Next World identifier to allocate.
    next_world_id: u64,
    /// Live Worlds keyed by daemon identity.
    worlds: BTreeMap<WorldId, Arc<Mutex<World>>>,
}

impl std::fmt::Debug for WorldRegistry {
    /// Format hosted World registration state.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("WorldRegistry")
            .field("bindings", &self.bindings)
            .field("worlds", &self.state.read().worlds.keys())
            .finish()
    }
}

impl WorldRegistry {
    /// Create one empty World registry.
    pub(crate) fn new(blobs: Arc<dyn BlobStore>) -> Self {
        let bindings = BindingTable::new().with_fiber_bindings();
        let state = WorldRegistryState {
            next_world_id: 1,
            worlds: BTreeMap::new(),
        };

        Self {
            blobs,
            bindings: Arc::new(bindings),
            state: Arc::new(RwLock::new(state)),
        }
    }

    /// Create one World and return its daemon identity.
    pub(crate) fn create(
        &self,
        options: WorldOptions,
        environment: Environment,
    ) -> Result<WorldId, Status> {
        let world = World::new(&options, environment).map_err(Self::runtime_status)?;
        let mut state = self.state.write();
        let world_id = state.allocate_id()?;
        state.worlds.insert(world_id, Arc::new(Mutex::new(world)));

        Ok(world_id)
    }

    /// Close one World when it is registered.
    pub(crate) fn close(&self, world_id: WorldId) {
        self.state.write().worlds.remove(&world_id);
    }

    /// Close every registered World.
    pub(crate) fn close_all(&self) {
        self.state.write().worlds.clear();
    }

    /// Return one registered World.
    fn world(&self, world_id: WorldId) -> Result<Arc<Mutex<World>>, Status> {
        self.state
            .read()
            .worlds
            .get(&world_id)
            .cloned()
            .ok_or_else(|| {
                Status::new(
                    Code::NotFound,
                    format!("World {} is not open", world_id.get()),
                )
            })
    }

    /// Capture the selected current or committed World image.
    fn image(&self, world_id: WorldId, moment: Option<Moment>) -> Result<WorldImage, Status> {
        let world = self.world(world_id)?;
        let mut world = world.lock();
        let image = match moment {
            Some(moment) => world.lineage().image(moment),
            None => world.image(),
        }
        .map_err(Self::runtime_status)?;

        Ok(image)
    }

    /// Copy the selected current or committed Debugger state.
    fn debugger(&self, world_id: WorldId, moment: Option<Moment>) -> Result<Debugger, Status> {
        let world = self.world(world_id)?;
        let world = world.lock();
        if let Some(moment) = moment {
            let image = world
                .lineage()
                .image(moment)
                .map_err(Self::runtime_status)?;

            Ok(image.debugger().clone())
        } else {
            Ok(world.debugger().clone())
        }
    }

    /// Load one Program from shared Blob storage.
    fn program(&self, blob: Blob) -> Result<Arc<program::Program>, Status> {
        let memory = self
            .blobs
            .open(blob)
            .map_err(DaemonError::from)
            .map_err(Status::from)?;
        let storage = SectionStorage::from_memory(memory);
        let program = program::Program::load(storage)
            .map_err(|error| Status::new(Code::InvalidArgument, error.to_string()))?;

        Ok(Arc::new(program))
    }

    /// Convert one runtime failure into an RPC status.
    fn runtime_status(error: Box<RuntimeError>) -> Status {
        let code = match error.as_ref() {
            RuntimeError::Entity {
                reason: EntityError::NotFound(_),
            } => Code::NotFound,
            RuntimeError::Entity {
                reason: EntityError::AlreadyExists(_),
            } => Code::AlreadyExists,
            RuntimeError::Configuration { .. } => Code::InvalidArgument,
            RuntimeError::Memory { .. } => Code::ResourceExhausted,
            RuntimeError::Machine {
                reason: MachineError::Unsupported { .. },
            } => Code::Unimplemented,
            RuntimeError::Internal { .. } => Code::Internal,
            _ => Code::FailedPrecondition,
        };

        Status::new(code, error.to_string())
    }
}

impl WorldRegistryState {
    /// Allocate one World identifier.
    fn allocate_id(&mut self) -> Result<WorldId, Status> {
        let world_id = self.next_world_id;
        self.next_world_id = world_id.checked_add(1).ok_or_else(|| {
            Status::new(Code::ResourceExhausted, "World identifier space exhausted")
        })?;

        Ok(WorldId::new(world_id))
    }
}

impl WorldService for WorldRegistry {
    /// Spawn one Runtime from an encoded Program Blob.
    async fn spawn_runtime(
        &self,
        request: Request<SpawnRuntimeRequest>,
    ) -> Result<Response<RuntimeId>, Status> {
        let request = request.value;
        let program = self.program(request.program)?;
        let engine = Engine::new(program, MachineLimits::default());
        let world = self.world(request.world_id)?;
        let options = request.options.unwrap_or_default();
        let environment = request.environment.unwrap_or_default();
        let runtime_id = world
            .lock()
            .spawn_runtime(
                environment,
                &options,
                request.conditions,
                self.bindings.clone(),
                engine,
            )
            .map_err(Self::runtime_status)?;

        Ok(Response::new(runtime_id))
    }

    /// Remove one Runtime from its World.
    async fn remove_runtime(
        &self,
        request: Request<RemoveRuntimeRequest>,
    ) -> Result<Response<()>, Status> {
        let request = request.value;
        let world = self.world(request.world_id)?;
        world
            .lock()
            .remove_runtime(request.runtime_id)
            .map_err(Self::runtime_status)?;

        Ok(Response::new(()))
    }

    /// Invoke one Runtime entrypoint.
    async fn invoke(
        &self,
        request: Request<InvokeRequest>,
    ) -> Result<Response<program::Value>, Status> {
        let request = request.value;
        let world = self.world(request.world_id)?;
        let value = world
            .lock()
            .run_entrypoint(request.runtime_id, &request.entry, &request.arguments)
            .map_err(Self::runtime_status)?;

        Ok(Response::new(value))
    }

    /// Run one unit of World work.
    async fn run(&self, request: Request<RunRequest>) -> Result<Response<RunOutcome>, Status> {
        let request = request.value;
        let world = self.world(request.world_id)?;
        let outcome = world
            .lock()
            .run(request.run)
            .map_err(Self::runtime_status)?;

        Ok(Response::new(outcome))
    }

    /// Read one World's current Moment.
    async fn read_moment(
        &self,
        request: Request<WorldRequest>,
    ) -> Result<Response<Moment>, Status> {
        let world = self.world(request.value.world_id)?;
        let moment = world.lock().moment();

        Ok(Response::new(moment))
    }

    /// List the Runtime identifiers in one World.
    async fn list_runtimes(
        &self,
        request: Request<WorldRequest>,
    ) -> Result<Response<Vec<RuntimeId>>, Status> {
        let world = self.world(request.value.world_id)?;
        let runtime_ids = world.lock().runtime_ids();

        Ok(Response::new(runtime_ids))
    }

    /// Read one Branch in a World's lineage.
    async fn read_branch(
        &self,
        request: Request<ReadBranchRequest>,
    ) -> Result<Response<Branch>, Status> {
        let request = request.value;
        let world = self.world(request.world_id)?;
        let branch = world
            .lock()
            .branch_info(request.branch_id)
            .map_err(Self::runtime_status)?;

        Ok(Response::new(branch))
    }

    /// List the Branches in one World's lineage.
    async fn list_branches(
        &self,
        request: Request<WorldRequest>,
    ) -> Result<Response<Vec<Branch>>, Status> {
        let world = self.world(request.value.world_id)?;
        let branches = world.lock().lineage().branches().into_vec();

        Ok(Response::new(branches))
    }

    /// Rewind one World to a committed Moment.
    async fn rewind(&self, request: Request<RewindRequest>) -> Result<Response<()>, Status> {
        let request = request.value;
        let world = self.world(request.world_id)?;
        world
            .lock()
            .rewind(request.moment)
            .map_err(Self::runtime_status)?;

        Ok(Response::new(()))
    }

    /// Fork one hosted World from a committed Moment.
    async fn fork(&self, request: Request<ForkRequest>) -> Result<Response<WorldId>, Status> {
        let request = request.value;
        let world = self.world(request.world_id)?;
        let child = world
            .lock()
            .fork(request.moment, request.name)
            .map_err(Self::runtime_status)?;
        let mut state = self.state.write();
        let world_id = state.allocate_id()?;
        state.worlds.insert(world_id, Arc::new(Mutex::new(child)));

        Ok(Response::new(world_id))
    }

    /// Read one Checkpoint in a World's lineage.
    async fn read_checkpoint(
        &self,
        request: Request<ReadCheckpointRequest>,
    ) -> Result<Response<Checkpoint>, Status> {
        let request = request.value;
        let world = self.world(request.world_id)?;
        let checkpoint = world
            .lock()
            .checkpoint_info(request.checkpoint_id)
            .map_err(Self::runtime_status)?;

        Ok(Response::new(checkpoint))
    }

    /// List the Checkpoints in one World's lineage.
    async fn list_checkpoints(
        &self,
        request: Request<WorldRequest>,
    ) -> Result<Response<Vec<Checkpoint>>, Status> {
        let world = self.world(request.value.world_id)?;
        let world = world.lock();
        let checkpoints = world
            .checkpoint_ids()
            .into_iter()
            .map(|checkpoint_id| world.checkpoint_info(checkpoint_id))
            .collect::<Result<Vec<_>, _>>()
            .map_err(Self::runtime_status)?;

        Ok(Response::new(checkpoints))
    }

    /// Create one Checkpoint for a World.
    async fn checkpoint(
        &self,
        request: Request<CheckpointRequest>,
    ) -> Result<Response<CheckpointId>, Status> {
        let request = request.value;
        let world = self.world(request.world_id)?;
        let checkpoint_id = world
            .lock()
            .checkpoint(request.name)
            .map_err(Self::runtime_status)?;

        Ok(Response::new(checkpoint_id))
    }

    /// Store one Checkpoint snapshot as a Blob.
    async fn snapshot(&self, request: Request<SnapshotRequest>) -> Result<Response<Blob>, Status> {
        let request = request.value;
        let world = self.world(request.world_id)?;
        let snapshot = world
            .lock()
            .snapshot_checkpoint(request.checkpoint_id)
            .map_err(Self::runtime_status)?;
        let bytes = snapshot.encode().map_err(Self::runtime_status)?;
        let mut input = Cursor::new(bytes);
        let blob = self
            .blobs
            .put(&mut input)
            .map_err(DaemonError::from)
            .map_err(Status::from)?;

        Ok(Response::new(blob))
    }

    /// Restore one World from a snapshot Blob.
    async fn restore(&self, request: Request<RestoreRequest>) -> Result<Response<Moment>, Status> {
        let request = request.value;
        let memory = self
            .blobs
            .open(request.snapshot)
            .map_err(DaemonError::from)
            .map_err(Status::from)?;
        let snapshot = WorldSnapshot::decode(memory.bytes()).map_err(Self::runtime_status)?;
        let world = self.world(request.world_id)?;
        let mut world = world.lock();
        world
            .restore_snapshot(&snapshot, RestoreContext::empty())
            .map_err(Self::runtime_status)?;
        let moment = world.moment();

        Ok(Response::new(moment))
    }
}

impl DebuggerService for WorldRegistry {
    /// Read captured Frames from one World.
    async fn read_frames(
        &self,
        request: Request<FrameRequest>,
    ) -> Result<Response<Vec<Frame>>, Status> {
        let request = request.value;
        let image = self.image(request.world_id, request.moment)?;
        let frames = match request.worker_id {
            Some(worker_id) => image.worker_frames(worker_id),
            None => image.frames(),
        }
        .map_err(Self::runtime_status)?;

        Ok(Response::new(frames))
    }

    /// Read one exact World memory range.
    async fn read_memory(
        &self,
        request: Request<MemoryRequest>,
    ) -> Result<Response<MemoryChunk>, Status> {
        let request = request.value;
        let image = self.image(request.world_id, request.moment)?;
        let bytes = image
            .memory()
            .read_bytes(request.range.offset, request.range.byte_len)
            .map_err(Box::<RuntimeError>::from)
            .map_err(Self::runtime_status)?;

        Ok(Response::new(MemoryChunk {
            range: request.range,
            bytes,
        }))
    }

    /// List the Breakpoints in one World.
    async fn list_breakpoints(
        &self,
        request: Request<DebuggerRequest>,
    ) -> Result<Response<Vec<Breakpoint>>, Status> {
        let request = request.value;
        let debugger = self.debugger(request.world_id, request.moment)?;

        Ok(Response::new(debugger.breakpoints().to_vec()))
    }

    /// Add one Breakpoint to a World.
    async fn add_breakpoint(
        &self,
        request: Request<AddBreakpointRequest>,
    ) -> Result<Response<program::BreakpointId>, Status> {
        let request = request.value;
        let world = self.world(request.world_id)?;
        let breakpoint_id = world
            .lock()
            .add_breakpoint(request.filter)
            .map_err(Self::runtime_status)?;

        Ok(Response::new(breakpoint_id))
    }

    /// Update one Breakpoint in a World.
    async fn update_breakpoint(
        &self,
        request: Request<UpdateBreakpointRequest>,
    ) -> Result<Response<()>, Status> {
        let request = request.value;
        let world = self.world(request.world_id)?;
        world
            .lock()
            .update_breakpoint(request.breakpoint)
            .map_err(Self::runtime_status)?;

        Ok(Response::new(()))
    }

    /// Remove one Breakpoint from a World.
    async fn remove_breakpoint(
        &self,
        request: Request<RemoveBreakpointRequest>,
    ) -> Result<Response<()>, Status> {
        let request = request.value;
        let world = self.world(request.world_id)?;
        world
            .lock()
            .remove_breakpoint(request.breakpoint_id)
            .map_err(Self::runtime_status)?;

        Ok(Response::new(()))
    }

    /// List the Watchpoints in one World.
    async fn list_watchpoints(
        &self,
        request: Request<DebuggerRequest>,
    ) -> Result<Response<Vec<Watchpoint>>, Status> {
        let request = request.value;
        let debugger = self.debugger(request.world_id, request.moment)?;

        Ok(Response::new(debugger.watchpoints().to_vec()))
    }

    /// Add one Watchpoint to a World.
    async fn add_watchpoint(
        &self,
        request: Request<AddWatchpointRequest>,
    ) -> Result<Response<program::WatchpointId>, Status> {
        let request = request.value;
        let world = self.world(request.world_id)?;
        let watchpoint_id = world
            .lock()
            .add_watchpoint(request.filter)
            .map_err(Self::runtime_status)?;

        Ok(Response::new(watchpoint_id))
    }

    /// Update one Watchpoint in a World.
    async fn update_watchpoint(
        &self,
        request: Request<UpdateWatchpointRequest>,
    ) -> Result<Response<()>, Status> {
        let request = request.value;
        let world = self.world(request.world_id)?;
        world
            .lock()
            .update_watchpoint(request.watchpoint)
            .map_err(Self::runtime_status)?;

        Ok(Response::new(()))
    }

    /// Remove one Watchpoint from a World.
    async fn remove_watchpoint(
        &self,
        request: Request<RemoveWatchpointRequest>,
    ) -> Result<Response<()>, Status> {
        let request = request.value;
        let world = self.world(request.world_id)?;
        world
            .lock()
            .remove_watchpoint(request.watchpoint_id)
            .map_err(Self::runtime_status)?;

        Ok(Response::new(()))
    }

    /// List the Probes in one World.
    async fn list_probes(
        &self,
        request: Request<DebuggerRequest>,
    ) -> Result<Response<Vec<Probe>>, Status> {
        let request = request.value;
        let debugger = self.debugger(request.world_id, request.moment)?;

        Ok(Response::new(debugger.probes().collect()))
    }

    /// Read one Probe matching-event count.
    async fn read_probe_count(
        &self,
        request: Request<ProbeRequest>,
    ) -> Result<Response<u64>, Status> {
        let request = request.value;
        let debugger = self.debugger(request.world_id, request.moment)?;
        let count = debugger.probe_count(request.probe_id).ok_or_else(|| {
            Status::new(
                Code::NotFound,
                format!("Probe {} does not exist", request.probe_id.get()),
            )
        })?;

        Ok(Response::new(count))
    }

    /// Add one Probe to a World.
    async fn add_probe(
        &self,
        request: Request<AddProbeRequest>,
    ) -> Result<Response<ProbeId>, Status> {
        let request = request.value;
        let world = self.world(request.world_id)?;
        let probe_id = world
            .lock()
            .add_probe(request.filter, request.action)
            .map_err(Self::runtime_status)?;

        Ok(Response::new(probe_id))
    }

    /// Update one Probe in a World.
    async fn update_probe(
        &self,
        request: Request<UpdateProbeRequest>,
    ) -> Result<Response<()>, Status> {
        let request = request.value;
        let world = self.world(request.world_id)?;
        world
            .lock()
            .update_probe(request.probe)
            .map_err(Self::runtime_status)?;

        Ok(Response::new(()))
    }

    /// Remove one Probe from a World.
    async fn remove_probe(
        &self,
        request: Request<RemoveProbeRequest>,
    ) -> Result<Response<()>, Status> {
        let request = request.value;
        let world = self.world(request.world_id)?;
        world
            .lock()
            .remove_probe(request.probe_id)
            .map_err(Self::runtime_status)?;

        Ok(Response::new(()))
    }
}
