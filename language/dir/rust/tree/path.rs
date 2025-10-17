use dyst_container::SmallVec;

use crate::{Definition, Intrinsic, NodeId, StringId};

/// The base of a path.
#[derive(Debug, Clone, PartialEq)]
pub enum PathBase {
    /// The self base type.
    SelfType,
    /// The self base value.
    SelfValue,
    /// The module base.
    Module,
    /// The package base.
    Package,
}

/// A resolved path.
#[derive(Debug, Clone, PartialEq)]
pub enum Path {
    /// Unevaluated base.
    UnevaluatedBase { base: PathBase },
    /// Unevaluated relative string path.
    UnevaluatedRelativeString {
        base: PathBase,
        segments: SmallVec<StringId, 3>,
    },
    /// Unevaluated absolute string path.
    UnevaluatedAbsoluteString { segments: SmallVec<StringId, 3> },

    /// Resolved Path to an intrinsic.
    Intrinsic { intrinsic: Intrinsic },
    /// Resolved to a Definition.
    Definition { definition: NodeId<Definition> },
    /// Error path.
    Error,
}

impl Path {
    /// Whether the path is resolved (ignoring child nodes).
    pub fn is_resolved(&self) -> bool {
        matches!(self, Path::Intrinsic { .. } | Path::Definition { .. })
    }
}

/// A destination is a target for a control flow statement.
#[derive(Debug, Clone, PartialEq)]
pub enum Destination {
    /// Unevaluated Destination with a string label.
    UnevaluatedString { label: StringId },
    /// Resolved Destination to a Definition.
    Definition { definition: NodeId<Definition> },
    /// Error destination.
    Error,
}

impl Destination {
    /// Whether the destination is resolved (ignoring child nodes).
    pub fn is_resolved(&self) -> bool {
        matches!(self, Destination::Definition { .. })
    }
}
