use std::collections::BTreeMap;
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::runtime::replay::{TraceImage, TraceSequence};
use crate::runtime::time::WorldInstant;

use super::{
    Branch, BranchId, BranchOrigin, Checkpoint, CheckpointId, Image, ImageId, Revision, RevisionId,
    TraceImageId,
};

/// First active branch identifier for one new world.
pub(super) const ROOT_BRANCH_ID: BranchId = BranchId::new(0);
/// First active revision identifier for one new world.
pub(super) const ROOT_REVISION_ID: RevisionId = RevisionId::new(0);
/// Root image identifier for one new world.
pub(super) const ROOT_IMAGE_ID: ImageId = ImageId::new(0);
/// Root trace image identifier for one new world.
pub(super) const ROOT_TRACE_IMAGE_ID: TraceImageId = TraceImageId::new(0);
/// First allocated branch identifier after the root branch.
const INITIAL_BRANCH_ID: u128 = 1;
/// First allocated revision identifier after the root revision.
const INITIAL_REVISION_ID: u128 = 1;
/// First allocated checkpoint identifier.
const INITIAL_CHECKPOINT_ID: u128 = 1;
/// First allocated image identifier.
const INITIAL_IMAGE_ID: u128 = 1;
/// First allocated trace image identifier.
const INITIAL_TRACE_IMAGE_ID: u128 = 1;

/// World-owned lineage metadata and durable restore metadata.
#[derive(Debug)]
pub(super) struct Lineage {
    /// The next branch identifier to allocate.
    pub next_branch_id: u128,
    /// The next revision identifier to allocate.
    pub next_revision_id: u128,
    /// The next checkpoint identifier to allocate.
    pub next_checkpoint_id: u128,
    /// The next image identifier to allocate.
    pub next_image_id: u128,
    /// The next trace image identifier to allocate.
    pub next_trace_image_id: u128,
    /// The known branch metadata records.
    pub branches: BTreeMap<BranchId, Branch>,
    /// The known revision metadata records.
    pub revisions: BTreeMap<RevisionId, Revision>,
    /// The known checkpoint metadata records.
    pub checkpoints: BTreeMap<CheckpointId, Checkpoint>,
    /// The known image metadata records.
    pub images: BTreeMap<ImageId, Arc<Image>>,
    /// The known trace image records.
    pub trace_images: BTreeMap<TraceImageId, Arc<TraceImage>>,
}

/// Fully resolved backing for one materialized revision.
#[derive(Debug, Clone)]
pub(super) struct RevisionBacking {
    /// The resolved revision metadata.
    pub revision: Revision,
    /// The resolved world image.
    pub image: Arc<Image>,
    /// The resolved trace image.
    pub trace_image: Arc<TraceImage>,
}

/// One committed lineage update for one new materialized revision.
#[derive(Debug, Clone)]
pub(super) struct CommittedRevision {
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
    /// The next trace image identifier to allocate.
    pub next_trace_image_id: u128,
    /// The known branch metadata records.
    pub branches: BTreeMap<BranchId, Branch>,
    /// The known revision metadata records.
    pub revisions: BTreeMap<RevisionId, Revision>,
    /// The known checkpoint metadata records.
    pub checkpoints: BTreeMap<CheckpointId, Checkpoint>,
    /// The known image metadata records.
    pub images: BTreeMap<ImageId, Image>,
    /// The known trace image records.
    pub trace_images: BTreeMap<TraceImageId, TraceImage>,
}

impl Lineage {
    /// Create one bootstrap lineage before the root backing exists.
    pub(super) fn bootstrap() -> Self {
        let mut branches = BTreeMap::new();
        let mut revisions = BTreeMap::new();

        revisions.insert(
            ROOT_REVISION_ID,
            Revision {
                id: ROOT_REVISION_ID,
                branch_id: ROOT_BRANCH_ID,
                parent_revision_id: None,
                sequence: TraceSequence::new(0),
                image_id: ROOT_IMAGE_ID,
                trace_image_id: ROOT_TRACE_IMAGE_ID,
                wall: WorldInstant::new(0),
                mono: WorldInstant::new(0),
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

        Self {
            next_branch_id: INITIAL_BRANCH_ID,
            next_revision_id: INITIAL_REVISION_ID,
            next_checkpoint_id: INITIAL_CHECKPOINT_ID,
            next_image_id: INITIAL_IMAGE_ID,
            next_trace_image_id: INITIAL_TRACE_IMAGE_ID,
            branches,
            revisions,
            checkpoints: BTreeMap::new(),
            images: BTreeMap::new(),
            trace_images: BTreeMap::new(),
        }
    }

    /// Create one lineage with one fully materialized root revision.
    pub(super) fn bootstrap_root(image: Arc<Image>, trace_image: Arc<TraceImage>) -> Self {
        let mut lineage = Self::bootstrap();
        lineage.images.insert(ROOT_IMAGE_ID, image);
        lineage
            .trace_images
            .insert(ROOT_TRACE_IMAGE_ID, trace_image);

        lineage
    }

    /// Capture one durable lineage snapshot.
    pub(super) fn snapshot(&self) -> LineageSnapshot {
        let images = self
            .images
            .iter()
            .map(|(image_id, image)| (*image_id, image.as_ref().clone()))
            .collect();
        let trace_images = self
            .trace_images
            .iter()
            .map(|(trace_image_id, trace_image)| (*trace_image_id, trace_image.as_ref().clone()))
            .collect();

        LineageSnapshot {
            next_branch_id: self.next_branch_id,
            next_revision_id: self.next_revision_id,
            next_checkpoint_id: self.next_checkpoint_id,
            next_image_id: self.next_image_id,
            next_trace_image_id: self.next_trace_image_id,
            branches: self.branches.clone(),
            revisions: self.revisions.clone(),
            checkpoints: self.checkpoints.clone(),
            images,
            trace_images,
        }
    }

    /// Rebuild lineage state from one durable lineage snapshot.
    pub(super) fn from_snapshot(snapshot: LineageSnapshot) -> Self {
        let images = snapshot
            .images
            .into_iter()
            .map(|(image_id, image)| (image_id, Arc::new(image)))
            .collect();
        let trace_images = snapshot
            .trace_images
            .into_iter()
            .map(|(trace_image_id, trace_image)| (trace_image_id, Arc::new(trace_image)))
            .collect();

        Self {
            next_branch_id: snapshot.next_branch_id,
            next_revision_id: snapshot.next_revision_id,
            next_checkpoint_id: snapshot.next_checkpoint_id,
            next_image_id: snapshot.next_image_id,
            next_trace_image_id: snapshot.next_trace_image_id,
            branches: snapshot.branches,
            revisions: snapshot.revisions,
            checkpoints: snapshot.checkpoints,
            images,
            trace_images,
        }
    }

    /// Allocate one new branch identifier.
    pub(super) fn allocate_branch_id(&mut self) -> BranchId {
        let branch_id = BranchId::new(self.next_branch_id);
        self.next_branch_id += 1;
        branch_id
    }

    /// Allocate one new revision identifier.
    pub(super) fn allocate_revision_id(&mut self) -> RevisionId {
        let revision_id = RevisionId::new(self.next_revision_id);
        self.next_revision_id += 1;
        revision_id
    }

    /// Allocate one new checkpoint identifier.
    pub(super) fn allocate_checkpoint_id(&mut self) -> CheckpointId {
        let checkpoint_id = CheckpointId::new(self.next_checkpoint_id);
        self.next_checkpoint_id += 1;
        checkpoint_id
    }

    /// Allocate one new image identifier.
    pub(super) fn allocate_image_id(&mut self) -> ImageId {
        let image_id = ImageId::new(self.next_image_id);
        self.next_image_id += 1;
        image_id
    }

    /// Allocate one new trace image identifier.
    pub(super) fn allocate_trace_image_id(&mut self) -> TraceImageId {
        let trace_image_id = TraceImageId::new(self.next_trace_image_id);
        self.next_trace_image_id += 1;
        trace_image_id
    }

    /// Set the current head revision for one branch.
    pub(super) fn set_branch_head_revision_id(
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
    pub(super) fn fork_branch(
        &mut self,
        parent_revision_id: RevisionId,
        name: String,
    ) -> RuntimeResult<Branch> {
        // resolve the parent revision before mutating lineage state
        let parent_revision = self.revisions.get(&parent_revision_id).ok_or_else(|| {
            RuntimeError::Internal {
                message: format!("revision {} does not exist", parent_revision_id.get()),
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
    pub(super) fn commit_revision(
        &mut self,
        branch_id: BranchId,
        mut image: Image,
        trace_image: TraceImage,
        wall: WorldInstant,
        mono: WorldInstant,
        checkpoint_name: Option<String>,
    ) -> RuntimeResult<CommittedRevision> {
        let parent_branch = self.branches.get(&branch_id).cloned().ok_or_else(|| {
            RuntimeError::Internal {
                message: format!("branch {} does not exist", branch_id.get()),
            }
            .boxed()
        })?;

        image.id = self.allocate_image_id();
        let image = Arc::new(image);
        let trace_image_id = self.allocate_trace_image_id();
        let trace_image = Arc::new(trace_image);
        let revision = Revision {
            id: self.allocate_revision_id(),
            branch_id,
            parent_revision_id: Some(parent_branch.head_revision_id),
            sequence: trace_image.next_sequence,
            image_id: image.id,
            trace_image_id,
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

        self.images.insert(image.id, image.clone());
        self.trace_images
            .insert(trace_image_id, trace_image.clone());
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
    pub(super) fn revision_id_for_image_id(&self, image_id: ImageId) -> Option<RevisionId> {
        self.revisions.iter().find_map(|(revision_id, revision)| {
            (revision.image_id == image_id).then_some(*revision_id)
        })
    }

    /// Resolve one materialized revision and all of its backing records.
    pub(super) fn resolve_revision_backing(
        &self,
        revision_id: RevisionId,
    ) -> RuntimeResult<RevisionBacking> {
        let revision = self.revisions.get(&revision_id).cloned().ok_or_else(|| {
            RuntimeError::Internal {
                message: format!("revision {} does not exist", revision_id.get()),
            }
            .boxed()
        })?;
        let image = self
            .images
            .get(&revision.image_id)
            .cloned()
            .ok_or_else(|| {
                RuntimeError::Internal {
                    message: format!(
                        "revision {} is missing image {}",
                        revision.id.get(),
                        revision.image_id.get()
                    ),
                }
                .boxed()
            })?;
        let trace_image = self
            .trace_images
            .get(&revision.trace_image_id)
            .cloned()
            .ok_or_else(|| {
                RuntimeError::Internal {
                    message: format!(
                        "revision {} is missing trace image {}",
                        revision.id.get(),
                        revision.trace_image_id.get()
                    ),
                }
                .boxed()
            })?;

        Ok(RevisionBacking {
            revision,
            image,
            trace_image,
        })
    }
}
