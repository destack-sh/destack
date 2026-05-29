use destack_dir as dir;

use crate::check::{TypeOperand, VariableId};

/// Writable storage selected by source syntax.
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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) enum PlaceTarget {
    /// Local or imported value binding.
    Binding {
        /// The local binding symbol selected by syntax.
        symbol: dir::GlobalSymbolId,
    },
    /// Structural or nominal member target.
    MemberTerm {
        /// The receiver type.
        owner: VariableId,
        /// The selected member key.
        key: dir::StaticKey,
    },
    /// Protocol-backed index target.
    IndexTerm {
        /// The indexed receiver type.
        receiver: VariableId,
        /// The index expression type.
        index: VariableId,
    },
    /// Protocol-backed dereference target.
    Dereference {
        /// The dereference output type.
        output: VariableId,
    },
}
