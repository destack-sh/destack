use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use super::{WorldEdgeId, WorldEdgeKind, WorldEntityId, WorldEntityKind};

/// Topology entity-kind registration payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorldEntityKindDefinition {
    /// Stable kind identifier.
    pub kind: WorldEntityKind,
    /// Kind labels for selector matching.
    pub labels: BTreeMap<String, String>,
    /// Fault verbs supported by this kind.
    #[serde(default)]
    pub supported_faults: BTreeSet<String>,
}

impl WorldEntityKindDefinition {
    /// Create one entity kind definition.
    pub fn new(kind: impl Into<WorldEntityKind>) -> Self {
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
pub struct WorldEdgeKindDefinition {
    /// Stable kind identifier.
    pub kind: WorldEdgeKind,
    /// Kind labels for selector matching.
    pub labels: BTreeMap<String, String>,
    /// Fault verbs supported by this kind.
    #[serde(default)]
    pub supported_faults: BTreeSet<String>,
}

impl WorldEdgeKindDefinition {
    /// Create one edge kind definition.
    pub fn new(kind: impl Into<WorldEdgeKind>) -> Self {
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
pub struct WorldEntity {
    /// Stable entity identifier.
    pub id: WorldEntityId,
    /// Stable entity kind identifier.
    pub kind: WorldEntityKind,
    /// Entity labels.
    pub labels: BTreeMap<String, String>,
}

impl WorldEntity {
    /// Create one entity payload.
    pub fn new(id: impl Into<WorldEntityId>, kind: impl Into<WorldEntityKind>) -> Self {
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
pub struct WorldEdge {
    /// Stable edge identifier.
    pub id: WorldEdgeId,
    /// Stable edge kind identifier.
    pub kind: WorldEdgeKind,
    /// Source entity identifier.
    pub from: WorldEntityId,
    /// Destination entity identifier.
    pub to: WorldEntityId,
    /// Edge labels.
    pub labels: BTreeMap<String, String>,
}

impl WorldEdge {
    /// Create one edge payload.
    pub fn new(
        id: impl Into<WorldEdgeId>,
        kind: impl Into<WorldEdgeKind>,
        from: impl Into<WorldEntityId>,
        to: impl Into<WorldEntityId>,
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
