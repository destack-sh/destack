use destack_dir as dir;

use super::{TypeRelation, VariableId};

/// Check failure found during solving.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) enum CheckFailure {
    /// Variable was bound to incompatible values.
    VariableConflict {
        /// The conflicting variable.
        variable: VariableId,
    },
    /// Static expression could not be evaluated during check.
    StaticEvaluation {
        /// The static expression node.
        node: dir::GlobalNodeIdAny,
    },
    /// Type expression could not be evaluated during check.
    TypeEvaluation {
        /// The type expression node.
        node: dir::GlobalNodeIdAny,
    },
    /// Solved types failed a required relation.
    TypeRelation {
        /// The required relation.
        relation: TypeRelation,
        /// The source type.
        left: VariableId,
        /// The target type.
        right: VariableId,
    },
}
