use serde::{Deserialize, Serialize};

use crate::platform::diagnostic::PlatformError;
use crate::platform::resource::{ResourceId, ResourceKind};

/// Checkpoint capability for a resource.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResourceSnapshotPolicy {
    /// Resource can be fully serialized and restored.
    Checkpointable,
    /// Resource can be reattached from a stored recipe.
    Reattachable,
    /// Resource cannot be checkpointed and acts as a barrier.
    Uncheckpointable,
}

/// Descriptor for reattaching an external resource.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceDescriptor {
    /// Resource kind for validation.
    pub kind: ResourceKind,
    /// Adapter-defined payload for reattaching.
    pub payload: Vec<u8>,
}

/// Snapshot payload for a resource.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceSnapshot {
    /// Resource identifier in the table.
    pub resource_id: ResourceId,
    /// Resource kind for validation.
    pub kind: ResourceKind,
    /// Snapshot policy applied to the resource.
    pub policy: ResourceSnapshotPolicy,
    /// Snapshot payload for restoring state.
    pub payload: Vec<u8>,
    /// Optional recipe for reattaching external resources.
    pub reattach: Option<ResourceDescriptor>,
}

/// Adapter used to snapshot or reattach a resource.
pub trait ResourceSnapshotAdapter: Send + Sync {
    /// Snapshot the resource state into a payload.
    fn snapshot(&self, resource_id: ResourceId) -> Result<ResourceSnapshot, Box<PlatformError>>;

    /// Restore a resource from the snapshot payload.
    fn restore(&self, snapshot: &ResourceSnapshot) -> Result<(), Box<PlatformError>>;
}
