use destack_dir as dir;
use smallvec::SmallVec;

use crate::check::VariableId;

/// Runtime identity equality term.
///
/// ```ts
/// left !== right
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub(in crate::check) struct IdentityTerm {
    /// The source identity expression.
    pub(in crate::check) source: dir::GlobalNodeIdAny,
    /// The source operator.
    pub(in crate::check) operator: dir::BinaryOperator,
    /// The left operand type.
    pub(in crate::check) left: VariableId,
    /// The right operand type.
    pub(in crate::check) right: VariableId,
}

impl IdentityTerm {
    /// Return variables referenced by this term.
    pub(in crate::check) fn referenced_variables(&self) -> SmallVec<[VariableId; 4]> {
        smallvec::smallvec![self.left, self.right]
    }
}
