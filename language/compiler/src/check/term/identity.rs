use destack_dir as dir;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::check::{
    CheckState, IdentityDecision, IdentityFailure, IdentityResolution, TypeLiteralTerm,
    TypeOperand, TypeTerm, VariableId,
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

impl IdentityTerm {
    /// Return variables referenced by this term.
    pub(in crate::check) fn referenced_variables(
        &self,
        state: &CheckState<'_>,
    ) -> SmallVec<[VariableId; 2]> {
        let mut variables = SmallVec::new();

        variables.extend(self.left.referenced_variables(state));
        variables.extend(self.right.referenced_variables(state));

        variables
    }
}

impl CheckState<'_> {
    /// Reduce one strict identity comparison to boolean.
    pub(in crate::check) fn reduce_identity_term(
        &mut self,
        identity: &IdentityTerm,
    ) -> CompilerResult<Option<TypeTerm>> {
        let Some(left) = self.type_operand_term(identity.left)? else {
            return Ok(None);
        };
        let Some(right) = self.type_operand_term(identity.right)? else {
            return Ok(None);
        };

        // choose identity comparison only for identity bearing domains
        let is_compatible = self.identity_compatible(&left, &right)?;
        if is_compatible {
            let resolution = IdentityResolution {
                source: identity.source,
                left: identity.left,
                right: identity.right,
            };

            self.select_identity(identity.source, IdentityDecision::Resolved(resolution))?;

            return Ok(Some(TypeTerm::Literal(TypeLiteralTerm::boolean())));
        }

        self.select_identity(
            identity.source,
            IdentityDecision::Rejected(IdentityFailure::Incompatible),
        )?;

        Ok(None)
    }

    /// Return whether strict identity can compare both operands.
    fn identity_compatible(&mut self, left: &TypeTerm, right: &TypeTerm) -> CompilerResult<bool> {
        let left = self.supports_identity(left)?;
        let right = self.supports_identity(right)?;

        Ok(left && right)
    }

    /// Return whether one type carries scalar or reference identity.
    fn supports_identity(&mut self, term: &TypeTerm) -> CompilerResult<bool> {
        let is_supported = match term {
            TypeTerm::Literal(literal) => Self::literal_supports_identity(literal),
            TypeTerm::Form { payload, .. } => {
                let Some(term) = self.type_operand_term(*payload)? else {
                    return Ok(false);
                };

                return self.supports_identity(&term);
            }
            TypeTerm::Union { elements } => {
                for element in elements {
                    let Some(term) = self.type_operand_term(*element)? else {
                        return Ok(false);
                    };
                    if !self.supports_identity(&term)? {
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
