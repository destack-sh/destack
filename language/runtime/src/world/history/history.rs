use std::collections::BTreeMap;
use std::sync::Arc;

use destack_heap as heap;
use serde::{Deserialize, Serialize};

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::runtime::time::Instant;
use crate::runtime::{RuntimeImage, SharedCollector, WorkerId, WorkerImage};
use crate::world::trace::{ObservationRecord, TraceImage, TraceSequence};
use crate::world::{RuntimeId, WorldImage};

use super::{
    Branch, BranchId, BranchOrigin, Checkpoint, CheckpointId, ImageId, Moment, Revision, RevisionId,
};

/// First active branch identifier for one new world.
pub(crate) const ROOT_BRANCH: BranchId = BranchId::new(0);
/// First active revision for one new world.
pub(crate) const ROOT_REVISION: RevisionId = RevisionId::new(0);
/// Root image identifier for one new world.
pub(crate) const ROOT_IMAGE_ID: ImageId = ImageId::new(0);
/// First allocated branch identifier after the root branch.
const INITIAL_BRANCH_ID: u128 = 1;
/// First allocated revision identifier after the root revision.
const INITIAL_REVISION_ID: u128 = 1;
/// First allocated checkpoint identifier.
const INITIAL_CHECKPOINT_ID: u128 = 1;
/// First allocated image identifier after the root image.
const INITIAL_IMAGE_ID: u128 = 1;
/// First allocated runtime image identifier.
const INITIAL_RUNTIME_IMAGE_ID: u128 = 0;
/// First allocated worker image identifier.
const INITIAL_WORKER_IMAGE_ID: u128 = 0;
/// History-root metadata and durable restore metadata.
#[derive(Debug)]
pub(crate) struct History {
    /// The shared allocator backing retained and live world images in this history.
    allocator: Arc<heap::Allocator>,
    /// The collector scheduler shared by live worlds in this history.
    collector: Arc<SharedCollector>,
    /// The next branch identifier to allocate.
    pub next_branch_id: u128,
    /// The next revision identifier to allocate.
    pub next_revision_id: u128,
    /// The next checkpoint identifier to allocate.
    pub next_checkpoint_id: u128,
    /// The next image identifier to allocate.
    pub next_image_id: u128,
    /// The next runtime image identifier to allocate.
    pub next_runtime_image_id: u128,
    /// The next worker image identifier to allocate.
    pub next_worker_image_id: u128,
    /// The known branch metadata records.
    pub branches: BTreeMap<BranchId, Branch>,
    /// The known revision metadata records.
    pub revisions: BTreeMap<RevisionId, Revision>,
    /// The known checkpoint metadata records.
    pub checkpoints: BTreeMap<CheckpointId, Checkpoint>,
    /// The known image payloads keyed by image identifier.
    pub images: BTreeMap<ImageId, Arc<WorldImage>>,
    /// The known trace image payloads keyed by trace image identifier.
    pub trace_images: BTreeMap<RevisionId, Arc<TraceImage>>,
    /// The canonical retained runtime image payloads.
    runtime_images: BTreeMap<RuntimeImageId, Arc<RuntimeImage>>,
    /// The canonical retained worker image payloads.
    worker_images: BTreeMap<WorkerImageId, Arc<WorkerImage>>,
    /// The committed observation history keyed by branch.
    pub observations: BTreeMap<BranchId, Vec<ObservationRecord>>,
}

/// Durable history metadata captured in one world snapshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistorySnapshot {
    /// The next branch identifier to allocate.
    pub next_branch_id: u128,
    /// The next revision identifier to allocate.
    pub next_revision_id: u128,
    /// The next checkpoint identifier to allocate.
    pub next_checkpoint_id: u128,
    /// The serialized shared-heap allocator pages reachable from this history.
    pub allocator: heap::AllocatorImage,
    /// The next image identifier to allocate.
    pub next_image_id: u128,
    /// The next runtime image identifier to allocate.
    pub next_runtime_image_id: u128,
    /// The next worker image identifier to allocate.
    pub next_worker_image_id: u128,
    /// The known branch metadata records.
    pub branches: BTreeMap<BranchId, Branch>,
    /// The known revision metadata records.
    pub revisions: BTreeMap<RevisionId, Revision>,
    /// The known checkpoint metadata records.
    pub checkpoints: BTreeMap<CheckpointId, Checkpoint>,
    /// The known image payloads keyed by image identifier.
    pub images: BTreeMap<ImageId, WorldImage>,
    /// The known trace image payloads keyed by trace image identifier.
    pub trace_images: BTreeMap<RevisionId, TraceImage>,
    /// The committed observation history keyed by branch.
    pub observations: BTreeMap<BranchId, Vec<ObservationRecord>>,
}

/// History-local identifier for one canonical retained runtime image.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct RuntimeImageId(u128);

impl RuntimeImageId {
    /// Create a new runtime image identifier.
    const fn new(value: u128) -> Self {
        Self(value)
    }
}

/// History-local identifier for one canonical retained worker image.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct WorkerImageId(u128);

impl WorkerImageId {
    /// Create a new worker image identifier.
    const fn new(value: u128) -> Self {
        Self(value)
    }
}

impl History {
    /// Capture one durable history snapshot for all retained history.
    pub(crate) fn full_snapshot(&self) -> RuntimeResult<HistorySnapshot> {
        let reachable_pages = self.reachable_image_pages(self.images.values().map(Arc::as_ref));

        Ok(HistorySnapshot {
            next_branch_id: self.next_branch_id,
            next_revision_id: self.next_revision_id,
            next_checkpoint_id: self.next_checkpoint_id,
            allocator: self.allocator.image_pages_from_ids(&reachable_pages)?,
            next_image_id: self.next_image_id,
            next_runtime_image_id: self.next_runtime_image_id,
            next_worker_image_id: self.next_worker_image_id,
            branches: self.branches.clone(),
            revisions: self.revisions.clone(),
            checkpoints: self.checkpoints.clone(),
            images: self
                .images
                .iter()
                .map(|(image_id, image)| (*image_id, image.as_ref().clone()))
                .collect(),
            trace_images: self
                .trace_images
                .iter()
                .map(|(revision_id, trace_image)| (*revision_id, trace_image.as_ref().clone()))
                .collect(),
            observations: self.observations.clone(),
        })
    }

    /// Capture one durable history snapshot for one exact revision closure.
    pub(crate) fn exact_snapshot(
        &self,
        revision_id: RevisionId,
        image: &WorldImage,
        trace_image: &TraceImage,
    ) -> RuntimeResult<HistorySnapshot> {
        let revision = self.revision(revision_id)?;
        let branch = self.branch(revision.branch_id)?;
        let reachable_pages = self.reachable_image_pages([image]);

        // exact snapshot history
        let branch = Branch {
            origin: BranchOrigin::Root,
            head_revision_id: revision_id,
            ..branch
        };
        let image_id = revision.image_id;
        let revision = Revision {
            parent_revision_id: None,
            ..revision
        };
        let checkpoints = self
            .checkpoints
            .iter()
            .filter(|(_, checkpoint)| checkpoint.revision_id == revision_id)
            .map(|(checkpoint_id, checkpoint)| (*checkpoint_id, checkpoint.clone()))
            .collect();
        let observations = self
            .observations
            .get(&revision.branch_id)
            .map(|records| {
                records
                    .iter()
                    .filter(|record| record.moment.sequence <= revision.sequence)
                    .cloned()
                    .collect::<Vec<_>>()
            })
            .filter(|records| !records.is_empty())
            .map(|records| BTreeMap::from([(revision.branch_id, records)]))
            .unwrap_or_default();

        Ok(HistorySnapshot {
            next_branch_id: self.next_branch_id,
            next_revision_id: self.next_revision_id,
            next_checkpoint_id: self.next_checkpoint_id,
            allocator: self.allocator.image_pages_from_ids(&reachable_pages)?,
            next_image_id: self.next_image_id,
            next_runtime_image_id: self.next_runtime_image_id,
            next_worker_image_id: self.next_worker_image_id,
            branches: BTreeMap::from([(branch.id, branch)]),
            revisions: BTreeMap::from([(revision_id, revision)]),
            checkpoints,
            images: BTreeMap::from([(image_id, image.clone())]),
            trace_images: BTreeMap::from([(revision_id, trace_image.clone())]),
            observations,
        })
    }

    /// Create one history with one fully materialized root revision.
    pub(crate) fn new_root(
        allocator: Arc<heap::Allocator>,
        collector: Arc<SharedCollector>,
        wall: Instant,
        mono: Instant,
        sequence: TraceSequence,
        image: Arc<WorldImage>,
        trace_image: Arc<TraceImage>,
    ) -> Self {
        let mut branches = BTreeMap::new();
        let mut revisions = BTreeMap::new();
        let mut images = BTreeMap::new();
        let mut trace_images = BTreeMap::new();
        let observations = BTreeMap::new();

        images.insert(ROOT_IMAGE_ID, image);
        trace_images.insert(ROOT_REVISION, trace_image);

        revisions.insert(
            ROOT_REVISION,
            Revision {
                branch_id: ROOT_BRANCH,
                parent_revision_id: None,
                sequence,
                image_id: ROOT_IMAGE_ID,
                wall,
                mono,
                labels: BTreeMap::new(),
            },
        );

        branches.insert(
            ROOT_BRANCH,
            Branch {
                id: ROOT_BRANCH,
                head_revision_id: ROOT_REVISION,
                origin: BranchOrigin::Root,
                name: "root".to_string(),
                labels: BTreeMap::new(),
            },
        );

        Self {
            allocator,
            collector,
            next_branch_id: INITIAL_BRANCH_ID,
            next_revision_id: INITIAL_REVISION_ID,
            next_checkpoint_id: INITIAL_CHECKPOINT_ID,
            next_image_id: INITIAL_IMAGE_ID,
            next_runtime_image_id: INITIAL_RUNTIME_IMAGE_ID,
            next_worker_image_id: INITIAL_WORKER_IMAGE_ID,
            branches,
            revisions,
            checkpoints: BTreeMap::new(),
            images,
            trace_images,
            runtime_images: BTreeMap::new(),
            worker_images: BTreeMap::new(),
            observations,
        }
    }

    /// Rebuild history state from one durable history snapshot.
    pub(crate) fn from_snapshot(
        snapshot: HistorySnapshot,
        collector: Arc<SharedCollector>,
    ) -> RuntimeResult<Self> {
        let allocator = Arc::new(heap::Allocator::from_image(&snapshot.allocator)?);
        let images = snapshot
            .images
            .into_iter()
            .map(|(image_id, image)| Ok((image_id, Arc::new(image))))
            .collect::<RuntimeResult<_>>()?;
        let trace_images = snapshot
            .trace_images
            .into_iter()
            .map(|(revision_id, trace_image)| (revision_id, Arc::new(trace_image)))
            .collect();

        let mut history = Self {
            allocator,
            collector,
            next_branch_id: snapshot.next_branch_id,
            next_revision_id: snapshot.next_revision_id,
            next_checkpoint_id: snapshot.next_checkpoint_id,
            next_image_id: snapshot.next_image_id,
            next_runtime_image_id: snapshot.next_runtime_image_id,
            next_worker_image_id: snapshot.next_worker_image_id,
            branches: snapshot.branches,
            revisions: snapshot.revisions,
            checkpoints: snapshot.checkpoints,
            images,
            trace_images,
            observations: snapshot.observations,
            runtime_images: BTreeMap::new(),
            worker_images: BTreeMap::new(),
        };
        history.rebuild_image_tables();

        Ok(history)
    }

    /// Return the shared collector for this history.
    pub(crate) fn collector(&self) -> Arc<SharedCollector> {
        self.collector.clone()
    }

    /// Return the shared allocator for this history.
    pub(crate) fn allocator(&self) -> Arc<heap::Allocator> {
        self.allocator.clone()
    }

    /// Collect the allocator pages reachable from one set of retained world images.
    fn reachable_image_pages<'a>(
        &self,
        images: impl IntoIterator<Item = &'a WorldImage>,
    ) -> Vec<heap::PageId> {
        let mut reachable_pages = Vec::new();

        for image in images {
            for runtime in image.runtimes.values() {
                reachable_pages.extend(runtime.shared_heap.page_ids());
            }
        }

        reachable_pages.sort_unstable();
        reachable_pages.dedup();

        reachable_pages
    }

    /// Allocate one new branch identifier.
    pub(crate) fn allocate_branch_id(&mut self) -> BranchId {
        let branch_id = BranchId::new(self.next_branch_id);
        self.next_branch_id += 1;
        branch_id
    }

    /// Allocate one new revision.
    pub(crate) fn allocate_revision(&mut self) -> RevisionId {
        let revision = RevisionId::new(self.next_revision_id);
        self.next_revision_id += 1;
        revision
    }

    /// Allocate one new checkpoint identifier.
    pub(crate) fn allocate_checkpoint_id(&mut self) -> CheckpointId {
        let checkpoint_id = CheckpointId::new(self.next_checkpoint_id);
        self.next_checkpoint_id += 1;
        checkpoint_id
    }

    /// Allocate one new image identifier.
    pub(crate) fn allocate_image_id(&mut self) -> ImageId {
        let image_id = ImageId::new(self.next_image_id);
        self.next_image_id += 1;
        image_id
    }

    /// Set the current head revision for one branch.
    pub(crate) fn set_branch_head(&mut self, branch_id: BranchId, revision_id: RevisionId) {
        self.revisions
            .get(&revision_id)
            .expect("world history must contain the requested revision");
        let branch = self
            .branches
            .get_mut(&branch_id)
            .expect("world history must contain the requested branch");
        branch.head_revision_id = revision_id;
    }

    /// Create one child branch from one parent revision.
    pub(crate) fn fork_branch(
        &mut self,
        parent_revision_id: RevisionId,
        name: String,
    ) -> RuntimeResult<Branch> {
        // resolve the parent revision before mutating history state
        self.revisions.get(&parent_revision_id).ok_or_else(|| {
            RuntimeError::RevisionNotFound {
                revision_id: parent_revision_id.get(),
            }
            .boxed()
        })?;

        let branch = Branch {
            id: self.allocate_branch_id(),
            head_revision_id: parent_revision_id,
            origin: BranchOrigin::Fork { parent_revision_id },
            name,
            labels: BTreeMap::new(),
        };
        self.branches.insert(branch.id, branch.clone());

        Ok(branch)
    }

    /// Commit one new revision and update the branch head.
    pub(crate) fn commit_revision(
        &mut self,
        branch_id: BranchId,
        image_id: ImageId,
        sequence: TraceSequence,
        wall: Instant,
        mono: Instant,
        checkpoint_name: Option<String>,
    ) -> RuntimeResult<(RevisionId, Revision, Option<Checkpoint>)> {
        let parent_branch = self.branches.get(&branch_id).cloned().ok_or_else(|| {
            RuntimeError::BranchNotFound {
                branch_id: branch_id.get(),
            }
            .boxed()
        })?;

        let revision_id = self.allocate_revision();
        let revision = Revision {
            branch_id,
            parent_revision_id: Some(parent_branch.head_revision_id),
            sequence,
            image_id,
            wall,
            mono,
            labels: BTreeMap::new(),
        };

        let branch = Branch {
            head_revision_id: revision_id,
            ..parent_branch
        };

        let checkpoint = checkpoint_name.map(|name| Checkpoint {
            id: self.allocate_checkpoint_id(),
            revision_id,
            name,
            labels: BTreeMap::new(),
        });

        self.revisions.insert(revision_id, revision.clone());
        self.branches.insert(branch_id, branch);

        if let Some(checkpoint) = checkpoint.clone() {
            self.checkpoints.insert(checkpoint.id, checkpoint);
        }

        Ok((revision_id, revision, checkpoint))
    }

    /// Return the revision that owns one image identifier.
    pub(crate) fn revision_for_image_id(&self, image_id: ImageId) -> Option<RevisionId> {
        self.revisions.iter().find_map(|(revision_id, revision)| {
            (revision.image_id == image_id).then_some(*revision_id)
        })
    }

    /// Return the nearest materialized revision at or before one target revision.
    pub(crate) fn nearest_image_revision(
        &self,
        revision_id: RevisionId,
        has_image: impl Fn(ImageId) -> bool,
    ) -> RuntimeResult<RevisionId> {
        let mut current_revision = revision_id;

        loop {
            let revision = self.revisions.get(&current_revision).ok_or_else(|| {
                RuntimeError::RevisionNotFound {
                    revision_id: current_revision.get(),
                }
                .boxed()
            })?;

            if has_image(revision.image_id) {
                return Ok(current_revision);
            }

            let Some(parent_revision_id) = revision.parent_revision_id else {
                return Err(RuntimeError::RevisionImageMissing {
                    revision_id: current_revision.get(),
                    image_id: revision.image_id.get(),
                }
                .boxed());
            };

            current_revision = parent_revision_id;
        }
    }

    /// Return one committed revision by identifier.
    pub(crate) fn revision(&self, revision_id: RevisionId) -> RuntimeResult<Revision> {
        self.revisions.get(&revision_id).cloned().ok_or_else(|| {
            RuntimeError::RevisionNotFound {
                revision_id: revision_id.get(),
            }
            .boxed()
        })
    }

    /// Append committed observations for one branch in stable moment order.
    pub(crate) fn record_observations(
        &mut self,
        branch_id: BranchId,
        observations: Vec<ObservationRecord>,
    ) {
        if observations.is_empty() {
            return;
        }

        // committed observations are appended in stable branch-local order
        let branch_observations = self.observations.entry(branch_id).or_default();
        branch_observations.extend(observations);
    }

    /// Return committed observations within one exact branch-local moment range.
    pub(crate) fn observation_records_between(
        &self,
        start: Moment,
        end: Moment,
    ) -> RuntimeResult<Vec<ObservationRecord>> {
        if start.branch_id != end.branch_id {
            return Err(RuntimeError::MomentBranchMismatch {
                moment_branch_id: start.branch_id.get(),
                world_branch_id: end.branch_id.get(),
            }
            .boxed());
        }

        if start.sequence.get() > end.sequence.get() {
            return Err(RuntimeError::Internal {
                message: "moment query start sequence must be at or before the end sequence"
                    .to_string(),
            }
            .boxed());
        }

        let head_revision = self.head_revision(end.branch_id)?;
        if end.sequence.get() > head_revision.sequence.get() {
            return Err(RuntimeError::MomentNotFound {
                branch_id: end.branch_id.get(),
                sequence: end.sequence.get(),
            }
            .boxed());
        }

        let Some(records) = self.observations.get(&end.branch_id) else {
            return Ok(Vec::new());
        };

        let records = records
            .iter()
            .filter(|record| {
                record.moment.sequence.get() > start.sequence.get()
                    && record.moment.sequence.get() <= end.sequence.get()
            })
            .cloned()
            .collect();

        Ok(records)
    }

    /// Return the head revision for one branch.
    pub(crate) fn head_revision(&self, branch_id: BranchId) -> RuntimeResult<Revision> {
        let branch = self.branches.get(&branch_id).ok_or_else(|| {
            RuntimeError::BranchNotFound {
                branch_id: branch_id.get(),
            }
            .boxed()
        })?;

        let revision = self
            .revisions
            .get(&branch.head_revision_id)
            .cloned()
            .ok_or_else(|| {
                RuntimeError::RevisionNotFound {
                    revision_id: branch.head_revision_id.get(),
                }
                .boxed()
            })?;

        Ok(revision)
    }

    /// Return the branch-origin moment for one branch.
    pub(crate) fn branch_origin_moment(&self, branch_id: BranchId) -> RuntimeResult<Moment> {
        let branch = self.branches.get(&branch_id).ok_or_else(|| {
            RuntimeError::BranchNotFound {
                branch_id: branch_id.get(),
            }
            .boxed()
        })?;

        let sequence = match branch.origin {
            BranchOrigin::Root => TraceSequence::new(0),
            BranchOrigin::Fork { parent_revision_id } => {
                let parent_revision_id =
                    self.revisions.get(&parent_revision_id).ok_or_else(|| {
                        RuntimeError::RevisionNotFound {
                            revision_id: parent_revision_id.get(),
                        }
                    })?;
                parent_revision_id.sequence
            }
        };

        Ok(Moment::new(branch_id, sequence))
    }

    /// Return the current committed head moment for one branch.
    pub(crate) fn branch_head_moment(&self, branch_id: BranchId) -> RuntimeResult<Moment> {
        let head_revision = self.head_revision(branch_id)?;

        Ok(Moment::new(branch_id, head_revision.sequence))
    }

    /// Return one branch metadata record.
    pub(crate) fn branch(&self, branch_id: BranchId) -> RuntimeResult<Branch> {
        self.branches.get(&branch_id).cloned().ok_or_else(|| {
            RuntimeError::BranchNotFound {
                branch_id: branch_id.get(),
            }
            .boxed()
        })
    }

    /// Return every ancestor branch id from root to the requested branch.
    pub(crate) fn branch_ancestor_ids(&self, branch_id: BranchId) -> RuntimeResult<Vec<BranchId>> {
        let mut branch = self.branch(branch_id)?;
        let mut ancestors = vec![branch.id];

        while let BranchOrigin::Fork { parent_revision_id } = branch.origin {
            let parent_revision_id = self.revisions.get(&parent_revision_id).ok_or_else(|| {
                RuntimeError::RevisionNotFound {
                    revision_id: parent_revision_id.get(),
                }
            })?;
            branch = self.branch(parent_revision_id.branch_id)?;
            ancestors.push(branch.id);
        }

        ancestors.reverse();
        Ok(ancestors)
    }

    /// Report whether one branch descends from one ancestor branch.
    pub(crate) fn is_descendant_branch(
        &self,
        branch_id: BranchId,
        ancestor_branch_id: BranchId,
    ) -> RuntimeResult<bool> {
        let ancestors = self.branch_ancestor_ids(branch_id)?;
        Ok(ancestors.contains(&ancestor_branch_id))
    }

    /// Return every branch that descends from the requested ancestor branch.
    pub(crate) fn descendant_branches(
        &self,
        ancestor_branch_id: BranchId,
    ) -> RuntimeResult<Vec<Branch>> {
        let mut descendants = Vec::new();

        for branch in self.branches.values() {
            if self.is_descendant_branch(branch.id, ancestor_branch_id)? {
                descendants.push(branch.clone());
            }
        }

        descendants.sort_by_key(|branch| branch.id);
        Ok(descendants)
    }

    /// Return the common ancestor revision for two branches.
    pub(crate) fn common_ancestor_revision(
        &self,
        left_branch_id: BranchId,
        right_branch_id: BranchId,
    ) -> RuntimeResult<Revision> {
        let mut left_chain = BTreeMap::new();
        let mut current_left = Some(self.branch(left_branch_id)?.head_revision_id);
        while let Some(revision_id) = current_left {
            let revision = self.revisions.get(&revision_id).ok_or_else(|| {
                RuntimeError::RevisionNotFound {
                    revision_id: revision_id.get(),
                }
                .boxed()
            })?;
            left_chain.insert(revision_id, revision.clone());
            current_left = revision.parent_revision_id;
        }

        let mut current_right = Some(self.branch(right_branch_id)?.head_revision_id);
        while let Some(revision_id) = current_right {
            let revision = self.revisions.get(&revision_id).ok_or_else(|| {
                RuntimeError::RevisionNotFound {
                    revision_id: revision_id.get(),
                }
                .boxed()
            })?;

            if let Some(common) = left_chain.get(&revision_id) {
                return Ok(common.clone());
            }

            current_right = revision.parent_revision_id;
        }

        Err(RuntimeError::Internal {
            message: "branches in one history must share a common ancestor revision".to_string(),
        }
        .boxed())
    }

    /// Return the latest committed revision at or before one target sequence on one branch.
    pub(crate) fn latest_revision_at_or_before(
        &self,
        branch_id: BranchId,
        sequence: TraceSequence,
    ) -> RuntimeResult<RevisionId> {
        let mut revision_id = self.branch(branch_id)?.head_revision_id;
        let revision = self.revisions.get(&revision_id).ok_or_else(|| {
            RuntimeError::RevisionNotFound {
                revision_id: revision_id.get(),
            }
            .boxed()
        })?;

        if revision.sequence.get() < sequence.get() {
            return Err(RuntimeError::MomentNotFound {
                branch_id: branch_id.get(),
                sequence: sequence.get(),
            }
            .boxed());
        }

        loop {
            let revision = self.revisions.get(&revision_id).ok_or_else(|| {
                RuntimeError::RevisionNotFound {
                    revision_id: revision_id.get(),
                }
                .boxed()
            })?;

            if revision.sequence.get() <= sequence.get() {
                return Ok(revision_id);
            }

            let Some(parent_revision_id) = revision.parent_revision_id else {
                return Ok(revision_id);
            };
            revision_id = parent_revision_id;
        }
    }

    /// Insert one retained image payload.
    pub(crate) fn insert_image(&mut self, image_id: ImageId, image: Arc<WorldImage>) {
        self.images.insert(image_id, image);
    }

    /// Insert one retained trace image payload.
    pub(crate) fn insert_trace_image(
        &mut self,
        revision_id: RevisionId,
        trace_image: Arc<TraceImage>,
    ) {
        self.trace_images.insert(revision_id, trace_image);
    }

    /// Return one retained image payload.
    pub(crate) fn image(&self, image_id: ImageId) -> RuntimeResult<Arc<WorldImage>> {
        self.images.get(&image_id).cloned().ok_or_else(|| {
            RuntimeError::ImageNotFound {
                image_id: image_id.get(),
            }
            .boxed()
        })
    }

    /// Return one retained trace image payload.
    pub(crate) fn trace_image(&self, revision_id: RevisionId) -> RuntimeResult<Arc<TraceImage>> {
        self.trace_images.get(&revision_id).cloned().ok_or_else(|| {
            RuntimeError::RevisionTraceImageMissing {
                revision_id: revision_id.get(),
            }
            .boxed()
        })
    }

    /// Return whether one retained image payload exists.
    pub(crate) fn contains_image(&self, image_id: ImageId) -> bool {
        self.images.contains_key(&image_id)
    }

    /// Return identifiers for all retained images in stable order.
    pub(crate) fn image_ids(&self) -> Vec<ImageId> {
        self.images.keys().copied().collect()
    }

    /// Canonicalize retained runtime and worker images inside one world image.
    pub(crate) fn retain_image_payloads(
        &mut self,
        branch_id: BranchId,
        image: &mut WorldImage,
    ) -> RuntimeResult<()> {
        let parent_image = self.parent_retained_image(branch_id)?;

        // runtime images
        for (runtime_id, runtime_image) in &mut image.runtimes {
            let canonical_runtime_image = self.retain_runtime_image(
                *runtime_id,
                runtime_image.clone(),
                parent_image.as_deref(),
            );
            *runtime_image = canonical_runtime_image;
        }

        // worker images
        for (worker_id, worker_image) in &mut image.workers {
            let canonical_worker_image =
                self.retain_worker_image(*worker_id, worker_image.clone(), parent_image.as_deref());
            *worker_image = canonical_worker_image;
        }

        Ok(())
    }

    /// Rebuild the canonical runtime and worker image tables from retained images.
    fn rebuild_image_tables(&mut self) {
        let image_ids = self.images.keys().copied().collect::<Vec<_>>();

        // canonicalize the retained images in stable image order
        for image_id in image_ids {
            let Some(image) = self.images.get(&image_id).cloned() else {
                continue;
            };

            let mut image = image.as_ref().clone();
            self.rebuild_runtime_image_entries(&mut image);
            self.rebuild_worker_image_entries(&mut image);
            self.images.insert(image_id, Arc::new(image));
        }
    }

    /// Return the nearest retained parent image for one branch head.
    fn parent_retained_image(&self, branch_id: BranchId) -> RuntimeResult<Option<Arc<WorldImage>>> {
        let Some(branch) = self.branches.get(&branch_id) else {
            return Err(RuntimeError::BranchNotFound {
                branch_id: branch_id.get(),
            }
            .boxed());
        };

        let retained_revision = self
            .nearest_image_revision(branch.head_revision_id, |image_id| {
                self.contains_image(image_id)
            })?;

        let Some(revision) = self.revisions.get(&retained_revision) else {
            return Err(RuntimeError::RevisionNotFound {
                revision_id: retained_revision.get(),
            }
            .boxed());
        };

        Ok(self.images.get(&revision.image_id).cloned())
    }

    /// Return one canonical runtime image for one retained runtime payload.
    fn retain_runtime_image(
        &mut self,
        runtime_id: RuntimeId,
        runtime_image: Arc<RuntimeImage>,
        parent_image: Option<&WorldImage>,
    ) -> Arc<RuntimeImage> {
        if let Some(parent_image) = parent_image
            && let Some(parent_runtime_image) = parent_image.runtimes.get(&runtime_id)
            && parent_runtime_image.as_ref() == runtime_image.as_ref()
        {
            return parent_runtime_image.clone();
        }

        let runtime_image_id = RuntimeImageId::new(self.next_runtime_image_id);
        self.next_runtime_image_id += 1;
        self.runtime_images
            .insert(runtime_image_id, runtime_image.clone());

        runtime_image
    }

    /// Return one canonical worker image for one retained worker payload.
    fn retain_worker_image(
        &mut self,
        worker_id: WorkerId,
        worker_image: Arc<WorkerImage>,
        parent_image: Option<&WorldImage>,
    ) -> Arc<WorkerImage> {
        if let Some(parent_image) = parent_image
            && let Some(parent_worker_image) = parent_image.workers.get(&worker_id)
            && parent_worker_image.as_ref() == worker_image.as_ref()
        {
            return parent_worker_image.clone();
        }

        let worker_image_id = WorkerImageId::new(self.next_worker_image_id);
        self.next_worker_image_id += 1;
        self.worker_images
            .insert(worker_image_id, worker_image.clone());

        worker_image
    }

    /// Canonicalize runtime images while rebuilding retained history state.
    fn rebuild_runtime_image_entries(&mut self, image: &mut WorldImage) {
        for runtime_image in image.runtimes.values_mut() {
            if let Some(existing_runtime_image) =
                self.runtime_images.values().find(|existing_runtime_image| {
                    existing_runtime_image.as_ref() == runtime_image.as_ref()
                })
            {
                *runtime_image = existing_runtime_image.clone();
                continue;
            }

            let runtime_image_id = RuntimeImageId::new(self.next_runtime_image_id);
            self.next_runtime_image_id += 1;
            self.runtime_images
                .insert(runtime_image_id, runtime_image.clone());
        }
    }

    /// Canonicalize worker images while rebuilding retained history state.
    fn rebuild_worker_image_entries(&mut self, image: &mut WorldImage) {
        for worker_image in image.workers.values_mut() {
            if let Some(existing_worker_image) =
                self.worker_images.values().find(|existing_worker_image| {
                    existing_worker_image.as_ref() == worker_image.as_ref()
                })
            {
                *worker_image = existing_worker_image.clone();
                continue;
            }

            let worker_image_id = WorkerImageId::new(self.next_worker_image_id);
            self.next_worker_image_id += 1;
            self.worker_images
                .insert(worker_image_id, worker_image.clone());
        }
    }
}
