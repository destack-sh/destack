use dyst_source::SmallVec;

use crate::StringId;

/// The base of a path.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PathBase {
    /// The self base type.
    SelfType,
    /// The self base value.
    SelfValue,
    /// The module base.
    Module,
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
