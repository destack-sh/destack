use super::{BranchId, ImageId, RevisionId};

/// First active branch identifier for one new World.
pub(crate) const ROOT_BRANCH: BranchId = BranchId::new(0);
/// First active revision for one new World.
pub(crate) const ROOT_REVISION: RevisionId = RevisionId::new(0);
/// Root image identifier for one new World.
pub(crate) const ROOT_IMAGE_ID: ImageId = ImageId::new(0);
/// First allocated branch identifier after the root branch.
pub(super) const INITIAL_BRANCH_ID: u64 = 1;
/// First allocated revision identifier after the root revision.
pub(super) const INITIAL_REVISION_ID: u64 = 1;
/// First allocated checkpoint identifier.
pub(super) const INITIAL_CHECKPOINT_ID: u64 = 1;
/// First allocated image identifier after the root image.
pub(super) const INITIAL_IMAGE_ID: u64 = 1;
