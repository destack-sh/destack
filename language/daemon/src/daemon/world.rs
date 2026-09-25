use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::io::Cursor;
use std::sync::Arc;

use parking_lot::{Mutex, RwLock};
use tspp_core::{Blob, BlobStore, SectionStorage};
use tspp_program as program;
use tspp_repository::{Environment, WorldOptions};
use tspp_rpc::{Code, Request, Response, ResponseSender, Status};
use tspp_runtime::binding::BindingTable;
use tspp_runtime::debugger::Debugger;
use tspp_runtime::diagnostic::{EntityError, MachineError, RuntimeError};
use tspp_runtime::machine::Engine;
use tspp_runtime::service::{
    self, AddRuleRequest, CaptureRequest, ForkRequest, InvokeRequest, ListBranchesRequest,
    ListImagesRequest, ListObservationsRequest, ListRuntimesRequest, ObservationPage,
    ReadBranchRequest, ReadImageRequest, ReadMomentRequest, ReadPolicyRequest, ReadRuntimeRequest,
    ReadTopologyRequest, ReloadRuntimeRequest, RemoveRuleRequest, RemoveRuntimeRequest,
    ReplacePolicyRequest, ReplaceRuleRequest, RewindRequest, RunRequest, SnapshotRequest,
    SpawnRuntimeRequest, WatchObservationsRequest, WorldId, WorldService,
};
use tspp_runtime::world::observation::ObservationEntry;
use tspp_runtime::world::{
    Branch, Image, Moment, Policy, RestoreContext, RunOutcome, Snapshot, World, WorldImage,
    WorldSnapshot,
};
use tspp_vm::MachineLimits;

use crate::DaemonError;

/// Worlds hosted by one daemon process.
#[derive(Clone)]
pub(crate) struct WorldRegistry {
    /// Content-addressed bytes shared by daemon services.
    blobs: Arc<BlobStore>,
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

impl fmt::Debug for WorldRegistry {
    /// Format hosted World registration state.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("WorldRegistry")
            .field("bindings", &self.bindings)
            .field("worlds", &self.state.read().worlds.keys())
            .finish()
    }
}

impl WorldRegistry {
    /// Create one empty World registry.
    pub(crate) fn new(blobs: Arc<BlobStore>) -> Self {
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

    /// List the registered World identifiers.
    pub(crate) fn list(&self) -> Vec<WorldId> {
        self.state.read().worlds.keys().copied().collect()
    }

    /// Restore one World from a Blob-backed Snapshot.
    pub(crate) fn restore(&self, snapshot: Snapshot) -> Result<WorldId, Status> {
        let memory = self
            .blobs
            .open(snapshot.blob)
            .map_err(DaemonError::from)
            .map_err(Status::from)?;
        let snapshot = WorldSnapshot::decode(memory.bytes()).map_err(Self::runtime_status)?;
        let restore = RestoreContext::empty().with_bindings(&self.bindings);
        let world = World::from_snapshot(&snapshot, restore).map_err(Self::runtime_status)?;
        self.publish_programs(&snapshot)?;

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
    pub(crate) fn world(&self, world_id: WorldId) -> Result<Arc<Mutex<World>>, Status> {
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
    pub(crate) fn image(
        &self,
        world_id: WorldId,
        moment: Option<Moment>,
    ) -> Result<(Moment, WorldImage), Status> {
        let world = self.world(world_id)?;
        let mut world = world.lock();
        let (moment, image) = match moment {
            // materialize the requested committed Moment
            Some(moment) => {
                let restore = RestoreContext::empty().with_bindings(&self.bindings);
                let image = world
                    .lineage()
                    .materialize(moment, restore)
                    .map_err(Self::runtime_status)?;

                (moment, image)
            }
            // capture the current live Moment and image together
            None => {
                let moment = world.moment();
                let image = world.image().map_err(Self::runtime_status)?;

                (moment, image)
            }
        };

        Ok((moment, image))
    }

    /// Copy the selected current or committed Debugger state.
    pub(crate) fn debugger(
        &self,
        world_id: WorldId,
        moment: Option<Moment>,
    ) -> Result<Debugger, Status> {
        let world = self.world(world_id)?;
        let world = world.lock();
        if let Some(moment) = moment {
            let restore = RestoreContext::empty().with_bindings(&self.bindings);
            let image = world
                .lineage()
                .materialize(moment, restore)
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

    /// Publish every distinct Program retained by one Snapshot.
    fn publish_programs(&self, snapshot: &WorldSnapshot) -> Result<(), Status> {
        let mut blobs = BTreeSet::new();
        for program in snapshot.programs() {
            let expected = program.blob();

            // skip Programs already seen in another retained Image
            if !blobs.insert(expected) {
                continue;
            }

            // skip Programs already available through this daemon
            let is_present = self
                .blobs
                .contains(expected)
                .map_err(DaemonError::from)
                .map_err(Status::from)?;
            if is_present {
                continue;
            }

            // publish the exact image before exposing its Blob through the service
            let mut bytes = Cursor::new(program.bytes());
            let published = self
                .blobs
                .retain(&mut bytes)
                .map_err(DaemonError::from)
                .map_err(Status::from)?;
            if published != expected {
                return Err(Status::new(
                    Code::Internal,
                    format!("Program Blob changed from {expected} to {published}"),
                ));
            }
        }

        Ok(())
    }

    /// Convert one runtime failure into an RPC status.
    pub(crate) fn runtime_status(error: Box<RuntimeError>) -> Status {
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
    // =============================================================================
    // World
    // =============================================================================

    /// Read one World's current Moment.
    async fn read_moment(
        &self,
        request: Request<ReadMomentRequest>,
    ) -> Result<Response<Moment>, Status> {
        let world = self.world(request.value.world_id)?;
        let moment = world.lock().moment();

        Ok(Response::new(moment))
    }

    // =============================================================================
    // Runtime
    // =============================================================================

    /// Read one Runtime in a World.
    async fn read_runtime(
        &self,
        request: Request<ReadRuntimeRequest>,
    ) -> Result<Response<service::Runtime>, Status> {
        let request = request.value;
        let world = self.world(request.world_id)?;
        let world = world.lock();
        let runtime = world
            .runtime(request.runtime_id)
            .map(service::Runtime::from)
            .map_err(Self::runtime_status)?;

        Ok(Response::new(runtime))
    }

    /// List the Runtimes in one World.
    async fn list_runtimes(
        &self,
        request: Request<ListRuntimesRequest>,
    ) -> Result<Response<Vec<service::Runtime>>, Status> {
        let world = self.world(request.value.world_id)?;
        let world = world.lock();
        let runtimes = world
            .runtime_ids()
            .into_iter()
            .map(|runtime_id| world.runtime(runtime_id).map(service::Runtime::from))
            .collect::<Result<Vec<_>, _>>()
            .map_err(Self::runtime_status)?;

        Ok(Response::new(runtimes))
    }

    /// Spawn one Runtime from an encoded Program Blob.
    async fn spawn_runtime(
        &self,
        request: Request<SpawnRuntimeRequest>,
    ) -> Result<Response<service::Runtime>, Status> {
        let request = request.value;
        let program = self.program(request.program)?;
        let engine = Engine::new(program, MachineLimits::default());
        let world = self.world(request.world_id)?;
        let options = request.options.unwrap_or_default();
        let environment = request.environment.unwrap_or_default();
        let mut world = world.lock();
        let runtime_id = world
            .spawn_runtime(
                environment,
                &options,
                request.conditions,
                self.bindings.clone(),
                engine,
            )
            .map_err(Self::runtime_status)?;
        let runtime = world
            .runtime(runtime_id)
            .map(service::Runtime::from)
            .map_err(Self::runtime_status)?;

        Ok(Response::new(runtime))
    }

    /// Replace one Runtime's Program at a committed safepoint.
    async fn reload_runtime(
        &self,
        _request: Request<ReloadRuntimeRequest>,
    ) -> Result<Response<service::Runtime>, Status> {
        todo!("reload one Runtime Program at a committed safepoint")
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

    // =============================================================================
    // Execution
    // =============================================================================

    /// Invoke one Runtime entrypoint.
    async fn invoke(
        &self,
        request: Request<InvokeRequest>,
    ) -> Result<Response<program::Value>, Status> {
        let request = request.value;
        let world = self.world(request.world_id)?;
        let value = world
            .lock()
            .invoke(request.runtime_id, &request.entry, &request.arguments)
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

    // =============================================================================
    // Observation
    // =============================================================================

    /// List one page of World Observations.
    async fn list_observations(
        &self,
        request: Request<ListObservationsRequest>,
    ) -> Result<Response<ObservationPage>, Status> {
        let request = request.value;

        // require a cursor that can make forward progress
        if request.limit == 0 {
            return Err(Status::new(
                Code::InvalidArgument,
                "Observation page limit must be greater than zero",
            ));
        }

        // retain one additional match to detect another page
        let limit = usize::try_from(request.limit)
            .map_err(|_| Status::new(Code::OutOfRange, "Observation page limit exceeds usize"))?;
        let query_limit = limit
            .checked_add(1)
            .ok_or_else(|| Status::new(Code::OutOfRange, "Observation page limit exceeds usize"))?;

        // stop the Observation log traversal at the page boundary
        let world = self.world(request.world_id)?;
        let world = world.lock();
        let mut observations = world.observations().query(&request.query, query_limit);

        // return only the requested page and its continuation cursor
        let has_more = observations.len() > limit;
        observations.truncate(limit);
        let next = if has_more {
            observations.last().map(|entry| entry.sequence)
        } else {
            None
        };

        Ok(Response::new(ObservationPage { observations, next }))
    }

    /// Watch World Observations after one sequence.
    async fn watch_observations(
        &self,
        _request: Request<WatchObservationsRequest>,
        _responses: ResponseSender<ObservationEntry>,
    ) -> Result<Response<()>, Status> {
        todo!("stream live World Observations without polling")
    }

    // =============================================================================
    // Topology
    // =============================================================================

    /// Read one World's Topology.
    async fn read_topology(
        &self,
        request: Request<ReadTopologyRequest>,
    ) -> Result<Response<service::Topology>, Status> {
        let request = request.value;
        let topology = match request.moment {
            // materialize retained topology from one committed Moment
            Some(moment) => {
                let (_, image) = self.image(request.world_id, Some(moment))?;

                service::Topology {
                    entity_definitions: image.entity_kinds().into_values().collect(),
                    edge_definitions: image.edge_kinds().into_values().collect(),
                    entities: image.entities().into_values().collect(),
                    edges: image.edges().into_values().collect(),
                }
            }
            // copy current topology without capturing an entire World image
            None => {
                let world = self.world(request.world_id)?;
                let world = world.lock();

                service::Topology {
                    entity_definitions: world.entity_kinds().into_values().collect(),
                    edge_definitions: world.edge_kinds().into_values().collect(),
                    entities: world.entities().into_values().collect(),
                    edges: world.edges().into_values().collect(),
                }
            }
        };

        Ok(Response::new(topology))
    }

    // =============================================================================
    // Policy
    // =============================================================================

    /// Read one World's Policy.
    async fn read_policy(
        &self,
        request: Request<ReadPolicyRequest>,
    ) -> Result<Response<Policy>, Status> {
        let request = request.value;
        let policy = match request.moment {
            // materialize retained policy from one committed Moment
            Some(moment) => {
                let (_, image) = self.image(request.world_id, Some(moment))?;

                image.policy().clone()
            }
            // copy current policy without capturing an entire World image
            None => {
                let world = self.world(request.world_id)?;

                world.lock().policy()
            }
        };

        Ok(Response::new(policy))
    }

    /// Replace one World's active Policy.
    async fn replace_policy(
        &self,
        request: Request<ReplacePolicyRequest>,
    ) -> Result<Response<()>, Status> {
        let request = request.value;
        let world = self.world(request.world_id)?;
        world
            .lock()
            .set_policy(request.policy)
            .map_err(Self::runtime_status)?;

        Ok(Response::new(()))
    }

    /// Add one Rule to a World's active Policy.
    async fn add_rule(&self, request: Request<AddRuleRequest>) -> Result<Response<()>, Status> {
        let request = request.value;
        let world = self.world(request.world_id)?;
        world
            .lock()
            .add_rule(request.rule)
            .map_err(Self::runtime_status)?;

        Ok(Response::new(()))
    }

    /// Replace one Rule in a World's active Policy.
    async fn replace_rule(
        &self,
        request: Request<ReplaceRuleRequest>,
    ) -> Result<Response<()>, Status> {
        let request = request.value;
        let rule_id = request.rule.id.clone();
        let world = self.world(request.world_id)?;
        world
            .lock()
            .replace_rule(rule_id, request.rule)
            .map_err(Self::runtime_status)?;

        Ok(Response::new(()))
    }

    /// Remove one Rule from a World's active Policy.
    async fn remove_rule(
        &self,
        request: Request<RemoveRuleRequest>,
    ) -> Result<Response<()>, Status> {
        let request = request.value;
        let world = self.world(request.world_id)?;
        world
            .lock()
            .remove_rule(request.rule_id)
            .map_err(Self::runtime_status)?;

        Ok(Response::new(()))
    }

    // =============================================================================
    // Branch
    // =============================================================================

    /// Read one Branch in a World's lineage.
    async fn read_branch(
        &self,
        request: Request<ReadBranchRequest>,
    ) -> Result<Response<Branch>, Status> {
        let request = request.value;
        let world = self.world(request.world_id)?;
        let branch = world
            .lock()
            .lineage()
            .branch(request.branch_id)
            .map_err(Self::runtime_status)?;

        Ok(Response::new(branch))
    }

    /// List the Branches in one World's lineage.
    async fn list_branches(
        &self,
        request: Request<ListBranchesRequest>,
    ) -> Result<Response<Vec<Branch>>, Status> {
        let world = self.world(request.value.world_id)?;
        let branches = world.lock().lineage().branches();

        Ok(Response::new(branches))
    }

    /// Rewind one World to a committed Moment.
    async fn rewind(&self, request: Request<RewindRequest>) -> Result<Response<()>, Status> {
        let request = request.value;
        let world = self.world(request.world_id)?;
        let restore = RestoreContext::empty().with_bindings(&self.bindings);
        world
            .lock()
            .rewind(request.moment, restore)
            .map_err(Self::runtime_status)?;

        Ok(Response::new(()))
    }

    /// Fork one hosted World from a committed Moment.
    async fn fork(&self, request: Request<ForkRequest>) -> Result<Response<WorldId>, Status> {
        let request = request.value;
        let world = self.world(request.world_id)?;
        let restore = RestoreContext::empty().with_bindings(&self.bindings);
        let child = world
            .lock()
            .fork(request.moment, request.name, restore)
            .map_err(Self::runtime_status)?;
        let mut state = self.state.write();
        let world_id = state.allocate_id()?;
        state.worlds.insert(world_id, Arc::new(Mutex::new(child)));

        Ok(Response::new(world_id))
    }

    // =============================================================================
    // Image
    // =============================================================================

    /// Read one retained Image in a World's lineage.
    async fn read_image(
        &self,
        request: Request<ReadImageRequest>,
    ) -> Result<Response<Image>, Status> {
        let request = request.value;
        let world = self.world(request.world_id)?;
        let image = world
            .lock()
            .lineage()
            .image(request.image_id)
            .map_err(Self::runtime_status)?;

        Ok(Response::new(image))
    }

    /// List the retained Images in one World's lineage.
    async fn list_images(
        &self,
        request: Request<ListImagesRequest>,
    ) -> Result<Response<Vec<Image>>, Status> {
        let world = self.world(request.value.world_id)?;
        let images = world.lock().lineage().images();

        Ok(Response::new(images))
    }

    /// Capture one named Image for a World.
    async fn capture(&self, request: Request<CaptureRequest>) -> Result<Response<Image>, Status> {
        let request = request.value;
        let world = self.world(request.world_id)?;
        let image = world
            .lock()
            .capture(request.name)
            .map_err(Self::runtime_status)?;

        Ok(Response::new(image))
    }

    // =============================================================================
    // Snapshot
    // =============================================================================

    /// Store one retained Image as a Blob-backed Snapshot.
    async fn snapshot(
        &self,
        request: Request<SnapshotRequest>,
    ) -> Result<Response<Snapshot>, Status> {
        let request = request.value;
        let world = self.world(request.world_id)?;
        let snapshot = world
            .lock()
            .snapshot_image(request.image_id)
            .map_err(Self::runtime_status)?;
        let bytes = snapshot.encode().map_err(Self::runtime_status)?;
        let mut input = Cursor::new(bytes);
        let blob = self
            .blobs
            .retain(&mut input)
            .map_err(DaemonError::from)
            .map_err(Status::from)?;

        Ok(Response::new(Snapshot { blob }))
    }
}
