use serde::{Deserialize, Serialize};

use crate::GlobalTypeId;

/// One solved relation between declarations.
///
/// Examples:
/// ```ds
/// class Admin extends User { ... }
/// struct Point implements Printable { ... }
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Relation {
    /// The relation kind.
    pub kind: RelationKind,
    /// The related type after static evaluation.
    pub ty: GlobalTypeId,
}

impl Relation {
    /// Create a solved relation.
    pub fn new(kind: RelationKind, ty: GlobalTypeId) -> Self {
        Self { kind, ty }
    }

    /// Create a solved extends relationship.
    pub fn extends(ty: GlobalTypeId) -> Self {
        Self::new(RelationKind::Extends, ty)
    }

    /// Create a solved implements relationship.
    pub fn implements(ty: GlobalTypeId) -> Self {
        Self::new(RelationKind::Implements, ty)
    }
}

/// The kind of solved relation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RelationKind {
    /// A single inheritance parent relation, like `class Admin extends User`.
    Extends,
    /// An explicit interface conformance relation, like `struct Point implements Printable`.
    Implements,
}
