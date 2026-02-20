use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::runtime::replay::{BranchId, CheckpointId, LogSequence};
use crate::runtime::snapshot::SnapshotMetadata;
use destack_base::fnv1a_128;

/// Snapshot persistence and restore service.
#[derive(Debug)]
pub struct SnapshotStore {
    // NOTE #Incomplete: implement snapshot reading and verification
    /// Root directory where snapshot files are written.
    root: PathBuf,
    /// Next checkpoint identifier.
    next_id: AtomicU64,
}

impl SnapshotStore {
    /// Create a snapshot store rooted at the given path.
    pub fn new(root: PathBuf) -> Self {
        Self {
            root,
            next_id: AtomicU64::new(1),
        }
    }

    /// Return the root directory for snapshot files.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Allocate a new checkpoint identifier.
    pub fn allocate_checkpoint_id(&self) -> CheckpointId {
        let next = self.next_id.fetch_add(1, Ordering::Relaxed);
        CheckpointId::new(next as u128)
    }

    /// Write a snapshot payload and return its metadata.
    pub fn write_snapshot(
        &self,
        checkpoint_id: CheckpointId,
        branch_id: BranchId,
        sequence: LogSequence,
        payload: &[u8],
    ) -> RuntimeResult<SnapshotMetadata> {
        // create snapshot directory if needed
        std::fs::create_dir_all(&self.root).map_err(|error| {
            RuntimeError::Internal {
                message: format!("snapshot directory create failed: {error}"),
            }
            .boxed()
        })?;

        // determine output path
        let filename = format!("checkpoint-{:032x}.snap", checkpoint_id.get());
        let path = self.root.join(filename);

        // write payload bytes to disk
        std::fs::write(&path, payload).map_err(|error| {
            RuntimeError::Internal {
                message: format!("snapshot write failed: {error}"),
            }
            .boxed()
        })?;

        // compute payload hash and size
        let hash = fnv1a_128(payload);
        let size_bytes = payload.len() as u64;

        Ok(SnapshotMetadata {
            checkpoint_id,
            branch_id,
            sequence,
            path: path.to_string_lossy().into_owned(),
            hash,
            size_bytes,
        })
    }
}

impl Default for SnapshotStore {
    fn default() -> Self {
        Self::new(PathBuf::from(".destack/snapshots"))
    }
}
