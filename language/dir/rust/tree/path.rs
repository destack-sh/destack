use dyst_source::SmallVec;

use crate::{Definition, Intrinsic, NodeId, StringId};

/// The base of a path.
#[derive(Debug, Clone, PartialEq)]
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

/// A evaluated path.
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

    /// Evaluated Path to an intrinsic.
    Intrinsic { intrinsic: Intrinsic },
    /// Evaluated to a Definition.
    Definition { definition: NodeId<Definition> },
    /// Error path.
    Error,
}

impl Path {
    /// Whether the path is evaluated (ignoring child nodes).
    pub fn is_evaluated(&self) -> bool {
        matches!(self, Path::Intrinsic { .. } | Path::Definition { .. })
    }
}

/// A block target for a control flow statement.
#[derive(Debug, Clone, PartialEq)]
pub enum BlockTarget {
    /// Unevaluated block target with a string label.
    UnevaluatedString { label: StringId },
    /// Evaluated block target to a Definition.
    Definition { definition: NodeId<Definition> },
    /// Error target.
    Error,
}

impl BlockTarget {
    /// Whether the target is evaluated (ignoring child nodes).
    pub fn is_evaluated(&self) -> bool {
        matches!(self, BlockTarget::Definition { .. })
    }
}
