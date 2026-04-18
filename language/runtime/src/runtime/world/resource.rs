use std::cmp::Ordering;

use crate::platform::{ResourceBacking, ResourceCapture, ResourceId, ResourcePortability};
use crate::runtime::WorkerId;
use serde::{Deserialize, Serialize};

use super::topology::{WorldEdgeId, WorldEntityId, WorldEntityKind};

/// Stable identifier for one world resource record.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WorldResourceId {
    /// Worker owner of this resource.
    pub worker_id: WorkerId,
    /// Resource identifier in the owning worker table.
    pub resource_id: ResourceId,
}

impl WorldResourceId {
    /// Create one world resource identifier.
    pub const fn new(worker_id: WorkerId, resource_id: ResourceId) -> Self {
        Self {
            worker_id,
            resource_id,
        }
    }

    /// Return the canonical topology entity id for this resource.
    pub fn entity_id(self) -> WorldEntityId {
        WorldEntityId::new(format!(
            "resource.{}.{}",
            self.worker_id.0, self.resource_id.0
        ))
    }

    /// Return the canonical ownership edge id for this resource.
    pub fn ownership_edge_id(self) -> WorldEdgeId {
        WorldEdgeId::new(format!(
            "worker.{}.owns.resource.{}",
            self.worker_id.0, self.resource_id.0
        ))
    }
}

impl PartialOrd for WorldResourceId {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for WorldResourceId {
    fn cmp(&self, other: &Self) -> Ordering {
        (self.worker_id.0, self.resource_id.0).cmp(&(other.worker_id.0, other.resource_id.0))
    }
}

/// Logical world resource record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorldResource {
    /// Stable world resource identifier.
    pub id: WorldResourceId,
    /// Resource kind identifier.
    pub kind: WorldEntityKind,
    /// Optional resource label.
    pub label: Option<String>,
    /// Backing model for this resource.
    pub backing: ResourceBacking,
    /// Capture model for this resource.
    pub capture: ResourceCapture,
    /// Portability model for this resource.
    pub portability: ResourcePortability,
}

impl WorldResource {
    /// Create one logical world resource record.
    pub fn new(
        id: WorldResourceId,
        kind: impl Into<WorldEntityKind>,
        label: Option<String>,
        backing: ResourceBacking,
        capture: ResourceCapture,
        portability: ResourcePortability,
    ) -> Self {
        Self {
            id,
            kind: kind.into(),
            label,
            backing,
            capture,
            portability,
        }
    }
}
