use destack_dir as dir;

use crate::CompilerResult;
use crate::check::{
    Answer, CheckState, Definition, Origin, TypeLiteralTerm, TypeOperand, TypeTerm,
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
    /// The declaration that supplies the lexical receiver.
    pub(in crate::check) owner: Option<dir::GlobalSymbolId>,
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

impl CheckState<'_> {
    /// Reduce one contextual receiver to its selected receiver type.
    pub(in crate::check) fn reduce_receiver_term(
        &mut self,
        term: ReceiverTerm,
    ) -> CompilerResult<Answer<TypeOperand>> {
        if self.type_operand_term_id(term.ty)?.is_none() {
            return Ok(Answer::pending(term.ty.dependencies(self)));
        }

        Ok(Answer::Ready(term.ty))
    }

    /// Reduce `super` to the inherited class receiver type.
    pub(in crate::check) fn reduce_super_term(
        &mut self,
        term: SuperTerm,
    ) -> CompilerResult<Answer<TypeOperand>> {
        let Some(receiver) = term.receiver else {
            let ty = self.type_term_operand(TypeTerm::Literal(TypeLiteralTerm::Error));

            return Ok(Answer::Ready(ty));
        };
        let Answer::Ready(receiver) =
            self.reduce_type_operand(Origin::Node(term.source), receiver)?
        else {
            return Ok(Answer::pending(receiver.dependencies(self)));
        };
        let Some(term_id) = self.type_operand_term_id(receiver)? else {
            return Ok(Answer::pending(receiver.dependencies(self)));
        };
        let TypeTerm::Reference {
            origin: _,
            symbol,
            arguments: _,
        } = self.inference.term(term_id)
        else {
            let ty = self.type_term_operand(TypeTerm::Literal(TypeLiteralTerm::Error));

            return Ok(Answer::Ready(ty));
        };
        let symbol = *symbol;

        let extends = match self.definitions.definition(symbol) {
            Some(Definition::Class(definition)) => definition.extends.clone(),
            _ => None,
        };
        let Some(extends) = extends else {
            let ty = self.type_term_operand(TypeTerm::Literal(TypeLiteralTerm::Error));

            return Ok(Answer::Ready(ty));
        };

        let arguments = extends
            .instance
            .map(|instance| instance.arguments.into_vec())
            .unwrap_or_default();

        let ty = self.type_term_operand(TypeTerm::Reference {
            origin: Origin::Node(extends.source),
            symbol: extends.symbol,
            arguments: arguments.into(),
        });

        Ok(Answer::Ready(ty))
    }
}
