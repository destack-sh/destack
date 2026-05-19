use std::fmt;

use serde::{Deserialize, Serialize};

use super::{EdgeId, EdgeKind, EntityId, EntityKind};

/// Error type for world topology definition and mutation failures.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum TopologyError {
    /// One kind identifier was empty.
    EmptyKindId,
    /// One topology entity was already defined.
    DuplicateEntity {
        /// The duplicated entity identifier.
        entity_id: EntityId,
    },
    /// One topology edge was already defined.
    DuplicateEdge {
        /// The duplicated edge identifier.
        edge_id: EdgeId,
    },
    /// One entity kind was already defined.
    DuplicateEntityKind {
        /// The duplicated kind identifier.
        kind: EntityKind,
    },
    /// One edge kind was already defined.
    DuplicateEdgeKind {
        /// The duplicated kind identifier.
        kind: EdgeKind,
    },
    /// One builtin kind cannot be undefined.
    BuiltinKind {
        /// The protected kind identifier.
        kind: String,
    },
    /// One entity kind is still used by live entities.
    EntityKindInUse {
        /// The kind identifier still in use.
        kind: EntityKind,
    },
    /// One edge kind is still used by live edges.
    EdgeKindInUse {
        /// The kind identifier still in use.
        kind: EdgeKind,
    },
    /// One entity kind was not defined.
    UnknownEntityKind {
        /// The missing kind identifier.
        kind: EntityKind,
    },
    /// One edge kind was not defined.
    UnknownEdgeKind {
        /// The missing kind identifier.
        kind: EdgeKind,
    },
    /// One topology entity was missing.
    UnknownEntity {
        /// The missing entity identifier.
        entity_id: EntityId,
        /// The entity role in the failed relation.
        role: EntityRole,
    },
    /// One topology entity had the wrong identifier.
    EntityIdMismatch {
        /// The expected entity identifier.
        expected: EntityId,
        /// The provided entity identifier.
        actual: EntityId,
    },
    /// One topology entity had the wrong kind.
    EntityKindMismatch {
        /// The entity identifier.
        entity_id: EntityId,
        /// The expected entity kind.
        expected: EntityKind,
        /// The provided entity kind.
        actual: EntityKind,
    },
}

/// Result type for world topology operations.
pub(crate) type TopologyResult<T> = Result<T, TopologyError>;

/// Role of one referenced topology entity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum EntityRole {
    /// The source side of one edge.
    Source,
    /// The destination side of one edge.
    Destination,
}

impl fmt::Display for EntityRole {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let role = match self {
            EntityRole::Source => "source",
            EntityRole::Destination => "destination",
        };

        f.write_str(role)
    }
}

impl fmt::Display for TopologyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TopologyError::EmptyKindId => f.write_str("topology kind id must not be empty"),
            TopologyError::DuplicateEntity { entity_id } => {
                write!(f, "topology entity {entity_id} is already defined")
            }
            TopologyError::DuplicateEdge { edge_id } => {
                write!(f, "topology edge {edge_id} is already defined")
            }
            TopologyError::DuplicateEntityKind { kind } => {
                write!(f, "topology entity kind {kind} is already defined")
            }
            TopologyError::DuplicateEdgeKind { kind } => {
                write!(f, "topology edge kind {kind} is already defined")
            }
            TopologyError::BuiltinKind { kind } => {
                write!(f, "topology builtin kind {kind} cannot be undefined")
            }
            TopologyError::EntityKindInUse { kind } => {
                write!(f, "topology entity kind {kind} is still in use")
            }
            TopologyError::EdgeKindInUse { kind } => {
                write!(f, "topology edge kind {kind} is still in use")
            }
            TopologyError::UnknownEntityKind { kind } => {
                write!(f, "topology entity kind {kind} is not defined")
            }
            TopologyError::UnknownEdgeKind { kind } => {
                write!(f, "topology edge kind {kind} is not defined")
            }
            TopologyError::UnknownEntity { entity_id, role } => {
                write!(f, "topology {role} entity {entity_id} does not exist")
            }
            TopologyError::EntityIdMismatch { expected, actual } => {
                write!(
                    f,
                    "topology entity id mismatch: expected {expected}, got {actual}"
                )
            }
            TopologyError::EntityKindMismatch {
                entity_id,
                expected,
                actual,
            } => {
                write!(
                    f,
                    "topology entity {entity_id} kind mismatch: expected {expected}, got {actual}"
                )
            }
        }
    }
}

impl std::error::Error for TopologyError {}
