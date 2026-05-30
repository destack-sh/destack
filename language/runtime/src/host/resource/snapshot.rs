use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::diagnostic::HostError;
use crate::host::resource::table::ResourceEntry;
use crate::host::resource::{ResourceAffinity, ResourceId, ResourceKind};

/// Restore model for one resource kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResourceRestore {
    /// The resource does not support capture.
    None,
    /// The resource restores from serialized host state.
    State,
    /// The resource restores through an external rebinding hook.
    Rebind,
}

/// Durable restore payload for one attached resource.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResourceSnapshot {
    /// Fully serialized resource state.
    Snapshot {
        /// Resource identifier in the table.
        resource_id: ResourceId,
        /// Resource kind for validation.
        kind: ResourceKind,
        /// Snapshot payload for restoring state.
        payload: Vec<u8>,
    },
    /// Reattach recipe for one external resource.
    Reattach {
        /// Resource identifier in the table.
        resource_id: ResourceId,
        /// Resource kind for validation.
        kind: ResourceKind,
        /// Host-defined payload for reattaching.
        payload: Vec<u8>,
    },
}

impl ResourceSnapshot {
    /// Return the resource identifier captured in this snapshot.
    pub fn resource_id(&self) -> ResourceId {
        match self {
            Self::Snapshot { resource_id, .. } | Self::Reattach { resource_id, .. } => *resource_id,
        }
    }

    /// Return the resource kind captured in this snapshot.
    pub fn kind(&self) -> ResourceKind {
        match self {
            Self::Snapshot { kind, .. } | Self::Reattach { kind, .. } => *kind,
        }
    }
}

/// Materialized resource entry captured in one resource-table image.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResourceImageEntry {
    /// Resource identifier in the table.
    pub resource_id: ResourceId,
    /// Resource kind for validation and restore.
    pub kind: ResourceKind,
    /// Optional diagnostic label for the resource.
    pub label: Option<String>,
    /// Restore model for the resource.
    pub restore: ResourceRestore,
    /// Optional execution-affinity requirement for the resource.
    pub affinity: Option<ResourceAffinity>,
    /// Optional captured restore payload.
    pub snapshot: Option<ResourceSnapshot>,
}

/// Rebinder used to restore one externally rebound resource kind.
pub trait ResourceRebinder: Send + Sync {
    /// Rebind one resource entry from one captured snapshot payload.
    fn rebind(&self, snapshot: &ResourceSnapshot) -> Result<ResourceEntry, Box<HostError>>;
}

/// Rebinding hooks for restoring external resources.
#[derive(Default, Clone)]
pub struct ResourceRebinders {
    /// Rebinding hooks keyed by resource kind.
    rebinders: HashMap<ResourceKind, Arc<dyn ResourceRebinder>>,
}

impl fmt::Debug for ResourceRebinders {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ResourceRebinders")
            .field("rebinder_count", &self.rebinders.len())
            .finish()
    }
}

impl ResourceRebinders {
    /// Register one rebinding hook for one resource kind.
    pub fn register(&mut self, resource_kind: ResourceKind, rebinder: Arc<dyn ResourceRebinder>) {
        let _ = self.rebinders.insert(resource_kind, rebinder);
    }

    /// Return one registered rebinder when present.
    pub fn rebinder(&self, resource_kind: ResourceKind) -> Option<Arc<dyn ResourceRebinder>> {
        self.rebinders.get(&resource_kind).cloned()
    }
}

/// Provider used to snapshot or restore one resource kind.
pub trait ResourceProvider: Send + Sync {
    /// Snapshot the resource state into a payload.
    fn snapshot(&self, resource_id: ResourceId) -> Result<ResourceSnapshot, Box<HostError>>;

    /// Restore one resource entry from the snapshot payload.
    fn restore(&self, snapshot: &ResourceSnapshot) -> Result<ResourceEntry, Box<HostError>>;
}
