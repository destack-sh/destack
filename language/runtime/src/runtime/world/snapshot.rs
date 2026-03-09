use std::collections::BTreeMap;
use std::fmt;
use std::sync::Arc;
use std::sync::atomic::Ordering;

use destack_base::{Capture, CaptureMode, SnapshotCodec, fnv1a_128};
use serde::{Deserialize, Serialize};

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::resource::ResourceRebindContext;
use crate::runtime::bindings::BindingReplayPayload;
use crate::runtime::policy::PolicyState;
use crate::runtime::random::RandomImage;
use crate::runtime::time::ClockImage;
use crate::runtime::topology::Topology;
use crate::runtime::{AgentId, AgentImage, Runtime, RuntimeImage};
use crate::simulation::Simulation;
use destack_workspace::{
    ExecutionMode, RandomMode, RandomOptions, ReplayOptions, ReplayPayloadMode, RuntimeOptions,
    TimeMode, TimeOptions,
};
use postcard::to_allocvec;

use super::{
    BranchId, CheckpointId, Lineage, LineageSnapshot, RevisionId, RuntimeId, World, WorldResource,
    WorldResourceId,
};

/// Rebinding context for restoring one world image or snapshot.
#[derive(Default, Clone)]
pub struct RebindContext {
    /// Resource rebinding hooks keyed by resource kind.
    resources: ResourceRebindContext,
}

impl fmt::Debug for RebindContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RebindContext")
            .field("resources", &self.resources)
            .finish()
    }
}

impl RebindContext {
    /// Return the resource rebinding context.
    pub fn resources(&self) -> &ResourceRebindContext {
        &self.resources
    }

    /// Return the resource rebinding context mutably.
    pub fn resources_mut(&mut self) -> &mut ResourceRebindContext {
        &mut self.resources
    }
}

/// Runtime construction metadata needed to rebuild one world for snapshot restore.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotConfig {
    /// Execution mode for the rebuilt world.
    pub execution: ExecutionMode,
    /// Effective world time mode.
    pub time_mode: TimeMode,
    /// Effective world random mode.
    pub random_mode: RandomMode,
    /// Replay payload policy used for the trace.
    pub replay_payload: ReplayPayloadMode,
    /// Trace chunk sizing in megabytes, when configured.
    pub replay_chunk_size_mb: Option<u64>,
}

/// Image identifier for one materialized world state image.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ImageId(u128);

impl ImageId {
    /// Create a new image identifier.
    pub const fn new(value: u128) -> Self {
        Self(value)
    }

    /// Return the raw image identifier value.
    pub const fn get(self) -> u128 {
        self.0
    }
}

/// Trace image identifier for one materialized lineage trace state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct TraceImageId(u128);

impl TraceImageId {
    /// Create a new trace image identifier.
    pub const fn new(value: u128) -> Self {
        Self(value)
    }

    /// Return the raw trace image identifier value.
    pub const fn get(self) -> u128 {
        self.0
    }
}

/// Image payload for one materialized world restore point.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Image {
    /// The image identifier.
    pub id: ImageId,
    /// The next runtime id to allocate after restore.
    pub(crate) next_runtime_id: u64,
    /// The next agent id to allocate after restore.
    pub(crate) next_agent_id: u64,
    /// Captured dynamic policy state.
    pub(crate) policy: PolicyState,
    /// Captured topology metadata graph.
    pub(crate) topology: Topology,
    /// Captured logical world resources.
    pub(crate) resources: BTreeMap<WorldResourceId, WorldResource>,
    /// Captured simulation state.
    pub(crate) simulation: Simulation,
    /// Captured world clock state.
    pub(crate) clock: ClockImage,
    /// Captured world random state.
    pub(crate) random: RandomImage,
    /// Captured runtime metadata keyed by runtime id.
    pub(crate) runtimes: BTreeMap<RuntimeId, RuntimeImage>,
    /// Captured agent metadata keyed by agent id.
    pub(crate) agents: BTreeMap<AgentId, AgentImage>,
}

/// Serialized snapshot for one world image.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snapshot {
    /// The snapshot format version.
    pub format_version: u32,
    /// Runtime construction metadata for the restored world.
    pub config: SnapshotConfig,
    /// The active branch captured by this snapshot.
    pub active_branch_id: BranchId,
    /// The revision selected for restore from this snapshot.
    pub revision_id: RevisionId,
    /// The captured image.
    pub image: Image,
    /// The captured lineage metadata.
    pub lineage: LineageSnapshot,
}

impl Snapshot {
    /// Create one serialized snapshot wrapper for one image.
    pub const fn new(
        config: SnapshotConfig,
        active_branch_id: BranchId,
        revision_id: RevisionId,
        image: Image,
        lineage: LineageSnapshot,
    ) -> Self {
        Self {
            format_version: 1,
            config,
            active_branch_id,
            revision_id,
            image,
            lineage,
        }
    }

    /// Encode one snapshot into bytes.
    pub fn encode(&self) -> RuntimeResult<Vec<u8>> {
        to_allocvec(self).map_err(|_| {
            RuntimeError::Internal {
                message: "failed to encode world snapshot".to_string(),
            }
            .boxed()
        })
    }

    /// Decode one snapshot from bytes.
    pub fn decode(bytes: &[u8]) -> RuntimeResult<Self> {
        postcard::from_bytes(bytes).map_err(|_| {
            RuntimeError::Internal {
                message: "failed to decode world snapshot".to_string(),
            }
            .boxed()
        })
    }
}

impl World {
    /// Build one snapshot world configuration from the live world.
    fn snapshot_config(&self) -> SnapshotConfig {
        let header = self.trace.log().header();
        let replay_payload = match header.replay_payload {
            BindingReplayPayload::Results => ReplayPayloadMode::ResultsOnly,
            BindingReplayPayload::ArgumentsAndResults => ReplayPayloadMode::ArgumentsAndResults,
        };
        let replay_chunk_size_mb = if header.max_chunk_bytes == 0 {
            None
        } else {
            Some(header.max_chunk_bytes / (1024 * 1024))
        };

        SnapshotConfig {
            execution: self.trace.mode(),
            time_mode: self.time_mode,
            random_mode: self.random_mode,
            replay_payload,
            replay_chunk_size_mb,
        }
    }

    /// Build runtime options for one serialized world snapshot.
    fn runtime_options_from_snapshot(snapshot: &Snapshot) -> RuntimeOptions {
        let mut options = RuntimeOptions {
            execution: snapshot.config.execution,
            time: TimeOptions {
                mode: snapshot.config.time_mode,
                ..TimeOptions::default()
            },
            random: RandomOptions {
                mode: snapshot.config.random_mode,
                ..RandomOptions::default()
            },
            replay: ReplayOptions {
                chunk_size_mb: snapshot.config.replay_chunk_size_mb,
                payload: snapshot.config.replay_payload,
                ..ReplayOptions::default()
            },
            ..RuntimeOptions::default()
        };

        if snapshot.config.execution == ExecutionMode::Replay {
            options.time.mode = TimeMode::Virtual;
            options.random.mode = RandomMode::Deterministic;
        }

        options
    }

    /// Return metadata for one stored image.
    pub fn image_info(&self, image_id: ImageId) -> RuntimeResult<Image> {
        let lineage = self.lineage.read();
        let image = lineage.images.get(&image_id).ok_or_else(|| {
            RuntimeError::Internal {
                message: format!("image {} does not exist", image_id.get()),
            }
            .boxed()
        })?;

        Ok(image.as_ref().clone())
    }

    /// Return identifiers for all stored images in stable order.
    pub fn image_ids(&self) -> Vec<ImageId> {
        self.lineage.read().images.keys().copied().collect()
    }

    /// Create one serialized snapshot from one stored image.
    pub fn snapshot(&self, image_id: ImageId) -> RuntimeResult<Snapshot> {
        let (image, revision_id, lineage_snapshot) = {
            let lineage = self.lineage.read();
            let image = lineage.images.get(&image_id).ok_or_else(|| {
                RuntimeError::Internal {
                    message: format!("image {} does not exist", image_id.get()),
                }
                .boxed()
            })?;
            let revision_id = lineage.revision_id_for_image_id(image_id).ok_or_else(|| {
                RuntimeError::Internal {
                    message: format!("image {} does not belong to one revision", image_id.get()),
                }
                .boxed()
            })?;

            (image.as_ref().clone(), revision_id, lineage.snapshot())
        };

        Ok(Snapshot::new(
            self.snapshot_config(),
            self.branch_id,
            revision_id,
            image,
            lineage_snapshot,
        ))
    }

    /// Create one serialized snapshot from one specific revision.
    pub fn snapshot_revision(&self, revision_id: RevisionId) -> RuntimeResult<Snapshot> {
        let image_id = {
            let lineage = self.lineage.read();
            let revision = lineage.revisions.get(&revision_id).ok_or_else(|| {
                RuntimeError::Internal {
                    message: format!("revision {} does not exist", revision_id.get()),
                }
                .boxed()
            })?;

            revision.image_id
        };

        self.snapshot(image_id)
    }

    /// Create one serialized snapshot from one stored checkpoint.
    pub fn snapshot_checkpoint(&self, checkpoint_id: CheckpointId) -> RuntimeResult<Snapshot> {
        let revision_id = {
            let lineage = self.lineage.read();
            let checkpoint = lineage.checkpoints.get(&checkpoint_id).ok_or_else(|| {
                RuntimeError::Internal {
                    message: format!("checkpoint {} does not exist", checkpoint_id.get()),
                }
                .boxed()
            })?;

            checkpoint.revision_id
        };

        self.snapshot_revision(revision_id)
    }

    /// Build one fresh world from one serialized snapshot.
    pub fn from_snapshot(
        snapshot: &Snapshot,
        rebind_context: Option<&RebindContext>,
    ) -> RuntimeResult<Arc<Self>> {
        let options = Self::runtime_options_from_snapshot(snapshot);
        let world = Self::build_with_branch(snapshot.active_branch_id, &options, None)?;
        world.restore_snapshot(snapshot, rebind_context)?;

        Ok(world)
    }

    /// Build one fresh world from one encoded snapshot payload.
    pub fn from_snapshot_bytes(
        bytes: &[u8],
        rebind_context: Option<&RebindContext>,
    ) -> RuntimeResult<Arc<Self>> {
        let snapshot = Snapshot::decode(bytes)?;

        Self::from_snapshot(&snapshot, rebind_context)
    }

    /// Restore one serialized snapshot into this world.
    pub fn restore_snapshot(
        &self,
        snapshot: &Snapshot,
        rebind_context: Option<&RebindContext>,
    ) -> RuntimeResult<()> {
        if snapshot.active_branch_id != self.branch_id {
            return Err(RuntimeError::Internal {
                message: format!(
                    "snapshot active branch {} does not match world branch {}",
                    snapshot.active_branch_id.get(),
                    self.branch_id.get()
                ),
            }
            .boxed());
        }

        let _exclusive_access = self.acquire_exclusive_access()?;
        *self.lineage.write() = Lineage::from_snapshot(snapshot.lineage.clone());
        let trace_image = {
            let lineage = self.lineage.read();
            let backing = lineage.resolve_revision_backing(snapshot.revision_id)?;

            backing.trace_image
        };

        self.restore_revision_image(
            snapshot.revision_id,
            &snapshot.image,
            &trace_image,
            rebind_context,
        )
    }

    /// Capture one materialized world image while the world is under exclusive access.
    pub(crate) fn capture_image(&self, mode: CaptureMode) -> RuntimeResult<Image> {
        // runtime and agent state
        let mut runtimes = self.runtimes.write();
        let mut runtime_images = BTreeMap::new();
        let mut agent_images = BTreeMap::new();

        for runtime in runtimes.values_mut() {
            let (runtime_image, runtime_agents) = runtime.capture_image(mode)?;
            runtime_images.insert(runtime_image.runtime_id, runtime_image);
            agent_images.extend(runtime_agents);
        }

        Ok(Image {
            id: ImageId::new(0),
            next_runtime_id: self.next_runtime_id.load(Ordering::SeqCst),
            next_agent_id: self.next_agent_id.load(Ordering::SeqCst),
            policy: self.policy.read().clone(),
            topology: self.topology.read().clone(),
            resources: self.resources.read().clone(),
            simulation: self.simulation.read().clone(),
            clock: self.clock.snapshot(),
            random: self.random.snapshot(),
            runtimes: runtime_images,
            agents: agent_images,
        })
    }

    /// Restore one materialized image into this world while the world is quiesced.
    pub(crate) fn restore_image(
        &self,
        image: &Image,
        rebind_context: Option<&RebindContext>,
    ) -> RuntimeResult<()> {
        // world-owned state
        self.next_runtime_id
            .store(image.next_runtime_id, Ordering::SeqCst);
        self.next_agent_id
            .store(image.next_agent_id, Ordering::SeqCst);

        *self.policy.write() = image.policy.clone();
        *self.topology.write() = image.topology.clone();
        *self.resources.write() = image.resources.clone();
        *self.simulation.write() = image.simulation.clone();

        self.clock.restore_snapshot(&image.clock);
        self.random.restore_snapshot(&image.random)?;

        // runtime and agent state
        let mut agent_images_by_runtime = BTreeMap::new();
        for agent_image in image.agents.values() {
            agent_images_by_runtime
                .entry(agent_image.runtime_id)
                .or_insert_with(BTreeMap::new)
                .insert(agent_image.agent_id, agent_image.clone());
        }

        let mut restored_runtimes = BTreeMap::new();
        for runtime_image in image.runtimes.values() {
            let runtime_agent_images = agent_images_by_runtime
                .remove(&runtime_image.runtime_id)
                .unwrap_or_default();
            let runtime =
                Runtime::from_image(self, runtime_image, &runtime_agent_images, rebind_context)?;
            restored_runtimes.insert(runtime_image.runtime_id, Box::new(runtime));
        }

        // reject dangling agent images that do not belong to any restored runtime
        if let Some((runtime_id, _)) = agent_images_by_runtime.into_iter().next() {
            return Err(RuntimeError::Internal {
                message: format!("image contains agents for missing runtime {}", runtime_id.0),
            }
            .boxed());
        }

        *self.runtimes.write() = restored_runtimes;

        Ok(())
    }

    /// Return the encoded size and hash for one image.
    pub(crate) fn image_size_and_hash(image: &Image) -> RuntimeResult<(u64, u128)> {
        let bytes = to_allocvec(image).map_err(|_| {
            RuntimeError::Internal {
                message: "failed to encode world image".to_string(),
            }
            .boxed()
        })?;
        let size_bytes = bytes.len() as u64;
        let hash = fnv1a_128(&bytes);

        Ok((size_bytes, hash))
    }
}

impl Capture for World {
    type Image = Image;
    type Error = Box<RuntimeError>;
    type CaptureContext<'a> = ();
    type RestoreContext<'a> = Option<&'a RebindContext>;

    /// Capture one world image under one exclusive-access lease.
    fn capture_image(
        &mut self,
        mode: CaptureMode,
        _context: Self::CaptureContext<'_>,
    ) -> Result<Self::Image, Self::Error> {
        let _exclusive_access = self.acquire_exclusive_access()?;
        World::capture_image(self, mode)
    }

    /// Restore one world image under one exclusive-access lease.
    fn restore_image(
        &mut self,
        image: &Self::Image,
        context: Self::RestoreContext<'_>,
    ) -> Result<(), Self::Error> {
        let _exclusive_access = self.acquire_exclusive_access()?;
        World::restore_image(self, image, context)
    }
}

impl SnapshotCodec for World {
    type Snapshot = Snapshot;

    /// Encode one world image as one world snapshot.
    fn encode_snapshot(_image: &Self::Image) -> Result<Self::Snapshot, Self::Error> {
        Err(RuntimeError::Internal {
            message: "world snapshot export requires world lineage context".to_string(),
        }
        .boxed())
    }

    /// Decode one world snapshot back into one world image.
    fn decode_snapshot(snapshot: &Self::Snapshot) -> Result<Self::Image, Self::Error> {
        Ok(snapshot.image.clone())
    }
}
