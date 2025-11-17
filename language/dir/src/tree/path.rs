use dyst_source::SmallVec;

use crate::{Definition, Intrinsic, NodeId, ScopeId, StringId};

/// The base of a path.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PathBase {
    /// The self base type.
    SelfType,
    /// The self base value.
    SelfValue,
    /// The super base type.
    SuperType,
    /// The super base value.
    SuperValue,
    /// The module base.
    Module,
    /// The package base.
    Package,
}

/// A resolved path.
#[derive(Debug, Clone, PartialEq)]
pub enum Path {
    /// Unresolved base.
    UnresolvedBase { base: PathBase },
    /// Unresolved relative string path.
    UnresolvedRelativeString {
        base: PathBase,
        segments: SmallVec<StringId, 3>,
    },
    /// Unresolved absolute string path.
    UnresolvedAbsoluteString { segments: SmallVec<StringId, 3> },

    /// Resolved Path to an intrinsic.
    Intrinsic { intrinsic: Intrinsic },
    /// Resolved to a Definition.
    Definition { definition: NodeId<Definition> },
}

impl Path {
    /// Whether the path is resolved (ignoring child nodes).
    pub fn is_resolved(&self) -> bool {
        matches!(self, Path::Intrinsic { .. } | Path::Definition { .. })
    }
}

/// A block target for a control flow statement.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BlockTarget {
    /// Unresolved block target with a string label.
    Unresolved { label: StringId },
    /// Resolved block target to a Scope.
    Scope { scope: ScopeId },
}

impl BlockTarget {
    /// Whether the target is resolved (ignoring child nodes).
    pub fn is_resolved(&self) -> bool {
        matches!(self, BlockTarget::Scope { .. })
    }
}
