use destack_dir as dir;

use crate::CompilerResult;
use crate::check::{Answer, CheckState, Origin, TypeLiteralTerm, TypeOperand, TypeTerm};

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

impl CheckState<'_> {
    /// Reduce one runtime key membership check to boolean.
    pub(in crate::check) fn reduce_key_membership_term(
        &mut self,
        membership: KeyMembershipTerm,
    ) -> CompilerResult<Answer<TypeOperand>> {
        let origin = Origin::Node(membership.source);

        // wait for the key operand
        let Answer::Ready(_) = self.reduce_type_operand(origin, membership.key)? else {
            return Ok(Answer::pending(membership.key.dependencies(self)));
        };

        // wait for the receiver operand
        let Answer::Ready(_) = self.reduce_type_operand(origin, membership.receiver)? else {
            return Ok(Answer::pending(membership.receiver.dependencies(self)));
        };

        let term = TypeTerm::Literal(TypeLiteralTerm::boolean());
        let operand = self.type_term_operand(term);

        Ok(Answer::Ready(operand))
    }
}
