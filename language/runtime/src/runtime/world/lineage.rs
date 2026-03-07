use std::collections::BTreeMap;
use std::sync::Arc;

use super::{Branch, BranchId, BranchOrigin, Checkpoint, CheckpointId, Snapshot, SnapshotId};

/// First active branch identifier for one new world.
pub(super) const ROOT_BRANCH_ID: BranchId = BranchId::new(0);
/// First allocated branch identifier after the root branch.
const INITIAL_BRANCH_ID: u128 = 1;
/// First allocated checkpoint identifier.
const INITIAL_CHECKPOINT_ID: u128 = 1;
/// First allocated snapshot identifier.
const INITIAL_SNAPSHOT_ID: u128 = 1;

/// World-owned lineage metadata and durable restore metadata.
#[derive(Debug)]
pub(super) struct Lineage {
    /// The next branch identifier to allocate.
    pub next_branch_id: u128,
    /// The next checkpoint identifier to allocate.
    pub next_checkpoint_id: u128,
    /// The next snapshot identifier to allocate.
    pub next_snapshot_id: u128,
    /// The known branch metadata records.
    pub branches: BTreeMap<BranchId, Branch>,
    /// The known checkpoint metadata records.
    pub checkpoints: BTreeMap<CheckpointId, Checkpoint>,
    /// The known snapshot metadata records.
    pub snapshots: BTreeMap<SnapshotId, Arc<Snapshot>>,
}

impl Default for Lineage {
    fn default() -> Self {
        let mut branches = BTreeMap::new();
        let _ = branches.insert(
            ROOT_BRANCH_ID,
            Branch {
                id: ROOT_BRANCH_ID,
                origin: BranchOrigin::Root,
                name: "root".to_string(),
                labels: BTreeMap::new(),
            },
        );

        Self {
            next_branch_id: INITIAL_BRANCH_ID,
            next_checkpoint_id: INITIAL_CHECKPOINT_ID,
            next_snapshot_id: INITIAL_SNAPSHOT_ID,
            branches,
            checkpoints: BTreeMap::new(),
            snapshots: BTreeMap::new(),
        }
    }
}
