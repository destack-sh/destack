use crate::platform::{ResourceBacking, ResourceCapture, ResourceId, ResourcePortability};
use serde::{Deserialize, Serialize};

use super::topology::{EdgeId, EntityId, EntityKind};

/// World resource record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Resource {
    /// Stable resource identifier.
    pub id: ResourceId,
    /// Resource kind identifier.
    pub kind: EntityKind,
    /// Optional resource label.
    pub label: Option<String>,
    /// Backing model for this resource.
    pub backing: ResourceBacking,
    /// Capture model for this resource.
    pub capture: ResourceCapture,
    /// Portability model for this resource.
    pub portability: ResourcePortability,
}

impl Resource {
    /// Create one world resource record.
    pub fn new(
        id: ResourceId,
        kind: impl Into<EntityKind>,
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

/// Return the canonical topology entity id for one resource.
pub(crate) fn resource_entity_id(resource_id: ResourceId) -> EntityId {
    EntityId::new(format!(
        "resource.{}.{}",
        resource_id.worker_id.0, resource_id.local_id
    ))
}

/// Return the canonical ownership edge id for one resource.
pub(crate) fn resource_ownership_edge_id(resource_id: ResourceId) -> EdgeId {
    EdgeId::new(format!(
        "worker.{}.owns.resource.{}",
        resource_id.worker_id.0, resource_id.local_id
    ))
}
