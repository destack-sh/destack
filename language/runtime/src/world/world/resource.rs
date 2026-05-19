use serde::{Deserialize, Serialize};

use crate::host::{ResourceBacking, ResourceCapture, ResourceId, ResourcePortability};

use super::topology::{EdgeId, EntityId};

/// World resource record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Resource {
    /// Stable resource identifier.
    pub id: ResourceId,
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
        backing: ResourceBacking,
        capture: ResourceCapture,
        portability: ResourcePortability,
    ) -> Self {
        Self {
            id,
            backing,
            capture,
            portability,
        }
    }

    /// Return the canonical topology entity id for this resource.
    pub fn entity_id(&self) -> EntityId {
        EntityId::new(format!(
            "runtime.resource.{}.{}",
            self.id.worker_id.0, self.id.local_id
        ))
    }

    /// Return the canonical ownership edge id for this resource.
    pub fn ownership_edge_id(&self) -> EdgeId {
        EdgeId::new(format!(
            "runtime.worker.{}.owns.resource.{}",
            self.id.worker_id.0, self.id.local_id
        ))
    }
}
