use destack_dir as dir;
use smallvec::SmallVec;

use crate::check::VariableId;

/// Runtime nominal instance check term.
///
/// ```ts
/// value instanceof Shape.Circle
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub(in crate::check) struct InstanceCheckTerm {
    /// The source instance check expression.
    pub(in crate::check) source: dir::GlobalNodeIdAny,
    /// The checked value type.
    pub(in crate::check) value: VariableId,
    /// The target constructor or nominal type value.
    pub(in crate::check) target: VariableId,
}

impl InstanceCheckTerm {
    /// Return variables referenced by this term.
    pub(in crate::check) fn referenced_variables(&self) -> SmallVec<[VariableId; 4]> {
        smallvec::smallvec![self.value, self.target]
    }
}
