use destack_core::StringPool;
use destack_dir::StringId;

/// Cached string ids for type literal identifiers.
#[derive(Debug)]
pub(crate) struct TypeLiteralIdentifiers {
    /// The `undefined` id.
    pub(crate) undefined: StringId,
    /// The `null` id.
    pub(crate) null_: StringId,
}

impl TypeLiteralIdentifiers {
    /// Create cached ids for the current string pool.
    pub(crate) fn new(strings: &StringPool) -> Self {
        Self {
            undefined: strings.intern("undefined"),
            null_: strings.intern("null"),
        }
    }
}
