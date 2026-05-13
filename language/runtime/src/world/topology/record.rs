use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use super::{EdgeId, EdgeKind, EntityId, EntityKind};

/// Topology entity-kind registration payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EntityDefinition {
    /// Stable kind identifier.
    pub kind: EntityKind,
    /// Kind labels for selector matching.
    pub labels: BTreeMap<String, String>,
    /// Fault verbs supported by this kind.
    #[serde(default)]
    pub supported_faults: BTreeSet<String>,
}

impl EntityDefinition {
    /// Create one entity kind definition.
    pub fn new(kind: impl Into<EntityKind>) -> Self {
        Self {
            kind: kind.into(),
            labels: BTreeMap::new(),
            supported_faults: BTreeSet::new(),
        }
    }

    /// Add one label.
    pub fn label(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.labels.insert(key.into(), value.into());
        self
    }

    /// Replace labels.
    pub fn labels(mut self, labels: BTreeMap<String, String>) -> Self {
        self.labels = labels;
        self
    }

    /// Add one supported fault verb id.
    pub fn supports_fault(mut self, verb_id: impl Into<String>) -> Self {
        self.supported_faults.insert(verb_id.into());
        self
    }

    /// Extend supported fault verb ids.
    pub fn supports_faults(mut self, verb_ids: impl IntoIterator<Item = String>) -> Self {
        self.supported_faults.extend(verb_ids);
        self
    }
}

/// Topology edge-kind registration payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EdgeDefinition {
    /// Stable kind identifier.
    pub kind: EdgeKind,
    /// Kind labels for selector matching.
    pub labels: BTreeMap<String, String>,
    /// Fault verbs supported by this kind.
    #[serde(default)]
    pub supported_faults: BTreeSet<String>,
}

impl EdgeDefinition {
    /// Create one edge kind definition.
    pub fn new(kind: impl Into<EdgeKind>) -> Self {
        Self {
            kind: kind.into(),
            labels: BTreeMap::new(),
            supported_faults: BTreeSet::new(),
        }
    }

    /// Add one label.
    pub fn label(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.labels.insert(key.into(), value.into());
        self
    }

    /// Replace labels.
    pub fn labels(mut self, labels: BTreeMap<String, String>) -> Self {
        self.labels = labels;
        self
    }

    /// Add one supported fault verb id.
    pub fn supports_fault(mut self, verb_id: impl Into<String>) -> Self {
        self.supported_faults.insert(verb_id.into());
        self
    }

    /// Extend supported fault verb ids.
    pub fn supports_faults(mut self, verb_ids: impl IntoIterator<Item = String>) -> Self {
        self.supported_faults.extend(verb_ids);
        self
    }
}

/// Topology entity payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Entity {
    /// Stable entity identifier.
    pub id: EntityId,
    /// Stable entity kind identifier.
    pub kind: EntityKind,
    /// Entity labels.
    pub labels: BTreeMap<String, String>,
}

impl Entity {
    /// System label key that stores one runtime display name.
    pub const LABEL_RUNTIME_NAME: &'static str = "runtime.instance.name";

    /// System label key that stores one worker display name.
    pub const LABEL_WORKER_NAME: &'static str = "runtime.worker.name";

    /// Create one entity payload.
    pub fn new(id: impl Into<EntityId>, kind: impl Into<EntityKind>) -> Self {
        Self {
            id: id.into(),
            kind: kind.into(),
            labels: BTreeMap::new(),
        }
    }

    /// Add one label.
    pub fn label(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.labels.insert(key.into(), value.into());
        self
    }

    /// Replace labels.
    pub fn labels(mut self, labels: BTreeMap<String, String>) -> Self {
        self.labels = labels;
        self
    }
}

/// Topology edge payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Edge {
    /// Stable edge identifier.
    pub id: EdgeId,
    /// Stable edge kind identifier.
    pub kind: EdgeKind,
    /// Source entity identifier.
    pub from: EntityId,
    /// Destination entity identifier.
    pub to: EntityId,
    /// Edge labels.
    pub labels: BTreeMap<String, String>,
}

impl Edge {
    /// Create one edge payload.
    pub fn new(
        id: impl Into<EdgeId>,
        kind: impl Into<EdgeKind>,
        from: impl Into<EntityId>,
        to: impl Into<EntityId>,
    ) -> Self {
        Self {
            id: id.into(),
            kind: kind.into(),
            from: from.into(),
            to: to.into(),
            labels: BTreeMap::new(),
        }
    }

    /// Add one label.
    pub fn label(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.labels.insert(key.into(), value.into());
        self
    }

    /// Replace labels.
    pub fn labels(mut self, labels: BTreeMap<String, String>) -> Self {
        self.labels = labels;
        self
    }
}
