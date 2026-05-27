use destack_dir as dir;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::check::{
    CheckState, IdentityDecision, IdentityFailure, IdentityResolution, TypeLiteralTerm, TypeTerm,
    VariableId,
};

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

impl CheckState<'_> {
    /// Reduce one strict identity comparison to boolean.
    pub(in crate::check) fn reduce_identity_term(
        &mut self,
        identity: &IdentityTerm,
    ) -> CompilerResult<Option<TypeTerm>> {
        let Some(left) = self.solved_type_term(identity.left)? else {
            return Ok(None);
        };
        let Some(right) = self.solved_type_term(identity.right)? else {
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

            self.record_identity_decision(identity.source, IdentityDecision::Resolved(resolution));

            return Ok(Some(TypeTerm::Literal(TypeLiteralTerm::boolean())));
        }

        self.record_identity_decision(
            identity.source,
            IdentityDecision::Rejected(IdentityFailure::Incompatible),
        );

        Ok(None)
    }

    /// Return whether strict identity can compare both operands.
    fn identity_compatible(&self, left: &TypeTerm, right: &TypeTerm) -> CompilerResult<bool> {
        let left = self.supports_identity(left)?;
        let right = self.supports_identity(right)?;

        Ok(left && right)
    }

    /// Return whether one type carries scalar or reference identity.
    fn supports_identity(&self, term: &TypeTerm) -> CompilerResult<bool> {
        let is_supported = match term {
            TypeTerm::Variable(variable) => {
                let Some(term) = self.solved_type_term(*variable)? else {
                    return Ok(false);
                };

                return self.supports_identity(&term);
            }
            TypeTerm::Literal(literal) => Self::literal_supports_identity(literal),
            TypeTerm::Reference { symbol, .. } => self.symbol_supports_identity(*symbol)?,
            TypeTerm::Form { payload, .. } => {
                let Some(term) = self.solved_type_term(*payload)? else {
                    return Ok(false);
                };

                return self.supports_identity(&term);
            }
            TypeTerm::Union { elements } => {
                for element in elements {
                    let Some(term) = self.solved_type_term(*element)? else {
                        return Ok(false);
                    };
                    if !self.supports_identity(&term)? {
                        return Ok(false);
                    }
                }

                true
            }
            TypeTerm::Function(_) => true,
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

    /// Return whether one nominal symbol carries reference identity.
    fn symbol_supports_identity(&self, symbol: dir::GlobalSymbolId) -> CompilerResult<bool> {
        let binding_table = self.input(symbol.module_id).binding_table();
        let symbol = binding_table.get_symbol(symbol.local_id);

        Ok(matches!(
            symbol.kind,
            dir::SymbolKind::Class | dir::SymbolKind::Function
        ))
    }
}
