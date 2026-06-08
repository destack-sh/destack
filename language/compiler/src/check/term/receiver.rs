use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::check::{
    CheckState, Definition, Origin, TypeLiteralTerm, TypeOperand, TypeTerm, VariableId,
};

/// Runtime contextual receiver term.
///
/// ```ds
/// this
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub(in crate::check) struct ReceiverTerm {
    /// The source receiver expression.
    pub(in crate::check) source: dir::GlobalNodeIdAny,
    /// The receiver syntax kind.
    pub(in crate::check) kind: dir::ReceiverKind,
    /// The receiver type variable.
    pub(in crate::check) ty: TypeOperand,
}

/// Runtime `super` receiver term.
///
/// ```ds
/// super
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub(in crate::check) struct SuperTerm {
    /// The source super expression.
    pub(in crate::check) source: dir::GlobalNodeIdAny,
    /// The visible lexical receiver.
    pub(in crate::check) receiver: Option<TypeOperand>,
}

impl ReceiverTerm {
    /// Return variables referenced by this term.
    pub(in crate::check) fn referenced_variables(
        &self,
        state: &CheckState<'_>,
    ) -> SmallVec<[VariableId; 2]> {
        self.ty.referenced_variables(state)
    }
}

impl SuperTerm {
    /// Return variables referenced by this term.
    pub(in crate::check) fn referenced_variables(
        &self,
        state: &CheckState<'_>,
    ) -> SmallVec<[VariableId; 2]> {
        let mut variables = SmallVec::new();

        if let Some(receiver) = self.receiver {
            variables.extend(receiver.referenced_variables(state));
        }

        variables
    }
}

impl CheckState<'_> {
    /// Reduce one contextual receiver to its selected receiver type.
    pub(in crate::check) fn reduce_receiver_term(
        &self,
        term: &ReceiverTerm,
    ) -> CompilerResult<Option<TypeTerm>> {
        let Some(ty) = self.type_operand_term(term.ty)? else {
            return Ok(None);
        };

        Ok(Some(ty))
    }

    /// Reduce `super` to the inherited class receiver type.
    pub(in crate::check) fn reduce_super_term(
        &mut self,
        module: ModuleId,
        term: &SuperTerm,
    ) -> CompilerResult<Option<TypeTerm>> {
        let Some(receiver) = term.receiver else {
            return Ok(Some(TypeTerm::Literal(TypeLiteralTerm::Error)));
        };
        let Some(receiver) = self.reduce_type_operand(Origin::Node(term.source), receiver)? else {
            return Ok(None);
        };
        let Some(TypeTerm::Reference {
            origin: _,
            symbol,
            arguments: _,
        }) = self.type_operand_term(receiver)?
        else {
            return Ok(Some(TypeTerm::Literal(TypeLiteralTerm::Error)));
        };

        let extends = match self.definition(module, symbol)? {
            Some(Definition::Class(definition)) => definition.extends.clone(),
            _ => None,
        };
        let Some(extends) = extends else {
            return Ok(Some(TypeTerm::Literal(TypeLiteralTerm::Error)));
        };

        let arguments = extends
            .instance
            .map(|instance| instance.arguments.into_vec())
            .unwrap_or_default();

        Ok(Some(TypeTerm::Reference {
            origin: Origin::Node(extends.source),
            symbol: extends.symbol,
            arguments: arguments.into(),
        }))
    }
}
