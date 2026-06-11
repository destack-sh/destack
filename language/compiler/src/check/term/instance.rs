use destack_dir as dir;

use crate::CompilerResult;
use crate::check::{Answer, CheckState, Origin, TypeLiteralTerm, TypeOperand, TypeTerm};

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

impl CheckState<'_> {
    /// Reduce one runtime instance check to boolean.
    pub(in crate::check) fn reduce_instance_check_term(
        &mut self,
        instance: InstanceCheckTerm,
    ) -> CompilerResult<Answer<TypeOperand>> {
        let origin = Origin::Node(instance.source);

        // wait for the checked value
        let Answer::Ready(_) = self.reduce_type_operand(origin, instance.value)? else {
            return Ok(Answer::pending(instance.value.dependencies(self)));
        };

        // wait for the nominal target value
        let Answer::Ready(_) = self.reduce_type_operand(origin, instance.target)? else {
            return Ok(Answer::pending(instance.target.dependencies(self)));
        };

        let term = TypeTerm::Literal(TypeLiteralTerm::boolean());
        let operand = self.type_term_operand(term);

        Ok(Answer::Ready(operand))
    }
}
