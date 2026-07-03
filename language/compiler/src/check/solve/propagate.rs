use destack_dir as dir;

use crate::CompilerResult;
use crate::check::{
    Answer, CheckState, Origin, Relation, TryPropagation, TryPropagationTarget, answer,
};

impl CheckState<'_> {
    /// Propagate one fallible try value into its propagation target.
    pub(in crate::check) fn run_propagate(
        &mut self,
        propagation: TryPropagation,
    ) -> CompilerResult<Answer<()>> {
        let value = answer!(self.node_type_at(propagation.value)?);

        match propagation.target {
            TryPropagationTarget::Failure { ty } => {
                let origin = Origin::Node(propagation.source, propagation.value.scope);
                let failure = answer!(self.try_residual(origin, propagation.source, value)?);
                let is_assignable =
                    answer!(self.constrain(origin, Relation::Assignable, failure, ty,)?);
                if !is_assignable {
                    self.report_relation_failure(origin, Relation::Assignable, None, failure, ty)?;
                }

                Ok(Answer::Ready(()))
            }
            TryPropagationTarget::Return { ty } => {
                let origin = Origin::Node(propagation.source, propagation.value.scope);

                self.run_return_propagation(origin, propagation.source, value, ty)
            }
        }
    }

    /// Propagate one try value through an enclosing return type.
    fn run_return_propagation(
        &mut self,
        origin: Origin,
        source: dir::GlobalNodeIdAny,
        value: dir::GlobalTypeId,
        return_type: Option<dir::GlobalTypeId>,
    ) -> CompilerResult<Answer<()>> {
        let Some(return_type) = return_type else {
            self.report_try_outside_function(source);

            return Ok(Answer::Ready(()));
        };

        let is_implemented =
            answer!(self.decide_try_propagation(origin, source, value, return_type)?);
        if !is_implemented {
            self.report_try_propagation_not_implemented(source, value, return_type);
        }

        Ok(Answer::Ready(()))
    }

    /// Decide whether a propagated try failure fits an enclosing return type.
    fn decide_try_propagation(
        &mut self,
        origin: Origin,
        source: dir::GlobalNodeIdAny,
        value: dir::GlobalTypeId,
        return_type: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<bool>> {
        let residual = answer!(self.try_residual(origin, source, value)?);

        // require the return type to accept the residual
        let symbol = self.language_symbol(dir::LanguageItem::FromResidual);
        let arguments = self.intern_type_ids(source.module_id, &[residual])?;
        let target = self.intern_type(
            source.module_id,
            dir::Type::Instance(dir::GenericInstance { symbol, arguments }),
        )?;

        self.decide_relation(origin, Relation::Implements, return_type, target)
    }

    /// Return the residual projected from one try value.
    fn try_residual(
        &mut self,
        origin: Origin,
        source: dir::GlobalNodeIdAny,
        value: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<dir::GlobalTypeId>> {
        self.reduce_operation_type(
            self.origin_at(origin, source),
            dir::TypeOperation::TryResidual { value },
        )
    }
}
