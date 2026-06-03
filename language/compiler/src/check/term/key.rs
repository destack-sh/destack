use destack_dir as dir;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::check::{CheckState, TypeOperand, TypeTerm, VariableId};

/// Runtime key membership check term.
///
/// ```ds
/// key in value
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub(in crate::check) struct KeyMembershipTerm {
    /// The source membership expression.
    pub(in crate::check) source: dir::GlobalNodeIdAny,
    /// The key expression type.
    pub(in crate::check) key: TypeOperand,
    /// The receiver expression type.
    pub(in crate::check) receiver: TypeOperand,
}

impl KeyMembershipTerm {
    /// Return variables referenced by this term.
    pub(in crate::check) fn referenced_variables(
        &self,
        state: &CheckState<'_>,
    ) -> SmallVec<[VariableId; 2]> {
        let mut variables = SmallVec::new();

        variables.extend(self.key.referenced_variables(state));
        variables.extend(self.receiver.referenced_variables(state));

        variables
    }
}

impl CheckState<'_> {
    /// Reduce one runtime key membership check to boolean.
    pub(in crate::check) fn reduce_key_membership_term(
        &self,
        _membership: &KeyMembershipTerm,
    ) -> CompilerResult<Option<TypeTerm>> {
        todo!("reduce key membership through structural and nominal member tables")
    }
}
