use destack_dir as dir;

use crate::check::TypeOperand;

/// Writable storage selected by source syntax.
///
/// Examples:
/// ```ds
/// value = next
/// object.field = next
/// values[index] = next
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) struct Place {
    /// The selected storage type.
    pub(in crate::check) ty: TypeOperand,
    /// How source syntax selected the place.
    pub(in crate::check) target: PlaceTarget,
    /// The source syntax node for diagnostics.
    pub(in crate::check) source: dir::GlobalNodeIdAny,
}

impl Place {
    /// Create a place.
    pub(in crate::check) fn new(
        ty: TypeOperand,
        target: PlaceTarget,
        source: dir::GlobalNodeIdAny,
    ) -> Self {
        Self { ty, target, source }
    }
}

/// How source syntax selects a place.
///
/// Examples:
/// ```ds
/// value
/// object.field
/// values[index]
/// *pointer
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) enum PlaceTarget {
    /// Local or imported value binding.
    ///
    /// Examples:
    /// ```ds
    /// value = next
    /// ```
    Binding {
        /// The local binding symbol selected by syntax.
        symbol: dir::GlobalSymbolId,
    },
    /// Structural or nominal member target.
    ///
    /// Examples:
    /// ```ds
    /// object.field = next
    /// ```
    Member {
        /// The receiver type.
        owner: TypeOperand,
        /// The selected member key.
        key: dir::StaticKey,
    },
    /// Protocol-backed index target.
    ///
    /// Examples:
    /// ```ds
    /// values[index] = next
    /// ```
    Index {
        /// The indexed receiver type.
        receiver: TypeOperand,
        /// The index expression type.
        index: TypeOperand,
    },
    /// Protocol-backed dereference target.
    ///
    /// Examples:
    /// ```ds
    /// *pointer = next
    /// ```
    Dereference,
}
