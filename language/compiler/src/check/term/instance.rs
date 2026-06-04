use destack_dir as dir;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::check::{CheckState, TypeOperand, TypeTerm, VariableId};

/// Runtime nominal instance check term.
///
/// ```ds
/// value instanceof Shape.Circle
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub(in crate::check) struct InstanceCheckTerm {
    /// The source instance check expression.
    pub(in crate::check) source: dir::GlobalNodeIdAny,
    /// The value type operand.
    pub(in crate::check) value: TypeOperand,
    /// The target constructor or nominal type value.
    pub(in crate::check) target: TypeOperand,
}

impl InstanceCheckTerm {
    /// Return variables referenced by this term.
    pub(in crate::check) fn referenced_variables(
        &self,
        state: &CheckState<'_>,
    ) -> SmallVec<[VariableId; 2]> {
        let mut variables = SmallVec::new();
        variables.extend(self.value.referenced_variables(state));
        variables.extend(self.target.referenced_variables(state));
        variables
    }
}

impl CheckState<'_> {
    /// Reduce one runtime instance check to boolean.
    pub(in crate::check) fn reduce_instance_check_term(
        &self,
        _instance: &InstanceCheckTerm,
    ) -> CompilerResult<Option<TypeTerm>> {
        Ok(None)
    }
}
