use destack_dir as dir;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::check::{CheckState, TypeLiteralTerm, TypeTerm, VariableId};

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

impl CheckState<'_> {
    /// Reduce one runtime instance check to boolean.
    pub(in crate::check) fn reduce_instance_check_term(
        &self,
        instance: &InstanceCheckTerm,
    ) -> CompilerResult<Option<TypeTerm>> {
        // wait for both operands so failed operands own their diagnostics
        if self.solved_type_term(instance.value)?.is_none()
            || self.solved_type_term(instance.target)?.is_none()
        {
            return Ok(None);
        }

        Ok(Some(TypeTerm::Literal(TypeLiteralTerm::boolean())))
    }
}
