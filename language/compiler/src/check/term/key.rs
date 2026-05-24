use destack_dir as dir;
use smallvec::SmallVec;

use crate::check::VariableId;

/// Runtime key membership check term.
///
/// ```ts
/// key in value
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub(in crate::check) struct KeyMembershipTerm {
    /// The source membership expression.
    pub(in crate::check) source: dir::GlobalNodeIdAny,
    /// The key expression type.
    pub(in crate::check) key: VariableId,
    /// The receiver expression type.
    pub(in crate::check) receiver: VariableId,
    /// The direct key when syntax makes it statically obvious.
    pub(in crate::check) static_key: Option<dir::StaticKey>,
}

impl KeyMembershipTerm {
    /// Return variables referenced by this term.
    pub(in crate::check) fn referenced_variables(&self) -> SmallVec<[VariableId; 4]> {
        smallvec::smallvec![self.key, self.receiver]
    }
}
