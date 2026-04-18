use std::borrow::Borrow;
use std::fmt;

use serde::{Deserialize, Serialize};

use crate::runtime::WorkerId;

/// Stable identifier for one runtime instance in one world.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct RuntimeId(pub u64);

impl RuntimeId {
    /// Return the canonical topology entity id for this runtime.
    pub fn entity_id(self) -> WorldEntityId {
        WorldEntityId::new(format!("runtime.{}", self.0))
    }

    /// Return the canonical ownership edge id for one worker owned by this runtime.
    pub fn owns_worker_edge_id(self, worker_id: WorkerId) -> WorldEdgeId {
        WorldEdgeId::new(format!("runtime.{}.owns.worker.{}", self.0, worker_id.0))
    }
}

impl fmt::Display for RuntimeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl WorkerId {
    /// Return the canonical topology entity id for this worker.
    pub fn entity_id(self) -> WorldEntityId {
        WorldEntityId::new(format!("worker.{}", self.0))
    }
}

/// Stable identifier for one topology entity.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct WorldEntityId(pub String);

impl WorldEntityId {
    /// Create one entity id.
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// Return the stable id as a string slice.
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl Borrow<str> for WorldEntityId {
    fn borrow(&self) -> &str {
        self.as_str()
    }
}

impl From<String> for WorldEntityId {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl From<&str> for WorldEntityId {
    fn from(value: &str) -> Self {
        Self(value.to_string())
    }
}

impl fmt::Display for WorldEntityId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Stable identifier for one topology edge.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct WorldEdgeId(pub String);

impl WorldEdgeId {
    /// Create one edge id.
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// Return the stable id as a string slice.
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl Borrow<str> for WorldEdgeId {
    fn borrow(&self) -> &str {
        self.as_str()
    }
}

impl From<String> for WorldEdgeId {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl From<&str> for WorldEdgeId {
    fn from(value: &str) -> Self {
        Self(value.to_string())
    }
}

impl fmt::Display for WorldEdgeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Stable identifier for one topology entity kind.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct WorldEntityKind(pub String);

impl WorldEntityKind {
    /// Create one entity kind id.
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// Return the stable id as a string slice.
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl Borrow<str> for WorldEntityKind {
    fn borrow(&self) -> &str {
        self.as_str()
    }
}

impl From<String> for WorldEntityKind {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl From<&str> for WorldEntityKind {
    fn from(value: &str) -> Self {
        Self(value.to_string())
    }
}

impl fmt::Display for WorldEntityKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Stable identifier for one topology edge kind.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct WorldEdgeKind(pub String);

impl WorldEdgeKind {
    /// Create one edge kind id.
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// Return the stable id as a string slice.
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl Borrow<str> for WorldEdgeKind {
    fn borrow(&self) -> &str {
        self.as_str()
    }
}

impl From<String> for WorldEdgeKind {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl From<&str> for WorldEdgeKind {
    fn from(value: &str) -> Self {
        Self(value.to_string())
    }
}

impl fmt::Display for WorldEdgeKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}
