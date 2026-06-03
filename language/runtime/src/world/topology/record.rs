use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::{EdgeId, EdgeKind, EntityId, EntityKind};

/// Topology entity-kind registration payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EntityDefinition {
    /// Stable kind identifier.
    pub kind: EntityKind,
    /// Kind labels for selector matching.
    pub labels: BTreeMap<String, String>,
}

impl EntityDefinition {
    /// Create one entity kind definition.
    pub fn new(kind: impl Into<EntityKind>) -> Self {
        Self {
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

/// Topology edge-kind registration payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EdgeDefinition {
    /// Stable kind identifier.
    pub kind: EdgeKind,
    /// Kind labels for selector matching.
    pub labels: BTreeMap<String, String>,
}

impl EdgeDefinition {
    /// Create one edge kind definition.
    pub fn new(kind: impl Into<EdgeKind>) -> Self {
        Self {
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

/// Topology entity payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Entity {
    /// Stable entity identifier.
    pub id: EntityId,
    /// Stable entity kind identifier.
    pub kind: EntityKind,
    /// Stable entity name.
    pub name: String,
    /// Entity labels.
    pub labels: BTreeMap<String, String>,
}

impl Entity {
    /// Create one entity payload.
    pub fn new(id: impl Into<EntityId>, kind: impl Into<EntityKind>) -> Self {
        let id = id.into();
        let name = id.to_string();

        Self {
            id,
            kind: kind.into(),
            name,
            labels: BTreeMap::new(),
        }
    }

    /// Set the entity name.
    pub fn named(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
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
    /// Stable edge name.
    pub name: String,
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
        let id = id.into();
        let name = id.to_string();

        Self {
            id,
            kind: kind.into(),
            name,
            from: from.into(),
            to: to.into(),
            labels: BTreeMap::new(),
        }
    }

    /// Set the edge name.
    pub fn named(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
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
