use std::borrow::Borrow;
use std::fmt;

use serde::{Deserialize, Serialize};
use tspp_serde::Reflect;

use crate::runtime::RuntimeId;
use crate::worker::WorkerId;

impl RuntimeId {
    /// Return the canonical topology entity id for this runtime.
    pub fn entity_id(self) -> EntityId {
        EntityId::new(format!("runtime.instance.{}", self.0))
    }

    /// Return the canonical ownership edge id for one worker owned by this runtime.
    pub fn owns_worker_edge_id(self, worker_id: WorkerId) -> EdgeId {
        EdgeId::new(format!(
            "runtime.instance.{}.owns.worker.{}",
            self.0, worker_id.0
        ))
    }
}

impl WorkerId {
    /// Return the canonical topology entity id for this worker.
    pub fn entity_id(self) -> EntityId {
        EntityId::new(format!("runtime.worker.{}", self.0))
    }
}

/// Stable identifier for one topology entity.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Reflect)]
#[serde(transparent)]
pub struct EntityId(pub String);

impl EntityId {
    /// Create one entity id.
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// Return the stable id as a string slice.
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl Borrow<str> for EntityId {
    fn borrow(&self) -> &str {
        self.as_str()
    }
}

impl From<String> for EntityId {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl From<&str> for EntityId {
    fn from(value: &str) -> Self {
        Self(value.to_string())
    }
}

impl fmt::Display for EntityId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Stable identifier for one topology edge.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Reflect)]
#[serde(transparent)]
pub struct EdgeId(pub String);

impl EdgeId {
    /// Create one edge id.
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// Return the stable id as a string slice.
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl Borrow<str> for EdgeId {
    fn borrow(&self) -> &str {
        self.as_str()
    }
}

impl From<String> for EdgeId {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl From<&str> for EdgeId {
    fn from(value: &str) -> Self {
        Self(value.to_string())
    }
}

impl fmt::Display for EdgeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Stable identifier for one topology entity kind.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Reflect)]
#[serde(transparent)]
pub struct EntityKind(pub String);

impl EntityKind {
    /// System label key that stores one topology kind id.
    pub const LABEL_KIND: &'static str = "runtime.topology.kind";

    /// Builtin runtime entity kind id.
    pub const RUNTIME: &'static str = "runtime.instance";

    /// Builtin worker entity kind id.
    pub const WORKER: &'static str = "runtime.worker";

    /// Create one entity kind id.
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// Return the stable id as a string slice.
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl Borrow<str> for EntityKind {
    fn borrow(&self) -> &str {
        self.as_str()
    }
}

impl From<String> for EntityKind {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl From<&str> for EntityKind {
    fn from(value: &str) -> Self {
        Self(value.to_string())
    }
}

impl fmt::Display for EntityKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Stable identifier for one topology edge kind.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Reflect)]
#[serde(transparent)]
pub struct EdgeKind(pub String);

impl EdgeKind {
    /// Builtin runtime-to-worker edge kind id.
    pub const RUNTIME_OWNS_WORKER: &'static str = "runtime.instance.owns.worker";

    /// Builtin worker-to-resource edge kind id.
    pub const WORKER_OWNS_RESOURCE: &'static str = "runtime.worker.owns.resource";

    /// Create one edge kind id.
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// Return the stable id as a string slice.
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl Borrow<str> for EdgeKind {
    fn borrow(&self) -> &str {
        self.as_str()
    }
}

impl From<String> for EdgeKind {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl From<&str> for EdgeKind {
    fn from(value: &str) -> Self {
        Self(value.to_string())
    }
}

impl fmt::Display for EdgeKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}
