use destack_dir as dir;

use crate::CompilerResult;
use crate::check::{
    Answer, CheckState, IdentityDecision, IdentityFailure, IdentityResolution, TermId,
    TypeLiteralTerm, TypeOperand, TypeTerm,
};

/// Runtime identity equality term.
///
/// ```ds
/// left !== right
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub(in crate::check) struct IdentityTerm {
    /// The source identity expression.
    pub(in crate::check) source: dir::GlobalNodeIdAny,
    /// The source operator.
    pub(in crate::check) operator: dir::BinaryOperator,
    /// The left operand type.
    pub(in crate::check) left: TypeOperand,
    /// The right operand type.
    pub(in crate::check) right: TypeOperand,
}

impl CheckState<'_> {
    /// Reduce one strict identity comparison to boolean.
    pub(in crate::check) fn reduce_identity_term(
        &mut self,
        identity: &IdentityTerm,
    ) -> CompilerResult<Answer<TypeOperand>> {
        if let Some(decision) = self.inference.identity(identity.source) {
            return Ok(match decision {
                IdentityDecision::Resolved(_) => {
                    let term = TypeTerm::Literal(TypeLiteralTerm::boolean());
                    let operand = self.type_term_operand(term);

                    Answer::Ready(operand)
                }
                IdentityDecision::Rejected(_) => {
                    let term = TypeTerm::Literal(TypeLiteralTerm::Error);
                    let operand = self.type_term_operand(term);

                    Answer::Ready(operand)
                }
            });
        }

        let Some(left) = self.type_operand_term_id(identity.left)? else {
            return Ok(Answer::pending(identity.left.dependencies(self)));
        };
        let Some(right) = self.type_operand_term_id(identity.right)? else {
            return Ok(Answer::pending(identity.right.dependencies(self)));
        };

        // choose identity comparison only for identity bearing domains
        let is_compatible = self.identity_compatible(left, right)?;
        if is_compatible {
            let resolution = IdentityResolution {
                source: identity.source,
                left: identity.left,
                right: identity.right,
            };

            self.inference
                .select_identity(identity.source, IdentityDecision::Resolved(resolution))?;

            let term = TypeTerm::Literal(TypeLiteralTerm::boolean());
            let operand = self.type_term_operand(term);

            return Ok(Answer::Ready(operand));
        }

        self.inference.select_identity(
            identity.source,
            IdentityDecision::Rejected(IdentityFailure::Incompatible),
        )?;

        let term = TypeTerm::Literal(TypeLiteralTerm::Error);
        let operand = self.type_term_operand(term);

        Ok(Answer::Ready(operand))
    }

    /// Return whether strict identity can compare both operands.
    fn identity_compatible(
        &mut self,
        left: TermId<TypeTerm>,
        right: TermId<TypeTerm>,
    ) -> CompilerResult<bool> {
        let left = self.supports_identity(left)?;
        let right = self.supports_identity(right)?;

        Ok(left && right)
    }

    /// Return whether one type carries scalar or reference identity.
    fn supports_identity(&mut self, term: TermId<TypeTerm>) -> CompilerResult<bool> {
        let is_supported = match self.inference.term(term) {
            TypeTerm::Literal(literal) => Self::literal_supports_identity(literal),
            TypeTerm::Form { payload, .. } => {
                let Some(term) = self.type_operand_term_id(*payload)? else {
                    return Ok(false);
                };

                return self.supports_identity(term);
            }
            TypeTerm::Union { elements } => {
                let elements = elements.iter().copied().collect::<Vec<_>>();

                for element in elements {
                    let Some(term) = self.type_operand_term_id(element)? else {
                        return Ok(false);
                    };
                    if !self.supports_identity(term)? {
                        return Ok(false);
                    }
                }

                true
            }
            TypeTerm::Function(_) => true,
            TypeTerm::Reference { .. } => false,
            _ => false,
        };

        Ok(is_supported)
    }

    /// Return whether one literal type carries scalar or reference identity.
    fn literal_supports_identity(literal: &TypeLiteralTerm) -> bool {
        matches!(
            literal,
            TypeLiteralTerm::Null
                | TypeLiteralTerm::Undefined
                | TypeLiteralTerm::Object
                | TypeLiteralTerm::Scalar(_)
                | TypeLiteralTerm::Primitive(_)
        )
    }
}
