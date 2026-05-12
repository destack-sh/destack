use std::collections::BTreeMap;
use std::sync::Arc;

use destack_core::{CaptureMode, fnv1a_128};
use serde::{Deserialize, Serialize};

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::ResourceId;
use crate::platform::resource::ResourceRebinders;
use crate::runtime::binding::BindingReplayPayload;
use crate::runtime::policy::{Policy, PolicyState};
use crate::runtime::random::RandomImage;
use crate::runtime::time::ClockImage;
use crate::runtime::topology::{Edge, Entity, RuntimeId, Topology};
use crate::runtime::world::{Resource, World};
use crate::runtime::{Runtime, RuntimeImage, WorkerId, WorkerImage};
use crate::simulation::Simulation;
use destack_workspace::{
    ExecutionMode, RandomMode, ReplayPayloadMode, RuntimeOptions, SchedulerMode, TimeMode,
};
use postcard::to_allocvec;

use super::{CheckpointId, Lineage, LineageSnapshot, Revision, RevisionState};

/// Runtime construction metadata needed to rebuild one world for snapshot restore.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotConfig {
    /// Execution mode for the rebuilt world.
    pub execution: ExecutionMode,
    /// Scheduler mode for the rebuilt world.
    pub scheduler_mode: SchedulerMode,
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

/// Image identifier for one retained trace restore record.
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

/// World image payload for one materialized world restore point.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldImage {
    /// The next runtime id to allocate after restore.
    pub(crate) next_runtime_id: u64,
    /// The next worker id to allocate after restore.
    pub(crate) next_worker_id: u64,
    /// Captured dynamic policy state.
    pub(crate) policy: PolicyState,
    /// Captured topology metadata graph.
    pub(crate) topology: Topology,
    /// Captured world resources.
    pub(crate) resources: BTreeMap<ResourceId, Resource>,
    /// Captured simulation state.
    pub(crate) simulation: Simulation,
    /// Captured world clock state.
    pub(crate) clock: ClockImage,
    /// Captured world random state.
    pub(crate) random: RandomImage,
    /// Captured runtime metadata keyed by runtime id.
    pub(crate) runtimes: BTreeMap<RuntimeId, Arc<RuntimeImage>>,
    /// Captured worker metadata keyed by worker id.
    pub(crate) workers: BTreeMap<WorkerId, Arc<WorkerImage>>,
}

impl WorldImage {
    /// Return the captured world policy specification.
    pub fn policy(&self) -> &Policy {
        &self.policy.spec
    }

    /// Return the captured runtimes keyed by runtime id.
    pub fn runtimes(&self) -> &BTreeMap<RuntimeId, Arc<RuntimeImage>> {
        &self.runtimes
    }

    /// Return the captured workers keyed by worker id.
    pub fn workers(&self) -> &BTreeMap<WorkerId, Arc<WorkerImage>> {
        &self.workers
    }

    /// Return the captured world resources keyed by resource id.
    pub fn resources(&self) -> &BTreeMap<ResourceId, Resource> {
        &self.resources
    }

    /// Return the number of captured runtimes.
    pub fn runtime_count(&self) -> usize {
        self.runtimes.len()
    }

    /// Return the number of captured workers.
    pub fn worker_count(&self) -> usize {
        self.workers.len()
    }

    /// Return the number of captured logical resources.
    pub fn resource_count(&self) -> usize {
        self.resources.len()
    }

    /// Return the number of captured topology entities.
    pub fn entity_count(&self) -> usize {
        self.topology.entities().len()
    }

    /// Return the number of captured topology edges.
    pub fn edge_count(&self) -> usize {
        self.topology.edges().len()
    }

    /// Report whether one runtime exists in this image.
    pub fn has_runtime(&self, runtime_id: RuntimeId) -> bool {
        self.runtimes.contains_key(&runtime_id)
    }

    /// Report whether one worker exists in this image.
    pub fn has_worker(&self, worker_id: WorkerId) -> bool {
        self.workers.contains_key(&worker_id)
    }

    /// Report whether one resource exists in this image.
    pub fn has_resource(&self, resource_id: ResourceId) -> bool {
        self.resources.contains_key(&resource_id)
    }

    /// Report whether one topology entity exists in this image.
    pub fn has_entity(&self, entity_id: &str) -> bool {
        self.topology.entities().contains_key(entity_id)
    }

    /// Report whether one topology edge exists in this image.
    pub fn has_edge(&self, edge_id: &str) -> bool {
        self.topology.edges().contains_key(edge_id)
    }

    /// Return one runtime image by id.
    pub fn runtime(&self, runtime_id: RuntimeId) -> RuntimeResult<&RuntimeImage> {
        self.runtimes
            .get(&runtime_id)
            .map(Arc::as_ref)
            .ok_or_else(|| {
                RuntimeError::RuntimeNotFound {
                    runtime_id: runtime_id.0,
                }
                .boxed()
            })
    }

    /// Return labels for one runtime image.
    pub fn runtime_labels(
        &self,
        runtime_id: RuntimeId,
    ) -> RuntimeResult<&BTreeMap<String, String>> {
        let entity_id = runtime_id.entity_id();
        let entity = self.topology.entities().get(entity_id.as_str()).ok_or(
            RuntimeError::RuntimeNotFound {
                runtime_id: runtime_id.0,
            },
        )?;

        Ok(&entity.labels)
    }

    /// Return the topology name for one runtime image.
    pub fn runtime_name(&self, runtime_id: RuntimeId) -> RuntimeResult<&str> {
        let (name, _) = self.topology.runtime_subject(runtime_id).ok_or_else(|| {
            RuntimeError::RuntimeNotFound {
                runtime_id: runtime_id.0,
            }
            .boxed()
        })?;

        Ok(name)
    }

    /// Return one worker image by id.
    pub fn worker(&self, worker_id: WorkerId) -> RuntimeResult<&WorkerImage> {
        self.workers
            .get(&worker_id)
            .map(Arc::as_ref)
            .ok_or_else(|| {
                RuntimeError::WorkerNotFound {
                    worker_id: worker_id.0,
                }
                .boxed()
            })
    }

    /// Return labels for one worker image.
    pub fn worker_labels(&self, worker_id: WorkerId) -> RuntimeResult<&BTreeMap<String, String>> {
        let entity_id = worker_id.entity_id();
        let entity = self.topology.entities().get(entity_id.as_str()).ok_or(
            RuntimeError::WorkerNotFound {
                worker_id: worker_id.0,
            },
        )?;

        Ok(&entity.labels)
    }

    /// Return the topology name for one worker image.
    pub fn worker_name(&self, worker_id: WorkerId) -> RuntimeResult<&str> {
        let (name, _) = self.topology.worker_subject(worker_id).ok_or_else(|| {
            RuntimeError::WorkerNotFound {
                worker_id: worker_id.0,
            }
            .boxed()
        })?;

        Ok(name)
    }

    /// Return whether one captured runtime owns one captured worker.
    pub fn runtime_owns_worker(&self, runtime_id: RuntimeId, worker_id: WorkerId) -> bool {
        self.topology.runtime_owns_worker(runtime_id, worker_id)
    }

    /// Return the owning runtime for one captured worker.
    pub fn worker_runtime_id(&self, worker_id: WorkerId) -> RuntimeResult<RuntimeId> {
        self.runtimes
            .keys()
            .copied()
            .find(|runtime_id| self.runtime_owns_worker(*runtime_id, worker_id))
            .ok_or_else(|| {
                RuntimeError::WorkerNotFound {
                    worker_id: worker_id.0,
                }
                .boxed()
            })
    }

    /// Return one logical world resource by id.
    pub fn resource(&self, resource_id: ResourceId) -> Option<&Resource> {
        self.resources.get(&resource_id)
    }

    /// Return one topology entity by id.
    pub fn entity(&self, entity_id: &str) -> Option<&Entity> {
        self.topology.entities().get(entity_id)
    }

    /// Return one topology edge by id.
    pub fn edge(&self, edge_id: &str) -> Option<&Edge> {
        self.topology.edges().get(edge_id)
    }
}

/// Serialized snapshot for one world image.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldSnapshot {
    /// The snapshot format version.
    pub format_version: u32,
    /// Runtime construction metadata for the restored world.
    pub config: SnapshotConfig,
    /// The revision selected for restore from this snapshot.
    pub revision: Revision,
    /// The captured lineage metadata.
    pub lineage: LineageSnapshot,
}

impl WorldSnapshot {
    /// Create one serialized snapshot for one materialized revision.
    pub const fn new(config: SnapshotConfig, revision: Revision, lineage: LineageSnapshot) -> Self {
        Self {
            format_version: 1,
            config,
            revision,
            lineage,
        }
    }

    /// Return the captured revision metadata.
    pub fn revision(&self) -> RuntimeResult<&RevisionState> {
        self.lineage.revisions.get(&self.revision).ok_or_else(|| {
            RuntimeError::RevisionNotFound {
                revision_id: self.revision.get(),
            }
            .boxed()
        })
    }

    /// Return the captured image metadata.
    pub fn image(&self) -> RuntimeResult<&WorldImage> {
        let revision = self.revision()?;

        self.lineage.images.get(&revision.image_id).ok_or_else(|| {
            RuntimeError::RevisionImageMissing {
                revision_id: self.revision.get(),
                image_id: revision.image_id.get(),
            }
            .boxed()
        })
    }

    /// Encode one snapshot into bytes.
    pub fn encode(&self) -> RuntimeResult<Vec<u8>> {
        to_allocvec(self).map_err(|_| {
            RuntimeError::InconsistentImage {
                detail: "failed to encode world snapshot".to_string(),
            }
            .boxed()
        })
    }

    /// Decode one snapshot from bytes.
    pub fn decode(bytes: &[u8]) -> RuntimeResult<Self> {
        postcard::from_bytes(bytes).map_err(|_| {
            RuntimeError::InconsistentImage {
                detail: "failed to decode world snapshot".to_string(),
            }
            .boxed()
        })
    }
}

impl World {
    /// Build one snapshot world configuration from the live world.
    fn snapshot_config(&self) -> SnapshotConfig {
        let header = self.state.trace.log().header();
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
            execution: self.state.trace.mode(),
            scheduler_mode: self.lineage.read().collector().mode().to_scheduler_mode(),
            time_mode: self.state.time_mode,
            random_mode: self.state.random_mode,
            replay_payload,
            replay_chunk_size_mb,
        }
    }

    /// Build runtime options for one serialized world snapshot.
    fn runtime_options_from_snapshot(snapshot: &WorldSnapshot) -> RuntimeOptions {
        let mut options = RuntimeOptions::default();

        options.set_execution_mode(snapshot.config.execution);
        options.scheduler.mode = snapshot.config.scheduler_mode;
        options.trace.chunk_size_mb = snapshot.config.replay_chunk_size_mb;
        options.trace.payload = snapshot.config.replay_payload;

        options
    }

    /// Return metadata for one stored image.
    pub fn image_info(&self, image_id: ImageId) -> RuntimeResult<WorldImage> {
        let image = self.lineage.read().image(image_id)?;

        Ok(image.as_ref().clone())
    }

    /// Return identifiers for all stored images in stable order.
    pub fn image_ids(&self) -> Vec<ImageId> {
        self.lineage.read().image_ids()
    }

    /// Return the revision that owns one stored image.
    pub fn revision_for_image(&self, image_id: ImageId) -> RuntimeResult<Revision> {
        let lineage = self.lineage.read();
        let revision = lineage.revision_for_image_id(image_id).ok_or_else(|| {
            RuntimeError::InconsistentImage {
                detail: format!("image {} does not belong to one revision", image_id.get()),
            }
            .boxed()
        })?;

        Ok(revision)
    }

    /// Create one lineage-wide serialized snapshot from one stored image.
    pub fn snapshot_lineage(&self, image_id: ImageId) -> RuntimeResult<WorldSnapshot> {
        let (revision, lineage_snapshot) = {
            let lineage = self.lineage.read();
            lineage.image(image_id)?;
            let revision = lineage.revision_for_image_id(image_id).ok_or_else(|| {
                RuntimeError::InconsistentImage {
                    detail: format!("image {} does not belong to one revision", image_id.get()),
                }
                .boxed()
            })?;

            (revision, lineage.full_snapshot()?)
        };

        Ok(WorldSnapshot::new(
            self.snapshot_config(),
            revision,
            lineage_snapshot,
        ))
    }

    /// Restore one stored image into the active world.
    pub fn restore_image_id(
        &mut self,
        image_id: ImageId,
        rebind_context: Option<&ResourceRebinders>,
    ) -> RuntimeResult<()> {
        // resolve the owning revision first
        let (revision, image, trace_image) = {
            let lineage = self.lineage.read();
            let revision = lineage.revision_for_image_id(image_id).ok_or_else(|| {
                RuntimeError::ImageNotFound {
                    image_id: image_id.get(),
                }
                .boxed()
            })?;
            let (_, image, trace_image) = self.revision_data(revision)?;

            (revision, image, trace_image)
        };

        self.restore_revision_image(revision, &image, &trace_image, rebind_context)
    }

    /// Create one exact serialized snapshot for one stored image.
    pub fn snapshot(&self, image_id: ImageId) -> RuntimeResult<WorldSnapshot> {
        let revision = {
            let lineage = self.lineage.read();
            lineage.image(image_id)?;
            lineage.revision_for_image_id(image_id).ok_or_else(|| {
                RuntimeError::InconsistentImage {
                    detail: format!("image {} does not belong to one revision", image_id.get()),
                }
                .boxed()
            })?
        };

        self.snapshot_revision(revision)
    }

    /// Create one lineage-wide serialized snapshot from one specific revision.
    pub fn snapshot_lineage_revision(&self, revision: Revision) -> RuntimeResult<WorldSnapshot> {
        let revision_info = {
            let lineage = self.lineage.read();
            lineage.revision_state(revision)?
        };

        if self.lineage.read().contains_image(revision_info.image_id) {
            return self.snapshot_lineage(revision_info.image_id);
        }

        let (target_revision, base_revision, image, trace_image) = {
            let lineage = self.lineage.read();
            let target_revision = lineage.revision_state(revision)?;
            let base_revision = self.nearest_image_revision(revision)?;
            let (base_revision, image, _) = self.revision_data(base_revision)?;
            let trace_image = self.trace_image(revision)?;

            (target_revision, base_revision, image, trace_image)
        };
        let image =
            self.revision_image(&target_revision, &base_revision, &image, &trace_image, None)?;
        let mut lineage_snapshot = self.lineage.read().full_snapshot()?;
        lineage_snapshot
            .images
            .insert(revision_info.image_id, image);

        Ok(WorldSnapshot::new(
            self.snapshot_config(),
            revision,
            lineage_snapshot,
        ))
    }

    /// Create one exact serialized snapshot from one specific revision.
    pub fn snapshot_revision(&self, revision: Revision) -> RuntimeResult<WorldSnapshot> {
        let (image, trace_image) = {
            let lineage = self.lineage.read();
            let revision_state = lineage.revision_state(revision)?;

            if lineage.contains_image(revision_state.image_id) {
                let image = lineage.image(revision_state.image_id)?;
                let trace_image = lineage.trace_image(revision_state.trace_image_id).map_err(
                    |error| match *error {
                        RuntimeError::ImageNotFound { .. } => {
                            RuntimeError::RevisionTraceImageMissing {
                                revision_id: revision.get(),
                            }
                            .boxed()
                        }
                        _ => error,
                    },
                )?;

                (image.as_ref().clone(), trace_image.as_ref().clone())
            } else {
                drop(lineage);

                let (target_revision, base_revision, image, trace_image) = {
                    let lineage = self.lineage.read();
                    let target_revision = lineage.revision_state(revision)?;
                    let base_revision = self.nearest_image_revision(revision)?;
                    let (base_revision, image, _) = self.revision_data(base_revision)?;
                    let trace_image = self.trace_image(revision)?;

                    (target_revision, base_revision, image, trace_image)
                };
                let image = self.revision_image(
                    &target_revision,
                    &base_revision,
                    &image,
                    &trace_image,
                    None,
                )?;

                (image, trace_image.as_ref().clone())
            }
        };
        let lineage_snapshot =
            self.lineage
                .read()
                .exact_snapshot(revision, &image, &trace_image)?;

        Ok(WorldSnapshot::new(
            self.snapshot_config(),
            revision,
            lineage_snapshot,
        ))
    }

    /// Create one lineage-wide serialized snapshot from one stored checkpoint.
    pub fn snapshot_lineage_checkpoint(
        &self,
        checkpoint_id: CheckpointId,
    ) -> RuntimeResult<WorldSnapshot> {
        let revision = {
            let lineage = self.lineage.read();
            let checkpoint = lineage.checkpoints.get(&checkpoint_id).ok_or_else(|| {
                RuntimeError::CheckpointNotFound {
                    checkpoint_id: checkpoint_id.get(),
                }
                .boxed()
            })?;

            checkpoint.revision
        };

        self.snapshot_lineage_revision(revision)
    }

    /// Create one exact serialized snapshot from one stored checkpoint.
    pub fn snapshot_checkpoint(&self, checkpoint_id: CheckpointId) -> RuntimeResult<WorldSnapshot> {
        let revision = {
            let lineage = self.lineage.read();
            let checkpoint = lineage.checkpoints.get(&checkpoint_id).ok_or_else(|| {
                RuntimeError::CheckpointNotFound {
                    checkpoint_id: checkpoint_id.get(),
                }
                .boxed()
            })?;

            checkpoint.revision
        };

        self.snapshot_revision(revision)
    }

    /// Build one fresh world from one serialized snapshot.
    pub fn from_snapshot(
        snapshot: &WorldSnapshot,
        rebind_context: Option<&ResourceRebinders>,
    ) -> RuntimeResult<Self> {
        let options = Self::runtime_options_from_snapshot(snapshot);
        let mut world = Self::new_at_branch(snapshot.revision()?.branch_id, &options, None)?;
        world.restore_snapshot(snapshot, rebind_context)?;

        Ok(world)
    }

    /// Build one fresh world from one encoded snapshot payload.
    pub fn from_snapshot_bytes(
        bytes: &[u8],
        rebind_context: Option<&ResourceRebinders>,
    ) -> RuntimeResult<Self> {
        let snapshot = WorldSnapshot::decode(bytes)?;

        Self::from_snapshot(&snapshot, rebind_context)
    }

    /// Restore one serialized snapshot into this world.
    pub fn restore_snapshot(
        &mut self,
        snapshot: &WorldSnapshot,
        rebind_context: Option<&ResourceRebinders>,
    ) -> RuntimeResult<()> {
        let revision = snapshot.revision()?;

        if revision.branch_id != self.state.branch_id {
            return Err(RuntimeError::SnapshotBranchMismatch {
                snapshot_branch_id: revision.branch_id.get(),
                world_branch_id: self.state.branch_id.get(),
            }
            .boxed());
        }

        let collector = self.lineage.read().collector();
        *self.lineage.write() = Lineage::from_snapshot(snapshot.lineage.clone(), collector)?;
        let (image, trace_image) = {
            let (_, image, trace_image) = self.revision_data(snapshot.revision)?;

            (image, trace_image)
        };

        self.restore_revision_image(snapshot.revision, &image, &trace_image, rebind_context)
    }

    /// Capture one materialized world image while the world is under exclusive access.
    pub(crate) fn capture_image(&mut self, mode: CaptureMode) -> RuntimeResult<WorldImage> {
        self.quiesce_shared_gc();

        // runtime and worker state
        let image = (|| {
            let mut runtime_images = BTreeMap::new();
            let mut worker_images = BTreeMap::new();

            for runtime in self.runtimes.values_mut() {
                let (runtime_image, runtime_workers) = runtime.capture_image(mode)?;
                runtime_images.insert(runtime.runtime_id(), runtime_image);
                worker_images.extend(runtime_workers);
            }

            Ok(WorldImage {
                next_runtime_id: self.state.next_runtime_id,
                next_worker_id: self.state.next_worker_id,
                policy: self.state.policy.clone(),
                topology: self.state.topology.clone(),
                resources: self.state.resources.clone(),
                simulation: self.state.simulation.clone(),
                clock: self.state.clock.snapshot(),
                random: self.state.random.snapshot(),
                runtimes: runtime_images,
                workers: worker_images,
            })
        })();

        self.resume_shared_gc();

        image
    }

    /// Restore one materialized image into this world while the world is quiesced.
    pub(crate) fn restore_image(
        &mut self,
        image: &WorldImage,
        rebind_context: Option<&ResourceRebinders>,
    ) -> RuntimeResult<()> {
        self.quiesce_shared_gc();
        let result = (|| {
            // world-owned state
            self.state.next_runtime_id = image.next_runtime_id;
            self.state.next_worker_id = image.next_worker_id;
            self.state.policy = image.policy.clone();
            self.state.topology = image.topology.clone();
            self.state.resources = image.resources.clone();
            self.state.simulation = image.simulation.clone();

            self.state.clock.restore_snapshot(&image.clock);
            self.state.random.restore_snapshot(&image.random)?;
            self.state.observations.reset();

            // runtime and worker state
            let mut restored_runtimes = BTreeMap::new();
            for (runtime_id, runtime_image) in &image.runtimes {
                let runtime_name = self
                    .state
                    .topology
                    .runtime_subject(*runtime_id)
                    .ok_or_else(|| {
                        RuntimeError::RuntimeNotFound {
                            runtime_id: runtime_id.0,
                        }
                        .boxed()
                    })?
                    .0
                    .to_string();
                let runtime_worker_images = image
                    .workers
                    .iter()
                    .filter(|(worker_id, _)| image.runtime_owns_worker(*runtime_id, **worker_id))
                    .map(|(worker_id, worker_image)| (*worker_id, worker_image.clone()))
                    .collect::<BTreeMap<_, _>>();
                let runtime_worker_names = runtime_worker_images
                    .keys()
                    .map(|worker_id| {
                        let worker_name = image.worker_name(*worker_id)?;

                        Ok((*worker_id, worker_name.to_string()))
                    })
                    .collect::<RuntimeResult<BTreeMap<_, _>>>()?;
                let (allocator, collector) = {
                    let lineage = self.lineage.read();
                    (lineage.allocator(), lineage.collector())
                };

                let runtime = Runtime::from_image(
                    &mut self.state,
                    allocator,
                    collector,
                    *runtime_id,
                    runtime_name,
                    runtime_image.as_ref(),
                    &runtime_worker_names,
                    &runtime_worker_images,
                    rebind_context,
                )?;
                restored_runtimes.insert(*runtime_id, Box::new(runtime));
            }

            self.runtimes = restored_runtimes;

            Ok(())
        })();

        self.resume_shared_gc();

        result
    }

    /// Return the encoded size and hash for one image.
    pub(crate) fn image_size_and_hash(image: &WorldImage) -> RuntimeResult<(u64, u128)> {
        let bytes = to_allocvec(image).map_err(|_| {
            RuntimeError::InconsistentImage {
                detail: "failed to encode world image".to_string(),
            }
            .boxed()
        })?;
        let size_bytes = bytes.len() as u64;
        let hash = fnv1a_128(&bytes);

        Ok((size_bytes, hash))
    }
}
