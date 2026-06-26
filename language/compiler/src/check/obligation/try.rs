use destack_artifact::DiagnosticBuilder;
use destack_dir as dir;

use crate::check::{
    Answer, CheckState, Origin, Relation, TryPropagationObligation, TryPropagationTarget,
    TryPropagationValue, answer,
};
use crate::{CheckError, CompilerResult};

impl CheckState<'_> {
    /// Check whether one try expression can propagate through its target.
    pub(in crate::check) fn check_try_propagates(
        &mut self,
        obligation: &TryPropagationObligation,
    ) -> CompilerResult<Answer<Option<DiagnosticBuilder<CheckError>>>> {
        let value = match obligation.value {
            TryPropagationValue::Type(ty) => ty,
            TryPropagationValue::Node(node) => answer!(self.node_type_answer(node)?),
        };

        match obligation.target {
            TryPropagationTarget::Failure { ty } => {
                let origin = Origin::Node(obligation.source);
                let failure = answer!(self.try_residual(obligation.source, value)?);
                answer!(self.constrain(origin, Relation::Assignable, failure, ty)?);

                Ok(Answer::Ready(None))
            }
            TryPropagationTarget::Return { ty } => {
                let Some(return_type) = ty else {
                    let (module, anchor) = self.source_anchor(obligation.source);
                    let diagnostic = CheckError::TryOutsideFunction { anchor, module };

                    return Ok(Answer::Ready(Some(diagnostic.into())));
                };

                if answer!(self.decide_try_propagation(obligation.source, value, return_type)?) {
                    return Ok(Answer::Ready(None));
                }

                let source_text = self.format_type(return_type);
                let target_text = format!("FromResidual<{}>", self.format_type(value));
                let (module, anchor) = self.source_anchor(obligation.source);

                let error = CheckError::InterfaceNotImplemented {
                    anchor,
                    module,
                    source: source_text,
                    target: target_text,
                };

                Ok(Answer::Ready(Some(error.note(
                    "the '?' operator propagates failures into the enclosing return type",
                ))))
            }
        }
    }

    /// Decide whether a propagated try failure fits an enclosing return type.
    fn decide_try_propagation(
        &mut self,
        source: dir::GlobalNodeIdAny,
        value: dir::GlobalTypeId,
        return_type: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<bool>> {
        let origin = Origin::Node(source);
        let residual = answer!(self.try_residual(source, value)?);

        // require the return type to accept the residual
        let symbol = self.language_symbol(dir::LanguageItem::FromResidual);
        let target = self.push_type(
            source.module_id,
            dir::Type::Instance(dir::GenericInstance {
                symbol,
                arguments: vec![residual],
            }),
            source.local_id,
        )?;

        self.decide_relation(origin, Relation::Implements, return_type, target)
    }

    /// Return the failure channel projected from one try value.
    fn try_residual(
        &mut self,
        source: dir::GlobalNodeIdAny,
        value: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<dir::GlobalTypeId>> {
        let operation = dir::TypeOperation::TryResidual { value };
        let residual = self.push_type(
            source.module_id,
            dir::Type::Operation(operation),
            source.local_id,
        )?;
        let residual = answer!(self.reduce_type_root(Origin::Node(source), residual)?);

        Ok(Answer::Ready(residual))
    }
}
