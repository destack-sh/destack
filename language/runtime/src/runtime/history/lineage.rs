use std::collections::BTreeMap;
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::runtime::time::WorldInstant;
use crate::runtime::trace::{TraceImage, TraceSequence};
use crate::runtime::world::ObservationRecord;

use super::{
    Branch, BranchId, BranchOrigin, Checkpoint, CheckpointId, Image, ImageId, Moment, Revision,
    RevisionId,
};

/// First active branch identifier for one new world.
pub(crate) const ROOT_BRANCH_ID: BranchId = BranchId::new(0);
/// First active revision identifier for one new world.
pub(crate) const ROOT_REVISION_ID: RevisionId = RevisionId::new(0);
/// Root image identifier for one new world.
pub(crate) const ROOT_IMAGE_ID: ImageId = ImageId::new(0);
/// First allocated branch identifier after the root branch.
const INITIAL_BRANCH_ID: u128 = 1;
/// First allocated revision identifier after the root revision.
const INITIAL_REVISION_ID: u128 = 1;
/// First allocated checkpoint identifier.
const INITIAL_CHECKPOINT_ID: u128 = 1;
/// First allocated image identifier.
const INITIAL_IMAGE_ID: u128 = 1;

/// World-owned lineage metadata and durable restore metadata.
#[derive(Debug)]
pub(crate) struct Lineage {
    /// The next branch identifier to allocate.
    pub next_branch_id: u128,
    /// The next revision identifier to allocate.
    pub next_revision_id: u128,
    /// The next checkpoint identifier to allocate.
    pub next_checkpoint_id: u128,
    /// The next image identifier to allocate.
    pub next_image_id: u128,
    /// The known branch metadata records.
    pub branches: BTreeMap<BranchId, Branch>,
    /// The known revision metadata records.
    pub revisions: BTreeMap<RevisionId, Revision>,
    /// The known checkpoint metadata records.
    pub checkpoints: BTreeMap<CheckpointId, Checkpoint>,
    /// The known image metadata records.
    pub images: BTreeMap<ImageId, Arc<Image>>,
    /// The known trace image records keyed by revision identifier.
    pub trace_images: BTreeMap<RevisionId, Arc<TraceImage>>,
    /// The committed observation history keyed by branch.
    pub observations: BTreeMap<BranchId, Vec<ObservationRecord>>,
}

/// Fully resolved backing for one materialized revision.
#[derive(Debug, Clone)]
pub(crate) struct RevisionBacking {
    /// The resolved revision metadata.
    pub revision: Revision,
    /// The resolved world image.
    pub image: Arc<Image>,
    /// The resolved trace image.
    pub trace_image: Arc<TraceImage>,
}

/// One restore plan for reaching one specific revision.
#[derive(Debug, Clone)]
pub(crate) struct RevisionRestorePlan {
    /// The requested target revision.
    pub target_revision: Revision,
    /// The nearest materialized base revision.
    pub base_revision: Revision,
    /// The materialized image for the base revision.
    pub image: Arc<Image>,
    /// The materialized trace image for the target revision.
    pub trace_image: Arc<TraceImage>,
}

impl RevisionRestorePlan {
    /// Report whether this restore plan needs replay past its base image.
    pub(crate) fn requires_replay(&self) -> bool {
        self.base_revision.id != self.target_revision.id
    }
}

/// One restore plan for reaching one specific moment.
#[derive(Debug, Clone)]
pub(crate) struct MomentRestorePlan {
    /// The requested target moment.
    pub target_moment: Moment,
    /// The latest committed revision at or before the target moment.
    pub anchor_revision: Revision,
    /// The nearest materialized base revision.
    pub base_revision: Revision,
    /// The materialized image for the base revision.
    pub image: Arc<Image>,
}

impl MomentRestorePlan {
    /// Report whether this restore plan needs replay past its base image.
    pub(crate) fn requires_replay(&self) -> bool {
        self.base_revision.sequence != self.target_moment.sequence
    }
}

/// One committed lineage update for one new materialized revision.
#[derive(Debug, Clone)]
pub(crate) struct CommittedRevision {
    /// The committed world revision metadata.
    pub revision: Revision,
    /// The committed world image.
    pub image: Arc<Image>,
    /// The committed checkpoint metadata, when created.
    pub checkpoint: Option<Checkpoint>,
}

/// Durable lineage metadata captured in one world snapshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LineageSnapshot {
    /// The next branch identifier to allocate.
    pub next_branch_id: u128,
    /// The next revision identifier to allocate.
    pub next_revision_id: u128,
    /// The next checkpoint identifier to allocate.
    pub next_checkpoint_id: u128,
    /// The next image identifier to allocate.
    pub next_image_id: u128,
    /// The known branch metadata records.
    pub branches: BTreeMap<BranchId, Branch>,
    /// The known revision metadata records.
    pub revisions: BTreeMap<RevisionId, Revision>,
    /// The known checkpoint metadata records.
    pub checkpoints: BTreeMap<CheckpointId, Checkpoint>,
    /// The known image metadata records.
    pub images: BTreeMap<ImageId, Image>,
    /// The known trace image records keyed by revision identifier.
    pub trace_images: BTreeMap<RevisionId, TraceImage>,
    /// The committed observation history keyed by branch.
    pub observations: BTreeMap<BranchId, Vec<ObservationRecord>>,
}

impl Lineage {
    /// Create one lineage with one fully materialized root revision.
    pub(crate) fn new_root(image: Arc<Image>, trace_image: Arc<TraceImage>) -> Self {
        let mut branches = BTreeMap::new();
        let mut revisions = BTreeMap::new();
        let mut images = BTreeMap::new();
        let mut trace_images = BTreeMap::new();
        let observations = BTreeMap::new();

        let wall = image.clock.virtual_wall;
        let mono = image.clock.virtual_mono;
        let sequence = trace_image.next_sequence;

        revisions.insert(
            ROOT_REVISION_ID,
            Revision {
                id: ROOT_REVISION_ID,
                branch_id: ROOT_BRANCH_ID,
                parent_revision_id: None,
                sequence,
                image_id: ROOT_IMAGE_ID,
                wall,
                mono,
                labels: BTreeMap::new(),
            },
        );

        branches.insert(
            ROOT_BRANCH_ID,
            Branch {
                id: ROOT_BRANCH_ID,
                head_revision_id: ROOT_REVISION_ID,
                origin: BranchOrigin::Root,
                name: "root".to_string(),
                labels: BTreeMap::new(),
            },
        );

        images.insert(ROOT_IMAGE_ID, image);
        trace_images.insert(ROOT_REVISION_ID, trace_image);

        Self {
            next_branch_id: INITIAL_BRANCH_ID,
            next_revision_id: INITIAL_REVISION_ID,
            next_checkpoint_id: INITIAL_CHECKPOINT_ID,
            next_image_id: INITIAL_IMAGE_ID,
            branches,
            revisions,
            checkpoints: BTreeMap::new(),
            images,
            trace_images,
            observations,
        }
    }

    /// Capture one durable lineage snapshot.
    pub(crate) fn snapshot(&self) -> LineageSnapshot {
        let images = self
            .images
            .iter()
            .map(|(image_id, image)| (*image_id, image.as_ref().clone()))
            .collect();
        let trace_images = self
            .trace_images
            .iter()
            .map(|(revision_id, trace_image)| (*revision_id, trace_image.as_ref().clone()))
            .collect();

        LineageSnapshot {
            next_branch_id: self.next_branch_id,
            next_revision_id: self.next_revision_id,
            next_checkpoint_id: self.next_checkpoint_id,
            next_image_id: self.next_image_id,
            branches: self.branches.clone(),
            revisions: self.revisions.clone(),
            checkpoints: self.checkpoints.clone(),
            images,
            trace_images,
            observations: self.observations.clone(),
        }
    }

    /// Rebuild lineage state from one durable lineage snapshot.
    pub(crate) fn from_snapshot(snapshot: LineageSnapshot) -> Self {
        let images = snapshot
            .images
            .into_iter()
            .map(|(image_id, image)| (image_id, Arc::new(image)))
            .collect();
        let trace_images = snapshot
            .trace_images
            .into_iter()
            .map(|(revision_id, trace_image)| (revision_id, Arc::new(trace_image)))
            .collect();

        Self {
            next_branch_id: snapshot.next_branch_id,
            next_revision_id: snapshot.next_revision_id,
            next_checkpoint_id: snapshot.next_checkpoint_id,
            next_image_id: snapshot.next_image_id,
            branches: snapshot.branches,
            revisions: snapshot.revisions,
            checkpoints: snapshot.checkpoints,
            images,
            trace_images,
            observations: snapshot.observations,
        }
    }

    /// Allocate one new branch identifier.
    pub(crate) fn allocate_branch_id(&mut self) -> BranchId {
        let branch_id = BranchId::new(self.next_branch_id);
        self.next_branch_id += 1;
        branch_id
    }

    /// Allocate one new revision identifier.
    pub(crate) fn allocate_revision_id(&mut self) -> RevisionId {
        let revision_id = RevisionId::new(self.next_revision_id);
        self.next_revision_id += 1;
        revision_id
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
    pub(crate) fn set_branch_head_revision_id(
        &mut self,
        branch_id: BranchId,
        revision_id: RevisionId,
    ) {
        self.revisions
            .get(&revision_id)
            .expect("world lineage must contain the requested revision");
        let branch = self
            .branches
            .get_mut(&branch_id)
            .expect("world lineage must contain the requested branch");
        branch.head_revision_id = revision_id;
    }

    /// Create one child branch from one parent revision.
    pub(crate) fn fork_branch(
        &mut self,
        parent_revision_id: RevisionId,
        name: String,
    ) -> RuntimeResult<Branch> {
        // resolve the parent revision before mutating lineage state
        let parent_revision = self.revisions.get(&parent_revision_id).ok_or_else(|| {
            RuntimeError::RevisionNotFound {
                revision_id: parent_revision_id.get(),
            }
            .boxed()
        })?;
        let head_revision_id = parent_revision.id;

        let branch = Branch {
            id: self.allocate_branch_id(),
            head_revision_id,
            origin: BranchOrigin::Fork { parent_revision_id },
            name,
            labels: BTreeMap::new(),
        };
        self.branches.insert(branch.id, branch.clone());

        Ok(branch)
    }

    /// Commit one new materialized revision and update the branch head.
    pub(crate) fn commit_revision(
        &mut self,
        branch_id: BranchId,
        mut image: Image,
        trace_image: TraceImage,
        retain_image: bool,
        wall: WorldInstant,
        mono: WorldInstant,
        checkpoint_name: Option<String>,
    ) -> RuntimeResult<CommittedRevision> {
        let parent_branch = self.branches.get(&branch_id).cloned().ok_or_else(|| {
            RuntimeError::BranchNotFound {
                branch_id: branch_id.get(),
            }
            .boxed()
        })?;

        image.id = self.allocate_image_id();
        let image = Arc::new(image);
        let trace_image = Arc::new(trace_image);
        let revision = Revision {
            id: self.allocate_revision_id(),
            branch_id,
            parent_revision_id: Some(parent_branch.head_revision_id),
            sequence: trace_image.next_sequence,
            image_id: image.id,
            wall,
            mono,
            labels: BTreeMap::new(),
        };

        let branch = Branch {
            head_revision_id: revision.id,
            ..parent_branch
        };

        let checkpoint = checkpoint_name.map(|name| Checkpoint {
            id: self.allocate_checkpoint_id(),
            revision_id: revision.id,
            name,
            labels: BTreeMap::new(),
        });

        if retain_image {
            self.images.insert(image.id, image.clone());
        }
        self.trace_images.insert(revision.id, trace_image.clone());
        self.revisions.insert(revision.id, revision.clone());
        self.branches.insert(branch_id, branch.clone());

        if let Some(checkpoint) = checkpoint.clone() {
            self.checkpoints.insert(checkpoint.id, checkpoint);
        }

        Ok(CommittedRevision {
            revision,
            image,
            checkpoint,
        })
    }

    /// Return the revision that owns one image identifier.
    pub(crate) fn revision_id_for_image_id(&self, image_id: ImageId) -> Option<RevisionId> {
        self.revisions.iter().find_map(|(revision_id, revision)| {
            (revision.image_id == image_id).then_some(*revision_id)
        })
    }

    /// Resolve one materialized revision and all of its backing records.
    pub(crate) fn resolve_revision_backing(
        &self,
        revision_id: RevisionId,
    ) -> RuntimeResult<RevisionBacking> {
        let revision = self.revisions.get(&revision_id).cloned().ok_or_else(|| {
            RuntimeError::RevisionNotFound {
                revision_id: revision_id.get(),
            }
            .boxed()
        })?;
        let image = self
            .images
            .get(&revision.image_id)
            .cloned()
            .ok_or_else(|| {
                RuntimeError::RevisionImageMissing {
                    revision_id: revision.id.get(),
                    image_id: revision.image_id.get(),
                }
                .boxed()
            })?;
        let trace_image = self
            .trace_images
            .get(&revision.id)
            .cloned()
            .ok_or_else(|| {
                RuntimeError::RevisionTraceImageMissing {
                    revision_id: revision.id.get(),
                }
                .boxed()
            })?;

        Ok(RevisionBacking {
            revision,
            image,
            trace_image,
        })
    }

    /// Return the nearest materialized revision at or before one target revision.
    pub(crate) fn nearest_image_revision_id(
        &self,
        revision_id: RevisionId,
    ) -> RuntimeResult<RevisionId> {
        let mut current_revision_id = revision_id;

        loop {
            let revision = self.revisions.get(&current_revision_id).ok_or_else(|| {
                RuntimeError::RevisionNotFound {
                    revision_id: current_revision_id.get(),
                }
                .boxed()
            })?;

            if self.images.contains_key(&revision.image_id) {
                return Ok(current_revision_id);
            }

            let Some(parent_revision_id) = revision.parent_revision_id else {
                return Err(RuntimeError::RevisionImageMissing {
                    revision_id: revision.id.get(),
                    image_id: revision.image_id.get(),
                }
                .boxed());
            };

            current_revision_id = parent_revision_id;
        }
    }

    /// Resolve one revision restore plan from the nearest materialized image.
    pub(crate) fn resolve_revision_restore_plan(
        &self,
        revision_id: RevisionId,
    ) -> RuntimeResult<RevisionRestorePlan> {
        let target_revision = self.revisions.get(&revision_id).cloned().ok_or_else(|| {
            RuntimeError::RevisionNotFound {
                revision_id: revision_id.get(),
            }
            .boxed()
        })?;
        let base_revision_id = self.nearest_image_revision_id(revision_id)?;
        let base_backing = self.resolve_revision_backing(base_revision_id)?;
        let trace_image = self
            .trace_images
            .get(&target_revision.id)
            .cloned()
            .ok_or_else(|| {
                RuntimeError::RevisionTraceImageMissing {
                    revision_id: target_revision.id.get(),
                }
                .boxed()
            })?;

        Ok(RevisionRestorePlan {
            target_revision,
            base_revision: base_backing.revision,
            image: base_backing.image,
            trace_image,
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

        let branch_observations = self.observations.entry(branch_id).or_default();
        branch_observations.extend(observations);
        branch_observations.sort_by_key(|record| (record.moment.sequence, record.sequence));
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

        let head_revision = self.head_revision_for_branch(end.branch_id)?;
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
    pub(crate) fn head_revision_for_branch(&self, branch_id: BranchId) -> RuntimeResult<Revision> {
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
                let parent_revision = self.revisions.get(&parent_revision_id).ok_or_else(|| {
                    RuntimeError::RevisionNotFound {
                        revision_id: parent_revision_id.get(),
                    }
                })?;
                parent_revision.sequence
            }
        };

        Ok(Moment::new(branch_id, sequence))
    }

    /// Return the current committed head moment for one branch.
    pub(crate) fn branch_head_moment(&self, branch_id: BranchId) -> RuntimeResult<Moment> {
        let head_revision = self.head_revision_for_branch(branch_id)?;

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
            let parent_revision = self.revisions.get(&parent_revision_id).ok_or_else(|| {
                RuntimeError::RevisionNotFound {
                    revision_id: parent_revision_id.get(),
                }
            })?;
            branch = self.branch(parent_revision.branch_id)?;
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
        let left_head = self.head_revision_for_branch(left_branch_id)?;
        let right_head = self.head_revision_for_branch(right_branch_id)?;

        let mut left_chain = BTreeMap::new();
        let mut current_left = Some(left_head.id);
        while let Some(revision_id) = current_left {
            let revision = self.revisions.get(&revision_id).ok_or_else(|| {
                RuntimeError::RevisionNotFound {
                    revision_id: revision_id.get(),
                }
                .boxed()
            })?;
            left_chain.insert(revision.id, revision.clone());
            current_left = revision.parent_revision_id;
        }

        let mut current_right = Some(right_head.id);
        while let Some(revision_id) = current_right {
            let revision = self.revisions.get(&revision_id).ok_or_else(|| {
                RuntimeError::RevisionNotFound {
                    revision_id: revision_id.get(),
                }
                .boxed()
            })?;

            if let Some(common) = left_chain.get(&revision.id) {
                return Ok(common.clone());
            }

            current_right = revision.parent_revision_id;
        }

        Err(RuntimeError::Internal {
            message: "branches in one lineage must share a common ancestor revision".to_string(),
        }
        .boxed())
    }

    /// Return the latest committed revision at or before one target sequence on one branch.
    fn latest_revision_at_or_before(
        &self,
        branch_id: BranchId,
        sequence: TraceSequence,
    ) -> RuntimeResult<Revision> {
        let mut revision = self.head_revision_for_branch(branch_id)?;

        if revision.sequence.get() < sequence.get() {
            return Err(RuntimeError::MomentNotFound {
                branch_id: branch_id.get(),
                sequence: sequence.get(),
            }
            .boxed());
        }

        loop {
            let Some(parent_revision_id) = revision.parent_revision_id else {
                return Ok(revision);
            };

            let parent_revision = self.revisions.get(&parent_revision_id).ok_or_else(|| {
                RuntimeError::RevisionNotFound {
                    revision_id: parent_revision_id.get(),
                }
                .boxed()
            })?;

            if parent_revision.sequence.get() > sequence.get() {
                revision = parent_revision.clone();
                continue;
            }

            return Ok(parent_revision.clone());
        }
    }

    /// Resolve one moment restore plan from the nearest materialized image.
    pub(crate) fn resolve_moment_restore_plan(
        &self,
        moment: Moment,
    ) -> RuntimeResult<MomentRestorePlan> {
        let head_revision = self.head_revision_for_branch(moment.branch_id)?;
        if head_revision.sequence.get() < moment.sequence.get() {
            return Err(RuntimeError::MomentNotFound {
                branch_id: moment.branch_id.get(),
                sequence: moment.sequence.get(),
            }
            .boxed());
        }

        let anchor_revision =
            self.latest_revision_at_or_before(moment.branch_id, moment.sequence)?;
        let base_revision_id = self.nearest_image_revision_id(anchor_revision.id)?;
        let base_backing = self.resolve_revision_backing(base_revision_id)?;
        Ok(MomentRestorePlan {
            target_moment: moment,
            anchor_revision,
            base_revision: base_backing.revision,
            image: base_backing.image,
        })
    }
}
