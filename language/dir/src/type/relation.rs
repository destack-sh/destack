use serde::{Deserialize, Serialize};

use crate::LocalTypeId;

/// One solved relation between declarations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Relation {
    /// The relation kind.
    pub kind: RelationKind,
    /// The related type after static evaluation.
    pub ty: LocalTypeId,
}

impl Relation {
    /// Create a solved relation.
    pub fn new(kind: RelationKind, ty: LocalTypeId) -> Self {
        Self { kind, ty }
    }

    /// Create a solved extends relationship.
    pub fn extends(ty: LocalTypeId) -> Self {
        Self::new(RelationKind::Extends, ty)
    }

    /// Create a solved implements relationship.
    pub fn implements(ty: LocalTypeId) -> Self {
        Self::new(RelationKind::Implements, ty)
    }
}

/// The kind of solved relation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RelationKind {
    /// A single inheritance parent relation.
    Extends,
    /// An explicit interface conformance relation.
    Implements,
}
