use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::host::{ResourceBacking, ResourceCapture, ResourceId, ResourcePortability};

use super::topology::{EdgeId, EntityId, EntityKind};

/// World resource record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Resource {
    /// Stable resource identifier.
    pub id: ResourceId,
    /// Resource kind identifier.
    pub kind: EntityKind,
    /// Resource labels.
    pub labels: BTreeMap<String, String>,
    /// Backing model for this resource.
    pub backing: ResourceBacking,
    /// Capture model for this resource.
    pub capture: ResourceCapture,
    /// Portability model for this resource.
    pub portability: ResourcePortability,
}

impl Resource {
    /// System label key that stores one resource display name.
    pub const LABEL_NAME: &'static str = "runtime.resource.name";

    /// Create one world resource record.
    pub fn new(
        id: ResourceId,
        kind: impl Into<EntityKind>,
        backing: ResourceBacking,
        capture: ResourceCapture,
        portability: ResourcePortability,
    ) -> Self {
        Self {
            id,
            kind: kind.into(),
            labels: BTreeMap::new(),
            backing,
            capture,
            portability,
        }
    }

    /// Add one resource label.
    pub fn label(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.labels.insert(key.into(), value.into());
        self
    }

    /// Replace resource labels.
    pub fn labels(mut self, labels: BTreeMap<String, String>) -> Self {
        self.labels = labels;
        self
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
