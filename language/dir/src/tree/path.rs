use dyst_source::SmallVec;

use crate::{ScopeId, StringId};

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

/// Path to something. Resolves to symbols, expressions, etc.
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
